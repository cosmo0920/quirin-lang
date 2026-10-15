# SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a hand-off by return code

Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (wave 48, card-high). Gateway rule
**12m** amended in place, **14e** amended in place (the `#absent`
migration), rule **13l** added (the surface: `#` keywords, `@` names,
aliases, pinning). Contract specification **C04.19.5** added, C04.19.1,
C04.19.4 and C04.18 amended; grammar G05 and G05d. Lean
`QCore.ReturnCodeHandOff`. Base: local `main` `10eb7692d`. The census and the
frozen [prediction](PREDICTION.md) were committed (`9e3cac1e1`) before any
implementation code.

## 1. User decisions (2026-10-15, verbatim in C04.19.5)

`#` marks binders (`#result`, `#rko`) and value keywords (`#absent`,
`#owner`, `#callback`); the clause-kind word after `quant` stays bare;
`@NAME` is a C name Clang resolves (an enumerator of the guarded value's own
enum type, pinned at both revisions; a typo, a wrong-type constant or an
unresolvable name rejects the module); module-local aliases
(`alias #PASS = @RD_KAFKA_OP_RES_PASS;`); the clause
`quant live #rko: #owner |-> #callback when #result != @RD_KAFKA_OP_RES_PASS;`;
`|-> absent` becomes `|-> #absent` everywhere, a bare `absent` rejecting;
a hand-off is admitted only where the caller's body branches on the same
result with the same constant and treats ownership consistently, checked
against the bodies of `rd_kafka_op_handle` and the serve loops. Nothing
beyond this surface was needed, so the card did not pause.

## 2. Census

Per-candidate return codes and the caller branch check by reading are in
the [prediction](PREDICTION.md) (sections 2 and 3). The outcome:

| Candidate | Returns | Clause | Caller branch check (tool, `examples/handoff_census.rs`) | Outcome |
| --- | --- | --- | --- | --- |
| `rd_kafka_poll_cb` | `PASS` (7 paths, op untouched); `HANDLED`, `YIELD`, `rd_kafka_consume_cb`'s and `rd_kafka_op_call`'s results (op taken) | `!= #PASS` (alias) | admitted at `rd_kafka_q_serve0`, `_serve`, `_serve_maybe_consume`; refused at `rd_kafka_q_pop_serve0`, `_pop_serve`, `_pop_serve_maybe_consume` (a value outside HANDLED/KEEP/YIELD returns the op) and at `rko_serve` (called through a local copy) | **declared**; reads uncontracted as a candidate (its own twelve member invocations); its four direct calls refuse `callback-handoff-direct` |
| `rd_kafka_txn_op_send_offsets_to_transaction` | `HANDLED` (`__DESTROY`, errors); `KEEP` (the request holds the op) | `== @RD_KAFKA_OP_RES_KEEP` | admitted at both serve loops' `rko_op_cb` and at `rd_kafka_op_destroy` | **declared** |
| `rd_kafka_admin_worker` | `KEEP` (five waits); `HANDLED` | — | — | **named, refused**: its HANDLED arm destroys `rko_u.admin_request.eonce` and moves `args`/`fanout_parent` out (shape (b)) |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` | `KEEP`; `HANDLED` | — | — | **named, refused**: its HANDLED arm moves `rko_u.leaders.partitions` out and clears the eonce (shape (b)) |

The tool's answers match the reading of section 3 of the prediction row by
row ([census answers](artifacts/handoff-census.json), log
`private/sorch/handoff-census-c3.log`). Two readings found while
implementing: a pointer to a function typedef (`rd_kafka_q_serve_cb_t *`)
has no desugared spelling in Clang's JSON, so the unit's typedef table
decides a function-pointer formal (`c2` refused every serve as "not read as
a function pointer"); and the serve forwarders' sites are recorded because
they hand the callback formal on, whatever their result type.

## 3. Mechanism

* **Surface (rule 13l).** `@` is one lexer symbol; `alias #NAME = @C;`
  is a module-level declaration of a contract module
  (`SourceProgram::aliases`; the compiler driver checks a staged file's own
  mode), validated by `contract_handoff::module_aliases`; the hand-off is
  read by `contract_handoff::quant_live_clause` (a `#P:` binder other than
  `#result`) and stored as `RetainingContract::handoff`
  (`qcore_schemas::handoff`). A declaration with a `#` form rejects a
  parameter named like a reserved keyword; 13 parameters in the tree carry
  such a name, none beside a `#` form, so nothing on main rejects (the
  global reading of "a C parameter whose name matches a reserved keyword
  rejects" would reject the serve family's `callback` and
  `msgpack_packer_init`; the card took the scoped reading, the binder
  ambiguity being the reason the rule exists). An alias is part of the
  digest of every declaration spelling it.
