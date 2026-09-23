# `source.branch.v1` — the source branching construct

Card `IF-STATEMENT` (wave 19). Design record written **before** the
implementation, per the card brief. Base: main `c7c0a3c`.

QCore's source language has had no branching construct. Card `PIPE-OPERATOR`
measured the consequence at wave 18: `|` and `&&` could be *claimed* as tokens
but not lowered, "because the executable subset has no branch construct to
lower a short-circuit onto" (spec §L21, `Q038`). This document specifies the
construct that removes that blocker.

Throughout, the future logical operators that will lower onto this construct
are spelled **`||`** (logical or) and **`&&`** (logical and). The single
characters `|` and `&` are reserved for future **binary/bitwise** operations
and are not logical operators. This card implements neither: it neither reads
nor writes those tokens, and their existing `Q038` behaviour is byte-unchanged
by this change. Retabling them is the phase-2 `OR-AND-LOWERING` card's work.

---

## 1. The census (§1 of the brief), measured on the base commit

### 1.1 What already exists

| Probe | Measured result |
|---|---|
| An `if` statement in the source AST | **already there.** `StatementKind::If { branches: Vec<SourceBlock> }`, parsed by `SourceParser::if_statement`, with `else` and `else if` already flattened into one `branches` vector. |
| The condition | **discarded.** `if_statement` locates the `(`, jumps to its `)`, and never records what is between them. `StatementKind::If` has no condition field. |
| `if (…) { … }` reaching a checker | **no.** `qcore-verification::driver::synthesize_function` refuses every statement with a nested block: `Q500: structured control flow is not in the Cycle 12 executable subset`. Re-measured on `c7c0a3c` (`measure/census/`), including `if (3 < 5) { }` — which **parses**, so the comparison tier is a lexing and semantics question, not a parsing one. |
| A boolean expression tier | **none**, confirming wave 18. There is no expression node of any kind: `StatementKind::Expression` is payload-free. |
| A branch constructor in the lowering IR | **none.** `qcore_core::cleanup_lowering::SourceOp` is exactly `Acquire { succeeds }`, `Check { passes }`, `Drop`, `TransferToResult`, `Return`. |
| A branch **terminator** in the CFG | **two, and they are real.** `CfgTerminator::TryAcquire { success, failure }` and `CfgTerminator::Check { success, failure }`, and `c_emit` already emits `if (…) { goto …; } else { goto …; }` for both. |
| A branch join in the **other** models | **four, all shipped** — `OwnershipOp::Branch`, `ErrorOp::Branch`, `UsageExpr::Branch`, `Flow::join_branches`, each with its own mismatch error. See §3.3; this is the card's headline finding and it corrects wave 18's premise. |
| Comparison tokens | **already lexed.** `Symbol::{EqualEqual, NotEqual, LessEqual, GreaterEqual, Less, Greater}` all exist. |

The wave-18 premise holds with one important refinement that this card's whole
design turns on: **the missing piece is not branching in the models, it is
branching in the *source lowering*.** The or-operator amendment's claim that a
short-circuit could lower onto "existing branch/CFG constructs" was recorded by
wave 18 as false. It is **substantially true**, and §3.3 is the measurement:
five of the six models a statement passes through already branch, and the sixth
— `cleanup_lowering::SourceOp` — is the one linear IR that does not. This card
adds that one constructor and reuses everything else.

### 1.2 What controls control flow today

Every branch the executable subset can already express is controlled by a
**literal** written in the source: `qcore_acquire(id, true, Failure)` and
`qcore_check(false, Failure)`. `parse_bool` accepts exactly `true` and `false`.
Nothing in the subset produces a runtime value that a branch could read — this
is the same missing tier card `PIPE-CHAIN` recorded as `PIPE-SCALAR-RESULT`.

This is load-bearing for the design, not incidental. `validate_translation`
compares **one** observation from the reference interpreter, **one** from the
CFG interpreter and **one** parsed from the compiled C program's own output. A
condition that varied at run time would make "the" observation ill-defined, and
the certificate would have nothing to compare. So `source.branch.v1`'s
condition tier is decidable at check time, exactly as `succeeds` and `passes`
are, and the follow-on that lifts it is named `IF-RUNTIME-CONDITION` (§7).

