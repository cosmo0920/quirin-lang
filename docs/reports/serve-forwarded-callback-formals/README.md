# SERVE-FORWARDED-CALLBACK-FORMALS: forwarded callback formals

Card SERVE-FORWARDED-CALLBACK-FORMALS (wave 48, round 27), gateway rule
**[15w](../../agents/gateway.md#constraint-15w)** (rules 13u and 13y amended in
place), Lean `QCore.CallbackForward`. Base: local `main` at `2afce9973`, rebased
onto `89907068d` (OP-CALLBACK-CANDIDATE-CONTRACTS, section 9). The
[prediction](PREDICTION.md) was committed before any implementation code.

## 1. Authority and decision

The ledger entry (registered by CG22-OP-CALLBACKS-AND-SERVED-QUEUES) asked for
"a rule that reads a callback formal handed to the same clause position of a
callee as the caller's own declaration's clause (as rule 12s forwards
dispositions), with Lean, measured on the four forwarders". The user decided on
2026-10-14 that the CG22 card would carry no such rule; this card does.

## 2. The rule

A mention of a callback formal `F` is a **forward** when it is the whole
actual, at position `p`, of a direct call `G(…)`. It is **exact** when `G`'s
declaration in force names `p` in exactly one clause,
`resource_callback_invoked(p, A'…)`, keeps it `resource_borrowed`, never
retains it, registers it in no constructed result, and `A'…` read through the
call's actuals is the caller's `A…` position by position (a parameter word
against the caller's own formal, unchanged; `_` against `_`; `supplied_owned`
against `supplied_owned`).

