# Background UI Update Notice

## Goal

Notify users when a newer stable nfqws2-UI release with the required ARM64 package and checksum is available, while keeping installation an explicit action.

## Behavior

- The Rust service checks GitHub Releases immediately after startup and then every four hours. A failed request is retried after fifteen minutes without replacing the last successful result.
- Only releases accepted by the existing release eligibility filter are considered. Compare strict `vMAJOR.MINOR.PATCH` versions numerically.
- A read-only status endpoint exposes the current version, newest eligible version, availability, release URL, and last successful check time.
- The UI polls this local endpoint at startup and every five minutes while visible. It highlights the existing UI update control with a blue outline when an update is available.
- A persistent bottom-right notice appears once per newly available tag. Dismissal is stored per browser and tag; a newer tag can notify again. The notice can open the existing update picker or link to the release page.
- No update is downloaded, installed, or applied automatically. Existing installer validation, checksum checks, health check, and rollback remain authoritative.

## Approach

Use a small Rust background checker and status endpoint, following XKeen UI's four-hour server-side polling and persistent update toast concept. Adapt it to this project's existing stable-release eligibility and guarded installer rather than adding a separate updater or letting the browser fetch GitHub directly.

## Verification

- Unit tests cover version selection, same/older/newer versions, and releases rejected by the existing eligibility filter.
- API tests verify a read-only status response.
- Frontend tests cover highlight/notice behavior, per-tag dismissal, and the UI picker action.
- Run Rust formatting, all backend tests, Clippy, frontend tests, typecheck, and production build; the release workflow builds ARM64 assets.
