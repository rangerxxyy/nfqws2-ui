# DPI Lab — Vertical Slice Specification

## Goal

Add a usable, isolated DPI Lab route to nfqws2-ui that imports a bounded classic PCAP, reconstructs observed TCP byte streams using a documented deterministic policy, parses TLS ClientHello fields with byte offsets, visualizes packet order and byte provenance, and saves/reopens the analysis as a local experiment.

## Evidence and source findings

- NFQWS2's Lua init scripts are loaded once at startup; each configured `--lua-desync` function is called for packets that reach the configured strategy/profile. This is not a static guarantee that every configured step emits a packet.
- `zapret-antidpi.lua` documents standard args and helpers, including `rawsend`, `repeats`, payload filtering, `badsum`, and file/blob references. The actual behavior is function-specific; no Lua execution or packet prediction is part of this slice.
- `desync.c` has explicit TCP/UDP reassembly limits of 16,384 bytes in the current upstream source. The lab parser does not claim to duplicate nfqws2's C reassembly or the Lua library's exact runtime behavior.
- The current `nfqws2-keenetic` sample config loads `zapret-lib.lua`, `zapret-antidpi.lua`, and `zapret-auto.lua` through `--lua-init=@...`; this is a reference only, not a universal installed config.
- Existing frontend uses TypeScript, Vite, Inter and JetBrains Mono; backend uses Axum/Tokio/Serde. API routes are protected by the existing authentication middleware.

## Supported input and deterministic behavior

- Classic PCAP only, Ethernet or raw IPv4/IPv6 link types; malformed headers are rejected. Unsupported link types, non-TCP packets, fragments, and unsupported network/transport layouts are reported as skipped evidence.
- Maximum upload: 2 MiB; maximum captured packets: 5,000; maximum captured frame: 65,535 bytes; maximum persisted experiments: 20; each experiment is capped at 2 MiB.
- TCP reconstruction is bounded to 64 directional flows and 32 KiB of sequence space per flow; excess flows and truncated stream spans are explicitly reported.
- TCP ordering is reconstructed per 4-tuple and direction with a first-seen byte policy. Packet arrival order is retained separately. Gaps and overlaps are explicit. This policy is a deterministic analysis view, not a claim about a remote OS or provider DPI.
- TLS parsing is bounded to complete TLS handshake records present in the reassembled bytes. It reports ClientHello fields/extensions with absolute stream offsets and protocol-derived evidence. Truncation and unsupported structures remain explicit.
- Packet bytes and experiment data remain local. Experiment save is user initiated. No live capture, network probes, Lua execution, blob loading, QUIC parsing, provider-specific DPI claims, or working NFQWS2 configuration mutation are included.
- Every output identifies provenance as `CAPTURE OBSERVED`, `PROTOCOL DERIVED`, or `UNKNOWN`; reconstructions are labeled as analysis output, not emitted packets.

## API and persistence

- `POST /api/nfqws2/dpi-lab/analyze`: authenticated `application/octet-stream` body containing PCAP bytes and a safe display name in the query; enforce the 2 MiB body limit before parsing.
- `POST /api/nfqws2/dpi-lab/experiments`: authenticated save of an analyzed experiment; backend generates the ID, stores only bounded analyzed metadata/bytes, and prunes oldest entries above 20.
- `GET /api/nfqws2/dpi-lab/experiments`: authenticated list of experiment summaries.
- `GET /api/nfqws2/dpi-lab/experiments/{id}`: authenticated retrieval and deterministic redisplay of saved analysis.
- Storage is under the application's existing data directory in a dedicated `dpi-lab/experiments` directory; filenames are generated UUIDs, never user paths.

## User interface

- Add a first-class DPI Lab navigation route, matching the existing panel palette and typography.
- Provide PCAP file selection, analysis, save, and saved experiment loading.
- Show a packet timeline/table with observed timestamp, direction, endpoint tuple, sequence/acknowledgement, flags, payload size, and source packet number.
- Selecting a packet shows a bounded hex/ASCII byte inspector. Selecting a TCP flow shows the reconstructed start sequence, byte offsets, first-seen source packet for each range, gaps/overlap count, and parsed TLS fields when found.
- Keep errors, unsupported items, and evidence labels visible. Avoid fabricated charts and animations that imply unobserved sends.

## Tests and non-goals

- Unit tests cover both PCAP byte orders/resolutions, link-type handling, frame truncation, size/packet limits, IPv4 TCP parsing, gaps, retransmitted/overlapping bytes, malformed/truncated TLS records, extension bounds, SNI, and API auth/persistence/path safety.
- Frontend tests cover evidence labels, selection, errors, save/load API calls, and route navigation. Run full Rust and frontend suites and production build.
- This is a finished first slice, not the completion of the full master brief. Strategy Decoder/source mapping, packet simulation, QUIC, DPI models, Packet Diff, live capture, broad TCP policy comparison, and multi-experiment reports remain future work and must not appear as implemented controls.
