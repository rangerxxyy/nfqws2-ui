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

## Unified 16 KB screen — 2026-10-23 18:28 UTC

- Re-read upstream `curlProbe`, `padProbe`, `freezeJob`, and `freezeCandidates` against the supplied Zemidala screenshots. The screenshot's “Запустить пробу” is one long-running operation: optional user sites are prepended to the reference network list; it runs direct baselines, identifies affected networks, tries fake-SNI templates only for those networks, confirms candidates with three passes, and reuses a found variant only for a verified matching ASN.
- The existing Rust `freeze-scan` endpoint already implements that combined flow. The separate transfer-diagnostic card was an unrelated second launch path that made the UI contradict the reference workflow.
- Consolidated the 16 KB screen into one “Обрыв на 16 КБ” card with optional “Свои сайты”, one “Запустить пробу” button, one cancel button, persisted progress, and the single baseline/candidate results table. Removed the separate transfer-diagnostic controller and TypeScript API wrapper from this section; no Rust behavior changed in this UI-only consolidation.
- Added a template regression test asserting a single launch/cancel pair and absence of the second transfer card. Observed RED before editing (button label missing), then GREEN. Added a tested aggregate status banner matching the upstream “X networks affected / Y confirmed names” summary, with explicit partial/cancelled wording.
- Verification: `pnpm test` passed (97 tests), `pnpm typecheck`, `pnpm build`, and `git diff --check` passed. GitHub workflow run `40003029412` succeeded for ARM release cross-build/package on commit `9c1eb6b`; it did not publish a GitHub Release. This UI consolidation is not yet in that artifact.
- The current source still has not been run on the router; no claim of live device verification is made. Backend/system-side work remains gated on router deployment and validation, including safe real checks for the updated freeze scan.

## Upstream picker recheck and local continuation — 2026-10-23 18:38 UTC

- Re-opened the live upstream `src/www/api.php` after its `main` branch had moved since the pinned source audit. Confirmed the exact source sequence in `testJob()`: active config → selected standard set → inserted/custom strategies → generated `catalogCandidates()` → host/other successful history → ASN community entries → 16 KB fake-SNI discoveries. Every entry remains a candidate until a real isolated probe confirms it. Large catalogs are batch-tested only as a first pass and successful candidates are checked serially; `auto` mode stops at its first candidate passing every requested repeat. Added this recheck and current-versus-upstream gap list to `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`.
- User clarified the screenshot's multi-site selector is an example, not a strict requirement; no multi-site picker path was added.
- Auto-picker frontend now allows a bounded repeat count (2–5, default 3). Rust enforces this range, and a regression test verifies invalid counts return 400 without creating a job. Candidate confirmation now requires the configured number of actual isolated probe results.
- Added a custom-strategy mode: accepts one `--lua-desync=...` argument per line; Rust validates grammar, installed Lua functions and blob availability before queueing the single candidate. It does not interpret the input with a shell. Added validation and API/template tests.
- Previously displayed picker history now also feeds the next search: at most five confirmed candidates from the same exact host and protocol are considered, revalidated against the currently installed Lua/blob catalog and freshly tested. Other hosts, protocols, partial results and failed/unconfirmed runs are excluded. Added a fixture-backed persistence test.
- Verification: `pnpm test` (97), `pnpm typecheck`, `pnpm build` passed after the frontend changes. Final Rust `cargo test --offline --all-targets --locked` passed (57 library + 68 binary + 10 freeze + 3 service-status + 13 strategy-picker + 13 transfer tests); the targeted history test is included in the binary count. `cargo clippy --offline --all-targets --locked -- -D warnings`, `cargo fmt --check`, `cargo check --offline --all-targets --target aarch64-unknown-linux-musl --locked`, and root `git diff --check` passed after the final backend edits.
- These code changes remain uncommitted/unreleased. No current build was installed on the router. Full generated catalog, community catalog, auto-first-success, parameter refinement, parallel probing, and candidate application/rollback remain incomplete. The multi-site screenshot controls are intentionally omitted. Do not claim behavioral parity with the complete upstream picker.

## Extended blockcheck2 catalog port — 2026-10-23 18:43 UTC

