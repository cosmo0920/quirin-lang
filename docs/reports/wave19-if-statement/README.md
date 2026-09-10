# IF-STATEMENT — the source branching construct (wave 19)

Card brief: *"QCore today has NO source branching construct. Land the construct
itself — the named prerequisite for the logical-operator lowering follow-on.
Custody joins are the heart."*

Base commit: main `c7c0a3c`.

**What landed.** `source.branch.v1` in full, on the sequential route: `if`/`else`
/`else if`, a decidable scalar condition tier, nested branching, and a custody
join checked by **four independent models** — of which **three were already
there**, alongside a fourth pre-existing model that joins the arms' demands. The
card's most consequential finding is that correction (§1), and it reverses wave
18's "there are none" to "there was exactly one missing, and it was the source
lowering". The construct emits real two-armed C11, runs clean under
ASan/UBSan/LSan against its own reference observation, and a program that does
not use it emits byte-identical C. Thirty-two new tests, 35 audited Lean
theorems, no new axiom, **zero new reserved words and zero new tokens**.

**Operator-amendment prose honored.** The future logical operators that will
lower onto this construct are spelled **`||`** (or) and **`&&`** (and)
throughout this report, the design note and the new spec/grammar sections; bare
`|` and `&` are treated as reserved for future **binary/bitwise** operations.
This card implements neither operator and reads neither token: the wave-18
`Q038` refusals are **byte-unchanged** in this diff, and pinned so by
`the_tier_has_no_connectives`. Retabling the operator family is the successor
card's business.

---

## 1. The measurement that reversed the premise

Wave 18 recorded (`docs/reports/wave18-pipe-operator/README.md` §4): *"There is
nothing to lower a short-circuit onto… Amendment 2 item 1 said to lower onto the
EXISTING branch/CFG constructs (nested branching). There are none."*

Re-measured on `c7c0a3c`, before any code was written:

| Model a statement passes through | Branch join before this card? |
|---|---|
| ownership | **yes** — `OwnershipOp::Branch { then_ops, else_ops }`, joining bindings, ledger and live footprint, refusing at `BranchOwnershipMismatch` |
| typed errors | **yes** — `ErrorOp::Branch { then_ops, else_ops }`, refusing at `BranchNormalLedgerMismatch` |
| resource flow | **yes** — `Flow::join_branches`: *"Joins mutually exclusive paths. Their net changes must agree because an ownership merge may not invent an optional live resource."* |
| usage | **yes** — `UsageExpr::Branch`, demands are the componentwise join of the arms' |
| typed CFG / C backend | **yes** — `CfgTerminator::TryAcquire` and `Check` are two-way terminators, and `c_emit` has always emitted `if (…) { goto …; } else { goto …; }` |
| **source lowering** | **no** — `cleanup_lowering::SourceOp` was exactly `Acquire`/`Check`/`Drop`/`TransferToResult`/`Return` |

So the amendment's claim was **substantially true** and wave 18's flat reading
was one level too pessimistic. Five of six models already branched; the sixth,
the *source* lowering, was the only linear IR without a branch constructor.
**`SourceOp::Branch` is the whole of this card's new semantics**, and the design
note's planned "the untaken arm is only covered by the lowering validator"
boundary evaporated with it: both arms pass through every checker, unedited.

Two smaller census corrections to wave 18's statement of the same premise:

* **The `if` statement already parsed.** `StatementKind::If { branches }` and
  `SourceParser::if_statement` were shipped, with `else` and `else if` already
  flattened. What was discarded was the *condition*: the parser found the `(`,
  jumped to its `)`, and recorded nothing between them.
* **`if (3 < 5) { }` already parsed too.** `Symbol::{EqualEqual, NotEqual,
  LessEqual, GreaterEqual, Less, Greater}` were all already lexed, so the
  comparison tier needed no lexer change.

Confirmed unchanged from wave 18: there is still no expression tier of any kind
(`StatementKind::Expression` is payload-free), and there were **zero `if`
statements in any shipped source file** (§4).

---

## 2. What the construct is

Full specification: `docs/design/SOURCE_BRANCH_V1.md`, written before the
implementation per the brief. Grammar: `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`
§G02i. Diagnostics: `docs/QCORE_SPEC_r0_3.md` §L21.

