# PIPE-OPERATOR — `|>`, and the operator token family (wave 18)

Card brief: *"The first revision of `std.io` reads poorly — its statement forms
don't compose. Add a pipe operator `|>` that composes calls so streams read
naturally for functional programmers."* Plus two mid-card amendments claiming
`|` as logical or and `&&` as logical and, with `&` reserved.

Base commit: main `ad58360`.

**What landed.** The pipe operator, in full: a pure parse-time token
desugaring, with attribution preserved, byte-identical C emission against a
hand-unpiped twin, linearity through a pipeline exercised on the real extern
route, a sanitized native run, and 35 audited Lean theorems. The two logical
operators landed as **decided tokenization plus an actionable refusal**, not as
lowerings — on a measured blocker, under the scope guard both amendments
carried. §4 is that measurement and it is the most consequential finding in
this card.

---

## 1. The measurement that shaped every design decision

The brief's decision 1 said `a |> f(b, c)` rewrites to `f(a, b, c)` so "the
checkers see an ordinary **call tree**". There is no call tree.

| Probe | Result | Evidence |
|---|---|---|
| An expression AST | **none** | `StatementKind::Expression` is a payload-free marker (`crates/qcore-frontend/src/frontend.rs:174-227`). There is no expression node of any kind. |
| A user-callable function call | **none** in the sequential subset | a bare expression statement refuses: `Q500: unsupported expression statement in the Cycle 12 executable subset` |
| A call form anywhere | **yes, two** | `source.extern-call.v1`'s `CALLEE(ARGS);` and `let N = CALLEE(ARGS) succeeds …;` (G02d) — parsed from raw token windows, not an AST |
| A statement form with an operand | **yes** | `source.std-io.v1`'s `print RENDERING OPERAND;` and `log LEVEL RENDERING OPERAND;` (G02g) |
| How every route reads its own statement | **a byte-span token window** | `source_std_io::window`, `source_extern::statement_tokens` — each filters the program's tokens by the statement's span |

So the architecture that actually exists is: **every route parses its own
statements out of a token window.** That makes the honest realization of
decision 1 a *token-stream* desugaring rather than an AST rewrite — and it is a
strictly stronger form of the same claim. An AST rewrite would produce a tree
that *represents* the same program; a token rewrite produces literally the same
tokens, so `SourceProgram::syntax_equivalent` holds between a piped program and
its hand-written twin. That is pinned
(`a_piped_program_and_its_hand_written_twin_are_syntactically_the_same_program`)
and it is the strongest available statement of "zero new checker rules".

---

## 2. The desugaring specification

`source.pipe.v1` is **not a profile**: a file does not select it, no route
knows it exists, and `SourceExecutionProfile` is unchanged. The rewrite runs in
`SourceParser::new`, after `lex` and **before** the delimiter map and before any
parsing, so a synthesized `(` participates in delimiter matching exactly as a
written one does.

```ebnf
stmt      = OPERAND, "|>", STAGE, ";" ;
OPERAND   = token, { token } ;             (* verbatim: `x`, `-7`, `move b` *)
STAGE     = "print", RENDERING             (* a G02g sink *)
          | "log", LEVEL, RENDERING        (* a G02g sink *)
          | NAME, [ "(", [ arg, { ",", arg } ], ")" ] ;   (* a G02d call *)
```

| Written | Desugars to |
|---|---|
| `total \|> print dec;` | `print dec total;` |
| `total \|> log fatal dec;` | `log fatal dec total;` |
| `x \|> f;` and `x \|> f();` | `f(x);` |
| `b \|> f(7);` | `f(b, 7);` |
| `move b \|> release;` | `release(move b);` |

### 2.1 Decision record: first-argument insertion

