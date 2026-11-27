# MEASURE-CONTRACT-ONLY-SIDE-WIRING

Base `8fd7c8d0d`. Named follow-up registered by
[MEASURE-LOOKUP-RECORD-REUSE](../measure-lookup-record-reuse/README.md)
(sections 3 and 7). Tooling only: no verdict, coverage, contract or pinned
baseline changes.

Outcome: `quirin-audit measure --reuse-contract-only` measures the tip of a
classified contract-only diff with the base side's analyzer binary, resuming
the base side's pinned group checkpoints, from one working directory; every
other diff keeps today's separate sides. Wiring it exposed that the
acceptance it relies on was inert on every base that holds it: the
classifier refused its own source (§2). On this host's locally prepared
corpus the composed tip is byte-identical to a full run for both a replayed
sampled landing and a diff the route does not read (§3). The reuse saves
analysis only for the second kind; the first recomputes every group, and the
reason is not only `GATEWAY-WHOLE-TABLE-READS` (§4).

## 1. Design

`crates/quirin-audit-plugin-measure/src/reuse.rs`, wired in `lib.rs` and
`sides.rs`.

- **Gate** (`reuse::candidate`, before anything is built). Only the first
  comparison, only two `rev:` sides, only with a pinned corpus route, and
  neither `--rules-off` (switched sides stay separate) nor `--build-commit`
  (the accepted binary must record the base commit). Anything else records
  `contract_only_reuse.taken: false` with the reason and measures as before.
- **Classification by the base binary.** The base side is built first. The
  tip is then a shared clone checked out at the tip (submodules cloned as for
  any snapshot), not a `git archive`, because the run checks the
  repository's `HEAD` and contract cleanliness. The base side's binary
  classifies `BASE..TIP` with the new `quirin-audit pinned --classify-only
  --accept-base-analyzer BASE --repository DIR` (one
  `quirin-contract-only-classification/1` JSON line, exit 0, or the refusal,
  exit 1): the classifier that decides is the one the tip run is checked by.
  The tip tree's `run_pinned_audit.py` must forward the acceptance (it now
  takes `--accept-base-analyzer BASE --repository DIR`). On acceptance the
  base binary is copied into the tip tree in place of a build, so the tip
  side records the base side's binary digest and build commit. **Any refusal**
  (a mixed diff, a compiled-in contract, an unenumerable include, a binary
  without `--classify-only`, a tree whose runner does not forward the
  acceptance) discards the clone and builds the tip as a separate side,
  exactly as without the option: fail-closed, never a partial reuse.
