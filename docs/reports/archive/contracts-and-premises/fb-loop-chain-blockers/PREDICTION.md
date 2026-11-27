# FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: census and prediction

Frozen before any implementation code. Base: local `main` `a423fca5d`
(records-only over `3f8f1a617`: no analyzer, contract or provider file differs,
so the BEFORE binary built at `3f8f1a617` is the base's analyzer).

## BEFORE

Binary `1fa10894…` ([`scripts/build.sh`](scripts/build.sh) `before`, clean
tree). Each route ran once under the shared heavy lock with the null property
([`scripts/measure.sh`](scripts/measure.sh)); identities in
[`artifacts/runs-before.json`](artifacts/runs-before.json).

| Route | Custody | Null (unconditional / conditional / Unknown / Unsafe) | Report |
| --- | --- | --- | --- |
| corpus `66910c10` | 939 / 9 / 3,886 | 1,114 / 1,435 / 2,354 / 18 | `f2eb590a…` (= round 31) |
| system `046accf6` | 95 / 4 / 349 | 86 / 125 / 243 / 0 | `1cb9a059…` |
| slice `90978ac6` | 52 / 1 / 347 / 4 Pending | 69 / 121 / 216 / 0 | `a9d0cb04…` |

The system and slice report bytes differ from the lifetime card's `9d3fc333…`
and `cfd3065a…` only outside their rows: the configured route records the
checkout's profile directory, which is this worktree's. The null streams are
byte-identical to that card's (`ccc198b7…`, `fa3f7fcc…`).

A second build with the `indirect-census` feature (`2319d71e…`, `before-census`)
ran the system and slice routes at `--jobs 1` with three measurement streams
([`scripts/census-run.sh`](scripts/census-run.sh)): the gate's own token view
of each refused statement (`QCORE_INDIRECT_CENSUS`), the record bodies the
lexical reader refused (`QCORE_INDIRECT_CENSUS_RECORDS`) and the Clang
reader's answer at every refused position (`QCORE_TYPED_PATH_PARITY`). Its
rows equal the plain BEFORE's ([`scripts/units_equal.py`](scripts/units_equal.py)).

## Census

Scope: exactly the ledger entry's chain list (the lifetime card's
`scripts/chain_blockers.py`), [`scripts/chain_blockers_census.py`](scripts/chain_blockers_census.py).
Sizes reproduce the entry: system 36 rows, 92 access and 184 identity leaves;
slice 50 rows, 206 access and 148 identity leaves. Each leaf, with its
original coordinate, source line and census view, is in
`artifacts/before-chain-{system,slice}.json`.

[`scripts/classify.py`](scripts/classify.py) assigns every leaf a kind and
the brief's class; 0 leaves are unclassified (the hand-read ones are in its
`OVERRIDES`, each with its reason). An identity leaf whose name an earlier
refusal of the same body poisoned is not a blocker of its own: it is counted
under that refusal's kind (it clears exactly when its root does). Output:
`artifacts/before-classes-{system,slice}.json`. Rows count both configurations
of each unit.

| Route | Class | Kind | Rows | Access | Identity |
| --- | --- | --- | --- | --- | --- |
| system | (a) | `fn-pointer-member-record` | 14 | 22 | 34 |
| system | (a) | `fn-pointer-parameter-function` | 2 | 4 | 8 |
| system | (b) | `formal-handed-to-uncontracted-call` | 8 | 6 | 24 |
| system | (b) | `uncontracted-result` | 10 | 0 | 22 |
| system | (c) | `after-retention-undecided` | 2 | 0 | 4 |
| system | (c) | `callback-member-call` | 2 | 2 | 0 |
| system | (c) | `formal-may-bind` | 4 | 0 | 8 |
| system | (c) | `logging-macro` | 2 | 8 | 0 |
| system | (c) | `member-descriptor-slots` | 4 | 8 | 0 |
| system | (c) | `nested-or-condition-call` | 2 | 4 | 0 |
| system | (c) | `pointer-store-into-member` | 12 | 36 | 28 |
| system | (c) | `tls-get` | 2 | 0 | 4 |
| system | (d) | `interior-address-of-looked-up-record` | 2 | 2 | 10 |
| system | (d) | `list-or-lookup` | 8 | 0 | 42 |
| slice | (a) | `fn-pointer-member-record` | 4 | 8 | 2 |
| slice | (a) | `fn-pointer-parameter-function` | 2 | 4 | 2 |
| slice | (b) | `uncontracted-call-with-member` | 4 | 8 | 0 |
| slice | (b) | `uncontracted-result` | 10 | 0 | 12 |
| slice | (c) | `callback-member-call` | 6 | 20 | 0 |
| slice | (c) | `errno` | 2 | 2 | 0 |
| slice | (c) | `formal-may-bind` | 4 | 2 | 16 |
| slice | (c) | `logging-macro` | 20 | 62 | 2 |
| slice | (c) | `member-descriptor-slots` | 6 | 12 | 14 |
| slice | (c) | `member-release` | 8 | 54 | 18 |
| slice | (c) | `pointer-read-from-member` | 2 | 0 | 4 |
| slice | (c) | `pointer-store-into-member` | 8 | 20 | 12 |
| slice | (c) | `void-thread-argument` | 2 | 0 | 10 |
| slice | (d) | `interior-address-of-looked-up-record` | 8 | 14 | 12 |
| slice | (d) | `list-or-lookup` | 12 | 0 | 44 |

