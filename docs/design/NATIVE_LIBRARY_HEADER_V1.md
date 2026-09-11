<!-- Authored and implemented by card `NATIVE-LIBRARY-HEADER` (wave 22). -->

# Reading the library's exported header: `source.native-library.v1`, header level

> **Status: landed for the subset this document names.** Card
> `NATIVE-LIBRARY-HEADER` implements §2–§7 in
> [`crates/qcore-core/src/native_library_header.rs`](../../crates/qcore-core/src/native_library_header.rs);
> the subset census of §3 is measured by the shipped reader over the pinned
> `third_party/libevent` headers and over the pinned Cycle 19 msgpack header,
> and is reported in
> [`docs/reports/wave22-native-library-header/README.md`](../reports/wave22-native-library-header/README.md).
> The named refusals of §4 are the boundary; what they exclude is follow-on
> work, listed in §9, not omission.

This is the **exported-header level** of the linkage spectrum
([`NATIVE_LIBRARY_LINKAGE_V1.md`](NATIVE_LIBRARY_LINKAGE_V1.md) §4.2, the piece
its "Landed since" table records as "reading a real `.h` is still follow-on
`NATIVE-LIBRARY-HEADER`"). Cards `UNSAFE-LINKAGE-ARTIFACT` (wave 19) and
`NATIVE-LIBRARY-CONTRACT-SLICE` (wave 20) built a manifest in which each entry
carries the exported header declaration **as the manifest author transcribed
it**. That transcription is a premise: nothing compared it against the header
the library actually ships. This level compares them.

## 1. What the header supplies and what the manifest still declares

The division is exactly the one the wave-20 report argued for the contract
slice, applied one level out.

| | Supplied by | Why there |
|---|---|---|
| Which symbols are bound | **manifest** | the manifest is the selection. A header declares the library's whole surface; the manifest names the part this build is bound through |
| Which declared `.qcore` contract each symbol is read through | **manifest** | unchanged from wave 20; a contract reference is a selection key |
| The artifact identity and the four ABI premises | **manifest** | unchanged from wave 19; §4.3–§4.4 of the linkage design |
| **The C declaration of each bound symbol** | **header** | this level. The manifest keeps its transcription, and the transcription is now *checked against* the header rather than believed |
| The header's own identity | **manifest states, this level verifies** | §5 |

The manifest's transcription is deliberately **not** deleted in favour of the
header. Three reasons:

1. **A manifest must stay readable on its own.** An entry that named only a
   symbol would make the manifest unreadable without opening a second file, and
   the manifest is the thing a reader of a certificate is pointed at.
2. **Two readings that must agree are a check; one reading is a belief.** The
   whole content of this level is the comparison. Deleting one side would
   delete the check and leave the header as a new single source of truth read by
   a reader with its own boundaries (§4) — strictly weaker than what wave 20
   already had, since a declaration this reader *cannot* read would then silently
   supply nothing instead of contradicting something.
3. **The manifest's vocabulary is the §C04.1 projection; a header's is C.** They
   are different languages, and the projection between them is exactly what a
   reader can get wrong. Keeping both spellings makes the projection auditable
   at the point where it is applied.

## 2. The binding

Two new **optional-as-a-pair** manifest header fields:

```text
header include/qcore_buffer_demo.h;
header_hash sha256:2b0f…c41d;
```

* Both present, or neither. `header` without `header_hash` and `header_hash`
  without `header` are each a `MissingField` rejection of the manifest, on the
  wave-19 ground that an identity missing a field is not a weaker identity but
  an unusable one.
* Neither present is the **default**, and it is exactly the wave-19/wave-20
  world: every manifest written before this card loads byte-identically, admits
  the same symbols and publishes the same premises. This level cannot refuse a
  program that did not opt into it.
* `header` is a **reference**, compared exactly and never normalised — the same
  discipline constraint 11 applies to a spelled input path and wave 20 applied
  to a contract module reference, for the same reason: two spellings that
  normalise together are two different selections.
* The reference is resolved by the **driver**, exactly as manifest and contract
  texts already are (`CompileOptions::native_libraries`,
  `CompileOptions::extern_contracts`). `qcore-core` opens no file; it is handed
  the text and the digest measured over it. This is not a weakening — see §5.

## 3. The preprocessing boundary

A header is not a preprocessed translation unit. The gateway's declared-ABI
reader (`crates/qcore-c-gateway/src/legacy_declared_abi.rs`) never sees a `#`,
because Clang has already run. This reader does. Writing half a C preprocessor
to close that gap would be the worst of both: unbounded work, and a second
implementation of the one component in the toolchain whose bugs are silent.

So v1 defines a **subset**, and refuses everything outside it **by name**.

### 3.1 What is read

* **Comments and whitespace.** `/* … */` and `// …` are replaced by whitespace;
  line splicing (`\` at end of line) is performed. Both are lexical and cannot
  change which declaration a token belongs to.
* **The include guard.** A leading `#ifndef NAME` / `#define NAME` whose matching
  `#endif` closes the file's last directive **and outside which the file has no C
  token at all** is recognised, and its body is read at depth zero. This is not
  an interpretation of a condition: a conditional that wraps the entire file is
  true on the reading this reader performs — it reads the file once — so the
  guarded body *is* the file. The whole-file requirement was **narrowed after a
  measurement**, and is what keeps the rule from admitting a feature conditional
  that happens to be shaped like a guard; a region with tokens outside it is a
  region, and its contents stay unread. A `#pragma once` is **not** admitted
  (§3.2) — it is a directive whose meaning is a compiler extension, and the guard
  rule above needs no extension.
* **The `HEADER-STDINT-PRELUDE`.** The typedef names C11 itself fixes
  (`intN_t`/`uintN_t` exactly, and `intptr_t`/`uintptr_t`/`size_t`/`ptrdiff_t`
  under the manifest's own declared `data_model lp64` premise) are read without
  their defining `#include` being followed. Without this the level would decide
  nothing at all — `<stdint.h>` is where the manifest's own canonical spellings
  come from — and it is a reading of the *standard* rather than a guess about an
  implementation. Names the standard leaves open in width are absent and
  therefore unreadable, and the prelude is consulted only where the header does
  not typedef the name itself. See the card report §4.1.
* **Declarations at depth zero**, in the form
  `SPECIFIERS DECLARATOR ( PARAMETERS ) ;` or the same followed by a `{` body,
  whose types project into the §C04.1 vocabulary through the same rules the
  gateway reader uses (§6).
* **`typedef`s at depth zero**, including the two shapes cards
  `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` and `ABI-TYPEDEF-ADDRESS-OF-MEMBER`
  taught the gateway reader: an aggregate defined by the typedef itself, and a
  function-pointer typedef.

### 3.2 What is refused, by name

| Condition | Refusal | Ground |
|---|---|---|
| A directive outside `if/ifdef/ifndef/elif/else/endif/define/undef/include` | `UnsupportedDirective` | `#pragma`, `#line`, `#error`, `#warning`, `#include_next` each either change semantics this reader does not model or (for `#line`) could move attribution, which constraint 12 forbids on the analysed side and this level has no reason to admit on the declared side |
| Unbalanced conditional nesting | `UnbalancedConditional` | a file this reader cannot bracket is one whose depth-zero region is not defined |
| A bound symbol the header `#define`s as a macro | `MacroSymbol` | a macro is not an exported symbol of an artifact; a caller reaches the macro and not the declaration |
| A bound symbol declared **only** inside a conditional region | `ConditionalDeclaration` | v1 evaluates no condition. The region's contents are not read, and a symbol found only there is refused *naming that reason*, never silently treated as undeclared |
| A bound symbol the header does not declare at all | `UndeclaredSymbol` | the manifest asserts the artifact exports it and the header is the artifact's exported surface |
| A bound symbol whose declaration carries `static` | `InternalDeclaration` | a `static` function has internal linkage and **is not an exported symbol**. Binding one to an artifact identity would be a false statement about the artifact, not a reader boundary. This is a deliberate divergence from the gateway reader, which is analysing a translation unit where a `static` declaration is exactly what it must read |
| A declaration whose specifier prefix carries an identifier this reader cannot resolve | `UnreadableResult` / `UnreadableParameter`, naming the spelling | the dominant real-world case (§3.3): an export-decoration macro whose `#define` is behind an unread `#include` |
| An empty parameter list `f()` in a declaration | `UnspecifiedParameters` | C11 6.7.6.3p14: it specifies nothing about parameters. A manifest entry, unlike a declared contract, always **asserts** a parameter vector — `fn f() -> T` asserts that the C declaration is `f(void)` — so there is always something to contradict and nothing to check it against |
| A variadic declaration | `VariadicDeclaration` | the manifest's entry grammar has no tail; a fixed vector cannot be compared against `…` |
| Two depth-zero declarations of one bound symbol that project differently | `AmbiguousDeclaration` | the fail-closed composition the gateway reader uses, unchanged |
| A readable projection with no manifest spelling — `fnptr`, `f32`, `f64` | `UnspellableProjection` | the header type is *read*; what is missing is a manifest spelling for it. Naming this separately is what keeps "this reader could not read it" and "this vocabulary cannot say it" from collapsing into one number |

`#include` is **never followed**. The included file's declarations are not read
and its typedefs are not known, so a type that depends on one is refused above
rather than guessed. That is the single largest source of refusals in the census
(§3.3), and making it a *named* refusal rather than an unknown is the point.

### 3.3 Census over real headers — which subset covers which declarations

Measured by the shipped reader; full rows in the card report. The headline:

* **Libevent (`third_party/libevent/include/event2/*.h`, pinned submodule).**
  **0 of 423** candidates are inside the subset. Essentially every exported
  declaration is written `EVENT2_EXPORT_SYMBOL` on its own line above
  `int event_base_dispatch(struct event_base *base);` — an object-like macro
  defined in `event2/visibility.h`, which is `#include`d and therefore unread.
  The reader refuses each one **naming that token**. `event2/util.h` refuses as a
  whole file, carrying `#error`; `event2/thread.h` is 8 of 9 conditional-only.
* **ChunkIO (`chunkio/chunkio.h`, pinned Cycle 19 input).** **13 of 15**
  candidates are inside the subset — plain undecorated declarations over `struct`
  tags, `char *`, `char **`, `size_t` and `int`. The two exceptions carry an
  *inline* function-pointer parameter, which every nested declarator refuses here
  and in the gateway's reader alike.
* **msgpack (`msgpack/pack.h`, pinned Cycle 19 input).** A third shape: 43 of 52
  candidates are `static` (internal linkage, so not exported symbols at all) and
  6 more are *ambiguous* — declared `static` up front and defined `inline` later,
  two readings differing in exactly the linkage fact this level decides on. The
  one `MSGPACK_DLLEXPORT`-decorated declaration is Libevent's reason again.

So the census's statement is not "third-party headers are out of reach". It is
that the **export-decoration macro is the whole difference**: a library that does
not use one is largely readable today, and a library that does is nil. That, plus
an `#include` story, is exactly what a header level costs to make general —
rather than "C is hard".

## 4. Total or reject, and at which granularity

**A header binding that cannot be read rejects the whole manifest selection at
load**, exactly as a contract slice binding that does not resolve does (`Q047`,
wave-20 report §1). The argument is that card's, unchanged: a partially read
header binding leaves the manifest attributing a build on the strength of a
header the reader could check only in part, and *which* part would then depend
on the reader's own coverage rather than on anything the manifest says.

Two consequences worth stating rather than discovering:

* **Opting in is all-or-nothing per manifest.** A manifest that names a header
  is checked against it for every symbol it binds. There is no per-entry opt-out,
  because an entry that opted out would be a transcription believed beside
  transcriptions checked, with nothing marking which was which.
* **Unnamed declarations in the header are ignored.** The totality is
  one-directional here, and deliberately so — the *opposite* of the contract
  slice's both-directions rule (§4.1 of the linkage design). A contract module
  in a manifest's slice is a slice *authored for this library*, so a declaration
  in it that no entry binds is a partial binding. A header is the library's
  entire exported surface, authored without reference to any manifest;
  demanding that a manifest bind every declaration of `event2/event.h` would
  make the level unusable and would assert something no one intends.

## 5. The header hash: verified, not pinned — and why that differs from §4.3

Wave 19 decided the **artifact** identity is a *pin, not a probe*: nothing is
opened, nothing is hashed. Two grounds, both still standing: a hash of a file on
this disk is not evidence about the *link*, and a premise must be rederivable
from the program alone.

**The header is different in exactly the way that matters: it is read.** Its
text decides admission. So:

* **v1 verifies the header digest.** The manifest declares `header_hash`; the
  driver measures SHA-256 over the bytes it read for the `header` reference; the
  kernel refuses the manifest on any mismatch (`DigestMismatch`).
* Neither wave-19 ground is contradicted. There is no *link* claim here, so the
  first does not apply. The second is **satisfied rather than strained**: the
  decision this level takes is a function of the header text, so a certificate
  that did not name that text by content could not be rederived at all — a
  recheck on another host whose `header` reference resolved to different bytes
  would silently take a different decision. Verifying the digest is what makes
  the rederivation honest, and it is available *because* the file was opened.
* The premise a certificate publishes therefore has **two kinds** in one record:
  three declared identities (library, version, artifact hash) and one
  **verified** one (the header reference and digest). The rendered premise says
  which is which, in words, because a reader who cannot tell them apart has been
  handed a stronger-looking statement than the one that was checked.
* **Who measures.** `qcore-core` has no dependencies at all — that is what makes
  the manifest rederivable in the kernel (wave-20 report §1) — so it implements
  no SHA-256. The driver measures with `qcore_schemas::digest::sha256_hex`,
  where the bytes are read, and the *rule* ("must equal the declared digest") is
  the kernel's. This is the same split as wave 20's `CustodyShape`: projection
  where the data lives, decision in the kernel. Recorded as
  **`HEADER-DIGEST-DRIVER-MEASURED`**.
* `sha256` remains the only admitted algorithm, as for the artifact hash;
  anything else rejects rather than being carried opaquely
  (`NATIVE-LIBRARY-MULTI-HASH` is unchanged).

## 6. The agreement relation, and which reading is weaker

### 6.1 What is compared

For each bound symbol, the header's projection of a type position against the
manifest's spelling at the same position — arity first, then parameters in
order, then the result. The comparison is `admits`: the header's projection is a
**set** of admissible spellings (`plain char` admits `int8_t` and `uint8_t`;
`unsigned long` on LP64 admits `uint64_t` and `uintptr_t`), and the manifest's
spelling must be a member. That is the gateway reader's relation, unchanged,
and it is why a faithful `size_t` transcription is not reported as a
disagreement.

### 6.2 Disagreement **refuses**; it does not withdraw

Wave 20 stated the rule that decides this:

> Withdraw only where the weaker admissible reading refuses.

Below a withdrawn *header* binding the weaker admissible reading is **the
manifest without a header** — the wave-20 world, which is **accepted**.
Withdrawing to it would reward a contradiction with a premise that says *less*
about the build: an author whose manifest disagreed with the shipped header
would get a certificate that simply stops mentioning the header. That is L1's
situation exactly (`Q041`), and it gets L1's disposition: **refuse**.

There is a second, independent reason, and it is the sharper one. The
contract-slice withdrawal is per-unit *because one of its two sides is per-unit*
— the analysed unit's own declaration. **Both sides of this comparison are load
inputs.** The manifest text and the header text are in hand before any unit is
looked at, so there is no per-unit reading to withdraw and no unit for which the
disagreement could fail to hold. A per-unit withdrawal here would be a refusal
wearing a withdrawal's name.

So a manifest/header disagreement refuses the manifest at **load**, and the
program falls to whatever it would have been with no manifest selected — which,
per the wave-19 rule that a selection that does not load is refused rather than
ignored, is a refusal, not a silent downgrade.

### 6.3 Monotone in the refusing direction

Binding a header can only remove admissions. It admits no symbol the
unheadered manifest refused, changes no ABI vocabulary, and gives no entry a
type it did not already spell. This is stated in Lean
(`a_header_binding_only_removes_admissions`) as well as in prose, because it is
the property a lazy implementation would lose.

## 7. Diagnostics

A **third band** for the existing `NativeLibrary` pass, `Q051`–`Q052`, through
the §L21 process.

| Code | Condition |
|---|---|
| `Q051` | the header binding does not load: the named header is not supplied, its measured digest does not equal the declared one, it carries a directive v1 does not admit or unbalanced conditionals, or the declaration of a bound symbol cannot be read — undeclared, conditional-only, `static`, unspecified parameter list, variadic, ambiguous, an unreadable type, or a readable type the manifest vocabulary cannot spell |
| `Q052` | a manifest entry's stated header declaration and the exported header's own text disagree in arity, a parameter or the result — a **refusal**, deliberately the same disposition as `Q041` and the opposite of `Q048` (§6.2) |

**No third code, deliberately.** A publication tripwire on the model of `Q049`
was considered and **not** allocated: the header identity is a *field* of the
residence record `Q049` already requires, so a certificate that carried the
residence and lost the header could not be constructed, and an unreachable
diagnostic code is a claim a reader cannot check
(`CONTRACT-SLICE-WAIVER-MIRROR`'s discipline, applied to a code that was never
written rather than to one that was and then removed).

**Band assumption.** `Q050` is `ComposedTasks`. `Q051`+ is contested inside wave
22 (cards `STD-BUFFER` and `STD-EVENT`); this band is **unshipped** in the sense
`T07`/`T09` established, and the code-to-condition mapping is stable under a
relocation.

## 8. Lean obligations

* the header reading is a **function** of the header text and the bound symbol
  set, so a rederivation repeats it;
* the agreement relation is the **same relation** `QCore.DeclaredAbiAgreement`
  models — stated as a theorem relating the two, not as a comment — and the
  typedef targets are `QCore.TypedefProjection`'s, imported rather than
  restated;
* **monotonicity**: a header binding only removes admissions (§6.3);
* **refusal completeness** over the subset boundary: every named refusal is
  reachable and every non-admitted shape refuses by name;
* the one-directional totality of §4 is stated on both sides: a bound symbol the
  header does not declare refuses, and a declaration the manifest does not name
  changes nothing;
* the digest gate: a mismatched digest refuses, and equal digests over equal
  texts decide alike;
* no new axioms; a new `Audit*.lean` registered with
  `scripts/check_lean_audits.py`. That the supplied text is the header the
  reference names remains a premise, as does everything else outside the model.

## 9. Out of scope — named follow-ons, not omissions

| Name | What is open |
|---|---|
| `NATIVE-LIBRARY-HEADER-DECORATION` | export-decoration macros (`EVENT2_EXPORT_SYMBOL`, `MSGPACK_DLLEXPORT`). The obvious design — a manifest-declared erasure list — converts a *reader boundary* into an *unchecked premise about the header's own text*, which is precisely the thing this level exists to remove; v1 declines to make that trade unilaterally. Any successor must argue it |
| `NATIVE-LIBRARY-HEADER-INCLUDE` | following `#include` to a bounded, manifest-declared set of further header texts, so a typedef defined one file away stops being unreadable |
| `NATIVE-LIBRARY-HEADER-FNPTR` | a manifest spelling for `fnptr` (and for `f32`/`f64`), which needs `qcore_core::extern_call::AbiType` widened — a change to what may cross a call boundary, not to a manifest grammar, and therefore its own card |
| `NATIVE-LIBRARY-HEADER-CONDITIONAL` | evaluating the bounded conditional subset an exported header actually uses (`#ifdef __cplusplus`, feature guards) rather than refusing every symbol declared only inside one |
| `NATIVE-LIBRARY-HEADER-ONE-READER` | this reader and the gateway's are two readers of one vocabulary, separated by the crate layering (`qcore-core` has no dependencies; the gateway's reader is above the contract loader). The divergences are censused in the card report §7; retiring either one requires invariant 3 parity first |
| `NATIVE-LIBRARY-CERT-REPLAY` | unchanged from wave 20, and now with a second identity to replay against |

Buffer extents, struct/string/callback transport, sentinel adapters, extern
calls in concurrent or error-region contexts, target-aware scalar projection,
dynamic loading and verifying the link step stay out of scope exactly as they
are for every other level of this profile.
