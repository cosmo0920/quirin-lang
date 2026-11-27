# HANDOFF-DIRECT-CALL-READING: a direct call of a hand-off declaration

Card HANDOFF-DIRECT-CALL-READING (wave 48, card-high). Gateway rule **12m**
amended in place; no new rule id. Contract specification **C04.19.5**
amended; no grammar change, no new surface syntax (so no pause). Lean
`QCore.HandOffDirect`. Base: local `main` `b2fbde2c5`, rebased onto
**`4c0803f1f`** (LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER; section 7 measures
main against the rebased tip). The census and the frozen
[prediction](PREDICTION.md) were committed (`e05676fd5`, `acc0dc2e7` after
the rebase) before any implementation code.

## 1. Defect and reading

Rule 12m read a hand-off (`quant live #P: #owner |-> #callback when #result
OP @C;`) only as a callback candidate; a direct call refused
(`callback-handoff-direct`), because the caller holds the object on one class
of results and not on the other. Four corpus rows call `rd_kafka_poll_cb`
(`!= PASS`, parameter 2) directly. The bodies are byte-identical at
`66910c10` (the corpus copy) and `046accf6`; the per-class readings are in
the [prediction](PREDICTION.md):

| row | caller's branch | reading |
| --- | --- | --- |
| `rd_kafka_consume0` (`rdkafka.c:5045`) | `res = …; if (res == PASS) break;` | agrees: `PASS` keeps the op, every other result returns or rebinds `rko` from `rd_kafka_q_pop` before naming it |
| `rd_kafka_consumer_close0` (`:5292`) | `res = …; if (res == PASS) rd_kafka_op_destroy(rko);` | agrees |
| `rd_kafka_q_serve_rkmessages` (`rdkafka_queue.c:803`) | `KEEP \|\| HANDLED` continue, `YIELD` break, the rest names `rko` | disagrees: values outside the enumeration, which `!= PASS` gives the callback, are kept |
| `rd_kafka_background_queue_serve` (`rdkafka_background.c:90`) | `HANDLED \|\| YIELD` return, the rest destroys `rko` | disagrees: `KEEP` is destroyed |

## 2. Mechanism

* **The split** (`crates/qcore-c-gateway/src/handoff_direct.rs`). A
  hand-off is two exit classes, `eq K` and `ne K`, each with the owner after
  the call. The caller's state forks at the call, **taken** (the actual
  `resource_consumed(P, nonnull)`) where the guard holds and **kept** (the
  declared `resource_owned(P, nonnull)`) where it fails, joined only at the
  caller's own test of the result, read as rule 11r reads a return-code
  disposition: the call tested where it stands, or stored by `NAME = f(…);`
  and tested by the next statement. `read_test` reads an enumerator as its
  value in the callee's own result type (rule 13l's `resolve` in the
  analysing unit), accepts only a non-negative constant (so the comparison
  with an unsigned or enumeration result is the integer one), and requires
  the constant to be one Clang reads the body comparing the result with.
  Each edge takes the owner every class it reaches agrees on (`edge_owner`,
  `split`); an edge reaching both refuses.
