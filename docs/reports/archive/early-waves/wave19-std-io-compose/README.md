# STD-IO-COMPOSE — one module that both calls and prints (wave 19)

Card brief: *"A module that both calls an extern function and prints stays on
the extern route today, where `print` refuses at `Q500`. Your job is the checked
composition of the `source.std-io.v1` and `source.extern-call.v1` routes, so one
module can do both — completing the user's literal stream chain
`value |> render |> log`."*

Pre-existing follow-on of `STD-MODULES`
([`docs/reports/wave17-std-modules/README.md`](../wave17-std-modules/README.md)
§7) and the second of the two boundaries `PIPE-CHAIN`
([`../wave19-pipe-chain/README.md`](../wave19-pipe-chain/README.md) §3)
measured under `value |> render |> log`. The first was retired by
`PIPE-SCALAR-RESULT`. Base commit: main `5c1f949`.

**What landed.** A new route `source.std-io-extern.v1`, built as a **union of
two checked models rather than a third one**: a composed program carries an
`ExternProgram` and a `StdIoProgram` side by side plus the order that
interleaves them, and the composed checker runs `check_extern_program` and
`check_std_io_program` **unmodified** before checking anything of its own. The
established sequential passes still re-check the extern half. One diagnostic
code, `Q043`, owns the three join rules and nothing else. Beside it, two
widenings of the extern route's *value* namespace — the scalar literal binding
`EXTERN-SCALAR-LITERAL` the brief folded in, and the declared outcome
`succeeds INTEGER` the payoff turned out to need (§3).

**The payoff landed end to end.** `value |> render |> log` compiles in its
literal reading, emits **byte-identical C11** against its hand-written twin, and
its sanitized native run byte-compares against reference execution **per
stream**, with the fixture allocating and releasing a real buffer so the leak
check is a *positive* control.

---

## 1. BEFORE — the census the brief asked for

Measured by compiling each shape on main `5c1f949`, not read off the source.
Raw output in this card's `measure/` (worktree-private, per `CLAUDE.md`); the
codes are reproduced here in full.

| # | Module | BEFORE |
|---|---|---|
| C1 | the module `PIPE-CHAIN` measured: an acquisition, `let total = 42;`, `print dec total;`, a release | `Q025` "an extern `let` must read either an acquisition … or a scalar-result binding" **+** `Q500` "the extern-call profile admits `CALLEE(ARGS);`, an acquisition `let` and a bare `return;`" |
| C2 | an extern call and `print dec 7;` | `Q500`, same text |
| C3 | `let n = 42;` on the extern route | `Q025` **+** `Q024` "`n` is not a bound scalar in this body" |
| C4 | standard IO alone | accepted, `StandardIoV1` |
| C5 | extern calls alone | accepted, `ExternCallsV1` |
| C6 | the literal chain, both features | `Q025` + `Q024` + `Q500` |
| C7 | `print` inside a `thread scope` | `Q600` |
| C8 | an extern call inside a `thread scope` | `Q600` |
| C9 | a bound call result and `log info dec n;` | `Q500` |
| C10 | a discarding call and `flush;` | `Q500` |

C1 reproduces `PIPE-CHAIN`'s own measurement exactly, which is what the brief
asked for as the BEFORE.

**The route-selection decision, as it stood.** `driver.rs` selects in this
order: composed epoch → epoch → mutex → tasks → extern → standard IO → the
sequential route. The extern selector's own comment was the boundary — *"a
module that both prints and spawns, or both prints and calls an extern function,
stays with the more specific model, which keeps its existing refusal … Lifting
either composition is a separate card"* — and this is that card, for one of the
two pairs.

Two things the census establishes that the card's framing did not assume.

1. **`Q500` is the refusal, not `Q028`.** `print dec total;` reaching the extern
   route is a `StatementKind::Expression` whose tokens are not a call, so the
   extern lowering's *unsupported-construct* arm refuses it under the
   `CEmission` pass. The standard-IO codes were never reached at all: composing
   is not a matter of relaxing `Q500`, it is a matter of the statement reaching
   a model that owns it.
2. **The two feature sets are told apart with no new predicate.** Route
   selection already had `has_source_extern` and `has_source_std_io`, and both
   were already exact — declaring an extern function is not calling one, and
   importing `std.io` is not printing. The composed selector is their
   conjunction and nothing else, which is why the "a module using one keeps its
   route" property is structural rather than tested into place (§4).

## 2. AFTER — the same shapes, re-measured

