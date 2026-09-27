# NATIVE-LIBRARY-HEADER — reading the library's own `.h` — 2026-09-11

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Card `NATIVE-LIBRARY-HEADER` of wave 22: the **exported-header level** of the
linkage spectrum. Wave 19 built a manifest whose entries carry the exported
header declaration *as the author transcribed it*; wave 20 put a library's
**pointer** symbols behind exactly those transcriptions. Nothing compared a
transcription against the header the library ships. This card compares them.

**Base tree:** main `ec5535a9`. Design (new, this card):
[`docs/design/NATIVE_LIBRARY_HEADER_V1.md`](../../design/NATIVE_LIBRARY_HEADER_V1.md),
which implements the piece
[`NATIVE_LIBRARY_LINKAGE_V1.md`](../../design/NATIVE_LIBRARY_LINKAGE_V1.md) §4.2
left open. Parent records:
[`wave20-native-library-contract-slice`](../wave20-native-library-contract-slice/README.md)
(which sharpened this card) and
[`wave19-unsafe-linkage-artifact`](../wave19-unsafe-linkage-artifact/README.md).
Reader precedents: [`wave19-abi-typedef`](../wave19-abi-typedef/README.md) and
[`wave20-abi-address-of-member`](../wave20-abi-address-of-member/README.md).

---

## Headline

**A manifest can now name its exported header, and the header decides.** Every
declaration the manifest binds is read out of that header's own text — no typed
Clang AST, a token stream and a bounded directive subset — and compared against
the entry's transcription. A certificate for such a callee says

> `qcore_extern_alloc` behaves as its selected contract states
> **— and —** it is read through the declared contract `…/buffer_lib.qcore`,
> from artifact `libqcore_buffer_demo.a` of library `qcore_buffer_demo 0.2.0` at
> `sha256:4d1f…32ed`, under stated ABI premises
> **— and —** its exported header `include/qcore_buffer_demo.h` was **read** at
> `sha256:a302…779d` and every bound declaration was compared against it.

The third clause is the card. It is also the first identity in this profile that
is **verified** rather than declared, and §3 is the argument for why that is
honest here and would not have been for the artifact.

| | |
|---|---|
| Manifest delta | two fields, `header` and `header_hash`, **optional as a pair**. Neither is the default and is byte-identical to waves 19–20 |
| The reader | `crates/qcore-core/src/native_library_header.rs`: comments, line splicing, a whole-file include guard, depth-zero declarations, the two typedef shapes cards `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` and `ABI-TYPEDEF-ADDRESS-OF-MEMBER` taught the gateway reader, and the `HEADER-STDINT-PRELUDE` (§4) |
| Outside the subset | **eleven named refusals**, including one this level invents: *read, and unspellable in a manifest* |
| Disagreement | **refuses** (`Q064`) — the `Q041` disposition, not `Q048`'s, from the one rule all three rest on (§2) |
| The hash | **verified**, not pinned, and the argument turns on the header being *read* (§3) |
| Totality | one-directional, deliberately the opposite of the contract slice's (§5) |
| Diagnostics | a **third band** for `NativeLibrary`, `Q063`–`Q064`, and a deliberately unallocated third code |
| New reserved words | **none.** A manifest is not source and neither is a header (§9) |
| `qcore-schemas` | **untouched at 14.0.0** — no provider rebuild |
| Evidence | 67 new Rust tests (49 core unit, 17 route, 1 CLI process) and **31 audited Lean theorems** in a new `QCore.NativeLibraryHeader`, no new axiom |
| Census | §6: **0 of 423** Libevent candidates inside the subset, for one named reason; **13 of 15** in ChunkIO's `chunkio.h`, for the absence of that reason |

**Nothing is promoted.** The header level can only refuse: it admits no symbol
the unheadered manifest refused, moves no ABI vocabulary, and gives no entry a
type it did not already spell. No raw Cycle 19 unit moves, no CT mapping
changes, the contract corpus is untouched, and full language conformance stays
false.

---

## 1. What the header supplies, and what the manifest still declares

The manifest keeps its transcription. Deleting it in favour of the header was
considered and rejected, on three grounds:

1. **A manifest must stay readable on its own.** An entry naming only a symbol
   would make the manifest unreadable without opening a second file, and the
   manifest is the artifact a certificate points a reader at.
2. **Two readings that must agree are a check; one reading is a belief.** The
   whole content of this level is the comparison. Deleting one side deletes the
   check and promotes the header to a single source of truth read by a reader
   with its own boundaries — strictly weaker than wave 20, because a declaration
   this reader *cannot* read would then silently supply nothing instead of
   contradicting something.
