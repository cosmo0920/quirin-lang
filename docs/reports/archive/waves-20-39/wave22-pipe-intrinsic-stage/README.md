# PIPE-INTRINSIC-STAGE — a model intrinsic as a pipeline stage (wave 22)

Card brief: *"COMPOSE-CONCURRENCY landed `value |> log info dec` over
`qcore_buffer_read`'s result in statement position, but the literal chain
`buffer |> read |> render |> log` is not spellable: the pipe desugaring has no
**intrinsic stage** — a stage whose callee is a checked model intrinsic (like
the task route's `read` of a buffer) rather than a declared extern or a sink.
Land it."*

Filed by `COMPOSE-CONCURRENCY` §5, which recorded that "giving the pipe a stage
for it is a pipe-grammar change (the sink stage's selector-word rule,
`PIPE-BORROW-SINK`'s neighbourhood), not a rendering change". Base commit: main
`ec5535a9` (wave-21 close).

**What landed.** The census the brief asked for first, and it changes the card's
shape: **the pipe grammar needed no stage entry at all.** A model intrinsic is
spelled `NAME(ARGS)` and its statement form is `let NAME = INTRINSIC(ARGS);`, so
`source.pipe.v1`'s *generic call stage* already produces exactly that statement
with the hygienic temporary. `returned |> qcore_buffer_read |> log info dec;`
compiles on `source.std-io-tasks.v1`, emits **byte-identical C11** against its
hand-written twin, and its sanitized native run byte-matches the composed
reference per stream. What the wave-21 note was missing is the *spelling*, not
the mechanism: `read` is a reserved concurrency keyword and there is no `read`
function, so a sink-style stage with selector words was the wrong shape to look
for.

**One production change, in one function.** Measuring the other routes turned up
the single place in the compiler where a later phase could still tell a `|>` had
been written: the sequential route recovered each statement's tokens by
**re-lexing its raw source bytes** rather than reading the program's own
post-desugaring stream. For a pipeline that recovers the pipeline, so that
route's own two call-shaped intrinsics — `qcore_acquire`, `qcore_check` —
refused through a pipe, at `Q500` naming the hand-written form the reader had in
fact written. §4. Nineteen new tests and nine new audited Lean theorems, **no new
diagnostic code, no new reserved word, no code moved.** Corpus-neutral by
construction: no gateway, audit, contract or provider file is touched.

---

## 1. The census

The brief asked for it first, and it is the finding. Every *model intrinsic* the
source language has — an operation the checked model performs, as opposed to a
declared extern callee or a `std.io` sink — falls into one of two shapes, and
`source.pipe.v1` already decides both.

| Intrinsic | Statement form | Route(s) | Shape | Stage? |
|---|---|---|---|---|
| `qcore_buffer_new(N)?` | `let B = qcore_buffer_new(N)?;` | the checked task routes (`ScopedOwnedTasksV1`, `ScopedReadSharesV1`, …) and every route that composes them (`ComposedStdIoTasksV1`, `ComposedEpochTasksV1`) | call | **yes** (§2.2) |
| `qcore_buffer_read(B)` | `let V = qcore_buffer_read(B);` | same | call | **yes** — the payoff (§2.1) |
| `qcore_buffer_write(B, V)` | `let U = qcore_buffer_write(B, V);` | same | call | **yes** as a final bound stage; mid-chain refuses (`unit`, §3.2) |
| `qcore_error_payload(E)` | `let B = qcore_error_payload(E);` | same | call | **yes** |
| `qcore_acquire(ID, BOOL, ERR)` | `let R = qcore_acquire(…);` | `SequentialCleanupV1` | call | **yes**, after §4 |
| `qcore_check(BOOL, ERR)` | `qcore_check(…);` | `SequentialCleanupV1` | call, statement-valued | **yes** as a final stage, after §4 |
| `spawn S W(ARGS)?` | `let H = spawn S W(ARGS)?;` | task routes | keyword | **no** (§3.6) |
| `join move H?` | `let R = join move H?;` | task routes | keyword | **no** (§3.6) |
| `share N`, `read N;`, `write N;` | their own | `MutexGuardV1` | keyword | **no** |
| `drop N;` | `drop N;` | every route with custody | keyword | **no** (§3.7) |
| `grow`, `open`, `close`, `assert`, `use`, `wait` | their own | `MutexGuardV1` | keyword | **no** |
| `bound`, `overhead`, `peak`, `effort`, `credit`, `retained` | plan clauses | epoch routes | keyword | **no** |

Two things follow, and they are the whole card.

**A call-shaped intrinsic is already a call stage.** `source.pipe.v1`'s stage
grammar is `"print" RENDERING | "log" LEVEL RENDERING | NAME [ "(" args ")" ]
[ ADAPTER ]`. The closed sink table names `print` and `log`; the operandless
table names `flush`; everything else that is a word falls to the third
alternative. An intrinsic is a word, so it is a call stage — and
`produces_a_result()` (widened from "carries `succeeds`" to "is a call stage" by
card `PIPE-SCALAR-RESULT`) answers `true` for it, so it may stand **non-finally**
and bind the hygienic temporary the next stage receives. Nothing was added,
because the wave-19 widening already covered it a card early. Lean:
`no_intrinsic_is_a_sink_or_operandless`,
`every_call_shaped_intrinsic_produces_a_result`.

**A keyword-shaped intrinsic is unreachable, and that is a statement rather than
an omission.** A call stage emits `HEAD "(" incoming … ")"`, and `join move H?`
is not of that shape whatever the pipe does with it. Lean states it as the
inequality `a_keyword_shaped_intrinsic_is_not_reachable`, rather than leaving it
to the closed table's silence.

### 1.1 What "the honest spelling the census supports" is

The brief's literal `buffer |> read |> render |> log` is not spellable and never
will be under this vocabulary, for two reasons neither of which is the pipe's:

* **`read` is a reserved word, not a function.** It is a mutex-route statement
  head (`read NAME;`) and an epoch plan clause, reserved in
  `qcore_frontend::frontend` precisely so a local binding cannot shadow those
  statements. The buffer read's name is `qcore_buffer_read`.
* **`render` is not a stage, it is a selector word.** Card `PIPE-CHAIN` §3 and
  `PIPE-SCALAR-RESULT` both measured this: the rendering is `dec`/`hex`, a
  closed word belonging to the sink, so `render` collapses into `log info dec`.

The honest spelling is therefore two stages, not four:

```qcore
returned |> qcore_buffer_read |> log info dec;
```

which desugars, token for token, to the pair of statements
`COMPOSE-CONCURRENCY` §5 shipped as the admitted form:

```qcore
let ⟨t0⟩ = qcore_buffer_read(returned);
log info dec ⟨t0⟩;
```

## 2. What is admitted, and how it is pinned

### 2.1 The payoff

`a_buffer_read_is_a_non_final_pipe_stage_on_the_composed_task_route` compiles
the chain on `ComposedStdIoTasksV1`. Three further tests make "the desugaring
adds no semantics" measurable rather than asserted, on the wave-18 pattern:

| Test | What it measures |
|---|---|
| `the_intrinsic_chain_and_its_hand_written_twin_are_the_same_program` | `syntax_equivalent` against the twin that spells `qcore__pipe0` itself — the whole post-parse token stream is equal |
| `the_intrinsic_chain_emits_byte_identical_c_against_its_hand_written_twin` | equal `c_source` **and** equal `reference_observation` against a twin binding `rendered`, a name a reader would actually choose: the temporary's name is not observable, because the task backend names slots |
| `the_piped_composed_program_matches_the_reference_per_stream` | the emitted C compiled `-Wall -Wextra -Werror -pedantic` under ASan/UBSan (LSan on Linux) and its stdout/stderr compared **per stream** against `execute_with_source_lines` — the number rendered is the one the model computed out of the modelled buffer, `STD-FMT-BUFFER-READ`'s own standard |

### 2.2 The other call-shaped intrinsics

Each is pinned against its hand-written twin's emitted C, so the claim is
identity rather than "it compiles":

* `a_buffer_allocation_is_a_stage_and_keeps_its_adapter_tail` —
  `7 |> qcore_buffer_new?` is `qcore_buffer_new(7)?`. The `?` is an adapter
  tail, passed through verbatim after the argument list by the same mechanism
  the acquisition adapter uses (Lean:
  `an_allocating_intrinsic_keeps_its_adapter_tail`).
* `a_buffer_write_is_a_stage_and_the_piped_buffer_is_its_first_argument` —
  `returned |> qcore_buffer_write(7)` is `qcore_buffer_write(returned, 7)`:
  first-argument insertion on an intrinsic, unchanged from an ordinary callee
  (Lean: `an_intrinsic_takes_the_piped_value_as_its_first_argument`).
* `an_error_payload_take_is_a_stage` — `failure |> qcore_error_payload` is
  `qcore_error_payload(failure)`. This is the census's one *custody-transferring*
  call-shaped intrinsic — it moves an owned Buffer out of a bound typed error —
  and it is a stage on exactly the same terms as the rest: the desugaring
  produces the route's own `let` form and the ledger that moves the payload is
  the pre-existing one.
* `the_sequential_routes_acquisition_is_a_stage`,
  `the_sequential_routes_check_is_a_stage` — §4.

## 3. The refusals, each by name

Both directions, as the brief required. Every one of these is **the route's own
diagnostic at the reader's own bytes**, never a new pipe rule — which is the
`PIPE-SCALAR-RESULT` precedent restated: whether a stage's result is bindable is
the route's question, and it answers at that stage's span.

| # | Shape | Refusal | Test |
|---|---|---|---|
| 3.1 | an intrinsic as a bare final stage whose value is discarded — `returned \|> qcore_buffer_read;` | `Q500` "unsupported expression statement in task execution": every value-producing intrinsic is a `let` initializer on this route, and there is no discarding form | `an_intrinsic_whose_value_is_discarded_is_not_a_bare_final_stage` |
| 3.2 | a `unit`-returning intrinsic mid-chain — `returned \|> qcore_buffer_write(7) \|> log info dec;` | `Q050` naming the temporary's type, **not** `Q037` naming the pipe — exactly the `Q024` answer `PIPE-SCALAR-RESULT` pinned for a `unit`-returning extern callee | `a_unit_returning_intrinsic_refuses_mid_chain_at_the_routes_own_code` |
| 3.3 | a custody-bearing temporary reaching a sink — `7 \|> qcore_buffer_new? \|> log info dec;` | `Q050`: an `own<Buffer>` is not the `ValueType::Scalar(Scalar::I32)` the composed rendering operand rule admits, so nothing crosses from custody into the value namespace | `a_custody_bearing_temporary_cannot_cross_into_the_rendering_namespace` |
| 3.4 | a custody-**producing** intrinsic mid-chain — `7 \|> qcore_buffer_new? \|> qcore_buffer_read;` | `Q600` "discards a handle, unconsumed Result or owned buffer": the temporary is an owned buffer no reader can name and therefore cannot `drop`, so it leaks under the **unedited** checker | `an_allocating_intermediate_leaks_its_hygienic_temporary` |
| 3.5 | consuming the temporary into a read — `\|> move qcore_buffer_read` | `Q500` "buffer read requires one exact owned-buffer name": a read consumes nothing, so `qcore_buffer_read(move t)` is not its statement form | `consuming_the_temporary_into_a_read_refuses` |
| 3.6 | keyword-shaped intrinsics as stages — `workers \|> spawn`, `returned \|> join` | `Q000` / `Q500`: `spawn(workers);` does not parse and `join(returned);` is not the join form | `the_keyword_shaped_intrinsics_are_not_stages` |
| 3.7 | `returned \|> drop;` | `Q500` "buffer drop requires one exact name": `drop` is keyword-shaped and is deliberately **not** added to the closed sink table | `drop_is_not_a_pipe_stage_on_the_task_route` |
| 3.8 | an intrinsic stage on a route whose model has no buffer — a `qcore_buffer_read` stage inside a mutex generation | refused by `source.mutex.guard.v1`, which is selected **before** the task route and models no buffer at all | `a_route_whose_model_has_no_buffer_intrinsic_keeps_its_own_refusal` |

3.4 is the one worth reading twice. It is not a pipe rule and it was not written
for this card: the chain desugars to `let ⟨t0⟩ = qcore_buffer_new(7)?;
let value = qcore_buffer_read(⟨t0⟩);`, `⟨t0⟩` is an ordinary binding by the time
any checker runs, and the task route's pre-existing exit rule refuses it. That is
card `PIPE-CHAIN`'s "custody flows through the desugared statements under the
unedited checkers" holding on an intrinsic, and it is the reason the hygienic
temporary is deliberately unnameable rather than a hazard (follow-on
`PIPE-DROP-STAGE`).

## 4. The one production change: the sequential route's statement reader

### 4.1 The measurement

`crates/qcore-verification/src/driver.rs::statement_tokens` read

```rust
let slice = &source[span.start..span.end];
let mut tokens = lex(slice)?;
```

— the statement's **raw source bytes**, re-lexed. For every pipe-free file that
is the same token kinds in the same order as the parsed program carries, the
spans differing only in being slice-relative. For a desugared pipeline it is not,
in two ways at once: the synthesized delimiters and the temporary are missing,
and the tokens that survive are in the order the *bytes* have them rather than
the order the *statement* has them. In a call stage the operand sits to the left
of the callee in the bytes and to the right of it in the statement.

The observable consequence was exact. `parse_acquire` matches
`NAME "(" INTEGER "," WORD "," WORD ")" ";"` after the `=`, so

```qcore
let a = 1 |> qcore_acquire(true, Failure);
```

handed it `1 |> qcore_acquire ( true , Failure )` and it refused with

> `` `let` must use `qcore_acquire(resource_id, succeeds, ErrorType)` in the
> executable subset ``

naming the very form the reader had written. Its hand-written twin was admitted.
This is not a fail-closed hazard — it refuses — but it falsifies the sentence
§L21 has carried since card `PIPE-OPERATOR`: "the pass runs before every other
one … so no later pass can tell a pipe was written". That was true of the token
*stream* and not of every reader of it.

### 4.2 The change, and why it is parity rather than invention

```rust
fn statement_tokens(program: &SourceProgram, span: Span) -> Result<Vec<Token>, SourceDiagnostic> {
    Ok(program
        .lexical_tokens()
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .cloned()
        .collect())
}
```

That is **the same span filter
`qcore_frontend::source_extern::statement_tokens` already uses**, and the same
one `source_std_io::window` and `source_tasks::tokens_for` use. (The extern
reader additionally refuses an empty result; this one does not need to, because
an empty span produced an empty list under `lex("")` too and reaches the same
refusals it always did.) The sequential route was the lone outlier; every other
route has read the desugared stream since its own card. Three properties make
the change safe to state rather than merely to test:

1. **Identical on every pipe-free file.** `SourceProgram::lexical_tokens` *is*
   the lex of that source, and a statement's tokens are exactly those whose spans
   lie inside its own. Lean:
   `the_two_readers_agree_on_a_pipe_free_statement`.
2. **No consumer reads a returned token's span.** Every diagnostic in the
   sequential lowering and in `source.branch.v1`'s condition tier is attributed
   to `statement.span` or `*condition_span`; the token spans were slice-relative
   before and are absolute now, and nothing observes the difference. Checked by
   inspection over both call sites and by the whole workspace suite.
3. **Conditions are untouched by construction.** The desugarer only rewrites a
   `|>` at parenthesis depth zero, so no pipeline is ever rewritten inside an
   `if` condition and no synthesized token can land in a condition's span. A
   `|>` written in a condition still reaches the condition tier and still
   refuses. Pinned: `a_decidable_condition_still_reads_the_same_tokens`.

### 4.3 What it admits

`the_sequential_route_reads_the_desugared_stream_not_the_source_bytes` pins
`let a = 1 |> qcore_acquire(true, Failure);` emitting the twin's C byte for byte,
and `the_sequential_routes_check_is_a_stage` pins
`true |> qcore_check(Failure);`. Both are the sequential route's own intrinsics,
so the card's positive claim is uniform across the census: **every call-shaped
intrinsic in the language is a pipeline stage, on every route that has it.**

The desugarer's own refusals are unchanged and pinned in the same test: an
ill-formed pipeline is left untouched, its residual `|>` survives, and `Q036`
still reports it.

### 4.4 Proposed standing constraint

This card's production change is exactly the kind of rule the `AGENTS.md`
"standing subsystem constraints" inventory exists for, so it is proposed here
rather than assumed (the integrator lands it; a delegated card does not edit
`AGENTS.md`):

> **A route reads its statement tokens from the parsed program's
> post-desugaring stream, never by re-lexing source bytes.** `source.pipe.v1`
> rewrites the *token stream* before parsing, so a re-lex of `source[span]`
> recovers the reader's pipeline rather than the statement it desugared to — the
> operand and the callee change places, and the synthesized delimiters and
> hygienic temporaries are absent entirely. Every reader is the same span filter
> (`source_extern::statement_tokens`, `source_std_io::window`,
> `source_tasks::tokens_for`, `driver::statement_tokens`); a new one must be too,
> and none of them may attribute a diagnostic to a *token's* span, since those
> are zero-width for everything the desugaring invents. `wave22-pipe-intrinsic-stage`.

## 5. What did not move, and what this card did not do

* **No diagnostic code was added, moved or changed**, and no reserved word was
  added. `Q036` and `Q037` keep their conditions verbatim; what §L21 records is
  a **restatement** of the row's first sentence through the §L21 process, on the
  `OR-AND-LOWERING` precedent the brief named. `Q051+` stays free for
  `STD-BUFFER`/`STD-EVENT`.
* **The checkers are unedited.** No custody, effect, linearity, concurrency or
  emission rule changed; every refusal in §3 predates this card.
* **The closed sink table is unchanged.** `print`, `log`, `flush` and nothing
  else. Adding `drop` would make a custody-releasing statement pipeable, which
  is a decision of its own (`PIPE-DROP-STAGE`).
* **`examples/threads/owned_buffer.qcore` was deliberately not rewritten.** Its
  `let local = qcore_buffer_read(second); local |> log info dec;` pair is exactly
  the chain this card admits and would compile to identical C, but the example
  files are shared ground with the `STD-BUFFER` and `STD-EVENT` cards of this
  wave. Follow-on `PIPE-INTRINSIC-EXAMPLE`.
* **The `ValueId` wall and `Q028`'s operand rule stand.** An intrinsic stage's
  scalar result is a value binding exactly like `PIPE-SCALAR-RESULT`'s. `Q028`
  is the `source.std-io.v1` rule that "keeps every resource unrenderable through
  a bare operand", and its composed counterpart on this route is the `Q050`
  operand rule: the temporary an intrinsic stage binds is a task-model slot, the
  rendering admits `ValueType::Scalar(Scalar::I32)` and no other type, and a
  custody-bearing temporary is therefore refused at the sink rather than
  rendered (§3.3). A hygienic temporary gains no privilege a written binding
  lacks, and nothing crosses the namespace.

## 6. Deviations from the brief, recorded

1. **The brief expected a desugaring change; the census says none is needed for
   the task routes.** Recorded as a deviation because it is one, and because the
   wave-21 note it supersedes is explicit ("a pipe-grammar change … the sink
   stage's selector-word rule"). The measurement is §1: the intrinsic is
   call-shaped, and card `PIPE-SCALAR-RESULT` widened "produces a value" to "is a
   call stage" a wave early, so the admission was already structural. The tests
   in §2 and §3 are what turn that from an accident into a decision — if the
   stage-kind rule is ever narrowed again, they fail.
2. **One production change the brief did not anticipate.** §4. It is in
   `qcore-verification`'s sequential lowering rather than in the pipe pass, and
   it is included because without it the census's own claim — "every call-shaped
   intrinsic is a stage" — would be false on one route for a reason unrelated to
   intrinsics. It weakens no gate: the shapes it admits are the ones whose
   hand-written twins were already admitted, and the emitted C is compared.
3. **`buffer |> read |> render |> log` is not the landed spelling.** §1.1. Two
   of its four stages do not exist as words in this language. The landed spelling
   is `returned |> qcore_buffer_read |> log info dec`, and the report says so
   rather than presenting the chain as the brief's sentence.

## 7. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `PIPE-DROP-STAGE` | `buffer \|> drop;` and the other keyword-shaped statement heads as stages. Needs a decision about the closed sink table — `drop` takes an operand like `print` does, so it *fits*, but it releases custody, and a stage table entry that releases is a different kind of entry from one that renders. It is also what would make §3.4's allocating chain spellable end to end. |
| `PIPE-INTRINSIC-EXAMPLE` | rewrite `examples/threads/owned_buffer.qcore`'s reading pair as the chain. Blocked only by wave-22 file sharing, not by anything technical. |
| `SEQ-SCALAR-RESULT` | *(unchanged)* `PIPE-SCALAR-RESULT`'s follow-on for a non-acquiring scalar `let` on the sequential route. §4 removes one obstacle to *reaching* that route through a pipe but adds no `let` form. |
| `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY`, `PIPE-ASSIGN`, `PIPE-TAP`, `PIPE-BINDER-SPAN`, `PIPE-BORROW-SINK` | *(unchanged)* |
| `COMPOSE-WORKER-LINES`, `COMPOSE-TASK-BORROW`, `COMPOSE-TASK-EXTERN` | *(unchanged, `COMPOSE-CONCURRENCY` §9)* |
| `PIPE-INTRINSIC-STAGE` | **retired** by this card. |

## 8. Evidence

### 8.1 The standing gate

Run in this worktree on the wave-21 close (`ec5535a9`) plus this card. The
workspace suite was run **per package** rather than as one `cargo test
--workspace` invocation: five sibling cards were building and running the
Clang-backed route-parity suites on the same host, and a single whole-workspace
run exceeded the session's own command window three times before finishing. The
union of the per-package runs is the same set of tests, and each run's own exit
code is recorded.

| Step | Result |
|---|---|
| `cargo test` — `qcore-core`, `qcore-frontend`, `qcore-contract`, `qcore-schemas`, `qcore-c-gateway`, `qcore-verifier-host`, `qcore-audit`, `qcorec` | exit 0 · 100 suites · **2,233 passed, 0 failed, 8 ignored** |
| `cargo test -p qcore-verification` | exit 0 · 35 suites · **265 passed, 0 failed** |
| `cargo test -p qcore-compiler` | exit 0 · 55 suites · **864 passed, 0 failed** |
| **workspace total** | **190 suites · 3,362 passed · 0 failed · 8 ignored** (+19 from this card) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | exit 0 · **90 Lean audits passed**, no project axiom (the nine new theorems use only `propext` and `Quot.sound`) |
| `python3 -m unittest discover -s scripts/tests` | **193 passed** |

No constraint-16 flake was tripped; nothing needed an isolated re-run.

### 8.2 Artifacts

| Artifact | Where |
|---|---|
| Tests | `compiler/tests/pipe_intrinsic_stage.rs` (19 tests: 18 plus one Linux/macOS sanitized ASan/UBSan native run) |
| Production change | `crates/qcore-verification/src/driver.rs::statement_tokens` |
| Lean | `proof/QCore/PipeDesugaring.lean` §5.5–§5.6, registered in `proof/AuditPipeDesugaring.lean` |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 `Q036`–`Q038` row (restatement) |
| Grammar | `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02h, "A model intrinsic is a call stage" |

## 9. Draft increment paragraph

> The intrinsic pipe stage (PIPE-INTRINSIC-STAGE): the census
> `COMPOSE-CONCURRENCY` asked for finds that the source language's model
> intrinsics are **call-shaped** — `qcore_buffer_new`, `qcore_buffer_read`,
> `qcore_buffer_write`, `qcore_error_payload` on the task routes,
> `qcore_acquire` and `qcore_check` on the sequential one — or **keyword-shaped**
> (`spawn`, `join`, `share`, `drop`, the mutex and epoch clause words), and that
> the pipe grammar already decides both: a call-shaped intrinsic is a call stage
> by the definition card `PIPE-SCALAR-RESULT` landed, so
> `returned |> qcore_buffer_read |> log info dec` desugars to the statement pair
> the composed task route already lowers and emits **byte-identical C11** against
> its hand-written twin, sanitized-native compared per stream; a keyword-shaped
> one is unreachable because no call stage produces a statement whose operands
> are not a parenthesized argument list, proved as an inequality rather than left
> to the closed table's silence. The brief's literal
> `buffer |> read |> render |> log` is **not** the landed spelling and cannot be:
> `read` is a reserved concurrency keyword rather than a function and `render` is
> a sink selector word, so the honest chain is two stages. One production change:
> the sequential route recovered each statement's tokens by **re-lexing its raw
> source bytes**, the last place a later phase could tell a `|>` had been
> written — for a pipeline that recovers the pipeline, so that route's own
> `qcore_acquire` refused through a pipe while its hand-written twin was
> admitted — and it now uses the same span filter over the parsed program's
> stream that every other route has used since its own card, identical on every
> pipe-free file and observed by no diagnostic, since all of them attribute to
> the statement or condition span. Every refusal is pinned by name and every one
> is the route's own: a discarded intrinsic result at `Q500`, a `unit`-returning
> intrinsic mid-chain at `Q050` naming the type rather than `Q037` naming the
> pipe, an allocating intermediate leaking its unnameable temporary at `Q600`
> under the unedited checker, `move` into a read, `spawn`/`join`/`drop` as
> stages, and an intrinsic stage inside a mutex generation keeping that route's
> refusal. The `ValueId` wall stands: a custody-bearing temporary is refused at
> the sink because the composed rendering operand rule admits `i32` and nothing
> else. Nineteen new tests, nine new audited Lean theorems, **no diagnostic
> code added, moved or changed and no reserved word** — §L21's `Q036`–`Q038` row
> and grammar §G02h restate the condition in the same change, on the
> `OR-AND-LOWERING` precedent. Corpus-neutral by construction. Evidence:
> `docs/reports/wave22-pipe-intrinsic-stage/README.md`.
