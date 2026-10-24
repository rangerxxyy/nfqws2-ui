# Background UI Update Notice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect eligible newer UI releases in the Rust service and show a dismissible bottom-right notice plus blue update-button outline.

**Architecture:** Add a cached update status to `AppState`, populate it from the existing eligible release catalog in a startup/four-hour background checker, and expose it via a read-only API route. The frontend polls that local route while visible, highlights the existing UI update button, and shows one persistent notice per tag that opens the existing update picker.

**Tech Stack:** Rust, Axum, Tokio, TypeScript, DOM APIs, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-24-background-ui-update-notice.md`

## Global Constraints

- Only stable releases that pass the existing ARM64 package and SHA-256 eligibility filter can be announced.
- Version comparison is strict numeric `vMAJOR.MINOR.PATCH`.
- Notification never installs automatically; the existing update flow remains authoritative.
- Repeated notices for the same dismissed version stay suppressed in that browser.

## Review Focus

- GitHub unavailable at startup: retain the last known result and retry after 15 minutes.
- Tag ordering differs from API ordering: choose the greatest semantic version.
- A release without a package/checksum: never mark it available.
- Dismissed toast after reload: remain dismissed for that same tag.
- New tag after dismissal: show the notice again.

### Task 1: Backend release checker

**Files:** `backend/src/main.rs`, `backend/Cargo.toml`, `backend/Cargo.lock`

- [ ] Add a failing unit test for selecting only the greatest eligible UI release and comparing it to the current version.
- [ ] Run the targeted test and verify it fails for the missing behavior.
- [ ] Add `UiUpdateStatus`, `AppState` cache, a pure status builder, and tests for older/equal/newer releases.
- [ ] Add a read-only `/api/updates/status` route and API test.
- [ ] Start a non-blocking Tokio checker after service state initialization: first check immediately, then four-hour intervals; retry failures after fifteen minutes without clearing successful state.
- [ ] Run targeted Rust tests and formatting.

### Task 2: Frontend notice and highlight

**Files:** `web/src/api.ts`, `web/src/panel.ts`, `web/src/panel.css`, `web/src/App.tsx`, `web/src/ui-update-notice.ts`, tests under `web/src`

- [ ] Add failing tests for showing a notice only for an undismissed available tag and allowing a different tag after dismissal.
- [ ] Run the targeted test and verify the expected failure.
- [ ] Poll local status at startup and every five minutes while visible; style the existing UI update pill with a blue outline when available.
- [ ] Add an accessible bottom-right persistent notice with close, release link, and action to open the existing UI update picker.
- [ ] Persist dismissed tag in local storage and respect reduced-motion styles.
- [ ] Run frontend tests, typecheck, and production build.

### Task 3: Release 0.3.5

**Files:** `backend/Cargo.toml`, `web/package.json`, `CHANGELOG.md`, `.github/workflows/release.yml`

- [ ] Bump versions and add concise release notes.
- [ ] Run the full Rust, frontend, formatting, Clippy, and diff checks.
- [ ] Publish the changes through a PR and the existing ARM64 release workflow.
