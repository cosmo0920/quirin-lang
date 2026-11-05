# NULL-WITNESS-GUARD-CORRELATION

Round 22, 2026-11-06. Integrator-found defect: `land.py publish` at
`855b34ade` measured the corpus null property's provisional `Unsafe` rows
17 -> 23 against baseline `c8c7edf21`. The user queued this card next (both
offered answers: "Queue NULL-WITNESS-GUARD-CORRELATION as the next card").
Base `13337d0af` (origin/main); `855b34ade..13337d0af` changes no analyzer,
contract or Lean file, so the base's null rows are the `855b34ade` landing
measurement's.

## 1. Diagnosis

**Inputs** (main checkout, read only): base
`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/eight-home/corpus/corpus-null.jsonl`
(SHA-256 `feafb9e8…40ed46`) and tip
`private/measure/landing-855b34adef125604a6a94535ef15b1dcd7791d0e-20261105T134026829943Z/sides/tip/corpus/corpus-null.jsonl`
(`10f005e3…d1bbec`). [`scripts/diagnose.py`](scripts/diagnose.py) lists every
check that is a finding at the tip and was not one at the base, with both
sides' sources: [`diagnosis.json`](diagnosis.json), **7 checks in 6 rows**, all
librdkafka, no other null verdict moved on any route (system and slice have no
lookup source). Every row's custody is `Unknown`; its combined verdict moved
`Unknown -> Unsafe`; its null contract dependencies are identical at both
sides.

