use serde::Serialize;
pub mod change_history;
pub mod community_strategies;
pub mod config_document;
pub mod config_format;
pub mod freeze_diagnostic;
pub mod freeze_profile_join;
pub mod interception_status;
pub mod managed_asn_lists;
pub mod probe_lanes;
pub mod quic_probe;
pub mod runtime_profiles;
pub mod site_diagnostics;
pub mod site_trace;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Running,
    Stopped,
    Error,
    Unknown,
}

pub fn classify_init_status(exit_code: i32, output: &str) -> ServiceState {
    let output = output.to_ascii_lowercase();
    if exit_code == 0 && output.contains("service nfqws2 is running") {
        ServiceState::Running
    } else if output.contains("service nfqws2 is stopped") {
        ServiceState::Stopped
    } else if exit_code != 0 && !output.trim().is_empty() {
        ServiceState::Error
    } else {
        ServiceState::Unknown
    }
}

pub fn valid_pid(input: &str) -> Option<u32> {
    let value = input.trim();
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse::<u32>().ok().filter(|pid| *pid > 0)
}

pub fn parse_meminfo_kb(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let (name, rest) = line.split_once(':')?;
        if name != "MemTotal" {
            return None;
        }
        let mut fields = rest.split_whitespace();
        let value = fields.next()?.parse::<u64>().ok()?;
        (fields.next() == Some("kB")).then_some(value)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

impl TryFrom<&str> for ServiceAction {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            "restart" => Ok(Self::Restart),
            _ => Err(()),
        }
    }
}

impl ServiceAction {
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

fn service_success_message(action: ServiceAction) -> &'static str {
    match action {
        ServiceAction::Start => "nfqws2 запущен",
        ServiceAction::Stop => "nfqws2 остановлен",
        ServiceAction::Restart => "nfqws2 перезапущен",
    }
}

pub fn known_init_scripts() -> [PathBuf; 2] {
    [
        PathBuf::from("/opt/etc/init.d/S51nfqws2"),
        PathBuf::from("/etc/init.d/nfqws2-keenetic"),
    ]
}

pub fn known_config_paths() -> [PathBuf; 2] {
    [
        PathBuf::from("/opt/etc/nfqws2/nfqws2.conf"),
        PathBuf::from("/etc/nfqws2/nfqws2.conf"),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfqws2Paths {
    pub root: PathBuf,
    pub binary: PathBuf,
    pub config: PathBuf,
    pub lists_dir: PathBuf,
    pub lua_dir: PathBuf,
    pub blobs_dir: PathBuf,
    pub init_script: PathBuf,
    pub pid_file: PathBuf,
    pub log_dir: PathBuf,
}

impl Nfqws2Paths {
    /// Discover a single installation from actual package files. Callers provide
    /// the supported roots (`/opt` and `/`) so tests can use isolated fixtures.
    /// Multiple matching roots are deliberately treated as ambiguous.
    pub fn discover(roots: &[PathBuf]) -> Option<Self> {
        let mut matches = roots.iter().filter_map(|root| {
            let binary = root.join("usr/bin/nfqws2");
            let config = root.join("etc/nfqws2/nfqws2.conf");
            (binary.is_file() && config.is_file()).then(|| (root.clone(), binary, config))
        });
        let (root, binary, config) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }

        let entware_init = root.join("etc/init.d/S51nfqws2");
        let openwrt_init = root.join("etc/init.d/nfqws2-keenetic");
        let present_init = [entware_init.clone(), openwrt_init.clone()]
            .into_iter()
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        let init_script = match present_init.as_slice() {
            [path] => path.clone(),
            [] if root == Path::new("/opt") => entware_init,
            [] => openwrt_init,
            _ => return None,
        };

        Some(Self {
            lists_dir: root.join("etc/nfqws2/lists"),
            lua_dir: root.join("etc/nfqws2/lua"),
            blobs_dir: config.parent()?.join("blobs"),
            pid_file: root.join("var/run/nfqws2.pid"),
            log_dir: root.join("var/log"),
            root,
            binary,
            config,
            init_script,
        })
    }
}

pub fn discover_system_paths() -> Option<Nfqws2Paths> {
    Nfqws2Paths::discover(&[PathBuf::from("/opt"), PathBuf::from("/")])
}

pub fn network_interface_names(sys_class_net: &std::path::Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(sys_class_net) else {
        return Vec::new();
    };
    let mut names = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.metadata().is_ok_and(|metadata| metadata.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}

pub fn default_network_interfaces(ipv4_routes: &str, ipv6_routes: &str) -> Vec<String> {
    let mut routes = Vec::<(usize, u64, usize, String)>::new();
    for (line_index, line) in ipv4_routes.lines().enumerate().skip(1) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 7 || fields[1] != "00000000" {
            continue;
        }
        let flags = u16::from_str_radix(fields[3], 16).unwrap_or_default();
        if flags & 1 == 0 {
            continue;
        }
        let metric = u64::from_str_radix(fields[6], 16).unwrap_or(u64::MAX);
        routes.push((0, metric, line_index, fields[0].to_owned()));
    }
    for (line_index, line) in ipv6_routes.lines().enumerate() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 10 || !fields[0].bytes().all(|byte| byte == b'0') || fields[1] != "00" {
            continue;
        }
        let flags = u32::from_str_radix(fields[8], 16).unwrap_or_default();
        if flags & 1 == 0 {
            continue;
        }
        let metric = u64::from_str_radix(fields[5], 16).unwrap_or(u64::MAX);
        routes.push((1, metric, line_index, fields[9].to_owned()));
    }
    routes.sort_by_key(|(family, metric, line_index, _)| (*family, *metric, *line_index));
    let mut interfaces = Vec::new();
    for (_, _, _, interface) in routes {
        if !interface.is_empty() && !interfaces.contains(&interface) {
            interfaces.push(interface);
        }
    }
    interfaces
}

