# OR-AND-LOWERING — `||` and `&&`, and the operator retable (wave 19, phase 2)

Card brief: *"Retable the tokens so `||` is logical or and `&&` logical and,
freeing single `|`, `&` and `^` for future bitwise operations; lower both
connectives onto `source.branch.v1`; bitwise semantics out of scope."*

Base commit: main `5c1f949`.

**What landed.** Both halves, in full. The operator family is retabled — `||`
lexes atomically as logical or, `&&` stays logical and, and all three single
characters are reserved with actionable refusals — and both connectives are
lowered onto card `IF-STATEMENT`'s branch construct as a **pure desugaring**:
`qcore-core` is byte-unchanged, no checker learned a rule, and `a || b` emits
**byte-identical C11** to its hand-lowered `else if` twin. 25 new tests, one new
lexer unit test and one extended, 24 audited Lean theorems in a new module plus
a rewritten operator section in the pipe module, no new axiom, **no new
keyword**, **no new diagnostic code and no band move**.

This report supersedes card `PIPE-OPERATOR`'s operator table §4.1 by explicit
user direction, and it closes that card's open note that bitwise or had no
spelling left.

---

## 1. The retable, BEFORE and AFTER

| Spelling | Card `PIPE-OPERATOR` (wave 18) | **This card** |
|---|---|---|
| `[\|` / `\|]` | `QuantOpen` / `QuantClose`, unchanged | **unchanged** |
| `\|>` | `PipeInto`, landed | **unchanged** |
| `\|\|` | two `Pipe` tokens; refused at `Q038`, message *"QCore spells logical or as a single `\|`, not `\|\|`"* | **`PipePipe`, one token — logical or.** Admitted in an `if` condition and lowered there; `Q038` outside one |
| `\|` | `Pipe`; **claimed as logical or**, refused at `Q038` pending `OR-LOWERING` | **`Pipe`, reserved for bitwise or.** `Q038` everywhere, message redirecting to `\|\|` |
| `&&` | `AndAnd`; claimed as logical and, refused at `Q038` pending `AND-LOWERING` | **`AndAnd` — logical and.** Admitted in a condition; `Q038` outside one. Binds **tighter** than `\|\|` |
| `&` | `Ampersand`; reserved, consumed by nothing, refusing *somewhere downstream* and asserted never at `Q038` | **`Ampersand`, reserved for bitwise and.** Refuses at `Q038` with the reservation stated |
| `^` | **not a token** — `unexpected character '^'`, a lex error | **`Caret`, reserved for bitwise xor.** `Q038` everywhere |

**The load-bearing invariant across the whole move: no program's verdict
changed.** Every byte that was refused before is refused now. Three of the seven
rows moved a *layer* or a *message*; none moved an acceptance. The two rows that
gained acceptance (`||` and `&&`) gained it only in a position that did not
exist when wave 18 wrote its table — card `IF-STATEMENT` landed the `if`
statement in between.

### 1.1 Why the inversion, and what it closes

Wave 18's asymmetry (or is `|`, and is `&&`) was the user's convention at the
time and it was recorded honestly, including its consequence: with `|` taken by
logical or and `&` held for bitwise and, **bitwise or had no symbol left**, and
that card flagged the problem rather than answering it (§4.3). The user's
retable amendment answers it the simple way: make **both** logical operators
double-character, which frees **all three** single characters for one coherent
bitwise family. `|` = bitwise or, `&` = bitwise and, `^` = bitwise xor, all
reserved, none spelled. The open design note is resolved, not deferred.

### 1.2 Census first (the `PIPE-OPERATOR` precedent)

Measured before any edit, over every `.qcore`/`.qc` file in the tree
(`measure/census_operators.txt`, reproducible with `measure/census.sh`):

| Byte sequence | Occurrences in shipped source |
|---|---|
| `[\|` or `\|]` | **none, in any file** — the quant alias is exercised only from Rust test strings, exactly as wave 18 measured |
| `\|>` | the four `compiler/tests/fixtures/pipe/*.qcore` fixtures, and nowhere else |
| `\|\|` | **one**, inside a `//` comment in `contract/qcore/fluent_bit/routes_mask.qcore` quoting C. Comments never reach the lexer |
| `&` | comments in five `contract/qcore/` files, plus two archived probes under `docs/reports/remaining-v03/probes/` (`-> &Buffer`, `-> &[i32]`) that `acceptance_matrix.json` records the standing gate does not replay |
| `^` | **none, in any file** |

