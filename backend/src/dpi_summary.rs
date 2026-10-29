//! Evidence-linked, conservative explanations of an already parsed capture.
use crate::dpi_lab::{CapturedPacket, TcpFlow};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

const MAX_EVENTS: usize = 256;
const MAX_REFERENCES: usize = 32;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: &'static str,
    pub title: &'static str,
    pub state: &'static str,
    pub detail: String,
    pub packet_numbers: Vec<usize>,
    pub flow_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub kind: &'static str,
    pub title: &'static str,
    pub detail: String,
    pub packet_numbers: Vec<usize>,
    pub flow_index: Option<usize>,
    pub timestamp_micros: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub findings: Vec<Finding>,
    pub events: Vec<Event>,
    pub events_truncated: bool,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Direction {
    source: String,
    source_port: u16,
    destination: String,
    destination_port: u16,
}

impl Direction {
    fn from_packet(packet: &CapturedPacket) -> Option<Self> {
        Some(Self {
            source: packet.source.clone(),
            source_port: packet.source_port?,
            destination: packet.destination.clone(),
            destination_port: packet.destination_port?,
        })
    }
    fn reverse(&self) -> Self {
        Self {
            source: self.destination.clone(),
            source_port: self.destination_port,
            destination: self.source.clone(),
            destination_port: self.source_port,
        }
    }
    fn connection(&self) -> (String, u16, String, u16) {
        if (&self.source, self.source_port) <= (&self.destination, self.destination_port) {
            (
                self.source.clone(),
                self.source_port,
                self.destination.clone(),
                self.destination_port,
            )
        } else {
            (
                self.destination.clone(),
                self.destination_port,
                self.source.clone(),
                self.source_port,
            )
        }
    }
}

fn make_finding(
    id: &'static str,
    title: &'static str,
    state: &'static str,
    detail: impl Into<String>,
    packet_numbers: Vec<usize>,
    flow_indices: Vec<usize>,
) -> Finding {
    Finding {
        id,
        title,
        state,
        detail: detail.into(),
        packet_numbers,
        flow_indices,
    }
}

fn observed_flow_packets(flow: &TcpFlow) -> Vec<usize> {
    let end = flow
        .tls_client_hello
        .as_ref()
        .and_then(|hello| {
            hello
                .fields
                .iter()
                .filter_map(|field| field.offset.checked_add(field.length))
                .max()
        })
        .unwrap_or(0);
    flow.byte_sources
        .iter()
        .take(end)
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_REFERENCES)
        .collect()
}

