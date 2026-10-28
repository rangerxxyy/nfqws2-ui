//! Types and bounded parsing for an nfqws2 website trace.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::Ipv4Addr,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
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
static POLICY_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub type TraceProgressCallback = Arc<dyn Fn(&'static str, TraceReport) + Send + Sync>;

#[derive(Default)]
struct TraceRunContext {
    known_baseline: Option<bool>,
    progress: Option<TraceProgressCallback>,
    candidate_repeats: usize,
}

#[derive(Clone, Copy)]
pub enum ProbeTransport {
    Tcp,
    Udp,
}

fn transport_rule(mut rule: Vec<String>, transport: ProbeTransport) -> Vec<String> {
    if let Some(index) = rule.iter().position(|part| part == "-p") {
        rule[index + 1] = match transport {
            ProbeTransport::Tcp => "tcp",
            ProbeTransport::Udp => "udp",
        }
        .into();
    }
    rule
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracePhase {
    pub state: String,
    pub ok: Option<bool>,
    pub code: Option<u16>,
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_classification: Option<String>,
    pub reason: String,
}

impl TracePhase {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: "unavailable".into(),
            ok: None,
            code: None,
            duration_ms: None,
            diagnostic_classification: None,
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
    #[serde(skip)]
    pub candidate_attempts: Vec<TracePhase>,
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

pub async fn read_policy_route_mark() -> Result<Option<String>, String> {
    let program = program_path("iptables");
    let (status, output) = command(&program, &["-w", "-t", "mangle", "-S", "nfqws_post"]).await?;
    if status != 0 {
        return Err(format!(
            "Не удалось прочитать правило метки из nfqws_post: {}",
            String::from_utf8_lossy(&output).trim()
        ));
    }
    Ok(policy_route_mark(&String::from_utf8_lossy(&output)))
}

pub struct PolicyProbeRoute {
    cleanup: Option<Vec<Vec<String>>>,
}

fn policy_probe_route_guard(chain: &str) -> PolicyProbeRoute {
    PolicyProbeRoute {
        cleanup: Some(vec![vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-X".into(),
            chain.into(),
        ]]),
    }
}

impl PolicyProbeRoute {
    pub async fn cleanup(mut self) -> Result<(), String> {
        let Some(rules) = self.cleanup.as_ref().cloned() else {
            return Ok(());
        };
        match remove_rules(&rules).await {
            Ok(()) => {
                self.cleanup = None;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

impl Drop for PolicyProbeRoute {
    fn drop(&mut self) {
        let Some(rules) = self.cleanup.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = remove_rules(&rules).await;
            });
        }
    }
}

pub async fn install_policy_probe_route(mark: &str) -> Result<PolicyProbeRoute, String> {
    if !valid_policy_mark(mark) {
        return Err("Маршрутный fwmark не прошёл проверку формата".into());
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or_default()
        ^ POLICY_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let token = format!(
        "{:04x}{:010x}",
        std::process::id() & 0xffff,
        nonce & 0xffffffffff
    );
    let chain = format!("N2UM{token}");
    // Arm cleanup before the create call. A timeout may occur after the kernel
    // accepted -N, so the route must already know how to remove the chain.
    let mut route = policy_probe_route_guard(&chain);
    let create = vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        "-N".into(),
        chain.clone(),
    ];
    if let Err(error) = iptables_owned(&create).await {
        return Err(match route.cleanup().await {
            Ok(()) => error,
            Err(cleanup_error) => format!(
                "{error}; не удалось подтвердить очистку временной цепочки после создания: {cleanup_error}"
            ),
        });
    }

    let Some(rule) = policy_mark_rule_for_ports(&chain, mark, "41000:41099") else {
        return Err(match route.cleanup().await {
            Ok(()) => "Маршрутный fwmark не прошёл проверку формата".into(),
            Err(cleanup_error) => format!(
                "Маршрутный fwmark не прошёл проверку; не удалось удалить тестовую цепочку: {cleanup_error}"
            ),
        });
    };
    if let Err(error) = iptables_owned(&rule).await {
        return Err(match route.cleanup().await {
            Ok(()) => error,
            Err(cleanup_error) => format!(
                "{error}; не удалось удалить тестовую цепочку после ошибки: {cleanup_error}"
            ),
        });
    }
    route
        .cleanup
        .as_mut()
        .expect("cleanup route is armed")
        .push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-F".into(),
            chain.clone(),
        ]);

    let jump = chain_jump("-I", "OUTPUT", &chain);
    // Register the inverse before attempting the insert: an iptables command can
    // time out after the kernel accepted the rule, and cleanup must still try to remove it.
    route
        .cleanup
        .as_mut()
        .expect("cleanup route is armed")
        .push(chain_jump("-D", "OUTPUT", &chain));
    if let Err(error) = iptables_owned(&jump).await {
        return Err(match route.cleanup().await {
            Ok(()) => error,
            Err(cleanup_error) => format!(
                "{error}; не удалось очистить тестовую цепочку после ошибки: {cleanup_error}"
            ),
        });
    }
    Ok(route)
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

async fn drain_bounded(
    mut stream: impl tokio::io::AsyncRead + Unpin,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut kept = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let size = stream.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if size == 0 {
            return Ok(kept);
        }
        let retain = size.min(limit.saturating_sub(kept.len()));
        kept.extend_from_slice(&chunk[..retain]);
    }
}

async fn command(program: &str, args: &[&str]) -> Result<(i32, Vec<u8>), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Не удалось запустить {program}: {e}"))?;
    let result = timeout(Duration::from_secs(4), async {
        let stdout = child.stdout.take().ok_or("Нет stdout команды")?;
        let stderr = child.stderr.take().ok_or("Нет stderr команды")?;
        let (read_out, read_err) =
            tokio::join!(drain_bounded(stdout, 65536), drain_bounded(stderr, 8192));
        let mut out = read_out?;
        let err = read_err?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !status.success() {
            out.extend_from_slice(&err);
        }
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
    fw_args_for_ports(action, hook, address, direction, port, queue, "40000:40999")
}

fn fw_args_for_ports(
    action: &str,
    hook: &str,
    address: Ipv4Addr,
    direction: &str,
    port: u16,
    queue: Option<u16>,
    source_ports: &str,
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
            source_ports.into(),
        ]);
    } else {
        args.extend([
            "-s".into(),
            address.to_string(),
            "--sport".into(),
            port.to_string(),
            "--dport".into(),
            source_ports.into(),
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
    policy_mark_rule_for_ports(chain, mark, "40000:40999")
}

fn policy_mark_rule_for_ports(chain: &str, mark: &str, source_ports: &str) -> Option<Vec<String>> {
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
        source_ports.into(),
        "-j".into(),
        "MARK".into(),
        "--set-xmark".into(),
        mark.into(),
    ])
}

fn batch_mark_rule(
    chain: &str,
    address: Ipv4Addr,
    local_source: Ipv4Addr,
    direction: &str,
    port: u16,
    lane: crate::probe_lanes::ProbeLane,
) -> Vec<String> {
    let mut rule = vec![
        "-w".into(),
        "-t".into(),
        "mangle".into(),
        "-A".into(),
        chain.into(),
        "-p".into(),
        "tcp".into(),
    ];
    if direction == "out" {
        rule.extend([
            "-s".into(),
            local_source.to_string(),
            "-d".into(),
            address.to_string(),
            "--sport".into(),
            format!("{}:{}", lane.source_port_start, lane.source_port_end),
            "--dport".into(),
            port.to_string(),
        ]);
    } else {
        rule.extend([
            "-m".into(),
            "conntrack".into(),
            "--ctorigsrc".into(),
            local_source.to_string(),
            "-s".into(),
            address.to_string(),
            "--sport".into(),
            port.to_string(),
            "--dport".into(),
            format!("{}:{}", lane.source_port_start, lane.source_port_end),
        ]);
    }
    rule.extend([
        "-j".into(),
        "MARK".into(),
        "--set-xmark".into(),
        format!("0x{:08x}/0x{:08x}", lane.mark, lane.mark_mask),
    ]);
    rule
}

