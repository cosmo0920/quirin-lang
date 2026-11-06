# CORPUS-GROUP-CHECKPOINTS

Round 21 (2026-11-06), base `13337d0af`. User decision (verbatim, 2026-11-06),
answering MEASURE-QUOTA-AND-INCREMENTAL's R8 stop: "D then A (rec.)", the
button's text being: "R8: option D first (per-group checkpoints for the pinned
corpus route), then option A (gateway publishes per-unit lookups including
missed ones; reuse only for contract-only diffs with matching cross-unit facts).
I approve using that lookup record as a reuse input once it is published and
checked." This card is D: per-group `.qo` checkpoints for the pinned corpus
route (`W45-CHECKPOINT-PINNED-REPLAY-ROUTE`), within one revision. Cross-revision
reuse is A's (`MEASURE-LOOKUP-RECORD-REUSE`, §6).

Outcome: `quirin-audit pinned --checkpoint-dir DIR [--resume]` seals each
library group of each phase the replay runs and reuses a group only when its
whole identity is unchanged; the published outputs are byte-identical to the
whole run's (§4). No analyzer, gateway or contract semantics change and the
route is unchanged without the flag, so there is no prediction and no declared
measurement. The format and identity are specified in
[audit checkpoints](../../development/audit-checkpoints.md#the-pinned-corpus-route).

## 1. Design

The pinned route (`crates/qcore-audit/src/pinned_route.rs`) runs up to three
phases over every unit: phase one under the loaded contracts; phase two under
phase one's refutation table when a body refutes its own declaration (C06.4);
and, when `QCORE_CROSS_UNIT_LINK_MAP` selects homes, an import phase under the
imported environment, accepted only when its rows leave the table unchanged.
Every measure corpus route sets the link map, so a corpus measurement analyses
every unit twice (phase one, then the import phase; the table has been empty).

- **Groups.** The route's libraries (`cmetrics`, `ctraces`, `librdkafka`,
  `chunkio`, `fluent-bit-chunk`, `monkey-libevent`). ChunkIO's prelude and
  postlude share typed certificates across a library's units, so the library
  is the smallest result that stands alone; the postlude is run when the group
  completes and its console lines and appended teardown results are stored, so
  no certificate is serialized.
- **Stages.** Each phase is a stage with its own file,
  `DIR/<library>.<stage>.qo` (`first`, `refuted`, `imported`): a resumed run
  finds each stage's checkpoints where it left them, unlike the current route's
  shared paths (`CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES`, which stays open for
  that route).
- **Sealing in flight.** All groups of a stage run concurrently, as in the whole
  run (no one-group-at-a-time limit, cf. `W45-CHECKPOINT-GROUPS-IN-FLIGHT`).
  The worker that completes a group runs its postlude and seals it, whatever
  the other groups are doing, so an interrupted run keeps every completed group.
- **One container, one fold.** The `.qo` container is the current route's,
  generalized by a `Layout` (schema version and section list); the pinned
  layout is schema 1 with five sections, and each reader refuses the other's
  files. The whole-run fold is shared: the final stage's groups are expanded
  into the steps the fold already consumes, and each stored unit is restored
  into its report fragment (`AuditReport::{into,from}_fragment_parts`) when the
  fold reaches it. Without `--checkpoint-dir` the route runs the code it ran
  before, with the home export and premise moved into two functions both paths
  call.

## 2. The identity, and what forces a re-run

A group is reused only when its whole `pinned-identity` section equals the one
computed now (field list in the specification):

| Part | Covers | Invalidates |
|---|---|---|
| run | analyzer build (binary SHA-256, build commit, profile); Fluent Bit root and verified revision; the process working directory (§4.1); every file of the source-set profile directory; the source-contract configuration and the ChunkIO package's files; contract manifest, every lock read, the pin template; Clang identity; every `QCORE_*` variable except the job count, with the null, outer and member-prepass output paths reduced to `selected` and the link map to its content digest; the run's member facts | every group of every stage |
| stage | the exact contract environment every unit of the stage is analysed under (its serde form: pins, holds, result ranges, the refutation table, imports, link resolution); the homes supplied to every unit; the bounds and null environments with the bounds refutation table | every group of that stage only |
| group | per unit: the compile record `[file, directory, command]`, raw source digest, validated preprocessing digest, and the preparation facts (line-marked preprocessing digest or error, validation error) — three Clang runs per unit, no analysis | that group only |