3. **They are different languages.** The manifest writes the §C04.1 projection;
   a header writes C. The projection between them is exactly what a reader can
   get wrong, and keeping both spellings makes it auditable where it is applied.

So: the manifest names the symbols, the contract bindings and the artifact
identity; the header supplies the C declarations; and the header's own identity
is *stated by the manifest and verified by this level*.

## 2. The disagreement direction — refusal, and why it is not `Q048`'s

Card `NATIVE-LIBRARY-CONTRACT-SLICE` stated the rule that decides this:

> **Withdraw only where the weaker admissible reading refuses.**

Below a withdrawn *header* binding the weaker admissible reading is **the
manifest without a header** — the wave-20 world, which is **accepted**.
Withdrawing to it would reward a contradiction with a premise that says *less*
about the build: an author whose manifest disagreed with the shipped header
would get a certificate that quietly stops mentioning the header. That is level
L1's situation exactly (`Q041`), and it gets L1's disposition: **refuse**.

There is a second reason, and it is the sharper one. The contract-slice
withdrawal is per-unit **because one of its two sides is per-unit** — the
analysed unit's own declaration. Both sides of *this* comparison are **load
inputs**: the manifest text and the header text are in hand before any unit is
looked at, so there is no per-unit reading to withdraw and no unit for which the
disagreement could fail to hold. A per-unit withdrawal here would be a refusal
wearing a withdrawal's name.

Both halves are pinned side by side rather than asserted:
`a_disagreement_refuses_rather_than_withdrawing_to_the_headerless_reading`
exhibits the refusal *and* compiles the weaker reading successfully, and in Lean
`a_contradicting_header_refuses_and_the_weaker_reading_is_admitted` states the
same pair.

Three polarities, one rule, and the table is now complete:

| Level | Weaker admissible reading | Is it accepted? | Disposition |
|---|---|---|---|
| L1 waiver vs manifest (`Q041`) | the bare waiver | yes | refuse |
| L3 contract vs manifest (`Q048`) | the uncontracted callee | no (`Q023`) | withdraw for the unit |
| manifest vs **header** (`Q064`) | the manifest with no header | yes | refuse |

## 3. The hash: verified here, still a pin there

Wave 19 decided the **artifact** identity is a *pin, not a probe*, on two
grounds. Both still stand, and neither applies here:

* *A hash of a file on this disk is not evidence about the link.* True, and
  there is **no link claim** at this level. The header is not linked; it is read.
* *A premise must be rederivable from the program alone.* This is the interesting
  one, and the header **satisfies** it rather than straining it. The decision
  this level takes is a function of the header's text, so a certificate that did
  not name that text *by content* could not be rederived at all: a recheck on
  another host whose `header` reference resolved to different bytes would
  silently take a different decision. Verifying the digest is what makes the
  rederivation honest, and it is available precisely *because* the file was
  opened.

So v1 **verifies** the header digest, and the residence premise now carries one
verified identity beside three declared ones. The rendered `quirin` line says
which is which in words, because a reader who cannot tell them apart has been
handed a stronger-looking statement than the one that was checked.

Two consequences worth stating:

* **The digest decides before any declaration is read.** A substituted header
  that agrees with the manifest in *every* declaration is still refused —
  pinned in Rust (`a_header_whose_digest_is_not_the_declared_one_refuses`, whose
  substitution edits only a comment) and in Lean
  (`a_substituted_header_that_agrees_is_still_refused`, whose unused agreement
  hypothesis is the point).
* **Who measures.** `qcore-core` has no dependencies at all — that is what makes
  a manifest rederivable in the kernel — so it implements no SHA-256. The driver
  measures with `qcore_schemas::digest::sha256_hex` where the bytes are read, and
  the *rule* is the kernel's. Same split as wave 20's `CustodyShape`; recorded as
  **`HEADER-DIGEST-DRIVER-MEASURED`**.

`sha256` remains the only admitted algorithm, so `NATIVE-LIBRARY-MULTI-HASH` is
unchanged and now covers two hashes rather than one.

## 4. The subset, and the one admission that needed an argument

A header is not a preprocessed translation unit. The gateway's declared-ABI
reader never sees a `#`, because Clang has already run; this one does. Writing
half a C preprocessor would be the worst of both — unbounded work, and a second
implementation of the one component whose bugs are silent — so v1 defines a
subset and refuses everything outside it **by name**.

**Read:** comments and line splicing; a **whole-file** include guard; depth-zero
declarations and `typedef`s; the two typedef shapes the wave-19 and wave-20
reader cards landed.

