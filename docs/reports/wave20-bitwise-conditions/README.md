# BITWISE-CONDITIONS — the integer tier, unary `!` and condition bindings (wave 20)

Card brief: *"Unify three named follow-ons that all live in the
condition/expression tier — `BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION` and
`IF-CONDITION-BINDINGS`. Decide v1 scope by measurement. Precedence declared
explicitly for every new operator, or unparenthesized mixing refused — decide,
record, pin."*

Base commit: main `e01cac2`.

**What landed.** All three, in full, behind one decision that the brief asked
for and that turned out to be the card's spine:

> **A precedence is declared only where it agrees with C. Where this tier's two
> sorts would force a different ranking than C's, the unparenthesized spelling
> is refused rather than silently redefined.**

`|`, `^` and `&` are bitwise or/xor/and over a two-sorted condition grammar;
unary `!` is a **pure arm swap** with a byte-equality pin against its
hand-swapped twin; and `let NAME = INTEGER;` is a **check-time constant
binding** a condition may read — the decidable subset of `IF-CONDITION-BINDINGS`,
with the runtime remainder measured and named rather than claimed. 31 new tests,
39 audited Lean theorems in a new module, no new axiom, **no new reserved word,
no new token and no new diagnostic code**.

---

## 1. BEFORE and AFTER, measured

Twenty shapes, compiled on the base commit and again after
(`measure/census_before.txt`, `measure/census_after.txt`; the probe was a
temporary test target and is not shipped).

| # | Shape | BEFORE | AFTER |
|---|---|---|---|
| C1 | `if (3 \| 4)` | `Q038` *reserved, no meaning* | **`Q045`** — an integer is not a condition |
| C2 | `if ((3 \| 4) == 7)` | `Q038` | **accepted** |
| C3 | `if (3 \| 4 == 7)` | `Q038` | **`Q045`** — the C-divergence refusal (§2) |
| C4 | `if (6 & 3 == 2)` | `Q038` | **`Q045`**, same rule |
| C5 | `if (6 ^ 3 == 5)` | `Q038` | **`Q045`**, same rule |
| C6 | `if (!true)` | `Q045` | **accepted** |
| C7 | `if (!(3 < 5))` | `Q045` | **accepted** |
| C8 | `let n = 3; if (n == 3)` | `Q500` | **accepted** |
| C9 | `if (n == 3)`, `n` unbound | `Q013` | **`Q013`, unchanged** |
| C10 | `let n = 3;` alone | `Q500` | **accepted** |
| C11 | `let n = 3 \| 4;` | `Q038` | **`Q038`**, message names the one admitted position |
| C12 | `return 3 \| 4;` | `Q038` | **`Q038`**, same |
| C13 | `let n = !true;` | `Q500` | **`Q500`, unchanged** |
| C14 | a `std.io` module | `Q015` | **`Q015`, unchanged** |
| C15 | `let total = 42 \| 1;` on `std.io` | `Q038` | **`Q038`** |
| C16 | `if (!!true)` | `Q045` | **accepted** |
| C17 | `if (!true \|\| false)` | `Q045` | **accepted** |
| C18 | `if (true) { let n = 3; }` | `Q500` | **`Q044`**, naming `IF-ARMS` |
| C19 | `if ((-1 & 255) == 255)` | `Q038` | **accepted** |
| C20 | `if ((1 \| 2 & 3) == 3)` | `Q038` | **accepted** |

Eight rows gained acceptance; nine kept a refusal and moved only a code or a
message; three (C9, C13, C14) are byte-unchanged. **No row lost a refusal it was
entitled to**: every previously refused shape that is still refused is refused
for a reason this card can state, and the three shapes that moved from `Q038` to
`Q045` (C1, C3–C5) moved because the *gate* stopped owning them, not because the
program became legal.

---

## 2. The decision: precedence declared where it agrees with C, refused where it would not

The brief offered a choice — declare a precedence for every new operator, or
refuse unparenthesized mixing. Measurement forced a **split**, and the split has
a rule.

The tier is **two-sorted**. A comparison consumes integers and produces a
condition; a connective consumes conditions; there is no coercion in either
direction, and adding one would mean deciding that "non-zero is true", which is
a semantic rule none of the four checked models has. So a bitwise operator can
only sit *inside* a comparison operand, i.e. **tighter** than every comparison.

C ranks them the other way. In C, `|`, `^` and `&` are all **looser** than the
comparisons, so `flags | MASK == 0` is `flags | (MASK == 0)`. And `!` is
**tighter** than every comparison, so `!a < b` is `(!a) < b`. Adopting the
ranking the sorts force would give those two spellings a different meaning from
the one a C reader brings — silently, with no diagnostic, on exactly the kind of
mask test the operators exist for.

