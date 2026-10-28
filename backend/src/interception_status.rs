use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptionStatus {
    pub missing_hooks: Vec<&'static str>,
}

pub const INTERCEPTION_MISSING_GRACE: Duration = Duration::from_secs(20);

fn iptables_status_arguments() -> [&'static str; 4] {
    ["-w", "-t", "mangle", "-S"]
}

pub fn visible_interception_status(
    status: Option<InterceptionStatus>,
    missing_since: Option<std::time::Instant>,
    now: std::time::Instant,
) -> (Option<InterceptionStatus>, Option<std::time::Instant>) {
    let Some(status) = status else {
        return (None, None);
    };
    if status.missing_hooks.is_empty() {
        return (Some(status), None);
    }
    let missing_since = missing_since.unwrap_or(now);
    if now.saturating_duration_since(missing_since) >= INTERCEPTION_MISSING_GRACE {
        (Some(status), Some(missing_since))
    } else {
        (None, Some(missing_since))
    }
}

pub fn parse_iptables_snapshot(snapshot: &str) -> Option<InterceptionStatus> {
    let mut post_chain_exists = false;
    let mut pre_chain_exists = false;
    let mut post_hook_connected = false;
    let mut pre_hook_connected = false;

    for line in snapshot.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["-N", "nfqws_post", ..] | ["-P", "nfqws_post", ..] => post_chain_exists = true,
            ["-N", "nfqws_pre", ..] | ["-P", "nfqws_pre", ..] => pre_chain_exists = true,
            ["-A", hook, rest @ ..] => {
                let target = rest.windows(2).find_map(|pair| {
                    matches!(pair[0], "-j" | "--jump" | "-g" | "--goto").then_some(pair[1])
                });
                if *hook == "POSTROUTING" && target == Some("nfqws_post") {
                    post_hook_connected = true;
                }
                if *hook == "PREROUTING" && target == Some("nfqws_pre") {
                    pre_hook_connected = true;
                }
            }
            _ => {}
        }
    }

    if !post_chain_exists || !pre_chain_exists {
        return None;
    }
    let mut missing_hooks = Vec::new();
    if !post_hook_connected {
        missing_hooks.push("POSTROUTING");
    }
    if !pre_hook_connected {
        missing_hooks.push("PREROUTING");
    }
    Some(InterceptionStatus { missing_hooks })
}

pub async fn read_iptables_status() -> Option<InterceptionStatus> {
    const TIMEOUT: Duration = Duration::from_secs(2);
    let program = ["/opt/sbin/iptables", "/usr/sbin/iptables", "/sbin/iptables"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .unwrap_or("iptables");
    let output = timeout(
        TIMEOUT,
        Command::new(program)
            .args(iptables_status_arguments())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_iptables_snapshot(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::{InterceptionStatus, parse_iptables_snapshot, visible_interception_status};
    use std::time::{Duration, Instant};

    const CONNECTED: &str =
        "-N nfqws_post\n-N nfqws_pre\n-A POSTROUTING -j nfqws_post\n-A PREROUTING -j nfqws_pre\n";

    #[test]
    fn reports_no_missing_hooks_when_both_nfqueue_chains_are_connected() {
        assert_eq!(
            parse_iptables_snapshot(CONNECTED),
            Some(InterceptionStatus {
                missing_hooks: vec![]
            })
        );
    }

    #[test]
    fn reads_the_mangle_table_in_one_iptables_call() {
        assert_eq!(
            super::iptables_status_arguments(),
            ["-w", "-t", "mangle", "-S"]
        );
    }

    #[test]
    fn finds_nfqueue_hooks_in_a_full_mangle_table_snapshot() {
        let snapshot = "-P PREROUTING ACCEPT\n-N nfqws_pre\n-N nfqws_post\n-N unrelated_chain\n-A PREROUTING -j nfqws_pre\n-A POSTROUTING -j nfqws_post\n-A unrelated_chain -j RETURN\n";
        assert_eq!(
            parse_iptables_snapshot(snapshot),
            Some(InterceptionStatus {
                missing_hooks: vec![]
            })
        );
    }

    #[test]
    fn reports_only_the_missing_hook_when_one_direction_is_unlinked() {
        assert_eq!(
            parse_iptables_snapshot("-N nfqws_post\n-N nfqws_pre\n-A PREROUTING -j nfqws_pre\n"),
            Some(InterceptionStatus {
                missing_hooks: vec!["POSTROUTING"]
            })
        );
    }

    #[test]
    fn reports_both_missing_hooks_when_neither_direction_is_linked() {
        assert_eq!(
            parse_iptables_snapshot("-N nfqws_post\n-N nfqws_pre\n"),
            Some(InterceptionStatus {
                missing_hooks: vec!["POSTROUTING", "PREROUTING"]
            })
        );
    }

    #[test]
    fn treats_unknown_or_incomplete_snapshots_as_unavailable() {
        assert_eq!(parse_iptables_snapshot(""), None);
        assert_eq!(
            parse_iptables_snapshot("iptables v1.8: permission denied"),
            None
        );
        assert_eq!(
            parse_iptables_snapshot("-N nfqws_post\n-A POSTROUTING -j nfqws_post\n"),
            None
        );
    }

    #[test]
    fn does_not_match_similarly_named_chains_or_hooks() {
        assert_eq!(
            parse_iptables_snapshot(
                "-N nfqws_post\n-N nfqws_pre\n-N nfqws_post_extra\n-A CUSTOM_POSTROUTING -j nfqws_post\n-A PREROUTING -j nfqws_pre_extra\n"
            ),
            Some(InterceptionStatus {
                missing_hooks: vec!["POSTROUTING", "PREROUTING"]
            })
        );
    }

    #[test]
    fn hides_a_newly_observed_missing_hook_during_the_router_rebuild_grace_period() {
        let now = Instant::now();
        let observed = parse_iptables_snapshot("-N nfqws_post\n-N nfqws_pre\n").unwrap();
        let (visible, missing_since) = visible_interception_status(Some(observed), None, now);
        assert_eq!(visible, None);
        assert_eq!(missing_since, Some(now));
    }

    #[test]
    fn reports_missing_hooks_only_after_twenty_seconds_of_known_disconnection() {
        let now = Instant::now();
        let observed = parse_iptables_snapshot("-N nfqws_post\n-N nfqws_pre\n").unwrap();
        let (visible, missing_since) = visible_interception_status(
            Some(observed.clone()),
            Some(now),
            now + Duration::from_secs(19),
        );
        assert_eq!(visible, None);
        assert_eq!(missing_since, Some(now));

        let (visible, missing_since) = visible_interception_status(
            Some(observed.clone()),
            Some(now),
            now + Duration::from_secs(20),
        );
        assert_eq!(visible, Some(observed));
        assert_eq!(missing_since, Some(now));
    }

    #[test]
    fn unknown_or_connected_observations_reset_the_disconnection_grace() {
        let now = Instant::now();
        let connected = parse_iptables_snapshot(
            "-N nfqws_post\n-N nfqws_pre\n-A POSTROUTING -j nfqws_post\n-A PREROUTING -j nfqws_pre\n",
        )
        .unwrap();
        assert_eq!(
            visible_interception_status(None, Some(now), now + Duration::from_secs(30)),
            (None, None)
        );
        assert_eq!(
            visible_interception_status(Some(connected.clone()), Some(now), now),
            (Some(connected), None)
        );
    }
}
