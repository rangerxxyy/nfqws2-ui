# DPI Lab Human-Friendly Redesign Implementation Plan

> **For agentic workers:** Native implementation in the current release branch; execute task by task with tests.

**Goal:** Make PCAP results understandable at a glance while preserving Rust-backed expert analysis.

**Architecture:** Keep the Rust PCAP schema unchanged. Build a pure TypeScript overview from summary findings, observed packets, and reconstructed flows. Toggle existing UI sections by detail level without re-running analysis.

**Tech Stack:** TypeScript, React markup, existing CSS/Tailwind, Vitest, current Rust PCAP API.

**Spec:** User request in the conversation, “DPI LAB HUMAN-FRIENDLY REDESIGN”.

## Global Constraints

- Simple level is selected after each analysis.
- Never infer website availability, NFQWS2 participation, Lua execution, TLS negotiation, or strategy improvement without evidence.
- Detail switching reuses the existing analysis result.
- Preserve packet, hex, sequence, and reassembly expert tools.
- Do not add dependencies or claim unmeasured device performance.

## Review Focus

- Capture without SYN or with partial handshake: incomplete evidence, not connection failure.
- ClientHello without ServerHello: client hello observed, TLS negotiation unknown.
- Multiple flows and SNI: show observed values without inferring site outcome.
- Gaps, conflicts, and truncation: preserve exact counts and reconstruction caveat.
- Unsupported/empty frames: show warnings, no success verdict.

---

### Task 1: Evidence-based overview

**Files:** `web/src/dpi-lab.ts`, `web/src/dpi-lab.test.ts`

- [x] Test confirmed and partial evidence, SNI, gaps, and unknown HTTP/TLS states.
- [x] Implement pure `dpiHumanOverview(analysis)` with evidence references.

### Task 2: Progressive detail levels

**Files:** `web/src/App.tsx`, `web/src/panel.ts`, `web/src/panel.css`, `web/src/panel-template.test.ts`

- [x] Add concise summary and evidence links in simple mode.
- [x] Gate findings, limitations, journey, correlation, and timeline events behind advanced mode.
- [x] Preserve packet timeline and bytes in expert mode; switch without re-analysis.
- [x] Verify targeted tests, complete frontend suite, frontend build, and Rust tests.

### Task 3: Release review

- [x] Review `git diff --check` and changed files.
- [ ] Record hardware/mobile visual limits honestly.
