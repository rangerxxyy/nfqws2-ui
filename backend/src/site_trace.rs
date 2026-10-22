//! Types and bounded parsing for an nfqws2 website trace.

use serde::{Deserialize, Serialize};
use std::{
    net::Ipv4Addr,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    net::lookup_host,
    process::{Child, Command},
    time::timeout,
};

pub const MAX_TRACE_LINES: usize = 1_500;
pub const MAX_TRACE_LINE_BYTES: usize = 2_048;
pub const MAX_TRACE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracePhase {
    pub state: String,
    pub ok: Option<bool>,
    pub code: Option<u16>,
    pub duration_ms: Option<u64>,
    pub reason: String,
}

impl TracePhase {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: "unavailable".into(),
            ok: None,
            code: None,
            duration_ms: None,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceReport {
    pub host: String,
    pub address: Option<String>,
    pub without_nfqws2: TracePhase,
    pub with_nfqws2: TracePhase,
    pub trace_lines: Vec<String>,
    pub trace_truncated: bool,
    pub trace_hostname: Option<String>,
    pub trace_profile: Option<String>,
    pub trace_actions: Vec<String>,
    pub trace_warnings: Vec<String>,
    pub summary: String,
}

pub fn trace_insights(
    lines: &[String],
) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let mut hostname = None;
    let mut profile = None;
    let mut actions = Vec::new();
    let mut warnings = Vec::new();
    for line in lines {
        if let Some(start) = line.find("hostname='") {
            let value = &line[start + 10..];
            if let Some(end) = value.find('\'') {
                hostname = Some(value[..end].to_owned());
            }
        }
        if let Some(rest) = line.strip_prefix("desync profile ") {
            let id = rest.split_whitespace().next().unwrap_or_default();
            if id != "0" && id.parse::<u16>().is_ok() {
                profile = Some(
                    rest.split_once('(')
                        .and_then(|(_, name)| name.split(')').next())
                        .map_or_else(|| format!("#{id}"), |name| format!("#{id} {name}")),
                );
            }
        }
        if let Some(event) = line.strip_prefix("LUA: ") {
            let event = event.split(" : ").next().unwrap_or(event).trim().to_owned();
            if [
                "not inside",
                "not found",
                "skipped",
                "error",
                "cannot",
                "fail",
            ]
            .iter()
            .any(|word| event.to_ascii_lowercase().contains(word))
            {
                warnings.push(event);
            } else {
                actions.push(event);
            }
        } else if line.to_ascii_lowercase().contains("lua error")
            || line.to_ascii_lowercase().starts_with("error")
        {
            warnings.push(line.clone());
        }
    }
    actions.truncate(100);
    warnings.truncate(100);
    (hostname, profile, actions, warnings)
}

pub fn select_trace_queue(args: &[String], firewall_snapshot: &str) -> Option<u16> {
    let mut occupied = std::collections::HashSet::new();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let q = if arg == "--qnum" {
            iter.next().and_then(|value| value.parse::<u16>().ok())
        } else {
            arg.strip_prefix("--qnum=")
                .and_then(|value| value.parse::<u16>().ok())
        };
        if let Some(q) = q {
            occupied.insert(q);
        }
    }
    for line in firewall_snapshot.lines() {
        for marker in ["--queue-num ", "--queue-balance "] {
            if let Some(value) = line
                .split_once(marker)
                .map(|(_, rest)| rest.split_whitespace().next().unwrap_or_default())
                && let Some(q) = value.split('-').next().and_then(|n| n.parse::<u16>().ok())
            {
                occupied.insert(q);
            }
        }
    }
    (301..=399).find(|queue| !occupied.contains(queue))
}

pub fn append_kernel_queue_listeners(snapshot: &mut String, proc_queue_status: &str) {
    for line in proc_queue_status.lines() {
        if let Some(queue) = line
            .split_whitespace()
            .next()
            .and_then(|value| u16::from_str_radix(value, 16).ok())
        {
            snapshot.push_str(&format!("\n--queue-num {queue}"));
        }
    }
}

pub fn trace_argv(args: &[String], queue: u16, debug_path: &Path) -> Vec<String> {
    let mut result = Vec::with_capacity(args.len() + 3);
    let mut skip_value = false;
    for arg in args.iter().skip(1) {
        if skip_value {
            skip_value = false;
            continue;
        }
        if ["--qnum", "--user", "--uid", "--pidfile"].contains(&arg.as_str()) {
            skip_value = true;
            continue;
        }
        if ["--daemon", "--daemonize", "--debug"].contains(&arg.as_str())
            || ["--qnum=", "--debug=", "--user=", "--uid=", "--pidfile="]
                .iter()
                .any(|prefix| arg.starts_with(prefix))
        {
            continue;
        }
        result.push(arg.clone());
    }
    result.extend([
        format!("--qnum={queue}"),
        "--user=nobody".into(),
        format!("--debug=@{}", debug_path.display()),
    ]);
    result
}

