# CG10-BOUND-UNTRACKED-ROOTS

Wave 48, round 6, route G (the custody gateway). The residual
`CG10-BOUND-UNTRACKED-ROOTS` registered by
[CG10-COMMON-TYPED-PLACE](../cg10-common-typed-place/README.md). Gateway rule
[11d](../../agents/gateway.md#constraint-11d); `QCore.BoundUntracked`
([`proof/QCore/BoundUntracked.lean`](../../../proof/QCore/BoundUntracked.lean),
audited in `proof/AuditBoundUntracked.lean`). Base `a235ead6` (= local `main`).
The B08 soundness repair (rule 11c) had not landed when this was measured; see
§7.

## 1. What changed

- **The premise** ([`bound_untracked.rs`](../../../crates/qcore-c-gateway/src/bound_untracked.rs),
  flow side [`legacy_bound_untracked.rs`](../../../crates/qcore-c-gateway/src/legacy_bound_untracked.rs)).
  A chain root of the typed reader (rule 10t's mixed places and W46's arrow
  chains) that the closure does not prove never bound is *eligible* when the
  Clang AST shows a pointer declared once as a parameter or automatic local,
  every reference naming it, and no `&` of it. At an access, `chain_owner`
  reads an eligible root as untracked, exactly as a never-bound root, when the
  state entering the access gives it no binding (or a known, non-null,
  target-free one in an unwidened state), binds no place rooted at it, and
  carries no `unmodeled` flag: no path reaching the access passed a refusal or
  an unresolved effect.
- **Why the path flag is the custody test.** In the flow every custody-taking
  position that receives an object with no tracked identity reports a
  refusal (`apply_resource_action` for `Consumed`/`Released`,
  `require_live_resource` for `Owned`/`Stored`/`Removed`), and so does every
  uncontracted call; the report sets `unmodeled` on the path, joined by
  disjunction. So where the flag is clear, no custody position received the
  root or any alias of it on any reaching path. It is path-wide, not per
  root, by decision: it also ends the premise after unrelated refusals, which
  frees no function (a refused path is `Unknown` anyway) but lifts fewer
  leaves.
- **Rule 10k(3).** A pointer read handed to a `Borrowed` or data-tail actual,
  refused at classification unless its roots were never bound, is left to the
  flow when each other root is an eligible chain root; the flow reads such a
  root **only** as untracked (`root_untracked_only`), so a tracked root still
  refuses (the first build without this read `strcmp(a->topic, b->topic)` on
  a comparator's protected formals as tracked and admitted it; it was
  corrected before the AFTER, see §4). An address actual keeps rule 10n's
  never-bound premise; rule 10k(2)'s return premise is untouched.
- **Shared items added** (additive only): `AccessChain::root_bound_untracked`,
  `AccessChain::root_untracked_only` (every older construction site sets
  both false), `ChainView::bound_untracked` (default `false`; `AbstractState`
  implements it), `borrowed_actual::closure_reasons` (census only), and the
  parity positions `bound-untracked-root` and `bound-untracked-closure`. No
  existing variant changes meaning; `elaboration.rs`, `call_instantiation.rs`
  and `typed_place.rs`'s decision table are untouched (`typed_place.rs` only
  fills the new field).

## 2. Cited claims

