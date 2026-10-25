# Release v0.6.1 work log

## Changes

- Reduced spacing in the strategy picker and moved custom strategy input into the mode tile; choosing the custom mode widens that tile.
- Added animated copy confirmation for entered strategies and for the expanded parameters of each strategy result. Clipboard API fallback supports HTTP router interfaces.
- Moved configuration copy/apply actions into the tab row.
- Expanded theme/layout radio hit targets to cover their cards.
- Matched the ASN lookup route and timeouts to the zemidala RIPEstat behavior, and accepted numeric ASNs with an `AS` prefix.

## Verification

- Web tests: 172 passed.
- Web TypeScript typecheck: passed.
- Web production build: passed.
- Full Rust test suite: passed.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: passed.
- Targeted Rust freeze diagnostic tests: 9 passed.
- A single RIPEstat lookup for `1.1.1.1` returned ASN 13335 and HTTP 200.
- Browser UI preview was blocked by the app browser URL policy; no browser automation workaround was used.
- Router was not touched or restarted in this release pass.

Release archive preflight and GitHub Actions run after the release branch is pushed.
