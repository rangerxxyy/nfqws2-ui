# Verified Strategy Application Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Roll back a confirmed per-site strategy application when the target or a previously healthy monitored site fails its post-restart check.

**Architecture:** Keep the existing config backup/write/restart transaction and add an optional post-restart verifier. The strategy endpoint snapshots the target plus monitored sites whose latest saved result is `ok`, probes them concurrently, retries only failures once, and lets transaction failure restore the original config and restart the service.

**Tech Stack:** Rust, Tokio, Axum, existing monitoring probe and config transaction.

**Spec:** Zemidala `nfqws2-ui` current `src/www/api.php` `autoApply()` (main at `09014d8092161f3bfbbb596288f1a7cf43966ce1`) verifies the target and previously healthy monitored peers after restart and rolls back on failure.

## Global Constraints

- Only the explicit confirmed-strategy application endpoint gains network verification.
- Existing generic configuration apply behavior stays unchanged.
- Do not probe monitored sites whose latest saved status is not `ok`.
- Retry a failed probe at most once; run independent host checks concurrently.
- On failed verification, restore only if the written config revision still matches the transaction candidate.

## Review Focus

- Empty monitoring history: still verify the target once.
- Duplicate target in monitoring: probe once.
- A previously failed monitored site: exclude it from regression checks.
- One failed verification followed by a healthy retry: keep the candidate config.
- Persistent failure or changed config revision: report rollback outcome accurately and preserve newer external edits.

---

### Task 1: Add verified config transaction

**Files:**
- Modify: `backend/src/main.rs`
- Test: `backend/src/main.rs` unit tests

**Interfaces:**
- Add `apply_config_transaction_with_verification(state, write, restart, verify)` while keeping `apply_config_transaction` as a no-op-verifier wrapper.
- `verify` returns `Result<(), String>` after a successful restart.

- [x] Add tests that verification failure restores the original bytes and calls restart a second time, while a successful retry leaves the new config.
- [x] Run the focused Rust transaction tests and confirm the new failure case fails first.
- [x] Implement the verifier callback in the transaction and run the focused tests again.

### Task 2: Check target and healthy peers after strategy apply

**Files:**
- Modify: `backend/src/main.rs`
- Modify: `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`

**Interfaces:**
- Add a helper to build the bounded unique verification host list from the operation host and latest `ok` monitoring results.
- Add an async helper that probes hosts concurrently and retries only failures once.

- [x] Test host-list selection for target deduplication and latest healthy status.
- [x] Test result handling for recovered and persistent failures.
- [x] Wire the helpers only into `apply_confirmed_strategy` and update the source audit with current limitations.
- [ ] Run full Rust format, unit/integration tests and clippy.
