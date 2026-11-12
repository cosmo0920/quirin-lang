# MEASURE-LOOKUP-RECORD-REUSE

Round 22 (2026-11-12), base `74105dd0e`. R8 option A, approved by the user
(USER DECISION 2026-11-06, verbatim: "D then A (rec.)", the button being "R8:
option D first (per-group checkpoints for the pinned corpus route), then
option A (gateway publishes per-unit lookups including missed ones; reuse only
for contract-only diffs with matching cross-unit facts). I approve using that
lookup record as a reuse input once it is published and checked."). Option D
landed as [CORPUS-GROUP-CHECKPOINTS](../corpus-group-checkpoints/README.md);
its §6 states what A needs.

Outcome: the mechanism is built and proved; its discharge on the corpus is
blocked by a finding. The gateway publishes a checked per-unit lookup record;
pinned group checkpoints (schema 2) key their stage environment on the
environment restricted to the names their units read; and a contract-only
analyzer acceptance is classified, checked and recorded. On constructed
contract-only diffs the composed run is byte-identical to the full run, and a
consumer that reads by name is invalidated exactly where a read name changed.
But **every unit of today's pinned analysis reads whole contract tables**
(22 accessors, among them the function map and the contract texts each row's
hypothesis context digests), so every corpus group seals `whole` and behaves
exactly as under schema 1: a contract-only card's measurement reuses no group
whose environment changed at all (§4). That is fail-closed by design, and it
is the open work (`GATEWAY-WHOLE-TABLE-READS`, §7). The specification is
[audit checkpoints](../../development/audit-checkpoints.md#the-pinned-corpus-route).

## 1. The lookup record

- **Recorder** (`crates/qcore-schemas/src/contract_lookup_record.rs`). A
  `LookupRecorder` attached to one `ResourceContracts` instance
  (`with_lookup_recorder`) records every read of it and of every clone (a
  clone shares the recorder), on any thread: a read of one name as `found` or
  `missed`; every read that can depend on more than named entries as a
  whole-map read, by accessor. Instrumented: every `&self` accessor of the
  environment (27 whole-table accessors, 10 named ones), the names a mutation
  of a clone consults, copies from another environment (`carry_*`), and the
  three trait reads a derive would hide: serialization (a
  `skip_serializing_if` hook on the recorder slot, which is never written),
  `Debug` (a hand-written impl that reproduces the derived output field for
  field) and equality (the slot is the first field and compares equal after
  recording). The slot is invisible to all three, so an environment with a
  recorder answers, serializes and prints exactly as one without. Unlike the
  rule 13f consultation log it is never rolled back (a declined probe still
  read what it read) and is not thread-local (the gateway's `thread::scope`
  helpers in `null_deref`, `index_extent` and others are covered).
- **Restricted digest** (`ResourceContracts::restricted_sha256`): for each
  name, its entry in every one of the 17 name-keyed tables (or `null`) and the
  two scalars (link-resolution digest, callee-frame acceptance). Refutation
  entries are a name-keyed table, so "the refutation entries meeting the
  closure" are in it. Include/inherit resolution: the tables hold elaborated
  entries, so a parent's change moves every inheriting name's entry; the
  module-keyed texts (`source_assumptions`, `inclusion_evidence`) are
  reachable only through whole-table accessors, so a consumer that read them
  recorded a whole-map read.
- **Gateway record** (`crates/qcore-c-gateway/src/lookup_record.rs`).
  `LookupSession::open(env)` gives the environment to analyse under;
  `scoped` also collects the decided callback candidates the unit relied on
  (`contract_dependencies::relied`, one added line); the published
  `UnitLookupRecord` is `{found, missed, relied, whole_map_read,
  whole_map_reads}`. `check(env)` refuses a flag that disagrees with its
  accessor list, a found name the environment does not hold, and a relied
  candidate never read by name ("its lookups cannot be enumerated").
  `restricted_sha256(records, ..)` answers `None` (key on the whole
  environment) for any record that fails the check or read a whole table.

## 2. The restricted identity (schema 2)

`crates/qcore-audit/src/pinned_route/checkpoints.rs`. Each unit of a
checkpointed stage runs under a session; the session's environment copy is
dropped when the unit ends and its recorder kept, and the records are taken
after the group's postlude, so reads by a clone the unit's products still hold
count. The records are sealed in a new section, `pinned-lookups` (schema 2,
six sections; a schema 1 file is refused and recomputed). `StageIdentity`
gains `environment_scope`:

- at seal, `restricted` when every record checks against the stage
  environment and none read a whole table (`environment_sha256` is then the
  restricted digest over the union of the group's closures), `whole`
  otherwise (schema 1's digest);
- on resume, the identity is recomputed over the **stored** scope: the
  current environment restricted to the stored records' names, or the whole
  current environment. A stored `restricted` claim over records with a
  whole-map read or an unread relied candidate is refused.

Everything else stays: run identity, fingerprints, homes (whole: the gateway
receives them as a slice whose reads are not recorded, and each home's import
identity digests its whole environment), bounds environment, stages. Within one
revision a `whole` group is reused exactly as under schema 1, so D's resume is
unchanged.

**Why the stored-scope comparison is sound** (`QCore.LookupRestrictedReuse`,
§5): a deterministic consumer that read no whole table made every decision
from the entries of the names it read and the scalars; under an environment
agreeing on those it makes the same reads in the same order and returns the
same rows. A changed read name, an added entry for a missed name, or a removed
one changes the restricted digest.

## 3. The contract-only analyzer acceptance

`crates/qcore-audit/src/contract_only_diff.rs`, `quirin-audit pinned
--checkpoint-dir DIR --accept-base-analyzer BASE --repository DIR`. Accepting
the base side's analyzer means the **base binary** measures the tip's contract
data: its SHA-256, build commit and the build commit every row's
`evidence_version` names are then the base side's, so no identity field needs
an exception. (A tip binary with an accepted base identity would publish its
own build commit in every row, so a base group could never stand for it byte
for byte.) Classified before any work, fail-closed:

- every path of `git diff --name-only --no-renames BASE HEAD` is contract data
  (`contract/**`, `providers/*/contracts/**`, a provider's `contracts.lock`,
  `bounds-contracts.lock`, `outer-contracts.lock`, `contract-inventory.lock`;
  never Rust, a manifest or a build script);
- no such path is compiled into an analyzer: the base tree's non-test root
  Rust sources' `include_str!`/`include_bytes!`/`include!` targets, resolved
  relative to the file or (for `concat!(env!("CARGO_MANIFEST_DIR"), ..)`) the
  crate; an include of any other form refuses the classification. The
  pinned route compiles in three provider contracts (`source_projection.qv`,
  `chunkio/error_state.qv`, `monkey/wrapper_paths.qv`) and the gateway
  `libc/errno.qv`; a diff touching one is refused. The provider workspace is
  not an analyzer: its crates' embedded contracts reach a run only through a
  provider package whose files the run identity digests.

Once the contract inputs are resolved the route checks the run: build commit
equal to `BASE`, the repository at the classified tip and clean in contract
data, and the manifest, every lock, the source profile and the source-contract
configuration read from the repository. The acceptance is printed, written to
the run record (`analyzer`) and into every reused and sealed decision.

Sampled real history (`sampled_real_commits_are_classified`): `94bca8d9c` (a
contract comment and its inventory lock), `df6172543` and `ca9ee5d93` (lock-only
pin refreshes) are accepted; `74105dd0e` (licence headers, which also touched
test fixtures) is refused. Classified on this tree, `94bca8d9c~1..94bca8d9c`
was refused at first because `providers/quirin-provider-libc/src/lib.rs`
compiles in `result_sets.qv`; the provider workspace exclusion above is the
decision that followed.

## 4. Evidence

All on this card's tree, Clang 21.1.8 (`clang-21_1:21.1.8-6ubuntu1`, unpacked
privately: this host has no system Clang, and neither the corpus inputs
(`build/cycle19`) nor the system and slice preparations; §6).

**Composed equals full on constructed contract-only diffs**
(`crates/qcore-audit/src/pinned_route/checkpoints/lookup_tests.rs`). Each
scenario seals every group of a stage under BASE (real `seal`, real files),
then measures TIP twice: composed (each group `load`ed with the identity
recomputed over its stored scope, else recomputed) and full; the composed
groups' `pinned-units` and `pinned-lookups` sections must equal the full run's
byte for byte.

| Consumer | Diff | Sealed scope | Reused (alpha, beta, gamma) | Composed = full |
|---|---|---|---|---|
| real pinned unit analysis (`audit_unit`, real Clang, three one-unit libraries) | A: a contract file outside the loaded set (environment byte-identical) | whole ×3 | yes, yes, yes | yes |
| same | B: `keep`, called by alpha only, borrowed → released | whole ×3 | no, no, no | yes |
| name-only consumer (reads the real environment by `contract(name)`) | none | restricted ×3 | yes, yes, yes | yes |
| same | `keep` (alpha found) changes | restricted | no, yes, yes | yes |
| same | `probe` (beta missed) declared | restricted | yes, no, yes | yes |
| same | an unread declaration added (module text changes) | restricted | yes, yes, yes | yes |
| same | `touch` (beta found) removed | restricted | yes, no, yes | yes |
| name-only consumer that also reads `functions()` | an unread declaration added | whole ×3 | no, no, no | yes |

The real analysis' record for a one-call unit (`alpha`, calling `keep`)
reads 22 whole tables: `cell_protocols`, `contract_conflict`,
`cross_unit_import_identities`, `declared_guarded`, `declared_revisions`,
`functions`, `guarded_retaining`, `inclusion_evidence`, `inferred_imports`,
`inferred_lifetimes`, `lifetime_declarations`, `lifetime_imports`,
`lifetime_summaries`, `member_effects`, `pending_review`, `pin_mismatch`,
`refuted_by_body`, `relational`, `result_ranges`, `retaining`,
`source_assumptions`, `validate_lifetime_partition`. Textual non-test call
sites in the gateway, audit, contract and source-analysis crates: 63 of
`source_assumptions()`, 44 `retaining()`, 43 `functions()`, 16
`relational()`, 15 `guarded_retaining()`, 12 `inferred_imports()`, and 2–10
each for the rest. Two of them bind published bytes, not only decisions: each
row's `hypothesis_context.assumptions` is the SHA-256 of the whole
`source_assumptions` map (`crates/qcore-audit/src/projection.rs`), and
`loop_lifetime_source` digests it into its keys. So any change to any loaded
contract text changes every row of every unit, and restriction cannot be
byte-identical until those digests are restricted too, a row change that moves
the pinned baselines (out of this card's scope).

**Fixtures** (evidence item: invalidation): the table's name-only rows are
`a_contract_change_invalidates_exactly_the_group_that_read_the_name`; a
whole-map reader always reruns (`a_whole_map_reader_always_reruns_on_any_environment_change`);
a stored restricted claim over records that cannot support it is refused, and
an unchecked record seals whole
(`a_stored_restricted_claim_the_records_cannot_support_is_refused`); a
non-contract diff, a compiled-in contract and an unenumerable include refuse
the base analyzer, and `verify_run` refuses a tip build commit, an input outside
the tip checkout and uncommitted contract data
(`only_a_diff_confined_to_contract_data_not_compiled_in_accepts_the_base_analyzer`);
the CLI refuses a non-contract diff before any replay
(`a_base_analyzer_for_a_non_contract_diff_is_refused_before_any_replay`) and
malformed option sets. Recorder and gateway tests: named reads (found/missed,
on a helper thread, through a clone), whole reads (an accessor,
serialization, `Debug` and `{:#?}` byte-equal to an unrecorded environment,
equality), the restricted digest's coverage, and the record check.

**Configured routes** (system, slice via `current`). Not extended: the
current route's checkpoints key on the whole environment too, the same
gateway reads the same whole tables there, and a restricted current-route
identity would therefore never be used. Registered as
`CONFIGURED-ROUTE-LOOKUP-REUSE` rather than built unused.

## 5. Lean

`QCore.LookupRestrictedReuse` (`proof/QCore/LookupRestrictedReuse.lean`),
audited by `proof/AuditLookupRestrictedReuse.lean`: 12 theorems, no
placeholders, axioms within `propext`/`Quot.sound`. A unit's analysis is a
consumer `Prog K V R` of the stage environment `K → V` (`ask` one name,
`whole` read); its record is `keys`/`readsWhole`. `run_eq_of_agree`: a run
with no whole read, under an environment agreeing on its keys, makes the same
reads and returns the same result. A `Consumer` (run identity, fingerprint,
the program its analysis is) instantiates `QCore.StagedCheckpoints.Analysis`
with the stage environment as the facts (`Consumer.determined` discharges
D's premise). `sealGroup` seals `restricted (restrict f keys)` when no unit read
the whole environment and `whole (digest f)` otherwise; `resume` recomputes the
scope over the stored one (`current`). `reused_eq_recomputed` (premise:
`wholeSound`, equal whole digests are equal environments) gives the recomputed
rows for any sealed checkpoint; `changed_key_recomputes` and
`whole_reader_recomputes` are the invalidations; `stageResumed_eq_stage`,
`resumed_eq_whole` and `resumed_publication_eq_whole` lift it to every group of
every stage and the publication through `QCore.StagedCheckpoints.whole` and
`QCore.Checkpoints.compose_eq_whole`. The Rust boundary (the recorder sees
every read; the digest covers each read name in every table) is §1's tests.

## 6. Measurement

[PREDICTION.md](PREDICTION.md) (committed before the pinned-route, gateway and
measure code; only the evidence-only recorder preceded it) declares the
identity on all three live routes. The arbiter comparison could not run on
this host: `quirin-audit measure --declared` verifies every prepared input of
the three live profiles before anything runs, and none exists here (no
`build/cycle19`, no `private/w45-fb/...` preparations, no system Clang at
`/usr/lib/llvm-21/bin/clang`). The attempt and its refusal are recorded in
[measure-attempt.txt](measure-attempt.txt). The identity rests on the code: a
run without `--checkpoint-dir` executes the code it executed before (the only
change on that path is the recorder slot, invisible to serialization, `Debug`
and equality, and one evidence-only `relied` note), and the checkpointed path
reuses a group only under §2's identity. The integrator's landing measurement
is the arbiter.

## 7. Follow-ups

- `GATEWAY-WHOLE-TABLE-READS` (new): every pinned unit reads 22 whole contract
  tables, and rows bind the whole contract text set
  (`hypothesis_context.assumptions`, the lifetime-source key), so no corpus
  group can key on a restricted environment. Discharge: the analysis' reads
  narrowed to named lookups (or each whole read replaced by a recorded named
  equivalent) and the row digests restricted to the texts of the modules the
  unit's names come from, with the pinned baselines moved deliberately; then a
  real corpus unit seals `restricted`.
- `MEASURE-CONTRACT-ONLY-SIDE-WIRING` (new): `quirin-audit measure` does not
  yet use the acceptance: for a contract-only diff it would run the base
  binary over a tip checkout with the base side's checkpoint directory,
  from one working directory (`PINNED-ROWS-BIND-WORKING-DIRECTORY`).
  Discharge: that wiring, with a sampled landing's composed comparison equal
  to a full run; useful once `GATEWAY-WHOLE-TABLE-READS` lets a group seal
  restricted.
- `CONFIGURED-ROUTE-LOOKUP-REUSE` (new): system and slice (`current`) keep the
  whole-environment identity (§4).
- `PINNED-ROWS-BIND-WORKING-DIRECTORY` (amended): not discharged; it blocks
  only the measure-side wiring, which `GATEWAY-WHOLE-TABLE-READS` blocks first.
  The fixtures and the acceptance do not need it.
- `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` (amended): A's mechanism is in;
  its corpus effect waits on the two follow-ups above.
- `MEASURE-LOOKUP-RECORD-REUSE`: retired by this card. The mechanism, its
  proof and the constructed evidence are here; the corpus discharge ("a
  contract-only card's measurement reuses unaffected groups") is rehomed to
  `GATEWAY-WHOLE-TABLE-READS` and `MEASURE-CONTRACT-ONLY-SIDE-WIRING`, and
  the configured routes to `CONFIGURED-ROUTE-LOOKUP-REUSE`.
- Untouched: `PINNED-RESUME-FIXED-COSTS`, `PINNED-CHECKPOINT-PEAK-MEMORY`
  (the records add a few names per unit; the session's environment copy is
  dropped at unit end), `CORPUS-CHECKPOINT-BOUNDS-SELECTION`,
  `PINNED-CHECKPOINT-INSPECTION` (schema 2 is still read only by the route).

## 8. Gate

See [gate.json](gate.json).
