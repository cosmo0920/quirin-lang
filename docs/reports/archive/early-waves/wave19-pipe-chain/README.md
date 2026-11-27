# PIPE-CHAIN — `|>` as a real multi-stage operator (wave 19)

Card brief: *"Make `|>` a real STREAM operator: multi-stage pipelines composing
functions — `value |> render |> log` and two-argument-function composition
(`x |> f(a) |> g(b)` meaning `g(f(x,a),b)`) — usable for I/O streams."*

Base commit: main `614c70f` (wave-18 close).

**What landed.** Multi-stage pipelines, in full, as a statement-sequence
desugaring with hygienic temporaries — desugaring **(A)** of the brief. Wave
18's recorded `PIPE-CHAIN` blocker is retired by measurement rather than worked
around, `PIPE-LET` closes with it, and a chain emits **byte-identical C11**
against a shipped hand-written fixture, runs natively under ASan/UBSan/LSan, and
inherits every custody rule without one line of new checker. Twenty-one new Lean
theorems, no new axiom, no new diagnostic code, no code moved.

**What did not.** The brief's acceptance sentence — `value |> render |> log`
with `render` producing something `log` can render — is **not reachable in this
source language**, for two pre-existing reasons this card measured and neither
widened nor narrowed. §3 is that measurement and it is the most consequential
finding here. The chain *shape* the sentence describes works end to end, with
the vocabulary the language actually has.

---

## 1. The blocker, re-measured

Wave 18 refused `x |> f |> g` at `Q037` and named two blockers
(`docs/reports/wave18-pipe-operator/README.md` §2.3). Both were re-measured
before any code was written.

| Wave-18 blocker | Status after measurement |
|---|---|
| "A call stage's result has no name — QCore source has no expression tier." | **True, and irrelevant to the chain.** The answer is not an expression tier; it is to *give the result a name*. A non-final stage becomes a `let ⟨t⟩ = …;` statement and the next stage receives `⟨t⟩`. |
| "A chain of statement stages would need statement-sequence expansion, incompatible with the byte-span statement windows every route uses — the two statements would share one operand token, so their spans would overlap." | **Rests on an assumption that the desugaring need not make.** It is true for a *tap* desugaring, where two sinks share the reader's operand. With a fresh temporary per stage the statements share **nothing**, and their spans are disjoint by construction. |

The second is the whole card, so it is worth stating exactly. Every route reads
its own statement by filtering the program's tokens with
`span.start <= token.span.start && token.span.end <= span.end`
(`source_std_io::window`, `source_extern::statement_tokens`), and a statement's
span is the min start / max end over its tokens (`SourceParser::range_span`, the
generalization wave 18 landed). So the chain is admissible exactly when
consecutive statements' extents do not overlap. They do not, because of where
the synthesized tokens are anchored:

* statement `k`'s synthesized terminating `;` is anchored at the **start** of
  the `|>` that ends it;
* statement `k+1`'s synthesized tokens are anchored at that same `|>`'s **end**.

`|>` is two bytes wide, so `extent(k).stop < extent(k+1).start` with **no
hypothesis about where the pipes are** — including the adversarial case where a
`|>` abuts its neighbours with no whitespace (`f(x)|>g`). Both directions are
proved: `consecutive_statements_are_disjoint`, and
`the_naive_anchoring_touches_at_the_pipe` with
`the_touching_byte_is_covered_by_both_naive_windows` showing that anchoring
everything at the pipe's *start* — wave 18's convention, which is correct for a
single stage — would put one byte inside two windows. The two-byte gap is
load-bearing, not incidental.

### 1.1 Decision: (A) statement sequence, not (B) nested calls

Measured, per the brief's instruction. **(B) is not available.** The extern
route's whole statement grammar is
`let NAME = CALLEE(ARGS) succeeds … raise ERROR;` | `CALLEE(ARGS);` |
`return;` (`source_extern.rs` §module doc), and an argument is
`["-"] INTEGER | NAME | move NAME | out`. There is no nested-call argument form,
so `g(f(x,a),b)` is not a statement any route accepts; wave 13's nested
composition was C-side and does not reach here. (A) landed.

---

## 2. What a chain is