**So they are refused.** `A | B == C` and `!A < B` are not programs; `(A | B) == C`
and `!(A < B)` are. Everything whose C ranking the sorts *permit* is declared.

### 2.1 The precedence table

| Level | Operators | Associativity | Operand sort | Result sort | Agrees with C? |
|---|---|---|---|---|---|
| 1 (loosest) | `\|\|` | left | condition | condition | **yes — declared** |
| 2 | `&&` | left | condition | condition | **yes — declared** |
| 3 | `!` (prefix) | right | condition | condition | **no — operand must be grouped** |
| 4 | `==` `!=` `<` `<=` `>` `>=` | **none** | integer | condition | n/a (non-associative both here and in effect) |
| 5 | `\|` | left | integer | integer | **no vs. level 4 — operand must be grouped**; yes vs. 6/7 |
| 6 | `^` | left | integer | integer | as level 5 |
| 7 (tightest) | `&` | left | integer | integer | as level 5 |
| — | `\|>` | — | — | — | **not in the tier at all**; the mixture is refused, never ranked |

Grammar (spec §G02i, §L21):

```ebnf
cond  = conj , { "||" , conj } ;
conj  = neg  , { "&&" , neg  } ;
neg   = "!" , neg | cmp ;
cmp   = int , [ op , int ] ;
int   = xor , { "|" , xor } ;
xor   = and , { "^" , and } ;
and   = atom , { "&" , atom } ;
atom  = "(" , cond , ")" | "true" | "false" | [ "-" ] , INTEGER | NAME ;
```

Both the "declared" and the "refused" halves are pinned, each against the
intended grouping (equal emission) **and** against the other grouping (different
emission), so neither can pass vacuously:

* `the_bitwise_ranking_is_cs_own_and_is_declared` — the witness is chosen so the
  groupings genuinely differ: `4 | (1 & 1)` is `5` and `(4 | 1) & 1` is `1`; and
  for the middle level, `4 | (1 ^ 5)` is `4` and `(4 | 1) ^ 5` is `0`. (The
  first witness attempted, `1 | 2 & 3`, was **discarded on measurement**: both
  groupings evaluate to `3`, so the pin would have been vacuous.)
* `the_negation_binds_tighter_than_the_connectives`
* `an_ungrouped_bitwise_comparison_operand_is_refused`
* `an_ungrouped_negation_operand_is_refused`
* `a_comparison_is_not_associative` — `1 < 2 < 3` refuses
* `the_two_sorts_do_not_coerce` — both directions

### 2.2 One parser, two sorts, linear time

The first design carried the old parser and **backtracked**: try the
parenthesized group as a condition, and on failure re-parse it as an integer.
That is correct and it was rejected on measurement of its shape rather than of
its output: each nesting level would parse its contents twice, so a condition
with *n* nested parentheses costs `2^n`. A compiler should not have that
property.

The shipped parser is a single pass over a two-sorted grammar: each level parses
its sub-level, then loops on its own operator, and a **sort mismatch is a
refusal rather than a second parse**. That is also what makes each refusal
specific — the message says which rule was broken, not "outside the tier".

### 2.3 The `|>` versus `| >` constraint, inherited and answered

Card `OR-AND-LOWERING` handed this card the longest-match constraint it had
recorded and could not spend: `|>` wins over `|` by width, so a bitwise or
adjacent to a `>` cannot be written `|>`.

Measured, and the answer is better than the inheritance suggested: **the
adjacency can never produce a wrong acceptance, only a refusal**, because `>`
has no left operand inside an integer expression. `1 |> 2` in a condition is a
pipeline the desugarer left alone (its bracket-depth-0 rule), and it reaches the
tier as an unknown operator and refuses at `Q045`. `(1 | 2) > 0` is the spaced
spelling and it parses. Both are pinned in
`a_pipeline_is_still_refused_rather_than_ranked`, together with
`(1 | 2) == 3 || false`, where both bar spellings coexist in one condition.

Every `[|`, `|]`, `|>`, `||` surface is untouched: no lexer byte changed in this
card (§5.4), so the sixteen adjacency rows card `OR-AND-LOWERING` pinned in
`crates/qcore-frontend/tests/unit/lexer.rs` are **unedited and passing**, and the
28-test `pipe_operator` suite and the `quant_index_contracts` suite are unedited
and passing too.

---

## 3. Positions: what is admitted, and what every unclaimed position keeps

