# SCALAR-ROUTE-EXIT-EDGE (wave 37, first arbiter)

**Headline.** The pre-existing scalar/value route really does delete a loop
header's **exit** edge, and the consequence is a false `Safe` — measured, in
both arms, in all three loop kinds, and in one case (`volatile`) against
perfectly well-defined C rather than merely against an unverified premise. The
card **restricts** the route: a loop header's exit edge is never removed,
whatever the modelled value says. On the pinned 185-unit corpus the hazard is
**unreached** — 0 of 201 loop-header `BranchOnNull` nodes take either deciding
arm — so the re-pin moves **nothing**: AFTER is byte-identical to BEFORE and to
the wave-36 baseline in both verdict-bearing artifacts, 0 leaves added, 0
removed, 0 unit moves, 0 function moves, leak pin **272 unmoved**.

The card therefore **retires `CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`** and
registers five successors, including the sibling back-edge arm it deliberately
did not fold in.

---

## 1. The finding

`refine_pointer_null` (`crates/qcore-c-gateway/src/legacy_c.rs`) answers
`PointerRefinement::Infeasible` for a branch edge whose null assumption
contradicts the abstract state, and the custody work list then does not
propagate that successor. It is the **only** place in that file where a
successor is dropped. The route is original Cycle-19 code (one commit,
`696944d7`), carried by no card, no Lean and no report — the fact
`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED` recorded.