- Re-read the live upstream `catalogCandidates()` implementation at lines 3890–3978 and ported its extended-mode HTTP/TLS candidate generation: split/disorder positions, fake variants and fooling fields, hostfakesplit variants, fake-plus-split candidates, and the extended seqovl cases. Exact-step deduplication and runtime Lua/blob compatibility filtering happen before these candidates enter the test plan.
- Kept full catalog (`ip_ttl`, fakedsplit/fakeddisorder and repeated tcpseg combinations; ~3,000 TLS candidates) disabled until batch testing, low-resource concurrency limits, and bounded result persistence are ported. The currently supported extended search is sequential and cancellable. UI now labels this mode as the blockcheck2 catalog.
- Expanded the persisted operation candidate limit to 512 so the extended plan plus active-profile/history candidates can fit. The operation JSON cap remains 512 KiB; runtime result payloads are bounded summaries without raw logs. Confirmed tests include the generated plan size and deduplication; full validation still pending after edits.

## Freeze-scan candidate reuse in the picker — 2026-10-23 18:50 UTC

- Rechecked upstream `freezeCandidates()` at `api.php` 5393 onward: confirmed 16 KB scan discoveries are inserted into normal strategy picking, with exact-step deduplication and a fresh test against the requested site. They are hints from another network, not assumed successes.
- Implemented this missing bridge: the picker reads only the newest completed or partial persisted freeze-scan operation, keeps strategies that were confirmed in three attempts on a network marked affected, revalidates each candidate against the currently installed Lua/blob inventory, and puts it before the large generated catalog. The candidate still goes through the existing live isolated probe against the user-entered domain.
- Added tests for rejecting non-affected/unconfirmed rows, selecting the newest eligible persisted scan, deduplication, installed-function validation, and ordering before the blockcheck2 catalog. The test was run and passed.
- Fixed a stale candidate-catalog test fixture: it asserted that `multidisorder` was present despite not listing that function as installed. The corrected fixture now tests its intended extended-catalog behavior without weakening runtime compatibility filtering.
- Verification after these edits: `cargo fmt --check`, `cargo test --offline --all-targets --locked` (169 tests), `cargo clippy --offline --all-targets --locked -- -D warnings`, host `cargo build --release --offline --locked`, and ARM64 `cargo check --offline --all-targets --target aarch64-unknown-linux-musl --locked` passed. Frontend `pnpm test` passed (97 tests), `pnpm typecheck`, `pnpm build`, and root `git diff --check` passed.
- No target-router deployment or release has been made from the current working tree. Community-catalog import, other-site history, full catalog, bounded batch testing, refinement and automatic apply/rollback remain outstanding; a current build is not yet behaviorally equivalent to the full upstream picker.

## Optional first-confirmed automatic stop — 2026-10-23 18:54 UTC

- Re-read the upstream `auto` path in `testJob()`: when requested, it stops after a strategy passes all repeats rather than continuing through the catalog. Added an explicit checkbox in the existing picker, a typed request flag, and a Rust runner branch. The default remains off so a normal comparison still evaluates the candidate list.
- Added a runner regression test proving only the first confirmed candidate is attempted, plus API payload and UI template assertions. The backend still performs the chosen number of actual per-host probes before stopping.
- Targeted Rust test passed; full Rust tests passed (170 across suites: 57 library, 70 binary, 11 freeze, 3 service, 16 picker, 13 transfer). Frontend `pnpm test` (97), `pnpm typecheck`, and `pnpm build` passed. `cargo fmt --check`, Clippy, host release build, ARM64 `cargo check`, and `git diff --check` passed after rustfmt. The ARM64 check is not a linked router executable.

## Provider-ASN community candidates and 16 KB output review — 2026-10-23 19:03 UTC