The brief asked for the positions to be decided honestly, with a named refusal
for each one not claimed. The integer tier lives in **one** position.

| Position | v1 | What it does |
|---|---|---|
| a **parenthesized comparison operand** inside an `if` condition | **admitted** | the whole of the integer tier |
| a whole condition (`if (mask & 4)`) | **refused**, `Q045` | no integer-to-Boolean coercion; the C idiom is deliberately not inherited |
| a `let` initializer, any route (`let n = 3 \| 4;`) | **refused**, `Q038` | follow-on `BITWISE-LET-INITIALIZER` |
| a `return` operand | **refused**, `Q038` | follow-on `BITWISE-LET-INITIALIZER` covers the same missing tier |
| a `std.io` rendering operand (`let total = 42 \| 1;`) | **refused**, `Q038` | follow-on `BITWISE-STD-IO-OPERAND`; card `STD-FMT-BORROW` owns that route |
| an extern call argument | **refused**, `Q038` | the extern route's scalar arguments are `PIPE-SCALAR-RESULT`'s; follow-on `BITWISE-EXTERN-ARGUMENT` |
| a `qcore_check` / `qcore_acquire` argument | **refused**, `Q038` | those take literal outcome controls, not expressions |
| a contract, library or task module | **refused**, `Q038` | the gate runs per file, before route selection |

`a_bitwise_operator_outside_a_condition_keeps_its_refusal` pins five of those
rows as one list, and `the_gate_message_names_the_one_admitted_position` pins
that the message is actionable rather than a bare reservation.

**No custody-bearing value gains a position anywhere in this tier.** The rule is
stated at the position it protects: a name in a condition resolves against the
constant-binding map only, and a name the driver knows as a *resource* binding
is refused with a message that says so (`a_resource_binding_is_not_an_integer`,
pinned through the bitwise tier too, which is the shape that would have put an
identity into an expression for the first time).

---

## 4. The three halves

### 4.1 `BITWISE-OPERATORS`: the integer tier

`qcore_core::cleanup_lowering` gains `BitwiseOp` (three constructors, closed) and
`IntExpr` (`Literal` | `Bitwise`), and `BranchCondition::Compare`'s operands move
from `i32` to `IntExpr`. That is the whole core change; `BranchCondition` loses
`Copy` and nothing else moves.

**Why three and not five.** `|`, `^` and `&` are the three single-byte spellings
card `OR-AND-LOWERING` freed, and they are also exactly the arithmetic-free
operators C defines for **every** pair of operands. A shift is undefined in C for
some operand values and a complement raises the same signedness questions;
neither is an omission, both are decisions with a named follow-on
(`BITWISE-SHIFT-COMPLEMENT`). `the_operators_are_total` states the admitted half
in Lean.

**The emitted C computes the bit operations itself.** Card `IF-STATEMENT` chose
to emit the comparison rather than the decided literal, so that the
reference/CFG/native agreement is a cross-check of the Rust evaluator against a C
compiler instead of a tautology. Folding the operand tree at check time would
have quietly given that up for the new half, so the tree reaches `qcore-core` and
`c_emit` writes it, fully parenthesized:

```c
q_bb_2:
    if ((INT32_C(12) | INT32_C(3)) == INT32_C(15)) { goto q_bb_3; } else { goto q_bb_5; }
```

The parentheses are emitted for every `Bitwise` node and for no `Literal` node,
which is what keeps the grouping this tier declared independent of the C
compiler's own table **and** keeps a program that writes no bit operation
byte-identical (`a_program_that_writes_none_of_this_emits_byte_identical_c`).

### 4.2 `IF-CONNECTIVE-NEGATION`: the arm swap

One line:

```rust
ConditionTree::Not(inner) => build_connective_branch(span, inner, else_output, then_output),
```

`if (!C) { A } else { B }` is the branch a reader writes as `if (C) { B } else { A }`.
`qcore-core` gains no constructor, no checker learns a rule, and the byte-equality
pin is the measurement:

```
qcorec emit -o negated.c fixtures/bitwise/negated_condition.qcore
qcorec emit -o twin.c    fixtures/bitwise/negated_condition_twin.qcore
diff negated.c twin.c     # empty
```

`a_negated_condition_emits_byte_identical_c_to_its_hand_swapped_twin` asserts
`c_source` equality **and** equal reference observations, with `assert_ne!` on the
sources so the equality is a statement about the swap. The two fixtures'
comment blocks are padded so `fn main` lands on the same line and both are
compiled under one file name — the `#line` constraint card `OR-AND-LOWERING`
recorded, and rediscovering it cost one iteration (the first pair was off by
exactly one line, with byte-identical bodies).

