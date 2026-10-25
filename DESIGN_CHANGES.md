# v0.6.0 visual audit

## Reference and existing UI

The XKeen UI source uses the variable Inter family for interface text and JetBrains Mono for code. This panel already packages both fonts and uses them in its Tailwind sans/mono stacks, so the audit keeps the same local font files and does not add a network font request. The panel also already has a shared keyboard accessible select, visible focus rings, light-theme select colors, panel transitions, and a reduced-motion mode.

## v0.6.0 adjustments

- Set the local Inter family and native dark/light color scheme on the document root so menus and controls follow the selected theme consistently.
- Set a comfortable shared line height and a 12 px minimum for explanatory and tabular text in every main panel. Small screens receive a little more line spacing.
- Give the existing custom select menu a short entrance transition and translucent depth while retaining its current keyboard, pointer, and selection behavior.
- Disable the select entrance animation when the user requests reduced motion.

## Panel audit

| Area | Existing pattern retained | v0.6.0 check |
| --- | --- | --- |
| Header and service state | Inter labels, semantic status colors, accessible action controls | Start/stop text and icon use a shared state presentation |
| Configuration and list editors | JetBrains Mono, line gutters, syntax coloring | Long values wrap or scroll within their editor; theme contrast remains explicit |
| Diagnosis and strategy picker | Ordered result cards and status tones | Explanations meet the shared minimum text size |
| Monitoring and Telegram | Responsive cards and status bars | Field labels and history remain readable on narrow screens |
| Appearance and backups | Theme/layout cards and grouped settings | Existing light/neon overrides stay scoped to their theme |
| About and release controls | Monospace commands and version pills | Focus, hover, and select states use the same shared interaction language |

## v1.0.5 interface pass — 2026-10-25

- Updated the shared custom select to match the compact dark menu reference: selected values keep a blue highlight, readable white text and a checkmark; hover/focus use the same blue family. Opening and closing animate, and reduced-motion disables the motion.
- Lifecycle toasts now remain deduplicated across their enter, visible and exit periods. The stack is capped at four, accounts for mobile safe areas and can scroll if several long notices need more room. Dismissal is idempotent and animated.
- Delayed action tooltips keep the existing two-second delay, skip touch-only pointers, and clear when the page scrolls or resizes so a tooltip cannot linger over a different control.
- Release notes retain safe Markdown links and formatting; strong text is white in the dark theme and follows the foreground in the light theme. Version action icons use a 3 px alignment adjustment.
- Site diagnosis now includes active host/IP lists used by the running process, in addition to standard lists. Only files that resolve inside the configuration directory are read; symlinks and paths outside it are ignored.
- The panel continues to use its bundled Inter Variable and JetBrains Mono files. No remote font dependency or new UI framework was added.

### Verification

- Frontend: 29 test files, 241 tests passed; TypeScript production build completed.
- Backend: Rust formatting and the full Cargo test suite completed successfully.
- Chromium desktop and mobile captures were reviewed. The local preview has no router backend, so it displays the designed “Нет связи с панелью” state instead of populated panels; this does not verify interactive routes. Firefox’s headless capture remained on the initial loading screen, so its interactive rendering still needs a backend-connected browser session.
- No physical router was available for deployment checks. The release workflow’s aarch64-musl build remains the router-target verification gate.