**Refused, by name:** a directive outside
`if`/`ifdef`/`ifndef`/`elif`/`else`/`endif`/`define`/`undef`/`include`;
unbalanced conditionals; a symbol declared only inside a conditional region; a
symbol defined as a macro; a `static` declaration; an empty (unspecified)
parameter list; a variadic declaration; two depth-zero declarations that project
differently; a type this reader does not commit to; and a type it **does** read
for which the manifest's vocabulary has no spelling. `#include` is never
followed.

Two of these are worth their own paragraph.

**`static` is not a reader boundary.** A `static` function has internal linkage
and **is not an exported symbol of the artifact**; binding one to an artifact
identity would be a false statement about the library. The gateway reader admits
`static` declarations and must, because it is analysing a translation unit where
one is exactly what it has to compare against. That is divergence 2 of §7, and
it is a difference of *purpose*, not of capability. Lean:
`an_internal_declaration_refuses_even_when_it_agrees`.

**"Read" and "unspellable" are different facts.** §C04.1 spells `fnptr`, `f32`
and `f64`; `qcore_core::extern_call::AbiType` — the manifest's table — does not.
A header declaring `double demo_now(void);` or a function-pointer typedef
parameter is *read*; what is missing is a **manifest spelling**. Collapsing that
into "unreadable" would report a vocabulary gap as a reader gap, and the two
have different repairs. It gets its own refusal
(`UnspellableResult`/`UnspellableParameter`) and its own follow-on.

### 4.1 `HEADER-STDINT-PRELUDE` — the one commitment to an unseen definition

The first implementation read *nothing*: `uint8_t` and `uintptr_t` come from
`<stdint.h>`, which is `#include`d and therefore unread, so the manifest's own
canonical spellings were unreadable wherever a real header wrote them. The level
would have decided nothing at all.

The repair is a **prelude of the typedef names C11 itself fixes**, and it is a
reading of the *standard* rather than a guess about an implementation. Two
classes, kept apart on purpose:

* **Exact-width names** (C11 7.20.1.1). `intN_t`/`uintN_t` are specified to be
  exactly `N` bits, two's complement, no padding. Their reading rests on the
  standard alone.
* **Pointer- and size-carrying names** (7.20.1.4, 7.19). `intptr_t`,
  `uintptr_t`, `size_t` and `ptrdiff_t` are *some* integer type of sufficient
  width, which the standard does **not** fix. Their reading here is the LP64 one
  — and LP64 is not assumed: it is the manifest's own declared `data_model lp64`
  premise (linkage design §4.4), which is exactly where a target assumption
  belongs. This is `LOADER-ABI-LP64` consumed rather than re-asserted.

Three guards keep it from being a convenience list:

1. names the standard leaves open in width (`intmax_t`, `int_least*_t`,
   `int_fast*_t`, `wchar_t`) and names that are not C11's (`ssize_t`, `off_t`)
   are **absent**, and therefore unreadable;
2. the prelude is consulted **only** where the header does not typedef the name
   itself — a header redefining one is read on its own terms, and an unresolvable
   own reading refuses rather than falling back here;
3. the readings are the *sets* the arithmetic projection already yields, so this
   reader and the gateway's compare at one granularity. Narrowing `int64_t` to a
   single spelling would have made the manifest gate **stricter** than the
   gateway gate on the same pair of texts, which is a divergence with no argument
   behind it.

Lean states all four facts: `the_exact_width_names_are_single_readings`,
`the_open_width_names_are_the_lp64_pairs`,
`a_name_the_standard_leaves_open_is_absent`, and
`every_prelude_reading_is_spellable` — the last being why the prelude is what
lets the level decide anything.

### 4.2 The include guard, narrowed after a measurement

The first rule was "a leading `#ifndef X`/`#define X` whose matching `#endif` is
the file's last directive". A test intended to *fail* on a non-guard passed,
because a file can have C tokens after the guard's `#endif` and still satisfy
that. The rule now additionally requires **no C token outside the region the
guard brackets** — an include guard wraps the whole file, and a conditional that
wraps the whole file is true on the reading this reader performs, since it reads
the file once. A region with tokens outside it is a *region*, and its contents
stay unread. Pinned as
`an_ifndef_that_is_not_the_file_wide_guard_is_a_conditional_region`.

## 5. Totality, one-directional — and why that is not the contract slice's rule

A header binding that cannot be read **rejects the whole manifest selection at
load**, exactly as an unresolved contract slice binding does (`Q047`). The
argument is wave 20's, unchanged: a partially read header binding leaves the
manifest attributing a build on the strength of a header this reader could check
only in part, and *which* part would then depend on the reader's own coverage
rather than on anything the manifest says. Opting in is therefore per manifest
and covers every symbol it binds; there is no per-entry opt-out, because an
entry that opted out would be a transcription believed beside transcriptions
checked, with nothing marking which was which.