### 1.3 Keyword census (§4 of the brief)

`is_reserved_source_word` in `crates/qcore-frontend/src/frontend.rs` already
contains **`if`**, **`else`**, **`true`** and **`false`**.

| Candidate | Status | Evidence |
|---|---|---|
| `if` | already reserved; already a parsed statement head | `frontend.rs` `statement()` |
| `else` | already reserved; already parsed after a then-block | `frontend.rs` `if_statement()` |
| `true` / `false` | already reserved; already the literal outcome controls | `parse_bool` |
| `==` `!=` `<` `<=` `>` `>=` | already lexed as single tokens | `Symbol` enum |

**This card mints zero new reserved words and zero new tokens**, on the
`EXTERN-MODES` precedent ("deliberately added none"). The `work` → `effort` and
`error` → `fatal` respelling precedents are therefore not needed.

Occurrence census across all 369 `.qcore`/`.qc` files in the tree, 317 of them
source-side (outside the canonical contract corpus), where wave 18 counted 294:

* `if` as a word: **4 files**. Three are `//` comment text inside
  `providers/quirin-provider-fluent-bit/contracts/fluent_bit/` contracts (quoted C); one is
  `docs/reports/remaining-v03/probes/S21_structured_control_flow.qcore`, an
  archived probe. `acceptance_matrix.json` records for that directory that
  "nothing in the standing gate replays them", so no pinned expectation moves.
  **No shipped fixture contains an `if` statement.**
* `else` as a word: 16 files — every one of them the extern-call adapter's
  `succeeds BOOL else … raise ERROR` tail, which is a `;`-terminated statement
  and never follows a `}`. The two uses cannot collide: `if_statement` reads
  `else` only at the token immediately after the then-block's closing brace.

---

## 2. Surface syntax

