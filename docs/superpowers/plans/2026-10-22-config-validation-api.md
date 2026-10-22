# nfqws2 Configuration Validation API Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real Rust API for safe, non-mutating static validation of candidate nfqws2 configurations before apply.

**Architecture:** Validate the candidate with the existing conservative `ConfigDocument` parser, without evaluating shell or starting nfqws2. Return structured diagnostics distinguishing syntax/known package-layout errors from checks that require the installed binary or device. Keep current apply route behavior intact.

**Tech Stack:** Rust, Axum, serde, Tokio test router.

**Spec:** `C:\Users\RaNgeR\.codex\attachments\b4673ee7-d0e4-48c3-b6a7-96d823143618\Вставленный текст.txt`, sections 4, 6, 8 and 9.

## Global Constraints

- Never `source` or `eval` a user configuration.
- Do not claim version compatibility without verifying against the installed nfqws2 binary.
- Return clear warnings when syntax cannot be statically resolved.
- Do not write files or restart service during validation.

## Review Focus

- Executable shell fragments and malformed quotes must be rejected with line diagnostics.
- `--new` placement must follow the audited nfqws2 package layout, not nfqws1 assumptions.
- Dynamic shell variables must be reported as unverified, not expanded or silently accepted as compatible.
- Missing optional variables must not fail a configuration that uses the package's documented defaults.
- Endpoint must be a preview only: no file writes and no service commands.

---

### Task 1: Add static config validation logic and endpoint

**Files:**
- Modify: `backend/src/main.rs`
- Modify: `backend/src/main.rs` test module
- Modify: `backend/src/main.rs` `api_router`

**Interface:** `POST /api/nfqws2/config/validate` accepts `{ "content": string }` and returns `{ "valid": boolean, "diagnostics": [{ "severity": "error"|"warning", "code": string, "message": string, "line": number|null }], "checks": { "parser": "passed"|"failed", "installedBinary": "not_run" } }`.

- [x] Add failing tests for accepted upstream fixture, malformed/executable shell, forbidden `--new` placement, allowed `--new` in `NFQWS_ARGS_CUSTOM`, dynamic variable warning, and no file/service side effects.
- [x] Run those tests and confirm the endpoint is missing or returns the wrong response.
- [x] Implement conservative validation by parsing assignments and inspecting literal supported argument assignments only; do not spawn subprocesses.
- [x] Add the route and enforce the existing config byte limit.
- [x] Run Rust fmt, all tests, Clippy, frontend tests/typecheck/build. Shell scripts were unchanged; the prior v0.1.13 Release workflow's Linux shell checks passed. Local Windows Git-Bash `dash -n` is not a valid substitute here (it fails on existing `scripts/install.sh:60`).
- [x] Update API documentation and completion audit with the exact scope and limitations.

**Review outcome:** This endpoint alone does not fulfill profile CRUD or installed-binary compatibility; those remain separate required tasks.
