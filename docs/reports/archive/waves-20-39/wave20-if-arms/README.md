# IF-ARMS — widening `source.branch.v1`'s arm statement subset (wave 20)

Card brief: *"Widen `source.branch.v1`'s ARM statement subset, in soundness
order, implementing the IF-STATEMENT follow-ons: `IF-ARM-RETURN`,
`IF-ARM-CALL`, `IF-ARM-ACQUIRE-ATTRIBUTION`. Every widening is fail-closed with
named refusals for what stays out. Get the Lean story right."*

Base commit: main `e01cac2`.

**What landed.** All three, in soundness order. An arm may now `return`, an arm
may now contain a zero-argument call, and the acquisition that stays refused
says which arm and which identity. The join rule changed shape as the brief
predicted — a returning arm discharges at its own exit and the join is over the
arms that survive — and one further mechanism was needed that the brief did not
name: the statements after a branch one arm of which returns are **moved into
the arm that survives to reach them**, because four of the five models that join
a branch cannot express an exiting arm. That move is proved observation-
preserving and proved to be a *move* (no duplication) in Lean. 35 new tests
including three sanitized native runs, 28 audited Lean theorems in a new module
plus a restated join section in `QCore.BranchJoin`, no new axiom, **no new
keyword, no new diagnostic code and no band move**.

---

## 1. The measurement that decided the shape of `IF-ARM-CALL`

Card `IF-STATEMENT` refused a call in an arm because *"a call takes a position in
the whole-program call sequence, which has no branch structure"*, and the brief
framed the difficulty as keeping the reference-versus-native observation
comparison well-defined. Measured first, on the base commit:

```
$ cat probe.qcore
error Failure { Failed; }
fn leaf() -> i32 { return 0; }
fn main() -> i32 { let a = qcore_acquire(7, true, Failure); leaf(); drop a; return 0; }
$ qcorec emit -o probe.c probe.qcore
$ grep -c leaf probe.c
0
```

**A source call emits no C at all.** In this executable subset a call is a pure
*checking* construct: it composes the callee's summary flow and effects into the
caller's (`unfold_summary`), splits `resource_segments`, and demands the callee's
usage. It contributes nothing to the emitted program, nothing to the reference
observation and nothing to the CFG observation. The callee is not even emitted.

So the observation half of `IF-ARM-CALL` was never the difficulty — it is
vacuous, and saying so is more useful than claiming the card solved it. The pin
the brief asked for (both condition polarities, reference against native) is
still worth having and is landed as two fixtures and two sanitized runs, but it
is a pin on the *branch*, with a call standing beside it, not a pin on a call's
observable behaviour.

What is real is the flat, positional call sequence, and it is answered in §4.

A second measurement bounded the practical cost of that answer. A call to a
callee whose summary flow is non-neutral **already refuses at the top level**,
because a caller's declared `ResourceBounds` are derived from its own counters
and never include a callee's:

```
$ qcorec emit -o probe.c probe.qcore        # leaf() acquires and releases
probe.qcore:3:1: Q200: whole-function summary exceeds its declared resource bounds
```

Every callable source function in an admitted program is therefore already
flow-neutral. The conditional-call gate §4 adds is strictly stronger than
nothing and, in the corpus of programs the subset can express, costs nothing.

---

## 2. `IF-ARM-RETURN`, and the join-shape change

### 2.1 Why an arm may return

`IF-STATEMENT`'s ground was positional: *"`SourceOp::Return` is the last
operation of a program, and an early exit from an arm has no position in this
lowering."* True, and not load-bearing. What makes a `return` safe is not where
it stands but what it demands, and `SourceOp::Return` demands the same thing
wherever it stands:

```rust
SourceOp::Return(_) => {
    if !custody.live.is_empty() {
        return Err(LoweringError::LiveResourcesAtNormalReturn(custody.live.clone()));
    }
    custody.returned = true;
}
```

That *is* the cleanup obligation the sequential route imposes at any exit: every
owned resource released or moved. The error-region rules are unchanged, because
a `return` contributes no error operation on either route. So an arm exit is the
program exit, one case of one step function, and the arm that takes it has
discharged rather than deferred.

### 2.2 The join, restated

`crates/qcore-core/src/cleanup_lowering.rs` gains a named `join_arms`:

| Arms | Join |
|---|---|
| neither returns | **agreement**, exactly as `IF-STATEMENT` landed it: equal ordered live lists, equal transferred sets, or `BranchArmsDisagree` |
| exactly one returns | the survivor's state. A one-element join is that element |
| both return | the branch terminates the function; `live` is empty on both sides and the transferred sets are **unioned** (the refusing direction, and inert — nothing can follow) |