```ebnf
stmt    = "if", "(", cond, ")", block, [ "else", ( block | stmt ) ] ;
cond    = "true" | "false" | operand, cmp, operand ;
operand = [ "-" ], INTEGER ;
cmp     = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

`else if` lowers to a right-nested branch. Nesting an `if` inside an arm is
admitted — it is exactly the shape a short-circuit lowering needs, and it is
pinned (`a_branch_nests_inside_a_branch_arm`).

### 2.1 Why the condition is decided at check time, and what is nonetheless real

The executable subset's control flow has always been controlled by **literals**
written in the source: `qcore_acquire(id, true, Failure)` and
`qcore_check(false, Failure)`, with `parse_bool` accepting exactly `true` and
`false`. That is not incidental. `validate_translation` compares **one**
observation from the reference interpreter, **one** from the CFG interpreter and
**one** parsed from the compiled C program's own output; a run-time-varying
condition would leave "the" observation ill-defined and the certificate with
nothing to compare. So `source.branch.v1`'s condition tier is decidable at check
time on the same terms, and the follow-on that lifts it is `IF-RUNTIME-CONDITION`
— whose real prerequisite is the same missing scalar value tier `PIPE-CHAIN`
recorded as `PIPE-SCALAR-RESULT`.

Stated plainly, because the distinction matters:

| Real | Not real |
|---|---|
| both arms exist as basic blocks in the typed CFG | the *observation* is single-valued |
| both arms appear in the emitted C | no data-dependent execution is added |
| both arms are validated for linearity, leaks and double-release | |
| the arms' custody states are compared at the join | |
| **the emitted C computes the condition itself** | |

That last row is a deliberate design choice with a measurable payoff. The
emitter writes `if (INT32_C(3) < INT32_C(5)) { goto …; } else { goto …; }`
rather than the decided literal, so the reference/CFG/native agreement is a real
cross-check of the Rust evaluator against a C compiler instead of a tautology.
Measured: such a comparison compiles clean under `-std=c11 -Wall -Wextra
-Werror -pedantic`.

### 2.2 The condition tier stops deliberately short

The brief instructed: *"do not build a general boolean expression language —
that is exactly what the lowering follow-on adds later on top of your
construct."* The tier has no variables, no nesting, no parenthesised
sub-conditions and **no connectives**. `||` and `&&` are absent by construction
in the Rust (`BranchCondition` is `Literal | Compare`), in the Lean model, and
in the grammar, and `the_tier_has_no_connectives` pins that writing either one
inside a condition still produces exactly the wave-18 `Q038` vector.

---

## 3. The custody join

Both arms are validated from the **same** incoming state and must reach the
join agreeing on:

| Component | Why it is compared |
|---|---|
| the **ordered** live list | a cleanup block releases live resources in reverse acquisition order, so the order is state, not presentation. Comparing sequences rather than sets is the fail-closed choice. |
| the **transferred** set | `move r` and `drop r` both remove `r` from live and mean opposite things. Comparing live alone would let one arm move what the other releases — and that shape is pinned as a refusal. |
| the **returned** flag | must be false in both; `return` inside an arm is refused (§5). |

Arm `let` bindings are **arm-local**: they are not visible after the join, so a
resource acquired in an arm cannot be named later.

### 3.1 Where each disagreement is reported, measured

| Shape | Noticed by | Code |
|---|---|---|
| `if (c) { drop a; }` — the conditional leak | resource flow, `Flow::join_branches` → `BranchNetMismatch`, caught in the driver | **`Q046`** |
| `if (c) { drop a; } else { move a; }` | the cleanup lowering's own new join, re-presented into the branch pass | **`Q046`** |
| an arm that acquires | ownership, `BranchOwnershipMismatch`, **inherited unedited** | **`Q103`** |

The first two are one condition — the arms' custody states differ — so both
present at `Q046` whichever model notices first. `branch_aware_lowering_diagnostic`
does that re-presentation for the two `LoweringError` variants this card added
and for nothing else.

The third is a **different** condition and keeps its own code. This is a finding
worth stating plainly: **an acquisition inside a branch arm is refused today**,
because two arms acquiring different identities leave different resource-identity
ledgers, and `OwnershipState::branch` compares ledgers. That is a pre-existing
fail-closed gate. It was **not weakened** — the working rules forbid exactly
that — so the shape is pinned as measured
(`an_acquisition_inside_an_arm_refuses_at_the_pre_existing_ownership_join`,
`["Q103"]`) and recorded as follow-on `IF-ARM-ACQUIRE`, with its poor
attribution (`Q103` lands on the function declaration) as
`IF-ARM-ACQUIRE-ATTRIBUTION`.

### 3.2 The alternative considered and rejected

The plan drafted before measurement was to feed the flat ownership /
typed-error / usage / flow streams the **taken** arm's operations, justified by
join agreement, and record the untaken arm as covered only by the lowering
validator. §1 made that unnecessary. The second alternative — concatenating both
arms into the flat streams and requiring each arm to be *custody-neutral* — was
rejected because it refuses `if (c) { drop a; } else { drop a; }`, which is
exactly the case the brief calls the heart of the card.

---

## 4. Keyword and token census

**Zero new reserved words. Zero new tokens.** On the `EXTERN-MODES` precedent
("deliberately added none"), so the `work` → `effort` and `error` → `fatal`
respelling precedents were not needed.

| Candidate | Status on `c7c0a3c` |
|---|---|
| `if`, `else`, `true`, `false` | already in `is_reserved_source_word` |
| `==` `!=` `<` `<=` `>` `>=` | already single `Symbol` variants |

Occurrence census, all **369** `.qcore`/`.qc` files in the tree — **317** of
them source-side (outside `contract/qcore/`), where wave 18 counted 294; the
denominator grew with the tree and the finding did not change:

* **`if` as a word: 4 files.** Three are `//` comment text quoting C inside
  `contract/qcore/fluent_bit/{routes_mask,logging,hash_table_publication}.qcore`;
  the fourth is `docs/reports/remaining-v03/probes/S21_structured_control_flow.qcore`,
  an archived probe for which `acceptance_matrix.json` records that "nothing in
  the standing gate replays them". **No shipped fixture contains an `if`
  statement**, confirming wave 18.
