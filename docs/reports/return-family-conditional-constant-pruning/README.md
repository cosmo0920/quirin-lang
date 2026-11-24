# RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING: a decided condition prunes the arm C never evaluates

Card RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING (USER DECISION 2026-11-24:
"launch the conditional-access follow-up cards"). The base is `1187c3f6d`,
the tip of RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (rule 74x, approved as
r79). That sits on rules 74w (r77), 74t (r74), 74r (r72) and 74m (r68).
[PREDICTION.md](PREDICTION.md) was committed (`d88297484`) before the
implementation (`a197c18ca`). The card adds gateway rule
[74z](../../agents/gateway.md#constraint-74z), which amends rule 74x, and
`QCore.ReturnFamilyConditionalConstant` with its audit.

**This card relaxes a refusal.** It can move a verdict from `Unknown` to
`Safe`. §2 gives the argument that such a move discharges only bases of an
arm C never evaluates. §3 asks for one user decision, about macros.

## 1. Accepted constant grammar

`constant::constant_condition` (`legacy_return_family_constant.rs`) decides
a condition only when it is, as a whole:

* an integer literal: decimal, octal (`0` and `017`) or hex (`0x…`), with
  the C11 suffixes `u`, `l`, `ll` in either case and order (`ul`, `llu`,
  `ULL`, …; mixed-case `lL` is refused);
* a parenthesised accepted expression;
* a unary `+ - ! ~` of one;
* a binary `* / % + - << >> < > <= >= == != & ^ | &&` or `||` of two, with
  C's precedence and left associativity.

The token stream spells every punctuator as one byte, so `<<`, `<=`, `==`,
`!=`, `&&` and `||` are read from two adjacent tokens. `+ +` and `- -` are
undecided, since they may be an increment. A two-token operator followed by
`=` is undecided, because it is an assignment.

**Evaluation** follows C11 under the readers' LP64 premise (rule 11y refuses
every other target). `int` is 32 bits, and `long` and `long long` are 64.
Signed types are two's complement, as on every Clang target. A literal takes
the first type of its 6.4.4.1p5 list that represents it, so `2147483648` is
`long` and `0x80000000` is `unsigned int`. A binary operator applies the
usual arithmetic conversions (6.3.1.8), so `-1 < 0u` is `0` and `-1L < 0u`
is `1`. A shift promotes each operand alone (6.5.7). Unsigned arithmetic is
modulo `2^width`. Division truncates toward zero (6.5.5p6). `&&` and `||`
need both operands decided, which never decides more than C's short circuit
does.

**Undecided** cases keep both arms, which is rule 74x's reading:

* any name: a formal, a local, an enumeration constant, and a macro that
  expands to a name;
* `sizeof`, a cast (its type name is a name), a call, a float (`1.0` is the
  tokens `1 . 0`), `1e5`, a character constant, a string, `?:`, a comma;
* malformed or unrepresentable literals: `08`, `0b1`, `0x`, and
  `18446744073709551616`, which no listed type represents;
* division or modulo by zero, a signed result its type does not represent
  (`2147483647 + 1`, and `INT_MIN / -1` with its remainder), a shift count
  that is negative or not below the promoted left operand's width
  (`1 << 32`), a negative left operand of a signed shift (`-1 >> 1`, which
  is implementation-defined), and a signed left shift out of range
  (`1 << 31`).

## 2. Change and soundness

* **`legacy_return_family.rs`.** In `arm_roots`, the conditional branch now
  calls the new `conditional_arm_roots`. That function classifies the middle
  arm and the last arm into separate lists, exactly as rule 74x classified
  them, so an arm that is not classified still refuses the expression. It
  pushes only the middle arm's roots when the condition is decided non-zero,
  only the last arm's when it is decided zero, and both when it is
  undecided. The module header gains a rule 74z paragraph, the `arm_roots`
  doc line changes, and two `mod` lines are added at the end: `constant` and
  the new unit tests. No other function changes.
* **`legacy_return_family_constant.rs`** is new. It holds the evaluator.

**Soundness argument (brief item 5).** Rule 74x's result for an expression
is the old tests' bases plus `conditional_roots`' bases. Rule 74z changes
only `conditional_roots`, and only inside a conditional whose condition
`constant_condition` decides. There it drops the roots that `arm_roots`
yields for the arm the decided value does not select. Everything else is
unchanged:

* both arms are still classified (`pruning_keeps_every_refusal`);
* the old tests are untouched and read the untaken arm too, so
  `(0 ? p->next : q)->n` keeps `p`;