This is a change of *shape*, not a relaxation, and the Lean model states the
sense in which that is true as a biconditional: **the only combination with no
joined state is two surviving arms that disagree**
(`the_join_refuses_only_two_disagreeing_survivors`). Nothing that refused at the
join before is admitted at it now; the returning arm is admitted by its own
exit's obligations instead, and `an_arm_return_must_discharge_its_custody` shows
that obligation is real.

`LoweringError::ReturnInBranchArm` is removed, being a refusal this revision no
longer has.

### 2.3 The mechanism the brief did not name: the continuation moves

The core model implements the general rule above. The **driver cannot use it**,
and finding out why is the card's second finding.

`IF-STATEMENT` §1's correction was that five of six models already branched. Four
of those five — resource flow (`Flow::join_branches`), ownership
(`OwnershipOp::Branch`), typed errors (`ErrorOp::Branch`) and usage
(`UsageExpr::Branch`) — join two arms and have **no vocabulary for an arm that
exited**. Handing `OwnershipOp::Branch { then_ops: [Drop("a")], else_ops: [] }`
to a model that compares ledgers refuses the canonical early return:

```qcore
let a = qcore_acquire(1, true, Failure);
if (3 < 5) { drop a; return 1; }
drop a;
return 0;
```

So the driver does not hand them a one-sided join. **The statements written after
the branch are lowered inside the arm that survives to reach them**, which is
where they run:

```text
if (C) { S; return v; }  REST      ==>      if (C) { S; return v; } else { REST }
```

Every model then sees a two-armed branch it already understood, both of whose
arms end the function — and the four join models still run, unedited, comparing
the arm against the arm-plus-continuation. The rewrite is a front-end
desugaring, in the same spirit as `OR-AND-LOWERING`'s connective elimination,
and `qcore-core`'s rule stays the general one.

Three properties make it safe, and all three are proved rather than asserted
(`proof/QCore/BranchArms.lean`):

* **It preserves the observation.** `the_hoist_preserves_the_observation`, by
  cases on the decided condition, against `BranchJoin`'s own interpreter. The
  supporting lemma is that a terminating arm never falls out of its branch
  (`a_terminating_sequence_observes`), which is why appending the continuation
  to the *other* arm loses nothing.
* **It is a move, not a duplication.**
  `the_hoist_moves_statements_without_duplicating_them`: the rewritten program
  has exactly as many operations as the program it came from. This is
  load-bearing rather than decorative — duplicating the continuation would
  duplicate every acquisition identity in it (refused as `ResourceAlreadyOwned`),
  every call's position in the flat call vector, and every usage demand.
* **It is inert without an arm exit.**
  `the_hoist_is_the_identity_without_an_arm_exit`, the model counterpart of the
  byte-identity pin, and `a_branch_with_no_arm_exit_is_numbered_as_before` in
  Rust.

### 2.4 What that costs, stated as refusals

Because the continuation is placed **once**, the shape needing it twice is
refused rather than duplicated:

| Written | Result | Ground |
|---|---|---|
| `if (C) { drop a; return 1; } drop a; return 0;` | admitted | one survivor, one continuation |
| `if (C) { return 1; } else { return 0; }` | admitted | no survivor; nothing follows |
| `if (A) { drop a; return 1; } else if (B) { drop a; return 2; } drop a; return 0;` | admitted | the chain's last non-returning arm carries it |
| `if (C) { return 1; }` with nothing after | **`Q044`** | the survivor has no continuation, so one path reaches no `return` |
| `if (C) { return 1; } else { return 0; } return 2;` | **`Q044`** | unreachable statements |
| `if (A) { if (B) { drop a; return 1; } drop a; } else { drop a; } return 0;` | **`Q044`** | the tail would be needed in the inner `else` **and** the outer `then`. `IF-ARM-RETURN-NESTED` |
| `if (A) { drop a; } else if (B) { drop a; return 1; } drop a; return 0;` | **`Q044`** | the chain's *second* level spends the tail and the first level then needs it too. Same follow-on, different half — and the refusal says which |
| `if (C) { return 1; } let a = qcore_acquire(…); drop a; return 0;` | **`Q103`** | the hoist places the acquisition inside an arm, which the pre-existing ownership ledger join refuses. `IF-ARM-RETURN-ACQUIRE` |
| `if (C) { move a; return 1; } drop a; return 0;` | **`Q103`** | §7 finding 3 |
| `if (C) { return 1; } drop a; return 0;` with `a` live at the exit | **`Q202`** | the exit leaks; §3 |

