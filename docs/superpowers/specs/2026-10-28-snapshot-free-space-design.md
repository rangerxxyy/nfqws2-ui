# Snapshot free-space indicator

- Intent: help router users understand whether the filesystem holding UI snapshots has room for another archive, alongside the configured snapshot byte budget.
- Goal: report available bytes from the snapshots filesystem and warn when fewer than the backend's 64 MiB per-archive ceiling remain. Unknown platform/filesystem data must be shown as unavailable, not guessed.
- Non-goals: changing snapshot retention, disabling snapshots automatically, or probing/removing files.
- Approaches considered: shelling out to `df` (depends on output format and command availability); adding a filesystem library (new dependency); using the existing `libc` dependency with `statvfs` on Unix and returning unknown elsewhere. Chosen: `statvfs`, because the router target is Linux and the backend already depends on libc; unsupported platforms can safely report unknown.
- UI: add one compact status line to the snapshots panel. Keep the configured storage budget and current-used amount as separate values.
- Builder call: compute available bytes for the existing data directory's filesystem; saturate arithmetic; do not mutate the filesystem while reading status.