But the *other* direction of wave 20's totality is deliberately **absent**.
Declarations the manifest does not name are ignored. A contract module in a
manifest's slice is authored *for that library*, so a declaration in it that no
entry binds is a partial binding; a header is the library's **entire exported
surface**, authored without reference to any manifest. Demanding that a manifest
bind every declaration of `event2/event.h` would make the level unusable and
would assert something nobody intends.

It is pinned as a property of the gate rather than of one header: Lean's
`unnamed_declarations_are_not_read` quantifies over two sources agreeing on the
named symbols and **arbitrary** elsewhere, and the Rust row
`declarations_the_manifest_does_not_name_are_ignored` puts four shapes that each
refuse when named — variadic, `static`, macro-decorated and unreadable — into a
header whose bound symbol is accepted.

## 6. The subset census over real headers

Measured by the **shipped reader**. The candidate set is every identifier the
text writes immediately before a `(` (`census_candidates`), so the denominator is
a statement about the header rather than about a curated list; `absent` is
reported as its own row because a candidate is not a declaration — a call inside
a macro body is one too.

### 6.1 Libevent — the pinned submodule (`d82464a`)

| Header | candidates | **read** | unreadable | conditional-only | variadic | absent |
|---|---|---|---|---|---|---|
| `event2/event.h` | 70 | **0** | 69 | 1 | 0 | 0 |
| `event2/buffer.h` | 56 | **0** | 51 | 4 | 1 | 0 |
| `event2/bufferevent.h` | 65 | **0** | 64 | 0 | 0 | 1 |
| `event2/http.h` | 122 | **0** | 121 | 0 | 0 | 1 |
| `event2/dns.h` | 42 | **0** | 41 | 1 | 0 | 0 |
| `event2/rpc.h` | 25 | **0** | 25 | 0 | 0 | 0 |
| `event2/tag.h` | 20 | **0** | 20 | 0 | 0 | 0 |
| `event2/listener.h` | 9 | **0** | 9 | 0 | 0 | 0 |
| `event2/thread.h` | 9 | **0** | 1 | 8 | 0 | 0 |
| `event2/watch.h` | 5 | **0** | 5 | 0 | 0 | 0 |
| `event2/visibility.h` | 0 | — | — | — | — | — |
| `event2/util.h` | — | **the whole file refuses**: it carries `#error`, which v1 does not interpret | | | | |
| **total (files read)** | **423** | **0** | **406** | **14** | **1** | **2** |

**One token accounts for it.** Every exported declaration is written

```c
EVENT2_EXPORT_SYMBOL
struct event_base *event_base_new(void);
```

and `EVENT2_EXPORT_SYMBOL` is defined in `event2/visibility.h`, which is
`#include`d and therefore unread. The refusal **names the token**, which is what
turns this census into a follow-on rather than a shrug; the route test
`the_dominant_census_refusal_names_one_token` pins that it dominates every other
reason on `event.h` by more than four to one (69 against 5 secondary reasons
across the same declarations).

Two smaller findings worth keeping: `event2/thread.h` is 8/9 *conditional-only*
(its `evthread_*` surface sits behind feature guards), and `event2/util.h`
refuses **as a file** — which is the right granularity, because a reader that
half-read a file carrying `#error` would be reporting on a text the compiler
never sees.

### 6.2 Fluent Bit's bundled libraries — pinned Cycle 19 build inputs

Read from `build/cycle19/wave20-pin/fluent-bit-clang-source/` (the persistent
audit inputs of `docs/CYCLE19_BUILD_INPUTS.md`). These are **not** files of this
repository, so their counts are reported and not pinned in CI; the *shapes* are
pinned as fixture rows instead (§6.3).

| Header | candidates | **read** | unreadable | internal (`static`) | ambiguous | conditional | absent |
|---|---|---|---|---|---|---|---|
| `chunkio/chunkio.h` | 15 | **13** | 2 | 0 | 0 | 0 | 0 |
| `msgpack/pack.h` | 52 | 0 | 1 | 43 | 6 | 0 | 2 |
| `msgpack/object.h` | 3 | 0 | 2 | 0 | 0 | 1 | 0 |
| `msgpack/sbuffer.h` | 13 | 0 | 0 | 7 | 0 | 0 | 6 |
| `monkey/mk_core/mk_list.h` | 16 | 0 | 0 | 16 | 0 | 0 | 0 |