The arms are deliberately **distinguishable** — one carries a `qcore_check` the
other does not, which is custody-neutral so the join still admits them — because
of §6 finding 2.

`!!A` is literally `A`'s own lowering (`a_double_negation_is_the_identity_lowering`),
and the emitted entry function contains no `!` at all
(`a_negation_adds_no_operation_to_the_program`).

### 4.3 `IF-CONDITION-BINDINGS`: the decidable subset, and the honest remainder

The brief asked for a sound v1 for runtime conditions if one is writable, and for
the decidable subset plus a named remainder if not. **It is not writable here,
and the measurement says why in one sentence: on the route where `if` lives,
there was no scalar binding at all.**

| Route | Has scalar bindings? | Admits `if`? |
|---|---|---|
| `source.extern-call.v1` | **yes** — `ValueId`, from `PIPE-SCALAR-RESULT`, with `succeeds INTEGER` declared outcomes from `STD-IO-COMPOSE` | **no** — the extern route's nested-block refusal |
| `source.std-io.v1` | **yes** — integer bindings, the operands `Q028` requires | **no** — `Q030` |
| `source.std-io-extern.v1` | yes, both | no, both refusals |
| **`SequentialCleanupV1`** | **no** — its `let` was `qcore_acquire(…)`-only (`Q500`) | **yes** — the only route that does |

So the `ValueId` bindings the brief pointed at are on routes that refuse the
construct, and the construct's own route had nothing to read. Reaching them is
`IF-ROUTE-WIDENING`, which is a whole route's checked model and not this card.

What is writable, and what landed, is the sequential route's own
**check-time constant binding**:

```qcore
let mask = 12;
if ((mask | 3) == 15 && (mask & 4) == 4) { … }
```

`let NAME = INTEGER;` (optionally negated) at the **top level** of a sequential
entry body. It is custody-free by construction — its initializer is a signed
32-bit literal — so it opens no obligation, enters no ledger and takes no
position in any of the four checked models. It is **substituted** for its value
where a condition reads it, exactly as a connective is eliminated before any
model sees it, so `a_constant_binding_is_its_value` pins that the program which
reads the binding and the program which writes the literal out emit the same
bytes and observe the same trace.

Stated plainly, because it is the boundary:

| Real | Not real |
|---|---|
| the name resolves, scopes and shadows like any local (`Q012` on a duplicate, `Q013` before it is bound) | the binding has **no runtime existence** — it is not a C variable |
| the condition tier reads it, in every operand position | nothing else reads it: `drop n`, `move n`, `return n`, `qcore_check(n, …)` and `n = 4` all keep their existing refusal |
| its value is check-time known **by construction**, not by analysis | it cannot carry a call result, so `succeeds INTEGER` is out of reach from here |

`a_constant_binding_gains_no_other_position` pins the second column's first row as
one list. **`IF-RUNTIME-CONDITION` is unmoved** and is still what would make a
condition read a run-time value; this card did not touch the single-observation
certificate story, and could not have, because every operand it admits is
decided.

A `let` **inside a branch arm** is refused at `Q044` with a message naming
`IF-ARMS`, the concurrent card that owns the arm subset. That refusal is
deliberate boundary discipline, not a limitation of the mechanism.

---

## 5. Evidence

### 5.1 Tests

`compiler/tests/bitwise_conditions.rs` — **31 tests, all passing**, including one
sanitized native run. Three new fixtures under `compiler/tests/fixtures/bitwise/`:
`negated_condition.qcore`, its hand-swapped twin, and `masked_condition.qcore`.

**Three pinned expectations moved, each deliberately.**

| Pin | Was | Now | What survived |
|---|---|---|---|
| `branch_statement.rs::the_tier_gained_exactly_the_two_connectives` | `!true`, `true \| false`, `true & false`, `true ^ false` all refuse | renamed `…_the_connectives_the_negation_and_the_integer_operands`; `!true`, `!(3 < 5)` and `(3 \| 4) == 7` admitted, and seven refused shapes listed including the two C-divergence spellings | the closedness of the tier against everything else, and the card's own boundary-marker role |
| `logical_connectives.rs::the_three_single_byte_operators_are_reserved_not_spelled` | `\|`/`&`/`^` refuse at `Q038` with *"is reserved"* naming `BITWISE-OPERATORS` | renamed `…_are_spelled_only_in_a_condition`; same code, same gate, message naming the one admitted position | **the load-bearing half** — outside a condition all three still refuse, at the same code, and none is a logical operator |
| `logical_connectives.rs::a_reserved_character_is_reserved_inside_a_condition_too` | `true \| false` refuses at `Q038` — *"the reservation is not positional"* | renamed `a_single_byte_operator_in_a_condition_is_the_branch_pass_to_decide`; `Q045` instead, with the sort-rule message asserted | **the refusal itself** — `true \| false` is still not a program, because a condition is not an integer |

