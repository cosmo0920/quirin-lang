# Validation gate and Lean audit policy

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

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

**The Python suite is listed as of the wave-37 close** (`DOCS-COMPACT-7` §7.3
item 3): every close since wave 32 had run `scripts/tests` and published its
count in the merged-gate row while this block omitted it, so the block now
names what the gate measures. The suite reads the committed matrix
(`CommittedMatrixIsSelfConsistent`), so it runs **after** any matrix
regeneration, not before.

**`--no-fail-fast` is part of the gate as of the wave-26 close (`GATE-NO-FAIL-FAST`,
a deliberate strengthening):** cargo's default stops at the first failing binary,
so one of constraint 16's load-sensitive rows silently hides every target
after it in crate order — a wave-26 card's first run ended at
`qcore-verifier-host` and never reached the `qcorec` crate at all,
`examples_cli.rs` included, making the number that run printed a lower bound
rather than a measurement. A flake must not be able to shrink the suite it is
reported against.

A delegated card's worktree is a **fresh checkout**, so the submodule step above
is due every time and nothing forces it: wave 33 lost two runs to a missing
`third_party/libevent`, whose absence surfaces late and reads like a regression
(`WORKTREE-SUBMODULE-UNGATED`). Separately, a gate step's exit must be
**captured, not piped** — a wave-35 merge reached origin with the Python suite
red because a pipe returned `tail`'s exit status, twice in one wave
(`GATE-EXIT-SWALLOWED-BY-PIPE`). Both are registered residuals whose mechanical
discharge is open; the instruction here is the interim.

**The `clang` gate key is a disclosure convention, not a check** — recorded at
the wave-40 close on `STABILITY-REEVAL-4`'s measurement
(`CLANG-RECORDING-UNBOUND-TO-ITS-RUN`), so a green regeneration is never read as
evidence that Clang was present for the run being recorded.
`build_matrix.py`'s `clang_recording()` requires only that a file named
`clang-version.txt` sit in the gate file's own directory or a `gate/`
subdirectory beside it, that the claim parse as `present: <banner>` /
`absent: <banner>`, and that the banner equal the recording's **first line**;
the `clang-version.exit` arm fires only **if that file happens to exist**. There
is no re-invocation, no host identity, no timestamp and no commit binding, so a
recording copied from any other report directory passes every arm. **Run
`clang --version` in the worktree that took the measurement and put the
recording beside that gate file**; borrowing one is permitted only where the
gate file's own note says so in those words
(`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`). The honesty is the measurer's, and
both residuals stay open on the mechanical arm.

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
