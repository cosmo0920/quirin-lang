# NATIVE-LIBRARY-HEADER-DECORATION — the export macro, derived — 2026-09-11

Card `NATIVE-LIBRARY-HEADER-DECORATION` of wave 23: making a **decorated**
exported header readable, without creating an unchecked premise about its text.

**Base tree:** main `52e3889c`. Parent record:
[`wave22-native-library-header`](../wave22-native-library-header/README.md),
whose §6 census measured **0 of 423** Libevent declarations inside the v1 subset
and §11 named this card, its bar and the design it declined. Design:
[`NATIVE_LIBRARY_HEADER_V1.md`](../../design/NATIVE_LIBRARY_HEADER_V1.md);
spec §L14.4.1 (new) and grammar §G02d.2.

---

## Headline

**A manifest says which texts may be read. The reader says what they mean.**

Card `NATIVE-LIBRARY-HEADER` could not read a single declaration of Libevent's
423, because every one is written

```c
EVENT2_EXPORT_SYMBOL
struct event_base *event_base_new(void);
```

and that macro's `#define` lives in `event2/visibility.h`, a header the reader
did not follow. That card named the obvious repair and refused to take it:

> a manifest-declared erasure list converts a *reader boundary* into an
> **unchecked premise about the header's own text**, which is precisely what
> this level exists to remove; any successor must argue it.

This card does not take it either. One repeatable manifest statement declares a
hash-identified text the header's own `#include` may be read from, and the
reader **derives** from that text's own `#define`s that `EVENT2_EXPORT_SYMBOL`
is an erasable declaration decoration — evaluating none of the four conditional
arms it is defined in.

| | |
|---|---|
| Manifest delta | one repeatable statement, `include SPELLING from REFERENCE HASH`; **requires** a `header`, declaring none is the default and is byte-identical to wave 22 |
| What a manifest may say | *which* texts. Nothing about their contents, and in particular **not** which names may be erased (§2) |
| What the reader derives | erasable decoration names, from the declared texts' own `#define`s, under four conditions, evaluating **no** condition (§3) |
| The vocabulary | the reader's own, published in `ERASABLE_ATTRIBUTES`: 16 attribute names that change neither the declared type, the parameter vector nor the linkage name. Everything else refuses **by name** (§3.2) |
| Libevent census | **0 → 264 of 423** read, on **one** declared text; unreadable 406 → 116 (§5) |
| Fail-closed, measured | declaring `event2/util.h` refuses seven of the ten headers *as files* rather than reading them in part — it carries `#error` (§5.2) |
| A finding, repaired | a prefix decoration ending in `)` stopped the specifier walk **silently**, so `__attribute__((mode(__DI__)))` was ignored rather than refused (§4) |
| Diagnostics | **no new code.** `Q063` extends; a second name for one fact is a claim a reader cannot check (§7) |
| New reserved words | **none.** A manifest is not source and neither is a header |
| `qcore-schemas` | **untouched at 14.0.0** — no provider rebuild |
| Corpus | **untouched.** No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/qcore/` or `providers/` is in this diff |
| Evidence | 22 new core unit rows, 22 new route rows, 1 new CLI process row, and **25 audited Lean theorems** in a new `QCore.NativeLibraryHeaderDecoration`, no new axiom |

**Nothing about a verdict is promoted.** The derivation widens what this reader
can **read**; the comparison it feeds is card `NATIVE-LIBRARY-HEADER`'s,
unchanged, and a disagreement still refuses at `Q064`. No raw Cycle 19 unit
moves, no CT mapping changes, the contract corpus is untouched, and full
language conformance stays false.

---

## 1. What the statement is

```text
header include/qcore_buffer_demo_decorated.h;
header_hash sha256:88d5ce10…8f6e;
include "qcore_buffer_demo_visibility.h" from include/qcore_buffer_demo_visibility.h sha256:8fce29e5…1c0b;
```

Three parts, and keeping them apart is load-bearing.

* The **spelling** is the `#include` operand exactly as the header writes it,
  delimiters and all, because that is the token a reader matches on. `<x.h>` and
  `"x.h"` are two different `#include` lines and are two different declarations
  here.
* The **reference** is where the text is resolved from, in the same namespace as
  `header` — so the transport is the one `native_library_headers` map, no new
  `CompileOptions` field, and `quirin` resolves it against the manifest's own
  directory with one candidate path and no search (`HEADER-CLI-RELATIVE`,
  extended).