* **Resolution.** Per unit through Clang's AST (`MemberFacts::result_types`,
  `typed_constants`, `handoff_branch::resolve`); on the pinned route, after
  the pre-pass, `member_prepass::check_handoff_constants` rejects the run's
  contracts for a constant that does not resolve in its candidate's result
  type (the loader runs no Clang, so this is where the module rejection is
  made). **Pinning**: `quirin-audit pin` adds each `@` name's top-level
  enumeration (site kind `enumeration`) to the declaration's identity and
  refuses an unresolvable name; both declarations carry `rdkafka_op.h`'s
  `rd_kafka_op_res_t` enumeration at `66910c10` and `046accf6`.
* **The caller branch check (rule 12m).** `handoff_branch::function_handling`
  reads every function definition's sites (calls through a function-pointer
  formal or member, direct calls handing a callback formal on, direct calls
  with an enumeration result) and, per handed pointer variable and per class
  of the result (each compared constant, then the rest), walks the body from
  entry to every exit (`if`, loops, `switch`, `goto`, `?:`, `&&`/`||`,
  statement expressions, noreturn calls) recording whether the variable is
  named again while it designates the object and what is returned.
  `handoff_branch::admit` follows the invocation's path from the invoker and
  checks each level (section 12m of the rules). Admitted, the candidate is
  classified with the handed formal taken (`supplied_owned`) or kept by the
  invoker (`_`, a parameter); refused, `callback-handoff-unbranched`; a direct
  call refuses `callback-handoff-direct`.
* **Fail-closed boundaries.** A body not in the facts, two different
  readings, a callback formal that escapes, an unread construct, a member
  copied into a local and a guard no function on the path compares each
  refuse. The check reads the invoker's own path (the decision's scope);
  that the invoker invokes the callback only there is the declaration's own
  assumption.

Tests: `crates/qcore-contract/tests/return_code_handoff_clause.rs` (each
reject case, alias equivalence, the digest), `op_constructor_clause.rs`
(the migration), `crates/qcore-c-gateway/tests/return_code_handoff.rs`
(KEEP-only and not-PASS forms admitted, a mismatched caller branch refuses,
an unbranched guard refuses, a typo and a wrong-type constant refuse, a
direct call refuses), `c_identity`'s enumeration test, the lexer test.

## 4. Measurement

BEFORE `before` (`6e5ecc58…`, base code and contracts) and AFTER `a1`
(binary `a0`, `b9f13b1b…`, the card's code at `781dad65a` with its
contracts and pins), each `run.json` naming its binary, every route with
the null property ([summary](artifacts/summary.json),
[`measure.sh`](scripts/measure.sh)):

| Route | BEFORE | AFTER | Report sha256 | Wall (s) |
| --- | --- | --- | --- | --- |
| Corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `e93a68bd…` → `99d6362f…` | 655.0 → 661.7 |
| System | 93 / 4 / 351 | 93 / 4 / 351 | `0ba647e3…` → `e572c2e2…` (contract digests) | 77.2 → 77.1 |
| Slice | 52 / 1 / 347, 4 Pending | the same | `d2e6ec19…` → `b84c12b7…` (the same) | 39.5 → 39.2 |
| Null property | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every row identical (4,921, 237, 271 rows) | | |