* **`else` as a word: 16 files**, every one of them the extern adapter's
  `succeeds BOOL else … raise ERROR` tail. That tail is inside a
  `;`-terminated statement and never follows a `}`, and `if_statement` reads
  `else` only at the token immediately after a then-block's closing brace, so
  the two uses cannot collide. `shipped_fixtures_are_unmoved_by_this_card`
  compiles both a `std.io` and an extern fixture to pin it.

---

## 5. The arm subset, and the named refusals

| Arm statement | v1 | Reason |
|---|---|---|
| `qcore_check(BOOL, Error);` | **yes** | pinned |
| `drop NAME;` / `move NAME;` | **yes** | pinned |
| a nested `if` | **yes** | the shape `||`/`&&` lower onto |
| an empty arm; `if` with no `else` | **yes** | the missing arm is the empty arm |
| `let n = qcore_acquire(…);` | admitted here, **refused downstream at `Q103`** | §3.1; `IF-ARM-ACQUIRE` |
| `return …;` | no — **`Q044`** | `SourceOp::Return` is the last operation of a program; an early exit from an arm has no position in this lowering. `IF-ARM-RETURN` |
| a call `f();` | no — **`Q044`** | a call takes a position in the flat `calls` vector the whole-program summary unfolds positionally. `IF-ARM-CALL` |
| `while`, `match`, `defer`, `raise`, … | no — each keeps its own code | `while` inside an arm is pinned as still `Q500` |

---

## 6. Route admission

**Sequential route only** (`SourceExecutionProfile::SequentialCleanupV1`). No
checked concurrency profile was widened; "correctly refused" is the outcome for
every other route, measured on the base commit and pinned by test.