**The change: LIBRDKAFKA-HEADER-INLINES (`68c8065e1`), not rule 38h or
MEMBER-ACCESS.** Its module `librdkafka/header_inlines.qv` declares
`rd_kafka_msgq_first`, `rd_kafka_msgq_last` and `rd_tmpabuf_alloc0` with
`ensures normal resource_return_borrowed_from(P, nullable);`. Before, each
call's result was `uncontracted-result:NAME`, a weak source the path search
(rule 14r) never runs on. A nullable borrowed-from result is
`ResourceReturnMode::Derived { nullable: true }`, which the walk reads as
`Source::Lookup` (`null_deref/walk.rs`, the call's result) and
`STRONG_SOURCE_KINDS` includes `lookup`, so each candidate became searchable.
The dependencies did not move because the walk and the search read the
resource contract through `contract_dependencies::unrecorded` (the
`inferred:` entries are the in-unit summaries of the same inline bodies,
present at both sides).

**The validator step.** In the search (`null_deref/witness.rs`), `call`
gives a lookup's result `P::Opaque(Origin::Lookup(NAME, LINE), false)` and
`judge` names "the lookup `NAME` at line L misses (returns NULL)" as the NULL
event whenever that value reaches the target dereference. Nothing reads
whether the miss is feasible on the path: a lookup's result is a function of
the state its arguments reach, and the path may have decided, written or
built that state. That is the step that should have refused each witness.
(An allocation's failure is different: C11 7.22.3 lets any allocation fail,
whatever the state.)

| Row (corpus, librdkafka) | Check | Witness path (decisions, NULL event) | What the route can read | NULL event feasible on the path? |
|---|---|---|---|---|
| `rd_kafka_msgq_allow_wakeup_at` | 15, `rdkafka_msg.c:1830` `rkm->rkm_u` | `RD_KAFKA_MSGQ_EMPTY(dest_rkmq)` false (`:1819`), lookup `rd_kafka_msgq_first(dest_rkmq)` (`:1826`) | `rd_kafka_msgq_first`'s body is `return TAILQ_FIRST(&rkmq->rkmq_msgs);`, the place `(&dest_rkmq->rkmq_msgs)->tqh_first` that the `:1819` test fixed non-null; no call or store between | **No**: excluded by the guard |
| `rd_kafka_ProduceRequest` | 16, `rdkafka_request.c:5099` | `rkbuf` non-null (`:5084`), `required_acks` false (`:5093`), lookup `rd_kafka_msgq_first(&rkbuf->rkbuf_batch.msgq)` | the queue belongs to `rkbuf`, a call's result (`rd_kafka_msgset_create_ProduceRequest`); its length is read at `:5087` (`rd_dassert(cnt > 0)`), and no body ties the count to `tqh_first` (a list invariant) | **Not readable**: state another function built, not an entry value; not validated |
| `rd_kafka_handle_idempotent_Produce_success` | 9, `rdkafka_request.c:4789` | no decision; lookup `rd_kafka_msgq_first(&batch->msgq)` | the body reads `batch->msgq.rkmq_msgs.tqh_first`, an entry memory value; rule 14r never names a memory value's null-ness as a NULL condition | **Not a NULL event** of the lookup's own: the queue's emptiness at entry is the function's precondition domain, like a formal |
| (same) | 11, `rdkafka_request.c:4790` | no decision; lookup `rd_kafka_msgq_last(&batch->msgq)` | `TAILQ_LAST`: a read through `tqh_last`; the same queue was just passed to `rd_kafka_msgq_first` | **Not a NULL event** of its own (as check 9) |
| `rd_kafka_metadata_cache_entry_new` | 27, `rdkafka_metadata_cache.c:308` | `include_racks` false (`:289`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:306`) | `tbuf` is a local arena the function initialised (`rd_tmpabuf_new(&tbuf, 0, rd_true)`) and sized (`rd_tmpabuf_add_alloc`) on the path. `rd_tmpabuf_alloc0` (`rdkafka_buf.h:104-126`) returns NULL only if `tab->failed` (no tmpabuf body sets it non-zero) or `tab->of + size > tab->size`, where `assert_on_fail` asserts first (the corpus compile database does not define `NDEBUG`) | **No in this build** (the size branch aborts; `failed` stays 0); with `NDEBUG`, only on mis-accounting. The size arithmetic (string lengths, loops) is not readable: refuse |
| `rd_kafka_topic_info_new_with_rack` | 20, `rdkafka_topic.c:2201` | `i < partition_cnt` false (`:2175`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:2200`) | as above (`rd_tmpabuf_new(&tbuf, 0, rd_true)`, accounting at `:2173-2191`) | as above |
| `rd_kafka_handle_Produce_metadata_update` | 19, `rdkafka_request.c:3900` | `leader_change_cnt` true (`:3876`), lookup `rd_tmpabuf_alloc(&tbuf, …)` (`:3897`) | as above (accounting through three `rd_kafkap_leader_discovery_tmpabuf_add_alloc_*` calls) | as above |

So none of the seven is a validated reachable violation (AGENTS.md invariant
2): one is excluded by the guard, three rest on an arena the function itself
established, one on state a callee built, and two name an entry memory value
as a lookup's miss.

## 2. The rule: gateway rule 39

[Rule 39](../../agents/rules/39.md) (rendered in
[gateway](../../agents/gateway.md#constraint-39)); rule 14r's named-condition
list now defers a lookup's miss to it, and contract specification C09.3 says
the same. In the path search (`null_deref/witness.rs`, decisions in
`qcore_core::null_lookup`), at a call whose declared result is
`resource_return_borrowed_from(P, nullable)`:

1. **Tie.** When the unit holds the callee's definition and its body is one
   `return` of a load of a place built only from its parameters, member
   accesses, `&`, `*` and casts, the result is that place, spelled as the
   caller's own read of it (each parameter replaced by its argument's
   spelling), read before the call forgets the cached memory values. NULL
   there (the path's own test fixed it) names the miss: a finding whose
   condition cites the test. Anything else (fixed non-null, tested before a
   call or store, or an unconstrained memory value, which rule 14r never
   names) refuses it.
2. **State.** Otherwise the miss is named only when every argument is a
   constant (literal, enumeration constant, `sizeof`, predefined name) or is
   rooted at a formal no earlier decision, store, assignment or call on the
   path read or wrote through (a NULL test of a pointer root itself
   excepted), or at a global with, in addition, no call before. A local's
   state (the arena a function initialised and sized), a call's result and
   any form the search does not root are correlated. What a decision read
   includes the spellings it tested, the memory value a tested alias came
   from, and the memory spelling of every integer atom a decision narrowed
   or constrained.
3. **Refusal.** A refused miss is a constrained value
   (`Origin::Refused`): reaching the dereference it ends the path
   `unresolved` with its reason (never `Ended`), and a test of it is inexact
   (rule 15r). So it validates no witness, proves no candidate infeasible and
   makes nothing `Safe`; the tie is read only to name or refuse, never to
   prove a result non-null.

The six rows' refusals: `rd_kafka_msgq_allow_wakeup_at` by the tie (fixed
non-null by the `:1819` test), `rd_kafka_ProduceRequest` and
`rd_kafka_handle_idempotent_Produce_success` by the tie (entry or callee-built
memory no condition names; `TAILQ_LAST`'s cast read too, or, failing the tie,
the earlier `rd_kafka_msgq_first(&batch->msgq)` call), the three tmpabuf rows
by state (`&tbuf` is rooted at a local).

## 3. Lean parity

`QCore.NullLookupEvent` ([module](../../../proof/QCore/NullLookupEvent.lean),
audited by [`AuditNullLookupEvent.lean`](../../../proof/AuditNullLookupEvent.lean),
placeholder-free, no project axioms): `missEvent`, `argumentState`,
`lookupState`; theorems `a_named_miss_is_decided_or_free`,
`a_tied_lookup_is_named_only_when_decided_null`,
`a_correlated_untied_miss_is_refused`, `correlation_never_names`,
`touching_never_frees`, `a_call_before_never_frees_a_global`,
`a_local_or_unrooted_argument_is_correlated`,
`one_correlated_argument_correlates`, `a_free_lookup_has_only_free_arguments`,
`a_correlated_miss_never_contributes` (through
`QCore.NullTriagePaths.a_call_result_never_contributes`: a correlated miss is
a value the path chose, like a call result),
`a_refused_miss_never_concludes_infeasible`,
`a_refused_miss_is_never_a_finding`; `missVectors` (6) and `stateVectors` (9)
bound to the Rust by `crates/qcore-core/tests/null_lookup.rs`, which reads
them from the Lean text. That the search roots arguments, recognises a
place-read body and sees the touching decisions, stores and calls as the Rust
does is trust boundary 28, pinned by the fixtures below.

## 4. Fixtures

`crates/qcore-c-gateway/tests/null_witness_guard_correlation.rs`, each on a
real Clang AST (fixture contracts are test assumptions): the guard the body
ties to the result refuses (`allow_wakeup_at`'s shape); **true positive** a
lookup used on the empty arm stays a `null-on-normal-path` finding; an
accessor over entry memory is not a NULL event (`idempotent_Produce_success`);
a test through the argument correlates an untied lookup (the length test's
shape); control: an untied lookup over free state stays a finding (rule 14r
unchanged); a local arena the path established refuses (`rd_tmpabuf`'s
shape); **true positive** an arena received without size accounting stays a
finding; a call that received the argument correlates; a test of a refused
result is never a witness.

Against the base search (`witness.rs` at `13337d0af`, the fixtures
unchanged) the six refusal fixtures are `null-on-normal-path` findings and
both controls pass: the defect reproduced in miniature
(`private/logs/fixtures-at-base-search.log`, card-private).

## 5. Spot check on the real units (not the measurement)

[`spot-check/run.py`](spot-check/run.py): the card's release `quirin-audit`
(tree `635291591`) on `current`, the corpus checkout
`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source` (66910c10) and its
compile database, selecting only the four librdkafka units of the six rows
([`selection.json`](spot-check/selection.json)), with the null property
(`spot-null.jsonl`, SHA-256 `115ea019…a04a432`, card-private). The units'
preprocessed lines are the corpus run's, so every check keeps its id and
line. [`compare.txt`](spot-check/compare.txt): all seven former findings are
`unresolved` with rule 39's reason (the tie's "already fixed non-null" for
`allow_wakeup_at`; an unnamed memory value for `ProduceRequest` and both
`idempotent_Produce_success` checks, `TAILQ_LAST`'s cast read included; the
local `tbuf` for the three tmpabuf rows); the six rows are null `Unknown`,
combined `Unknown`; the units' 6 already-unresolved triage records stay
unresolved. Three other rows differ from the corpus run
(`rd_kafka_group_MemberState_consumer_write` Safe -> Unknown,
`rd_kafka_metadata_cache_propagate_changes` and `rd_kafka_topic_new` Unknown
-> Safe): none has a triage record; each rests on cross-unit travel
(`cross-unit-result:` / `cross-unit-inferred:`), which a four-unit selection
lacks. They are route artifacts of the spot check, not predictions; the
corpus measurement is the integrator's.

## 6. Prediction, measurement and records

[PREDICTION.md](PREDICTION.md) (committed before the fix, base
`rev:13337d0af`): the six rows back to `Unknown`, corpus null Unsafe 23 -> 17
and Unknown 2,349 -> 2,355, combined Unsafe 34 -> 28, no custody, system or
slice move. The restored null counts equal the `eight-home` baseline record's
(`contract/evaluations/eight-home/baseline.json`: Safe 2,549, Unknown 2,355,
Unsafe 17); its `null_verdicts_sha256` will still differ, because 38 null rows
carry custody (and 15 combined) values other cards of this round moved.

Follow-ups (registered through
[`integration-amendments.json`](integration-amendments.json)):
`NULL-LOOKUP-TIE-INFEASIBILITY` (a tie the path fixed non-null could remove
the candidate; held because this card never makes a row Safe),
`NULL-LOOKUP-STATE-INVARIANTS` (the tmpabuf arena discipline, `NDEBUG`
dependent, and the message queue's count/list invariant),
`NULL-LOOKUP-SOURCE-DEPENDENCY` (rows do not list the declaration a strong
source or finding rests on).

## 7. Card gate

`python3 scripts/gate_scope.py --base 13337d0af --out <worktree>/private/gate`
on head `635291591` (record [`gate.json`](gate.json); logs card-private under
`private/gate/`): mode **full** (the diff reaches `qcore-c-gateway` and
`qcore-core`); submodules exit 0; `cargo fmt --check` exit 0; `cargo clippy
--all-targets -D warnings` exit 0; Rust suite exit 0, **614 units, 9,256
passed, 0 failed, 16 ignored**; both reevaluation probes exit 0; Lean
`check_lean_audits.py` exit 0, **411 audits**; Python suite exit 0, **625
tests**. No provider path changed. The rust-suite slot was held by another
card's suite for about two hours; while it waited, the new fixtures ran once
outside the slot (`cargo test -p qcore-c-gateway --test
null_witness_guard_correlation`, `-p qcore-core --test null_lookup`: 9 and 4
passed), then the gate ran them under its slot.