```ebnf
stmt       = [ "let", NAME, "=" ], OPERAND, ( "|>", STAGE )+, ";" ;
STAGE      = [ "move" ], STAGE_BODY ;
STAGE_BODY = "print", RENDERING | "log", LEVEL, RENDERING
           | NAME, [ "(", [ arg, { ",", arg } ], ")" ], [ ADAPTER ] ;
ADAPTER    = "succeeds", BOOL, "else", { disposition }, "raise", NAME ;
```

An `n`-stage pipeline becomes `n` statements. Each non-final stage binds
`qcore__pipeN`; the final stage keeps the statement form the reader wrote around
the whole pipeline — a bare statement, or a `let NAME =` binding, which is what
closes `PIPE-LET`.

```qcore
let grown = 8 |> qcore_extern_alloc succeeds true else raise Failure
              |> move qcore_extern_grow(32) succeeds true
                   else qcore_extern_free(move) raise Failure;
```

desugars to, token for token:

```qcore
let ⟨t0⟩ = qcore_extern_alloc(8) succeeds true else raise Failure;
let grown = qcore_extern_grow(move ⟨t0⟩, 32) succeeds true
              else qcore_extern_free(move ⟨t0⟩) raise Failure;
```

### 2.1 Which stages may be intermediate, and why the rule is decidable