| Route | Refusal |
|---|---|
| `source.std-io.v1` | `Q030` — "`source.std-io.v1` models one flat statement sequence; this statement opens a nested block" |
| `source.extern-call.v1` | the extern route's own nested-block refusal — "extern calls are admitted in a flat sequential entry body only" |
| scoped tasks | `Q500` — "statement is outside the copy-scalar task execution profile" |
| `source.mutex.guard.v1`, `source.epoch.bound.v1` | their own structural refusals; not pinned by this card's suite (they are reached only through their own profile markers) — recorded as a coverage gap, not a claim |

Follow-on `IF-ROUTE-WIDENING`.

---

## 7. Diagnostics band

A new semantic pass, `Branch`, owning **`Q044`–`Q046`**, through the §L21
process, with spec §L21 and grammar §G02i updated in the same change.

| Code | Condition |
|---|---|
| `Q044` | a shape `source.branch.v1` does not model — a `return` or a call inside an arm, or inconsistent block/condition counts. Also the pass's generic presentation code. |
| `Q045` | a condition outside the decidable tier |
| `Q046` | the arms do not reach the join in the same custody state |

`Q036`–`Q038` (`Pipe`) are untouched in every file. **`Q039`–`Q043` are
deliberately left free**: `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` adjustment
A2 reserves "`Q039` onward" for a native-library manifest band, and the
concurrently running `UNSAFE-LINKAGE-ARTIFACT` card owns that reservation.
Taking `Q044` leaves it a five-code cushion.

**A band collision discovered at integration is resolved by relocating the
unshipped band**, per the wave-3 `T07`/`T09` precedent that wave 18 itself
followed when `Q032`–`Q034` became `Q036`–`Q038`. The code-to-condition mapping
is stable under such a shift; the three conditions above move together.

One further note for the integrator: `statement_metadata` now maps
`StatementKind::If` to `SemanticPass::Branch` (it was `Usage`). `match` and
`while` are untouched and keep `Usage`, because this card claims neither.

---

## 8. Evidence

### 8.1 Tests

`compiler/tests/branch_statement.rs` — **32 tests, all passing**, one of them a
sanitized native run. Two new fixtures under `compiler/tests/fixtures/branch/`:
`conditional_release.qcore` and its hand-written unbranched twin.

### 8.2 The deliberate pin update

**One pinned expectation moved, and it was moved deliberately rather than
relaxed.** `compiler/tests/unit/driver.rs::reports_unsupported_source_at_the_owning_file`
used `fn main() -> i32 { if (true) { return 1; } }` as an arbitrary unsupported
construct and asserted `Q500` at the owning file. `source.branch.v1` now reads
that `if`, so the program refuses at `Q044` instead.

The fix keeps the test's *claim* exactly where it was and moves only its
fixture: `while (true) { return 1; }` is still outside the executable subset,
still reports `Q500`, and still exercises the file attribution the test is named
for. What the old fixture does now is pinned separately by
`an_early_return_from_a_branch_arm_refuses` in the new suite. **No assertion was
weakened and no gate was downgraded**; the diff is one string plus a comment
recording why.