| # | AFTER |
|---|---|
| C1 | **accepted**, `ComposedStdIoExternV1` |
| C2 | **accepted**, `ComposedStdIoExternV1` |
| C3 | **accepted**, `ExternCallsV1` — the literal binding is the extern route's own widening, and composing is not required for it |
| C4 | **unchanged**: accepted, `StandardIoV1` |
| C5 | **unchanged**: accepted, `ExternCallsV1` |
| C6 | `Q043` "`qcore__pipe0` is bound to a call result this module models no number for, so rendering it would publish a line no model computed; write `let … succeeds INTEGER;` …" |
| C7 | **unchanged**: `Q600` |
| C8 | **unchanged**: `Q600` |
| C9 | `Q043`, the same rule naming `n` |
| C10 | **accepted**, `ComposedStdIoExternV1` — `flush` renders nothing, so it needs no operand and no number |
| C11 | the payoff with its declared outcome, `value \|> qcore_extern_scale(2) succeeds 84 \|> log info dec;` — **accepted** |
| C12/C13 | its hand-written twin — accepted, and `c_source` **and** the reference observation compare **equal** |

C6 and C9 are the composition's own new refusal and are the honest answer to a
question neither composed model could ask. See §3.

## 3. The one thing the payoff needed that no card had

The brief's payoff is a *rendered* extern result. `PIPE-SCALAR-RESULT` §4.3
recorded, correctly, that the number a bound call returns is **not observed**:
"a reference interpreter cannot compute a C function's result, so observing it
would make the two runs disagree by construction." A rendering of such a value
would therefore publish a line the reference execution cannot predict, and the
per-stream byte comparison the card demands would be unwritable.

Two honest answers existed. Refuse the rendering and record the payoff as still
unreachable; or give the site a way to *declare* the number, on the mechanism
this profile already has for exactly this problem. This card does **both**, in
the fail-closed order:

* **A bound call result with no declared number is not a renderable operand**
  (`Q043`). It stays a perfectly good value — it may still be passed at a
  declared scalar parameter, unchanged from `PIPE-SCALAR-RESULT`. What it may
  not do is become a line.
* **`succeeds INTEGER` declares it**, and is *the same construct* the
  acquisition adapter already spells with that word. `succeeds BOOL` declares
  which arm a nullable owned return takes; `succeeds INTEGER` declares which
  number a scalar result carries. Both are fixture-literal outcome controls in
  the `qcore_acquire`/`qcore_check` sense — a controlled model operation, never
  a claim about a production callee — and, decisively, **neither is believed by
  the emitted program**:

```c
int32_t v1 = qcore_extern_sum(r1, 8);
(void) v1;
if (v1 != 56) {
    QCORE_IO_WRITE(stderr, "declared outcome violated: qcore_extern_sum -> %ld\n", (long) v1);
    return 70;
}
```

That is the existing adapter's runtime comparison, verbatim in shape, at status
70. It is what makes a rendered line comparable against a native run at all: if
the linked C stops returning what the source declared, the program fails loudly
instead of observing a different number.

**No reserved word was added**, which the brief expected. The two tails are
disambiguated by the *literal kind* — `true`/`false` versus an integer — so a
malformed adapter tail can never be read as a scalar outcome and `Q025` keeps
its exact shape. The constraint-14 keyword census was therefore not needed and
no fixture surface moves.

This is a deviation from the brief's scope and is named as one: the follow-on
`EXTERN-SCALAR-OUTCOME` that `PIPE-SCALAR-RESULT` recorded is **discharged for
the scalar-binding shape** by this card, because the payoff pin cannot hold
without it. What it does not do is observe the number in the *trace*: the
`value` observation line still names the callee, the value and the arguments and
not the result, so `EXTERN-SCALAR-OUTCOME`'s trace half stays open (§8).

## 4. The route-selection decision table

Selection order, after this card:

```
composed epoch → epoch → mutex → tasks → COMPOSED → extern → standard IO → sequential
```

| Module uses | Route | Why |
|---|---|---|
| extern calls only | `ExternCallsV1` | the composed conjunction is false; falls through **byte for byte** |
| `print`/`log`/`flush` only | `StandardIoV1` | same, in the other direction |
| declares an `extern fn` but never calls it, and prints | `StandardIoV1` | declaring is not calling — `has_source_extern` was already exact |
| imports `std.io` but never prints, and calls | `ExternCallsV1` | importing is not using — `has_source_std_io` was already exact |
| **both** | `ComposedStdIoExternV1` | this card |
| prints **and** spawns | task route, refuses `Q600` | selected earlier; unchanged |
| extern-calls **and** spawns | task route, refuses `Q600` | selected earlier; unchanged |
| all three | task route, refuses `Q600` | selected earlier; unchanged, and pinned as its own test |