The second and third moves are the same shape card `OR-AND-LOWERING` made when it
moved card `PIPE-OPERATOR`'s `a_single_ampersand_gains_no_new_meaning`: the pin's
negative assertion was written so a *future* card could not silently claim the
character. This card does not claim it silently — it claims it in one position,
states the other positions' refusals as a list, and re-asserts the load-bearing
half at the new layer.

Unedited and passing: `crates/qcore-frontend/tests/unit/lexer.rs` (all sixteen
adjacency rows), `compiler/tests/pipe_operator.rs`, `compiler/tests/quant_index_contracts.rs`,
`compiler/tests/std_modules.rs`, `compiler/tests/extern_calls.rs`.

### 5.2 Sanitizer evidence — reported honestly

`the_native_run_of_a_masked_condition_matches_its_reference`, x86_64 Linux,
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined
-fno-omit-frame-pointer`, with `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. `masked_condition.qcore`
compiles clean under `-Werror` — including all three constant bit operations and
all three constant comparisons the emitter writes — runs with exit 0, and its
parsed observation equals the reference observation exactly. The leak check is a
**positive** control: the program allocates and releases, inside a branch arm.

**Honest scope**, carried from cards `IF-STATEMENT` and `OR-AND-LOWERING`: the
decided condition means one path executes. The other arms are compiled, linked
and reachable from the emitted `goto` table but are not entered at run time;
their evidence is static. What the run *does* establish, and what is new here, is
that a C compiler computes `|`, `^` and `&` on these operands as the Rust
evaluator does.

### 5.3 Lean

`proof/QCore/Bitwise.lean`, audited by `proof/AuditBitwise.lean`: **39 theorems**,
no new axiom and no placeholder, no `native_decide`. Every theorem depends on
nothing, or only on the allowed `propext` / `Quot.sound`. Built on
`QCore.Connectives` and through it on `QCore.BranchJoin`, reusing that card's own
`Op`, `Seq`, `validate` and interpreter, so the theorems are about the model the
branch card proved. `QCore.lean` imports it, so `AuditAll.lean` includes it in the
full axiom closure. `check_lean_audits.py` moves **85 → 86** audits.

By family:

* **The operators** — `the_operators_are_their_truth_tables` characterizes each
  output bit against Lean's own `BitVec` operations rather than a private
  definition; `the_three_operators_are_distinct` keeps it non-vacuous;
  `the_operators_are_total` states the C-definedness that admitted the family;
  `each_operator_is_commutative_and_associative` and
  `the_operators_do_not_commute_with_each_other` bracket where the ranking is
  load-bearing.
* **Neutrality** — `the_fold_preserves_the_value` is the bridge that lets the
  model work over `BranchJoin.Condition` while the Rust carries the operand tree
  for emission; `the_shipped_tier_evaluates_unchanged` is the "no verdict moves"
  half, and `a_bitwise_operand_has_no_shipped_spelling` is its non-vacuity
  companion. `a_negation_free_tree_lowers_exactly_as_before` proves this
  lowering **is** `Connectives.lower` on a negation-free tree, with
  `a_tree_has_a_previous_spelling_exactly_when_it_is_negation_free` as the iff.
* **The negation** — `the_negation_is_the_arm_swap`,
  `a_negated_leaf_is_its_hand_swapped_twin` (the model counterpart of the
  byte-equality pin), `a_double_negation_is_the_identity_lowering` (`rfl`, so the
  two are the *same term*), `the_negation_obeys_de_morgan` (the swap is the same
  rewrite De Morgan would have made, so the card did not have to choose).
* **Correctness** — `the_lowering_is_the_truth_table`, extended to the negation
  by induction against `BranchJoin`'s interpreter.
* **Fail-closed** — the conditional leak, move-versus-release and `return`-in-arm
  all still refuse through a negation, with
  `refused_negation_shapes_are_exactly_these` as one list and
  `agreeing_arms_are_admitted_through_a_negation` as the positive control.
* **Precedence** — a recursive-descent model with explicit fuel over the
  two-sorted grammar, proving both declared rankings and both refusals, with
  `refused_condition_shapes_are_exactly_these` as one list.
