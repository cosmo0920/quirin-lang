# FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: the access and identity blockers on the event-loop chains

Census and targeted fixes, card-high. USER DECISION 2026-10-17 D6
("Blockers first"): before a loop-lifetime model is built, clear the access
and identity refusals that keep the registration and teardown chains from
reaching rule 12t. Authority: the ledger entry of the same name and the
census of [FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME](../fb-event-loop-cross-function-lifetime/README.md)
(§1.4 R1–R6, §7).

Base: the census and [PREDICTION.md](PREDICTION.md) at local `main`
`3f8f1a617` (= `a423fca5d`'s analyzer); rebased onto `b157c9cba`
(RETENTION-UNMATCHED-REMOVAL-POISONS) before the final measurement. No surface
syntax and no new rule id: rule [5g](../../agents/gateway.md#constraint-5g) is
amended in place.

## 1. What changed

Both fixes are the lexical declaration reader's (`typed_field_access.rs`),
extended to one declarator shape it refused whole:

* **F1, records.** `read_record_body` refused a whole record body on any `(`.
  It now reads a body whose only unparsed member declarations are exactly
  `SPECIFIERS ( * NAME ) ( PARAMETERS )` (one declarator, no array, no
  qualifier, a balanced parameter list that closes the declaration). That
  member joins no class, so every access of it keeps its refusal; every other
  member is classified as in the same body without it. A `{`, `}`, `:` or `=`,
  and any other parenthesised declarator (an array of function pointers, an
  attribute, a second declarator), still refuse the whole record
  (`record_body_refusal`, which the record-body census now shares).
* **F2, formals.** `read_parameters` refused every declaration of a function
  with a function-pointer formal. It now reads such a formal as a name of
  `Opaque` type, which no typed rule admits.
* **Rule 5g keeps its domain.** Rule 5g lowers `B->F = scalar` (with
  `may_bind(B)`) to a store whose flow check refuses an unsettled base, and it
  runs before the statement's refusal reaches the Clang-typed fallback
  (rule 10k). On the first corpus run (`after1`, §4.3) F1 handed
  `cio_options_init`'s stores to it and the row went `Safe -> Unknown`.
  `scalar_member_store_base` therefore reads the member through
  `scalar_field_of_a_fully_read_record` and declines a function with a
  function-pointer formal, so every statement keeps the route order it had.

Premise stated, not proved: a function pointer holds a code address, never an
object the projection tracks. Lean: `QCore.FunctionPointerDeclarators`
([`proof/QCore/FunctionPointerDeclarators.lean`](../../../proof/QCore/FunctionPointerDeclarators.lean),
audit `proof/AuditFunctionPointerDeclarators.lean`):
`skipping_preserves_the_other_members`,
`a_function_pointer_member_is_never_classified` (over
`a_read_member_is_a_parsed_declaration`),
`an_unreadable_member_refuses_the_record`,
`a_body_without_unreadable_members_is_read`,
`the_reading_extends_the_old_reading`,
`a_record_read_past_a_function_pointer_is_outside_the_store_route`,
`on_a_fully_read_record_the_store_route_reads_as_before`,
`a_function_pointer_formal_is_opaque`, `an_opaque_formal_is_never_committed`,
`an_unreadable_formal_refuses_the_function`,
`a_function_read_past_a_function_pointer_formal_is_outside_the_store_route`,
`vectors_hold`. Tests: the reader's unit tests (the former
`a_function_pointer_member_refuses_the_whole_record` pin is replaced by
`a_function_pointer_member_joins_no_class_and_the_rest_is_read` and
`any_other_parenthesised_member_still_refuses_the_whole_record`; formals;
`rule_5g_keeps_its_domain_of_fully_read_declarations`) and route G's
[`tests/function_pointer_declarators.rs`](../../../crates/qcore-c-gateway/tests/function_pointer_declarators.rs)
(the registration/removal pair on a record with a function-pointer member is
`Safe`, the registration alone reaches `retention-undecided`, an array of
function pointers keeps the access refusal; two rows state the base
behaviour they were measured against).

**Pinned tests updated deliberately.** Five route-G test files used a
function-pointer declarator as their "the lexical reader cannot read this"
device: `typed_root_declarations.rs` (23 sites) and
`cg26_tail_residual_actuals.rs` (3) with a formal `int (*cb)(int)`, and the
certificate-substitution tests of `o04_arms.rs` (3),
`o04_constructor_caller.rs` (2) and `borrowed_field_actuals.rs` (1) with a
member `void (*hook)(int);`. Gate g1 failed them (10 tests). Each now uses an
array of function pointers (`int (*cb[1])(int)`, `void (*hook[1])(int);`),
which the reader still refuses, so every test keeps its subject: the AST
reader answering where the lexical one answers nothing, and a certificate
refused when a record body becomes unreadable.

## 2. Census

Scope: exactly the ledger entry's chain list (the lifetime card's
`scripts/chain_blockers.py`): 36 system and 50 slice rows (both
configurations). [`scripts/chain_blockers_census.py`](scripts/chain_blockers_census.py)
lists every access leaf (`indirect memory access …`, a nested or unmodeled
call) and identity leaf (`unresolved resource identity`) with its coordinate,
source line and, from an `indirect-census` build, the gate's own view of the
refused statement (base, base kind, record tag, member class, the base
type's cause). [`scripts/classify.py`](scripts/classify.py) gives each a kind
and the brief's class; every leaf the rules did not cover was read by hand and
is in its `OVERRIDES` (BEFORE) or `OVERRIDES_AFTER` with its reason; 0 leaves
are unclassified. An identity leaf poisoned by an earlier refusal of the same
body counts under that refusal's kind. Sizes reproduce the ledger entry.

BEFORE (`3f8f1a617`; `artifacts/before-{chain,classes}-*.json`), rows /
access / identity:

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

Totals (access / identity): system (a) 26 / 42, (b) 6 / 46, (c) 58 / 44,
(d) 2 / 52; slice (a) 12 / 4, (b) 8 / 12, (c) 172 / 76, (d) 14 / 56. Beyond
their own kind, 8 system and 30 slice leaves also ended in a member of a
record refused for a function-pointer member (`also_behind_*`).

The kinds, with the rule that keeps each (the `kinds` table of the artifacts
holds the full text):

* **(a)** `fn-pointer-member-record`: every member of `flb_input_collector`
  (`cb_collect`), `flb_sched_timer` (`cb`), `mk_event` (`handler`) and
  `flb_input_plugin` was untyped (the records stream's `refused_on: "("`);
  `fn-pointer-parameter-function`: `collector_create`'s and
  `flb_sched_timer_cb_create`'s declarations were unread. Both fixed (§1).
* **(b)** `formal-handed-to-uncontracted-call`, `uncontracted-result`,
  `uncontracted-call-with-member`: a callee with no contract on the route,
  mostly Fluent Bit functions whose own rows are Unknown (`collector_create`,
  `flb_sched_timer_create`, `flb_sched_create`, `flb_input_collector_pause`,
  `flb_sched_request_invalidate`, `MK_EVENT_ZERO`) and the
  `pthread_mutex_lock`/`_unlock` pair. A declaration of a body that does not
  validate would be a declared assumption on the very chains being cleared,
  and the mutex pair belongs to the thread-synchronization obligations
  (rules 15n/15o): not mechanical, named.
* **(c)** `pointer-store-into-member` (a formal's or a read pointer stored
  into a record: W45-SUB-OBJECT-STORES, C00 store direction),
  `member-release` (a release or close of a member outside W45-CALLEE-SUMMARY-CASCADE's
  P1–P4, rule 5k), `member-descriptor-slots` (descriptors written into a
  caller's memory, C04.6), `logging-macro` and `nested-or-condition-call`
  (CG05 nested calls, an array member's decay), `errno`
  (CG05-CALL-ROOTED-PLACES), `callback-member-call` (rules 12n/13u),
  `formal-may-bind` (rule 5g's settled-base check on a formal the body
  releases or links), `pointer-read-from-member`, `void-thread-argument`,
  `tls-get`, `after-retention-undecided` (rule 12t, the lifetime card's), and
  since `b157c9cba` `loop-destroy-at-path` (RETENTION-PATH-LOOP-DESTROY-ADMISSION).
* **(d)** `list-or-lookup` (list iteration, `get_collector`, the loop's
  ready list), `interior-address-of-looked-up-record`.

**The new base.** RETENTION-UNMATCHED-REMOVAL-POISONS declared
`mk_event_loop_create` / `_destroy`. On the chain rows (main `b157c9cba`,
`scripts/chain_blockers_census.py` over its plain reports) that adds
`loop-destroy-at-path` at `input_thread_instance_destroy` and `flb_destroy`
(+4 slice access leaves) and an identity leaf where the created loop is stored
into a member (`input_thread_instance_create` `thi`, `flb_engine_start`
`evl`; +2 and +2), and removes three `call to uncontracted function
mk_event_loop_*` leaves: system 92 / 186, slice 210 / 150. No status moved.

## 3. Pause point

Neither class (a) repair needs surface syntax or a rule with user-visible
semantics: each extends an existing reading to one declarator shape, every
other shape keeps its refusal, and rule 5g's domain is unchanged. Classes (b),
(c) and (d) need declarations of Unknown bodies, rule changes or identity
models, so they are named, not fixed.

## 4. Measurement

Binaries: base `77fb9a35…` (main `b157c9cba`), tip `b7babf64…` (`9978c37a4`).
Each route ran under the shared heavy lock with the null property
([`scripts/measure.sh`](scripts/measure.sh)); `run.json`'s `binary_sha256`
equals each binary. Identities: [`artifacts/runs-base.json`](artifacts/runs-base.json),
[`artifacts/runs-after.json`](artifacts/runs-after.json) (and
[`artifacts/runs-before.json`](artifacts/runs-before.json) for `3f8f1a617`,
equal to round 31). Row-by-row movement: [`scripts/compare.py`](scripts/compare.py),
`artifacts/movement-{corpus,system,slice}.json`.

| Route | Custody base → tip | Null base → tip | Rows trading leaves |
| --- | --- | --- | --- |
| corpus `66910c10` | 939 / 9 / 3,886 → **941 / 9 / 3,884** | 1,114 / 1,435 / 2,354 / 18, unchanged | 104 |
| system `046accf6` | 95 / 4 / 349 → **97 / 4 / 347** | 86 / 125 / 243, unchanged | 34 |
| slice `90978ac6` | 52 / 1 / 347 / 4 Pending, unchanged | 69 / 121 / 216, unchanged | 34 |

The null rows are identical on every route except the custody column of the
four newly Safe rows and its `custody_null_premises`.

### 4.1 Safe gains, read by hand

* **`flb_sched_timer_cb_disable`** (system, both configurations).
  `if (timer->timer_fd != -1) { mk_event_timeout_destroy(timer->sched->evl, &timer->event); timer->timer_fd = -1; }`.
  `flb_sched_timer` is now read past `cb`, so the chain
  `timer->sched->evl` and the sub-object address `&timer->event` are typed;
  the removal names a path holder and matches no record of this body, so it
  is the identity (rule 12t; RETENTION-UNMATCHED-REMOVAL-POISONS poisons only
  existing records), and the scalar store is decided by the Clang-typed
  statement reading as before. Dependencies: `mk_event_timeout_destroy`
  (declared, premise `mk_event_descriptors`: the timer descriptor the call
  closes is not tracked by v1 custody). A custody verdict only: the loop
  lifetime of the removed registration is FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME's.
* **`mk_sched_worker_cb_free`** (corpus, Monkey `mk_scheduler.c`). An
  `mk_list_foreach_safe` over `server->sched_worker_callbacks` unlinks each
  `wcb` (`mk_list_del(&wcb->_head)`, `resource_unpublished`) and frees it
  (`mk_mem_free`, inherited from `libc::free`). `mk_sched_worker_cb` is now
  read past `cb_func`, so `&wcb->_head` is an addressable member. Rests on
  rule 5l's list iteration; dependencies `mk_list_del`, `mk_mem_free`.
* **`mk_sched_conn_pending_output`** (corpus). Reads only: the condition
  `input->bytes_total > 0` through the first entry of `stream->inputs`
  (`mk_stream_input` is now read past its callbacks). Dependencies: the
  validated summaries `mk_channel_is_empty`, `mk_list_is_empty`.

### 4.2 Leaf trades

Corpus: 356 leaves leave and 65 arrive, all on Unknown rows: 25 `call to
uncontracted function`, 13 `publication by mk_list_add`, 3 `unlink by
mk_list_del` and 2 `loop re-acquisition` refusals the cleared accesses had
hidden, and 22 `unresolved resource identity`, of which one row
(`rd_string_render`: `t`, `te` from `strstr`/`strchr`) has them at lines that
had no leaf; it loses six. System and slice: every chain row's class (a) leaf
is gone (§5); the exposed leaves are named there. Three slice rows outside
the chains (`flb_input_set_test`, `flb_output_set_http_test`,
`flb_output_set_test_with_ctx_callback`) and `flb_input_net_property_check`
gain one or two identity leaves at statements the Clang-typed fallback
decided before (§4.3); each row was Unknown before and after.

### 4.3 Corrections, prediction misses and the precedence finding

* **No Safe → non-Safe** at the tip. The first AFTER run (`after1`, F1 and F2
  without rule 5g's domain, on `3f8f1a617`) moved `cio_options_init`
  (chunkio) `Safe -> Unknown`: `memset(options, …)` then scalar stores, which
  the Clang-typed reading had decided, were taken by rule 5g's store and
  refused (`artifacts/movement-corpus-first-run.json`). That was a lost
  `Safe`, not a correction (the old verdict was sound), and the domain fix
  restored it; the tip measures it `Safe`.
* **The precedence itself** is registered as
  `LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS`: rule 5g's store, W43's typed
  scalar statement and the W44 chain reading of a condition all run before
  the Clang-typed fallback and refuse bases it decides (each read from the
  typed-path parity streams of the BEFORE and AFTER census runs). This card
  keeps 5g's domain only. The others account for the four slice rows' new
  identity leaves: `flb_input_net_property_check`'s conditions
  `ins->p->flags & …` (W44 chains, now typed through `flb_input_plugin`;
  `ins` is later destroyed), and `i_ins->test_mode = 1` / `o_ins->…` in the
  three test setters (W43's typed scalar store, reachable now that their
  function-pointer formals no longer refuse the function; `i_ins` comes from
  `in_instance_get`).
* **Prediction misses.** `flb_input_collector_destroy` stays Unknown: its
  loop is `config->evl` with `config = coll->instance->config`, a pointer
  loaded from the formal's members (`pointer-read-from-member`), not the
  record reading. `flb_sched_timer_destroy` stays Unknown:
  `mk_list_del(&timer->_head)` now reaches the container rule (`unlink …
  which this function has not published`). `flb_sched_timer_cb_disable`
  became `Safe` against the prediction: the store after the removal is decided
  by the Clang-typed reading on a record with a function-pointer member (the
  fixture `disable_then_store` has none, so rule 5g refuses it there). The
  corpus estimate (0–15 gains) holds at 2; the null property did not move.

## 5. Residual on the chain rows

AFTER at the tip (`artifacts/after-{chain,classes}-*.json`, from an
`indirect-census` build of the tip whose rows equal the plain run's):
system 74 access / 132 identity leaves (from 92 / 186), slice 190 / 124 (from
210 / 150). **Class (a) is 0 on both routes.**

| Route | Class | Kind | Rows | Access | Identity |
| --- | --- | --- | --- | --- | --- |
| system | (b) | `formal-handed-to-uncontracted-call` | 6 | 6 | 12 |
| system | (b) | `uncontracted-call-with-member` | 2 | 0 | 14 |
| system | (b) | `uncontracted-result` | 8 | 6 | 26 |
| system | (c) | `callback-member-call` | 4 | 4 | 0 |
| system | (c) | `formal-may-bind` | 8 | 0 | 12 |
| system | (c) | `logging-macro` | 2 | 8 | 0 |
| system | (c) | `member-descriptor-slots` | 4 | 8 | 0 |
| system | (c) | `nested-or-condition-call` | 2 | 4 | 0 |
| system | (c) | `pointer-store-into-member` | 12 | 36 | 34 |
| system | (c) | `tls-get` | 2 | 0 | 4 |
| system | (d) | `interior-address-of-looked-up-record` | 2 | 2 | 10 |
| system | (d) | `list-or-lookup` | 8 | 0 | 20 |
| slice | (b) | `uncontracted-call-with-member` | 4 | 8 | 0 |
| slice | (b) | `uncontracted-result` | 10 | 0 | 12 |
| slice | (c) | `callback-member-call` | 8 | 22 | 6 |
| slice | (c) | `formal-may-bind` | 4 | 2 | 16 |
| slice | (c) | `logging-macro` | 20 | 62 | 2 |
| slice | (c) | `loop-destroy-at-path` | 4 | 4 | 0 |
| slice | (c) | `member-descriptor-slots` | 6 | 12 | 14 |
| slice | (c) | `member-release` | 8 | 54 | 18 |
| slice | (c) | `pointer-read-from-member` | 4 | 4 | 6 |
| slice | (c) | `pointer-store-into-member` | 8 | 16 | 12 |
| slice | (c) | `void-thread-argument` | 2 | 0 | 10 |
| slice | (d) | `interior-address-of-looked-up-record` | 2 | 6 | 0 |
| slice | (d) | `list-or-lookup` | 10 | 0 | 28 |

Chain rows that moved (first configuration; access / identity, base → tip):
`collector_start` 3/0 → 1/0, now refusing **only** at rule 12t
(`retention-undecided` at both registrations) and at
`mk_event_closesocket(coll->fd_event)` (`member-release`); `collector_create`
7/2 → 3/3; `flb_input_collector_pause` 3/1 → 1/4 and `_resume` 5/10 → 5/2
(`coll` from `get_collector`, class (d)); `_delete` 4/1 → 3/2;
`flb_input_instance_destroy` 26/10 → 25/9; `input_thread` 14/19 → 14/10;
`flb_engine_start` 21/27 → 21/11 (the engine loop's 16 event leaves);
`flb_sched_create` 5/13 → 4/10; `flb_sched_timer_cb_create` 4/7 → 4/1;
`flb_sched_timer_create` 5/4 → 1/7 (`MK_EVENT_ZERO` uncontracted, then
`timer`); `flb_sched_timer_destroy` 1/1 → 0/1 and `_invalidate` 2/1 → 0/1
(the unlink rule); `schedule_request_now` 0/4 → 0/2;
`flb_sched_timer_cb_disable` 1/0 → Safe. Unchanged:
`flb_input_collector_destroy` 2/1, `flb_sched_destroy`,
`flb_sched_request_invalidate` (class (d)).

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), idempotent; 793 → 794 (on main `1474edaf1`):

* **`FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS` narrowed** to the residual of §5,
  every leaf named with its rule; it stays open because its purpose (a
  lifetime model showing movement on these rows) is met only by
  `collector_start` so far.
* **`RECORD-BODY-TOTAL-OR-REJECT` amended**: the function-pointer part is
  repaired; `{`, `:`, `=` and other parenthesised shapes remain.
* **`W44-CHAIN-INCOMPLETE-RECORD` amended** likewise for chains.
* **`LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS` registered** (§4.3).

## 7. Follow-ons

* `MK_EVENT_ZERO` is uncontracted on the system and slice routes and poisons
  `timer` in `flb_sched_timer_create` and `coll` in `collector_create`; a
  summary or declaration of it is the next mechanical step on two chain rows.
* `flb_input_collector_destroy`'s loop is read through
  `coll->instance->config`; reaching rule 12t there needs a pointer-read
  identity, which is part of the lifetime card's equality model (D3).
