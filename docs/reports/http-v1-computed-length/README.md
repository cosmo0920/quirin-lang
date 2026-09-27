# HTTP-V1-COMPUTED-LENGTH — a `Content-Length` the program works out, decided exactly as hard (wave 26)

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

**Card:** `HTTP-V1-COMPUTED-LENGTH`. **Route:** `source.http.v1`, behind the
`std.http` bundled module. **Codes:** none allocated — every refusal is a
condition extension of the existing `Q090`–`Q096` band. **Reserved words:**
none. **Base:** `origin/main` at `e26bb0c3` (the wave-25 close).

The seam both sides recorded, closed. `HTTP-QUIRIN-V1` §11 named
`STRING-SLICE-CONCAT` as what a computed length needed; `STRING-CONCAT` §7
landed computed text, published `examples/strings/computed_length.qcore` — an
HTTP-shaped head with a computed length rendered **outside** the HTTP route —
and named this card, saying exactly what it needed:

> a `content` clause that takes a *bound view* rather than a quoted literal,
> plus a `Content-Length` value that is a composed string rather than a literal
> — both inside a route this card does not own.

Both are here, and one published example now writes that head **through** the
checked route.

**Status language.** This card publishes no verdict about any pinned corpus unit
and moves no corpus number. Corpus-neutral by construction (§9).

---

## 1. The headline: the widening is at the *positions*, not at the gates

A header value and a content may now be a value the entry computed. Nothing else
moved. In particular:

> **`admit_field` and `admit_framing` are unedited.** A component value is
> resolved to bytes *before* either runs, so the `CR`/`LF` exclusion and the
> `Content-Length` equality are the same two functions `HTTP-QUIRIN-V1` shipped,
> deciding the same questions over the same bytes. Neither ever learns which
> spelling produced them.

That is the whole soundness story, and it is why the card is small. The reason it
works is `STRING-CONCAT`'s measured design decision (standing constraint 15h):
a `concat` result is **static storage the compiler computed**, its length the sum
of its pieces' lengths and itself a check-time constant. Had the sibling card
materialised compositions into owned buffers — the alternative it measured and
rejected — the length would have been a run-time quantity and this card would
have been impossible, not merely harder. The dependency `HTTP-QUIRIN-V1` recorded
was real; the property that discharges it is the one `STRING-CONCAT` went out of
its way to preserve.

### 1.1 What a program can now write

```qcore
let count = 3;
let body = concat("{\"items\":", dec(count), "}\n");
let length = len(body);

let rs = response "HTTP/1.1" 200 "OK";
header rs "Content-Type" "application/json";
header rs "Content-Length" dec(length);
content rs body;
```

Nothing is spelled twice. Change `count` and the body, the header and the
transcript move together, because all three are derived from it. And
`header rs "Content-Length" dec(count);` — the same program with one word
changed — is **refused at `Q093`**, because 3 is not 12.

---

## 2. The grammar, and what it adds

```ebnf
stmt   = … (* every HTTP-QUIRIN-V1 statement, with two positions widened *)
       | "header", NAME, STRING-TOKEN, VALUE, ";"
       | "content", NAME, VALUE, ";"
       | "let", NAME, "=", "concat", "(", VALUE, { ",", VALUE }, ")", ";"
       | "let", NAME, "=", "len", "(", NAME, ")", ";" ;
VALUE  = STRING-TOKEN                        (* this module's own syntax *)
       | NAME                                (* text this entry bound *)
       | ( "dec" | "hex" ), "(", NAME, ")" ; (* a bound integer's text *)
```

`VALUE`'s three forms are the **same three** a `source.string.v1` `concat` piece
takes, minus that route's inline slice — slicing beside HTTP stays
`HTTP-V1-STRING-COMPOSE` — so a spelling this route admits is one that route
admits too, and the containment is in the stated direction.

**The field *name* stays a written token.** A `token` is this module's own
syntax and a computed field name is a different question; it is named
`HTTP-V1-COMPUTED-FIELD-NAME` (§10) and
`the_content_length_field_name_is_still_a_written_token` pins it.

### 2.1 Three decisions worth stating

**Integer bindings are the transport's, not a second reader's.** `let n = 200;`
already lowers to `SocketStatement::Let` through `source.socket.v1`'s own reader,
which is how `exchange.qcore` binds the status it prints. This route *observes*
that value as it walks; it does not read integers itself. A second scalar reader
here could disagree with the one that already exists.