fn batch_chain_rules(
    chain: &str,
    address: Ipv4Addr,
    local_source: Ipv4Addr,
    direction: &str,
    port: u16,
    queue: u16,
    lanes: &[crate::probe_lanes::ProbeLane],
) -> Result<Vec<Vec<String>>, String> {
    let first = lanes.first().ok_or("Не заданы дорожки для теста")?;
    let last = lanes.last().ok_or("Не заданы дорожки для теста")?;
    if lanes
        .iter()
        .enumerate()
        .any(|(index, lane)| lane.index != index || lane.source_port_start > lane.source_port_end)
    {
        return Err("Портовые дорожки имеют неверный порядок или диапазон".into());
    }
    let source_ports = format!("{}:{}", first.source_port_start, last.source_port_end);
    let mut rules = lanes
        .iter()
        .map(|lane| batch_mark_rule(chain, address, local_source, direction, port, *lane))
        .collect::<Vec<_>>();
    let mut queue_rule = fw_args_for_ports(
        "-A",
        chain,
        address,
        direction,
        port,
        Some(queue),
        &source_ports,
    );
    let conn_direction = if direction == "out" {
        "original"
    } else {
        "reply"
    };
    let insert = queue_rule
        .iter()
        .position(|part| part == "-j")
        .unwrap_or(queue_rule.len());
    queue_rule.splice(
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
    rules.push(queue_rule);
    rules.push(fw_args_for_ports(
        "-A",
        chain,
        address,
        direction,
        port,
        None,
        &source_ports,
    ));
    Ok(rules)
}

fn parse_fwmark(value: &str) -> Option<(u32, u32)> {
    let (mark, mask) = value.split_once('/').unwrap_or((value, "0xffffffff"));
    let parse = |part: &str| {
        part.strip_prefix("0x")
            .or_else(|| part.strip_prefix("0X"))
            .map_or_else(
                || part.parse::<u32>().ok(),
                |hex| u32::from_str_radix(hex, 16).ok(),
            )
    };
    Some((parse(mark)?, parse(mask)?))
}

/// Marks constrained by partial routing rules cannot be used for candidate
/// IDs. Full-mask entries are exact values and are handled as collisions.
fn router_mark_constraints(snapshot: &str) -> (u32, Vec<u32>) {
    let mut reserved_bits = 0_u32;
    let mut exact_marks = Vec::new();
    for line in snapshot.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let mut index = 0;
        while index + 1 < tokens.len() {
            let is_match = matches!(tokens[index], "--mark" | "fwmark");
            if is_match && let Some((mark, mask)) = parse_fwmark(tokens[index + 1]) {
                if mask == u32::MAX {
                    exact_marks.push(mark);
                } else {
                    reserved_bits |= mask;
                }
            }
            index += 1;
        }
    }
    (reserved_bits, exact_marks)
}

fn allocate_router_lanes(
    count: usize,
    port_start: u16,
    port_end: u16,
    firewall_snapshot: &str,
) -> Result<Vec<crate::probe_lanes::ProbeLane>, String> {
    let (mut reserved_bits, exact_marks) = router_mark_constraints(firewall_snapshot);
    for _ in 0..u32::BITS {
        let lanes =
            crate::probe_lanes::allocate_probe_lanes(count, port_start, port_end, reserved_bits)?;
        let collisions = lanes
            .iter()
            .filter(|lane| exact_marks.contains(&lane.mark))
            .collect::<Vec<_>>();
        if collisions.is_empty() {
            return Ok(lanes);
        }
        for lane in collisions {
            reserved_bits |= lane.mark;
        }
    }
    Err("Не удалось выделить fwmark без конфликта с правилами роутера".into())
}

fn select_batch_port_range(
    ephemeral_range: Option<(u16, u16)>,
    lane_count: usize,
) -> Option<(u16, u16)> {
    if lane_count == 0 || lane_count > crate::probe_lanes::MAX_BATCH_LANES {
        return None;
    }
    let count = u32::try_from(lane_count)
        .ok()?
        .checked_mul(u32::from(crate::probe_lanes::PORTS_PER_LANE))?;
    let high = u32::from(u16::MAX).checked_sub(count - 1)?;
    let mut candidates = (32_768_u32..=high)
        .step_by(count as usize)
        .chain((1024_u32..32_768_u32).rev().step_by(count as usize));
    candidates.find_map(|start| {
        let end = start.checked_add(count - 1)?;
        let (low, high) = (u16::try_from(start).ok()?, u16::try_from(end).ok()?);
        let overlaps_ephemeral = ephemeral_range.is_some_and(|(ephemeral_low, ephemeral_high)| {
            low <= ephemeral_high && ephemeral_low <= high
        });
        (!overlaps_ephemeral).then_some((low, high))
    })
}

fn batch_resource_budget() -> usize {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
    let metric = |key: &str| {
        meminfo.as_deref()?.lines().find_map(|line| {
            let (name, rest) = line.split_once(':')?;
            (name == key)
                .then(|| rest.split_whitespace().next()?.parse::<u64>().ok())
                .flatten()
        })
    };
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok());
    let cpus = std::thread::available_parallelism().ok().map(usize::from);
    crate::probe_lanes::safe_parallel_lane_count(
        metric("MemTotal"),
        metric("MemAvailable"),
        load,
        cpus,
    )
}

fn ephemeral_port_range() -> Option<(u16, u16)> {
    let contents = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range").ok()?;
    let mut ports = contents
        .split_whitespace()
        .filter_map(|value| value.parse::<u16>().ok());
    Some((ports.next()?, ports.next()?))
}

fn batch_debug_path(queue: u16, nonce: u64) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nfqws2-ui-batch-{}-{queue}-{nonce}.log",
        std::process::id()
    ))
}

async fn prepare_batch_debug_file(path: &Path) -> Result<(), String> {
    tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(|error| format!("Не удалось подготовить временный журнал: {error}"))?;
    #[cfg(unix)]
    {
        let Some(path_arg) = path.to_str() else {
            let _ = tokio::fs::remove_file(path).await;
            return Err("Путь временного журнала не является UTF-8".into());
        };
        if !matches!(command("chown", &["nobody", path_arg]).await, Ok((0, _))) {
            let _ = tokio::fs::remove_file(path).await;
            return Err("Не удалось назначить временный журнал тестовому пользователю".into());
        }
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            if tokio::fs::set_permissions(path, permissions).await.is_err() {
                let _ = tokio::fs::remove_file(path).await;
                return Err("Не удалось ограничить права временного журнала".into());
            }
        }
    }
    Ok(())
}

pub struct CandidateBatchRuntime {
    pub host: String,
    pub protocol: crate::strategy_picker::StrategyProtocol,
    pub active_args: Vec<String>,
    pub binary: String,
    pub functions: HashSet<String>,
    pub blobs: Vec<String>,
    pub cancel: Arc<AtomicBool>,
}

