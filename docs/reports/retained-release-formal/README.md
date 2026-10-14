# RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN

Card of wave 48, round 26. Base local `main` `f3e1d724c`, rebased onto
`2f0f65a60` (CALLBACK-MEMBER-DATA-ONLY-UNITS). Prediction
[PREDICTION.md](PREDICTION.md), committed before any implementation code.
Rule: gateway [12n](../../agents/gateway.md#constraint-12n), amended in place
(no new id). Lean `QCore.MemberSummaryOrder`
([AuditMemberSummaryOrder.lean](../../../proof/AuditMemberSummaryOrder.lean)).

## 1. The defect, reproduced

Over the fixture contracts `tests/fixtures/contracts/op_callbacks.qv`, with a
unit that stores `handler` (borrows every position, returns nothing) into
`op.op_cb`:

| body | base `f3e1d724c` | this card |
| --- | --- | --- |
| `void drop_formal(struct op *o) { c_op_destroy(o); }` | **Unknown**, one leaf `UnknownAlias { variable: "o" }` | **Safe** |
| a tracked local (`c_alloc`, null check, `c_op_destroy`) | Safe | Safe |
| a formal through arms only (`c_op_free_all(o)`) | Safe | Safe |
| a formal through a plain release (`c_free(o)`) | Safe | Safe |

## 2. Where `UnknownAlias` came from: accidental, not a guard

Instrumented once (a throwaway `eprintln!` in `Parser::member_candidates` and
the member branch of `callback_operands`, removed): `drop_formal` is built
three times. The two builds of its **W4 summary derivation** decide the member
over `{handler}` with `handler` classified `Missing` — its own summary was
not yet in the derivation environment — so the call refuses there
(`callback-uncontracted`) and `drop_formal` gets no summary. The **reported
run** classifies `handler` `Identity` and admits the call, but with no summary
the formal `o` is never bound to an entry resource, and
`legacy_transfer::apply_resource_action` reads the release of an unbound name
as `UnknownAlias`. A tracked local needs no summary, which is the whole
difference.

The order comes from `legacy_summary::derive_custody_summaries`: direct calls
plus `callback_route::callback_edges`, which before this card added edges only
for rule 13u's `Invoked` / `Table` clauses and only to candidates the body
names. A member invocation's candidates are stores elsewhere in the unit, so
no edge reached them.

Nothing in that path asks whether the closure candidate could alias the
formal. That question is answered, in every run, by the candidate's own
contract: a candidate that releases what it receives, releases a global,
keeps something or has no contract is not the identity and refuses the call.
The reported run already used that evidence for the local. So the
`UnknownAlias` was **narrowed, not justified**: the formal now reads exactly
as the local, on the same evidence.

## 3. The change

`crates/qcore-c-gateway/src/legacy_callback_route.rs`, `callback_edges`: a
body that calls a declaration with a `resource_callback_member_invoked`
clause also gets an edge to every in-unit function of that member's closed
set in its unit (`callback_member_closure::closure` over the unit's facts, the
set `member_candidates` decides over). An open set adds none, because the
call refuses in every run. This changes ordering only. No decision, class,
fold or verdict rule changes. Rule 13u's edges are unchanged.

Rule 12n gains the sentence and a **Do not** clause: do not derive a
member-invoking caller before its closed candidates.

## 4. Controls (`crates/qcore-c-gateway/tests/cg22_op_callbacks.rs`)

| test | asserts | base | card |
| --- | --- | --- | --- |
| `a_formal_released_through_a_decided_member_reads_as_a_tracked_local` | `drop_formal`, `drop_local` and `hand_on` (a caller handing its tracked object to `drop_formal`) are Safe | **fails** (`drop_formal` Unknown) | passes |
| `a_formal_released_through_arms_only_stays_safe` | the arms-only formal is Safe | passes | passes |
| `a_formal_whose_candidate_could_alias_it_stays_refused` | a candidate freeing the object it receives (the formal itself): `callback-effectful`; a candidate freeing a global the formal may alias: `callback-uncontracted` (it has no summary); an open member: `callback-member-open` | passes | passes |

Base run: `private/rr/fixture-base.log`; rebased tip:
`private/rr/fixture-rebased.log` (11 passed). The CG22 card's leftover
`eprintln!("PROBE …")` of the arms-only formal is replaced by the second test.

## 5. Lean: `QCore.MemberSummaryOrder`

`memberEdges`, `derivationEdges`, `classAt` (a candidate's class at
derivation time: its summary class once installed, `missing` before),
`localSafe`, `formalSafe` (the summary-time decision and the reported one).
Theorems: `a_closed_member_candidate_precedes_its_caller`,
`an_open_member_adds_no_edge`, `installed_classes_agree`,
`an_uninstalled_candidate_refuses_at_summary_time` (the defect),
`a_formal_reads_as_the_local_once_candidates_precede` (the narrowing),
`the_formal_is_never_safe_where_the_local_is_not` (never wider),
`a_candidate_that_could_alias_keeps_the_formal_unsafe` (the alias control),
and `fixture_vectors_hold` (the four fixture rows). Audits 318 → **319**.

## 6. Measurement

The scripts are `scripts/build.sh`, `measure.sh`, `measure-binary.sh` and
`measure-both.sh`, with the null property on every route (`NULL=1`). The
comparisons are `scripts/identity.py` and `scripts/graph_edges.py`. Every
heavy step ran under the shared heavy lock, the corpus at `--jobs 4`. Every
`run.json` `binary_sha256` matches the binary measured.

| pair | binaries | corpus | system | slice |
| --- | --- | --- | --- | --- |
| base `f3e1d724c` / card | `7a33fc99…` / `0adafcfd…` | `527a4cad…` = | `dc79064b…` = | `681999fc…` = |
| rebased: main `2f0f65a60` / card | `a28f8d80…` / `13deaa63…` | `527a4cad…` = | `dc79064b…` = | `681999fc…` = |

- **Verdict rows are byte-identical on every route, in both pairs.**
  - Corpus: 935 / 9 / 3,890.
  - System: 93 / 4 / 351.
  - Slice: 52 / 1 / 347 / 4 Pending.
- **The null property is byte-identical too** (rows and summary).
  - Corpus: 1,113 / 1,435 / 2,355 / 18 Unsafe.
  - System: 84 / 125 / 245.
  - Slice: 71 / 103 / 232.

Artifacts: [identity-f3e1d724c.json](artifacts/identity-f3e1d724c.json) and
[identity-2f0f65a60.json](artifacts/identity-2f0f65a60.json).

The typed-path parity streams differ only as follows:
- **Run-varying fields.** Clang AST addresses and the frontend digest change
  between any two runs. `graph_edges.py` blanks them.
- **The derivation call graph.** The only rows that differ after that are
  three `summary-premises-graph` rows in the corpus. They gain exactly
  **22 edges in 3 units** and lose none:
  - 19 edges to `rd_kafka_share_fetch_reply_op_cb` and `rd_kafka_poll_cb` in
    the 249-function unit that defines `rd_kafka_thread_main` and
    `rd_kafka_poll`;
  - `rd_kafka_mock_cluster_io_poll -> rd_kafka_mock_cluster_op_serve` (187
    functions);
  - `rd_kafka_commit_queue ->` `rd_kafka_offset_reset_op_cb` and
    `_validate_op_cb` (64 functions).
- **System and slice.** No graph row moves.

The edge sets are the same in both pairs
([corpus](artifacts/graph-edges-corpus-2f0f65a60.json); base pair
[corpus](artifacts/graph-edges-corpus-f3e1d724c.json); system and slice
files beside them).

No verdict moves because every corpus call these edges order still refuses.
The ledger entry says every member invocation refuses before the release, and
the `OP-CALLBACK-CANDIDATE-CONTRACTS` candidates have no contract in force.
So there is no Safe gain to read by hand and no Safe → non-Safe correction.
The prediction held for the verdicts. It did not predict the graph rows, but
they are the mechanism it named.

Wall time (corpus, inside the lock): 537.6 s base / 544.5 s card, then
532.2 s main / 519.5 s card after the rebase. That is noise, measured once.

## 7. Ledger

Running `scripts/update_ledger.py` (idempotent) **retires**
`RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN`. Its DISCHARGE holds:
- the rule was identified (the W4 derivation order, `callback_edges`);
- it was narrowed and named in rule 12n;
- there is a control on a formal;
- it was measured.

Nothing new is registered. `dimensions.named_followups` 770 → **769**.

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/).