pub fn process_matches(pid: u32, cmdline: &[u8], expected_binaries: &[&str]) -> bool {
    if pid == 0 || cmdline.is_empty() {
        return false;
    }
    let executable = cmdline.split(|byte| *byte == 0).next().unwrap_or_default();
    expected_binaries
        .iter()
        .any(|path| executable == path.as_bytes())
}

pub fn reconcile_init_state(
    state: ServiceState,
    verified_pid: Option<u32>,
    saw_pid_file: bool,
) -> (ServiceState, String) {
    match (state, verified_pid) {
        (ServiceState::Running, Some(pid)) => (
            ServiceState::Running,
            format!("Init-скрипт подтвердил службу; процесс nfqws2 активен (PID {pid})"),
        ),
        (ServiceState::Running, None) => (
            ServiceState::Error,
            if saw_pid_file {
                "Init-скрипт сообщает running, но PID не соответствует живому nfqws2".into()
            } else {
                "Init-скрипт сообщает running, но PID-файл nfqws2 не найден".into()
            },
        ),
        (ServiceState::Stopped, Some(pid)) => (
            ServiceState::Error,
            format!("Init-скрипт сообщает stopped, но процесс nfqws2 активен (PID {pid})"),
        ),
        (ServiceState::Stopped, None) => (
            ServiceState::Stopped,
            "Штатный init-скрипт сообщает, что служба остановлена".into(),
        ),
        (ServiceState::Error, _) => (
            ServiceState::Error,
            "Init-скрипт сообщил об ошибке при проверке службы".into(),
        ),
        (ServiceState::Unknown, _) => (
            ServiceState::Unknown,
            "Не удалось распознать ответ init-скрипта".into(),
        ),
    }
}

pub async fn service_status() -> (ServiceState, String) {
    let Some(paths) = discover_system_paths() else {
        return (
            ServiceState::Unknown,
            "Не удалось однозначно обнаружить установленный nfqws2 (бинарник и конфигурацию)"
                .into(),
        );
    };
    service_status_at(&paths).await
}