/// Run one full-picker catalog slice in one owned nfqws2 process. It returns
/// preliminary lane hits only; the caller must use the serial runner to
/// confirm positives before persisting them as working strategies.
pub async fn run_candidate_batch(
    runtime: CandidateBatchRuntime,
    candidates: &[crate::strategy_picker::StrategyCandidate],
) -> Result<Vec<crate::strategy_picker::StrategyBatchProbeResult>, String> {
    let CandidateBatchRuntime {
        host,
        protocol,
        active_args,
        binary,
        functions,
        blobs,
        cancel,
    } = runtime;
    if candidates.len() < 2 || candidates.len() > crate::probe_lanes::MAX_BATCH_LANES {
        return Err("Для безопасного пакета нужно от 2 до 3 кандидатов".into());
    }
    if !matches!(
        protocol,
        crate::strategy_picker::StrategyProtocol::Http
            | crate::strategy_picker::StrategyProtocol::Https
    ) {
        return Err("Пакетный runner поддерживает только HTTP/TLS".into());
    }
    if cancel.load(Ordering::Acquire) {
        return Err("Пакетная проверка отменена до запуска".into());
    }
    let port = if protocol == crate::strategy_picker::StrategyProtocol::Http {
        80
    } else {
        443
    };
    let address = timeout(Duration::from_secs(5), lookup_host((host.as_str(), port)))
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|values| {
            values
                .filter_map(|socket| match socket.ip() {
                    std::net::IpAddr::V4(ip) if public_v4(ip) => Some(ip),
                    _ => None,
                })
                .next()
        })
        .ok_or_else(|| "Пакетный runner не разрешил публичный IPv4".to_owned())?;

    let iptables = program_path("iptables");
    let firewall_snapshot = command(&iptables, &["-w", "-t", "mangle", "-S"])
        .await
        .ok()
        .filter(|(status, _)| *status == 0)
        .map(|(_, output)| String::from_utf8_lossy(&output).into_owned())
        .ok_or_else(|| {
            "Не удалось прочитать mangle-правила для collision check; пакет отключён".to_owned()
        })?;
    let ip_rules = command(&program_path("ip"), &["rule", "show"])
        .await
        .ok()
        .filter(|(status, _)| *status == 0)
        .map(|(_, output)| String::from_utf8_lossy(&output).into_owned())
        .ok_or_else(|| "Не удалось прочитать ip rule; пакет отключён".to_owned())?;
    let mut mark_snapshot = firewall_snapshot.clone();
    mark_snapshot.push('\n');
    mark_snapshot.push_str(&ip_rules);
    let lane_count = batch_resource_budget().min(candidates.len());
    if lane_count < 2 {
        return Err("Ресурсный лимит роутера разрешает только последовательную проверку".into());
    }
    let (source_port_start, source_port_end) =
        select_batch_port_range(ephemeral_port_range(), lane_count).ok_or_else(|| {
            "Не найден безопасный исходный диапазон портов; используйте последовательный режим"
                .to_owned()
        })?;
    let lanes = allocate_router_lanes(
        lane_count,
        source_port_start,
        source_port_end,
        &mark_snapshot,
    )?;
    let policy_mark = policy_route_mark(&firewall_snapshot);
    let mut queue_snapshot = firewall_snapshot.clone();
    let queue_status = tokio::fs::read_to_string("/proc/net/netfilter/nfnetlink_queue")
        .await
        .map_err(|error| format!("Не удалось проверить NFQUEUE listeners: {error}"))?;
    append_kernel_queue_listeners(&mut queue_snapshot, &queue_status);
    let Some(queue) = select_trace_queue(&active_args, &queue_snapshot) else {
        return Err("Не найдена свободная тестовая NFQUEUE".into());
    };
    let port = match protocol {
        crate::strategy_picker::StrategyProtocol::Http => 80,
        crate::strategy_picker::StrategyProtocol::Https => 443,
        crate::strategy_picker::StrategyProtocol::Quic => unreachable!("QUIC checked above"),
    };
    let batch_candidates = &candidates[..lane_count];
    let candidate_profiles = batch_candidates
        .iter()
        .zip(&lanes)
        .map(|(candidate, lane)| (candidate.steps.clone(), lane.mark, lane.mark_mask))
        .collect::<Vec<_>>();
    let profile_args = crate::strategy_picker::candidate_batch_argv(
        &active_args,
        protocol,
        &candidate_profiles,
        &functions,
        &blobs,
    )?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let debug_path = batch_debug_path(queue, nonce);
    prepare_batch_debug_file(&debug_path).await?;
    let mut traced_argv = vec![binary.clone()];
    traced_argv.extend(profile_args);
    let traced_args = trace_argv(&traced_argv, queue, &debug_path);
    let token = format!(
        "{:04x}{:010x}",
        std::process::id() & 0xffff,
        nonce & 0xffffffffff
    );
    let rules =
        match install_batch_rules(address, port, queue, &token, &lanes, policy_mark.as_deref())
            .await
        {
            Ok(rules) => rules,
            Err(error) => {
                let _ = tokio::fs::remove_file(&debug_path).await;
                if error.contains("очистка") || error.contains("удалить частично")
                {
                    return Err(format!("CLEANUP_UNCONFIRMED: {error}"));
                }
                return Err(format!("Пакетные правила не установлены: {error}"));
            }
        };

    let process_run = async {
        let mut child = Command::new(&binary)
            .args(&traced_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("nfqws2 batch process не запустился: {error}"))?;
        tokio::time::sleep(Duration::from_millis(350)).await;
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Err(format!(
                "nfqws2 batch process завершился до probes (код {:?})",
                status.code()
            ));
        }
        let mut jobs = Vec::new();
        for lane in &lanes {
            let host = host.clone();
            let proto = if protocol == crate::strategy_picker::StrategyProtocol::Http {
                "http"
            } else {
                "https"
            };
            let local_ports = format!("{}:{}", lane.source_port_start, lane.source_port_end);
            let cancel = cancel.clone();
            jobs.push(async move { probe(&host, address, proto, &local_ports, &cancel).await });
        }
        let outcomes = match jobs.len() {
            2 => {
                let (one, two) = tokio::join!(jobs.remove(0), jobs.remove(0));
                vec![one, two]
            }
            3 => {
                let first = jobs.remove(0);
                let second = jobs.remove(0);
                let third = jobs.remove(0);
                let (one, two, three) = tokio::join!(first, second, third);
                vec![one, two, three]
            }
            _ => return Err("Пакетный runner получил меньше двух lanes".into()),
        };
        stop_child(&mut child).await;
        Ok(outcomes)
    }
    .await;
    let cleanup = remove_rules(&rules).await;
    let debug = tokio::fs::read(&debug_path).await.unwrap_or_default();
    let _ = tokio::fs::remove_file(&debug_path).await;
    if let Err(error) = cleanup {
        return Err(format!(
            "CLEANUP_UNCONFIRMED: не удалось полностью удалить тестовые правила: {error}"
        ));
    }
    let outcomes = process_run?;
    if debug.is_empty() {
        return Err("Пакетные HTTP-ответы получены без подтверждения попадания в тестовую очередь; повторите последовательно".into());
    }
    Ok(outcomes
        .into_iter()
        .enumerate()
        .map(|(candidate_index, outcome)| {
            let (code, elapsed_ms, reason) = match outcome {
                Ok(values) => values,
                Err(error) => (None, None, error),
            };
            crate::strategy_picker::StrategyBatchProbeResult {
                candidate_index,
                elapsed_ms: elapsed_ms.unwrap_or_default(),
                probe: crate::strategy_picker::StrategyProbeResult {
                    baseline_ok: Some(false),
                    candidate_ok: Some(code.is_some()),
                    detail: reason,
                },
            }
        })
        .collect())
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
    hook_jump_for_ports(action, hook, address, direction, port, chain, "40000:40999")
}