- **One checkpoint directory.** The base side's pinned route seals into
  `<output>/checkpoints/<route>`; the tip side's waits for the base side's
  route to end, then runs with `--checkpoint-dir` that directory, `--resume`,
  `--accept-base-analyzer BASE --repository <tip as run>`. The analyzer
  decides reuse by its own identity check; the tool decides nothing. Each side
  keeps the checkpoint run record it produced
  (`sides/<side>/<route>.checkpoint-run.json`, summarised per stage in the
  route's `checkpoint_run`).
- **One working directory** (the minimal part of
  `PINNED-ROWS-BIND-WORKING-DIRECTORY`). The pinned route digests the process
  working directory into every row's `hypothesis_context.build_identity` and
  into the checkpoint identity, and each route ran from its own
  `sides/<side>/work-<route>`, so no tip group could ever equal a base group.
  Every route of a snapshot side now runs from `<mount root>/work` with its
  private directory bound there (hazard 2 still holds on disk), so every side
  of every measurement spells one directory. This is that entry's "fixed
  measurement working directory" discharge for snapshot sides; checkout and
  binary sides (no namespace) keep their own directories. It changes the
  bytes of `build_identity` relative to earlier measurements, which already
  differed per output directory; the comparison counts them as
  environment-only, and no status moves.
- **Verification** (`--verify-reuse`). Also measures `<tip>-full` (the same
  tree and binary, no checkpoints, concurrently) and fails unless, per route,
  every report artifact of the tip equals the full side's byte for byte with
  no substitution and the base-to-tip comparison equals the base-to-full one
  (its rows are not counted as interactions). The record's
  `contract_only_reuse` slot holds the decision, classification, checkpoint
  directories and verification.
- Not for landing baselines: `land.py publish` compares a `baseline:` side,
  which never takes the path, and a tip measured by the base build names the
  base commit in its evidence versions.

## 2. The classifier refused every base that holds it

Classifying the sampled landing on this card's base refused:
`crates/qcore-audit/src/contract_only_diff.rs: an include whose argument
cannot be enumerated`. The scan read every textual `include_str!(` in the
base tree's non-test Rust, and the classifier's own inline tests spell
unenumerable includes in string literals (`"include_str!(PATH)"`). Its
sampled commits were accepted only because their bases predate the file. So
on every base at or after `1932d511f` the acceptance, and with it any reuse,
was impossible. `contract_only_diff::code_mask` now marks which bytes are
code (not a line, nested block or doc comment, nor a string, raw string, byte
or C string, or character literal; a lifetime or label stays code), and only
an occurrence starting in code is an include; its argument is read as before.
The only unsound direction is a lexer error that hides real code, so the
test `only_includes_in_code_count_and_literals_do_not_hide_code` puts an
unenumerable and an enumerable include after each literal form that could
swallow code if lexed wrongly (`'"'`, `b'"'`, `'\''`, `'\\'`, `'\u{22}'`,
`'é'`, lifetimes and labels, `"a\\"`, raw `r"a\"`, the raw identifier
`r#type`, comments) and requires both to stay visible; the sampled-commit
test now also classifies `HEAD` as a base and accepts `a1bb5a981`.

## 3. Byte identity on a sampled landing

**Why not through the tool on this host.** `rev:` sides build and run in a
private mount namespace (`unshare -rm`), and this host's AppArmor policy
refuses unprivileged user namespaces
(`kernel.apparmor_restrict_unprivileged_userns = 1`; `unshare: write failed
/proc/self/uid_map: Operation not permitted`), so no `rev:` side runs here,
with or without this card. No published live-route inputs exist here either.
So the corpus inputs were prepared locally (Fluent Bit `66910c10` by
`quirin-audit prepare clang-build` and `prepare monkey` with Clang 21, the
ChunkIO package by `providers/package.py build`; 1,408 production units, 185
corpus units in 179 unit rows and 4,704 function rows), and
[evidence/demo.sh](evidence/demo.sh) executes the command sequence the reuse
path runs, with the namespace's one spelled tree and working directory
replaced by **one real tree** checked out at each revision in turn and **one
real working directory**: the base binary built at `BASE` with
`QCORE_AUDIT_BUILD_COMMIT=BASE`; the base side sealing into one checkpoint
directory; per tip, `pinned --classify-only` by the base binary, then the tip
resumed under `--accept-base-analyzer BASE --repository <tree>` from a copy of
that directory, beside a full run of the same tree and binary without
checkpoints. The environment is the measure corpus route's, with Clang at
this host's path (`run_pinned_audit.py` fixes `/usr/bin/clang`, absent here),
so the binary is called directly. The side directories are compared by
`quirin-audit measure` itself over `run:` sides
([evidence/compare.sh](evidence/compare.sh)), and
[evidence/verify.py](evidence/verify.py) applies the `--verify-reuse` check
to its records ([verify.jsonl](evidence/verify.jsonl), exit 0).

`BASE` is this card's `a11df6a5d` (the classifier fix needs a base that has
it). The tips are commits in the private clone, not on this branch:

- **Tip A, the sampled landing replayed**: `git revert a1bb5a981`, the last
  contract-only landing on main (`a1bb5a981~1..a1bb5a981`: a loaded
  contract's licence header and its `contract-inventory.lock` entry), applied
  to `BASE` (`8558e2956`).
- **Tip B, a diff the route does not read**: one comment line in
  `providers/quirin-provider-shared/contracts/library/custody_evidence.qv`
  (not in the pinned manifest) and its inventory lock entry (`f63cde2ef`).

Both were accepted by the base binary
([tip-a](evidence/tip-a.classification.json),
[tip-b](evidence/tip-b.classification.json); 137 compiled-in paths checked),
and both resumed runs printed `analyzer accepted from the base side`.

| Comparison | Tip A | Tip B |
|---|---|---|
| composed tip vs full tip: the nine report artifacts | byte-identical | byte-identical |
| composed vs full: function rows | 4,704 identical | 4,704 identical |
| base→composed equals base→full (the comparison record) | yes | yes |
| base→tip function rows | 4,704 environment-only, 0 moved | 4,704 identical |
| tip function status | Safe 1,081, Unknown 3,617, Unsafe 6 | same |

The nine artifacts: `corpus/resource-diagnostics.json`, its `components`,
`contracts`, `chunk-contracts` and `source-effects` sidecars,
`corpus/monkey-source.json`, `corpus-null.jsonl`, `corpus-null.summary.json`,
`corpus-outer.json`; no substitution was needed. The harness's own timing
files are the only route-directory files that differ, and `verify.py` drops
exactly those. Records: [tip A](evidence/native-comparison-tip-a.json),
[tip B](evidence/native-comparison-tip-b.json), checkpoint run records
([base](evidence/base.checkpoint-run.json),
[tip A](evidence/tip-a.checkpoint-run.json),
[tip B](evidence/tip-b.checkpoint-run.json)). Tip A's base→tip rows are
environment-only because every row's hypothesis context digests the whole
contract text set, which the header changes.

**Timings** ([evidence/timings.txt](evidence/timings.txt); `/usr/bin/time`,
one measurement each, on a 32-core host shared throughout with other cards'
corpus runs and this card's gate, so noisy):

| Run | Jobs | Wall | Peak RSS | Groups |
|---|---:|---:|---:|---|
| base, fresh, sealing (attempt 1) | 12 | 1:42:31 | 17.4 GB | 12 sealed; then failed at the Cycle 18 libevent pin check (the tree lacked its submodule), so discarded as evidence |
| base (the evidence run): attempt 1's groups resumed | 12 | 3:45 | 17.4 GB | 12 reused (identity equal) |
| tip A composed (resume) | 8 | 1:25:05 | 17.3 GB | 12 recomputed (`run.pin_evaluation_sha256 differs`) and resealed |
| tip A full, concurrently | 8 | 1:25:26 | 10.7 GB | none |
| tip B composed (resume) | 8 | **4:19** | 17.3 GB | 12 reused; checkpointed phases 53.8 s |
| tip B full, concurrently then alone | 8 | 52:21 | 10.4 GB | none |

## 4. What the reuse saves, and what stays inert

- **A diff the route does not read** (tip B): the tip's pinned analysis is
  replaced by a 4 min resume (the whole-run steps outside the checkpointed
  phases, `PINNED-RESUME-FIXED-COSTS`), and the tip is not built. On a host
  whose slots run both sides at once the measurement's wall time is the base
  side's plus that resume, against the longer side's alone; the saving is the
  tip's analysis time and slot occupancy (here 52-85 min of corpus analysis),
  and wall time where slots serialize the sides.
- **A diff that edits a loaded contract** (tip A, the sampled landing, and in
  practice every card that edits a contract the corpus loads): every group recomputes, so the tip costs a
  full run and, starting only after the base side's, lengthens the
  measurement by it. The recorded reason is not the whole-environment scope:
  the **run** identity already differs, in `run.pin_evaluation_sha256`: the
  pin template (`contract_revision::template`) records, for every pinned
  contract, the text digest of its module (`module_sha256`), and the edit
  changed `logging.qv`'s text. Run identity is checked before the stage
  environment, so this holds even once `GATEWAY-WHOLE-TABLE-READS` lets a
  group seal `restricted`: an edit to any module holding a pinned contract
  recomputes every group until the run-wide pin facts are keyed per group
  too (`PINNED-RUN-IDENTITY-PIN-EVALUATION`, registered). Use the option today
  for diffs the corpus route does not read; for any other contract diff it
  costs time.
- Checkpointed runs peak at 17.3 GB against 10.4-10.7 GB plain, above the
  measure tool's 7G `audit` slot budget (`PINNED-CHECKPOINT-PEAK-MEMORY`,
  unchanged).
- The working-directory binding is in effect for every snapshot side;
  checkout and binary sides still run from their own directories, so the
  pinned rows of those sides keep binding a per-measurement directory
  (`PINNED-ROWS-BIND-WORKING-DIRECTORY`, amended, not discharged for them).

## 5. Tests

`quirin-audit-plugin-measure` (`reuse.rs` units): the gate admits only two
`rev:` sides with a pinned route and no switch or explicit build commit
(every other side kind, in either position, refused with its reason); the
base seals and the tip resumes under the acceptance (exact arguments); the
verification needs identical artifacts with no substitution and an equal
comparison (each failure mode); the checkpoint summary counts decisions per
stage. Integration (`tests/measure.rs`): a measurement that cannot take the
path measures and compares exactly as without the option and records why;
usage refusals (`--verify-reuse` alone, a repeated flag, `publish` with the
option). `qcore-audit`: `classify_only_answers_the_contract_only_classification_without_a_run`
(acceptance JSON, refusal, misuse), §2's lexer test and the extended
sampled-commit test. A `rev:` measurement through the path itself needs user
namespaces, which this host refuses; the integrator's measurement on the
release host with `--reuse-contract-only --verify-reuse` is the arbiter for
the tool path.

## 6. Gate

`python3 scripts/gate_scope.py --base 8fd7c8d0d` on `0bbad3f67` (the code as
committed here; the commits after it add only this report's files and the
harness's resume option), under `private/toolchain/env.sh` (system Clang
21, Lean 4.35.0): [gate.json](gate.json), logs
`private/gate-scope/20261113T070558Z-462247/`. Scoped: `qcore-audit`,
`qcore-compiler`, `qcorec`, `quirin-audit-plugin-measure` and
`qcore-verification::test::composition_bounds`; no Lean or provider step.
**Exit 0, 8 steps.**

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| fmt / Clippy (4 crates, all targets) / Clippy `composition_bounds` | 0 / 0 / 0 | |
| Rust suite | 0 | 203 units, 205 summaries, 2,665 passed, 0 failed, 4 ignored |
| reevaluation probe / follow-up | 0 / 0 | |
| Python suite | 0 | 761 tests |

The increment and status paragraphs and the ledger entries are in
[integration-amendments.json](integration-amendments.json)
(`apply_amendments.py --check` exit 0); they are not inserted by hand, which
would make the check refuse them as already present.

## 7. Follow-ups

- `MEASURE-CONTRACT-ONLY-SIDE-WIRING`: retired by this card (wiring,
  classification gate, byte identity on a replayed sampled landing).
- `PINNED-RUN-IDENTITY-PIN-EVALUATION` (new, §4): the pinned run identity
  digests the whole pin template, which holds the text digest of every module
  with a pinned contract, so an edit to any such module recomputes every
  group regardless of the stage environment's scope.
- `PINNED-ROWS-BIND-WORKING-DIRECTORY` (amended): snapshot sides now share one
  spelled working directory; checkout and binary sides do not.
- `MEASURE-REUSE-NAMESPACE-HOSTS` (new): the reuse path, like every `rev:`
  side, needs unprivileged user namespaces; on a host that refuses them
  (Ubuntu's AppArmor default) the tool cannot measure `rev:` sides at all.
- `GATEWAY-WHOLE-TABLE-READS`, `CONFIGURED-ROUTE-LOOKUP-REUSE`,
  `PINNED-RESUME-FIXED-COSTS`, `PINNED-CHECKPOINT-PEAK-MEMORY`: unchanged;
  `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` amended to point here.