pub async fn service_status_at(paths: &Nfqws2Paths) -> (ServiceState, String) {
    let init = &paths.init_script;
    if !init.is_file() {
        return (
            ServiceState::Unknown,
            format!("Штатный init-скрипт nfqws2 не найден: {}", init.display()),
        );
    }
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        tokio::process::Command::new(init).arg("status").output(),
    )
    .await;
    let output = match output {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return (
                ServiceState::Unknown,
                format!("Не удалось выполнить {}: {error}", init.display()),
            );
        }
        Err(_) => {
            return (
                ServiceState::Unknown,
                "Проверка службы превысила таймаут".into(),
            );
        }
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let state = classify_init_status(output.status.code().unwrap_or(1), &text);
    if matches!(state, ServiceState::Running | ServiceState::Stopped) {
        let mut saw_pid_file = paths.pid_file.is_file();
        let mut verified_pid = None;
        if let Ok(contents) = tokio::fs::read_to_string(&paths.pid_file).await {
            saw_pid_file = true;
            if let Some(pid) = valid_pid(&contents) {
                let proc_cmdline = format!("/proc/{pid}/cmdline");
                if let Ok(cmdline) = tokio::fs::read(proc_cmdline).await
                    && process_matches(pid, &cmdline, &[paths.binary.to_string_lossy().as_ref()])
                {
                    verified_pid = Some(pid);
                }
            }
        }
        return reconcile_init_state(state, verified_pid, saw_pid_file);
    }
    match state {
        ServiceState::Error => (ServiceState::Error, text.trim().to_string()),
        ServiceState::Unknown => (
            ServiceState::Unknown,
            if text.trim().is_empty() {
                "Не удалось распознать ответ init-скрипта".into()
            } else {
                text.trim().into()
            },
        ),
        _ => unreachable!(),
    }
}

pub async fn service_action(action: ServiceAction) -> Result<String, String> {
    let paths = discover_system_paths()
        .ok_or_else(|| "Не удалось однозначно обнаружить установленный nfqws2".to_string())?;
    service_action_at(action, &paths).await
}