---

## 3. Attribution: two refusals given a span they earn

`docs/QCORE_SPEC_r0_3.md` §L21 says fixing where a diagnostic is *attributed* is
not a code change. Two refusals moved span and wording and **not** code.

### 3.1 An arm exit that leaks — `Q202`, not a join refusal

`if (3 < 5) { return 1; }` with `a` live reported, before this card's
attribution work:

```
Q046: the arms of this branch do not join in the resource-flow model:
      conditional branches have different net resource changes
```

which is fail-closed but a wrong diagnosis: the arms *do* differ in net resource
change, because one of them exited. The condition is a leak at an exit, which is
`LiveResourcesAtNormalReturn` — `Q202`'s own condition — and it is now reported
there, at the exit that leaks:

```
arms.qcore:4:16: Q202: 1 owned resource(s) are still live at this `return`: an
exit from a branch arm carries the same cleanup obligation as the program's own
exit, so every owned resource must be released or moved before it
```

The **top-level** exit is deliberately untouched: its attribution is pinned
elsewhere and this card does not move it. The check is gated on being inside an
arm precisely so that nothing already pinned changes.

### 3.2 An acquisition inside an arm — `Q103`, `IF-ARM-ACQUIRE-ATTRIBUTION`

The brief was explicit: **do not** admit arm acquisitions. Measured on four
shapes before deciding, and every one refuses — the same binding name with
different identities under `drop` and under `move`, two different binding names,
and an acquisition in one arm only — because `OwnershipState::branch` compares
resource-identity *ledgers* and an acquisition on one path leaves one the other
does not have. Two arms can
never coincide there: distinct identities differ in the ledger, and the same
identity is `DuplicateResource`. So the refusal is total, and it is now made at
the acquisition rather than inherited at the declaration:

```
arms.qcore:3:16: Q103: `a` acquires resource identity 1 inside a branch arm: the
arm that does not run leaves a different resource-identity ledger, so the arms
cannot join (follow-on `IF-ARM-ACQUIRE`)
```

The pinned test `an_acquisition_inside_an_arm_refuses_at_the_pre_existing_ownership_join`
in `branch_statement.rs` is **unedited** and still passes: same code, same
refusal.

A second wording exists for the one situation the reader did not write: a
statement placed inside an arm by §2.3's hoist says so, rather than claiming the
reader wrote it in an arm.

### 3.3 What was measured and deliberately not attempted

Whether an arm acquisition is *sound* when both arms acquire and release
internally — the brief invited an honest measurement. It is not sound under the
model as it stands, and not for a reason about custody: the arms leave different
**ledgers** and different **bindings**, which `OwnershipState::branch` compares
by design. Admitting it needs a scope-exit notion in the ownership model, which
is `IF-ARM-ACQUIRE`'s content and changes a pinned model. Not attempted.

---

## 4. `IF-ARM-CALL`: the flat call sequence, answered by a premise

`FunctionSemantics` carries `calls: Vec<CallSite>` and
`resource_segments: Vec<Flow>` with `resource_segments.len() == calls.len() + 1`,
and `unfold_summary` composes them **linearly**:

```rust
flow = flow.then(&callee.flow)
           .and_then(|with_call| with_call.then(&segments[index + 1]))
```

Two things break if a call inside an arm is treated as a top-level one.

1. **The arm's own flow would leave the branch.** At a call the driver does
   `resource_segments.push(std::mem::take(&mut output.flow))`. Inside an arm
   that takes the arm's accumulated custody flow *out* of the branch and into
   the flat chain, where it is applied on both paths — the conditional release,
   silently made unconditional. So a conditional call pushes `Flow::zero()`
   instead and leaves the arm's flow where the four-model join can see it. The
   pin is `a_conditional_call_does_not_launder_the_custody_join`: the
   conditional leak still refuses at `Q046` with a call standing beside it.
2. **The callee's flow is still composed on every path.** That is faithful
   exactly when composing it is the identity, so `unfold_summary` **refuses** a
   conditional call whose callee summary flow is not neutral:

   ```
   arms.qcore:4:16: Q044: `source.branch.v1` admits a call inside a branch arm
   only when the callee's summary flow is neutral: the whole-program call
   sequence is flat, so a conditional call would otherwise compose its callee's
   resource flow on paths that never perform it
   ```

   This is a *premise stated and checked*, not an approximation: a neutral flow
   composes as the identity, so nothing is over- or under-claimed. `CallSite`
   gains one field, `conditional: bool`, which is how the check knows. Lifting
   the premise — making the whole-program call summary genuinely branch-aware —
   is follow-on `IF-ARM-CALL-FLOW`.