Card `C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) pinned it "conservative on all
four probed shapes" in
`crates/qcore-c-gateway/tests/constant_do_while.rs`'s
`a_modeled_scalar_condition_is_a_separate_pre_existing_route`. Every one of
those four rows spells the header **positively** (`while (flag)`) and asserts on
`double_releases()`. Both halves of that are load-bearing:

* a positive header with a known-null binding loses its **body** edge, so only
  the back edge was ever measured; and
* `double_releases()` is a proxy that a deleted **exit** edge does not move,
  because a deleted exit edge suppresses *every* downstream diagnostic at once.

Inverting the polarity — `while (!flag)` or `while (flag == 0)` with
`int flag = 0;` — makes the same arm refuse the **exit** edge instead. The
function's exit node then has no incoming state, the work list's
`let Some(state) = incoming[node_id] else { continue }` skips it, `report_leaks`
never runs there, and `classify_diagnostics` returns `Safe`.

### 1.1 What was measured, before any change

Run through the real gateway, `void *p = q_alloc();` live **after** the loop:

| subject | status | leaks |
|---|---|---:|
| `while (flag) { }` *(wave 35's polarity)* | `Unsafe` | 1 |
| `while (!flag) { }` | **`Safe`** | **0** |
| `while (flag == 0) { }` | **`Safe`** | **0** |
| `do { } while (!flag);` | **`Safe`** | **0** |
| `for (; !flag; ) { }` | **`Safe`** | **0** |
| `while (p) { }`, `p` known non-null *(second arm)* | **`Safe`** | **0** |
| control: no loop at all | `Unsafe` | 1 |

Two further rows say how bad the consequence is and how it is not merely
undefended:

* `while (!flag) { } q_free(p); q_free(p);` → **`Safe`**. A deleted exit edge
  swallows a genuine **double release** after the loop, not only a leak. The
  exit node is never visited, so nothing after the loop is judged at all.
* `volatile int flag = 0; while (!flag) { }` → **`Safe`**. A `volatile` object
  may be modified by means outside the program (C11 §5.1.2.3), so this loop
  **terminates** in well-defined C and the code after it runs. Here the route's
  must-fact is not unproven, it is **false**.

The body-write arm, which is what the old rule's correctness rested on, is
sound and was simply never driven: a body that writes the scalar takes
`ForgetScalar`, which *removes* the binding, and `AbstractState::join`'s
`(Some, None)` arm turns that into `unknown`, so the header reaches
`PointerRefinement::Unknown` on the second iterate and propagates both edges.
Writes through `&flag`, through an aliased `int *`, and via an uncontracted call
all reach it too, as do `break`, `goto` and `return`, each of which gives the
continuation its own incoming edge.

---

## 2. Polarity/arm census over the pinned corpus

Measured by a read-only pass over the **settled** incoming states — the same
discipline `require_live_borrowed_actuals` uses, because the arm is reached on
intermediate iterates and a census taken inside the work list would depend on
the work-list schedule (constraint 6). The census binary is the **base**
analyzer (`measure/scalar-route-exit-edge/unrestrict.py` undoes the card's
restriction, keeping only the structural field the census reads a role off), so
the table describes the corpus as it stood before this card. Control: that
binary's own audit reproduces the baseline's two verdict-bearing artifacts
**byte for byte**, which is the evidence that the census is read-only and the
unrestrict patch faithful.

```
census rows (BranchOnNull edges, settled states): 17,588
distinct BranchOnNull nodes:                       4,240
distinct functions with a BranchOnNull:            1,691
distinct LOOP HEADER BranchOnNull nodes:             201
```

A function is analysed more than once — W4 validates a summary by re-analysis —
so edge rows exceed twice the node count; the node counts are deduplicated by
`(function, node)`.

**Loop-header edges, by polarity, role, arm and result.** This is the table the
brief asks for.

| `true_is_null` | role | arm | result | rows |
|---|---|---|---|---:|
| false | body | `untracked` | refined | 332 |
| false | **exit** | `untracked` | refined | 332 |
| false | body | `unknown_or_multi` | unknown | 43 |
| false | **exit** | `unknown_or_multi` | unknown | 43 |
| **true** | body | `untracked` | refined | 37 |
| **true** | **exit** | `untracked` | refined | 37 |

**Loop-header edges the value route calls `Infeasible`: 0.** Of which exit
edges: **0**. Of which body edges: **0**.

Three things in that table are worth saying out loud.

1. **Inverted headers do occur** — 37 loop-header edge rows spell
   `true_is_null = true`. The hazard's *syntax* is present in the corpus; what
   is absent is the hazard's *binding*.
2. **The reason the hazard is unreached is the `untracked` arm**, which is
   369 of 412 loop-header edge rows per role — **89.6%**. `untracked` is
   `refine_pointer_null`'s first early return: the deciding name is not in
   `state.pointers` at all, because "the reduced importer has no scalar type
   environment", so an ordinary integer loop condition is left alone. The
   remaining 10.4% are `unknown_or_multi`, which propagates both edges by
   construction. **Neither deciding arm is reached at a loop header anywhere in
   the corpus.**
3. **The route is not idle** — it is simply never decisive at a *loop header*.
   At `if` nodes it prunes **154** edges (24 through the `empty_targets` arm,
   130 through `known_non_null`). That is real path sensitivity this card
   deliberately preserves, and it is why the restriction is structural rather
   than a blanket "never return `Infeasible`".

---

## 3. The pruning-disabled verdict diff

The decisive measurement. The card's restriction **is** "exit-edge pruning
disabled", so the BEFORE/AFTER pair is that diff.

| stage | binary | audit exit | seconds |
|---|---|---:|---:|
| `rb-before` | base tree | 0 | 367.0 |
| `rb-after` | this card's tree | 0 | 310.3 |
| `census` | base behaviour + read-only census | 0 | 443.3 |

**Byte identity, against `docs/reports/joined-provenance-call-frames/artifacts/`
(the current baseline named by `AGENTS.md`):**

| artifact | BEFORE | AFTER | census |
|---|---|---|---|
| `diagnostics` (verdict-bearing) | **IDENTICAL** | **IDENTICAL** | **IDENTICAL** |
| `chunk-contracts` (verdict-bearing) | **IDENTICAL** | **IDENTICAL** | **IDENTICAL** |
| `source` | identical | identical | identical |
| `source-effects` | differs | differs | differs |
| `contracts` | differs | differs | differs |

`source-effects` and `contracts` carry the checkout root and identity digests
and have differed at every arbiter since wave 33; they are not verdict-bearing.
BEFORE reproducing the baseline byte for byte is what makes this chain
attributable, per the standing rule.

**The three-way leaf split**, leaf identity
`(unit file, function, preprocessed line, category, description)` — the
**internal** coordinate, constraint 12a:

| | |
|---|---:|
| added (only in AFTER) | **0** |
| removed (only in BEFORE) | **0** |
| unmoved | **54,503** |
| unit status moves | **0** |
| function status moves | **0** |

**Diagnostic counts, BEFORE → AFTER:**

| category | BEFORE | AFTER |
|---|---:|---:|
| `leak` | 272 | **272** |
| `unknown_alias` | 4,308 | **4,308** |
| `unsupported` | 49,922 | **49,922** |
| `summary_conflict` | 1 | **1** |
| `double_release` | 0 | **0** |

All five are the ledger's current headline numbers, unmoved. **Constraint 7's
leak pin needs no re-pin, and `AGENTS.md`'s corpus row needs no number changed.**

**Does any function's `Safe` today rest on a deleted exit edge?** No — and this
is measured twice over, not inferred: the census finds zero loop-header
`Infeasible` edges, and re-running the audit with the deletion disabled produces
byte-identical verdict artifacts. The 205 `Safe` functions are exactly the 205
that were `Safe` before.

---

## 4. Defend or restrict, and why

**Restricted.** The measurement decided it, in the direction the brief
anticipated but for a reason stronger than "undefended".

**Why not defend.** A defence would have to prove that a pruned exit edge is
unreachable in the concrete C. That claim is **false as stated**, and one
witness is enough: a `volatile` object may change outside the program, so
`while (!flag) { }` terminates. There is no premise to record that would make
the old arm correct — only a premise that would make it correct *for
non-`volatile`, non-asynchronously-modified objects*, and the reduced importer
does not read the `volatile` qualifier, so it could not check it. A second,
independent premise would still have been needed even then: that every body
write to the deciding name reaches the header as a forget, an opaque value or a
poison — constraint 10a's `Sparing` completeness, in a role constraint 10a does
not record. Two unverified premises, one of them demonstrably false, is not a
theorem.

**Why this restriction and not a wider one.** The failure is not "path
sensitivity is unsound"; it is **orphaning**. An `if`'s two arms rejoin at the
same continuation, so pruning one drops a path and no destination — and the
corpus has 154 such prunings whose precision the analysis rests on. A loop
header's two successors do **not** rejoin: everything after the loop hangs off
one edge. So the rule is keyed on a **structural** CFG fact, not a value:

> A loop header's exit edge is never removed.

That is card `C-GATEWAY-CONSTANT-DO-WHILE`'s own property
(`the_exit_edge_is_never_removed`), which arbiter #1 proved for its token rule
and did not check for its neighbour. This card makes it true of the value route
too.

**Why the back edge is not folded in.** Deleting a *body* edge is exactly what
`C-GATEWAY-CONSTANT-DO-WHILE` proved for a literal token, and the value route's
version of it rests on the same two premises (`while (flag)` with a `volatile`
zero flag was measured `Safe` while swallowing a double release **inside** the
body). It is a real finding and it is registered as
`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED` — but it needs its own Lean statement and
its own measured re-pin, on the same ground `CONSTANT-DO-WHILE-OTHER-CONSTANTS`
gives for the mirror case: "it must not be folded into this rule." Measured
corpus reach today: **0** body edges deleted at any loop header.

### 4.1 What landed

`LegacyCfgNode` gains `loop_exit_edge: Option<usize>`, recorded **where each
loop header's successors are assigned** — a static structural fact, never
re-derived from reachability, because constraint 6 records that a fact read off
an intermediate iterate depends on the work-list schedule and this one gates a
refusal. `while` and the two-successor `do`-`while` record index 1; the
literal-zero `do`-`while` records index 0 (its continuation is its only
successor); `for (;;)` records **`None`**, having no exit successor at all —
that is the separate unowned neighbour `FOREVER-FOR-EXIT-EDGE-UNOWNED`,
deliberately untouched.

The `Infeasible` arm retains that edge carrying the **unrefined** state and
marks the path `unmodeled`. The refusal itself is reported from the settled
states by `report_retained_loop_exit_edges`, **not** inside the work list: the
arm is reached on intermediate iterates of a header whose fixed point later
widens the deciding binding to `unknown`, so reporting inside the loop would be
schedule-dependent. `require_live_borrowed_actuals` reads its own must-fact the
same way and for the same stated reason. Retaining the edge is itself monotone,
so the fixed point stays order-independent.

The diagnostic is `Unsupported` with the pinned text
`loop exit edge retained: a modeled value cannot remove it`, so
`classify_diagnostics` reads `Unknown` — the fail-closed answer for a path whose
absence the analyzer cannot prove. **No `Q` code is allocated and none is
widened**: gateway diagnostics are `LegacyDiagnosticKind` values, not `Q`-space
codes, so the iceboxed `Q-NAMESPACE-DECISION` is not engaged.

---

## 5. Lean — `QCore.ScalarRouteExitEdge`

Registered in `proof/QCore.lean`; audited by `proof/AuditScalarRouteExitEdge.lean`.
Axiom surface: `propext` only on the decidable rows, nothing on the rest — no
project axioms, no placeholders.

**The rule.** `the_exit_edge_is_never_removed` — a loop header's exit edge is
always propagated.

**Nothing is lost**, at three levels, because *adding* an edge is the accusing
direction and must be shown not to be the admitting one:
`propagation_only_grows` (edges), `base_reach_implies_reach` (destinations, by
induction on the reachability relation), `no_diagnostic_is_withdrawn`
(diagnostics), and `the_rule_never_promotes_to_safe`, which is invariant 1
stated: if the function is `Safe` after this card it was `Safe` before it.

**The boundary**, so the rule is no wider than its finding:
`a_branch_is_unchanged` (an `if` keeps exactly the path sensitivity it had — the
154 corpus prunings) and `the_body_edge_is_unchanged` (the sibling back-edge arm
is untouched).

**Non-vacuity**, so the rule is not decoration: `the_old_rule_strands_the_exit`
and `the_new_rule_keeps_the_exit` are the two rules genuinely parting on one
header, and `the_witness_agrees_on_the_body_edge` shows they part on the exit
edge **alone**.

**Why restricted rather than proved** — the part that carries the finding rather
than the fix. `the_abstract_state_does_not_decide_the_exit` exhibits two
concrete situations that agree on **everything `refine_pointer_null` reads** and
disagree on whether the exit is reachable; the witness is the `volatile` object.
`the_second_arm_has_the_same_gap` says the same of the known-non-null arm, so
the finding is not about `int` bindings.
`retention_does_not_read_the_unchecked_premise` is the content of the repair:
this card's rule does not consult the premise it cannot check.
`a_spared_write_leaves_the_fact_standing` and `no_write_leaves_the_fact_standing`
name the second premise (`SCALAR-ROUTE-POISON-DEPENDENCE`), with
`an_unspared_write_widens_the_fact` as the arm that made the old rule look safe
wherever it had been tested — real, but not the only one.

---

## 6. Tests, and the mutations that falsify them

`crates/qcore-c-gateway/tests/scalar_route_exit_edge.rs` — 11 tests, all
end-to-end through the real gateway, every one asserting on the leak, the double
release or the retained-edge refusal rather than on the `double_releases()`
proxy. Withdrawing half: `an_inverted_loop_header_does_not_delete_its_exit_edge`
(6 headers × 3 loop kinds), `the_known_non_null_arm_does_not_delete_its_exit_edge`
(the second arm, 3 spellings),
`a_retained_exit_edge_still_reports_a_double_release_after_the_loop`,
`a_volatile_flag_loop_is_not_safe`. Accusing half:
`a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm`,
`the_back_edge_arm_is_unchanged`, `a_branch_keeps_its_ordinary_path_sensitivity`,
`a_forever_for_is_untouched_by_this_card`,
`the_other_zero_spellings_reach_neither_arm`,
`a_break_in_the_body_answers_as_before`,
`the_retained_edge_carries_the_state_the_header_had`.

`constant_do_while.rs`'s wave-35 row is **amended in place**, not replaced: its
four rows are kept exactly as wave 35 measured them, its docstring now says
which edge they are about, and one assertion is added that the exit is reached
on every row — the assertion whose absence is why the finding survived a wave.

**Mutations** (`measure/scalar-route-exit-edge/mutate.py`). Each edits
`legacy_c.rs`, runs the card's binary, restores the pristine file **with its
mtime advanced**, and the run ends by re-checking the restored file's sha256 and
re-running green. All six go red; a green one would have been a finding.

| mutation | what it removes | tests that go red |
|---|---|---|
| M1 prune the exit edge again | the whole restriction | 5 |
| M2 drop the settled report | the refusal is never reported | 2 |
| M3 `while` header forgets its exit edge | the rule cannot see the edge | 5 |
| M4 protect every edge | fires on branches too | 2 (incl. the branch row) |
| M5 protect the body edge instead | the wrong index | 5 |
| M6 report inside the work list | schedule-dependent reporting | 1 |

M4 reddens both `a_branch_keeps_its_ordinary_path_sensitivity` **and**
`the_back_edge_arm_is_unchanged`, which is the measurement that the two sibling
arms are genuinely distinguished by the field rather than by accident. M6
reddens exactly `a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm`,
which is the schedule hazard caught by a test rather than by argument.

Final line: `restored: sha256 EQUAL  tests exit=0`.

---

## 7. The re-pin

BEFORE reproduced **`docs/reports/joined-provenance-call-frames/artifacts/`**,
byte for byte in both verdict-bearing artifacts — the baseline `AGENTS.md`
names, checked against the ledger rather than read out of a script docstring.
`origin/main` was re-fetched before the final BEFORE and had not moved past
`477b733d`, so no re-run was needed; the second arbiter
`W37-CORPUS-CONTRACTS-4` runs its own BEFORE against whichever of us the
integrator merges first.

Archived under `artifacts/`: five stems × two stages plus the manifest, gzipped
with `mtime=0` so archive bytes depend only on content. Every earlier archive is
untouched (constraint 21).

**No leaf moved, so there is no leaf to attribute.** That is the whole of §7:
the three-way split in §3 is `0 / 0 / 54,503`, and the internal coordinate
(constraint 12a) is unchanged for every one of the 54,503.

---

## 8. Gate

Run at the card tip. `git submodule update --init --recursive` first, exit
captured to its own file; every step's exit captured to a file with **no pipe in
the command**. Disk checked before the run (`df -h .`): 78 GB free, above the
60 GB threshold, so the full gate ran. Per-command exits transcribed into
`gate.json` beside this report.

| step | exit | |
|---|---:|---|
| `git submodule update --init --recursive` | **0** | |
| `cargo test --workspace --no-fail-fast` | **0** | **4,622 passed / 0 failed / 8 ignored**, 232 result summaries, 0 failing |
| `cargo fmt --all -- --check` | **0** | exited 1 first; see below |
| `cargo clippy --all-targets -- -D warnings` | **0** | re-run after the reflow |
| `cargo run … --example reevaluation_probe` | **0** | |
| `cargo run … --example reevaluation_followup` | **0** | |
| `python3 scripts/check_lean_audits.py` | **0** | **136** audits (135 before this card) |

Prior workspace figures, for the delta only: 4,580 at the wave-35 close, 4,559
at wave 34, 4,544 at wave 33.

Two things are recorded rather than smoothed over.

**`fmt` exited 1 on the first run.** `cargo fmt --all` re-wrapped two statements
this card added — one joined onto a single line, one `if`/`else` split across
lines. The diff is whitespace only and **no token changed**; the re-check exits
0. Because the reflow touched `legacy_c.rs` after the workspace run,
`cargo clippy` was re-run rather than carried, and `cargo test -p qcore-c-gateway`
was run as an extra step (all targets green, not part of the standing gate). The
**4,622 figure is from the pre-reflow tree** and is reported as such.

**The filesystem reached 100% during the run**, several cards building
concurrently. No `ENOSPC` reached this gate's own steps — the workspace build had
completed before the window, a grep of `test.log` for the error is empty, and
85–88 GB were free again afterwards. A gate figure is a measurement on one
machine under stated conditions, never a property, so the condition is stated.

`origin/main` was re-fetched after the final BEFORE and is still `477b733d`, an
ancestor of this branch: the baseline had not moved, so no BEFORE re-run was
needed.

---

## 9. Named follow-ons

Each with a discharge condition.

* **`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`** — the sibling arm. A positively
  spelled loop header (`while (flag)` with a known-null binding, `while (p)`
  with a known-non-null one) still loses its **body** edge, decided from a
  modelled value, on the same two premises this card found unverifiable: the
  `volatile` witness applies unchanged (`volatile int flag = 0;` with
  `while (flag) { q_free(p); q_free(p); }` was measured `Safe`, swallowing a
  double release *inside* the body), and `Sparing` completeness applies
  unchanged. It is deliberately not folded in, because deleting a body edge is
  what `C-GATEWAY-CONSTANT-DO-WHILE` proved for a literal token, and
  `CONSTANT-DO-WHILE-OTHER-CONSTANTS` gives the ground for the mirror case:
  the other direction needs its own Lean statement and its own re-pin.
  Measured corpus reach today: **0** of 201 loop-header nodes.
  **Discharge condition:** a card that either proves the back-edge deletion
  sound under a premise it states and the analyzer can check, or restricts it,
  in both cases with its own measured re-pin and a test driving a `volatile`
  flag and a spared body write.

* **`GATEWAY-VOLATILE-PREMISE`** — the abstract state of `legacy_c.rs` models no
  asynchronous modification of any object. This card measured it load-bearing on
  one route in both directions, but it is a **global** premise of the analyzer
  of the same kind as `AOM-POISON-NO-UB`, and it is written down nowhere: the
  reduced importer does not read the `volatile` qualifier, so every value-derived
  must-fact in the file inherits it silently. This card's restriction removes the
  premise's *load* at a loop header's exit edge and nowhere else.
  **Discharge condition:** a census of the places in `legacy_c.rs` where a
  value-derived must-fact decides an admission or a CFG edge, followed either by
  recording the premise as a standing constraint beside 5a and 10a with that
  census as its evidence, or by giving the importer the qualifier so the premise
  can be checked rather than assumed.

* **`SCALAR-ROUTE-BRANCH-PRUNING-UNPROVED`** — the 154 corpus `if`-edge prunings
  this card deliberately preserves (24 `empty_targets`, 130 `known_non_null`)
  rest on exactly the two premises above and carry no theorem. They are not the
  orphaning class — an `if`'s arms rejoin, so no destination is lost — but a
  pruned arm's diagnostics are still not reported, and under the `volatile`
  witness a pruned arm can be taken. Whether any of the 205 corpus `Safe`
  functions depends on one is **not measured by this card**; §3 measures only
  that none depends on a deleted *exit* edge.
  **Discharge condition:** re-run this card's census harness with branch pruning
  disabled and diff the verdicts, which answers the question the same way §3
  answered it for loop headers; then either prove the branch arm sound under a
  stated premise or record the measured dependence.

* **`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`** — the "hazard unreached" result in
  §3 is **conditional on a coverage fact, not on a safety fact**. 369 of 412
  loop-header edge rows (89.6%) land in `refine_pointer_null`'s `untracked`
  early return, whose comment says why: "the reduced importer has no scalar type
  environment." The hazard is unreached because the corpus's loop conditions are
  names the importer does not bind — not because the corpus lacks the shape (37
  rows spell the inverted polarity). Any card that gives the reduced importer a
  scalar type environment, or otherwise binds integer loop conditions, moves
  those 369 rows into the deciding arms and makes this hazard reachable in one
  step. That is precisely why the rule was landed now rather than deferred.
  **Discharge condition:** re-run `measure/scalar-route-exit-edge/census.sh` when
  such a card lands and publish the moved table beside it; the restriction needs
  no change, but the "0 of 201" figure must not be carried forward as though it
  were stable.

* **`SCALAR-ROUTE-ZERO-SPELLINGS`** — the brief asked whether this moves with
  `CONSTANT-DO-WHILE-ZERO-SPELLINGS`. **Measured: it does not, and here is why.**
  The two rules key **different predicates in different positions**.
  `condition_is_literal_zero` (`legacy_c.rs:4652`) is
  `matches!(tokens, [token] if token.text == "0")` over a *condition* token
  stream. The value route keys on `is_null_constant` (`legacy_c.rs:4799`), which
  matches `NULL`, `nullptr`, `0` and `(void *)0`, applied to an *initializer's*
  right-hand side and to the operands of `==`/`!=`. Neither is a subset of the
  other — `is_null_constant` already accepts two spellings
  `condition_is_literal_zero` refuses — so widening one does not widen the
  other, and `CONSTANT-DO-WHILE-ZERO-SPELLINGS`'s discharge (a preprocessed
  census of `do … while (E)` conditions) does not answer this question. Pinned
  by `the_other_zero_spellings_reach_neither_arm`: `0x0`, `0u`, `0L`, `00`,
  `'\0'` and `1 - 1` bind no known null, so they reach neither deciding arm and
  keep every edge.
  **Discharge condition:** a census of *initializer* right-hand sides over the
  185 units classifying every single-token integer constant expression, followed
  either by a widening of `is_null_constant` with its own measured re-pin or by a
  recorded statement that the set is empty. Note the direction: widening
  `is_null_constant` would create new exit-edge deletions on this route, which is
  now harmless — this card refuses them whatever the binding says.

### Retirement

* **`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED` — RETIRED.** Verified registered in
  `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups` before naming
  it. Its discharge condition was "a card that states the route's premise,
  measures its corpus reach, and either gives it a theorem or records it as a
  measured boundary — or a finding that the two routes can disagree, which would
  be the more interesting outcome." All of it is now done, and the last clause
  is what happened: the routes **do** disagree. `C-GATEWAY-CONSTANT-DO-WHILE`'s
  token rule never removes an exit edge and is proved not to; the value route
  removed one, and the disagreement is a false `Safe`. The premise is stated
  (§4, and in Lean), the corpus reach is measured (§2: 0 of 201 loop headers,
  154 branch prunings), and the route now has a theorem, a report and a card.

---

## 10. For the integrator

Land these in these words, or record `declined` with the ground.

**1. Ledger — `AGENTS.md`, Cycle 19 raw corpus row.** **No number changes.** This
arbiter moved zero leaves, zero functions and zero units, and its AFTER is
byte-identical to its BEFORE. Append to the row's chain sentence, after the
`JOINED-PROVENANCE-CALL-FRAMES` clause:

> `SCALAR-ROUTE-EXIT-EDGE` (wave 37's first arbiter) then moved **zero** leaves,
> zero functions and zero units — its AFTER is byte-identical to its BEFORE and
> to the wave-36 baseline in both verdict-bearing artifacts — so all counts
> above stand: the pre-existing value route's deletion of a loop header's
> **exit** edge, which returns a false `Safe` for a leak, a double release or a
> release after escape placed after the loop and does so against well-defined C
> for a `volatile` flag, is refused at the structural level, and the corpus is
> measured to reach neither deciding arm at any of its **201** loop-header
> `BranchOnNull` nodes — 89.6% of those edges landing in the route's
> `untracked` early return, which is a coverage fact and not a safety one
> (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`).