fn hook_jump_for_ports(
    action: &str,
    hook: &str,
    address: Ipv4Addr,
    direction: &str,
    port: u16,
    chain: &str,
    source_ports: &str,
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
            source_ports.into(),
        ]);
    } else {
        args.extend([
            "-s".into(),
            address.to_string(),
            "--sport".into(),
            port.to_string(),
            "--dport".into(),
            source_ports.into(),
        ]);
    }
    args.extend(["-j".into(), chain.into()]);
    args
}

async fn iptables_owned(args: &[String]) -> Result<(), String> {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    iptables(&refs).await
}

fn local_flow_hook(mut rule: Vec<String>, direction: &str, source: Ipv4Addr) -> Vec<String> {
    let index = rule
        .iter()
        .position(|value| value == "-j")
        .unwrap_or(rule.len());
    let scope = if direction == "out" {
        vec!["-s".into(), source.to_string()]
    } else {
        vec![
            "-m".into(),
            "conntrack".into(),
            "--ctorigsrc".into(),
            source.to_string(),
        ]
    };
    rule.splice(index..index, scope);
    rule
}

async fn install_rules_for_transport(
    address: Ipv4Addr,
    port: u16,
    queue: Option<u16>,
    token: &str,
    policy_mark: Option<&str>,
    transport: ProbeTransport,
) -> Result<Vec<Vec<String>>, String> {
    // UDP connect selects a local route/source address without sending packets.
    // Match the original router source as well as test ports so forwarded LAN
    // clients using the same source ports are never captured by these hooks.
    let socket = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .map_err(|e| e.to_string())?;
    socket
        .connect((address, port))
        .await
        .map_err(|e| e.to_string())?;
    let source = match socket.local_addr().map_err(|e| e.to_string())?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_unspecified() => ip,
        _ => return Err("Не найден исходящий IPv4 роутера".into()),
    };
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
            let rule = transport_rule(rule, transport);
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
        let insert = transport_rule(
            local_flow_hook(
                hook_jump("-I", hook, address, direction, port, chain),
                direction,
                source,
            ),
            transport,
        );
        if let Err(error) = iptables_owned(&insert).await {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error}; очистка частично подключённых цепочек не подтверждена: {cleanup_error}"
                )),
            };
        }
        cleanup.push(transport_rule(
            local_flow_hook(
                hook_jump("-D", hook, address, direction, port, chain),
                direction,
                source,
            ),
            transport,
        ));
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
            let mut rule = transport_rule(rule, transport);
            let index = rule
                .iter()
                .position(|value| value == "-j")
                .unwrap_or(rule.len());
            rule.splice(
                index..index,
                [
                    "-d".into(),
                    address.to_string(),
                    "--dport".into(),
                    port.to_string(),
                ],
            );
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

async fn install_batch_rules(
    address: Ipv4Addr,
    port: u16,
    queue: u16,
    token: &str,
    lanes: &[crate::probe_lanes::ProbeLane],
    policy_mark: Option<&str>,
) -> Result<Vec<Vec<String>>, String> {
    if lanes.is_empty() || lanes.len() > crate::probe_lanes::MAX_BATCH_LANES {
        return Err("Размер тестового пакета вне безопасного диапазона".into());
    }
    let socket = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .map_err(|error| error.to_string())?;
    socket
        .connect((address, port))
        .await
        .map_err(|error| error.to_string())?;
    let source = match socket.local_addr().map_err(|error| error.to_string())?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_unspecified() => ip,
        _ => return Err("Не найден исходящий IPv4 роутера".into()),
    };
    let first = lanes.first().ok_or("Не заданы дорожки")?;
    let last = lanes.last().ok_or("Не заданы дорожки")?;
    let source_ports = format!("{}:{}", first.source_port_start, last.source_port_end);
    let mut cleanup = Vec::new();
    for (chain, direction) in [
        (trace_chain_name(token, "out"), "out"),
        (trace_chain_name(token, "in"), "in"),
    ] {
        let create = vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-N".into(),
            chain.clone(),
        ];
        if let Err(error) = iptables_owned(&create).await {
            let cleanup_result = remove_rules(&cleanup).await;
            return Err(match cleanup_result {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                ),
            });
        }
        cleanup.push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-X".into(),
            chain.clone(),
        ]);
        let rules = match batch_chain_rules(&chain, address, source, direction, port, queue, lanes)
        {
            Ok(rules) => rules,
            Err(error) => {
                let cleanup_result = remove_rules(&cleanup).await;
                return Err(match cleanup_result {
                    Ok(()) => error,
                    Err(cleanup_error) => format!(
                        "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                    ),
                });
            }
        };
        let mut flush_registered = false;
        for rule in rules {
            if let Err(error) = iptables_owned(&rule).await {
                let cleanup_result = remove_rules(&cleanup).await;
                return Err(match cleanup_result {
                    Ok(()) => error,
                    Err(cleanup_error) => format!(
                        "{error}; очистка частично установленных правил не подтверждена: {cleanup_error}"
                    ),
                });
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
    for (direction, chain) in [
        ("out", trace_chain_name(token, "out")),
        ("in", trace_chain_name(token, "in")),
    ] {
        let hook = trace_hook(direction);
        let insert = local_flow_hook(
            hook_jump_for_ports("-I", hook, address, direction, port, &chain, &source_ports),
            direction,
            source,
        );
        if let Err(error) = iptables_owned(&insert).await {
            let cleanup_result = remove_rules(&cleanup).await;
            return Err(match cleanup_result {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; очистка частично подключённых цепочек не подтверждена: {cleanup_error}"
                ),
            });
        }
        cleanup.push(local_flow_hook(
            hook_jump_for_ports("-D", hook, address, direction, port, &chain, &source_ports),
            direction,
            source,
        ));
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
            let cleanup_result = remove_rules(&cleanup).await;
            return Err(match cleanup_result {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; очистка частично созданных цепочек не подтверждена: {cleanup_error}"
                ),
            });
        }
        cleanup.push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-X".into(),
            chain.clone(),
        ]);
        let Some(mut rule) = policy_mark_rule_for_ports(&chain, mark, &source_ports) else {
            return match remove_rules(&cleanup).await {
                Ok(()) => Err("Маршрутный fwmark не прошёл проверку формата".to_owned()),
                Err(cleanup_error) => Err(format!(
                    "Не удалось удалить частично созданные правила после ошибки fwmark: {cleanup_error}"
                )),
            };
        };
        let jump = rule
            .iter()
            .position(|value| value == "-j")
            .unwrap_or(rule.len());
        rule.splice(
            jump..jump,
            [
                "-s".into(),
                source.to_string(),
                "-d".into(),
                address.to_string(),
                "--dport".into(),
                port.to_string(),
            ],
        );
        if let Err(error) = iptables_owned(&rule).await {
            let cleanup_result = remove_rules(&cleanup).await;
            return Err(match cleanup_result {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; очистка частично установленных правил не подтверждена: {cleanup_error}"
                ),
            });
        }
        cleanup.push(vec![
            "-w".into(),
            "-t".into(),
            "mangle".into(),
            "-F".into(),
            chain.clone(),
        ]);
        let insert = chain_jump("-I", "OUTPUT", &chain);
        if let Err(error) = iptables_owned(&insert).await {
            let cleanup_result = remove_rules(&cleanup).await;
            return Err(match cleanup_result {
                Ok(()) => error,
                Err(cleanup_error) => format!(
                    "{error}; очистка частично подключённой цепочки не подтверждена: {cleanup_error}"
                ),
            });
        }
        cleanup.push(chain_jump("-D", "OUTPUT", &chain));
    }
    Ok(cleanup)
}