* The **digest** is measured over the bytes and verified before any of the text
  is read.

`from` is the contract slice entry's keyword, reused: a statement naming two
references says which is which rather than relying on position.

Totality, in the three directions that are checkable:

* a declared text must be **supplied** (`IncludeNotSupplied`);
* it must **measure its declared digest**, decided before it is read
  (`IncludeDigestMismatch`);
* it must be **reached** by an `#include` of its spelling, in the header or in a
  text reached from it (`IncludeNotReached`) — a declared text nothing reads is
  a statement with no effect, and this level's content is that its statements
  have effects.

And one on the transport: once the selection binds a header, a text supplied for
a reference **no manifest declares** refuses (`SourceNotDeclared`). The gate is
on a header being bound so that wave-22's
`a_manifest_naming_no_header_is_unaffected_by_supplied_texts` stays true, and
that row is re-asserted here rather than assumed.

The fourth direction is deliberately **absent**: an `#include` the manifest does
not declare is **not followed**, and nothing about that changed. Refusing on
every undeclared `#include` would make the level unusable on any header that
writes `#include <stdint.h>` — which is every real one, including this card's own
fixture. Pinned in both crates
(`an_undeclared_include_is_not_followed_and_nothing_about_that_changed`,
`an_include_the_manifest_does_not_declare_is_not_followed`).

## 2. The argument: why this is not the design that was declined

The declined design and this one differ in exactly one place, and it is the
place the whole objection lives in.

> A manifest-declared erasure list says **"`EVENT2_EXPORT_SYMBOL` means
> nothing"** — a claim about the header's text, made by the manifest author,
> that nothing checks.
>
> This card's manifest says **"`<event2/visibility.h>` is `sha256:…` and the
> header may read it"** — a claim about *which file*, which the digest check
> discharges completely.

Everything that follows the second statement is a **reading**:

1. the reader splices that text and finds every `#define` of every name in it;
2. it concludes a name is an erasable decoration only if **every** definition is
   object-like and has a body in the reader's own published vocabulary, the name
   is never `#undef`ed, and it is defined on **every path** through the
   conditional structure preceding its first use;
3. and it erases the name, which — under exactly those conditions — *is*
   expanding it.

Three properties make that honest rather than a re-labelling.

**The manifest cannot reach the conclusion.** There is no manifest statement
whose value changes which names are derived. The derivation's inputs are the
supplied texts, and Lean's `the_derivation_is_a_function_of_the_text` says so;
the route row
`the_derivation_names_exactly_the_decoration_and_reads_it_from_the_declared_text`
additionally asserts that no *statement* of the fixture manifest so much as
contains the token that got erased.

**The vocabulary is not a claim about this header.** `ERASABLE_ATTRIBUTES` is 16
names, fixed, in this repository, reviewable independently of any library, and
it is the same kind of commitment `HEADER-STDINT-PRELUDE` already made to C11
7.20: a reading of a *language*, not of a *file*. The manifest binds no name to
it; the header's own `#define`s do.

**The erasure is checkable against the expansion.** The vocabulary erases a
decoration whether it arrived through a derived macro name or was written out in
full, in either position of a declaration, so "deleting the name" and "expanding
the name and deleting what came out" are literally the same token stream. That
is a row, not a claim: `erasing_the_name_is_expanding_it` reads the fixture
header four times, once per expansion its visibility text actually contains, and
requires all four readings equal to the macro form. In Lean it is
`expanding_then_erasing_equals_erasing` and
`erasing_the_name_is_expanding_it`.

What is **left as a premise** is exactly what wave 22 already carried and named:
that the supplied bytes are the text the reference names. It is discharged the
same way, by digest, and the certificate publishes every one of those identities
(§6).

So the declined design's objection does not apply, and the reason is not that it
was overruled — it is that the claim it would have made is now derived.

## 3. The derivation

### 3.1 Four conditions, no condition evaluated

A name is an erasable declaration decoration when all four hold, and each is
read out of the declared texts:

| # | Condition | Why it is there | Refusing row |
|---|---|---|---|
| 1 | every `#define` of it is **object-like** | a function-like macro's expansion depends on its arguments at each use site, and this reader evaluates no expansion | `a_function_like_definition_is_not_derivable` |
| 2 | every one of those bodies is in the erasure vocabulary, or is a name already derived | see §3.2 | `an_unerasable_body_is_not_derivable`, `one_unerasable_definition_withdraws_the_derivation` |
| 3 | it is never `#undef`ed, anywhere | this reader tracks no scope for a macro, and the conservative reading is the refusing one | `an_undefined_name_is_not_derivable` |
| 4 | it is defined on **every path** through the conditional structure preceding its first use | §3.3 | `a_group_with_no_else_is_never_derivable`, `an_arm_that_does_not_define_withdraws_the_group` |

Condition 2 is a **least fixed point**, so `#define A B` stands on a `B` already
derived — which is how a real library writes its second export macro.

### 3.2 The vocabulary, and the names deliberately absent

```
visibility  dllexport  dllimport  deprecated  unused  used
warn_unused_result  malloc  nonnull  format  always_inline
noinline  noreturn  nothrow  hot  cold
```

plus the empty body, the bare `__global` keyword, and chaining. The one property
that admits a name is that it changes **neither the declared type, nor the
parameter vector, nor the linkage name** of the declaration it decorates.

The names that are **not** in it are the argument that this is a vocabulary and
not a shrug:

* `mode`, `aligned`, `packed`, `vector_size`, `transparent_union` — change the
  **type**;
* `ms_abi`, `sysv_abi`, `regparm`, `stdcall`, `cdecl`, `fastcall` — change the
  **calling convention**;
* `alias`, `weak`, `weakref`, `constructor`, `destructor` — change **what the
  artifact exports under this name**.

An attribute list carrying any name outside the whitelist is **not** erased, and
the declaration keeps the refusal it already had. The unit row
`every_body_must_be_erasable_for_the_name_to_be` puts each of seven such
poisons into each of five conditional arms in turn — 35 combinations, all
withdrawing — and `the_erasable_attribute_vocabulary_is_closed_and_pinned` is
the positive direction over all fourteen attribute names.

*Argument nesting.* Names are read at the attribute-list depth of their
introducer — two for `__attribute__ ((…))`, one for `__declspec (…)` — so
`format(printf, 2, 3)` reads `format` as the attribute and `printf` as its
argument, which is what Libevent's `evbuffer_add_printf` actually writes.

### 3.3 All-paths definedness, which is what makes it a reading

This is the condition the card turns on. Libevent's `EVENT2_EXPORT_SYMBOL` is
defined five times, in a nested structure the reader decides nothing about:

```c
#if defined(event_shared_EXPORTS) || …
# if defined (__SUNPRO_C) …
#  define EVENT2_EXPORT_SYMBOL __global
# elif defined __GNUC__
#  define EVENT2_EXPORT_SYMBOL __attribute__ ((visibility("default")))
# elif defined(_MSC_VER)
#  define EVENT2_EXPORT_SYMBOL __declspec(dllexport)
# else
#  define EVENT2_EXPORT_SYMBOL /* unknown compiler */
# endif
#else
# define EVENT2_EXPORT_SYMBOL
#endif
```

The reader evaluates none of `event_shared_EXPORTS`, `__SUNPRO_C`, `__GNUC__` or
`_MSC_VER` — it cannot, those are the consuming build's. What it decides is
**structural**: a group establishes definedness only when it wrote an `#else`,
so that no implicit empty arm escapes, and only when every one of its arms
establishes it. Both groups above qualify. So whichever arm a consumer selects,
the name is defined and its expansion is erasable — and therefore deleting the
name is the expansion, without knowing which one.

A group with no `#else` fails, and the measurement shows that is not a
hypothetical: `EVENT2_CORE_EXPORT_SYMBOL`, defined in the same file, is **not**
derived, because its `#if defined(_MSC_VER)` group writes none
(`the_libevent_derivation_names_exactly_the_one_export_macro`).

### 3.4 Splicing

A declared text is spliced where its `#include` stands, after its whole-file
include guard is removed — the guarded body *is* the file on this reader's
one-pass reading, which is wave 22's own argument, and removing it is what puts
the body at the depth of the `#include` rather than one level deeper. Each
declared text is spliced **at most once**, which is what an include guard does
and what keeps a cycle finite.