* **The bindings** — `a_bound_name_is_its_value`,
  `a_binding_contributes_no_operation`, `a_resource_name_is_not_an_integer`, and
  `the_custody_refusal_is_about_custody` showing the refusal is not an artefact
  of an empty environment.

**Not proved:** that the Rust parser builds these trees from real source the way
the model says (a tested Rust boundary, per the `ConcurrencyPrelude` /
`PipeDesugaring` / `BranchJoin` / `Connectives` precedent); that a C compiler
computes `|`, `^` and `&` as `BitVec` does (that is what the sanitized native run
cross-checks); and anything about a run-time condition.

### 5.4 Corpus neutrality

**Corpus-neutral by construction.** The whole diff is:

```
crates/qcore-core/src/cleanup_lowering.rs      (BitwiseOp, IntExpr, BranchCondition)
crates/qcore-core/src/cfg.rs                   (borrow the terminator instead of moving it)
crates/qcore-core/src/c_emit.rs                (c_int_expr)
crates/qcore-frontend/src/pipe.rs              (the operator gate, positional for all five)
crates/qcore-frontend/src/lexer.rs             (doc comments only — no byte of behaviour)
crates/qcore-verification/src/driver.rs        (the two-sorted parser, Not, constant bindings)
compiler/tests/bitwise_conditions.rs           (new suite)
compiler/tests/fixtures/bitwise/               (three new fixtures)
compiler/tests/branch_statement.rs             (one deliberate pin update)
compiler/tests/logical_connectives.rs          (two deliberate pin updates)
docs/QCORE_SPEC_r0_3.md                        (§L21, Q038 and Q045 conditions restated)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md        (§G02h table, §G02i grammar and refusals)
docs/design/SOURCE_BRANCH_V1.md                (an addendum)
proof/QCore/Bitwise.lean, proof/AuditBitwise.lean, proof/QCore.lean
```

**No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/qcore/`
or `providers/` is touched.** No CT manifest entry changes and no pinned artifact
is edited. The contract corpus stays **69 modules**, CT stays **33/50**, the
185-unit baseline and its leak count of **272** are untouched, and nothing is
promoted.

`crates/qcore-core/` *is* touched, which the connective card's diff was not, and
the claim is therefore structural rather than incidental-by-absence:
`BranchCondition` is reachable only from `SourceOp::Branch` and
`CfgTerminator::Branch`, both of which exist only on the sequential source route.
No gateway or audit path constructs, reads or matches either. The one place a
corpus effect could hide is the lexer, and **no lexer byte changed** — the only
edits there are doc comments.

### 5.5 CT candidates

None proposed. The negation and the constant bindings add no checked model — both
are eliminated before any model runs — and the integer tier's addition to
`qcore-core` is exercised by this card's own suite rather than by a conformance
case's own artifact. Mapping one would grow the denominator without adding
evidence, which constraint 23 forbids.

---

## 6. Deviations and findings

1. **The decidable subset of `IF-CONDITION-BINDINGS` was forced by a route
   measurement, not chosen** (§4.3). The `ValueId` bindings the brief pointed at
   live on routes that refuse `if`; the route that admits `if` had no scalar
   binding at all. Recorded as the reason the card added one rather than reached
   for theirs.
2. **A negation over two *identical* arms is unobservable.** Found by a failing
   `assert_ne!` non-vacuity check, exactly as card `OR-AND-LOWERING` found its
   associativity finding, and recorded rather than quietly dropped:
   `a_negation_over_identical_arms_is_unobservable` pins that `3 < 5`,
   `!(3 < 5)` and `!(!(!(3 < 5)))` emit identical C over two `drop a` arms, and
   that the swap **is** visible once the arms differ. It is why the byte-equality
   fixtures carry distinguishable arms.
3. **The first precedence witness was vacuous and was replaced.** `1 | 2 & 3`
   evaluates to `3` under both groupings, so the intended non-vacuity assertion
   would have passed for the wrong reason. `4 | 1 & 1` (`5` versus `1`) and
   `4 | 1 ^ 5` (`4` versus `0`) are the witnesses that separate the three levels.
4. **A backtracking parser was rejected on complexity, not on output** (§2.2).
   Both designs accept and refuse the same programs; one is exponential in
   parenthesis depth.
5. **`BranchCondition` lost `Copy`.** The ripple is four call sites and is listed
   in the diff; it is the price of the operand tree reaching the emitter, which
   §4.1 argues is the point.
6. **The census was re-measured, over 381 `.qcore`/`.qc` files** (card
   `OR-AND-LOWERING` counted 369; the denominator grew with the tree and the
   finding did not change). Fifteen files contain one of the three characters:
   six `contract/qcore/` modules where every occurrence is inside a `//` comment
   quoting C, two archived probes under `docs/reports/remaining-v03/probes/`
   that `acceptance_matrix.json` records the standing gate does not replay, five
   pipe fixtures and one compose fixture where the character is part of `|>`,
   and this card's own new fixture. **`^` appears in no file but this card's.**
   The census bounds the reachable effect to zero, and the structural argument
   bounds it independently: this card changes no lexer byte and only *narrows*
   the gate, and a narrowing cannot make a previously accepted file refuse.
