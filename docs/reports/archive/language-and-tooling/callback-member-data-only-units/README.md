# CALLBACK-MEMBER-DATA-ONLY-UNITS: every unit read for the member closure

Card CALLBACK-MEMBER-DATA-ONLY-UNITS (wave 48, round 26), gateway rule
**12n** amended in place (no new id), contract specification **C04.19.3**
amended in place, Lean `QCore.OpCallbacks` extended. Base: local `main` at
`f3e1d724c`; the census and the frozen [prediction](PREDICTION.md) were
committed before any implementation code.

## 1. The defect and the census

Rule 12n's fold (`crates/qcore-audit/src/callback_member_fold.rs`) opens every
member when a unit row recorded no member facts (premise
`P-CG22-MEMBER-CLOSED-WORLD`). Facts reached the fold only through rule 13y's
`settle` hook, which runs inside the gateway's analysis, and
`projection::project_prepared_unit_into` / `project_unit_into` return before
any analysis for a unit whose definition inventory is empty. On the corpus
four such units opened both op members for the run (BEFORE sidecar,
[artifact](artifacts/callback-members-corpus-before.json): "4 analysed
unit(s) recorded no facts"):

| Unit | Why it is data-only | Function-pointer content |
| --- | --- | --- |
| `ctraces/ctr_utils.c` | an empty file (0 bytes at `66910c10`) | none |
| `librdkafka/opentelemetry/common.pb.c` | nanopb descriptors only (`PB_BIND`, `PB_STATIC_ASSERT`) | `pb_msgdesc_t` initializer lists, some naming `pb_default_field_callback` |
| `librdkafka/opentelemetry/metrics.pb.c` | the same | the same |
| `librdkafka/opentelemetry/resource.pb.c` | the same | the same |

None stores into, initializes or takes the address of a `struct
rd_kafka_op_s` member.

## 2. What the card built

* **A data-only unit is read whole for its facts alone.**
  `callback_member_closure::read_unit` runs Clang's whole-unit JSON AST over the
  unit's preprocessed text with its dialect and frontend flags (the input an
  analysed unit's index is read from), read within rule 13o's depth bound
  (`deep_unit_ast::from_slice`), and collects `MemberFacts` from every
  top-level declaration with the collector rule 12n's in-unit decision uses.
  The root must be a `TranslationUnitDecl` with a declaration array; anything
  else is an error. `callback_member_fold::data_only_facts` calls it from both
  data-only branches of `projection`, only when a declaration invokes a
  member; a unit whose preprocessing, Clang run or JSON fails records `None`,
  which still opens every member. A unit read with no store contributes the
  empty set, read from its AST.
* **Initializer lists are mapped onto their record's fields.** Before, any
  initializer list holding a function-pointer value opened its record, so a
  data-only unit's initializer could never contribute a candidate. Now a list
  of a record the unit defines once at file scope is mapped entry by entry
  (Clang's semantic form: one entry per named field, unnamed bit-fields
  skipped; a union by its `field`): each function-pointer field's entry is a
  store into `TAG.FIELD` (an implicit entry is a null pointer). A list that
  cannot be mapped exactly keeps the old reading and opens the record: an
  anonymous member, a tag defined again inside a function, an entry count
  that differs, a union with no `field`, or another field's entry that is a
  function, a function pointer, or a record or array value (a copy that may
  carry one). The mapping is used for analysed units too, so an in-unit
  decision now sees an initializer's candidate instead of refusing the whole
  member as open (`tests/callback_member_data_only.rs`: an effectful
  candidate stored only by a designated initializer refuses with
  `callback-effectful`).
* The fold itself is unchanged and stays demote-only.

Tests: `crates/qcore-c-gateway/tests/callback_member_data_only.rs` (designated,
positional and implicit entries; a unit with no store and the empty file
close; a union designator; four unmappable lists open the record; an unread
unit is an error; the analysed-unit decision), and
`callback_member_fold`'s `a_data_only_unit_is_read_from_its_ast` (through
`project_unit_into` and the fold: a data-only initializer's candidate joins the
run's set, a data-only unit with no store and an empty unit close it, a unit
whose preprocessing fails keeps it open and is named) and `run_vectors_hold`.

## 3. Measurement

Release binaries: BEFORE `aed30fc6…` (built from `git archive f3e1d724c`
into `private/mdo/base-src`, with the libevent submodule cloned at its pinned
`d82464a2`), AFTER `a1` `d550f4f6…` (tip `3fc08a102` code). Each run's
`run.json` `binary_sha256` matches. Null property selected on every run.
Totals and digests: [summary](artifacts/summary.json).

| Route | BEFORE | AFTER |
| --- | --- | --- |
| Corpus (`66910c10`, `--jobs 4`) | 935 / 9 / 3,890, `527a4cad…` | 935 / 9 / 3,890, `527a4cad…` (byte-identical) |
| System | 93 / 4 / 351, `51498d50…` | identical |
| Slice | 52 / 1 / 347 / 4 Pending, `acfab4cf…` | identical |
| Null property (corpus / system / slice) | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | rows and tallies byte-identical |

**The fold section** (corpus sidecar, [AFTER](artifacts/callback-members-corpus.json)):
`units_without_facts` 4 → **0**; `rd_kafka_op_s.rko_op_cb` **closed over 27**
designators and `rd_kafka_op_s.rko_serve` **closed over 4**, each exactly the
set `members_over_recorded_units` listed before; `demoted` empty. Contract
dependency counts and the Safe roll-up are unchanged on every route. The
system and slice selections hold no librdkafka and no data-only unit: their
sections were closed and empty before and after.

