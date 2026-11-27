# STRING-SLICE-IO — rendering text, and the three decisions behind it (wave 24)

Card brief: implement the named follow-on `STRING-SLICE-IO` of the revision-0.3
string-view amendment — *explicit composition of `std.string` with the output
modules, so a string value can be rendered to a stream.*

Amendment §7 is the charter, quoted verbatim:

> Output modules may render a string only after their own capability and
> observation rules are extended; this amendment does not make `std.io` or
> `std.logger` accept text.

This card is that extension. Base commit: main `cb2905f0` (the post-`DOCS-COMPACT-2`
checkpoint). Route: `source.string.v1`, card `STRING-SLICE`
(`docs/reports/string-slice/README.md`). Composed-route precedent:
`COMPOSE-EVENT-LINES` (`docs/reports/compose-event-lines/README.md`).

**What landed.** A `source.string.v1` entry may now write `print TEXT;` and
`log LEVEL TEXT;`, where `TEXT` is a quoted literal written at the operand, a
string this entry bound, or one slice of one written inline — and the `|>`
spellings `s |> print;`, `"…" |> print;` and `s[0:4] |> log info;` are the same
statements. Two new diagnostic codes, **`Q075`–`Q076`**, the `StandardString`
pass's second band. One new published example,
`examples/strings/status_line.qcore`, whose subject is an HTTP/1.1 response head.
**44** audited Lean declarations in a new `QCore.StringSliceIo`. **Zero** new
reserved words.

**The capability rule did not move, and that is the point.** `std.io` alone
still grants no string and `std.string` alone still opens no stream; what
changed is that a file naming **both** can render text. That is §7's own
sentence, and it is a registry fact rather than a promise
(`the_output_imports_alone_still_grant_no_string`).

**Corpus-neutral by construction** (§10). No file under
`crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/qcore/` or
`providers/` appears in this card's diff.

---

## 1. The three decisions the brief asked to be argued

### 1.1 The rendering rule: the observation is the view's own bytes

**Decision.** A rendered string's observation is the view's **exact UTF-8
payload** — no re-encoding, no escaping, no terminator, and no newline beyond the
one the sink's line discipline already writes for every line.

The framing is not this card's invention and deliberately so. It is read from
how `print` and `log` frame a line today, and it is now one function:

```rust
pub fn line_framing(level: Option<Level>) -> (Stream, String) {
    match level {
        None => (Stream::Out, String::new()),
        Some(level) => (Stream::Err, format!("[{}] ", level.tag())),
    }
}
```

`rendered_line` (a scalar) and `rendered_text_line` (a view) both call it, so a
rendered number and a rendered string cannot be framed differently. The
statement is pinned twice: as a Rust equation over every rendering, every level
and five values —
`qcore_core::std_io::a_rendered_scalar_line_is_a_text_line_of_its_digits` — and
in Lean as `the_two_line_kinds_share_one_framing`, which is `rfl` because there
is one framing rather than two implementations that agree.

What that buys, concretely: `log warn "ready";` writes `[WARN] ready\n` and
`log warn dec 5;` writes `[WARN] 5\n`, and no future change can move one without
moving the other.

**The empty string renders an empty line, not nothing.** `print "";` writes one
newline: a zero-length view is a value (amendment §4), nothing dereferences it,
and the framing's newline is the framing's
(`the_empty_view_writes_only_the_framing`).

**A body containing a newline byte writes more than one line's worth of bytes**,
and that is recorded rather than forbidden: the view's bytes are the view's
bytes. The framing appends **exactly one** newline whatever the body holds
(`the_framing_appends_exactly_one_newline`, with the two premises that no level
tag and no print prefix contains one).

### 1.2 The selector decision: a string takes none, and both directions are pinned

**Decision.** A bare operand renders a string's bytes; a `dec`/`hex` selector in
front of a string **refuses**, at `Q072`, where it refused before this card.

The argument is not stylistic. `dec` and `hex` are `std.fmt`'s two **total
functions from an `i32` to digits**; a `string` is already the bytes, so there is
nothing for either to do to one, and admitting `print dec s;` would have to
invent a meaning. `Q072`'s condition — "a string at a position it may not
cross" — is exactly right for a *selected* rendering operand and needed no
change; what the card did was widen what a **bare** operand means, at that
operand and nowhere else.

Both directions are pinned:

