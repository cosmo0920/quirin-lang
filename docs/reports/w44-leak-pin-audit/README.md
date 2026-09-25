# W44-LEAK-PIN-AUDIT: every leak leaf classified; no leak claimed through a refused hand-on

Card W44-LEAK-PIN-AUDIT, 2026-09-25, on base `c1e69bb5` ("Open the wave-44
increment record"). Every one of the corpus's **271** `leak` leaves sat on one
of the **85** provisional-Unsafe functions. Provisional Unsafe is never an
upstream-defect claim, but it must be a validated reachable violation of the
stated property under the stated premises (hard invariant 2). This card audits
that claim leaf by leaf. It repairs the cast return, and it implements the
coordinator's refused hand-on rule (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`): no leak
is claimed through an identity named in a refused statement. It publishes the
leaves that remain as the provisional-Unsafe evidence base.

## Headline

* **Classification of all 271 leaves**, frozen before any rule
  ([classification.json](classification.json), [classification.md](classification.md)),
  from a leak-witness census of the analyzer's own settled states plus source
  reading ([reading.json](reading.json)):

  | Class | Leaves | Functions | Mechanisms |
  |---|---:|---:|---|
  | (a) validated | **139** | 18 | test-failure path 127, library error path 11, descriptor 0 (`ctr_random_get`) 1 |
  | (b) projection artifact the analyzer can see | **31** | 27 | refused escape 30, cast return 1 |
  | (c) premise or approximation gap | **101** | 53 | uncontracted callee releases it 44, uncontracted callee retains it 37, borrowed `mk_list_add` publishes it 8, scalar branch correlation 7, undecided null test 2, destination suffix spelling 2, chained assignment 1 |
  | (d) unclassifiable | **0** | | |

* **The refused hand-on overlay**, frozen before the rule was committed
  ([PREDICTION-HANDOFF.md](PREDICTION-HANDOFF.md),
  [handoff-classification.json](handoff-classification.json),
  [handoff-classification.md](handoff-classification.md)): **252** of the 270
  non-cast leaves name their object in a refused statement on some path before
  the exit. By frozen class that is (a) 128, (b) 30 and (c) 94. With the cast
  return, **253** are withdrawn and **18** stay: 11 (a) and 7 (c).
* **`RETURN-CAST-PLACE-NOT-ESCAPED` fixed** (`legacy_cast_return.rs`,
  `QCore.CastReturnEscape`):
  * a carrier-preserving cast return is decided as the uncast return;
  * a crossing between an integer and a pointer, or a descriptor cast to
    another integer type, is refused;
  * `(int) fd` stays custody.
* **The refused hand-on rule** (`legacy_refused_operands.rs`,
  `QCore.RefusedHandOnCustody`): a live identity that a refused statement names
  as a store value, `return` operand, call argument, operand, `&x` or interior
  address carries an unresolved effect after that node. It never discharges an
  obligation and never makes anything `Safe`.
* **Arbiter** (constraint 7): leak **271 → 18**, exactly the 253 predicted
  leaves, with 0 collateral and 0 missed.
  * Functions **294 / 85 / 4,455 → 297 / 9 / 4,528**.
  * Units **3 / 43 / 135 / 4 → 3 / 8 / 170 / 4**.
  * unknown_alias 4,358 → 4,374; unsupported 45,848 → 45,844.
  * 0 violation candidates added.
* **The cost, stated plainly**: 128 of the 139 leaves the reading validated
  are withdrawn. The analyzer cannot validate an accusation that rests on what
  a refused statement does. They are registered as
  `REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`, with their reading kept.
  [class-a.md](class-a.md) now tabulates the **11** class (a) leaves in 3
  functions that remain.
* **Ledger 345 → 355**: `RETURN-CAST-PLACE-NOT-ESCAPED` retired, 11 residuals
  registered, three entries amended. `CT-CTWIN-O`'s `CT-CTWIN-UNRESOLVED-LEAK`
  arm is discharged, and CT43's `owned_local.c` control is re-pinned to `Unknown`.

## 1. How each leaf was classified

### 1.1 The witness census

`report_leaks` publishes a site and an exit line, never a path. The census
([`legacy_leak_witness.rs`](../../../crates/qcore-c-gateway/src/legacy_leak_witness.rs),
off-by-default `indirect-census` feature, `QCORE_LEAK_WITNESS`) runs after each
CFG's custody fixed point. It re-runs every reachable node's transfer and edge
refinement from its settled incoming state, exactly as the work list does. For
every reported leak it searches four CFG paths from the node that produced the
site to the reporting exit:

* the **shortest** path whose edges keep the site `LIVE`;
* the shortest **clean** one, avoiding every refused node whose value side
  (a store's right-hand side or a `return` operand) hands the site on;
* the shortest **strict** one, avoiding every refused node that hands it on at
  all;
* the **handed** path, which is the refused hand-on rule's exact population
  under the union join. It runs from a producer of the site to a refused node
  whose hand-on marks the site (the `withdraw` cascade over `contained` and
  `field_links` included), then on over feasible edges to the exit.

Joined states make a path the projection's may-evidence, not an execution; the
reading decides.

The BEFORE census binary's diagnostics are byte-identical to the baseline
([evidence/before-census-identity.json](evidence/before-census-identity.json)).
[scripts/join_witnesses.py](scripts/join_witnesses.py) joins all 271 published
leaves ([evidence/before-joined.json.gz](evidence/before-joined.json.gz)).

* Whether each path kind exists is the same in every joined invocation.
* Only `ctr_random_get`'s `exit_unmodeled` differs between its invocations.
* The settled exit no longer reports **12** leaves. They were published on an
  intermediate iterate (`C02-P-ALIAS-LEAK-ORDER`). This is recorded, not used
  as a class: four of the twelve (`mk_vhost_read`) are true path witnesses.

The handoff census ([evidence/handoff-census-run.json](evidence/handoff-census-run.json))
is the cast-return tree built with the rule's `apply` disabled
([evidence/census-rule-off.patch](evidence/census-rule-off.patch)), with
reader v3 and the handed path. Its diagnostics differ from the baseline only by
the cast-return deltas ([evidence/handoff-census-vs-before.json](evidence/handoff-census-vs-before.json)).
All 270 of its leaves join ([evidence/handoff-joined.json.gz](evidence/handoff-joined.json.gz)).

### 1.2 The frozen classes

* **(b) refused escape, 30 leaves in 26 functions.** Every witness passes a
  refused statement that stores or returns the identity. Examples:
  * `map->parent = (void *) counter;` (the six `create_*_instance`);
  * `*bufferp = buf;`, `*out_buf = tmp_buf;`, `*routes = result;`,
    `*outbuf = decompressed;`;
  * `arr[cnt++] = val;`, `rktpar->metadata = metadata_bytes;`,
    `server->api = api;`, `event = &listener->event;`.
* **(b) cast return, 1 leaf.** `mk_string_copy_substr`'s `return (char *) buffer;`.
  The W43 entry counted three "at a cast return". The other two
  (`rd_kafka_anyconf_dump`) are refused-escape stores.
* **(c)**, each with its premise named in [reading.json](reading.json):
  * an uncontracted callee that releases the object (44) or retains it (37);
  * `MK-LIST-ADD-BORROWED-PUBLICATION` (8);
  * `LEAK-PATH-SCALAR-CORRELATION` (7);
  * `LEAK-UNDECIDED-NULL-TEST` (2);
  * `DESTINATION-SUFFIX-PLACE-SPELLING` (2);
  * `CHAINED-ASSIGNMENT-INNER-DESTINATION` (1).
* **(a)**: the witness is reachable under the premises, and the resource is
  unreleased on it.
  * 127 are the `RD_UT_ASSERT` failure exits of the `rdunittest_msgset_errors.c`
    and `rdkafka_sasl_oauthbearer_oidc.c` tests and of `do_unittest_erase`.
  * 11 are library error paths.
  * 1 is `ctr_random_get`'s descriptor 0, natively observed by
    W43-DESCRIPTOR-NATIVE-CHECK.

One mechanism label changed after the first prediction froze:
`append_to_ring_buffer`'s exit at `flb_input_chunk.c:3656` is callee-retention.
Its class is unchanged.

### 1.3 The refused hand-on overlay

[scripts/classify_handoff.py](scripts/classify_handoff.py) adds three fields to
each frozen row: `mentioned` (a handed path exists), `withdrawn` (predicted
withdrawn) and an attribution. The frozen classes are not rewritten.

| Attribution | Leaves | Frozen reading |
|---|---:|---|
| `b:cast-return` | 1 | (b) |
| `b:refused-escape` | 30 | (b) |
| `h:refused-handoff` | **222** | (a) test-failure path 118, (a) error path 10, (c) callee release 44, (c) callee retention 37, (c) branch correlation 7, (c) borrowed list insertion 5, (c) undecided null test 1 |
| stays (no refused mention) | **18** | (a) `ut_sasl_oauthbearer_oidc_assertion_create` 9, `mk_net_conn_create` 1, `ctr_random_get` 1; (c) borrowed `mk_list_add` 3, destination suffix 2, chained assignment 1, undecided null test 1 |

The refused statements behind the 222 are mostly uncontracted calls taking the
identity:
* the msgset unit-test writers `ut_write_msgset_v2_header`,
  `ut_build_valid_msgset_v2`, `rd_kafka_buf_new_shadow`, `ut_put_be32` and
  `rd_crc32c`;
* the `cmt_*`/`destroy_*` destructors;
* refused indirect accesses and branch conditions that name the identity.

The nine `ut_sasl_oauthbearer_oidc_assertion_create` leaves stay:
`RD_UT_ASSERT(strlen(payload_part) > 0, …)` names the buffer only in a
modelled condition, and the refused `fprintf` beside it on the macro line does
not name it.

## 2. The repairs

### 2.1 The cast return (`RETURN-CAST-PLACE-NOT-ESCAPED`)

[`legacy_cast_return.rs`](../../../crates/qcore-c-gateway/src/legacy_cast_return.rs)
adds one hook in the `return` arm of `legacy_c.rs`. It reads
`return ( TYPE ) OPERAND ;` with `TYPE` a run of identifiers and `*`s; a chain
of casts is allowed.

| Target | Operand | Answer |
|---|---|---|
| pointer (every link ends in `*`) | one place, not a declared scalar, no possible descriptor | decided as `return OPERAND;` |
| `int` | a plain name that may hold a tracked descriptor | decided as `return NAME;` (stays custody) |
| any other | a possible descriptor | **refused** |
| non-pointer | a name the reader commits to as a pointer | **refused** |
| anything | untracked scalar data, an uncommitted operand, not one cast | pre-card parse |

There is no new `LegacyInstruction`, so `admits_descriptor_read` is untouched.
The first AFTER run was withdrawn ([first-after-attempt.json](first-after-attempt.json)):
it escaped the operand's place directly, which admitted 22 member-chain casts
the uncast return refuses and moved 5 functions Unknown → Safe on that
admission alone. The rule now delegates to the uncast return.

Lean: `QCore.CastReturnEscape`, 12 audited theorems. They cover:
* delegation only on a carrier-preserving conversion of a place;
* every integer↔pointer crossing of a possibly tracked value refused;
* scalar data unchanged;
* the cast decided as the uncast return;
* only withdrawal;
* non-vacuity.

### 2.2 The refused hand-on rule (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`)

[`legacy_refused_operands.rs`](../../../crates/qcore-c-gateway/src/legacy_refused_operands.rs)
adds one hook each in the work list and in the settled exit pass of
`legacy_c.rs`. `PublicationFacts` gains `line_reads`: per preprocessed line, a
**condition** table (an `if`/`while`/`switch` controlling expression and every
`for` clause) and a **statement** table. Each table maps the places the line
hands on to the strongest tag:
* `Escape`: a store's value side or a `return` operand;
* `Call`: a call argument;
* `Operand`: any other evaluated position, including `&x` and an interior
  address `&p->f`.

A node refused as a controlling expression reads the condition table; any
other refused node reads the statement table.

After a node whose own transfer reported `Unsupported`, every site that is live
before it and bound to a handed place joins `unresolved_effect_sites`, together
with everything it contains or links to. The state join is the existing union,
and `report_leaks` already skips such a site.

What is **not** a mention (`REFUSED-OPERAND-MEMBER-VALUE-READ`):
* a member value read through the identity (`p->len`, `p->buf[i]`);
* an assignment's destination (`p->f = …`, `*out = …`, `p[i] = …`);
* the base of a subscript;
* the operand of `sizeof`.

Only a mention the classifier's tokens show counts, so a name inside an
unexpanded macro body is not one. The first reader matched a root name
anywhere, and would have withdrawn `mk_net_conn_create`'s true leak through
`mk_net_fd_connect(conn->fd, …)`.

The rule only adds unresolved effects. It never accuses and never discharges,
and it fires only at a node that reported `Unsupported`, so every function it
touches keeps that refusal and cannot become `Safe`. It withdraws more than the
statement strictly warrants in two ways, both only withdrawing
(`REFUSED-HANDON-LINE-GRANULARITY`):
* the line key unions the statements a macro puts on one line;
* the union join carries the effect onto every path through the node.

Lean: `QCore.RefusedHandOnCustody`, renamed from `RefusedEscapeCustody` with
its docstrings rewritten for any hand-on; 10 audited theorems. They cover:
* unresolved effects only grow; statuses are untouched; an admitted node is
  untouched;
* only withdrawal: an unhanded site keeps its report, and a handed-on live site
  is withdrawn;
* distribution over the union join;
* never `Safe`;
* the union-join collateral stated by a witness.

### 2.3 The three exposed mechanisms (`W44-CALL-ARGUMENT-EXPOSED-LEAKS`)

W44-INDIRECT-CALL-ARGUMENT-ACCESS exposed 11 corpus leaves, false by reading,
from three base semantics. None is a baseline leaf here: each needs that card's
admission to be tracked. [`tests/exposed_leak_mechanisms.rs`](../../../crates/qcore-c-gateway/tests/exposed_leak_mechanisms.rs)
pins each on a body with no call-argument access and one statement per line.

With the rule disabled, each fixture is `Unsafe` with a `leak`, reproducing the
base. With it on, each is `Unknown` with no `leak`:

| Mechanism | Refused statement that withdraws it |
|---|---|
| 1. unsanctioned out-slot store `*out = p` | the store itself (`store through *out without an out-parameter contract`), value side `p` |
| 2. object linked into a returned owner (`cmt_histogram_buckets_create_*`; the pinned shape and the corpus shape) | the link store `b->upper = upper`, itself refused as indirect access, value side `upper`; the typed fill `upper[i] = …` is not a mention |
| 3. release through an uncontracted helper (`free_metrics`, `free_resource_attributes`) | the refused call, argument `p` |

So all three fall under the one hand-on rule. Neither escape is modelled; the
projection states only that it cannot tell. The integrator re-checks the 11
corpus leaves on the combined tree.

### 2.4 What was examined and not repaired

* **Settled reporting.** Reporting leaks only from settled exits would remove
  the 12 intermediate-iterate leaves, but four are true (`mk_vhost_read`). After
  the rule, 11 of the 12 are withdrawn anyway; `rd_kafka_sasl_safe_string`'s
  stays. The measurement is appended to `C02-P-ALIAS-O`.
* **The strict-suffix destination.** `unpack_basic_type`'s two leaves bind
  `(*map)->opts = calloc(…)` to `opts`. They belong to
  **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**, which has started; they are
  recorded here as its corpus evidence, not repaired.
* **`rd_string_render`'s three removed `unknown_alias`** (§3) are not
  localised, and are registered as `REFUSED-HANDON-UNKNOWN-ALIAS-SHIFT`.

### 2.5 Tests and pins

* [`tests/leak_pin_audit.rs`](../../../crates/qcore-c-gateway/tests/leak_pin_audit.rs),
  12 tests, each asserting the complete diagnostic vector and status:
  * cast returns for heap and stream, with a cast chain and member chains
    decided as the uncast return, and the integer and descriptor refusals;
  * a refused store or return, or a call argument including `&b->item`,
    withdraws only the identity it names;
  * a member-value store and a modelled condition beside a refused statement
    keep the accusation.
* [`tests/exposed_leak_mechanisms.rs`](../../../crates/qcore-c-gateway/tests/exposed_leak_mechanisms.rs),
  4 tests (§2.3).
* Unit tests: 15 for the hand-on reader and 4 for the cast shape reader.
* **Pins flipped deliberately.** Every one still asserts non-`Safe`, none
  weakens a refusal, and each was a leak that rested on a repaired mechanism.
  * Cast return:
    * `descriptor_custody.rs::a_cast_return_is_not_an_escape_for_any_kind` is
      now `a_cast_return_escapes_for_every_pointer_kind` (`Safe`), the
      discharged `RETURN-CAST-PLACE-NOT-ESCAPED`.
  * A refused store or return:
    * the `legacy_summary` unit test `an_overwritten_out_slot_leaks_the_first_resource`;
    * `constructor_residuals.rs::a_write_through_a_dereferenced_call_result_never_discharges_a_resource`;
    * `indexed_collection.rs::storing_a_named_pointer_keeps_the_refusal_it_had`;
    * `publication_custody.rs::a_store_through_a_local_pointer_to_the_object_keeps_its_pre_existing_answer`;
    * `publication_custody.rs::an_indirect_store_into_a_local_aggregate_publishes_nothing`
      (the local-aggregate control);
    * `derived_result_returned.rs::a_result_derived_from_a_non_formal_base_is_refused`.
  * A refused call or condition:
    * `branch_condition_calls.rs::a_short_circuit_release_stays_refused_and_cannot_hide_the_live_resource`;
    * `return_refused_call_lowering.rs` (`z_take(p)`: 1 leak → 0, still not
      certified);
    * `scalar_member_store.rs::a_local_helper_cannot_launder_the_store_into_a_w4_summary`
      (`Unsafe` → `Unknown`);
    * `indexed_collection.rs::a_stream_element_keeps_the_refusal_it_had`, on a
      one-line body where the line key also reads `x_free(v)`;
    * `loader_abi.rs` (the withdrawn call site) and `loader_abi_modes.rs` (two
      arms): `LOADER-ABI-WITHDRAWAL-UNSAFE`'s direction is now `Unknown`;
    * CT43's `owned_local.c` control (`compiler/tests/fixtures/ct/CT43/expected.json`),
      found by the gate's workspace tests: an owned local handed to the
      uncontracted `q_register`. It was a provisional `Unsafe` with a `leak`
      (`CT-CTWIN-UNRESOLVED-LEAK`) and is now `Unknown` with only the call's
      refusal. The registration still discharges nothing. The wave-9
      stage-evidence record keeps its checkpoint claim.

## 3. Arbiter (constraint 7)

All runs use one command shape,
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
with `QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package. Identities
are checked by `baseline_identity.py --root-alias <this worktree>=…/hungry-thompson-1740da`.

| Run | Tree | Binary | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `c1e69bb5` | `80bc4122…` | 0 | [before-run.json](before-run.json) |
| census | `c1e69bb5` + census | `5ee87031…` | 0 | [evidence/before-census-run.json](evidence/before-census-run.json) |
| AFTER (withdrawn) | `11915d99` | `03ce265e…` | 0 | [first-after-attempt-run.json](first-after-attempt-run.json) |
| AFTER v2 (superseded) | `3eebf2e3`, refused escape only | `0415f39a…` | 0 | [after-run.json](after-run.json), [attribution-v2.json](attribution-v2.json), [manifest](after-v2-artifacts-manifest.json) |
| handoff census | `3eebf2e3` + reader v3 + handed path, rule off | `282f5976…` | 0 | [evidence/handoff-census-run.json](evidence/handoff-census-run.json) |
| **AFTER** | `1e5a9bb5` | `eecc6f98…` | 0 | [after4-run.json](after4-run.json), [artifacts/](artifacts/) |

**BEFORE** ([before-identity.json](before-identity.json)) is byte-identical to
`w43-indexed-resource-collections/integration/artifacts/` in diagnostics,
chunk-contracts and source. Contracts are identical after the root alias.
Source-effects differ only in the path-derived digests.

**AFTER** ([after-vs-baseline-identity.json](after-vs-baseline-identity.json))
has chunk-contracts and source byte-identical to the baseline, and contracts
identical after the alias. Its contracts and source-effects are byte-identical
to BEFORE's (same checkout). Every diagnostics delta
([attribution.json](attribution.json), [scripts/attribute.py](scripts/attribute.py)
over the overlay):

| Delta | Leaves | Attribution |
|---|---:|---|
| leak removed | 253 | exactly the leaves the overlay marks withdrawn: `b:cast-return` 1, `b:refused-escape` 30, `h:refused-handoff` 222; 0 collateral, 0 predicted withdrawal missed |
| unsupported removed | 4 | cast return: `rd_kafka_anyconf_dump` and the three `rdkafka_admin.c` accessors, decided as their uncast return |
| unknown_alias added | 1 | cast return: `rd_buf_get_segment_at_offset` (`return (rd_segment_t *)seg;` reports `seg` as `return seg;` does) |
| unknown_alias added | 18 | the rule: a later Owned or aggregate use of an identity it marked unresolved reports an unresolved identity (`initialize_metrics_data` 1, `initialize_resource_metrics` 3, `otlp_array_value_initialize`/`otlp_kvlist_value_initialize` in cmetrics and ctraces 4, `append_metric_to_timeseries` 1, `flb_input_chunk_write_header_v2` 9) |
| unknown_alias removed | 3 | the rule: `rd_string_render`'s `rd_free(buf)` error exits (`rdstring.c:109, :121, :134`), in a loop whose refused `callback(tmpkey, buf + of, …)` hands `buf` on; not localised, and no status moves (21 refusals kept) |

Movement:
* 76 functions provisional Unsafe → Unknown: every leak of each was withdrawn,
  and each keeps a refusal.
* **3 functions Unknown → Safe**: the `rdkafka_admin.c` accessors whose only
  refusal was the cast return. They are `Safe` relative to the declared
  premises and the existing return-position chain rule, as the uncast source
  would be.
* 35 units Unsafe → Unknown.
* Violation candidates: the 18 remaining leaks are 11 class (a) and 7 class (c)
  leaves, each with its witness in the overlay. No
  `use_after_release`/`invalid_release`/`double_release` leaf was added or
  removed.

The AFTER artifacts are the new baseline ([artifacts/manifest.json](artifacts/manifest.json));
`w43-indexed-resource-collections/integration/artifacts/` is its frozen
predecessor.

## 4. Prediction and outcome

[PREDICTION.md](PREDICTION.md) (`50f4d5f1`) predicted the first design. Its
outcome at AFTER v2:
* leak 271 → 240, as predicted;
* the 25 predicted functions moved;
* 3 accessors moved Unknown → Safe that it missed.

The coordinator's rule then superseded that design. [PREDICTION-HANDOFF.md](PREDICTION-HANDOFF.md)
was committed (`c49c053b`) before the rule (`1e5a9bb5`):

| Predicted | Measured |
|---|---|
| leak 271 → 18, exactly the 253 withdrawn leaves | **yes**, 0 collateral, 0 missed |
| functions 297 / 9 / 4,528 (76 Unsafe → Unknown, 3 cast-return Unknown → Safe, no other Safe) | **yes** |
| units 3 / 8 / 170 / 4 | **yes** |
| unsupported −4; unknown_alias +1 and non-decreasing otherwise | −4; **+16 net**: +19 and −3. The three removals in `rd_string_render` break the "non-decreasing" part (§3) |
| no violation candidate moves | **yes** |
| the three exposed mechanisms `Unknown`, no `leak` | **yes** |
| (not predicted) conformance pins | CT43's `owned_local.c` control moved `Unsafe` → `Unknown`, which the gate's workspace tests found; re-pinned (§2.5) |

## 5. Ledger

* **Retired**: `RETURN-CAST-PLACE-NOT-ESCAPED` ([retired-followups.json](retired-followups.json)).
  Its re-declaration half is rehomed to `W44-DLOPEN-DECLARATION-REMEASURE`.
* **Registered** ([followups.json](followups.json)):
  * `REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`: 222 withdrawn, 128 of them
    validated by reading;
  * `MK-LIST-ADD-BORROWED-PUBLICATION` (3 still published);
  * `LEAK-PATH-SCALAR-CORRELATION` (0 still published);
  * `LEAK-UNDECIDED-NULL-TEST` (1);
  * `DESTINATION-SUFFIX-PLACE-SPELLING`, to merge into
    W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE;
  * `CHAINED-ASSIGNMENT-INNER-DESTINATION`;
  * `W44-DLOPEN-DECLARATION-REMEASURE`;
  * `CAST-RETURN-UNCOMMITTED-OPERAND`;
  * `REFUSED-HANDON-LINE-GRANULARITY`;
  * `REFUSED-OPERAND-MEMBER-VALUE-READ`;
  * `REFUSED-HANDON-UNKNOWN-ALIAS-SHIFT`.

  No (d) group exists.
* **Amended** ([amended-followups.json](amended-followups.json)):
  * `C02-P-ALIAS-O`: the -LEAK-ORDER measurement, 11 of 12 now withdrawn;
  * `RETURN-REFUSAL-LEAK-REPORT-PREMISE`: narrowed, not discharged;
  * `CT-CTWIN-O`: its `CT-CTWIN-UNRESOLVED-LEAK` arm discharged (CT43's
    control re-pinned); `CT-CTWIN-ENTRY` and the `CTWIN-PUBLISH` residuals
    unchanged.
* `corpus.cycle19_raw_units` points at [artifacts/](artifacts/); the previous
  row is in [previous-corpus-row.json](previous-corpus-row.json).
* `W44-REFUSED-HANDOFF-LEAK-CLAIMS` and `W44-CALL-ARGUMENT-EXPOSED-LEAKS` are
  registered on their own cards' branches, not on this base. This card
  implements the first and pins the second's mechanisms (§2.3). Their
  retirement is the integrator's call on the combined tree.

## 6. Validation

See [gate.json](gate.json), [gate-facts.json](gate-facts.json) and the table
below, all measured at `ece0cc98`. Exits are captured to files, never piped;
logs are in this worktree's `measure/gate/`.
* `cargo clean --profile dev` ran after the test step (exit 0, 17.5 GiB).
* The matrix was regenerated with `--gate gate.json` (exit 0) before the Python
  suite, which was then run again on the final regeneration.
* A first gate run at `0abd0b5a` failed only CT43's control. That run is kept
  privately as `measure/gate-aborted-3`, and the control was re-pinned in
  `ece0cc98` (§2.5).

<!-- gate -->

| Step | Exit | Measured |
|---|---:|---|
| submodules | 0 |  |
| clang | 0 | Ubuntu clang version 21.1.8 (6ubuntu1) |
| test | 0 | 5425 passed / 0 failed / 11 ignored over 289 summaries |
| fmt | 0 |  |
| clippy | 0 |  |
| probe | 0 |  |
| followup | 0 |  |
| lean | 0 | 179 audits, no placeholders, no project axioms |
| census | 0 | 469 passed / 0 failed / 0 ignored over 1 summaries |
| ptest | 0 | 124 passed / 0 failed / 0 ignored over 39 summaries |
| pisolated | 0 | 1 passed / 0 failed / 0 ignored over 1 summaries |
| pfmt | 0 |  |
| pclippy | 0 |  |
| pcontracts | 0 |  |
| pbuild | 0 |  |
| pverify | 0 |  |
| ppython | 0 | 13 tests |
| python | 0 | 330 tests |
| docsindex | 0 |  |

<!-- /gate -->

## 7. For the integrator

* **Constraint 7** still reads 272 in its first sentence; the pin is now **18**.
  The chain: 271 → 18, exactly the 253 withdrawn leaves (§3).
* **Constraint-8 direction**: `LOADER-ABI-WITHDRAWAL-UNSAFE`'s withdrawn call
  site is a refused statement handing the identity on, so it now yields
  `Unknown` rather than a provisional `Unsafe`. The loader pins are flipped.
* **Composition.**
  * W44-COLLECTION-ALLOCATION: its 107 of 125 new claims attributed to a
    refused hand-off should be withdrawn by this rule on the combined tree.
    Re-run the handoff census (`scripts/run_census.py` on a rule-off build,
    `scripts/classify_handoff.py`) to attribute them.
  * W44-INDIRECT-CALL-ARGUMENT-ACCESS: its 11 exposed leaves are pinned here by
    mechanism (§2.3).
  * W44-CORPUS-CALLEE-SUMMARIES and W44-LIBC-RESIDUALS turn refused calls into
    modelled ones, which re-decides a withdrawn leaf rather than withdrawing
    it; that is how `REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS` is discharged.
  * The rule composes with descriptor custody, `Derivation::Derived` and the
    `B[X++]` forget rule, because it acts only on a refused node's output.
* **Files likely to conflict**:
  * `crates/qcore-c-gateway/src/legacy_c.rs`: module declarations after
    `descriptor_custody`, the `return` arm, the two transfer call sites in
    `analyze_cfg_with_entry`, and the census call after
    `report_collection_violations`;
  * `src/publication.rs`: `PublicationFacts` gains `line_reads`;
  * the flipped test files listed in §2.5, including
    `compiler/tests/fixtures/ct/CT43/expected.json` and `owned_local.c`;
  * `proof/QCore.lean`: two imports after `KindedCustody`;
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/development/increments/wave-44.md` and `docs/reports/INDEX.md`.

## Reproduction

```sh
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/before c1e69bb5
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit --features qcore-c-gateway/indirect-census
python3 docs/reports/w44-leak-pin-audit/scripts/run_census.py <census binary> measure/census c1e69bb5+census
python3 docs/reports/w44-leak-pin-audit/scripts/join_witnesses.py <baseline diagnostics.json.gz> measure/census/leak-witness.jsonl joined.json
python3 docs/reports/w44-leak-pin-audit/scripts/classify.py joined.json docs/reports/w44-leak-pin-audit/reading.json classification.json classification.md
# rule-off census (apply evidence/census-rule-off.patch, build with the feature, revert)
python3 docs/reports/w44-leak-pin-audit/scripts/run_census.py <rule-off binary> measure/census-handed <label>
python3 docs/reports/w44-leak-pin-audit/scripts/join_witnesses.py measure/census-handed/resource-diagnostics.json measure/census-handed/leak-witness.jsonl handed-joined.json
python3 docs/reports/w44-leak-pin-audit/scripts/classify_handoff.py classification.json handed-joined.json handoff-classification.json handoff-classification.md
python3 docs/reports/w44-leak-pin-audit/scripts/predict.py <baseline diagnostics.json.gz> handoff-classification.json prediction-handoff-movement.json
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/after <commit>
python3 docs/reports/w44-leak-pin-audit/scripts/attribute.py measure/before/resource-diagnostics.json measure/after/resource-diagnostics.json handoff-classification.json attribution.json
python3 docs/reports/w44-leak-pin-audit/scripts/class_a_table.py classification.json class-a.md handoff-classification.json
python3 docs/reports/w44-leak-pin-audit/scripts/update_ledger.py
```
