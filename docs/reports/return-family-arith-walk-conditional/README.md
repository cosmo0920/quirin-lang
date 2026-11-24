# RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL: rule 74w's walk stops at a conditional's arm

Card RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL (USER DECISION
2026-11-24: "launch the conditional-access follow-up cards"). The base is
`1187c3f6d`, the tip of RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (rule
74x, approved as r79), on rules 74w (r77), 74t (r74), 74r (r72) and 74m
(r68). [PREDICTION.md](PREDICTION.md) was committed (`639d4c050`) before the
implementation (`d09d2d5d9`). The card adds gateway rule
[82](../../agents/gateway.md#constraint-82), which refines rule 74w at rule
74x's registered follow-up, and `QCore.ReturnFamilyArithWalkConditional`
with its audit. Rule id 82 is the one the brief reserved; the rules README
does not make a 74-family id clearly better.

## 1. The refusal and its reach

At the base, in `( n ? 1 + p - n : a ) - > m` and `( n ? a : n + p ) [ 0 ]`,
rule 74w's `accessed_through_arithmetic` meets a binary `+` before `p` and an
operator or `)` after it. It takes the conditional's parentheses as the
additive group (`enclosing_group`), finds `?` among the other operands
(`integer_operands` rejects it), and returns `Unclassified`, so family S
refuses the expression. Rule 74x's `arm_roots` would read that arm, and it
does read `( n ? 1 + p : a ) - > m`, where `:` follows `p` and the walk does
not cross.

**End to end the refusal costs no verdict on the spellings probed.** Family
S is asked before rule 10z's `typed_connective_return`, and when family S
refuses, rule 10z lowers the conditional by branching and reads each arm
(each arm's sum is then rule 74w's ordinary case). A scratch probe (not
committed; `private/rawc/zz_probe_rawc.rs`) ran 19 return expressions, each
over null locals and over unbound formals, with the new reading disabled
(`private/rawc/probe-base.log`, `probe-base-2.log`) and at the tip
(`probe-tip.log`, `probe-tip-2.log`). The logs are identical: null roots are
`Unknown` with every arm's `unknown_alias` key, and formals are `Safe`, on
both sides. They include `(k ? 1 + p - k : q)->n`, `(k ? q : k + p)[0].n`,
casts around the conditional, a sum around it, nested conditionals, three
conditionals in one sum, and a root only in the condition. So this card is a
precision gain inside family S (the row is read by the family the brief
names, and a later route's lowering is not needed), not a verdict change on
these spellings. The measurement (§5) finds no row on the route either.

## 2. Change and soundness argument

* **`legacy_return_family.rs`, the walk** (the hunk the sibling card may
  meet). `accessed_through_arithmetic` keeps its path, its `classified`
  flag and its old result. For each group it crosses it also computes a
  second classification. A new helper, `arm_around`, returns the arm of a
  conditional group that holds the span: the group holds a top-level `?`, a
  top-level `?` or `:` precedes the span, and the first top-level `?` or `:`
  after the span is not a `?`. The tokens between those delimiters (or the
  group's `)`) are the arm. The group is then classified as rule 74w
  classifies a sum (the same `clean_left`, `clean_right` and
  `integer_operands`, now one closure `term_of`), with the arm's bounds in
  place of the group's parentheses. Two flags record whether some crossing
  was an arm (`delimited`) and whether every crossing is classified under
  that reading (`classified_in_arms`). When the old result is
  `Unclassified` and both flags hold, the walk returns a new outcome,
  `ArithmeticAccess::Arm(arrow)`, where `arrow` says `->` accesses.
* **`scalar_read`.** `Arm(_)` refuses, exactly as `Unclassified` did,
  unless rule 74x's `conditional_roots` put this root's index among its
  bases. When it did, the root is a base, and `Arm(true)` makes it a `->`
  root too (rule 74x's own `->` reading gives the same). `conditional_roots`
  maps `Arm(_)` to its refusal, like `Unclassified`. Its own walk,
  `conditional_group_access`, never returns `Arm`.
* **The only refusal removed** is rule 74w's arm-delimited `Unclassified`,
  and only for a root rule 74x made a base. So a row moves from `Unknown`
  to `Safe` only when:
  1. rule 74x accepted every accessed conditional in the expression, which
     needs every arm of each one classified, with at most one non-integer
     term per sum, not subtracted;
  2. every untracked, non-scalar root of those arms is a base, this one
     included;
  3. the flow check discharged every base and every `->` root's
     C02-P-ALIAS obligation.

  No access is left unobligated: the accessed pointer is the arm's one
  pointer term, a base. Every other operand of the arm is an integer by
  `integer_operands`, and the other arm's roots are bases too.
* **Everything else stays unclassified.** No arm reading exists for a root
  in a condition, also of a nested conditional (`(1 + p - n ? a : b)->m`,
  `(n ? n > 1 + p - n ? a : b : b)[0]`), or for a group with no top-level
  `?`. These cases also keep the refusal:
  * an arm whose other operands are not integers, or not additive:
    `(n ? 1 + p - a : b)->m`, `(n ? 1 + p / 2 : a)->m`,
    `(n ? 1 + p == a : b)[0]`;
  * an unclassified group before or after the conditional:
    `((n ? 1 + p - n : a) - b)->m`;
  * a subtracted pointer `(n ? n - p + 1 : a)->m` and a may-bind arm:
    rule 74w's arm reading accepts both, but rule 74x refuses them, so `p`
    is no rule 74x base;
  * a conditional nested inside an arm's sum,
    `(n ? 1 + (n ? 1 + p - n : a) - n : b)->m`: rule 74x's
    `conditional_group_access` has the same reading of the outer
    conditional as the inner one's additive group, and refuses (registered
    follow-up, §7);
  * `?:` in a call's argument, which keeps family S's call refusal.
* **Direction.** No refusal is added (`no_new_refusal`), and every old base
  and `->` root is kept. Where the base admitted the expression's shape, the
  tip admits only what the base admitted. Nothing admits more than a reading
  in which both rules read the arm: rule 74w classifying the arm's term as an
  access of the root, and rule 74x making the arm's roots bases.
* **Rule text.** Rule 74x's sentence that rule 74w's walk "is unchanged"
  and refuses these spellings now points to rule 82 (a minimal edit of
  `docs/agents/rules/74x.md`; one current authority per rule).

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_arith_walk_conditional.rs`, a
  new module of `legacy_return_family`):
  * `rule_82_walk_vectors_hold` reads `walkVectors` (17 rows) from the Lean
    module. It checks each row's old refusal and arm fields against
    `accessed_through_arithmetic` (`Unclassified` or `Arm`, and `Arm`), and
    its last three fields against `scalar_read`. `walk_vectors_hold` proves
    the rows in Lean.
  * `an_arm_sum_term_is_a_base` covers 13 spellings and the `->` root
    checks.
  * `the_other_refusals_stand` covers 20 adversarial spellings: condition
    roots, two pointer terms, subtracted pointers, non-additive operands,
    may-bind names in an arm, nested conditionals inside a sum, unclassified
    groups around the conditional, `?:` in call arguments, and GNU `?:`.
  * `the_base_readings_are_kept` covers unaccessed conditionals, the `:`
    case rule 74x already read, a direct sum, an index, `&` and `sizeof`.
* **The two 74x pins, changed deliberately**
  (`tests/unit/legacy_return_family_untracked_conditional_access.rs`,
  `an_unclassifiable_arm_refuses`). `( n ? 1 + p - n : a ) - > m` and
  `( n ? a : n + p ) [ 0 ]` asserted the base's refusal, as the registered
  follow-up described. They now assert bases `a` and `p`. The reason is
  that rule 82 classifies the conditional group within the arm, and rule
  74x reads that arm. The comment there says so.
* **End to end** (`tests/return_family_untracked_conditional_access/arith_walk_conditional.rs`,
  a module of rule 74x's `return_family_untracked_conditional_access` test
  target, for the reason in §6):
  * `null_roots_of_an_arm_sum_are_bases`: 5 rows `Unknown` with every arm's
    `unknown_alias` key.
  * `unbound_formals_in_both_arms_are_safe`: 3 rows `Safe`.
  * `the_other_refusals_stand`: a condition root, a tracked (may-bind) arm,
    a nested conditional inside a sum, and `?:` in a call argument, each
    with its full key set.

**Checked at the base reading.** With the implementation in place, the
guard returning `Arm` was disabled (`if false && …`). That is exactly the
base's walk, and it leaves `Arm` unreachable. The tests were then run
(`private/rawc/base-fixtures/unit.log`, `e2e.log`; the end-to-end rows then
lived in their own target, `tests/return_family_arith_walk_conditional.rs`,
with the same content). Afterwards the source
was restored byte for byte from a saved copy (`cmp` clean, then `touch`).

* `unit.log`: the change pins fail. These are `rule_82_walk_vectors_hold`,
  `an_arm_sum_term_is_a_base`, and rule 74x's `an_unclassifiable_arm_refuses`
  with its two changed pins. The other 34 `return_family` unit tests pass,
  `the_other_refusals_stand` and `the_base_readings_are_kept` among them.
* `e2e.log`: **all three end-to-end tests pass at the base reading too.**
  They are not change pins. As §1 says, rule 10z's lowering read these
  spellings at the base. They pin that the relaxation keeps every
  end-to-end verdict and key set, and the file's header says so. The brief
  asked for e2e tests that fail at base. None exists on the spellings
  probed, so the change is pinned at unit level.

No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyArithWalkConditional` and
`proof/AuditReturnFamilyArithWalkConditional.lean` have no placeholders and
no project axioms. The module imports rule 74w's `Layer`, `Ctx`, `Outcome`
and `arith`.

* A `Crossing` is one crossed group: its classification as a whole group,
  and `arm : Option Bool`, its classification within the arm when a
  conditional's arm holds the span. `walkOf` is the tip's walk. It returns
  `.arm` exactly when `arith` is `unclassified` and the crossings are
  `delimited` and `inArms`. Otherwise it returns `.old (arith …)`.
* A `Root` carries its crossings, its context, the unchanged direct
  readings, and rule 74x's verdict on it (`condBase`, `condArrow`). The
  model defines three readings: the base's (`oldRefuses`, `oldBase`,
  `oldArrow`), the tip's (`newRefuses`, `newBase`, `newArrow`), and
  both-rules-read-the-arm (`bothRefuses`, `bothBase`, `bothArrow`).
* `the_only_refusal_removed`: if the base refuses a root and the tip does
  not, the walk was `.arm` and the root is a rule 74x base.
  `an_arm_was_unclassified` says `.arm` replaces only an `unclassified`.
* `a_condition_keeps_its_refusal` and `another_group_keeps_its_refusal`: a
  crossing with no arm reading, or one unclassified within its arm, rules
  out `.arm`.
* The next four are fail-closed:
  * `no_new_refusal`, `old_base_kept` and `old_arrow_kept`: no refusal is
    added, and the base's bases and `->` roots are kept.
  * `where_the_base_admitted_nothing_admits_more`: where the base admitted
    the expression's shape, the tip admits only what the base admitted.
  * `a_relaxed_root_is_discharged`: wherever the tip admits an expression
    the base refused for a root, that root is a rule 74x base and its
    obligation discharged.
  * `nothing_admits_more_than_both`: the tip admits nothing that the reading
    with both rules reading the arm would not admit.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `1187c3f6d` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `d09d2d5d9`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset**
(`moved-rows-base-tip.jsonl`: `{"moves": {}}`; `changed-rows-base-tip.txt`
is empty). So there is no `Unknown` -> `Safe` move to justify. That matches
the prediction (`none` on every route): the census, a superset of this
card's reach, finds no `return` on the route with an accessed conditional
group. The base row counts equal rule 74x's measured tip, as expected. The
later commit `c7eb896f1` only moves the end-to-end rows into another test
target and adds evidence; the measured source is unchanged.

**Pinned routes.** These were not run. The inputs they need are still absent
on this host, as for rules 74r, 74t, 74w and 74x:
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`
and `build/cycle19` do not exist. No move is predicted there. A move would be
`Unknown` -> `Safe` (or `Unsafe`) at a `return` family S refused only
because rule 74w's walk read a conditional as an additive group, and whose
later routes did not read it.

## 6. Gate

The scoped card gate ran as `python3 scripts/gate_scope.py --base 1187c3f6d
--out <dir>` through `private/toolchain/env.sh` (Clang 21, Lean 4.35). No
provider file changed, so the provider gate was not selected.

**Attempt 1** ran at head `d09d2d5d9` with a clean tree, alongside both
measurement runs. Its record is [gate/gate-1.json](gate/gate-1.json), and
its logs are in `private/rawc/gate-1/`. It **exited 1**. Every step passed
except the Python suite, which ran 807 tests with 2 failures:

* `test_gate_scope_census.test_the_census_covers_the_suite`: "671 not
  greater than 671.4". The frozen read census observes 671 of the base's
  745 root-workspace units (90.07%), and this card's new end-to-end test
  target made 746. The failure comes from the new target, not from the
  change. The census is another card's frozen evidence and is not
  re-recorded here. The card instead compiled its end-to-end rows as a
  module of rule 74x's existing target (`c7eb896f1`). That follow-up is
  registered (§7).
* `test_gate_slot_lane.test_a_short_job_overtakes_queued_audits_and_a_draining_suite`
  (`['short/1'] != ['short/0']`). This is a timing-dependent scheduling
  test, run under the load of two route measurements. Run alone afterwards
  (`python3 -m unittest discover -s scripts/tests -p
  test_gate_slot_lane.py`), its 37 tests pass. It passed in attempt 2.

**Attempt 2** ran at head `c7eb896f1` with a clean tree. Its record is
[gate/gate-2.json](gate/gate-2.json), and its logs are in
`private/rawc/gate-2/`. It **exited 0**, with every step passed. Later
commits change only this report, its amendments and gate records.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 689 units, 734 summaries; 10,046 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 502 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

## 7. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL and registers two follow-ups:

* **RETURN-FAMILY-CONDITIONAL-GROUP-WALK-NESTED** (precision, fail-closed).
  Rule 74x's `conditional_group_access` reads an outer conditional as the
  additive group around an inner conditional group that is a later term of
  an arm's sum, and refuses (`(c ? 1 + (d ? p : a) - n : b)->m`,
  `(c ? 1 + (d ? 1 + p - n : a) - n : b)->m`). That is rule 82's gap one
  walk over. End to end, rule 10z's lowering reads these at the base
  (`Unknown` over null roots, report §3). This card left that walk alone,
  because the brief scoped the fix to rule 74w's walk and the sibling card
  edits rule 74x's code.
* **GATE-SCOPE-READ-CENSUS-COVERAGE-EDGE** (gate infrastructure). The read
  census observes 671 of the base's 745 root-workspace test units, just over
  `test_the_census_covers_the_suite`'s 90% bound, so any card adding a test
  target fails the Python suite (§6). Sibling cards from this base that add
  a target will meet it. Recording the census again belongs to a landing.

`status_edits` is empty: STATUS states no fact this card changes. The id
this card retires is registered by rule 74x's amendments (r79), which sit
on r77's, r74's, r72's and r68's. So `--check` against this tree alone is
refused ("no registered follow-up has this id",
`private/rawc/amend-check-tree.log`). I extracted a `git archive` of this
card's head into `private/rawc/amend-root` and applied the r68, r72, r74,
r77 and r79 amendments there in that order
(`return-family-deref-resolved-cast`, `-untracked-deref-cast`,
`-untracked-member-cast`, `-untracked-arith-access`,
`-untracked-conditional-access`). All five exited 0
(`private/rawc/amend-apply-*.log`). Against that root, `--check` on this
card's file exits 0 (`private/rawc/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`:
  * the module header: one paragraph after rule 74x's;
  * `enum ArithmeticAccess`: one new variant `Arm(bool)` after
    `Unclassified`;
  * a new function `arm_around` between `enclosing_group` and
    `accessed_through_arithmetic`;
  * `accessed_through_arithmetic`: two `let mut` flags after `classified`;
    the classification block after `enclosing_group` (the old
    `clean_left`/`clean_right`/`integer_operands` test becomes the closure
    `term_of`, called for the group and for the arm); one match arm before
    `(true, false) => ArithmeticAccess::Unclassified`;
  * `conditional_roots`: `Unclassified` becomes
    `Unclassified | Arm(_)` in its match;
  * `scalar_read`: the `match arithmetic` block (one guarded `Arm(_)` arm,
    `Arm(true)` with `Arrow`, `Arm(false)` with the no-op arm);
  * one `mod` line at the end.

  RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING (rule 74z) runs from the same
  base in the same file. If it prunes an arm in `arm_roots`, a root of a
  pruned arm is no rule 74x base, so rule 82's `Arm` keeps its refusal there
  (fail-closed). A merge should keep the guard `!conditional_arms.bases
  .contains(&index)`.
* `crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_conditional_access.rs`
  (the two pins, at the end of `an_unclassifiable_arm_refuses`),
  `crates/qcore-c-gateway/tests/return_family_untracked_conditional_access.rs`
  (one `mod` line at the end) and
  `docs/agents/rules/74x.md` (one sentence).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/82.md`,
`tests/return_family_untracked_conditional_access/arith_walk_conditional.rs`
(with one `mod` line at the end of
`tests/return_family_untracked_conditional_access.rs`),
`tests/unit/legacy_return_family_arith_walk_conditional.rs`,
`proof/QCore/ReturnFamilyArithWalkConditional.lean`,
`proof/AuditReturnFamilyArithWalkConditional.lean`, and this report with its
scripts and evidence.

## 8. Increment paragraph

The paragraph `apply_amendments.py` prepends to the current increment record
is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json):

> **RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL** (USER DECISION 2026-11-24: "launch the conditional-access follow-up cards"; gateway rule 82, refining rule 74w at rule 74x's follow-up; `QCore.ReturnFamilyArithWalkConditional`; [report](../../reports/return-family-arith-walk-conditional/README.md)). Rule 74w's walk read a parenthesised conditional as the additive group around a later term of an arm's sum: in `(c ? 1 + p - n : a)->m` and `(c ? a : n + p)[0]` it crossed the conditional's parentheses, found `?` among the other operands and returned `Unclassified`, so family S refused although rule 74x reads that arm. The walk keeps its path and its old result, and also classifies each crossed conditional group within the arm that holds the span (`arm_around`: the arm's `?` or `:` stand in for the parentheses; no arm reading for a root in a condition). When that is the only unclassified reading it returns `ArithmeticAccess::Arm`, and family S refuses as before unless rule 74x made the root a base. So the one refusal removed is the arm delimiter's, and a row can move to `Safe` only when rule 74x accepted every accessed conditional and every root of every arm is a discharged base. A root in a condition, a non-additive operand, another pointer term, a subtracted pointer, a may-bind arm, an unclassified group around the conditional, a conditional nested in an arm's sum and `?:` in a call argument still refuse. Lean proves that the only refusal removed is that one and its root is a rule 74x base, that no refusal is added, and that nothing admits more than with both rules reading the arm. Pinned at unit level (`walkVectors`, 17 rows; 13 base and 20 adversarial spellings); rule 74x's two refusal pins for these spellings were changed deliberately; the change pins fail with the new reading disabled. End to end the relaxation changes no verdict on 19 probed spellings, because rule 10z's connective lowering read them at the base; the 12 end-to-end rows (null roots `Unknown`, unbound formals `Safe`, adversarial controls) pass on both sides and live in rule 74x's test target, since the gate's read census sits at its 90% coverage edge. Opt-in `current --monkey-linked` over Fluent Bit 046accf6: no row moves (1,171 / 2 / 3,623 / 8 at both), as predicted. The pinned routes need inputs this host lacks and were not run. Registered: RETURN-FAMILY-CONDITIONAL-GROUP-WALK-NESTED and GATE-SCOPE-READ-CENSUS-COVERAGE-EDGE.
