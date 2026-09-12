# STRING-CONCAT — computed text, and the dependency that dissolved (wave 25)

Card brief: implement the named follow-ons `STRING-SLICE-BUFFER-BORROW`
(amendment §5) and `STRING-SLICE-CONCAT` (amendment §9) of the revision-0.3
string-view amendment — *a string a program **computes** rather than writes.*

`STRING-SLICE-IO` recorded the obstacle verbatim:

> every payload v1 can render is a literal or a slice of one, so a program
> renders text it **wrote** and never text it computed (`STRING-SLICE-CONCAT`)

Base: `origin/main` at `a53fe2a4` (the wave-24 close). Route: `source.string.v1`,
cards `STRING-SLICE` and `STRING-SLICE-IO`. Sibling route the bridge composes
with: `source.buffer.v1`, card `STD-BUFFER`.

**What landed.** Both operations, plus a third the brief's payoff needed:

* **`concat(PIECE, …)`** — two or more **check-time-known** pieces composed into
  a new `static const` payload of the emitted program. No allocation, no
  run-time copy, no custody, and a length that is the sum of its pieces'
  lengths and is itself a check-time constant.
* **`dec(NAME)` / `hex(NAME)`** as a piece — the **text of a bound integer**,
  produced by the same `Rendering::render` a `print dec n;` line renders with.
  This is the one scalar-to-text direction v1 has, and it is what makes a
  computed `Content-Length` expressible at all.
* **`borrow_utf8(NAME)`** — amendment §5's first operation, with its two
  obligations decided at check time: UTF-8 validity out of the buffer's own
  write history, and a mutation-exclusion window over an interval of statement
  indices. `freeze_utf8` is **not** here, argued in §2.

Three diagnostic codes, **`Q077`–`Q079`**, the `StandardString` pass's third
band — the three `STRING-SLICE-IO` returned to the pool. **Zero** new reserved
words. One new published example, `examples/strings/computed_length.qcore`. 47
end-to-end tests in `compiler/tests/string_concat.rs`, two of them sanitized
native runs with a **positive** leak control. **53** declarations in a new
`QCore.StringConcat`, **47** of them audited by name.

**Corpus-neutral by construction** (§10). No file under
`crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/qcore/` or
`providers/` appears in this card's diff.

---

## 1. The headline: the dependency the brief inherited is **false**

The brief, `STRING-SLICE-IO` §11 and amendment §9 all carried the same sentence
about `STRING-SLICE-CONCAT`:

> It needs a building operation, which needs storage a program owns, which is
> `STRING-SLICE-BUFFER-BORROW`'s question first.

**It does not, and the measurement is the whole design of this card.** Every
piece a `concat` can take in v1 is check-time-known — a literal is decoded at
construction, a slice of one is decided by constant bounds, a borrowed view is a
buffer whose bytes are constant writes at constant indices, and `dec(n)` is a
constant. So the *compiler* holds the composed bytes, and the result is a new
**static** payload: storage no program owns, with no allocation, no copy at run
time, no custody obligation and no lifetime rule. The soundness order the brief
asked for ("borrow first, because concat needs owned storage") describes a
different operation from the one v1 wants.

The two landed in one card because one card was assigned both, not because
either needed the other. `a_composing_module_that_holds_no_buffer_needs_no_buffer_module`
is that statement as a test: a module that composes text imports `std.string`
and an output module, and **not** `std.buffer`.

The materialising alternative was measured rather than waved away, and is named
rather than left implicit — see §3.2.

---

## 2. The four decisions the brief asked to be argued

### 2.1 Composition: **static**, not materialised

**Decision.** A `concat`'s result is a payload the checker computes and the
backend emits as a file-scope `static const unsigned char` array. The view that
names it is `{ composed_payload, length }`, built exactly as a literal view is.

**What each route costs, measured.**

| | Static composition (landed) | Materialised into an owned buffer (named) |
|---|---|---|
| Allocation | none | one per composition |
| Copy at run time | none | one per piece |
| Effect row | unchanged — neither `Alloc` nor `Mut` | must carry `Alloc` and `Mut` honestly |
| Custody | none; the result is static | a release obligation on a *string* |
| `len` of the result | a **check-time constant** | a run-time quantity |
| `allocator_calls` tripwire | still empty for a string-only program | would fire, by design |
| Amendment §7 | survives verbatim | would need amending |

The last two rows decided it. `source.string.v1`'s standing evidence is that
`std_string_c_emit::allocator_calls` finds no `malloc`/`calloc`/`realloc`/
`strdup`/`memcpy`/`memmove`/`strcpy`/`strlen` in **any** program the backend
writes; the materialising route would have retired that tripwire for an
operation whose result the compiler already held. And the length row is the
card's whole purpose: a computed `Content-Length` is decidable *because* the
composed length is a constant, and materialising would have taken that away in
the course of implementing the feature that needs it.

The named follow-on is **`STRING-SLICE-CONCAT-RUNTIME`**, and it becomes
necessary at exactly the point v1's premise fails: a piece the compiler cannot
evaluate. That is the same wall `STRING-SLICE-VARYING-INDEX` and
`STD-BUFFER-VARYING-INDEX` name, and the same second arm amendment §2 permits.

