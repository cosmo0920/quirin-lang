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

| run | Safe / Pending / Unknown / Unsafe |
|---|---|
| base `1b7028430` | 1,171 / 2 / 3,623 / 8 |
| tip-1 `d671a4447` (part B only) | 1,171 / 2 / 3,623 / 8 |
| **tip-2 `9c479777c`** (B and C; later commits change docs, Lean and tests only) | **1,171 / 2 / 3,623 / 8** |

**No status moves** at either tip
([moved-rows-base-tip2.jsonl](evidence/moved-rows-base-tip2.jsonl):
`{"moves": {}}`). Access rows 2,118 / alone 215 / leaves 8,992, unchanged.
110 rows change leaves only
([changed-rows-base-tip2.txt](evidence/changed-rows-base-tip2.txt); 96 at
tip-1, [changed-rows-base-tip1.txt](evidence/changed-rows-base-tip1.txt)):

* `possibly-null resource … returned under a non-null result mode` +121;
* `nullable resource … used by a non-null contract` −120;
* `unresolved resource identity` −118;
* `unmodeled call, aggregate or indirect access in branch condition` +4
  (four `cmt_decode_opentelemetry.c` rows: `data_point_list` now survives to
  its refused condition, which names itself instead of the identity).

Part C alone touches 14 rows (ShareFetch, `rd_kafka_mock_handle_Fetch`,
`rd_kafka_offsets_store`, `rd_kafka_handle_OffsetCommit`,
`rd_kafka_topic_metadata_update`, `mk_rconf_read`, three
`cmt_encode_opentelemetry.c` initialisers, `rd_hdr_histogram_min`, the four
decoders above).

**The eight rows and their 11 leaves**
([rows-base-tip2.txt](evidence/rows-base-tip2.txt)):

| row | nullable leaves base → tip-2 | status | what remains, and why |
|---|---:|---|---|
| `rd_kafka_cgrp_owned_but_not_exist_partitions` | 1 → 0 | Unknown | the new return leaf; the per-iterate identity leaf from the NULL-only entry state (rule 6); the `RD_KAFKA_TPLIST_FOREACH` increment, the `rd_list_find` condition and the uncontracted `add_copy` block the summary |
| `rd_kafka_cgrp_calculate_subscribe_revoking_partitions` | 1 → 0 | Unknown | the new return leaf; the destroy's identity leaf (23532); the loops' increments and the uncontracted `rd_kafka_topic_partition_list_add` |
| `rd_kafka_mock_handle_ShareFetch` | 3 → 0 | Unknown | the three guarded releases are decided; three identity leaves on the condition and loop reads at 24874-24894 and the row's other refusals (270 leaves) |
| `rd_kafka_cgrp_consumer_assignment_with_metadata` | 1 → 1 | Unknown | REGISTRY-UNDECIDED-CALL-POISONS-EVERY-BINDING |
| `rd_kafka_cgrp_handle_SyncGroup_memberstate` | 1 → 1 | Unknown | UNCONTRACTED-ARM-JOIN-NULLABLE-LEAF |
| `rd_kafka_mock_cgrp_consumer_member_needs_revocation` | 2 → 2 | Unknown | UNCONTRACTED-ARM-JOIN-NULLABLE-LEAF |
| `rd_kafka_mock_handle_ConsumerGroupHeartbeat` | 1 → 1 | Unknown | LATER-DECLARATOR-INITIALISER-UNREAD |
| `balance` | 1 → 1 | Unknown | FLAG-CORRELATED-RELEASE |

5 of the 11 leaves are gone (2 renamed to the leaf that names the return-mode
obstacle, 3 decided); the other 6 stay, each with its obstacle named and
registered. No row moved, no leak appeared or vanished.

**Against the prediction.** As predicted: no status moves on the opt-in
route; `owned_but_not_exist` and `revoking` lose their nullable leaf to the
new one; `needs_revocation`'s return is not renamed (its binding is unknown);
the other rows keep theirs. Not predicted: part C, added after the prediction
when the ShareFetch trace showed the `&&` reading; it clears ShareFetch's
three leaves (the prediction said they stay) and touches 13 other rows,
leaf-only.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Prediction for the integrator,
matching the committed `measure-prediction` block (`none` on every route):

* corpus (eight-home, twelve-home), system and tail-pause slice: no status
  moves; leaf-only renames where a resolved, possibly-null binding is returned
  by a body whose summary did not validate, and fewer identity / nullable
  leaves where a name after `&&` now survives a refused statement;
* possible, not expected: part C sparing a name could expose a violation
  (provisional Unsafe) or let an admitted statement's frame credit a release
  (Unknown → Safe, obligations discharged), as rule 74c recorded for its own
  frame widening. Either would be an unpredicted move.

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 1b7028430 --out
private/nlc/gate-2` through `private/toolchain/env.sh` (Clang 21, Lean 4.35),
head `e9576d2f6` (later commits change only this report and its evidence).
Record: [gate/gate-2.json](gate/gate-2.json). Logs are in the worktree's
`private/nlc/gate-2/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 677 units, 722 summaries; 9,904 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 484 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

An earlier attempt (`gate-1`, head `19b628f8e`) ran while the tree was being
edited and is discarded (its only failure was `cargo fmt`). No provider file
changed, so the provider gate was not selected.

## 6. Ledger and follow-ups

**NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP is retired.** Its DISCHARGE clause asked
to read each shape, refine it or name the refusal, and re-measure the eight
rows. The lazy guard needed no refinement (already decided, now pinned); the
two return rows' leaf names its obstacle; ShareFetch's leaves are decided by
the `&&` frame fix; the other five rows' obstacles are named and each is a
new entry, because none is a nullability problem.

Registered in [integration-amendments.json](integration-amendments.json):

* **LATER-DECLARATOR-INITIALISER-UNREAD**: `T *a = X, *b = Y;` reads `*b = Y`
  as a store (ConsumerGroupHeartbeat).
* **REGISTRY-UNDECIDED-CALL-POISONS-EVERY-BINDING**: the refused
  `*missing_topic_ids = rd_list_new(…)` poisons bindings it cannot write
  (`assignment_with_metadata`).
* **UNCONTRACTED-ARM-JOIN-NULLABLE-LEAF**: a constructor joined with an
  uncontracted result (`needs_revocation`, `SyncGroup_memberstate`).
* **FLAG-CORRELATED-RELEASE**: `balance`'s copy-on-write flag.
* **AMPERSAND-RUN-PARITY-OTHER-READERS**: other token readers decide
  address-taking from one `&`; `loop_owner_instance::address_taken` misses
  `p && &q` (the unsound direction).

Out of scope, met, already registered (not registered again):
OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY (the `rd_list_new` out-slot
delivery), RK-TOPIC-PARTITION-LIST-ADD-ELEMENT-POINTER,
SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN (no withdrawal moved here).

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`: `TransferContext`
  (`formal_names`), its construction, the `EscapeIfOwned` arm.
* `crates/qcore-c-gateway/src/legacy_c.rs`: the module list,
  `frame_protected_locals` and the new `unary_address_runs`.
* `crates/qcore-c-gateway/tests/resource_kinds.rs` (the one pin).
* `docs/agents/rules/10a.md`, `14l.md` (appended amendment notes).
* Generated: `docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean`; rerun their generators on conflict.