Two further mechanical points:

* **Usage demands.** `synthesize_function`'s `may_exit` unfolding — a call may
  exit before its continuation, so everything after it is optional — was inlined
  in the entry body's lowering, and `join_branch_arms` collapsed an arm's demands
  into a flat `Sequence` with the comment *"an arm admits no call, so every arm
  demand is a non-exiting one"*. That comment is now false, so the unfolding is
  factored out as `unfold_exit_demands` and each arm runs it; the joined node
  inherits `may_exit` from its arms.
* **Everything else a call may be is unchanged.** An unknown callee, an indirect
  call, a callee with parameters and a non-function callee all keep the codes
  they have at the top level. The arm position widens *where* a call may appear,
  not *what* a call may be.

---

## 5. Diagnostics: no new code, no band move

The brief allowed a new code from the next free band. None was needed, and
taking one would have risked a collision with the concurrently running
`BITWISE-CONDITIONS` card for nothing.

| Code | What changed |
|---|---|
| `Q044` | **condition narrowed and re-specified.** It no longer refuses a `return` or a call in an arm; it refuses the shapes the widening cannot place (a mixed branch with no continuation, a continuation reaching no `return`, the nested-duplication shape, unreachable statements after a fully returning branch) and a conditional call whose callee is not flow-neutral. Still the pass's generic presentation code |
| `Q045` | untouched in code and in condition; the tier is `BITWISE-CONDITIONS`'s territory and this card changes no condition parsing |
| `Q046` | untouched in code and condition; its *scope* narrows with the join's, to two surviving arms |
| `Q202` | **re-attributed** (§3.1). Same code, same condition, better span |
| `Q103` | **re-attributed** (§3.2). Same code, same refusal, better span and wording |

`Q047` onward remains entirely free; nothing in the tree reserves it. Spec §L21
and grammar §G02i are updated in the same change, both restatements attributed.

**No new keyword and no new token.** `return` was already a reserved source word
and a zero-argument call is already the subset's call spelling; on the
`EXTERN-MODES` precedent, constraint 14 needs no entry.

---

## 6. Evidence

### 6.1 Tests

`compiler/tests/branch_arms.rs` — **35 tests, all passing**, three of them
sanitized native runs. Three new fixtures under
`compiler/tests/fixtures/branch_arms/`: `early_return.qcore`,
`call_then_arm.qcore` and `call_else_arm.qcore`.

Two pinned expectations moved in `compiler/tests/branch_statement.rs`, both
deliberately, both with their load-bearing halves preserved:

| Pin | Was | Now | What survived |
|---|---|---|---|
| `an_early_return_from_a_branch_arm_refuses` | `fn main() -> i32 { if (true) { return 1; } }` is `Q044` because a `return` in an arm is refused outright | renamed `…_keeps_this_programs_refusal`; the **same program**, the **same code**, the **same message assertion**, on the narrower ground that this branch's survivor has no continuation to reach | the code, the refusal, and the message naming `return` |
| `a_call_inside_a_branch_arm_refuses` | a call in an arm is `["Q044"]` | renamed `…_is_now_admitted`; the program is admitted, and the load-bearing half is kept as prose plus a live pin elsewhere: a conditional call to a **non**-neutral callee still refuses | the gate — it moved from "no calls" to "no calls that compose anything" |

`crates/qcore-frontend/tests/unit/whole_program.rs` gains `conditional: false` at
its eight `CallSite` literals; no assertion moved.

### 6.2 The observation pins, honestly scoped

