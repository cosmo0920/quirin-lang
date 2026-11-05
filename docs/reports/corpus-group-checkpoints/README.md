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
| run | analyzer build (binary SHA-256, build commit, profile); Fluent Bit root and verified revision; every file of the source-set profile directory; the source-contract configuration and the ChunkIO package's files; contract manifest, every lock read, the pin template; Clang identity; every `QCORE_*` variable except the job count, with the null, outer and member-prepass output paths reduced to `selected` and the link map to its content digest; the run's member facts | every group of every stage |
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

RESULTS_PLACEHOLDER

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
this identity by replacing exactly two parts:

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

Everything else stays: group fingerprints, the run environment, member facts,
the bounds environment, and the per-stage files. The stored groups are the
published rows a reused unit contributes, so A's equality check compares a
composed run with a full one exactly as §4 does.

## 7. Gate

GATE_PLACEHOLDER

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
- `PINNED-RESUME-FIXED-COSTS` (new): FIXED_COSTS_PLACEHOLDER
- `MEASURE-LOOKUP-RECORD-REUSE` (new): option A, as §6 states, on the user's
  approval quoted above.
- `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` (amended): D done; A remains.
- `W45-CHECKPOINT-PINNED-REPLAY-ROUTE`: retired by this card (§4).