Splicing is not a macro table: the text becomes part of the one text the reader
reads, so a `typedef` one file away stops being unreadable
(`a_spliced_texts_typedefs_are_read_as_the_headers_own`) and a spliced text's
directives are read under the same bounded subset — its `#error` refuses the
whole reading rather than half of it
(`a_spliced_text_is_read_under_the_same_directive_subset`). §5.2 is that rule
meeting a real header.

### 3.5 Why the conditional route was not needed, and not taken

The card's brief was to combine declared includes with *enough of*
`NATIVE-LIBRARY-HEADER-CONDITIONAL` to decide the decoration macro. The
measurement says none was needed: the derivation is structural, so it decides
`EVENT2_EXPORT_SYMBOL` without evaluating anything, and the census reaches its
ceiling (§5.1) on the include route alone.

It was then not taken, because the conditionals that remain are the ones a
reader **cannot** decide honestly. `event2/thread.h` is 8 of 9
conditional-only, behind `EVENT__DISABLE_THREAD_SUPPORT` and
`EVENT__HAVE_PTHREADS` — values of the *library's own build configuration*,
which no text in the declared set contains. Deciding them would require assuming
a configuration, which is the unchecked premise this card exists to avoid, one
level up. `NATIVE-LIBRARY-HEADER-CONDITIONAL` stays open with that boundary
recorded.

## 4. A finding, in passing

The specifier prefix is read by walking backwards from the declarator name over
identifiers, `*` and qualifiers. A decoration ends in `)`, which is not one of
those, so the walk stopped there — **silently**. Before this card,

```c
__attribute__((mode(__DI__))) uint8_t *qcore_extern_alloc(uintptr_t size);
```

was read as `uint8_t *`, with the attribute that changes the declared type simply
not in the prefix. The same hole was open in the *suffix* position, where
`SUFFIX_INTRODUCERS` stepped over any parenthesised group after the declarator.

Both are repaired in the refusing direction, by the one vocabulary: the erasable
decorations are deleted from the token stream before the prefix is read, so
anything still standing there is a decoration this reader does not commit to, and
it makes the declaration unreadable by name. `__attribute__` and `__attribute`
left `SUFFIX_INTRODUCERS` for the same reason. Pinned by
`a_prefix_decoration_outside_the_vocabulary_no_longer_passes_unread` and
`a_decoration_that_changes_the_type_is_not_erased`.

**`HEADER-ASM-LABEL-UNREAD`** is the residual left behind: `__asm__("name")` is
still stepped over unread, and an asm label renames the exported symbol — so a
manifest binding the source name would be naming a symbol the artifact does not
export. It is recorded rather than repaired (follow-on
`NATIVE-LIBRARY-HEADER-ASM-LABEL`), because tightening it is a separate
strengthening with its own rows and no census evidence in this corpus: no
Libevent or ChunkIO header in the measured set writes one.

## 5. The census, over the pinned Libevent submodule

Measured by the **shipped reader** over `third_party/libevent` at the pinned
revision `d82464a`, with the same denominator card `NATIVE-LIBRARY-HEADER` used
(`census_candidates`: every identifier the text writes immediately before a `(`).
Raw rows: [`artifacts/header_census_decoration.txt`](artifacts/header_census_decoration.txt).

### 5.1 One declared text

| Header | candidates | read before | **read after** | unreadable after | unspellable after | conditional | variadic | absent |
|---|---|---|---|---|---|---|---|---|
| `event2/event.h` | 70 | 0 | **53** | 10 | 6 | 1 | 0 | 0 |
| `event2/http.h` | 122 | 0 | **88** | 31 | 2 | 0 | 0 | 1 |
| `event2/bufferevent.h` | 65 | 0 | **43** | 20 | 1 | 0 | 0 | 1 |
| `event2/dns.h` | 42 | 0 | **28** | 6 | 7 | 1 | 0 | 0 |
| `event2/buffer.h` | 56 | 0 | **25** | 21 | 5 | 2 | 1 | 2 |
| `event2/rpc.h` | 25 | 0 | **17** | 8 | 0 | 0 | 0 | 0 |
| `event2/listener.h` | 9 | 0 | **4** | 2 | 3 | 0 | 0 | 0 |
| `event2/watch.h` | 5 | 0 | **3** | 0 | 2 | 0 | 0 | 0 |
| `event2/tag.h` | 20 | 0 | **2** | 18 | 0 | 0 | 0 | 0 |
| `event2/thread.h` | 9 | 0 | **1** | 0 | 0 | 8 | 0 | 0 |
| `event2/util.h` | — | **the whole file refuses**: it carries `#error`, which the subset does not interpret | | | | | | |
| **total (files read)** | **423** | **0** | **264** | **116** | **26** | **12** | **1** | **4** |

