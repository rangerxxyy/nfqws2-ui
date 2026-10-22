# nfqws2-ui release picker design

## Goal

Let an administrator select and install published versions from the panel using separate targets: nfqws2-ui and the nfqws2 core. Keep the existing TypeScript/TSX frontend and Rust backend split. Both catalogs use real GitHub Releases metadata, and both update operations report terminal success only after a post-install health/status check.

## Agreed scope

- The version controls in the header open independent pickers for nfqws2-ui and nfqws2 core.
- Panel releases come from `rangerxxyy/nfqws2-ui`; core releases come from `nfqws/nfqws2-keenetic`. Frontend code cannot provide repository URLs, arbitrary download URLs, shell commands, or package paths.
- The panel modal shows stable releases with this project's ARM64 musl package and SHA-256 sidecar. The core modal shows stable versions with the matching aarch64-3.10 Entware package and GitHub's SHA-256 asset digest. Notes are inserted as text, never as HTML.
- Core update is limited to Entware aarch64-3.10 in this release. Other router architectures are reported as unsupported rather than receiving a fabricated package choice.
- The modal has a selected version and an Install button. The layout follows the supplied XKeen UI reference while retaining nfqws2-ui branding.
- A notification appears at the bottom of the panel for install start, progress/connection recovery, success, and failure. Success is shown only after the update transaction reports health-check success.

## Backend and update transaction

1. Add a Rust endpoint to list eligible GitHub Releases. The server requests only the fixed GitHub API URL, applies bounded timeouts and response-size limits, filters drafts/prereleases and tags outside the accepted version format, and only returns releases containing the expected `aarch64-unknown-linux-musl` archive and `.sha256` asset.
2. Add an install endpoint that accepts a release tag, refreshes or reuses the server-side allowlist, and rejects any tag not returned as eligible. It does not accept an asset URL, filesystem path, or command from the browser.
3. Start a fixed, packaged panel updater helper as a detached operation so it survives the panel service restart. It downloads the selected archive/checksum, verifies SHA-256, rejects unsafe tar paths/types, stages the package, switches `/opt/share/nfqws2-ui/current`, and performs a local `/api/health` check.
4. For core updates, the Rust API independently resolves both the selected and currently installed package from the fixed upstream release catalog. The packaged helper downloads the matching `.ipk` files, verifies GitHub's SHA-256 asset digests, installs only the selected package with Entware `opkg`, restarts `S51nfqws2`, and installs the previous package with `--force-downgrade` if installation or service status fails.
5. Keep the prior panel release until panel health succeeds. Persist operation status under the panel data directory so the restarted Rust process can answer status polling. Core and panel updates share a single active-operation lock.

## Frontend behavior

- Replace the currently disabled UI update actions with a real release list and selected-version state.
- Match the attached XKeen UI screenshot approximately: dark modal; download icon and title at top left; close control at top right; short “choose version” hint and repository/GitHub badges; vertically stacked release cards showing tag, date, and available notes; selected card uses a clear blue border/background and radio indicator. Keep the list region independently scrollable and pin a full-width Install button to the modal bottom so it stays visible while browsing.
- Keep compact cards for releases without notes; expand note-bearing selected releases enough to show readable, sanitized release notes, using a bounded card/list area for long histories.
- Preserve nfqws2-ui branding and responsive layout; do not copy XKeen text or branding. The modal and list must remain usable on narrow screens and keyboard-operable.
- Use the existing API client for list/install/status calls.
- Render release notes via `textContent`; show loading, empty, error, installing, completed, and rolled-back states.
- Poll the operation endpoint while installing and reconnect after the panel restarts. The bottom notification stays visible long enough to communicate the final outcome.
- Keep the two update targets visibly separate and show the installed core version in its own picker.

## Verification

- Rust unit/API tests: parse and filter stable eligible releases; reject invalid tags, prereleases, drafts, incompatible/missing assets, oversized or invalid API responses, and unlisted install tags.
- Updater tests/preflight: invalid arguments, unsafe package names, checksum handling, unsafe archive paths/types, successful switch/health result, and rollback after start or health-check failure using temporary roots and fake service/wget/opkg tools. Do not execute destructive tests on a router.
- Frontend tests: API paths and payloads, release selection, safe release-note rendering, loading/error/operation states, and bottom success/failure notifications.
- Run Rust fmt/tests/clippy, frontend tests/typecheck/build, shell syntax/ShellCheck, and release packaging checks. Actual Keenetic verification remains a separate device check.

## Constraints and known limits

- Current release workflow publishes only `aarch64-unknown-linux-musl`; releases without that archive and checksum are not selectable.
- The device needs HTTPS-capable `wget`, `tar`, `sha256sum`, root execution, and the current service/init paths used by the existing installer.
- The API currently has no authentication by explicit user request; the panel must remain restricted to a trusted LAN by router policy.
- A browser disconnect during service replacement is expected. Persisted operation status and a fresh health request determine the result.

## Self-review

- No arbitrary URL, shell command, or filesystem path crosses from the browser into the updater.
- Release descriptions are treated as untrusted plain text.
- The rollback boundary is explicit: keep the prior release until health succeeds, then clean up only older releases according to the existing retention behavior.
- The feature is limited to the panel and does not claim router verification in CI.
