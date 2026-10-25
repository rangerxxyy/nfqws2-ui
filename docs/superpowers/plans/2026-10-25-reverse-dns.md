# Reverse DNS in site diagnosis implementation plan

> **For agentic workers:** Execute inline in this task; keep each step small and verify it before proceeding.

**Goal:** Show reverse DNS PTR names for each IP discovered by site diagnosis.

**Architecture:** The Rust diagnostic service runs one available local DNS utility (`kdig`, `dig`, or `nslookup`) with argument arrays and bounded output/timeouts. The report returns PTR names per address; the frontend adds a readable diagnosis row while tolerating older backend responses.

**Tech Stack:** Rust/Tokio/Axum backend, TypeScript frontend, existing Cargo and Vitest suites.

**Spec:** User request in this thread: identify site IPs' domain names through reverse nslookup/kdig/dig and display them.

## Global constraints

- Never invoke a shell or interpolate user input into a command string.
- Query at most four resolved IP addresses, with a bounded timeout and output per command.
- Do not treat missing PTR as a diagnosis failure.
- Keep older API responses readable by treating the new field as optional in the frontend.

## Review focus

- `dig` and `kdig` short output and BusyBox/standard `nslookup` formats parse into hostnames.
- Empty, malformed, or nonzero responses never become fake hostnames.
- No installed DNS utility produces an explicit unavailable result without failing the diagnosis.
- Literal IP inputs and hostname inputs both report reverse results when addresses exist.
- Process timeout and output size are bounded.

### Task 1: PTR parsing

**Files:** Modify `backend/src/site_diagnostics.rs`.

- [ ] Add tests for `dig`/`kdig` and `nslookup` output plus empty or invalid output.
- [ ] Run targeted tests and confirm the missing parser fails.
- [ ] Implement conservative PTR-name parsing.
- [ ] Run targeted tests.

### Task 2: Bounded lookup and report

**Files:** Modify `backend/src/site_diagnostics.rs`, `web/src/api.ts`.

- [ ] Add report records and select one available utility in priority order.
- [ ] Query up to four addresses with bounded process time/output; keep lookup optional on failure.
- [ ] Test utility argument formation, report serialization, and no-address behavior.

### Task 3: Diagnosis display

**Files:** Modify `web/src/diagnosis-layout.ts`, tests, `web/src/panel.ts` only if needed.

- [ ] Add an “Обратный DNS · PTR” row with names per IP and utility detail.
- [ ] Show a clear no-record or unavailable state without turning it into a blocking diagnosis error.
- [ ] Run full web and backend suites and production build.

### Task 4: Release

- [ ] Update v0.7.1 notes with PTR support and accurate verification scope.
- [ ] Run final release checks and publish only after CI succeeds.