The declared set is **one text**, `<event2/visibility.h>`, and the derivation
names **one** erasure, `EVENT2_EXPORT_SYMBOL`. Nothing else about the reader
changed.

*Two cells of the "before" column moved* from card `NATIVE-LIBRARY-HEADER`'s
archived row for `event2/buffer.h`: conditional 4 → 2 and absent 0 → 2. The cause
is §4's repair reaching the unread conditional stream: `printf`, inside
`__attribute__((format(printf, 2, 3)))` in an `#ifdef __GNUC__` region, was
counted a conditional-only *declaration* and is now correctly not one. The
archived wave-22 artifact is **not edited** (constraint 21); the delta is
recorded here and in the new artifact's own header.

### 5.2 What the remaining 116 are, and the fail-closed measurement

The dominant remaining reasons are `evutil_socket_t` (16), `ev_ssize_t` (23),
`ev_off_t` (6), `ev_uint16_t`/`ev_uint32_t`/`ev_uint64_t` (29 across the set) —
and all of them are `typedef`s of **`event2/util.h`**.

Declaring it does not read them. `util.h` carries `#error`, so splicing it
refuses **every header that includes it, as a file**: seven of the ten rows above
become `FILE-REFUSED`. That is the correct disposition — a reader that half-read
a text the compiler never sees would be reporting on something else — and it is
pinned as a row
(`declaring_a_text_outside_the_directive_subset_refuses_the_headers_that_read_it`),
because a fail-closed boundary that is only described is not a boundary.

So the honest statement of this card's reach is: **the export decoration was the
whole of the first barrier, and `#error` plus build-configuration conditionals
are the second.** The second is `NATIVE-LIBRARY-HEADER-CONDITIONAL`'s, and §3.5
says why it is not this card's.

## 6. What the certificate says

The residence premise's header identity gained the spliced texts, as a field of
the record it already carried:

> `qcore_extern_alloc` behaves as its selected contract states
> **— and —** it is read through the declared contract `…/buffer_lib.qcore`,
> from artifact `libqcore_buffer_demo.a` … under stated ABI premises
> **— and —** its exported header `include/qcore_buffer_demo_decorated.h` was
> **read** at `sha256:88d5…8f6e`, **reading**
> `include/qcore_buffer_demo_visibility.h` (`"qcore_buffer_demo_visibility.h"`)
> at `sha256:8fce…1c0b`, and every bound declaration was compared against it —
> those digests are verified, not declared.

Naming the spliced text is not decoration of the certificate. The reading of a
decorated declaration is a function of *that text's* bytes as much as of the
header's, so a premise naming only the header would publish a decision a recheck
could not reproduce — which is card `NATIVE-LIBRARY-HEADER` §3's argument,
applied to the second text for the same reason. Pinned in the route suite and in
the CLI process row; the headerless and include-less renderings are
byte-unchanged.

## 7. Diagnostics: no new code, on purpose

`Q063` — "the header binding does not load" — absorbs all four new refusals and
the underivable decoration, because they **are** that condition, reached by a new
route. Allocating a code for them would put two names on one fact, which is
`CONTRACT-SLICE-WAIVER-MIRROR`'s discipline in the other direction: a reader who
saw `Q063` and a second code would be entitled to think they meant different
things.

This also means the card claims **no band** and cannot collide with any other
wave-23 card's allocation. `Q064` is untouched: a disagreement still refuses,
with the same message, from the same comparison.

## 8. The two readers of §7 — extended in **neither**, and why

Card `NATIVE-LIBRARY-HEADER` §7 records two readers of one projection that must
both reproduce every decision, and requires a successor to extend both or
neither. This card extends **neither**, by measurement, and the divergence is
recorded rather than assumed away.