Forced re-runs (fail-closed): any identity difference (the first differing
field is printed: `checkpoint cmetrics.first: recomputed (run.environment
differs)`); a missing, truncated, tampered or other-layout file (`checkpoint
refused: …`); a unit that cannot be fingerprinted (its group is recomputed and
never sealed); a group with a failed step or postlude (never sealed). Refused
before any work, because a reused group would not reproduce what they write or
the identity does not digest what they read: census streams,
`QCORE_TYPED_PATH_PARITY`, `QCORE_AUDIT_IR_DIR`, `QCORE_UNIT_TIMING`,
`QCORE_BOUNDS_PROPERTY` (`CORPUS-CHECKPOINT-BOUNDS-SELECTION`) and
`QCORE_SOURCE_CONTRACTS`. Whole-run steps that read no group result run every
time: library summary profiles, pin, bounds and outer evaluations, the member
facts pre-pass, fingerprinting, the definition census and home export, the
Monkey gate and every sidecar.

The three refusals of the current route (`QCORE_NULL_PROPERTY`,
`QCORE_OUTER_PROJECTION`, `QCORE_CROSS_UNIT_LINK_MAP`) are lifted here because
the identity covers them: the null and outer selections are recorded as
selected (their rows are written by the fold from the stored rows and
fragments), the link map by its content, and the cross-unit facts by the stage
environment and homes.

No semantic choice fail-closed leaves open was found: every whole-run fact a
group reads is a function of earlier stages' rows of every group, so the stage
identity carries it and a change recomputes every group of that stage.

## 3. Fixtures (evidence item 3)

`crates/qcore-audit/src/pinned_route/checkpoints/tests.rs` (5 tests) and
`crates/qcore-audit/tests/pinned_cli.rs` (options and refusals):

- `a_changed_input_of_each_kind_invalidates_exactly_the_groups_that_read_it`:
  three fixture groups fingerprinted with real Clang. A group's source, a
  header it includes, or its compile record invalidates that group alone (a
  comment the preprocessor removes invalidates nothing); the contract
  environment, the analyzer build, the pin evaluation, a lock, the source
  profile, Clang, a selection, the rule switch or the link map's content, and
  the member facts invalidate every group, each naming its field; an output
  path or the job count invalidates none; a changed refutation table
  invalidates every group's `refuted` stage and none of its `first` stage; a
  changed bounds environment every group of the stage; a changed cross-unit
  home body every group's `imported` stage (`stage.homes_sha256`).
- `a_sealed_group_is_reused_only_under_its_identity_and_refused_when_damaged`:
  round trip; provenance outside the content digest; a changed stage or
  analyzer identity named; the current-route reader refuses the file; a flipped
  byte, a truncation and a missing file recompute; an incomplete group is never
  sealed.
- `a_restored_unit_is_the_stored_unit`, `the_environment_records_selections_and_the_link_map_content`,
  `a_tree_digest_covers_every_file_by_path`; `pinned_checkpoints_refuse_unsupported_environments_before_any_replay`.

## 4. Corpus evidence (evidence items 1 and 2)

All runs: the release binary built at `cf04b717c` (SHA-256 in
[evidence/binary.sha256](evidence/binary.sha256)), the 185-unit corpus inputs
at Fluent Bit `66910c10` (`build/cycle19` of the primary checkout, read only),
three jobs, each under an `audit` slot (`gate_slot.py audit --memory-budget
7G`, admitted at 11 GiB), with the measure corpus route's environment (link
map, null property, runtime projection) and `run_pinned_audit.py`'s (Clang,
ChunkIO package), from one fixed working directory
([evidence/run_corpus.sh](evidence/run_corpus.sh)). Wall time and peak RSS
are `/usr/bin/time -v` inside the slot, so slot waits are excluded; the host
was shared with other cards' gates and measurements throughout, so wall times
are noisy. Outputs were compared byte for byte
([evidence/compare_runs.py](evidence/compare_runs.py): the nine published
artifacts and the console, whose last line names the output directory).

| Run | Checkpoints | Exit | Wall | Peak RSS | Groups |
|---|---|---:|---:|---:|---|
| plain: the whole run, no `--checkpoint-dir` | none | 0 | 2:44:22 | 10.98 GB | — |
| interrupted: fresh into `qo-one`, `kill -9` once `cmetrics.imported.qo` was sealed | `qo-one` | 137 | 1:11:14 | 15.85 GB | sealed: 6 `first`, 1 `imported` |
| resumed: `--resume` over `qo-one` | `qo-one` | 0 | 1:37:32 | 15.85 GB | reused 6 `first` + `cmetrics.imported`; recomputed and sealed 5 `imported` (`no checkpoint`) |
| reused: `--resume` again | `qo-one` | 0 | **14:25** | 15.83 GB | reused all 12 (`identity equal`) |