So the only shipped tokens the retable could reach are the four pipe fixtures'
`|>`, and `|>` is unmoved by width. That is pinned three ways: the lexer
adjacency table below, `the_pipe_operator_is_untouched_by_the_retable`, and the
28-test `pipe_operator` suite passing unchanged apart from its four deliberately
moved rows.

### 1.3 Adjacency, pinned

`crates/qcore-frontend/tests/unit/lexer.rs`. Two-byte spellings are tried before
every one-byte spelling, so the resolution is longest match at a fixed width of
two, and the five vertical-bar-family spellings are pairwise distinct byte pairs
(or a single byte) — the table order is immaterial.

| Input | Lexes as | Status |
|---|---|---|
| `[\|>` | `QuantOpen` `Greater` | unchanged |
| `[\|>\|]` | `QuantOpen` `Greater` `QuantClose` | unchanged |
| `[\|\|]` | `QuantOpen` `QuantClose` | **new row** — `[` wins, so `\|\|` cannot form inside a quant type |
| `a \|> b` | `PipeInto` | unchanged |
| `a \| > b` | `Pipe` `Greater` | unchanged |
| `a \|\| b` | **`PipePipe`** | **moved** — was `Pipe` `Pipe` |
| `a \|\|\| b` | `PipePipe` `Pipe` | **new row** |
| `a \|\|> b` | `PipePipe` `Greater` | **new row** — never `\|` then `\|>` |
| `\|>\|` | `PipeInto` `Pipe` | unchanged |
| `a [ \| ] b` | `LBracket` `Pipe` `RBracket` | unchanged |
| `a && b` | `AndAnd` | unchanged |
| `a & & b` | `Ampersand` `Ampersand` | unchanged |
| `&&&` | `AndAnd` `Ampersand` | unchanged |
| `&Buffer` | `Ampersand` `Word` | unchanged |
| `a \| b & c ^ d` | `Pipe` `Ampersand` `Caret` | **new test** |
| `^^` | `Caret` `Caret` | **new test** — `^` has no two-byte spelling, so it never absorbs a neighbour |

**The residual ambiguity moves owner.** Wave 18 recorded `a |> b` against a
hypothetical `a | > b` as the one genuine future ambiguity, and told "the
or-operator wave" to live with it. That wave is this one, and it does not have
to: `|| >` is unambiguous. The constraint now belongs to `BITWISE-OPERATORS` —
a bitwise or followed by `>` must be spaced or parenthesized — and it is
recorded there instead.

### 1.4 Every pin moved, and why

Five pinned expectations moved. Each is listed with what it asserted, what it
asserts now, and what survived unchanged.

| Pin | Was | Now | What survived |
|---|---|---|---|
| `lexer.rs::lexes_the_vertical_bar_family_by_longest_match` | `a \|\| b` → `Pipe` `Pipe` | `a \|\| b` → `PipePipe`, plus three new adjacency rows | Every other row byte-identical, both quant rows included |
| `pipe_operator.rs::the_logical_or_operator_refuses_with_its_measured_blocker` | a bare `\|` refuses at `Q038` naming `OR-LOWERING` and the `Q500` blocker | renamed `the_single_bar_is_reserved_for_bitwise_or`; same code, message redirecting to `\|\|` and naming `BITWISE-OPERATORS` | the code (`Q038`) and the refusal |
| `pipe_operator.rs::the_c_family_double_bar_is_redirected_to_the_single_bar_spelling` | `\|\|` refuses, one diagnostic, message *"not `\|\|`"* | renamed `the_double_bar_outside_a_condition_names_the_only_position_it_is_admitted_in`; still one diagnostic (now because `\|\|` is one token rather than because the second bar is consumed), message naming the `if` condition | the code, the refusal, the one-mistake-one-diagnostic property |
| `pipe_operator.rs::the_logical_and_operator_refuses_and_redirects_the_single_ampersand` | `&&` refuses at `Q038` naming `AND-LOWERING` | renamed `the_double_ampersand_outside_a_condition_refuses_the_same_way`; same code, message naming the `if` condition | the code and the refusal |
| `pipe_operator.rs::a_single_ampersand_gains_no_new_meaning` | a lone `&` refuses, and **never at `Q038`** — because a `Q038` would have meant wave 18 had claimed it | the same test name, now asserting `["Q038"]` with a bitwise-reservation message | **the load-bearing half**: a lone `&` still refuses, and it is still not logical and |
| `branch_statement.rs::the_tier_has_no_connectives` | `true && false` and `true \|\| false` in a condition each refuse with `["Q038"]` | renamed `the_tier_gained_exactly_the_two_connectives`: both are admitted, and `\|`, `&`, `^` and `!` are asserted still outside the tier | the closedness of the tier against everything else |