**2. Amendment to standing constraint 5c**, proposed in these words. Append,
after the existing sentence naming `CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`:

> That neighbour is now owned by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37). The
> value route decides a loop header from a **modelled value**, and deleting a
> loop header's **exit** edge orphans the function's exit node, so
> `report_leaks` never runs and `classify_diagnostics` returns `Safe` — measured
> for `while (!flag)`, `while (flag == 0)`, all three loop kinds and the
> known-non-null arm, and **unsound** rather than merely undefended for a
> `volatile` object, which C11 §5.1.2.3 lets change outside the program. A loop
> header's exit edge is therefore **never removed**, whatever the value says:
> `LegacyCfgNode::loop_exit_edge` records the leaving successor where each
> header's successors are assigned, the `Infeasible` arm retains that edge
> carrying the unrefined state, and the refusal is reported from the **settled**
> states by `report_retained_loop_exit_edges` rather than inside the work list,
> because the arm is reached on intermediate iterates (constraint 6). The
> sibling **back** edge is deliberately unchanged
> (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`), an `if`'s path sensitivity is unchanged
> (154 corpus prunings), and `for (;;)` carries no exit successor for the rule
> to reach. Proved as `QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed`,
> with `the_rule_never_promotes_to_safe` for the other direction.

**3. `named_followups`.** Retire **`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`**
(registered; discharge condition met in full, including its "more interesting
outcome" clause). Register the five successors in §9:
`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`, `GATEWAY-VOLATILE-PREMISE`,
`SCALAR-ROUTE-BRANCH-PRUNING-UNPROVED`, `SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`,
`SCALAR-ROUTE-ZERO-SPELLINGS`. Net **171 → 175**; keep `AGENTS.md`'s stated
count on the line after `named_followups` in step.

**4. Diagnostics namespace.** Nothing to do. The card allocates no `Q` code and
widens no `Q` condition — the refusal is a `LegacyDiagnosticKind::Unsupported`
description, which is gateway-internal and outside `Q` space, so the iceboxed
`Q-NAMESPACE-DECISION` is not engaged. It is **not** an instance of the
condition-extension rule either, and should not be counted as one.

**5. A note for the second arbiter.** This card's AFTER is byte-identical to the
wave-36 baseline, so if `SCALAR-ROUTE-EXIT-EDGE` merges first,
`W37-CORPUS-CONTRACTS-4`'s BEFORE may be checked against **either**
`docs/reports/joined-provenance-call-frames/artifacts/` or this card's
`artifacts/` — the two agree byte for byte in both verdict-bearing artifacts.
That is a convenience, not a licence to skip the check.