Totals by class (access / identity leaves): system (a) 26 / 42, (b) 6 / 46,
(c) 58 / 44, (d) 2 / 52; slice (a) 12 / 4, (b) 8 / 12, (c) 172 / 76,
(d) 14 / 56. Beyond their own kind, 8 system and 30 slice leaves also end in
a member of a record the reader refuses for a function-pointer member, and 4
and 8 in a function whose declarations are unread for a function-pointer
formal (`also_behind_*` in the artifacts): those readings must land too
before they can clear.

The two class (a) kinds are both the lexical declaration reader's
total-or-reject refusals of a function-pointer declarator:

* **`fn-pointer-member-record`.** `read_record_body` refuses a whole record
  body on any `(`, so every member of `flb_input_collector` (`cb_collect`),
  `flb_sched_timer` (`cb`), `mk_event` (`handler`) and `flb_input_plugin` is
  untyped, though each other member is plain. The records stream confirms the
  token for each (`refused_on: "("`). Ledger `RECORD-BODY-TOTAL-OR-REJECT`
  names the repair as its discharge; `W44-CHAIN-INCOMPLETE-RECORD` names it
  for chains.
* **`fn-pointer-parameter-function`.** `read_parameters` refuses every
  declaration of a function one of whose formals is a function pointer
  (`ParameterListToken "("`): `collector_create` (its `cb`) and
  `flb_sched_timer_cb_create` (its `cb`).

Class (b) needs declarations of Fluent Bit functions whose own rows are
Unknown (`collector_create`, `flb_sched_create`, `flb_sched_timer_cb_create`,
`flb_input_collector_pause`, `flb_sched_request_invalidate`, ...) or of
`pthread_mutex_lock`/`_unlock` under the concurrency rules (15n/15o). A
declaration of a body that does not validate would be a declared assumption
on the chains this card is clearing, and the mutex pair belongs to the
thread-synchronization obligations: none is mechanical, so class (b) is named,
not fixed. Classes (c) and (d) are named with the rule that keeps each.

## Pause point

Neither class (a) repair needs surface syntax or a rule with user-visible
semantics: each extends an existing reading to one declarator shape, and every
other shape keeps its refusal. Implemented here:

* **F1.** `read_record_body` reads a record whose only unparsed member
  declarations are function pointers (`SPECIFIERS ( * NAME ) ( PARAMETERS )`,
  one declarator, no array): the function-pointer member is recorded as
  unreadable and joins no class; every other member is classified exactly as
  before. A body with `{`, `}`, `:` or `=`, or any other parenthesised
  declarator, is still refused whole.
* **F2.** `read_parameters` reads a function-pointer formal of the same shape
  as a name of `Opaque` type (never admitted by any typed rule) instead of
  refusing the function.

## Prediction (F1 + F2)

**Chain rows.** Leaf trades on the class (a) rows; status moves only where (a)
is a row's last blocker:

* Unknown → Safe possible: `flb_input_collector_destroy` (slice, both
  configurations: its removal through `coll->instance->config->evl` matches no
  record, which is the identity under rule 12t, and `flb_free(coll)` then
  releases the formal), `flb_sched_timer_destroy` (system, both:
  `mk_list_del(&timer->_head)` then `flb_free(timer)`). Each would be read by
  hand; each rests on rule 12t not forwarding a retention, so the release of a
  formal whose interior a caller registered is the open lifetime obligation
  (`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`), named in the report beside it.
* Stay Unknown, leaves trade: `collector_start` (its two registrations reach
  rule 12t: `retention-undecided` at the exit; `mk_event_closesocket(coll->fd_event)`
  stays a member-release refusal), `flb_sched_timer_cb_disable` (the removal
  is the identity; the following `timer->timer_fd = -1` store keeps a refusal,
  as `tests/event_loop_retention.rs::disable_then_store` pins),
  `flb_sched_timer_create` (the stores of `sched` and `sched->config` keep
  `pointer-store-into-member`), `flb_input_collector_{pause,resume}` (`coll`
  from `get_collector`, class (d)), `flb_input_collector_delete` (the mutex
  pair, class (b)), `flb_sched_destroy` and `flb_sched_request_invalidate`
  (class (d)), `collector_create` and `flb_sched_timer_cb_create` (their
  stores of formals become `pointer-store-into-member`).
* No chain row Safe → non-Safe. `flb_sched_timer_cb_destroy` stays Safe.

**Routes.**

* System: custody 95 + at most 2 / 4 / 349 − at most 2.
* Slice: custody 52 + at most 2 / 1 / 347 − at most 2 / 4 Pending.
* Corpus: both repairs reach every record with a function-pointer member and
  every function with a function-pointer formal (librdkafka's op and conf
  records, Monkey's handlers, cmetrics). Prediction: no Safe → non-Safe;
  Unknown → Safe gains bounded by the functions whose only blockers were these
  refusals (the wave-36 census measured that standalone ceiling at 2 for the
  record repair; routes added since may raise it), estimated 0–15, each read
  by hand; leaf trades in tens of Unknown rows; an Unknown → Unsafe only where
  a newly typed body exposes a leak or violation, each read.
* Null property: the newly typed accesses can add dereference checks;
  predicted small movement between Unknown and the two Safe kinds on every
  route, each moved row read; no new Unsafe predicted.
