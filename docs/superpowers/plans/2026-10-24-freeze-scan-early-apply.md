# Early 16 KB Strategy Apply Implementation Plan

> **For agentic workers:** Use the established inline workflow, one testable step at a time.

**Goal:** Apply a domain's own 3/3 confirmed strategy while the rest of the 16 KB scan continues.

**Architecture:** Persisted per-domain results become apply-eligible while the operation is running. The scan accepts a pause request only at a completed target boundary, yields the shared diagnostic lock, and resumes after the existing transactional apply succeeds or fails.

**Tech Stack:** Rust/Tokio, TypeScript, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-10-24-freeze-scan-early-apply-design.md`

## Global Constraints

- Require a completed user-host result, an observed possible freeze, and an unreused strategy with at least 3 attempts and 3 successes.
- Keep configuration mutation, dry-run, backup, restart, revision check, and rollback in the existing backend transaction.
- Never restart the service while a temporary isolated probe is active.
- Preserve the active scan and continue remaining targets after apply success or failure.

## Review Focus

- In-flight result persistence: only the strategy row already written to the operation can be applied.
- Pause races: cancel, timeout, apply failure, and simultaneous requests must release locks and resume or terminate deterministically.
- Process restart: later isolated probes must retain correct baseline process arguments and remain target-scoped.
- Ineligible rows: IP/reference targets, ASN-reused candidates, partial/unconfirmed candidates stay unavailable.
- UI polling: a button appears only after the confirmed row is returned by status polling and remains protected against duplicate clicks.

---

### Task 1: Result eligibility

**Files:** Modify `backend/src/main.rs`, `web/src/freeze-presentation.ts`; tests in matching Rust and Vitest files.

- [ ] Add failing tests allowing running scans only when the specific user-host result is persisted and confirmed 3/3.
- [ ] Verify failures.
- [ ] Update the backend validator and presentation predicate; keep IP, reused, partial, and unconfirmed results rejected.
- [ ] Run focused backend and UI tests.

### Task 2: Safe scan pause and resume

**Files:** Modify `backend/src/main.rs`, `backend/src/freeze_diagnostic.rs`; tests in `backend/src/main.rs`.

- [ ] Add failing coordinator tests for request, safe-boundary acknowledgment, resume, and failure cleanup.
- [ ] Verify failures.
- [ ] Implement a per-scan pause coordinator that retains cancel state and only yields the shared diagnostic lock between completed target candidate passes.
- [ ] Let the apply endpoint request/await the pause, apply through the existing transaction, and always signal resume on every response path.
- [ ] Run focused concurrency and transaction tests.

### Task 3: UI action while scanning

**Files:** Modify `web/src/panel.ts`, `web/src/freeze-presentation.test.ts`, and panel tests.

- [ ] Add a failing assertion for the action on a confirmed result in a running operation.
- [ ] Verify failure.
- [ ] Show the row action for the eligible persisted row; preserve in-flight, duplicate-click, and status feedback behavior.
- [ ] Run full web tests and typecheck.

### Task 4: Full verification and release

- [ ] Run `cargo fmt --all -- --check`, full `cargo test --locked --offline`, `cargo clippy --locked --offline --all-targets -- -D warnings`, full `pnpm test`, `pnpm typecheck`, `pnpm build`, and `git diff --check`.
- [ ] Review diff for lock release on every branch and ensure no router probe is started or interrupted.
- [ ] Update changelog/release notes, bump patch version, publish through the repository workflow, and verify tag/assets/workflow.