- Re-read upstream `communityLoad()` / `communityCandidates()` at `src/www/api.php` 5532–5666. Confirmed its per-provider `data/AS<asn>.json` source, 24-hour successful/404 caching, narrow step validation, 60-day freshness window, same-host/same-target-ASN ranking, mostly-failed filter and maximum of eight candidates.
- Added a provider catalog adapter. It detects the router's public egress IP and ASN through RIPEstat, resolves the requested site's ASN as a ranking hint, fetches only that provider's file with bounded curl timeouts/size, caches positive and 404 responses for 24 hours, filters by ASN/protocol/date/failure count/Lua/blob inventory, and inserts at most eight candidates before the generated catalog. Every candidate still runs through the real isolated picker probe; retrieval failure is shown in operation status and falls back to normal candidate sources. Remote JSON is never passed through a shell or applied to the live configuration.
- Wired the community option to the picker (checked by default) and typed API request; a community-only plan can run if local catalog candidates are unavailable. Added parser/ranking/cap/unsafe-input tests.
- Rechecked the 16 KB response model/rendering: every row includes network label, hostname/path, resolved IP (or “address not found”) and verified/changed/unavailable ASN. Added serialization coverage. The job remains sequential: 20 reference targets plus up to 20 supplied sites, direct baseline, isolated current-route test, then fake-SNI checks only for affected networks with three consecutive confirmations and ASN-gated reuse.
- Improved 16 KB result-row readability/entry animation and picker control focus/selection feedback. Reduced-motion preference disables the new animations.
- Verification: `cargo test --offline --locked --all-targets` passed (174 tests), `cargo clippy --offline --all-targets --locked -- -D warnings` passed, `cargo fmt --check`, ARM64 musl `cargo check`, host `cargo build --release --offline --locked`, frontend `pnpm test` (97), `pnpm typecheck`, `pnpm build`, and root `git diff --check` passed. The local browser preview served the UI but returned HTTP 500 from the backend status endpoint, so visual verification of live result states is still pending.
- No router execution was performed. This environment has no `aarch64-linux-musl-gcc` and the changed code has no linked router executable; the router address is also not present in this task context. Therefore real Keenetic behavior remains unverified. No service, configuration or firewall changes were made, and this work is not released.

## Other-host successful strategy history — 2026-10-23 19:07 UTC

- Re-read upstream `PICKS_OTHER`, `picksAdd()` and `picksCandidates()` in `src/www/api.php` 5421–5493: for the same protocol, up to five variants which passed every repeat on another host are offered after exact-host history. They are still retested on the current host.
- Added this source to Rust history selection, retaining exact-host candidates first, deduplicating exact steps, limiting other-host candidates to five, and exposing a UI checkbox. Protocol mismatch, unconfirmed candidates, and jobs without confirmed results are excluded. The checkbox state is carried in the typed POST request.
- Queried the live upstream strategy `index.json` and provider files read-only. Confirmed current schema and representative `AS8402` TLS entry; added a parser regression based on that entry so real catalog token shape stays accepted only when the installed Lua function supports it.
- Targeted Rust history test passed; full Rust suite passed (175 tests). Frontend suite passed (97 tests), TypeScript check and production build passed. Remaining this continuation: rerun Clippy/format/diff after the last docs/UI edits, then continue evaluating full-catalog batching and device constraints. Router execution and release remain outstanding.

## Candidate source parity and cache follow-up — 2026-10-23 19:13 UTC

- Corrected candidate insertion order to match `testJob()`: profile/standard → successful history → community → confirmed 16 KB discoveries → generated catalog. Freeze discoveries now load in the background; a previously confirmed strategy already present in the generated catalog is promoted before the catalog instead of silently staying at its old position. Added a regression that first failed on that duplicate case, then passed after the fix.
- Community records without a usable `last` date are retained as stale, low-priority candidates like upstream. Candidate cards include provider/site/network match, report/failure counts and stale status. Added a missing-date fixture test.
- Cached the router provider ASN for 24 hours without persisting the router's public IP; added a no-network cache test. Catalog data and ASN are separate cache entries.
- Latest Rust suite passed (177 tests), including the real-shape ASN 8402 catalog fixture and the candidate-promotion regression. ARM64 musl type-check passed. Clippy caught one style issue in the new cache writer and it was corrected; rerun Clippy/release build and final diff checks are in progress. Latest frontend checks remain green (97 tests/typecheck/build).
- GitHub read confirmed `v0.2.9` is already the latest published release, so a duplicate release was not created. Current changes are still uncommitted and not published; full catalog batching/refinement/application rollback and real Keenetic verification remain incomplete.

## Full blockcheck2 catalog mode — 2026-10-23 19:19 UTC

