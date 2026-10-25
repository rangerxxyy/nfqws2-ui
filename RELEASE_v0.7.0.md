# Release v0.7.0

## Changes

- Restyled the update notice with a darker navy background, the locally bundled Inter font, a Tabler Circle Check icon, and separate GitHub and dismiss actions.
- Strategy picking now marks candidates that rely on TCP timestamps and shows the Windows enable command with a copy button. The reminder can be disabled in UI appearance settings.
- The list editor supports `Ctrl+/` to comment selected lines or remove comment markers from already-commented lines.

## Verification

- Web: 180 unit tests passed; production build passed.
- Backend: locked test suite, `cargo fmt --check`, and Clippy with warnings denied passed.
- No live router probes were run for this UI and editor update.