**Item 1 (interrupted, then resumed).** The interrupted run left seven sealed
files ([kill record](evidence/interrupted/kill.txt)); the resumed run reused
exactly those and recomputed the rest
([run record](evidence/resumed/pinned-checkpoint-run.json)), and its outputs
equal the uninterrupted plain run's: all nine artifacts and the console
byte-identical ([evidence/compare-plain-resumed.txt](evidence/compare-plain-resumed.txt),
exit 0). Interrupted plus resumed took 2:48:46 against the plain run's 2:44:22.

**Item 2 (the whole corpus twice).** The resumed run is the first complete
checkpointed run (its seven reused groups were sealed by the interrupted run,
under the same identity); the second, `reused`, reused every group of both
stages and wrote outputs byte-identical to it and to the plain run
([evidence/compare-plain-reused.txt](evidence/compare-plain-reused.txt), exit
0). Its 14 min 25 s break down as the checkpointed phases' 90.8 s
(fingerprinting 15.5 s, loading the six `first` files 1.6 s, the definition
census and home export about 72 s, loading the six `imported` files 1.5 s;
[run record](evidence/reused/pinned-checkpoint-run.json)) and about 12.9 min
of whole-run steps outside them, mainly before the phases (the member-facts
pre-pass over the 1,444-unit production database, the outer reach, the pin and
bounds evaluations) and the fold and sidecars after
(`PINNED-RESUME-FIXED-COSTS`). An uninterrupted fresh checkpointed run was
measured once, at 2:22:09, by the first attempt below (binary `28a0c3ecc`,
whose identity lacked the working directory and which built its refutation
tables in one piece; its analysis is the same code).