pub fn firewall_trace_safe(firewall_snapshot: &str) -> bool {
    let mut hooks = std::collections::HashSet::new();
    for line in firewall_snapshot.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.first() == Some(&"-P")
            && let Some(hook) = tokens.get(1)
        {
            hooks.insert(*hook);
        }
    }
    ["PREROUTING", "OUTPUT", "POSTROUTING"]
        .iter()
        .all(|hook| hooks.contains(hook))
}

pub fn policy_route_mark(firewall_snapshot: &str) -> Option<String> {
    for line in firewall_snapshot.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.first() != Some(&"-A") || tokens.get(1) != Some(&"nfqws_post") {
            continue;
        }
        let Some(module) = tokens.windows(2).position(|pair| pair == ["-m", "mark"]) else {
            continue;
        };
        let Some(mark_flag) = tokens.iter().position(|token| *token == "--mark") else {
            continue;
        };
        let negated = mark_flag > module + 2 && tokens[mark_flag - 1] == "!";
        if !negated || !tokens.windows(2).any(|pair| pair == ["-j", "RETURN"]) {
            continue;
        }
        let Some(value) = tokens.get(mark_flag + 1) else {
            continue;
        };
        if valid_policy_mark(value) {
            return Some((*value).to_owned());
        }
    }
    None
}

fn valid_policy_mark(value: &str) -> bool {
    let Some((mark, mask)) = value.split_once('/') else {
        return false;
    };
    let is_hex = |value: &str| {
        value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .is_some_and(|digits| {
                !digits.is_empty()
                    && digits.len() <= 8
                    && digits.bytes().all(|b| b.is_ascii_hexdigit())
            })
    };
    is_hex(mark)
        && is_hex(mask)
        && ["0xfffffff", "0x0fffffff"].contains(&mask.to_ascii_lowercase().as_str())
}

async fn command(program: &str, args: &[&str]) -> Result<(i32, Vec<u8>), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Не удалось запустить {program}: {e}"))?;
    let result = timeout(Duration::from_secs(4), async {
        let mut out = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            stdout
                .take(8193)
                .read_to_end(&mut out)
                .await
                .map_err(|e| e.to_string())?;
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((status.code().unwrap_or(-1), out))
    })
    .await;
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(message)) => {
            stop_child(&mut child).await;
            Err(message)
        }
        Err(_) => {
            stop_child(&mut child).await;
            Err(format!("Таймаут команды {program}"))
        }
    }
}

fn program_path(name: &str) -> String {
    let candidates = match name {
        "iptables" => [
            "/opt/sbin/iptables",
            "/usr/sbin/iptables",
            "/sbin/iptables",
            "",
        ],
        "curl" => ["/opt/bin/curl", "/usr/bin/curl", "/bin/curl", ""],
        "chown" => [
            "/opt/bin/chown",
            "/opt/sbin/chown",
            "/usr/bin/chown",
            "/bin/chown",
        ],
        _ => ["", "", "", ""],
    };
    candidates
        .iter()
        .find(|path| !path.is_empty() && Path::new(path).is_file())
        .map_or_else(|| name.to_owned(), |path| (*path).to_owned())
}

async fn iptables(args: &[&str]) -> Result<(), String> {
    let program = program_path("iptables");
    let (status, output) = command(&program, args).await?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "{} {}: {}",
            program,
            args.join(" "),
            String::from_utf8_lossy(&output).trim()
        ))
    }
}

fn fw_args(
    action: &str,
    hook: &str,
    address: Ipv4Addr,
    direction: &str,
    port: u16,
    queue: Option<u16>,
) -> Vec<String> {
    let mut args = vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        action.into(),
        hook.into(),
    ];
    if action == "-I" {
        args.push("1".into());
    }
    args.extend(["-p".into(), "tcp".into()]);
    if direction == "out" {
        args.extend([
            "-d".into(),
            address.to_string(),
            "--dport".into(),
            port.to_string(),
            "--sport".into(),
            "40000:40999".into(),
        ]);
    } else {
        args.extend([
            "-s".into(),
            address.to_string(),
            "--sport".into(),
            port.to_string(),
            "--dport".into(),
            "40000:40999".into(),
        ]);
    }
    if let Some(queue_num) = queue {
        args.extend([
            "-m".into(),
            "mark".into(),
            "!".into(),
            "--mark".into(),
            "0x40000000/0x40000000".into(),
        ]);
        args.extend([
            "-j".into(),
            "NFQUEUE".into(),
            "--queue-num".into(),
            queue_num.to_string(),
            "--queue-bypass".into(),
        ]);
    } else {
        args.extend(["-j".into(), "ACCEPT".into()]);
    }
    args
}

fn trace_chain_rules(
    address: Ipv4Addr,
    port: u16,
    direction: &str,
    queue: Option<u16>,
) -> Vec<Vec<String>> {
    let mut rules = Vec::new();
    let mut primary = fw_args("-A", "trace-chain", address, direction, port, queue);
    if queue.is_some() {
        let conn_direction = if direction == "out" {
            "original"
        } else {
            "reply"
        };
        let insert = primary
            .iter()
            .position(|part| part == "-j")
            .unwrap_or(primary.len());
        primary.splice(
            insert..insert,
            [
                "-m",
                "connbytes",
                "--connbytes-dir",
                conn_direction,
                "--connbytes-mode",
                "packets",
                "--connbytes",
                "1:15",
            ]
            .map(str::to_owned),
        );
        rules.push(primary);
        rules.push(fw_args("-A", "trace-chain", address, direction, port, None));
    } else {
        rules.push(primary);
    }
    rules
}

