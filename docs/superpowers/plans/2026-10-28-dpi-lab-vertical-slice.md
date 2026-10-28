# DPI Lab Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the bounded PCAP → TCP/TLS → Packet Timeline → Byte Inspector → saved experiment vertical slice defined in the spec.

**Architecture:** Add a pure Rust analysis module with strict size limits and no network or Lua execution. Expose it through authenticated Axum routes and UUID-based persistence below the application data directory. Add a first-class TypeScript route that renders only observed packets and deterministic stream reconstruction, retaining the existing UI palette and fonts.

**Tech Stack:** Rust 2024, Axum, Serde, TypeScript, Vite, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-28-dpi-lab-vertical-slice.md`

## Global Constraints

- Target router has 512 MB RAM and two CPU cores.
- Do not execute Lua from imported or user-provided files.
- Do not change working NFQWS2 configuration or start packet capture.
- Reject oversized inputs and bound retained experiments.
- Distinguish observed capture bytes from protocol-derived interpretation and unknowns.
- Do not claim a modeled result is a real send or a provider DPI behavior.
- Preserve existing routes and authentication behavior.

## Review Focus

- Malformed PCAP lengths or hostile count values must be rejected without panic or excessive allocation.
- TCP sequence wrap, retransmissions, gaps, and overlaps must not silently manufacture bytes.
- Truncated TLS extensions and invalid nested lengths must not panic or yield false SNI.
- Experiment IDs must not permit path traversal and storage must stay bounded.
- Selecting/loading files must not expose binary contents in HTML unsafely or break smaller viewports.

---

### Task 1: Bounded classic PCAP and TCP/TLS analysis core

**Files:**
- Create: `backend/src/dpi_lab.rs`
- Modify: `backend/src/lib.rs`
- Test: inline unit tests in `backend/src/dpi_lab.rs`

**Interfaces:**
- Produces `analyze_pcap(name: &str, bytes: &[u8]) -> Result<Analysis, DpiLabError>`.
- `Analysis` serializes source file metadata, packet timeline, flows/directions, byte ranges, TLS ClientHello fields, warnings, and evidence labels.
- TCP overlap policy is first-seen and named in the result.

- [x] Add tests for supported PCAP endian/resolution variants, Ethernet IPv4 TCP extraction, malformed/truncated capture rejection, limits, TCP gap/overlap provenance, sequence wrap, and TLS SNI/extension bounds.
- [x] Run the new focused Rust tests and confirm expected failures during implementation.
- [x] Implement bounded PCAP parsing, deterministic per-direction TCP reassembly, and bounded TLS ClientHello field parser.
- [x] Run focused tests and `cargo fmt --check`.

### Task 2: Authenticated experiment API and bounded storage

**Files:**
- Modify: `backend/Cargo.toml` only if a vetted decoder/storage helper is necessary (prefer standard library implementation).
- Modify: `backend/src/main.rs`
- Test: API tests in `backend/src/main.rs` or a focused backend module test.

**Interfaces:**
- Implements analyze, save, list, and get routes described in the spec.
- Storage path is derived from `AppState.data_dir`; IDs are generated UUIDs and validated on read.

- [x] Add tests for valid analysis API, body/upload bounds, unauthenticated access, save/list/get, invalid IDs, and pruning.
- [x] Run focused backend tests and confirm expected failures during implementation.
- [x] Add handlers and routes under existing auth middleware; store generated UUID names and bounded PCAP captures for deterministic replay.
- [x] Run focused backend tests and `cargo fmt --check`.

### Task 3: First-class DPI Lab UI and user flows

**Files:**
- Modify: `web/src/navigation.ts`
- Modify: `web/src/App.tsx`
- Modify: `web/src/api.ts`
- Modify: `web/src/panel.ts`
- Modify: `web/src/panel.css`
- Test: `web/src/dpi-lab.test.ts` plus navigation coverage.

**Interfaces:**
- Adds route `dpiLab` and API functions typed against the backend DTOs.
- File input reads bytes and sends bounded hex; backend remains the authority on validation.

- [x] Add tests for route registration, byte/evidence formatting, file upload, and navigation.
- [x] Run focused Vitest tests and confirm expected failures during implementation.
- [x] Add responsive UI using existing panel cards, font faces, status colors, and monospace byte view; do not add placeholder tabs for future modes.
- [x] Run focused Vitest tests and TypeScript typecheck.

### Task 4: Documentation, review, and verification

**Files:**
- Create: `docs/DPI_LAB.md`
- Modify: `CHANGELOG.md`
- Modify release metadata already in progress for `v1.11.13` only after slice verification.

- [x] Document studied upstream semantics, provenance labels, API, limits, usage, and deferred scope.
- [ ] Run full `cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, Rust audit, full `npm test`, `npm run typecheck`, and production build.
- [ ] Inspect the full diff against the base release, check all slice requirements, and correct Critical/Important findings before any release.
- [ ] Release only after the slice and all existing release checks are green; do not publish incomplete DPI Lab functionality.