/// The caller must keep this future alive through cleanup (the API runs the
/// whole diagnosis in an owned task, including when its HTTP client disconnects).
pub async fn bypass_probe<T>(
    address: Ipv4Addr,
    port: u16,
    transport: ProbeTransport,
    probe: impl std::future::Future<Output = T>,
) -> Result<T, String> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or_default();
    let token = format!(
        "{:04x}{:010x}",
        std::process::id() & 0xffff,
        nonce & 0xffffffffff
    );
    let rules = install_rules_for_transport(address, port, None, &token, None, transport).await?;
    let result = probe.await;
    remove_rules(&rules)
        .await
        .map_err(|error| format!("Не удалось удалить временные правила: {error}"))?;
    Ok(result)
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

fn http_probe_args(host: &str, address: Ipv4Addr, proto: &str, local_ports: &str) -> Vec<String> {
    let port = if proto == "http" { 80 } else { 443 };
    vec![
        "--disable".into(),
        "--silent".into(),
        "--show-error".into(),
        "--insecure".into(),
        "--noproxy".into(),
        "*".into(),
        "--connect-timeout".into(),
        "4".into(),
        "--max-time".into(),
        "8".into(),
        "--max-filesize".into(),
        "16384".into(),
        "--max-redirs".into(),
        "0".into(),
        "--local-port".into(),
        local_ports.into(),
        "--resolve".into(),
        format!("{host}:{port}:{address}"),
        "--output".into(),
        if cfg!(windows) { "NUL" } else { "/dev/null" }.into(),
        "--write-out".into(),
        "\nNFQWS_TRACE|%{http_code}|%{time_total}|%{errormsg}|%{time_connect}|%{time_appconnect}|%{time_starttransfer}".into(),
        format!("{proto}://{host}/"),
    ]
}

async fn probe(
    host: &str,
    address: Ipv4Addr,
    proto: &str,
    local_ports: &str,
    cancel: &AtomicBool,
) -> Result<(Option<u16>, Option<u64>, String), String> {
    let program = program_path("curl");
    let args = http_probe_args(host, address, proto, local_ports);
    let run = async {
        timeout(Duration::from_secs(10), async {
            let mut child = Command::new(&program)
                .args(&args)
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
        .map_err(|_| "Таймаут HTTP-запроса".to_string())?
    };
    tokio::pin!(run);
    let output = tokio::select! {
        result = &mut run => result?,
        _ = async {
            while !cancel.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } => return Err("HTTP-запрос отменён".into()),
    };
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

fn trace_summary_for_mode(
    mode: &TraceProbeMode,
    without_nfqws2: Option<bool>,
    with_nfqws2: Option<bool>,
) -> &'static str {
    if !matches!(mode, TraceProbeMode::Quic(_)) {
        return trace_summary(without_nfqws2, with_nfqws2);
    }
    match (without_nfqws2, with_nfqws2) {
        (Some(true), Some(true)) => "QUIC Initial ответил напрямую и через nfqws2",
        (Some(false), Some(true)) => "QUIC Initial ответил только через nfqws2",
        (Some(true), Some(false)) => "QUIC ответил напрямую, но не через nfqws2",
        (Some(false), Some(false)) => "QUIC Initial не получил ответа ни в одном режиме",
        _ => "Результат QUIC неопределён: один из этапов завершился технической ошибкой",
    }
}

fn phase_from_probe_result(
    mode: &TraceProbeMode,
    result: (Option<u16>, Option<u64>, String, Option<String>),
) -> TracePhase {
    let (code, duration_ms, reason, classification) = result;
    let mut phase = phase_from_http(code, duration_ms, reason);
    if matches!(mode, TraceProbeMode::Quic(_)) {
        // The synthetic success code is internal to the existing result pipeline;
        // do not present it to the user as an HTTP response code.
        phase.code = None;
        phase.diagnostic_classification = Some("quic_initial".into());
    } else {
        phase.diagnostic_classification = classification;
    }
    phase
}

async fn stop_child(child: &mut Child) {
    let _ = child.start_kill();
    let _ = timeout(Duration::from_secs(2), child.wait()).await;
}

async fn probe_padded_head(
    host: &str,
    address: Ipv4Addr,
    cancel: &AtomicBool,
) -> Result<(Option<u16>, Option<u64>, String, Option<String>), String> {
    let args = crate::freeze_diagnostic::padded_head_curl_args(address, 443, host)?;
    let started = std::time::Instant::now();
    let mut child = Command::new(program_path("curl"))
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Не удалось запустить padded curl-запрос: {error}"))?;
    let result = tokio::select! {
        result = timeout(Duration::from_secs(105), child.wait()) => result,
        _ = async {
            while !cancel.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } => {
            let _ = child.start_kill();
            let _ = timeout(Duration::from_secs(2), child.wait()).await;
            return Ok((None, Some(started.elapsed().as_millis() as u64), "Проверка отменена; тестовый curl остановлен".into(), None));
        }
    };
    let elapsed_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(status)) => {
            let mut stdout = Vec::new();
            if let Some(pipe) = child.stdout.take() {
                use tokio::io::AsyncReadExt;
                let _ = timeout(
                    Duration::from_secs(1),
                    pipe.take(64 * 1024).read_to_end(&mut stdout),
                )
                .await;
            }
            let details = crate::freeze_diagnostic::classify_padded_head(
                status.code().unwrap_or(-1),
                &String::from_utf8_lossy(&stdout),
            );
            let complete =
                details.classification == crate::freeze_diagnostic::PadClassification::Complete;
            let classification = format!("{:?}", details.classification);
            Ok((
                complete.then_some(200),
                Some(details.elapsed_ms.max(elapsed_ms)),
                details.reason,
                Some(classification),
            ))
        }
        Ok(Err(error)) => Err(format!("Не удалось выполнить padded curl-запрос: {error}")),
        Err(_) => Ok((
            None,
            Some(elapsed_ms),
            "Тайм-аут padded HEAD-проверки".into(),
            Some("TimeoutBeforeResponse".into()),
        )),
    }
}