**`len` binds a check-time scalar and lowers into the transport only where the
transport reads it.** A length used solely as a header value produces no C object
at all; one a `print dec length;` renders becomes the transport's own `int32_t`,
so the two spellings read the same object. Without that rule the emitted program
would carry an unread `int32_t` and fail `-Werror=unused-variable` — a C compile
failure rather than a QCore refusal, which is exactly the shape of the three
hygiene findings `STRING-CONCAT` §8 recorded.
`a_length_binds_a_c_object_exactly_when_the_emitted_program_renders_it` measures
both directions.

**A composition of one is refused**, at `Q090`. A composition of one *is* that
piece, and admitting it would give the route two spellings of one value with
nothing to tell them apart by — `source.string.v1`'s own rule, inherited rather
than re-decided. It costs nothing here because the digits-only case is
`dec(NAME)` written directly at the position.

---

## 3. Zero new diagnostic codes, and where each refusal landed

`AGENTS.md`'s Q-space note (`Q-NAMESPACE-DECISION`) puts a new allocation in the
ice box: *no card allocates new `Q` codes*, and a new refusal lands as a
**condition extension of an existing band** on the `SOCKET-EVENT-COMPOSE` /
`NATIVE-LIBRARY-HEADER-DECORATION` precedent. This card allocates none. The
widening is stated in spec §L21 and here rather than taken silently.

| Refusal | Code | Why that code |
|---|---|---|
| a malformed `concat(…)`, `len(…)` or rendering piece | `Q090` | the band's statement-shape condition, unchanged in meaning |
| a composition of fewer than two pieces | `Q090` | a bound on the construct's shape |
| a composed payload above 4096 bytes | `Q090` | `source.string.v1`'s own payload bound, inherited |
| a rendering selector with no `(NAME)` | `Q090` | shape; the message names the repair |
| an unbalanced parenthesis | `Q000` | the parser, before any route — the split `STRING-CONCAT` §4 records |
| **a composed component carrying `CR` or `LF`** | `Q092` | `admit_field`, **unedited**; the wall is about the head, not the escape |
| **a computed `Content-Length` ≠ the content's byte count** | `Q093` | `admit_framing`, **unedited** |
| a `hex` or negative rendering at a `Content-Length` | `Q093` | not `1*DIGIT`; the existing condition, reached by a new spelling |
| a name at a value, piece or `len` position bound to the other kind | `Q094` | the band's "a name at a position that is not one", **widened** |
| a name bound to **nothing at all** | `Q013` | name resolution first — the standing `STD-FMT-DIAGNOSTIC` residual, unchanged |
| a value name bound twice | `Q012` | name resolution first; the model's guard is kept but unreachable (§10, `HTTP-COMPUTED-REBIND-REACH`) |
| a composed binding with no `#import std.string;` | `Q015` | the existing string gate, which already fires for `concat`/`len` |

### 3.1 The import rule, decided by the rule that already existed

Constraint 15f states it in two halves, and both apply here **without a line of
gate code**:

* a **`let`-bound** composition constructs a `string` value a later statement
  names, measures and can transfer, so it takes `std.string`'s `Q015` gate —
  which `source_std_string::string_span` already reports for `concat` and `len`,
  because those are that route's own constructs
  (`a_composed_binding_needs_the_string_module`);
* a **`dec(NAME)` written at a message component** constructs no `string` value
  — nothing in the language can bind it, slice it, measure it or render it — so
  it is this module's syntax on the same terms as a quoted token there, and needs
  neither `std.string` nor `std.fmt`
  (`a_rendered_component_is_this_modules_syntax_and_needs_no_text_module`).

The second half is **inherited, not decided here**: `STRING-CONCAT`'s own
published example `examples/strings/computed_length.qcore` writes
`concat("Content-Length: ", dec(length), "\r")` and imports no `std.fmt`, because
`source_std_io::rendering_span` reads the selector position of a `print`/`log`
head and nothing else. Deciding it differently here would have made one word mean
two things in two routes.

---

## 4. Keyword census (standing constraint 14): **zero new reserved words**

Ran over **427** `.qcore`/`.qc` files and **582** tracked `.rs` files.
Reproduction: `artifacts/keyword_census.sh`, output `artifacts/keyword_census.txt`.