The gateway's declared-ABI reader's input is **preprocessed** — Clang has already
run and it never sees a `#`. That is §7's divergence 0, and everything about this
card follows from it: there is no macro to derive and no `#include` to follow on
that side. What *can* appear there is the **expansion**: in a shared build,
Libevent's preprocessed text carries
`__attribute__ ((visibility("default")))` in front of every declaration, which
the gateway reader refuses. In the default (static) build it expands to nothing
and the two readers agree exactly.

So the divergence is real, it is in the **refusing** direction on the gateway's
side, and it cannot make the gateway admit anything this reader refuses. It is
recorded as **divergence 5**, `HEADER-DECORATION-GATEWAY-DIVERGENCE`.

**Why not extend the gateway anyway.** Its declared-ABI reader is the
`LOADER-ABI` withdrawal gate of standing constraint 8. Widening what it reads can
*create* withdrawals where absence of evidence currently withdraws nothing, and a
withdrawal can turn a `Safe` unit into a provisional `Unsafe`. That is a
corpus-moving change, this card is corpus-neutral by assignment, and the pinned
Cycle 19 build inputs are not present in this worktree, so the parity measurement
invariant 3 would require **cannot be taken here**. Doing it blind would be
exactly the kind of unmeasured widening the rest of this card argues against.
Follow-on `NATIVE-LIBRARY-HEADER-GATEWAY-DECORATION`, which must land with a
corpus re-measurement beside it.

## 9. What did not move — pinned in both directions

* **Declaring no include text is wave 22, byte for byte.** The wave-22 route,
  unit and CLI suites are untouched apart from one added row each, and
  `a_manifest_declaring_no_include_is_the_wave_22_binding` compiles the wave-22
  fixture through the new code and asserts an empty `includes` list on the
  published premise.
* **A manifest naming no header is still unaffected by supplied texts**, which is
  the wave-22 row the new transport totality could have broken. It is re-asserted
  under the new gate rather than assumed
  (`a_manifest_naming_no_header_is_still_unaffected_by_supplied_texts`).
* **The comparison is unchanged.** A read declaration is compared by card
  `NATIVE-LIBRARY-HEADER`'s relation and a disagreement still refuses at `Q064`.
  Lean's `erasure_preserves_every_code_token` says the erasure deletes no token
  the comparison reads.
* **Waves 18–22 pass unchanged**: `compiler/tests/native_library.rs` (16),
  `native_library_contract.rs` (15), `unsafe_linkage.rs` (28) and
  `native_library_header.rs` (17) are untouched, and the 49 wave-22 core unit
  rows are untouched. One wave-22 **fixture** changed: the comment block of
  `include/qcore_buffer_demo_decorated.h`, whose forward reference to this card
  is now a record that it landed. No declaration in it moved, and the wave-22
  row that reads it measures its digest at run time, so nothing pinned shifted.
* **New reserved words: none** (constraint 14). `include` and `from` are manifest
  statement keywords; a manifest shares no lexer with source, which is why its
  grammar lives in §G02d.2 and not §G02d. A header is C, read by a reader of its
  own. No token was added to the source grammar.
* **`qcore-schemas` stays 14.0.0.** Constraint 18 is not engaged; no provider
  package changed or needed to.
* **Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
  `crates/qcore-audit/`, `crates/qcore-contract/`, `contract/qcore/` or
  `providers/` is in this diff. The 72-module corpus pin and the Cycle 19
  artifacts are untouched. The corpus baseline stays
  `docs/reports/wave23-c-operations/artifacts/`.
* **No `CompileOptions` field was added.** The declared texts travel in the
  existing `native_library_headers` map, keyed by the same kind of reference,
  because they are the same kind of input: a text this level reads and identifies.

## 10. Lean

`proof/QCore/NativeLibraryHeaderDecoration.lean` (new) +
`proof/AuditNativeLibraryHeaderDecoration.lean` (new, registered), imported into
`proof/QCore.lean`. **25 theorems audited, no new axiom** (only `propext`,
`Classical.choice`, `Quot.sound`).

* **Erasure is expansion** — `expanding_then_erasing_equals_erasing`,
  `erasing_the_name_is_expanding_it`, `erase_keeps_exactly_the_code`,
  `erase_append`, `an_empty_body_is_erasable`.
* **Only the readable set widens, never the admitted one** —
  `an_underived_name_survives`, `erasure_preserves_every_code_token`,
  `erasure_is_idempotent`.
