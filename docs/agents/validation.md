# Validation gate and Lean audit policy

Current gate and Lean audit policy, routed by [AGENTS.md](../../AGENTS.md).
Paths in backticks are repository-relative. History behind these requirements is
in the [agent-cycle checkpoint](../development/increments/structure-checkpoints/agent-cycle-guidance.md).

<!-- preserved:validation:start -->
## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # every fresh checkout, worktrees included
python3 scripts/run_test_binaries.py      # parallel form of: cargo test --workspace --no-fail-fast
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

The Rust suite runs through `scripts/run_test_binaries.py` as of wave 44
(`W44-TEST-SUITE-WALL-TIME`); `cargo test --workspace --no-fail-fast` stays
the equivalent reference command and the arbiter when the two disagree.
`cargo test` runs its test executables one after another; the runner builds
exactly what that command builds (`--no-run`), records cargo's own launch of
every test executable (argv, cwd = package root and the complete environment,
through a capturing `target.<host>.runner`, so nothing is recomputed), then runs
them several at a time, longest first (`--jobs`, default half the CPUs), with
one `cargo test --doc` unit for the doc-tests. Binaries in its `EXCLUSIVE`
table — constraint 16's load-sensitive rows and every other binary that
launches a process provider under a declared timeout — run afterwards, one at
a time, with nothing else of the runner's running; a stale entry fails the run.
It is fail-closed like `--no-fail-fast`: every unit runs, and a unit passes
only on exit 0 with at least one libtest summary, every summary clean; a build
failure, a crash or signal, a missing executable, a built-but-unlaunched binary
or an unparsable result fails the run (exit 101). Report its totals line
(`test binaries: N units, M summaries; P passed; F failed; I ignored; ...`),
which sums the same summaries `cargo test` prints, and its log directory
(`private/test-binaries/<stamp>/`, one stdout/stderr log per unit plus
`summary.json`). Constraint 16's instruction is unchanged: a load-sensitive row
that fails is verified in isolation, never weakened. The provider workspace is
not covered — its conformance binaries launch process providers throughout —
and keeps plain `cargo test`; the runner refuses any workspace but the root.
[Report](../reports/w44-test-suite-wall-time/README.md).

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
fail the gate, including library-build warnings under the pinned Lean 4.35.0.
Before any build or audit the runner compares `lake env lean --version`, run
inside `proof`, with `proof/lean-toolchain` and fails naming both versions when
they differ, so an elan override (`ELAN_TOOLCHAIN`, `+toolchain`), a non-elan
`lake` or a moved `stable` default never checks the proofs under another
parser. A pin move builds and audits under both versions and diffs the audit
lists. Since 4.35, `given`, `requires` and `ensures` are forbidden in the
binders and type of every `optDeclSig` signature (`def`, `abbrev`, `example`,
`inductive`/`structure` headers, constructors, structure fields: the def
contract clauses), and `recall` and `monotonicity_by` are keywords everywhere;
use none of the five as a name anywhere
(`docs/reports/lean-4-35-toolchain/`).
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

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-29"></a>

## Tracked test inputs

29. **A test discovers repository inputs from tracked files only.** A test or
    test helper that finds its inputs by walking the repository — a census of
    `examples/`, the contract roots, `compiler/tests/fixtures/`, `proof/QCore/`,
    source directories or provider crates — lists them with
    `tests/support/tracked_files.rs` (Rust, included with `#[path]`) or
    `scripts/tests/tracked_files.py` (Python), which read `git ls-files`, never
    with `read_dir` recursion, `WalkDir`, `os.walk`, `glob`/`rglob` or `find`
    over the working tree. Ignored and untracked content (`private/`, `build/`,
    `target/`, exports, other worktrees) must never change a result: on
    2026-09-26 an export left under the main checkout's `private/` failed
    `std_socket_inet` on a correct tree, and a stale copy could as well satisfy
    a check. The index is the source, so a file added with `git add` is seen.
    The helpers fail closed: without `git`, outside a checkout (an exported
    tarball) or with nothing tracked under the path, they fail the test with
    the reason and never fall back to walking. Three walks stay, each stated at
    its site: one bounded to the test's own declared fixture directory (an
    inventory there fails on any extra file, which is its purpose); one of a
    temporary directory the test created; and a test documented as runnable
    outside git, which keeps a separate, bounded walk (none today). A test
    whose expected value mirrors a production walk of the same directory keeps
    that walk, since the two cannot disagree; the production readers are
    tracked as `W45-PRODUCTION-WALKS-READ-WORKING-TREE`. No standing- or
    provider-gate step runs these tests outside a checkout;
    `providers/package.py` builds and verifies packages without running tests.
    [W45-TEST-HYGIENE](../reports/w45-test-hygiene/README.md).

<!-- generated:rules:end -->

## Corpus arbiter and its job count

The pinned corpus arbiter is the release binary `quirin-audit pinned`
(`cargo build --release -p qcore-audit --bin quirin-audit`), run through `scripts/run_pinned_audit.py OUT_DIR COMMIT [--inputs-root DIR]
[--census] [--jobs N]`. It analyses translation units on `N` threads
(`--jobs N` on the command, or `QCORE_AUDIT_JOBS=N`; the default is the
host's available parallelism, capped at 16), and folds every unit's report
rows, sidecar results, counts and console lines in unit order. Every archived
artifact and `audit.log` are byte-identical at every job count, and
`--jobs 1` is the sequential loop; the first error in unit order is the one
reported (`W44-AUDIT-PARALLEL-UNITS`). The configured routes
`quirin-audit current` and `quirin-audit CONFIG.json` take the same flag:
`--jobs`, then a configuration `jobs`, then the
variable ([W44-QUIRIN-AUDIT-PROVIDERS](../reports/w44-quirin-audit-providers/README.md)). Two limits:

- A census run always analyses one unit at a time, whatever `N` says: any
  census variable set in a census-feature build forces it, because census
  streams record in analysis order.
- Do not run a many-job arbiter at the same time as `cargo test --workspace`:
  it saturates the host, which is what the load-sensitive rows of constraint
  16 measure. Record `N` (`run.json` does, as `QCORE_AUDIT_JOBS`).

The configured route's `preparation:` line sums per-unit wall-clock time; it
is a measurement, differs between any two runs, and at `N > 1` exceeds the
process's own wall time. [Evidence](../reports/w44-audit-parallel-units/README.md).