`the_native_run_of_an_early_return_matches_its_reference` compiles the emitted C
under `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, runs it, and compares the
parsed observation with the reference. The program exits through the **arm's**
own `return 1`, having released inside the arm; the leak check is a positive
control.

The two call fixtures pin both condition polarities, as the brief asked. Their
honest scope is §1's finding: a source call emits no C, so what these two runs
cross-check is the branch and the release, with a call standing in the taken arm
of one and the untaken arm of the other. They are worth having as a boundary
marker; they are not evidence about a call's run-time behaviour, because there
is none to have.

### 6.3 Lean

**`proof/QCore/BranchJoin.lean` §4 restated.** The join rule changed shape, so
the section that characterises it had to. `joinArms` becomes the survivor join;
the `returnInBranchArm` refusal constructor is deleted; five theorems are
restated for survivors (`the_join_of_two_survivors_is_their_common_state`,
`the_join_widens_neither_survivor`, `disagreeing_survivors_do_not_join`,
`a_returning_arm_does_not_constrain_the_join` and its else-twin,
`the_join_rule_is_exactly_agreement_between_survivors`), and two are added:
`a_branch_whose_arms_both_return_returns` and, as the completeness statement,
`the_join_refuses_only_two_disagreeing_survivors`.

§5's `refused_shapes_are_exactly_these` **changed, as it was designed to**: its
fourth row was `returnInsideArm`, refusing at a constructor this revision no
longer has. It is replaced by `earlyReturnLeaks` — the obligation the widening
rests on rather than the shape it opened — and the admitted spelling is pinned
beside it as `an_arm_return_joins_over_the_survivor`. That is the list doing its
job: a widening had to change a theorem rather than slip past one.

`AuditBranchJoin.lean` moves from **35 to 39** checked names and
`QCore.BranchJoin` from 35 to 39 theorems: five join theorems restated under
new names, four added (`a_returning_else_arm_does_not_constrain_the_join`,
`a_branch_whose_arms_both_return_returns`,
`the_join_refuses_only_two_disagreeing_survivors`,
`an_arm_return_must_discharge_its_custody`), and one
(`a_return_inside_an_arm_refuses`) replaced by
`an_arm_return_joins_over_the_survivor`, which asserts the opposite.

**`proof/QCore/BranchArms.lean`**, audited by `proof/AuditBranchArms.lean`: **28
theorems, 22 of them audited by name**, no new axiom, no placeholder, no
`native_decide`. Built on `QCore.BranchJoin`'s own `Op`, `Seq`, `validate`,
`stepOp` and interpreter, so the theorems are about the model card
`IF-STATEMENT` proved.

* **Termination** — `a_return_terminates`,
  `a_branch_terminates_when_both_arms_do`, `the_missing_arm_falls_through`.
* **The arm exit is the program exit** — `the_arm_exit_is_the_program_exit`
  (`rfl`: one `ret` case, one obligation), with both halves stated separately so
  neither direction is implicit.
* **The hoist** — `the_hoist_is_the_identity_without_an_arm_exit` (neutrality),
  `a_mixed_branch_with_no_continuation_refuses` (fail-closed),
  `the_hoist_preserves_the_observation` (correctness, by cases on the decided
  condition), and its two lemmas `interp_append` and the mutually recursive
  `a_terminating_operation_observes` / `a_terminating_sequence_observes`.
* **It is a move** — `appendSeq_size` and
  `the_hoist_moves_statements_without_duplicating_them`.
* **Concrete** — the canonical early return in both spellings, observing the
  same thing and exiting through the arm's own `return`; a statement after a
  fully returning branch; a mixed branch whose *surviving* arm leaks; and
  `refused_arm_shapes_are_exactly_these` beside
  `admitted_arm_shapes_are_exactly_these`, so the refused list is a statement
  about a live gate rather than about a model that refuses everything.

`QCore.lean` imports the new module, so `AuditAll.lean` includes it in the full
axiom closure. `check_lean_audits.py` moves **85 → 86** audits.

**Not proved:** that the Rust driver performs the hoist the way the model says
(a tested Rust boundary, per the `ConcurrencyPrelude` / `PipeDesugaring` /
`BranchJoin` / `Connectives` precedent); that the four models the hoist exists
for accept the rewritten program (outside this module's vocabulary — the Rust
suite measures it, and §7 finding 3 is a measured case where they do not);
and anything about an acquisition inside an arm, which stays refused.

### 6.4 Corpus neutrality

**Corpus-neutral by construction.** The whole diff is:

```
crates/qcore-core/src/cleanup_lowering.rs         (join_arms, sequence_returns)
crates/qcore-frontend/src/whole_program.rs        (CallSite.conditional, the neutrality gate)
crates/qcore-frontend/src/source_extern.rs        (one call-site field)
crates/qcore-verification/src/driver.rs           (ArmPosition, the hoist, the attributions)
crates/qcore-frontend/tests/unit/whole_program.rs (eight literals)
compiler/tests/branch_arms.rs                     (new suite)
compiler/tests/fixtures/branch_arms/              (three new fixtures)
compiler/tests/branch_statement.rs                (two deliberate pin updates)
docs/QCORE_SPEC_r0_3.md                           (§L21, two restatements)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md           (§G02i)
docs/design/SOURCE_BRANCH_V1.md                   (an addendum)
proof/QCore/BranchArms.lean, proof/AuditBranchArms.lean,
proof/QCore/BranchJoin.lean, proof/AuditBranchJoin.lean, proof/QCore.lean
```

**No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`contract/qcore/` or `providers/` is touched.** No CT manifest entry changes and
no pinned artifact is edited. The contract corpus stays **69 modules**, CT stays
**33/50**, the 185-unit baseline and its leak count of **272** are untouched, and
nothing is promoted.

The one place a corpus effect could hide is the shared frontend, and it is
bounded exactly: `CallSite` gained a field that is `false` at every construction
site except a conditional source call, and `unfold_summary`'s new gate is
guarded on that field. `source.extern-call.v1` admits a flat sequential entry
body only, so no extern call is ever conditional — its own nested-block refusal
is what makes the constant a constant rather than a hope, and
`shipped_fixtures_are_unmoved_by_this_card` compiles a `std.io` and an extern
fixture to pin it.

### 6.5 Reproducing the measurements

The probes behind §1, §2.4 and §3 live in this worktree's gitignored `measure/`
directory and are all one-file programs runnable with
`cargo run -p qcorec -- check FILE` or `… emit -o OUT.c FILE`. Each is also
pinned as a test in `compiler/tests/branch_arms.rs`, so nothing in this report
rests on a measurement the standing gate does not replay.

### 6.6 CT candidates

None proposed. The widening adds no new checked model — the arm exit reuses the
program exit's rule and the hoist is eliminated before any model runs — so
mapping one would grow the denominator without adding evidence, which
constraint 23 forbids.

---

## 7. Deviations and findings

1. **A source call emits no C at all** (§1). The observation half of
   `IF-ARM-CALL` was vacuous; the positional call sequence was the real content.
   Recorded rather than smoothed over, because the card's brief predicted the
   opposite difficulty.
2. **The join-over-survivors rule is implemented in `qcore-core` in full, and
   the driver never produces its mixed case.** The four models that cannot
   express an exiting arm are why: the hoist turns every admitted mixed branch
   into a both-arms-return one before they see it. So `qcore-core` is more
   general than the front end can currently exercise, and that is stated rather
   than hidden — `the_unhoisted_spelling_is_admitted_by_the_survivor_join` pins
   the general rule in Lean and no Rust source program reaches it. Making the
   four models branch-exit-aware, which would retire the hoist, is
   `IF-ARM-EXIT-MODELS`.
3. **An early return whose path disposes of a resource differently from the
   surviving path refuses** (`Q103`). `if (C) { move a; return 1; } drop a;
   return 0;` is admitted by the core rule and refused by the unedited ownership
   ledger join, because after the hoist the two arms genuinely differ — `Moved`
   on one path, `Dropped` on the other. Fail-closed, measured, pinned, and named
   `IF-ARM-RETURN-DIVERGENT-CUSTODY`. Both paths disposing the same way is
   admitted, so the pin is about the divergence.
4. **An early return placed before an acquisition refuses** (`Q103`). The hoist
   puts the acquisition inside an arm, and arm acquisitions are refused
   (`IF-ARM-ACQUIRE`). This is the widening's sharpest practical limit and it is
   named `IF-ARM-RETURN-ACQUIRE`; it is not a new refusal so much as an old one
   reached by a new route, and the message says which route.
5. **`Q046` was answering for a leaking arm exit, and now `Q202` does** (§3.1).
   An attribution fix, not a code change — but worth recording, because the
   pre-fix diagnostic was actively misleading about which model had found what.
6. **The `may_exit` unfolding had to be factored out** (§4). `join_branch_arms`
   carried a comment asserting an arm admits no call; admitting one made the
   comment false, and collapsing an arm's demands flat would have asserted that
   the statements after an arm's call always run.
7. **`CallSite` gained a public field.** One production construction site and
   eight test literals. The alternative — relying on the unstated fact that every
   admitted callee is flow-neutral — is exactly the kind of unchecked premise
   this project refuses, so the premise is checked instead.
8. **The nested-duplication refusal is a budget, not a syntax rule.** The
   continuation is a permission `lower_branch` may spend once, spent by whichever
   level needs it first; a second level asking for it refuses. That makes the
   refusal a statement about the *program shape* rather than about how deeply an
   `if` is written, which is why an `else if` chain of returning arms is admitted
   while a return in one arm of a nested `if` beside a falling-through sibling is
   not.

---

## 8. Named follow-ons

| Name | What it is |
|---|---|
| `IF-ARM-EXIT-MODELS` | teach resource flow, ownership, typed errors and usage to express an exiting arm, so the join over survivors runs in all five models and the continuation hoist can be retired (invariant 3 parity first). This is the prerequisite for the three rows below |
| `IF-ARM-RETURN-NESTED` | an early return in one arm of a nested branch whose sibling falls through. Refused today because the continuation would have to be placed twice |
| `IF-ARM-RETURN-ACQUIRE` | an early return standing *before* an acquisition. Refused today because the hoist places the acquisition inside an arm (§7.4); it closes with `IF-ARM-ACQUIRE` |
| `IF-ARM-RETURN-DIVERGENT-CUSTODY` | an early return whose path disposes of a resource differently from the surviving path (§7.3) |
| `IF-ARM-CALL-FLOW` | a conditional call to a callee whose summary flow is not neutral. Needs a genuinely branch-aware whole-program call summary rather than the flat positional vector |
| `IF-ARM-ACQUIRE` | *(inherited from `IF-STATEMENT`)* an acquisition inside an arm. Needs a scope-exit notion in the ownership ledger; still refused, now well attributed |
| `IF-RUNTIME-CONDITION` | *(inherited)* a condition that reads a run-time value |
| `IF-ROUTE-WIDENING` | *(inherited)* admitting the construct on the std-io, extern, task, mutex or epoch routes. Unchanged by this card and re-pinned |
| `IF-CONDITION-BINDINGS`, `SEQ-STATEMENT-TOKENS`, `IF-CONNECTIVE-NEGATION`, `BITWISE-OPERATORS` | *(inherited, untouched)* |

---

## 9. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | checked out `d82464a277d0f42703702c4dfd9af6af38595a83` (`release-2.2.2-alpha-11-gd82464a2`), the declared revision |
| `cargo test -p qcore-compiler --test branch_arms` | **35 pass**, including three sanitized native runs |
| `cargo test -p qcore-compiler --test branch_statement` | **32 pass** (two pin updates) |
| `cargo test -p qcore-compiler --test logical_connectives` | **25 pass**, unedited |
| `cargo test --workspace --no-fail-fast` | **3,168 passed, 1 failed, 8 ignored** across 185 targets; the one failure is constraint 16's load-sensitive provider row, re-verified in isolation — §9.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `python3 scripts/check_lean_audits.py` | **86/86 pass**, exit 0; no new axiom, no warning, no placeholder |
| `python3 -m unittest discover -s scripts/tests` | **193 ran, 192 pass, 1 fail** — the derived Lean-audit-count tripwire, §10 |
| `reevaluation_probe` / `reevaluation_followup` | **exit 0 / exit 0** |

### 9.1 The numbers, and one load-sensitive row that tripped

**3,168 passed, 1 failed, 8 ignored across 185 targets.** The wave-19 partial
checkpoint recorded 2,986 across 181 targets and the two wave-19 branch cards
added 57 and 26 on top of it; against the base measured in this worktree, this
card's delta is exactly its own additions — **35** in `branch_arms.rs`, with
`branch_statement.rs` staying at 32 and `logical_connectives.rs` at 25 because
this card's two pin updates are rewrites rather than additions or deletions. The
delta therefore carries no hidden removal.

**One of constraint 16's four load-sensitive rows tripped and was re-verified in
isolation, never re-run to make it pass.**
`cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
— the wall-clock row the constraint records as failing at load 50 and passing at
load 37 on the same binary — failed at a one-minute load average above 30 with
five sibling agents' gates running. Re-run alone: **1 passed**; and the whole
target re-run alone: **20 passed, 0 failed**. No file this card touches is
reachable from that target, which links the provider host and process transport
and reads no branch code. The other three load-sensitive rows did not trip.

