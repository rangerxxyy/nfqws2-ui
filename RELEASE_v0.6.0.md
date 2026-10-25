# Release v0.6.0 work log

Starting point: GitHub tag `v0.5.8` (`16cec225f24ab7c6fd9c1dc1ef1e7bb32f5722ee`). Work branch: `release/v0.6.0`.

## Requested changes

- [x] Status bar start/stop icon reflects service state.
- [x] Cross-panel typography, contrast, transitions, and dropdown styling audited.
- [x] Clear, deduplicated stop/restart/UI-update/core-update notifications.
- [x] XKeen-inspired update notice with safe Markdown and visible blue links.
- [x] Configuration Structure enabled initially.
- [x] Updater icons aligned.
- [x] Diagnosis stages listed and shown in order.
- [x] Shared custom tooltips display after 2 seconds.
- [x] Animated accessible checkboxes with reduced-motion support.
- [x] Whole appearance card activates by pointer and keyboard.
- [x] Terminal commands differ from zemidala commands.
- [x] Direct Telegram mode hides interface and proxy settings.
- [x] Strategy picker has explicit per-strategy result labels.

## Verification record

- Local web build: passed with `pnpm build`.
- Full frontend suite: 22 files / 170 tests passed; Rust tests, formatting, and clippy passed.
- Chromium desktop and 390 px mobile screenshot review passed; Firefox headless screenshot review passed.
- Router read-only SSH confirmed both services running and the configured panel port. A loopback health request failed without a response body; no install, restart, or other mutation was attempted.
- GitHub Actions run `40131066783`: passed, including source audit, ARM64 cross-build, package verification and release publication.
- Published release: https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.6.0; includes installer, ARM64 archive and SHA-256 file.

### Block 1: Status bar

- Added a single presentation mapping for service state, action, label, icon, and tone.
- The stopped/unknown initial state shows a play icon; the running state shows a stop icon.
- Verification: `pnpm --dir web exec vitest run src/service-control.test.ts src/panel-template.test.ts` — 61 tests passed; `git diff --check` — passed.

### Block 2: Cross-panel visual system

- Matched XKeen's bundled Inter Variable / JetBrains Mono font stack already in this project; no remote font dependency was added.
- Added a 12 px minimum for main-panel explanatory/table text and looser mobile leading.
- Added a short styled-select entrance and blur, and preserved reduced-motion behavior and light/dark color-scheme selection.
- Audit notes: `DESIGN_CHANGES.md`.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts src/styled-select.test.ts` — 62 tests passed; `pnpm --dir web build` — passed; `git diff --check` — passed.

## Release notes

The release notes are limited to the current v0.6.0 changes. The GitHub release is published with its installer, ARM64 archive, and SHA-256 checksum.

### Final responsive QA

- Mobile navigation now uses a single column with wrapping labels and a scrollable 50 vh region, keeping long section names readable at 390 px.
- Verification: full frontend suite — 170 tests passed; production build — passed; Chromium desktop/mobile and Firefox desktop screenshots reviewed; `git diff --check` — passed.
- Router note: read-only SSH status checks passed. The loopback health endpoint returned no response body, so an HTTP health pass is not claimed; router state was not changed.

### Block 4: Update release notes

- Replaced flattened changelog text with a safe Markdown renderer for headings, emphasis, lists, code, quotes, and HTTPS/HTTP/mail links; raw HTML remains text and unsafe URL schemes are never rendered as links.
- Release cards now use an accessible expandable header and scrollable description, with the newest release expanded on open. Links are blue and emphasized text remains high contrast; light-theme styles are included.
- Verification: `pnpm --dir web exec vitest run src/release-markdown.test.ts src/panel-template.test.ts` — 62 tests passed; `pnpm --dir web build` — passed; `git diff --check` — passed.

### Block 5: Configuration Structure

- The configuration outline now starts visible, and the toggle starts pressed/expanded with matching accessible state. Toggling updates both `aria-expanded` and `aria-pressed` and its action label.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 61 tests passed; `git diff --check` — passed.

### Block 6: Updater alignment

- Nudged the paired nfqws2 and nfqws2-UI updater controls four pixels left at desktop breakpoints while preserving their responsive grouping.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 62 tests passed; `git diff --check` — passed.

### Block 7: Site diagnosis stages

- Added a visible checklist before the router diagnosis request, then replayed the returned per-stage statuses in order with active, passed, failed, and unknown visual states. Results remain tied to backend measurements; the animation runs only after the response arrives.
- Verification: `pnpm --dir web exec vitest run src/diagnosis-stages.test.ts src/panel-template.test.ts` — 65 tests passed; `pnpm --dir web build` — passed; `git diff --check` — passed.

### Block 8: Action tooltips

- Extended the existing 2-second custom tooltip system to every button, including controls without data-action attributes; native titles are removed to prevent duplicate browser popups. Tooltip placement remains viewport-bounded and non-interactive.
- Verification: `pnpm --dir web exec vitest run src/action-tooltips.test.ts src/panel-template.test.ts` — 66 tests passed; `git diff --check` — passed.

### Block 9: Checkbox styling

- Replaced browser-default checkbox visuals with a consistent check mark, focus ring, pressed feedback, and theme-aware colors. Inputs remain native controls for keyboard and screen-reader semantics; motion is disabled when requested by the OS.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 64 tests passed; `pnpm --dir web build` — passed; `git diff --check` — passed.

### Block 10: Appearance choices

- Appearance options already use full-card native `<label>` elements, so clicks anywhere on each card select its radio option and keyboard selection remains native. Added visible hover and focus feedback to make the clickable surface clear.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 65 tests passed; `git diff --check` — passed.

### Block 11: Panel-specific terminal command

- Audited `zemidala/nfqws2-ui` `src/bin/nfqws-ui-setup`: its own commands are `nfqws-ui-setup` with the `nfqws-ui` alias. This project now ships `nfqws2-ui-ctl`, with `update`, `--check`, explicit version/`--force`, `status`, and `help`; it is installed and removed with the panel and does not replace nfqws2's service command.
- Added release packaging and install/update/uninstall hooks plus preflight validation. The release archive now requires the command helper.
- Verification: Windows Git `dash -n` — passed; `tests/nfqws2-ui-ctl-preflight.sh` and `tests/update-helper-preflight.sh` — passed; web template tests — 68 passed; production build — passed; `git diff --check` — passed.

### Block 12: Telegram transport fields

- The network-interface field is shown only for interface mode, and the proxy field only for proxy mode. Both remain hidden for the default «как обычно» route and are synchronized after settings load.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 68 tests passed; production build — passed.

### Block 13: Strategy results

- Every candidate now has a visible confirmation label in addition to its icon, success count, timing, and detail; the result remains per strategy and the winner summary remains separate.
- Verification: `pnpm --dir web exec vitest run src/panel-template.test.ts` — 68 tests passed; production build — passed.

### Block 3: Lifecycle notifications

- Replaced the generic service-action toast with event-specific start/stop/restart labels.
- Added titled, icon-led success/error toasts with live-region semantics and light-theme styling.
- Added a short duplicate suppression window; update success is now shown once after the page reloads.
- Verification: `pnpm --dir web exec vitest run src/notifications.test.ts src/panel-template.test.ts` — 62 tests passed; `pnpm --dir web build` — passed; `git diff --check` — passed.