* **The calling body's reading** (`handoff_branch::direct_admission`): the
  caller branch check applied in the calling body, over the analysing unit's
  Clang AST: every direct site of the callee read, the handed actual a
  pointer variable the walk follows, no class the guard admits naming the
  object again before a rebinding or a noreturn call (rule 12m's `forward`),
  the result compared with the guard's own constant, and the result not
  stored through a conversion (`Site::result_converted`, new, serde-default).
  A disagreeing body refuses `callback-handoff-unbranched`, never a finding;
  every reading not made refuses `callback-handoff-direct`.
* **Route hooks** (`legacy_handoff_direct.rs`): the stored-return rewrite
  (rule 11r's, shared through `legacy_relational_post::stored_returns_with`),
  the `if` plan after rule 11r's (a branch that decides nothing, each edge the
  call classified under its owner through `retaining_call`, the callback
  operands' hand-off refusal skipped only on an edge), and the refusal in
  `callback_operands`.
* **Line order** (found by measurement, section 4): the hand-off rewrite
  keeps the stream's lines non-decreasing (`keep_line_order`), because
  `typed_place::occurrence` declines every statement of a stream whose lines
  are not sorted.

Tests: `crates/qcore-c-gateway/tests/handoff_direct_call.rs` (agreeing
callers admitted and `Safe`: the call tested in place, the stored result
tested next, `!=`, the `== KEEP` form; the kept edge still owes the object
(a dropped op is the caller's own leak, not a hand-off refusal); the wrong
class refused `callback-handoff-unbranched` and never `Unsafe`; an ignored
result refused; a stored result tested after another statement, in a
declaration's initialiser, through an edge reaching both classes, or through
a narrowing store refused `callback-handoff-direct`), the unit tests of
`handoff_direct` (`direct_vectors_hold` bound to Lean, the split against
`owner_after` on every value, the enumerator reader) and
`legacy_relational_post::a_line_ordered_rewrite_keeps_the_stream_sorted`.
The existing `return_code_handoff` direct-call test still refuses
`callback-handoff-direct` (its result is returned, not tested).

## 3. Lean

`proof/QCore/HandOffDirect.lean`, audited by `proof/AuditHandOffDirect.lean`,
8 theorems, standard axioms only: `a_class_owner_is_the_guard`,
`every_value_has_a_class`, **`the_split_state_is_sound_per_class`** (through
`QCore.RelationalPost.the_edge_disposition_is_the_returned_class`),
**`a_caller_that_disagrees_never_decides`**,
`an_unbranched_direct_call_never_decides`,
`a_decided_call_never_touches_a_taken_object`,
`a_decided_split_is_the_guard_on_every_value`, `direct_vectors_hold`
(`DIRECT_VECTORS`). Lean 4.35.0 (the pin). Lean audits 327 → 328 on
`b2fbde2c5`; **328 → 329** on `4c0803f1f` (which added
`QCore.NullMemberPremise`).

## 4. Measurement

BEFORE `before` (`7073bccf…`, built at `bae40c795`, the base code) and AFTER
`a2` (`25c57074…`, built at `da54f2c65`), each `run.json` naming its binary,
every route with the null property ([summary](artifacts/summary.json),
scripts [`before.sh`](scripts/before.sh),
[`after-chain.sh`](scripts/after-chain.sh)):

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 938 / 9 / 3,887 | **938 / 9 / 3,887** | `fc645e9b…` → `1e76a508…` | 706.4 → 702.5 |
| system | 93 / 4 / 351 | the same, byte-identical | `1a7e4609…` | 192.1 → 195.6 |
| slice | 52 / 1 / 347 / 4 Pending | the same, byte-identical | `264fc46c…` | 152.5 → 153.9 |
| null | 1,113 / 1,435 / 2,355 / 18; 84 / 125 / 245; 71 / 103 / 232 | rows and tallies identical on all three | | |

The BEFORE corpus report equals the one HANDOFF-CANDIDATE-OWN-CALLBACKS
published for its tip (`fc645e9b…`). Wall times measured once each on a
shared host; not a performance claim.

**Moving rows** ([compare](artifacts/compare-corpus.json)): exactly the four,
all Unknown before and after, one leaf each, no Safe → non-Safe change, no
leak, violation or Unsafe added:

| row | leaf before | leaf after |
| --- | --- | --- |
| `rd_kafka_consume0` | `callback-handoff-direct` | `callback-member-open`: `rd_kafka_poll_cb`'s member invocation of `rd_kafka_op_s.rko_u.offset_commit.cb` (the union's type-keyed opaque writes) |
| `rd_kafka_consumer_close0` | `callback-handoff-direct` | the same |
| `rd_kafka_q_serve_rkmessages` | `callback-handoff-direct` | `callback-handoff-unbranched` (the rest class names the op) |
| `rd_kafka_background_queue_serve` | `callback-handoff-direct` | `callback-handoff-unbranched` (`KEEP` falls to the destroy) |

**Safe gains:** none, so no clause or premise list to read. The dependency
counts and the Safe roll-up are identical (431 declarations consulted; 384
Safe rows on a declared assumption).

**The calling bodies' readings** ([census](artifacts/handoff-census.json),
`examples/handoff_census.rs`, [`census.sh`](scripts/census.sh), over the 94
librdkafka units): `rd_kafka_consume0` compares `PASS` and `YIELD` (`PASS`
touches, kept; `YIELD` and the rest do not) and `rd_kafka_consumer_close0`
compares `PASS` (touches, kept; the rest does not): **admitted**.
`rd_kafka_q_serve_rkmessages` compares `HANDLED`, `KEEP`, `YIELD` (none
touches) and the rest touches; `rd_kafka_background_queue_serve` compares
`HANDLED`, `YIELD` and the rest touches: **disagree**. No result is stored
through a conversion.

**Next blocker per row** ([grouping](artifacts/blockers-corpus.json)):

* `rd_kafka_consume0`: `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` at the call;
  then five uncontracted callees (`rd_kafka_app_poll_start`,
  `rd_kafka_app_polled`, `rd_kafka_fetch_op_app_prepare`,
  `rd_kafka_message_get`, `rd_kafka_set_last_error`), three unresolved
  identities and the `rd_kafka_q_pop` loop condition.
* `rd_kafka_consumer_close0`: the same union blocker at the call; then ten
  unresolved identities, five indirect accesses, two reference drops
  (`reference-drop-unheld`), two `retention-undecided`, uncontracted
  `rd_kafka_consumer_close_q`.
* `rd_kafka_q_serve_rkmessages`, `rd_kafka_background_queue_serve`: the
  disagreement itself, which is the bodies' real behaviour against the
  declared guard (no reading change can admit it); behind it 52 and 4 other
  leaves (unresolved identities, indirect access, uncontracted
  `rd_kafka_q_deq0`, `rd_kafka_call_background_event_cb` and others).

**Prediction against outcome.** As predicted: no status move, the four
leaves trade exactly as stated (the two agreeing rows to
`callback-member-open` at `rko_u.offset_commit.cb`, the two others to
`callback-handoff-unbranched`), system, slice and null unchanged, one new
Lean audit. **Not as predicted:** the first AFTER binary (`a1`, `2b5ad9d1…`,
built at `9b24a5cb5`) also moved a fifth row, `rd_kafka_DescribeGroups_resp_cb`
in the same unit (`rdkafka.c`), which lost 9 `unresolved resource identity
__tmpstr` leaves with no status move. A second run of the BEFORE binary
(`before2`) was identical to BEFORE (no row moves), so the move was the
card's: the rewrite put the `if`'s line ahead of the moved call's, and
`typed_place::occurrence` declines every statement of a stream whose lines
are not sorted. `a2` keeps the hand-off rewrite's lines in order and moves
only the four rows. Rule 11r's own rewrite has the same property and is
unchanged (`RELATIONAL-REWRITE-LINE-ORDER`).

## 5. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), re-applied on main's
ledger after the rebase ([`resolve_records.py`](scripts/resolve_records.py)):
on `4c0803f1f` 785 → **787** (equal to `dimensions.named_followups`; 786 →
788 on `b2fbde2c5`).

* **Retired:** `HANDOFF-DIRECT-CALL-READING` (its DISCHARGE holds: the split
  at the caller's own branch, the caller branch check in the calling body,
  Lean parity, measured on the four rows).
* **Amended:** `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` (now also the named leaf
  of `rd_kafka_consume0` and `rd_kafka_consumer_close0`; 4 rows).
* **Registered:** `HANDOFF-DIRECT-LATER-BRANCH` (a stored result tested
  later than the next statement or initialised in a declaration; 0 corpus
  rows), `HANDOFF-INVOKER-CONVERTED-RESULT` (rule 12m's invoker check does
  not yet refuse a converted store; 0 rows), `RELATIONAL-REWRITE-LINE-ORDER`
  (section 4; size not measured).

Not changed: the comment above `rd_kafka_poll_cb` in the librdkafka
`client.qv` still says a direct call refuses; it is provider contract text
outside this card's files and now describes the refusal of an unread
direct call only.

## 6. Gate

Gate **g2** ([gate.json](gate.json), exits in [gate/](gate/), logs
`private/hdcr/gate-g2/`): the full standing gate, with the provider gate run
as well (not required: `providers/` and `qcore-schemas` are unchanged; run so
the regenerated matrix carries a measured provider count). Every code step
ran at **`bacd2d082`** on `4c0803f1f`; later commits change records only. One
exclusive flock on the shared heavy lock covered the whole gate
(22:33:21Z–22:49:57Z, [`gate-run.sh`](scripts/gate-run.sh),
[`gate-whole.sh`](scripts/gate-whole.sh), [`gate.sh`](scripts/gate.sh),
[`finish-gate.sh`](scripts/finish-gate.sh)). Lean 4.35.0.

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets`, submodule, `fmt --check`, `clippy --locked -D warnings` | 0, 0, 0, 0 | |
| `run_test_binaries.py` | 0 | 503 units, 516 summaries, **8,144 passed, 0 failed, 15 ignored** (`private/hdcr/tests-g2/`), no rerun |
| both reevaluation probes | 0, 0 | |
| `check_lean_audits.py` | 0 | **329** audits passed |
| provider gate | 0 each | `package.py` build, verify `qcore.libevent` and `qcore.librdkafka`; `cargo check --locked`; `contracts.py check`; `cargo test` 133 passed / 0 failed; providers Python 25 OK; fmt; clippy |
| `cargo clean --profile dev` | 0 | |
| Python suite before the matrix regeneration (08) | **1** | fails only `test_the_derived_facts_are_recomputed_rather_than_carried` (the committed matrix's Lean count, 328 → 329), as for every card adding an audit |
| docs index check (09) | 0 | |
| `build_matrix.py . --gate gate.json` (09a), Python suite (09b, 374 tests), docs index (10b, 10c) | 0, 0, 0, 0 | |

The bold rule-id duplicate check is empty. Clang:
[`gate/clang-version.txt`](gate/clang-version.txt).

**Discarded attempt:** gate **g1** at `7f3410092` (logs
`private/hdcr/gate-g1/`) failed `clippy` (exit 101,
`clippy::type_complexity` on `handoff_direct::read_test`'s `compared`
parameter); `bacd2d082` names the type (`Compared`) and changes nothing
else. Its other steps were 0 apart from the accounted Python step (8,144
passed, 329 Lean audits).

## 7. After the rebase (main `4c0803f1f`)

LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER landed while the card waited (the
premise-shape check, bounds v1 beside `nonnull_member_*`, null-row premise
fields, librdkafka contracts and pins, rules 14x/14z/13h, C04.18), disjoint
from this card's code. The code commits rebased without a conflict; the
records conflicted and were re-applied on main's
([`resolve_records.py`](scripts/resolve_records.py)). After the rebase,
`cargo check --workspace --all-targets` 0 and no conflict marker.

**Combined arbiter** ([`build-main.sh`](scripts/build-main.sh),
[`rebased-chain.sh`](scripts/rebased-chain.sh)): main's binary `m4c`
(`b272f50f…`, built from an archive export of `4c0803f1f`, reading that
export's contracts, its libevent submodule cloned at the pinned commit)
against the rebased tip's `b1` (`c5d986a4…`, built at `4c3dd0eba`, the code
of `bacd2d082`), each `run.json` naming its binary, every route with the
null property:

| route | main `4c0803f1f` | tip | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 938 / 9 / 3,887 | **938 / 9 / 3,887** | `fc645e9b…` → `1e76a508…` | 679.4 → 708.9 |
| system | 93 / 4 / 351 | the same, byte-identical | `27ad0099…` | 190.6 → 195.1 |
| slice | 52 / 1 / 347 / 4 Pending | the same, byte-identical | `3e070be7…` | 153.9 → 153.0 |
| null | 1,113 / 1,435 / 2,355 / 18; 84 / 125 / 245; 71 / 103 / 232 | rows and tallies identical ([compare](artifacts/null-compare-rebased.txt)) | | |

The corpus moves exactly as in section 4 (the same four rows, the same
leaves, [compare](artifacts/compare-corpus-rebased.json)); the tip's corpus
report is byte-identical to `a2`'s. The system and slice digests differ from
section 4's only by main's contract-source digests. **No other movement.**

## Files

* `crates/qcore-c-gateway/src/{handoff_direct,legacy_handoff_direct,handoff_branch,legacy_relational_post,relational_post,legacy_callback_route,legacy_c,lib}.rs`,
  `tests/handoff_direct_call.rs`,
  `tests/fixtures/contracts/handoff_direct.qv`;
  `crates/qcore-audit/examples/handoff_census.rs`.
* Lean: `proof/QCore/HandOffDirect.lean`, `proof/AuditHandOffDirect.lean`,
  `proof/QCore.lean`.
* Records: `docs/agents/gateway.md` (12m), `docs/agents/README.md` (12m row),
  the contract specification (C04.19.5), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