| Word | Status before this card | What this card does |
|---|---|---|
| `concat` | **contextual** clause word of `source.string.v1` (`STRING-CONCAT`), reserved nowhere | reads it at a new position, through that route's own `is_string_clause_word` |
| `len` | **contextual** clause word (`STRING-SLICE`), reserved nowhere — the census found `len` as a *reference* in the pinned contract corpus | reads it through that route's own `is_length_query` |
| `dec`, `hex` | globally reserved since `STD-MODULES` | reads them at a new position |
| `request`, `response`, `header`, `content` | this route's four, since `HTTP-QUIRIN-V1` | unchanged |

The census measures the two facts that matter: `is_reserved_source_word` has
**111** entries before and **111** after (`crates/qcore-frontend/src/frontend.rs`
is not in this card's diff at all), and `HTTP_KEYWORDS` is still `[&str; 4]`.

**The contextual rule is one predicate, consulted once.** The census's third
section lists every reader of `is_string_clause_word`: `frontend.rs`'s reference
collector, `source_std_string.rs` itself, and now `source_http.rs`. A second copy
of the rule here would let the two routes drift about what `concat(` means; there
is none.

Four candidate spellings were **not taken** and are recorded rather than merely
avoided: `length` (11 source occurrences), `size` (96), `body` (34 — already
rejected by `HTTP-QUIRIN-V1`'s census and still a binding name in shipped
fixtures), `value` (271). `render` is clean at zero and was not needed, because
`dec`/`hex` already spell what it would have meant.

---

## 5. Tests: `compiler/tests/http_computed_length.rs`, **33 passing**

Exact names, in the file's own order.

**The payoff**

* `a_computed_content_length_is_admitted_and_declares_the_content_byte_count`
* `the_length_of_a_composition_is_the_sum_of_its_pieces` — `piece_lens` is
  `[9, 1, 2]`, `bytes.len()` is their sum, and the certificate's scalar
  environment carries `length = 12`
* `a_computed_content_length_that_is_not_the_content_length_refuses` — two pairs
* `the_certificate_says_which_components_were_computed` — both polarities of the
  new flags, **and** that the computed and written spellings produce
  byte-identical wire bytes

**The injection wall, both polarities** (the card's explicit requirement)

* `a_composed_field_value_carrying_a_framing_byte_refuses` — three shapes, one of
  them `concat("text/plain", "\r\nX-Injected: 1")`, all `Q092`
* `a_composed_content_may_carry_a_framing_byte_where_a_field_value_may_not` — the
  contrast that shows `Q092` is about the head and not about the escape
* `a_composed_head_carries_exactly_the_framing_crlfs_the_compiler_wrote` — the
  **positive** polarity: a clean composition is admitted and the head carries
  exactly `1 + fields + 1` CRLFs and ends `\r\n\r\n`
* `a_rendered_integer_is_field_content_but_only_digits_are_a_length` — a negative
  `dec` is admitted at a general field value and refused at a `Content-Length`
* `a_hex_content_length_is_not_decimal_and_refuses`

**The positions**

* `a_bound_view_stands_at_a_header_value_and_at_a_content_position`
* `a_composed_view_is_a_transfer_payload_like_a_written_one`
* `a_declared_message_with_a_wrong_computed_length_is_refused_even_untransferred`
  — `HTTP-QUIRIN-V1`'s "checked exactly as hard" rule, reached through the new
  spelling

**Refusals**

* `a_message_name_at_a_value_position_refuses` (`Q094`)
* `an_operand_naming_nothing_at_all_keeps_the_generic_refusal` (`Q013`)
* `a_rendering_of_a_name_that_is_not_an_integer_refuses` (`Q094`)
* `a_len_of_a_name_that_is_not_text_refuses` (`Q094`)
* `a_composition_of_one_piece_refuses` (`Q090`)
* `a_composition_that_is_not_closed_refuses` (`Q090`, and `Q000` for an
  unbalanced parenthesis)
* `a_rendering_selector_without_its_operand_refuses` (`Q090`)
* `a_value_name_bound_twice_keeps_the_generic_duplicate_refusal` (`Q012`)
* `a_composition_longer_than_the_payload_bound_refuses` (`Q090`, 8192 bytes)

**The gate and route selection, unmoved**

* `a_composed_binding_needs_the_string_module`
* `a_rendered_component_is_this_modules_syntax_and_needs_no_text_module`
* `composing_text_selects_no_route_a_message_does_not_already_select` — both
  directions in one test

**The emitted program**

* `a_length_binds_a_c_object_exactly_when_the_emitted_program_renders_it`
* `a_message_with_computed_components_is_one_object_in_the_emitted_program`
* `the_certificate_with_computed_components_is_independently_rederivable`
* `the_content_length_field_name_is_still_a_written_token`

**The published example**

* `the_computed_length_example_takes_the_http_route`
* `the_examples_declared_length_is_the_length_of_the_content_it_declares`
* `the_computed_length_readme_transcript_is_the_real_one` — both the transcript
  and the wire bytes are read out of `examples/http/README.md` and compared with
  what the pipeline produces, so the page cannot drift
* `native::the_computed_length_example_runs_sanitized_and_writes_the_reference_observation`
* `native::two_runs_of_the_computed_length_example_write_identical_bytes`

`compiler/tests/std_http.rs` is **unchanged at 67 tests, all passing**, which is
the statement that nothing `HTTP-QUIRIN-V1` decided moved.

---

## 6. The executed evidence

`examples/http/computed_length.qcore` — a third published program in that
directory, admitted, emitted, compiled under
`-std=c11 -Wall -Wextra -Werror -pedantic` and run under ASan/UBSan/LSan.

| | |
|---|---|
| Route | `source.http.v1` |
| Shape | one entry, loopback client+server pair; a **composed** body, a length taken of it, a computed `Content-Length`, a composed `Server`-style header value, both messages verified on receipt, three closes |
| Response wire | `HTTP/1.1 200 OK` · `Content-Type: application/json` · `Content-Length: 12` · `Connection: close` · empty line · `{"items":3}\n` |
| Transcript | `3\n12\n` |
| **Byte-comparison result** | **identical** — the native run's stdout equals the reference observation's stdout projection, byte for byte |

`two_runs_of_the_computed_length_example_write_identical_bytes` passes, so no
descriptor number, port or timing reaches the observation.

The `-Werror` build is load-bearing for this card specifically: a `len` binding
lowered into the transport but never rendered would be an unused `int32_t`, and
this run is what would say so.

**Per-arm honesty, unchanged from `HTTP-QUIRIN-V1` §8.** Linux is
runtime-verified under ASan/UBSan/LSan; the `_WIN32` and `__APPLE__` arms remain
**compile-path-correct only** and keep that card's residuals
`HTTP-V1-WINDOWS-RUN` and `HTTP-V1-APPLE-RUN`. This card adds no platform-
specific text: the composed payload is folded into the same
`static const unsigned char` message array the route already emitted.

---

## 7. Lean: `QCore.HttpComputedLength`, audited by `AuditHttpComputedLength.lean`

Registered in `proof/QCore.lean` (so `AuditAll.lean` includes it in the
whole-project axiom closure). **23 declarations audited by name**,
placeholder-free, every closure a subset of
`{propext, Classical.choice, Quot.sound}` — two depend on **no axiom at all**
(`no_computed_statement_derives_an_effect`,
`the_added_statement_forms_are_exactly_two`) and one on `propext` alone
(`the_composed_length_is_the_sum_of_the_piece_lengths`). `Classical.choice`
appears in the closures that go through `List.all`'s decidability lemmas, which
is where `QCore.StringConcat` records it too.

**The card's theorem**, over `QCore.StdHttp.admitFraming` itself, unedited:

* `a_rendered_content_length_is_admitted` — a `Content-Length` written as
  `dec(len(content))` is admitted, and the number it declares *is* the content's
  byte count;
* `a_rendered_length_that_is_not_the_content_length_is_refused` — the converse,
  which is what keeps the check a **gate** rather than a description;
* `a_composed_content_is_framed_by_the_sum_of_its_pieces` and
  `a_composed_content_with_a_wrong_declared_length_is_refused` — the same pair
  for a content that is a composition, declaring the sum of its pieces' lengths.
  That is the published example's own shape.

The two lemmas those rest on:

* `the_composed_length_is_the_sum_of_the_piece_lengths` — constraint 15h as an
  equation. A piece form whose length the compiler could not compute would change
  this theorem rather than slip past it;
* `a_rendering_parses_back_to_the_number_it_rendered` — the decimal roundtrip,
  proved from `fromDigits_digits`, `digits_lt_ten` and `digitByte_toNat`. This is
  what makes a computed length *decidable* rather than a premise: the compiler
  renders a number it computed, and the `1*DIGIT` reader recovers exactly that
  number.

**The injection wall over composed bytes, in both directions:**

* `a_composition_is_framing_free_exactly_when_its_pieces_are` — an **iff**, so
  composing can neither launder a `CR` into a head nor spoil a head built from
  clean pieces;
* `a_framing_byte_in_a_piece_survives_the_composition` — the contrapositive
  spelled out, the direction the Rust refusal test measures;
* `a_rendering_carries_no_framing_byte` — decimal digits are field content, so
  the wall is **unreachable** for a rendered component rather than merely
  satisfied.

Plus `compose_eq_flatten` (a composition is the flattening of its resolved
pieces), the effect discipline (`no_computed_statement_derives_an_effect`), the
closed vocabulary (`the_added_statement_forms_are_exactly_two`) and the
composition arity (`a_composition_of_one_is_refused`,
`a_composition_of_none_is_refused`, `a_composition_of_two_is_admitted`).

**`QCore.StdHttp.the_statement_forms_are_exactly_three` is untouched and still
true.** That theorem is about the **message** vocabulary — declare, header,
content — which this card does not widen; `HTTP-QUIRIN-V1` already kept its own
`let NAME = "…";` binding statement outside it, so the precedent is that card's.
The two statements *this* card adds are enumerated in this module's own `Added`
type instead, on the `QCore.StringConcat.the_added_operations_are_exactly_two`
pattern.

**What is not proved:** that the Rust lowering extracts these pieces faithfully
from source tokens (a tested Rust boundary, as everywhere in this family); that
`hex`'s `0x…` spelling is modelled — it is deliberately not a `Content-Length`
and the `1*DIGIT` rule refuses it, measured on the Rust side; and the same for a
negative `dec` rendering's `-`.

---

## 8. What changed, file by file

| File | Change |
|---|---|
| `crates/qcore-core/src/std_http.rs` | `HttpPiece`; `HttpStatement::{Header, Content}` carry one; `Compose` and `Length` added; `HttpComposition`; `ValueEnvironment` and `transport_scalar_reads`; the two new certificate fields and two new `HttpMessageRecord` flags; `HttpError::NotAValue`; two new recheck variants |
| `crates/qcore-frontend/src/source_http.rs` | the `piece` reader; `concat`/`len` in `lower_let`; `is_computed_binding`; the widened `header`/`content` arms; grammar and census in the module header |
| `compiler/tests/http_computed_length.rs` | new, 33 tests |
| `examples/http/computed_length.qcore` | new published example |
| `examples/http/README.md`, `examples/README.md` | the third program, its wire block and transcript, the follow-on table |
| `docs/QCORE_SPEC_r0_3.md` | §L21's `Q090`–`Q096` row: the widened conditions, stated |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | §G07.10's grammar and its computed-component paragraph |
| `proof/QCore/HttpComputedLength.lean`, `proof/AuditHttpComputedLength.lean`, `proof/QCore.lean` | the Lean module, its audit and one import line |
| `docs/reports/http-v1-computed-length/` | this report and its four artifacts |

**No backend file changed.** `std_http_c_emit.rs` is untouched: it emits from the
certificate's message records, and a computed component only changes which bytes
are in them. That is the measure of how much of the widening the existing
architecture already carried.

---

## 9. Corpus neutrality

By construction. `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`crates/qcore-contract/`, `contract/`, `providers/`, `examples/threads`,
`examples/events`, `examples/sockets`, `examples/strings` and every frozen
archive appear **nowhere** in this diff. The pinned 185-unit artifacts, the leak
count of **266** (constraint 7), the 73-module contract pin and the CT manifest
are main's, untouched and unread.

---

## 10. Named follow-ons

| Name | What is missing |
|---|---|
| `HTTP-V1-COMPUTED-FIELD-NAME` | a **field name** that is a computed value. The value beside it is computable now; the name stays a written token, because a `token` is this module's own syntax and a computed one would need the same argument made for the name position |
| `STRING-SLICE-CONCAT-RUNTIME` | *(the sibling route's, inherited)* a component whose bytes the compiler **cannot** evaluate. That is the exact point at which the `Content-Length` equality would stop being decidable and would have to become a premise — which invariant 2 would not allow it to be |
| `HTTP-COMPUTED-REBIND-REACH` | the model's own "this name is already bound" guard is unreachable from a single source module: name resolution refuses a duplicate binding at `Q012` first. Kept as a fail-closed guard on the model, on the `EXTERN-F1-STORE-REACH` and `HTTP-V1-REDECLARE-REACH` precedent, and pinned by `a_value_name_bound_twice_keeps_the_generic_duplicate_refusal` |
| `HTTP-COMPUTED-UTF8-REACH` | the model's "these composed bytes are not text" guard at a field-value position is likewise unreachable: every piece is a decoded literal (well-formed UTF-8 by `decode_string_literal`) or ASCII digits, and a concatenation of well-formed UTF-8 is well-formed. Kept rather than an `expect` |
| `HTTP-V1-STRING-COMPOSE` | *(unchanged, `HTTP-QUIRIN-V1`'s, narrowed)* a **slice** of a bound view beside HTTP. The route now admits a bound view, a composition and a rendering at a component; a slice and a rendered text line are still outside it |
| `HTTP-V1-BUFFER-CONTENT` | *(unchanged)* a content that is a `std.buffer` extent. A module that buffers still reaches the buffer route, and one that spells `borrow_utf8(…)` still reaches the string route — both selected above this one, deliberately untouched |
| `HTTP-V1-TEXT-TRANSCRIPT` | *(unchanged)* a rendered **text** line in this route's transport body. A computed value reaches a *message*, not a stream; the `StdIoEvent`/`SocketStatement` widening `STRING-SLICE-IO` §5 declined is still declined |
| `HTTP-V1-WINDOWS-RUN`, `HTTP-V1-APPLE-RUN` | *(unchanged)* a native run, sanitizer or byte-comparison claim for those arms. This card's example joins them rather than changing them |
| `HTTP-V1-EXAMPLE-SUITE` | *(unchanged)* `examples/http/` is verified by this route's own test files rather than by `compiler/tests/examples_suite.rs`. The new example joins that arrangement |
| `STRING-SLICE-CONCAT`, `HTTP-V1-COMPUTED-LENGTH` | **retired** — the first by `STRING-CONCAT`, the second by this card |

---

## 11. Gate

Run in this worktree with `$HOME/.elan/bin` on `PATH`. Raw log:
`measure/gate.log` (worktree-private, gitignored).

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree, **before** the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,122 passed, 0 failed, 8 ignored** across 209 result lines, exit **0** |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **Passed 108 Lean audits** (107 → 108: `AuditHttpComputedLength.lean`), no project axiom |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — §11.2 |
| `compiler/tests/http_computed_length.rs` | **33 passed, 0 failed** |
| `compiler/tests/std_http.rs` | **67 passed, 0 failed** (unchanged) |
| `compiler/tests/string_concat.rs` | **53 passed, 0 failed** (the sibling route, unmoved) |
| `compiler/tests/std_string.rs` | **62 passed, 0 failed** · `string_slice_io.rs` **37** · `std_buffer.rs` **52** · `source_loop.rs` **39**, all unmoved |
| `compiler/tests/std_socket.rs` | **49 passed, 0 failed** (the inherited transport, unmoved) |

### 11.1 The count, and what it is made of

**4,122 against the base's 4,089** (`docs/reports/wave25-close/gate.json`), and
the difference is exactly this card's **33** new tests in
`compiler/tests/http_computed_length.rs`: 4,089 + 33 = 4,122. **No existing test
was deleted, rewritten or weakened**, which matters here because this card
changed two variants of an enum the HTTP route's own 67 tests read.

`cargo fmt --all -- --check` failed on its first run — the new test file was
written after this card's earlier `cargo fmt --all` — and is reported as exit 0
above because the formatter was then run and the check redone. Recorded rather
than quietly re-run.

### 11.2 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`lean_audits: 107 != 108`. That is the derived-value tripwire working as
designed: this card adds one audited Lean module and
`docs/reviews/r0_3/acceptance_matrix.json` — which this card may not edit —
carries the pre-card count. §12 item 3. The other 192 Python tests pass.

### 11.3 Every existing program is byte-identical, measured

`artifacts/baseline_emission.sh` emits every published program of every route
this card touches **twice** — once with this card's compiler and once with its
two source files reverted to `origin/main` — and diffs the results
(`artifacts/baseline_emission.txt`):

| Program | Result |
|---|---|
| `examples/http/{exchange,raw_request}.qcore` | **byte-identical** |
| `examples/sockets/{tcp_echo,udp_echo}.qcore` | **byte-identical** |
| `examples/strings/{status_line,computed_length}.qcore` | **byte-identical** |
| `examples/events/{self_pipe,socket_server,wake_payload}.qcore` | **byte-identical** |
| `examples/events/write_interest.qcore` | emits nothing on **either** side — `SOCKET-EVENT-COMPOSE`'s `accept_check_only` arm, that program's own published behaviour |
| `examples/http/computed_length.qcore` | new with this card; the baseline refuses it at `Q090`, which is the before-picture of the seam |

### 11.4 The load this was measured under

Sibling wave-26 card agents were running their own suites in other worktrees on
the same machine throughout, with the one-minute load average between **11 and
81** and mostly in the 25–40 band. The workspace run took roughly two hours of
wall clock under that contention and was driven from a detached script
(`measure/run_gate.sh`) writing a sentinel, which is the measurement-hygiene
arrangement `STRING-CONCAT` §14.4 recorded.

**No `P02-FLAKE` row tripped** (standing constraint 16). Three of the five rows
are in this workspace suite; the other two live in the separate `providers/`
workspace, which this card neither touches nor ran — no provider, schema, package
template or protocol changed, so constraint 18's rebuild is not owed and the
122-test provider checkpoint is **carried, not rerun**.

The Lean leg is read from the **text** line rather than the exit code, per the
script's own behaviour: `scripts/check_lean_audits.py` reports a missing `lake`
and exits 0 when `$HOME/.elan/bin` is not on `PATH`. That is the environment note
`STRING-SLICE`, `STRING-SLICE-IO`, `HTTP-QUIRIN-V1` and `STRING-CONCAT` each
recorded, and it recurred.

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

---

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Four things are the integrator's.

1. **No diagnostics-table row changes.** `AGENTS.md`'s `Q090`–`Q096` row stays
   as it is, or may gain the clause "a component value may be a quoted token, a
   name this entry bound to text, or `dec(NAME)`/`hex(NAME)`; the gates are
   unchanged (`HTTP-V1-COMPUTED-LENGTH`)". Spec §L21 carries the full widening
   and is the authority.

2. **A proposed amendment to standing constraint 15f**, offered because the
   constraint's wording pins a fact this card changed:

   > **15f.** … A declared `Content-Length` must **equal** the content's byte
   > count, decided at check time **because every component is check-time-known**
   > — a quoted token, or (card `HTTP-V1-COMPUTED-LENGTH`) a value the entry
   > composed from check-time-known pieces. Do not turn that equality into a
   > premise, and do not admit a content whose length the route cannot compute:
   > **a component is resolved to bytes before `admit_field` and `admit_framing`
   > run**, which is what keeps both gates unedited and keeps a computed length
   > decided rather than assumed. The `CR`/`LF` exclusion applies to composed
   > bytes exactly as to written ones — a composition is framing-free *if and
   > only if* every piece is — so header injection stays unexpressible however a
   > value was built. The field **name** stays a written token. …

3. **The derived `lean_audits` count moves by one** (107 → 108) for this card's
   new `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the
   stale matrix count until the derived section is regenerated. The same one-line
   reconciliation every wave's new audit file receives (constraint 25).

4. **Public API changes**, one of them breaking:

   * **breaking**, deliberately: `qcore_core::std_http::HttpStatement::Header`
     now carries `{ message, name: String, value: HttpPiece }` rather than
     `{ message, field: HttpField }`, and `Content` carries
     `{ message, value: HttpPiece }` rather than `{ message, bytes }`. A component
     is now a *kind of thing* rather than always a byte sequence, which is the
     change the follow-on asked for; the alternative (parallel variants) would
     have left two spellings of one concept. `HttpField` itself is unchanged and
     is still what a certificate publishes.
   * new in `qcore-core`: `std_http::{HttpPiece, HttpComposition}`;
     `HttpStatement::{Compose, Length}`; `HttpError::NotAValue`;
     `HttpCertificate::{compositions, scalars}`;
     `HttpMessageRecord::{declared_length_computed, content_computed}`;
     `HttpTransport::{compositions, scalars}`;
     `HttpRecheckFailure::{Compositions, Scalars}`. A downstream `match` on
     `HttpStatement`, `HttpError` or `HttpRecheckFailure` needs new arms.
   * `qcore-frontend`: no new public item — `source_http` reads
     `source_std_string`'s already-published `CONCAT_KEYWORD`,
     `is_string_clause_word` and `is_length_query`.