7. **The `#line` byte-equality constraint bit again** (§4.2): the first fixture
   pair had byte-identical function bodies and `#line 17` versus `#line 18`.
   Recorded a second time so the next byte-equality pin budgets for it.
8. **`Q044` gained a condition, `Q045`'s tier widened, and no code moved.** The
   `let`-inside-an-arm refusal is "a shape this profile does not model", which is
   `Q044`'s condition verbatim; the rest is "a condition outside the decidable
   tier", which is `Q045`'s. Adding a code for either would give one condition two
   codes. `Q047` onward stays free, and no reservation was spent.

---

## 7. Named follow-ons

| Name | What it is |
|---|---|
| `BITWISE-LET-INITIALIZER` | the integer tier in a `let` initializer and a `return` operand on the sequential route. Refused at `Q038` today; the tier exists now, so this is a position decision rather than a missing mechanism. |
| `BITWISE-STD-IO-OPERAND` | the same in a `std.io` rendering operand and in an extern scalar argument. Two routes, each with its own literal parser; card `STD-FMT-BORROW` owns the first. |
| `BITWISE-EXTERN-ARGUMENT` | *(the extern half of the row above, named separately because `PIPE-SCALAR-RESULT` owns that argument position)* |
| `BITWISE-SHIFT-COMPLEMENT` | `<<`, `>>` and `~`. Deliberately out: C leaves a shift undefined for some operand values, so admitting one needs an operand-range obligation this tier has no way to discharge (§4.1). Neither token exists in the lexer. |
| `IF-CONDITION-BINDINGS-RUNTIME` | a condition reading a binding whose value is **not** check-time known. Its prerequisites are unchanged and are `IF-RUNTIME-CONDITION`'s: a certificate story for a multi-valued observation, plus `IF-ROUTE-WIDENING` to reach a route that has such bindings. |
| `SEQ-SCALAR-BINDING-RUNTIME` | a sequential-route binding with a runtime existence — a C variable — rather than a check-time constant. What would make §4.3's second column's first row real. |
| `IF-RUNTIME-CONDITION` | *(inherited, unmoved)* |
| `IF-ROUTE-WIDENING`, `IF-ARM-RETURN`, `IF-ARM-CALL`, `IF-ARM-ACQUIRE`, `SEQ-STATEMENT-TOKENS` | *(inherited, unmoved)* |

`BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION` and `IF-CONDITION-BINDINGS` are
**closed** by this card, the last of the three in its decidable subset with the
remainder renamed `IF-CONDITION-BINDINGS-RUNTIME` rather than left ambiguous.

---

## 8. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test -p qcore-compiler --test bitwise_conditions` | **31 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test branch_statement` | **32 pass** (one pin update) |
| `cargo test -p qcore-compiler --test logical_connectives` | **25 pass** (two pin updates) |
| `cargo test -p qcore-compiler --test pipe_operator` | **41 pass**, unedited |
| `cargo test --workspace --no-fail-fast` | *(see §8.1)* |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `python3 scripts/check_lean_audits.py` | *(see §8.1)* |
| `python3 -m unittest discover -s scripts/tests` | *(see §8.2)* |
| `reevaluation_probe` / `reevaluation_followup` | *(see §8.1)* |

### 8.1 Numbers

*(filled in at close)*

### 8.2 The one Python failure, and why it is the integrator's

