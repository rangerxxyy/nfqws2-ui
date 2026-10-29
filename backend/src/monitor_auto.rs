use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutoStrategySettings {
    pub enabled: bool,
    pub apply_automatically: bool,
    pub failures_before_pick: usize,
    pub cooldown_hours: u64,
}

impl Default for AutoStrategySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            apply_automatically: false,
            failures_before_pick: 3,
            cooldown_hours: 12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPickReadiness {
    Disabled,
    NotEnoughFailures,
    Cooldown,
    Ready,
}

pub fn failed_streak(statuses_newest_first: &[&str]) -> usize {
    let mut failures = 0;
    for status in statuses_newest_first {
        if *status == "error" {
            failures += 1;
        } else {
            break;
        }
    }
    failures
}

pub fn readiness(
    settings: &AutoStrategySettings,
    statuses_newest_first: &[&str],
    last_attempt_at: Option<u64>,
    now: u64,
) -> AutoPickReadiness {
    if !settings.enabled {
        return AutoPickReadiness::Disabled;
    }
    let failures = failed_streak(statuses_newest_first);
    if failures < settings.failures_before_pick {
        return AutoPickReadiness::NotEnoughFailures;
    }
    if last_attempt_at
        .is_some_and(|last| now.saturating_sub(last) < settings.cooldown_hours.saturating_mul(3600))
    {
        return AutoPickReadiness::Cooldown;
    }
    AutoPickReadiness::Ready
}

#[cfg(test)]
mod tests {
    use super::{AutoPickReadiness as R, AutoStrategySettings as S, failed_streak, readiness};

    #[test]
    fn starts_after_three_consecutive_errors_without_a_success_baseline() {
        assert_eq!(failed_streak(&["error", "ok"]), 1);
        assert_eq!(
            readiness(
                &S {
                    enabled: true,
                    ..S::default()
                },
                &["error", "error"],
                None,
                1000
            ),
            R::NotEnoughFailures
        );
        assert_eq!(
            readiness(
                &S {
                    enabled: true,
                    ..S::default()
                },
                &["error", "error", "error"],
                None,
                1000
            ),
            R::Ready
        );
    }

    #[test]
    fn warning_or_unavailable_breaks_the_consecutive_error_streak() {
        assert_eq!(
            readiness(
                &S {
                    enabled: true,
                    ..S::default()
                },
                &["error", "warning", "error", "error"],
                None,
                1000
            ),
            R::NotEnoughFailures
        );
        assert_eq!(
            readiness(
                &S {
                    enabled: true,
                    ..S::default()
                },
                &["error", "unavailable", "error", "error"],
                None,
                1000
            ),
            R::NotEnoughFailures
        );
    }

    #[test]
    fn other_down_sites_do_not_block_a_host_and_cooldown_is_respected() {
        let s = S {
            enabled: true,
            ..S::default()
        };
        assert_eq!(
            readiness(&s, &["error", "error", "error"], Some(999), 1000),
            R::Cooldown
        );
        assert_eq!(
            readiness(&s, &["error", "error", "error"], None, 1000),
            R::Ready,
            "the host's failure streak is evaluated independently"
        );
    }

    #[test]
    fn recognizes_dns_and_routing_failures_for_suppression() {
        assert!(super::is_infrastructure_failure(
            "DNS не вернул публичный IPv4-адрес"
        ));
        assert!(super::is_infrastructure_failure("Ошибка маршрута до узла"));
        assert!(!super::is_infrastructure_failure(
            "TCP 443: connection timeout"
        ));
    }

    #[test]
    fn settings_are_clamped_to_the_supported_safe_range() {
        let settings = super::normalize_settings(S {
            enabled: true,
            apply_automatically: false,
            failures_before_pick: 99,
            cooldown_hours: 1,
        });
        assert_eq!(settings.failures_before_pick, 4);
        assert_eq!(settings.cooldown_hours, 6);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutoStrategyStore {
    #[serde(default)]
    pub settings: AutoStrategySettings,
    #[serde(default)]
    pub active_operation_id: Option<String>,
    #[serde(default)]
    pub active_host: Option<String>,
    #[serde(default)]
    pub last_attempt_at: std::collections::HashMap<String, u64>,
    #[serde(default)]
    pub retry_after: std::collections::HashMap<String, u64>,
    #[serde(default)]
    pub offers: Vec<AutoStrategyOffer>,
    #[serde(default)]
    pub log: Vec<AutoStrategyEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoStrategyOffer {
    pub host: String,
    pub operation_id: String,
    pub candidate_index: usize,
    pub strategy_name: String,
    pub steps: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoStrategyEvent {
    pub host: String,
    pub kind: String,
    pub message: String,
    pub timestamp: u64,
}

pub fn normalize_settings(mut settings: AutoStrategySettings) -> AutoStrategySettings {
    settings.failures_before_pick = settings.failures_before_pick.clamp(3, 4);
    settings.cooldown_hours = settings.cooldown_hours.clamp(6, 24);
    settings
}

pub fn is_infrastructure_failure(summary: &str) -> bool {
    let normalized = summary.to_ascii_lowercase();
    normalized.contains("dns") || normalized.contains("podkop") || normalized.contains("маршрут")
}
