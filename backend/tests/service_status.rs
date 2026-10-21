use nfqws2_ui::{ServiceState, classify_init_status, parse_meminfo_kb, valid_pid};

#[test]
fn init_status_is_classified_conservatively() {
    assert_eq!(
        classify_init_status(0, "Service NFQWS2 is running"),
        ServiceState::Running
    );
    assert_eq!(
        classify_init_status(0, "Service NFQWS2 is stopped"),
        ServiceState::Stopped
    );
    assert_eq!(
        classify_init_status(1, "Service NFQWS2 is running"),
        ServiceState::Error
    );
    assert_eq!(classify_init_status(0, ""), ServiceState::Unknown);
}

#[test]
fn pid_file_accepts_only_a_positive_decimal_pid() {
    assert_eq!(valid_pid("24720\n"), Some(24720));
    for invalid in ["", "0", "-1", "1; reboot", "12 13", "abc"] {
        assert_eq!(valid_pid(invalid), None, "accepted invalid PID {invalid:?}");
    }
}

#[test]
fn memory_parser_uses_memtotal_and_keeps_kib_precision() {
    let input = "MemFree: 100 kB\nMemTotal:       524288 kB\nMemAvailable: 400000 kB\n";
    assert_eq!(parse_meminfo_kb(input), Some(524288));
    assert_eq!(parse_meminfo_kb("MemFree: 100 kB\n"), None);
}
