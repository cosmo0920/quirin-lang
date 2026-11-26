# RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS: both arms of an accessed conditional are bases

Card RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (USER DECISION 2026-11-24:
"launch the two arithmetic-access follow-up cards"). The base is
`03aadb221`, the tip of RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC
(rule 74w, approved as r77), on rules 74t (r74), 74r (r72) and 74m (r68).
[PREDICTION.md](PREDICTION.md) was committed (`87eb66d7c`) before the
implementation (`c88cf5357`). The card adds gateway rule
[74x](../../agents/gateway.md#constraint-74x), which extends rule 74w, and
`QCore.ReturnFamilyUntrackedConditionalAccess` with its audit.

## 1. Reachability

**The gap is reachable, and family S is the reader that admits it.**
`scalar_read` refuses only GNU `X ?: B` (`?` then `:`), so a conditional
reaches its untracked branch. Family S is asked inside
`return_expression_instruction` (`legacy_return_routes.rs`), and rule 10n's
`typed_connective_return`, which applies rule 10z's arm lowering to a
`return`, is asked in `legacy_c.rs` only when that call returned `None`. So
rule 10z never sees an expression family S admits. In family S no test read
an arm of the accessed conditional: the direct tests need `->`, `[` or `*`
next to the name, and rules 74r's, 74t's and 74w's walks stop at the `?` or
`:` beside it.

A scratch end-to-end probe (not committed; kept as
`private/rfc/zz_probe_rfc.rs`) ran every row below at the base
(`private/rfc/probe-base.log`) and at the tip (`private/rfc/probe-tip.log`).
The declarations are `k_alloc`/`k_free` and `struct s { long n; long a[2];
struct s *next; }`. `k` is a `long` formal, and `p`, `q` and `r` are null
locals unless the row says otherwise.

| body (`return …;`) | base | tip |
|---|---|---|
| `(0 ? p : q)->n`, `(1 ? p : q)->n`, `(k ? p : q)->n + 1` | **`Safe`** | `Unknown`, `unknown_alias:p`, `unknown_alias:q` |
| `(k ? p : q)[0]`, `*(k ? p : q)` | **`Safe`** | `Unknown`, both |
| `(k ? (k > 1 ? p : r) : q)->n`, `(k ? p : k > 1 ? q : r)->n` | **`Safe`** | `Unknown`, all three |
| `((const struct s *)(k ? p : q))->n`, `(k ? (const struct s *)p : q)->n`, `*(const long *)(k ? p : q)` | **`Safe`** | `Unknown`, both |
| `(k ? p + 1 : q)->n`, `((k ? p : q) + 1)->n` | **`Safe`** | `Unknown`, both |
| `(k ? p : 0)->n` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `(k ? a->next : q)->n` (`a` a formal) | **`Safe`** | `Unknown`, `unknown_alias:q` |
| `(p ? q : r)->n` (`p` only the condition) | **`Safe`** | `Unknown`, `unknown_alias:q`, `unknown_alias:r` |
| `(k ? p : "ab")[0]` (an arm family S cannot classify) | **`Safe`** | `Unknown`, `unknown_alias:p` (rule 10z's `p[0]` edge) |
| `k ? p->n : q->n`, `(k ? p->n : q->n)` | `Unknown`, both | same |
| `(k ? p : q) != 0`; `sizeof((k ? p : q)->n)` | `Safe` | same |
| `(p ? q : r)->n` with `q`, `r` formals; `(k ? p : q)->n` with `p`, `q` formals | `Safe` | same |
| `&(k ? p : q)->n != 0`; tracked `(k ? p : p)->n`; `(k ? (k, p) : q)->n`; a `k_alloc` call arm | `Unknown`, return refusal | same |

## 2. Change and direction

* **`legacy_return_family.rs`.** The untracked branch of `scalar_read`
  keeps every old test and adds one: a root that `conditional_roots`
  returns is a base, and a `->` root when the access is `->`.
  `conditional_roots` runs once per expression, before the scan. It takes
  every parenthesised group whose top level holds a `?`, and
  `conditional_group_access` widens it as rule 74w's walk widens a root: a
  type group before it (`cast_semantics::type_group_open`, the one
  detector), a `(` and `)` around it, a parenthesised arithmetic group
  classified by rule 74w's own `integer_operands`. Unlike rule 74w's walk,
  no additive group needs to be crossed. When `->` or `[` follows, or a `*`
  that may be unary under either of rule 74h's readings precedes
  (`operator_at`), `arm_roots` reads the group's content. The helpers
  `top_level`, `matching_colon` and `loaded_value` and the struct
  `ConditionalRoots` are new. `accessed_through_arithmetic`,
  `accessed_through_groups`, `dereferenced_through_groups`, `stars_before`,
  `operator_at` and the may-bind admission are not touched. The module
  header gains a rule 74x paragraph, and one `mod` line is added at the end.
* **Arms.** After its parentheses, `arm_roots` reads an arm by C's
  precedence. A top-level `?` makes it a conditional: the condition is a
  read, and both arms are classified, the middle one up to its matching `:`.
  Top-level binary `+` and `-` make it a sum: every term must be
  `integer_operands` but at most one, and that one must be classified and
  not subtracted. Then, in order: a loaded value (a unary `*` of an operand,
  or a name or parenthesised group followed by `- > F`, `. F` or `[ … ]`,
  whose roots the old tests read); a cast (a type group before a classified
  operand); a numeric literal or declared scalar (no root); a name the body
  never binds (a root). Anything else returns `false`, and `conditional_roots`
  returns `None`, so family S refuses: a string, a unary `-`, `!` or `~`,
  a product or comparison of the whole arm, a may-bind name, two pointer
  terms, a subtracted pointer, an empty arm. An access through an
  unclassified arithmetic group refuses too. Calls, assignments and commas
  keep family S's own refusals.
* **Both arms, never the condition.** The condition is never decided, even
  when constant: `(0 ? p : q)->n` makes `p` a base though C never evaluates
  it. That is the fail-closed reading. Pruning a constant condition would be
  a precision gain, and it is a registered follow-up (§7).
* **Direction.** Every old base and `->` root is kept (`old_base_kept`,
  `old_arrow_kept`, `names_kept`). The one new refusal is the expression's.
  So a result the tip admits, the base admitted
  (`the_fix_only_refuses_more`).
* **The readings the brief keeps** are unchanged end to end: tracked and
  may-bind roots (refused in an arm or in the condition, as before), unbound
  formals (`Safe`), the address form `&(k ? p : q)->n`, and
  `return k ? p->n : q->n;`. That last one is checked both bare and
  parenthesised: the conditional group is not accessed, and its arms are
  bases through the direct test, as at the base.
* **Rule 74w's walk is unchanged.** When the root is a later term of a sum
  in an arm and an operator or the group's `)` follows it
  (`(c ? 1 + p - n : q)->m`, `(c ? q : n + p)[0]`), rule 74w's walk reads
  the conditional group as the sum around the root, finds `?` among its
  other operands, and refuses. That was the base's reading, and it is kept.
  Admitting it here would admit more than at the base, so it is a
  registered follow-up (§7). `(c ? 1 + p : q)->m`, where `:` follows, is not
  touched by that walk and is now a base.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_untracked_conditional_access.rs`,
  a new module of `legacy_return_family`):
  * `rule_74x_cond_vectors_hold` reads `condVectors` (23 rows) from
    `proof/QCore/ReturnFamilyUntrackedConditionalAccess.lean`. It checks
    each row's old fields against the base's own functions (the direct
    tests, `dereferenced_through_groups`, `accessed_through_groups`,
    `accessed_through_arithmetic`). It checks the new fields against
    `scalar_read`, where a refused row is `None`. `cond_vectors_hold`
    proves the rows in Lean.
  * `both_arms_of_an_accessed_conditional_are_bases` covers 23 spellings
    with both arms bases, plus null-constant, scalar, loaded-arm and
    condition rows and the `->` root checks.
  * `an_unclassifiable_arm_refuses` covers 13 spellings, plus rule 74w's
    two kept refusals.
  * `the_base_readings_are_kept` covers unaccessed conditionals, accessed
    arms, the direct, cast and arithmetic spellings, `&`, `sizeof`, may-bind
    roots and GNU `?:`.
* **End to end** (`tests/return_family_untracked_conditional_access.rs`):
  * `both_null_arms_of_an_accessed_conditional_are_bases`: 15 rows, all
    `Unknown` with their `unknown_alias` keys.
  * `an_unclassifiable_arm_refuses`: the string arm.
  * `the_base_readings_are_kept`: both `k ? p->n : q->n` spellings, the
    comparison, `sizeof`, 3 unbound-formal rows, `&`, the tracked root, and
    the comma and call arms.

**Checked at the base reading.** With the implementation in place,
`conditional_roots` was made to return an empty `ConditionalRoots` at entry.
That is exactly the base's reading, since the old tests are kept. The new
tests were run (`private/rfc/base-fixtures/unit.log`, `e2e.log`). Then the
source was restored byte for byte from a saved copy (`cmp` clean, then
`touch`), and the tip's tests pass again.

* `unit.log`: `rule_74x_cond_vectors_hold`,
  `both_arms_of_an_accessed_conditional_are_bases` and
  `an_unclassifiable_arm_refuses` fail. `the_base_readings_are_kept`
  passes.
* `e2e.log`: the null-arms test fails at its first row with `("Safe", [])`,
  and so does the string-arm test. `the_base_readings_are_kept` passes.

**Pins:** no contract, lock, submodule or CT pin is touched, and no existing
test changed.

## 4. Lean

`QCore.ReturnFamilyUntrackedConditionalAccess` and
`proof/AuditReturnFamilyUntrackedConditionalAccess.lean` have no
placeholders and no project axioms. The audit prints only `propext` and
`Quot.sound`. The module imports rule 74w's `Layer`, `Ctx`, `Outcome` and
`classified`.

* An `Arm` is `root`, `inert`, `loaded`, `paren`, `cast`, `sum classified`,
  `cond (condition : Option Bool)` or `other`. `armRoots` is `arm_roots`.
* `the_condition_is_not_decided` says the reading is the same for every
  condition, constant or not. `both_arms_are_taken` says the roots are the
  union of the arms'. `an_unclassifiable_arm_refuses` covers either side.
* A `Group` is the reader's layers, context and arms (trust boundary 28),
  bound by `condVectors`. Family S's result is modelled as `oldRead` and
  `newRead`, and `the_fix_only_refuses_more` proves that nothing admits more
  than at the base.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `03aadb221` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `c88cf5357`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset**
(`moved-rows-base-tip.jsonl`: `{"moves": {}}`; `changed-rows-base-tip.txt`
is empty). There is no moved row to account for. That matches the
prediction (`none` on every route): the census finds no `return` on the
route whose accessed group holds a top-level `?`, so neither the new bases
nor the new refusal can apply. The base row counts equal rule 74w's
measured tip, as expected, since only reports changed between them.

**Pinned routes.** These were not run. The inputs they need are still absent
on this host, as for rules 74r, 74t and 74w:
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`
and `build/cycle19` do not exist. No move is predicted there. A move would be
`Safe` -> `Unknown` at a `return` family S admits that accesses a conditional
with an untracked, possibly-null arm, or a new return refusal for an
unclassifiable arm.

## 6. Gate

The scoped card gate was run as `python3 scripts/gate_scope.py --base
03aadb221 --out private/rfc/gate-1`, through `private/toolchain/env.sh`
(Clang 21, Lean 4.35), at head `c88cf5357` with a clean tree. It exited 0,
with all 8 steps passed. Later commits change only this report and its
evidence. The record is [gate/gate-1.json](gate/gate-1.json), and the logs
are in this worktree's `private/rfc/gate-1/`. The tip measurement ran
alongside the gate.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 689 units, 734 summaries; 10,039 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 501 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 7. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS and registers two follow-ups:

* **RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING** (precision). A constant
  condition (`(0 ? p : q)->n`) still makes both arms bases, so a null `p`
  that C never evaluates makes the function `Unknown`. Pruning an arm needs
  the condition decided as an integer constant expression (C11 6.6), and the
  brief left that out.
* **RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL** (precision, fail-closed).
  Rule 74w's walk reads a parenthesised conditional as the additive group
  around a later term of an arm's sum, and refuses
  (`(c ? 1 + p - n : q)->m`, `(c ? q : n + p)[0]`). The fix belongs in
  `enclosing_group` or `accessed_through_arithmetic`, which this card left
  alone because its sibling edits the same file. The fix must stop at a
  top-level `?` or `:` of the enclosing group.

`status_edits` is empty: STATUS states no fact this card changes. The id
this card retires is registered by rule 74w's amendments (r77), which sit on
r74's, r72's and r68's. So `--check` against this tree alone is refused. I
extracted a `git archive` of this card's head into `private/rfc/amend-root`
and applied the r68, r72, r74 and r77 amendments there in that order. All
four exited 0 (`private/rfc/amend-apply-*.log`). Against that root, `--check` on this card's file exits 0 (`private/rfc/amend-check.log`); against this tree alone it is refused with "no registered follow-up has this id", as expected.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`:
  * the untracked branch of `scalar_read` (one `if` before the base test and
    one more disjunct in it), and one `let` after `conditional_positions`;
  * new functions between `accessed_through_arithmetic` and `scalar_read`;
  * the module header, and one `mod` line at the end.

  RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT runs from the same base in the
  same file, and so its new functions and `mod` line land in the same
  places. Rules 74s (r71) and 74u (r76) edit this file and
  `cast_semantics.rs`. This card calls `type_group_open` but does not edit
  it. If 74u widens what it reads as a type group, the widening absorbs
  that cast too, which only adds bases.
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74x.md`,
`tests/return_family_untracked_conditional_access.rs`,
`tests/unit/legacy_return_family_untracked_conditional_access.rs`,
`proof/QCore/ReturnFamilyUntrackedConditionalAccess.lean`,
`proof/AuditReturnFamilyUntrackedConditionalAccess.lean`, and this report
with its scripts and evidence.

## 8. Increment paragraph

The paragraph `apply_amendments.py` prepends to the current increment record
is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json):

> **RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS** (USER DECISION 2026-11-24: "launch the two arithmetic-access follow-up cards"; gateway rule 74x, extending rule 74w; `QCore.ReturnFamilyUntrackedConditionalAccess`; [report](../../reports/return-family-untracked-conditional-access/README.md)). Family S of `legacy_return_family` read no root that is an arm of a parenthesised conditional that `->`, `[` or a unary `*` accesses: the direct tests need the access next to the name, and rules 74r's, 74t's and 74w's walks stop at the `?` or `:`. So `long w(void) { const struct s *p = 0, *q = 0; return (0 ? p : q)->n; }`, `(k ? p : q)[0]` and `*(k ? p : q)` were `Safe`, while `return k ? p->n : q->n;` is `Unknown`. Family S is asked before rule 10z's connective return, so it was the admitting reader. The old tests are kept, and `conditional_roots` adds one: it widens each conditional group as rule 74w widens a root (type groups, parentheses, classified additive groups) and, when the group is accessed, makes every untracked, non-scalar root of both arms a base, and a `->` root for `->`. Nested conditionals, casts around the group or an arm, and a classified sum in an arm are read. The condition is a read and is never decided, even when constant. An arm the reader cannot classify (a string, a unary `-`, a product, a may-bind name, two pointer terms) or an access through an unclassified sum refuses the expression. Bases only grow and the one new refusal is the expression's, so nothing admits more than at the base. Tracked and may-bind roots, unbound formals, `&(k ? p : q)->n`, `sizeof`, a comparison and `return k ? p->n : q->n;` read as before. Pinned at unit level (`condVectors`, 23 rows, old fields checked against the base's functions) and end to end (15 null rows, a refused string arm, controls); the change pins fail with the new reading disabled. Opt-in `current --monkey-linked` over Fluent Bit 046accf6: no row moves (1,171 / 2 / 3,623 / 8 at both), as predicted; the census finds no accessed conditional in a `return` on the route. The pinned routes need inputs this host lacks and were not run. Registered: RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING and RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL.