**What "static" is, as a checkable field rather than a claim.** The certificate
gains `composed: Vec<Vec<u8>>` — the payloads themselves — and
`concats: Vec<ConcatRecord>`, each carrying `piece_lens` and the result view.
`the_length_of_a_composition_is_the_sum_of_its_pieces` asserts
`result.len == piece_lens.iter().sum()` on a real compilation, and
`a_composed_payload_is_static_storage_of_its_own` asserts the result's storage is
`StringStorage::Composed(0)` and its offset is `0` — not a window onto a piece.
In Lean, `a_composition_addresses_none_of_its_pieces` proves the storage index is
one the environment did not hold.

**A composition of one is refused.** A composition of one *is* that piece;
admitting it would give the route two spellings of one view with nothing to tell
them apart by. Both the Rust refusal (`a_composition_of_one_piece_refuses`,
`Q077`) and the Lean statement (`a_composition_of_one_is_refused`) are pinned.

### 2.2 The scalar piece, and why it is the third operation

**Decision.** `dec(NAME)` and `hex(NAME)` are admitted as `concat` pieces, and
produce **exactly the bytes `print dec NAME;` writes**, because both go through
`qcore_core::std_io::Rendering::render` and nothing else.

This was not in the brief's list, and it is the piece that makes the brief's own
payoff reachable. A computed `Content-Length` is the *digits of a number the
program derived*; without a scalar-to-text direction, `concat` can compose a
header's name and a header's terminator and never its value.

It costs one match arm and no new machinery, because the digits of a
check-time-constant integer are themselves check-time-known — the same premise
the whole static-composition route rests on. Both renderings are admitted rather
than one: `dec` and `hex` are the closed rendering vocabulary, and supporting one
would have invented an asymmetry the reader would then have to remember.

`the_text_of_a_number_is_the_bytes_that_number_renders_as` measures both
spellings of the same number in one program:
`concat("n=", dec(n))` renders `n=42` and `print dec n;` renders `42`.

`STRING-SLICE-IO` recorded that it supplied "no conversion between a string and a
scalar in either direction". **One direction is now supplied**, and the other is
not: there is no parse of text into a number, which needs a failure arm v1 has
nowhere to put (`STRING-SLICE-PARSE`).

### 2.3 The bridge: **borrow only**, and why the freeze is a separate card

**Decision.** `borrow_utf8(buffer) -> string` is implemented.
`freeze_utf8(own buffer) -> own string` is **not**, and the reason is structural
rather than a matter of budget.

A frozen string is a **new custody shape**. v1's string value is
`StringViewRecord { storage, offset, len }` and its Lean counterpart is
`View { storage, offset, len }`; an owned one needs a fourth component for a
release to discharge, a `drop` spelled on a *string*, and a lifetime rule for
every view sliced out of one — because a slice of a frozen string is a view of
storage that can end, which is exactly the thing `STRING-SLICE` recorded as not
existing in v1. Every theorem in `QCore.StdString` and every field of the
published certificate is about the three-component view.

**And the borrow plus a composition reaches every byte the freeze would.** This
is the load-bearing part of the argument rather than a convenience: when a
program needs a buffer's bytes to outlive the buffer, it composes them, and the
composition copies them into static storage at check time. `a_composition_of_a_borrow_ends_the_window_at_the_composition`
measures exactly that — a program borrows, composes, **releases the buffer**, and
then renders the composed text, and the rendered line is right. So nothing is
blocked on the freeze, which is why it is a follow-on
(**`STRING-SLICE-FREEZE`**) rather than a gap.

The Lean record of the argument is `a_view_carries_no_custody`, written as a
structural fact so that adding a fourth component changes a theorem rather than
slipping past.

### 2.4 UTF-8 validation: **static**, and the follow-on named

**Decision.** The validity of a borrowed buffer's bytes is decided **at check
time**, by running a decision on the buffer's contents as the checker has
modelled them. There is no run-time scan, no failure arm and no trap.

The brief anticipated this and it holds exactly: `source.buffer.v1` admits only
constant bytes written at constant indices (`BufferOperand` has two constructors
and neither can denote a resource, and every index is a check-time constant), so
a buffer's contents are a **check-time value**. The decision is therefore the
first of the two arms amendment §2 permits, applied to the bridge, exactly as
`STRING-SLICE` applied it to the slice bound.

One case says **yes** and is worth its own line: `bytes(N)` zero-initialises, and
a run of `U+0000` is well-formed UTF-8, so an untouched buffer borrows.
`an_untouched_buffer_borrows_as_its_own_zero_bytes` pins it, and
`QCore.StringConcat.a_zeroed_buffer_is_well_formed` proves it — a model that
refused it would disagree with the route's own `calloc`.

Three refusals are pinned in the other direction and each names a different way
UTF-8 can fail: a byte that begins no sequence (`255`), a lead byte whose
continuation is missing (`195, 33`), and a continuation byte with no lead
(`97, 128`). The message names the byte the decoding stopped at, which is the
evidence the decision was made **by decoding** rather than by a whole-buffer
guess.

The varying-content follow-on is **`STRING-SLICE-BORROW-VARYING`**, and what it
must add is a run-time validation and a checked failure arm — the second arm §2
permits, and the same wall `STD-BUFFER-VARYING-INDEX` names.

