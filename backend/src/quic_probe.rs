//! Bounded QUIC Initial reachability, independent of curl HTTP/3 support.

use crate::site_diagnostics::{DiagnosticStep, capture, normalize_host};
use std::{
    net::{Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::{net::UdpSocket, time::timeout};

pub struct QuicPacket {
    bytes: Vec<u8>,
    source_cid: Vec<u8>,
}

fn parse_packet(output: &str) -> Result<QuicPacket, String> {
    let line = output
        .lines()
        .find_map(|line| line.strip_prefix("QUICHEX "))
        .filter(|hex| (2400..=3000).contains(&hex.len()))
        .ok_or("nfqws2 не сформировал QUIC Initial")?;
    let bytes = hex::decode(line).map_err(|_| "Некорректный QUICHEX")?;
    if bytes[0] & 0xc0 != 0xc0 || bytes[1..5] != [0, 0, 0, 1] || bytes[5] > 20 {
        return Err("Некорректный заголовок QUIC Initial".into());
    }
    let scid_offset = 6 + bytes[5] as usize;
    let scid_size = bytes[scid_offset] as usize;
    if !(1..=20).contains(&scid_size) {
        return Err("Некорректный SCID QUIC Initial".into());
    }
    let source_cid = bytes[scid_offset + 1..scid_offset + 1 + scid_size].to_vec();
    Ok(QuicPacket { bytes, source_cid })
}

fn matches_response(packet: &QuicPacket, response: &[u8]) -> bool {
    if response.len() < 7
        || response[0] & 0xc0 != 0xc0
        || response[1..5] != [0, 0, 0, 1]
        || ![0, 0x30].contains(&(response[0] & 0x30))
    {
        return false;
    }
    let size = response[5] as usize;
    size == packet.source_cid.len()
        && response.get(6..6 + size) == Some(packet.source_cid.as_slice())
}

pub async fn generate(binary: &str, host: &str) -> Result<QuicPacket, String> {
    if binary.is_empty() {
        return Err("nfqws2 не найден: QUIC Initial не сформирован".into());
    }
    if normalize_host(host).as_deref() != Ok(host) {
        return Err("Некорректное имя для QUIC".into());
    }
    let args = vec![
        "--intercept=0".into(),
        format!("--lua-init=SNI=\"{host}\""),
        format!("--lua-init={}", include_str!("quic_initial.lua")),
    ];
    let (code, output) = capture(binary, &args, 10, 16384).await?;
    if code != 0 {
        return Err(format!("nfqws2 не выполнил генератор QUIC (код {code})"));
    }
    parse_packet(&String::from_utf8_lossy(&output))
}

pub async fn probe(packet: &QuicPacket, ip: Ipv4Addr, direct: bool) -> DiagnosticStep {
    udp_probe(
        packet,
        SocketAddr::from((ip, 443)),
        direct,
        Duration::from_secs(2),
    )
    .await
}

async fn udp_probe(
    packet: &QuicPacket,
    address: SocketAddr,
    direct: bool,
    wait: Duration,
) -> DiagnosticStep {
    let mut socket = None;
    let ports: Box<dyn Iterator<Item = u16> + Send> = if direct {
        Box::new(40000..41000)
    } else {
        Box::new(std::iter::once(0))
    };
    for port in ports {
        if let Ok(candidate) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).await {
            socket = Some(candidate);
            break;
        }
    }
    let Some(socket) = socket else {
        return DiagnosticStep::new(
            "quic",
            "unavailable",
            "Не удалось занять проверочный UDP-порт",
        );
    };
    if let Err(error) = socket.connect(address).await {
        return DiagnosticStep::new("quic", "error", format!("UDP: {error}"));
    }
    let started = Instant::now();
    let mut response = [0; 4096];
    for _ in 0..2 {
        if let Err(error) = socket.send(&packet.bytes).await {
            return DiagnosticStep::new("quic", "error", format!("Отправка QUIC: {error}"));
        }
        let received = timeout(wait, async {
            loop {
                let size = socket.recv(&mut response).await?;
                if matches_response(packet, &response[..size]) {
                    return Ok::<_, std::io::Error>(size);
                }
            }
        })
        .await;
        match received {
            Ok(Ok(size)) => {
                let mut step = DiagnosticStep::new(
                    "quic",
                    "ok",
                    format!(
                        "Получен ответ QUIC v1 ({size} байт). Доступность QUIC подтверждена; полный HTTP/3-запрос не выполнялся."
                    ),
                );
                step.duration_ms = Some(started.elapsed().as_millis() as u64);
                return step;
            }
            Ok(Err(error)) => {
                return DiagnosticStep::new("quic", "error", format!("QUIC: {error}"));
            }
            Err(_) => {}
        }
    }
    let mut step = DiagnosticStep::new(
        "quic",
        "error",
        "Нет ответа на QUIC Initial после двух попыток; это не доказывает блокировку — сервер может не поддерживать QUIC.",
    );
    step.duration_ms = Some(started.elapsed().as_millis() as u64);
    step
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> QuicPacket {
        let mut bytes = vec![0xc3, 0, 0, 0, 1, 8];
        bytes.extend_from_slice(b"dest-cid");
        bytes.push(8);
        bytes.extend_from_slice(b"self-cid");
        bytes.resize(1200, 0);
        parse_packet(&format!("QUICHEX {}", hex::encode(bytes))).unwrap()
    }
    fn reply(cid: &[u8]) -> Vec<u8> {
        let mut result = vec![0xc0, 0, 0, 0, 1, cid.len() as u8];
        result.extend_from_slice(cid);
        result.extend_from_slice(&[0; 24]);
        result
    }
    #[test]
    fn accepts_only_bounded_initial_and_correlated_quic_replies() {
        let packet = packet();
        assert!(matches_response(&packet, &reply(b"self-cid")));
        assert!(!matches_response(&packet, &reply(b"othercid")));
        assert!(!matches_response(&packet, b"unrelated UDP response"));
        assert!(parse_packet("QUICHEX 1234").is_err());
        assert!(parse_packet(&format!("QUICHEX {}", "00".repeat(9000))).is_err());
    }
    #[tokio::test]
    async fn udp_probe_sends_initial_and_ignores_unrelated_cid() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = server.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let mut bytes = [0; 1500];
            let (size, peer) = server.recv_from(&mut bytes).await.unwrap();
            assert_eq!(size, 1200);
            server.send_to(&reply(b"othercid"), peer).await.unwrap();
            server.send_to(&reply(b"self-cid"), peer).await.unwrap();
        });
        let result = udp_probe(
            &packet(),
            address,
            false,
            std::time::Duration::from_millis(100),
        )
        .await;
        assert_eq!(result.status, "ok");
        assert!(result.detail.contains("QUIC"));
        assert_eq!(result.http_code, None);
        server_task.await.unwrap();
    }
    #[tokio::test]
    async fn udp_silence_is_failure_not_http3_success() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let result = udp_probe(
            &packet(),
            server.local_addr().unwrap(),
            false,
            std::time::Duration::from_millis(20),
        )
        .await;
        assert_eq!(result.status, "error");
        assert_eq!(result.http_code, None);
    }
}
