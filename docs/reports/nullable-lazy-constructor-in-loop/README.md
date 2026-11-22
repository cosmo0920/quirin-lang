# NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP: the lazy guard was already decided; three readers named or fixed

Card NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP (USER DECISION 2026-11-23: launch after
RK-TOPIC-PARTITION-LIST-CONSTRUCTORS landed). Base `1b7028430` (main, r57).
[PREDICTION.md](PREDICTION.md) was committed (`5341e3fb6`) before the
implementation (`62f63371a`, `9c479777c`). Gateway rule
[74d](../../agents/gateway.md#constraint-74d), amending rules 14l and 10a;
`QCore.NullableLazyConstructor` with its audit.

## 1. What the eight rows actually were

Each row was read in the prepared checkout (librdkafka 2.15.0) and traced on
the real preprocessed unit: a temporary, uncommitted trace build printed every
transfer and join that made the row's binding unknown or possibly null, once
on a local reduced-contract harness and once on the real opt-in route over a
four-unit selection ([evidence/runs.md](evidence/runs.md)). The ledger named
two shapes. Neither was what the leaves came from.

| row | where the leaf is | the real obstacle | shape |
|---|---|---|---|
| `rd_kafka_cgrp_owned_but_not_exist_partitions` | `return result;` | `result` is genuinely NULL when no partition qualifies; a possibly-null return needs a nullable result mode, which only the validated summary gives, and the row's other refusals block the summary | (1) as a **return**, not a guard |
| `rd_kafka_cgrp_calculate_subscribe_revoking_partitions` | `return revoking;` | same: `destroy(revoking); revoking = NULL;` then return. No poisoning (the trace shows no refusal touching `revoking`) | same as (1), not (2) |
| `rd_kafka_cgrp_consumer_assignment_with_metadata` | `return …;` with the binding unknown | `*missing_topic_ids = rd_list_new(1, rd_list_Uuid_destroy)` (`rdkafka_cgrp.c:3036-3037`): a refused contract call (registry holder undecided) whose refusal poisons every binding, `assignment_with_metadata` among them, at the loop head | (2): a frameless refusal |
| `rd_kafka_cgrp_handle_SyncGroup_memberstate` | the unconditional destroy | the other arm assigns `rd_kafka_buf_read_topic_partitions(...)`, an uncontracted result, inside a refused branch condition | join with an uncontracted result |
| `rd_kafka_mock_cgrp_consumer_member_needs_revocation` | return and destroy | the other arm assigns `rd_kafka_topic_partition_list_intersection_by_id(...)`, uncontracted, which binds nothing; the join makes the name unknown | join with an uncontracted result |
| `rd_kafka_mock_handle_ConsumerGroupHeartbeat` | `RD_IF_FREE(existing_assignment, …)` | the declaration `*sent_assignment = NULL, *existing_assignment = NULL, …` reads the later declarators as stores through `*existing_assignment` (`indirect memory access` at the declaration), so the name has no binding on the path that skips its construction | a later declarator unread |
| `balance` | `if (…_allocated) destroy(leftover…)` | `leftoverUnassignedPartitions` is the borrowed formal or a copy, and a boolean records which: a flag-correlated release | flag correlation |
| `rd_kafka_mock_handle_ShareFetch` | three `RD_IF_FREE(forgotten_partitions, …)` | rule 10a's frame dropped `forgotten_partitions` as address-taken because of `session && forgotten_partitions` (`:4291`): the lexer spells `&&` as `&` `&`, and the frame read the second `&` as `&forgotten_partitions`. Every refused statement then poisoned it, so the guards were refused | a frame reader defect |

The lazy guard of shape (1) is **already decided** (rules 14l, 10b) in a loop
and outside one: the guard's non-null edge keeps the earlier site and its
obligation, its null edge holds no site and constructs, both arms leave the
name non-null, and a loop head keeps that only when every edge has it (the
NULL entry edge makes the name NULL on the zero-iteration exit, which is C).
Pinned in the fixtures below; it needed nothing new.

## 2. Changes

**(B) The return leaf names its obstacle**
(`crates/qcore-c-gateway/src/legacy_nullable_return.rs`, new; one hook in
`legacy_transfer.rs`'s `EscapeIfOwned` arm; one context field). A returned
binding that is resolved (identity derivation, one target, no unknown
alternative) and may be null, under a non-null result mode, is refused by
one leaf, `possibly-null resource `NAME` returned under a non-null result
mode`, in place of `unresolved resource identity` plus `nullable resource …
used by a non-null contract` (no contract is involved; the identity is
resolved). The marking is exactly `resource_action`'s refusing branch, so the
state, the schedule and every leak claim are unchanged. A formal named by the
entry's contract keeps the old pair (the W4 search binds a formal it sees as
an unresolved identity), and a targetless binding keeps it too. Accepting the
return in the contract-free run was rejected: a nullable result needs the
validated summary.

**(C) An even run of `&` takes no address** (`unary_address_runs` in
`legacy_c.rs`, one loop in `frame_protected_locals`). By maximal munch
(C11 6.4p4) a run of `k` adjacent `&` is `k/2` `&&` plus one `&` when `k` is
odd; `a & &b` is not conforming (binary `&` takes integer operands,
C11 6.5.10p2). The frame now drops a name only after an odd run. The
conditional names (rule 10b) are the unique locals intersected with the
frame, so they widen with it.

## 3. Tests and pins

[`tests/nullable_lazy_constructor.rs`](../../../crates/qcore-c-gateway/tests/nullable_lazy_constructor.rs),
15 fixtures:

* lazy construction in a loop (`!r` and `r == 0`) with a borrowed use and a
  guarded release: `Safe`; released and reset per iteration: `Safe`;
* lazy construction outside a loop: `Safe` alone, and decided beside a
  refused statement; with an earlier construction on one path the guard
  decides nullability and the release is refused on identity only;
* a reassignment after the guard (in and out of a loop), an escaped slot
  (`u_take(&r)`), and a release after the loop outside any guard: refused,
  no leak;
* a genuine leak in a lazy loop (an early return): `Unsafe`;
* the returned lazy loop: `Safe` with no refusal (the nullable summary
  validates); beside a refused call, the new leaf, no leak (the per-iterate
  identity leaf from the NULL-only entry state stays, rule 6);
* the `revoking` return: the new leaf; an unknown binding returned keeps its
  identity refusal;
* the other shapes stay refused without a leak (a later declarator, a join
  with an uncontracted result, a flag-correlated release);
* `flag && p` beside refused statements: the guard decides, no nullable leaf,
  no leak (fails under the old reading, checked); an odd run `flag && &p`
  still poisons.

**Deliberate pin update:** `resource_kinds.rs`
`a_possibly_null_handle_returned_without_a_contract_refuses` (`k_dlopen`
returned): `unknown_alias:h` plus the nullable leaf become the one new leaf,
still `Unknown` (comment in place). The `&&` reading moved no pinned
expectation (4,072 gateway tests pass).

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each run on its own
`git archive` tree and release binary ([evidence/runs.md](evidence/runs.md)).

TIP2-TABLE

## 5. Gate

GATE-SECTION

## 6. Ledger and follow-ups

LEDGER-SECTION