* the may-bind refusal (`conditional[index]`) and the condition's own
  reading are unchanged.

So the only obligations the tip can discharge that rule 74x kept are the
dropped bases and their `->` readings (`only_dropped_bases_are_discharged`).
A dropped base is a root only of the arm the decided value does not select.
The evaluator gives the value C computes for that expression, because every
operation it decides is the C11 operation under the stated premise
(`signed_arith_exact`, `unsigned_arith_modular`, the vectors), and every
case C leaves undefined or implementation-defined is undecided. By C11
6.5.15p4, that arm is not evaluated (`only_one_arm_is_evaluated`). So a
dropped base is in no execution (`pruned_covers_every_run`,
`pruning_removes_only_unevaluated_bases`). That makes an `Unknown` ->
`Safe` move possible only when the obligation it discharges is such a base.

**Rule 10z and the `if` routes agree.** A scratch end-to-end probe (not
committed; `private/rfcp/zz_probe_rfcp.rs`, log `private/rfcp/probe-10z.log`)
asked what the other readers do with a constant condition.

| body | status |
|---|---|
| `return (0 ? p : "ab")[0];` (family S refuses the string arm; rule 10z branches) | `Safe` |
| `return (1 ? "ab" : p)[0];` | `Safe` |
| `return (k ? p : "ab")[0];` | `Unknown`, `unknown_alias:p` |
| `if (0) return p->n; return q->n;` | `Safe` |
| `if (1) return q->n; return p->n;` | `Safe` |
| `return 0 ? p->n : q->n;` (family S's old direct tests) | `Unknown`, `unknown_alias:p` |
| `long v = (0 ? p : q)->n; return v;` and `long v = 0 ? p->n : q->n;` (declaration routes) | `Unknown`, `unknown_alias:p` |

Rule 10z's branching and the `if` routes already prune a constant edge, and
this grammar is a subset of what they fold. So family S now agrees with
them on every expression it decides, and no verdict changes because the
readers disagree. Family S's old tests still read the arms of an unaccessed
conditional (`return 0 ? p->n : q->n;`), and the declaration routes do not
prune either. Both are precision gaps the brief kept out of scope, so they
are registered follow-ups (§8).

## 3. Decision requested: macros

The brief lists macros among the forms that never count as decided, and it
asks for `(N ? p : q)` with `N` a macro as a refused fixture. **Family S
reads the preprocessed token stream**, which every legacy route reads
(`legacy_c.rs` feeds Clang's `-E` output to the tokenizer). The reader never
sees a macro's name, only its expansion. So:

* `#define N k` then `(N ? p : q)->n` reads `( k ? p : q )`. That is
  undecided, so it stays `Unknown`. This is pinned.
* `#define N 0` then `(N ? p : q)->n` reads `( 0 ? p : q )`. That is decided,
  so it becomes `Safe`. This is pinned as observed, with the label "macro
  expanding to a literal".

The second case is sound. Translation phase 4 replaces `N` before semantic
analysis, so the analysed translation unit, under the compile command that
scopes the verdict, evaluates `0`, and `p` is never evaluated. Rule 10z and
the `if` routes read the same expansion and prune it too: Clang folds the
condition. But it departs from the brief's wording, and fail-closed does not
settle it, since both readings are sound. The options:

* **A (implemented, recommended).** Read the expansion, because it is the
  program C compiles in this configuration. This matches rule 10z and the
  `if` routes, and it is configuration-scoped like every other verdict.
* **B.** Treat a condition that came from a macro as undecided. That needs a
  provenance side channel from the raw source to the token stream, which the
  parser does not have. It would touch `legacy_c.rs` (the shared `Parser`),
  so it would be a separate card. Until then, B would mean dropping pruning
  for every condition.
* **C.** Withdraw the card.

The integrator or user should choose. Under B, the "macro expanding to a
literal" pin flips to `Unknown`. An enumeration constant is a name in the
preprocessed stream, so it is undecided under every option. This is pinned
(`enum { N = 0 };`).

## 4. Pins

* **Unit** (`tests/unit/legacy_return_family_conditional_constant.rs`, a new
  module of `legacy_return_family`):
  * `rule_74z_const_vectors_hold` reads `constVectors` (41 rows) from
    `proof/QCore/ReturnFamilyConditionalConstant.lean` and checks each
    value against `constant_value`. `const_vectors_hold` proves the same
    rows in Lean.
  * `rule_74z_prune_vectors_hold` reads `pruneVectors` (22 rows). It checks
    the old base against the base's own tests and the base and refusal
    against `scalar_read`. `prune_vectors_hold` proves them in Lean.
  * `a_decided_condition_prunes_the_unevaluated_arm` covers 15 spellings,
    the brief's three admitted fixtures among them, plus the `->` root.
  * `an_undecided_condition_keeps_both_arms` covers 17 spellings and a
    nested conditional whose inner condition is undecided.
  * `pruning_keeps_every_other_obligation` covers refusals in the untaken
    arm, old-test bases in the untaken arm and unaccessed conditionals.
  * `literals_take_their_c11_types` covers literal types and the undefined
    or malformed cases.
* **End to end** (`tests/return_family_untracked_conditional_access/constant_pruning.rs`,
  a module of rule 74x's end-to-end target, whose helpers it reuses; `p` is
  a null local, `q` and `r` are unbound formals). It was first a test target
  of its own, and the first gate run failed on that (§7):
  * `a_decided_condition_prunes_the_unevaluated_arm`: 8 rows, `Safe`, plus
    the macro-to-literal row (§3). The brief's admitted fixtures are
    `(0 ? p : q)->n`, `(1 ? q : p)->n` and `((2 - 2) ? p : q)->n`.
  * `an_undecided_condition_keeps_both_arms`: `(k ? p : q)`, `sizeof(int) -
    4`, `1 / 0`, `(int)0`, `2147483647 + 1`, `1 << 32`, the nested
    `(1 ? (k ? p : r) : q)`, `x - x` over an `int` local, a macro to a name
    and an enumeration constant. All are `Unknown` with `unknown_alias:p`.
  * `pruning_keeps_every_other_obligation`: a null evaluated arm, an old
    test in the untaken arm, and `0 ? p->n : q->n`. All stay `Unknown`.
* **Changed rule 74x pins.** Two rows of 74x's `condVectors` (and its unit
  list), and two rows of its end-to-end test, spelled a constant condition
  to show that rule 74x did not decide it. That reading is now rule 74z's,
  so the rows are respelled with an undecided condition (`n - n`, `1 / 0`,
  `k - k`, `sizeof(int) - 3`). They keep their old expectations, and the
  constant spellings are now this card's pins. Rule 74x's theorems are
  unchanged.

**Checked at the base reading.** `constant_condition` was made to return
`None` for every condition. That is exactly rule 74x's reading
(`undecided_keeps_74x_reading`). The new tests were run
(`private/rfcp/base-fixtures/unit.log`, `e2e.log`). Then the source was
restored byte for byte from a saved copy (`cmp` clean, then `touch`).

* In the unit log, `rule_74z_prune_vectors_hold`,
  `a_decided_condition_prunes_the_unevaluated_arm`,
  `an_undecided_condition_keeps_both_arms` (its decided-outer nested row) and
  `literals_take_their_c11_types` (its decided rows) fail.
  `pruning_keeps_every_other_obligation` and the evaluator's
  `rule_74z_const_vectors_hold` pass.
* In the end-to-end log, `a_decided_condition_prunes_the_unevaluated_arm`
  fails. The two control tests pass.

**Pins:** no contract, lock, submodule or CT pin is touched.

## 5. Lean

`QCore.ReturnFamilyConditionalConstant` and
`proof/AuditReturnFamilyConditionalConstant.lean` have no placeholders and
no project axioms. The audit prints only `propext`, `Classical.choice` and
`Quot.sound`. The module imports rule 74x's `Arm`, `armRoots`, `Group`,
`access`, `groupRoots`, `Root`, `base`, `arrowRoot` and `newRead`.

* **The evaluator.** `Kind`, `Val`, `literalKind` (6.4.4.1p5), `common`
  (6.3.1.8), `convert`, `unary`, `arith`, `shift`, `binary` and `eval` over
  `Expr` (`lit`, `un`, `bin`, `other`). For exactness, `signed_arith_exact`
  shows that a decided signed `+ - * /` is the mathematical result, and
  `unsigned_arith_modular` shows that an unsigned one is reduced modulo
  `2^width`. `div_by_zero_undecided`, `rem_by_zero_undecided`,
  `signed_overflow_undecided`, `shift_out_of_range_undecided` and
  `negative_signed_shift_undecided` cover the undefined cases.
  `undecided_propagates` shows that `other` makes the whole expression
  undecided. `decided_only_when_evaluated` and `undecided_condition` relate
  `eval` to the condition.
* **C11 6.5.15p4.** `runs` gives the roots of each execution, and
  `only_one_arm_is_evaluated` holds.
* **Pruning.** `prunedRoots` is `arm_roots` with `conditional_arm_roots`.
  The theorems are `pruning_removes_only_unevaluated_bases`,
  `pruned_covers_every_run`, `pruned_subset`, `pruned_refuses_iff` and
  `undecided_keeps_74x_reading`.
* **The read.** `prunedRead` comes with `pruning_keeps_every_refusal`,
  `old_base_kept` and `only_dropped_bases_are_discharged`.

## 6. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `1187c3f6d` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `a197c18ca`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset.**
`moved-rows-base-tip.jsonl` is `{"moves": {}}`, and
`changed-rows-base-tip.txt` is empty. So there is no `Unknown` -> `Safe`
move to justify. That matches the prediction (`none` on every route): the
census, rerun at the base, finds no `return` on the route that accesses a
conditional group, and pruning applies only inside one. The base row counts
equal rule 74x's measured tip, as expected, since only reports changed
between them.

**Pinned routes.** These were not run. The inputs they need are still absent
on this host, as for rules 74r, 74t, 74w and 74x:
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`
and `build/cycle19` do not exist. No result is claimed. A move there would
be `Unknown` -> `Safe` at a `return` family S admits, where an accessed
conditional has a decided condition and a possibly-null untracked root in
the arm C does not evaluate.

## 7. Gate

GATE

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING and registers one follow-up:

* **RETURN-FAMILY-UNTAKEN-ARM-DIRECT-BASES** (precision). Family S's old
  tests (the direct tests and rules 74r's, 74t's and 74w's walks) read every
  root of a conditional's arms whatever the condition. So `return 0 ? p->n :
  q->n;` and `(0 ? p->next : q)->n` keep a null `p` as a base. They are
  `Unknown`, while rule 10z and the `if` routes prune the same constant. The
  declaration routes (`long v = (0 ? p : q)->n;`) do not prune either. The
  fix is to prune by token position, at every conditional the reader
  decides, in the old tests and in the may-bind refusal. The proof would
  show that every dropped base belongs to an unevaluated arm.

The macro question (§3) is a decision, not a follow-up. Under option B it
becomes a provenance card.

`status_edits` is empty, since STATUS states no fact this card changes. The
id this card retires is registered by rule 74x's amendments (r79), which sit
on r77's, r74's, r72's and r68's. So `--check` against this tree alone is
refused. A `git archive` of this card's head was extracted into
`private/rfcp/amend-root`, and the r68, r72, r74, r77 and r79 amendments were
applied there in that order. All five exited 0
(`private/rfcp/amend-apply-*.log`). Against that root, `--check` on this
card's file exits 0 (`private/rfcp/amend-check.log`: it would write
`wave-48-part12.md` and the acceptance matrix). Against this tree alone it
exits 1 with "no registered follow-up has this id", as expected
(`private/rfcp/amend-check-tree.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`:
  * the conditional branch of `arm_roots` (two lines) and its doc comment;
  * the new `conditional_arm_roots` before `conditional_group_access`;
  * the module header paragraph after rule 74x's;
  * two `mod` blocks at the end.

  RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL (rule 82) runs from the same
  base in the same file. It edits `accessed_through_arithmetic` or
  `enclosing_group`, which this card does not touch, but its `mod` line and
  header paragraph land in the same places. It may also edit
  `tests/unit/legacy_return_family_untracked_conditional_access.rs`, whose
  `both_arms_of_an_accessed_conditional_are_bases` list this card changes in
  two rows.
* `proof/QCore/ReturnFamilyUntrackedConditionalAccess.lean` (two
  `condVectors` rows) and
  `crates/qcore-c-gateway/tests/return_family_untracked_conditional_access.rs`
  (two rows, a doc comment, and the `mod constant_pruning` line).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74z.md`,
`crates/qcore-c-gateway/src/legacy_return_family_constant.rs`,
`crates/qcore-c-gateway/tests/return_family_untracked_conditional_access/constant_pruning.rs`,
`crates/qcore-c-gateway/tests/unit/legacy_return_family_conditional_constant.rs`,
`proof/QCore/ReturnFamilyConditionalConstant.lean`,
`proof/AuditReturnFamilyConditionalConstant.lean`, and this report with its
scripts and evidence.

## 9. Increment paragraph

The paragraph `apply_amendments.py` prepends to the current increment record
is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json).