| Claim (ledger, CG10's report) | Outcome |
|---|---|
| Rule 10t's tracked-root places are refused by `chain_resolves` on roots the body may bind but does not track; top roots `rk`, `rkb`, `rktp`, `rkcg`, `rkbuf`, `sr` | **reproduced in kind**: on this base 7,455 root-refusal leaves on 1,635 functions over every typed-reader chain (wider than CG10's increment of 2,550, which counted rule 10t's additions against its own base); top roots `rkbuf` 851, `rk` 776, `rkb` 604, `rkcg` 382, `rktp` 331 |
| The roots are formals or locals the closure binds by a non-Borrowed actual or an assignment from a call | **reproduced, with a correction of weight**: the dominant reason is an actual of an **uncontracted** callee (class b, 5,973 leaves / 1,223 functions); a contracted non-Borrowed actual (class a) is 458 / 164 |
| This is the largest remaining blocker of the wave | **correction**: it is the largest *leaf* population, but only **3** corpus functions have root refusals as their only leaves (census, every premise granted); the functions carrying it are blocked by the uncontracted calls that bind the roots |
| CG04: 1,768 condition leaves on 708 functions wait on this premise | partly: the rule-10k(3) widening lets 1,227 more typed call actuals reach the flow (999 → 2,226 admitted consultations); most are refused there (path refused by an uncontracted call) |
| CG11: 470 identity-free stores wait on this premise | 24 admitted (flow decisions `unresolved-owner` 482 → 458, `admit` 129 → 153, per statement and analysis) |

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen at `bb39a316`, after the parity-only
census step (`508b9a43`) and before the premise was consulted.

| Measure | Predicted | Outcome |
|---|---|---|
| BEFORE reproduces `2fc3af8c…` | — | **byte-identical**, and the census binary identical on all three routes |
| Corpus Unknown → Safe | 2 (band 0–5) | **6**, above the point, inside the band |
| Safe → non-Safe | 0 | **0** on every route |
| Provisional Unsafe added | 0–5 | **0**; no leak or violation added on any route |
| Root-refusal `unknown_alias` leaves removed | ≥ 336, point 700 | net `unknown_alias` **−404** (−402 root refusals, deferred actuals adding some back) |
| `unsupported` leaves moved by the 10k(3) widening | 100–1,500 | **−230** net (indirect access −145, branch condition −77) |
| CG11 stores admitted | 6–60 | **24** |
| System slice | 0–2 | **+4** (above) |
| Tail-pause slice | 0–1 | **+2** (above) |

The corpus and slice gains above the point are callee-summary cascades the
prediction did not count: `static inline` accessors whose `*out` scalar stores
the premise admits (`flb_task_get_route_data` and its siblings, which
W47-JOIN-AST-CONSERVATIVE-CLASSES had named) now publish validated summaries.

## 4. Measurement

`scripts/measure.sh NAME` runs the brief's corpus arbiter, the system route
and the tail-pause slice into `private/cg10bur/NAME*`; `scripts/identity.py`
compares two runs on all three routes; `scripts/census.py` sizes the
population and its classes; `scripts/attribute.py` attributes every function
whose status or leaves move (`premise`, `deferred-actual`, `callee-summary`,
and `read` for the few whose leaf names no callee, each with the source line
read).

| Route | BEFORE (`a235ead6`) | AFTER (`f477d6c0`) |
|---|---|---|
| corpus (185 units, `66910c10`) | 602 / 10 / 4,222, `2fc3af8c…` | **608 / 10 / 4,216**, `3a25c4e9…` |
| system (`046accf6`) | 37 / 2 / 195, `062a990d…` | **41 / 2 / 191**, `f41b88cb…` |
| slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending, `e4e4197d…` | **28 / 1 / 240 / 1 Pending**, `6949e223…` |

**Corpus: 594 functions move, every one attributed**
([attribution](artifacts/after-corpus.attribution.json.gz)): premise 458,
deferred actual 126, callee summary 6, read 4. The six Unknown → Safe:

| Function | Cause |
|---|---|
| `mk_fifo.c::mk_fifo_queue_get` (`q`), `mk_plugin.c::mk_plugin_cap` (`plugin`) | premise; two of W47-JOIN-AST-CONSERVATIVE-CLASSES's withdrawn functions, restored by a separate premise (the AST closure is unchanged) |
| `mk_plugin.c::mk_plugin_lookup` (`p`), `cmt_filter.c::filter_get_label_index` (`label`) | premise (a branch condition's typed read, now admitted on its root) |
| `rdkafka_subscription.c::_share_invalid_topic_cb` (`rktpar`) | premise |
| `flb_input_chunk.c::flb_input_chunk_drop_task_route` | read: its condition (`:883`) calls `flb_task_get_route_data`, whose body the premise admits |

System +4: `sb_find_segregated_backlog_by_output_plugin_instance` (`backlog`)
and `flb_engine_get_chunk_backpressure_percent` (`ins`), the other two
withdrawals W47-JOIN-AST-CONSERVATIVE-CLASSES named, `engine_has_fluentbit_logs_input`
(`ins`), and `flb_input_chunk_drop_task_route` (34 functions move, all
attributed). Slice +2: `flb_input.c::get_collector` (`coll`) and
`flb_input_collector_running` (its callee summary); 51 move, all attributed.

**New leaves, by reading.** No leak, double release, use after release or
other violation is added on any route. Three corpus functions (all Unknown
before and after) gain refusals where an admitted read lets an older route
see the rest of its statement: `rd_kafka_clusterid` (`rdkafka.c:6337`,
nullable `ret` at a non-null contract), `rd_kafka_cgrp_consumer_assignment_with_metadata`
(`rdkafka_cgrp.c:3016, 3023`, rule 11a's loop re-acquisition refusal),
`rd_kafka_anyconf_set_prop0` (`rdkafka_conf.c:2072`, a discarded `rd_strdup`
and a `*str` store with no out contract). The system route gains one
`summary_conflict` in `flb_task_retry_count` (Unknown before and after): the
inference reads `data` as `Owned` where the declared `Borrowed` contract
governs; fail-closed.

**The first AFTER** (`after1`, at `7e0c06af`, before `root_untracked_only`)
moved 8: the two extra were `rd_kafka_topic_partition_cmp` and its
`_cmp_opaque` cascade, whose `strcmp(a->topic, b->topic)` read the
comparator's protected formals as tracked through the widened actual. Rule
10k(3) does not relax its pointer-actual premise to a tracked root, so the
flow now refuses them; they are not in the AFTER and were never published.

**What remains** ([census](artifacts/after-census.json)): 7,053 root-refusal
leaves on 1,529 corpus functions: the path refused (an uncontracted call or
another refusal before the access) 5,126; a possibly-null or tracked binding
1,409; ineligible (address taken, shadowed, not a pointer formal or local)
505; a rooted place bound 13. No corpus function has root refusals as its
only leaves.

## 5. Tests and pins

- **New controls** (`tests/bound_untracked_roots.rs`, 9): a Borrowed actual
  then an access (admitted), a Released actual then an access (refused), an
  uncontracted call then an access (refused), a borrowed result root
  (admitted; a derived result only once null-tested), the per-path join, an
  address-taken root (refused), a tracked root keeping the tracked reading,
  the widened 10k(3) actual (admitted before a release, refused after, an
  address actual refused), and every shared event vector run through the
  gateway. Unit (`tests/unit/bound_untracked.rs`, 6): the decision table, its
  exhaustive characterisation, rooted places, and the Lean decision and event
  vectors bound to Rust, with the integration copy pinned.
- **Flipped pins, each named in its test:**
  `borrowed_field_actuals::a_later_opaque_reassignment_of_the_base_refuses_every_earlier_read`
  → `…_refuses_only_later_reads` (the read of `in->storage` before
  `b_pick(in)` is admitted: the refusal is flow-sensitive now);
  `chained_tail`: `*storage->leaf->label` moves to the admitted tail rows, and
  the released base is refused at the flow (`UnknownAlias storage`) instead of
  the gate; `neighbours`: the constructor's before-state count 22 → 21 (the
  read through `in` at line 204 precedes the first uncontracted call);
  `o04_arms` (the `link` row) and `o04_read_bound` ("a chained access") are
  refused at the flow instead of the gate; `reach_gate_positions`:
  `*view->count` is admitted (it was CG10's `UnknownAlias view` pin);
  `typed_root_declarations`: `int n = b->flag` where only the AST reader
  answers is now Safe (a custody position before it still refuses);
  `access_path_positions`: the tracked-root actual is refused at the flow
  (`unknown_alias:b`) instead of the gate; unit
  `typed_access_positions::a_call_actual_is_matched_by_callee_and_index`
  (the root `p` is deferred, `root_untracked_only`). Every flip that admits is
  a read before any refusal on its path; every other flip stays non-Safe.

## 6. Lean

`QCore.BoundUntracked`, **38** audited theorems, no placeholders, standard
axioms only. The decision (`decision_vectors_hold`,
`the_premise_holds_exactly_on_an_untracked_unrefused_root`,
`an_ineligible_root_never_holds`, `a_bound_root_never_holds`,
`an_address_taken_root_is_refused`, `an_unlisted_root_is_refused`); the
interlock (`the_never_bound_answer_is_unchanged`,
`an_ineligible_root_is_read_as_before`,
`a_tracked_root_keeps_the_tracked_reading`,
`a_record_object_root_is_read_as_before`, `the_never_bound_reading_only_adds`,
`every_old_admission_stands`); per-hop sufficiency
(`under_the_premise_an_unbound_root_resolves`,
`under_the_premise_a_target_free_root_resolves`,
`an_unbound_root_resolves_only_under_the_premise`, and the refusals); rule
10k(3) (`actualClassified`, `a_deferred_actual_never_reads_a_tracked_root`,
`a_deferred_actual_is_read_under_the_premise`); the per-path event model
(`a_borrowed_actual_preserves_the_premise`,
`a_custody_actual_ends_the_premise`, `an_uncontracted_call_ends_the_premise`,
`a_refusal_is_kept_on_every_later_path`,
`a_none_result_satisfies_the_premise`,
`a_derived_result_of_an_untracked_base_satisfies_the_premise`,
`an_owned_result_keeps_the_tracked_reading`, `the_join_keeps_a_refusal`,
`a_one_sided_binding_is_refused_after_the_join`, `event_vectors_hold`); and
non-vacuity on `IndirectChainAccess.example_state`. The event model states the
flow's arms; the gateway runs every event vector
(`every_event_vector_decides_the_access_as_the_lean_model_does`).

## 7. The B08 repair

`B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED` (rule 11c) runs beside this card and
lands first. This card does not rely on a join or escape cascade silently
leaving an obligation unresolved: the premise reads `unmodeled`, bindings and
rooted places only, never an identity's unresolved effect. The AFTER above is
on `a235ead6`; the re-measurement on the repaired base is recorded here when
the repair lands.

## 8. Follow-ups

- **Retired:** `CG10-BOUND-UNTRACKED-ROOTS`. Its DISCHARGE (a premise that
  proves, per path, that no custody position received the root before the
  access, with Lean parity, Rust refusal controls and a measured re-pin) holds.
- **Registered:** `CG10-BOUND-ROOT-PATH-REFUSED` (5,126 leaves on the path of a
  refusal: the per-root form of the premise and the P4 callee closure) and
  `CG10-BOUND-ROOT-NULLABLE-OR-TRACKED` (1,409 leaves: a possibly-null
  borrowed alias or a tracked root at a place rule 10t does not read).
- **Amended:** `CG11-UNRESOLVED-OWNER-POINTER-STORES` (24 admitted),
  `W44-CONDITION-CALL-INDIRECT-ARGUMENT` (the 10k(3) widening),
  `W47-JOIN-AST-CONSERVATIVE-CLASSES` (four of its six withdrawn functions are
  Safe again through rule 11d; the AST closure itself is unchanged, so its
  DISCHARGE stands).

## 9. Gate

See [gate.json](gate.json) and [gate/](gate/) (commands, exits, counts; logs
in `private/cg10bur/gate`).