**Prediction against outcome:** every predicted figure held (closure 27 and
4, no verdict move, report and null rows byte-identical). No Safe gain, no
Safe → non-Safe move, no added leak or violation.

**Wall time:** corpus 585.5 s BEFORE, 526.0 s AFTER; system 83.5 / 79.8 s;
slice 43.0 / 39.7 s, each timed inside the heavy lock on a host shared with
two other cards, so the difference is load, not the card. The added work is
one whole-unit Clang AST read for each of the corpus's 4 data-only units.

**Discarded attempts** (retained under `private/mdo/`, not evidence): the
first BEFORE corpus run exited 1 because the exported tree had no libevent
checkout (`SourceRevisionMismatch`, the pin check found the worktree's head);
the first `a1` build failed (exit 101) because it shared a target directory
with the exported tree and cargo read the base gateway crate as fresh
(`build.sh` now gives an export its own target directory).

## 4. Lean

`proof/QCore/OpCallbacks.lean`, section "Every unit read", audited by
`proof/AuditOpCallbacks.lean` (20 theorems; standard axioms only, no
placeholder, no project axiom): a unit's reading is `none` (its AST not
read) or the closure its facts give; `runClosure` joins them.
**`an_unread_unit_keeps_the_member_open`** and `an_unread_unit_demotes` (an
unread unit opens the member and the fold demotes every decision resting on
it); `a_read_unit_without_a_store_adds_nothing`;
**`the_run_set_is_exactly_the_read_sets`** (when the run is closed its set is
exactly the designators the units read: a data-only initializer's candidate
is in it and nothing else is); `run_vectors_hold` (bound to
`callback_member_fold`'s `RUN_VECTORS`). Abstract rule theorems, not an
extraction of the Rust reader.

## 5. Ledger

`CALLBACK-MEMBER-DATA-ONLY-UNITS` is **retired**: its DISCHARGE ("member facts
collected from a unit the gateway does not analyse (its initializers and
stores), so the run's closure is exactly the recorded one where every unit is
read; measured: both members closed") holds: section 3. Registered:
`CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS` (below).

**Follow-on found by reading** (not changed here): a store through an
embedded record is keyed by the outermost record (`x.inner.cb` and
`p->inner.cb` are `outer.inner.cb`), so a member spelled `inner_tag.cb` does
not see it, and a record value copied inside an initializer list that holds
no function-pointer value opens nothing. Corpus size for the declared
members: 0 (`struct rd_kafka_op_s` is embedded by value nowhere in
librdkafka; every store is `rko->…`).

## 6. Gate

Standing gate (`docs/agents/validation.md`), code steps at `60972816b` (the
tip's code; later commits change records only), run by
[gate.sh](scripts/gate.sh) `g1 code` under one exclusive flock on the shared
heavy lock, logs `private/mdo/gate-g1/` of this worktree; exits in
[gate/](gate/), summary in [gate.json](gate.json):

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update `--init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 491 units, 504 summaries; **7,996 passed; 0 failed; 14 ignored** (`private/mdo/tests-g1/`), no rerun |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **318** Lean audits (unchanged: an existing module extended) |
| `python3 -m unittest discover -s scripts/tests` | 0, 0 | 365 OK after the records (08, at `9f4f90c68`) and 365 OK after the matrix regeneration (09b) |
| `build_matrix.py . --gate gate.json` | 0 | 09a (a first attempt exited 1: `gate.json` lacked `provider_tests`; it now states why the provider gate is not due) |
| `python3 scripts/build_docs_index.py --check` | 0, 0 | 09, 10c (10b regenerated it, exit 0) |

`providers/` is unchanged, so the provider gate does not apply. Clang:
`Ubuntu clang version 21.1.8 (6ubuntu1)` ([recording](gate/clang-version.txt)).

## Files

* `crates/qcore-c-gateway/src/callback_member_closure.rs` (`read_unit`,
  initializer mapping), `crates/qcore-audit/src/callback_member_fold.rs`
  (`data_only_facts`, tests), `crates/qcore-audit/src/projection.rs` (two
  hooks), `crates/qcore-c-gateway/tests/callback_member_data_only.rs`.
* `proof/QCore/OpCallbacks.lean`, `proof/AuditOpCallbacks.lean`.
* `docs/agents/gateway.md` (12n), `docs/agents/README.md` (router row),
  `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.3).
* Scripts (adapted from CG22-OP-CALLBACKS-AND-SERVED-QUEUES'):
  [build.sh](scripts/build.sh), [measure.sh](scripts/measure.sh),
  [measure-binary.sh](scripts/measure-binary.sh),
  [build-measure.sh](scripts/build-measure.sh),
  [summarize.py](scripts/summarize.py), [totals.py](scripts/totals.py),
  [lean.sh](scripts/lean.sh), [gate.sh](scripts/gate.sh),
  [gate-whole.sh](scripts/gate-whole.sh),
  [finish-gate.sh](scripts/finish-gate.sh),
  [write_gate_json.py](scripts/write_gate_json.py),
  [update_ledger.py](scripts/update_ledger.py).
* Artifacts: [summary](artifacts/summary.json), member sections
  [before](artifacts/callback-members-corpus-before.json) and
  [after](artifacts/callback-members-corpus.json).
