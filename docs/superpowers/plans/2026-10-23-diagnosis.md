# Диагноз доступности Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans; independent frontend work may run in parallel with backend using superpowers:dispatching-parallel-agents.

**Goal:** Диагностика direct/current и QUIC без HTTP3 curl, понятные итоги трассировки.
**Architecture:** Reuse unique bypass chains from site_trace for TCP/UDP. Add bounded QUIC helper and structured results to site_diagnostics. Existing TypeScript controller renders comparisons and details.
**Tech Stack:** Rust/Tokio, Lua nfqws2, TypeScript/React template, Vitest.
**Spec:** docs/superpowers/specs/2026-10-23-diagnosis-design.md

## Global Constraints
- Preserve UI style, service/config, diagnostic lock, public destination validation.
- No claim of HTTP3 completion based solely on UDP or Alt-Svc.
- Keep MIT attribution in source/distribution, no third-party branding in panel.

## Review Focus
- Cleanup on probe failures; timeout must not cancel a future with active firewall rules.
- Known HTTP error responses versus transport failures versus unavailable steps.
- Ordinary router traffic is not necessarily intercepted by nfqws2 under policy-only routing.
- Correlated QUIC long header reply; random UDP bytes are not success.
- curl absent, nfqws2 absent, no public IPv4, empty kernel queue list.

### Task 1: Structured UI results
- [x] Add direct/current/quic types to api.ts; pure verdict presentation tests for all result states.
- [x] Render comparison, QUIC method and explicit trace banners using textContent and accessible status; hide detailed steps behind details.
- [x] Run pnpm test and build.

### Task 2: Bypass and QUIC
- [x] Add failing TCP/UDP scoped-rule and stderr tests; implement shared bypass lifetime and bounded command output.
- [x] Add failing packet parsing/correlation and loopback UDP tests; implement QUIC Initial Lua, bounded generator and two probes, retain MIT notice.
- [x] Run Rust tests/clippy.

### Task 3: Diagnosis integration
- [x] Add verdict tests, implement direct/current sequential comparison with guaranteed cleanup and HTTP3/native QUIC fallback.
- [x] Pass installed binary from Rust endpoint; remove outer cancellation while firewall rules may be active.
- [x] Run full tests/build/audits; independent final review, fix important findings, publish authorized release after CI.

## Verification ledger

- Frontend TDD: expected matrix failures observed; 78/78 tests, typecheck and build passed.
- Backend RED/GREEN: stderr was missing; large output deadlocked for 4 seconds; both regression tests failed before fixes and passed after. UDP correlation and loopback/timeout tests passed.
- Final local suite: 50 library + 52 binary + 3 integration Rust tests, Clippy, fmt, dependency audits, installer/updater preflight passed.
- Browser: real Windows backend received HTTP206 for current route; direct unavailable correctly showed missing iptables, not blockage. No real Keenetic validation.
- Independent read-only review: one Important finding (shared QUIC CID between routes); fixed by fresh generator invocation per route. No Critical findings. Device interoperability remains outside available evidence.
- Ruling: keep GET probes and omit reference volume POST — comparison must not upload random content to arbitrary sites; this does not diagnose volume-dependent freezes.
- Ruling: pin both modes to the same validated public IPv4 — avoids DNS rebinding and makes comparison reproducible; ordinary router DNS/route limitations are disclosed.
- Release v0.2.5 published after successful Linux/ARM64 CI (run 39906363689). Downloaded archive SHA-256 verified: 6bda7d90e416e63b07bfc6d51366c008c29b6c2427f19439c5511a47c3db90eb. Required binary/UI/notices and installer VERSION=0.2.5 confirmed; no index.html in package.
