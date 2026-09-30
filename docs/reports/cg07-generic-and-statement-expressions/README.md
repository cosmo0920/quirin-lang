# CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: Clang's selected `_Generic` association and GNU statement expressions

Card CG07 of the [Clang gap queue](../../plans/clang-gap-cards/README.md#phase-1-expression-elaboration-and-contract-application-f),
wave 48 round 7, gap entries **E17** and **E19**, route **G** (route T unchanged).
Base `c374de96` (local `main` = `origin/main`). Gateway
[rule 11g](../../agents/gateway.md#constraint-11g); `QCore.StatementExpressions`
(new `AuditStatementExpressions.lean`). Frozen [prediction](PREDICTION.md).

**Outcome.** 1,414 corpus statement expressions are now lowered (1,327 of them
glibc 2.43's `assert`), and braced-construct leaves on their lines fall from
1,338 to 69; **no function moves** (608 / 10 / 4,216 before and after), as
re-predicted: each lowered block exposes its own inner refusals, above all
the call to `__assert_fail`, which does not return and which no contract
declares (card CG21, `W43-LIBC-NORETURN`). No corpus `_Generic` needs Clang's
selection: every one is glibc's selection-invariant shape, which route 1
already read (a correction to CG00's E17 attribution).

## 1. What changed

### 1.1 E17: Clang's selected association (`generic_association.rs`, `legacy_generic_selection.rs`)

- `generic_association::associations` / `selected` read a
  `GenericSelectionExpr`'s association kinds in source order and the one Clang
  marks `"selected": true`, and its expression (the last child of the entry).
- **Route 2** in `legacy_generic_selection.rs` is asked only where route 1
  (W44's selection invariance) refuses as `NotSelectionInvariant`,
  `CastOperandNotACall` or `CalleeResultNotADeclaredPointer`; every
  selection route 1 lowered keeps its lowering. The keyword is matched to one
  AST node by line and its ordinal among that line's `_Generic` keywords, only
  where the stream's line equals the analysed source's line token for token
  and the counts agree; this reader's associations must be Clang's, kind for
  kind. The selection is replaced by the selected association (nested
  selections first), parenthesised unless a direct call or a name. The
  controlling expression and the other associations are dropped. With no AST
  (unit tests, a failed Clang run) the selection keeps its refusal.
- The elaborator reads a `GenericSelectionExpr` as its selected association
  (`FORM_TABLE` row `generic selection` admitted).

### 1.2 E19: the statement-expression reading (`statement_expression.rs`)

`decide(body, node, position)` computes the premises and asks `admit`
(bound to Lean by `ADMISSION_VECTORS`):

| Premise | Refusal |
|---|---|
| Position: discarded (through parentheses, `__extension__`, implicit conversions, `(void)`, commas), whole one-declarator initializer, whole value assigned to a named object (`NAME` or `*&NAME`), whole `return` operand (`POSITIONS`) | `Position(name)`: operand, call actual, condition (a `?:` arm), assigned path, cast value |
| Hygiene (`Body::hygiene`): no inner name is a formal's, a declaration's outside every other statement expression, an enclosing or second declaration's, or a referenced non-inner declaration's | `Shadow` |
| No `static`/`extern`/thread-local inner declaration | `Storage` |
| Where the value is used: no inner object's address taken (`&`, array decay, a member or element of either) | `Escape` |
| Where the value is used: the last item is an expression | `NoValue` |

The route has no block scopes, so the hygiene premise is what makes its flat
names exact (`flat_names_exact`; `shadow_breaks_flat` shows the premise is
needed). Disjoint statement expressions may reuse a name, as sibling blocks do.

### 1.3 The route-G lowering (`legacy_statement_expression.rs`)

A fallback after every older route of a statement (`expression_statement`'s
last step) or `return` (its chain's last step), recorded in the parity stream
(`statement-expression-statement` / `-return`, with each block's position,
family, admission and refusal). The full expression is matched to its Clang
node by line and exact tokens; every statement expression is decided before
anything is lowered; the lowering then reads **the parser's own stream**:

- discarded: each comma operand in order, a block parsed by `parse_block` as
  an ordinary block, any other operand a statement of its own;
- initializer / assigned / returned: the block's statements before its last
  (`parse_statement` from the stream), then `T NAME = e`, `NAME = e` or
  `return e` through that position's own routes.

An inner statement is therefore read exactly as it would be in an ordinary
block, including nested statement expressions (this hook again), `if`,
`return` out of the block and custody calls; an inner refusal keeps its own
reason. A decline restores the cursor and the site and element counters, so
the older refusal stands byte for byte.

### 1.4 The elaborator (`elaboration.rs`, additive)

A straight-line block (declarations, expression and null statements) is its
inner full expressions in order, each sequenced before the next; each inner
object's `LifetimeStart` at its declaration and `LifetimeEnd` after the
value; every event of the block is a value event of the whole, so a consumer
is ordered after the block. `commutes` gains a lifetime arm (a lifetime event
touches its own object only; the fallback arm is now unreachable and the
`Unlisted` commutation is kept for a future event kind). A block with another
statement refuses as `statement expression control` (a new `FORM_TABLE` row);
a pointer value where the block takes an inner address as
`Refusal::InnerEscape`. Route G's own elaborator hooks still decline braces,
so this arm serves the model binding and future positions
(`CG07-STATEMENT-EXPRESSION-POSITIONS`); it changes no corpus figure.

### 1.5 Shared items added (for the integrator)

- `elaboration.rs`: arms `GenericSelectionExpr` and `StmtExpr` (method
  `statement_expression`); `FORM_TABLE` rows `generic selection` and
  `statement expression` flipped from `Refused("CG07")` to `Admitted`, new row
  `statement expression control` (`Refused("CG07")`), length 38 → 39; a
  lifetime arm in `commutes` (the `_` arm removed as unreachable,
  `Commutation::Unlisted` kept with an `allow`); `Refusal::InnerEscape`.
  `QCore.ElaborationCore.formTable` updated to match.
- `typed_access_path.rs`: `StatementIndex::original_line_texts` (and the
  private `original_lines` the CG10 `by_occurrence` now shares).
- `legacy_generic_selection.rs`: `ClangSelections`, `clang_selection`,
  `primary` (route 1's parenthesising, factored out unchanged).
- `legacy_c.rs`: the module line, one hook at the end of
  `expression_statement`, one at the end of the `return` chain, and
  `expression_origin` in the `return` branch.
- No change to `typed_place.rs` or `call_instantiation.rs`.

## 2. Cited claims (Q11, Q21, Q37, Q38)

| Claim | Reproduced? |
|---|---|
| Q21: G reads `_Generic` only by selection invariance; Clang's selection is unimplemented | **Reproduced** (W44's module; CG00's `e17_…` control refused differing associations as `call to uncontracted function \`_Generic\``). |
| Q11: route T has no `StmtExpr` or `GenericSelectionExpr` branch | **Reproduced** (both refused as `unmodeled callback expression …`, unchanged by this card). |
| Q37: the typed position shapes refuse a statement expression (`statement-expression` path cause) | **Reproduced**; unchanged (that reader's shapes are rule 10n's). |
| Q38: the lexical nested-assignment closure treats a statement expression's braces conservatively, which is not elaboration | **Reproduced**; unchanged (the closure is a premise reader). |
| CG00's E17 attribution: 19 leaves on 13 functions | **Correction.** No corpus leaf is a refusal of a selection: BEFORE and AFTER have 0 `_Generic` call leaves. The leaves on `_Generic` lines (52 on 19 functions here, [generic-lines-before.json](artifacts/generic-lines-before.json)) are other constructs' refusals, and one class of them is caused by route 1's own lowering (`CG07-GENERIC-TOKEN-AST-MATCH`, §5). |
| CG00's "18 freed alone" | **Correction** (upper bound): 15 of the 18 block on glibc `assert`, whose block ends in `__assert_fail`; the other three on `container_of`/typeof bodies or a `_Generic` line (see [PREDICTION.md](PREDICTION.md)). |

CG00-G-CORRECTION-RESIDUALS (E14/C11/N14/C12) is not touched.

## 3. Measurements

Scripts: [measure.sh](scripts/measure.sh) (system route and tail-pause slice;
the corpus arbiter ran through `scripts/run_pinned_audit.py` with the command
of the round's brief at `--jobs 4`), [compare.py](scripts/compare.py),
[census.py](scripts/census.py), [generic_lines.py](scripts/generic_lines.py),
[sole_blockers.py](scripts/sole_blockers.py). Runs under `private/cg07/`.

### 3.1 BEFORE (base `c374de96`, binary `6c120e5b…`)

Corpus arbiter (`66910c10`): **608 / 10 / 4,216**, `resource-diagnostics.json`
`3a25c4e9…`, **byte-identical** to the round's baseline (CG10-BOUND-UNTRACKED-ROOTS'
AFTER). System route 41 / 2 / 191 and tail-pause slice 28 / 1 / 240 / 1 Pending,
the round's figures (the base binary rebuilt from `c374de96` after the AFTER
runs, byte-identical `6c120e5b…`). 2,000 braced-construct leaves on 856 functions (initializer lists
included); 1,338 of them on lines holding `({`.

### 3.2 AFTER (code `5220b1a9`, binary `e2dbc773…`)

The tip's code differs from the measured code only by formatting, two elided
lifetimes and one reordering: a discarded chain now locates every operand's
span before lowering any (a decline never follows a partly parsed block). The
reordering changes an outcome only where a span fails to locate, and the
AFTER parity stream records no such decline, so the measured figures stand for
the tip.

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus | 608 / 10 / 4,216 | **608 / 10 / 4,216** (`8b9e4047…`) | **none**; 584 Unknown functions change leaves |
| system (`046accf6`) | 41 / 2 / 191 | **41 / 2 / 191** | none; 3 functions change leaves (a `container_of` block: braced −3, indirect access +3, `__builtin_offsetof` +3), [system-comparison.json](artifacts/system-comparison.json) |
| tail-pause slice (`90978ac6`) | 28 / 1 / 240 / 1 Pending | **28 / 1 / 240 / 1 Pending** | none; 2 functions change leaves (the same family, −2 / +2 / +2), [slice-comparison.json](artifacts/slice-comparison.json) |

Corpus leaf deltas ([corpus-comparison.json](artifacts/corpus-comparison.json)):

| Leaf | Δ |
|---|---:|
| braced initializer, expression, or declaration is not modeled | −1,311 |
| call to uncontracted function `__assert_fail` | +1,232 |
| unmodeled call, aggregate or indirect access in branch condition | +165 |
| unresolved resource identity (`unknown_alias`) | +109 |
| indirect memory access is outside the resource projection | +77 |
| call to uncontracted function `__builtin_alloca` | +41 |
| call to uncontracted function `__builtin_offsetof` | +38 |
| resource call with a formal embedded in an unmodeled expression | +3 |
| unsupported assignment expression; unrecognized assignment destination | +1; +1 |

**No Safe → non-Safe move, no correction, no new provisional Unsafe, and no
leak or violation added** (no `Leak`, `DoubleRelease`, `UseAfterRelease` or
conflict entry in the deltas). Every change is an Unknown function's leaves:
one braced refusal replaced by the refusals of the statements it hid. The
`assert` family accounts for the `__assert_fail` leaves and the branch
conditions (its `if (E)` where `E` reads a path the condition routes refuse);
`STR_DUPA` (`__builtin_alloca`), `container_of` and typeof temporaries for
`__builtin_offsetof`, the indirect accesses and the alias leaves.

### 3.3 The consultations ([census-after.json](artifacts/census-after.json))

1,414 consultations admitted (593 functions), 46 declined (28 functions).
Blocks by position and family:

| Family | Admitted | Position |
|---|---:|---|
| glibc `assert` (`((void) sizeof (…), __extension__ ({ if (E) ; else __assert_fail (…); }))`) | 1,327 | discarded |
| `RD_KAFKAP_STR_DUPA` (`*(&dst) = ({ const char *__old = …; … __builtin_alloca …; __new; })`) | 41 | assigned (`*&NAME`) |
| typeof temporaries (`container_of`, `RD_MIN`-style) | 39 | assigned, initializer |
| `({ rd_kafka_fetch_pos_t _fetchpos = { … }; _fetchpos; })` | 6 | initializer, returned |
| other | 1 | |

Declined: 14 under a `?:` arm, 11 call actuals (the `_fetchpos` family
among them), 11 assigned to a path, 9 with no unique Clang node (a run an
earlier stream pass rewrote), 1 under a cast. Statement expressions in
controlling expressions and `for` clauses never reach the hook (libcurl's
`curl_easy_getinfo` type check). 69 braced leaves remain on `({` lines
([stmtexpr-lines-after.json](artifacts/stmtexpr-lines-after.json)); no
declined consultation hits a hygiene, storage or escape premise in the corpus.

### 3.4 `_Generic`: `memchr` and `bsearch`

Every corpus selection is glibc's `__glibc_const_generic` shape, identical up to
a pointer cast, so route 1 lowers it before route 2 is ever asked: `memchr`
reads as its declared call, `bsearch` as `call to uncontracted function
\`bsearch\`` (7 leaves on 6 functions, BEFORE and AFTER;
`W44-BSEARCH-UNDECLARED` amended with the re-measurement). Leaves on `_Generic`
lines: 52 on 19 functions before and after.

### 3.5 Prediction against outcome

| Predicted ([PREDICTION.md](PREDICTION.md)) | Outcome |
|---|---|
| Safe 0 from CG00's list (18 → 0), at most 3 elsewhere | **0** anywhere |
| Unsafe 0, Safe → non-Safe 0 | 0, 0 |
| braced leaves fall, `__assert_fail` and inner families rise | −1,311; +1,232 `__assert_fail`, inner families as §3.2 |
| E17: 0 corpus moves | 0 |

The 13 `assert` functions of CG00's list are Unknown on `__assert_fail`; 10
Unknown functions in the corpus are now blocked by it alone
([assert-fail-sole.json](artifacts/assert-fail-sole.json), an upper bound
for a no-return path end): the four `tinycthread_extra.c` rwlock wrappers,
`rd_kafka_cgrp_set/clear_wait_resp`, `rd_list_set_cnt`,
`rd_kafka_confval_get_int`, `isFullyBalanced0`, `get_u32_at_offset`.

## 4. Evaluation order and axes

No order C does not guarantee is introduced: each admitted position has no
event unsequenced with the block (a named destination designates without
evaluating; a `return`'s exit and a store follow the value; a comma sequences
its operands), and the block's own statements are full expressions. The
elaborator arm makes the same claim checkable in any position (the block's
events are one group against its siblings). Axes covered: the four positions;
nested blocks; a `return` out of a block; inner custody (acquire, release,
leak); disjoint reuse of a name. Refused: operand, call-actual, `?:`-arm,
path-destination and cast positions; shadowing names; static inner objects;
an inner address in a used value.

## 5. Follow-ups

- **Retired:** `CG07-GENERIC-AND-STATEMENT-EXPRESSIONS` (every DISCHARGE clause
  holds; no absorbed follow-up).
- **Registered:** `CG07-STATEMENT-EXPRESSION-POSITIONS` (69 braced leaves on
  statement-expression lines, 46 declined consultations on 28 functions) and
  `CG07-GENERIC-TOKEN-AST-MATCH` (52 leaves on 19 functions on `_Generic`
  lines; `rd_kafka_conf_validate_single` blocked by it alone: CG02, CG05 and
  CG06 each decline its lowered `return` with `no unique Clang node`).
- **Amended:** `W43-LIBC-NORETURN` (1,232 `__assert_fail` leaves on 533
  functions, 10 blocked by it alone) and `W44-BSEARCH-UNDECLARED` (7 / 6,
  unchanged).
- Ledger 659 → 660 entries; `dimensions.named_followups` was 658 on the base
  (one short) and is now 660 ([update_ledger.py](scripts/update_ledger.py)).

## 6. Tests and controls

- `tests/cg07_statement_expressions.rs` (7): the selected association is
  evaluated and an unselected one is not (a release in an unselected
  association is no event: the later free is the only one, and without it
  the allocation leaks), the controlling expression is never evaluated,
  route 1 keeps its lowering; the four positions (`*&NAME` included);
  `({ void *q = q_alloc(); q; })` with no release is a **leak**, an inner
  release before an outer one a double release, a `return` out of a block;
  `({ int t; &t; })`, an inner array's decay, a shadowing name, operand,
  call-actual and condition positions and a static inner object refused;
  glibc's exact `assert` shape leaves only the `__assert_fail` call; an inner
  refusal keeps its own reason.
- **Flipped deliberately:** CG00's `e17_only_selection_invariant_generics_are_read`
  (both differing selections now Safe) and `e19_statement_expressions_are_refused`
  (both initializers now Safe; an operand position added as the refusal
  control); W44's `associations_that_differ_refuse`,
  `a_cast_of_an_integer_result_refuses` and
  `a_cast_that_does_not_cover_the_association_refuses`, renamed `…_reads_clangs_selection`
  and now asserting each selection is analysed exactly as its selected
  association spelled out; CG02's `cg02_unlisted_and_out_of_scope_forms_are_refused_by_name`
  (the two CG07 rows replaced by the `statement expression control` refusal).
  Route T's controls are unchanged.
- Unit tests: `generic_association` (selection reading), `statement_expression`
  (premises over Clang ASTs, the Lean vectors), `elaboration` (selection and
  block elaboration, lifetime ordering, the sibling conflict, the lifetime
  commutation vectors).

## 7. Lean

`QCore.StatementExpressions` (`proof/QCore/StatementExpressions.lean`,
registered in `QCore.lean` and `AuditStatementExpressions.lean`, 23 theorems,
no `sorry`, no project axiom): `select_events`, `control_no_event`,
`unselected_no_event`; `value_is_final`, `lifetimes_end_at_block`,
`events_exec`, `lowering_exact`, `discarded_exact`, `returned_exact`;
`flat_names_exact`, `shadow_breaks_flat`; `admitted_no_escape`,
`escape_refused`, `shadow_refused`, `static_refused`, `no_value_refused`,
`discarded_needs_no_value`, `admission_vectors_hold`;
`unlisted_position_refused`, `admitted_positions`,
`operand_positions_refused`; `lifetime_touches_own`,
`lifetime_vectors_hold`. Bound to Rust by
`statement_expression::tests::cg07_lean_vectors_match` (`admissionVectors`,
`positionTable`) and `elaboration::tests::cg07_lifetime_vectors_match_commutes`
(`lifetimeVectors`); `QCore.ElaborationCore.formTable` follows `FORM_TABLE`.

## 8. Gate

At tip code `692e1c07` (scripts [gate.sh](scripts/gate.sh) and
[gate_after.sh](scripts/gate_after.sh), logs `private/cg07/gate-692e1c07`,
exits in [gate/](gate/), figures in [gate.json](gate.json)):

| Step | Exit | Counts |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 401 units, 414 summaries, **7,045 passed, 0 failed**, 11 ignored |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 256 audits (255 + `AuditStatementExpressions.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 1, then **0** after `build_matrix.py . --gate gate.json` (`09a` 0, `09b`) | 365 (the one failure: the derived Lean audit count 255 → 256) |
| `python3 scripts/build_docs_index.py` | 0, 0 (`10b`) | |

`providers/` is untouched, so the provider gate is not due. Clang 21.1.8. An
earlier gate start at `3446abd4` was stopped by the card before its first step
completed, to reorder the discarded-chain lowering (§3.2); it produced no
result.