The `a_single_ampersand_gains_no_new_meaning` move is the one worth dwelling on,
because its negative assertion was deliberately load-bearing. Wave 18 wrote it
so that *that card* could not silently claim `&`. This card does not claim it
either — it **states its reservation**, which is a different thing, and states
it at the gate where an actionable message is possible. The set of accepted
programs is unchanged: `&` refused before (downstream, in whichever route
claimed the module) and refuses now (at the gate). That is the same shape of
move card `PIPE-OPERATOR` made for a bare `|` when it went from a lex error to a
`Q038`, and the same shape this card makes for `^`.

---

## 2. The lowering

### 2.1 What it is

A connective in an `if` condition is a **desugaring onto `source.branch.v1`**:

```text
if (A || B) T else E   ==>   if (A) { T } else if (B) { T } else { E }
if (A && B) T else E   ==>   if (A) { if (B) { T } else { E } } else { E }
```

The right-hand sides are programs `IF-STATEMENT` already admitted — an `else if`
chain and a nested `if` — which is why this card adds **no semantic rule**:

* `qcore-core` is **untouched**. `BranchCondition` is byte-unchanged and gains
  no connective constructor. The tree lives in the front end
  (`ConditionTree` in `crates/qcore-verification/src/driver.rs`) and is
  eliminated before any checked model sees it.
* The arms are lowered **once each** and their *operation vectors* are
  duplicated structurally into nested branch nodes. A connective therefore
  contributes no statement, no acquisition and no release of its own —
  measured, not asserted: `a_connective_adds_no_operation_to_the_program`
  counts one release site per nested arm and exactly one `q_acquire` call
  however many connectives the condition carries.
* Every nesting level runs the **same four-model custody join**
  (`Flow::join_branches`, `OwnershipOp::Branch`, `ErrorOp::Branch`, the cleanup
  lowering's own), factored out as `join_branch_arms` so this is literally the
  same code path the single-leaf case runs.

### 2.2 The byte-equality pin

The card's load-bearing measurement, in the form wave 18 used for the pipe:

```
qcorec emit -o or.c   fixtures/connectives/short_circuit_or.qr
qcorec emit -o twin.c fixtures/connectives/short_circuit_or_twin.qr
diff or.c twin.c        # empty
```

`an_or_condition_emits_byte_identical_c_to_its_hand_lowered_twin` asserts
`compile(or).c_source == compile(twin).c_source` **and** equal reference
observations, with `assert_ne!` on the sources so the equality is a statement
about the lowering rather than a restatement.
`an_and_condition_emits_byte_identical_c_to_its_nested_twin` does the same for
`&&` against the nested-`if` twin.

One mechanical detail, recorded rather than hidden: the emitted C carries a
`#line` naming its input, so both members of a byte-equality pair are compiled
under **one** file name, and the two fixtures are padded so `fn main` lands on
the same line. Comparing after stripping `#line` would have been the weaker
pin; padding keeps the comparison total.

The emitted shape, reproducible from the CLI, is exactly card `IF-STATEMENT`
§8.3.1's `else if` chain — which is the point:

```c
q_bb_1:
    if (INT32_C(3) < INT32_C(5)) { goto q_bb_2; } else { goto q_bb_3; }
q_bb_2:
    q_release(&q_res_1, UINT64_C(1));
    goto q_bb_6;
q_bb_3:
    if (INT32_C(2) == INT32_C(2)) { goto q_bb_4; } else { goto q_bb_5; }
...
```

### 2.3 Short-circuit — real, and honestly bounded

Short-circuit here is **structural**: the right operand is the condition of a
branch that sits *inside an arm* of the left operand's branch (the else arm for
`||`, the then arm for `&&`), so the emitted C reaches it on exactly one path.
There is no C-level `||` in the output at all —
`the_right_operand_is_the_condition_of_a_nested_branch` asserts the emitted C
contains neither `||` nor `&&`.

What it is **not** is *observable*. Every leaf of the condition tier is a pure
literal comparison (`BranchJoin.evaluation_is_state_free`), so a strict
evaluator would compute the same answer and no side effect is avoided. Saying so
plainly matters more than claiming a property the tier cannot yet exhibit. The
follow-on that makes the difference observable is `IF-RUNTIME-CONDITION` —
already `IF-STATEMENT`'s named residual, whose prerequisite is a scalar value
tier plus a translation-validation story for a multi-valued observation.

The Lean counterpart is proved rather than assumed:
`the_lowering_is_the_truth_table` shows, by induction over the tree against
`BranchJoin`'s own interpreter, that the nested branch observes what the arm the
ordinary two-valued truth table selects observes.