---

## 3. The borrow window, and what it buys

### 3.1 The rule

A borrow's window is the interval of statement indices from the borrow to **the
last statement that reads the view or anything derived from it**. Inside it, a
store into or a release of the buffer refuses at `Q078`; after it, both are
admitted.

The interval is **exact rather than conservative**, and that is a property of the
route rather than a choice: `source.string.v1` is one flat statement sequence
with no branch to make a later index unreachable and no loop to make an earlier
one run again, so the last-use index *is* the end of the window. A parallel card
owns the loop construct; when one exists, this rule is the first thing it must
revisit, which is why the window's computation lives in one function
(`last_view_uses`) over one exhaustive `match` (`view_uses`).

"Anything derived from it" is not a second reachability pass. A slice of a
borrowed view carries the same `StringStorage::Buffer(name)` — that is
`admit_slice`'s existing "the storage is carried through unchanged", which was
the zero-copy claim and is now also the borrow-extension claim — so the derived
view is registered as a borrow of the same buffer with **its own** last use.
`a_slice_of_a_borrow_extends_the_window_the_borrow_excludes_over` measures it,
and `a_derived_view_extends_the_window` proves it.

**A borrow that is never read constrains only its own statement.** That is what
makes a program that borrows able to release its buffer at all: a window
running to the end of the entry would make every borrowing program leak.
`the_window_closes_after_the_last_use_so_a_write_after_it_is_admitted` measures
the useful consequence — borrow, render, **write a new byte**, borrow again,
render again, release — and the two rendered lines differ, which is the proof
that the second borrow saw the second write.

### 3.2 Why this is soundness and not bookkeeping

`QCore.StringConcat.an_excluded_window_preserves_the_buffers_bytes`: over a
window in which no statement touches the buffer, the bytes a view addresses at
the end of the window **are** the bytes that were there at its start. Composed
with §2.4, that is the statement the whole bridge rests on: the view a program
renders is the view the checker decided was well-formed UTF-8, and not a later
one. Its contrapositive, `a_changed_buffer_had_a_touching_statement`, is the
form the checker actually runs.

### 3.3 Custody is inherited, not reimplemented

Every leak, double release, use after release and rebound live name in a
composed module is `qcore_core::ownership::check_ownership`'s, over an operation
list `std_string::string_custody_operations` derives — the `PIPE-CHAIN` and
`STD-BUFFER` discipline, applied to the composed route.

The one addition is that **a borrow is a `Read`**. So `drop b; let s = borrow_utf8(b);`
is that checker's own `UseAfterDrop`, not a rule this card wrote —
`a_borrow_of_a_released_buffer_refuses_from_the_inherited_ownership_checker`
asserts both the code and that the message is the checker's own text.

`Q078` carries both the borrow exclusion and the inherited custody refusal.
That is a deliberate economy on a three-code band and it is coherent: both are
statements about the custody and liveness of a buffer this composed entry holds,
and the message says which. Splitting them would have needed a fourth code the
band does not have.

---

## 4. The band: `Q077`–`Q079`, and what it deliberately does **not** take

The integrator pre-assigned `Q077`–`Q079` — the three codes `STRING-SLICE-IO`
returned to the pool, reassigned to the same pass. This card uses all three.

| Code | Condition |
|---|---|
| `Q077` | The **shape or the bound** of a construct this band adds — a malformed `concat(…)`, `borrow_utf8(…)` or `dec(…)`/`hex(…)` piece, a composition of fewer than two pieces, a composed payload longer than 4096 bytes, a bridge position naming something this entry acquired no buffer for, a buffer statement outside the entry, and an embedded `source.buffer.v1` statement that route's own model refused |
| `Q078` | The **borrow discipline** — a store into or a release of a buffer inside a live borrow's window — and a refusal reported by the **inherited, unedited** ownership checker, carrying that checker's own error value |
| `Q079` | **The static UTF-8 decision**: the bytes a `borrow_utf8` would view are not well-formed UTF-8, decided out of the buffer's own write history |

**`Q079` is deliberately not `Q070`.** `Q070`'s condition is "a quoted token this
route cannot decode"; this is a buffer's write history, not a token, and it is
the card's headline soundness check. The `STRING-SLICE-IO` precedent (`Q076`
rather than a reuse of `Q069`) governs: a condition that reads alike from a
distance and is about a different thing gets its own code.

**Everything else a construct of this band can be refused for is an existing
refusal at its existing code**, which is the honest measure of how much the card
actually added:

| Shape | Code | Decided by |
|---|---|---|
| a literal piece that does not decode | `Q070` | the base route's decoder, wherever the token is |
| an inline slice piece out of range or off a code-point boundary | `Q071` | `admit_slice`, the same one a `let` uses |
| a piece naming no string | `Q069` | the base route's view-position rule (§4.1) |
| a string or a buffer at a scalar position | `Q072` | unchanged |
| an undeclared derived effect | `Q073` | unchanged |
| a missing `std.string` or `std.buffer` import | `Q015` | the import gate, unchanged |
| an unbalanced parenthesis | `Q000` | the parser, before any route |
| a buffer-route statement in a module with no bridge | `Q051` | `source.buffer.v1`, unchanged |