fn trace_chain_name(token: &str, direction: &str) -> String {
    format!("N2U{}{}", if direction == "out" { 'O' } else { 'I' }, token)
}

fn trace_hook(direction: &str) -> &'static str {
    if direction == "out" {
        "POSTROUTING"
    } else {
        "PREROUTING"
    }
}

fn policy_mark_rule(chain: &str, mark: &str) -> Option<Vec<String>> {
    if !valid_policy_mark(mark) || !chain.starts_with("N2UM") {
        return None;
    }
    Some(vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        "-A".into(),
        chain.into(),
        "-p".into(),
        "tcp".into(),
        "--sport".into(),
        "40000:40999".into(),
        "-j".into(),
        "MARK".into(),
        "--set-xmark".into(),
        mark.into(),
    ])
}

fn chain_jump(action: &str, hook: &str, chain: &str) -> Vec<String> {
    let mut args = vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        action.into(),
        hook.into(),
    ];
    if action == "-I" {
        args.push("1".into());
    }
    args.extend(["-j".into(), chain.into()]);
    args
}

fn hook_jump(
    action: &str,
    hook: &str,
    address: Ipv4Addr,
    direction: &str,
    port: u16,
    chain: &str,
) -> Vec<String> {
    let mut args = vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        action.into(),
        hook.into(),
    ];
    if action == "-I" {
        args.push("1".into());
    }
    args.extend(["-p".into(), "tcp".into()]);
    if direction == "out" {
        args.extend([
            "-d".into(),
            address.to_string(),
            "--dport".into(),
            port.to_string(),
            "--sport".into(),
            "40000:40999".into(),
        ]);
    } else {
        args.extend([
            "-s".into(),
            address.to_string(),
            "--sport".into(),
            port.to_string(),
            "--dport".into(),
            "40000:40999".into(),
        ]);
    }
    args.extend(["-j".into(), chain.into()]);
    args
}

async fn iptables_owned(args: &[String]) -> Result<(), String> {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    iptables(&refs).await
}

async fn install_rules(
    address: Ipv4Addr,
    port: u16,
    queue: Option<u16>,
    token: &str,
    policy_mark: Option<&str>,
) -> Result<Vec<Vec<String>>, String> {
    let output_chain = trace_chain_name(token, "out");
    let input_chain = trace_chain_name(token, "in");
    let mut cleanup = Vec::new();
    for (chain, direction) in [(&output_chain, "out"), (&input_chain, "in")] {
        let create = vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-N".into(),
            chain.clone(),
        ];
        if let Err(error) = iptables_owned(&create).await {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                )),
            };
        }
        cleanup.push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-X".into(),
            chain.clone(),
        ]);
        let mut flush_registered = false;
        for mut rule in trace_chain_rules(address, port, direction, queue) {
            rule[4] = chain.clone();
            if let Err(error) = iptables_owned(&rule).await {
                return match remove_rules(&cleanup).await {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(format!(
                        "{error}; очистка частично установленных правил не подтверждена: {cleanup}"
                    )),
                };
            }
            if !flush_registered {
                cleanup.push(vec![
                    "-w".into(),
                    "-t".into(),
                    "mangle".into(),
                    "-F".into(),
                    chain.clone(),
                ]);
                flush_registered = true;
            }
        }
    }
    for (direction, chain) in [("out", output_chain.as_str()), ("in", input_chain.as_str())] {
        let hook = trace_hook(direction);
        let insert = hook_jump("-I", hook, address, direction, port, chain);
        if let Err(error) = iptables_owned(&insert).await {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error}; очистка частично подключённых цепочек не подтверждена: {cleanup_error}"
                )),
            };
        }
        cleanup.push(hook_jump("-D", hook, address, direction, port, chain));
    }
    if let Some(mark) = policy_mark {
        let chain = format!("N2UM{token}");
        let create = vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-N".into(),
            chain.clone(),
        ];
        if let Err(error) = iptables_owned(&create).await {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                )),
            };
        }
        cleanup.push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-X".into(),
            chain.clone(),
        ]);
        if let Some(rule) = policy_mark_rule(&chain, mark) {
            if let Err(error) = iptables_owned(&rule).await {
                return match remove_rules(&cleanup).await {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(format!(
                        "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                    )),
                };
            }
            cleanup.push(vec![
                "-w".into(),
                "-t".into(),
                "mangle".into(),
                "-F".into(),
                chain.clone(),
            ]);
        }
        let insert = chain_jump("-I", "OUTPUT", &chain);
        if let Err(error) = iptables_owned(&insert).await {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error}; очистка частично подключённых цепочек не подтверждена: {cleanup_error}"
                )),
            };
        }
        cleanup.push(chain_jump("-D", "OUTPUT", &chain));
    }
    Ok(cleanup)
}

