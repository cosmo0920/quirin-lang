# Claude Code project notes

@AGENTS.md

`AGENTS.md` is the source of truth and carries the current work status, the
compact ledger, the six hard invariants, the **standing subsystem constraints**
(the "when touching X, do not Y" inventory — read that section before editing
the C gateway, the compile-task transport, the diagnostics tables, the
concurrency routes or the provider host), the per-change gate, the entry points,
the Lean CI hygiene rules and the diagnostics namespaces. Per-increment status
history lives in `DEVELOPMENT.md`'s "Increment records". This file carries only
the operational guidance that is in neither of those.

## Working rules

- **Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass.** If a pinned table (contract corpus
  counts, submodule revision, archived report, CT manifest) conflicts with your
  change, update the pin *deliberately* and say so in the commit message and
  report.
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
- **New status text goes in one place** — one paragraph in `DEVELOPMENT.md`'s
  "Increment records", plus `AGENTS.md`'s headline numbers. That is the sync
  rule stated in `AGENTS.md`; do not re-grow a per-increment chronicle here.
- **Compact these three files every 2–3 waves.** `AGENTS.md`, `CLAUDE.md` and
  `README.md` accrete: constraints restate their card reports, sections drift
  apart, README falls behind the ledger. A recurring compaction card (precedent:
  DOCS-COMPACT 2026-09-10, DOCS-COMPACT-2 2026-09-11) re-reads every headline
  number from its source, inventories the standing constraints before and after,
  and merges redundancy. Compaction is **not** deletion: no constraint, hard
  invariant or evidence pointer may be dropped, and a fact that lives only in
  these files moves to `DEVELOPMENT.md` verbatim rather than disappearing.

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

This is the home for per-platform observations; `AGENTS.md` keeps the toolchain
requirements that are part of the gate.

- **Cross-platform build (user-validated, 2026-09-11):** the codebase builds on
  Linux, macOS and Windows with a small number of recorded limitations (the
  per-platform residual lists in the reports below), and **ARM64 Windows**
  starts the `quirin` compiler and compiles emitted C sources. The ARM64
  observation is the user's own environment report — compiler runs and emitted C
  builds; no test-suite, sanitizer or byte-comparison claim is made for that
  architecture (candidate residual names: `QUIRIN-ARM64-GATE`,
  `QUIRIN-ARM64-RUN`).
- **Windows:** native C11 thread subset and MSVC/Clang fixtures —
  `docs/reports/windows-c11-threads/README.md`,
  `docs/reports/windows-vs2026/README.md`,
  `docs/reports/windows-clang-fixtures/README.md`. Visual Studio Clang
  discovery orders vswhere newest first with VS 2026/2022 fallbacks.
- **macOS:** pthread-backed C11 thread subset, Apple ASan/UBSan (LSan is
  Linux-only, so leak detection stays a Linux observation), and Apple Clang
  option transport — `docs/reports/macos-cycle19-compiler-options/README.md`.
  Configured-audit inputs compare canonical existing paths so the
  `/var` → `/private/var` alias cannot invalidate a pinned task.
  **`EVENT-V1-APPLE-RUN` is discharged (user-validated, 2026-09-12):** the
  `std_event` suite, including its `cfg(not(windows))` native comparison,
  ran green on the user's macOS environment — the exact run that residual
  named as its discharge. Scope: Apple ASan/UBSan and byte comparison; no
  LSan claim. `SOCKET-V1-APPLE-RUN` remains open until a macOS run at or
  after the socket card (`47712702`) covers `std_socket`'s native tests.
- **Linux:** the sanitizer, mapping (`MAP_FIXED_NOREPLACE`) and OS-endpoint
  observations are Linux-only by scope, not by omission.

## Pointer

Any supplied archive review is older than the merged W2–W5 implementations;
read `docs/reviews/r0_3/acceptance.json` for current repair status. Every other
pointer an agent needs — spec, audit setup, entry points, editor support — is in
`AGENTS.md`.