```ebnf
stmt   = "if", "(", COND, ")", block, [ "else", ( block | stmt ) ] ;
COND   = BOOL | OPERAND, CMP, OPERAND ;
BOOL   = "true" | "false" ;
OPERAND= [ "-" ], INTEGER ;
CMP    = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

The statement shape is the one the parser already accepts, unchanged. The only
parser change is that the condition's byte span is now **recorded** rather than
discarded, so a semantic pass can read it — the same architecture every other
route uses (a route reads its own statement out of a byte-span token window).

`else if` keeps its existing flattening: an `if` with `n` conditions and either
`n` branches (no trailing `else`) or `n + 1` branches (with one). Nesting an
`if` inside an arm is admitted; it is what `||`/`&&` will lower onto.

### 2.1 The condition tier, and why it stops here

The brief instructs: "do not build a general boolean expression language —
that is exactly what OR/AND-LOWERING adds later on top of your construct". The
tier above is the minimal decidable scalar comparison tier and nothing more.
It has no variables, no nesting, no parenthesised sub-conditions and no
connectives — `||` and `&&` are not part of it, by construction, because
supplying them is the next card's entire content.

`BranchCondition` is defined in `qcore-core` as

```rust
enum BranchCondition { Literal(bool), Compare { left: i32, operator: CompareOp, right: i32 } }
```

with a total `evaluate() -> bool`. `Literal` is there because `if (true)` is
the shape a lowering of a *decided* short-circuit produces, and because it is
the honest spelling of the existing `succeeds`/`passes` controls.

### 2.2 Decided, but not faked

The condition is decided at check time. The emitted C nonetheless carries the
**real** comparison — `if (INT32_C(3) < INT32_C(5)) { goto …; } else { goto …; }`
— not the decided literal. That is deliberate: emitting the literal would make
the reference/CFG/C observation agreement a tautology, whereas emitting the
comparison makes the compiled C actually compute the condition, so the
translation-validation triangle and the sanitized native run *cross-check* the
Rust evaluator against a C compiler. Measured: such a comparison compiles clean
under `-std=c11 -Wall -Wextra -Werror -pedantic`.

What is real, and what is not, stated plainly:

* **Real**: both arms exist as basic blocks in the typed CFG; both arms appear
  in the emitted C; both arms are validated for linearity, double-release and
  leaks; the arms' custody states are compared at the join; the C computes the
  condition.
* **Not real**: the *observation* is single-valued, because the condition is
  decided. This construct does not add data-dependent execution. Follow-on
  `IF-RUNTIME-CONDITION`.

---

## 3. Custody join semantics (§3 of the brief)

### 3.1 The rule

Both arms are validated from the **same** incoming custody state. They must
reach the join in states that agree on all three components:

| Component | Why it must agree |
|---|---|
| the **ordered** live-resource list | cleanup blocks release live resources in reverse acquisition order, so the order is part of the state, not a presentation detail. Comparing sequences rather than sets is the fail-closed choice. |
| the **transferred** set | `move r` and `drop r` both remove `r` from live but mean opposite things (a payload on the result versus a release). Comparing live alone would let one arm move what the other released. |
| the **returned** flag | must be `false` in both: `return` inside an arm is refused in v1 (§4). |

A disagreement is `Q046`, fail-closed, naming both arms' states.

The archetypal bug this refuses is the conditional leak / conditional
double-free:

```qcore
if (c) { drop a; }              // refused at Q046: then releases `a`, else does not
```

and it admits the correct spelling:

```qcore
if (c) { drop a; } else { drop a; }   // admitted: both arms agree
```

### 3.2 Binding scope

An arm's `let` bindings are **arm-local**: they are not visible after the join.
A resource acquired inside an arm therefore cannot be named later, and if the
arm does not release or move it the lowering validator reports the leak — the
existing `LiveResourcesAtNormalReturn` accounting, unedited, applied to the arm
(and, via §3.1, the arm could not have agreed with its sibling anyway).

At the driver, a binding whose resource is no longer live after the branch is
removed from the owner map. Because the arms agree on the live list, the
removal set is agreed too.

### 3.3 The measurement that removed this section's boundary

This section was drafted as the design's one boundary — the plan was to feed
the flat ownership / typed-error / usage / flow streams the *taken* arm's
operations and record the untaken arm as covered only by the lowering
validator. **The measurement made that unnecessary, and it is the card's
headline finding.**

Every one of those models already has a branch join, shipped, with its own
mismatch error:

| Model | Constructor | Join rule | Its refusal |
|---|---|---|---|
| ownership | `OwnershipOp::Branch { then_ops, else_ops }` | bindings, ledger and live footprint must all be equal | `BranchOwnershipMismatch` → `Q103` |
| typed errors | `ErrorOp::Branch { then_ops, else_ops }` | the normal-exit ledgers must join | `BranchNormalLedgerMismatch` → `Q302` |
| resource flow | `Flow::join_branches` | "their net changes must agree because an ownership merge may not invent an optional live resource" | `BranchNetMismatch` |
| usage | `UsageExpr::Branch` | demands are the componentwise join of the arms' | — |
| typed CFG | `CfgTerminator::TryAcquire` / `Check` | two-way terminators with real C emission | — |

So the or-operator amendment's claim that lowering onto "existing branch/CFG
constructs" needs no new semantic rules is **substantially true**, and wave
18's flat "there are none" was one level too pessimistic. The correction is
precise: of the six models a statement passes through, **five already
branched**, and the sixth — `cleanup_lowering::SourceOp`, the source lowering —
was the only linear IR without a branch constructor. That one constructor is
the whole of this card's new semantics.

**Both arms therefore pass through every checker, unedited.** The driver emits
`OwnershipOp::Branch`, `ErrorOp::Branch`, `UsageExpr::Branch` and
`Flow::join_branches` alongside the new `SourceOp::Branch`, so the join is
checked four times over by four independent models. Nothing is fed a "taken
arm" and no arm is skipped.

### 3.4 Where each disagreement is reported

Because four models check the join, a disagreement can be noticed by more than
one. Measured, on this implementation:

| Shape | Reported by | Code |
|---|---|---|
| `if (c) { drop a; }` — the conditional leak | resource flow (`BranchNetMismatch`), caught in the driver | `Q046` |
| `if (c) { drop a; } else { move a; }` | the cleanup lowering's own new join, re-presented | `Q046` |
| an arm that acquires — two arms leave different resource-identity ledgers | ownership (`BranchOwnershipMismatch`), **inherited unchanged** | `Q103` |

The first two are one condition — the arms' custody states differ — so both are
presented at `Q046` whichever model notices first. The third is a **different**
condition: the ledgers differ by identity, not by custody state, and the
ownership join is a pre-existing fail-closed gate this card does not weaken. It
keeps `Q103`, is pinned by test, and the shape is recorded as follow-on
`IF-ARM-ACQUIRE`, with its attribution quality (`Q103` lands on the function
declaration, not on the branch) as `IF-ARM-ACQUIRE-ATTRIBUTION`.

---

## 4. The arm statement subset, and the named refusals

Inside an arm, v1 admits exactly the executable-subset statements whose effect
is a custody effect:

| Arm statement | v1 | Refusal / note |
|---|---|---|
| `let n = qcore_acquire(id, BOOL, Error);` | admitted by this profile, **refused downstream** | measured: the pre-existing ownership join refuses it at `Q103` (§3.4). Recorded rather than pre-empted with a second refusal; follow-on `IF-ARM-ACQUIRE`. |
| `qcore_check(BOOL, Error);` | **yes** | |
| `drop NAME;` | **yes** | |
| `move NAME;` | **yes** | |
| a nested `if` | **yes** | required by `OR-AND-LOWERING` |
| `return …;` | no — `Q044` | follow-on `IF-ARM-RETURN`. `SourceOp::Return` must be the last operation of a program (`OperationAfterReturn`); an early return from an arm is a second exit the linear validator has no representation for. |
| a zero-argument call `f();` | no — `Q044` | follow-on `IF-ARM-CALL`. A call splits `resource_segments` and appends to a flat `calls` vector that the whole-program summary unfolds positionally; a call inside a branch has no position in that sequence. |
| anything else (`while`, `match`, `defer`, `raise`, `error region`, `with lock`, `thread scope`, `unsafe`, an expression statement) | no — `Q044` | each keeps its pre-existing refusal reason; the arm gate names the statement. |

An empty arm is admitted. `if` with no `else` is admitted (the missing arm is
the empty arm, which trivially agrees only if the present arm is also
custody-neutral — which is exactly the fail-closed behaviour wanted).

---

## 5. Route admission (§"Route interactions" of the brief)

**v1 is sequential-route only.** The construct is part of
`SourceExecutionProfile::SequentialCleanupV1`. Every other route keeps its
existing refusal, measured on the base commit rather than assumed:

| Route | `if` in v1 | Measured refusal on `c7c0a3c` |
|---|---|---|
| sequential (`SequentialCleanupV1`) | **admitted** | — |
| `source.std-io.v1` | refused | `Q030`: "`source.std-io.v1` models one flat statement sequence; this statement opens a nested block" |
| `source.extern-call.v1` | refused | the extern route's own nested-block refusal (`source_extern.rs`): "extern calls are admitted in a flat sequential entry body only; nested blocks … keep their existing refusals" |
| scoped tasks | refused | `Q500`: "statement is outside the copy-scalar task execution profile" |
| `source.mutex.guard.v1`, `source.epoch.bound.v1` | refused | their own structural refusals; pinned by test |

No checked concurrency profile is widened. "Correctly refused" is the outcome
for every one of them, and each is pinned by a test so a later widening must be
deliberate. Follow-on `IF-ROUTE-WIDENING`.

---

## 6. Diagnostics band (§5 of the brief)

A new semantic pass, `Branch`, owning **`Q044`–`Q046`**.

| Code | Condition |
|---|---|
| `Q044` | a branch shape `source.branch.v1` does not model: a statement inside an arm outside the arm subset (§4), a `return` in an arm, or an `if` whose branch/condition counts are inconsistent. Also the pass's generic presentation code. |
| `Q045` | a condition outside the decidable condition tier (§2): a name, a call, a connective (`||`, `&&`), a parenthesised sub-condition, a non-integer operand, an out-of-range integer, or an empty condition. |
| `Q046` | the arms disagree at the join (§3.1), naming both arms' custody states. |

`Q036`–`Q038` (`Pipe`) are untouched. `Q039`–`Q043` are deliberately left
free: `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` adjustment A2 reserves
"`Q039` onward" for a native-library manifest band, and the concurrently
running `UNSAFE-LINKAGE-ARTIFACT` card owns that reservation. Taking `Q044`
leaves it a five-code cushion.

A band collision discovered at integration is resolved by **relocating the
unshipped band**, per the wave-3 `T07`/`T09` precedent that wave 18 followed
when `Q032`–`Q034` became `Q036`–`Q038`. The code-to-condition mapping is
stable under such a shift.

---

## 7. Named follow-ons

| Name | What it is |
|---|---|
| `IF-RUNTIME-CONDITION` | a condition that reads a run-time value. Real prerequisite: a scalar value tier — the same missing piece as `PIPE-SCALAR-RESULT` — plus a translation-validation story for a multi-valued observation (a certificate per path, or a symbolic observation). |
| `IF-ARM-RETURN` | `return` inside an arm; needs a lowering IR with more than one exit position. |
| `IF-ARM-CALL` | a call inside an arm; needs the whole-program call summary to be branch-aware rather than positional. |
| `IF-ARM-ACQUIRE` | an acquisition inside a branch arm. Refused today by the pre-existing ownership join, because two arms acquiring different identities leave different resource-identity ledgers (§3.4). Lifting it means changing that join, which is a pinned model and a separate card. |
| `IF-ARM-ACQUIRE-ATTRIBUTION` | that refusal's span: `Q103` lands on the function declaration rather than on the branch. |
| `IF-ROUTE-WIDENING` | admitting the construct on the std-io, extern, task, mutex or epoch routes. Each is a separate checked model (§5). |
| `IF-CONDITION-BINDINGS` | comparing against an integer `let` binding rather than a literal; a strict subset of `IF-RUNTIME-CONDITION` only if the binding's value is itself decidable. Note that a name written in a condition is collected as a referenced name and refuses at `Q013` before the branch pass runs — fail-closed, and recorded rather than forced to `Q045`. |
| `SEQ-STATEMENT-TOKENS` | `qcore-verification::driver::statement_tokens` still re-lexes source bytes, the divergent tokenization path wave 18 repaired in `source_extern::statement_tokens` but not here. Inert for the condition spans this card added (a condition contains no pipe, so re-lexing and filtering agree), but it is a second tokenization path. |
| `OR-AND-LOWERING` | *(the successor card)* lowering `||` and `&&` onto this construct, and retabling the operator family so `||` is logical or, `&&` logical and, and single `|`/`&` are reserved for binary/bitwise use. **Landed.** See the addendum below. |

---

## Addendum: what `OR-AND-LOWERING` did to this design (wave 19, phase 2)

The successor card landed against this document without changing anything in
it. Recorded here so the two records read as one design rather than two:

* **The condition tier widened, and nothing else did.** §2.1's `BranchCondition`
  is byte-unchanged and `qcore-core` is untouched by that card: the connectives
  live only in a front-end tree the driver eliminates, so every checked model
  still sees the leaf tier this document specified. `A || B` becomes the
  right-nested branch of §2's own `else if` flattening; `A && B` becomes a
  branch nested in the then arm.
* **The arms are lowered once.** A connective duplicates the arms' *operation
  vectors*, not their statements, so §3's custody join runs unchanged at every
  connective level and the conditional leak refuses through any number of them.
  That is why nesting an `if` inside an arm — admitted here "because it is what
  `||`/`&&` will lower onto" — turned out to be the whole of the requirement.
* **Precedence** is declared in spec §L21 and §G02i: `&&` binds tighter than
  `||`, both left-associative, parentheses group, `|>` refused rather than
  ranked. One measured consequence: for a homogeneous chain the lowering is
  associative on the nose, so the associativity declaration is load-bearing only
  where the two connectives mix.
* **The operator table this document assumed is now the shipped one.** The
  header above spells or as `||` and and as `&&` with `|`/`&` reserved; that is
  what landed, plus `^` reserved for bitwise xor. `BITWISE-OPERATORS` is the
  follow-on.

Evidence: [`docs/reports/wave19-or-and-lowering/README.md`](../reports/wave19-or-and-lowering/README.md).

## Addendum, card `BITWISE-CONDITIONS`

The second successor card landed against this document, again without changing
anything in it. It unifies the three follow-ons that all lived in this tier —
`BITWISE-OPERATORS`, `IF-CONNECTIVE-NEGATION` and `IF-CONDITION-BINDINGS`.

* **The condition tier widened a second time, and the construct did not.** The
  three bitwise operators reach `qcore-core` — `BranchCondition::Compare` now
  carries an `IntExpr` operand tree instead of a bare `i32`, so the emitted C
  computes the bit operation itself on §2.1's own ground — but no `SourceOp`, no
  `CfgTerminator` and no checker rule was added, and a comparison of two plain
  literals emits exactly the bytes it emitted before.
* **Unary `!` is the arm swap.** `if (!C) { A } else { B }` is `if (C) { B }
  else { A }`, so it is eliminated in the front end like a connective, it costs
  nothing, `!!A` is literally `A`'s own lowering, and §3's join runs unchanged
  because a swap of two arms that must agree is a swap of equals. The
  byte-equality pin against the hand-swapped twin is the measurement.
* **The one new rule for declaring precedence.** A precedence is declared only
  where it agrees with C; where this tier's two sorts force a different ranking,
  the unparenthesized spelling is **refused** rather than silently redefined.
  That is why `(A | B) == C` is a program and `A | B == C` is not, and why
  `!(A < B)` is a program and `!A < B` is not.
* **The decidable subset of `IF-CONDITION-BINDINGS`.** `let NAME = INTEGER;` at
  the top level of a sequential entry body is a check-time constant binding,
  substituted for its value where a condition reads it. §2.1's reason for a
  decidable tier is therefore untouched: the certificate still compares one
  observation. A resource name in a condition is refused, so §3's custody
  discipline is not reached from the expression tier at all —
  `IF-RUNTIME-CONDITION` remains the follow-on that would move the boundary.
* **A `let` inside an arm is refused (`Q044`).** Widening §5's arm subset is
  card `IF-ARMS`' business, and the refusal names it.

Evidence: [`docs/reports/wave20-bitwise-conditions/README.md`](../reports/wave20-bitwise-conditions/README.md).

## Addendum: what `IF-ARMS` did to this design (wave 20)

The third successor card widened §4's arm statement subset and, in doing so,
changed the *shape* of §3's join rule. Recorded here so the three records read
as one design.

* **§4's two named refusals are gone, and their grounds with them.** This
  document refused a `return` in an arm because "`SourceOp::Return` is the last
  operation of a program", and a call because "a call takes a position in the
  flat `calls` vector the whole-program summary unfolds positionally". Both
  grounds were true; neither was load-bearing.
  * A `return` is not privileged by position but by *obligation*:
    `SourceOp::Return` demands an empty live list wherever it stands, which is
    the whole cleanup discharge the sequential route imposes at any exit. So an
    arm may return, having discharged.
  * A call was measured to emit **no C at all** in this executable subset — it
    composes the callee's summary flow and effects into the caller's and demands
    the callee's usage, and contributes nothing to the emitted program or to the
    observation. The positional call vector is still real, and is what the card
    had to answer; the observation was never the difficulty §4 assumed.
* **§3.1's rule becomes a join over survivors.** An arm that returns does not
  reach the join, so the join compares the arms that fall through — agreement,
  unweakened, wherever there are two of them; the surviving arm's own state
  wherever there is one; and a terminated branch wherever there are none. The
  complete characterisation is that the only combination with no joined state is
  two surviving arms that disagree, so nothing that refused at the join before is
  admitted at it now.
* **§3.4's table gains a row, and one moves.** An arm exit that leaks is `Q202` —
  the program exit's own condition and code, reported at the exit that leaks
  rather than at the declaration. A conditional call whose callee's summary flow
  is not neutral is `Q044`.
* **The continuation moves with the exit, and that is a front-end rewrite.**
  §3.3 recorded that five of six models already branched. Four of them still
  cannot express "this arm exited", so the driver does not hand them a one-sided
  join: the statements after the branch are lowered inside the arm that survives
  to reach them. `qcore-core`'s own rule is the general one; the rewrite exists
  for the other four. It is a **move**, never a duplication — which is why the
  shape needing the continuation in two arms is refused.
* **§4's `let n = qcore_acquire(…)` row is unchanged and deliberately so.** The
  pre-existing ownership ledger join still refuses it at `Q103`; only its span
  and wording moved. `IF-ARM-ACQUIRE` is still open, and it is what would let an
  early return stand *before* an acquisition.

Evidence: [`docs/reports/wave20-if-arms/README.md`](../reports/wave20-if-arms/README.md).
