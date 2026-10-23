# Autopick and transfer-diagnostics work log

## Time record

- Goal timer started: 2026-10-23 16:20:28 UTC. This is the measured goal creation time; the exact first keystroke/tool action was not separately recorded.
- Final checkpoint: 2026-10-23 17:26:02 UTC. The goal timer reported 65m 31s of active work. This is measured active work; idle wall time is excluded.
- Published `v0.2.9` at https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.2.9 from commit `7e19c4187059c747175c6c1e1092ffd09342eeb0`. Release workflow #49 completed successfully, including Linux ARM64 test/build/package steps.
- Router validation and the incomplete features listed below remain outstanding; the release is a limited milestone, not completion of the full autopick/16 KB scope.

## Audited sources

- `zemidala/nfqws2-ui` at `631878b8b9a6908d106904c188a83afb7aa43dbf`; key PHP functions, line ranges, safety findings, and remaining gaps are documented in `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`.
- `bol-van/zapret2` at `221d7dff24bfec0cd5ac0a5070b8acef2008af3d`; inspected `blockcheck2.sh`, standard modules, QNUM/chain ownership and signal/normal cleanup paths.
- `nfqws/nfqws2-keenetic` at `5d155f33ab00f3a4879c485c1094e8ec1e014586` and `nfqws/nfqws-keenetic-web` at `c225df33a8f3147f8f9a3ae9bbddccbc469a0baf`.
- Audited candidate arrays and behavior are attributed in `THIRD_PARTY_NOTICES.md` under the upstream MIT license.

## Implemented in the current working tree

- Rust bounded strategy planner: derives protocol-compatible current profiles, validates candidates against active Lua functions and blob inventory, constructs isolated single-profile argv, and sequentially confirms candidates with two real successful probes. Direct success short-circuits the search.
- Rust background API for quick/extended candidate checks with persisted progress/status, cancellation, restart-interruption handling, shared diagnostic lock, input limits, and bounded/pruned operation history.
- Rust 16–20 KB transfer classifier and HTTPS byte-volume comparison: two samples directly and through the current path against the same resolved public IPv4, bounded bytes/time, private scratch directory, and conservative “possible interruption” wording.
- TypeScript API/client and existing-style UI pages for strategy testing and transfer measurements. Results come only from backend responses. The main nfqws2 config/service is not changed by these operations.
- Bounded strategy history endpoint/UI (up to 12 terminal jobs), newest-first, with buttons to reopen persisted results; stored IDs survive reloads.
- Narrow-phone layout adjustment for the picker results panel; corresponding template test added.
- Corrected UI attribution labels; added third-party notice. Operation result directories require mode 0700 and check permission errors. Strategy-pick history pruning now retains active tasks and bounds completed records.
- Fixed the curl exit-18 path so partial-file results are distinguished from timeouts/inconclusive measurements; normalized runner failure state to the API's terminal `failed` value.
- Audited the real strategy application/refinement paths (`autoApplyList`, `autoApply`, `testRefine`) and recorded their safety checks and the missing Rust application flow in the source map.
- Found a false QUIC autopick path: the HTTP trace runner was being called with protocol `quic` and constructed an unsupported `quic://` curl URL. The API now rejects QUIC strategy picking as unimplemented, and the picker UI no longer advertises it; separate QUIC Initial diagnosis remains distinct. Regression tests cover this boundary.
- Preserved the earlier third-party notice for the adapted QUIC Initial Lua generator while adding the separate strategy-catalog attribution. Clarified the design record: transfer tests are controlled input/parser/classifier tests, not a local TLS-server fixture.
- Added explicit backend test coverage for transfer-task cancellation, cancellation between strategy repeats, and distinct curl error mappings (DNS/connect/timeout/partial/TLS/reset).

## Verification observed

- Latest: `cargo fmt --check`, `cargo test --all-targets --locked` (137 Rust tests), `cargo clippy --all-targets --locked -- -D warnings`, host `cargo build --release --locked`, `cargo check --all-targets --target aarch64-unknown-linux-musl --locked`, and `cargo audit --file backend/Cargo.lock` passed after the 0.2.9 version bump.
- Latest frontend after the 0.2.9 version bump: `pnpm test` passed (92 tests), `pnpm typecheck`, Vite production build and `pnpm audit --prod` passed.
- `pnpm audit --prod` passed with no known vulnerabilities. The remote `v0.2.8` tag is already present; it points to commit `12c0cde919de16e9fc1542f76166a4ee5dd2302a`.
- Verified commit `4764faa59bf47c30eea52b5f5f059d1836c87d5c` on pushed branch `codex/autopick-transfer-diagnostics`; the remote branch points to the same SHA. This is a feature branch, not a release tag.
- ARM64 `cargo check` passed, but target `cargo test` and release linking require the Linux `cross` toolchain; they are delegated to the GitHub release workflow.
- Local browser loads TypeScript app at `http://127.0.0.1:5173/`; the configured backend currently returns HTTP 500, so backend flows could not be exercised interactively on this host.

