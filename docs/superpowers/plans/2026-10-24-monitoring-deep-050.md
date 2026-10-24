# Monitoring Deep Test Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Release nfqws2-UI 0.5.0 with manual monitoring that reports a bounded HTTPS transfer and the DNS/TCP/TLS/HTTP stages.

**Architecture:** The backend resolves and filters public IPv4 addresses, pins a bounded HTTPS GET to one address, parses curl timings and emits ordered stages. The frontend persists and displays those stages for each check. Release metadata is bumped together and the existing tag-triggered workflow publishes the package.

**Tech Stack:** Rust/Tokio, curl already required by diagnostics, TypeScript/Vitest, Vite, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-24-monitoring-deep-test-design.md`

## Global Constraints

- Keep manual monitoring capped at 20 sites and 10 saved results per site.
- Each HTTPS probe is IPv4-only, pinned to a public resolved address, limited to 64 KiB and 8 seconds, and does not follow redirects.
- Do not add POST probes, schedules, notifications, or strategy application.
- A timeout after an HTTP response is a possible transfer interruption, not a confirmed DPI cause.
- Publish version `0.5.0` through the repository's existing `v*` release workflow.

## Review Focus

- HTTP response followed by body timeout must not be marked as a successful complete transfer — parser regression test.
- DNS answers containing only private/reserved or IPv6-only addresses must not be passed to curl — route validation test.
- Old browser history without `steps` must still load — storage migration test.
- Large response bodies and redirects must remain bounded — curl argument test.
- An HTTP 404 response still means the server answered — status classification test.

---

### Task 1: Backend HTTPS monitoring probe

**Files:**
- Modify: `backend/src/site_diagnostics.rs`
- Modify: `backend/src/main.rs`
- Test: `backend/src/site_diagnostics.rs`

**Interfaces:**
- Produces `site_diagnostics::monitor_probe(host: &str, ip: Ipv4Addr) -> MonitorProbeResult` with `summary`, `current`, and ordered `steps`.
- `monitor_sites` resolves a public IPv4 before calling the probe and includes DNS as the first step.

- [ ] Add parser tests for complete 200 response, HTTP 404, connection failure, TLS failure, and response-then-timeout.
- [ ] Add a curl-argument test for HTTPS GET, pinned IPv4, range/max size, no redirects, and no POST.
- [ ] Observe those tests fail before implementation.
- [ ] Implement the parser/probe and expose its stage results from the API.
- [ ] Run backend tests and formatting.

### Task 2: Persist and display monitoring stages

**Files:**
- Modify: `web/src/api.ts`
- Modify: `web/src/monitoring.ts`
- Modify: `web/src/monitoring.test.ts`
- Modify: `web/src/panel.ts`
- Modify: `web/src/App.tsx`
- Test: `web/src/monitoring.test.ts`, `web/src/panel-template.test.ts`

- [ ] Add tests for stage retention and reading version-1 history without stages.
- [ ] Observe tests fail before implementation.
- [ ] Store per-stage statuses/timings, show an expandable DNS/TCP/TLS/HTTP/transfer list, and correct the page's copy to describe bounded parallel HTTPS checks.
- [ ] Run typecheck, frontend tests, and production build.

### Task 3: Prepare and publish 0.5.0

**Files:**
- Modify: `backend/Cargo.toml`, `backend/Cargo.lock`, `web/package.json`
- Modify: `CHANGELOG.md`, `.github/workflows/release.yml`

- [ ] Bump package versions and add 0.5.0 release notes.
- [ ] Run the full release verification available locally, including package scripts and static checks.
- [ ] Commit the release changes, push the branch, and push tag `v0.5.0` so GitHub Actions builds and publishes the ARM64 assets.
- [ ] Verify the GitHub release workflow and assets complete successfully.