* **No condition is evaluated** — `a_group_with_no_else_establishes_nothing`,
  `a_group_with_no_else_is_never_derivable`,
  `an_arm_that_does_not_define_withdraws_the_group`.
* **Every definition must qualify** —
  `a_function_like_definition_is_not_derivable`,
  `an_unerasable_body_is_not_derivable`,
  `one_unerasable_definition_withdraws_the_derivation`,
  `an_undefined_name_is_not_derivable`.
* **Non-vacuity, and the manifest's absence from the decision** —
  `the_real_visibility_shape_is_derivable`,
  `the_derivation_is_a_function_of_the_text`.
* **The declared texts** — `an_unsupplied_declared_text_refuses`,
  `a_substituted_declared_text_refuses`, `an_unreached_declared_text_refuses`,
  `an_undeclared_source_refuses`, `declaring_no_text_binds_trivially`,
  `a_declared_supplied_and_reached_text_binds`.

**Stated boundary of the Lean model.** That each name in `ERASABLE_ATTRIBUTES`
really is a decoration contributing no type, no parameter and no linkage name is
a reading of **C**, published and auditable there — not a theorem. It is the same
kind of commitment `HEADER-STDINT-PRELUDE` made to C11 7.20, and the model takes
it as the classification of a token rather than deriving it. The concrete lexical
layer — splicing, guard removal, tokenising — is a Rust boundary tested there.

## 11. Named follow-ons

| Name | What is open |
|---|---|
| `NATIVE-LIBRARY-HEADER-CONDITIONAL` | **sharpened by this card's measurement.** The conditionals that remain are the library's own build configuration (`EVENT__DISABLE_THREAD_SUPPORT`, `EVENT__HAVE_PTHREADS`) and `#error`; no declared text contains their values, so deciding them is assuming a configuration. `event2/thread.h` is 8/9 conditional-only and `event2/util.h` refuses as a file |
| `NATIVE-LIBRARY-HEADER-ASM-LABEL` | `__asm__("name")` is stepped over unread; an asm label renames the exported symbol, so a manifest binding the source name names a symbol the artifact does not export (`HEADER-ASM-LABEL-UNREAD`) |
| `NATIVE-LIBRARY-HEADER-GATEWAY-DECORATION` | §8's divergence 5. The gateway's declared-ABI reader refuses a declaration still carrying an expanded decoration. Extending it moves the `LOADER-ABI` withdrawal gate and must land with a Cycle 19 corpus re-measurement beside it |
| `NATIVE-LIBRARY-HEADER-INCLUDE-SEARCH` | the declared set is closed and each text is named individually; a library with fifty exported headers must name fifty texts. Whether a *directory* may be declared instead, and what identity such a declaration could carry, is open |
| `NATIVE-LIBRARY-HEADER-FNPTR` | unchanged, and now the **26** unspellable rows of §5.1 are measured rather than hypothetical: they were unreachable behind the decoration before this card |
| `NATIVE-LIBRARY-HEADER-ONE-READER` | unchanged from wave 22; §8 adds one divergence to its census |
| `NATIVE-LIBRARY-SOURCE-BINDING`, `NATIVE-LIBRARY-CERT-REPLAY`, `UNSAFE-LINKAGE-ARTIFACT-PROBE`, `NATIVE-LIBRARY-MULTI-HASH`, `NATIVE-LIBRARY-SLICE-SHAPE`, `UNSAFE-LINKAGE-CLI` | unchanged. `-CERT-REPLAY` now has a third kind of identity to replay against, and `-MULTI-HASH` covers a third and repeatable hash |

`NATIVE-LIBRARY-HEADER-DECORATION` itself is closed.

## 12. Integrator reconciliation

1. **No diagnostic band is claimed.** `Q063` is extended in place and no code was
   allocated, so this card cannot collide with any other wave-23 allocation and
   needs no `T07`/`T09` relocation. Sites touched:
   `docs/QCORE_SPEC_r0_3.md` §L21 (both copies of the `Q063`–`Q064` row) and the
   new §L14.4.1, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2,
   `crates/qcore-frontend/src/frontend.rs` (`SemanticPass::NativeLibrary` doc).
   No emission site changed.
