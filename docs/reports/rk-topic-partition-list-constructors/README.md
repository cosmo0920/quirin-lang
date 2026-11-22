# RK-TOPIC-PARTITION-LIST-CONSTRUCTORS: the topic-partition constructors

Card RK-TOPIC-PARTITION-LIST-CONSTRUCTORS (USER DECISION 2026-11-21: launch
the card the ledger entry of that name specifies, registered by
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4). Base `8e7e8d509`
(integrate/r48). [PREDICTION.md](PREDICTION.md) was committed (`141e0bccd`)
before the declarations (`e03c4835f`). No Rust, gateway rule or Lean
change: five pinned declarations in a new module.

## 1. Bodies read (step a)

At Fluent Bit `66910c10` (a shared clone of the local repository) and
`046accf6` (the prepared checkout); `rdkafka_partition.c`,
`rdkafka_partition.h`, `rdkafka.h` and `rd.h` are byte-identical at both.

| function | body (`rdkafka_partition.c`) | effect on the caller |
|---|---|---|
| `rd_kafka_topic_partition_list_new(int size)` | `:2843-2852`: `rd_calloc` list, `_grow` (static) `rd_realloc`s its element array | owns one list |
| `rd_kafka_topic_partition_list_copy(src)` | `:3282-3292`: `_list_new(src->size)`, then `_add_copy` per element (strdup, metadata `rd_malloc`, a toppar reference into the new element's `_private`) | owns one list; `src` read |
| `rd_kafka_topic_partition_copy(src)` | `:2933-2940`: `rd_kafka_topic_partition_new` then `_update` (metadata, toppar reference into `dst`) | owns one partition; `src` read |
| `rd_kafka_topic_partition_new(topic, partition)` | `:2866-2873`: `rd_calloc`, `rd_strdup(topic)` | owns one partition; `topic` read |
| `rd_kafka_topic_partition_destroy(rktpar)` | `:3108-3110`: `rd_kafka_topic_partition_destroy0(rktpar, 1)` | releases it (nonnull) |
| `rd_kafka_topic_partition_list_destroy(list)` | `:3117-3128` | already declared released (`client.qv`) |

`rd_calloc`/`rd_malloc`/`rd_realloc`/`rd_strdup` assert their result
(`rd.h:139-170`), so every constructor's result is `nonnull`.

**The reference half (decision class 3), decided as encapsulation.** The
copies take toppar references (`rd_kafka_toppar_keep`, `:2912-2913` and
`_add0`), but store them only in the result's own elements; the result's
destructor (`_list_destroy` through `destroy0(&elems[i], 0)`, or
`rd_kafka_topic_partition_destroy`) drops them. No reference is returned to
the caller and none the caller holds is taken or dropped, so the caller's
one obligation is the result's release: `resource_return_owned`. `src` is
borrowed: a counted reference keeps the toppar, not `src`. This is the
reading the list destructor's existing declaration already makes.

**Left undeclared: `rd_kafka_topic_partition_list_add`** (`:3190-3196`).
It returns `&rktparlist->elems[cnt++]`, a pointer into an array the next
add may reallocate (`_grow`); `resource_return_borrowed_from(rktparlist, …)`
would keep it valid until the list's release, which is false after a later
add, so the declaration would admit in the unsafe direction. It is the most
frequent co-blocking callee (22 of the 68 base rows), but with the card's
functions it alone blocks only 2 (`ut_topic_partitions`,
`ut_create_topic_partition_lists`). Registered:
RK-TOPIC-PARTITION-LIST-ADD-ELEMENT-POINTER.

## 2. Declarations (step b)

`providers/qcore-provider-librdkafka/contracts/librdkafka/topic_partition_constructors.qv`,
its own module (so card CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT's librdkafka
declarations do not touch the same source): the four constructors
`resource_return_owned(nonnull)` with every formal `resource_borrowed`; the
destructor `resource_released(rktpar, nonnull)`, `resource_return_none()`,
naming `rk_plugin_callback_closure` as `rd_kafka_topic_partition_destroy0`
does (the release reaches `rd_kafka_toppar_destroy`'s final drop, which may
log). Each declaration quotes its body and states its ABI.
`rd_kafka_topic_partition_new` was added beside the entry's three because
the callers of `rd_kafka_topic_partition_destroy` largely build their
partition with it (11 base rows name it uncontracted, 7 beside the
destructor).

Registered in `cycle19-resource-contracts.json` (by hand) and
`providers/contract-load-path.json` (by `contracts.py sync`). Pinned with
`quirin-audit pin` ([scripts/pin.sh](scripts/pin.sh), logs
`private/rk/pin-2/`): `pin` 0 at `66910c10` and 0 at `046accf6`; `pin
--check --crate qcore-provider-librdkafka` 0 at `66910c10`, 1 at `046accf6`
only for `rd_kafka_log0`/`rd_kafka_log_buf` (in force through their
`66910c10` identity, the pre-existing state round 4 recorded; 237 recorded,
0 not in force, 0 stale); `pin --verify-contracts` 0. `providers/contracts.py
sync` regenerated `contract-inventory.lock` (one module, five
declarations, 16 clauses, every one in the `functions` table),
`contracts.json` and the load path; `providers/contracts.py check` 0
(`private/rk/check-1.log`). `memory.qv`, `client.qv` and `null_deref.qv`
(frozen by PREMISE-SYMBOLIC-ADOPTION's records) are byte-identical to the
base.

**Validation against the bodies** (the tip report's
`contract_body_evidence` for `rdkafka_partition.c`):

| declaration | result |
|---|---|
| `rd_kafka_topic_partition_new` | **validated** (W4, entry): every clause |
| `rd_kafka_topic_partition_destroy` | **validated** (W4, entry): released, none; the premise is stated, not checked |
| `rd_kafka_topic_partition_list_new` | **not validated**: its row is `Unknown` on the uncontracted static `rd_kafka_topic_partition_list_grow` (a `rd_realloc` of a member) |
| `rd_kafka_topic_partition_list_copy` | **not validated**: `Unknown` on an access leaf (`_add_copy` into `dst`'s elements) |
| `rd_kafka_topic_partition_copy` | **not validated**: `Unknown` on the uncontracted static `rd_kafka_topic_partition_update` |

The three unvalidated declarations are trusted assumptions pinned to the
text read (registered: RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED).

## 3. Measurement (step c)

Route and inputs: [evidence/runs.md](evidence/runs.md) (opt-in `current
--monkey-linked` over Fluent Bit 046accf6, one binary `ff817b5c…` built at
the base, each run reading its own side tree).

| run | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---:|---:|---:|
| base `8e7e8d509` | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| **tip `e03c4835f`** | **1,161 / 2 / 3,632 / 9** | **2,129** | **211** | **8,991** |

**Four moves** ([evidence/moved-rows.jsonl](evidence/moved-rows.jsonl)):

| row | move | why |
|---|---|---|
| `rdkafka_partition.c::rd_kafka_topic_partition_copy_void` | Unknown -> Safe | returns `rd_kafka_topic_partition_copy(src)`: an owned-result summary on the unvalidated declaration |
| `rdkafka_partition.c::rd_kafka_topic_partition_list_copy_opaque` | Unknown -> Safe | returns `rd_kafka_topic_partition_list_copy(src)`: the same, on `_list_copy` |
| `rdkafka_sticky_assignor.c::ut_testSameSubscriptions` | **Unknown -> Unsafe** (provisional) | four leak leaves on `subscription` (`_list_new`, `:2967`) |
| `rdkafka_mock_handlers.c::rd_kafka_mock_handle_Metadata` | **Unknown -> Unsafe** (provisional) | one leak leaf on `requested_topics` (`_list_new`, `:1362`) |

**No Safe -> non-Safe move; Pending 2, the same rows.** Both Safe moves rest
on unvalidated declarations (declared-assumption Safe rows 536 -> 538).

**The two provisional Unsafe rows**, each read in its body; neither is an
upstream defect claim:

* `ut_testSameSubscriptions` (a librdkafka unit test). The leaks are at
  `:2999`, `:3001`, `:3012`, `:3014`: two `RD_UT_ASSERT(!err, …)` and two
  `verifyValidityAndBalance(…)` (`rdkafka_assignor.h:314-319`), each of
  which `return 1`s on a test failure without destroying `subscription`
  (nor the metadata or the members, which the projection does not track
  here). The paths exist in the C text, but only when the test itself
  fails; it is a test-failure path of unit-test code, reported because the
  list is now tracked and the row's other leaves are Unknown (the join puts
  Unsafe above Unknown).
* `rd_kafka_mock_handle_Metadata` (the mock cluster). **Spurious on
  reading**: `requested_topics` starts `NULL` and is assigned only when
  `TopicsCnt > 0`; both exits release it under `if (requested_topics)`
  (`:1485-1486`, `:1493-1494`), and every parse failure jumps to
  `err_parse`. The analysis does not correlate the `NULL` initialisation,
  the conditional constructor and the guard, so at both releases the
  variable is a nullable resource (`nullable resource \`requested_topics\`
  used by a non-null contract`: `rd_kafka_topic_partition_list_destroy` is
  declared `nonnull`), the release is refused, and the obligation is still
  live at `return 0` (`:1490`). A refused release reaching a leak verdict
  rather than holding the row `Unknown` is an analysis imprecision, not a
  leak (registered: NULLABLE-GUARDED-RELEASE-REPORTED-AS-LEAK).

**Leaves** ([evidence/changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt),
83 rows change, 79 stay Unknown). Removed: the 83 `call to uncontracted
function` leaves naming the five functions (in 68 rows: 44 `_list_new`, 14
`rd_kafka_topic_partition_destroy`, 12 `_list_copy`, 12
`rd_kafka_topic_partition_new`, 1 `rd_kafka_topic_partition_copy`), 45
`unresolved resource identity` leaves and 15 access leaves. Added on rows
that stay Unknown (every one `unsupported`, so fail-closed): `nullable
resource … used by a non-null contract` 20 in 11 rows (the guarded-release
pattern above; three of them in the Metadata row), `unresolved resource
identity` 10 in 8, `member-store-through-formal` 9 in 9 (an owned list
stored into a formal's member: `rd_kafka_cgrp_group_assignment_set`,
`ut_init_member_internal`, …), `member-release-through-formal` 4 in 4 (a
member's partition released: `rd_kafka_ListOffsetsResultInfo_destroy`,
`rd_kafka_share_partition_offsets_destroy`, …), `discarded owned result of
resource API` 4 in 4 (`*partitions = rd_kafka_topic_partition_list_new(0)`
in `rd_kafka_assignment`, `rd_kafka_subscription`: an owned result
delivered through an out-parameter, read as discarded and refused),
`unmodeled expression may mutate a formal resource identity` 4, `loop
re-acquisition` 4 (the sticky-assignor unit tests), access 2. The five leak
leaves are all in the two Unsafe rows. Access rows 2,135 -> 2,129 and
access leaves 9,027 -> 8,991: the Metadata row's access leaves leave the
Unknown count with the row, and 15 removed against 2 added elsewhere.

**Round 4's four rows are unchanged.** `rd_kafka_DeleteRecords_new`,
`rd_kafka_ElectLeaders_new`, `rd_kafka_ListOffsetsResultInfo_new` and
`rd_kafka_assignment_init` stay `Unknown` on their access leaves: the
lexical gate refuses the member store before the call's contract is
looked up, so the constructor's declaration does not reach them (an owned
result into a member is MEMBER-EFFECT-STORES').

**Against the prediction.** The two declared moves happened. The possible
extras (`rd_kafka_ListOffsetsResultInfo_destroy`,
`rd_kafka_share_partition_offsets_destroy`, the three round-4 constructors)
did not: the first two now stop on `member-release-through-formal`, the
others on the access gate. Safe -> non-Safe 0, as predicted. New Unsafe: 2
(predicted 0, "at most a few possible"); both explained above.

**The balance, plainly.** +2 Safe (both on unvalidated declarations), +2
provisional Unsafe (one a unit test's assertion-failure path, one spurious),
0 Safe lost, 83 uncontracted leaves converted into custody leaves that name
the next obstacle (the guarded release, member stores and releases, the
out-parameter delivery, `rd_kafka_topic_partition_list_add`). Whether to
land with the spurious Unsafe is the integrator's call; the declarations
are honest, and the spurious row is an analysis imprecision the
declarations expose rather than cause.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). Prediction for the
integrator: corpus (and twelve-home), which analyses librdkafka at
`66910c10` with byte-identical bodies and the declarations pinned there: the
same two Unknown -> Safe rows, and the two Unknown -> Unsafe rows where the
route analyses `rdkafka_sticky_assignor.c` and `rdkafka_mock_handlers.c`
(the measure-prediction block lists only the two Safe moves, written before
the measurement, so expect the two Unsafe moves as unpredicted). System and
slice: none.

## 4. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 8e7e8d509 --out
private/rk/gate-1` through `private/toolchain/env.sh` (Clang 21, Lean 4.35),
head `e03c4835f` (the declarations; later commits change this report only);
scoped mode (no crate source changes; the contract and load-path files
select 42 test targets), provider gate included, Lean not selected (no Lean
or rule change). Record [gate/gate-1.json](gate/gate-1.json), logs in the
worktree's `private/rk/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy` (workspace and the four scoped crates) | 0 | |
| `run_test_binaries.py` (scoped) | 0 | 272 units, 279 summaries; 3,432 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 -m unittest discover -s scripts/tests` | **1** | 779 tests, 1 failure: `test_build_matrix_sections.CommittedMatrixIsSelfConsistent.test_each_derived_dimension_matches_the_section_it_counts` (`dimensions.named_followups is 1041 against a named_followups of 1044`) |
| provider package build / verify-all / verify | 0 / 0 / 0 | |
| provider `cargo test` / fmt / clippy | 0 / 0 / 0 | 49 summaries, 168 passed, 0 failed, 1 ignored |

**The Python failure is the base's, not this card's.** This card does not
touch `docs/reviews/r0_3/acceptance_matrix.json`; the same assertion fails
on `git archive 8e7e8d509` with the card base set
(`QUIRIN_CARD_GATE_BASE=8e7e8d509… python3 -m unittest
scripts.tests.test_build_matrix_sections`, `private/rk/python-base-matrix.log`:
33 tests, 1 failure, the same `1041 != 1044`). Round 4's amendments
registered three entries on integrate/r48 without regenerating the matrix's
derived dimensions (`docs/reports/remaining-d02/build_matrix.py`), which
the integrator's regeneration moves. No Lean audit was added, so the
derived audit count is unchanged by this card.

## 5. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **RK-TOPIC-PARTITION-LIST-CONSTRUCTORS** retired: the constructors are
  declared with the reference half decided, every caller's move reported.
* **RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED** (new): `_list_new`,
  `_list_copy` and `rd_kafka_topic_partition_copy` are trusted assumptions.
* **RK-TOPIC-PARTITION-LIST-ADD-ELEMENT-POINTER** (new): no contract form
  states a result borrowed from a container only until the container's next
  mutation.
* **NULLABLE-GUARDED-RELEASE-REPORTED-AS-LEAK** (new): a `NULL`-initialised,
  conditionally constructed resource released under `if (p)` is a nullable
  resource at a `nonnull` release; the refused release leaves a leak verdict
  (`rd_kafka_mock_handle_Metadata`), and 20 such leaves hold 11 rows.
* **OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY** (new): `*out =
  rd_kafka_topic_partition_list_new(0)` is read as a discarded owned result
  (4 rows); related to CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT, which is about
  a member slot handed to an allocating callee.
* Not registered again: MEMBER-EFFECT-STORES (round 4's rows, the 9
  member stores), decision class 3 beyond the encapsulation decided here.

**Files likely to conflict.** `providers/qcore-provider-librdkafka/contracts.lock`,
`contract-inventory.lock`, `contracts.json` (any card declaring librdkafka
contracts, CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT; rerun `pin` and `sync`),
`providers/contract-load-path.json` and `cycle19-resource-contracts.json`
(one line each). The OpenSSL provider is not touched.
