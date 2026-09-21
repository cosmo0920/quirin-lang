# Quirin string views via `std.string` — revision 0.3 amendment

> **Status: partly implemented (cards `STRING-SLICE`, 2026-09-11, and
> `STRING-SLICE-IO`, 2026-09-12).** §§1–4, §7 and
> §8 are implemented as the checked source profile `source.string.v1`: the
> `std.string` module is registered and file-locally gated, a string literal is
> lexed and decoded to a well-formed UTF-8 payload, the `{ptr, len}` view is
> modelled and reference-executed, `len(s)` and `s[low:high]` are checked and
> lowered to §6's C11 aggregate, and the §8 evidence list is met at
> `docs/reports/string-slice/README.md`. The implementation decides every §2
> admission condition **statically** — the first of the two arms §2 permits —
> because every bound in v1 is a check-time constant; it therefore introduces no
> run-time failure arm, and a bound it cannot decide is refused by name.
>
> §7's **output composition is implemented** by card `STRING-SLICE-IO`
> (`Q075`–`Q076`, the same pass's second band): a module naming `std.string` and
> an output module may write `print TEXT;` and `log LEVEL TEXT;`, where `TEXT` is
> a quoted literal, a bound string or one slice of one, and the observation is
> the view's **exact UTF-8 bytes** — no re-encoding, no escaping, no terminator,
> and no newline beyond the framing every line already carries. A string takes
> **no** rendering selector, and one written in front of a string still refuses
> at `Q072`. Neither output module accepts text on its own: what changed is that
> a file naming **both** can render it, which is what §7 asked for. Evidence:
> `docs/reports/string-slice-io/README.md`.
>
> §5's **buffer borrow is implemented** by card `STRING-CONCAT`
> (`Q077`–`Q079`, the same pass's third band), together with §9's
> `STRING-SLICE-CONCAT`: a module naming `std.string` and `std.buffer` may write
> `let s = borrow_utf8(b);` to view an owned buffer's bytes as an immutable
> UTF-8 string — validity decided **statically**, out of the buffer's own write
> history, with a mutation-exclusion window that runs to the last statement
> reading the view or anything derived from it — and any module naming
> `std.string` may write `let s = concat(PIECE, …);` to compose two or more
> **check-time-known** pieces into a new static payload, one of which may be
> `dec(NAME)`/`hex(NAME)`, the text of a bound integer. A composition allocates
> nothing and copies nothing at run time; its length is the sum of its pieces'
> lengths and is a check-time constant, which is what makes a **computed
> `Content-Length`** decidable. Evidence:
> `docs/reports/string-concat/README.md`.
>
> Still **proposed and unimplemented**: §5's **freeze** (`STRING-SLICE-FREEZE`),
> §6's extern/native-library **crossing**, which stays refused
> (`STRING-SLICE-ABI`), §3.7's equality/ordering/hashing/search operations, and
> the separate mutable byte slice (`BYTE-SLICE-CAP`). No conformance claim is
> made: this is one bounded source profile, not revision-0.3 source-language
> conformance.

## 1. Decision

The opt-in bundled module `std.string` provides Quirin strings. A source file
MUST explicitly declare `import std.string;` to use its string type, construct
a string literal value, query its length, or slice it. These capabilities are
unavailable in the default language environment.

Quirin strings are immutable UTF-8 **views**, represented semantically by
`{ ptr, len }`. A string literal denotes a view of statically stored bytes.
Taking `s[low:high]` constructs another view by checked pointer offset and
length subtraction; it does not scan for a terminator, copy bytes, allocate,
or transfer custody. Consequently, `len(s)` and a successful slice operation
are O(1) in the number of bytes.

The Go/Hike `{ ptr, len, cap }` representation is adopted only for a future
**mutable byte slice**. An immutable string has no append operation and
therefore no semantic use for `cap`; putting capacity in every string would
increase its value and ABI size without enabling an operation in this profile.
The existing owned `std.buffer` value remains a distinct custody-bearing type.

## 2. Proposed source surface

```qcore
import std.string;
```

The import grants the string type and operations in this amendment for that
source file only, including its declarations and uncalled function bodies.
Repeated imports are idempotent. Imports in another compilation input, or of
`std.buffer`, `std.fmt`, `std.io`, `std.logger`, or `std.concurrency`, MUST NOT
grant string capabilities. Conversely, `std.string` grants none of those
modules' capabilities. Importing it does not authorize unsupported operations
or bypass the selected execution profile's checks.

An implementation claiming this amendment MUST refuse a string capability use
without the file's import. Recognizing quoted tokens for structural syntax or
existing declaration metadata, such as plugin versions, does not construct a
string value and MUST NOT acquire this new import requirement. Compiler support
for literal decoding and view lowering implements the module's semantics; it
does not make those semantics implicitly available as built-in capabilities.

This amendment reserves no word in the current implementation. An
implementation may expose the following grammar only after the repository's
keyword-collision census and the §L21 diagnostic allocation process:

```ebnf
string_import  = "import", "std.string", ";" ;
string_type    = "string" ;
string_literal = STRING-TOKEN ;
string_expr    = string_literal | NAME | NAME, "[", index, ":", index, "]" ;
string_query   = "len", "(", NAME, ")" ;
index          = INTEGER | NAME ;
```

The `string` type name and the string form of `len` are supplied by the module;
the spellings above do not establish universally available built-in names.

**Implemented (card `STRING-SLICE`), and the census decided how.** The compiler
registers `std.string` and exposes this grammar, and it **reserves no word**, as
this section requires:

- `len` is a **contextual** clause word, skipped by name resolution only where it
  is immediately followed by `(`. A global reservation was measured unsafe: `len`
  occurs in the pinned contract corpus as a *reference* inside
  `ensures normal resource_borrowed(len);`
  (`providers/shared/contracts/fluent_bit/resource_memory.qcore`,
  `providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qcore`). The `len(` shape occurs zero times
  in QCore source anywhere in the repository, so the narrow rule collides with
  nothing.
- `string` needs no rule at all: it appears only in a `let NAME : string = …;`
  ascription, and the reference collector already drains every token up to a
  `let`'s `=`. It occurs three times in `.qcore` fixtures, all in comments.
- `string_expr`'s single-index form is **not** implemented and has no meaning in
  v1: a byte of UTF-8 text is not a character, so the only indexing form is the
  two-bound slice. `NAME` alone denotes a view, as the grammar says.

The `STRING-TOKEN` and `NAME` productions are read from the existing lexer; the
implementation adds no token. The defined escapes are `\\`, `\"`, `\n`, `\t`,
`\r`, `\0` and `\xHH`; any other escape, and any payload whose decoded bytes are
not well-formed UTF-8, is refused rather than passed through. A payload is at
most 4096 bytes, refused rather than truncated above that.

`string` is a sequence of bytes known to be well-formed UTF-8. `len` and slice
indices count bytes, not Unicode scalar values or grapheme clusters. A slice is
admitted only when `0 <= low <= high <= len(s)` and both boundaries are UTF-8
code-point boundaries. Implementations may decide these conditions statically
or emit checked failure according to the enclosing effect/error profile; they
must not silently round an index, produce ill-formed text, or invoke undefined
behaviour.

## 3. Value and storage invariants

For every value `s = { ptr, len }`:

1. `len` is the exact byte count and is available without scanning.
2. `len == 0` permits a canonical non-null empty address or a null address;
   no operation may dereference it.
3. `len > 0` requires `ptr` to address at least `len` readable bytes.
4. `ptr[0..len]` is well-formed UTF-8 for the whole lifetime of the view.
5. The bytes are immutable through every authority reachable as a `string`.
6. A derived view cannot outlive the storage and custody from which it was
   derived. Creating a view neither creates nor discharges that custody.
7. Equality, ordering, hashing, and search, when added, are length-bounded byte
   operations; none may call `strlen` or read beyond `len`.

String literals have static storage duration. Their semantic byte sequence is
the decoded UTF-8 payload, excluding any C implementation terminator. Identical
literals may be pooled only when observable identity is absent; this profile
does not expose literal addresses.

## 4. Slicing rule

Given `s = { p, n }`, a successful `s[low:high]` evaluates to:

```text
{ ptr = p + low, len = high - low }
```

The operation performs no allocation and no copy. It preserves the source
view's storage provenance and read authority. The empty slice at `n:n` is
valid, subject to the no-dereference rule. Slicing is not ownership splitting:
overlapping views are allowed because strings are immutable, and dropping a
view never frees its backing storage.

## 5. Relation to owned buffers

`std.buffer` remains `{ identity, extent, custody }`, mutable and explicitly
released. It is not implicitly convertible to `string`. A future checked
conversion must choose and prove one of these operations:

- `borrow_utf8(buffer) -> string`: validates UTF-8, yields a view whose lifetime
  is bounded by the live buffer, and forbids mutation for that borrow; or
- `freeze_utf8(own buffer) -> own string`: validates UTF-8 and transfers the
  allocation into an immutable owned-string value.

Neither conversion is part of this amendment. A byte buffer's extent is not
evidence of UTF-8 validity or immutability.

**Implemented (card `STRING-CONCAT`), and the choice is the first of the two.**
`borrow_utf8(NAME)` is implemented and `freeze_utf8` is **not**, for a reason
this section can now state rather than leave open: an owned string is a new
custody shape, and v1's string value has nowhere to carry it. A view is
`{storage, offset, len}`; an owned one would need a fourth component for a
release to discharge, would make every slice of it a view of storage that can
end, and would need a `drop` on a *string*. The borrow reaches every byte the
freeze would, because a composition (§9's `STRING-SLICE-CONCAT`, landed by the
same card) already copies a borrowed view's bytes into static storage when a
program needs them to outlive the buffer. The follow-on is
`STRING-SLICE-FREEZE`, and what it must add is stated there.

Three rules the borrow rests on, each measured:

1. **The UTF-8 validation is static.** `source.buffer.v1` admits only constant
   bytes written at constant indices, so a buffer's contents are a check-time
   value and the validity question is decided by running a decision on it —
   before emission, with no run-time scan and no failure arm. That is the first
   of the two arms §2 permits, applied to the bridge. A buffer whose contents
   vary is the follow-on `STRING-SLICE-BORROW-VARYING`.
2. **The mutation exclusion is an interval of statement indices**, from the
   borrow to the last statement that reads the view or **anything derived from
   it**. A store into, or a release of, the buffer inside that interval is
   refused; one after it is admitted. The interval is exact rather than
   conservative because the route is one flat statement sequence: there is no
   branch to make a later index unreachable and no loop to make an earlier one
   run again.
3. **A borrow creates and discharges no custody.** It contributes an
   `OwnershipOp::Read` to the same cycle-3 operation list the buffer's
   acquisition and release contribute to, so borrowing a released buffer is that
   checker's own `UseAfterDrop` rather than a rule this card wrote.

**A borrow lends a range (card `CT07-SLICE-PERMISSIONS`).**
`borrow_utf8(NAME, LOW, HIGH)` views the bytes `[LOW, HIGH)`, and the
whole-extent spelling is that form at `[0, extent)` rather than a second
construct — measured as an identical certificate, an identical observation and
byte-identical emitted C. Rule 1 is unchanged except that the decision is made
on **the lent bytes**, which is why a range whose end splits a code point is
refused by the existing UTF-8 decision rather than by a new rule; rule 3 is
unchanged entirely. Rule 2 gains a second dimension and loses none: the
exclusion is still the same interval of statement indices, and **within** it a
write is refused when its byte lies inside a live loan and admitted when it lies
outside every one of them. That admission is the point — it is what makes two
disjoint permissions over one buffer expressible — and the disjointness is
decided rather than assumed, `max low < min high` being the set-theoretic
question itself because every bound here is a check-time constant. Three
asymmetries are deliberate. A **release** is refused for any live loan however
narrow, because it ends the storage all of them address. An index the route
cannot resolve to a constant is *not provably disjoint* and conflicts with every
live loan. And two overlapping **borrows** are not refused at all: both are
immutable views, they conflict on nothing, and a slice of a borrow is a fully
overlapping second borrow of the same buffer by construction. There is no
**write** lend — the owner keeps the write authority over every byte it did not
lend — which is why rule 2's exactness argument survives untouched and a `for`
beside a loan is still refused. The follow-on was `SLICE-PERMISSIONS-WRITE-LEND`,
landed at wave 29 **on the uncomposed buffer route** (`lend(b, lo, hi)`, write
authority genuinely transferred through the ownership IR's range-carrying
borrow node); on *this* composed route a lend still refuses by name until the
cross-model overlap rule with these read loans exists
(`SLICE-PERMISSIONS-COMPOSED-LEND`,
`docs/reports/slice-permissions-write-lend/README.md`), so every sentence above
stands here unchanged.

## 6. C11 lowering and ABI boundary

The internal C11 representation should be an aggregate equivalent to:

```c
typedef struct {
    const unsigned char *ptr;
    size_t len;
} qcore_string_view;
```

A literal may lower to a static byte array plus a constant length. A trailing
zero may be emitted for toolchain convenience, but it is outside `len` and is
not part of the language invariant. Internal operations consume the pair and
must not recover the length with `strlen`.

No existing C04 pointer contract transports this aggregate or its extent.
Crossing an extern/native-library boundary therefore remains refused until a
separate ABI profile specifies aggregate layout, target `size_t`, provenance,
lifetime, and whether the callee may retain the view. Interoperation with an
API requiring `const char *` is not implicit: it requires a checked adapter
that proves an in-bounds trailing NUL or allocates a terminated copy and records
that allocation's custody.

## 7. Effects, failures, and observations

Literal construction, `len`, and successful slicing have no `Alloc` or `Mut`
effect. Bounds failure, invalid UTF-8, conversion, and terminated-C adaptation
must be represented in the enclosing typed-error/effect rules; they may not be
hidden as traps or optimistic assumptions. Output modules may render a string
only after their own capability and observation rules are extended; this
amendment does not make `std.io` or `std.logger` accept text.

**Implemented (card `STRING-SLICE-IO`), and the extension is stated here.** An
output module's capability rule is unchanged — `std.io` alone still grants no
string and `std.string` alone still opens no stream — and what a rendering
module must now name is **both**. The observation rule is extended as follows,
and an implementation claiming this amendment must meet it exactly:

1. **The rendered observation of a string is the view's own bytes**, verbatim.
   No re-encoding, no escaping, no terminator, and no newline beyond the one the
   sink's line discipline already writes for every line — including the
   `[LEVEL] ` prefix a logged line carries, which is the same prefix a rendered
   scalar gets. A rendered scalar line and a rendered text line with the same
   body bytes MUST be the same line.
2. **A byte equal to `0` is an ordinary byte of the text**, inside `len`, and
   MUST be written. An implementation MUST NOT render a string through any
   operation that stops at a terminator; the write MUST be bounded by `len`, and
   a short write MUST fail the run rather than truncate the line.
3. **A string takes no rendering selector.** Selectors belong to the formatting
   module and are total functions from a scalar to text; a string is already the
   text. A selector applied to a string MUST be refused, and a module that
   renders only text MUST NOT be required to import the formatting module.
4. **Rendering carries the output effect and no other.** It allocates nothing
   and mutates nothing, and it neither creates nor discharges custody: a view is
   not a resource and publishes no identity, so a borrowed render of one has no
   meaning and MUST be refused.

Rendering text does **not** create a view of storage that can end, so §3.6 is
discharged exactly as before: every rendered view names the static literal it
addresses.

## 8. Required implementation evidence

An implementation claiming this amendment must provide positive and negative
tests for module import admission, missing imports for each string capability,
cross-file import isolation, independence from other bundled modules, and
preservation of quoted declaration metadata without the import. It must also
test literal decoding, embedded NUL bytes, empty strings, multibyte UTF-8
boundaries, bounds checks, zero-allocation slicing, lifetime/custody preservation,
and refusal at unsupported C boundaries. Reference execution and emitted C must
agree byte-for-byte on the declared observations. Existing programs and the
`source.buffer.v1` route must remain byte-identical, and no current `Unknown`
may become `Safe` merely because a token now parses as a string value.

## 9. Named implementation follow-ons

- ~~`STRING-SLICE-LEX-TYPE`~~: **landed** by card `STRING-SLICE`. `std.string`
  registration and per-file capability gating, literal decoding, type checking,
  keyword census (which reserved nothing) and diagnostic allocation — allocated
  as `Q065`–`Q070` and **relocated to `Q068`–`Q073` at integration** when
  `NATIVE-LIBRARY-SOURCE-BINDING` merged first and took `Q065`–`Q067`; a pure
  renumber, spec §L21.
- ~~`STRING-SLICE-EXEC-C11`~~: **landed** by the same card. Reference semantics,
  `{ptr, len}` C11 emission, and zero-allocation evidence — a certificate whose
  slice records carry the §4 arithmetic, and an emitted-program tripwire that
  finds no allocator, copier or scanner call.
- ~~`STRING-SLICE-BUFFER-BORROW`~~: **landed** by card `STRING-CONCAT`, as
  `borrow_utf8` only — see §5 for the three rules it rests on and for why the
  freeze is a separate follow-on. A module that holds a buffer *without*
  spelling the bridge still stays on the buffer route and keeps that route's
  refusal: the bridge construct is the whole of the route-selection change.
- `STRING-SLICE-FREEZE`: `freeze_utf8(own buffer) -> own string`. It needs a
  custody component on the string value, a release operation spelled on a
  *string*, and a lifetime rule for every view sliced out of one — none of which
  v1's `{storage, offset, len}` view can carry. The borrow plus a composition
  reaches every byte it would, so nothing is blocked on it.
- `STRING-SLICE-BORROW-VARYING`: a borrow of a buffer whose contents are not a
  check-time value. v1 decides UTF-8 validity statically out of the buffer's own
  write history, which is exact for the only buffers `source.buffer.v1` admits;
  a varying buffer needs a run-time validation and a checked failure arm, which
  is the second of the two arms §2 permits and the same wall
  `STD-BUFFER-VARYING-INDEX` names.
- `STRING-SLICE-VARYING-INDEX`: a slice bound that is not a check-time constant.
  v1 decides every bound statically and refuses anything else by name (`Q069`);
  admitting a varying bound needs a relational domain over indices and a checked
  failure arm, which is the second of the two arms §2 permits.
- `STRING-SLICE-NOTASCALAR-REACH`: a recorded, unrepaired measurement rather than
  a gap to close. `Q069`'s "not a check-time constant" arm cannot be reached from
  a single source module — name resolution refuses an unbound name at `Q013`
  first — so the guard is checked through the library instead. Kept rather than
  deleted, on the `EXTERN-F1-STORE-REACH` precedent.
- ~~`STRING-SLICE-IO`~~: **landed** by the card of the same name. Explicit
  composition with the output modules, `Q075`–`Q076`, with the four observation
  rules of §7 above. What it did **not** do, each recorded rather than implied:
  it supplies no formatting of a string (there is nothing to format), no
  conversion between a string and a scalar in either direction, and no way to
  build a string at run time — a rendered payload is a literal or a slice of
  one, so a program can render text it wrote and not text it computed
  (`STRING-SLICE-CONCAT`).
- `STRING-SLICE-ABI`: extent- and lifetime-carrying extern/native-library ABI.
- ~~`STRING-SLICE-CONCAT`~~: **landed** by card `STRING-CONCAT`, and the
  dependency this entry recorded was **dissolved by measurement rather than
  discharged**. It read: "It needs a building operation, which needs storage a
  program owns, which is `STRING-SLICE-BUFFER-BORROW`'s question first." That
  premise is false for the operation v1 wants. Every piece a `concat` can take is
  check-time-known, so the *compiler* holds the composed bytes and the result is
  a new **static** payload of the emitted program — storage no program owns, no
  allocation, no copy at run time, no custody and no lifetime rule. The two
  operations landed together because one card was assigned both, not because
  either needed the other.

  The surface is `let NAME = concat(PIECE, …);` over two or more pieces, each a
  quoted literal, a string this entry bound, one inline slice of one, or
  `dec(NAME)`/`hex(NAME)` for the **text of a check-time-constant integer** —
  the one scalar-to-text direction this profile has, producing exactly the bytes
  `print dec NAME;` writes because both go through one rendering function. The
  result's length is the sum of its pieces' lengths and is itself a check-time
  constant, which is precisely what makes a `Content-Length` a program *derives*
  as decidable as one it writes.

  The alternative was measured and named rather than left implicit:
  materialising into an owned buffer at run time would add an allocation, a copy
  and a custody obligation to an operation whose result the compiler already
  holds, and would make `len` of the result a run-time quantity — the one
  property the operation exists to preserve. It is the follow-on
  `STRING-SLICE-CONCAT-RUNTIME`, and it becomes necessary exactly when a piece
  is not check-time-known.
- `STRING-SLICE-CONCAT-RUNTIME`: a composition with a piece the compiler cannot
  evaluate. It needs storage the program owns, an `Alloc` effect the row states
  honestly, a release, and a length that is no longer a constant.
- `BYTE-SLICE-CAP`: a separate mutable `{ptr,len,cap}` byte-slice profile; it
  must not retroactively add `cap` to immutable strings without ABI evidence.

## 10. Design source

The constant-time slice motivation follows the Hike description of a Go-like
byte-slice representation: <https://qiita.com/kanryu/items/95147e22ed5ac542ba58>.
This amendment narrows that representation for immutable text and adds the
UTF-8, ownership, failure, and C-boundary requirements needed by Quirin.