**ChunkIO is the answer to the Libevent row.** `chunkio.h` writes plain,
undecorated declarations over `struct` tags, `char *`, `char **`, `size_t` and
`int` — and **13 of its 15** candidates are inside the v1 subset. The two that
are not are `cio_qsort` and `cio_set_log_callback`, both carrying an *inline*
function-pointer parameter (`int (*compar)(const void *, const void *)`), which
every nested declarator refuses in this reader and in the gateway's alike.

So the census's real statement is not "third-party headers are out of reach". It
is: **the export-decoration macro is the whole difference**, and a library that
does not use one is largely readable today.

msgpack is the third shape. Its `msgpack_pack_*` surface is `static`/`inline`
in the header — 43 `internal` rows plus 6 *ambiguous* ones, the latter being
symbols declared `static` up front and defined `inline` later, whose two
readings differ in exactly the linkage fact this level decides on. Both refuse,
and both refuse *correctly*: those symbols are not exported by any artifact. The
one `MSGPACK_DLLEXPORT`-decorated declaration, `msgpack_pack_object`, is
Libevent's reason again.

### 6.3 What is pinned in CI

* `the_subset_census_over_libevents_core_header_is_pinned` — `read == 0`,
  `unreadable >= 50`, no internal and no unspellable rows, over the pinned
  submodule.
* `the_dominant_census_refusal_names_one_token`.
* `every_libevent_core_header_stays_inside_the_directive_subset` and
  `a_header_using_error_directives_refuses_as_a_file`.
* `an_undecorated_third_party_declaration_shape_is_inside_the_subset` and
  `an_inline_function_pointer_parameter_stays_outside_the_subset` — ChunkIO's
  fifteen, transcribed, since the header itself is a build input rather than a
  repository file.
* A fixture reproducing the decoration shape in-repo
  (`qcore_buffer_demo_decorated.h`), so the dominant census refusal is pinned
  here and not only in a submodule.

Raw rows, with the per-token `reason` tallies:
[`artifacts/header_census.txt`](artifacts/header_census.txt).

## 7. Two readers of one vocabulary — the divergence census

The crate layering **forbids** sharing the Rust code. `qcore-core` has no
dependencies at all — that is what makes a manifest rederivable in the kernel
(wave-20 report §1) — and the gateway's declared-ABI reader sits above the
contract loader. So this card's reader is necessarily a second implementation of
one projection, and the wave-20 lesson about second readers is that the
divergences must be **measured and named**. They are pinned in CI by
`the_two_readers_agree_on_a_declaration_both_can_read` and
`the_reader_divergences_are_named_and_pinned`, which run **both** readers over
one text.

| # | Divergence | Gateway | Here | Why, deliberately |
|---|---|---|---|---|
| 0 | input class | a **preprocessed** unit: Clang ran, and it never sees a `#` | an **unpreprocessed** header: a bounded directive subset (§4) | the inputs differ; everything below follows from it |
| 1 | `HEADER-STDINT-PRELUDE` | `uint32_t` is an unknown name — `<stdint.h>` was already expanded away | read, per C11 (§4.1) | a header writes exactly these names, and they *are* the manifest's vocabulary |
| 2 | `static` | read and compared — a TU is what it analyses | refused, `InternalDeclaration` | a manifest binds an artifact's **exported** symbols |
| 3 | unspecified parameter list `f()` | admitted where the contract names no parameters (it asserts nothing about them) | refused | a manifest entry `fn f() -> T` **asserts** the C declaration is `f(void)`, so there is something to contradict and nothing to check it against |
| 4 | `fnptr`, `f32`, `f64` | projected and compared — they are §C04.1 spellings | read, then refused as **unspellable** | the manifest's ABI table is `extern_call::AbiType`, which has none of the three |

Everything else is the same algorithm, ported rather than re-invented: the
specifier and qualifier tables, the typedef targets and their depth limit, the
array/K&R/nested-declarator refusals, the multiset arithmetic projection, and
the set-valued admission. `ABI-FNPTR-DEPTH1` and the aggregate-value refusal
carry over unchanged.

**The gateway's reader was not touched.** No file under
`crates/qcore-c-gateway/` is in this diff. Retiring either reader for the other
needs invariant 3 parity first, and is follow-on
`NATIVE-LIBRARY-HEADER-ONE-READER`.

## 8. Diagnostics

A **third band** for the existing `NativeLibrary` pass, `Q063`–`Q064`, through
the §L21 process, with spec §L21, the new §L14.4 and grammar §G02d.2 all updated
in this change.

| Code | Condition | Pinned by |
|---|---|---|
| `Q063` | the header binding does not load: not supplied, digest mismatch, an uninterpreted directive, unbalanced conditionals, or a bound symbol's declaration that cannot be read (undeclared, conditional-only, macro, `static`, unspecified, variadic, ambiguous, unreadable type, unspellable type) | 6 route rows + 28 unit rows |
| `Q064` | a manifest entry's transcription and the header's own text disagree in arity, a parameter or the result — a **refusal** | 3 route rows (one covering three separate edited projections) + 5 unit rows |