**Decided: the piped value occupies the stage's first operand position**
(Elixir-style), and the rejected alternative is last-argument insertion
(Haskell/F#-style `&`/`|>` composition).

The argument is not taste. Last-argument insertion earns its keep in curried
languages, where `f a b` partially applied is a function and `x |> f a` is
ordinary application. **QCore has no currying and no partial application** —
there is no expression tier at all (§1) — so a last-argument rule would buy
nothing and would read against the language's subject-first grammar, where the
thing being acted on is named first (`drop b`, `move b`, `print dec total`).

The one wrinkle is that "first argument" needs a definition for a *statement*
stage, which has a head and closed selector words rather than a parenthesized
list. The rule adopted: **a statement head and its closed selectors are not
arguments**, so the first operand position is after them. For a parenthesized
call it is immediately after `(`, before anything already written. Both v1
statement stages take exactly one operand, so "first operand position" and
"append" coincide and no selector-arity table is needed; a multi-operand
statement stage would need one, and that is recorded as `PIPE-STAGE-ARITY`.

The insertion position is **observable, not asserted**: reversing it
(`7 |> qcore_extern_fill(b)` instead of `b |> qcore_extern_fill(7)`) is refused
by the extern bridging table, pinned by
`the_argument_order_of_a_call_stage_is_the_declared_one`.

Left-associativity is specified in the grammar but currently vacuous, because a
pipeline has exactly one stage (§2.3).

### 2.2 Attribution design

The brief called this the real machinery touch, and it is: it is the reason the
desugaring is a *permutation of the existing token vector* rather than a
re-lex of rewritten text.

* **Every token the reader wrote keeps its own span, unmoved.** A refusal
  inside a desugared statement therefore points at the bytes that were typed:
  `an_operand_fault_is_attributed_to_the_piped_operand` (the operand's span),
  `a_stage_fault_is_attributed_to_the_stage` (the stage's).
* **Only `(`, `)` and `,` are synthesized**, each anchored **zero-width at the
  `|>` it came from** — so even a fault landing on punctuation names the pipe,
  which is the truthful origin, rather than an invented position. Nothing that
  could carry meaning is ever invented (`only_delimiters_are_synthesized`).
* **One generalization was required.** `SourceParser::range_span` computed a
  statement's extent as `first.start .. last.end`. After a permutation the
  first token is no longer the leftmost, so that formula would have produced a
  span *excluding the operand*, and every route's byte-span token window would
  then have dropped it. It is now the minimum start and maximum end over the
  range. For a token range in source order — every range in a program with no
  pipe — **the two agree byte for byte**, so no pinned golden moves. This is a
  correction as much as an extension: a statement's span should be the extent
  of its tokens.
* **No new code for an existing refusal class.** `Q013`, `Q028`, `Q030`,
  `Q024`, `Q202` and the rest all fire through a pipe exactly as they do
  without one, and the tests assert the *piped and unpiped diagnostic vectors
  are equal*, not merely that both refuse.

### 2.3 What v1 does not do, and the measured reason

**A pipeline has exactly one stage.** `x |> f |> g` refuses at `Q033`. The
user's own example shape, `value |> render |> log`, is therefore **not
admitted in v1**, and saying so plainly matters more than shipping something
that looks like it. Two independent blockers, both measured:

1. **A call stage's result has no name.** QCore source has no expression tier
   (§1), so `f(x)` cannot be an operand of anything. This is not a parser
   limitation; there is nowhere in the language for the value to live.
2. **A chain of *statement* stages would need statement-sequence expansion**,
   and that is incompatible with the byte-span statement windows every route
   uses. `x |> print dec |> log info dec;` would have to become two statements
   sharing one operand token, so their statement spans would overlap and each
   route's `window()` would return both statements' tokens for either. The
   pass-through ("tap") semantics that would make sink chaining *sound* — the
   operand is structurally a scalar, so duplicating it is not aliasing — are
   available; the windowing is the blocker. Recorded as **`PIPE-CHAIN`**, whose
   real prerequisite is index-based statement windows or an expression tier.

**A pipeline is a whole statement.** `let r = x |> f succeeds …;` refuses at
`Q032`; follow-on **`PIPE-LET`**.

---

## 3. Composability of the std.io sinks

The brief asked to extend the operand/sink grammar "only as far as the pipe
needs", with every non-extension a named refusal. The measured answer is that
**`source.std-io.v1` needed no extension at all** — `source_std_io.rs` is
byte-for-byte unchanged by this card — because the desugaring hands it exactly
the statements it already accepts. The Q028 rule (a rendering operand cannot
denote a resource) is untouched and is re-pinned through a pipe
(`a_resource_operand_is_still_unrenderable_through_a_pipe`, measured
`["Q030", "Q028"]`, the same vector as the unpiped case).

| Sink / stage | Pipeable? | Evidence or reason |
|---|---|---|
| `x \|> print dec` | **yes** | `the_print_pipeline_is_the_print_statement`; native run |
| `x \|> print hex` | **yes** | fixture `lines_piped.qcore` |
| `x \|> log info dec` / `warn` / `fatal` | **yes** | fixture; all three levels exercised |
| `-7 \|> print dec` | **yes** | a literal operand pipes; fixture |
| `x \|> flush` | **refused, `Q033`** | `flush` takes no operand — the brief's own instruction, pinned |
| `x \|> print dec \|> log info dec` | **refused, `Q033`** | `PIPE-CHAIN` (§2.3) |
| `x \|> print` (no rendering) | **refused, `Q032`** | wrong selector arity |
| `x \|> print dec hex` | **refused, `Q032`** | wrong selector arity |
| a resource as the piped operand | **refused, `Q030`+`Q028`** | the v1 operand type has no resource constructor; unchanged |
| `let y = x \|> …` | **refused, `Q032`** | `PIPE-LET` |
| **extern call stage** `move b \|> release` | **yes** | this is where linearity is real — §5 |
| a `drop`/`move`/`defer` statement as a stage | **refused, `Q032`** | not in the closed stage table; `PIPE-STMT-STAGES` |

**Composition with other routes**, measured rather than assumed: a pipeline
does not change route selection at all
(`a_pipeline_does_not_change_the_selected_route`). Because the pipe is gone
before the selectors run, a piped `print` inside a `thread scope` keeps the
task route's existing refusal exactly as the unpiped one does — the
`STD-IO-COMPOSE` boundary is neither widened nor narrowed. Extern-call adapters
are the one place a pipe *does* reach a concurrency-adjacent surface, and it
reaches it as an ordinary call.

---

## 4. The operator family, and why two of them did not land

Both amendments carried an explicit escape hatch ("measure, don't guess") and a
scope guard (pipe is the priority; tokenization + refusals may land with the
lowering as a follow-on). Both fired, on the same measurement.

**There is nothing to lower a short-circuit onto.**

| Probe | Result |
|---|---|
| `if (a) { … }` in the executable subset | `Q500: structured control flow is not in the Cycle 12 executable subset` — **`if` itself is refused** |
| A branch constructor in the lowering IR | **none.** `qcore_core::cleanup_lowering::SourceOp` is exactly `Acquire{succeeds: bool}`, `Check{passes: bool}`, `Drop`, `TransferToResult`, `Return`. Control flow comes only from *literal* outcome controls. |
| `if` statements in the 294 source-side `.qcore` files | **zero** |
| A boolean expression tier to extend | **none** (§1) |
| What `StatementKind::If` lowers to | nothing — it is handled at exactly two sites, name-resolution reference collection and a pass label |

Amendment 2 item 1 said to lower onto "the EXISTING branch/CFG constructs
(nested branching)". There are none. Item 3 said not to invent a boolean
expression tier. Building the lowering would require inventing both a branch IR
and an expression tier — far past a bounded landing, and a different card.

**What landed instead** is the half that actually prevents a future collision:
both tokens are claimed now, with `||` refused and an actionable message in
both directions.

### 4.1 The operator table

This is the table future waves build against.

| Spelling | Token | Status |
|---|---|---|
| `[\|` / `\|]` | `QuantOpen` / `QuantClose` | **unchanged** — G04's quant-index delimiter alias |
| `\|>` | `PipeInto` | **landed** — desugared away before parsing |
| `\|` | `Pipe` | **claimed, refused at `Q034`** — logical or; lowering is `OR-LOWERING` |
| `\|\|` | `Pipe` `Pipe` | **refused at `Q034`**, message: *"QCore spells logical or as a single `\|`, not `\|\|`"* |
| `&&` | `AndAnd` | **claimed, refused at `Q034`** — logical and; lowering is `AND-LOWERING` |
| `&` | `Ampersand` | **reserved, unchanged** — held for a future binary/bitwise operation |

**The asymmetry is deliberate and recorded as the user's declared convention:**
or is the single `|` and and is the double `&&`, because the single `&` is
being held for binary/bitwise use. Both messages redirect a C-family reader
correctly: typing `||` is told or is `|`; typing `&&` is told the single `&` is
reserved and is *not* logical and. Both are pinned
(`the_c_family_double_bar_is_redirected_to_the_single_bar_spelling`,
`the_logical_and_operator_refuses_and_redirects_the_single_ampersand`).

### 4.2 Token-hazard record

**Census, and one correction to the amendment's premise.** The premise —
"today we only use the `|` symbol inside `[|` and `|]`" — is confirmed with a
refinement that makes it *stronger*: `[|` and `|]` appear in **no `.qcore`
file at all**. The only `|` byte anywhere in the 294 source-side files is
inside a `//` comment in one contract file. The quant alias is exercised only
from Rust test strings (`compiler/tests/quant_index_contracts.rs`,
`crates/qcore-frontend/tests/unit/lexer.rs`) and documented in the spec. And
`&` was measured to be **consumed by nothing**: `Symbol::Ampersand` is produced
by the lexer and matched at zero sites in the whole workspace, so leaving it
alone is genuinely leaving it alone.

**Tokenization rule, stated in the grammar doc:** two-byte spellings are tried
before every one-byte spelling — longest match at a fixed width of two. The
four vertical-bar-family spellings are pairwise distinct byte pairs, so their
order in the table is immaterial and the resolution is decided by width alone.

**Adjacency pins** (`crates/qcore-frontend/tests/unit/lexer.rs`, two new
tests):

| Input | Lexes as | Note |
|---|---|---|
| `[\|>` | `QuantOpen` `Greater` | matched at `[`, so **never** `[` `\|>` |
| `[\|>\|]` | `QuantOpen` `Greater` `QuantClose` | |
| `a \|> b` | `PipeInto` | the pipe wins by longest match |
| `a \| > b` | `Pipe` `Greater` | the spaced spelling |
| `a \|\| b` | `Pipe` `Pipe` | grammar refuses with the actionable message |
| `\|>\|` | `PipeInto` `Pipe` | decided; the trailing `\|` refuses |
| `a [ \| ] b` | `LBracket` `Pipe` `RBracket` | |
| `a && b` | `AndAnd` | |
| `a & & b` | `Ampersand` `Ampersand` | |
| `&&&` | `AndAnd` `Ampersand` | decided; refuses in the grammar |
| `&Buffer` | `Ampersand` `Word` | byte-identical to before this card |

**The one genuine future ambiguity, and its baked-in resolution:** `a |> b`
against a hypothetical `a | > b`. `|>` wins by longest match — the standard
resolution — so the or-operator wave must live with it and write the spaced or
parenthesized form. That constraint is recorded **now** rather than discovered
later, which is what the first amendment asked for.

**One deliberate pin update.** A bare `|` was a **lex error** before this card,
asserted by name in `rejects_a_bare_vertical_bar_outside_quant_delimiters` and
by behaviour in `mixed_or_unterminated_quant_delimiters_are_refused_not_repaired`.
It is still refused, with an actionable message instead of `unexpected
character '|'`. **The set of accepted programs is unchanged; only the message
and the layer move.**

Which layer, exactly, was a measured correction to this card's own first
answer. The initial edit assumed the refusal moved to *the quant-index
grammar* and kept the pin as `parse_source_program(...).is_err()`. The full
workspace gate caught it: `quant<n where n := 5> | i32` now **parses**, and
the refusal is raised one step further out, by the frontend's operator gate at
`Q034`. The pin is rewritten to assert what actually happens — the program
parses, and `compile_files` refuses it with exactly `["Q034"]` — rather than
being relaxed to "is refused somehow". The other four mixed/unterminated
delimiter cases still fail at parse and are untouched, a new
`quant[|… |>` mixed-pair case joins them, and
`a_single_ampersand_gains_no_new_meaning` asserts a lone `&` still refuses and
**never at `Q034`**, which would mean this card had claimed it.

### 4.3 Flagged, not resolved: bitwise or has no symbol

With `|` claimed by logical or and `&` held for bitwise and, **bitwise *or*
currently has no spelling.** This card does not invent one. The options a
future bitwise wave has — a spelled operator, a different sigil, or revisiting
the or/and asymmetry — are all still open, and nothing landed here forecloses
them: `|>`, `|`, `&&` and `&` are the only claims made, and `&` is a
reservation rather than a meaning.

---

## 5. Linearity through a pipeline

This is where the card's soundness claim is actually testable, and it is
testable *because* the extern route has real custody. Fixture
`compiler/tests/fixtures/pipe/round_trip_piped.qcore` is the piped twin of the
shipped `extern_calls/round_trip.qcore`:

```qcore
let b = qcore_extern_alloc(8) succeeds true else raise Failure;
b |> qcore_extern_fill(7);
b |> qcore_extern_sum(8);
move b |> qcore_extern_release;
```

| Property | Evidence |
|---|---|
| The consuming call checks through the pipe | `an_owned_resource_piped_into_a_consuming_call_is_consumed` — **byte-identical `c_source`** and identical reference observation against the unpiped twin |
| A use after a piped consumption refuses | `using_a_binding_after_piping_it_into_a_consumer_refuses` — the piped and unpiped **diagnostic vectors are equal** (`["Q024"]`) |
| A pipeline that leaks refuses | `a_pipeline_that_leaks_refuses_exactly_as_its_unpiped_twin_does` — equal vectors (`["Q202"]`) |
| Positive controls | both twins compile and check |

The point of asserting *vector equality* rather than "both refuse" is that it
rules out the failure mode where a pipe refuses for an accidental reason and
looks sound.

---

## 6. Deviations found by measurement

1. **There is no call tree** (§1). The brief's decision 1 is realized as a
   token-stream desugaring, which is a strictly stronger version of the same
   claim (`syntax_equivalent`, not merely "represents the same program").
2. **The `|` / `&&` lowering has no target** (§4). Recorded rather than faked,
   under both amendments' scope guards.
3. **The amendment's `|` premise was stronger than stated** (§4.2): `[|`/`|]`
   appear in no `.qcore` file at all, and `&` is consumed nowhere.
4. **`source_extern::statement_tokens` re-lexed the original source bytes**
   (`lex(&source[span])` plus span shifting), which bypassed the desugaring
   entirely — a piped statement reached the extern route in its *written* form
   and was refused at `Q500`. Found by a failing test, not by reading. It now
   filters the program's own tokens by span, exactly as `source_std_io::window`
   does. For a program with no pipe the two agree token for token; every
   existing extern fixture is unmoved. This is a correctness fix that also
   removes a second, divergent tokenization path.
5. **A pipeline missing its `;` mid-body runs into the next statement.** The
   next statement's `;` terminates the run, so the following head becomes a
   second stage and the multi-stage refusal fires first (`Q033`, not `Q032`).
   Fail-closed and correctly spanned; pinned as measured rather than forced.
6. **A pipe nested inside an argument list is not desugared.** The run scanner
   records a `|>` only at bracket depth 0, so `f(move b |> g)` is left alone;
   the residual `|>` survives into the tokens and the route's own grammar
   refuses the argument. Fail-closed, but it is the *route's* refusal rather
   than a pipe diagnostic, and it is pinned so a future widening must decide
   the case deliberately rather than inherit it.
7. **Dropping the `;` of a body's *last* statement is `Q000`,** not a pipe
   diagnostic: the parser's own "a statement ends in `;`" rule runs before any
   frontend gate. The desugarer's block-boundary arm still exists and still
   prevents a pipeline swallowing a block, but is unreachable from that
   direction; pinned as the reachable truth.
8. **`range_span` needed generalizing** (§2.2) — byte-identical for every
   source-ordered range, so no golden moved.
9. **The bare-`|` pin moved further than this card first assumed** (§4.2). The
   first edit kept it as a parse-level refusal; the workspace gate caught that
   `quant<…> | i32` now parses and is refused by the operator gate at `Q034`.
   The pin was rewritten to assert the measured behaviour exactly, not relaxed.

---

## 7. Evidence

### 7.1 Tests

`compiler/tests/pipe_operator.rs` — **28 tests, all passing**, including one
sanitized native run. Two new fixtures under `compiler/tests/fixtures/pipe/`
(`lines_piped.qcore`, `round_trip_piped.qcore`), each the piped twin of a
shipped fixture so the comparison is against real pinned code rather than a
program written to make the test pass. Two new lexer unit tests pin the
adjacency table.

### 7.2 The byte-equality emission pin

`the_piped_and_unpiped_programs_emit_byte_identical_c` asserts
`compile(piped).c_source == compile(unpiped).c_source` for the std.io twin
under **both** line-ending conventions, and
`an_owned_resource_piped_into_a_consuming_call_is_consumed` asserts the same
for the extern twin. Reproducible from the CLI:

```
qcorec emit -o piped.c   fixtures/pipe/lines_piped.qcore
qcorec emit -o unpiped.c fixtures/std_modules/lines.qcore
diff piped.c unpiped.c        # empty
```

### 7.3 Sanitizer evidence — reported honestly

One fixed ASan/UBSan/LSan run, x86_64 Linux, `-std=c11 -Wall -Wextra -Werror
-pedantic -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`:
`lines_piped.qcore` compiles clean under `-Werror`, runs with exit 0, and its
stdout and stderr match the reference's projections **per stream**
(`the_native_run_of_a_piped_program_matches_its_reference_per_stream`).

Honest scope, carried forward from STD-MODULES: this program allocates
nothing, so the leak check is a *negative control*. The extern twin's custody
is exercised by the checker and by emission equality, not by a second native
run — the shipped `extern_calls` suite already runs that C, and this card emits
the identical bytes.

### 7.4 Lean

`proof/QCore/PipeDesugaring.lean`, audited by `proof/AuditPipeDesugaring.lean`:
**35 theorems**, no new axiom and no placeholder — every one depends on
nothing, or only on the allowed `propext` / `Classical.choice` / `Quot.sound`.
`QCore.lean` imports the module, so `AuditAll.lean` includes it in the full
axiom closure. `check_lean_audits.py` moves **77 → 78** audits.

Load-bearing statements, by family:

* **The identity** — `sink_pipeline_is_its_hand_written_statement`,
  `log_pipeline_is_its_hand_written_statement`,
  `call_pipeline_is_its_hand_written_call`, with `handWritten` defined
  *independently of* `rewrite` so the theorem has content, plus the two
  concrete instances the fixtures pin and
  `an_existing_argument_follows_the_piped_value` (first-argument insertion).
* **Neutrality** — `pipe_free_is_unchanged`,
  `a_pipe_free_stream_stays_pipe_free`, and the three
  `…_contains_no_pipe` theorems (the operator is consumed, not relocated).
* **Attribution** — `rewrite_preserves_written_origins`,
  `a_sink_pipeline_synthesizes_nothing` (a sink pipeline is a *pure*
  permutation), `synthesized_tokens_are_anchored_at_the_pipe`,
  `only_delimiters_are_synthesized`.
* **Fail-closed** — six refusal theorems plus `refused_shapes_rewrite_to_nothing`
  as one list (so a future widening must change a theorem rather than slip
  past it), `a_refused_statement_is_unchanged` and
  `a_refused_statement_keeps_its_pipe`.
* **The operators** — `the_or_operator_refuses`, `the_and_operator_refuses`,
  `the_double_bar_refuses`, `a_single_ampersand_is_not_claimed` and
  `the_operator_family_is_exactly_as_declared`.
* **Tokenization** — `the_quant_delimiters_are_untouched`,
  `the_pipe_wins_by_longest_match`, `the_and_wins_by_longest_match`,
  `the_two_byte_table_is_injective`,
  `three_ampersands_are_and_then_reserved`.

**Not proved:** that the Rust desugarer extracts these token runs from real
source the way the model says (source token extraction is a tested Rust
boundary, per the `ConcurrencyPrelude`/`StdIo` precedent); that any route's
grammar accepts a rewritten statement (it need not, and its own diagnostic is
then the answer); and anything about `|`/`&&` *semantics*, which were not
implemented.

### 7.5 Corpus neutrality

**Corpus-neutral by construction.** This is source-language work: no gateway,
contract-loader, audit or provider file was touched, no `contract/qcore/` file
changed (corpus stays **67 modules**), no CT manifest entry changed
(**32/50**), no pinned artifact was edited. The expectation to verify at
consolidation is that the canonical CMetrics artifacts and the 185-unit
baseline are **byte-identical**, with the headline unchanged at 2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only.

The one place a corpus effect could hide is the lexer: `|` and `&&` are now
tokens. Both were measured absent from every `.qcore` file (§4.2), and
`no_existing_program_changes_meaning` pins that shipped fixtures record no pipe
refusal and parse identically to themselves.

### 7.6 CT candidates

None proposed. The pipe adds no checked model and maps no conformance case; a
piped fixture is the same program as its unpiped twin, so mapping one would
grow the denominator without adding evidence.

### 7.7 Gate

| Gate step | Result |
|---|---|
| `cargo test -p qcore-compiler --test pipe_operator` | **28 pass**, including the sanitized native run |
| `cargo test -p qcore-frontend` | **225 pass** across 16 targets, 0 failed |
| `cargo test --workspace --no-fail-fast` | **2,931 passed, 0 failed, 8 ignored** across 176 targets, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | 186 tests, **1 expected failure** — see the closing note |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean; one unused import from this card's own change found and removed, not suppressed |
| `check_lean_audits.py` | **78/78 pass** (77 → 78); no new axiom |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |

One environment finding, recorded rather than worked around: the
`third_party/libevent` submodule was not initialized in this fresh worktree, so
three suites failed the pinned-revision check on the first workspace run. The
pin was **not** weakened; the gate's own documented
`git submodule update --init --recursive` fixed it and all three pass. The
2,931/0/8 figures above come from one complete run after that.

**Left for the integrator, by the brief's own instruction:** the derived
`lean_audits` count in `docs/reviews/r0_3/acceptance_matrix.json` now lags by
one (77 vs 78), so `scripts/tests`'s
`test_the_derived_facts_are_recomputed_rather_than_carried` will report
`lean_audits: 77 != 78`. That is the derived-value tripwire working as
designed; this card is forbidden from editing acceptance files, so the matrix is
deliberately left stale for regeneration, the same handling every wave's
derived sections receive.

---

## 8. Named follow-ons

| Name | What it is |
|---|---|
| `PIPE-CHAIN` | multi-stage pipelines. Real prerequisite: index-based statement windows (so a statement sequence can share an operand token) or an expression tier (so a call's result has a name). The sink half's soundness argument is already available — operands are structurally scalars, so duplication is not aliasing. |
| `PIPE-LET` | a pipeline on the right of a binding, `let r = x \|> f succeeds …;` |
| `PIPE-STMT-STAGES` | admitting `drop` / `move` / `defer` as pipeline stages |
| `PIPE-STAGE-ARITY` | a selector-arity table, needed only when a statement stage takes more than one operand |
| `OR-LOWERING` | the short-circuit lowering of `\|`. Blocked on a branch construct existing in the executable subset at all. |
| `AND-LOWERING` | the same for `&&` |
| `BITWISE-SYMBOLS` | the open question of §4.3 — bitwise or has no spelling, and bitwise and's `&` is only reserved |

---

## 9. Draft increment paragraph

> Pipe operator (PIPE-OPERATOR): `\|>` lands as a **pure parse-time token
> desugaring** — a piped statement is rewritten into the tokens of the
> statement a reader would have written without it, after lexing and before the
> delimiter map, so name resolution, custody, effects, linearity, concurrency,
> whole-program checking, reference execution and C emission all see an
> ordinary statement and **no checker rule was added**. That claim is measured,
> not asserted: a piped program and its hand-written twin are
> `syntax_equivalent`, emit **byte-identical C11**, and produce identical
> reference observations, on both the `std.io` and the extern-call twin of a
> shipped fixture. Linearity through a pipeline is exercised where it is real —
> `move b \|> qcore_extern_release;` — with the piped and unpiped **diagnostic
> vectors asserted equal** for use-after-consumption and for a leak, not merely
> both refusing. Attribution required the one machinery change: tokens are
> permuted keeping their own spans, only `(`/`)`/`,` are synthesized (anchored
> at the `\|>`), and `range_span` generalizes to min-start/max-end — byte-identical
> for every source-ordered range. `source_std_io.rs` is unchanged: the sinks
> needed no extension, and Q028's rule that a rendering operand cannot denote a
> resource is re-pinned through a pipe. Three measurements shaped the card and
> two contradict its brief: **there is no call tree** (`StatementKind::Expression`
> is payload-free; an expression statement refuses at `Q500`), so the AST
> rewrite the brief described became a stronger token-identity claim; **there is
> nothing to lower a short-circuit onto** (`if` itself refuses at `Q500`, the
> lowering IR has no branch constructor, and there are zero `if` statements in
> 294 source-side files), so `\|` (logical or) and `&&` (logical and) landed as
> **claimed tokens with actionable refusals at `Q034`** and named follow-ons
> `OR-LOWERING`/`AND-LOWERING`, under both amendments' scope guards; and a
> second, divergent tokenization path in `source_extern::statement_tokens` was
> re-lexing original bytes and bypassing the desugaring — found by a failing
> test and repaired to read the program's own tokens. Multi-stage pipelines
> (the user's `value \|> render \|> log` shape) are refused at `Q033` with both
> blockers named — `PIPE-CHAIN`. The operator table is now `[\|`/`\|]`
> unchanged, `\|>` landed, `\|` and `&&` claimed-and-refused, `\|\|` refused
> naming the single-`\|` spelling, and `&` **reserved with no new meaning**; the
> longest-match adjacency table is pinned by test, and the one deliberate pin
> update (a bare `\|` moves from lex error to grammar refusal) keeps its
> load-bearing negative half at the new layer. Bitwise or having no symbol is
> flagged open, not answered. `Q032`-`Q034` under a new `Pipe` pass through the
> §L21 process, with spec §L21 and grammars §G02h updated in the same change.
> 28 new tests, 35 audited Lean theorems, no new axiom, 77 → 78 audits.
> Corpus-neutral by construction: contract corpus stays 67 modules, CT stays
> 32/50, nothing promoted. Evidence:
> `docs/reports/wave18-pipe-operator/README.md`.