Five sibling agents were running their own gates against their own worktrees for
the whole of this card's closing run, with the host's swap fully committed for
part of it. Wall-clock timings here are therefore not comparable with a quiet
run. Nothing about the results depends on that fact.

The `third_party/libevent` submodule was uninitialized in this fresh worktree, as
waves 18 and 19 each recorded in the same situation. The pin was **not**
weakened: the gate's own `git submodule update --init --recursive` was run before
the closing suite and checked out the declared revision.

---

## 10. Reconciliations the integrator must make

* **`docs/reviews/r0_3/acceptance_matrix.json`** — outside this card's write
  scope. `validation.lean_audits` composes to **86** and
  `validation.lean_audit_files` gains `AuditBranchArms.lean` in sorted position.
  Until then `python3 -m unittest discover -s scripts/tests` reports one failure,
  `test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`
  — the tripwire doing its job, exactly as waves 18 and 19 each left it before
  their integration follow-ups.
  The `PIPE-OPERATOR-O` entry's residual list also now describes closed rows:
  `IF-ARM-RETURN`, `IF-ARM-CALL` and `IF-ARM-ACQUIRE-ATTRIBUTION` are done, and
  `IF-ARM-ACQUIRE` is unchanged but re-attributed. A new `IF-ARMS-O` entry
  carrying §8's list is the reconciliation.
