# W44-BRANCH-CONDITION-CALLS: calls in controlling expressions

Card W44-BRANCH-CONDITION-CALLS, 2026-09-25, on base `c1e69bb5` ("Open the
wave-44 increment record"). The recogniser and its census were committed with
the rule switched off (`2a4fe098`, `45d4c5ce`, `df5edb61`); the forecast was
frozen from the census before the rule was switched on
([PREDICTION.md](PREDICTION.md), `80997b44`); the admission is `173ae5c2`.
Later commits change the report, the ledger, scripts and evidence only.

## Headline

* **Rule** ([§2](#2-the-rule)). A controlling expression (`if`, `while`,
  `for`, `do`-`while`, `switch`) that every pre-existing route refused and that
  carries a call is desugared: it is read as a Boolean tree over `&&`, `||`,
  `!` and a top-level `?:`; every call of an operand becomes its own
  `ContractCall` node, classified by the ordinary statement classifier, in C
  evaluation order; each operand then decides through one branch node; and the
  connectives become **edges** — the right operand of `&&` is entered only from
  the left's true edge, that of `||` only from its false edge, the arms of `?:`
  only from the matching edge, and `!` swaps exits. `__builtin_expect(E, K)`
  (the `likely`/`unlikely` macros) is read as `E`. `(NAME = CALL(...))` into a
  plain declared scalar or single-level pointer local lowers to the statement
  `NAME = CALL(...)` followed by the test of `NAME`.
* **What stays refused.** Every uncontracted or classifier-refused call, every
  custody result (owned, reallocated, descriptor) — so
  `if ((p = malloc(n)) == NULL)` keeps **exactly** today's lowering, the
  refusal node and its site numbering — a callee in the function's own
  recursive component (constraint 4), two effectful calls of one operand
  (unsequenced in C), type operators, braces, increments, calls through
  expressions, and any operand no existing route decides.
* **Lean.** `QCore.ConditionCallDesugar`, 25 theorems in
  `AuditConditionCallDesugar.lean`; the central one,
  `the_desugaring_is_c_evaluation`, proves that for **every** assignment of
  operand values the plan applies exactly the calls C's short-circuit
  evaluation applies, in C's order, and leaves by C's exit. No placeholders,
  no project axioms.
* **Arbiter.** BEFORE and the census run reproduce the
  W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte.
  AFTER removes **292** condition leaves and adds **36**, every one attributed
  (`attribution_complete: true`): 36 same-line exchanges into
  `unknown_alias`, one moved `unknown_alias` coordinate and **one decided false
  leak**.
* **Moves: 0 functions, 0 units, 0 violation candidates. Leak 271 → 270**
  (a withdrawal by refinement, [§4.3](#43-the-leak-withdrawal)).
* **Prediction:** 276 leaves forecast on Unknown functions — **276 measured,
  exact**; 0 function and 0 unit moves, as forecast. The leak forecast
  (271 to 276, accusing) **missed**: the only leak move is a withdrawal.
* **Ledger 345 → 354**: `W43-INDEX-TRAVERSAL-CONDITION-CALL` retired against
  its discharge condition; eight blocker-sized residuals, one decision record
  and one finding registered ([§6](#6-ledger)).

| Measure | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | 294 / 85 / 4,455 | 0 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `leak` / `unknown_alias` / `unsupported` | 271 / 4,358 / 45,848 | **270 / 4,393 / 45,556** | −1 / +35 / −292 |
| `summary_conflict` / violation candidates | 1 / 0 | 1 / 0 | 0 |
| branch condition: leaves / functions / sole | 8,287 / 2,320 / 49 | **8,018 / 2,268 / 49** | −269 / −52 / 0 |
| loop condition | 294 / 187 / 5 | 288 / 183 / 5 | −6 / −4 / 0 |
| for-loop condition | 839 / 537 / 1 | 839 / 537 / 1 | 0 |
| do-while condition | 29 / 27 / 0 | 28 / 27 / 0 | −1 / 0 / 0 |
| switch expression | 78 / 75 / 1 | 78 / 75 / 1 | 0 |

Family rows are over Unknown functions, from
[blocker-census.txt](blocker-census.txt) and its predecessor at the baseline.
The 16 further removed leaves are on provisional-Unsafe functions.

**Why the movement is small.** The census answers the brief's question
directly: most call-bearing conditions do not fail on their connective but on
a **call** the statement classifier refuses — an indirect argument
(`strcmp(p->name, x)`, 1,686 leaves at AFTER) or an uncontracted callee (1,000)
— or on a **call-free operand** beside the calls (1,211), which are other
cards' subjects. Short-circuit, ternary and assignment shapes were a minority
of the family. The rule consumes the ordinary contract lookup and the ordinary
classifier, so W44-CORPUS-CALLEE-SUMMARIES, W44-INDIRECT-CALL-ARGUMENT-ACCESS
and W44-INDIRECT-CHAIN-ACCESS reach condition calls without further change; the
residual classes are registered by blocker so that each landing can be
re-measured against its own class.

## 1. Census

### 1.1 Instrumentation

`crates/qcore-c-gateway/src/condition_call_census.rs` (feature
`indirect-census`, stream `QCORE_CONDITION_CALL_CENSUS`) records every
controlling expression every pre-existing route refused, **after** that
decision, with the desugaring's own answer (`admitted` or its refusal code), the
classifier's refusal of a refused call, and a lexical inventory of callees
(contract in force, local definition, result mode). A refused call-free
expression is recorded as `no-call` because it shares its leaf with any
call-bearing expression on the same line and position. The rule was compiled
in and not consulted (`ADMIT = false`), and a declined plan restores the
classifier's counters, so the census run reproduces all five BEFORE artifacts'
verdict-bearing bytes ([census-identity.txt](census-identity.txt)). The final
records (the last per expression, which is the final analysis's) are archived
as [census/condition-call-census.final.jsonl.gz](census/condition-call-census.final.jsonl.gz).

Two census iterations preceded the one of record, both before the forecast:
the first run (`2a4fe098`) was stopped when the need for `no-call` records
became clear; the second (`45d4c5ce`) showed pointer-local destinations
(`(elem = rd_list_elem(l, i))`) and a parenthesized `__builtin_expect`
constant refused, and the recogniser was widened to read both (`df5edb61`).
The census of record is `df5edb61`.

### 1.2 Split by operator shape, callee contract status and result kind

[census/predict.txt](census/predict.txt) splits the families by the rule's
first refusal, by operator shape, by callee status and by the callees' result
modes. For branch conditions on Unknown functions: 3,900 leaves are call-free
(`no-call`), 1,780 fail on a call the classifier refuses, 1,200 on a call-free
operand beside calls, 269 are admitted. By shape, call-bearing branch leaves
are `plain` (2,049) or `!` (709) far more often than `&&` (101), `||` (57) or
mixed. By callee status, 3,926 carry some uncontracted callee, 142 are all
externally contracted, 58 carry a contracted local callee. By result, owned
and reallocated results are 6 leaves: custody results in conditions are rare,
and every one is refused.

## 2. The rule

`crates/qcore-c-gateway/src/legacy_condition_desugar.rs`, a child module of
`legacy_c` with thin, commented hooks there (a `condition_plan` field on
`If`/`While`/`For`/`DoWhile`, `expression_plan` on `Switch`, one parse hook and
one compile hook per position, a `desugar` field on `Parser`).

1. **When.** Only where the position's existing lowering is its refusal: not a
   supported neutral expression, no B02/B03 isolated call, no typed read, and
   (for `if`) no null, distinct or element test. Every expression another route
   admits keeps its lowering byte for byte.
2. **Parse.** `parse` reads a top-level `?:`, then `||`, then `&&`, then a
   grouped `!`, over full-group parentheses; a top-level comma or assignment
   makes the operand a leaf. `& &` is always `&&` in valid C (a binary `&` with
   an address-of operand is a constraint violation), and `| |` is always `||`.
   `__builtin_expect(E, K)` with a literal `K` (parenthesized or not) is read
   as `E`: it has the value of `E`, `K` is constant.
3. **Operands.** A call-free operand decides through the existing routes, in
   the existing order: null / validity test, distinct test (branch position),
   neutral expression, typed read — and refuses otherwise, or when it reads a
   tracked descriptor as data. A call-bearing operand: every top-level call is
   classified as a statement by `classify_simple` (so the descriptor guard,
   nested-call siblings, alias and liveness obligations are the classifier's)
   and must be a `ContractCall` with a resource-free result; with two or more
   calls, each must be the identity on the abstract state (C leaves their order
   unspecified, `identity_calls_commute`); the residual, every call replaced by
   `0`, must be a supported neutral expression and carry no descriptor. One
   `(NAME = CALL(...))` per operand is admitted for a declared scalar or
   single-level pointer local that is no formal and no descriptor name and is
   read nowhere else in the operand; the operand then decides on `NAME` (its
   null test, or neutral).
4. **Local callees.** A callee defined in the unit is admitted through its
   validated W4 summary only when it lies outside the enclosing function's own
   recursive component, decided on the unit's direct-call graph (a superset of
   the derivation's candidates, so its components contain the derivation's).
   Inside the component a contract may be the neutral route's provisional
   `N(m)`; the B-series premise is kept for that case.
5. **Edges.** The plan's nodes are pushed with the position's line. In a loop,
   every edge to the false exit is recorded as a loop exit edge, so
   SCALAR-ROUTE-EXIT-EDGE's rule that such an edge is never removed holds for
   each; the plan's entry is the header every back edge and `continue` revisits;
   `for` keeps its increment before the header and its init before it; the
   literal-`0` do-while rule never applies (a call-bearing condition is not the
   token `0`).
6. **No new instruction.** The plan emits `ContractCall`, `NoOp`,
   `BranchOnNull`, `BranchOnValidity`, `BranchOnDistinct` and
   `IndirectScalarAccess` only, so `descriptor_custody::admits_descriptor_read`
   needs no new arm.

**Fresh temporaries.** A resource-free call result binds nothing the
projection tracks, so the call node carries no destination — B02's lowering.
A **derived** result is a deliberate reading recorded for the integrator
(`W44-CONDITION-DISCARDED-DERIVED-RESULT`): discarded, it is the classifier's
own "discarded derived = discarded `None`" (W43-LIBC-RESULT-ALIAS); written to
the program's pointer local, that binding tracks it (tested:
`a_derived_result_assigned_in_a_condition_is_tracked_by_its_binding`).

### 2.1 Tests and pins

`crates/qcore-c-gateway/tests/condition_call_desugar.rs` (17 Clang-backed
tests) covers conjunction, disjunction (right operand only on the false edge:
a double release, no leak), ternary (one arm), negation, `__builtin_expect`,
scalar and derived assignments, the unchanged `malloc` refusal (no call node
lifted, the refusal node kept), uncontracted calls, unsequenced effects,
refused shapes, a validated local summary versus a recursive callee, all three
loops (re-evaluated on the back edge: double release) with both loop exit edges
recorded, switch, and descriptors (validity leaf refines, owned descriptor
assignment refused, a descriptor read as data refused). Every `Safe` result is
rechecked through `check_legacy_safety_certificate`. Four unit tests pin the
parser. Two B-series pins change **deliberately** and say so where they stand:
`branch_condition_calls.rs`'s short-circuit and conditional releases (renamed
`…_is_desugared_…`; they keep the leak the refusal protected) and
`loop_switch_condition_calls.rs`'s `ready && r_release_code(p)` loop (keeps the
leak and the back-edge double release). B02's frozen mutation harness names the
old short-circuit test; it is historical and was not rerun.

## 3. Lean parity

`proof/QCore/ConditionCallDesugar.lean`, registered in `QCore.lean`, audited
by `proof/AuditConditionCallDesugar.lean` (25 `#print axioms`):

* `Cond` (leaves with their calls; `and`, `or`, `not`, `choose`), C's
  evaluation `eval`, the plan `Node`, its path `run`, and the desugaring
  `assemble` exactly as the Rust wires targets.
* `run_assemble` (general continuation form) and
  **`the_desugaring_is_c_evaluation`**: `run v (desugar c) = eval v c` for every
  leaf valuation `v`.
* The per-connective evaluation laws (`and_…_only_when_the_left_holds`,
  `or_…_only_when_the_left_fails`, `choose_evaluates_exactly_the_selected_arm`,
  `not_swaps_only_the_exit`), `a_skipped_operand_applies_nothing`, and the
  non-vacuity pair `the_short_circuited_release_is_not_applied` /
  `the_evaluated_release_is_applied`.
* Loops: `the_loop_exit_is_the_false_value`,
  `every_visit_re_evaluates_the_condition`.
* Admission: `admits` over `LeafFacts` with one refusal theorem per premise
  (`a_custody_result_refuses` — the `malloc` case —,
  `a_callee_in_the_own_component_refuses`, `unsequenced_effects_refuse`, …),
  `one_refused_leaf_refuses_the_condition`, `identity_calls_commute`.

The model is of the rule; faithful C extraction and the Rust–Lean
correspondence remain review premises (constraint 28).

## 4. Arbiter

### 4.1 Runs

| Run | Commit | Binary | Identity against the baseline |
|---|---|---|---|
| BEFORE | `c1e69bb5` | plain release | diagnostics, chunk-contracts, source byte-identical; contracts identical after the root alias; source-effects differs in its checkout-path digest ([before-identity.txt](before-identity.txt), [before-run.json](before-run.json)) |
| census | `df5edb61` | `--features qcore-c-gateway/indirect-census` | the same five answers ([census-identity.txt](census-identity.txt), [census-run.json](census-run.json)) |
| AFTER | `173ae5c2` | plain release | chunk-contracts and source byte-identical, contracts after alias; diagnostics differ as attributed ([after-identity.txt](after-identity.txt), [after-run.json](after-run.json)) |

The AFTER artifacts are archived as the new baseline in [artifacts/](artifacts/)
(deterministic gzip, `manifest.json` with raw and gzip SHA-256).

### 4.2 Attribution

[scripts/attribute.py](scripts/attribute.py) →
[attribution.txt](attribution.txt), [attribution.json.gz](attribution.json.gz),
`attribution_complete: true`:

| Class | Leaves |
|---|---:|
| `removed_admitted` — condition leaf whose every census record at its line is `admitted` | 292 (269 branch + 6 loop + 1 do-while on Unknown functions; 16 branch on provisional-Unsafe functions) |
| `exchange_at_admitted_line` — added `unknown_alias` at the very line of a removed condition | 36 |
| `desugared_function` — other delta in a function with an admitted condition | 2 (one moved `unknown_alias` coordinate in `mk_vhost_fdt_close`, L6122 → L6112; one removed leak) |
| `summary_neighbour` / `unattributed` | 0 / 0 |

Of the 292 removed conditions, 229 are read through `__builtin_expect`
(`likely`/`unlikely`); 40 are single operands, mostly one call to a locally
defined callee with a validated W4 summary (`flb_log_check`,
`cio_chunk_is_up`, `LZ4F_isError`), which B02 refused for any local
definition, and a few with two identity calls (`strlen(a) + strlen(b) < n`);
22 carry `&&`/`||` without the builtin; 1 is an assignment
(`(r = strcmp(a->name, b->name))`).

**The 36 exchanges** are all `__builtin_expect((…x…), K)` whose operand is a
null test of a name the projection cannot resolve (`if (unlikely(!rktp))`,
`likely(offsets != NULL)`, `unlikely(err)`): the refusal became a
`BranchOnNull`, whose `Unknown` refinement reports `unresolved resource
identity` — exactly what the unwrapped `if (!rktp)` reports. The function stays
`Unknown`; no answer is weakened.

### 4.3 The leak withdrawal

`rd_kafka_snappy_java_uncompress` (librdkafka `snappy.c`, provisional Unsafe
before and after) allocates `outbuf = rd_malloc(uof)` at line 1705 and tests
`if (unlikely(!outbuf)) { …; return NULL; }`. Before, the `unlikely` refusal
refined nothing, so the `return NULL` on the null edge carried a live
obligation and was reported (L20509). After, the condition is the null test it
spells, the null edge carries no resource, and the leaf is **decided** — a
withdrawal by refinement on a path C does not have, not by refusal. The
function keeps its other leaks and its status. Leak pin **271 → 270**
(constraint 7: attributed, not absorbed).

### 4.4 Prediction against outcome

| Forecast (frozen) | Outcome |
|---|---|
| 276 condition leaves leave their families on Unknown functions (269 / 6 / 1 / 0 / 0) | **276, exact per family**; plus 16 on provisional-Unsafe functions, outside the forecast's join by design |
| 0 function moves, 0 unit moves | 0, 0 |
| leak 271 → 271 to 276 (accusing) | **270 — missed**: no admitted path exposed a leak; one false leak was decided by the new null refinement |
| `unknown_alias` 4,358 ± 100 | 4,393 (+35) |
| `unsupported` ≈ 45,570 | 45,556 (−292) |
| no new violation candidate | none |
| `if ((p = malloc(n)) == NULL)` keeps its refusal | kept (test, and 3 `custody-result` leaves still refused in the corpus) |

### 4.5 Violation candidates

None added. The one leak move is a withdrawal. Provisional Unsafe results are
not upstream defect claims (invariant 2).

## 5. Validation

Gate commands, exits and counts are in [gate.json](gate.json) and §5.1; logs
are worktree-private under `measure/gate/` and `measure/provider-gate/` of the
card's worktree.

## 6. Ledger

* **Retired** `W43-INDEX-TRAVERSAL-CONDITION-CALL` (4,419 / 1,603 / 28 at its
  own census) against its discharge condition — the B-series call composed with
  the typed read per operand, obligations kept, Lean parity, measured re-pin —
  recorded verbatim with its ground in
  [retired-followups.json](retired-followups.json). Its remainder is refused by
  the call routes themselves and re-registered by blocker.
* **Registered** ([followups.json](followups.json); sizes from
  [residuals.txt](residuals.txt), AFTER Unknown functions, all five positions):

| Follow-up | Leaves | Functions | Sole |
|---|---:|---:|---:|
| W44-CONDITION-CALL-INDIRECT-ARGUMENT | 1,686 | 630 | 8 |
| W44-CONDITION-CALL-FREE-OPERAND | 1,211 | 622 | 0 |
| W44-CONDITION-CALL-UNCONTRACTED | 1,000 | 575 | 11 |
| W44-CONDITION-CALL-TYPE-OPERATOR | 259 | 113 | 1 |
| W44-CONDITION-CALL-ASSIGNMENT | 196 | 160 | 0 |
| W44-CONDITION-CALL-REFUSED-SHAPES | 74 | 62 | 3 |
| W44-CONDITION-CALL-RECURSIVE-LOCAL | 31 | 18 | 0 |
| W44-CONDITION-CALL-ERRNO-RESIDUAL | 22 | 19 | 2 |

  plus the decision record `W44-CONDITION-DISCARDED-DERIVED-RESULT` and the
  finding `W44-B02-RESIDUAL-DESCRIPTOR-READ` (the pre-existing one-call route
  admits `k_read(fd, b, 1) > fd`, a descriptor read as data in its residual;
  this card's route refuses it and does not change the older one). The
  call-free classes (4,766 `no-call` leaves) remain under W43's untyped, chain,
  address and write residuals.
* Ledger 345 → 354. `corpus.cycle19_raw_units` rewritten from the AFTER
  artifacts; the previous row is kept in
  [previous-corpus-row.json](previous-corpus-row.json).

## 7. For the integrator

* **Conflict surface.** `crates/qcore-c-gateway/src/legacy_c.rs` (module
  declaration, `Statement` fields, `Parser.desugar`, five parse hooks, five
  compile hooks — all next to the B02/B03 and W43 condition code that
  W44-INDIRECT-CHAIN-ACCESS and W44-INDIRECT-CALL-ARGUMENT-ACCESS may also
  touch), `crates/qcore-c-gateway/src/lib.rs` (one feature-gated module),
  `proof/QCore.lean` (one import after `LoopSwitchCallEvaluation`),
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
  `docs/development/increments/wave-44.md`, `docs/reports/INDEX.md`.
* **Composition.** A card that makes the classifier admit more calls moves this
  card's residual classes without touching its module; re-run
  `scripts/residuals.py` over the combined AFTER with this census, or re-take
  the census (the stream is behind `indirect-census`).
* **Order.** A combined arbiter must attribute removed condition leaves between
  this card and any card widening condition reads: a condition removed here is
  one whose census answer was `admitted`.

## Reproduction

```sh
python3 docs/reports/w44-branch-condition-calls/scripts/run_audit.py OUT COMMIT [--census]
python3 docs/reports/w44-branch-condition-calls/scripts/baseline_identity.py OUT BASELINE --root-alias ROOT=CANONICAL
python3 docs/reports/w44-branch-condition-calls/scripts/predict.py CENSUS_RUN OUT.json OUT.txt
python3 docs/reports/w44-branch-condition-calls/scripts/attribute.py BEFORE/resource-diagnostics.json AFTER/resource-diagnostics.json docs/reports/w44-branch-condition-calls/census/condition-call-census.final.jsonl.gz OUT.json OUT.txt
python3 docs/reports/w44-branch-condition-calls/scripts/residuals.py AFTER/resource-diagnostics.json docs/reports/w44-branch-condition-calls/census/condition-call-census.final.jsonl.gz BEFORE/resource-diagnostics.json OUT.json OUT.txt
python3 docs/reports/w44-branch-condition-calls/scripts/archive.py AFTER docs/reports/w44-branch-condition-calls/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w44-branch-condition-calls/artifacts
```