### 2.4 Precedence: the decision

**Declared: `&&` binds tighter than `||`; both are left-associative;
parentheses group.** The grammar is in spec §L21 and §G02i:

```ebnf
cond     = disjunct, { "||", disjunct } ;
disjunct = primary,  { "&&", primary  } ;
primary  = "(", cond, ")" | leaf ;
```

Pinned by `and_binds_tighter_than_or`, which compares the bare spelling against
the fully parenthesized intended grouping (equal) **and** against the other
grouping (different), so it cannot pass vacuously. The Lean model carries a
recursive-descent parser and proves the same thing on tokens.

**`|>` versus `||`: refused, not ranked.** The decision is that the two are
never given a relative precedence, because `|>` is not in the condition tier at
all. Mechanically: the pipe desugarer records a `|>` only at bracket depth 0, so
one inside a condition is never rewritten and survives into the tokens, where
the tier refuses it at `Q045`. Pinned for `1 |> 2`, `true || 1 |> 2` and
`1 |> 2 || true`. This is fail-closed and it is recorded so a future widening has
to decide the case deliberately rather than inherit it.

**Parenthesized and nested combinations are admitted**, honestly: `(true)`,
`((true))`, `(A || B) && C` and `((A && B) || (C && D))` all check. Note that
this makes `if ((true)) { … }` a program that checks where it previously refused
at `Q045` — a deliberate widening of the tier, pinned by
`parentheses_group_and_nest`.

### 2.5 A finding: homogeneous chains are associative on the nose

The associativity declaration turned out to be **unobservable for a homogeneous
chain**, and this was measured rather than designed. `(A || B) || C` and
`A || (B || C)` lower to the *same* nested branch and emit identical C; likewise
for `&&`. The test that started life as `both_connectives_are_left_associative`
failed its own `assert_ne!` non-vacuity check, and was rewritten as
`a_homogeneous_chain_lowers_the_same_under_either_grouping` recording the fact.

It is proved in Lean as `or_lowers_associatively` / `and_lowers_associatively`
(both `rfl`, for arbitrary sub-trees), with
`the_groupings_are_distinct_trees_that_lower_alike` showing the *trees* are
different so the theorem is about the lowering rather than about the parser.

The declaration is therefore load-bearing exactly where the two connectives
**mix**, which is what `and_binds_tighter_than_or` pins. It is recorded here so
that a future widening — an effectful leaf, or a run-time condition — has to
re-measure it: associativity stops being free the moment a leaf can do
something.

### 2.6 Condition position only

Nothing custody-bearing gains a position and no expression tier appeared. The
gate reads the condition extents **the parser recorded**, so "condition
position" means the parser decided this is a condition, never "an `if` appears
to the left". Outside one, `||` and `&&` refuse at `Q038`:

| Written | Result |
|---|---|
| `let x = true \|\| false;` | `Q038`, message naming the `if` condition |
| `return 1 \|\| 2;` | `Q038` |
| `drop a \|\| b;` | `Q038` |
| `while (true \|\| false) { }` | `Q038` (and `while` keeps its own `Q500` besides) |
| a contract, library or executable module | `Q038` on all three, for both connectives |

Route composition is measured, not assumed: an `if` with a connective on the
`std.io` route keeps that route's own nested-block refusal, byte-for-byte the
same diagnostic vector as an `if` with a plain condition
(`a_connective_on_another_route_keeps_that_routes_refusal`). The
`STD-IO-COMPOSE` boundary is neither widened nor narrowed.

### 2.7 The custody join survives every level

The heart of card `IF-STATEMENT`, re-measured through the new tier. Because each
connective level is an ordinary branch, a connective cannot launder a
disagreement:

| Shape | Result |
|---|---|
| `if (true) { drop a; }` | `Q046` (the conditional leak — the pre-card baseline) |
| `if (true \|\| false) { drop a; }` | **`Q046`** |
| `if (true && false) { drop a; }` | **`Q046`** |
| `if (true \|\| false \|\| true) { drop a; }` | **`Q046`** |
| `if ((true \|\| false) && (2 == 2)) { drop a; }` | **`Q046`** |
| one arm `drop`, the other `move`, through a connective | refused |
| an acquisition inside an arm, through a connective | refused (the pre-existing ownership ledger join, `IF-ARM-ACQUIRE`, **not weakened**) |
| `return` inside an arm, through a connective | `Q044` |

---

## 3. Diagnostics: the band decision

**No new code, no band move, and `Q043` left free.** This is the decision the
brief asked for, and the reasoning is that both halves of the card already had
a code whose condition they *are*:

* **`Q045` widens.** Its condition has always been "a condition outside the
  decidable tier". The tier grew a connective tree over the same leaves; the
  code's condition is unchanged in kind. Adding a code for "a malformed
  connective" would give one condition two codes.
* **`Q038`'s condition is restated**, through the §L21 process, from "a
  claimed-but-unlowered logical operator" to "**an operator token used where
  this revision gives it no meaning**". That is the positional decision the
  operator gate has always made; wave 18's phrasing was a description of the
  *reason* every occurrence refused at the time, not of the condition. The gate
  is a per-file lexical/positional pass that runs before route selection, which
  is why the misplacement refusal belongs there rather than in `Branch`.

`Q043` is deliberately **not** taken. Card `IF-STATEMENT` left `Q039`–`Q043`
free for `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` adjustment A2's "`Q039`
onward" reservation; `UNSAFE-LINKAGE-ARTIFACT` took `Q039`–`Q042`, leaving
`Q043` as that band's remaining cushion. Taking it for a condition that already
has a code would have spent someone else's reservation for nothing.

Spec §L21 and grammars §G02h/§G02i are updated in the same change, with both
restatements attributed and the supersession of wave 18's table recorded in
prose.

---

## 4. Deviations and findings

1. **The associativity declaration is unobservable for homogeneous chains**
   (§2.5). Measured by a failing non-vacuity assertion, recorded rather than
   quietly dropped, and proved in Lean.
2. **Parenthesized conditions are a genuine widening.** `if ((true))` refused at
   `Q045` before this card and checks now. Admitting parentheses is what makes
   the precedence declaration usable, so it is deliberate — but it is a widening
   of the tier, not only of the connectives, and it is pinned as one.
3. **A `|>` inside a condition refuses at `Q045`, not at a pipe code.** The
   desugarer's bracket-depth-0 rule (wave 18 finding 6) means a pipe in a
   condition is never rewritten and reaches the tier as an unknown operator.
   Fail-closed; pinned; and it is what makes the `|>`-versus-`||` precedence
   question moot rather than answered.
4. **A name in a condition still refuses at `Q013`**, before the branch pass
   runs — `IF-STATEMENT` finding 4, inherited unchanged. So `1 < 2 || x` reports
   `Q013`, not `Q045`. Recorded rather than forced.
5. **The `&` pin's negative half moved deliberately** (§1.4). Its load-bearing
   content — a lone `&` refuses and is not logical and — is preserved and
   re-asserted at the new layer.
6. **`^` was promoted from a lex error to a token.** The set of accepted
   programs is unchanged; only the layer and the message move. This is the
   `PIPE-OPERATOR` bare-`|` precedent applied one card later, and it is why `^`
   could be reserved cheaply rather than only named in prose.
7. **The `#line` directive constrains byte-equality pins** (§2.2). Both members
   of a pair must be compiled under one file name and their entry functions must
   start on the same line. Recorded so the next byte-equality pin does not
   rediscover it.
8. **`qcore-core` needed no change at all.** The design that made that true —
   eliminating connectives in the front end rather than adding a core IR node —
   is also what makes the corpus-neutrality claim structural rather than
   incidental.

---

## 5. Evidence

### 5.1 Tests

`compiler/tests/logical_connectives.rs` — **25 tests, all passing**, including
one sanitized native run. Two new fixtures under
`compiler/tests/fixtures/connectives/`: `short_circuit_or.qcore` and its
hand-lowered `else if` twin.

`crates/qcore-frontend/tests/unit/lexer.rs` — one new test
(`lexes_the_three_reserved_bitwise_characters`) and four new rows in the
vertical-bar family test.

Deliberately moved: four tests in `compiler/tests/pipe_operator.rs` and one in
`compiler/tests/branch_statement.rs` (§1.4). `compiler/tests/quant_index_contracts.rs`
is **unchanged and passing**, including its wave-18 pin that a bare `|` parses
and then refuses at `Q038` — the retable does not disturb it.

### 5.2 Sanitizer evidence — reported honestly

`the_native_run_of_a_connective_matches_its_reference`, x86_64 Linux,
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. The or-fixture compiles
clean under `-Werror` — including both constant comparisons the emitter writes —
runs with exit 0, and its parsed observation equals the reference observation
exactly. The leak check is a **positive** control: the program allocates and
releases, and the release it performs is inside a branch arm.

**Honest scope**, carried from card `IF-STATEMENT`: the decided condition means
one path executes. The other arms are compiled, linked and reachable from the
emitted `goto` table but are not entered at run time; their evidence is static —
every model validates them, and the refusal tests in §2.7 pin them.
`IF-RUNTIME-CONDITION` is what would move that boundary.

