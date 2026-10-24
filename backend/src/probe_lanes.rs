use serde::{Deserialize, Serialize};

pub const MAX_BATCH_LANES: usize = 3;
pub const PORTS_PER_LANE: u16 = 64;
const MEMORY_RESERVE_KIB: u64 = 64 * 1024;
const MEMORY_PER_LANE_KIB: u64 = 32 * 1024;
const TOTAL_MEMORY_PER_LANE_KIB: u64 = 128 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeLane {
    pub index: usize,
    pub source_port_start: u16,
    pub source_port_end: u16,
    pub mark: u32,
    pub mark_mask: u32,
}

/// Conservative cap for one batch. Unknown or invalid metrics disable
/// concurrency; the caller should then run the same candidates serially.
pub fn safe_parallel_lane_count(
    total_kib: Option<u64>,
    available_kib: Option<u64>,
    load_1m: Option<f64>,
    logical_cpus: Option<usize>,
) -> usize {
    let (Some(total), Some(available), Some(load), Some(cpus)) =
        (total_kib, available_kib, load_1m, logical_cpus)
    else {
        return 1;
    };
    if total == 0 || cpus == 0 || !load.is_finite() || load < 0.0 {
        return 1;
    }

    let by_available = available.saturating_sub(MEMORY_RESERVE_KIB) / MEMORY_PER_LANE_KIB;
    let by_total = total / TOTAL_MEMORY_PER_LANE_KIB;
    let cpu_cap = if load >= cpus as f64 * 0.85 {
        1
    } else if load >= cpus as f64 * 0.5 {
        2
    } else {
        MAX_BATCH_LANES
    };
    (by_available.min(by_total).min(cpu_cap as u64) as usize).min(MAX_BATCH_LANES)
}

