# W44-LEAK-PIN-AUDIT: every leak leaf classified, two false-leak mechanisms repaired

Card W44-LEAK-PIN-AUDIT, 2026-09-25, on base `c1e69bb5` ("Open the wave-44
increment record"). Every one of the corpus's **271** `leak` leaves sat on one
of the **85** provisional-Unsafe functions. Provisional Unsafe is never an
upstream-defect claim, but it must be a validated reachable violation of the
stated property under the stated premises. This card audits that claim leaf by
leaf, repairs the two class (b) mechanisms it found, and publishes the
remaining class (a) leaves as the project's provisional-Unsafe evidence base.

## Headline

* **Classification of all 271 leaves** ([classification.json](classification.json),
  [classification.md](classification.md)), from a leak-witness census of the
  analyzer's own settled states plus source reading ([reading.json](reading.json)):

  | Class | Leaves | Functions | Mechanisms |
  |---|---:|---:|---|
  | (a) validated | **139** | 18 | test-failure path 127, library error path 11, descriptor 0 (`ctr_random_get`) 1 |
  | (b) projection artifact the analyzer can see | **31** | 27 | refused escape 30, cast return 1 |
  | (c) premise or approximation gap | **101** | 53 | uncontracted callee releases it 44, uncontracted callee retains it 37, borrowed `mk_list_add` publishes it 8, scalar branch correlation 7, undecided null test 2, destination suffix spelling 2, chained assignment 1 |
  | (d) unclassifiable | **0** | | |

  The class (a) leaves are tabulated in [class-a.md](class-a.md). They are
  provisional `Unsafe` candidates, **not** upstream defects; 127 of them are
  reachable only when a unit test fails.
* **`RETURN-CAST-PLACE-NOT-ESCAPED` fixed** (`legacy_cast_return.rs`,
  `QCore.CastReturnEscape`): a carrier-preserving cast return is decided as the
  uncast return; a crossing between an integer and a pointer, or a descriptor
  cast to another integer type, is refused; `(int) fd` stays custody.
* **The refused escape repaired** (`legacy_refused_operands.rs`,
  `QCore.RefusedEscapeCustody`): a refused store or `return` whose value side
  hands a live identity on marks it with an unresolved effect, the
  projection's existing "an unmodelled action may have discharged it" reading.
  Identities handed to an uncontracted **call** are deliberately not withdrawn
  (`REFUSED-CALL-ARGUMENT-PREMISE`).
* **Arbiter** (constraint 7): leak **271 → 240**, exactly the 31 class (b)
  leaves, 0 collateral. Functions **294 / 85 / 4,455 → 297 / 60 / 4,477**; units
  **3 / 43 / 135 / 4 → 3 / 34 / 144 / 4**; unknown_alias 4,358 → 4,359;
  unsupported 45,848 → 45,844. 0 violation candidates added.
* **Ledger 345 → 353**: `RETURN-CAST-PLACE-NOT-ESCAPED` retired (its dlopen half
  rehomed), nine residuals registered, two entries amended.

## 1. How each leaf was classified

### 1.1 The witness census

`report_leaks` publishes a site and an exit line, never a path. The census
([`legacy_leak_witness.rs`](../../../crates/qcore-c-gateway/src/legacy_leak_witness.rs),
off-by-default `indirect-census` feature, `QCORE_LEAK_WITNESS`) runs after each
CFG's custody fixed point, re-runs every reachable node's transfer and edge
refinement from its settled incoming state exactly as the work list does, and
for every reported leak searches three CFG paths from the node that produced
the site to the reporting exit along edges whose settled output keeps the site
`LIVE`:

* the **shortest** path;
* the shortest **clean** path, avoiding every node that refused its statement
  while that statement's value side (a store's right-hand side or a `return`
  operand) handed on a place bound to the site;
* the shortest **strict** path, also avoiding every refused node that passes
  such a place, or its address, to a call.

