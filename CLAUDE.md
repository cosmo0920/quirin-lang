# Claude Code project notes

@AGENTS.md

`AGENTS.md` is the source of truth: work status, ledger, invariants, per-change
gate, entry points, Lean CI hygiene and diagnostics namespaces. Read its
**standing subsystem constraints** — the "when touching X, do not Y" inventory —
before editing the C gateway, the compile-task transport, the diagnostics
tables, the concurrency routes or the provider host. Per-increment status
history lives in `docs/development/increments/`, reached through
`DEVELOPMENT.md#increment-records`. Follow `AGENTS.md`'s documentation-retrieval
strategy: the root development guide is a table of contents, and setup,
evidence and history pages are read on demand. This file carries only
the operational guidance that is in neither of those.

## Working rules

- **Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass** (`AGENTS.md` invariants 1 and 5). Where a
  pinned table — contract corpus counts, submodule revision, archived report,
  CT manifest — conflicts with your change, update the pin *deliberately* and
  say so in the commit message and report.
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
- **New status text goes in one place** — one paragraph, newest first, in the
  current record file named by `docs/development/increments/README.md`, plus
  `AGENTS.md`'s headline numbers, per that file's sync rule. Update the index
  when a new wave file opens. Do not re-grow a per-increment chronicle here
  or in the root development table of contents.
- **Compact these three files every 2–3 waves.** `AGENTS.md`, `CLAUDE.md` and
  `README.md` accrete: constraints restate their card reports, sections drift
  apart, README falls behind the ledger. A recurring compaction card
  (DOCS-COMPACT 2026-09-10, DOCS-COMPACT-2 2026-09-11, DOCS-COMPACT-3
  2026-09-12 after wave 26, DOCS-COMPACT-4 2026-09-13 after wave 29,
  DOCS-COMPACT-5 2026-09-13 after wave 31, DOCS-COMPACT-6 2026-09-17 in
  wave 34; next due around wave 36) re-reads every headline
  number **from its source and never from the file being compacted**,
  inventories the standing constraints before and
  after, and merges redundancy. Compaction is **not** deletion: no constraint,
  hard invariant or evidence pointer may be dropped, and a fact that lives only
  in these files moves verbatim into the development history indexed by
  `DEVELOPMENT.md`, rather than disappearing. Historical copies are labelled
  as checkpoints; they do not become a second current rule authority.
  Expect single-digit percentages: two passes in, these files are references
  rather than chronicles, and the rule exists to bound accretion, not to
  promise a shrinking file. **A pass may legitimately grow a file**, and
  `DOCS-COMPACT-6` did: the defects `STABILITY-REEVAL` #1 found were
  *understatements* — a conditional theorem presented as unconditional, a
  premise list missing its two largest members, a mutation count inflated — and
  correcting an understatement costs words. Compaction bounds accretion of
  *narrative*; it does not licence trading accuracy for bytes.
