# Validation gate and Lean audit policy

Current gate and Lean audit policy, routed by [AGENTS.md](../../AGENTS.md).
Paths in backticks are repository-relative. History behind these requirements is
in the [agent-cycle checkpoint](../development/increments/structure-checkpoints/agent-cycle-guidance.md).

<!-- preserved:validation:start -->
## Standing per-change gate

Who runs which gate (user decision 4 of 2026-10-24, card `GATE-ELASTIC`): a
delegated card closes on the [scoped card gate](#scoped-card-gate) on its final
commit; the integrator runs this full gate once per landing, on the merge result
before pushing, and it is the arbiter whenever the two disagree. A change made
outside a card runs this full gate. Its content:

```sh
git submodule update --init --recursive   # every fresh checkout, worktrees included
python3 scripts/run_test_binaries.py      # parallel form of: cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
mapfile -t test_config < <(python3 scripts/test_link_config.py)   # see "Test build storage"
cargo run "${test_config[@]}" -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run "${test_config[@]}" -p qcore-compiler --example reevaluation_followup   # expect exit 0
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
that fails is verified in isolation, never weakened. Every cargo command it
runs carries the test-build configuration (`--link`, `--batch-size`:
[Test build storage](#test-build-storage)). The provider workspace is
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

## Scoped card gate

```sh
python3 scripts/gate_scope.py --base <card base>   # --plan previews; --no-slots under the legacy flock; --landing on a landing
```

It maps `git diff --name-only --no-renames <base>...HEAD`, plus uncommitted and
untracked paths, to the crates and cargo targets of both workspaces through
`cargo metadata`, and gives a path to every crate that reads it:

- at compile time, by `include_str!`/`include_bytes!`/`include!` or a `#[path]`
  module, claiming the file read (`--reader-granularity directory`: its whole
  directory, `CT-FIXTURE-EXTERNAL-READERS`; `--readers` prints the census);
- at run time, by a string literal in code `cargo test` runs. A literal naming
  a tracked file claims exactly that file. A literal naming a directory claims
  only what the walker table `scripts/gate_scope_walkers.json` declares for
  that site: the globs of the files the site reads through it, or none for a
  base path whose files are named by other literals (card
  `GATE-SCOPE-PRECISE-SELECTION`, `GATE-SCOPE-RUNTIME-READS-HEURISTIC`). A
  directory literal no entry declares still claims its whole directory
  (fail-closed) and fails `scripts/tests/test_gate_scope_census.py` until it is
  declared. The table's `expansions` give the files a manifest lists to every
  reader of that manifest (`providers/contract-load-path.json` and its
  contract roots). Each entry is held to the run-time read census
  (`scripts/gate_read_census.py`: every root test executable under `strace`):
  every tracked file a unit was seen reading must select that unit, and every
  walker entry must be seen walked.

A reader compiled only into test executables claims for those alone, and
nothing depends on a test executable, so no reverse dependency follows. They
are integration-test targets (`tests/*.rs` and the modules they declare,
`PKG::test::NAME`) and a library's unit-test executable (`PKG::lib::NAME`) for
the library's `#[cfg(test)]` code, which dependents and doc-tests never build:
a module the library declares only under a test-only `#[cfg]` (`test`,
`all(.., test, ..)`, or `any` of test-only predicates) and the modules below
it, and an include, `#[path]` or literal inside a `#[cfg(test)]` item
(`GATE-SCOPE-LIB-UNIT-TEST-MODULES`). A file the library also reaches by an
ordinary module edge stays the crate's. What the scanner cannot classify
keeps the crate whole: an inner `#![cfg(test)]` (adding one takes a module out
of dependents' build), any other predicate, an attribute on something that is
not an item or a `let`, an item whose end it cannot find, a binary's
`cfg(test)` code, a library with `test = false`, and every file of a crate
whose library shows `include!` of Rust code, a `mod $name` in a macro or
`cfg_attr(.., path = ..)`. A path a crate owns selects only its test
executables when it is a Rust file only test executables compile (a `src/`
module declared only under `#[cfg(test)]` included), or a non-Rust file whose
readers in that crate are all in test executables; the rest of its `src/`,
`Cargo.toml`, `build.rs`, and any file nothing in the crate is seen reading
select the whole crate and its reverse dependencies
(`GATE-SCOPE-FULL-CRATES-TEST-ONLY`). The Rust suite is then
`run_test_binaries.py --packages <root members reached whole> --targets
<PKG::test::NAME,PKG::lib::NAME,...>` (cargo's `--test NAME` and `--lib`
filters), its `EXCLUSIVE` phase unchanged, or the whole suite when
the diff reaches `qcore-c-gateway`, `qcore-core`, `qcore-contract` or
`qcore-schemas` as a whole (a test-only reach is not full), a workspace
manifest or lockfile, the runner, the scoper or the test-build configuration
(`test_link_config.py`, `rustc_test_dylib.py`), or a path that no crate owns
or reads and that is not prose, Python, Lean or provider tooling. Beside it:
the submodule state, fmt, Clippy on the same packages (per crate with `--test`
filters for a test-only reach, and `--lib --profile test`, which lints the
library as its unit-test executable compiles it, for a library unit), both
reevaluation examples when
`qcore-compiler` is reached whole, Lean (`scripts/gate_lean.py` on the changed
`proof/QCore` modules, their importers and the audits importing them, or all of
`check_lean_audits.py` when a `proof/` root, a deletion or the Lean runner
changes), the Python suite always, and the provider gate when `providers/`
changes. When the root `Cargo.toml` or `Cargo.lock`, or the `Cargo.toml` of a
root crate the provider workspace builds by path (transitively), changes and
the provider gate is not selected, a `cargo metadata --locked --offline
--manifest-path providers/Cargo.toml` step checks that `providers/Cargo.lock`
still resolves (`GATE-SCOPE-PROVIDER-LOCK`). Every step's argv, exit, counts
and log go into `gate.json` in the run's directory
(`private/gate-scope/<stamp>-<pid>/`); commit it to the card's report with the
selection it printed. Exit 0 needs every step at exit 0.

The acceptance matrix's derived Lean-audit list is the integrator's record,
regenerated at landing (`CARD-GATE-NEW-AUDIT-MATRIX-COUNT`). A card does not
regenerate it: on a card gate the Python step gets the card's base in
`QUIRIN_CARD_GATE_BASE`, and `test_build_matrix_sections.py` then accepts a
committed list that differs from `proof/Audit*.lean` only by the audits the
card adds or removes and by the drift its base already had. The standing gate
and `gate_scope.py --landing` set no base and keep the exact check; a landing
run through the scoper passes `--landing`.

What the scope does not see, so the landing gate stays the arbiter: a path a
test computes from data or a filtered walk that started after the read census
was recorded (the census is one commit's observation; re-record it with
`gate_read_census.py record` and `reduce` when the table is revised); a code
path no test took when the census ran; cargo feature unification under `-p`,
which can differ from `--workspace`; and provider-workspace members reached
only through their dependencies on core crates, which `gate.json` lists but
which, as in the full gate, get the provider gate only when `providers/`
changes. A narrowing is accepted only with a replay showing that no unit that
failed at a recorded red commit is dropped
([evidence](../reports/gate-scope-precise-selection/README.md),
[library units](../reports/gate-scope-lib-unit-test-modules/README.md)).

## Gate slots

Heavy jobs take a named slot from `scripts/gate_slot.py <class>
[--memory-budget 7G|5G|4G] -- <cmd>` instead of one global lock (user decision
5 of 2026-10-24). A slot is a `flock` on `/tmp/quirin-gate/<class>/<n>`; the
command's exit is the script's, and a receipt records class, slots held,
budget, admission budget, the memory reading, the argv as run and the child's
peak resident set (`wait4`'s `ru_maxrss`: the largest single descendant).

| Class | Slots | Use |
|---|---|---|
| `rust-suite` | 1; while it runs it also holds every `audit` slot | the root Rust suite and the provider workspace's `cargo test` (constraint 16's load-sensitive rows never share the host with an audit) |
| `audit` | `max(1, floor(MemAvailable / admission budget))` when it asks; budget 7 GiB by default, 5 or 4 GiB selectable; the pinned corpus is admitted at 11 GiB | corpus, system, slice and other `quirin-audit` runs |
| `lean` | 2 | `check_lean_audits.py`, `gate_lean.py` |

The audit budget is the audit's own MiB planning budget (`audit-ir-working-set`):
`quirin-audit current` gets `--memory-budget-mib`, a configuration's
`memory_budget_mib` must equal it, and any other route gets `QCORE_AUDIT_JOBS`
capped at `(budget - 1024) / 2048` workers, with an explicit `--jobs` above the
cap refused, so a smaller budget lowers the run's own concurrency. Admission
counts the larger of that budget and the route's measured budget: the pinned
corpus (`run_pinned_audit.py`, `quirin-audit pinned`) peaks near 10.1 GB at
three workers, so it is admitted at 11 GiB while its workers stay planned at the
slot budget, the one measured exception to the 7 GiB default
(`ROUTE_ADMISSION_MIB`, card `LANDING-PUBLISH-COMPLETION`). During the
transition every slot job holds the legacy lock
`/tmp/qcore-c-card-validation-20261022.lock` shared, so a job under
`flock <legacy lock>` excludes every slot and every slot excludes it. Limits:
fmt, Clippy and builds take no slot, so another card's build can still load the
host beside a `rust-suite` job; `MemAvailable` already excludes running audits'
resident memory, so admission is conservative; an exclusive legacy waiter can
wait while slot jobs keep the lock shared.

## Test build storage

Building and running the test suites must fit a CI runner's disk: about 15 GB
at peak from an empty `target/` (user request of 2026-10-29, card
`TEST-STORAGE-BUDGET`). `scripts/test_link_config.py` is the one place that
names the test-build configuration; `run_test_binaries.py` (both gates), the
scoped gate's reevaluation examples and CI's test steps pass its `--config`
arguments to every cargo command that builds the root workspace's tests:

- `build.incremental=false` (the CI jobs also set `CARGO_INCREMENTAL=0`, the
  same setting): a gate or CI build starts from an empty or foreign `target/`,
  and incremental state is gigabytes nothing reads again.
- On Linux, `build.rustc-wrapper` = `scripts/rustc_test_dylib.py`: every
  root-workspace library is also emitted as a Rust dylib, and every test
  executable, binary and example of the root workspace links those and
  `libstd` dynamically, with absolute run paths (an executable started with a
  cleared environment still loads them). The wrapper withholds rustc's early
  metadata signal, so cargo starts a dependent only after the whole upstream
  build, `.so` included. A fresh `cargo test --workspace` peaked at 87.4 GB
  (`target/` plus temporary files); the gate's runner now peaks at 3.8 GB,
  with the same tests and outcomes and the same run time per executable.
- Elsewhere tests link statically: a Windows DLL of the large crates would
  exceed the 65,535-export limit. Windows CI runs the full suite as
  `run_test_binaries.py --jobs 1 --batch-size 32`: each batch of 32 target
  names is built, captured, run (parallel units, then its `EXCLUSIVE` units
  alone) and its executables deleted before the next is built; the examples
  `cargo test` compiles are compiled and deleted in batches too; the run fails
  closed unless the executables built are exactly the selection's test
  targets, each once, and runs the doc-tests last. Measured on Linux in that
  mode: 6.2 GB.
- Debug information stays `line-tables-only` in both workspaces' dev profiles
  (the provider workspace's since this card: its suite went from 15.0 to
  5.2 GB), so panics keep file:line; the budget is met without dropping it.

- The root-workspace probes of the provider tooling (`providers/package.py`'s
  source-mode probe, `providers/contracts.py`'s view, inventory and lock
  probes) are built by `package.py`'s `root_workspace_executable`: `cargo
  build --workspace --example|--bin NAME` with this configuration, then run
  directly. A command that builds into the test build's `target/` keeps the
  workspace's feature selection (`--workspace` with a target filter, as CI's
  quota step does): `-p <crate>` resolves that crate's features alone and
  rebuilds the libraries whose features differ.
- The Linux CI `rust` job, measured as one sequence from a fresh shallow
  checkout (card `CI-RUST-JOB-PEAK`): **10.7 GB** at peak (11.0 GB before it
  shared the probes' and the quota step's units), during the
  provider suite, with the checkout (2.6 GB), both workspaces' targets and the
  temporary directories counted. Its package step runs `package.py
  reproduce` (card `PACKAGE-BYTES-DEPEND-ON-CHECKOUT`): the second build's
  copy of the checkout without `docs/`, its `CARGO_HOME`, target and output
  (0.25 GB) and the first build's 38 MB stage exist only during that step,
  and the release target the provider suite runs beside is the same 0.11 GB.

The test set, the outcomes and the fail-closed rules are the same in every
mode; only the bytes on disk differ. The root manifests,
lockfile and cargo configuration files do not change, so release builds
(`quirin-audit`, `quirin-audit measure`'s `rev:` sides, provider packages)
are the builds they were, and cargo runs started by tests never inherit the
`--config` arguments. Plain `cargo test --workspace --no-fail-fast` stays the
reference command and builds statically in a fresh `target/`. The incremental
setting is part of cargo's fingerprint and the wrapper is not: in one
`target/`, a plain command with incremental builds on (the default) and a
configured one rebuild each other, while a plain command with
`CARGO_INCREMENTAL=0` reuses configured units, dynamically linked ones
included, and a configured build then reuses a library a plain build left
without its `.so` and, by the wrapper's fallback, links it statically.
[Report](../reports/test-storage-budget/README.md),
[whole-job measurement](../reports/ci-rust-job-peak/README.md).

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

Every pinned corpus, system or slice figure belongs to one evaluation profile
(`contract/evaluations/<name>/evaluation.json`, user decision 2 of 2026-10-24;
`scripts/evaluation_profiles.py` is the authority and the Python suite checks
it). Cycles are frozen and never re-measured; the live profiles `eight-home`,
`fluent-bit-system` and `fluent-bit-tail-pause` move only with a new baseline
recorded with its reason in the same commit, named in the card's amendments
`profiles[]`.

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
