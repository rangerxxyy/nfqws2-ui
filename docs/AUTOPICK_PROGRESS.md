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
- The 16 KB work in this branch now includes bounded HTTPS range probes, the upstream small-page upload probe, padded-HEAD checks for pinned addresses, ASN-verified reuse, and bounded fake-SNI candidate search. It uses IPv4 and does not prove DPI; controlled local TLS-server fixtures are still absent.
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

## Follow-up session — 2026-10-23 18:01 UTC (superseded by the 18:16 update below)

- Re-read the actual upstream `api.php` implementations of `curlProbe`, `padProbe`, `freezeJob`, and `freezeCandidates` alongside the four supplied screenshots. Confirmed the important distinction between (a) bounded 64 KiB range-download samples for real file endpoints and (b) 12 padded HEAD requests over one persistent connection for pinned addresses; reconnecting servers and timeouts before any response are inconclusive.
- Added a bounded padded-HEAD runner to the existing isolated trace lifecycle. It pins the resolved public IPv4, retains SNI, captures the 12 curl response records, and stops/reaps curl when cancellation is requested. Existing direct/through-test queue ownership and cleanup are reused. This runner is not yet wired into the network scan or SNI strategy search; the UI continues to disclose that.
- Added a regression test after confirming the red case: isolated picker argv now puts the nfqws2 executable back before profile arguments, because the trace cleanup strips argv[0] while the candidate builder returns profile args only. This prevents losing the first selected transport filter.
- Added responsive stacked cards for the 16–20 KB network table on narrow screens, stronger result text contrast, and an accessible animated progress indicator. `prefers-reduced-motion` disables the spinner. Tests assert the semantic mobile labels and busy state.
- `cargo test --locked --all-targets` passed (157 tests across suites) and `cargo clippy --all-targets --locked -- -D warnings` passed on Windows. An ARM64 Linux `cargo check` initially exposed a Unix-only name-shadowing error; that was fixed and `cargo check --all-targets --target aarch64-unknown-linux-musl --locked` now passes.
- Frontend: `pnpm test` passed (95), `pnpm typecheck` and `pnpm build` passed.
- Live router verification remains against the deployed 0.2.9 endpoint only; the new code has not been deployed. The earlier bounded `hetzner.com` request timed out before response body in both direct and current paths; queue 300 stayed the only listener and no temporary test chains remained. This does not verify fake-SNI search or prove a 16–20 KB cutoff.
- At that checkpoint, the network scan and candidate loop were still unwired. The following session records their implementation and current verification state.

## Follow-up session — 2026-10-23 18:16 UTC

- Rechecked the audited upstream PHP at `api.php` functions `curlProbe` (4139), `padProbe` (4186), and `freezeJob` (5226 onward). Added the small-response branch from `curlProbe`: for successful responses under 24,576 bytes, send a random 65,536-byte body and classify only a curl timeout after a completed TLS handshake as a possible upload stall. This has its own `possible_upload_stall` classification and UI wording; it is not mislabeled as a receive-side 16–20 KB cutoff. HTTP reachability is kept separate from complete byte-volume measurement so a short but clean response can still validate a candidate. Added parser/classifier tests for the handshake criterion and this distinction.
- Completed the bounded network scan wiring: large-file HTTPS references use pinned IPv4 range probes; fixed-address entries use 12 padded HEAD requests on one connection; affected targets run the upstream fake-SNI template/name order through the isolated active-config NFQUEUE trace runner. A candidate needs three successful tests, with one delayed retry after its first failure. Same-ASN result reuse requires live RIPEstat confirmation, not the catalog hint.
- Added per-candidate persisted results (SNI, exact Lua step, attempts, successes, mean duration, confirmation and bounded failure reason), matching the upstream `tried` history more closely. The results table shows the most recent checks alongside the selected result.
- Added explicit partial-completion reporting when the 15-minute overall or 150-second per-network search budget is reached. Completed measurements remain saved; the operation state is `partial`, and UI copy says time limits ended the work rather than claiming every network was exhausted. Cancellation still saves finished target results.
- No strategy is applied to config, no service restart is performed, and the primary nfqws2 config/process is not deliberately mutated by this scan. The existing isolated trace path installs temporary target/source-port-scoped rules and removes exact owned rules in its cleanup path. This has not yet been proven on the user's router with this changed binary.
- Per-network fake-SNI attempts are persisted with strategy, name, tries, successes, timing and bounded failure reason. Reaching the total/per-network deadline produces a partial state with preserved completed results, not a false complete state.
- Verification on this Windows host: `cargo fmt --check`, `cargo test --offline --all-targets --locked` (all suites passed, including 12 transfer and 10 freeze tests), `cargo clippy --offline --all-targets --locked -- -D warnings`, ARM64 `cargo check --offline --all-targets --target aarch64-unknown-linux-musl --locked`, and frontend `pnpm test` (95), `pnpm typecheck`, `pnpm build` passed.
- New ARM release linking is still unavailable locally: Windows has no Linux linker, WSL is present but has no installed distribution, and Docker/Podman/cross are absent. Prior ARM `cargo check` does not create a runnable router binary. Therefore no new code was uploaded or executed on the router and no release was created.
- A GitHub `workflow_dispatch` dry-run was started on pushed commit `e0e7f23` to obtain an ARM package; it builds that commit, so the subsequent small-page reachability fix in the current uncommitted follow-up still needs to be pushed and checked separately. The workflow dispatch does not create a release.
- The installed router remains on the already-published v0.2.9 code; its earlier read-only/direct probe is historical evidence only, not verification of this working tree. Before calling this equivalent to the upstream behavior or publishing it, build the ARM release binary, deploy the candidate build with rollback available, exercise a short-page and a pinned large-file scan, verify candidate retries/three-success confirmation, cancellation and cleanup, then inspect queue listeners and firewall rules.
