# Quirin string views via `std.string` — revision 0.3 amendment

> **Status: proposed.** This amendment specifies a future source-language
> facility. It does not claim parser, checker, reference-execution, C-emission,
> ABI, or conformance support in the current implementation.

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
The current compiler does not register `std.string`; the import and grammar
above are proposed requirements, not an executable example for this snapshot.

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

- `STRING-SLICE-LEX-TYPE`: `std.string` registration and per-file capability
  gating, literal decoding, type checking, keyword census, and diagnostic
  allocation.
- `STRING-SLICE-EXEC-C11`: reference semantics, `{ptr,len}` C11 emission, and
  zero-allocation evidence.
- `STRING-SLICE-BUFFER-BORROW`: checked UTF-8 borrowing/freezing from
  `std.buffer` with custody and mutation exclusion.
- `STRING-SLICE-IO`: explicit composition with formatting and output modules.
- `STRING-SLICE-ABI`: extent- and lifetime-carrying extern/native-library ABI.
- `BYTE-SLICE-CAP`: a separate mutable `{ptr,len,cap}` byte-slice profile; it
  must not retroactively add `cap` to immutable strings without ABI evidence.

## 10. Design source

The constant-time slice motivation follows the Hike description of a Go-like
byte-slice representation: <https://qiita.com/kanryu/items/95147e22ed5ac542ba58>.
This amendment narrows that representation for immutable text and adds the
UTF-8, ownership, failure, and C-boundary requirements needed by Quirin.