The composed selector is placed **after every concurrent route and before both
routes it composes**. That single placement is what makes the addition safe:
every concurrency exclusion the route-selected-last discipline of `STD-MODULES`
established still holds, because a concurrent module never reaches this line.

**"A module using one keeps its existing route byte-identically" is structural,
not merely tested.** Both single-model lowerings are now
`lower_source_*_skipping(…, skip)` with the old entry point defined as the same
call with an **empty** skip list, so a single-model module travels the identical
code path — there is no second lowering to drift. The tests pin the observable
consequence anyway: `a_printing_module_keeps_the_standard_io_route_byte_for_byte`
asserts the standard-IO route still declares its own `s_NAME` copies, and
`an_extern_calling_module_keeps_the_extern_route_byte_for_byte` asserts the
extern route's prologue is unchanged and carries no `<inttypes.h>`.

## 5. The composition, stated exactly

### 5.1 It is a union, and here is what that buys

```rust
pub struct ComposedProgram {
    pub entry: String,
    pub externs: ExternProgram,      // checked by check_extern_program, unedited
    pub io: StdIoProgram,            // checked by check_std_io_program, unedited
    pub order: Vec<ComposedStep>,    // Extern(i) | Io(j) | Return
    pub operand_values: BTreeMap<String, ValueId>,
    pub value_abi: BTreeMap<ValueId, AbiType>,
}
```

Nothing in `composed_std_io.rs` re-implements a custody rule, an adapter rule,
an effect rule or a rendering rule, and nothing there *can* weaken one, because
neither projection is constructed by that module. Concretely, a composed module
gets:

* every `Q022`–`Q027` refusal, from the extern checker, at the extern site —
  including the mandatory nullable-owned-return adapter, with the `Q023`
  contract-selection default **unmoved** (a module with no selected environment
  is refused exactly as before: composing buys no callee);
* every `Q028`–`Q031` refusal, from the standard-IO checker, at its own site,
  with the structural effect discipline intact — `StdIoOperand` still has two
  constructors and neither can denote a resource, so a rendering still cannot
  consume, release, store or retain custody;
* the established sequential passes over the extern lowering's semantics —
  leaks, use-after-move and the effect row — run by `compile_composed_source`
  exactly as `compile_extern_source` runs them;
* the `UncheckedLinkage` publication tripwire, asserted **twice** on the
  composed path, once against the extern certificate and once against the copy
  the composed certificate carries.

The composed *certificate* carries both projections' certificates verbatim, and
`recheck_composed_certificate` rechecks each of them by **that route's own
recheck** before rederiving the join. Four tamper directions are pinned:
editing the extern half, the standard-IO half, the interleaving or the rendered
values each fails the composed recheck.

### 5.2 The lowering owns no statement