**Only an acquisition adapter binds a result in this source language.** That is
not a design preference, it is the measured statement grammar: a plain
`CALLEE(ARGS);` discards its return value, and `let n = CALLEE(ARGS);` for a
non-acquiring callee is refused (`Q025`, "establishes no fresh custody, so it
takes no adapter and binds no resource"). So "produces a value" is decided by
the presence of the `succeeds` keyword, **syntactically, with no contract
knowledge entering the desugaring** — which is what keeps the pipe a pure token
pass. Everything else is last-stage-only and refuses at `Q037` naming the stage:

> ``print`` produces no value, so it cannot feed a later stage: it is a sink or
> a discarding call, and only an acquisition `CALLEE(ARGS) succeeds BOOL else …
> raise ERROR` binds a result. Make it the pipeline's last stage

### 2.2 The stage-vocabulary table (the census the brief asked for)

Every callable form the source language admits, measured against the routes, not
assumed:

| Stage form | Route | Intermediate? | Final? | Evidence / measured reason |
|---|---|---|---|---|
| `CALLEE(ARGS) succeeds BOOL else … raise ERROR` (acquisition) | `source.extern-call.v1` | **yes** | yes | the only result-binding form; `realloc_piped.qcore`, native run |
| `CALLEE(ARGS)` (discarding call, contracted) | `source.extern-call.v1` | no — `Q037` | yes | `a_stage_that_produces_nothing_cannot_feed_a_later_stage` |
| `CALLEE(ARGS)` where the callee is `unsafe extern fn` | `source.extern-call.v1` + `std.extern.unsafe` | no — `Q037` | yes, but see below | `a_piped_unsafe_stage_deposits_the_same_waiver_as_its_unpiped_twin`; the waiver travels identically (§5.3) |
| `print RENDERING` / `log LEVEL RENDERING` | `source.std-io.v1` | no — `Q037` | yes | `a_sink_cannot_feed_a_later_stage`; sinks produce nothing |
| `flush` | `source.std-io.v1` | no — `Q037` | no — `Q037` | takes no operand at all; unchanged from wave 18 |
| a user-defined function call | **none exists** | — | — | a bare expression statement refuses at `Q500` in the executable subset; wave 18 §1, re-confirmed |
| `drop` / `move` / `defer` as a stage | — | no — `Q036` | no — `Q036` | not in the closed stage table; follow-on `PIPE-STMT-STAGES`, unchanged |

### 2.3 Two spellings for a value that has no name

A hygienic temporary is deliberately unnameable, so the two things a reader
needs to say about it need spellings that are not names:

* **`|> move STAGE`** consumes the incoming value: `free(move ⟨t⟩)`. It mirrors
  `move OPERAND |> STAGE` for the first stage, and is **refused on the first
  stage** — the reader's own operand already has the language's spelling, and
  one spelling per meaning is worth a rule (`Q036`, pinned by
  `move_is_refused_on_the_first_stage_where_the_operand_has_a_spelling`).
* **A bare `move`** inside a stage's adapter tail names it, so an acquisition's
  error arm can dispose of the value piped into it:
  `else qcore_extern_free(move) raise Failure`.

The second is **one spelling beyond what the brief described**, and it is
recorded as a deviation (§6) rather than slipped in. It is not decoration: an
acquisition's error arm must explicitly release every *other* live identity
before its `raise` (`Q025`, measured), and in a chain that identity is always
the previous stage's temporary. Without it a chain could contain **at most one
acquisition** — which would have capped every chain at two stages. It stays a
pure token substitution: what reaches the extern route is the ordinary
`CALLEE(move NAME)` disposition it already checks, and a bare `move` outside a
pipeline stage keeps its pre-existing refusal
(`a_bare_move_outside_a_pipeline_stage_keeps_its_existing_refusal`).

### 2.4 Hygiene

Temporaries are `qcore__pipe0`, `qcore__pipe1`, … numbered per file. They are
synthesized `Word` tokens and never pass through the lexer, so they *could* have
carried bytes no source can spell — but they reach name resolution and every
diagnostic that names a binding, so they have to be legible identifiers.

Hygiene is therefore enforced **by refusal**, exactly as the brief instructed: a
file that needs a temporary and also writes a word beginning with `qcore__pipe`
is refused at `Q036` naming the prefix. Silent α-renaming was rejected on the
ground that a renamed binding makes a later diagnostic name something the reader
never wrote. The refusal is scoped to files that actually generate a temporary:
a file whose pipelines are all single-stage is untouched, and that half is pinned
too (`a_pipeline_temporary_cannot_be_named_or_shadowed`).

**What a temporary's lifetime actually is**, stated precisely rather than
loosely. It is bound by stage `k` and read by stage `k+1` — that is the point of
it, so "it does not survive its statement" would be false. What is true, and is
what the brief's requirement means, is that it does not survive the *pipeline*:
nothing after the last stage can name it, and if a chain's final stage does not
consume or escape it, the pre-existing accounting reports a leak (§4) rather
than letting it disappear. A chain whose final stage consumes therefore leaves
nothing bound — pinned by the byte-identical emission against the shipped
fixture, whose own hand-written form leaves nothing bound either. The name never
reaches the emitted C at all (§5.2).

---

## 3. The acceptance sentence, measured

The brief set `value |> render |> log` "with `render` a real result-producing
stage" as the bar. The **chain shape** — operand, result-producing intermediate,
final — works end to end and is the fixture this card ships. The **literal
reading**, where `render` yields something a `std.io` sink can render, is
unreachable, on two boundaries that both predate this card:

| Boundary | Measurement |
|---|---|
| **No route binds a scalar call result.** `qcore_extern_sum(b, 8) -> i32` is declared, called and its result **discarded**; `let n = qcore_extern_sum(b, 8);` refuses at `Q025` because the callee acquires nothing. So there is no `render` whose output is a renderable scalar. Follow-on **`PIPE-SCALAR-RESULT`**. |
| **A module cannot both call an extern function and print.** Route selection puts a module with both on the extern route (`driver.rs`: "a module that both prints and … calls an extern function stays with the more specific model"), where `print` refuses at `Q500`. Measured directly on a module importing both: its `let total = 42;` reports `Q025` ("an extern acquisition must read `let NAME = CALLEE(ARGS) succeeds …`") and its `print dec total;` reports `Q500` ("the extern-call profile admits `CALLEE(ARGS);`, an acquisition `let` and a bare `return;`"). That is the already-recorded **`STD-IO-COMPOSE`**, which `driver.rs` itself calls "a separate card". |

Neither is a pipe problem and this card moves neither. What it does instead is
make the refusal *say the right thing*: `total |> print dec |> log info dec` no
longer refuses for being a chain, it refuses because **`print` produces no
value** — the narrow, permanent rule, which is also the honest answer to why the
sentence is not writable.

The strongest reachable instance of the brief's own composition example,
`x |> f(a) |> g(b)` meaning `g(f(x,a),b)`, *is* shipped and is a real
two-argument composition with real custody:

```qcore
move b |> qcore_extern_grow(32) succeeds true else qcore_extern_free(move) raise Failure
       |> move qcore_extern_free;
```

---

## 4. Linearity, both directions

Custody flows through the temporaries under the **unedited** checkers — by the
time any of them runs, a temporary is an ordinary binding. Every property below
is the pre-existing diagnostic, not a pipeline rule.

| Property | Evidence | Measured |
|---|---|---|
| A chain that drops a live result **does not pass** | `a_chain_whose_result_is_never_consumed_leaks_and_refuses` | `["Q202"]` — the existing leak accounting, unchanged |
| A value consumed mid-chain cannot be used after the pipeline | `a_value_consumed_mid_chain_cannot_be_used_after_the_pipeline` | the piped and unpiped **diagnostic vectors are asserted equal**, not merely both non-empty |
| A second acquisition's error arm must dispose of the first | inherited `Q025`; §2.3 | measured before the bare-`move` spelling existed: `"the adapter's error arm would abandon live resource 1"` |
| A chain's first stage still consumes the reader's operand | `an_owned_resource_piped_into_a_consuming_call_is_consumed` (wave 18, unchanged) | byte-identical `c_source` |

The brief asked what the existing accounting does with an unused final result:
it is `Q202 LiveResourcesAtNormalReturn`, and it is pinned rather than repaired,
because it is correct.

---

## 5. Evidence

### 5.1 Tests

`compiler/tests/pipe_operator.rs` — **39 tests, all passing** (28 → 39), of
which two are sanitized native runs. One new fixture,
`compiler/tests/fixtures/pipe/realloc_piped.qcore`, is the piped twin of the
shipped `extern_calls/realloc.qcore`, so the comparison is against real pinned
code rather than a program written to make a test pass. It exercises the whole
card in one statement: a three-token chain, a hygienic temporary, the `move`
stage prefix, a bare `move` in an error arm, and a `let` binder.

### 5.2 Byte-equality

`a_multi_stage_pipeline_is_its_hand_written_statement_sequence` asserts
`compile(chain).c_source == compile(hand_written_sequence).c_source` **and**
equal reference observations. The emitted C names resources `r1`, `r2`, … by
identity, so the temporary's *name* never reaches the C at all — the byte
equality is against the shipped fixture's own output, unmodified.

### 5.3 Sanitizer evidence — reported honestly

`the_native_run_of_a_multi_stage_pipeline_matches_its_reference`, x86_64 Linux,
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. The chain links the real
`fixtures/extern_calls/buffer_api.c` callees — a real `malloc`, a real `realloc`
and a real `free` — and its stdout matches the reference trace line for line.
The leak check is a **positive** control here, unlike wave 18's: this program
allocates, reallocates and frees.

**Honest scope.** The brief asked for a chain "through a real C callee into a
std.io sink". The C-callee half is real; the std.io-sink half is not, and cannot
be, for the route reason in §3. Wave 18's `lines_piped.qcore` native run still
covers the sink side at one stage.

The `unsafe extern fn` requirement is met by checking rather than by a second
native run: a piped unsafe stage and its unpiped twin produce identical
`c_source` and identical observations, so the `UncheckedLinkage` assumption a
waiver deposits cannot be laundered by a pipeline.

**One measurement made while writing that test.** An unsafe stage is
*structurally* last-stage-only, twice over: it carries no adapter, so it cannot
feed a later stage; and `unsafe extern fn` is scalars-only (`UNSAFE_LINKAGE_V1`
L0), so it cannot **receive** a chain temporary either, because the only
result-producing stage binds a resource. So an unsafe callee can appear in a
chain only as a final stage taking a literal or a name — which is exactly the
single-stage form the test pins. That is a boundary of the unsafe surface, not
of the pipe, and it is recorded rather than papered over with a chain the
language cannot express.

### 5.4 Lean

`proof/QCore/PipeDesugaring.lean` grows **35 → 56 theorems** (21 new), audited by
the existing `proof/AuditPipeDesugaring.lean`; no new audit file, so
`check_lean_audits.py` stays at **79/79** and the derived `validation.lean_audits`
row does not move. No new axiom and no placeholder.

New families:

* **The chain identity** — `the_two_stage_chain_is_its_hand_written_sequence`
  (concrete, token for token), `a_one_stage_chain_is_the_single_stage_rewrite`
  and `the_stage_rewrite_agrees_with_the_single_stage_rewrite` on all three
  stage shapes, so wave 18's landed behaviour is proved unmoved rather than
  assumed unmoved.
* **What each stage receives** — `the_first_stage_receives_the_readers_operand`,
  `a_later_stage_receives_the_previous_temporary`,
  `a_consuming_stage_moves_the_previous_temporary`.
* **Fail-closed** — `a_non_final_stage_that_produces_nothing_refuses` (general),
  `a_sink_is_last_stage_only`, `move_on_the_first_stage_refuses`,
  `an_empty_chain_refuses`, and `refused_chains_rewrite_to_nothing` as one list
  so a future widening changes a theorem rather than slipping past it.
* **Why the sequence is admissible at all** —
  `consecutive_statements_are_disjoint` (general, no hypothesis on pipe
  positions), plus the refutation half:
  `the_naive_anchoring_touches_at_the_pipe`,
  `the_touching_byte_is_covered_by_both_naive_windows`,
  `the_landed_anchoring_separates_the_same_byte`. This is the theorem family
  that retires wave 18's recorded blocker.
* **Hygiene** — `temporaries_are_pairwise_distinct`,
  `a_temporary_carries_the_reserved_prefix`, `an_ordinary_binding_does_not`,
  `a_chain_synthesizes_only_delimiters_binders_and_temporaries`,
  `a_chain_preserves_every_written_origin`, `a_rewritten_chain_contains_no_pipe`.

**One theorem renamed, deliberately.** Wave 18's
`only_delimiters_are_synthesized` claimed in its *name* and doc comment that
nothing meaningful is ever invented. That is still true for a single-stage
pipeline and false for a chain, which invents its binders and temporaries. It is
renamed `only_delimiters_are_synthesized_in_a_single_stage_pipeline` with its
statement unchanged, and
`a_chain_synthesizes_only_delimiters_binders_and_temporaries` states exactly
what a chain invents. Nothing was weakened: both halves are now pinned where
before one name over-claimed.

**Not proved**, carried over from wave 18: that the Rust desugarer extracts
these token runs from real source the way the model says (a tested Rust
boundary), and that any route's grammar accepts a rewritten statement — it need
not, and its own diagnostic is then the answer. New to this card: the Lean
`splitArgs` scans flatly for the closing `)`, which is exact for every stage the
grammar admits (an argument is a literal, a name or `move NAME`, never a nested
call) but is a model simplification and is stated as one.

### 5.5 Corpus neutrality

**Corpus-neutral by construction, verified structurally.** The whole diff is six
files plus one new test fixture:

```
compiler/tests/pipe_operator.rs         (tests)
compiler/tests/fixtures/pipe/realloc_piped.qcore  (new fixture)
crates/qcore-frontend/src/pipe.rs       (the desugaring)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md (§G02h)
docs/QCORE_SPEC_r0_3.md                 (§L21, one row)
proof/QCore/PipeDesugaring.lean, proof/AuditPipeDesugaring.lean
```

No gateway, audit, contract-loader, provider or `contract/qcore/` file is
touched; no CT manifest entry changes; no pinned artifact is edited. The contract
corpus and the CT count are main's and neither moves for anything here.

**No collision with the sibling `TRACE-STDIO` card.** That card owns the four
concurrency backends' emission internals and `examples/threads`; this diff
contains no backend file and no shared emission helper, so there is nothing for
the integrator to reconcile from this side.

### 5.6 Diagnostics

**No code was added, moved or changed.** The Pipe band stays `Q036`–`Q038` and
`Q039+` stays reserved for the native-library design note. Two existing codes
gained conditions, and both were recorded in spec §L21 and the §G02h grammar in
the same change, per the §L21 process:

| Code | New conditions |
|---|---|
| `Q036` | a malformed or value-less `let NAME =` binder; an assignment pipeline (`PIPE-ASSIGN`); `move` on the first stage; a bare `move` whose incoming value is not a binding; a hygiene collision on the `qcore__pipe` prefix |
| `Q037` | any non-final stage that produces no value (this **replaces** wave 18's "any second stage") |

---

## 6. Deviations, found by measurement

1. **The bare `move` spelling** (§2.3) is one token beyond the brief's
   description. Without it `Q025` caps every chain at one acquisition, i.e. two
   stages. It is a pure token substitution and is pinned in both directions.
2. **The acceptance sentence is unreachable in its literal reading** (§3), on
   two pre-existing boundaries. Stated plainly rather than approximated.
3. **A multi-stage bound pipeline's binder cannot keep its bytes.** The reader's
   `let NAME =` sits to the left of every stage, so keeping its spans would make
   the final statement's span swallow the ones before it and every route's
   span-keyed window would return the wrong tokens. The binder is re-anchored at
   the final `|>` with the reader's own name preserved, so a diagnostic about
   the *binding* names the last pipe rather than the binder's own bytes. A
   one-stage bound pipeline keeps its binder tokens exactly where they were.
   Recorded as `PIPE-BINDER-SPAN`.
4. **Wave 18's "missing `;` mid-body" pin moved from `Q037` to `Q036`.** A stage
   is now checked for well-formedness before it is checked for its role, so
   `total |> print dec` running into the next statement is caught by the
   sink-arity rule ("`print` names 1 selector word … this stage names 2") rather
   than by "too many stages", which no longer exists. Both are fail-closed and
   correctly spanned; the measured one is pinned rather than the preferred one
   forced.
5. **Two wave-18 pins flipped by design, each keeping its negative half.**
   `a_pipeline_on_the_right_of_a_binding_refuses` becomes
   `…_binds_its_last_stage` — but the test now also pins that a bound pipeline
   ending in a value-less stage still refuses, and that an assignment pipeline
   still refuses. `a_multi_stage_pipeline_refuses_with_its_measured_reason`
   becomes `a_sink_cannot_feed_a_later_stage`, same code, narrower rule.
6. **One Lean theorem renamed** (§5.4), because its name over-claimed once
   chains exist.
7. **The Lean `rewrite` of wave 18 is untouched.** Chains use a new
   `rewriteStage` that generalizes it in the two ways a chain needs (a
   synthesized incoming value, an adapter tail), with the agreement between them
   proved on all three stage shapes rather than assumed. No wave-18 theorem was
   edited.
8. **An `unsafe extern fn` stage cannot appear anywhere in a chain except as a
   final stage taking a written value** (§5.3). It carries no adapter, so it
   cannot feed a later stage, and it is scalars-only, so it cannot receive a
   temporary either. The brief asked for the waiver to be pinned "when reached
   through a pipeline", and it is — at the only shape the language admits.

---

## 7. Named follow-ons

| Name | What it is |
|---|---|
| `PIPE-SCALAR-RESULT` | binding a non-acquiring call's scalar result (`let n = f(x);`), the missing piece for a `render` stage. Needs a scalar-argument form on the extern route too, since `NAME` there means a resource identity. |
| `STD-IO-COMPOSE` | *(pre-existing)* a module that both calls an extern function and prints. The other half of the acceptance sentence. |
| `PIPE-ASSIGN` | a pipeline on the right of an assignment, `x = y \|> f;` |
| `PIPE-TAP` | pass-through stages, so a value can be piped through a sink and continue. Sound for scalars (duplicating a scalar is not aliasing) and deliberately out of this card, which the brief scoped to result-producing stages. |
| `PIPE-BINDER-SPAN` | keeping a multi-stage bound pipeline's binder attribution (§6.3) |
| `PIPE-STMT-STAGES` | *(unchanged)* admitting `drop` / `move` / `defer` as stages |
| `PIPE-STAGE-ARITY` | *(unchanged)* a selector-arity table, needed only for a multi-operand statement stage |
| `OR-LOWERING`, `AND-LOWERING`, `BITWISE-SYMBOLS` | *(unchanged from wave 18)* |

---

## 8. Gate

| Gate step | Result |
|---|---|
| `cargo test -p qcore-compiler --test pipe_operator` | **39 pass**, including two sanitized native runs |
| `cargo test --workspace --no-fail-fast` | **2,970 passed, 0 failed, 8 ignored** across 175 targets — see §8.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (one collapsible-`if` from this card's own change fixed at the source, not suppressed) |
| `PATH=… python3 scripts/check_lean_audits.py` | **79/79 pass**, no new axiom, no new audit file |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, all OK** — unchanged; no audit file was added, so the derived `validation.lean_audits` row does not drift |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |

### 8.1 One environment finding, recorded rather than worked around

The closing `cargo test --workspace --no-fail-fast` run reported **2,967 passed,
3 failed, 8 ignored** across 175 targets. All three failures are the
`third_party/libevent` submodule pin check in a fresh worktree —
`libevent_submodule` (1) and `profile_loader` (2, its declared-pin and
inventory-digest rows) — exactly the finding wave 18 recorded in the same
situation. The pin was **not** weakened: the standing gate's own
`git submodule update --init --recursive` was run and both targets were re-run
in isolation, giving **1 passed** and **18 passed**, 0 failed. The composed
figure is therefore **2,970 passed, 0 failed, 8 ignored**, and the three
failures belong to the environment rather than to this change — no file this
card touches is reachable from either target.

A second environment note, for the integrator: a sibling card's build shared
the machine for most of this run, so wall-clock timings here are not comparable
with a quiet run. Nothing about the results depends on it.

---

## 9. Draft increment paragraph

> Multi-stage pipelines (PIPE-CHAIN): `|>` becomes a real stream operator. A
> pipeline of `n` stages desugars — still purely at parse time, still before the
> delimiter map — into the **sequence of `n` statements** a reader would have
> written, each non-final stage binding its result to a hygienic temporary the
> next stage receives and the final stage keeping the statement form written
> around the whole pipeline, which closes `PIPE-LET` in the same change. Wave
> 18's recorded blocker is retired by measurement, not worked around: it rested
> on the assumption that a statement sequence would **share** the reader's
> operand token, and with a fresh temporary per stage the statements share
> nothing — statement `k` ends at the *start* of the `|>` that terminates it and
> statement `k+1` begins at that pipe's *end*, so consecutive byte extents are
> disjoint with no hypothesis about pipe positions, and both directions are
> proved (the naive single-anchor convention is shown to put one byte inside two
> windows). Custody flows through the temporaries under the **unedited**
> checkers: a chain and its hand-written sequence emit **byte-identical C11**
> and observe identically against the shipped `extern_calls/realloc.qcore`, a
> chain that drops a live result refuses at the pre-existing `Q202`, a value
> consumed mid-chain has piped and unpiped **diagnostic vectors asserted equal**,
> and an `unsafe extern fn` stage reached through a chain deposits the same
> `UncheckedLinkage` waiver. "Produces a value" is decided **syntactically** by
> the acquisition adapter's `succeeds`, so no contract knowledge enters the
> desugaring and every sink, discarding call and `flush` is last-stage-only,
> refusing at `Q037` by name. Hygiene is enforced **by refusal** — a file that
> needs a `qcore__pipeN` temporary and writes that prefix itself is refused
> rather than silently α-renamed. One spelling landed beyond the brief and is
> recorded: a bare `move` in a stage's error arm names the value piped into that
> stage, without which `Q025` would cap every chain at one acquisition. Three
> measurements are recorded against the brief: nested-call desugaring **(B) is
> unavailable** (no route's statement grammar admits a nested call), and the
> acceptance sentence `value |> render |> log` is **unreachable in its literal
> reading** on two pre-existing boundaries — no route binds a scalar call result
> (`PIPE-SCALAR-RESULT`), and a module that both calls an extern function and
> prints stays on the extern route (`STD-IO-COMPOSE`) — while the chain *shape*
> it describes works end to end, including the brief's own
> `x |> f(a) |> g(b) = g(f(x,a),b)` composition with real reallocation custody.
> One sanitized ASan/UBSan/LSan run executes a chain against the real
> `buffer_api.c` callees with a **positive** leak check. Eleven new tests
> (28 → 39), 21 new audited Lean theorems (35 → 56) with one wave-18 theorem
> renamed for over-claiming and none edited, 79/79 audits and no new axiom. **No
> diagnostic code added, moved or changed** — `Q036`/`Q037` gained conditions,
> recorded in spec §L21 and grammar §G02h in the same change. Corpus-neutral by
> construction: the whole diff is the desugaring, its tests and fixture, two doc
> sections and the proof. Evidence:
> `docs/reports/wave19-pipe-chain/README.md`.