pub async fn service_action_at(
    action: ServiceAction,
    paths: &Nfqws2Paths,
) -> Result<String, String> {
    let init = &paths.init_script;
    if !init.is_file() {
        return Err(format!(
            "Штатный init-скрипт nfqws2 не найден: {}",
            init.display()
        ));
    }
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(25),
        tokio::process::Command::new(init)
            .arg(action.as_arg())
            .output(),
    )
    .await
    .map_err(|_| "Команда службы превысила таймаут".to_string())?
    .map_err(|error| format!("Не удалось выполнить команду службы: {error}"))?;
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    if !result.status.success() {
        return Err(if output.trim().is_empty() {
            format!(
                "Команда {} завершилась с кодом {:?}",
                action.as_arg(),
                result.status.code()
            )
        } else {
            output.trim().to_string()
        });
    }
    let (state, detail) = service_status_at(paths).await;
    let expected = match action {
        ServiceAction::Start | ServiceAction::Restart => ServiceState::Running,
        ServiceAction::Stop => ServiceState::Stopped,
    };
    if state != expected {
        return Err(format!(
            "Команда выполнена, но проверка службы не подтвердила ожидаемое состояние: {detail}"
        ));
    }
    Ok(service_success_message(action).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_actions_are_a_closed_enum() {
        assert_eq!(
            ServiceAction::try_from("restart").unwrap().as_arg(),
            "restart"
        );
        assert!(ServiceAction::try_from("restart;reboot").is_err());
        assert!(ServiceAction::try_from("reload").is_err());
    }

    #[test]
    fn successful_service_actions_return_only_a_short_status_message() {
        assert_eq!(
            service_success_message(ServiceAction::Start),
            "nfqws2 запущен"
        );
        assert_eq!(
            service_success_message(ServiceAction::Stop),
            "nfqws2 остановлен"
        );
        assert_eq!(
            service_success_message(ServiceAction::Restart),
            "nfqws2 перезапущен"
        );
    }

    #[test]
    fn process_validation_requires_exact_executable_argv0() {
        let paths = ["/opt/usr/bin/nfqws2", "/usr/bin/nfqws2"];
        assert!(process_matches(10, b"/opt/usr/bin/nfqws2\0--debug", &paths));
        assert!(!process_matches(10, b"/tmp/nfqws2-evil\0--debug", &paths));
        assert!(!process_matches(0, b"/opt/usr/bin/nfqws2\0", &paths));
    }

    #[test]
    fn network_interface_discovery_lists_only_real_sysfs_names_in_stable_order() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("wan0")).unwrap();
        std::fs::create_dir(root.path().join("br-lan")).unwrap();
        std::fs::write(root.path().join("not-a-directory"), "ignored").unwrap();

        assert_eq!(network_interface_names(root.path()), ["br-lan", "wan0"]);
        assert!(network_interface_names(&root.path().join("missing")).is_empty());
    }

    #[test]
    fn default_interface_discovery_orders_real_ipv4_and_ipv6_default_routes_by_metric() {
        let ipv4 = "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\nwan0 00000000 0100000A 0003 0 0 00000014 00000000 0 0 0\neth0 00000000 0100000A 0003 0 0 00000005 00000000 0 0 0\nlan0 0000010A 00000000 0001 0 0 00000000 00FFFFFF 0 0 0\neth1 00000000 0100000A 0000 0 0 00000001 00000000 0 0 0\n";
        let ipv6 = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 00000001 00000000 00000000 00000001 ppp0\n";
        assert_eq!(
            default_network_interfaces(ipv4, ipv6),
            ["eth0", "wan0", "ppp0"]
        );
        assert!(default_network_interfaces("", "").is_empty());
    }

    #[test]
    fn live_nfqws2_process_conflicting_with_stopped_init_is_error() {
        let (state, detail) = reconcile_init_state(ServiceState::Stopped, Some(123), true);
        assert_eq!(state, ServiceState::Error);
        assert!(detail.contains("PID 123"));
    }

    #[test]
    fn stopped_init_without_verified_process_is_stopped() {
        let (state, _) = reconcile_init_state(ServiceState::Stopped, None, false);
        assert_eq!(state, ServiceState::Stopped);
    }

    #[test]
    fn discovers_entware_paths_from_installed_binary_and_config() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path();
        std::fs::create_dir_all(root_path.join("usr/bin")).unwrap();
        std::fs::create_dir_all(root_path.join("etc/nfqws2")).unwrap();
        std::fs::create_dir_all(root_path.join("etc/init.d")).unwrap();
        std::fs::write(root_path.join("usr/bin/nfqws2"), "binary").unwrap();
        std::fs::write(root_path.join("etc/nfqws2/nfqws2.conf"), "MODE_LIST=''").unwrap();
        std::fs::write(root_path.join("etc/init.d/S51nfqws2"), "#!/bin/sh").unwrap();

        let paths = Nfqws2Paths::discover(&[root_path.to_path_buf()]).unwrap();
        assert_eq!(paths.root, root_path);
        assert_eq!(paths.binary, root_path.join("usr/bin/nfqws2"));
        assert_eq!(paths.config, root_path.join("etc/nfqws2/nfqws2.conf"));
        assert_eq!(paths.lists_dir, root_path.join("etc/nfqws2/lists"));
        assert_eq!(paths.init_script, root_path.join("etc/init.d/S51nfqws2"));
    }

    #[test]
    fn rejects_ambiguous_multiple_nfqws2_installations() {
        let roots = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
        for root in &roots {
            std::fs::create_dir_all(root.path().join("usr/bin")).unwrap();
            std::fs::create_dir_all(root.path().join("etc/nfqws2")).unwrap();
            std::fs::write(root.path().join("usr/bin/nfqws2"), "binary").unwrap();
            std::fs::write(root.path().join("etc/nfqws2/nfqws2.conf"), "").unwrap();
        }

        assert!(
            Nfqws2Paths::discover(
                &roots
                    .iter()
                    .map(|root| root.path().to_path_buf())
                    .collect::<Vec<_>>()
            )
            .is_none()
        );
    }

    #[test]
    fn does_not_infer_installation_from_config_without_binary() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("etc/nfqws2")).unwrap();
        std::fs::write(root.path().join("etc/nfqws2/nfqws2.conf"), "").unwrap();

        assert!(Nfqws2Paths::discover(&[root.path().to_path_buf()]).is_none());
    }

    #[test]
    fn openwrt_paths_use_root_without_entware_prefix() {
        let root = tempfile::tempdir().unwrap();
        let root_path: &Path = root.path();
        std::fs::create_dir_all(root_path.join("usr/bin")).unwrap();
        std::fs::create_dir_all(root_path.join("etc/nfqws2")).unwrap();
        std::fs::create_dir_all(root_path.join("etc/init.d")).unwrap();
        std::fs::write(root_path.join("usr/bin/nfqws2"), "binary").unwrap();
        std::fs::write(root_path.join("etc/nfqws2/nfqws2.conf"), "").unwrap();
        std::fs::write(root_path.join("etc/init.d/nfqws2-keenetic"), "#!/bin/sh").unwrap();

        let paths = Nfqws2Paths::discover(&[root_path.to_path_buf()]).unwrap();
        assert_eq!(paths.root, root_path);
        assert_eq!(
            paths.init_script,
            root_path.join("etc/init.d/nfqws2-keenetic")
        );
    }
}
pub mod strategy_picker;
pub mod transfer_diagnostic;