* **`AGENTS.md`** — the "Diagnostics namespaces" table's `Q044`–`Q046` row should
  read "the source branching construct and its arm subset" rather than implying
  the `IF-STATEMENT` arm subset, and the ledger's Lean-audit count moves
  **85 → 86**. (The ledger's headline still reads 80, from the wave-19 partial
  checkpoint; the base measured in this worktree is **85**, so the delta this
  card contributes is **+1** whatever the integrator's composed base turns out
  to be.) Spec §L21, which that table summarises, is already updated here.
  Constraint 14 (the keyword census) needs no change: **this card adds no
  keyword.** A candidate standing constraint, if the integrator wants one: *the
  continuation hoist is a move, never a duplication — a shape needing the
  continuation in two arms is refused, because duplicating it would duplicate
  every acquisition identity, call position and usage demand in it.*
* **`DEVELOPMENT.md`** — one increment paragraph; a draft is §11.
* **Concurrent card `BITWISE-CONDITIONS`** — boundary respected in both
  directions: this card reads no condition, touches no lexer file and allocates
  no diagnostic code, so a band this card holds cannot collide with one that
  card allocates. The only shared file is
  `crates/qcore-verification/src/driver.rs`, where this card's edits are confined
  to `lower_sequence`, `lower_branch`, `join_branch_arms`, the statement cases
  and two new module-scope types (`ArmPosition`, `Continuation`);
  `ConditionTree`, `parse_condition_tree` and everything below them are
  byte-unchanged.

