# VA-LOOP-AND-POINTER-ARG: prediction (frozen before implementation)

Frozen from the BEFORE census at base `958e76593` (local `main`), before any
implementation code of this card was written. Binary `9263920a…`
(`private/valoop/before.binary.sha256`).

## BEFORE (measured)

| Run | Totals | `resource-diagnostics.json` sha256 |
|---|---|---|
| Corpus `66910c10` | **647 / 10 / 4,177** | `ed384eb0…` (reproduces the brief's baseline) |
| System route | 83 / 4 / 361 | `471de81c…` |
| Tail-pause slice | 45 / 1 / 356 / 2 Pending | `0787f2a2…` |
| Probe, contracts route, tip `21a2c190` copy | `mk_vhost_set` Unknown: `:525` loop condition, `:526` pointer `va_arg`, `:533` `mk_vhost_set_property` | (scratch) |
| Probe, master copy | `mk_vhost_set` Unknown: four `va_end` protocol refusals, `:525` loop condition, `:532` | (scratch) |

The brief's three tip blockers reproduce exactly; the master row reproduces.

Census (`scripts/census.py`, `private/valoop/census-before-*.json`):

* **`va_arg` in a condition, `(NAME = va_arg(L, T))`**: corpus 7 leaves in 7
  functions (`ut_init_member_internal`, `verifyAssignment0`,
  `verifyMultipleAssignment0`, `ut_create_topic_partition_lists`:
  `const char *`; `rd_kafka_err_action`: `int`; `mk_config_set`,
  `mk_vhost_set`: `char *`); slice 8 leaves in 4 functions (two units each)
  (`flb_input_set`, `flb_output_set`, `flb_filter_set`, `flb_service_set`);
  system none.
* **`va_arg` in another condition shape**: `rd_kafka_producev`'s
  `while (!err && (vtype = va_arg(…)) != END)`, already refused by the
  protocol (event under `&&`); it stays refused.
* **Pointer `va_arg` stored into a local**: corpus 3 (`ut_topic_partitions`
  `topic_name = …`, `ut_populate_member_owned_partitions_generation`
  `char *topic = …`, `mk_config_set` `value = …`); slice 8 (the four setters'
  `value = …`, two units); `mk_vhost_set`'s `value` is under the refused
  protocol (master).
* **`va_arg` inside another expression** (member/element stores, an actual):
  corpus `cmt_histogram_buckets_create`, `rd_kafka_mock_push_request_errors`,
  `rd_kafka_mock_partition_push_request_errors` (3 uncontracted
  `__builtin_va_arg` leaves), `rd_kafka_metadata_new_topic_mockv_internal`
  (2 member stores, refused as indirect access), `rd_kafka_producev`'s member
  stores and actual (protocol refused). Not lowered by this card.

## Design (decided before implementation)

* **Rule** `12c` (gateway topic; `11a`–`11z`, `12`, `12a`, `12b` are taken).
* **Condition.** A new hook asked after every older route of an `if`,
  `while`, `for` or `do` controlling expression, including the CG02
  elaborator: only the whole expression `(NAME = va_arg(L, T))` (Clang node:
  an assignment of a `VAArgExpr` to a plain local) is lowered, into a plan of
  two nodes: the event's statement (what `NAME = va_arg(L, T);` lowers to),
  then the test on `NAME` (`BranchOnNull` for a pointer, a refining-nothing
  branch for a scalar). The back edge re-enters the event node, so the event
  happens once per evaluation, before the test, as C evaluates it. Any other
  expression holding a `VAArgExpr` is refused by name
  (`va_list operation refused: …`); under a refused protocol the condition
  carries the protocol's reason (rule 11w's wording).
* **Pointer value.** `T *x = va_arg(L, T *)` / `x = va_arg(L, T *)` for an
  object pointer `T *` into an automatic, non-formal, non-descriptor pointer
  local is `OpaqueValue { x }`: unknown, possibly null, never owned (C02-P's
  opaque read). A release, a store into custody and a return as owned keep a
  refusal or an unresolved alias; a function pointer, a member/element
  destination, a static local or a formal keeps the named refusal.
* **Not in this card**: the derived-summary half of CG26-POINTER-VA-ARG's
  DISCHARGE (a variadic definition's derived summary declaring its tail
  positions borrowed) — re-registered as a residual with its size;
  `va_arg` inside other expressions; the other precision items of
  CG26-VA-PROTOCOL-PRECISION.

## Prediction

* **Corpus: no status move** — 647 / 10 / 4,177. Every census function keeps
  another blocker (indirect accesses, uncontracted callees, unresolved
  identities, or `mk_vhost_set`'s protocol refusal). No Safe → non-Safe, no
  Unsafe, leak or violation added.
* **Leaf deltas (corpus)**: `… in loop condition` −7; pointer-`va_arg`
  refusal −3; `va_list protocol refused: \`va\` may reach a function exit
  without \`va_end\`` +1 (`mk_vhost_set:525`, the condition now named by the
  protocol). Other leaf changes, if any, only inside the 9 census functions
  (a downstream use of a binding that was a refused statement's write before).
* **System**: byte-identical.
* **Slice**: `flb_service_set` (both units) **Unknown → Pending**: its other
  leaf is the held `flb_config_set_property` (awaiting human review); the
  other three setters stay Unknown on their uncontracted property setters.
  Slice 45 / 1 / 354 / 4 Pending. Leaf deltas: loop condition −8, pointer
  `va_arg` −8.
* **Probe (contracts route)**: tip `mk_vhost_set` Unknown with the single
  leaf `call to uncontracted function \`mk_vhost_set_property\`` (blockers
  (1) and (2) gone); master keeps Unknown with its `:525` leaf now the
  protocol's `va_end` refusal; `mk_vhost_set_property` unchanged.
* Allowance: up to ±2 further leaf changes per census function from
  downstream bindings; any status move outside `flb_service_set` is a miss to
  be explained.