- User clarified the requested picker choice is the full mode. Re-read `catalogCandidates('full')` and `parTry()` in upstream `src/www/api.php` (around lines 3934–4028 and 4287–4350).
- Ported full-catalog axes into the existing Rust candidate generator: per-packet TTL values, all fake/split combinations, hostfakesplit modes, fakedsplit/fakeddisorder, tcpseg overlap and repeated segments. Existing strict Lua function/blob checks run before candidates enter the plan. Full plans are capped at 4,096.
- Enabled explicit `full` through the existing API, selector and panel event handling. The UI discloses sequential execution and the potentially longer run. Negative rows are compacted (upstream also omits failed catalog rows); only up to 256 partial/success candidates are persisted.
- Added Rust regression coverage for full-only candidate axes, uniqueness and bounded plan generation, plus full-mode request/UI integration assertions.
- Read-only router check over the user-authorized SSH connection: found `/opt/usr/bin/nfqws2`, service queue 300, 497,680 kB RAM total and 203,872 kB available. No router process, firewall, config, or service was changed. Did not run a candidate test because the current runner lacks upstream's per-candidate port/mark lanes needed for safe batching.
- This port is not yet full operational parity: full-catalog execution remains sequential, and `testRefine()` plus exact `parTry()` lanes have not been ported. A router-side behavioral check remains outstanding.

## Full mode verification — 2026-10-23 19:22 UTC

- Full HTTPS fixture creates 3,114 unique plan candidates (including baseline candidates), within the 4,096 plan bound. Tests cover TTL 12, disorder-after hostfakesplit, fakedsplit/fakeddisorder, tcpseg repeats 260, uniqueness and capped negative-result persistence.
- Corrected the prior endpoint test to assert that `full` passes mode validation and is then stopped because the test environment has no verified live nfqws2 process; unknown modes still return 400 and QUIC strategy runs remain unsupported.
- Verification passed: Rust all-target tests (179), Clippy with `-D warnings`, formatting, release build, ARM64 musl `cargo check`; frontend tests (99), typecheck and production build; root `git diff --check`.
- Router access was read-only. No candidate test, firewall mutation, config write or service action was performed. The current UI result still cannot claim upstream-parallel runtime equivalence or verified Keenetic execution.

## Mode-specific guidance and result feedback — 2026-10-23 19:24 UTC

- The mode selector now explains quick, extended, full and custom behavior dynamically; the full-mode notice calls out sequential operation, long duration and cancellation. The helper text and result cards animate briefly and respect reduced-motion settings.
- When a full scan finds no working variants, the results area now explains that outcome and includes the aggregate failure count instead of a generic empty-state message.
- Frontend tests (99), typecheck and production build passed after the UI changes.
- Re-read the 16 KB scan path against upstream `freezeJob()`/`padProbe()`: baseline results expose checked hostname/path and resolved or pinned IP plus ASN verification; only affected networks enter the SNI template/name search, with three consecutive successes required. Unit tests cover serialization and IPv4/ASN fields. No real router scan was run because the current workspace build is not installed on the router and this scan temporarily installs scoped firewall rules.

## Protocol catalog guard — 2026-10-23 19:26 UTC

- A source-order review caught that the shared catalog generator could be added to a QUIC plan, while upstream explicitly skips the HTTP/TLS catalog for QUIC. Added a failing regression, fixed the plan builder to omit that catalog for QUIC, and verified QUIC still receives only QUIC candidate families.
- Final Rust checks after the fix passed: 180 tests, Clippy, `cargo fmt --check`, release build, ARM64 musl `cargo check`, and `git diff --check`. Frontend remains green at 99 tests plus typecheck/build from the immediately preceding UI change.

## Full mode and baseline request follow-up — 2026-10-23 19:29 UTC

- Rechecked the full-mode catalogue against Zemidala's `catalogCandidates('full')`: the Rust generator carries the same full-only TTL, fake/split variants, hostfakesplit, fakedsplit/fakeddisorder, sequence-overlap and repeated tcpseg families. Generated plans remain bounded to 4,096 and filter every candidate against the device's installed Lua/blob inventory before probing.
- Matched the upstream baseline-first flow: one direct request is now made before candidate execution. If it succeeds, the picker reports that no strategy is needed; if its result is technically unavailable, the picker stops rather than running misleading candidate probes. Candidate repeats use the isolated test process without re-running the direct request.
- Fresh verification: `cargo test --offline --locked --all-targets` passed (180 tests); `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo fmt --check`, host `cargo build --release --offline --locked`, ARM64 musl `cargo check --offline --locked --all-targets --target aarch64-unknown-linux-musl`, `pnpm test` (99), `pnpm typecheck`, `pnpm build`, and `git diff --check` passed.
- Full runtime behavior still differs from Zemidala: its mark/source-port batch runner and `testRefine()` are not ported. This implementation runs the full compatible catalogue sequentially because the existing isolated runner only owns one lane; no multi-lane/firewall change has been validated on-device. The router remains unchanged; only read-only inventory was performed. No release was created.
