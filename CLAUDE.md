# Claude Code project notes

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

@AGENTS.md

`AGENTS.md` is the source of truth: the compact status ledger, the six hard
invariants, the **standing subsystem constraints** (the "when touching X, do
not Y" inventory — read that section before editing the C gateway, the
compile-task transport, the diagnostics tables, the concurrency routes or the
provider host), the per-change gate, the entry points and the diagnostics
namespaces all live there. Per-increment status history lives in
`DEVELOPMENT.md`'s "Increment records". This file carries only the operational
guidance for working in this repository that is not in either of those.

## Working rules

- **Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass.** If a pinned table (contract corpus
  counts, submodule revision, archived report, CT manifest) conflicts with your
  change, the change must update the pin *deliberately* and say so in the
  commit message and report.
- **Status language matters.** Keep the Safe/Unsafe/Unknown verdict vocabulary
  and the "unpromoted" phrasing exactly as defined in `AGENTS.md` when editing
  docs, reports or commit messages. A provisional `Unsafe` is never an upstream
  defect claim; a grown denominator is not new conformance.
- **Run the full standing per-change gate from `AGENTS.md`** before reporting a
  task complete, and report probe/example exit codes honestly — a nonzero probe
  exit is a finding, not a build failure.
- **Land code in the target crate.** Before starting checker/architecture work,
  read `docs/GENERALIZATION_PLAN.md` and follow its crate layering (§3) and
  invariants (§2); do not add new code to the monolithic `compiler/` grab-bag
  once a target crate exists.
- **Close a card with Rust plus audited Lean.** A card closes only with a Rust
  implementation and matching audited Lean modules registered in an
  `Audit*.lean`; Python artifacts are supplementary evidence, never the close.
- **New status text goes in one place.** Add one paragraph to `DEVELOPMENT.md`'s
  "Increment records" and update `AGENTS.md`'s headline numbers only. Do not
  re-grow a per-increment chronicle in `AGENTS.md` or here.

## Worktree and measurement hygiene

- **Prefer temporary WIP commits over `git stash`.** The shared git stash stack
  is not per-worktree; it caused one cross-worktree stash consumption during
  the second delegated wave (recovered from dangling commits, nothing lost).
- **Measurement outputs belong in worktree-private directories** — `measure/`
  or `private/`, both gitignored — not the shared session scratchpad and **not
  under `target/`**. Delegated agents share the scratchpad, and a mid-run
  overwrite was caught and re-measured during wave 15; a wave-17 card lost a
  whole providers run to its own `cargo clean` because the log lived under
  `target/`.
- Delegated cards work in isolated worktrees, merge one at a time under the
  failure-gated chain discipline, and close with a per-card report under
  `docs/reports/<card>/README.md`.

## Platform records

- **Cross-platform build (user-validated, 2026-09-11):** the codebase
  builds on Linux, macOS and Windows with a small number of recorded
  limitations (the per-platform residual lists in the reports below), and
  **ARM64 Windows** starts the `quirin` compiler and compiles emitted C
  sources. The ARM64 observation is the user's own environment report —
  compiler runs and emitted C builds; no test-suite, sanitizer or
  byte-comparison claim is made for that architecture (candidate residual
  names: `QUIRIN-ARM64-GATE`, `QUIRIN-ARM64-RUN`).
- **Windows:** native C11 thread subset and MSVC/Clang fixtures —
  `docs/reports/windows-c11-threads/README.md`,
  `docs/reports/windows-vs2026/README.md`,
  `docs/reports/windows-clang-fixtures/README.md`. Visual Studio Clang
  discovery orders vswhere newest first with VS 2026/2022 fallbacks;
  `QCORE_CLANG` overrides it and an invalid value is an error, never a silent
  fallback.
- **macOS:** pthread-backed C11 thread subset, Apple ASan/UBSan (LSan is
  Linux-only, so leak detection stays a Linux observation), and Apple Clang
  option transport — `docs/reports/macos-cycle19-compiler-options/README.md`.
  Configured-audit inputs compare canonical existing paths so the
  `/var` → `/private/var` alias cannot invalidate a pinned task.
- **Linux:** the sanitizer, mapping (`MAP_FIXED_NOREPLACE`) and OS-endpoint
  observations are Linux-only by scope, not by omission.
- Native threading examples: `examples/threads/README.md` explains `cl.exe`
  compilation; `build-windows.ps1 -Run` emits, builds and runs the
  copied-worker, owned-buffer and shared-reader examples into
  `target/thread-examples/`.

## Pointers

- Persistent audit setup: `docs/CYCLE19_BUILD_INPUTS.md` (replaces ephemeral
  `/tmp` build paths).
- Current normative specification: `docs/QCORE_SPEC_r0_3.md` and its contract
  companion; follow `docs/reviews/r0_3/acceptance.json` for repair status. Any
  supplied archive review is older than the merged W2–W5 implementations.
- Emacs source/contract highlighting: `editors/emacs/qcore-mode.el`, with
  installation and ERT checks in `editors/emacs/README.md`. This is lexical
  editor support, not a change to language conformance or verification.