## Explicitly incomplete / device-only acceptance

- Only quick and a small extended standard set are exposed. Full generated catalog, refinement, custom strategy input, community catalog and network-aware ranking are not implemented.
- No configured resource-based parallelism; tests remain sequential until measured on a router.
- Advanced strategy history (filtering/export/application) is not implemented; a basic persisted last-12 history view is available. Site monitoring/automatic retry, strategy apply preview/confirmation, config write, rollback and automatic application are not implemented. Current checked candidates cannot be applied from this UI.
- The 16 KB feature is a bounded same-domain HTTPS body-volume classifier, not Zemidala’s padded-HEAD/ASN/name-template search. It uses IPv4, does not prove DPI, and has parser/classifier tests rather than a controlled local TLS server fixture.
- No live Keenetic/Entware test was run. NFQUEUE/firewall lifecycle, actual curl routing, router CPU/RAM, and interruption cleanup therefore remain unverified on target hardware.
- No visual phone-browser/device QA was available; responsive source/template coverage is not equivalent to a real mobile screenshot review.
- This is a limited v0.2.9 milestone release, not completion of the full acceptance scope. The explicit feature gaps above remain; no claim of full autopick, application/rollback or target-router validation is made.

## Current review focus

- Final release preparation is limited to documented 0.2.9 functionality; publish excludes the pre-existing user-owned untracked `work/` directory.
- Confirm tag, release asset checksum, and Linux workflow completion before reporting publication.
- Keep the router-only and unimplemented requirements listed above visible; do not claim a full Zemidala feature port or target-device validation.

## Continued work after v0.2.9 — 2026-10-23 17:40 UTC

- Re-read the audited upstream freeze implementation in `work/source-audit/nfqws2-ui-upstream/src/www/api.php`: `curlProbe`, `padProbe`, `FREEZE_REFS`, `FREEZE_IPS`, `FREEZE_NAMES`, `FREEZE_TEMPLATES`, `freezeJob` and `freezeCandidates`. It tests known large-file references and pinned IPs; detects a pre-response stall only when a separate TCP connect succeeds; limits SNI-template search by total/per-network deadlines; requires three successful probes; and reuses a successful result only for a matching ASN.
- The supplied screenshots show that the current single-host page reports a 0-byte timeout before the response body for `hetzner.com`. That is not evidence of a repeated 16–20 KB cutoff. The current classifier is appropriately cautious, but the panel still lacks upstream's reference-network scan and fake-SNI search; the presentation alone cannot fill that functional gap.
- Added a read-only strategy preview path. Rust now reads only a saved, completed operation with a twice-confirmed HTTP/HTTPS candidate, checks its steps against the live Lua/blob catalog, rejects ambiguous/dynamic `NFQWS_ARGS_CUSTOM`, builds a host-scoped profile, validates the draft, and returns the current revision and both config versions. It does not write the config or restart the service.
- TypeScript now requests that preview by persisted operation ID and candidate index. The panel offers the button only for backend-confirmed results and displays the backend-generated config diff. There is no apply control yet.
- Added tests first and observed expected RED failures for the missing Rust helpers and TypeScript API before implementation. Rust targeted tests passed; full `cargo test --all-targets --locked`, `cargo fmt --check`, and `cargo clippy --all-targets --locked -- -D warnings` passed. Frontend `pnpm test` passed (94), `pnpm typecheck` and `pnpm build` passed.
- Ruling: preview refuses to replace a same-host custom profile because current config has no trustworthy provenance marker showing that it was generated by this UI. Refusal preserves user-owned strategies; a future replace flow needs explicit persisted provenance and exact profile parsing.
- Still incomplete: actual upstream-style multi-network 16 KB scan, padded HEAD probe, ASN-confirmed name reuse, candidate refinement, exact selection between an existing compatible list/profile and a per-domain custom profile, dry-run/apply/restart/health-check/rollback, and hardware validation. No release was created from this work.
