# Release v0.6.3

## Changes

- Monitoring queues one Telegram message containing all availability transitions from a single completed check cycle.
- Full monitoring completion time is stored after all probes finish; per-site checks do not move the full-cycle schedule.
- Purple accent utility classes are normalized to the active blue theme in dark and light modes; purple accents remain specific to neon mode.

## Verification

- Backend locked test suite, Rust formatting, and Clippy.
- Web unit tests, TypeScript/Vite production build, and `git diff --check`.
- No router deployment or live router checks were performed for this monitoring and presentation change.