pub fn allocate_probe_lanes(
    count: usize,
    port_start: u16,
    port_end: u16,
    reserved_mark_mask: u32,
) -> Result<Vec<ProbeLane>, String> {
    if count == 0 || count > MAX_BATCH_LANES {
        return Err(format!(
            "Число дорожек должно быть от 1 до {MAX_BATCH_LANES}"
        ));
    }
    if port_end < port_start {
        return Err("Диапазон исходных портов задан неверно".into());
    }

    let needed_ports = u32::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(u32::from(PORTS_PER_LANE)))
        .ok_or_else(|| "Переполнение диапазона исходных портов".to_owned())?;
    if u32::from(port_end) - u32::from(port_start) + 1 < needed_ports {
        return Err("Недостаточно исходных портов для изолированных дорожек".into());
    }

    // Use the smallest number of currently unreserved fwmark bits that can encode
    // each lane with a distinct nonzero value. Keep this choice local to the test;
    // the caller must still check router policy rules before installing MARK rules.
    let bits_needed = usize::BITS as usize - count.leading_zeros() as usize;
    let free_bits = (0..u32::BITS)
        .filter(|bit| reserved_mark_mask & (1_u32 << bit) == 0)
        .take(bits_needed)
        .collect::<Vec<_>>();
    if free_bits.len() != bits_needed {
        return Err("Не найден безопасный диапазон fwmark для дорожек".into());
    }
    let mark_mask = free_bits
        .iter()
        .fold(0_u32, |mask, bit| mask | (1_u32 << bit));

    (0..count)
        .map(|index| {
            let encoded = u32::try_from(index + 1)
                .map_err(|_| "Идентификатор дорожки переполнен".to_owned())?;
            let mark = free_bits
                .iter()
                .enumerate()
                .filter(|(value_bit, _)| encoded & (1_u32 << value_bit) != 0)
                .fold(0_u32, |value, (_, mark_bit)| value | (1_u32 << mark_bit));
            let offset = u32::try_from(index)
                .map_err(|_| "Смещение порта переполнено".to_owned())?
                .checked_mul(u32::from(PORTS_PER_LANE))
                .ok_or_else(|| "Смещение порта переполнено".to_owned())?;
            let start = u32::from(port_start)
                .checked_add(offset)
                .ok_or_else(|| "Исходный порт переполнен".to_owned())?;
            let end = start + u32::from(PORTS_PER_LANE) - 1;
            Ok(ProbeLane {
                index,
                source_port_start: u16::try_from(start)
                    .map_err(|_| "Исходный порт вышел за допустимый диапазон".to_owned())?,
                source_port_end: u16::try_from(end)
                    .map_err(|_| "Исходный порт вышел за допустимый диапазон".to_owned())?,
                mark,
                mark_mask,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_BATCH_LANES, PORTS_PER_LANE, allocate_probe_lanes, safe_parallel_lane_count};

    #[test]
    fn sizes_batches_by_memory_and_current_load_with_a_three_lane_cap() {
        assert_eq!(
            safe_parallel_lane_count(Some(512 * 1024), Some(230 * 1024), Some(0.2), Some(2)),
            3
        );
        assert_eq!(
            safe_parallel_lane_count(Some(256 * 1024), Some(160 * 1024), Some(0.2), Some(2)),
            2
        );
        assert_eq!(
            safe_parallel_lane_count(Some(512 * 1024), Some(230 * 1024), Some(1.7), Some(2)),
            1
        );
        assert_eq!(
            safe_parallel_lane_count(Some(128 * 1024), Some(63 * 1024), Some(0.1), Some(2)),
            0
        );
    }

    #[test]
    fn unknown_or_invalid_resource_metrics_fall_back_to_sequential() {
        assert_eq!(
            safe_parallel_lane_count(None, Some(200_000), Some(0.1), Some(2)),
            1
        );
        assert_eq!(
            safe_parallel_lane_count(Some(512_000), Some(200_000), Some(f64::NAN), Some(2)),
            1
        );
    }

    #[test]
    fn allocates_disjoint_port_ranges_and_marks_outside_router_reserved_bits() {
        let lanes = allocate_probe_lanes(3, 40_000, 40_191, 0x4000_0000).unwrap();

        assert_eq!(lanes.len(), 3);
        assert_eq!(
            lanes
                .iter()
                .map(|lane| (lane.source_port_start, lane.source_port_end))
                .collect::<Vec<_>>(),
            vec![(40_000, 40_063), (40_064, 40_127), (40_128, 40_191)]
        );
        assert!(lanes.iter().all(|lane| {
            lane.source_port_end - lane.source_port_start + 1 == PORTS_PER_LANE
                && lane.mark != 0
                && lane.mark & !lane.mark_mask == 0
                && lane.mark_mask & 0x4000_0000 == 0
        }));
        assert_ne!(lanes[0].mark, lanes[1].mark);
        assert_ne!(lanes[1].mark, lanes[2].mark);
        assert_ne!(lanes[0].mark, lanes[2].mark);
    }

    #[test]
    fn skips_mark_bits_reserved_by_firewall_or_policy_routing() {
        let reserved = 0x4000_0000 | 0x0000_0003;
        let lanes = allocate_probe_lanes(3, 40_000, 40_191, reserved).unwrap();

        assert!(lanes.iter().all(|lane| lane.mark_mask & reserved == 0));
        assert!(lanes.iter().all(|lane| lane.mark & reserved == 0));
    }

    #[test]
    fn rejects_lane_counts_and_ranges_that_cannot_be_isolated() {
        assert!(allocate_probe_lanes(0, 40_000, 40_191, 0).is_err());
        assert!(allocate_probe_lanes(MAX_BATCH_LANES + 1, 40_000, 40_999, 0).is_err());
        assert!(allocate_probe_lanes(2, 40_000, 40_063, 0).is_err());
        assert!(allocate_probe_lanes(2, 40_100, 40_000, 0).is_err());
        assert!(allocate_probe_lanes(3, 40_000, 40_191, u32::MAX).is_err());
    }
}