### 5.3 Lean

Two modules.

**`proof/QCore/Connectives.lean`**, audited by `proof/AuditConnectives.lean`:
**24 theorems**, no new axiom and no placeholder, no `native_decide`. Built on
`QCore.BranchJoin` — reusing that card's own `Op`, `Seq`, `validate` and
interpreter — so the theorems are about the model the branch card proved rather
than about a paraphrase. `QCore.lean` imports it, so `AuditAll.lean` includes it
in the full axiom closure.

* **Identity** — `or_lowers_to_its_hand_written_chain`,
  `and_lowers_to_its_hand_written_nesting`, with `handOr`/`handAnd` defined
  **independently of** `lower` so the theorems have content, plus
  `the_two_connectives_nest_oppositely`.
* **Short-circuit, structurally** —
  `the_right_operand_of_an_or_lives_in_the_else_arm` and its and-twin.
* **Neutrality** — `a_leaf_lowers_to_the_branch_it_already_was`, with
  `a_connective_free_tree_is_a_leaf` as the non-vacuity companion.
* **Correctness** — `the_lowering_is_the_truth_table` (induction over the tree
  against `BranchJoin`'s interpreter: the nested branch observes what strict
  evaluation would select), `only_observes_its_operation`,
  `an_or_is_disjunction`, `an_and_is_conjunction`.
* **Precedence** — a recursive-descent parser model with explicit fuel:
  `and_binds_tighter_than_or`, `parentheses_group`,
  `the_connectives_are_left_associative`, `the_two_groupings_are_different_trees`.
* **The associativity finding** — `or_lowers_associatively`,
  `and_lowers_associatively`, `the_groupings_are_distinct_trees_that_lower_alike`.
* **Fail-closed** — the conditional leak through either connective, move-versus-
  release through one, `refused_connective_shapes_are_exactly_these` as one list
  so a widening must change a theorem, the agreeing-arms positive control, and
  `the_connective_program_is_its_hand_written_twin` stating the point once: the
  two are the **same term**, so every verdict is shared by definition rather
  than by agreement.

**`proof/QCore/PipeDesugaring.lean` §6 and §7 rewritten** for the retable, since
that is where the operator gate's model lives. §6 goes from 5 theorems to 8
(`reserved_and_connective_are_disjoint`,
`a_reserved_character_refuses_everywhere`, the three `…_is_reserved` rows,
`a_connective_outside_a_condition_refuses`,
`a_connective_inside_a_condition_is_admitted`, and a retabled
`the_operator_family_is_exactly_as_declared`); §7 from 5 to 8, adding
`the_or_wins_by_longest_match`, `three_bars_are_or_then_reserved` and
`the_caret_never_absorbs_a_neighbour`, and extending
`the_two_byte_table_is_injective` with the four new `"||"` pairs.
`AuditPipeDesugaring.lean` moves from 60 to 66 checked names.

`check_lean_audits.py` moves **83 → 84** audits.

**Not proved:** that the Rust parser builds these trees from real source the way
the model says (a tested Rust boundary, per the
`ConcurrencyPrelude` / `PipeDesugaring` / `BranchJoin` precedent); that a C
compiler evaluates the emitted comparisons as the model does (that is what the
sanitized native run cross-checks); and anything about bitwise `|`, `&` or `^`,
which are reservations rather than operations.

### 5.4 Corpus neutrality

**Corpus-neutral by construction.** The whole diff is:

```
crates/qcore-frontend/src/lexer.rs            (PipePipe, Caret, the table)
crates/qcore-frontend/src/pipe.rs             (the retabled operator gate)
crates/qcore-frontend/src/module_imports.rs   (one call-site signature)
crates/qcore-verification/src/driver.rs       (ConditionTree, the nested lowering)
crates/qcore-frontend/tests/unit/lexer.rs     (adjacency pins)
compiler/tests/logical_connectives.rs         (new suite)
compiler/tests/fixtures/connectives/          (two new fixtures)
compiler/tests/pipe_operator.rs               (four deliberate pin updates)
compiler/tests/branch_statement.rs            (one deliberate pin update)
docs/QCORE_SPEC_r0_3.md                       (§L21, two condition restatements)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md       (§G02h table, §G02i grammar)
docs/design/SOURCE_BRANCH_V1.md               (an addendum)
proof/QCore/Connectives.lean, proof/AuditConnectives.lean,
proof/QCore/PipeDesugaring.lean, proof/AuditPipeDesugaring.lean, proof/QCore.lean
```