### 4.1 One code choice that needed a parameter

A `concat` piece naming no string refuses at **`Q069`**, not at `Q076`. The two
are the same *lookup* and different *positions*: `Q076` is the **output**
vocabulary's operand, where a reader who wrote `print x;` for an integer `x`
wants to be told to name a rendering; a `concat` piece is a **view position
inside the string vocabulary**, exactly like a slice's source and a `len`'s
argument, where that advice would name a statement the reader never wrote.

`text_view` resolves both, so the position is passed to it as `TextPosition` and
decides **only which code a name with no view refuses at** — nothing else
differs, which is what keeps "a piece cannot be admitted where the same spelling
at a rendering operand would be refused" true.

### 4.2 The embedded buffer statement, and §L21

A composed module holds both vocabularies, so its refusals span two passes'
conditions. §L21 forbids a second semantic pass taking another's codes — the
rule `STD-BUFFER` itself learned the hard way (`Q057` was `Q029` in an early
draft) and `QUIRIN-SOCKET-V1` restated (`Q083` deliberately does not reuse
`Q054`, "even where the inherited refusal is literally the same
`qcore_core::ownership` checker's").

So an embedded buffer statement's refusal is reported at **`Q077`** carrying
**`source.buffer.v1`'s own message, verbatim**. The code says which route asked;
the message says what is wrong, in the words of the route whose fault it is.
`a_buffer_statement_a_composed_module_cannot_model_keeps_the_buffer_models_own_words`
pins both halves: the code is `Q077` and the message contains
"is outside `b`, whose declared extent is 2 bytes".

---

## 5. Keyword census (standing constraint 14): **zero new reserved words**

Script output archived at `artifacts/keyword_census.txt`. Scope: every
`.qcore`/`.qc` file on disk — **418 at census time**, 420 after this card's two
new files — plus every tracked `.rs` and `.md` file.

| Candidate | `.qcore`/`.qc` bare word | `WORD(` shape, all tracked files | Disposition |
|---|---|---|---|
| `concat` | **0** | **0** | contextual clause word |
| `borrow_utf8` | **0** | **0** | contextual clause word |
| `dec`, `hex` | 78, 21 | **0** | already reserved (`STD-MODULES`) |
| `join` | **207** | — | rejected: `thread scope` join, in pinned fixtures |
| `build` | 10 | — | rejected: occurs in prose and comments |
| `freeze_utf8`, `utf8`, `append`, `compose` | 0 | 0 | unused, recorded for a later card |

`concat` and `borrow_utf8` join `len` as **contextual** clause words: each is
skipped by name resolution only where it is immediately followed by `(`. That is
the `STRING-SLICE` rule generalised into one predicate,
`source_std_string::is_string_clause_word`, which is the single hook
`frontend.rs`'s reference collector consults — so the module's contextual words
cannot drift from the words the lowering reads.

Both directions are pinned. `the_two_contextual_words_are_reserved_nowhere`
compiles a module that binds `let concat = 1;` and `let borrow_utf8 = 2;` and
renders both, which is what "reserved nowhere" means operationally; and the
existing `the_contextual_len_rule_still_leaves_a_contract_parameter_resolvable`
(card `STRING-SLICE`'s pinned-corpus reproduction) is untouched and still passes.

`join` is recorded rather than merely avoided: it is the obvious alternative
spelling for a composition, it occurs 207 times in shipped `.qcore` files, and
reserving it would have refused them at `Q013`. The `work` → `effort`,
`error` → `fatal` and `buffer` → `bytes` precedent — rename the card's syntax,
never the pinned surface — applied before a line was written.

---

## 6. Route selection: one new door, one new spelling

**The entire change to `crates/qcore-verification/src/driver.rs` is three lines
of code plus its comment**, placed above the buffer route:

```rust
if qcore_frontend::source_std_string::has_source_string_bridge(&parsed) {
    return compile_string_source(&parsed, &typed, mode, options, &ranges);
}
```

The condition is `borrow_utf8(…)` and nothing else. Three consequences, each
measured:

* **A module that holds a buffer without spelling the bridge keeps the buffer
  route**, and that route's existing refusal. That is the *pre-card* answer to
  amendment §5 — `std.buffer` is not implicitly convertible to `string` —
  unchanged for every module that does not ask for the conversion by name
  (`a_module_that_buffers_and_slices_without_the_bridge_keeps_the_buffer_routes_refusal`,
  still `Q051`).
* **A module that composes text holds no buffer** and never competed with that
  route, so `concat` changes no selection at all.
* **Both directions are pinned in one test**
  (`the_bridge_selects_the_string_route_and_a_buffer_without_it_does_not`): the
  bridged module reports `StandardStringV1` and the unbridged one
  `StandardBufferV1`.

Inside the lowering there is a second, narrower gate on the same principle:
**a composed module is one that acquires a buffer.** A module with no `bytes(N)`
in its entry embeds no buffer statement at all. That gate was not planned — it
was added because without it `drop s;` on a *string* stopped refusing at `Q068`
("`drop` is not a statement this route models") and started refusing at `Q077`
("`s` is not a buffer") plus a spurious undeclared-`Alloc` `Q073`. The pinned
test `a_statement_this_route_does_not_model_refuses` caught it, and the repair
was to narrow rather than to re-pin.

---

## 7. The payoff: a computed `Content-Length`

The brief: *"a program that computes text …, takes `len()` of the result, and
renders both — proving computed text reaches output. If the HTTP composition is
cheap … land it as a test."*

It was cheap, and it landed as a **published example**:
`examples/strings/computed_length.qcore`. The program writes `{"ok":true}` one
constant byte at a time into an owned buffer, borrows it as UTF-8, takes `len`
of the view, composes `concat("Content-Length: ", dec(length), "\r")`, and
renders a CRLF-framed HTTP/1.1 response head followed by the content — then
releases the buffer, which the borrow window admits because the content render
was the borrow's last use.

Three tests hold it up:

* `the_published_computed_length_example_runs_and_matches_its_readme_transcript`
  — compiled through the real pipeline, built under
  `-Wall -Wextra -Werror -pedantic`, run under ASan/UBSan/LSan, compared per
  stream with the reference execution, and compared against the transcript read
  **out of the README**, so the page cannot drift from the program. The header
  line is asserted on its **bytes** (`b"Content-Length: 11\r"`), because a line
  split is exactly what would hide the carriage return.
* `the_computed_length_is_the_length_of_the_content_it_declares` — the equation,
  over the program's own observation: the number in the header line is the byte
  count of the content line, and neither was written twice in the source.
* `a_length_composed_into_text_is_the_length_of_the_text_it_describes` — the
  same shape without the buffer, over a literal content.

**What remains, named rather than implied.** This is not HTTP over a wire: the
`source.http.v1` route (card `HTTP-QUIRIN-V1`) owns messages and `source.socket.v1`
owns transport, and this card touched neither. Connecting a computed length to
the HTTP route is the follow-on **`HTTP-V1-COMPUTED-LENGTH`**, and what it needs
is a `content` clause that takes a *bound view* rather than a quoted literal,
plus a `Content-Length` value that is a composed string rather than a literal —
both inside a route this card does not own. The remaining byte-level gap in the
example is stated in its README: the content's eleven bytes reach standard
output followed by the line feed every rendered line carries, which the message's
content does not contain.

---

## 8. Emission, and three hygiene findings

The emitted C gained three conditionals, each because `-Wall -Wextra -Werror`
made an unconditional form an error. All three were found by the sanitized
native run rather than reasoned about in advance, and all three are the same
constraint `STD-BUFFER` §7.6 recorded for its own prelude.

1. **Only the literals the emitted program addresses are defined.** A literal
   written as a `concat` piece is read by the checker and folded into the
   composed payload, so the emitted program never names it — and an
   unreferenced file-scope `static const` array is
   `-Werror=unused-const-variable`. The array names carry their pool index, so
   skipping one renumbers nothing.
2. **A `concat` piece is not a *consumer*.** The `consumed` map that decides
   which bindings get an explicit `(void)` discard deliberately excludes concat
   pieces: the composition happens at check time, so the emitted program never
   reads the piece's view or scalar. A binding whose only reader is a
   composition is `(void)`-discarded, which is the honest record of what the
   emitted program does with it. The first draft marked them consumed and
   produced `-Werror=unused-variable`.
3. **The slice helper is emitted only for a program that slices at run time.**
   `STRING_PRELUDE` split into `STRING_PRELUDE_VIEW` (the aggregate, always) and
   `STRING_PRELUDE_SLICE` (the helper, conditional). A program whose only slices
   are `concat` pieces calls nothing and would otherwise carry an unused
   `static` function. Both shipped fixtures and both published examples slice at
   run time, so **no program written before this card changes by a byte**.

The **buffer prelude** is contributed on the same terms: only a program that
acquires a buffer carries `calloc`, `free` and their accessors, and it is
`source.buffer.v1`'s own `buffer_prelude_for` rather than a second copy.
Likewise every embedded buffer statement is emitted by that route's own
`buffer_event_statement` with that route's own C names, so a composed module's
`bytes(8)` is the same `qcore_bytes_new` call the uncomposed route writes.

**The zero-allocation tripwire is scoped, not weakened.**
`std_string_c_emit::allocator_calls` is unchanged and still returns empty for
every program with no buffer in it — pinned by
`a_composed_program_still_allocates_nothing_copies_nothing_and_never_scans`,
which also asserts the composed payload's array is present. A composed module
that holds a buffer allocates exactly its buffers, which is what its effect row
says.

---

## 9. The one shared-ground refactor

Two files owned by `source.buffer.v1` were **published rather than duplicated**,
with no behaviour change. `compiler/tests/std_buffer.rs` — 52 tests, untouched —
passed before and after each step, which is the evidence that the refactor is
behaviour-preserving.

* **`crates/qcore-core/src/std_buffer.rs`**: the private `Evaluation` became
  `pub struct BufferEvaluation` with a published stepping function
  `apply(index, event) -> BufferStep`. `check_buffer_program`'s six value-event
  arms now call it, so the extent rule, the bounds rule, the byte-range rule and
  the decidability rule have **one** implementation that both routes run. The
  composed route shares the `scalars` map rather than keeping a second one, so a
  name bound by `len(s)` and a name bound by `b[0]` live in one environment.
* **`crates/qcore-core/src/std_buffer_c_emit.rs`**: `buffer_event_statement`,
  `buffer_prelude_for` and `buffer_name` are published; `emit_buffer_c11` calls
  the first two.
* **`crates/qcore-frontend/src/source_std_buffer.rs`**: `buffer_statement` is
  published on the `source_std_io::std_io_statement` shape — it returns the
  refusal as a **message** the caller attributes to its own code.

Everything else the card touched is the string route's own, plus the
`driver.rs` seam of §6 (**11 lines, 3 of them code**) and the
reference-collector hook of §5 (**one call changed**, from `is_length_query` to
`is_string_clause_word`; the old function is retained unchanged for the tests
that name it).

**Two existing test files were edited, and only for the renamed field.**
`compiler/tests/std_string.rs` (12 lines) and
`compiler/tests/string_slice_io.rs` (1 line) read
`StringViewRecord::literal`, which became `storage: StringStorage`. Every
assertion keeps its meaning: `slice.derived.literal == slice.from.literal`
became `slice.derived.storage == slice.from.storage`, which is the same claim
about a wider type. **No test was deleted, weakened or re-pinned**, and the two
suites' counts are unchanged at 62 and 37.

---

## 10. Corpus neutrality

By construction, and the file list is short enough to read whole — **24 files**:
four in `qcore-core`, three in `qcore-frontend`, one three-line seam in
`qcore-verification/src/driver.rs`, three normative documents, three Lean files
(one new module, one new audit, one import line), four test files, one fixture,
the two `examples/strings` files plus the examples index, and this report with
its two artifacts.
`contract/`, `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`crates/qcore-contract/`, `providers/`, `examples/threads`, `examples/events`,
`examples/sockets`, `examples/http` and every frozen archive appear nowhere in
it. The pinned 185-unit artifacts, the leak count of **272**, the 72-module
contract pin and the CT manifest are main's, untouched and unread.

---

## 11. Lean

`proof/QCore/StringConcat.lean` (registered in `proof/QCore.lean`, so
`AuditAll.lean` includes it in the whole-project axiom closure) and
`proof/AuditStringConcat.lean` — **53 declarations, 47 audited by name**, all
placeholder-free with no project axiom; every closure is a subset of
`{propext, Classical.choice, Quot.sound}`. Most closures are `[propext]` or
`[propext, Quot.sound]` and thirteen depend on **no axiom at all**, including
every case of the UTF-8 decision and `a_composition_addresses_none_of_its_pieces`;
`Classical.choice` appears in **three** closures — the borrow-window theorems,
where it arrives through the decidability lemmas `simp` uses, and the
contrapositive, which uses `Classical.byContradiction`. It is one of the three
allowed axioms and is recorded here rather than left to be discovered.
`python3 scripts/check_lean_audits.py` reports **Passed 104 Lean audits**
(103 → 104) with no warning and exit **0**.

What is proved, in the order the card argues:

* **The length arithmetic** — `the_composed_length_is_the_sum_of_the_piece_lengths`,
  with `composing_two_pieces_appends_their_bytes`, the arity generalisation
  (`a_composition_of_three_is_a_composition_of_a_composition`) and the identity
  (`an_empty_piece_contributes_no_bytes`).
* **A composition is storage of its own** —
  `the_composed_view_addresses_the_composed_payload`,
  `a_composition_addresses_none_of_its_pieces` (the statement that makes a
  composition of a borrow *not* a borrow), and
  `extending_the_storage_moves_no_existing_view`.
* **The admission rule**, in both directions — a composition of one or of none
  is refused, an over-long one is refused, and an admitted one is inside the
  bound.
* **The static UTF-8 decision** — a decision function over the lead/continuation
  structure, with the cases the card's tests measure, including the one it says
  *yes* to (`a_zeroed_buffer_is_well_formed`).
* **The borrow window as an interval** — refused inside, admitted after,
  constraining no other buffer, and extended by every derived view.
* **The exclusion as soundness** — `an_excluded_window_preserves_the_buffers_bytes`
  and its contrapositive.
* **The effect discipline** — neither added operation derives anything, so §7's
  "no `Alloc` or `Mut` for a string operation" survives the card.
* **Why the freeze is not here** — `a_view_carries_no_custody`, and
  `the_added_operations_are_exactly_two` written as one list so a widening
  changes a theorem rather than slipping past it.

**What it does not prove**, carried over from the family's convention: that the
Rust lowering extracts these events faithfully from source tokens (a tested Rust
boundary), and that this model's `wellFormed` is *full* UTF-8 validation — it
decodes the lead/continuation structure and does not reject overlong forms or
surrogates, which the Rust side rejects by delegating to the language's own
decoder at the one point bytes enter the model. That is the same boundary
`STRING-SLICE` recorded ("the Lean model is coarser than the Rust one on
UTF-8"), stated again because this card moved where the decision is made.

---

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Four things are the integrator's.

1. **Three new diagnostic codes.** `AGENTS.md`'s diagnostics-namespace table
   needs the third band added to the `StandardString` row (authority is spec
   §L21, which this card updated):

   > `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` in
   > three bands: the immutable UTF-8 `{ptr, len}` string view (`Q068`–`Q073`);
   > its composition with the output modules (`Q075`–`Q076`); and **computed
   > text** (`Q077`–`Q079`) — `concat(…)` over check-time-known pieces into a
   > new static payload, `dec(NAME)`/`hex(NAME)` for the text of a bound
   > integer, and `borrow_utf8(NAME)` bridging an owned `std.buffer` into an
   > immutable view. `Q077` is the band's shape and bound refusals, including an
   > embedded `source.buffer.v1` statement carrying **that route's own message**
   > under this pass's code; `Q078` the borrow-exclusion window together with
   > the inherited ownership checker's own refusal; `Q079` the **static UTF-8
   > decision** on a buffer's write history (`STRING-CONCAT`).

2. **A proposed standing constraint**, offered because it is the kind of rule the
   "when touching X, do not Y" inventory exists for:

   > **15f. A `concat` result is static storage the compiler computed, and a
   > `borrow_utf8` view is the buffer's own.** `source.string.v1` composes text
   > from **check-time-known** pieces into a new `static const` payload of the
   > emitted program: no allocation, no run-time copy, no custody, and a length
   > that is the **sum of the pieces' lengths** and is itself a check-time
   > constant. Do not make the composition materialise into an owned buffer —
   > that was measured and rejected, because it would add an allocation, a copy
   > and a release to an operation whose result the compiler already holds, and
   > would turn `len` of the result into a run-time quantity, which is the one
   > property the operation exists to preserve (`STRING-SLICE-CONCAT-RUNTIME`).
   > The `std.buffer` bridge is **borrow only**: an owned string is a new
   > custody shape the `{storage, offset, len}` view has nowhere to carry, and
   > composing a borrowed view already launders its bytes into static storage,
   > so nothing is blocked on the freeze (`STRING-SLICE-FREEZE`). A borrow's
   > UTF-8 validity is decided **statically**, out of the buffer's own write
   > history — `source.buffer.v1` admits only constant bytes at constant
   > indices — and its **window** runs from the borrow to the last statement
   > reading the view or **anything derived from it**, a slice of a borrow being
   > itself a borrow of the same buffer. Do not widen the window's computation
   > without revisiting it: the interval is exact because the route is one flat
   > statement sequence, and a loop construct invalidates that premise. A
   > borrow contributes an ordinary `Read` to the cycle-3 ownership operation
   > list; do not re-implement the custody rules beside it. **Route selection
   > changes only for the bridge construct** — a module holding a buffer without
   > spelling `borrow_utf8` keeps `source.buffer.v1` — and a composed module is
   > one that **acquires** a buffer, which is what keeps `drop s;` on a string
   > refusing at `Q068` where it always did. `string-concat`.

3. **The derived `lean_audits` count moves by one** for this card's new
   `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the stale
   matrix count until the derived section is regenerated. The same one-line
   reconciliation every wave's new audit file receives (constraint 25).

4. **Public API changes**, all in `qcore-core` and `qcore-frontend`:

   * `qcore_core::std_string`: `StringViewRecord::literal` is **replaced** by
     `StringViewRecord::storage: StringStorage`, a new three-constructor enum;
     `StringEvent` gains `Concat`, `Borrow` and `Buffer`; `StringError` gains
     `Composition`, `NotABuffer`, `BorrowExclusion`, `Custody` and
     `BufferNotUtf8`; `StringCertificate` gains `composed`, `concats`,
     `borrows`, `acquisitions` and `accesses`; `StringRecheckFailure` gains five
     variants; `StringPiece`, `ConcatRecord`, `BorrowRecord`,
     `concat_piece_name` and `string_custody_operations` are new. A downstream
     `match` on either enum needs new arms.
   * `qcore_core::std_string_c_emit`: `preamble` now takes the certificate;
     `STRING_PRELUDE` is replaced by `STRING_PRELUDE_VIEW` and
     `STRING_PRELUDE_SLICE`.
   * `qcore_core::std_buffer`: `BufferEvaluation` and `BufferStep` are new and
     public; no existing item changed meaning.
   * `qcore_core::std_buffer_c_emit`: `buffer_event_statement`,
     `buffer_prelude_for` and `buffer_name` are new and public.
   * `qcore_frontend::source_std_buffer`: `buffer_statement` is new.
   * `qcore_frontend::source_std_string`: `is_string_clause_word`,
     `has_source_string_bridge`, `CONCAT_KEYWORD` and `BORROW_UTF8_KEYWORD` are
     new; `StringUse` gains `Concat` and `Borrow`. `is_length_query` is retained
     unchanged.

---

## 13. Named follow-ons

| Name | What is missing |
|---|---|
| `STRING-SLICE-FREEZE` | `freeze_utf8(own buffer) -> own string`. Needs a custody component on the string value, a release spelled on a *string*, and a lifetime rule for every view sliced out of one. **Nothing is blocked on it**: a composition of a borrow already launders the bytes into static storage. §2.3. |
| `STRING-SLICE-CONCAT-RUNTIME` | A composition with a piece the compiler cannot evaluate. Needs storage the program owns, an `Alloc` effect stated honestly, a release, and a length that is no longer a constant. §2.1. |
| `STRING-SLICE-BORROW-VARYING` | A borrow of a buffer whose contents are not a check-time value. Needs a run-time validation and a checked failure arm — amendment §2's second arm, the wall `STD-BUFFER-VARYING-INDEX` names. §2.4. |
| `STRING-SLICE-PARSE` | Text into a number. The reverse of `dec(NAME)`, and the harder direction: it has a failure arm this flat route has nowhere to put. §2.2. |
| `HTTP-V1-COMPUTED-LENGTH` | A computed `Content-Length` inside `source.http.v1`. Needs a `content` clause taking a bound view rather than a quoted literal, and a header value that is a composed string — both in a route this card does not own. §7. |
| `STRING-CONCAT-LOOP-WINDOW` | **Recorded now rather than discovered later.** The borrow window is an interval of statement indices, exact because the route is one flat sequence. A loop construct (a parallel card's) invalidates that premise: a store inside a loop body can precede a read that is textually earlier. The computation lives in `last_view_uses` over one exhaustive `match` precisely so this is a small, locatable repair. §3.1. |
| `STRING-CONCAT-WINDOWS-RUN` | This card ran the **Linux** arm only. The emitted composition is a `static const` array and the borrow is a field initialisation, both C11 and both inside the existing prelude structure, and the test file carries the Windows `windows_c` path unchanged — but no Windows or Apple run was performed and none is claimed. The `EVENT-V1-WINDOWS-RUN` / `STRING-SLICE-IO-WINDOWS-RUN` precedent. |
| `STRING-SLICE-IO-EXAMPLE-SUITE` | *(unchanged, `STRING-SLICE-IO`'s)* `examples/strings/` is verified by the string cards' own test files rather than by `compiler/tests/examples_suite.rs`. This card's example joins that arrangement rather than changing it. |
| `STRING-SLICE-ABI`, `STRING-SLICE-VARYING-INDEX`, `STRING-SLICE-NOTASCALAR-REACH`, `BYTE-SLICE-CAP`, `STD-FMT-DIAGNOSTIC` | *(unchanged, earlier cards')* |
| `STRING-SLICE-BUFFER-BORROW`, `STRING-SLICE-CONCAT` | **retired by this card.** |

---

## 14. Gate

Run in this worktree on main `a53fe2a4` plus this card, with `$HOME/.elan/bin`
on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree |
| `cargo test --workspace --no-fail-fast` | see §14.1 |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **Passed 104 Lean audits** (103 → 104: `AuditStringConcat.lean`), no project axiom, exit **0** |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — see §14.3 |
| `compiler/tests/string_concat.rs` | **50 passed, 0 failed** |
| `compiler/tests/std_string.rs` | **62 passed, 0 failed** (base route, unmoved) |
| `compiler/tests/string_slice_io.rs` | **37 passed, 0 failed** (second band, unmoved) |
| `compiler/tests/std_buffer.rs` | **52 passed, 0 failed** (the refactored sibling route, unmoved) |

The four suite lines above are the load-bearing ones for this card and were
re-run at the **final committed state**. `std_buffer.rs` in particular was run
after each step of §9's refactor, which is what makes "no behaviour change" a
measurement rather than an intention.

### 14.2 The Lean toolchain is not on the default `PATH`

`lean`, `lake` and `elan` live at `$HOME/.elan/bin` and are **not** on this
worktree's default `PATH`; `scripts/check_lean_audits.py` reports a missing
`lake` **and exits 0** in that state, so the run above was made with that
directory exported and the "Passed 104 Lean audits" **text** verified rather
than the exit code alone. That is the environment note cards `STRING-SLICE` and
`STRING-SLICE-IO` each recorded, and it recurred.

### 14.3 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`lean_audits: 103 != 104`. That is the derived-value tripwire working as
designed: this card adds one audited Lean module, and
`docs/reviews/r0_3/acceptance_matrix.json` — which this card may not edit —
carries the pre-card count. §12 item 3. The other 192 Python tests pass.

### 14.4 The load this was measured under

Sibling wave-25 card agents were running their own suites in other worktrees on
the same machine throughout, with the one-minute load average between **40 and
81**. That is recorded so the integrator can read the workspace numbers
correctly, and it is the direct cause of the first workspace run being abandoned
rather than reported: it was still in flight when three tests and a `cargo fmt`
pass changed the tree, so the numbers below are a second, complete run made
after the last edit. Measuring a superseded state and reporting it would have
been worse than re-running.

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

## 15. Artifacts

- `artifacts/keyword_census.txt` — the constraint-14 census, with its
  reproduction commands.
- `artifacts/composed.c` — the emitted C11 for the accepted fixture. Three
  things in it are the card, readable without the certificate: the composed
  payloads are `static const` arrays (`qcore_string_composed_0..2`), the borrow
  is `qcore_string_view v_text = { b_body, (size_t) 3u };` — a view of the
  `calloc`ed block itself — and `qcore_bytes_drop(b_body);` stands **before**
  the render of `v_framed`, because that composition had already laundered the
  buffer's bytes into `qcore_string_composed_2`. Only one of the fixture's
  literals survives as an array; the rest were folded into compositions at check
  time and the emitted program never names them.