async fn remove_rules(rules: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for rule in rules.iter().rev() {
        let refs = rule.iter().map(String::as_str).collect::<Vec<_>>();
        if let Err(error) = iptables(&refs).await {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

async fn probe(
    host: &str,
    address: Ipv4Addr,
    proto: &str,
) -> Result<(Option<u16>, Option<u64>, String), String> {
    let program = program_path("curl");
    let port = if proto == "http" { 80 } else { 443 };
    let address = format!("{host}:{port}:{address}");
    let url = format!("{proto}://{host}/");
    let output = timeout(Duration::from_secs(10), async {
        let mut child = Command::new(&program)
            .args([
                "--disable",
                "--silent",
                "--show-error",
                "--insecure",
                "--noproxy",
                "*",
                "--connect-timeout",
                "4",
                "--max-time",
                "8",
                "--max-filesize",
                "16384",
                "--max-redirs",
                "0",
                "--local-port",
                "40000-40999",
                "--resolve",
                &address,
                "--output",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
                "--write-out",
                "\nNFQWS_TRACE|%{http_code}|%{time_total}|%{errormsg}|%{time_connect}|%{time_appconnect}|%{time_starttransfer}",
                &url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("curl недоступен: {e}"))?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("Нет stdout curl")?
            .take(1025)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((status.code().unwrap_or(-1), bytes))
    })
    .await
    .map_err(|_| "Таймаут HTTP-запроса".to_string())??;
    let text = String::from_utf8_lossy(&output.1);
    let fields = text
        .rsplit_once("NFQWS_TRACE|")
        .map(|(_, row)| row.trim().split('|').collect::<Vec<_>>())
        .unwrap_or_default();
    let code = fields
        .first()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|v| (100..600).contains(v));
    let ms = fields
        .get(1)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| (v * 1000.0).round() as u64);
    let metric = |index: usize| {
        fields
            .get(index)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or_default()
    };
    let reason = if let Some(code) = code {
        format!("Получен HTTP {code}; сервер ответил")
    } else {
        trace_failure_reason(
            proto,
            fields.get(2).copied().unwrap_or_default(),
            metric(4),
            metric(5),
            metric(6),
            output.0,
        )
    };
    Ok((code, ms, reason))
}

fn trace_failure_reason(
    proto: &str,
    curl_error: &str,
    time_connect: f64,
    time_appconnect: f64,
    time_starttransfer: f64,
    exit_code: i32,
) -> String {
    let stage = if time_connect == 0.0 {
        "TCP-подключения"
    } else if proto == "https" && time_appconnect == 0.0 {
        "TLS"
    } else if time_starttransfer == 0.0 {
        "HTTP-ответа"
    } else {
        "передачи данных"
    };
    let error = if curl_error.trim().is_empty() {
        format!("curl завершился с кодом {exit_code}")
    } else {
        curl_error.trim().to_owned()
    };
    let label = if error.to_ascii_lowercase().contains("timed out")
        || error.to_ascii_lowercase().contains("timeout")
    {
        "Тайм-аут"
    } else {
        "Ошибка"
    };
    format!("{label} на этапе {stage}: {error}")
}

fn trace_summary(without_nfqws2: Option<bool>, with_nfqws2: Option<bool>) -> &'static str {
    match (without_nfqws2, with_nfqws2) {
        (Some(true), Some(true)) => "Сайт ответил в обоих режимах",
        (Some(false), Some(true)) => "Сайт ответил только через nfqws2",
        (Some(true), Some(false)) => {
            "Базовый запрос прошёл, а с nfqws2 нет; проверьте журнал профиля"
        }
        (Some(false), Some(false)) => {
            "HTTP-ответ не получен ни в одном режиме; причина не установлена"
        }
        _ => "Результат неопределён: один из этапов завершился технической ошибкой",
    }
}

async fn stop_child(child: &mut Child) {
    let _ = child.start_kill();
    let _ = timeout(Duration::from_secs(2), child.wait()).await;
}

pub async fn run(
    host: String,
    proto: &str,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    let mut report = TraceReport {
        host: host.clone(),
        address: None,
        without_nfqws2: TracePhase::unavailable("Базовая проверка ещё не выполнена"),
        with_nfqws2: TracePhase::unavailable("Трассировка ещё не выполнена"),
        trace_lines: Vec::new(),
        trace_truncated: false,
        trace_hostname: None,
        trace_profile: None,
        trace_actions: Vec::new(),
        trace_warnings: Vec::new(),
        summary: String::new(),
    };
    let port = if proto == "http" { 80 } else { 443 };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let token = format!(
        "{:04x}{:010x}",
        std::process::id() & 0xffff,
        nonce as u64 & 0xffffffffff
    );
    if cancel.load(Ordering::Acquire) {
        report.summary = "Трассировка остановлена до начала проверки".into();
        return report;
    }
    let resolved = match timeout(Duration::from_secs(5), lookup_host((host.as_str(), port))).await {
        Ok(Ok(values)) => values
            .filter_map(|socket| match socket.ip() {
                std::net::IpAddr::V4(ip) if public_v4(ip) => Some(ip),
                _ => None,
            })
            .next(),
        _ => None,
    };
    let Some(address) = resolved else {
        report.without_nfqws2 =
            TracePhase::unavailable("DNS не вернул публичный IPv4; сетевые правила не менялись");
        report.with_nfqws2 = TracePhase::unavailable("Тестовая очередь не запускалась");
        report.summary = "Не найден публичный IPv4-адрес".into();
        return report;
    };
    report.address = Some(address.to_string());
    if cancel.load(Ordering::Acquire) {
        report.summary = "Трассировка остановлена до изменения firewall".into();
        return report;
    }
    let iptables_program = program_path("iptables");
    let firewall_snapshot = command(&iptables_program, &["-w", "-t", "mangle", "-S"])
        .await
        .ok()
        .filter(|(status, _)| *status == 0)
        .map(|(_, output)| String::from_utf8_lossy(&output).into_owned())
        .unwrap_or_default();
    if firewall_snapshot.is_empty() || !firewall_trace_safe(&firewall_snapshot) {
        report.without_nfqws2 = TracePhase::unavailable(
            "Не найдены mangle hooks PREROUTING/OUTPUT/POSTROUTING; firewall не изменялся",
        );
        report.with_nfqws2 =
            TracePhase::unavailable("Трассировка остановлена до изменения firewall");
        report.summary = "Трассировка недоступна: firewall не менялся".into();
        return report;
    }
    if cancel.load(Ordering::Acquire) {
        report.summary = "Трассировка остановлена до изменения firewall".into();
        return report;
    }
    let policy_mark = policy_route_mark(&firewall_snapshot);
    let (base_code, base_ms, base_reason) =
        match install_rules(address, port, None, &token, policy_mark.as_deref()).await {
            Ok(rules) => {
                if cancel.load(Ordering::Acquire) {
                    let cleanup = remove_rules(&rules).await;
                    report.without_nfqws2 = TracePhase::unavailable(match cleanup {
                        Ok(()) => "Остановлено до запроса; временные правила удалены".into(),
                        Err(error) => {
                            format!("Остановлено; очистка правил не подтверждена: {error}")
                        }
                    });
                    report.summary = "Трассировка остановлена".into();
                    return report;
                }
                let result = probe(&host, address, proto).await;
                let cleanup = remove_rules(&rules).await;
                match (result, cleanup) {
                    (Ok(value), Ok(())) => value,
                    (Err(e), Ok(())) => (None, None, e),
                    (_, Err(e)) => (
                        None,
                        None,
                        format!("Не удалось очистить временные правила: {e}"),
                    ),
                }
            }
            Err(error) => {
                report.without_nfqws2 = TracePhase::unavailable(format!(
                    "Не удалось изолировать базовый запрос: {error}"
                ));
                report.with_nfqws2 = TracePhase::unavailable("Тестовая очередь не запускалась");
                report.summary = "Проверка не завершена".into();
                return report;
            }
        };
    report.without_nfqws2 = phase_from_http(base_code, base_ms, base_reason);
    if report
        .without_nfqws2
        .reason
        .contains("Не удалось очистить временные правила")
    {
        report.with_nfqws2 = TracePhase::unavailable(
            "Трассировка остановлена, потому что очистка базовых правил не подтверждена",
        );
        report.summary = "Ошибка очистки временных правил".into();
        return report;
    }
    if cancel.load(Ordering::Acquire) {
        report.with_nfqws2 = TracePhase::unavailable("Трассировка остановлена пользователем");
        report.summary = "Трассировка остановлена после базовой проверки".into();
        return report;
    }
    if args.is_empty() {
        report.with_nfqws2 = TracePhase::unavailable(
            "Не найден единственный активный процесс nfqws2; базовая проверка сохранена",
        );
        report.summary = "Базовый результат есть; активный nfqws2 не найден".into();
        return report;
    }
    let mut firewall_snapshot = command(&iptables_program, &["-w", "-t", "mangle", "-S"])
        .await
        .ok()
        .filter(|(code, _)| *code == 0)
        .map(|(_, lines)| String::from_utf8_lossy(&lines).into_owned())
        .unwrap_or_default();
    if let Ok(queue_status) = tokio::fs::read_to_string("/proc/net/netfilter/nfnetlink_queue").await
    {
        append_kernel_queue_listeners(&mut firewall_snapshot, &queue_status);
    }
    if firewall_snapshot.is_empty() {
        report.with_nfqws2 = TracePhase::unavailable(
            "Не удалось проверить занятые очереди; новая очередь не запускалась",
        );
        report.summary = "Есть базовый результат; трассировка недоступна".into();
        return report;
    }
    let Some(queue) = select_trace_queue(&args, &firewall_snapshot) else {
        report.with_nfqws2 =
            TracePhase::unavailable("Не удалось выбрать свободную NFQUEUE; конфликты не менялись");
        report.summary = "Есть базовый ответ; тестовая очередь недоступна".into();
        return report;
    };
    let debug_path = std::env::temp_dir().join(format!(
        "nfqws2-ui-trace-{}-{queue}-{nonce}.log",
        std::process::id()
    ));
    let debug_file = match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&debug_path)
        .await
    {
        Ok(file) => file,
        Err(error) => {
            report.with_nfqws2 = TracePhase::unavailable(format!(
                "Не удалось подготовить временный журнал: {error}"
            ));
            report.summary = "Тест nfqws2 недоступен".into();
            return report;
        }
    };
    drop(debug_file);
    #[cfg(unix)]
    {
        let Some(path_arg) = debug_path.to_str() else {
            let _ = tokio::fs::remove_file(&debug_path).await;
            report.with_nfqws2 =
                TracePhase::unavailable("Путь временного журнала не является UTF-8");
            report.summary = "Тест nfqws2 недоступен".into();
            return report;
        };
        let owner = command("chown", &["nobody", path_arg]).await;
        if !matches!(owner, Ok((0, _))) {
            let _ = tokio::fs::remove_file(&debug_path).await;
            report.with_nfqws2 = TracePhase::unavailable(
                "Не удалось назначить временный журнал тестовому пользователю; доступ к журналу не расширялся",
            );
            report.summary = "Тест nfqws2 недоступен".into();
            return report;
        }
        if let Ok(metadata) = tokio::fs::metadata(&debug_path).await {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            if tokio::fs::set_permissions(&debug_path, permissions)
                .await
                .is_err()
            {
                let _ = tokio::fs::remove_file(&debug_path).await;
                report.with_nfqws2 = TracePhase::unavailable(
                    "Не удалось разрешить тестовому процессу записать временный журнал",
                );
                report.summary = "Тест nfqws2 недоступен".into();
                return report;
            }
        }
    }
    let rules = match install_rules(address, port, Some(queue), &token, policy_mark.as_deref())
        .await
    {
        Ok(rules) => rules,
        Err(error) => {
            report.with_nfqws2 =
                TracePhase::unavailable(format!("Не удалось поставить временные правила: {error}"));
            report.summary = "Тест nfqws2 недоступен".into();
            let _ = tokio::fs::remove_file(&debug_path).await;
            return report;
        }
    };
    if cancel.load(Ordering::Acquire) {
        let cleanup = remove_rules(&rules).await;
        let _ = tokio::fs::remove_file(&debug_path).await;
        report.with_nfqws2 = TracePhase::unavailable(match cleanup {
            Ok(()) => "Трассировка остановлена пользователем; правила очищены".into(),
            Err(error) => format!("Остановлено, очистка правил не подтверждена: {error}"),
        });
        report.summary = "Трассировка остановлена".into();
        return report;
    }
    let traced_args = trace_argv(&args, queue, &debug_path);
    let process = Command::new(&binary)
        .args(&traced_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn();
    match process {
        Ok(mut child) => {
            tokio::time::sleep(Duration::from_millis(350)).await;
            let started = std::time::Instant::now();
            let result = match child.try_wait() {
                Ok(None) => probe(&host, address, proto).await,
                Ok(Some(status)) => Err(format!(
                    "Тестовый nfqws2 завершился до запроса (код {})",
                    status.code().unwrap_or(-1)
                )),
                Err(error) => Err(format!(
                    "Не удалось проверить процесс тестового nfqws2: {error}"
                )),
            };
            stop_child(&mut child).await;
            report.with_nfqws2 = result
                .map(|(code, ms, reason)| {
                    phase_from_http(
                        code,
                        ms.or(Some(started.elapsed().as_millis() as u64)),
                        reason,
                    )
                })
                .unwrap_or_else(TracePhase::unavailable);
        }
        Err(error) => {
            report.with_nfqws2 = TracePhase::unavailable(format!(
                "Не удалось запустить изолированный nfqws2: {error}"
            ))
        }
    }
    if let Err(error) = remove_rules(&rules).await {
        report.with_nfqws2 = TracePhase::unavailable(format!(
            "Тест завершён, очистка временных правил не подтверждена: {error}"
        ));
    }
    let cancelled = cancel.load(Ordering::Acquire);
    if cancelled {
        report.with_nfqws2 = TracePhase::unavailable("Трассировка остановлена пользователем");
    }
    if let Ok(file) = tokio::fs::File::open(&debug_path).await {
        let mut bytes = Vec::new();
        if file
            .take((MAX_TRACE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .is_ok()
        {
            report.trace_truncated = bytes.len() > MAX_TRACE_BYTES;
            let bounded = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_TRACE_BYTES)]);
            let (lines, truncated) = bounded_trace_lines(&bounded);
            report.trace_lines = lines;
            report.trace_truncated |= truncated;
        }
    }
    if report.trace_lines.is_empty() && report.with_nfqws2.ok == Some(true) {
        report.with_nfqws2 = TracePhase::unavailable(
            "HTTP-ответ получен, но журнал --debug пуст; прохождение запроса через тестовую очередь не подтверждено",
        );
    }
    (
        report.trace_hostname,
        report.trace_profile,
        report.trace_actions,
        report.trace_warnings,
    ) = trace_insights(&report.trace_lines);
    let _ = tokio::fs::remove_file(debug_path).await;
    if cancelled {
        report.summary = "Трассировка остановлена; временный процесс и правила очищены".into();
        return report;
    }
    report.summary = trace_summary(report.without_nfqws2.ok, report.with_nfqws2.ok).into();
    report
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    let private_or_reserved = a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && (b == 168 || (b == 0 && (c == 0 || c == 2))))
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113);
    !private_or_reserved
}