**No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`contract/qcore/` or `providers/` is touched**, and `crates/qcore-core/` is
untouched too. No CT manifest entry changes and no pinned artifact is edited.
The contract corpus stays **69 modules**, CT stays **33/50**, the 185-unit
baseline and its leak count of **272** are untouched, and nothing is promoted.

The one place a corpus effect could hide is the lexer, and the census (§1.2)
bounds it: `||`, `[|`, `|]` and `^` appear in no shipped `.qcore` file, the one
`||` in the tree is inside a `//` comment, and every `&` is in a comment or an
archived probe the standing gate does not replay. `no_shipped_program_changes_meaning`
and `the_quant_index_alias_is_untouched_by_the_retable` are the end-to-end
controls.

### 5.5 CT candidates

None proposed. The connectives add no checked model — they are eliminated before
any model runs — so mapping one would grow the denominator without adding
evidence, which constraint 23 forbids.

---

## 6. Named follow-ons

| Name | What it is |
|---|---|
| `BITWISE-OPERATORS` | the bitwise family this card reserved: `\|` (or), `&` (and), `^` (xor). The real prerequisite is an **integer expression tier** — there is no expression node of any kind in the source language, so there is nothing for a bitwise operator to combine. It also inherits the `\|>`-versus-`\| >` longest-match constraint (§1.3). |
| `IF-CONNECTIVE-NEGATION` | unary `!` in a condition. Refused at `Q045` today. Cheap once decided — `!A` is the branch with its arms swapped — but it is a new token position and a new precedence row, so it is a decision rather than an omission. |
| `IF-RUNTIME-CONDITION` | *(inherited from `IF-STATEMENT`)* what would make short-circuit **observable** rather than only structural (§2.3). |
| `IF-CONDITION-BINDINGS` | *(inherited)* comparing against an integer binding; a name in a condition still refuses at `Q013`. |
| `SEQ-STATEMENT-TOKENS` | *(inherited)* the sequential driver still re-lexes source bytes to read a condition. Inert for connectives — a condition contains no pipe the desugarer would have rewritten — but it is still a second tokenization path. |

---

## 7. Reconciliations the integrator must make

* **`docs/reviews/r0_3/acceptance_matrix.json`** — outside this card's write
  scope. `validation.lean_audits` composes to **84** and
  `validation.lean_audit_files` gains `AuditConnectives.lean` in sorted
  position. Until then `python3 -m unittest discover -s scripts/tests` reports
  one failure,
  `test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`
  — the tripwire doing its job, exactly as waves 18 and 19 each left it before
  their integration follow-ups.
  The `PIPE-OPERATOR-O` entry's `why` text also now describes a superseded
  table: its "single `|` is logical or and `&&` is logical and BY DECISION"
  sentence and its `OR-LOWERING`/`AND-LOWERING` residuals are closed by this
  card. A new `OR-AND-LOWERING-O` entry with `BITWISE-OPERATORS` as the residual
  is the reconciliation.
* **`AGENTS.md`** — the "Diagnostics namespaces" table's `Q036`–`Q038` row
  should read "the pipe operator and the operator gate" rather than "the
  claimed logical-operator tokens", and the ledger's Lean-audit count moves
  **83 → 84**. Spec §L21, which that table summarises, is already updated here.
  Constraint 14 (the keyword census) needs no change: **this card adds no
  keyword.**
* **`DEVELOPMENT.md`** — one increment paragraph; a draft is §9.

---