| Written | Result |
|---|---|
| `print dec s;`, `print hex s;`, `log info dec s;` for a string `s` | `Q072` (`a_rendering_selector_in_front_of_a_string_keeps_refusing_at_its_old_code`) |
| `print dec n;` for a scalar `n`, inside a module that also renders text | admitted, unchanged (`a_rendering_selector_still_renders_a_scalar_in_a_string_module`) |
| `print n;` for a scalar `n`, inside a string module | `Q076`, and the message names the repair: `print dec n;` |

The two statement families are separated by a **closed word set** rather than by
a lookahead: `dec` and `hex` are the whole rendering vocabulary, so a statement
naming one renders a scalar and a statement naming neither renders text. One
corner follows from that and is stated rather than left to be discovered: the
selector position is decided by the *word*, so a binding spelled `dec` could not
be rendered bare. It cannot arise — `dec` and `hex` have been reserved words
since `STD-MODULES` — but the rule is the word and not a type test, and that is
what makes it decidable by a reader.

**A consequence worth its own line: a rendered string needs no `std.fmt`.** A
text line names no rendering, so the import gate narrowed from the statement
*head* to the **selector word**. This is the measurement card `STD-MODULES` made
for `flush` ("`flush` names none, so a logger-only file that never renders needs
no `std.fmt`"), applied to the operand. The published example rests on it: it
imports `std.string` and `std.io` and nothing else. The other direction is pinned
too — a module rendering a scalar without `std.fmt` still refuses at `Q015`
(`a_rendered_scalar_still_needs_the_formatting_module`).

That narrowing moves one other refusal, named here rather than glossed: a
`print` naming **no rendering word at all** in a `source.std-io.v1` module used
to ask for `std.fmt` and now refuses at `Q030` naming the missing `dec`/`hex`.
Both are refusals; the second names the actual fault.

### 1.3 The NUL decision, and the obstacle that had to be solved rather than accepted

**Decision.** A byte equal to `0` is an **ordinary byte of the text**, inside
`len`, and is written.

This is forced rather than chosen. Amendment §3 makes the payload the decoded
bytes and `len` their exact count; `"ab\0cd"` has length 5 where `strlen` would
say 2, and `QCore.StdString.an_embedded_nul_is_inside_the_length` is a shipped
theorem about exactly that disagreement. A render that stopped at the `0` would
write a **different string** than the one the checker admitted and the reference
execution predicted.

**The obstacle was real, and the brief was right to anticipate it.** The emitted
line went through `QCORE_IO_WRITE`, which is `fprintf`, and *every* `printf`
conversion for a byte range stops at the first `0`: `%s` scans to a terminator
and `%.*s` takes a precision but still stops early. Rendering through either
would have silently truncated an admitted value.

**The repair.** A second prelude fragment, `io_emit::TEXT_PRELUDE`, defining one
macro that writes the payload with a **length-bounded `fwrite`**:

```c
#define QCORE_IO_WRITE_TEXT(stream, prefix, ptr, len) \
    do { \
        QCORE_IO_WRITE((stream), "%s", (prefix)); \
        if (fwrite((ptr), 1u, (size_t) (len), (stream)) != (size_t) (len)) { \
            abort(); \
        } \
        QCORE_IO_WRITE((stream), "\n"); \
    } while (0)
```

Four things about it are deliberate.

1. **It is a separate fragment from `IO_PRELUDE`, inserted only into a program
   that renders text.** Putting the macro in the shared prelude would have
   changed the emitted bytes of every program on every route. As it stands, a
   program that renders no text — including card `STRING-SLICE`'s own
   `slices.qcore` — is byte-identical to what it was before this card, which is
   pinned (`the_scalar_only_fixture_is_byte_identical`) and stated in Lean as
   `a_program_with_no_text_is_its_own_erasure`.
2. **The single-emission-route discipline survives.** `direct_stdio_calls` now
   exempts both preludes, by whole-line identity as before, and the claim it
   makes is unweakened and re-measured: an emitted program contains **exactly
   one** `fprintf(` spelling and **exactly one** `fwrite(` spelling, both in a
   prelude (`the_single_emission_route_survives_the_text_write`).
3. **The `fwrite` result is checked, not discarded.** A short write means the
   bytes the reference execution predicted did not reach the stream, and a run
   that quietly wrote a prefix of a line would falsify the byte comparison this
   route's whole evidence rests on. Unlike the slice helper's bounds
   comparison — a *witness* of a decision the checker already made — nothing in
   the checker rules a short write out, so this is a real check.
4. **`strlen` is still absent**, as amendment §3.7 requires, and so is every
   allocator and copier: `allocator_calls` is empty for every program this
   backend writes, text included.

The native run is the proof that the decision holds end to end: the fixture's
logger stream comes back as `[WARN] ab\x00cd\n`, compared as **bytes**
(`the_emitted_program_matches_the_reference_per_stream`).

---

## 2. The surface, and why the route needed no new selection

```text
stmt ::= …                                        (* unchanged *)
       | "print" TEXT ";"
       | "log" LEVEL TEXT ";"
TEXT ::= STRING | NAME | NAME "[" OPERAND ":" OPERAND "]"
```

`print "HTTP/1.1 200 OK";` · `print s;` · `log info s[0:4];` · and through the
pipe, `s |> print;` · `"…" |> print;` · `s[0:4] |> log info;`.

**An inline slice is the same checked slice a `let` binds.** Both go through one
predicate, `std_string::admit_slice`, which this card factored out of the `let`
arm rather than reimplementing — so a slice cannot be admitted at a rendering
operand where the same slice written as a binding would be refused, or refused
where it would be admitted. Stated in Lean as
`an_inline_slice_is_admitted_exactly_when_the_bound_one_is`, and measured in
Rust: `print s[0:1];` on `"\xc3\xa9o!"` refuses at `Q071` and `print s[0:9];`
on a two-byte string refuses at `Q071`, both the base route's own code. The
inline slice's arithmetic goes into the certificate's own `SliceRecord` list
under a synthesized name, so the §4 zero-copy evidence covers a rendering
operand as completely as it covers a binding
(`a_rendered_view_is_published_in_the_certificates_provenance_list`).

**Route selection is untouched, and `driver.rs` was not edited at all.** The
string route was already selected before `source.std-io.v1` (card
`STRING-SLICE` §7), so a module that constructs a string and prints already
reached it. What this card changed is the *detector* — in `source_std_string.rs`,
which owns it — and it claims only the **constructing** forms:

* a quoted literal at an operand, or an inline slice at one → this route;
* a **bare name** at an operand → **not** a selector.

The bare form is deliberately excluded from selection, and the reason is
measured rather than aesthetic: `print bogus;` is what a `source.std-io.v1`
module writes when it misspells a rendering, and claiming it here would pull a
module that never mentions a string onto this route and answer a question it
never asked. That module keeps refusing at `Q030` naming the missing `dec`/`hex`
(`a_bare_name_at_an_operand_does_not_pull_a_standard_io_module_onto_this_route`).
A bare name is admitted as text only inside a module this rule already selected —
"a text render is a capability of a string module, not a way to become one".

**A text render outside the entry is refused.** The route models the entry; the
helper scan covers the new forms, so a `print "…"` in a helper is refused at
`Q068` rather than being neither emitted nor refused. That is the one
fail-closed hole the composition could have opened, and it is the same one
`COMPOSE-EVENT-LINES` closed for its own route.

---

## 3. The diagnostic band: `Q075`–`Q076` allocated, `Q077`–`Q079` unused

The integrator pre-assigned `Q075`–`Q079`. This card uses **two**, contiguously
from `Q075`, and returns `Q077`–`Q079` to the pool.

| Code | Condition |
|---|---|
| `Q075` | The **shape** of a rendered-text line: a missing operand, trailing tokens, a malformed inline slice, an integer at a text operand, or a `borrow NAME` marker |
| `Q076` | A text operand **naming something this entry bound no string for** |

`Q076` is deliberately this band's own rather than a reuse of `Q069`. The two
read the same from a distance and are not the same: `Q069` is a *view position
inside the string vocabulary* — a slice's source, a `len`'s argument — whereas
this is the **output** vocabulary's operand, where a reader who wrote `print x;`
for an integer `x` wants to be told to name a rendering rather than to be told
that `x` is not a string. The message says exactly that, and a test pins that it
contains `print dec n;`.

**Everything else a rendered string can be refused for is an existing refusal at
its existing code**, which is the honest measure of how much this composition
actually added:

| Shape | Code | Decided by |
|---|---|---|
| a literal that does not decode, written at an operand | `Q070` | the base route's decoder, wherever the token is |
| an inline slice out of range or off a code-point boundary | `Q071` | `admit_slice`, the same one a `let` uses |
| a selector applied to a string | `Q072` | unchanged (§1.2) |
| an undeclared derived effect | `Q073` | unchanged |
| `flush s;` | `Q068` | `source.std-io.v1`'s own "takes no operand", read by its own reader |
| an operand naming nothing at all | `Q013` | name resolution, generically |
| an unbalanced bracket (`print s[0:1;`) | `Q000` | the parser, before any route |
| a text render outside the entry | `Q068` | this route's helper scan |

**`Q013` for an unbound operand** is the standing `STD-FMT-DIAGNOSTIC` residual
that `STD-MODULES` recorded and `COMPOSE-EVENT-LINES` re-recorded: the refusal is
correct and only its specificity is the gap. Unchanged here, and pinned.

The §L21 row is at `docs/QCORE_SPEC_r0_3.md`; `SemanticPass::StandardString`'s
doc comment records the band at its definition site. The `AGENTS.md`
namespace-table row is integrator business and was deliberately not written here
(§9).

---

## 4. Keyword census (standing constraint 14): **zero new reserved words**

The composition is spelled entirely in words the resolver already reserves:
`print`, `log` and the six level words, all added by `STD-MODULES` and the
quiet-emission amendment; `borrow`, reserved since the checked mutex-guard
profile; and the literal's own delimiters and `[`, `]`, `:`, `-`, all already
lexed. The operand of a text line is a quoted token, a name, or a slice — none
of which is a new word.

That is what the card scope asked for and what amendment §2 asks for, and it is
one of the few compositions in this repository that needed **no** census
repair: `STD-BUFFER` needed two words, `STD-MODULES` eight plus `log_level`,
`T08-EPOCH-SRC` eight. The whole workspace gate is the measurement rather than
the argument.

---

## 5. The one shared-ground edit, and the seam for the integrator

**`crates/qcore-frontend/src/pipe.rs` — the closed sink-stage table.** Card
`PIPE-DROP-STAGE` owns that table. This card changes each entry's selector
**count** into an inclusive **range**:

| Head | Before | After |
|---|---|---|
| `print` | 1 | 0–1 |
| `log` | 2 | 1–2 |
| `drop` | 0 | 0–0 (unchanged) |

and `emit_stage` now emits the selector words the reader actually wrote rather
than a table constant. That is the whole change: 4 lines of table, one arithmetic
comparison, one message arm.

**What it costs, measured in both directions.** The pipe decides nothing about
which arity a route admits — it emits the head, the selectors and the operand,
and the route's own reader refuses a shape it does not model at its own code and
span. So:

* **Every pipeline written before this card desugars to the same tokens**,
  because every one of them wrote the maximum. Measured rather than assumed: a
  census of every complete `|> print …;` and `|> log …;` statement in every
  `.qcore`, `.qc` and `.rs` file on the base found **126 occurrences in 12
  distinct shapes**, and every one of them names the maximum selector count or
  more. The only two that are not exactly at the maximum are refusal fixtures —
  `total |> print nope;` (one selector, at the maximum, refused at `Q013`
  because `nope` is an unbound name) and `total |> print dec hex;` (two, above
  it, refused at `Q036`) — and both keep the code they had. **No `|> print;` and
  no `|> log LEVEL;` occurs on the base at all**; every one in the tree after
  this card is this card's own. Archived with its reproduction command at
  `artifacts/pipe_census.txt`.
* **One refusal moves**, from the pipe to the route: `x |> print;` on
  `source.std-io.v1` refused at `Q036` naming the pipe and now refuses at `Q030`
  naming the missing `dec`/`hex`. Refusal to refusal, and the second names the
  fault. Pinned by
  `a_scalar_piped_into_a_selectorless_print_refuses_at_the_routes_own_code`.
* **No acceptance is created.** A count outside every arity still refuses at
  `Q036` (`s |> print dec hex;`, `s |> log;`), and the four existing
  `pipe_operator.rs` tests that rest on the arity rule — the wrong-selector-count
  row, the missing-semicolon row, the empty-operand row and the stage-attribution
  row — all pass unchanged, because each writes a count inside or above the new
  range exactly as it did before.

**One archived claim is superseded, and is recorded rather than edited.**
`docs/reports/wave18-pipe-operator/README.md` §"the shapes" carries the row
`x |> print` (no rendering) | **refused, `Q036`** | wrong selector arity. That is
true of its own base and false after this card: the shape now desugars and the
route decides it. The archive is **not edited** (standing constraint 21); the
provenance is published here beside it, which is what that constraint asks for.

**`crates/qcore-verification/src/driver.rs` is NOT edited by this card.** The
brief flagged the route-selection area as shared ground with the parallel socket
card; this card needed nothing there, because `STRING-SLICE` already placed the
string route before `source.std-io.v1` and the detector that decides what
reaches it lives in `source_std_string.rs`. **The seam is clean: zero lines, zero
conflict surface.**

Two other files owned elsewhere were **extended rather than rewritten**:

* `crates/qcore-core/src/std_io.rs` gains two functions (`line_framing`,
  `rendered_text_line`) and changes none. `StdIoOperand` and `StdIoEvent` are
  **untouched**, deliberately: adding a text constructor to the shared operand
  enum would have reached every route that consumes it, and each would have had
  to grow a refusal arm. The text line is this route's own event, and the shared
  file holds only the byte decision, which is where the byte decision belongs.
* `crates/qcore-frontend/src/source_std_io.rs`'s `rendering_span` narrows from
  the statement head to the selector word (§1.2). That is the `std.fmt` gate's
  own reader, refined with the same grammar the lowering reads.

---

## 6. What did not move

* **The base route's answers.** A text line *erases* to the base route's own
  line, and the composed effect row is the row of the erasure
  (`erasing_the_text_lines_leaves_the_effect_row`) — so every theorem
  `QCore.StdString` proves about a program is a theorem about the same program
  with rendered text in it. This is `COMPOSE-EVENT-LINES`'s shape, reused for
  the same reason.
* **Effects.** A text line derives `Io` and nothing else: no `Alloc`, no `Mut`,
  for any string operation (amendment §7). Three Lean theorems and one
  end-to-end pair — an undeclared row refuses at `Q073`, a declared one
  compiles.
* **Custody.** Strings are custody-free views, and this card publishes no
  identity for one. `borrow NAME` at a text operand refuses at `Q075` **with the
  reason**: a view owns nothing, publishes no identity, and creating one neither
  creates nor discharges custody, so there is no identity for `borrow` to
  render. The card scope allowed arguing otherwise from measurement; there was
  nothing to measure, because the model has no identity to publish. `Q072`'s
  own `borrow` arm (a `borrow` at a *selected* rendering operand) is unmoved.
* **Every program that renders no text**, byte for byte: no text prelude, no
  `fwrite`, the same observation `STRING-SLICE` published
  (`the_scalar_only_fixture_is_byte_identical`).
* **Cross-file isolation.** Importing `std.string` in another compilation input
  grants this file nothing (`the_imports_do_not_cross_compilation_inputs`), and
  each output module is still gated separately: a `print` without `std.io`
  refuses naming `std.io`, a `log` without `std.logger` naming `std.logger`, and
  a rendered literal without `std.string` naming `std.string`.
* **The corpus.** No gateway, contract, audit or provider file is in the diff.

---

## 7. The HTTP dependency this card exists to unblock

The card scope: *"a fixed HTTP/1.1 response line like `HTTP/1.1 200 OK` must
become spellable as a rendered string"*, with at least one example proving the
payload character set survives the whole pipeline.

`examples/strings/status_line.qcore` renders a whole response head — status
line, two header lines, the empty line that ends a head, and a body with a slice
of it — through `print` and `|> print`. Its character set is letters, digits,
spaces, `/`, `.`, `:`, `;`, `-` and `=`, and it survives lexing, decoding, the
certificate, the emitted static byte array and a sanitized native run. The test
compiles it, builds the emitted C under `-Wall -Wextra -Werror -pedantic`, runs
it under ASan/UBSan/LSan, compares both streams with the reference execution,
and then compares stdout with the transcript **read out of the README**, so the
page cannot drift from the program.

**What it is not**, stated in the example's own README and here: not an HTTP
implementation. No socket, no request parsing, no header model, and no agreement
between the `Content-Length` line and the body — the two are independent
literals. The honest statement of what this card unblocks is narrower than
"HTTP": a program can render text it **wrote**, and not text it computed. Every
payload v1 can render is a literal or a slice of one, so a header whose value is
a number is still out of reach (`STRING-SLICE-CONCAT`, §11) — which is the next
thing `HTTP-QUIRIN-V1` will run into and is better known now than discovered
later.

---

## 8. Evidence

| Artefact | Where |
|---|---|
| Checked model | `crates/qcore-core/src/std_string.rs` (`TextOperand`, `StringEvent::Text`, `admit_slice`, `text_view`, `Q076`) |
| The line-byte decision | `crates/qcore-core/src/std_io.rs` (`line_framing`, `rendered_text_line`) |
| The emission route | `crates/qcore-core/src/io_emit.rs` (`TEXT_PRELUDE`, `write_text_bytes`, the widened exemption) |
| C11 emission | `crates/qcore-core/src/std_string_c_emit.rs` (the `Text` arm, the conditional prelude) |
| Source lowering | `crates/qcore-frontend/src/source_std_string.rs` (`text_line`, `StringUse::Render`, `Q075`) |
| The `std.fmt` gate | `crates/qcore-frontend/src/source_std_io.rs` (`rendering_span`) |
| The pipe table | `crates/qcore-frontend/src/pipe.rs` (`SINK_STAGES` selector ranges) |
| Model unit tests | `crates/qcore-core/tests/unit/std_io.rs` (3 → **5**) |
| End-to-end tests | `compiler/tests/string_slice_io.rs` (**37**, two of them sanitized native runs) |
| Fixture | `compiler/tests/fixtures/std_string/text.qcore` |
| Example | `examples/strings/status_line.qcore`, `examples/strings/README.md`, indexed in `examples/README.md` |
| Lean | `proof/QCore/StringSliceIo.lean`, `proof/AuditStringSliceIo.lean` (**44** declarations: 27 theorems, 12 definitions, 5 inductive types; 31 audited names) |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 row `Q075`–`Q076`, and the §G07.9 requirement block |
| Grammar | `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G07.9 and §G02h |
| Amendment | `docs/design/STRING_SLICE_V1.md` §7 (the four observation rules) and §9 |
| Artifacts | `artifacts/pipe_census.txt` (§5's measurement, with its reproduction command), `artifacts/text.c` (the emitted C11 for the accepted fixture) |

### 8.1 What the Lean proves, and what it does not

`QCore.StringSliceIo`, all placeholder-free with no project axiom — every closure
is a subset of `{propext, Classical.choice, Quot.sound}`.

**Neutrality (the erasure):** a text line erases to the base route's line, the
composed effect row equals the erased row, and a program with no text in it *is*
its own erasure. **The rendering rule:** the body is the view's own bytes, a
`print` line is unprefixed on standard output, a `log` line carries its
bracketed tag, and the framing appends exactly one newline. **One framing:**
`the_two_line_kinds_share_one_framing`, by `rfl`. **The NUL decision as a
disagreement:** on `[97, 98, 0, 99, 100]` a terminator scan writes two bytes and
this model writes five. **One decision for a slice**, wherever it is written, and
fail-closed on a name with no view. **The operand wall:** three forms, stated as
one list so a widening changes a theorem rather than slipping past it, and none
of them carries a custody identity. **The module gate:** both imports required,
neither granting the other, and a text line needing no formatting module while a
scalar line still does.

What it does **not** prove: that the Rust lowering extracts these events
faithfully from source tokens, that `fwrite` writes what C says it writes, or
anything about a view of storage that can end — v1 has none.

---

## 9. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Four things are the integrator's.

1. **Two new diagnostic codes.** `AGENTS.md`'s diagnostics-namespace table needs
   the band added to the `StandardString` row (authority is spec §L21, which
   this card updated):

   > `Q068`–`Q073`, `Q075`–`Q076` | `StandardString` | `source.string.v1` in two
   > bands: the immutable UTF-8 `{ptr, len}` string view (`Q068`–`Q073`); its
   > composition with the output modules — `print TEXT;` and `log LEVEL TEXT;`
   > over a literal, a bound string or one inline slice — where `Q075` is the
   > text line's shape and `Q076` a text operand naming no string this entry
   > bound (`STRING-SLICE-IO`)

   `Q077`–`Q079` of the pre-assigned range are **unused and returned to the
   pool**.

2. **A proposed standing constraint**, offered because it is the kind of rule the
   "when touching X, do not Y" inventory exists for:

   > **15e. A rendered string's observation is the view's exact bytes, and the
   > emitted write is length-bounded.** `source.string.v1`'s text lines write no
   > escaping, no terminator and no newline beyond the framing every line
   > already carries, because a payload's `0` is an ordinary byte inside `len`
   > and a scan-to-terminator write and the model **disagree** on a value the
   > route admits. Do not render a string through any `printf` conversion: `%s`
   > and `%.*s` both stop at the first `0`. The write lives in `io_emit`'s second
   > prelude fragment, inserted only into a program that renders text — which is
   > what keeps every other emitted program byte-identical — and exempted from
   > the single-emission-route scan by whole-line identity exactly as the first
   > is; its `fwrite` result is **checked**, because nothing in the checker rules
   > a short write out. The stream, the `[LEVEL] ` prefix and the single newline
   > are `std_io::line_framing` for **both** line kinds, so a rendered scalar
   > line is the text line of its own digits (`rfl` in Lean); do not give either
   > kind a second framing. A string takes **no** rendering selector and one
   > written in front of a string refuses at `Q072`. `string-slice`,
   > `string-slice-io`.

3. **The derived `lean_audits` count moves 99 → 100** for this card's one new
   `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the stale
   matrix count until the derived section is regenerated. The same one-line
   reconciliation every wave's new audit file receives (constraint 25).

4. **Public API changes in `qcore-core`**, all additive:
   `std_io::line_framing` and `std_io::rendered_text_line` are new;
   `io_emit::TEXT_PRELUDE` and `io_emit::write_text_bytes` are new;
   `std_string::TextOperand` and `std_string::text_slice_name` are new;
   `StringEvent` gains `Text` and `StringError` gains `NotRenderableText`. Both
   enums are non-exhaustive in practice, so a downstream `match` on either needs
   a new arm. In `qcore-frontend`, `StringUse` gains `Render`.

---

## 10. Corpus neutrality

By construction. The diff touches four `qcore-core` files, four
`qcore-frontend` files, three documents, two Lean files, one test file, one
fixture and one example directory. `contract/`, `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `crates/qcore-verification/`, `providers/`,
`examples/threads` and every frozen archive appear nowhere in it. The pinned
185-unit artifacts, the leak count of **266**, the 73-module contract pin and the
CT manifest are main's, untouched and unread.

---

## 11. Named follow-ons

| Name | What is missing |
|---|---|
| `STRING-SLICE-CONCAT` | A string a program **computes** rather than writes. Every payload v1 renders is a literal or a slice of one, which is enough for a fixed protocol head and not enough for a header whose value is a number. It needs a building operation, which needs storage a program owns — `STRING-SLICE-BUFFER-BORROW`'s question first. **This is `HTTP-QUIRIN-V1`'s next obstacle** and is named now rather than discovered later. |
| `STRING-SLICE-IO-EXAMPLE-SUITE` | `examples/strings/` is verified by this card's own test file rather than by `compiler/tests/examples_suite.rs`, which is the apparatus every other published example family uses (route assertion, README transcript extraction, refused-program extraction). Folding it in is mechanical and was kept out of this card's blast radius deliberately. |
| `STRING-SLICE-IO-WINDOWS-RUN` | This card ran the **Linux** arm only. The emitted text write uses `fwrite` and a `do { … } while (0)` macro, both C11 and both inside the existing prelude structure, and the test file carries the Windows `windows_c` path unchanged — but no Windows or Apple run was performed and none is claimed. The `EVENT-V1-WINDOWS-RUN`/`EVENT-V1-APPLE-RUN` precedent. |
| `STRING-SLICE-IO-NEWLINE-BODY` | A payload containing a newline byte renders as more than one line's worth of bytes. Recorded and proved (`the_framing_appends_exactly_one_newline`) rather than forbidden, because the view's bytes are the view's bytes; whether a sink should refuse or escape such a body is a decision with no data behind it yet. |
| `STD-FMT-DIAGNOSTIC` | *(unchanged, another card's)* an operand naming nothing refuses generically at `Q013`. |
| `STRING-SLICE-BUFFER-BORROW`, `STRING-SLICE-ABI`, `STRING-SLICE-VARYING-INDEX`, `STRING-SLICE-NOTASCALAR-REACH`, `BYTE-SLICE-CAP` | *(unchanged, card `STRING-SLICE`'s)* |
| `STRING-SLICE-IO` | **retired by this card.** |

---

## 12. Gate

Run in this worktree on main `cb2905f0` plus this card, with
`$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree — see §12.1 |
| `cargo test --workspace --no-fail-fast` | **3,815 passed, 0 failed, 8 ignored** across 203 result lines, exit **0** |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **Passed 100 Lean audits** (99 → 100: `AuditStringSliceIo.lean`), no project axiom, exit **0** |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — see §12.3 |

**The count is 3,815 against main's 3,776, and the difference is exactly this
card's.** 37 new end-to-end tests in `compiler/tests/string_slice_io.rs` plus 2
new model tests in `crates/qcore-core/tests/unit/std_io.rs` is 39, and
3,776 + 39 = 3,815. No existing test was deleted, rewritten or weakened.

**No `P02-FLAKE` row tripped** (standing constraint 16). Three of the five rows
are in this workspace suite and each passed on the first attempt of the
complete run: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths` and
`memory_growth_beyond_the_declared_limit_is_bounded`. The other two — the
`RLIMIT_NPROC` conformance row and
`real_packages_preserve_unknown_and_reject_mutations` — live in the separate
`providers/` workspace, which this card neither touches nor ran: no provider,
schema, package template or protocol changed, so constraint 18's rebuild is not
owed and the 122-test provider checkpoint is **carried, not rerun**. Nothing
here rests on a re-run and no assertion was weakened. Worth recording because
the machine carried sibling agents throughout, with a one-minute load average
between 11 and 24.

The Lean toolchain lives at `$HOME/.elan/bin` and is **not** on this worktree's
default `PATH`; `scripts/check_lean_audits.py` reports a missing `lake` **and
exits 0** in that state, so the run above was made with that directory exported
and the "Passed 100 Lean audits" text verified rather than the exit code alone.
That is the environment note card `STRING-SLICE` recorded, and it recurred.

### 12.1 The first run stopped at the submodule pin

`libevent_submodule_matches_the_audited_revision` failed on the first
`cargo test --workspace` of this fresh worktree. That is invariant 5's pin doing
its job on an **environment gap** rather than a code failure — the same thing
cards `STD-EVENT` and `COMPOSE-EVENT-LINES` recorded in their own fresh
worktrees — and the gate's own first line is the repair. The submodule was
checked out and the whole-workspace run redone from scratch; the numbers above
are that second, complete run.

### 12.3 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`lean_audits: 99 != 100`. That is the derived-value tripwire working as
designed: this card adds one audited Lean module, and
`docs/reviews/r0_3/acceptance_matrix.json` — which this card may not edit —
carries the pre-card count. §9 item 3. The other 192 Python tests pass.

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

---

## 13. Draft increment paragraph

> Rendering text, and the three decisions behind it (STRING-SLICE-IO): a
> `source.string.v1` entry may now write `print TEXT;` and `log LEVEL TEXT;`,
> where `TEXT` is a quoted literal written at the operand, a string the entry
> bound, or one slice of one written inline — with `s |> print;`,
> `"…" |> print;` and `s[0:4] |> log info;` the same statements — which
> implements amendment §7's own follow-on and leaves that section's capability
> rule exactly where it was: `std.io` alone still grants no string and
> `std.string` alone still opens no stream, and what changed is that a file
> naming **both** can render text. **The rendering rule is the identity on
> bytes**: the observation is the view's exact UTF-8 payload, with no
> re-encoding, no escaping, no terminator and no newline beyond the one the
> sink's framing already writes — a framing now read from one function for both
> line kinds, so a rendered scalar line **is** the text line of its own digits
> (`rfl` in Lean). **A string takes no rendering selector**, because `dec` and
> `hex` are total functions from an `i32` to digits and a string is already the
> bytes; one written in front of a string keeps refusing at `Q072` where it
> refused before, and the `std.fmt` import gate narrowed from the statement head
> to the selector word, so a module that renders only text imports two modules
> and not three. **An embedded NUL is rendered as a byte**, which was not free:
> every `printf` conversion for a byte range stops at the first `0`, so the
> emitted line writes the payload with a length-bounded `fwrite` from a **second
> prelude fragment** inserted only into a program that renders text — which is
> what keeps every other emitted program byte-identical — with the write's
> result checked rather than discarded, and the single-emission-route claim
> re-measured as exactly one `fprintf` and exactly one `fwrite` spelling, both
> in a prelude. An inline slice is decided by the **same predicate** a bound one
> is and records its arithmetic in the same certificate list, so no slice is
> admitted at a rendering operand that would be refused as a binding. Two new
> codes, **`Q075`–`Q076`**, the `StandardString` pass's second band, for the
> composition's own two conditions; every other refusal a rendered string can
> receive is an existing one at its existing code. **No reserved word was
> added**, and **`driver.rs` was not edited at all** — route selection already
> placed this route before `source.std-io.v1`, and the detector claims only the
> *constructing* forms, so `print bogus;` in a `source.std-io.v1` module keeps
> that route's own refusal. One shared-ground edit: the pipe's closed sink table
> carries selector **ranges** rather than counts, which moves one refusal from
> the pipe to the route (`x |> print;` now refuses at `Q030` naming the missing
> `dec`/`hex`) and creates no acceptance — a 123-occurrence census of every
> piped sink statement on the base found every one of them at the maximum. One
> new published example, `examples/strings/status_line.qcore`, an HTTP/1.1
> response head, sanitized-native with its README transcript compared. **44**
> audited Lean declarations in a new `QCore.StringSliceIo`, no new axiom.
> Corpus-neutral by construction. Evidence:
> `docs/reports/string-slice-io/README.md`.