Every statement of a composed entry is lowered by one of the two existing
lowerings, unedited. `source_composed.rs` decides only *which*, by the statement
head alone: `print`/`log`/`flush` are the standard-IO model's, `return;` is the
single return **both** models require (one source statement, one composed step,
two projections' `Return` events), and everything else is the extern model's.
Each lowering runs with the other's statements skipped.

The interleaving is then read off the source statement order rather than
reconstructed: both lowerings publish one span per event, so a statement is the
extern event with its span, the standard-IO event with its span, or the shared
return. A statement that produced an event in **neither** is refused at `Q043`
rather than silently dropped — the case that would have checked a program the
emitted C does not perform.

### 5.3 The three join rules

1. **The order is an interleaving.** Every event of both projections appears
   exactly once, in that projection's own order, ending in the shared return.
2. **A rendered value is bound earlier, is renderable, and its number is
   modeled.** The extern model owns the value namespace, so a rendering
   operand's name resolves against *its* bindings — never a resource identity,
   which has no operand form at all. The value must be bound by an earlier
   composed step, must be of the one ABI width the two renderings are total on
   (`RENDERABLE_ABI = i32`; the two renderings are the signed decimal and the
   unsigned reinterpretation of the *same 32 bits*, so any other width would be
   rendered through a conversion that does not describe it), and must carry a
   number the source determined.
3. **Nothing is dropped at the join.** The composed effect row is the **union**
   of the two derived rows and must be admitted by the one row the entry
   declares; the composed observation must reproduce both projections' own
   observations, per stream, and a leftover line on either side is a refusal —
   unless the run *raised*, in which case the lines after the raise are exactly
   the ones the emitted program does not write (§6.7).

### 5.4 One lowering per model in the backend too

`emit_composed_c11` lowers nothing itself. The call statements come from
`extern_c_emit::emit_extern_statements` and the rendering statements from
`std_io_c_emit::event_statement` — the same functions the two single-model
routes now use — so a composed module's call statements are byte-identical to
the extern route's and its rendering statements byte-identical to the
standard-IO route's **by construction rather than by comparison**. The one thing
the composed backend supplies is the operand expression: a composed rendering
reads `vN`, the object the extern model's own statement declared, never a fresh
copy.

## 6. Deviations and boundaries, found by measurement

1. **`Q029` and the composed effect-union arm are unreachable on this route.**
   `derived_effects` stamps `Io` on **every** extern call unconditionally — the
   measurement `STD-MODULES` §1.3 recorded — so an entry missing `io` is refused
   by the extern half's own per-call gate first. Removing `io` from the payoff
   fixture measures `["Q025", "Q025", "Q025"]`, and the test pins that vector
   **and** the absence of `Q029` and `Q043`, on the `PIPE-SCALAR-RESULT` §2.3
   precedent: both arms are kept as fail-closed backstops, and if the
   reachability ever changes the test says so rather than passing silently.
2. **`Q043`'s ordering arm is likewise beneath an earlier gate.** A rendering
   that names a binding written later refuses at name resolution's `Q013`,
   because that gate runs over the parsed body before any lowering. The arm is
   kept for the same reason.
3. **A declared `resource_out` callee still refuses at `Q025`, not `Q043`.**
   `qcore_extern_open` declares `resource_return_none()` with an `i32` status —
   its result *is* a scalar and it still acquires — so `binds_scalar` is false
   and the extern route refuses it before the join is consulted. That is the
   case a rule written over the result alone would have got wrong, and the test
   pins that the two refusals are told apart.
4. **One wave-19 pin was updated, deliberately, in both halves.**
   `pipe_operator.rs`'s `a_pipeline_temporary_cannot_be_named_or_shadowed`
   asserted that a file writing `qcore__pipe0` *and generating no temporary*
   carried no `Q036` **among its diagnostics** — it relied on the file being
   refused at all, because `let qcore__pipe0 = 0;` used to be an extern `let`
   naming no callee (`Q025`). That statement is now an admitted scalar literal,
   so the file compiles. The rule under test did not move and is now pinned
   *more* strongly: a file that generates no temporary may write the prefix and
   nothing refuses it. The `Q036` half of the same test is untouched.
5. **The declared outcome is scope beyond the brief**, taken because the payoff
   pin requires it (§3). It is `EXTERN-SCALAR-OUTCOME`'s *binding* half; its
   *trace* half stays open.
6. **Scalar width is still not checked at an argument.** Passing an `i32` value
   at a declared `u32` parameter is admitted exactly as an integer literal is —
   the pre-existing treatment `PIPE-SCALAR-RESULT` §5 named as
   `EXTERN-SCALAR-WIDTH`. What this card *does* check is the width of a
   **rendered** value, which is a different question and is the join's own rule.
7. **A raised error ends the composed run, and the first draft got it wrong.**
   The extern projection reports a raise by truncating its own trace; the
   composed interpreter has to truncate the *composed* observation at the same
   step, because the emitted C returns inside the failure arm. The first draft
   kept placing the standard-IO projection's later lines and would have published
   an observation the program never writes — the one way a per-stream comparison
   could have compared against fiction. Found by reading the raise path rather
   than by a failing test, then pinned by both a reference assertion and a real
   sanitized run (§7.3).
8. **Standard output carries two kinds of line.** A composed program's stdout
   holds this route's `print` lines *and* the extern route's synthesized call
   trace. Their order on that one stream is real and is checked — one `FILE *`,
   one sequence. The order **between** stdout and stderr is not modeled and is
   not claimed, which is the boundary `STD-MODULES` already stated.

## 7. Evidence

### 7.1 Tests

| Target | Count |
|---|---|
| `compiler/tests/std_io_compose.rs` | **36 new**, including two sanitized native runs |
| `compiler/tests/extern_calls.rs` | 80, unchanged and unmoved |
| `compiler/tests/std_modules.rs` | 31, unchanged and unmoved |
| `compiler/tests/pipe_operator.rs` | 41, with one pin updated in both halves (§6.4) |

Two shipped fixtures, twins of one another:

* `compiler/tests/fixtures/compose/chain.qcore` — an acquisition, a fill, a
  summed-and-declared scalar, a `print`, **two** literal `value |> render |> log`
  chains (one from a call result, one from a scalar literal), a release, a
  `flush` and a literal `print`;
* `compiler/tests/fixtures/compose/chain_hand_written.qcore` — the same program
  with both pipelines spelled as the statement sequences they desugar to.

One new declared callee, `qcore_extern_widen(value: i32) -> i64`, added to the
**test-fixture** contract module and its C implementation. It exists so the
unrenderable-width refusal is reachable against a real declaration rather than
being a rule with no instance; nothing links it in an accepted fixture, because
every fixture that names it is a refused one. Fixture contracts live under
`compiler/tests/fixtures/`, never under `contract/qcore/`, so the pinned corpus
counts are untouched (standing constraint 9 applies: a per-function result here
rests on those declarations and not on production ones).

Both directions are pinned throughout: the newly admitted forms *and* the still
refused ones — an acquisition without its adapter (`Q025`), a callee with no
selected contract (`Q023`), a leaked acquisition (`Q202`), a use after move, a
bound `unit` result (`Q024`), an acquiring result bound as a scalar (`Q025`), a
rendered resource (`Q028`), an unknown rendering (`Q013`), a missing `std.io`
import (`Q015`), a missing `std.extern` import (`Q022`), a **sink in a non-final
position** (`Q037`), a rendered indeterminate value (`Q043`), a rendered
value of an unrenderable width (`Q043`), a shadowed binding, an out-of-range
literal, and all three concurrency exclusions.

### 7.2 Byte-equality and the certificate

`the_literal_chain_emits_its_hand_written_twins_c_byte_for_byte` asserts
`compile(chain).c_source == compile(hand_written).c_source` and equal reference
observations, on the `PIPE-OPERATOR`/`PIPE-SCALAR-RESULT` precedent, and that
the hygienic temporary's name reaches neither. The emitted C names values `vN`
by identity exactly as it names resources `rN`.

`the_rendered_line_carries_the_number_the_extern_model_bound` pins the numbers
that crossed the join: `qcore_extern_fill(b, 7)` writes eight bytes of 7, so the
declared sum is 56 and the declared scale 112, and the observation carries
`out: 56`, `err: [INFO] 112` and `err: [WARN] 0x00000054`.

### 7.3 Native evidence

Two sanitized runs, x86_64 Linux.

`the_composed_program_matches_the_reference_per_stream` compiles the emitted C
against the real `compiler/tests/fixtures/extern_calls/buffer_api.c` with
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, `detect_leaks=1`, and
compares **stdout against the reference's stdout projection and stderr against
its stderr projection**. No cross-stream order is asserted anywhere.

This run is a **positive** leak control, unlike `STD-MODULES`' two: the fixture
allocates, lends and releases one real buffer, so if composing had lost the
release LSan would report it. A declared-outcome disagreement would exit 70 and
fail the test rather than producing a quietly different observation.

`a_raised_error_ends_the_composed_run_and_its_later_renderings` is the second,
and it is the run that caught a real defect in this card's own first draft. A
composed module whose acquisition takes its **null** arm returns inside that arm,
so every later step — a rendering included — writes nothing; the first draft's
composed interpreter kept consuming the standard-IO projection's own lines past
that point and would have published lines the program never writes. Both halves
are now pinned: the reference observation stops at the raise, and the native run
produces exactly those three lines on stdout and nothing on stderr.

Apple ASan provides no LSan, so leak detection stays a Linux-only observation
(the existing convention).

### 7.4 Diagnostics

**One code added, `Q043`, through the §L21 process**, under a new
`ComposedStdIo` semantic pass, in the one slot the branch and native-library
bands left free. It owns the three join rules and nothing else — deliberately a
*small* pass: if it ever owned a custody or a rendering rule, the composition
would have stopped being a union.

| Code | New condition |
|---|---|
| `Q043` | the composed order is not an interleaving; a rendering operand bound later, of an unrenderable width, or carrying no modeled number; the composed effect union not admitted; the composed certificate against its rederivation |
| `Q025` | a `let` that is none of the three admitted forms — the message now names the literal binding too |
| `Q024`, `Q026`, `Q027`, `Q028`–`Q031`, `Q037` | **unchanged** |

`Q020` and `Q021` are untouched and stay separate (standing constraint 2). The
`Q023` default is unmoved. **No reserved word was added** (§3), so the
constraint-14 census was not needed.

### 7.5 Lean

New module `proof/QCore/ComposedStdIo.lean`, audited by the new
`proof/AuditComposedStdIo.lean`, registered in `proof/QCore.lean` so
`AuditAll.lean`'s whole-project axiom closure covers it. **31 audited theorems**,
no new axiom, no placeholder, no `native_decide`. No existing `QCore/*.lean` or `Audit*.lean`
file was edited — the one-line import in `proof/QCore.lean` is the only change
to an existing proof file.

The module is a model of the **join**, mirroring the Rust: it imports
`QCore.ExternCall` and reuses that module's own `Ledger`, so the composed ledger
*is* the extern route's ledger rather than a copy of it.

The card's central claim is proved rather than asserted:

* `a_rendering_leaves_the_ledger_unchanged` and
  `the_shared_return_leaves_the_ledger_unchanged` — an IO step moves no custody;
* `the_composed_ledger_is_the_extern_projections_ledger` — running a whole
  composed program leaves **exactly** the ledger its extern projection alone
  leaves. That is "composing added no custody path" as a statement about the
  model, not a comment;
* `no_step_moves_a_name_between_the_namespaces` and
  `an_operand_is_a_literal_or_a_value_name` — the namespace wall;
* `the_projections_are_recovered_from_the_order` — the interleaving loses and
  duplicates nothing;
* `the_composed_effect_row_is_the_union`, `no_projection_effect_is_dropped` (an
  *iff*: nothing dropped **and** nothing invented) and
  `admitting_the_union_admits_each_projection` — nothing dropped at the join;
* `the_determined_values_are_exactly_these` exhibits the determinacy surface as
  one list, so a future widening changes a theorem rather than slipping past it,
  with `an_undetermined_operand_has_no_admitted_step`,
  `a_call_without_a_declared_outcome_determines_nothing` and
  `a_determined_rendering_publishes_exactly_one_line` beside it;
* `a_rendering_reads_only_an_earlier_binding` and
  `a_rendering_before_its_binding_has_no_admitted_step` — the ordering rule;
* `print_never_writes_stderr`, `log_never_writes_stdout`,
  `the_extern_trace_shares_standard_output`, `a_barrier_publishes_no_line` —
  stream separation, with **no** cross-stream order claimed (stated in the
  module header and on that theorem's own doc comment);
* `a_rendering_carries_no_premise` and `composing_adds_no_premise`;
* `the_payoff_fixture_is_admitted` and
  `the_payoff_fixture_carries_only_its_extern_premises` keep the surface
  non-vacuous by exhibiting the Lean image of this card's shipped fixture, with
  three refusal witnesses beside it.

**Seven `private` helper lemmas are not listed individually in the audit**, and
that is a new pattern in this proof directory — no existing `QCore/*.lean` uses
`private theorem`. They are covered twice over: `AuditAll.lean`'s whole-project
axiom closure includes them, and every public theorem that depends on one prints
its axioms transitively, so a private lemma resting on a project axiom would
surface in a public theorem's printed list. Recorded here so the integrator can
decide whether the convention should be to list them (this card did not change
`check_lean_audits.py`, which enforces no stale name rather than completeness).

**Two abstractions, named rather than left implicit.** The digits are not
modeled: a line carries the *number* that crossed from the extern model into it,
and `dec`/`hex` are `source.std-io.v1`'s and are compared against a native run
by the Rust tests. And the composed row being the union is *definitional*
(`composedEffects := union …`), exactly as the Rust computes it — its content is
carried by the two theorems beside it rather than by that definition.

**Not proved**, carried over from `STD-MODULES` and `PIPE-CHAIN`: that the Rust
lowering agrees with the model (a tested Rust boundary), and that a callee's C
implementation satisfies its declared contract (`ContractTrust::Declared`,
§C06).

### 7.6 Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched. The whole
diff is:

```
crates/qcore-core/src/composed_std_io.rs          (new: the union model and its join)
crates/qcore-core/src/composed_std_io_c_emit.rs   (new: emission and reference execution)
crates/qcore-core/src/extern_call.rs              (literal binding, declared outcome)
crates/qcore-core/src/extern_c_emit.rs            (per-event split, the outcome check)
crates/qcore-core/src/std_io_c_emit.rs            (per-event split)
crates/qcore-core/src/lib.rs
crates/qcore-frontend/src/source_composed.rs      (new: the router)
crates/qcore-frontend/src/source_extern.rs        (the two new `let` forms, the skip list)
crates/qcore-frontend/src/source_std_io.rs        (the skip list, the operand census)
crates/qcore-frontend/src/frontend.rs             (the ComposedStdIo pass)
crates/qcore-frontend/src/lib.rs
crates/qcore-verification/src/driver.rs           (the route and its compile function)
compiler/src/lib.rs                               (two re-exports)
crates/qcorec/src/main.rs                         (the CLI's profile label)
compiler/tests/std_io_compose.rs, pipe_operator.rs
compiler/tests/fixtures/compose/{chain.qcore, chain_hand_written.qcore}
compiler/tests/fixtures/extern_calls/{contracts/buffer_api.qcore, buffer_api.c}
docs/QCORE_SPEC_r0_3.md, docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md
proof/QCore/ComposedStdIo.lean, proof/AuditComposedStdIo.lean, proof/QCore.lean
docs/reports/wave19-std-io-compose/README.md
```

The contract-corpus pin (69 modules), the 185-unit artifacts, the leak count of
**272** and the CT manifest are main's, and none of them moves for anything
here.

## 8. Named follow-ons

| Name | What it is |
|---|---|
| `EXTERN-SCALAR-OUTCOME` *(trace half)* | observing the number a bound call returned **in the trace**. The binding half is discharged here (§3); the observation line still names the callee, the value and the arguments and not the result. |
| `COMPOSE-CONCURRENCY` | the other pair `STD-MODULES` named: a module that prints and spawns. Strictly larger than this card — a task worker is a separate checked model with its own ledger, and the union shape here does not carry over without one. |
| `COMPOSE-RENDER-WIDTH` | rendering a value of a width other than `i32`, which needs its own conversion table rather than a widened refusal. |
| `EXTERN-SCALAR-WIDTH` | *(unchanged from `PIPE-SCALAR-RESULT`)* checking a scalar **argument**'s declared width against its source. |
| `STD-FMT-BORROW` | *(unchanged from `STD-MODULES`)* a borrowed render of a live resource, with its own custody argument. It is **more** attractive now: a composed module has live identities beside its renderings, which is exactly the setting that argument would be about. |
| `STD-FMT-DIAGNOSTIC`, `STD-LOGGER-ASYNC` | *(unchanged from `STD-MODULES`)* |
| `SEQ-SCALAR-RESULT`, `PIPE-TAP`, `PIPE-ASSIGN`, `PIPE-BINDER-SPAN`, `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY` | *(unchanged)* |

## 9. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | **3,108 passed, 0 failed, 8 ignored** — composed, see §9.1 |
| `cargo test --workspace --doc --no-fail-fast` | 29 pass across 11 targets, exit 0 |
| `cargo test -p qcore-compiler --test std_io_compose` | **36 pass**, including two sanitized native runs |
| `cargo test -p qcore-compiler --test extern_calls` | 80 pass |
| `cargo test -p qcore-compiler --test pipe_operator` | 41 pass |
| `cargo test -p qcore-compiler --test std_modules` | 31 pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (three of this card's own lints found and fixed at the source, not suppressed) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **84/84**, no new axiom (83 → 84 for this card's new audit file) |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — see below |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

### 9.1 One self-inflicted environment finding, recorded rather than worked around

The closing `cargo test --workspace --no-fail-fast` reported **3,086 passed, 1
failed, 8 ignored**. Every non-doc target passed — **3,079 passed, 0 failed, 8
ignored** — and the single failure, plus four further doctest targets that
aborted without reporting, are all one thing:

```
error[E0460]: found possibly newer version of crate `qcore_frontend`
              which `qcore_verifier_host` depends on
```

The cause is mine and is named rather than blamed on the machine: while that run
was in flight I deleted an **uncalled** `pub fn` from
`crates/qcore-frontend/src/source_composed.rs`, which rebuilt that crate's rlib
under the running job. Doctests link against the built rlibs at run time and are
the only targets that can notice; every other target had already been compiled
and linked. Re-run with nothing in flight,
`cargo test --workspace --doc --no-fail-fast` gives **29 passed, 0 failed**
across 11 targets, exit 0.

The composed figure is therefore **3,108 passed, 0 failed, 8 ignored**. No
assertion was weakened and no result is inferred from a truncated run: the
non-doc figure is one complete run's, and the doc figure is one complete run's.

For the integrator: a sibling worktree was building and testing on the same
machine for part of this run, so wall-clock timings here are not comparable with
a quiet run. None of the four `P02-FLAKE` rows (standing constraint 16) tripped
on this run.

### 9.2 One expected failure, left for the integrator

`scripts/tests` reports
`test_the_derived_facts_are_recomputed_rather_than_carried` failing with
`lean_audits: 83 != 84`. That is the derived-value tripwire working exactly as
designed: this card adds one audited Lean module and
`docs/reviews/r0_3/acceptance_matrix.json` carries the pre-card count. The base
commit `5c1f949` was self-consistent (83 audit files, matrix value 83), so the
delta is entirely this card's and is not a pre-existing failure. **This card's
brief forbids editing acceptance files**, so the matrix is deliberately left
stale for the wave integrator to regenerate — the same handling every wave's
derived sections receive.

### 9.3 A stale headline the integrator should note

`AGENTS.md`'s ledger says **80 Lean audits**. Main `5c1f949` has **83** audit
files and the acceptance matrix says 83, so the headline was already stale
before this card; after it the true number is **84**. Recorded here rather than
fixed, because `AGENTS.md` is outside this card's footprint.

## 10. Draft increment paragraph

> Composing the two sequential source routes (STD-IO-COMPOSE): a module may now
> both call a declared extern function and print, on the new route
> `source.std-io-extern.v1`, built as a **union of two checked models rather
> than a third one** — a composed program carries an `ExternProgram` and a
> `StdIoProgram` side by side plus the order interleaving them, and the composed
> checker runs `check_extern_program` and `check_std_io_program` **unmodified**
> before checking anything of its own, with the established sequential passes
> still re-checking the extern half. Neither lowering was edited either: both
> gained a statement skip list whose empty case *is* the old entry point, so a
> module using one feature set travels the identical code path and keeps its
> route byte for byte, and the composed backend places the two existing
> backends' own per-event statements rather than lowering anything itself. The
> route is selected **after every concurrent route and before both routes it
> composes**, so a module that prints and spawns, or extern-calls and spawns,
> keeps its concurrent route's `Q600` refusal — the other pair stays a separate
> card. One code, `Q043`, under a new `ComposedStdIo` pass through the §L21
> process, owns exactly the three rules that exist because the two models meet:
> the order must be an interleaving; a rendering operand must resolve in the
> extern model's **value** namespace, be bound by an earlier step, be of the one
> width the two renderings are total on, and carry a number the source
> determined; and the composed effect row is the **union** of the two derived
> rows. The literal acceptance sentence `value |> render |> log` that card
> `PIPE-CHAIN` measured as unreachable now compiles end to end, emits
> **byte-identical C11** against its hand-written twin and byte-compares against
> reference execution **per stream** under ASan/UBSan/LSan with a *positive*
> leak check. Reaching it needed two widenings of the value namespace, both
> custody-free by construction: the scalar literal binding `let n = 42;`
> (`EXTERN-SCALAR-LITERAL`), and a **declared outcome** `succeeds INTEGER` on the
> existing `succeeds BOOL` precedent — the site's literal outcome control, which
> the emitted program does not believe but compares against the real result,
> exiting 70 on disagreement, which is what makes a rendered line comparable
> against a native run at all. A bound call result with **no** declared outcome
> stays a perfectly good value and is deliberately **not** renderable: rendering
> it would publish a line no model computed. No reserved word was added — the
> two `succeeds` tails are told apart by the literal alone — so no fixture
> surface moves. Three measurements are recorded: `Q029` and the composed
> effect-union arm are **unreachable** on this route, because `Effect::Io` is
> stamped on every extern call and the extern half's own gate refuses first
> (both kept as pinned fail-closed backstops); a raised error must truncate the
> *composed* observation and not only the extern projection's share of it, a
> defect in this card's own first draft caught by reading the raise path and now
> pinned by a second sanitized run; and one wave-19 pipe-hygiene pin was updated
> in both halves, its rule unmoved and now pinned more strongly. 36 new tests,
> 31 new audited Lean theorems (plus seven private helper lemmas) in a new
> module with **no** existing proof file edited, 84/84 audits and no new axiom. Corpus-neutral by construction.
> Evidence: `docs/reports/wave19-std-io-compose/README.md`.
