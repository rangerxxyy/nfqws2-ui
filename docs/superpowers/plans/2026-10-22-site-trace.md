# Site Trace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a separate “Трассировка” section that reports whether a site responds without nfqws2 and with nfqws2 and shows the test process journal.

**Architecture:** Add a Rust trace module with a testable operation boundary for preflight, temporary firewall rules, probes, isolated nfqws2 process and guaranteed cleanup. Expose bounded asynchronous start/status/stop endpoints, following the existing operation pattern and Zemidala’s background trace. Add an independent TypeScript navigation route and polling view beside “Проверка сайта”.

**Tech Stack:** Rust, Tokio, Axum, TypeScript, React/TSX, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-10-22-site-trace-design.md`

## Global Constraints

- Do not edit configuration, lists, service files, init-managed chains or stop/restart the primary nfqws2 process.
- Refuse to change firewall if ownership or safe coexistence with the active init script cannot be established.
- Accept only a validated public hostname; select and report one verified public IPv4 destination.
- Run all subprocesses using fixed executables and argument vectors, never shell-concatenated request data.
- Bound probes, operation time, process lifetime and response log size.
- Always remove only this operation’s temporary rules before its stored status becomes terminal, including partial setup and failures.
- Keep “Проверка сайта” and “Трассировка” as separate navigation routes and independent UI state.
- Treat HTTP 4xx/5xx as an HTTP response, not as transport failure.
- Local tests must not install firewall rules; do not report them as router tests.

## Review Focus

- Private/reserved DNS answers: prove rejection happens before firewall or process calls (Task 1).
- Init-managed/ambiguous firewall ownership: prove fail-closed with no mutation (Task 2).
- Partial rule setup and timeout: prove owned rules and child processes are cleaned up (Task 2).
- HTTP 4xx/5xx: prove the site is reported as responding while preserving its status code (Task 1).
- Missing nfqws2 process/binary or queue conflict: prove the baseline result remains available and the second phase reports unavailable (Task 3).
- Journal lines containing HTML/script text: prove UI renders them as text, not markup (Task 5).

---

### Task 1: Trace report model and safe pure helpers

**Files:**
- Create: `backend/src/site_trace.rs`
- Modify: `backend/src/lib.rs`
- Test: `backend/src/site_trace.rs`

**Interfaces:**
- Produces `TraceReport`, `TracePhase`, and pure helpers for public destination selection, HTTP reachability classification, active argv sanitization, test queue selection, and bounded log parsing.
- The report contains `host`, `address`, independent `withoutNfqws2` and `withNfqws2` phases, `traceLines`, `traceTruncated`, and a human-readable summary.

- [ ] **Step 1: Write failing Rust unit tests** for private/reserved IPv4 rejection, status-code reachability (including 404/503), queue conflict, protected process-argument removal, and log line/byte bounds.
- [ ] **Step 2: Run the focused tests and verify they fail** because the new module/types/helpers do not exist.
- [ ] **Step 3: Implement the report types and pure helpers** in `site_trace.rs`; export the module from `lib.rs`.
- [ ] **Step 4: Run focused tests, then all backend tests** and verify the helpers produce the exact tested classifications and bounds.
- [ ] **Step 5: Commit** as `feat: add site trace report and validation helpers`.

### Task 2: Isolated router operation and firewall cleanup

**Files:**
- Modify: `backend/src/site_trace.rs`
- Test: `backend/src/site_trace.rs`
- Modify: `backend/src/main.rs` only if an existing shared operation lock must be exposed

**Interfaces:**
- Consumes validated host/address and the report helpers from Task 1.
- Produces `run_trace(system, host, cancel) -> TraceReport` plus an internal injectable `TraceSystem` boundary that production implements with fixed `tokio::process::Command` calls and tests implement without touching the host firewall. The job caller owns the shared diagnostic lock for the task’s full lifetime.

- [ ] **Step 1: Write failing tests** for firewall ownership preflight, unique owned-chain command vectors, partial-setup cleanup, probe ordering, isolated nfqws2 launch/stop, and timeout cleanup.
- [ ] **Step 2: Run focused tests and verify they fail** on the missing operation boundary/cleanup behavior.
- [ ] **Step 3: Implement the operation**: preflight init/firewall ownership and binary/queue availability; install only owned exact-match rules; run the baseline request bypassing the main process; run one request through a separate nfqws2 clone and capture its bounded log; always clean up child and owned rules before reporting terminal status. Preserve the completed baseline result if the nfqws2 phase fails.
- [ ] **Step 4: Run focused tests and all backend tests**; verify fake-system assertions prove no production firewall command runs in tests.
- [ ] **Step 5: Commit** as `feat: run isolated nfqws2 site trace`.

### Task 3: Background operation store and bounded Axum endpoints

**Files:**
- Modify: `backend/src/main.rs`
- Test: `backend/src/main.rs`
- Modify: `backend/src/site_trace.rs` for operation state serialization if needed

**Interfaces:**
- Adds `POST /api/nfqws2/diagnostics/trace` with JSON `{ "host": string }`, returning 202 and a trace operation ID; `GET /api/nfqws2/diagnostics/trace/operations/{id}` returns state and partial/final report; `POST /api/nfqws2/diagnostics/trace/operations/{id}/stop` requests graceful termination and cleanup.
- Moves the existing shared site-diagnostic lock into `AppState` as an `Arc<Mutex<()>>`; the trace worker owns its guard until cleanup and terminal state. Invalid host is 400, a busy diagnostic/trace is 409, and handler errors use the existing JSON error envelope.
- Operation files live under `data_dir/site-trace-operations`; IDs are server-generated, status parser accepts only known states, journal/response sizes and retention are bounded.

- [ ] **Step 1: Write failing tests** for invalid/local host rejection before job creation, start/status/stop serialization, malformed/unknown operation IDs, state-file size bounds, and conflict behavior while another diagnostic owns the lock.
- [ ] **Step 2: Run focused endpoint tests and verify they fail** because the route and handler are absent.
- [ ] **Step 3: Implement start/status/stop handlers and worker spawning** using existing hostname validation and the shared lock; continue the worker after browser disconnect, persist phase progress, handle stop requests through a cancellation signal, and report terminal states only after cleanup.
- [ ] **Step 4: Run focused and full Rust tests plus clippy** and verify response codes and camelCase field names.
- [ ] **Step 5: Commit** as `feat: expose site trace API`.

### Task 4: Typed frontend API and independent trace route

**Files:**
- Modify: `web/src/api.ts`
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`
- Test: `web/src/api.test.ts`
- Test: `web/src/panel-template.test.ts`