**No third code, deliberately.** A publication tripwire on `Q049`'s model was
considered and **not** allocated: the header identity is a *field* of the
residence record `Q049` already requires, so a certificate that carried the
residence and lost the header cannot be constructed. That is
`CONTRACT-SLICE-WAIVER-MIRROR`'s discipline — an unreachable diagnostic code is
a claim a reader cannot check — applied to a code that was never written rather
than to one that was written and then removed.

**Band assumption, for the integrator.** `Q050` is `ComposedTasks`; `Q063`+ is
contested inside wave 22 (cards `STD-BUFFER` and `STD-EVENT`). This band is
**unshipped** in the `T07`/`T09` sense and the code-to-condition mapping is
stable under a relocation. Sites are listed in §12.

## 9. What did not move — pinned in both directions

* **A manifest naming no header is untouched**, and not merely "no source
  matched": the route row supplies a text that contradicts every entry, and the
  manifest is admitted with no header identity published
  (`a_manifest_naming_no_header_is_unaffected_by_supplied_texts`; in Lean,
  `a_manifest_naming_no_header_is_unchanged` over *every* source).
* **The delta is the premise and nothing else.**
  `the_header_binding_changes_the_premises_and_nothing_else` compares a bound and
  an unbound compilation field by field — profile, emitted C, entry, declaration
  count, and the premise inventory with the header identity stripped — and then
  asserts that *with* it they differ, so the level is not a no-op either.
* **Waves 18–20 pass unchanged**: `compiler/tests/native_library.rs` (16 rows),
  `native_library_contract.rs` (15) and `unsafe_linkage.rs` (28) are untouched
  apart from the new `CompileOptions` field.
