# Release v0.6.2

## Changes

- User-defined monitoring intervals now have a 10-minute minimum, matching the current `zemidala/nfqws2-ui` setting validation; the maximum remains 24 hours.
- Legacy saved intervals below 10 minutes are clamped when read, so the background scheduler cannot continue with an overly frequent old setting; the monitoring UI also clamps custom values before saving.
- The “About” page uses a neutral version example for forced update/rollback commands instead of showing the stale v0.6.1 number.
- Telegram transition behavior was reviewed against upstream `monitorRun()`: no notification on the first result, completed HTTP responses (including 404) count as available, and incomplete transfer remains a failed check. Existing behavior already matched and was retained.

## Verification

- Web tests: 173 passed.
- Web TypeScript typecheck: passed.
- Web production build: passed before the version-only metadata update; rerun as part of release verification.
- Full Rust test suite: passed.
- `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Router and browser visual testing were not performed in this pass.
