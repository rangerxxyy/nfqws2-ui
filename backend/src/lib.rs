use serde::Serialize;
use std::path::PathBuf;

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
    let Some(init) = known_init_scripts().into_iter().find(|path| path.is_file()) else {
        return (
            ServiceState::Unknown,
            "Штатный init-скрипт nfqws2 не найден".into(),
        );
    };
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        tokio::process::Command::new(&init).arg("status").output(),
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
        let pid_paths = ["/opt/var/run/nfqws2.pid", "/var/run/nfqws2.pid"];
        let expected = ["/opt/usr/bin/nfqws2", "/usr/bin/nfqws2"];
        let mut saw_pid_file = false;
        let mut verified_pid = None;
        for pid_path in pid_paths {
            let Ok(contents) = tokio::fs::read_to_string(pid_path).await else {
                continue;
            };
            saw_pid_file = true;
            let Some(pid) = valid_pid(&contents) else {
                continue;
            };
            let proc_cmdline = format!("/proc/{pid}/cmdline");
            if let Ok(cmdline) = tokio::fs::read(proc_cmdline).await
                && process_matches(pid, &cmdline, &expected)
            {
                verified_pid = Some(pid);
                break;
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
    let Some(init) = known_init_scripts().into_iter().find(|path| path.is_file()) else {
        return Err("Штатный init-скрипт nfqws2 не найден".into());
    };
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(25),
        tokio::process::Command::new(&init)
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
    let (state, detail) = service_status().await;
    let expected = match action {
        ServiceAction::Start | ServiceAction::Restart => ServiceState::Running,
        ServiceAction::Stop => ServiceState::Stopped,
    };
    if state != expected {
        return Err(format!(
            "Команда выполнена, но проверка службы не подтвердила ожидаемое состояние: {detail}"
        ));
    }
    Ok(if output.trim().is_empty() {
        detail
    } else {
        output.trim().to_string()
    })
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
}
