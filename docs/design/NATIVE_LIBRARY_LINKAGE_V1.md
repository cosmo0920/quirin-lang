<!-- Adopted, with recorded adjustments, by card `UNSAFE-LINKAGE` (wave 18). -->
<!-- Partly implemented by card `UNSAFE-LINKAGE-ARTIFACT` (wave 19); see the
     "Landed since" note below. -->

> **Landed since (card `UNSAFE-LINKAGE-ARTIFACT`, wave 19).** The **contract-less
> slice** of stage (a) is implemented:
> [`crates/qcore-core/src/native_library.rs`](../../crates/qcore-core/src/native_library.rs)
> loads a manifest total-or-reject with the artifact identity of §4.3 and the
> four ABI premise fields of §4.4, binds symbols through §4.1's
> may-not-bind-twice rule (extended across a selection), and applies §4.2's type
> gate to a waived declaration. The new pass `NativeLibrary` owns the reserved
> band as **`Q039`–`Q042`**. Status is therefore no longer "scoped only" as a
> whole; it is **scoped only except for the contract-less slice**, and the
> difference is exactly:
>
> | §4 piece | State |
> |---|---|
> | 4.1 symbol table, for **contract-less** entries | landed (a `contract fn` entry is a named rejection) |
> | 4.1 contract slice binding, in both directions | **not implemented** — follow-on `NATIVE-LIBRARY-CONTRACT-SLICE` |
> | 4.2 header agreement | landed as a **refusal** for a contract-less entry, not a withdrawal (see the report's §4 for why the two differ here); reading a real `.h` is follow-on `NATIVE-LIBRARY-HEADER` |
> | 4.3 artifact identity | landed as a **stated pin**: well-formedness checked, nothing hashed, nothing read from disk (report §5) |
> | 4.4 ABI premises | landed as four mandatory closed-vocabulary fields carried onto the certificate |
> | 4.4 closed-allocator-custody structural check | vacuous by construction — a contract-less entry cannot acquire (`UNSAFE_LINKAGE_V1.md` §9.1 point 2) |
> | §6 `std.buffer`, §12 stages (b)–(d) | untouched |
>
> Evidence: [`docs/reports/wave19-unsafe-linkage-artifact/README.md`](../reports/wave19-unsafe-linkage-artifact/README.md).

# Native-library linkage for bundled modules: `source.native-library.v1`

> **Provenance.** This document was authored in a prior workflow, on branch
> `claude/qcore-c-abi-linking-c8c443` at commit `e48786c` (ledger framing at
> `ecede86`). Card `UNSAFE-LINKAGE` adopts it into the tree, unchanged except
> where the wave-17 tree contradicts it; every adjustment is recorded in §0
> below and marked **[adjusted]** at its site. Its status is unchanged: **scoped
> only**, a design note with no implementation. The two designs are the two ends
> of one linkage spectrum — see
> [`UNSAFE_LINKAGE_V1.md`](UNSAFE_LINKAGE_V1.md) §2 for the spectrum and §9 for
> the reconciliation.
>
> Its accompanying ledger edits (`AGENTS.md`, `DEVELOPMENT.md`,
> `docs/reviews/r0_3/acceptance_matrix.json`) are **deliberately not carried**:
> acceptance and status files are integrator surface, and the adopting card does
> not edit them. Its `named_followups` entry `NATIVE-LIBRARY-LINKAGE` remains for
> the integrator to enter.

## 0. Adjustments on adoption

Four, all specifics that the tree moved under it between `e48786c` and wave 18.
Nothing about the design's substance is changed.

| # | Where | As written | Adjusted to | Why |
|---|---|---|---|---|
| A1 | §1, §11 | "the five-name … precedent" | **six** names | card `UNSAFE-LINKAGE` added `std.extern.unsafe` to the registry. |
| A2 | §7 | "the first free band after `Q031`" | **`Q039` onward** | card `UNSAFE-LINKAGE` took `Q032`–`Q035` for its `UncheckedLinkage` pass, and card `PIPE-OPERATOR` then took `Q036`–`Q038` for its `Pipe` pass at wave-18 integration. Adjusted twice for the same reason; this is the only load-bearing collision between the two documents. |
| A3 | §9 | "the 67-module corpus pin" | **69 modules** | card `CONSTRUCTOR-RESIDUALS` moved the pin 67 → 69 in wave 17. |
| A4 | §2, §4.2, §4.3, §4.4 | "constraint 8 / 18 / 28" | kept, with referents named | the numbering is the authoring workflow's, not this repository's. The referents are: **8** = `LOADER-ABI`'s declared-ABI agreement with per-unit withdrawal (`docs/reports/remaining-loader-abi/README.md`); **18** = `P01-F2`'s embedded-manifest field-for-field package verification (`docs/reports/remaining-p01-f2/README.md`); **28** = the declared-assumption discipline of hard invariant 2. Left in place rather than rewritten so the adopted text stays comparable to its original. |

One thing was checked and found **not** to need adjustment: §4.4's
`single-threaded-caller` premise is still vacuous for exactly the reason it
states — extern calls remain refused in concurrent contexts on both routes.

**Status:** scoped only. No production file changes with this document; no
checker, backend, contract-loader, packaging or Lean semantics change on its
account. No verdict, CT mapping, contract-corpus count or raw Cycle 19 unit
changes.

This document scopes the concept that turns the extern-call profile's
**builder's premise** — "symbol resolution/linkage is the builder's premise:
the increment claims nothing about which object file provides the symbol"
(`EXTERN_CALL_SEMANTICS_V1.md` §3.5) — into a **declared, checkable linkage
unit**, so that a bundled capability module (working first target
`std.buffer`; aspirational target `std.http`) can be *implemented by* a native
compiled library exposing a C ABI. "Linkage verification" has been carried as
a named follow-on by every card in the extern-call family
(`extern-call-v1` §8, `remaining-extern-modes` §13); this is that follow-on's
design.

The implementation language of the native library — C, or Rust compiled to a
C-ABI artifact — is **deliberately invisible at the boundary**. The boundary
admits only the C04 ABI projection, so a Rust `staticlib`/`cdylib` with
`extern "C"` exports and a hand-written C library are the same object from the
checker's point of view. What differs is the set of *premises* the library's
manifest must state (§5).

## 1. The concept in one paragraph

A **native-backed module** is a bundled capability module in the existing
registry sense (`import std.buffer;`, following the six-name **[adjusted, A1]**
`std.concurrency`/`std.extern`/`std.extern.unsafe`/`std.fmt`/`std.io`/`std.logger`
precedent, with
the same `Q014`/`Q015` file-local gating) whose extern surface does not float
free but is bound by a **library manifest** to one named native library. The
manifest binds four things together: (i) a slice of declared `.qcore`
contracts in the unchanged C04 vocabulary, (ii) the C header the library
exports, (iii) the library artifact's identity (name, version, content hash),
and (iv) the ABI premises the artifact was built under (data model, unwinding,
allocator custody). Source code then reaches the mechanism through the
**unchanged** `source.extern-call.v1` call semantics — this profile adds no
new call form, no new bridging mode and no new adapter. What it adds is that
the *selection* of the extern environment stops being an ad-hoc
`CompileOptions` list and becomes a named, pinned, verifiable unit — the
"short-circuit": the mechanism (buffer arithmetic, and later HTTP transport)
lives in audited native code, and QCore verifies the *custody protocol* at the
boundary rather than re-deriving the mechanism from generated C11.

Everything the extern-call family established about trust is inherited
verbatim: every contract in the slice is `ContractTrust::Declared`, every
downstream verdict is valid *relative to* the declarations, and the library's
conformance to its own contracts is a premise stated in the theorems — never
a proof, never an upstream claim (hard invariant 2).

## 2. What already exists (reused, not rebuilt)

| Piece | Where | Reuse |
|---|---|---|
| Call semantics, adapters, effect rows, refusal tables | `source.extern-call.v1` + `EXTERN-F1`/`EXTERN-ERR-REL`/`EXTERN-MODES` | the *only* call boundary; unchanged |
| C04 total-or-reject loader and ABI projection | `qcore-contract::contract_loader` | the *only* contract vocabulary; unchanged |
| Bundled-module registry and import gate | `qcore_core::module_imports`, `Q014`/`Q015` | one new name per native-backed module |
| Declaration-vs-declaration agreement with withdrawal | `remaining-loader-abi` (constraint 8) | the model for header agreement (§4.2) |
| LP64 scalar-projection premise | `LOADER-ABI-LP64` | becomes an *explicit manifest field* instead of an implicit reader assumption (§4.4) |
| Embedded-manifest, field-for-field package verification | `providers/package.py`, constraint 18 | the model for library-artifact identity (§4.3) |
| Native sanitizer harness linking a real C implementation | `compiler/tests/extern_calls.rs` (`buffer_api.c`) | grows into the per-library conformance observation harness (§6) |
| Certificate rederivation of selections | `Q027` | extended to reproduce the manifest binding |

## 3. Goal and non-goals

**Goal.** A source file that imports a native-backed module gets, fail-closed:

1. the module's contract slice as its extern environment — no per-invocation
   `CompileOptions` assembly, no way to pair the module's declarations with a
   different library's symbols;
2. a checked agreement between each declared contract's ABI projection and the
   library's own exported header declaration for that symbol, with the
   `remaining-loader-abi` discipline on disagreement: the declaration is
   **withdrawn for that unit**, call sites become uncontracted (refusing at
   `Q023`-class), never silently reinterpreted;
3. a recorded artifact identity, so a certificate names *which* library build
   the result is relative to, and replay against a different build refuses;
4. the ABI premises as named manifest fields a certificate carries, instead of
   unstated assumptions.

**Non-goals, stated as loudly as the goals.** This profile does **not** prove
the library correct, does not observe the library's runtime behaviour beyond
the fixed sanitizer harness, does not admit any callee shape the extern-call
family refuses today, and does not claim the linker resolved the symbol to the
manifest's artifact — the *link step* stays the builder's act; what changes is
that the certificate now states exactly which artifact that act was obligated
to use, so a substitution is a detectable premise violation rather than an
invisible one.

## 4. The library manifest (the substance of the card)

One declarative file per native-backed module (working format: the same
total-or-reject loading discipline as C04 — any unrecognized field or missing
section rejects the whole manifest; no partial admission).

### 4.1 Symbol table

Each entry binds `extern fn` name → contract (by content, within the module's
slice) → exported header declaration. The binding is total in both directions:
a contract with no symbol entry rejects the manifest, and a symbol entry whose
name is absent from the slice rejects it too. A manifest may not bind a name
twice, and two modules may not bind the same name (the existing duplicate-name
loader refusal, re-pinned at this boundary).

### 4.2 Header agreement

The check follows constraint 8's model exactly, including where it reads from:
both sides are compared as *declaration text* — the contract's ABI projection
against the header's C declaration — using a reader deliberate about being
reproducible at certificate recheck without a typed Clang AST. Disagreement is
a **withdrawal for the consuming unit** with its own diagnostic, monotone in
the refusing direction. The known consequence carries over: a withdrawal can
turn a previously checkable program into a refusing one, and that is
`Unknown`-class evidence about the declaration, never a defect claim about the
library.

### 4.3 Artifact identity

Name, version, and content hash of the library artifact (static archive or
shared object), following the provider-package precedent: the manifest is the
template, and a verification step compares the installed artifact
field-for-field/hash-exact, refusing on any mismatch (constraint 18's
discipline applied to libraries). The certificate records the identity
triple; `Q027`-class rederivation refuses a replay whose manifest, slice or
artifact hash differs from what the certificate names.

### 4.4 ABI premises

Named fields, all mandatory, none checked beyond well-formedness — these are
**premises the certificate carries**, in the constraint-28 sense:

- **Data model:** `lp64` is the only admitted value in v1, making
  `LOADER-ABI-LP64` an explicit statement instead of an implicit reader
  assumption. A manifest declaring anything else rejects; making the scalar
  projection genuinely target-aware remains that card's own follow-on.
- **No unwinding across the boundary:** the library promises its entry points
  terminate or return — for a Rust implementation this is the
  `panic = "abort"` (or caught-at-the-boundary) obligation, since unwinding
  through a C ABI frame is undefined behaviour. Stated, not checked.
- **Closed allocator custody:** every allocation the library hands out is
  released only through the library's own declared release entry points. This
  is what makes a Rust implementation honest — memory from the Rust allocator
  must never meet C `free` — and it is already the shape the extern-call
  fixtures model (`qcore_extern_alloc`/`qcore_extern_free` pairs). The
  contract slice makes it structural: a module whose slice contains an
  acquiring declaration but no consuming/releasing declaration for the same
  resource shape rejects at manifest load, because such a module could only
  ever leak or lie.
- **Thread posture:** v1 admits only `single-threaded-caller` (extern calls
  remain refused in concurrent contexts by the unchanged `Q026`-class rules,
  so this premise is currently vacuous but stated, ready for the day the
  concurrency refusal is lifted by its own card).

## 5. Rust as the implementation language

Nothing at the boundary distinguishes it, so this section is short and is a
statement about *packaging*, not checking: a Rust-implemented module ships as
a C-ABI artifact (`crate-type = ["staticlib"]` or `cdylib`) whose exported
symbols are `#[no_mangle] extern "C"` functions taking exactly the C04
projections (`*mut u8`, `*const u8`, `usize`, fixed-width scalars). The §4.4
premises are the Rust-specific obligations, and the manifest is where they
are stated. The payoff of the arrangement is the intended one: the library's
*internal* memory safety can come from Rust itself, while QCore checks the
*protocol* — who owns what, when, across the boundary — which is precisely
what Rust's own type system stops expressing once symbols cross `extern "C"`.

## 6. First target: `std.buffer` — and why `std.http` is staged, not scoped

**`std.buffer` v1** is deliberately little more than the existing fixture
surface, made real: acquire (nullable owned return), fill/read (borrowed),
grow (reallocated, both arms), release (consumed/released) — every mode
already callable today, exercised by `extern_calls.rs` against a linked C
implementation with ASan/UBSan/LSan. The increment's observable delta is the
manifest machinery of §4 plus one new registry name; the *call-level*
semantics move not at all. That is what makes it the right first card: it
isolates the new trust object (the manifest) from any new call semantics.

The honest limitation is inherited and must be restated in the module's
documentation: **extent is not transported** (C04 pointers are extentless
`u8` projections), so v1 `std.buffer` contracts cannot promise how many bytes
a callee touches — that stays a premise of the declared contract. A
`std.buffer` worth *recommending* (rather than one that exists to carry the
manifest machinery) needs the buffer-extent follow-on, which is therefore
stage (b), not an afterthought.

**`std.http` is a direction, not a v1 deliverable**, and this document
refuses to pretend otherwise. An HTTP surface needs, at minimum: struct
transport (requests/responses are not `u8` buffers), sentinel/errno adapters
(`EXTERN-MODES-SENTINEL`, with its three recorded constraints), callbacks or
a polling shape (function pointers are a named exclusion), string/extent
transport, and — for anything concurrent — lifting the extern-in-concurrent
refusal, which interacts with a separate checked model and is its own card
per the extern-call design. Each of those is an existing named follow-on with
its own owner. What this profile contributes to that future is exactly the
manifest: when those cards land, an HTTP library binds through the same §4
machinery with zero new trust concepts.

## 7. Diagnostics

New codes in the first free band, **`Q039` onward** **[adjusted, A2]**
(**taken**: card `UNSAFE-LINKAGE-ARTIFACT` allocated `Q039`–`Q042` to the new
`NativeLibrary` pass through the §L21 process; a contract-slice card continues
from `Q043`) (the
`Q028`–`Q031` band is owned by `StandardIo`, `Q032`–`Q035` by card
`UNSAFE-LINKAGE`'s `UncheckedLinkage` pass, and `Q036`–`Q038` by card
`PIPE-OPERATOR`'s `Pipe` pass), owned by a new semantic pass entry
through the §L21 process, in the same change that lands the code. Candidate conditions, one
code each: manifest missing/rejecting for an imported native-backed module;
manifest↔slice binding failure (either direction of §4.1); header
disagreement withdrawal (§4.2 — distinct from the binding failure because it
is per-unit and monotone, not a load rejection); artifact identity mismatch
(§4.3); certificate replay against a differing manifest/artifact. No existing
code changes; `Q022`–`Q027` keep their exact meanings, and in particular
`Q023` remains the "no admitted contract" refusal that a §4.2 withdrawal
routes into.

## 8. Lean obligations

Per the closing standard (Rust + audited Lean; Python auxiliary):

- the manifest binding is a function: same slice + same header + same
  identity → same admitted environment, so certificate rederivation repeats
  it independently;
- withdrawal monotonicity: a §4.2 disagreement never widens the admitted
  environment, and composition with the existing extern-call theorems
  preserves ledger accounting under the (stated) library-conformance premise;
- refusal completeness over the manifest grammar: every non-admitted field
  value and every binding-totality violation refuses by name;
- the closed-custody structural check of §4.4 is sound: a slice admitted by
  it contains a discharge path for every acquisition shape it can establish;
- no new axioms; a new `Audit*.lean` registered with
  `scripts/check_lean_audits.py`. The library's conformance to its declared
  contracts and the linker's use of the named artifact remain explicit
  premises, stated in the theorems.

## 9. Tests

Positive: the fixture library rebound through a manifest, all existing
accepted fixtures re-accepted byte-identically; a Rust `staticlib`
implementation of the same surface passing the same native harness (the
implementation-language-invisibility claim, made executable); certificate
rederivation across a rebuild with identical hash. Negative/mutation: each §7
refusal; a manifest binding a name absent from the slice and vice versa; an
edited header flipping one projection (must withdraw, and the withdrawal must
be able to turn an accepted program into a refusal — pinned, since that
direction is the one a lazy implementation would lose); an artifact hash
mismatch after relink; a second module binding an already-bound name.
Manifest and contract fixtures live under test fixtures, **not**
`contract/qcore/`, so the 69-module corpus pin **[adjusted, A3]** is untouched.

## 10. Out of scope (named follow-ons, not omissions)

Buffer extents (stage (b), prerequisite for a recommendable `std.buffer`);
struct, string, callback and function-pointer transport; sentinel/errno
adapters (`EXTERN-MODES-SENTINEL`); extern calls in concurrent or
error-region contexts; target-aware scalar projection (`LOADER-ABI-LP64`'s
own follow-on — this profile only makes the premise explicit); dynamic
loading, `dlopen`, or any runtime symbol resolution; cross-compilation
manifests; verifying the *link step* itself (the certificate names the
obligated artifact; confirming the linker honored it is builder-side
evidence, at most an observation); any `std.http` surface; any gateway,
audit-route or Cycle 19 interaction (untouched by construction); CT-manifest
changes.

## 11. Acceptance gate

The standing per-change gate, plus: all pinned 185-unit artifacts untouched
by construction (assert via the diff); contract corpus count unchanged; no
raw unit promoted; spec §L21, the module-registry section (the six-name list
**[adjusted, A1]** grows deliberately, in the same change), and
`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` updated together so no statement goes
silently stale; `EXTERN_CALL_SEMANTICS_V1.md` §7 gains a "landed since" note
retiring the "linkage verification" follow-on *only* for what §4 actually
covers, with the link-step caveat of §10 restated there.

## 12. Suggested staging

- **(a)** Manifest format + loader + binding/withdrawal semantics + registry
  name + Lean, over the existing fixture C library — no new call semantics,
  no new callable mode, independently gateable; the Rust `staticlib` twin of
  the fixture library rides in this stage as the invisibility proof.
- **(b)** Buffer-extent transport (its own design note first; it changes C04
  vocabulary and therefore the loader, which this profile deliberately does
  not).
- **(c)** `std.buffer` as a shipped module: contract slice, header, prebuilt
  artifact discipline through the provider-package machinery.
- **(d+)** The `std.http` prerequisite chain of §6, each its own card.

Stage (a) alone changes no observable execution for any existing program and
is the polishing increment this document exists to scope.
