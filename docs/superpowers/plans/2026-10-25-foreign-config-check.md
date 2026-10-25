# External nfqws2 Configuration Check

> **For agentic workers:** Execute this plan in order; each task is independently verified.

**Goal:** Compare the active router configuration with up to three supplied nfqws2 `.conf` files against the same small set of user-selected sites, following the useful safety and confirmation behavior found in Zemidala.

**Architecture:** Add an asynchronous diagnostic operation serialized by the existing site-diagnostic lock. Accept config text (including browser-read `.conf` files) or a bounded HTTPS URL; parse literal argument assignments without evaluating shell code, map known Lua/blob names to the installed paths, dry-run each candidate, and run it only with the existing isolated site-trace runner. Confirm each site with up to three sequential requests, stopping after the first failure. Persist statuses and names, never external config contents.

**Tech Stack:** Rust/Axum/Tokio, existing `ConfigDocument`, `site_trace` runner, TypeScript panel.

**Constraints:** At most 3 configurations and 3 target hosts; URL body capped at 64 KiB, HTTPS only, no redirects or credentials, resolve and pin a public IPv4; run checks serially; reject dynamic shell values and Lua init paths not already active; do not alter active config or service.

## Tasks

1. Add parsing and input validation helpers with unit tests: safe TCP argument extraction, literal-only enforcement, installed Lua/blob path mapping, bounded HTTPS URL parser and pinned fetch.
2. Add operation/result types, status persistence, cancellation, and a serial worker that checks active config and each candidate on the same hosts using the diagnostic lock and existing runner.
3. Add a compact Diagnostics navigation item and panel with text, URL, and `.conf` file inputs, host list, progress, cancellation, and per-site comparison rows.
4. Add API/panel regression tests, document the limits, then run Rust and frontend verification and inspect the local panel.

## Review Focus

- Host list rejects private targets and caps at three entries.
- URL rejects userinfo, non-HTTPS, non-443 ports, redirects, and private DNS answers.
- Trailing-dot URL authorities are rejected so the URL host and DNS pin cannot diverge.
- Each candidate is confirmed with up to three serial requests and stops after the first failed or inconclusive request.
- Config values with shell expansion or new Lua init paths are rejected before dry-run.
- Cancellation preserves completed comparisons and the operation never writes the active config.