## 8. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | checked out `d82464a277d0f42703702c4dfd9af6af38595a83` (`release-2.2.2-alpha-11-gd82464a2`), the declared revision |
| `cargo test -p qcore-compiler --test logical_connectives` | **25 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test pipe_operator` | **41 pass** (four pin updates) |
| `cargo test -p qcore-compiler --test branch_statement` | **32 pass** (one pin update) |
| `cargo test -p qcore-compiler --test quant_index_contracts` | pass, **unedited** |
| `cargo test --workspace --no-fail-fast` | **3,098 passed, 0 failed, 8 ignored** across 181 targets, exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `python3 scripts/check_lean_audits.py` | **84/84 pass**; no new axiom, no warning |
| `python3 -m unittest discover -s scripts/tests` | **192 pass, 1 fail** — the derived Lean-audit-count tripwire, §8.2 |
| `reevaluation_probe` / `reevaluation_followup` | **exit 0 / exit 0** |

### 8.1 Numbers

The workspace figure is **+26** against the base commit, which is exactly this
card's own additions: 25 tests in `logical_connectives.rs` plus one new lexer
unit test. The five moved pins (§1.4) are renames and rewrites, not additions or
deletions — `pipe_operator` stays at 41 and `branch_statement` at 32 — so the
delta carries no hidden removal.

**None of constraint 16's four load-sensitive rows tripped**, and no test was
re-run to make it pass. The `third_party/libevent` submodule was uninitialized
in this fresh worktree, as waves 18 and 19 each recorded in the same situation;
the pin was **not** weakened — the gate's own
`git submodule update --init --recursive` was run before the closing suite and
checked out the declared revision.

Two runs of the workspace suite are behind the figure. The first completed with
exit 0 but its output was captured with a `tail`, so only the tail survived; the
second was re-run from the same tree with the whole output kept, and is the one
counted. A third, intermediate attempt was killed by the session harness
part-way through and is not counted at all — reported here rather than quietly
dropped.

### 8.2 The one Python failure, and why it is the integrator's

`docs/reviews/r0_3/acceptance_matrix.json` carries derived
`validation.lean_audits` and `validation.lean_audit_files` rows.
`AuditConnectives.lean` makes the composed truth **84**, and that file is outside
this card's write scope, so the tripwire fires:
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`
asserting `['14.0.0', 83] != ['14.0.0', 84]`. That is the tripwire doing its job,
not a defect, and it doubles as the measurement that the base carried 83 audits.
§7 lists the reconciliation.

---

## 9. Draft increment paragraph

> Logical connectives (OR-AND-LOWERING): the operator family is **retabled by
> user direction** and both connectives are **lowered**. `||` now lexes
> atomically as logical or and `&&` stays logical and — both double-character,
> both short-circuit — while the single characters `|`, `&` and `^` are freed
> and reserved for future bitwise or/and/xor, which **supersedes card
> `PIPE-OPERATOR`'s "or is the single `|`" decision** and closes that card's open
> note that bitwise or had no spelling left. The retable moved five pinned
> expectations, each deliberately and each listed in the report, and the
> load-bearing invariant across all of them is that **no program's verdict
> changed**: every byte that refused before refuses now, with `^` promoted from
> a lex error to a `Q038` reservation on the same precedent that moved a bare
> `|` one card earlier. Census first, as `PIPE-OPERATOR` required: `[|`, `|]`,
> `^` and (outside one `//` comment) `||` appear in **no** shipped `.qcore`
> file, so the only shipped tokens in reach are the four pipe fixtures' `|>`,
> which is unmoved by width; sixteen adjacency rows are pinned, four of them new.
> The lowering is a **pure desugaring** onto `source.branch.v1`: `A || B` is the
> `else if` chain and `A && B` the nested `if` a reader would have written, the
> arms are lowered once and their operation vectors duplicated, so
> **`qcore-core` is byte-unchanged**, no checker learned a rule, and the emitted
> C is **byte-identical** to the hand-lowered twin's on both connectives.
> Short-circuit is structural — the right operand is the condition of a branch
> nested inside an arm of the left's, and the emitted C contains no `||` at all
> — but **not observable**, because every leaf of the tier is a pure literal
> comparison; that is stated rather than claimed, with `IF-RUNTIME-CONDITION`
> named as what would change it. Precedence is **declared**: `&&` binds tighter
> than `||`, both left-associative, parentheses group, and `|>` mixed with a
> connective is **refused (`Q045`) rather than ranked**. One finding, measured by
> a failing non-vacuity assertion and then proved: for a *homogeneous* chain the
> lowering is associative on the nose, so the associativity declaration is
> load-bearing only where the connectives mix. Every nesting level runs card
> `IF-STATEMENT`'s four-model custody join, so the conditional leak still refuses
> at `Q046` through any number of connectives and an arm acquisition still
> refuses at the pre-existing `Q103`. Condition position only: outside an `if`
> condition both connectives refuse at `Q038` on every route and in every module
> mode, and no expression tier appeared. **No new keyword, no new diagnostic code
> and no band move** — `Q045`'s tier widens and `Q038`'s condition is restated
> through the §L21 process, leaving `Q043` free for the native-library band's
> cushion. 25 new tests plus one new lexer unit test, 24 audited Lean theorems in
> a new `QCore.Connectives` built on `QCore.BranchJoin`'s own validator and
> interpreter, `PipeDesugaring`'s operator sections rewritten, no new axiom,
> 83 → 84 audits. Corpus-neutral by construction: no gateway, audit, contract,
> provider — or `qcore-core` — file touched; corpus stays 69 modules, CT 33/50,
> leak 272, nothing promoted. Evidence:
> `docs/reports/wave19-or-and-lowering/README.md`.
