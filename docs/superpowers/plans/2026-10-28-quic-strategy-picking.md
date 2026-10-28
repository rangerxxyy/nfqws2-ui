# QUIC strategy picking implementation plan

Spec: `docs/superpowers/specs/2026-10-28-quic-strategy-picking-design.md`

## Global constraints

- Reuse Rust/TypeScript code and dependencies already in the project.
- Search must not write the active config; candidate probes run one at a time through isolated, owned UDP/NFQUEUE rules.
- Apply only a repeat-confirmed preview using the current revision-aware config transaction.
- Multi-site QUIC, parallel batches, full catalog and refinement remain out of scope.

## Review focus

1. QUIC target is a literal IP: reject before an SNI probe or firewall change.
2. Temporary UDP rules fail partway through installation: remove every owned rule/chain and do not start nfqws2.
3. Cancellation during a probe: stop the candidate, process and temporary rules promptly.
4. QUIC server replies with unrelated data/CID or remains silent: do not count as success.
5. Config has existing custom UDP profiles or stale revision: preview must remain domain-scoped and apply must refuse stale writes.

### Task 1: Strategy input and candidate plan

**Files:** `backend/src/strategy_picker.rs`, `backend/src/strategy_picker` tests, `backend/src/main.rs` request tests, `web/src/App.tsx`, `web/src/panel.ts`, `web/src/panel-template.test.ts`

- [x] Add failing tests that QUIC quick plans include only QUIC-compatible bounded candidates, and QUIC is rejected for IP and multi-site requests.
- [x] Run targeted tests and confirm the expected failures.
- [x] Keep standard/custom/configured/history candidates available; exclude TLS community/refinement/catalog paths for QUIC.
- [x] Add the QUIC option to single-site picker, constrain the UI mode, explain domain/SNI and the reachability signal.
- [x] Run targeted Rust and frontend tests.

### Task 2: Isolated UDP probe mode

**Files:** `backend/src/quic_probe.rs`, `backend/src/site_trace.rs`, `backend/src/site_diagnostics.rs`, `backend/src/site_trace` and `quic_probe` tests

- [x] Add tests for QUIC candidate plans using UDP, direct/candidate QUIC probes, response CID matching and silence handling. Reuse the existing trace runner's cancellation cleanup paths rather than duplicating firewall management.
- [x] Extend the existing site-trace candidate runner for QUIC: direct baseline uses UDP bypass rules; each candidate gets a fresh QUIC Initial and a test-source port matched by the isolated NFQUEUE rules; repeats stay sequential.
- [x] Keep candidate search bounded by the existing quick-plan cap and skip search after a successful direct QUIC response. Do not infer QUIC unavailability from a missing `Alt-Svc` header.
- [x] Run focused backend tests including existing cleanup coverage and full test suite.

### Task 3: Preview and apply for a domain

**Files:** `backend/src/strategy_picker.rs`, `backend/src/main.rs`, `backend/src/main` tests, `web/src/panel.ts`, `web/src/panel-template.test.ts`

- [x] Add failing tests that a confirmed QUIC result previews as a UDP profile scoped to one host in `NFQWS_ARGS_CUSTOM`, while an unconfirmed result remains rejected.
- [x] Run targeted tests and confirm the expected failures.
- [x] Extend domain profile generation and `nfqws2 --dry-run` preparation for QUIC. Keep HTTP/TLS global insertion behavior intact.
- [x] Tailor preview/confirmation copy and post-apply verification to QUIC traffic for the selected site.
- [x] Run targeted frontend/backend tests.

### Task 4: Integration verification and release candidate

**Files:** `web/package.json`, `backend/Cargo.toml`, `CHANGELOG.md`, `RELEASE_v1.7.0.md`, `.github/workflows/*` as required by existing release process

- [x] Run full frontend tests, typecheck, production build, Rust format, full Rust suite, Clippy, `cargo audit`, and `pnpm audit --prod` locally.
- [ ] Let the release workflow run its shell/install preflight and ARM64 cross-build after pushing the release branch.
- [x] Review diff for config mutation, cleanup ownership, candidate caps and unchanged TCP behavior.
- [ ] Publish `v1.7.0` through the existing release branch workflow after its gates pass; verify the release and architecture assets.
- [ ] Record the tested commands and CI run in the release notes.