**Memory.** Every checkpointed run peaks at about 15.8 GB, including the fully
reused one, which analyses nothing, against 10.98 GB for the whole run: the
excess is the group records held in memory (typed binary values of every
unit's rows for both stages, the `first` stage's kept until the import stage
is accepted), not analysis. It exceeds the pinned corpus's 11 GiB slot
admission (`gate_slot` warns: "peak resident set … exceeds the admission
budget"); `PINNED-CHECKPOINT-PEAK-MEMORY`.

### 4.1 The working directory is an input (first attempt)

The first attempt ran a checkpointed fresh run and a plain run from different
working directories (`r1`, binary `28a0c3ecc`). Their outputs differed
([evidence/r1-working-directory/](evidence/r1-working-directory/)): 4,969
`build_identity` leaves of the report and components, 4,834 of the null rows,
10,224 of the contracts sidecar, the reading-site evidence ids, environment
digests and order derived from them, and the source-effect configuration
digests. Each traces to `std::env::current_dir()`: the pinned projection
digests `(unit, current_dir, Clang)` into every row's
`hypothesis_context.build_identity` (`projection.rs`), and the Boolean-field
source import digests it into its configuration (`boolean_source.rs`). With
the hypothesis contexts' and observations' derived fields masked, the 55
differing reading-site evidence lists were equal as sets. So the published
rows depend on the process's working directory, which the identity did not
cover: a group sealed in one directory and reused from another would have
published the first directory's digests. The identity now records the working
directory (`run.working_directory`), and the runs above share one. Measurement
side directories differ per run (`work-corpus` under each measurement), so a
measurement never reuses another's groups until the digests stop binding the
directory (`PINNED-ROWS-BIND-WORKING-DIRECTORY`). The current route's projection binds
its compile-task identity into the hypothesis context instead; whether any of
its checkpointed outputs bind the directory was not checked here.

## 5. Lean

`QCore.StagedCheckpoints` (`proof/QCore/StagedCheckpoints.lean`), audited by
`proof/AuditStagedCheckpoints.lean`: 6 theorems, no placeholders, no project
axioms. The model extends `QCore.Checkpoints` with stages: a checkpoint
records the stage's whole-run facts beside the run identity and fingerprints.
`reused_eq_recomputed` (under `Determined`: a row is a function of the run
identity, the stage's facts and the unit's fingerprint) and
`other_facts_recompute` give a stage's reuse; `stageResumed_eq_stage` lifts it
to every group of a stage for any store of sealed files; `resumed_eq_whole`
shows that with each later stage's facts computed from the previous stage's
rows of every group and accepted or refused on its own rows (the import
stage's table check), the checkpointed run's every decision and final rows are
the whole run's, for any number of stages; `resumed_publication_eq_whole`
composes it with `QCore.Checkpoints.compose_eq_whole`. The Rust boundary (the
identity covers every input; digests are checked; the fold prints the rows
byte for byte) is the fixtures' and §4's.

## 6. What A needs (`MEASURE-LOOKUP-RECORD-REUSE`)

Cross-revision reuse is not attempted: across revisions the analyzer build and
the stage environment differ, so every group recomputes. A can key reuse on
this identity by replacing two parts and fixing one input:

1. **The stage environment digest by a closure-restricted one.** Today
   `stage.environment_sha256` digests the whole contract environment, so any
   contract change recomputes every group. A needs, per unit, the gateway's
   published lookup record (every name looked up, found and missed, the
   `relied` callback candidates, and a whole-map-read flag), and then the
   digest of the environment restricted to the union of the group's units'
   closures (with include/inherit resolution of each name), plus the
   refutation-table entries and imported homes that meet that closure.
   `PinnedIdentity` keeps the field, so A changes what is digested, not the
   format's shape (a schema bump).
2. **The analyzer build by an analyzer-path condition.** A reuses only for
   contract-only diffs, so it needs the measure tool to state that the two
   revisions' analyzer sources are equal (the diff touches only
   `providers/*/contracts/**`, their locks, `contract/**`) and to accept the
   base side's binary identity for that diff, recorded in the reuse decision.

3. **A working directory every side shares, or rows that do not bind it.**
   The pinned rows digest the process working directory (§4.1), and the
   measure tool gives every run a fresh `work-<route>`; until
   `PINNED-ROWS-BIND-WORKING-DIRECTORY` is discharged, a base side's groups
   cannot stand for a tip side's.

Everything else stays: group fingerprints, the run environment, member facts,
the bounds environment, and the per-stage files. The stored groups are the
published rows a reused unit contributes, so A's equality check compares a
composed run with a full one exactly as §4 does.

## 7. Gate

`python3 scripts/gate_scope.py --base 13337d0af --out <worktree>/private/gate`
on `1096cd312` ([gate.json](gate.json)), exit 0: scoped mode, four packages
whole (qcore-audit, qcore-compiler, qcorec, quirin-audit-plugin-measure) and
the test target `qcore-verification::test::composition_bounds`; Lean all (a
proof root changed); no provider gate.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Clippy (and the qcore-verification test target) | 0 / 0 | |
| Rust suite (`rust-suite` slot) | 0 | 200 units, 202 summaries, 2,627 passed, 0 failed, 4 ignored; 7,294.7 s slot wait behind other cards' audits, then 9 min 44 s |
| reevaluation probe / follow-up | 0 / 0 | |
| Lean (`check_lean_audits.py`) | 0 | 411 audits |
| Python suite | 0 | 625 tests |

An earlier gate on `193269b7e` failed Clippy (`large_enum_variant` on the stored
unit and captures, boxed in `28a0c3ecc`); every other step passed there.

## 8. Follow-ups

- `CORPUS-CHECKPOINT-BOUNDS-SELECTION` (new): pinned checkpoints refuse
  `QCORE_BOUNDS_PROPERTY`. The stored fragments already carry bounds rows and
  body evidence and the stage identity the bounds environment; discharge by a
  bounds-selected corpus run checkpointed, resumed and compared byte for byte
  with the whole run, then lifting the refusal.
- `PINNED-CHECKPOINT-INSPECTION` (new): `quirin-audit checkpoint
  verify|decompose|recompose` read current-route files only; discharge by the
  same commands for the pinned layout, or a recorded decision that the route's
  own `--resume` check is the only reader.
- `PINNED-RESUME-FIXED-COSTS` (new): a fully reused corpus run took 14 min
  25 s, of which the checkpointed phases were 90.8 s; the rest is whole-run
  steps every run repeats (member-facts pre-pass over 1,444 units, outer
  reach, pin and bounds evaluations, the sequential definition census and home
  export, the fold and sidecars). Discharge: each step's cost measured, and
  each either checkpointed under its own identity or recorded as the floor.
- `PINNED-CHECKPOINT-PEAK-MEMORY` (new): checkpointed runs peak at 15.8 GB
  (fully reused included) against 10.98 GB whole, above the 11 GiB slot
  admission. Discharge: the stored records held more compactly (or the `first`
  stage's dropped and reloaded only on an import-stage refusal) with the peak
  measured at or under the admission, or the admission raised for
  `--checkpoint-dir` with the measurement.
- `PINNED-ROWS-BIND-WORKING-DIRECTORY` (new): the pinned route's rows bind the
  process working directory (§4.1), so groups reuse only from the same
  directory and measurement runs (fresh side directories) never reuse each
  other's. Discharge: those digests bound to the unit's compile record instead
  of the process directory, with the pinned baselines moved deliberately, or a
  measure-side fixed working directory.
- `MEASURE-LOOKUP-RECORD-REUSE` (new): option A, as §6 states, on the user's
  approval quoted above.
- `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` (amended): D done; A remains.
- `W45-CHECKPOINT-PINNED-REPLAY-ROUTE`: retired by this card (§4).