**Interfaces:**
- Adds `SiteTraceOperation`/`SiteTraceReport` types and `nfqwsDiagnostics.traceStart(host)`, `.traceStatus(id)`, `.traceStop(id)`.
- Adds route `trace` as a separate nav item and section; the existing `check` route and `nfqwsDiagnostics.site(host)` stay unchanged.

- [ ] **Step 1: Write failing frontend tests** for typed trace request path/body and presence/separation of the two navigation sections.
- [ ] **Step 2: Run focused frontend tests and verify they fail** on missing API method/route.
- [ ] **Step 3: Implement the API methods and route wiring** with a trace page that accepts its own hostname, displays loading/progress/error states, polls until terminal status, offers stop, shows separate baseline/with-nfqws2 result cards, and a bounded scrollable journal. Use text nodes/`textContent` for all backend strings.
- [ ] **Step 4: Run frontend tests, typecheck and production build**; verify existing site diagnosis navigation remains unchanged.
- [ ] **Step 5: Commit** as `feat: add site trace panel`.

### Task 5: Router smoke-test instructions and final review

**Files:**
- Create or modify: `docs/site-trace-smoke-test.md`
- Modify: `README.md` only if it documents the diagnostics routes

**Interfaces:**
- Documents checks a user can trigger from the computer’s browser while the Rust backend runs on the router, with explicit preflight, expected output and cleanup confirmation.
- Makes clear that local tests are not hardware validation and that fail-closed firewall ownership means “trace unavailable”, not a false success.

- [ ] **Step 1: Write the smoke-test checklist** for a known reachable site, a site requiring nfqws2, and a controlled invalid/private host; include confirmation that the main service and config remain unchanged.
- [ ] **Step 2: Review every checklist command** to ensure it is read-only or confined to the trace operation’s temporary rules; remove any instruction that asks the user to flush or manually edit existing chains.
- [ ] **Step 3: Run final verification**: `pnpm test`, `pnpm build`, `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `git diff --check`.
- [ ] **Step 4: Record router status honestly**: only mark hardware smoke tests passed if run against the actual Keenetic; otherwise report them as pending.
- [ ] **Step 5: Commit** as `docs: document safe site trace validation`.

## Final review

- Verify all spec requirements have a task and no task mutates init-managed rules.
- Verify the two frontend routes remain separate and the trace API cannot accept arbitrary commands, paths, interfaces, queue IDs or IPs.
- Verify baseline and with-nfqws2 outcomes survive partial failures independently.
- Verify cleanup is run before every API return and local tests do not use the real firewall.
