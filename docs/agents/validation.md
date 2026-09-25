# Validation gate and Lean audit policy

Current gate and Lean audit policy, routed by [AGENTS.md](../../AGENTS.md).
Paths in backticks are repository-relative. History behind these requirements is
in the [agent-cycle checkpoint](../development/increments/structure-checkpoints/agent-cycle-guidance.md).

<!-- preserved:validation:start -->
## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # every fresh checkout, worktrees included
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
python3 scripts/check_lean_audits.py
python3 -m unittest discover -s scripts/tests
```

The Python suite reads the committed matrix (`CommittedMatrixIsSelfConsistent`),
so run it after any matrix regeneration, not before.

`--no-fail-fast` is required (`GATE-NO-FAIL-FAST`): cargo's default stops at the
first failing binary, so one of constraint 16's load-sensitive rows would hide
every later target and the printed count would be a lower bound, not a
measurement. A flake must not be able to shrink the suite it is reported against.

A delegated card's worktree is a fresh checkout, so the submodule step is due
every time; a missing `third_party/libevent` surfaces late and reads like a
regression (`WORKTREE-SUBMODULE-UNGATED`). Capture each gate step's exit to a
file and never pipe a gate step, because a pipe reports the last command's exit
(`GATE-EXIT-SWALLOWED-BY-PIPE`). Both residuals are open without a mechanical
check; this instruction is the interim.

The `clang` gate key is a disclosure convention, not a check
(`CLANG-RECORDING-UNBOUND-TO-ITS-RUN`), so a green regeneration is never
evidence that Clang was present for the recorded run. `build_matrix.py`'s
`clang_recording()` requires only that a file named `clang-version.txt` sit in
the gate file's own directory or a `gate/` subdirectory beside it, that the
claim parse as `present: <banner>` / `absent: <banner>`, and that the banner
equal the recording's first line; the `clang-version.exit` arm fires only if
that file exists. Nothing binds the recording to a run, host, time or commit, so
a recording copied from another report directory passes every arm. Run
`clang --version` in the worktree that took the measurement and put the
recording beside that gate file; borrow one only where the gate file's own note
says so in those words (`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`). The honesty is
the measurer's, and both residuals stay open on the mechanical arm.

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `quirin
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan). Provider changes additionally
need `providers/package.py` build + verify (constraint 18).

## Lean CI hygiene

CI builds `proof/lean-toolchain` with Lean and runs every `Audit*.lean` via
`scripts/check_lean_audits.py`, which builds from the pinned proof directory
under finite time limits and cleans up workers on timeout or cancellation.
Parallel audit cancellation synchronizes stopped-state inspection,
child spawn and registration; failures are observed in completion order,
while successful logs are replayed in audit order, and every pending or
running worker is collected before return.
`AuditAll.lean` checks the compiled QCore axiom closure, allowing only
`propext`, `Classical.choice` and `Quot.sound`; stale audit names and warnings
fail the gate, including library-build warnings under the pinned Lean 4.34.0.
The same audit rejects self-equality and exact assumed conclusions in public,
non-internal theorem statements. This narrow check is not a semantic
non-vacuity proof; do not weaken its generated-name boundary or substitute it
for statement review (`docs/reports/lean-proof-integrity/`).
Both reevaluation examples run in Linux and Clang-enabled
Windows CI. Generic Lean names keep historical aliases
(`QCore.DescriptorCustody`, `EventComposition` / `checkEventComposition`,
`busy_effect_preserves_lock`), so the old Monkey module, composition accessors
and ChunkIO lock theorem remain usable. `docs/reports/w7-lean-ci/README.md`,
`docs/reports/w7-generic-lean-names/README.md`,
`docs/reports/lean-build-performance/README.md`.

<!-- preserved:validation:end -->
