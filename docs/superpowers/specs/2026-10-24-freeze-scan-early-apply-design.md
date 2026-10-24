# Apply confirmed 16 KB strategies during a scan

## Intent

Let a user apply a strategy to an explicitly entered domain as soon as that domain has a strategy confirmed by three consecutive successful isolated probes, while the remaining 16 KB diagnostic continues.

In the same 0.3.6 release, split the configuration screen into two clear sections: the full `nfqws.conf` editor opens by default; quick controls, profile summaries, runtime arguments, module files, and flag reference live under “Настройки конфигурации”.

## Constraints

- Never apply a pending, partial, ASN-reused, or network-reference result.
- Preserve the existing domain-only profile scope, config revision check, nfqws2 dry-run, backup, restart, and rollback.
- Do not restart nfqws2 while an isolated probe is running. The scan must pause at a safe result boundary, yield the diagnostic lock to the apply request, and resume after the apply transaction finishes.
- Keep persisted scan progress and cancellation semantics intact.
- Continue remaining probes from their existing isolated runner arguments; the newly added profile is scoped only to the already-completed user domain.

## Approaches

1. Show the button early but reject application until the scan finishes. This is misleading and fails the request.
2. Cancel the remaining scan when applying. This loses the requested continued scan.
3. Queue a pause request and yield the shared diagnostic lock after a target's complete candidate pass. Apply through the existing guarded transaction, then resume the scan. This satisfies the request while keeping service restarts outside active probe processes.

For the configuration screen, keep one full-width editor view and move the existing structured controls to a separate settings view. This preserves both editing paths while making the raw configuration the first view, as requested.

## Chosen behavior

Use approach 3. Show the action only for a completed, user-entered domain row with `freezeDetected` and its own strategy confirmed 3/3. A click requests a safe pause; the scan acknowledges at the end of the current target's candidate pass, releases the diagnostic lock, and waits. Backend re-reads the running operation and persisted row, validates the profile and config revision, dry-runs, writes a backup, restarts, and rolls back on failure. The scan resumes after either success or failure.

## Files and verification

- `backend/src/freeze_diagnostic.rs`, `backend/src/main.rs`: pause control, in-flight eligible result validation, shared-lock handoff, and regression tests.
- `web/src/freeze-presentation.ts`, `web/src/panel.ts`: show the per-domain action while scanning and submit the existing apply request.
- Tests must prove early eligibility only after 3/3, exclusion of partial/reused/IP results, pause/resume around apply errors and success, plus the existing 16 KB, apply transaction, and UI test suites.
