# Auto-pick Strategy Refinement Implementation Plan

> **For agentic workers:** Execute inline with `superpowers:executing-plans`. Use TDD for every behavior change and keep the work isolated in the `codex/strategy-refinement` worktree.

**Goal:** Port the bounded fake-strategy refinement phase from the audited upstream picker into the Rust strategy run, with an opt-in “try easier variant” control and truthful progress/results.

**Architecture:** Keep candidate parsing and one-axis variant generation in `strategy_picker.rs`. Run refinement only after the ordinary real-probe pass, selecting at most two attempted fake candidates when no candidate is confirmed, or the best confirmed fake candidate when the user explicitly forces refinement. Reuse the existing isolated probe callback, cancellation token, persistence, and UI result rendering; never apply the refined result automatically.

**Tech Stack:** Rust/Tokio/Axum, TypeScript UI template and API client, Vitest.

**Spec:** `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`, especially `testRefine()` and `REFINE_FOOL`, `REFINE_REPEATS`, `REFINE_NAMES`, plus the active autopick goal.

## Global Constraints

- Keep nfqws2-ui's existing TypeScript + Rust architecture and visual system.
- Validate every generated desync step against the installed Lua function and blob inventory before probing.
- Use only the existing isolated network test runner; do not change the primary nfqws2 process or firewall setup.
- Cancellation must stop between refinement probes and preserve completed results.
- A refined strategy is only a candidate; config changes still require the existing preview and explicit apply flow.
- Keep full-catalog execution sequential until router RAM/CPU measurements and independent lane isolation justify parallelism.

## Review Focus

- A direct-open baseline must still short-circuit before refinement.
- A partially successful or failed fake candidate must not be labeled confirmed.
- Force-refinement must not refine QUIC or a non-fake candidate.
- Missing clone Lua/blob support must omit clone variants instead of generating invalid candidates.
- Cancellation and the 300-second refinement deadline must leave the result state bounded and truthful.

---

### Task 1: Safe refinement candidate construction

**Files:**
- Modify: `backend/src/strategy_picker.rs`
- Test: `backend/src/strategy_picker.rs` unit tests

**Interfaces:**
- Consume the existing `StrategyCandidate` and `StrategyProtocol` types.
- Produce bounded axis candidates retaining one-axis-at-a-time changes and exact original step order.

- [ ] Write tests for the upstream fooling/repeat/name variants, step preservation, HTTP exclusions, duplicate elimination, and installed Lua/blob filtering.
- [ ] Run the focused tests and verify they fail because refinement generation is absent.
- [ ] Implement the pure candidate parser/mutator and variant builder, following the audited upstream value order.
- [ ] Run focused tests and then all strategy-picker tests.

### Task 2: Result-driven refinement execution

**Files:**
- Modify: `backend/src/strategy_picker.rs`
- Modify: `backend/src/main.rs`
- Test: `backend/src/strategy_picker.rs` and `backend/src/main.rs`

**Interfaces:**
- Consume the existing real-probe callback, requested repeat count, cancellation flag, and initial `StrategyRunResult`.
- Produce bounded refined result rows, truthful changing progress totals, and a terminal state that respects cancellation/deadline.

- [ ] Write tests that prove refinement starts only after no confirmed result unless forced, chooses at most two fake bases, chooses by success count rather than timing, and stops at the deadline/cancellation.
- [ ] Run focused tests and verify the expected failing assertions.
- [ ] Implement the sequential refinement state machine; never reduce the configured confirmation count for a strategy to be marked confirmed.
- [ ] Run focused Rust tests, then all Rust targets and Clippy.

### Task 3: Picker control and result presentation

**Files:**
- Modify: `backend/src/main.rs` request model and operation progress
- Modify: `web/src/api.ts`
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`
- Test: `web/src/api.test.ts`, `web/src/panel-template.test.ts`

- [ ] Add the default-off force-refinement checkbox and concise explanation of its bounded extra probes.
- [ ] Add request serialization coverage and assert refined candidates are visibly labeled.
- [ ] Run frontend tests to confirm the new assertions fail before implementation.
- [ ] Wire the control, progress stage and refined-result label to the real backend response.
- [ ] Run frontend tests, typecheck, production build and production dependency audit.

### Task 4: Full verification and documentation

**Files:**
- Modify: `docs/AUTOPICK_PROGRESS.md`
- Modify if needed: `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`

- [ ] Run format, all Rust tests, Clippy, host release build and ARM64 musl checks.
- [ ] Review the complete diff for unsafe shell construction, unbounded output, or implicit config application.
- [ ] Record what was ported and remaining router/runtime gaps. Do not claim device validation until observed.
