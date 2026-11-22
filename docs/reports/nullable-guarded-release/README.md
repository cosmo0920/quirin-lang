# NULLABLE-GUARDED-RELEASE: a refused release is no leak; a member address takes no slot

Card NULLABLE-GUARDED-RELEASE (USER DECISION 2026-11-22: "launch the
guarded-release fix card"). Base `96d41921c` (main). [PREDICTION.md](PREDICTION.md)
was committed (`be921682e`) before the implementation (`4d5325d05`, narrowed
in `686ddf8f4`). Gateway rule [74c](../../agents/gateway.md#constraint-74c),
amending rules 6 and 10a; `QCore.NullableGuardedRelease` with its audit.

## 1. What the Metadata row actually was

The held card RK-TOPIC-PARTITION-LIST-CONSTRUCTORS (`4174d25e5`) reported
`rd_kafka_mock_handle_Metadata` (librdkafka `rdkafka_mock_handlers.c`) as a
provisional Unsafe: `requested_topics` is NULL-initialised, built by
`rd_kafka_topic_partition_list_new` only when `TopicsCnt > 0` (`:1362`), and
released under `if (requested_topics)` on both exits (`:1485-1494`).

The guard refinement the brief asked for already existed (rules 14l, 10b): a
decided `if (p)` makes `p` non-null on its true edge (the `nonnull` release is
accepted) and removes the site on its null edge. A synthetic
`p = 0; if (n) p = ctor(); if (p) release(p);` was already Safe. Two other
things went wrong in the real body, found with a local reduced-unit probe and
the leak-witness census ([evidence/runs.md](evidence/runs.md)):

* **The binding never reached the guard.** `&requested_topics->elems[i]`
  (`:1415`) dropped the name from rule 10a's sparing frame, which removed any
  name an `&` precedes. Every refused statement (the `rd_kafka_buf_read_*`
  macros) then poisoned the binding, so the guard was refused (unknown
  binding, rule 10b), the release was refused (`unresolved resource identity`
  plus the nullable leaf), and the refused guard's null edge kept the site
  live.
* **The leak was an intermediate-iterate claim.** The refused release marked
  the site unresolved, and the union join carried the mark to `return 0`, but
  `report_leaks` runs on every iterate (rule 6) and claimed the site on an
  iterate the null edge reached first. The census records the leak
  `settled: false` with the site `unresolved: true` at the settled exit: the
  settled analysis did not make the claim the row carried.

## 2. Changes

**(A) Settled claims after a refused release**
(`crates/qcore-c-gateway/src/legacy_settled_leak.rs`, new). Every release
`apply_resource_action` applies records the identities it marked when it
refused, in `AbstractState::refused_release_sites` (union join, renamed with
loop generations). `report_leaks` holds each current-generation report, as
rule 42 holds earlier generations'; after the work list settles a report is
withdrawn exactly when the exit's settled incoming state marks the identity,
a refused release is among the marks, and the function carries an `Unknown`
leaf. Only `Unsafe -> Unknown` moves are possible.

The refusal kinds sharing this path, all recorded: a nullable resource at a
`nonnull` position, an unresolved or multi-target identity, an ambiguous
cascade, a retained, holder, registry or taken object, and a derived pointer.
Markers from other refusals (a refused test, a refused statement's hand-on, a
refused re-acquisition, a refused escape) are deliberately not read: a first
version withdrew any settled-unresolved claim and moved three pinned shapes
that are not refused releases (rule 42's snappy shape, the chained-assignment
scram shape and the scalar-route exit edge, each a genuine leak behind
another refusal). It was narrowed before measuring.

**(B) The frame reads a member address as C does** (one call in
`legacy_c.rs`). The frame now reads the body through rule 12e's
`member_addresses_read`: `&NAME->F` and `&(NAME)->F` take the address of a
member of `*NAME`, never of `NAME`'s slot, the reading the tie names already
had. `&NAME`, `&(NAME)` and `&NAME[i]` still drop the name; a write by name
still poisons. A surviving binding then reaches the existing guard
refinement. No new guard machinery was built.

Both halves are needed for the brief's acceptance. (B) alone makes the
Metadata row leak-free: the guard decides, so no refused release remains. (A)
is the fail-closed half for every refused release that remains.

## 3. Tests and pins

[`tests/nullable_guarded_release.rs`](../../../crates/qcore-c-gateway/tests/nullable_guarded_release.rs),
8 fixtures:

* the Metadata shape (refusals, `&p->elems[0]`, guarded releases on both
  exits): `Unknown` on the refusals, no leak, no nullable or `p` leaf. The
  base gives `UnknownAlias p` and the nullable leaf.
* without refusals: `Safe`.
* an unguarded `nonnull` release of a nullable resource: refused, no leak.
* a reassignment between guard and release: refused.
* an escaped slot (`u_take(&p)`): refused, no leak.
* an aliased slot (`void **pp = &p`): refused.
* a refused guard and release: no leak.
* a genuine leak on the non-NULL path, with or without a refused statement:
  `Unsafe`.

The schedule-dependent half of (A) is not reproducible in a small fixture.
The local schedule settles the marker first even on the real preprocessed
unit. It is pinned on hand-built settled states in
`tests/unit/legacy_settled_leak.rs`, together with the shared
`WITHDRAW_VECTORS` table bound to the Lean `withdrawVectors`.

**Deliberate pin updates** (each commented in place):

* *Frame (B), refusal kept, `unknown_alias` leaf gone:* `borrow_residuals_boundary`
  (`&b->item`), `cell_array_slots` ×2, `retaining_call_minted_name` ×2.
  `descriptor_cell_protocols` now names the record's own hazards
  (rules 38, 12t) instead of `unknown_alias:c`. `neighbours`
  `flb_input_chunk_create` before-state 19 → 9 diagnostics (data tail
  20 → 14). The `o04_arms` / `o04_constructor_caller` routes-mask arm now
  carries its two identities `Released` and the success exit three `Mixed`;
  both are still `Unknown`, with no residual.
* *Frame (B), Unknown → Safe:* `indirect_scalar_access`
  `x_note(&r->count); *c = 0; x_record_free(r);` is `Safe`. The admitted
  indirect scalar store carries the same frame and no refusal of its own, and
  the release is credited. The old fixture's comment claimed `&r->count`
  "takes an address inside `r`'s name", which is false in C. The test now
  pins `&r` (slot address) as still poisoned and `&r->count` as Safe.
  PREDICTION said Unknown → Safe 0; this fixture is the counterexample, and
  the rule states it.
* *Withdrawal (A), Unsafe → Unknown:* `condition_call_desugar`
  `if (flag && k_close(fd) == 0)`. The close of a possibly-invalid descriptor
  is a refused release, and its mark joins the `flag`-false path at the only
  exit. This is the precision (A) costs: a genuine leak on a path joining a
  refused release's path is withdrawn with it, because the settled state
  cannot tell the two apart.

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each run on its own
tree and binary ([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe |
|---|---|
| base `96d41921c` | 1,169 / 2 / 3,626 / 7 |
| **tip `686ddf8f4`** | **1,169 / 2 / 3,626 / 7** |

**No status moves** (as predicted). 119 rows change leaves only
([changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)):

* removed: `unresolved resource identity` −251, `nullable resource … used by
  a non-null contract` −37, `loop re-acquisition` −10, `reference-unbalanced`
  −4, publication −2, access −1;
* added: refusals the poisoned bindings had masked, `interior-borrow-ended`
  +31, `reference-dropped` +20, transfer-after-publication +2,
  `unresolved resource identity` +1.

Access-alone rows go 199 → 213. All 7 Unsafe rows are unchanged in status;
`mk_net_conn_create` loses five `unresolved resource identity conn` leaves
and keeps its leak.

**Second comparison, private tree only** (the held branch merged with
`--no-commit` into throwaway worktrees under `private/`, never committed):

| run | Safe / Pending / Unknown / Unsafe |
|---|---|
| base + held | 1,171 / 2 / 3,622 / 9 |
| **tip + held** | **1,171 / 2 / 3,623 / 8** |

**One move: `rd_kafka_mock_handle_Metadata` Unsafe → Unknown**
([moved-rows-merged.jsonl](evidence/moved-rows-merged.jsonl)). Its leak and
both nullable leaves are gone. Two `unresolved resource identity
requested_topics` leaves remain at `:1412`/`:1414` (the response loop's
`requested_topics->cnt` reads), beside the row's other refusals.

The held card's eleven nullable-leaf rows:

| row | nullable leaves | status |
|---|---|---|
| `rd_kafka_mock_handle_Metadata` | 2 → 0 | Unsafe → Unknown |
| `rd_kafka_metadata_topic_match` | 1 → 0 | Unknown |
| `rd_kafka_mock_handle_ShareAcknowledge` | 3 → 0 | Unknown |
| `rd_kafka_mock_handle_ShareFetch` | 6 → 3 (`forgotten_partitions` stays) | Unknown |
| the other seven (`rd_kafka_cgrp_calculate_subscribe_revoking_partitions`, `…_consumer_assignment_with_metadata`, `…_handle_SyncGroup_memberstate`, `…_owned_but_not_exist_partitions`, `rd_kafka_mock_cgrp_consumer_member_needs_revocation`, `rd_kafka_mock_handle_ConsumerGroupHeartbeat`, `balance`) | unchanged (8) | Unknown |

The seven keep their leaves because their nullability does not come from a
guarded release. Read in the local probe: a lazy constructor inside a loop
(`if (!result) result = ctor(); add(result, …)`), and a non-null constructor
result poisoned by an unbounded refusal and joined at a loop head
(`revoking`). They stay `Unknown` with no leak.

**`ut_testSameSubscriptions` stays provisional Unsafe** with its four leaks.
They are the real `RD_UT_ASSERT` / `verifyValidityAndBalance` failure returns
(`rdkafka_sticky_assignor.c:2999-3014`), which leave `subscription`
unreleased. No refused release is involved, so (A) does not apply. It is
unit-test code, not an upstream defect claim.

Across the merged trees, nullable leaves fall by 46 in total (40 rows,
including rows on the base whose nullability came from the same poisoned
frame).

**Against the prediction.** As predicted: the opt-in route had no status
moves; the merged tree had exactly the Metadata move, with the predicted row
set losing or keeping its nullable leaves; and `ut_testSameSubscriptions`
stayed Unsafe. Two things differ. Part A was narrowed after the prediction
(see §2). One unit fixture moves Unknown → Safe through an admitted frame
(see §3); PREDICTION said none could.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Prediction for the integrator:

* corpus (eight-home, twelve-home), librdkafka at `66910c10` without the
  constructor declarations: no status moves expected, with leaf-only changes
  of the kind above (fewer `unresolved resource identity` leaves in functions
  spelling `&NAME->F`, some newly exposed `interior-borrow-ended` /
  `reference-dropped` refusals);
* system and tail-pause slice: the same;
* possible, not expected: a provisional Unsafe withdrawn by (A), or a
  violation exposed behind a spared binding. Either would be an unpredicted
  move against the committed `measure-prediction` block (`none` on every
  route).

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 96d41921c --out
private/ngr/gate-1` through `private/toolchain/env.sh` (Clang 21, Lean 4.35),
head `69becff59` (later commits change only this report). Record:
[gate/gate-1.json](gate/gate-1.json). Logs are in the worktree's
`private/ngr/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` (24 packages) | 0 | 676 units, 721 summaries; 9,883 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` (all) | 0 | 483 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 6. The ledger entry and follow-ups

**NULLABLE-GUARDED-RELEASE-REPORTED-AS-LEAK** is registered only on the held
branch, so it is not retired here; the integrator reconciles it when the held
card lands. This card discharges:

* its Metadata instance (Unsafe → Unknown, leak-free, nullable leaves gone);
* the "hold the row Unknown when a release is refused" half of its DISCHARGE
  clause (rule 74c (A));
* the guarded-release correlation for bindings that reach the guard (rule
  74c (B) plus the existing rules 14l/10b), which clears 4 of the 11 rows'
  nullable leaves (12 of 20).

It does not discharge the other seven rows' nullable leaves, which come from
other shapes. They are registered below.

Registered in [integration-amendments.json](integration-amendments.json):

* **NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP**: the remaining nullable-leaf shapes
  (lazy construction in a loop, a poisoned non-null binding joined at a loop
  head).
* **SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN**: (A)'s precision cost (the
  descriptor fixture), and the census, wanted on the corpus, of how many
  provisional Unsafe rows rest on an intermediate-iterate claim.

Out of scope, met, already registered by the held card (not registered
again): member stores and releases through formals, out-parameter delivery
(OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY), `rd_kafka_topic_partition_list_add`
(RK-TOPIC-PARTITION-LIST-ADD-ELEMENT-POINTER).

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`: `AbstractState`, the join,
  `report_leaks`, `apply_resource_action`.
* `crates/qcore-c-gateway/src/legacy_c.rs`: the module list and the frame
  line.
* `crates/qcore-c-gateway/src/legacy_loop_generations.rs`: `rename`.
* The pinned test files listed in §3.
* `docs/agents/rules/6.md` and `10a.md` (the appended amendment notes).
* Generated files (`docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean`); rerun their generators on conflict.

The held branch touches none of these.