async fn probe_mode(
    host: &str,
    address: Ipv4Addr,
    mode: &TraceProbeMode,
    cancel: &AtomicBool,
) -> Result<(Option<u16>, Option<u64>, String, Option<String>), String> {
    match mode {
        TraceProbeMode::PaddedHead => probe_padded_head(host, address, cancel).await,
        TraceProbeMode::Quic(binary) => {
            let packet = tokio::select! {
                result = crate::quic_probe::generate(binary, host) => result?,
                _ = wait_for_cancel(cancel) => return Err("Проверка QUIC остановлена".into()),
            };
            let result = tokio::select! {
                result = crate::quic_probe::probe(&packet, address, true) => result,
                _ = wait_for_cancel(cancel) => return Err("Проверка QUIC остановлена".into()),
            };
            Ok((
                (result.status == "ok").then_some(200),
                result.duration_ms,
                result.detail,
                None,
            ))
        }
        TraceProbeMode::Transfer(path, allow_upload_probe) => {
            let mut attempts = Vec::with_capacity(2);
            let mut expected_bytes = None;
            for _ in 0..2 {
                if cancel.load(Ordering::Acquire) {
                    return Err("Проверка передачи отменена".into());
                }
                let parsed = crate::transfer_diagnostic::probe_transfer_at_path_with_upload_probe(
                    host,
                    address,
                    path,
                    false,
                    *allow_upload_probe,
                    cancel,
                )
                .await?;
                expected_bytes = parsed.expected_bytes.or(expected_bytes);
                let should_repeat = crate::transfer_diagnostic::should_probe_boundary_stall(
                    &parsed.attempt,
                    expected_bytes,
                );
                attempts.push(parsed.attempt);
                if !should_repeat {
                    break;
                }
            }
            let diagnosis =
                crate::transfer_diagnostic::classify_transfer(&attempts, expected_bytes);
            let min = diagnosis.min_bytes.unwrap_or_default();
            let max = diagnosis.max_bytes.unwrap_or_default();
            let bytes = if min == max {
                format!("{min} байт")
            } else {
                format!("{min}–{max} байт")
            };
            let reason = format!(
                "{} · получено {bytes} за {} запроса",
                diagnosis.summary,
                attempts.len()
            );
            Ok((
                crate::transfer_diagnostic::successful_http_response(&attempts).then_some(200),
                Some(attempts.iter().map(|attempt| attempt.elapsed_ms).sum()),
                reason,
                Some(format!("{:?}", diagnosis.classification)),
            ))
        }
        TraceProbeMode::Http(proto) => probe(host, address, proto, "40000-40999", cancel)
            .await
            .map(|(code, elapsed_ms, reason)| (code, elapsed_ms, reason, None)),
    }
}

async fn wait_for_cancel(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

enum TraceProbeMode {
    Http(String),
    Quic(String),
    PaddedHead,
    Transfer(String, bool),
}

impl TraceProbeMode {
    fn for_protocol(protocol: &str, binary: &str) -> Self {
        if protocol == "quic" {
            Self::Quic(binary.to_owned())
        } else {
            Self::Http(protocol.to_owned())
        }
    }

    fn port(&self) -> u16 {
        match self {
            Self::Http(protocol) if protocol == "http" => 80,
            _ => 443,
        }
    }

    fn transport(&self) -> ProbeTransport {
        match self {
            Self::Quic(_) => ProbeTransport::Udp,
            _ => ProbeTransport::Tcp,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransferBaselineOptions {
    pub baseline_ok: bool,
    pub allow_upload_probe: bool,
}

pub async fn run(
    host: String,
    proto: &str,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::for_protocol(proto, &binary),
        None,
        args,
        binary,
        cancel,
        TraceRunContext::default(),
    )
    .await
}

pub async fn run_with_progress(
    host: String,
    proto: &str,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
    progress: TraceProgressCallback,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::for_protocol(proto, &binary),
        None,
        args,
        binary,
        cancel,
        TraceRunContext {
            known_baseline: None,
            progress: Some(progress),
            candidate_repeats: 1,
        },
    )
    .await
}

/// Runs only the current-route candidate probe using the result of a separate
/// direct baseline. This avoids repeating the bypass request for every
/// candidate/repeat in the strategy picker.
pub async fn run_candidate(
    host: String,
    proto: &str,
    args: Vec<String>,
    binary: String,
    baseline_ok: bool,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::for_protocol(proto, &binary),
        None,
        args,
        binary,
        cancel,
        TraceRunContext {
            known_baseline: Some(baseline_ok),
            progress: None,
            candidate_repeats: 1,
        },
    )
    .await
}

/// Reuse the isolated test process and its exact-host firewall rules for the
/// requested consecutive candidate confirmations. Stops early on failure.
pub async fn run_candidate_repeated(
    host: String,
    proto: &str,
    args: Vec<String>,
    binary: String,
    baseline_ok: bool,
    repeats: usize,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::for_protocol(proto, &binary),
        None,
        args,
        binary,
        cancel,
        TraceRunContext {
            known_baseline: Some(baseline_ok),
            progress: None,
            candidate_repeats: repeats.max(1),
        },
    )
    .await
}

/// Executes the normal isolated nfqws2 test flow against a pinned public
/// address and evaluates it with the 12-request padded HEAD probe.
pub async fn run_padded_target(
    host: String,
    address: Ipv4Addr,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::PaddedHead,
        Some(address),
        args,
        binary,
        cancel,
        TraceRunContext::default(),
    )
    .await
}

/// Same isolated padded-HEAD route probe, reusing a direct baseline measured
/// by the caller instead of issuing a duplicate network request.
pub async fn run_padded_target_with_baseline(
    host: String,
    address: Ipv4Addr,
    args: Vec<String>,
    binary: String,
    baseline_ok: bool,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::PaddedHead,
        Some(address),
        args,
        binary,
        cancel,
        TraceRunContext {
            known_baseline: Some(baseline_ok),
            progress: None,
            candidate_repeats: 1,
        },
    )
    .await
}

/// Uses the same bounded transfer-volume probe before and after the isolated
/// nfqws2 process is installed. The path is validated by the transfer probe.
pub async fn run_transfer_target(
    host: String,
    address: Ipv4Addr,
    path: String,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::Transfer(path, false),
        Some(address),
        args,
        binary,
        cancel,
        TraceRunContext::default(),
    )
    .await
}

/// Same isolated transfer-volume route probe, reusing a direct baseline
/// measured by the caller instead of issuing a duplicate network request.
pub async fn run_transfer_target_with_baseline(
    host: String,
    address: Ipv4Addr,
    path: String,
    args: Vec<String>,
    binary: String,
    baseline_ok: bool,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_transfer_target_with_baseline_and_upload_probe(
        host,
        address,
        path,
        args,
        binary,
        TransferBaselineOptions {
            baseline_ok,
            allow_upload_probe: false,
        },
        cancel,
    )
    .await
}

pub async fn run_transfer_target_with_baseline_and_upload_probe(
    host: String,
    address: Ipv4Addr,
    path: String,
    args: Vec<String>,
    binary: String,
    options: TransferBaselineOptions,
    cancel: Arc<AtomicBool>,
) -> TraceReport {
    run_inner(
        host,
        TraceProbeMode::Transfer(path, options.allow_upload_probe),
        Some(address),
        args,
        binary,
        cancel,
        TraceRunContext {
            known_baseline: Some(options.baseline_ok),
            progress: None,
            candidate_repeats: 1,
        },
    )
    .await
}