* **Body check (13y's forward case).** `resource_callback_invoked(F, A…)`
  stands in a unit that defines its body when rule 13y's direct check holds or
  `F` is never rebound and every mention of `F` is an exact forward. A callee
  the unit defines is in force only once its own clauses stand: the standing
  set is the least fixed point (`settle`), so a recursive forward is never its
  own support and nothing stands on a withdrawn callee. A forwarded clause
  records no body evidence; it rests on the callee's declaration and stays a
  declared assumption (rule 13i). `13y` is recorded only where the direct
  check holds alone.
* **The forwarding call (13u's forward case).** At a call whose actual at the
  callee's stated position is the calling function's own formal `F`, with the
  calling function's declaration in force (so its body validated) and every
  clause of it naming `F` an invocation corresponding to the callee's, the
  clause is that declared invocation, decided at the calling function's own
  callers; the call neither resolves `F` nor carries its candidates'
  requirements.
* **Refusal unchanged** for a reassignment, an address-of, a store, a cast, a
  parenthesised use, an operand, a local invocation beside a forward (the
  conservative choice under "handle as safe as possible": a mixed body
  refuses even where each mention alone would validate), another position,
  non-corresponding actuals, and a callee with no declaration in force there.

Code: `crates/qcore-c-gateway/src/legacy_callback_forward.rs` (new module:
`forwards`, `stated_invocation`, `corresponds`, `forward_validates`,
`Parser::forwarded_invocation`); hooks in `legacy_callback_route.rs` (`settle`
reorganised into the fixed point, identical on every environment without a
forward; one call in `callback_operands`'s invocation arm). Shared items:
`callback_route::{parameter_names, rebound}` become `pub(super)`.

## 3. The four forwarders, read at both revisions

`lib/librdkafka-2.15.0/src/rdkafka_queue.c` is byte-identical at `66910c10` and
`046accf6` (`a1d55367…`). Each forwarder (`rd_kafka_q_pop_serve` 507-515,
`_pop_serve_maybe_consume` 522-533, `rd_kafka_q_serve` 652-660,
`_serve_maybe_consume` 667-677) is one `return` of `rd_kafka_q_pop_serve0` /
`rd_kafka_q_serve0`, handing `cb_type`, `callback` and `opaque` on as the
fourth, fifth and sixth actuals, unchanged; its declaration and its callee's
state the same `resource_callback_invoked(callback, _, _, supplied_owned,
cb_type, opaque)` at the same positions. Each is an exact forward.

Both callees are defined in the same unit and withdrawn there by rule 13y
independently of the forwarders: `rd_kafka_q_pop_serve0` hands `callback` to
`rd_kafka_op_handle` (`:450-452`) and to itself (`:498-499`, the forward-queue
recursion); `rd_kafka_q_serve0` to `rd_kafka_op_handle` (`:627-628`) and to
itself (`:585-586`). `rd_kafka_op_handle` has no declaration, and the least
fixed point never reads the recursion as its own support. So no forwarder's
callee is in force, and the four stay withdrawn, as predicted.

## 4. Measurement

BEFORE `bin/before` (`3eca2093…`, built from `2afce9973`); AFTER `bin/a2`
(`087147af…`, built from `83041c565`, the final code); every route with
`QCORE_NULL_PROPERTY`; each corpus `run.json` names its binary
([summary](artifacts/summary.json), [corpus comparison](artifacts/compare-corpus.json)).
An intermediate `bin/a1` (`b1c42efa…`, before the formatting and clippy fix)
was built and never measured.

| route | BEFORE | AFTER | report sha256 | wall (s, AFTER) |
| --- | --- | --- | --- | ---: |
| corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `ec4ccfeb…`, byte-identical | 522.2 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `968f193c…`, byte-identical | 84.5 |
| slice | 52 / 1 / 347, 4 Pending | the same | `b8823774…`, byte-identical | 39.2 |
| null, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | rows byte-identical | `a1067ba0…` | |
| null, system / slice | 84 / 125 / 245; 71 / 103 / 232 | rows byte-identical | `f42ef95e…`, `c6a6f683…` | |

No row moves on any route: no Safe gain to read, no Safe → non-Safe, no leaf
added or lost. The contract sidecars are byte-identical as well (the forwarders
record no evidence, and nothing new is validated or withdrawn). The corpus
typed-path parity log (687,062 records) differs only in Clang AST node
addresses, which vary per process; normalised, the two are the same multiset.

| predicted | measured |
| --- | --- |
| corpus, system and slice unchanged in status, every verdict row byte-identical | byte-identical reports on all three routes |
| the forwarders keep `callback-withdrawn` on their withdrawn callees; `rd_kafka_q_pop` keeps it on `rd_kafka_q_pop_serve` | unchanged leaves (5 rows in `rdkafka_queue.c` carry the leaf, 3 alone; the two `_maybe_consume` rows refuse first on the indirect access `rkq->rkq_rk->rk_type`) |
| null property byte-identical | identical on all three routes |
| movement 0 (bound +4) | 0 |

**What would move once the parallel contracts land.** Nothing through rule 15w
in `rdkafka_queue.c`: `OP-CALLBACK-CANDIDATE-CONTRACTS` and
`OP-APP-RK-CALLBACKS-VIA-POLL-CB` declare op-callback candidates and
application callbacks, not `rd_kafka_op_handle`, and leave the recursion as it
is, so both callees stay withdrawn. With the callees standing as well, the
forwarders' rows would still need the member clauses they repeat
(`rd_kafka_op_s.rko_serve`, `rd_kafka_op_s.rko_op_cb`) decided over the unit's
closed sets with contracted candidates (the two parallel cards); the
`_maybe_consume` rows keep their access refusal and `rd_kafka_q_pop` passes a
null callback. At most `rd_kafka_q_serve` and `rd_kafka_q_pop_serve` could
then move. The residual is registered.

## 5. Controls

`crates/qcore-c-gateway/tests/serve_forwarded_callback_formals.rs`, over the
synthetic `fw_*` declarations of `tests/fixtures/contracts/forwarded_callbacks.qv`
(trusted test assumptions): an exact forward is accepted (its caller decides
over its own candidate and is `Safe`, the forwarder's own row is `Safe`); the
forwarded clause is still decided at the outer callers (an effectful candidate
refuses `callback-effectful`, an undeclared function's formal stays
`callback-undecided`); a formal reassigned before the forward, a forward to a
different position, swapped actuals, a forward to an uncontracted callee, and a
formal both forwarded and invoked locally each keep `callback-withdrawn`; a
chain of two exact forwards stands while a forward to a withdrawn callee
refuses; a recursive forward alone refuses. With rule 15w disabled (both
hooks forced off, a control run kept in `private/sf/t3-control.log`) the three
positive tests fail and the six refusal tests pass unchanged. Unit tests in
the module: the forward reader, the correspondence, and `forward_vectors_hold`
(bound to Lean's `forwardVectors`).

## 6. Lean

`proof/QCore/CallbackForward.lean`, audited by `proof/AuditCallbackForward.lean`
(9 theorems; axioms within `propext`, `Classical.choice`, `Quot.sound`; no
placeholder, no project axiom): **`forwarded_invocations_meet_the_caller_clause`**
(every invocation meeting the callee's clause, read through the call's actuals,
meets the caller's), **`an_exact_forward_stands_iff_the_callee_clause_is_in_force`**,
`a_forward_to_a_callee_not_in_force_refuses`,
`a_forward_that_does_not_correspond_refuses`, `a_changed_formal_refuses`,
`any_other_mention_refuses`, `no_mention_refuses`,
**`a_recursive_forward_alone_never_stands`** (the least fixed point over
rounds), `forward_vectors_hold`. Lean audits 319 → **320**.

## 7. Ledger

`scripts/update_ledger.py` (idempotent): **`SERVE-FORWARDED-CALLBACK-FORMALS`
retired** (its DISCHARGE holds: the rule, its Lean, measured on the four
forwarders, each read and attributed above); **`SERVE0-CALLBACK-HAND-ON-WITHDRAWN`
registered** (the six serve declarations withdrawn in one unit, 5 rows carrying
the leaf, 3 alone; DISCHARGE: a declaration of `rd_kafka_op_handle` and a user
decision on a recursive exact forward, whose coinductive reading is sound for
this safety clause but is a stronger claim than the least fixed point).
Ledger 771 → 771 (774 → 774 after the rebase).

## 8. Gate

Standing gate at `4c6988cb3` (`scripts/gate-whole.sh g1`: `gate.sh` steps
00-09 under one exclusive flock on the shared heavy lock, then
`finish-gate.sh`; logs `private/sf/gate-g1/`, exits copied to [gate/](gate/),
[gate.json](gate.json)): cargo check 0; submodule 0; fmt 0; clippy `--locked`
0; `run_test_binaries.py` 0, **494 units, 507 summaries, 8,036 passed, 0
failed, 14 ignored**, no rerun (`private/sf/tests-g1/`); both reevaluation
probes 0; `check_lean_audits.py` 0, **320 Lean audits**; the Python suite 1
before the matrix regeneration (its one failure is the committed matrix's
`lean_audits` 319 against the produced 320, by design), then
`build_matrix.py . --gate gate.json` 0 and the suite rerun 0 (**365 OK**);
docs index 0 and its check 0. No provider gate: nothing under `providers/` or
`crates/qcore-schemas/` changed.

## 9. Rebase onto `89907068d` (OP-CALLBACK-CANDIDATE-CONTRACTS)

That card put nineteen op-callback candidate declarations in force (librdkafka
`client.qv`), amended rules 12n and 13u and moved the ledger to 774. It shares
no code with this card. The record conflicts (STATUS, the increment record,
the matrix) were resolved by taking main's text and reapplying
`scripts/update_ledger.py` and `scripts/edit_records.py`, whose anchors now
point at main's rows. Rule 13u's two amendments merged without conflict. Rule
15w reads callee clauses in force, so both checks were rerun on the rebased tip
`62f77988c`.

**Targeted check** (`scripts/targeted.sh t1` under one exclusive flock on the
shared heavy lock; logs in `private/sf/targeted-t1/`, exits in
[gate-rebased/](gate-rebased/)):

* `cargo check --workspace --all-targets`: 0.
* `cargo test -p qcore-c-gateway --no-fail-fast`: 0 (167 summaries, 2,896
  passed, 0 failed, 8 ignored).
* The compiler's contract tests, all 0: `contract_loader` (10 passed),
  `qcore_contract_corpus` (2 passed), `contract_view_census` (4 passed).
* `check_lean_audits.py`: 0 (**320 audits**).
* `build_matrix.py . --gate gate.json`: 0; then the Python suite: 0 (**365
  OK**).
* The docs index and its `--check`: 0 and 0.

The full standing gate in section 8 ran before the rebase.

**Combined arbiter measurement** ([summary](artifacts/summary-rebased.json),
[corpus comparison](artifacts/compare-rebased-corpus.json)). Two binaries,
every route run with `QCORE_NULL_PROPERTY`, and each corpus `run.json` names
its binary:

* main's `m2` (`560d9844…`), built from `89907068d` and run from a detached
  checkout of it, so it read main's contracts;
* the rebased tip's `b2` (`3f495627…`), built from `62f77988c`.

| route | main `89907068d` | rebased tip | report sha256 |
| --- | --- | --- | --- |
| corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `0a38c3b3…`, byte-identical |
| system | 93 / 4 / 351 | the same | `ed0a3380…`, byte-identical |
| slice | 52 / 1 / 347, 4 Pending | the same | `20769011…`, byte-identical |
| null rows (corpus / system / slice) | | identical | `e7bf22f8…`, `89f9d9d2…`, `9150b84a…` |

**No row moves.** None of the nineteen new declarations carries a callback
clause, so rule 15w finds no new forward and no new callee in force. The serve
family is unchanged: its callees `rd_kafka_q_serve0` and
`rd_kafka_q_pop_serve0` still hand `callback` to the undeclared
`rd_kafka_op_handle` and to themselves. The parity log again differs only in
Clang AST node addresses.