---

## 11. Draft increment paragraph

> Branch arm exits and calls (IF-ARMS): `source.branch.v1`'s arm statement
> subset is widened in soundness order, closing `IF-ARM-RETURN`, `IF-ARM-CALL`
> and `IF-ARM-ACQUIRE-ATTRIBUTION`. An arm may **return**, because what makes a
> `return` safe is not its position but its obligation — `SourceOp::Return`
> demands an empty live list wherever it stands, which is the whole cleanup
> discharge the sequential route imposes at any exit — so the returning arm
> discharges at its own exit and the **join is taken over the arms that survive
> it**. That rule's shape changed without weakening: the complete
> characterisation, proved as a biconditional, is that the only combination with
> no joined state is two surviving arms that disagree. One mechanism the brief
> did not name was needed, and it is the card's second finding: four of the five
> models that join a branch have no vocabulary for an exiting arm, so the driver
> does not hand them a one-sided join — the statements after the branch are
> **moved into the arm that survives to reach them**, a front-end rewrite proved
> observation-preserving and proved to be a *move* rather than a duplication,
> which is exactly why the shape needing the continuation in two arms is refused
> (`IF-ARM-RETURN-NESTED`) instead of duplicating every acquisition identity,
> call position and usage demand in it. An arm may contain a call, and the
> premise that difficulty rested on was measured away: **a source call emits no
> C at all** in this executable subset, so the observation was never ill-defined;
> what is real is the flat positional whole-program call sequence, answered by
> refusing a conditional call whose callee's summary flow is not neutral —
> a premise stated and checked rather than an approximation — while the arm's own
> custody flow is deliberately kept out of that chain so the four-model join
> still sees it (the conditional leak still refuses at `Q046` with a call beside
> it). An acquisition inside an arm is **still refused**, deliberately and
> measured across three shapes; only its span and wording move, onto the
> acquisition itself, naming the arm and the identity, and distinguishing one the
> reader wrote in an arm from one the hoist placed there — an attribution fix,
> not a code change, and `Q202` gains the same treatment for an arm exit that
> leaks, which a resource-flow join refusal had been answering misleadingly.
> **No new keyword, no new diagnostic code and no band move**: `Q044`'s condition
> narrows and is re-specified through §L21, `Q046`'s scope narrows with the
> join's, and `Q047` onward is left free for the concurrently running condition
> card. Two pinned expectations moved deliberately, each keeping its load-bearing
> half. 35 new tests including three sanitized native runs across both condition
> polarities, 28 audited Lean theorems in a new `QCore.BranchArms` built on
> `QCore.BranchJoin`'s own validator and interpreter, that module's join section
> restated and its refused-shapes list changed as designed, no new axiom,
> 85 → 86 audits. Corpus-neutral by construction: no gateway, audit, contract or
> provider file touched; corpus stays 69 modules, CT 33/50, leak 272, nothing
> promoted. Evidence: `docs/reports/wave20-if-arms/README.md`.
