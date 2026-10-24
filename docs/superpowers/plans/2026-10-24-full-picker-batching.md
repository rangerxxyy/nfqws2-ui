# Full Catalog Batch Execution Plan

> **Execution:** Continue inline in the isolated `codex/full-picker-batching` worktree, following TDD. The user has already requested implementation and router validation within safe nfqws2-only boundaries.

**Goal:** Port the current upstream full-picker batch semantics into Rust: isolated mark/source-port lanes, conservative runtime sizing, real batch probes, and serial confirmation of batch positives.

**Sources:** `zemidala/nfqws2-ui` current `main` at `3ec0fdf6281abe6762b02fb2937173462cc56f81`, `src/www/api.php` (`testRules`, `parSlots`, `parTry`, `testJob`); current project audit in `docs/ZEMIDALA_AUTOPICK_ANALYSIS.md`.

## Invariants

- Never alter the primary nfqws2 process/configuration or its live queue. Test flows are router-originated, target/port scoped, use a dynamically selected free queue, and keep `--queue-bypass`.
- Each candidate lane gets a unique non-overlapping router source-port range and mark; abort batching and fall back to serial if a safe mark range or isolated rules cannot be established.
- Reuse only uniquely owned firewall chains and owned child processes. Cancellation, startup failure, and probe failure must stop children and remove every registered rule/temporary file.
- A batch-positive candidate is preliminary only. Confirm each likely winner serially with the user-selected repeat count before it can be reported as confirmed or applied.
- Keep automatic first-success mode serial. Do not run QUIC candidates through the HTTP/TLS batch path.
- Cap this implementation at three lanes until repeatable device measurements justify changing the cap. Recalculate from live available RAM and load before every batch.

## Tasks

1. **Lane model and deterministic assignment**
   - Add failing tests for disjoint port ranges, mark/mask validation, collision rejection, maximum lane count, and safe candidate-profile assembly.
   - Implement pure lane selection and multi-profile argument construction; verify RED then GREEN.

2. **Owned target-scoped queue rules and batch runner**
   - Add failing tests proving lane marks are applied only to the exact router-originated target tuple, with inbound/outbound port pairing and owned-chain cleanup registration.
   - Implement per-lane mark rules, one selected NFQUEUE/process, bounded concurrent curl lanes, strict timeouts, cancellation and unconditional cleanup; test fallback to serial when the lane setup is unavailable.

3. **Full-mode orchestration and serial confirmation**
   - Add failing tests for solo high-priority candidates, one probe batch per catalog chunk, early stop after failed lane repeats, ordering candidates by batch latency, serial reconfirmation limits, cancellation, and never promoting preliminary results to confirmed.
   - Implement bounded memory/load-based sizing (maximum three for now), progress updates, compact persistence, and sequential fallback.

4. **Router verification and docs**
   - Run Rust fmt/tests/Clippy, frontend tests/typecheck/build and release CI.
   - Build the ARM64 package and, only after read-only queue/process checks, run one bounded target-scoped smoke using a temporary binary; verify queue/process/rule cleanup and primary queue 300 remains active.
   - Record measurements and gaps; publish only after the release package and checksum are verified.