2. **Manifest grammar, and the collision to watch.** One repeatable statement
   keyword, `include`, is added to the manifest grammar. Card
   `NATIVE-LIBRARY-SOURCE-BINDING` owns manifest **binding forms** this wave; if
   it also adds a statement keyword, check for a clash on `include` — nothing
   else of this card's grammar is contestable, and the `from` tail is reused from
   the existing contract slice entry rather than introduced.
3. **`HeaderIdentity` gained a public field** (`includes: Vec<IncludeIdentity>`),
   and `IncludeIdentity` is new and public. Every in-tree construction site is
   updated. An out-of-tree consumer constructing `HeaderIdentity` as a literal
   will need the field.
4. **`docs/reviews/r0_3/acceptance_matrix.json`** is integrator surface and is not
   edited here. `NATIVE-LIBRARY-HEADER-DECORATION` should be retired from
   `named_followups` and the four new follow-ons of §11 entered
   (`-ASM-LABEL`, `-GATEWAY-DECORATION`, `-INCLUDE-SEARCH`, and the sharpened
   `-CONDITIONAL` left in place). The derived `lean_audits` row must grow by one
   (`AuditNativeLibraryHeaderDecoration`), which is the expected `scripts/tests`
   drift below.
5. **Ledger.** `AGENTS.md`'s headline test/Lean numbers and `DEVELOPMENT.md`'s
   increment record are integrator surface and are not edited here. Lean audits
   go **95 → 96**.
6. **The wave-22 census artifact was not edited** (constraint 21). §5.1 records
   the two-cell delta and this card's artifact carries its own provenance header.
7. **No provider rebuild.** `qcore-schemas` is 14.0.0 and no manifest template
   changed.

## 13. Gate

| Check | Result |
|---|---|
| `cargo test` over all ten workspace members | GATE_TOTAL |
| `crates/qcore-core` unit `native_library_header` | **71 passed** (49 wave-22, unchanged; 22 new) |
| `compiler/tests/native_library_header_decoration.rs` (new) | **22 passed** |
| `crates/qcorec/tests/cli.rs` | 9 passed (8 + `the_cli_reads_the_include_texts_a_manifest_declares`) |
| `compiler/tests/native_library_header.rs` (wave 22) | 17 passed, unchanged |
| `compiler/tests/native_library_contract.rs` (wave 20) | 15 passed, unchanged |
| `compiler/tests/native_library.rs` (wave 19) | 16 passed, unchanged |
| `compiler/tests/unsafe_linkage.rs` (wave 18) | 28 passed, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 96 Lean audits** (95 at base + `AuditNativeLibraryHeaderDecoration`) |
| `python3 -m unittest discover -s scripts/tests` | 193 run, **1 failure**: the expected acceptance-matrix drift (§13.2) |

`third_party/libevent` was initialised in this fresh worktree before the run
(`git submodule update --init --recursive`), and this card's census reads it.

**How the run was executed.** A single `cargo test --workspace` invocation
exceeds the harness's foreground budget under the load of sibling worktrees, so
the suite was run in four package groups —
`qcore-core`/`-schemas`/`-frontend`/`-contract`, `qcore-c-gateway`/`-audit`/
`-verification`, `qcore-compiler`/`qcorec`, and `qcore-verifier-host` — whose
union is exactly the ten workspace members. No test was skipped, filtered or
weakened; only the process boundaries differ, and the ignored count (8) matches
the standing ledger and is unchanged by this card.

### 13.1 One recorded load flake, verified in isolation

`qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
failed in the first two `qcore-verifier-host` runs with `Io(Broken pipe)`. That
is **P02-FLAKE row three** (`AGENTS.md` constraint 16: "the confined-fixture
write test (`Io(Broken pipe)` under load)") and the documented condition held —
sibling agent worktrees building on the same host, load average above 20 and free
memory down to 1 GB. Re-run in isolation on the same binary, **unweakened**:

```
$ cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored
```

and the package then re-ran fully green (112 passed) on a later pass. No
assertion was touched, and this card's production diff contains no file under
`crates/qcore-verifier-host/`. The other four constraint-16 rows passed in the
package runs themselves.

### 13.2 One expected acceptance-matrix drift, left for the integrator

`scripts/tests` asserts the acceptance matrix's derived `lean_audits` row, which
must grow **95 → 96** now that this card registers an audit file. Acceptance
files are integrator surface and are not this card's to edit; reported here
rather than worked around, exactly as waves 18–22 did for the same row.