It records whether the settled exit still reports the leaf, the site's status,
unresolved and exempt bits, and per step the instruction, whether the node was
refused, the hand-on tags, and whether the edge taken was an undecided pointer
test. Joined states make a path the projection's may-evidence, not an execution;
the reading decides.

The census binary's diagnostics are byte-identical to the baseline
([evidence/before-census-identity.json](evidence/before-census-identity.json)),
so it perturbs nothing. [scripts/join_witnesses.py](scripts/join_witnesses.py)
joins all 271 published leaves to their records (invocations of one body under
different entry custody may differ in path; the class uses paths present in
every invocation, and only `ctr_random_get`'s settled fields differ between its
invocations): [evidence/before-joined.json.gz](evidence/before-joined.json.gz).

Measured: **30** leaves have no clean path; **241** have one, of which **17**
also have a strict path and **224** pass an identity to a refused call. The
settled exit no longer reports **12** leaves (published on an intermediate
iterate before a later join added an unresolved effect, `C02-P-ALIAS-LEAK-ORDER`).
That is recorded, not a class: four of the twelve (`mk_vhost_read`) are true path
witnesses the union join merely stops supporting.

Two earlier census runs are not evidence: the first used a hand-on reader that
matched a place's root name anywhere in a refused statement (it would have
withdrawn `mk_net_conn_create`'s real leak because `mk_net_fd_connect(conn->fd,
…)` reads a member through `conn`), the second was stopped to add the
unresolved/exempt fields.

### 1.2 The classes

* **(b) refused escape, 30 leaves / 26 functions.** Every witness passes a
  refused statement that stores or returns the identity: `map->parent = (void
  *) counter;` (the six `create_*_instance`), `*bufferp = buf;`, `*out_buf =
  tmp_buf;`, `*routes = result;`, `*out_matches = matches;`, `*outbuf =
  decompressed;`, `arr[cnt++] = val;`, `rktpar->metadata = metadata_bytes;`,
  `otel_span->links = otel_links;`, `server->api = api;`, `conn = (struct
  mk_sched_conn *) data;`, `event = &listener->event;`, and so on. The identity
  left the projection by the statement's own syntax, which the analyzer can see.
  Most are false by reading (the object is published to the caller or an
  aggregate); `mk_server_listen_init`'s two `listener` leaves are also leaked in
  C transitively through the abandoned `listeners`, and are still class (b):
  the accusation about *this* site is unsupported once its interior address
  escaped into `event`.
* **(b) cast return, 1 leaf.** `mk_string_copy_substr`'s `return (char *)
  buffer;`. The W43 entry counted three leaves "at a cast return"; the other two
  (`rd_kafka_anyconf_dump`) sit at `return (const char **)arr;` but are the
  refused stores `arr[cnt++] = val;` of refused-escape class.
* **(c)**, each with its premise named in [reading.json](reading.json):
  * `REFUSED-CALL-ARGUMENT-PREMISE` (81): the witness passes the identity to a
    refused call. Reading finds the premise false every time: 44 follow an
    uncontracted destructor (`destroy_metric` ×15, the `cmt_*_destroy` family,
    `cio_destroy`, `mk_rconf_free`, …), 37 a retaining call (`cfl_list_add` ×14,
    `rd_kafka_buf_new_shadow` at the nine `RD_UT_PASS` exits, `MK_TLS_SET` ×8,
    `rd_kafka_OffsetFetchRequest`'s opaque, `rb_tree_insert`, `mk_iov_add`,
    `flb_ring_buffer_write`, `pthread_create`).
  * `MK-LIST-ADD-BORROWED-PUBLICATION` (8): `mk_list_add` is declared borrowed
    on both positions, so linking `&obj->_head` into a caller's list publishes
    nothing the projection can see.
  * `LEAK-PATH-SCALAR-CORRELATION` (7): `decode_histogram_points` (a flag the
    acquisition implies) and `rd_kafka_snappy_java_uncompress` (pass-2 checks of
    bytes pass 1 accepted; a pass counter).
  * `LEAK-UNDECIDED-NULL-TEST` (2), `DESTINATION-SUFFIX-PLACE-SPELLING` (2:
    `(*map)->opts = calloc(…)` bound to `opts`), `CHAINED-ASSIGNMENT-INNER-DESTINATION`
    (1: `d = safe = rd_malloc(…)` binds `d` only).
* **(a)**: the strict or clean witness is reachable under the premises and
  the resource is unreleased on it: the `RD_UT_ASSERT` failure exits of the
  `rdunittest_msgset_errors.c` and `rdkafka_sasl_oauthbearer_oidc.c` tests and
  `do_unittest_erase` (127), and five library error paths
  (`pack_metric_metadata` ×4, `mk_vhost_read` ×4, `mk_net_conn_create`,
  `mk_sched_launch_thread`, `mk_server_listen_init`) plus `ctr_random_get`'s
  descriptor 0, natively observed by W43-DESCRIPTOR-NATIVE-CHECK.

One mechanism label changed after the prediction froze:
`append_to_ring_buffer`'s exit at `flb_input_chunk.c:3656` was labelled
callee-release with its function and is callee-retention
(`flb_ring_buffer_write`). Its class, (c), is unchanged; so are all counts but
the release/retention split (45/36 → 44/37).

## 2. The repairs

### 2.1 The cast return (`RETURN-CAST-PLACE-NOT-ESCAPED`)

[`legacy_cast_return.rs`](../../../crates/qcore-c-gateway/src/legacy_cast_return.rs),
one hook in the `return` arm of `legacy_c.rs`. `return ( TYPE ) OPERAND ;` with
`TYPE` a run of identifiers and `*`s (a chain of casts allowed):

| Target | Operand | Answer |
|---|---|---|
| pointer (every link ends in `*`) | one place, not a declared scalar, no possible descriptor | decided as `return OPERAND;` |
| `int` | a plain name that may hold a tracked descriptor | decided as `return NAME;` (stays custody) |
| any other | a possible descriptor | **refused** |
| non-pointer | a name the reader commits to as a pointer | **refused** |
| anything | untracked scalar data, an uncommitted operand, not one cast | pre-card parse |

The refusal is an `Unsupported` node before a place-less exit, so the
refused-escape rule withdraws the returned identity. No new
`LegacyInstruction`, so `admits_descriptor_read` is untouched.

**The first AFTER run was withdrawn as the arbiter** ([first-after-attempt.json](first-after-attempt.json)):
its rule escaped the operand's place directly, which admitted 22 member-chain
casts the uncast return refuses (`return (const rd_kafka_Node_t **)
partition->isr;`) and moved 5 functions Unknown → Safe on that admission alone.
The cast then added an admission of its own. The rule now delegates to the
uncast return, so every existing check (the strict place escape, the
return-position pointer-chain widening that escapes nothing, the return call,
their refusals) decides the operand.

Lean: `QCore.CastReturnEscape`, 12 audited theorems: delegation only on a
carrier-preserving conversion of a place; every integer↔pointer crossing of a
possibly tracked value refused; scalar data unchanged; the cast is the uncast
return; the rule only withdraws; the corpus leak withdrawn; non-vacuity.

### 2.2 The refused escape

[`legacy_refused_operands.rs`](../../../crates/qcore-c-gateway/src/legacy_refused_operands.rs),
one hook each in the work list and the settled exit pass of `legacy_c.rs`.
`PublicationFacts` gains `line_reads`: per preprocessed line, the places a
statement hands on, tagged `Escape` (a store's right-hand side or a `return`
operand, outside every call's argument list, as a value or an address inside
the object) or `Call`. A store into a record object of the function itself
(`local.slot = h;`, not through `->` or `*`) hands nothing on: the value stays
in storage that ends with the call, and the accusation stays, as the
publication rule leaves a local destination alone (the first gate run caught
this: `publication_custody.rs`'s local-aggregate control). After a node whose
own transfer reported `Unsupported`,
the live sites bound to its `Escape` places, and what they contain or link,
join `unresolved_effect_sites`.

The rule only adds unresolved effects: it never accuses, never discharges, and
fires only where the function keeps a refusal, so it cannot make a function
`Safe`. The line key unions a macro line's statements and the unresolved effect
joins by union; both only withdraw (`REFUSED-ESCAPE-LINE-GRANULARITY`), and the
arbiter measured no collateral.

Lean: `QCore.RefusedEscapeCustody`, 10 audited theorems: unresolved effects
only grow, statuses untouched, an admitted node untouched; only withdrawal, an
unhanded site keeps its report, a handed-on live site is withdrawn;
distribution over the union join; never `Safe`; the union-join collateral
stated by a witness.

### 2.3 What was examined and not repaired

* **Settled reporting.** Reporting leaks only from settled exits would remove
  the 12 intermediate-iterate leaves, but four are true (`mk_vhost_read`). The
  measurement is appended to `C02-P-ALIAS-O`.
* **Call arguments.** Withdrawing on every refused call handed an identity
  would also withdraw the class (a) test-failure leaves (`strlen(payload_part)`
  shares a line with the refused `fprintf` of `RD_UT_ASSERT`). What a callee
  does is a callee fact: `REFUSED-CALL-ARGUMENT-PREMISE`, per callee.

### 2.4 Tests and pins

* New [`tests/leak_pin_audit.rs`](../../../crates/qcore-c-gateway/tests/leak_pin_audit.rs)
  (11 tests): the pointer casts and the cast chain escape for heap and stream;
  a released identity under a cast reports what the plain return reports;
  member chains are decided as the uncast return (the corpus
  `rd_kafka_TopicPartitionInfo_isr` shape certifies); integer casts of a
  declared pointer and non-`int` casts of a descriptor refuse; `(int) fd` equals
  `return fd;`; scalar casts unchanged; a refused store or return withdraws only
  the identity it hands on; a call argument and a member-value store keep their
  accusation.
* Unit tests: 12 for the hand-on reader, 4 for the cast shape reader.
* **Pins flipped deliberately**, each a leak that rested on the repaired
  mechanism; every one still asserts non-`Safe`, and none weakens a refusal:
  * `descriptor_custody.rs::a_cast_return_is_not_an_escape_for_any_kind` (the
    W43 boundary, `Unsafe`/`leak`) is now `a_cast_return_escapes_for_every_pointer_kind`
    (`Safe`, no diagnostic), the discharged `RETURN-CAST-PLACE-NOT-ESCAPED`;
  * `legacy_summary` unit `an_overwritten_out_slot_leaks_the_first_resource`:
    `fill`'s leak rested on two refused `*out = …` stores; now `Unknown`, still
    no summary for `fill`;
  * `constructor_residuals.rs::a_write_through_a_dereferenced_call_result_never_discharges_a_resource`:
    `(*d_handle ()) = h;` is still never a discharge; now `Unknown`, 0 leaks;
  * `indexed_collection.rs::storing_a_named_pointer_keeps_the_refusal_it_had`:
    `v[0] = p` keeps its refusal; the leak of `p`, false by reading (`x_free(v[0])`
    frees it), is withdrawn; now `Unknown`;
  * `publication_custody.rs::a_store_through_a_local_pointer_to_the_object_keeps_its_pre_existing_answer`
    (`CTWIN-PUBLISH-INDIRECT-POINTER`): `*p = h` with `p = &g_published` is still
    no recognised publication; its false leak is withdrawn; now `Unknown`.

## 3. Arbiter (constraint 7)

One command shape, [`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
`QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO package; identities by
`baseline_identity.py --root-alias <this worktree>=…/hungry-thompson-1740da`.

| Run | Tree | Binary | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `c1e69bb5` | `80bc4122…` | 0 | [before-run.json](before-run.json) |
| census | `c1e69bb5` + census (feature on) | `5ee87031…` | 0 | [evidence/before-census-run.json](evidence/before-census-run.json) |
| AFTER (withdrawn) | `11915d99` | `03ce265e…` | 0 | [first-after-attempt-run.json](first-after-attempt-run.json) |
| AFTER 2 | `8cc59a8b` | `d9eec38e…` | 0 | [after2-run.json](after2-run.json) |
| **AFTER** | `3eebf2e3` | `0415f39a…` | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |

AFTER 2 and AFTER are byte-identical on all five artifacts: the
local-aggregate narrowing of §2.2, added after the first gate run, moved no
corpus leaf.

**BEFORE** ([before-identity.json](before-identity.json)): diagnostics,
chunk-contracts and source byte-identical to
`w43-indexed-resource-collections/integration/artifacts/`; contracts identical
after the root alias; source-effects differ only in `build_digest`,
`configuration_digest`, `task_digest`, `obligation_id`, `coverage[0]` and the
`resource_contracts:<digest>` key.

**AFTER vs BEFORE** ([attribution.json](attribution.json),
[scripts/attribute.py](scripts/attribute.py)): contracts and source-effects
byte-identical to BEFORE (same checkout); chunk-contracts and source
byte-identical to the baseline. Every diagnostics delta:

| Delta | Leaves | Attribution |
|---|---:|---|
| leak removed | 31 | exactly the class (b) leaves: 30 refused escape, 1 cast return; 0 class (a)/(c) |
| unsupported removed | 4 | `unmodeled … in return expression` at a pointer cast now decided as the uncast return: `rd_kafka_anyconf_dump` (`(const char **)arr` → `return arr;`), and `rd_kafka_TopicDescription_partitions`, `rd_kafka_TopicPartitionInfo_isr`, `rd_kafka_TopicPartitionInfo_replicas` (`(const T **) p->field` → the admitted chain return) |
| unknown_alias added | 1 | `rd_buf_get_segment_at_offset` (`rdbuf.c:230`): `return (rd_segment_t *)seg;` decided as `return seg;`, which reports the unresolved identity `seg` exactly as the uncast return does |

Movement: 25 functions provisional Unsafe → Unknown (the 25 predicted, each
keeping a refusal); **3 functions Unknown → Safe** (the three `rdkafka_admin.c`
accessors above, whose only refusal was the cast return: Safe relative to the
declared premises and the existing return-position chain rule, as the uncast
source would be); 9 units Unsafe → Unknown. Violation candidates: the 240
remaining leaks are the 139 class (a) and 101 class (c) leaves, each with the
witness status in [classification.json](classification.json); none added. The
AFTER artifacts are the new baseline ([artifacts/manifest.json](artifacts/manifest.json));
`w43-indexed-resource-collections/integration/artifacts/` is its frozen
predecessor.

## 4. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was committed (`50f4d5f1`) before any rule was
wired.

| Predicted | Measured |
|---|---|
| leak 271 → 240, exactly the 31 class (b) leaves, no collateral | **yes** |
| functions 294 / 60 / 4,480 (25 Unsafe → Unknown, 0 → Safe) | 297 / 60 / 4,477: the 25, and **3 Unknown → Safe** the prediction missed (cast returns of admitted chains) |
| units 3 / 34 / 144 / 4 | **yes** |
| unknown_alias and unsupported ±5 | +1 and −4 |
| no new accusation | **yes** |

The missed three are the cast rule working as designed (a pointer cast is the
uncast return), not a new admission; the first attempt's five were a new
admission, and were withdrawn.

## 5. Ledger

* **Retired**: `RETURN-CAST-PLACE-NOT-ESCAPED` ([retired-followups.json](retired-followups.json));
  its re-declaration half is rehomed to `W44-DLOPEN-DECLARATION-REMEASURE`.
* **Registered** ([followups.json](followups.json)): `REFUSED-CALL-ARGUMENT-PREMISE`,
  `MK-LIST-ADD-BORROWED-PUBLICATION`, `LEAK-PATH-SCALAR-CORRELATION`,
  `LEAK-UNDECIDED-NULL-TEST`, `DESTINATION-SUFFIX-PLACE-SPELLING` (it is also a
  discharging-direction hazard, stated in the entry), `CHAINED-ASSIGNMENT-INNER-DESTINATION`,
  `W44-DLOPEN-DECLARATION-REMEASURE`, `CAST-RETURN-UNCOMMITTED-OPERAND`,
  `REFUSED-ESCAPE-LINE-GRANULARITY`. No (d) group exists.
* **Amended** ([amended-followups.json](amended-followups.json)): `C02-P-ALIAS-O`
  (the -LEAK-ORDER measurement) and `RETURN-REFUSAL-LEAK-REPORT-PREMISE`
  (narrowed, not discharged).
* `corpus.cycle19_raw_units` points at [artifacts/](artifacts/); previous row in
  [previous-corpus-row.json](previous-corpus-row.json).

## 6. Validation

See [gate.json](gate.json) and the table below; exits captured to files,
never piped, logs in this worktree's `measure/gate/`.

GATE_TABLE

## 7. For the integrator

* **Constraint 7** still reads 272 in its first sentence; the pin is now 240.
  The chain: 271 → 240, exactly the 31 class (b) leaves (§3).
* **Composition with the concurrent wave-44 cards.** The refused-escape rule
  and the call-argument residual meet W44-CORPUS-CALLEE-SUMMARIES and
  W44-LIBC-RESIDUALS: a new summary or declaration turns a refused call into a
  modelled one, which can only move a (c) leaf toward discharge or keep it. A
  new refusal elsewhere (W44-INDIRECT-*) can make a statement refused whose
  value side hands an identity on, and then withdraws it; re-run the census
  (`scripts/run_census.py`) on the combined tree to re-attribute.
* **Files likely to conflict**: `crates/qcore-c-gateway/src/legacy_c.rs` (module
  declarations after `descriptor_custody`, the `return` arm, the two transfer
  call sites in `analyze_cfg_with_entry`, the census call after
  `report_collection_violations`), `src/publication.rs` (`PublicationFacts`
  gains `line_reads`), `tests/descriptor_custody.rs` (the flipped pin),
  `proof/QCore.lean` (two imports after `KindedCustody`),
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
  `docs/development/increments/wave-44.md`, `docs/reports/INDEX.md`.

## Reproduction

```sh
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/before c1e69bb5
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit --features qcore-c-gateway/indirect-census
python3 docs/reports/w44-leak-pin-audit/scripts/run_census.py <census binary> measure/census c1e69bb5+census
python3 docs/reports/w44-leak-pin-audit/scripts/join_witnesses.py <baseline diagnostics.json.gz> measure/census/leak-witness.jsonl joined.json
python3 docs/reports/w44-leak-pin-audit/scripts/classify.py joined.json docs/reports/w44-leak-pin-audit/reading.json classification.json classification.md
python3 docs/reports/w44-leak-pin-audit/scripts/predict.py <baseline diagnostics.json.gz> classification.json prediction-movement.json
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/after <commit>
python3 docs/reports/w44-leak-pin-audit/scripts/attribute.py measure/before/resource-diagnostics.json measure/after/resource-diagnostics.json classification.json attribution.json
python3 docs/reports/w44-leak-pin-audit/scripts/class_a_table.py classification.json class-a.md
python3 docs/reports/w44-leak-pin-audit/scripts/update_ledger.py
```
