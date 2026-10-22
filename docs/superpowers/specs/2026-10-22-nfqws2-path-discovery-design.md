# nfqws2-ui Path Discovery Design

## Goal

Resolve the real nfqws2 installation paths on supported Keenetic/Entware and OpenWrt layouts, using installed files, the active configuration and the running process as evidence. The UI must report missing or unresolved paths instead of inventing values or creating placeholder files.

## Scope

- Detect the installation root from the actual `nfqws2` executable and corroborating package files. Support `/opt` (Entware) and `/` (OpenWrt) without mixing their paths.
- Derive the config, lists directory, Lua/blob directories, init script, PID file, binary and standard log candidates from the detected root.
- Read the active config and the exact running `nfqws2` process argv (`/proc/<pid>/cmdline`) to resolve configured list, module and debug-log references. Do not execute, `source`, or `eval` config text.
- Resolve only supported literal values and a bounded subset of references to config variables. If a value is dynamic, conflicting, malformed or ambiguous, report it as unresolved with a reason; never substitute a guessed path.
- Return a diagnostic path map with source, detected/existing/readable/writable/active-use state. Keep the existing API's config/list response shapes where possible and add explicit diagnostic fields.
- Route service actions, list operations and log reads through the detected installation rather than selecting unrelated fixed paths independently.
- Preserve the test config override and inject discovery roots/process data in tests; no router-specific path is assumed in tests.

## Path and write policy

- Do not create missing configs, lists, module files, logs or PID files during discovery.
- Reject path traversal and symlink components for writes. An editable list must be a supported list ID, explicitly referenced by the active config or process argv, exist as a regular file, and canonicalize inside the detected nfqws2 package config/list roots. A configured path outside those roots is reported but is read-only until a separate policy is approved.
- Config writes remain limited to the detected `nfqws2.conf`; preserve revision checks, private backups, atomic replacement, read-back verification and apply rollback.
- Module and log references are diagnostic/read-only. Do not return file contents for them through the installation endpoint.
- Never expose credentials or sensitive file contents in diagnostics.

## Resolution precedence

1. Find installed binary candidates at the supported package locations. If nfqws2 is running, require its exact argv[0] to match one candidate; when stopped, require a unique installed candidate corroborated by its config or init script.
2. Determine `/opt` versus `/` from the selected binary and corroborating config/init files; if evidence conflicts or multiple installations remain plausible, report ambiguity and disable mutations.
3. Locate the expected config under the selected root. An explicit test override remains authoritative in tests only.
4. Parse literal config assignments without shell execution, resolve supported variable references with cycle/depth limits, and extract known nfqws2 flags.
5. Parse active argv and reconcile its paths with the config. Report mismatches; do not silently select one source for a mutation.
6. Canonicalize each existing candidate, reject symlink traversal for mutation, then report permissions and active use.

## API and UI

- Extend `GET /api/nfqws2/installation` into the authoritative path map for the detected installation, including unresolved references and reasons.
- Existing status/config/list/service/log handlers consume the same resolved installation snapshot. If discovery is ambiguous or a required file is absent, return a specific error rather than empty/mock values.
- The UI continues to use the separate TypeScript API client and displays the confirmed path and access state for config and each available list. Unavailable operations remain disabled with the backend reason.

## Verification

- Unit fixtures cover `/opt` and `/` layouts; binary/config/init mismatches; process argv; relative and absolute list references; supported variable expansion; cycles, shell substitutions, conflicting references; absent files; unreadable/unwritable paths; symlinks and traversal.
- API tests verify that status, config, lists, service and logs all use the same detected installation, and that ambiguity disables writes without creating files.
- Run Rust fmt, tests and clippy; TypeScript tests, typecheck and build; shell checks and release packaging checks.
- CI and source inspection do not count as router verification. Report actual device paths only after the user verifies them on Keenetic/OpenWrt.

## Constraints and limits

- Upstream `nfqws2-ui` confirms the conventional `/opt` and `/` package layouts; the target router's actual paths still require device verification.
- `nfqws-keenetic-web` is an older nfqws generation. Reuse its file-inventory/API ideas only; do not import legacy command flags.
- No arbitrary browser-supplied paths or shell commands are accepted.
- The unauthenticated API remains limited to the trusted LAN as previously requested; it must not bind these path-resolution rules to arbitrary client input.

## Self-review

- The resolver uses active configuration and process evidence while clearly reporting values it cannot safely understand.
- All mutation endpoints share one discovered installation, preventing config/list/service/log path drift.
- Custom configured files outside the package roots remain visible in diagnostics but are not editable under this design; changing that boundary requires a separate explicit policy decision.
- No behavior is claimed as verified on a physical router by fixture tests.