/// Builds only claims justified by available bytes. This never infers the
/// presence of nfqws2 or a successful application request from PCAP alone.
pub fn summarize(packets: &[CapturedPacket], flows: &[TcpFlow], warnings: &[String]) -> Summary {
    let mut generations = HashMap::<(String, u16, String, u16), u32>::new();
    let mut initial_syn = HashMap::<(String, u16, String, u16), (Direction, u32)>::new();
    let mut syns = HashMap::<(Direction, u32), (usize, u32)>::new();
    let mut pending = HashMap::<(Direction, u32, u32), (usize, usize, u32)>::new();
    let mut payloads =
        HashMap::<(Direction, u32, u32, usize, [u8; 32]), (usize, usize, Vec<usize>, u64)>::new();
    let mut handshake = Vec::<usize>::new();
    let mut all_events = Vec::<Event>::new();
    let mut tcp_packets = Vec::<usize>::new();
    for packet in packets {
        let Some(direction) = Direction::from_packet(packet) else {
            continue;
        };
        let (Some(flags), Some(seq), Some(ack)) = (
            packet.tcp_flags,
            packet.tcp_sequence,
            packet.tcp_acknowledgement,
        ) else {
            continue;
        };
        if tcp_packets.len() < MAX_REFERENCES {
            tcp_packets.push(packet.number);
        }
        let connection = direction.connection();
        let syn = flags & 0x02 != 0;
        let acknowledgment = flags & 0x10 != 0;
        let reset = flags & 0x04 != 0;
        let finished = flags & 0x01 != 0;
        if syn
            && !acknowledgment
            && !reset
            && initial_syn.get(&connection) != Some(&(direction.clone(), seq))
        {
            let generation = generations.entry(connection.clone()).or_default();
            *generation = generation.saturating_add(1);
            initial_syn.insert(connection.clone(), (direction.clone(), seq));
        }
        if reset || finished {
            let generation = generations.entry(connection.clone()).or_default();
            *generation = generation.saturating_add(1);
            initial_syn.remove(&connection);
        }
        let generation = *generations.get(&connection).unwrap_or(&0);
        if syn && !acknowledgment && !reset {
            let key = (direction.clone(), seq.wrapping_add(1));
            if !syns
                .get(&key)
                .is_some_and(|(_, old_generation)| *old_generation == generation)
            {
                syns.insert(key, (packet.number, generation));
            }
            all_events.push(Event {
                kind: "syn",
                title: "Начало TCP-соединения",
                detail: "Одна сторона отправила запрос на установление соединения.".into(),
                packet_numbers: vec![packet.number],
                flow_index: None,
                timestamp_micros: packet.timestamp_micros,
            });
        } else if syn && acknowledgment && !reset {
            if let Some(&(syn_number, syn_generation)) = syns.get(&(direction.reverse(), ack))
                && syn_generation == generation
            {
                pending.insert(
                    (direction.reverse(), ack, seq.wrapping_add(1)),
                    (syn_number, packet.number, generation),
                );
            }
            all_events.push(Event { kind: "syn_ack", title: "Ответ на запрос соединения", detail: "В захвате есть пакет SYN/ACK. Для полного подтверждения нужны также согласованные SYN и ACK.".into(), packet_numbers: vec![packet.number], flow_index: None, timestamp_micros: packet.timestamp_micros });
        } else if acknowledgment
            && !reset
            && handshake.is_empty()
            && let Some(&(syn_number, syn_ack_number, pending_generation)) =
                pending.get(&(direction.clone(), seq, ack))
            && pending_generation == generation
        {
            handshake = vec![syn_number, syn_ack_number, packet.number];
            all_events.push(Event { kind: "tcp_handshake", title: "Три этапа TCP видны в захвате", detail: "Найдены согласованные SYN, SYN/ACK и ACK между одними адресами и портами. Это не подтверждает ответ сайта.".into(), packet_numbers: handshake.clone(), flow_index: None, timestamp_micros: packet.timestamp_micros });
        }
        if reset {
            all_events.push(Event {
                kind: "rst",
                title: "Отправлен сброс соединения",
                detail:
                    "Одна из сторон отправила TCP RST. Причина сброса по одному пакету неизвестна."
                        .into(),
                packet_numbers: vec![packet.number],
                flow_index: None,
                timestamp_micros: packet.timestamp_micros,
            });
        }
        if finished {
            all_events.push(Event {
                kind: "fin",
                title: "Начато закрытие соединения",
                detail: "Одна из сторон отправила TCP FIN.".into(),
                packet_numbers: vec![packet.number],
                flow_index: None,
                timestamp_micros: packet.timestamp_micros,
            });
        }
        if packet.payload_len > 0 {
            let start = packet
                .payload_offset
                .and_then(|offset| offset.checked_mul(2));
            let end = start.and_then(|start| {
                packet
                    .payload_len
                    .checked_mul(2)
                    .and_then(|len| start.checked_add(len))
            });
            if let Some(payload) = start
                .zip(end)
                .and_then(|(start, end)| packet.frame_hex.get(start..end))
            {
                let digest: [u8; 32] = Sha256::digest(payload.as_bytes()).into();
                let key = (direction, generation, seq, packet.payload_len, digest);
                if let Some((_, count, references, first_repeat_time)) = payloads.get_mut(&key) {
                    *count = count.saturating_add(1);
                    if references.len() < MAX_REFERENCES.saturating_sub(1) {
                        references.push(packet.number);
                    }
                    if *count == 2 {
                        *first_repeat_time = packet.timestamp_micros;
                    }
                } else {
                    payloads.insert(key, (packet.number, 1, Vec::new(), 0));
                }
            }
        }
    }

    for (first, repeats, mut references, first_repeat_time) in payloads.into_values() {
        if repeats <= 1 {
            continue;
        }
        references.insert(0, first);
        all_events.push(Event {
            kind: "duplicate",
            title: "Повторяются те же данные",
        detail: format!("Одинаковые данные присутствуют в {repeats} пакетах. Это может быть повторная передача TCP или дубликат захвата; причина неизвестна."),
            packet_numbers: references,
            flow_index: None,
            timestamp_micros: first_repeat_time,
        });
    }

    let tls_flows = flows
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.tls_client_hello.is_some())
        .collect::<Vec<_>>();
    let sni_flows = tls_flows
        .iter()
        .filter(|(_, flow)| {
            flow.tls_client_hello
                .as_ref()
                .and_then(|hello| hello.server_name.as_ref())
                .is_some()
        })
        .collect::<Vec<_>>();
    let names = sni_flows
        .iter()
        .filter_map(|(_, flow)| flow.tls_client_hello.as_ref()?.server_name.as_ref())
        .cloned()
        .collect::<BTreeSet<_>>();
    for (index, flow) in &tls_flows {
        let references = observed_flow_packets(flow);
        let time = references
            .first()
            .and_then(|number| packets.iter().find(|packet| packet.number == *number))
            .map_or(0, |packet| packet.timestamp_micros);
        all_events.push(Event { kind: "client_hello", title: "Начало согласования защищённого соединения", detail: "В восстановленных данных найден TLS ClientHello; завершение TLS по нему не подтверждается.".into(), packet_numbers: references, flow_index: Some(*index + 1), timestamp_micros: time });
    }
    all_events.sort_by_key(|event| event.packet_numbers.last().copied().unwrap_or(usize::MAX));
    let unusual_count = all_events
        .iter()
        .filter(|event| matches!(event.kind, "rst" | "duplicate"))
        .count();
    let unusual_refs = all_events
        .iter()
        .filter(|event| matches!(event.kind, "rst" | "duplicate"))
        .flat_map(|event| event.packet_numbers.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_REFERENCES)
        .collect();
    let events_truncated = all_events.len() > MAX_EVENTS;
    all_events.truncate(MAX_EVENTS);
    let gap_count: usize = flows.iter().map(|flow| flow.gap_bytes).sum();
    let overlap_count: usize = flows.iter().map(|flow| flow.overlap_bytes).sum();
    let conflict_count: usize = flows
        .iter()
        .map(|flow| flow.conflicting_overlap_bytes)
        .sum();
    let truncated = flows.iter().any(|flow| flow.stream_truncated);
    let ambiguous_connection = warnings
        .iter()
        .any(|warning| warning.starts_with("Повторный SYN:"));
    let flow_indices = (1..=flows.len()).collect::<Vec<_>>();
    let data_flow_count = flows
        .iter()
        .filter(|flow| flow.byte_sources.iter().any(Option::is_some))
        .count();
    let data_flow_indices = flows
        .iter()
        .enumerate()
        .filter_map(|(index, flow)| {
            flow.byte_sources
                .iter()
                .any(Option::is_some)
                .then_some(index + 1)
        })
        .collect::<Vec<_>>();
    let references = |items: &[(usize, &TcpFlow)]| {
        items
            .iter()
            .flat_map(|(_, flow)| observed_flow_packets(flow))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(MAX_REFERENCES)
            .collect::<Vec<_>>()
    };
    let tls_refs = references(&tls_flows);
    let sni_refs = references(
        &sni_flows
            .iter()
            .map(|(index, flow)| (*index, *flow))
            .collect::<Vec<_>>(),
    );
    let mut findings = Vec::new();
    findings.push(make_finding(
        "protocol",
        "Что обнаружено",
        if tcp_packets.is_empty() {
            "not_confirmed"
        } else {
            "confirmed"
        },
        if tcp_packets.is_empty() {
            format!(
                "В файле {} пакетов; TCP в поддерживаемом формате не найден.",
                packets.len()
            )
        } else {
            format!(
                "В файле {} пакетов; найдены TCP-пакеты и {} направлений с данными.",
                packets.len(),
                data_flow_count
            )
        },
        tcp_packets.clone(),
        flow_indices.clone(),
    ));
    findings.push(make_finding("tcp_connected", "Установление TCP", if !handshake.is_empty() { "confirmed" } else if tcp_packets.is_empty() { "not_checked" } else { "not_confirmed" }, if !handshake.is_empty() { "В захвате видны согласованные SYN, SYN/ACK и ACK. Доступность сайта по ним не определяется." } else { "Полная согласованная последовательность SYN, SYN/ACK и ACK не найдена. Захват может начинаться после подключения." }, handshake, Vec::new()));
    findings.push(make_finding("reassembly", "Восстановление данных TCP", if data_flow_count == 0 { "not_checked" } else if gap_count > 0 || conflict_count > 0 || truncated || ambiguous_connection { "not_confirmed" } else { "confirmed" }, if data_flow_count == 0 { "Данных TCP для восстановления нет.".into() } else { format!("Направлений с данными: {}; пропущено байтов внутри диапазонов: {}; повторных позиций: {}, из них с различающимися байтами: {}. Использованы первые увиденные байты{}; это модель, а не ответ сервера. Полнота исходного соединения неизвестна.{}", data_flow_count, gap_count, overlap_count, conflict_count, if truncated { "; часть данных ограничена размером" } else { "" }, if ambiguous_connection { " Повторные подключения могли быть объединены." } else { "" }) }, Vec::new(), data_flow_indices));
    findings.push(make_finding("client_hello", "Начало TLS", if !ambiguous_connection && tls_flows.iter().any(|(_, flow)| flow.conflicting_overlap_bytes == 0) { "confirmed" } else { "not_confirmed" }, if tls_flows.is_empty() { "TLS ClientHello не найден или не удалось полностью разобрать в доступных данных." } else if tls_flows.iter().any(|(_, flow)| flow.conflicting_overlap_bytes > 0) { "В модели первых увиденных байтов найден TLS ClientHello; пересечения могут влиять на реконструкцию. Завершение TLS не проверялось." } else { "В восстановленных данных найден TLS ClientHello. Завершение TLS отдельно не проверялось." }, tls_refs, tls_flows.iter().map(|(index, _)| *index + 1).collect()));
    findings.push(make_finding("sni", "Имя сайта в TLS", if !ambiguous_connection && sni_flows.iter().any(|(_, flow)| flow.conflicting_overlap_bytes == 0) { "confirmed" } else { "not_confirmed" }, if names.is_empty() { "Имя сайта не извлечено; это не доказывает отсутствие имени в исходном соединении.".into() } else if sni_flows.iter().any(|(_, flow)| flow.conflicting_overlap_bytes > 0) { format!("В модели первых увиденных байтов: {}. Пересечения могут влиять на реконструкцию; имя, полученное сервером, неизвестно.", names.into_iter().take(10).collect::<Vec<_>>().join(", ")) } else { format!("В ClientHello указано: {}.", names.into_iter().take(10).collect::<Vec<_>>().join(", ")) }, sni_refs, sni_flows.iter().map(|(index, _)| *index + 1).collect()));
    let unusual_flow_indices = flows
        .iter()
        .enumerate()
        .filter_map(|(index, flow)| {
            (flow.gap_bytes > 0 || flow.conflicting_overlap_bytes > 0).then_some(index + 1)
        })
        .collect();
    findings.push(make_finding("unusual", "На что обратить внимание", if packets.is_empty() { "not_checked" } else if unusual_count > 0 || gap_count > 0 || conflict_count > 0 { "confirmed" } else { "not_confirmed" }, if unusual_count > 0 || gap_count > 0 || conflict_count > 0 { format!("Сбросов или групп повторяющихся данных: {unusual_count}; пропущенных байтов: {gap_count}; конфликтующих байтов: {conflict_count}. Это признаки для изучения, а не доказательство блокировки или неисправности.") } else { "В доступной части захвата проверяемые признаки сброса, повтора, пропусков и конфликтов не обнаружены. Другие проблемы этим анализом не исключаются.".into() }, unusual_refs, unusual_flow_indices));
    for (id, title, detail) in [
        (
            "tls_negotiated",
            "TLS согласовано",
            "ServerHello и завершение согласования этим анализатором не проверяются.",
        ),
        (
            "http_response",
            "Ответ HTTP",
            "Содержимое HTTPS не расшифровывается; ответ HTTP не проверялся.",
        ),
        (
            "profile_matched",
            "Профиль nfqws2",
            "В PCAP нет достоверной привязки к выбранному профилю.",
        ),
        (
            "lua_observed",
            "Выполнение Lua",
            "События выполнения Lua в этот захват не включены.",
        ),
        (
            "improvement",
            "Улучшение стратегии",
            "Нет сопоставимого контрольного теста без стратегии.",
        ),
    ] {
        findings.push(make_finding(
            id,
            title,
            "not_checked",
            detail,
            Vec::new(),
            Vec::new(),
        ));
    }
    let mut limitations = vec![
        "Пакеты показывают только то, что попало в захват; он может быть неполным.".into(),
        "Участие nfqws2 и доступность сайта нельзя подтвердить только по наличию пакетов.".into(),
    ];
    if ambiguous_connection {
        limitations.push("Повторный SYN может означать повторную отправку или новое подключение с теми же адресами и портами. Реконструкция могла объединить разные соединения; выводы TLS и SNI требуют проверки.".into());
    }
    if !warnings.is_empty() {
        limitations.push(format!("Пакеты или форматы с ограничениями: {} сообщений. Подробности доступны в экспертном режиме.", warnings.len()));
    }
    if events_truncated {
        limitations.push(format!("Показаны первые {MAX_EVENTS} событий; полный список пакетов доступен в экспертном режиме."));
    }
    Summary {
        findings,
        events: all_events,
        events_truncated,
        limitations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(
        number: usize,
        reverse: bool,
        flags: u16,
        seq: u32,
        ack: u32,
        payload: &str,
    ) -> CapturedPacket {
        let (source, source_port, destination, destination_port) = if reverse {
            ("192.0.2.2", 443, "192.0.2.1", 1234)
        } else {
            ("192.0.2.1", 1234, "192.0.2.2", 443)
        };
        CapturedPacket {
            number,
            timestamp_micros: number as u64,
            source: source.into(),
            source_port: Some(source_port),
            destination: destination.into(),
            destination_port: Some(destination_port),
            protocol: "TCP".into(),
            tcp_sequence: Some(seq),
            tcp_acknowledgement: Some(ack),
            tcp_flags: Some(flags),
            ttl: Some(64),
            payload_offset: Some(0),
            payload_len: payload.len() / 2,
            frame_hex: payload.into(),
            evidence: "CAPTURE OBSERVED",
        }
    }

    #[test]
    fn matches_handshake_and_preserves_evidence_without_claiming_http_or_lua() {
        let packets = vec![
            packet(1, false, 2, 100, 0, ""),
            packet(2, true, 18, 500, 101, ""),
            packet(3, false, 16, 101, 501, ""),
        ];
        let result = summarize(&packets, &[], &[]);
        let tcp = result
            .findings
            .iter()
            .find(|item| item.id == "tcp_connected")
            .unwrap();
        assert_eq!(tcp.state, "confirmed");
        assert_eq!(tcp.packet_numbers, vec![1, 2, 3]);
        for id in [
            "tls_negotiated",
            "http_response",
            "profile_matched",
            "lua_observed",
            "improvement",
        ] {
            assert_eq!(
                result
                    .findings
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .state,
                "not_checked"
            );
        }
    }

    #[test]
    fn partial_capture_wrong_ack_and_different_endpoint_do_not_confirm_connection() {
        let mut other = packet(2, true, 18, 500, 101, "");
        other.source = "192.0.2.3".into();
        for packets in [
            vec![packet(1, false, 16, 101, 501, "")],
            vec![
                packet(1, false, 2, 100, 0, ""),
                packet(2, true, 18, 500, 999, ""),
                packet(3, false, 16, 101, 501, ""),
            ],
            vec![
                packet(1, false, 2, 100, 0, ""),
                other,
                packet(3, false, 16, 101, 501, ""),
            ],
        ] {
            let result = summarize(&packets, &[], &[]);
            assert_eq!(
                result
                    .findings
                    .iter()
                    .find(|item| item.id == "tcp_connected")
                    .unwrap()
                    .state,
                "not_confirmed"
            );
        }
    }

    #[test]
    fn reset_invalidates_pending_handshake_and_duplicates_are_only_possible_retransmissions() {
        let packets = vec![
            packet(1, false, 2, 100, 0, ""),
            packet(2, true, 18, 500, 101, ""),
            packet(3, true, 4, 501, 101, ""),
            packet(4, false, 16, 101, 501, ""),
            packet(5, false, 16, 101, 501, "aabb"),
            packet(6, false, 16, 101, 501, "aabb"),
            packet(7, false, 16, 101, 501, "aabb"),
            packet(8, false, 16, 101, 501, "ccdd"),
        ];
        let result = summarize(&packets, &[], &[]);
        assert_eq!(
            result
                .findings
                .iter()
                .find(|item| item.id == "tcp_connected")
                .unwrap()
                .state,
            "not_confirmed"
        );
        let duplicates = result
            .events
            .iter()
            .filter(|event| event.kind == "duplicate")
            .collect::<Vec<_>>();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].packet_numbers, vec![5, 6, 7]);
        assert!(duplicates[0].detail.contains("в 3 пакетах"));
        assert!(duplicates[0].detail.contains("дубликат"));
    }

    #[test]
    fn tls_evidence_uses_only_packets_covering_hello_and_gaps_remain_uncertain() {
        use crate::dpi_lab::{TlsClientHello, TlsField};
        let flow = TcpFlow {
            source: "192.0.2.1".into(),
            source_port: 1234,
            destination: "192.0.2.2".into(),
            destination_port: 443,
            packet_numbers: vec![1, 2, 3],
            reassembly_policy: "first-seen",
            sequence_start: 100,
            data_hex: "".into(),
            data_ascii: "".into(),
            byte_sources: vec![Some(1), Some(1), Some(2), Some(2), Some(3)],
            gap_bytes: 1,
            overlap_bytes: 2,
            conflicting_overlap_bytes: 2,
            stream_truncated: false,
            tls_client_hello: Some(TlsClientHello {
                server_name: Some("example.com".into()),
                fields: vec![TlsField {
                    name: "SNI".into(),
                    offset: 2,
                    length: 2,
                    value: "example.com".into(),
                    hex: "".into(),
                    evidence: "PROTOCOL DERIVED",
                }],
            }),
            tls_parse_error: None,
            evidence: vec!["SIMULATED"],
        };
        let packets = vec![
            packet(1, false, 16, 100, 0, ""),
            packet(2, false, 16, 102, 0, ""),
            packet(3, false, 16, 104, 0, ""),
        ];
        let result = summarize(&packets, &[flow], &[]);
        let sni = result
            .findings
            .iter()
            .find(|item| item.id == "sni")
            .unwrap();
        assert_eq!(sni.state, "not_confirmed");
        assert!(sni.detail.contains("Пересечения"));
        assert_eq!(sni.packet_numbers, vec![1, 2]);
        let reassembly = result
            .findings
            .iter()
            .find(|item| item.id == "reassembly")
            .unwrap();
        assert_eq!(reassembly.state, "not_confirmed");
        assert!(
            reassembly
                .detail
                .contains("пропущено байтов внутри диапазонов: 1")
        );
        assert_eq!(
            result
                .findings
                .iter()
                .find(|item| item.id == "tls_negotiated")
                .unwrap()
                .state,
            "not_checked"
        );
    }

    #[test]
    fn fin_invalidates_pending_handshake_and_new_connection_does_not_create_duplicate() {
        let packets = vec![
            packet(1, false, 2, 100, 0, ""),
            packet(2, true, 18, 500, 101, ""),
            packet(3, true, 17, 501, 101, ""),
            packet(4, false, 16, 101, 501, ""),
            packet(5, false, 16, 101, 501, "aabb"),
            packet(6, false, 2, 100, 0, ""),
            packet(7, false, 16, 101, 501, "aabb"),
        ];
        let result = summarize(&packets, &[], &[]);
        assert_eq!(
            result
                .findings
                .iter()
                .find(|item| item.id == "tcp_connected")
                .unwrap()
                .state,
            "not_confirmed"
        );
        assert!(!result.events.iter().any(|event| event.kind == "duplicate"));
    }

    #[test]
    fn repeated_syn_during_handshake_does_not_erase_matching_syn_ack() {
        let packets = vec![
            packet(1, false, 2, 100, 0, ""),
            packet(2, true, 18, 500, 101, ""),
            packet(3, false, 2, 100, 0, ""),
            packet(4, false, 16, 101, 501, ""),
        ];
        let result = summarize(&packets, &[], &[]);
        let tcp = result
            .findings
            .iter()
            .find(|item| item.id == "tcp_connected")
            .unwrap();
        assert_eq!(tcp.state, "confirmed");
        assert_eq!(tcp.packet_numbers, vec![1, 2, 4]);
    }

    #[test]
    fn unusual_finding_includes_late_resets_even_when_timeline_is_truncated() {
        let mut packets = (1..=300)
            .map(|number| packet(number, false, 2, number as u32, 0, ""))
            .collect::<Vec<_>>();
        packets.push(packet(301, true, 4, 0, 0, ""));
        let result = summarize(&packets, &[], &[]);
        assert!(result.events_truncated);
        let finding = result
            .findings
            .iter()
            .find(|item| item.id == "unusual")
            .unwrap();
        assert_eq!(finding.state, "confirmed");
        assert_eq!(finding.packet_numbers, vec![301]);
        assert!(finding.detail.contains("не доказательство блокировки"));
    }

    #[test]
    fn handshake_without_payload_does_not_claim_reconstructed_data() {
        let packets = vec![
            packet(1, false, 2, 100, 0, ""),
            packet(2, true, 18, 500, 101, ""),
            packet(3, false, 16, 101, 501, ""),
        ];
        let flow = TcpFlow {
            source: "192.0.2.1".into(),
            source_port: 1234,
            destination: "192.0.2.2".into(),
            destination_port: 443,
            packet_numbers: vec![1, 3],
            reassembly_policy: "first-seen",
            sequence_start: 101,
            data_hex: "".into(),
            data_ascii: "".into(),
            byte_sources: vec![],
            gap_bytes: 0,
            overlap_bytes: 0,
            conflicting_overlap_bytes: 0,
            stream_truncated: false,
            tls_client_hello: None,
            tls_parse_error: None,
            evidence: vec!["CAPTURE OBSERVED"],
        };
        let result = summarize(&packets, &[flow], &[]);
        let reassembly = result
            .findings
            .iter()
            .find(|item| item.id == "reassembly")
            .unwrap();
        assert_eq!(reassembly.state, "not_checked");
        assert!(
            reassembly
                .detail
                .contains("Данных TCP для восстановления нет")
        );
        assert!(
            result
                .findings
                .iter()
                .find(|item| item.id == "protocol")
                .unwrap()
                .detail
                .contains("0 направлений с данными")
        );
    }

    #[test]
    fn repeated_packet_events_are_grouped_with_bounded_evidence_references() {
        let packets = (1..=80)
            .map(|number| packet(number, false, 16, 100, 501, "aabb"))
            .collect::<Vec<_>>();
        let result = summarize(&packets, &[], &[]);
        let duplicate = result
            .events
            .iter()
            .find(|event| event.kind == "duplicate")
            .unwrap();
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| event.kind == "duplicate")
                .count(),
            1
        );
        assert!(duplicate.detail.contains("в 80 пакетах"));
        assert_eq!(duplicate.packet_numbers.len(), MAX_REFERENCES);
        assert_eq!(duplicate.packet_numbers.first(), Some(&1));
        assert_eq!(duplicate.packet_numbers.last(), Some(&32));
    }
}