* **Moving rows** ([compare](artifacts/compare-corpus.json)): exactly the
  four predicted Unknown rows, one leaf each: `call to uncontracted function
  rd_kafka_poll_cb` → `callback-handoff-direct` in `rd_kafka_consume0`,
  `rd_kafka_consumer_close0`, `rd_kafka_q_serve_rkmessages`,
  `rd_kafka_background_queue_serve`. System and slice: none.
* **No Safe gain and no Safe → non-Safe move**; no leak, violation or
  `Unsafe` added; the fold demoted nothing. So there is no Safe row to list
  with its clauses and premises.
* `rd_kafka_op_destroy`'s 131 leaves are unchanged (eight undeclared
  candidates, first named `rd_kafka_DeleteRecords_leaders_queried_cb`); the
  serve calls naming `rd_kafka_poll_cb` stay `callback-uncontracted` (4
  leaves). Declarations consulted 418 → 420 (the two hand-offs, declared
  assumptions).

**Prediction against outcome.** As predicted on every route. Not as
predicted: the two declared candidates' own rows did not trade leaves
(their first refusals precede the entry custody).

## 5. Lean

`proof/QCore/ReturnCodeHandOff.lean`, audited by
`proof/AuditReturnCodeHandOff.lean`: 14 theorems, standard axioms only, no
placeholder, no project axiom: ownership after the call is a function of
(result, guard) (`owner_after_is_the_guard`, `owner_after_eq`,
`owner_after_ne`); an admitted branch never touches a taken object and
composes through a forwarder (`an_admitted_branch_never_touches_a_taken_object`,
`the_caller_keeps_what_the_guard_leaves`,
`admission_composes_through_a_forwarder`); a guard the caller does not
branch on never admits (`an_unbranched_guard_never_admits`); alias
canonicalisation is identity on meaning (`an_alias_means_its_name`,
`an_undeclared_alias_rejects`); the `#absent` migration preserves meaning
(`the_migration_preserves_absent`, `a_bare_absent_rejects`,
`a_function_designator_is_unchanged`); `owner_vectors_hold` and
`branch_vectors_hold` (Rust `OWNER_VECTORS`, `BRANCH_VECTORS`). Lean audits
322 → **323**.

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 779 → **780**.

* **Retired:** `OP-APP-RK-CALLBACKS-VIA-POLL-CB` (the decision taken,
  `rd_kafka_poll_cb` stated, read at both revisions, measured on the
  served-queue callers).
* **Amended:** `OP-CALLBACK-CANDIDATE-CONTRACTS` (nine → eight, all shape
  (b)).
* **Registered:** `HANDOFF-DIRECT-CALL-READING`,
  `HANDOFF-CANDIDATE-OWN-CALLBACKS`.
* **Unchanged:** `SERVE0-CALLBACK-HAND-ON-WITHDRAWN` (no
  `rd_kafka_op_handle` declaration falls out).

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/) (written by the gate).

## Files

* `crates/qcore-frontend/src/{lexer,frontend}.rs`; `crates/qcore-schemas/src/{handoff,constructed_state,retained_by}.rs`;
  `crates/qcore-contract/src/{contract_handoff,contract_constructed_state,contract_loader,contract_callback,contract_revision}.rs`;
  `crates/qcore-c-gateway/src/{handoff_branch,callback_member_closure,callback_contracts,legacy_callback_route,legacy_retained_by}.rs`;
  `crates/qcore-audit/src/{c_identity,contract_pin,contract_revision,member_prepass,pinned_route}.rs`,
  `crates/qcore-audit/examples/handoff_census.rs`;
  `crates/qcore-verification/src/{driver,source_imports}.rs`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`
  ([`write_contracts.py`](scripts/write_contracts.py)), `contracts.lock`,
  `contracts.json`; pins `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* Lean: `proof/QCore/ReturnCodeHandOff.lean`, `proof/AuditReturnCodeHandOff.lean`, `proof/QCore.lean`.
* Records: `docs/agents/{gateway,README}.md`, the spec, the grammar,
  `docs/STATUS.md`, `docs/development/increments/{wave-48-part8,README}.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