`docs/reviews/r0_3/acceptance_matrix.json` carries derived
`validation.lean_audits` and `validation.lean_audit_files` rows.
`AuditBitwise.lean` makes the composed truth **86**, and that file is outside this
card's write scope, so the tripwire fires:
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`.
That is the tripwire doing its job, not a defect, and it doubles as the
measurement that the base carried 85 audits. §9 lists the reconciliation.

---

## 9. Reconciliations the integrator must make

* **`docs/reviews/r0_3/acceptance_matrix.json`** — outside this card's write
  scope. `validation.lean_audits` composes to **86** and
  `validation.lean_audit_files` gains `AuditBitwise.lean` in sorted position.
  The `PIPE-OPERATOR-O` entry's residual list closes `BITWISE-OPERATORS`; the
  `IF-STATEMENT` / `OR-AND-LOWERING` entries close `IF-CONNECTIVE-NEGATION` and
  `IF-CONDITION-BINDINGS`, the latter with `IF-CONDITION-BINDINGS-RUNTIME` as its
  successor. A new `BITWISE-CONDITIONS-O` entry carrying §7's follow-ons is the
  reconciliation.
* **`AGENTS.md`** — the "Diagnostics namespaces" table needs no new row (no code
  was added or moved); the `Q036`–`Q038` row's meaning is now "the pipe operator
  and the **positional** operator gate", and the ledger's Lean-audit count moves
  **85 → 86**. Spec §L21, which that table summarises, is already updated here.
  Constraint 14 (the keyword census) needs no change: **this card adds no
  keyword.** A standing constraint worth landing, if the integrator agrees it is
  one: *precedence is declared only where it agrees with C; where the tier's
  sorts force a different ranking, the unparenthesized spelling is refused.*
* **`DEVELOPMENT.md`** — one increment paragraph; a draft is §10.

---

## 10. Draft increment paragraph

> The condition/expression tier (BITWISE-CONDITIONS): three named follow-ons
> close at once — `|`, `^` and `&` gain their meaning, unary `!` lands, and a
> condition may read a binding — behind one decision that is the card's spine:
> **a precedence is declared only where it agrees with C, and where this tier's
> two sorts would force a different ranking than C's, the unparenthesized
> spelling is refused rather than silently redefined.** C ranks `|`/`^`/`&`
> *looser* and `!` *tighter* than the comparisons; a tier in which a comparison
> yields a condition and a condition is never an integer operand must rank them
> the other way, so `A | B == C` and `!A < B` are **refused** (`Q045`) while
> `(A | B) == C` and `!(A < B)` are the programs — and every ranking the sorts
> permit (`&` over `^` over `|`, `!` over `&&` over `||`, all left-associative,
> comparison non-associative) is declared and pinned against both the intended
> and the other grouping. The integer tier lives in **one** position, a
> parenthesized comparison operand inside an `if` condition, and every unclaimed
> position keeps a named refusal at the same `Q038` gate, now positional for all
> five operator spellings. Negation is a **pure arm swap** — `if (!C) T else E`
> is `if (C) E else T` — so it costs no operation, `!!A` is literally `A`'s own
> lowering, and the emitted C is byte-identical to the hand-swapped twin's. The
> bindings half is the **decidable subset**, and the reason is measured rather
> than asserted: the `ValueId` scalar bindings of `PIPE-SCALAR-RESULT` and
> `STD-IO-COMPOSE` live on routes that refuse `if`, and the sequential route that
> admits `if` had no scalar binding at all, so the card added a **check-time
> constant binding** (`let NAME = INTEGER;`, top level only, substituted for its
> value) and renamed the remainder `IF-CONDITION-BINDINGS-RUNTIME` rather than
> claim it. No custody-bearing value gains a position anywhere in the tier: a
> resource name in a condition is refused by name, and a `let` inside an arm is
> refused at `Q044` naming the concurrent card that owns the arm subset. The
> operand tree reaches `qcore-core` rather than being folded, so the emitted C
> computes `(INT32_C(12) | INT32_C(3)) == INT32_C(15)` itself and card
> `IF-STATEMENT`'s cross-check of the Rust evaluator against a C compiler extends
> to the new operators, verified by a sanitized ASan/UBSan/LSan run with a
> positive leak control; a program that writes no bit operation emits
> byte-identical C. Three findings, each measured rather than designed: a
> negation over *identical* arms is unobservable (caught by a failing
> non-vacuity assertion), the first precedence witness `1 | 2 & 3` was vacuous
> because both groupings evaluate to 3, and a backtracking two-sort parser was
> rejected for being exponential in parenthesis depth in favour of a single-pass
> two-sorted grammar whose sort mismatches are specific refusals. **No new
> reserved word, no new token, no new diagnostic code and no band move** —
> `Q044` gained a condition it already owned, `Q045`'s tier widened, `Q047`
> onward stays free — and no lexer byte changed. 31 new tests plus three
> deliberate pin updates, 39 audited Lean theorems in a new `QCore.Bitwise`
> built on `QCore.Connectives` and `QCore.BranchJoin`, no new axiom, 85 → 86
> audits. Corpus-neutral by construction: no gateway, audit, contract or
> provider file touched; corpus stays 69 modules, CT 33/50, leak 272, nothing
> promoted. Evidence: `docs/reports/wave20-bitwise-conditions/README.md`.
