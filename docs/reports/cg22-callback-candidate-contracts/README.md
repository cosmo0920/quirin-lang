# CG22-CALLBACK-CANDIDATE-CONTRACTS: callback candidates in force

Card CG22-CALLBACK-CANDIDATE-CONTRACTS (launched by the user 2026-11-17),
follow-up of [CG22-CALLBACK-REGISTRY-CONTRACT](../cg22-callback-registry-contract/README.md).
Gateway rule **4a** amended in place (rule 13u's text corrected in place), Lean
`QCore.CallbackCandidateScope`. Base `32336c33d` (origin/main). The
[prediction](PREDICTION.md) was committed (`38a312a78`) before the semantic
commit (`c89aac4c9`).

## 1. The question

A synchronous invocation (`bsearch`, `qsort`, the cmetrics and ctraces mpack
unpackers, ...) reads as its callee's custody contract only when every
candidate callback has a contract in force (rule 13u); otherwise it refuses
`callback-uncontracted`. The follow-up asked for each candidate to be put in
force by a route the vocabulary already has (its own validated summary where
its body is analysed and Safe, a declaration where it is not), never by
reading it benign, and for a candidate Unknown in its own unit to keep its
caller refused.

## 2. Census on the base

Re-measured on this card's own corpus preparation (section 4): corpus
**1,082 / 6 / 3,616**; `callback-uncontracted` **51 leaves / 49 functions / 18
decided alone** (46 / 44 / 17 at registration; the growth is two op-callback
candidates exposed by declarations landed since). The 46 candidates named
([leaves](artifacts/leaves-before.txt), [status](artifacts/candidate-status-before.txt),
[reasons](artifacts/reasons-before.json)):

| class | candidates | route that would put them in force | outcome here |
| --- | --- | --- | --- |
| Unknown in their own unit | 41: the cmetrics and ctraces decoder trees (36 leaves; every table holds an Unknown handler), `rd_kafka_group_member_cmp`, `rd_qsort_r_trampoline`, `toppar_sort_by_list_cnt`, `rd_kafka_coord_rkb_monitor_cb`, `rd_kafka_toppar_op_serve` | their own verdicts | **stay refused** (no summary is published for an Unknown body) |
| Safe in another unit (external) | 4: `rd_kafka_metadata_broker_cmp`, `_internal_cmp`, `rd_strcmp3`, `rd_kafka_assignor_topic_cmp` | rule 4c's import under the stated link map | not in the eight-home map; follow-up registered (section 5) |
| header `static`, no row | 1: `rd_kafka_ApiVersion_key_cmp` (`rdkafka_proto.h:208`) | its validated in-unit summary | **put in force** (section 3) |

## 3. What changed

Rule 4a's derivation scope reached unselected internal-linkage definitions
through **direct calls only**. A header comparator is never called directly;
it is handed to `bsearch` or `qsort`, so W4 never derived its summary and the
invocation had no contract in force. The scope now also follows the
designators a scope member names beside a call to a declaration in force with
a synchronous invocation or a table dispatch
(`callback_route::invocation_designators`, the same designators rule 13u
already uses to order W4), still only to unselected **internal-linkage**
definitions the unit carries. The designator of such a definition denotes this
unit's body (C11 6.2.2p3), so the summary the unchanged derivation validates
is a statement about every invocation of it here. Nothing else moved: a body
that does not validate publishes nothing, the invocation still decides over
the candidates' contracts in force, an Unknown candidate (or one table member)
keeps the call refused, and the reported rows are the selection alone.

The reach is an over-approximation of the candidates (every designator in a
body that calls such a declaration): it only offers definitions to the
derivation. Measured, it also summarised `rd_list_string_copy` in
`rdkafka_metadata.c` (named beside a `qsort`), which two rows list as a
consulted summary with no leaf or status change.

**Rule 13u's text corrected.** 13u said "do not admit a candidate on a
summary of another unit", but the classifier reads candidates through the
function map, which includes rule 4c's import channel. The card pinned the
actual behaviour by test (`a_candidate_from_another_unit_is_in_force_only_under_its_import`):
with no import an external comparator refuses `callback-uncontracted`; under
rule 4c's import (stated link premise, home exported and replayed) it is in
force exactly as at a direct call, and the caller's certificate replays both
bodies. The text now says so. This is a documentation correction; no verdict
moves from it on the eight-home corpus (none of its eight homes is a callback
candidate).

## 4. Measurement

`quirin-audit measure --declared PREDICTION.md`, corpus route, on this card's
worktree-private preparation ([inputs](artifacts/inputs.json),
[prep.sh](artifacts/prep.sh), [arbiter.sh](artifacts/arbiter.sh),
[export.sh](artifacts/export.sh)), as RULE50-CONSUMER-COMPOSITION prepared it:
this host has no published live-route inputs. Both sides are `git archive`
exports with `third_party/libevent` at `d82464a2` and the one shared driver
adaptation (`run_pinned_audit.py`'s `QCORE_CLANG` = `/usr/lib/llvm-21/bin/clang`;
the first BEFORE attempt, `measure/before.failed-1`, failed because the host
has no `/usr/bin/clang` and is not evidence). Absolute figures differ from the
published baselines; the comparison is base against tip on the same inputs.

* BEFORE `measure/before` (base export `32336c33d`, binary `47db0eba…`, run
  twice as `base` and `again`, both 1,082 / 6 / 3,616).
* Arbiter `measure/arbiter-1`: `base=run:measure/before/sides/base` against
  the tip export of `c89aac4c9` (binary `b1299962…`), exit 0, route complete,
  no unmatched rows, **declared result `match`** (no unpredicted, no missing,
  no delta mismatch), tip wall 5,027 s.
  [Comparison](artifacts/native-comparison-corpus.json.gz) (gzip of
  `native-comparison.json`, sha256 `68a67097…`), [rows](artifacts/changes-arbiter-1.txt).

| route | BEFORE | AFTER | report sha256 |
| --- | --- | --- | --- |
| corpus | 1,082 / 6 / 3,616 | **1,083 / 6 / 3,615** | `0f3f068e…` -> `0503f615…` |

* **One move, attributed:** `rdkafka_feature.c::rd_kafka_ApiVersion_check`
  `Unknown -> Safe`. Its one leaf was the `bsearch` refusal; its row now lists
  the summary `rd_kafka_ApiVersion_key_cmp` and that comparator's null
  premises (`null-deref:rd_kafka_ApiVersion_key_cmp:…:a`, `:b`: the comparator
  dereferences what `bsearch` hands it).
* **Eleven content changes, no status move:** `rd_kafka_features_check`
  trades `call to uncontracted function rd_kafka_ApiVersion_check` for an
  unresolved identity of `match` (it stays Unknown on its other leaves);
  `rd_kafka_broker_ApiVersion_supported0`, `rd_kafka_broker_request_supported`,
  `rd_kafka_ApiVersions_copy` and `rd_kafka_handle_ApiVersion` list the
  comparator's summary and premises; six `rdkafka_metadata.c` rows change
  `rule_obligations`, two of them listing `rd_list_string_copy`.
* No `Safe -> non-Safe`, no `Unsafe` added or removed, no leak or violation
  added.
* `callback-uncontracted` **51 / 49 / 18 -> 50 / 48 / 17**
  ([reasons](artifacts/reasons-after.json), [leaves](artifacts/leaves-after.txt),
  [status](artifacts/candidate-status-after.txt)); `bsearch` 6 -> 5 leaves.

**Routes not measured: system and slice.** Not prepared (corpus only, as
above). The registering card's census had no `callback-uncontracted` leaf on
either; the change admits a definition only in the accepting direction (an
added validated summary), and no rule here can refuse more than before.

## 5. What remains

* **Unknown in their own unit** (44 leaves / 43 functions / 17 decided
  alone): unchanged by design. Many decoder handlers store what they decode
  into the context (`cmt_decode_msgpack.c`'s `create_*_instance`, the
  `unpack_*` handlers), so a handler whose body comes to validate with such a
  store is still not the identity rule 13u needs (`callback-effectful`). The
  follow-up stays open on that part.
* **External comparators Safe in their home unit** (6 leaves / 5 functions / 0
  decided alone): rule 4c already admits them at a callback position once the
  link map names them; the eight-home map does not, and extending it is a
  change of the stated premise and of the profile's input, not a code change.
  Each caller (`rd_kafka_Node_new_from_brokers`,
  `ut_populate_internal_topic_metadata`,
  `rd_kafka_topic_assignment_state_rack_search`,
  `rd_kafka_partition_internal_find_rack`,
  `rd_kafka_roundrobin_assignor_assign_cb`) carries another blocker, so no
  status would move (read from their leaves, not measured). Registered as
  `CG22-CALLBACK-CANDIDATE-LINK-PREMISE`. A variant map is in
  [link-map.eleven-home.json](artifacts/link-map.eleven-home.json)
  (`eleven_home.py`), unmeasured.

## 6. Lean

`proof/QCore/CallbackCandidateScope.lean`, audited by
`proof/AuditCallbackCandidateScope.lean` (8 theorems, axioms within
`propext`, `Quot.sound`; no placeholder, no project axiom), over
`QCore.CalleeSummaryScope`'s definitions: the widened scope admits only
selected or internal-linkage definitions the unit carries
(`every_member_is_selected_or_internal`,
`an_unselected_external_definition_is_never_admitted`,
`an_undefined_name_is_never_admitted`), contains rule 4a's direct-call scope
(`the_direct_call_scope_is_contained`), an admitted candidate is the only body
its invocation runs (`an_admitted_candidate_runs_only_its_analysed_body`), a
candidate with no declaration and no validated summary keeps the invocation
refused under `QCore.CallbackContracts.decide`
(`an_unvalidated_candidate_keeps_the_invocation_refused`), a validated
identity summary is classified the identity
(`a_validated_identity_candidate_is_in_force`), and a witness
(`the_witness_scope`: a designated static is admitted that the direct-call
scope did not admit; a designated external and an undesignated static are
not). The token reader (`invocation_designators`) is a Rust transcription of
`designates`, by review (trust boundary 28).

## 7. Tests

`crates/qcore-c-gateway/tests/callback_candidate_scope.rs` (Clang, main-source
scope, the header-defined callbacks of a fixture header over
`fixtures/contracts/callbacks.qv`): an identity header comparator decides the
invocation and the caller is Safe and re-certifies; an unvalidated one keeps
`callback-uncontracted`; an effectful one `callback-effectful`; a table with
one unvalidated handler stays refused while the closed identity table
decides; a designator beside no invocation reaches nothing; and the
cross-unit case of section 3. With the reach disabled the first, third and
fourth fail (checked once by hand). The unit tests of `legacy_callee_scope`
pass the environment.

## 8. Gate

GATE_SECTION

## Files

* Route: `crates/qcore-c-gateway/src/legacy_callee_scope.rs`
  (`derivation_scope` takes the environment), `legacy_callback_route.rs`
  (`invocation_designators`), `legacy_c.rs` (the two scope sites).
* Lean: `proof/QCore/CallbackCandidateScope.lean`,
  `proof/AuditCallbackCandidateScope.lean`, `proof/QCore.lean` (generated).
* Tests: `crates/qcore-c-gateway/tests/callback_candidate_scope.rs`,
  `crates/qcore-c-gateway/tests/unit/legacy_callee_scope.rs`.
* Records: `docs/agents/rules/4a.md`, `docs/agents/rules/13u.md`,
  `docs/agents/gateway.md` (generated), `docs/QCORE_CONTRACT_SPEC_r0_3.md`
  (C04.19 item 2), this report and its
  [amendments](integration-amendments.json).