async fn run_inner(
    host: String,
    mode: TraceProbeMode,
    pinned_address: Option<Ipv4Addr>,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
    context: TraceRunContext,
) -> TraceReport {
    let TraceRunContext {
        known_baseline,
        progress,
        candidate_repeats,
    } = context;
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
        candidate_attempts: Vec::new(),
        summary: String::new(),
    };
    let port = mode.port();
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
    let resolved = if let Some(address) = pinned_address {
        public_v4(address).then_some(address)
    } else {
        match timeout(Duration::from_secs(5), lookup_host((host.as_str(), port))).await {
            Ok(Ok(values)) => values
                .filter_map(|socket| match socket.ip() {
                    std::net::IpAddr::V4(ip) if public_v4(ip) => Some(ip),
                    _ => None,
                })
                .next(),
            _ => None,
        }
    };
    let Some(address) = resolved else {
        report.without_nfqws2 =
            TracePhase::unavailable("DNS не вернул публичный IPv4; сетевые правила не менялись");
        report.with_nfqws2 = TracePhase::unavailable("Тестовая очередь не запускалась");
        report.summary = "Не найден публичный IPv4-адрес".into();
        return report;
    };
    report.address = Some(address.to_string());
    if let Some(progress) = &progress {
        progress("resolve", report.clone());
    }
    if cancel.load(Ordering::Acquire) {
        report.summary = "Трассировка остановлена до изменения firewall".into();
        return report;
    }
    let iptables_program = program_path("iptables");
    // A table dump is useful for policy marks, but must not gate the baseline:
    // some router iptables builds reject -S while permitting exact rule edits.
    let firewall_snapshot = command(&iptables_program, &["-w", "-t", "mangle", "-S"])
        .await
        .ok()
        .filter(|(status, _)| *status == 0)
        .map(|(_, output)| String::from_utf8_lossy(&output).into_owned())
        .unwrap_or_default();
    if cancel.load(Ordering::Acquire) {
        report.summary = "Трассировка остановлена до изменения firewall".into();
        return report;
    }
    let mut policy_mark = policy_route_mark(&firewall_snapshot);
    if policy_mark.is_none()
        && let Ok((0, output)) = command(
            &iptables_program,
            &["-w", "-t", "mangle", "-S", "nfqws_post"],
        )
        .await
    {
        policy_mark = policy_route_mark(&String::from_utf8_lossy(&output));
    }
    if let Some(baseline_ok) = known_baseline {
        report.without_nfqws2 = TracePhase {
            state: if baseline_ok { "complete" } else { "error" }.into(),
            ok: Some(baseline_ok),
            code: None,
            duration_ms: None,
            diagnostic_classification: None,
            reason: "Прямой базовый запрос выполнен отдельным этапом подбора".into(),
        };
    } else {
        let (base_code, base_ms, base_reason, base_classification) =
            match install_rules_for_transport(
                address,
                port,
                None,
                &token,
                policy_mark.as_deref(),
                mode.transport(),
            )
            .await
            {
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
                    let result = probe_mode(&host, address, &mode, &cancel).await;
                    let cleanup = remove_rules(&rules).await;
                    match (result, cleanup) {
                        (Ok(value), Ok(())) => value,
                        (Err(e), Ok(())) => (None, None, e, None),
                        (_, Err(e)) => (
                            None,
                            None,
                            format!("Не удалось очистить временные правила: {e}"),
                            None,
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
        report.without_nfqws2 = phase_from_probe_result(
            &mode,
            (base_code, base_ms, base_reason, base_classification),
        );
    }
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
    if let Some(progress) = &progress {
        progress("direct", report.clone());
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
    let queue_status = match tokio::fs::read_to_string("/proc/net/netfilter/nfnetlink_queue").await
    {
        Ok(status) => status,
        Err(error) => {
            report.with_nfqws2 = TracePhase::unavailable(format!(
                "Не удалось проверить слушателей NFQUEUE: {error}; тестовая очередь не запускалась"
            ));
            report.summary = "Есть базовый результат; очередь nfqws2 не проверена".into();
            return report;
        }
    };
    append_kernel_queue_listeners(&mut firewall_snapshot, &queue_status);
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
    let rules = match install_rules_for_transport(
        address,
        port,
        Some(queue),
        &token,
        policy_mark.as_deref(),
        mode.transport(),
    )
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
            let mut result = Err("Проверка не запускалась".to_string());
            for _ in 0..candidate_repeats.max(1) {
                if cancel.load(Ordering::Acquire) {
                    result = Err("Трассировка остановлена пользователем".into());
                    break;
                }
                result = match child.try_wait() {
                    Ok(None) => probe_mode(&host, address, &mode, &cancel).await,
                    Ok(Some(status)) => Err(format!(
                        "Тестовый nfqws2 завершился до запроса (код {})",
                        status.code().unwrap_or(-1)
                    )),
                    Err(error) => Err(format!(
                        "Не удалось проверить процесс тестового nfqws2: {error}"
                    )),
                };
                let phase = result
                    .as_ref()
                    .map(|result| phase_from_probe_result(&mode, result.clone()))
                    .unwrap_or_else(|reason| TracePhase::unavailable(reason.clone()));
                let ok = phase.ok == Some(true);
                report.candidate_attempts.push(phase);
                if !ok {
                    break;
                }
            }
            stop_child(&mut child).await;
            report.with_nfqws2 = result
                .map(|(code, ms, reason, classification)| {
                    phase_from_probe_result(
                        &mode,
                        (
                            code,
                            ms.or(Some(started.elapsed().as_millis() as u64)),
                            reason,
                            classification,
                        ),
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
    if let Some(progress) = &progress {
        progress("route", report.clone());
    }
    let _ = tokio::fs::remove_file(debug_path).await;
    if cancelled {
        report.summary = "Трассировка остановлена; временный процесс и правила очищены".into();
        return report;
    }
    report.summary =
        trace_summary_for_mode(&mode, report.without_nfqws2.ok, report.with_nfqws2.ok).into();
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
            diagnostic_classification: None,
            reason: reason.into(),
        },
        None => TracePhase {
            state: "error".into(),
            ok: Some(false),
            code: None,
            duration_ms,
            diagnostic_classification: None,
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
    fn quic_trace_mode_uses_udp_on_port_443() {
        let mode = TraceProbeMode::for_protocol("quic", "nfqws2");
        assert_eq!(mode.port(), 443);
        assert!(matches!(mode.transport(), ProbeTransport::Udp));
        assert!(matches!(mode, TraceProbeMode::Quic(_)));
    }

    #[test]
    fn quic_trace_results_never_masquerade_as_http_statuses() {
        let mode = TraceProbeMode::for_protocol("quic", "nfqws2");
        let phase = phase_from_probe_result(
            &mode,
            (Some(200), Some(25), "QUIC Initial ответил".into(), None),
        );
        assert_eq!(phase.state, "complete");
        assert_eq!(phase.ok, Some(true));
        assert_eq!(phase.code, None);
        assert_eq!(
            phase.diagnostic_classification.as_deref(),
            Some("quic_initial")
        );
        assert_eq!(
            trace_summary_for_mode(&mode, Some(true), Some(false)),
            "QUIC ответил напрямую, но не через nfqws2"
        );
    }

    #[tokio::test]
    async fn command_keeps_stderr_for_real_firewall_errors() {
        let (program, args): (&str, &[&str]) = if cfg!(windows) {
            (
                "cmd.exe",
                &["/C", "echo trace-fixture-error 1>&2 & exit /b 7"],
            )
        } else {
            ("sh", &["-c", "printf trace-fixture-error >&2; exit 7"])
        };
        let (code, output) = command(program, args).await.unwrap();
        assert_eq!(code, 7);
        assert!(String::from_utf8_lossy(&output).contains("trace-fixture-error"));
    }

    #[tokio::test]
    async fn large_firewall_output_is_drained_without_pipe_deadlock() {
        let (program, args): (&str, &[&str]) = if cfg!(windows) {
            (
                "cmd.exe",
                &["/C", "for /L %i in (1,1,10000) do @echo fixture-rule-line"],
            )
        } else {
            (
                "sh",
                &[
                    "-c",
                    "i=0; while [ $i -lt 10000 ]; do printf 'fixture-rule-line\\n'; i=$((i+1)); done",
                ],
            )
        };
        let (code, output) = command(program, args).await.unwrap();
        assert_eq!(code, 0);
        assert!(output.len() <= 65536);
        assert!(output.len() > 8193);
    }

    #[test]
    fn udp_bypass_preserves_exact_destination_and_owned_ports() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let rule = transport_rule(
            fw_args("-A", "N2UOfixture", target, "out", 443, None),
            ProbeTransport::Udp,
        );
        assert!(rule.windows(2).any(|p| p == ["-p", "udp"]));
        assert!(rule.windows(2).any(|p| p == ["-d", "93.184.216.34"]));
        assert!(rule.windows(2).any(|p| p == ["--sport", "40000:40999"]));
        assert!(rule.ends_with(&["-j".into(), "ACCEPT".into()]));
    }

    #[test]
    fn router_mark_selection_skips_partial_mask_bits_and_exact_values() {
        let snapshot = "-A nfqws_post -m mark ! --mark 0x40000000/0x40000000 -j RETURN\n99: from all fwmark 0x1 lookup 111";
        let lanes = allocate_router_lanes(3, 40_000, 40_191, snapshot).unwrap();
        assert!(lanes.iter().all(|lane| lane.mark_mask & 0x4000_0000 == 0));
        assert!(lanes.iter().all(|lane| lane.mark != 0x1));
        assert_ne!(lanes[0].mark, lanes[1].mark);
        assert_ne!(lanes[1].mark, lanes[2].mark);
    }

    #[test]
    fn batch_source_ports_avoid_the_kernel_ephemeral_range() {
        let (start, end) = select_batch_port_range(Some((49_000, 61_001)), 3).unwrap();
        assert_eq!(end - start + 1, 192);
        assert!(end < 49_000 || start > 61_001);
        assert!(select_batch_port_range(Some((1024, u16::MAX)), 3).is_none());
    }

    #[test]
    fn hooks_only_match_connections_originating_on_this_router() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let local = Ipv4Addr::new(192, 168, 1, 1);
        let out = local_flow_hook(
            hook_jump("-I", "POSTROUTING", target, "out", 443, "N2UOfixture"),
            "out",
            local,
        );
        let incoming = local_flow_hook(
            hook_jump("-I", "PREROUTING", target, "in", 443, "N2UIfixture"),
            "in",
            local,
        );
        assert!(out.windows(2).any(|p| p == ["-s", "192.168.1.1"]));
        assert!(
            incoming
                .windows(2)
                .any(|p| p == ["--ctorigsrc", "192.168.1.1"])
        );
        assert!(incoming.windows(2).any(|p| p == ["-m", "conntrack"]));
    }

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

        let monitor_rule =
            policy_mark_rule_for_ports("N2UMabc123", "0x123/0x0fffffff", "41000:41099").unwrap();
        assert!(
            monitor_rule
                .windows(2)
                .any(|pair| pair == ["--sport", "41000:41099"])
        );
        assert!(
            monitor_rule
                .windows(2)
                .any(|pair| pair == ["--set-xmark", "0x123/0x0fffffff"])
        );
        let cleanup_jump = chain_jump("-D", "OUTPUT", "N2UMabc123");
        assert!(
            cleanup_jump
                .windows(2)
                .any(|pair| pair == ["-j", "N2UMabc123"])
        );
    }

    #[test]
    fn policy_probe_cleanup_guard_can_remove_a_chain_created_before_a_command_timeout() {
        let route = policy_probe_route_guard("N2UMfixture");
        let cleanup = route
            .cleanup
            .as_ref()
            .expect("new probe route is armed for cleanup");
        assert_eq!(
            cleanup,
            &vec![vec!["-w", "-t", "mangle", "-X", "N2UMfixture"]]
        );
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
    fn batch_lane_marks_are_limited_to_target_router_flow_and_lane_ports() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let source = Ipv4Addr::new(192, 0, 2, 1);
        let lane =
            crate::probe_lanes::allocate_probe_lanes(2, 40_000, 40_127, 0x4000_0000).unwrap()[1];
        let outgoing = batch_mark_rule("N2UOabc123def0", target, source, "out", 443, lane);
        assert!(outgoing.windows(2).any(|pair| pair == ["-s", "192.0.2.1"]));
        assert!(
            outgoing
                .windows(2)
                .any(|pair| pair == ["-d", "93.184.216.34"])
        );
        assert!(
            outgoing
                .windows(2)
                .any(|pair| pair == ["--sport", "40064:40127"])
        );
        assert!(outgoing.windows(2).any(|pair| pair == ["--dport", "443"]));
        assert!(
            outgoing
                .windows(2)
                .any(|pair| pair == ["--set-xmark", "0x00000002/0x00000003"])
        );

        let incoming = batch_mark_rule("N2UIabc123def0", target, source, "in", 443, lane);
        assert!(
            incoming
                .windows(2)
                .any(|pair| pair == ["-s", "93.184.216.34"])
        );
        assert!(
            incoming
                .windows(2)
                .any(|pair| pair == ["--ctorigsrc", "192.0.2.1"])
        );
        assert!(
            incoming
                .windows(2)
                .any(|pair| pair == ["--dport", "40064:40127"])
        );
        assert!(incoming.windows(2).any(|pair| pair == ["--sport", "443"]));
    }

    #[test]
    fn batch_queue_rules_mark_each_lane_before_queueing_only_the_owned_port_span() {
        let target = Ipv4Addr::new(93, 184, 216, 34);
        let source = Ipv4Addr::new(192, 0, 2, 1);
        let lanes =
            crate::probe_lanes::allocate_probe_lanes(2, 40_000, 40_127, 0x4fff_ffff).unwrap();
        let outbound =
            batch_chain_rules("N2UOabc", target, source, "out", 443, 307, &lanes).unwrap();
        assert_eq!(outbound.len(), 4);
        assert!(
            outbound[0]
                .windows(2)
                .any(|pair| pair == ["--sport", "40000:40063"])
        );
        assert!(
            outbound[1]
                .windows(2)
                .any(|pair| pair == ["--sport", "40064:40127"])
        );
        assert!(
            outbound[2]
                .windows(2)
                .any(|pair| pair == ["--sport", "40000:40127"])
        );
        assert!(
            outbound[2]
                .windows(2)
                .any(|pair| pair == ["--queue-num", "307"])
        );
        assert!(
            outbound[2]
                .windows(2)
                .any(|pair| pair == ["--connbytes", "1:15"])
        );
        assert!(outbound[3].ends_with(&["-j".into(), "ACCEPT".into()]));
    }

    #[test]
    fn candidate_curl_is_pinned_to_its_lane_port_interval() {
        let args = http_probe_args(
            "example.com",
            Ipv4Addr::new(93, 184, 216, 34),
            "https",
            "40064-40127",
        );
        let local_port = args.iter().position(|arg| arg == "--local-port").unwrap();
        assert_eq!(args[local_port + 1], "40064-40127");
        assert!(args.contains(&"example.com:443:93.184.216.34".into()));
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
    fn trace_queue_can_be_selected_from_kernel_inventory_without_firewall_dump() {
        let mut snapshot = String::new();
        append_kernel_queue_listeners(&mut snapshot, "0000012d 00000000 00000001\n");
        assert_eq!(select_trace_queue(&[], &snapshot), Some(302));
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