- **Re-evaluate stability every 2 waves** (user direction, 2026-09-13, after
  wave 31). Green tests are an *observation*, and an observation is not a
  proof: a recurring **`STABILITY-REEVAL`** card (#1 ran in wave 34,
  `docs/reports/stability-reeval-1/README.md`; next due after wave 35)
  asks, of the claims this project currently rests on, **under what conditions
  each is unstable or unproven** — and it reports that as a finding rather
  than as reassurance. The method, in the order that has caught the most:
  1. **Hunt vacuous greenness.** For every fail-closed arm added or changed in
     the last two waves, ask whether any test actually *drives* it. An arm no
     test reaches is not verified by a green suite; the suite is evidence
     about the paths it did take. Measure reachability (who calls the
     function; does any fixture produce the refused shape) rather than
     inferring it from the diff. The occasioning case:
     `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, where a semantic union
     resolved at integration was "verified" by two green suites that never
     touched the arm in question. That residual was **discharged** by wave 32's
     `TRACE-ALPHABET-LEASE-BOUNDARY`; it is named here as the case that
     occasioned the cadence, not as open work. The cadence earned itself on it
     twice over — `STABILITY-REEVAL` #1 found `AGENTS.md`'s CT row still
     describing the arm as unproved two closes after its own constraint 15n
     recorded the discharge, and found the discharging card's mutation evidence
     overstated in that constraint at five tests where the measurement is
     three. Both were repaired by `DOCS-COMPACT-6`.
  2. **Separate the proved side from the observed side of a boundary.** A
     theorem about one arm (CT03's write conservativity) says nothing about
     its sibling arm (the lease refusal). Where a boundary is proved on one
     side and observed on the other, say so in those words.
  3. **Find semantic edits made without a card.** An integrator resolving a
     merge, a hotfix, a "mechanical" rename: each can move meaning with no
     report, no Lean parity and no residual name (invariant 6). These are the
     hardest to see later precisely because nothing indexes them.
  4. **Ask what a later card would do to the claim.** A rule that is correct
     today because a construct is unreachable becomes load-bearing the moment
     a queued card makes it reachable — name that card in the finding.
  Every finding lands as a **named residual with a discharge condition** (prove
  it, exercise it, or state it as a measured boundary). Postponement is a valid
  outcome and must be recorded as one; what is forbidden is letting an
  observation stand where the text implies a proof.
- **Work every card's integrator reconciliation list at the wave close, and
  leave the record that you did** (card `W32-RECONCILE-SWEEP`, wave 32,
  discharging `DOCS-COMPACT-5-RECONCILIATION-LIST`). Cards close with a section
  addressed to the integrator — "Integrator reconciliation list", "For the
  integrator", "Integrator carry list", "Proposed amendment text for standing
  constraint N", "Proposed proof-stage evidence (the integrator's flip)". Three
  consecutive closes (waves 29, 30, 31) applied those sections' **headline
  numbers** and left the rest, costing one entire unlanded standing-constraint
  amendment. Two prior findings asked for more care and were skipped, because
  advice is skippable; the step is therefore mechanical:

  ```sh
  python3 scripts/reconciliation_sweep.py . --since <previous close> \
      --check docs/reports/wave<N>-close/reconciliation.json
  python3 scripts/reconciliation_sweep.py . --cards <the wave's cards> --residuals
  ```

  The close **writes** `docs/reports/wave<N>-close/reconciliation.json` beside
  its `gate.json`: one entry per swept item, verdict `applied` / `declined` /
  `superseded` / `deferred` / `noted`, every non-`applied` verdict carrying its
  ground, and a `no_section` reason for any swept card the sweep found nothing
  in (checked both ways, so it cannot wave away a card that does have one).
  `--residuals` lists the residual names the wave's reports introduced that
  reach no tracker, which is the same defect's other half — `DOCS-COMPACT-4`'s
  and `DOCS-COMPACT-5`'s own findings about this process were themselves in
  that untracked set for two waves. **The tool decides nothing.** A green
  `--check` means every item carries a recorded human verdict, never that any
  item was applied; only a person can measure the tree against a paragraph of
  English, and a reconciliation record that says `applied` without saying where
  is worth no more than silence. An amendment lands **in the words the card
  proposed** or is recorded `declined` with its ground — never silently
  softened while being "reconciled" (invariant 1).

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
  architecture. `QUIRIN-ARM64-GATE` and `QUIRIN-ARM64-RUN` were carried here as
  "candidate residual names" for three waves and reached no tracker; both are
  **registered in `named_followups`** as of `DOCS-COMPACT-6` (wave 34), each
  with its own discharge condition.
- **Windows:** native C11 thread subset and MSVC/Clang fixtures —
  `docs/reports/windows-c11-threads/README.md`,
  `docs/reports/windows-vs2026/README.md`,
  `docs/reports/windows-clang-fixtures/README.md`. Visual Studio Clang
  discovery orders vswhere newest first with VS 2026/2022 fallbacks.
  - **`SOCKET-V1-WINDOWS-RUN` discharged (user-validated, 2026-09-12):** both
    published `source.socket.v1` examples compiled by Microsoft's actual
    `cl.exe` (Visual Studio 18 Community, MSVC 19.51.36248), executed against
    Winsock loopback, compared byte for byte with the checked reference
    observation, the complete 49-test `std_socket` target passing —
    `docs/reports/socket-v1-windows-run/README.md`. Scope: native execution and
    byte agreement, **no sanitizer claim** (MSVC ships no ASan/UBSan in this
    harness).
  - Open, each with an exact test-name discharge condition:
    `HTTP-V1-WINDOWS-RUN` and `STRING-SLICE-IO-WINDOWS-RUN`
    (`docs/reports/windows-native-sockets-harness/README.md`), and
    `SOCKET-INET-WINDOWS-RUN` on the same footing — the two declared-address
    examples' MSVC rows are written and wired through
    `tests/support/windows_c.rs::run_msvc`, no such run has happened, so no
    claim is made (`docs/reports/socket-inet-addresses/README.md`).
    `SLICE-PERMISSIONS-WINDOWS-RUN` joins them (wave 27): the range-lending
    card's one native test ran the **Linux** arm only — the emitted difference
    is one pointer displacement inside an existing initialiser and the
    `windows_c` path is carried unchanged, but no Windows or Apple run was
    performed and none is claimed
    (`docs/reports/ct07-slice-permissions/README.md`).
    `WRITE-LEND-WINDOWS-RUN` is its wave-29 sibling on the same footing: the
    write-lend card's native test ran the Linux arm only, the `windows_c` path
    carried unchanged, no Windows run claimed
    (`docs/reports/slice-permissions-write-lend/README.md` §13), and wave 30's
    `CT07-WRITE-REMAP` left it unchanged. `WRITE-LEND-TASKS-WINDOWS-RUN`
    (wave 31) joins the pair for the task-route range write lease, on the
    `STRING-CONCAT-WINDOWS-RUN` precedent: the native test ran the Linux arm,
    macOS compiles the same test file, no MSVC run has happened and none is
    claimed (`docs/reports/write-lend-tasks/README.md` §9). `WRITE-LEND-WINDOWS-RUN`
    and `WRITE-LEND-TASKS-WINDOWS-RUN` were the two members of this family that
    `STD-MODULES-O`'s roll-up did not mention and that reached no tracker at
    all; both are **registered in `named_followups`** as of `DOCS-COMPACT-6`
    (wave 34).
  - Four tests keep `runs_sanitized` in their names while their Windows arm
    claims no sanitizer coverage (`WINDOWS-RUN-TEST-NAME-SANITIZED`).
- **macOS:** pthread-backed C11 thread subset, Apple ASan/UBSan (LSan is
  Linux-only, so leak detection stays a Linux observation), and Apple Clang
  option transport — `docs/reports/macos-cycle19-compiler-options/README.md`.
  Configured-audit inputs compare canonical existing paths so the
  `/var` → `/private/var` alias cannot invalidate a pinned task.
  - **`EVENT-V1-APPLE-RUN` discharged (user-validated, 2026-09-12):** the
    `std_event` suite, including its `cfg(not(windows))` native comparison, ran
    green on the user's macOS environment — the exact run that residual named
    as its discharge. Scope: Apple ASan/UBSan and byte comparison; no LSan
    claim.
  - Open: `SOCKET-V1-APPLE-RUN`, until a macOS run at or after the socket card
    (`47712702`) covers `std_socket`'s native tests, and `SOCKET-INET-APPLE-RUN`
    beside it for the declared-address examples
    (`docs/reports/socket-inet-addresses/README.md`).
- **Linux:** the sanitizer, mapping (`MAP_FIXED_NOREPLACE`) and OS-endpoint
  observations are Linux-only by scope, not by omission.

## Pointer

Any supplied archive review is older than the merged W2–W5 implementations;
read `docs/reviews/r0_3/acceptance.json` for current repair status. Every other
pointer an agent needs — spec, audit setup, entry points, editor support — is in
`AGENTS.md`.