* **New reserved words: none.** Constraint 14's census: `header` and
  `header_hash` are manifest field keywords, and a manifest shares no lexer with
  source (`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2 is where the manifest
  grammar lives precisely because it is not §G02d's). A header is C, read by a
  reader of its own. No token was added to the source grammar.
* **`qcore-schemas` stays 14.0.0.** Constraint 18 is not engaged: no provider
  package was rebuilt and none needed to be. `compiler` re-exports
  `qcore_schemas::digest` so a consumer of the facade measures with the same
  function the driver does rather than a second one.
* **Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
  `crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched, and
  `crates/qcore-contract/` is not touched either. The fixtures live under
  `compiler/tests/fixtures/`, so the 69-module corpus pin is untouched. No parser
  rule changed. The corpus baseline stays `docs/reports/wave17-corpus/`.

Two smaller recorded decisions:

* **`HEADER-CLI-RELATIVE`.** `quirin` resolves a manifest's `header` reference
  against the **manifest's own directory** — one candidate path, no search, no
  fallback — because the manifest is the only anchor the binary has for a
  reference the manifest itself spelled, and a search would make two
  byte-identical manifests mean different things depending on where they were
  invoked from. Without this the level would be unreachable through the shipped
  binary; with it, `quirin --native-library` uses it, and a CLI process test
  pins all three outcomes — accepted with the verified identity printed, a
  contradicting header refused at `Q064`, and a named header nobody can open
  reported as the `open` failure it is. (The L3 *contract* premise is still
  unreachable there — `UNSAFE-LINKAGE-CLI` is unchanged.)
* **`HEADER-FIXTURE-EOL-NORMALISED`.** The fixture manifest pins the fixture
  header's SHA-256, and a checkout that translated line endings would hand the
  test different bytes for the same file. The test normalises before supplying
  the text, so the pin is a statement about the header's *content*. Nothing in
  the shipped reader normalises anything: the driver digests exactly the bytes it
  was given.

## 10. Lean

`proof/QCore/NativeLibraryHeader.lean` (new) + `proof/AuditNativeLibraryHeader.lean`
(new, registered), imported into `proof/QCore.lean`. **31 theorems audited, no new
axiom** (only `propext`, `Classical.choice`, `Quot.sound`).

The agreement relation is **imported**, not restated: `positionOutcome` is stated
equal to `QCore.DeclaredAbiAgreement.admits` on every readable, spellable
position, which is the model's way of saying what the crate layering stops the
Rust from saying by sharing code. The typedef targets are
`QCore.TypedefProjection`'s, so the two shapes cards
`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` and `ABI-TYPEDEF-ADDRESS-OF-MEMBER` taught
the gateway reader are literally the same two shapes here.
`QCore.NativeLibrary.Residence` gained the `header` field and `Manifest` the
matching one, in the same change; every existing L1 and L3 theorem is unchanged
and still audited.

* **The shared relation** — `the_agreement_relation_is_the_declared_abi_one`,
  `a_read_position_that_does_not_admit_disagrees`.
* **Opt-in** — `a_manifest_naming_no_header_is_unchanged`,
  `the_level_is_not_a_no_op`.
* **The digest** — `a_mismatched_digest_refuses`,
  `a_substituted_header_that_agrees_is_still_refused`,
  `an_unsupplied_header_refuses`,
  `a_reference_that_is_not_the_one_named_is_not_supplied`.
* **One-directional totality** —
  `a_bound_symbol_the_header_does_not_declare_refuses`,
  `unnamed_declarations_are_not_read`.
* **Refusal, not withdrawal** — `the_weaker_reading_is_admitted`,
  `a_contradicting_header_refuses_and_the_weaker_reading_is_admitted`,
  `a_header_binding_only_removes_admissions`,
  `a_header_binding_removes_something`.
* **Read vs unspellable** —
  `the_manifest_vocabulary_excludes_exactly_three_spellings`,
  `an_unspellable_position_never_agrees`,
  `unreadable_and_unspellable_are_different_refusals`.
* **The imported typedef shapes** —
  `a_function_pointer_typedef_is_read_and_unspellable`,
  `an_aggregate_pointer_typedef_is_read_and_spellable`,
  `an_aggregate_value_is_unreadable`.
* **The prelude** — `the_exact_width_names_are_single_readings`,
  `the_open_width_names_are_the_lp64_pairs`,
  `a_name_the_standard_leaves_open_is_absent`,
  `every_prelude_reading_is_spellable`.
* **Refusal completeness and determinism** —
  `every_non_declaration_shape_refuses_by_name`,
  `every_incomparable_declaration_refuses_by_name`,
  `an_arity_difference_disagrees`,
  `an_internal_declaration_refuses_even_when_it_agrees`,
  `the_binding_is_a_function`, `an_agreeing_binding_is_admitted`.

**Stated boundary of the Lean model.** The header's concrete lexical subset, its
directive layer and the total-or-reject text reader are a Rust boundary and are
tested there; what the model is about is what a **read** header means. That the
supplied text is the header the reference names, that the library conforms to its
declarations, and that the linker used the artifact all remain premises, as does
the agreement of the Rust code with the transcription.

## 11. Named follow-ons

| Name | What is open |
|---|---|
| `NATIVE-LIBRARY-HEADER-DECORATION` | export-decoration macros — the census's single dominant reason. The obvious design, a manifest-declared erasure list, converts a *reader boundary* into an **unchecked premise about the header's own text**, which is precisely what this level exists to remove; v1 declines to make that trade unilaterally, and any successor must argue it |
| `NATIVE-LIBRARY-HEADER-INCLUDE` | following `#include` into a bounded, manifest-declared set of further texts, so a typedef one file away stops being unreadable (`ev_uint32_t`, `evutil_socket_t`, `msgpack_object` are the measured instances) |
| `NATIVE-LIBRARY-HEADER-FNPTR` | a manifest spelling for `fnptr`, `f32` and `f64`. Needs `qcore_core::extern_call::AbiType` widened — a change to what may cross a call boundary, not to a manifest grammar, and therefore its own card |
| `NATIVE-LIBRARY-HEADER-CONDITIONAL` | evaluating the bounded conditional subset an exported header actually uses. `event2/thread.h` is 8/9 conditional-only today |
| `NATIVE-LIBRARY-HEADER-ONE-READER` | the two readers of §7. Retiring either needs invariant 3 parity first |
| `NATIVE-LIBRARY-SOURCE-BINDING` | **unchanged from wave 20**: binding a manifest from the source file (`#import std.buffer;`) rather than by symbol name across the selection. Still needs a registry name or a new form, and still a constraint-14 census |
| `NATIVE-LIBRARY-CERT-REPLAY` | unchanged, and now with a **second** identity to replay against — and one that a recheck can actually verify |
| `UNSAFE-LINKAGE-ARTIFACT-PROBE`, `NATIVE-LIBRARY-MULTI-HASH`, `NATIVE-LIBRARY-SLICE-SHAPE`, `UNSAFE-LINKAGE-CLI` | unchanged from wave 20 |

Buffer extents, struct/string/callback transport, sentinel adapters, extern calls
in concurrent or error-region contexts, target-aware scalar projection, dynamic
loading and verifying the link step stay out of scope exactly as for every other
level.

## 12. Integrator reconciliation

1. **Diagnostic band `Q063`–`Q064`** is claimed for a *third* `NativeLibrary`
   range, and `Q063`+ is contested this wave by `STD-BUFFER` and `STD-EVENT`. If
   either shipped first, relocate this **unshipped** band per `T07`/`T09`; the
   mapping is stable under a shift. Sites: `docs/QCORE_SPEC_r0_3.md` §L21 row and
   the new §L14.4, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2,
   `crates/qcore-frontend/src/frontend.rs` (`SemanticPass::NativeLibrary` doc),
   `crates/qcore-verification/src/driver.rs` (one emission site, two codes).
2. **`docs/reviews/r0_3/acceptance_matrix.json`** is integrator surface and is
   not edited here. `NATIVE-LIBRARY-HEADER` should be retired from
   `named_followups` and the five new follow-ons of §11 entered
   (`-DECORATION`, `-INCLUDE`, `-FNPTR`, `-CONDITIONAL`, `-ONE-READER`).
   The derived `lean_audits` row must grow by one (`AuditNativeLibraryHeader`),
   which is the expected `scripts/tests` drift below.
3. **Ledger.** `AGENTS.md`'s headline test/Lean numbers and `DEVELOPMENT.md`'s
   increment record are integrator surface and are not edited here.
4. **`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`'s "Landed since" note** was
   updated in place, as that document's §0 discipline provides for: §4.2's row
   now records three dispositions from one rule, and §4.3's records the second,
   verified identity beside the pinned one. §6's `std.buffer` and §12 stages
   (b)–(d) remain untouched.
5. **`CompileOptions` gained a public field** (`native_library_headers`). Every
   in-tree construction site was updated; an out-of-tree consumer constructing it
   as a literal will need the field. This is a source-compatible-by-default
   struct with `Default`, so `..Default::default()` remains available.
6. **No provider rebuild.** `qcore-schemas` is 14.0.0 and no manifest template
   changed.

## 13. Gate

| Check | Result |
|---|---|
| `cargo test` over all ten workspace members | **3,410 passed, 8 ignored**, one failure: the recorded constraint-16 load flake, re-verified green in isolation (§13.1) |
| `crates/qcore-core` unit `native_library_header` (new) | 49 passed |
| `crates/qcore-core` unit `native_library` (waves 19–20) | 46 passed, unchanged |
| `compiler/tests/native_library_header.rs` (new) | 17 passed |
| `crates/qcorec/tests/cli.rs::the_cli_reads_the_exported_header_a_manifest_names` (new) | 1 passed |
| `compiler/tests/native_library_contract.rs` (wave 20) | 15 passed, unchanged |
| `compiler/tests/native_library.rs` (wave 19) | 16 passed, unchanged |
| `compiler/tests/unsafe_linkage.rs` (wave 18) | 28 passed, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 91 Lean audits** (90 at base + `AuditNativeLibraryHeader`) |
| `python3 -m unittest discover -s scripts/tests` | 193 run, 1 expected failure (§13.2) |

`third_party/libevent` was initialised in this fresh worktree before the run
(`git submodule update --init --recursive`), and this card's census reads it.

**How the run was executed.** A single `cargo test --workspace` invocation
exceeds the harness's foreground budget under the load of five sibling
worktrees, so the suite was run in four package groups —
`qcore-core`/`-schemas`/`-frontend`/`-contract` (1,151),
`qcore-c-gateway`/`-audit`/`-verification` (1,265),
`qcore-compiler`/`qcorec` (882), `qcore-verifier-host` (112) — whose union is
exactly the ten workspace members. No test was skipped, filtered or weakened;
only the process boundaries differ, and the ignored count (8) matches the
standing ledger and is unchanged by this card.

### 13.1 The one failure is a recorded load flake, verified in isolation

`qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
failed once during the package run. That is **P02-FLAKE row three**
(`AGENTS.md` constraint 16: "the confined-fixture write test (`Io(Broken pipe)`
under load)"), and the documented condition held — five sibling agent worktrees
building and testing on the same host, free memory down to 1–2 GB. Re-run in
isolation on the same binary, **unweakened**:

```
$ cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored
```

No assertion was touched. This card's production diff contains no file under
`crates/qcore-verifier-host/`, and the package re-ran fully green (112 passed)
on a second pass. The other three constraint-16 rows passed in the package run
itself.

### 13.2 One expected acceptance-matrix drift, left for the integrator

`scripts/tests` asserts the acceptance matrix's derived `lean_audits` row, which
must grow **90 → 91** now that this card registers an audit file. Acceptance
files are integrator surface and are not this card's to edit; reported here
rather than worked around, exactly as waves 18–20 did for the same row.