The `if` in `docs/reports/remaining-v03/probes/S21_structured_control_flow.qcore`
is the same program. That archive is **not edited** (frozen archives are never
edited in place, and it is outside this card's directory anyway), and
`acceptance_matrix.json` already records that nothing in the standing gate
replays those probes.

### 8.3 Byte-identity for programs that do not use the construct

Block ids are allocated by a pre-order walk — an operation takes the next id,
then its `then` arm, then its `else` arm, and cleanup blocks start at the total
operation count. On a **branch-free** program that is exactly the pre-card
numbering, so nothing shipped moves a byte.
`a_branch_free_program_emits_byte_identical_c` pins the six `q_bb_` labels and
three `q_cleanup_` labels of a two-acquisition program and asserts no tenth
block appeared. The Lean counterpart is
`a_branch_free_operation_takes_one_consecutive_block` plus
`the_branch_free_program_is_numbered_from_zero`, with
`a_branch_allocates_its_arms_after_itself` showing the claim is about
branch-free programs rather than vacuous.

The top-level continuation is `BlockId::MAX`, deliberately an id no block
carries: a validated program's last top-level operation is `Return` so it is
unreachable, and were it ever taken `TypedCfg::new` would refuse with
`MissingBlock` rather than wire an edge somewhere plausible.

### 8.3.1 What an `else if` chain actually emits

Reproducible from the CLI, and worth showing because it is the shape the
successor card's short-circuit lowering will produce:

```c
q_bb_1:
    if (INT32_C(3) > INT32_C(5)) { goto q_bb_2; } else { goto q_bb_3; }
q_bb_2:
    q_release(&q_res_1, UINT64_C(1));
    goto q_bb_6;
q_bb_3:
    if (INT32_C(2) == INT32_C(2)) { goto q_bb_4; } else { goto q_bb_5; }
q_bb_4:
    q_release(&q_res_1, UINT64_C(1));
    goto q_bb_6;
q_bb_5:
    q_release(&q_res_1, UINT64_C(1));
    goto q_bb_6;
q_bb_6:
```

A right-nested branch, three arms, one join. Compiles clean under
`-std=c11 -Wall -Wextra -Werror -pedantic` and runs. The nested-`if`-inside-an-arm
shape emits the same way and also compiles and runs clean.

### 8.4 Sanitizer evidence — reported honestly

`the_native_run_of_a_branch_matches_its_reference`, x86_64 Linux,
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. The program compiles clean
under `-Werror` — including the constant comparison the emitter writes — runs
with exit 0, and its parsed observation equals the reference observation exactly.

The leak check is a **positive** control: the program allocates and releases,
and the release it performs is the one inside a branch arm.

**Honest scope.** One arm executes; the other is compiled, linked and reachable
from the emitted `goto` table, but the decided condition means it is not entered
at run time. The untaken arm's evidence is static — it is validated by every
models and pinned by the refusal tests in §3 — not dynamic. That is the
boundary `IF-RUNTIME-CONDITION` would move.

### 8.5 Lean

`proof/QCore/BranchJoin.lean`, audited by `proof/AuditBranchJoin.lean`: **35
theorems**, no new axiom and no placeholder — every one depends on nothing, or
only on the allowed `propext` / `Quot.sound`. No `native_decide`. `QCore.lean`
imports the module, so `AuditAll.lean` includes it in the full axiom closure.
`check_lean_audits.py` moves **80 → 81** audits.

Load-bearing statements, by family:

* **The condition tier** — `every_condition_has_a_value` (totality),
  `evaluation_is_state_free` (the property the single-observation certificate
  rests on, stated rather than assumed), `the_comparisons_are_the_relations`.
* **The join** — `the_join_is_the_common_state`, `the_join_widens_neither_arm`
  (the joined state is *both* arms' state, so nothing is lost and nothing
  invented), `disagreeing_arms_do_not_join`, `a_returning_arm_does_not_join`,
  and `the_join_rule_is_exactly_agreement` as a biconditional. The join rule is
  factored out as `joinArms` and *used by* the validator, so these are theorems
  about the rule actually applied rather than about a paraphrase.
* **Fail-closed, concretely** — the conditional leak, move-versus-release, a
  leak in the arm the decided condition does **not** take, a `return` inside an
  arm, and two arms spending one acquisition identity; plus
  `refused_shapes_are_exactly_these` as one list, so a future widening must
  change a theorem rather than slip past one, and `both_arms_releasing_is_admitted`
  as the positive control.
* **Neutrality** — `a_branch_free_operation_steps_exactly_as_before` and
  `runSeq_is_linear_on_branch_free` against a `stepLinear`/`runLinear` pair
  written **independently** of the branch-aware ones (they have no branch case
  and refuse one outright), with
  `a_branch_is_the_one_operation_they_disagree_on` making the claim non-vacuous.
* **Block allocation** — monotonicity, `a_branch_precedes_its_arms`,
  `a_branch_free_operation_takes_one_consecutive_block`, and the two concrete
  numberings of §8.3.
* **The observation** — `the_observed_arm_is_the_one_the_condition_selects`,
  the two polarities, `a_comparison_condition_selects_by_its_relation`,
  `an_empty_arm_changes_nothing`, and
  `the_branch_observes_what_its_hand_written_twin_does` paired with
  `the_twin_is_branch_free_and_the_branched_program_is_not`.

**Not proved:** that the Rust parser extracts these operations from real source
the way the model says (a tested Rust boundary, per the
`ConcurrencyPrelude` / `PipeDesugaring` precedent); that a C compiler evaluates
the emitted comparison as `CompareOp.apply` does (that is what the sanitized
native run cross-checks); and anything about `||` or `&&`, which are absent from
the model's condition tier by construction. The model's `transferred` set uses
sorted insertion so list equality *is* set equality, matching the Rust
`BTreeSet`; that correspondence is by inspection.

### 8.6 Corpus neutrality

**Corpus-neutral by construction.** The whole diff is:

```
crates/qcore-core/src/cleanup_lowering.rs   (SourceOp::Branch, the join, the interpreter)
crates/qcore-core/src/cfg.rs                (CfgTerminator::Branch)
crates/qcore-core/src/c_emit.rs             (its C11 emission)
crates/qcore-frontend/src/frontend.rs       (condition spans, SemanticPass::Branch)
crates/qcore-verification/src/driver.rs     (the recursive lowering)
compiler/tests/branch_statement.rs          (new suite)
compiler/tests/fixtures/branch/             (two new fixtures)
compiler/tests/unit/driver.rs               (the one deliberate pin update, §8.2)
docs/QCORE_SPEC_r0_3.md                     (§L21, one row)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md     (§G02i, and one stale claim in §G02h)
docs/design/SOURCE_BRANCH_V1.md             (new)
proof/QCore/BranchJoin.lean, proof/AuditBranchJoin.lean, proof/QCore.lean
```

**No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`contract/qcore/` or `providers/` is touched.** No CT manifest entry changes and
no pinned artifact is edited. The contract corpus stays **69 modules**, CT stays
**33/50**, the 185-unit baseline and its leak count of **272** are untouched,
and nothing is promoted. The one place a corpus effect could hide is the
frontend — and the only frontend change is that `StatementKind::If` gained a
`conditions` field and a pass label, neither of which any gateway or audit path
reads.

### 8.7 CT candidates

None proposed. The construct adds a checked model, but mapping it would need a
conformance case whose artifact the case's own named workspace test runs; that
is a mapping lane's work, not this card's, and growing the denominator without
adding evidence is exactly what constraint 23 forbids.

---

## 9. Deviations and findings

1. **Wave 18's premise was one level too pessimistic** (§1). Four of the models
   already had branch joins and the CFG already had branch terminators; only
   `SourceOp` did not. Recorded as the correction it is, and it made the design
   note's planned untaken-arm boundary unnecessary.
2. **The `if` statement and the comparison tokens already parsed** (§1). The
   parser needed one change — record the condition's span instead of discarding
   it — and the lexer needed none.
3. **An acquisition inside an arm is refused by the pre-existing ownership
   join** (§3.1). Measured, not designed, and deliberately not weakened.
   `IF-ARM-ACQUIRE`.
4. **A name in a condition refuses at `Q013`, not `Q045`** — name resolution
   collects it as a referenced name before the branch pass runs. Fail-closed and
   pinned as measured rather than forced to the preferred code, on the
   `STD-MODULES` precedent for unreserved words in a rendering position.
5. **One pinned expectation moved, deliberately** (§8.2), with its claim
   preserved and its old fixture's new behaviour pinned separately.
6. **The join is reported from two codes, not one, and that is stated rather
   than smoothed over** (§3.1): `Q046` for a custody-state disagreement wherever
   noticed, `Q103` for the different ownership-ledger condition.
7. **`statement_metadata` moved `If` from `Usage` to `Branch`** (§7).
8. **`qcore-verification::driver::statement_tokens` still re-lexes source bytes**
   (`lex(&source[span])`), the divergent tokenization path wave 18 repaired in
   `source_extern::statement_tokens` but not here. It is reachable for the
   condition spans this card added. It is *inert* for them — a condition
   contains no pipe, so re-lexing and filtering agree — but it is a second
   tokenization path and it is recorded rather than repaired, being outside this
   card's scope. Follow-on `SEQ-STATEMENT-TOKENS`.

---

## 10. Named follow-ons

| Name | What it is |
|---|---|
| `IF-RUNTIME-CONDITION` | a condition that reads a run-time value. Prerequisites: a scalar value tier (the same missing piece as `PIPE-SCALAR-RESULT`) **and** a translation-validation story for a multi-valued observation — a certificate per path, or a symbolic observation. |
| `IF-ARM-RETURN` | `return` inside an arm; needs a lowering IR with more than one exit position. |
| `IF-ARM-CALL` | a call inside an arm; needs the whole-program call summary to be branch-aware rather than positional. |
| `IF-ARM-ACQUIRE` | an acquisition inside an arm, refused today by the ownership ledger join (§3.1). Lifting it changes a pinned model. |
| `IF-ARM-ACQUIRE-ATTRIBUTION` | that refusal's span: `Q103` lands on the function declaration, not the branch. |
| `IF-ROUTE-WIDENING` | admitting the construct on the std-io, extern, task, mutex or epoch routes. Each is a separate checked model. |
| `IF-CONDITION-BINDINGS` | comparing against an integer binding rather than a literal (§9.4). |
| `SEQ-STATEMENT-TOKENS` | the sequential driver's re-lexing tokenization path (§9.8). |
| `OR-AND-LOWERING` | *(the successor card)* lowering `||` and `&&` onto this construct, and retabling the operator family so `\|\|` is logical or, `&&` logical and, and single `\|`/`&` are reserved for binary/bitwise operations. **This card's construct is that card's prerequisite, and it is now in place.** |

---

## 11. Gate

| Gate step | Result |
|---|---|
| `cargo test -p qcore-compiler --test branch_statement` | **32 pass**, including the sanitized native run |
| `cargo test --workspace --no-fail-fast` | **3,017 passed, 0 failed, 8 ignored** across 181 targets, exit 0 — plus the one test added after that run, §11.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `python3 scripts/check_lean_audits.py` | **81/81 pass** (main's 80 + `AuditBranchJoin`); no new axiom |
| `python3 -m unittest discover -s scripts/tests` | **192 pass, 1 fail** — the derived Lean-audit-count tripwire, §11.2 |
| `reevaluation_probe` / `reevaluation_followup` | **exit 0 / exit 0** |

### 11.1 Workspace, probes, and two environment findings

**Finding 1 — the fresh-worktree submodule pin, recorded rather than worked
around.** The first workspace run reported two failing targets,
`-p qcore-compiler --test libevent_submodule` and
`-p qcore-compiler --test profile_loader`. Both are the pinned
`third_party/libevent` revision check in a worktree where the submodule was
never initialized — `git submodule status` showed the leading `-`. This is
exactly what waves 18 and 19 each recorded in the same situation. **The pin was
not weakened**: the standing gate's own
`git submodule update --init --recursive` was run (checking out
`d82464a277d0f42703702c4dfd9af6af38595a83`, `release-2.2.2-alpha-11-gd82464a2`,
the declared revision) and the whole suite re-run from clean. No file this card
touches is reachable from either target.

**Finding 2 — machine contention.** A sibling agent's gate was running against
the primary checkout for most of this card's closing run, and the host was under
memory pressure (swap fully committed). Wall-clock timings here are therefore
not comparable with a quiet run, and constraint 16's four load-sensitive rows
are re-verified in isolation if they trip. Nothing about the results depends on
either fact.

**The numbers.** The clean post-submodule run is **3,017 passed, 0 failed, 8
ignored across 181 targets, exit 0**. Against the wave-19 partial checkpoint's
2,986 that is exactly `+31`, this card's suite as it stood at that run.

**One test was added after that run** and is reported separately rather than
folded in silently: `a_branch_is_admitted_in_a_non_entry_function`, covering a
branch in a *leaf* function, whose semantics reach `check_whole_program`'s
summary unfolding rather than emission. The gap was found by hand at the end,
so it is pinned rather than left. The branch suite was re-run in isolation at
**32 pass, 0 failed**, and `cargo fmt --all -- --check` and
`cargo clippy --all-targets -- -D warnings` were both re-run clean afterwards.
The composed workspace figure is therefore **3,018**, and the honest form of
the claim is that 3,017 was measured whole and one test was added and verified
in isolation on top of it.

**None of constraint 16's four load-sensitive rows tripped**, despite a load
average of 17–23 for the whole run. No test was re-run to make it pass.

### 11.2 Reconciliations the integrator must make

`docs/reviews/r0_3/acceptance_matrix.json` carries derived
`validation.lean_audits` and `validation.lean_audit_files` rows, checked by
`scripts/tests`. This card adds `proof/AuditBranchJoin.lean`, so the composed
truth becomes **81** and the file list gains `AuditBranchJoin.lean` in sorted
position. **That file is outside this card's write scope** (the brief names it
explicitly), so it is left stale: `python3 -m unittest discover -s scripts/tests`
reports **192 pass, 1 fail** — exactly
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`,
asserting `['14.0.0', 80] != ['14.0.0', 81]`. That is the tripwire doing its
job, not a defect, and it is the same situation wave 18's `PIPE-OPERATOR` left
before its own integration follow-up. Reconciling the count to 81 and adding the
file name in sorted position clears it.

Two more, both in files this card may not write:

* **`AGENTS.md`** — the "Diagnostics namespaces" summary table needs a
  `Q044`–`Q046` / `Branch` row, and the ledger's Lean-audit count moves
  **80 → 81**. Spec §L21, which that table summarises, is already updated here.
* **`DEVELOPMENT.md`** — one increment paragraph; a draft is §12.

## 12. Draft increment paragraph

> Source branching (IF-STATEMENT): `source.branch.v1` lands the construct QCore
> had never had — `if (COND) { … } else { … }`, `else if`, nested branching, a
> decidable scalar condition tier, and a custody join — on the **sequential
> route only**, with every other route's refusal measured and pinned rather than
> widened. The card's headline is a correction to wave 18's premise: the models
> **already branched**. `OwnershipOp::Branch`, `ErrorOp::Branch`,
> `UsageExpr::Branch` and `Flow::join_branches` were all shipped, with their own
> mismatch errors, and the typed CFG has carried two-way terminators since Cycle
> 7; the one linear IR with no branch constructor was
> `cleanup_lowering::SourceOp`, and adding it is the whole of this card's new
> semantics — so **both arms pass through every existing checker, unedited**, and
> the planned "untaken arm" boundary evaporated. The join is **agreement, never
> widening**: both arms are validated from the same incoming state and must reach
> the join with equal ordered live lists and equal transferred sets, so the
> conditional leak refuses at `Q046` (through the pre-existing
> `Flow::join_branches` net check) and so does the shape where one arm moves what
> the other releases — which comparing live resources alone would have admitted.
> An acquisition inside an arm is refused by the **pre-existing** ownership
> ledger join at `Q103`, measured and deliberately not weakened (`IF-ARM-ACQUIRE`).
> The condition is decidable at check time for the same reason `succeeds` and
> `passes` are — the certificate compares one reference, one CFG and one compiled
> observation — but the branch is real: both arms are emitted as basic blocks and
> the C **computes the comparison itself**, so one sanitized ASan/UBSan/LSan run
> cross-checks the Rust evaluator against a C compiler with a positive leak
> control. Block ids are allocated pre-order, which is the identity numbering on a
> branch-free program, so nothing shipped moves a byte. **Zero new reserved words
> and zero new tokens** (`if`, `else`, `true`, `false` and all six comparison
> symbols were already there; `if` appears in no shipped source file), on the
> `EXTERN-MODES` precedent. `Q044`–`Q046` under a new `Branch` pass through the
> §L21 process, deliberately leaving `Q039`–`Q043` free for the native-library
> band. One pinned expectation moved **deliberately**: the driver test whose
> fixture was `if (true) { return 1; }` keeps its claim and changes its fixture to
> `while`, with the old fixture's new `Q044` behaviour pinned separately. 32 new
> tests, 35 audited Lean theorems, no new axiom, 80 → 81 audits. Corpus-neutral by
> construction: no gateway, audit, contract or provider file touched; corpus stays
> 69 modules, CT 33/50, leak 272, nothing promoted. This is the named prerequisite
> for the `||`/`&&` lowering follow-on, and it is now in place. Evidence:
> `docs/reports/wave19-if-statement/README.md`.
