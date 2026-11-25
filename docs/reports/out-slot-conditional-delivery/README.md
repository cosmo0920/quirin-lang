# OUT-SLOT-CONDITIONAL-DELIVERY: the keep-or-deliver summary of an in-out slot

Card OUT-SLOT-CONDITIONAL-DELIVERY. USER DECISION 2026-11-25: "lanich four of
queued cards", choosing the return-family and this-lane set, which includes
this card; the follow-up called adding the mode a user decision, and choosing
the card is that decision. Base `43410b230` (main). Probes `58c8e0108`,
[PREDICTION.md](PREDICTION.md) `3b3dfd98b`, semantic commit `80f747e4b`.
Gateway [rule 85](../../agents/gateway.md#constraint-85), Lean
`QCore.OutSlotKeepOrDeliver` (15 audited theorems).

**Outcome.** Shape 1 has a summary mode of its own, validated on two entries
and read by callers only by branching on their own slot. Shape 2 is refused
with a named reason and the options for a user decision. Shape 3 splits: its
literal-status part already has a reading (rule 11r(6)), the
`cio_file_native_lookup_*` part is refused as final, and
`rd_kafka_handle_ApiVersion` turns out to be an every-exit store. None of the
named rows moves; each keeps other blockers (section 5).

## 1. Probes at base

[evidence/probe-base.txt](evidence/probe-base.txt), minimised over the
synthetic `q_*` API through the whole Clang route:

| shape | fixture | base |
|---|---|---|
| 1, keep-or-deliver | `void fill(void **out) { if (!*out) *out = q_alloc(); }` | `Unknown` (rule 74l's slot refusal); callers with a null or a held slot `Unknown`, `call to uncontracted function` |
| 2, member then fresh | `*out = h->m; h->m = 0; if (!*out && !err) *out = q_alloc(); return err;` after an early `return -1` | `Unknown` (indirect access, unresolved `out`, the slot refusal) |
| 3, every-exit store | `*out = q_alloc(); if (c) return -1; return 0;` | `Safe` (W4 `Out`) |
| 3, literal status | `if (c) return -1; *out = q_alloc(); return 0;` | `Safe` (rule 11r(6)'s unwritten class) |

## 2. The mode (shape 1)

**What a call does.** The slot's value at entry decides: null, the slot
receives a fresh object (W4's `Out`); the caller's live object, it is still
there, still the caller's. No single `ResourceParameterMode` says both, and
either alone is unsound: an `Out` reading of a kept object mints a second
custody, a `Borrowed` reading of a delivery hides it
(`an_out_reading_mints_a_second_custody`,
`a_borrowed_reading_hides_a_delivery`).

**The callee's validation** (`legacy_keep_or_deliver_summary.rs`). After every
other route published nothing, for a singleton component with exactly one
formal it stores through. The body is built with a null test of exactly that
slot (`!*out`, `*out == NULL`, …) read as `BranchOnNull` on the slot's
binding, only inside this validation's builds (a thread-scoped `(function,
*NAME)`; every other build reads conditions exactly as before). Then:

1. *Proposal*: the live-entry bind loop from the slot seeded null; fully
   `Safe`, and W4's `out_mode` reads the slot `Out { nullable }`.
2. *Null entry*, on the CFG rebuilt with that delivered contract as the
   function's own (rule 4b): the unchanged W4 gate.
3. *Held entry*, on the same CFG: the slot seeded with a fresh live non-null
   caller-held site `S` (exempt from leak reporting, as a bound formal is).
   Fully `Safe`; every exit still holds `S` in the slot, live, non-null
   (`decide`); and the unchanged W4 gate under the kept contract (the slot
   `Owned { nullable: false }` on `S`), which also refuses containment under
   `S` and a result handing `S` back.

A store over a non-null slot (partial path, a member first, a second store),
a release or hand-off of the kept object fails step 3
(`an_overwrite_of_the_kept_object_reads_no_mode`).

**Where it lives.** `qcore_schemas::keep_or_deliver::KeepOrDeliverSummary
{ slot, delivered }` in a table of its own
(`ResourceContracts::keep_or_deliver`), never the function map, as the
member-effect summary is: records, cross-unit imports, callback candidates
and indirect calls see an uncontracted function. The callee's own row keeps
rule 74l's refusal.

**The caller** (`legacy_keep_or_deliver.rs`). A whole-statement `F(…, &NAME,
…);` or `D = F(…);` refused as uncontracted is lowered to the branch the
callee takes, `if (NAME == NULL) F(…, &NAME, …); else F(…, NAME, …);`, each
arm classified under a minted reserved name (the delivered contract; the kept
contract with the slot `Owned { nullable: false }` on `NAME`), then renamed
`F`. The flow decides it: a known-null slot takes custody of the delivery; a
held live object stays the caller's, never minted twice; anything else (an
uninitialised name, a possibly-null value, another object's member) reaches
the kept arm with no live identity and refuses. Another actual is refused by
name. Lean: `a_fresh_object_only_for_a_known_null_slot`,
`an_unplaced_slot_is_unknown`, `the_reading_is_the_effect` (the obligations
discharge exactly on the delivering exits),
`the_kept_arm_mints_no_second_custody`.

## 3. Shapes 2 and 3

**Shape 2, refused pending a user decision** (`rd_kafka_assignment`,
`rd_kafka_subscription`). The slot first takes `rko->rko_u.….partitions`,
overwriting whatever the caller held, then a fresh list only when that is
NULL and `err` is 0; the early `return RD_KAFKA_RESP_ERR__UNKNOWN_GROUP` /
`__TIMED_OUT` leave it unwritten, and `return err` (the op's error) can return
those same codes, so the status does not partition the slot. The held-entry
run fails (the member store overwrites `S`). Fail-closed does not settle what
a sound statement would be; the options:

* **(a)** declare the op's member custody (the union member moved out and
  nulled before `rd_kafka_op_destroy`), so the slot holds an owned, possibly
  null list on every exit past the op; the early exits still need (b) or (c).
* **(b)** a "delivers or leaves unwritten" mode read only by a caller whose
  slot is known null (the slot then holds null or an owned object on every
  exit). This amends rule 74l's "do not read a partial-path store as a
  delivery" for that caller, so it is the user's call.
* **(c)** keep the refusal (current).

**Shape 3.** *Literal status*: a store partitioned by an explicit literal
status already reads (rule 11r(6)); pinned unchanged
(`a_literal_status_partition_keeps_its_reading`). *Refused as final*:
`cio_file_native_lookup_user`/`_group` store `calloc(…)` when `user ==
NULL`, then, on success, a second `calloc` over it; `CIO_ERROR` exits hold a
live object (the first store, then a later failure) or nothing. No reading of
the status states the slot, so no summary exists; fail-closed settles it
(`an_unpartitioned_failure_store_stays_refused`). The second store is not
presented as an upstream defect. *Reclassified*:
`rd_kafka_handle_ApiVersion` writes `*apis = NULL` first and again on the
`err` path, and the fresh array on the `done` path, so every exit holds null
or a fresh array: W4's `Out { nullable: true }` shape, not a partial store.
Its `if (*apis)` test reads, outside this card's validation builds, as an
indirect access of the unbound formal, and its `rd_kafka_buf_read_*` macros
and `(*apis)[i]` element accesses block it besides.

## 4. Changes

New: `crates/qcore-schemas/src/keep_or_deliver.rs`;
`crates/qcore-c-gateway/src/legacy_keep_or_deliver.rs` (seeds, the slot-test
reading, minted environment, the caller's lowering);
`crates/qcore-c-gateway/src/legacy_keep_or_deliver_summary.rs` (inference,
`decide`, `KEEP_OR_DELIVER_VECTORS`); `proof/QCore/OutSlotKeepOrDeliver.lean`,
`proof/AuditOutSlotKeepOrDeliver.lean`; rule
[85](../../agents/rules/85.md); tests below. Hook points in shared files:

| file | hook |
|---|---|
| `crates/qcore-schemas/src/contract.rs` | the `keep_or_deliver` table: field, `Debug`, `restricted_sha256`, `holds_name`, `new`, three accessors after `insert_member_effect` |
| `crates/qcore-schemas/src/lib.rs` | `pub mod keep_or_deliver;` |
| `crates/qcore-c-gateway/src/legacy_summary.rs` | `mod keep_or_deliver;` after `relational_forwarding`; the publication arm after the forwarding route's `None` |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `pub(crate) mod keep_or_deliver;` before `owned_out_member_route`; the minted environment after rule 72's in `build_function_cfg_with`; the statement fallback after `member_effect_call` |
| `crates/qcore-c-gateway/src/legacy_condition_routes.rs` | `null_condition`: the slot-test reading after the parenthesised-callee check |
| `crates/qcore-c-gateway/src/legacy_reserved_spelling.rs` | two prefixes (array length 14 → 16) |
| `crates/qcore-c-gateway/src/contract_dependencies.rs` | two minted prefixes |

Generated: `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean`.

## 5. Pins

Unit (`tests/unit/keep_or_deliver.rs`, 3; `qcore-schemas` 1): the vectors
against `decide` over `out_mode`; only an `Out` null run delivers; the kept
reading.

End to end (`tests/out_slot_keep_or_deliver.rs`, 10, through Clang): a
known-null slot's caller that releases is `Safe`, one that ignores leaks
(`Unsafe`); a held object released once is `Safe`, none leaks it, twice is a
double release; an uninitialised, possibly-null or member slot is never
`Safe`; another actual is refused by name; every slot-test spelling and an
early exit read alike (the delivery then nullable); the callee's own row keeps
rule 74l's refusal; a partial-path store, a release of the kept object and a
second store publish nothing; shape 2 and the lookup shape stay refused; the
literal-status partition keeps its reading.

**Failing at the base.** The end-to-end file copied into the base tree
(`private/oscd/side-base`, its own target directory): 4 failed, 6 passed
(`private/oscd/pins-at-base-e2e.log`). The four failures are exactly the
change pins (`a_known_null_slot_receives_the_callers_object`,
`a_held_object_stays_the_callers_and_is_never_minted_twice`,
`an_other_actual_is_refused_by_name`,
`the_slot_tests_and_an_early_exit_read_alike`); the six pins of kept verdicts
hold at both. At the tip: 10/10, unit 3/3.

## 6. Measurement

[evidence/runs.md](evidence/runs.md): opt-in `current --monkey-linked` over
Fluent Bit `046accf6`, base and tip each from its own `git archive` tree and
release binary.

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `43410b230` | 1,187 / 2 / 3,607 / 8 | 2,105 / 215 / 8,917 |
| tip `80f747e4b` | **1,187 / 2 / 3,607 / 8** | 2,105 / 215 / 8,917 |

**No row moved and no leaf changed**
([moved-rows-base-tip.jsonl](evidence/moved-rows-base-tip.jsonl) `{"moves":
{}}`, [changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt) empty):
no body on the route validates as keep-or-deliver, so no summary is installed
and no call is lowered. No `Unknown -> Safe` move to justify.

**The named rows** ([evidence/named-rows-tip.txt](evidence/named-rows-tip.txt),
identical at base), all `Unknown`, with what still blocks each:

| row | shape | remaining blockers |
|---|---|---|
| `rdkafka_request.c::rd_kafka_handle_OffsetFetch` | 1 | 26 indirect accesses, 15 refused branch conditions, 14 unsupported assignments, seven uncontracted callees (`rd_kafka_topic_partition_list_add`, `…_find`, `…_set_offsets`, …), `__builtin_alloca`, a reference drop not held, unresolved `rkbuf`/`rkb`/`rktpar` |
| `rdkafka_broker.c::rd_kafka_broker_share_session_add_remove_toppar` | 1 | `rd_list_add(*slot, …)` read as a declaration (`local declaration may shadow…`, CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION), a second slot `*toppars_remove_list` stored with no contract, `rd_list_new`'s registry holder, uncontracted `rd_list_remove`, a reference drop not held |
| `rdkafka_cgrp.c::rd_kafka_cgrp_consumer_assignment_with_metadata` | 1 | the slot pointer is itself tested (`if (missing_topic_ids)`) and filled in a loop (loop re-acquisition), the registry holder, four uncontracted callees, four indirect accesses |
| `rdkafka_subscription.c::rd_kafka_assignment`, `::rd_kafka_subscription` | 2 | refused by rule 85(4) (OUT-SLOT-MEMBER-THEN-FRESH); three indirect accesses, unresolved `rko`/`rkcg` |
| `cio_file_unix.c::cio_file_native_lookup_user`, `_group` | 3 | refused as final (two stores, unpartitioned `CIO_ERROR`); `query_buffer` retention, an unrecognized destination |
| `rdkafka_request.c::rd_kafka_handle_ApiVersion` | 3 (every-exit) | 11 indirect accesses, the slot test outside validation builds (SLOT-NULL-TEST-ORDINARY-BUILDS), `(*apis)[i]` element stores, 9 unresolved `rkbuf` |

**Against the prediction.** Predicted: no status move on the named rows, zero
to a few elsewhere (expected none), no leaf text change expected. Measured:
none anywhere. The `measure-prediction` block (no pinned-route move) stands.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); not run, no result claimed.

## 7. Gate

GATE

## 8. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **OUT-SLOT-CONDITIONAL-DELIVERY** retired: the mode is added (rule 85),
  shape 2 decided as a refusal pending the user, shape 3 split as in section 3.
* **OUT-SLOT-MEMBER-THEN-FRESH** (new, needs a user decision): shape 2's
  options (a)-(c).
* **KEEP-OR-DELIVER-OWN-ROW** (new): the callee's own row keeps rule 74l's
  refusal.
* **SLOT-NULL-TEST-ORDINARY-BUILDS** (new): a slot null test reads as an
  indirect access outside rule 85's validation builds (`rd_kafka_handle_ApiVersion`).
* **CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION** (new): `f(*x, …);` is read
  as a function-pointer declarator (`add_remove_toppar`).