pub fn phase_from_http(
    code: Option<u16>,
    duration_ms: Option<u64>,
    reason: impl Into<String>,
) -> TracePhase {
    match code.filter(|code| (100..600).contains(code)) {
        Some(code) => TracePhase {
            state: "complete".into(),
            ok: Some(true),
            code: Some(code),
            duration_ms,
            reason: reason.into(),
        },
        None => TracePhase {
            state: "error".into(),
            ok: Some(false),
            code: None,
            duration_ms,
            reason: reason.into(),
        },
    }
}

pub fn bounded_trace_lines(input: &str) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut used = 0usize;
    let mut truncated = false;
    for line in input.lines() {
        if lines.len() >= MAX_TRACE_LINES || used >= MAX_TRACE_BYTES {
            truncated = true;
            break;
        }
        let sanitized = line
            .chars()
            .filter(|ch| !ch.is_control() || *ch == '\t')
            .collect::<String>();
        let clean = truncate_utf8(&sanitized, MAX_TRACE_LINE_BYTES);
        if clean.len() < sanitized.len() {
            truncated = true;
        }
        let remaining = MAX_TRACE_BYTES - used;
        if remaining == 0 {
            truncated = true;
            break;
        }
        let bounded = truncate_utf8(&clean, remaining);
        if bounded.len() < clean.len() {
            truncated = true;
        }
        used += bounded.len();
        lines.push(bounded);
    }
    (lines, truncated)
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    let end = value
        .char_indices()
        .take_while(|(index, ch)| index + ch.len_utf8() <= max_bytes)
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
        .unwrap_or(0);
    value[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_http_status_is_a_reachable_site_response() {
        for code in [200, 301, 404, 503] {
            let phase = phase_from_http(Some(code), Some(40), "получен ответ");
            assert_eq!(phase.ok, Some(true));
            assert_eq!(phase.code, Some(code));
        }
        assert_eq!(phase_from_http(None, None, "timeout").ok, Some(false));
    }

    #[test]
    fn timeout_reason_identifies_whether_tcp_tls_or_http_reached_timeout() {
        assert_eq!(
            trace_failure_reason("https", "Connection timed out", 0.0, 0.0, 0.0, 28),
            "Тайм-аут на этапе TCP-подключения: Connection timed out"
        );
        assert_eq!(
            trace_failure_reason("https", "Operation timed out", 0.2, 0.0, 0.0, 28),
            "Тайм-аут на этапе TLS: Operation timed out"
        );
        assert_eq!(
            trace_failure_reason("https", "Operation timed out", 0.2, 0.4, 0.0, 28),
            "Тайм-аут на этапе HTTP-ответа: Operation timed out"
        );
    }

    #[test]
    fn two_missing_http_responses_are_not_reported_as_an_internal_error() {
        assert_eq!(
            trace_summary(Some(false), Some(false)),
            "HTTP-ответ не получен ни в одном режиме; причина не установлена"
        );
        assert_eq!(
            trace_summary(Some(true), Some(true)),
            "Сайт ответил в обоих режимах"
        );
    }

    #[test]
    fn trace_log_is_bounded_and_keeps_text_as_text() {
        let input = format!(
            "<script>alert(1)</script>\n{}",
            "я".repeat(MAX_TRACE_LINE_BYTES + 10)
        );
        let (lines, truncated) = bounded_trace_lines(&input);
        assert_eq!(lines[0], "<script>alert(1)</script>");
        assert!(lines[1].len() <= MAX_TRACE_LINE_BYTES);
        assert!(truncated);
    }

    #[test]
    fn trace_queue_skips_active_and_firewall_queues() {
        let args = vec!["nfqws2".into(), "--qnum=301".into()];
        assert_eq!(
            select_trace_queue(&args, "-A PREROUTING -j NFQUEUE --queue-num 302"),
            Some(303)
        );
    }

    #[test]
    fn trace_queue_skips_kernel_queue_listeners() {
        let mut snapshot = String::new();
        append_kernel_queue_listeners(&mut snapshot, "0000012d 00000000 00000001\nnot-a-queue");
        assert_eq!(select_trace_queue(&[], &snapshot), Some(302));
    }

    #[test]
    fn temporary_firewall_rules_are_limited_to_target_and_owned_source_ports() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let baseline = fw_args("-A", "N2UOabc123def0", target, "out", 443, None);
        assert!(
            baseline
                .windows(2)
                .any(|pair| pair == ["-d", "93.184.216.34"])
        );
        assert!(
            baseline
                .windows(2)
                .any(|pair| pair == ["--sport", "40000:40999"])
        );
        assert!(baseline.ends_with(&["-j".into(), "ACCEPT".into()]));

        let queued = fw_args("-A", "N2UIabc123def0", target, "in", 80, Some(303));
        assert!(
            queued
                .windows(2)
                .any(|pair| pair == ["-s", "93.184.216.34"])
        );
        assert!(queued.windows(2).any(|pair| pair == ["--sport", "80"]));
        assert!(
            queued
                .windows(2)
                .any(|pair| pair == ["--mark", "0x40000000/0x40000000"])
        );
        assert!(queued.ends_with(&["--queue-num".into(), "303".into(), "--queue-bypass".into()]));
    }

    #[test]
    fn operation_firewall_rules_use_private_chains_and_exact_hook_jumps() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let chain = trace_chain_name("abcd1234ef", "out");
        assert_eq!(chain, "N2UOabcd1234ef");
        assert!(chain.len() <= 28);
        let insert = hook_jump("-I", "POSTROUTING", target, "out", 443, &chain);
        let delete = hook_jump("-D", "POSTROUTING", target, "out", 443, &chain);
        assert!(insert.windows(2).any(|pair| pair == ["-j", chain.as_str()]));
        assert!(delete.windows(2).any(|pair| pair == ["-j", chain.as_str()]));
        assert!(insert.contains(&"40000:40999".into()));
        assert!(insert.contains(&target.to_string()));
        assert!(
            hook_jump("-I", "PREROUTING", target, "in", 443, &chain)
                .windows(2)
                .any(|pair| pair == ["-j", chain.as_str()])
        );
    }

    #[test]
    fn detects_only_policy_mark_rules_and_reapplies_the_mark_before_route_lookup() {
        let rules = "-N nfqws_post\n-A nfqws_post -m mark ! --mark 0x123/0x0fffffff -j RETURN";
        assert_eq!(
            policy_route_mark(rules).as_deref(),
            Some("0x123/0x0fffffff")
        );
        assert_eq!(
            policy_route_mark("-A nfqws_post -m mark --mark 0x123/0x0fffffff -j RETURN"),
            None
        );
        assert_eq!(
            policy_route_mark("-A nfqws_post -m mark ! --mark injected;id -j RETURN"),
            None
        );
        let rule = policy_mark_rule("N2UMabc123", "0x123/0x0fffffff").unwrap();
        assert!(
            rule.windows(2)
                .any(|pair| pair == ["--sport", "40000:40999"])
        );
        assert!(
            rule.windows(2)
                .any(|pair| pair == ["--set-xmark", "0x123/0x0fffffff"])
        );
        assert!(rule.ends_with(&["--set-xmark".into(), "0x123/0x0fffffff".into()]));
    }

    #[test]
    fn trace_chain_accepts_reinjected_packets_before_existing_nfqueue_rules() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let rules = trace_chain_rules(target, 443, "out", Some(303));
        assert_eq!(rules.len(), 2);
        assert!(rules[0].windows(2).any(|pair| pair == ["-j", "NFQUEUE"]));
        assert!(rules[1].ends_with(&["-j".into(), "ACCEPT".into()]));
        assert!(
            rules[1]
                .windows(2)
                .any(|pair| pair == ["--sport", "40000:40999"])
        );
        assert_eq!(trace_hook("out"), "POSTROUTING");
        assert_eq!(trace_hook("in"), "PREROUTING");
    }

    #[test]
    fn trace_argv_removes_primary_process_identity_and_debug_options() {
        let args = vec![
            "nfqws2".into(),
            "--qnum".into(),
            "7".into(),
            "--debug=/tmp/main.log".into(),
            "--user=nobody".into(),
            "--debug".into(),
            "--filter-tcp=443".into(),
        ];
        let result = trace_argv(&args, 301, Path::new("/tmp/trace.log"));
        assert!(
            !result
                .iter()
                .any(|arg| arg.contains("main.log") || arg == "7" || arg == "--qnum")
        );
        assert!(result.contains(&"--filter-tcp=443".to_owned()));
        assert!(result.contains(&"--qnum=301".to_owned()));
        assert!(result.contains(&"--debug=@/tmp/trace.log".to_owned()));
    }

    #[test]
    fn trace_requires_only_the_mangle_hooks_it_uses() {
        let hooks =
            "-P PREROUTING ACCEPT\n-P INPUT ACCEPT\n-P OUTPUT ACCEPT\n-P POSTROUTING ACCEPT";
        assert!(firewall_trace_safe(hooks));
        assert!(!firewall_trace_safe("-P OUTPUT ACCEPT\n-P INPUT ACCEPT"));
    }

    #[test]
    fn trace_summary_extracts_host_profile_lua_actions_and_warnings() {
        let lines = vec![
            "packet: hostname='video.example'".into(),
            "desync profile 2 (youtube) matches".into(),
            "LUA: split packet".into(),
            "LUA: not found".into(),
        ];
        let (host, profile, actions, warnings) = trace_insights(&lines);
        assert_eq!(host.as_deref(), Some("video.example"));
        assert_eq!(profile.as_deref(), Some("#2 youtube"));
        assert_eq!(actions, ["split packet"]);
        assert_eq!(warnings, ["not found"]);
    }

    #[tokio::test]
    async fn cancelled_trace_returns_before_dns_or_firewall_commands() {
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let report = run(
            "example.com".into(),
            "https",
            Vec::new(),
            String::new(),
            cancelled,
        )
        .await;
        assert_eq!(report.address, None);
        assert!(report.summary.contains("остановлена"));
    }
}
