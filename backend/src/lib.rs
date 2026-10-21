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

pub fn process_matches(pid: u32, cmdline: &[u8], expected_binaries: &[&str]) -> bool {
    if pid == 0 || cmdline.is_empty() {
        return false;
    }
    let executable = cmdline.split(|byte| *byte == 0).next().unwrap_or_default();
    expected_binaries
        .iter()
        .any(|path| executable == path.as_bytes())
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
    match state {
        ServiceState::Running => {
            let pid_paths = ["/opt/var/run/nfqws2.pid", "/var/run/nfqws2.pid"];
            let expected = ["/opt/usr/bin/nfqws2", "/usr/bin/nfqws2"];
            let mut saw_pid_file = false;
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
                    return (
                        ServiceState::Running,
                        format!(
                            "Init-скрипт подтвердил службу; процесс nfqws2 активен (PID {pid})"
                        ),
                    );
                }
            }
            let detail = if saw_pid_file {
                "Init-скрипт сообщает running, но PID не соответствует живому nfqws2".to_string()
            } else {
                "Init-скрипт сообщает running, но PID-файл nfqws2 не найден".to_string()
            };
            (ServiceState::Error, detail)
        }
        ServiceState::Stopped => (
            ServiceState::Stopped,
            "Штатный init-скрипт сообщает, что служба остановлена".into(),
        ),
        ServiceState::Error => (ServiceState::Error, text.trim().to_string()),
        ServiceState::Unknown => (
            ServiceState::Unknown,
            if text.trim().is_empty() {
                "Не удалось распознать ответ init-скрипта".into()
            } else {
                text.trim().into()
            },
        ),
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
}
