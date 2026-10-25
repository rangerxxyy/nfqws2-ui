# Raw configuration formatter implementation plan

> **For agentic workers:** Execute inline in this task; keep each step small and verify it before proceeding.

**Goal:** Add a safe “Оформить” action for the raw nfqws2 configuration editor.

**Architecture:** A pure Rust formatter normalizes whitespace only in supported `NFQWS_ARGS*` assignments and rejects ambiguous input. A backend endpoint returns the candidate; the web editor inserts it into the unsaved draft and refreshes its visual aids.

**Tech Stack:** Rust/Axum backend, TypeScript/Vite frontend, existing Vitest and Cargo tests.

**Spec:** Upstream zemidala raw configuration `conf_format` behavior, adapted conservatively to our config editor.

## Global constraints

- Never source or evaluate configuration text.
- Never alter assignment names or argument token order/content.
- Keep formatting in the unsaved editor draft until the user applies it.
- Avoid new runtime dependencies.

## Review focus

- Multiline values and CRLF input retain the same argument sequence.
- Quoted/escaped or malformed values are rejected rather than rewritten ambiguously.
- Non-argument assignments, comments, and unrelated lines remain intact.
- Oversized requests return the existing 512 KiB payload error.
- Formatting is undoable in the editor and does not write to the router.

### Task 1: Pure conservative formatter

**Files:** Create `backend/src/config_format.rs`; modify `backend/src/lib.rs`.

- [ ] Add tests for multiline normalization, comment/unrelated-line preservation, malformed input rejection, and idempotence.
- [ ] Run the targeted Rust tests and confirm they fail before implementation.
- [ ] Implement a conservative formatter that only rewrites safely tokenized quoted `NFQWS_ARGS*` assignments.
- [ ] Run targeted Rust tests and formatter check.

### Task 2: Backend API

**Files:** Modify `backend/src/main.rs`.

- [ ] Add `POST /api/nfqws2/config/format` with the existing config size limit.
- [ ] Test the route for success and malformed input rejection.
- [ ] Run the targeted backend tests.

### Task 3: Editor action

**Files:** Modify `web/src/api.ts`, `web/src/api.test.ts`, `web/src/App.tsx`, and `web/src/panel.ts`.

- [ ] Add API coverage for the format endpoint.
- [ ] Add an “Оформить” button beside existing configuration actions.
- [ ] Replace textarea contents through the editor insertion path, refresh syntax/outline, and state that only the draft changed.
- [ ] Run web tests and production build.

### Task 4: Final review

- [ ] Run backend and frontend verification plus `git diff --check`.
- [ ] Review the final diff for semantic changes beyond whitespace formatting.
