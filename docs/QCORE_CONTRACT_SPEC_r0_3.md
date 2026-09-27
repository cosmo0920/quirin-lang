# Quirin contract specification — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the legacy `.qcore` alias and this file's name —
> and those component identifiers are unchanged by the branding. Current contract files use `.qv`, runtime sources use `.qr`, and native ABI manifests use `.qrlib`; legacy suffixes remain warned compatibility aliases.

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

**Revision date:** 2026-09-07, Asia/Tokyo.  
**Status:** Source-reviewed contract semantics, consumer boundaries, and implementation requirements.  
**Input identity:** `qcore-c.tar.gz`, SHA-256 `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.

## C01. Scope and authority

This document accompanies the [language specification](QCORE_SPEC_r0_3.md). Together they supersede conflicting claims that all `.qv` and legacy `.qcore` contracts have the same interpreter or the same proof strength. It specifies what a contract means, which current consumer can interpret it, which inputs are trusted, and what a result may claim. It does not change protocol version 1, resource-contract marker version 1, trace schemas, or compiler package versions.

`MUST` and `MUST NOT` are conformance requirements, not assertions that every reviewed implementation already satisfies them. The [review ledger](reviews/r0_3/REVIEW.md) identifies differences. The [implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) are the grammar reference for current consumers. The extended language BNF is a design target, not the grammar of the resource loader.

A contract is a relation between explicitly identified preconditions, an operation, and possible exit states. A contract is not proved merely because it was loaded. A named proposition is not true merely because its name resembles a theorem. The boundary between trusted declaration, source-derived summary, evaluated observation, and checked certificate MUST remain visible through composition.

## C02. Consumer selection and clause coverage

The current repository shares one structural syntax among several consumers. Their semantic vocabularies are intentionally different.

| Consumer | Selection and accepted payload | Meaning of success |
|---|---|---|
| Structural source frontend | A source module and declaration envelopes; preserves `requires`, `ensures`, and `quant` clauses. | Syntax and selected frontend checks succeeded. General predicates are not thereby proved. |
| External resource loader v1 | Explicit loader invocation; every declaration has exactly one `requires resource_contract_v1;` marker and only the admitted `resource_*` postconditions. | A complete, trusted external custody table was constructed. |
| Observation interpreter | Explicit invocation with source, a function name, and an integer fact map. | All relevant preconditions hold and all supported normal postconditions evaluate true for those supplied facts. |
| Chunk/source-bound interpreters | Explicit Rust adapters obtain restricted source summaries and bind them to selected policies. | The specific imported source facts meet that adapter's policy under its stated premises. |
| Mechanism/lifecycle interpreters | Explicit adapters derive or supply lifecycle facts for an identified model or execution. | Acceptance within that model or observation, not proof of arbitrary source. |
| Verifier host v1 | Resolved plugin manifest, request and response envelopes. | Transport, targeting and envelope checks only; no accepted v1 Safe certificate. |

A caller MUST select the interpreter deliberately. The v1 marker is mandatory within the resource loader; it does not make the ordinary `quirin check` command automatically dispatch every contract to that loader. Current verifier scopes enter bounded core derivation/dispatch; unsupported lifecycle source effects still leave obligations incomplete.

A verification result MUST account for every clause relevant to the requested property. Clauses can be retained without interpretation by an editor or structural tool, but the output MUST NOT imply that retained clauses were discharged. A consumer MUST reject an unsupported semantic clause, return Unknown for its requested property, or explicitly limit its claim to a different covered property. Silently deleting a clause and publishing the original full-contract claim is forbidden.

The ordinary checker now rejects nonempty source function contract clauses with Q021 until enforced. Extern envelopes remain structural unless an explicit consumer is invoked. Neither structural retention nor Q021 rejection proves arbitrary postconditions. See R08.

## C03. Common source envelope

A contract module normally begins with `module contract;`. It has no executable entry and does not synthesize a `main` function. The current resource loader requires that mode. The generic observation API itself does not require the explicit header; existing compatibility inputs may omit it. New reusable policy files SHOULD state the header explicitly.

A function contract is attached to an extern or defined function declaration. The structural parser retains clause expressions as source text and byte spans:

```qcore
module contract;
extern fn observe(result: i32) -> unit
contract {
    requires input_is_valid;
    ensures normal result >= 0;
};
```

This example has an observation-shaped predicate, not a resource-v1 declaration. Which identifiers are bound, what values are supplied, and whether `input_is_valid` has source evidence depend on the selected consumer. A structural parse does not supply those facts.

The shared envelopes are `requires expression;`, `ensures selector expression;`, and `quant selector expression;`. Selectors are `normal`, `any`, and `error(E)` with nonempty type text. Structural retention of `any`, a typed error, or a quantitative clause does not extend the narrower observation or resource-v1 vocabularies. Those consumers reject unsupported forms.

The source parser rejects empty clauses and unknown clause introducers. It does not implement a general theorem prover for the retained expression text. Language-level effects, `own`, `borrow`, usage annotations, access requirements and task protocols need their specified source/model elaborators; the external resource loader is not a fallback elaborator for them.

## C04. Resource-contract v1: total-or-reject loading

The implementation is `crates/qcore-contract/src/contract_loader.rs`. The public entry points load file lists, an immutable map of source texts, or a versioned manifest. The result is a `ResourceContracts` environment from `qcore-schemas`.

The selected source set MUST be nonempty. Every selected source MUST parse as a nonempty contract module. Every declaration MUST be an extern function, and function names MUST be unique across the complete selection. No partial environment escapes if a selected declaration fails. The loader does not skip observation modules, resource types, constants, errors, or unsupported lifecycle functions.

Every extern declaration MUST satisfy all of the following:

1. Its entire signature belongs to the admitted ABI projection, with unique parameter names and no usage annotations, generics, effects, extra signature syntax, or ignored tail tokens.
2. It has exactly one `requires resource_contract_v1;` marker. The only other admitted precondition is the literal-argument premise of C04.5; every other `requires` spelling is rejected.
3. Every other clause is an admitted `ensures normal` predicate call. Each parameter receives exactly one mode and the result receives exactly one result mode.
4. Parameter/result types, nullability arguments, ownership references and return references meet the compatibility rules below.

Reconstructing and comparing the whole declaration prevents a recognized subset from masking additional syntax. Predicate calls are exact identifier/comma forms; arbitrary arithmetic, nesting, trailing commas and general Boolean combinations are not admitted.

The marker is an interpreter/version selector, not a Boolean fact supplied by a caller. It is not evaluated by the resource loader as an ordinary observation precondition.

### C04.1. ABI projection

The scalar parameter vocabulary is exactly:

```text
bool i8 i16 i32 i64 isize u8 u16 u32 u64 usize f32 f64
```

The pointer vocabulary is exactly:

```qcore
ptr<mut, u8>
ptr<const, u8>
ptr<mut, ptr<mut, u8>>
```

The function-pointer vocabulary is exactly:

```text
fnptr
```

`fnptr` is the projection of a pointer to function (card
CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF). It is deliberately **not** a member of the
pointer vocabulary and is deliberately not spelled `ptr<…>`: a function
designator is never a resource identity, so `fnptr` MUST carry no custody, and
the mode rules below — which key on an admitted *pointer* type — deliver that
without a rule of its own. It MUST NOT be folded onto `usize`/`u64` even where
the target's data model gives them the same width, because the declared
spelling is compared against the analysed unit's C declaration and the two
readings are different readings. A consumer comparing projections MUST refuse
`fnptr` for a C type that is not a pointer to function, and MUST refuse every
other spelling for one that is. `fnptr` does not distinguish two function
pointers of different signatures, exactly as `ptr<mut, u8>` does not distinguish
two object pointers of different pointee types.

The variable-argument-list vocabulary is exactly:

```text
va_list
```

`va_list` is the projection of the compiler's `__builtin_va_list` at a
parameter (card W44-LIBC-RESIDUALS), the type glibc's `__gnuc_va_list` and
`va_list` name. It names the type, not its target-dependent representation (an
object pointer after array adjustment on x86-64, a structure on AArch64), so it
MUST NOT be folded onto a pointer or integer spelling. Like `fnptr` it is not
spelled `ptr<…>` and MUST carry no custody. It is a **parameter** type only: it
is not in the result vocabulary. A consumer comparing projections MUST refuse
`va_list` for a C type that is not the builtin, MUST refuse every other
spelling for one that is, and MUST keep refusing a pointer to it and a result
of it. The Lean model is `QCore.GnuBuiltinReader`.

The result vocabulary adds `unit` to the parameter vocabulary, less `va_list`. `unit` is not a parameter type. Arbitrary named types, general pointer nesting, `own<T>`, `borrow<...>`, and language-level `result<T,E>` are not accepted by this loader. The two real-valued scalars are admitted for readings a resource API in scope passes by value; they carry no custody, exactly as every other scalar does; `fnptr` was admitted on the same condition, for msgpack's `msgpack_packer_init` in the pinned Cycle 19 corpus.

**Agreement with the analysed source (card LOADER-ABI).** A consumer analysing a translation unit MUST compare a declaration's projections against that unit's own C declaration of the same function wherever it can read one, and MUST withdraw the declaration for that unit — at every call site, and as the interface any body defined there is checked against — when the two disagree in the result type, a parameter type, the parameter count, or a claimed variadic tail. Withdrawal makes the callee uncontracted, which is the refusing direction; a consumer may never repair a disagreement by preferring either side. The projections are not decoration: the mode admissibility rules of this section are keyed on them, so an unfaithful projection is what would buy a mode the C type could not carry.

**Agreement with a derived summary (card LOADER-ABI-MODES).** Where a consumer independently derives a *validated* custody summary from a contracted function's own body in the analysed translation unit, it MUST compare the declared custody duties against the derived ones and MUST withdraw the declaration for that unit, at every call site, when they contradict. A parameter's duty is `retained` for `resource_owned` and `resource_borrowed`, `transferred` for `resource_released`, `resource_consumed` and `resource_stored`, `delivered` for `resource_out` and `restored` for `resource_removed`; a result's duty is `produced` for an owned or reallocated result and absent otherwise. A contradiction is a duty inequality at some position and nothing else: nullability, a `resource_stored`/`resource_removed` owner parameter and a reallocated result's replaced parameter are refinements within one duty and MUST NOT withdraw on their own. A consumer MUST NOT require a derived summary before applying a declaration — absence of evidence is not evidence, and a callee defined in another translation unit is never compared — and MUST NOT install the derived summary in the declaration's place, because a derived summary may be weaker than the declaration it contradicts. Withdrawal makes the callee uncontracted, which is the refusing direction.

**Qualifiers after a parameter's last `*` (card W43-LIBC-RESULT-ALIAS).** A qualifier that follows a parameter declarator's last `*` — `const char *__restrict __format`, `void *const p` — qualifies the parameter object, and C drops it for type compatibility (C11 6.7.6.3p15). A consumer reading the C side MUST read such a parameter as its unqualified type and MUST keep refusing a qualifier *between* two `*`, which qualifies an inner pointer this vocabulary cannot spell. A result declarator is not a parameter and is not widened. The Lean model is `QCore.PointerQualifierReader`.

These are opaque interface projections. They do not check actual C layouts, ABI compatibility, pointee validity, allocation size, alignment, aliasing, or thread-safety. A scalar carries a descriptor number as data unless its position declares the `descriptor` kind (§C04.6), which is the one way a scalar carries custody. Otherwise only `resource_borrowed` may apply to a scalar, to `fnptr` or to `va_list`. Every other parameter mode requires an admitted pointer type or an `i32` of the `descriptor` kind; `resource_out` specifically requires `ptr<mut, ptr<mut, u8>>`. An owned result requires an admitted pointer result or an `i32` of the `descriptor` kind, and a reallocated result an admitted pointer result.

### C04.2. Parameter modes

In the following table, `p` identifies a parameter and `n` is exactly `nullable` or `nonnull`.

| Predicate | Custody interpretation in the imported resource model |
|---|---|
| `resource_borrowed(p)` | The call imposes no custody precondition on `p` and does not transfer a tracked obligation through it. This is not the full QCore lifetime-checked borrow type. |
| `resource_owned(p, n)` | The caller supplies the required live custody, and the call retains it in the modeled parameter disposition. |
| `resource_consumed(p, n)` | The call transfers the obligation away from the caller. Consumption is not a proof of destruction or endpoint release. |
| `resource_released(p, n)` | The trusted call contract discharges the resource obligation. A false declaration here can invalidate a safety conclusion. |
| `resource_out(p, n)` | The operation establishes fresh custody through the modeled output destination. It requires the pointer-to-pointer ABI projection. |
| `resource_stored(p, owner, n)` | Custody of `p` becomes indexed beneath the designated owner in the abstract resource model. |
| `resource_removed(p, owner, n)` | The designated resource is removed from owner-indexed custody and becomes directly held according to the modeled call. |

For stored/removed modes, `owner` MUST name a distinct parameter whose mode is `Owned`. The relation is an identity-bearing owner/child relation, not equality of aggregate counters. A declaration that a parent releases owned children is a trusted model of cascading destruction, not a proof that an arbitrary C destructor actually traverses or releases them.

A nullable pointer does not create a live obligation when the modeled value is null. It does not encode every possible failure result. Nullability is not equivalent to an arbitrary success predicate over integer returns, output arguments, `errno`, or non-null sentinels.

`resource_borrowed(p)` states what the *callee* demands, and it demands nothing. Independently of it, the analyzer imposes one **caller-side** refusal at a borrowed position (card `ALIAS-LOCALS-BORROW-LIVENESS`): it MUST NOT certify a body that supplies an actual its own projection has settled on a single identity released on every reaching path. This is a refusal to decide, published as an unresolved result under the existing unresolved-identity diagnostic, and it is deliberately not a violation claim: it establishes neither that the callee dereferences the actual nor that the path is feasible. A scalar actual — the only non-pointer this mode admits — an unresolved, ambiguous or possibly-null binding, an identity whose disposition differs across reaching paths, and a consumed identity whose custody moved without a declared destruction are all unaffected and keep their existing answers.

### C04.3. Result modes

Exactly one result predicate is required:

| Predicate | Interpretation |
|---|---|
| `resource_return_none()` | No fresh or replacement custody is returned. The ordinary ABI result may still carry scalar data. |
| `resource_return_owned(n)` | A non-null modeled result establishes fresh custody; the declared nullability controls the null branch. |
| `resource_return_reallocated(p, n)` | The incoming `p` MUST be `Owned`. A null failure preserves that incoming custody; a successful replacement consumes/replaces the old resource exactly once and establishes the result identity. |
| `resource_return_alias(p, n)` | The result is the value of `p` itself (`memcpy` returns `dst`). With `nullable` the call may return NULL instead; with `nonnull` it never substitutes NULL. The result owns nothing. See §C04.3.1. |
| `resource_return_interior(p, n)` | The result points into the object `p` points to (`strchr`, `memchr`), or is NULL when `n` is `nullable`. The result owns nothing. See §C04.3.1. |

The analyzer MUST retain the result-dependent alternatives until control-flow facts resolve them. It MUST NOT treat a possible null realloc result as proof that the old resource was released. Reallocation semantics include identity replacement, not just a zero aggregate net count.

A valid minimal resource declaration is:

```qcore
module contract;
extern fn allocate(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn release(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
```

This file asserts a trusted external API model. It does not verify the implementation of `allocate` or `release`.

#### C04.3.1. Derived results

Card W43-LIBC-RESULT-ALIAS added the two derived-result predicates. Both load to one mode, `ResourceReturnMode::Derived { parameter, extent, nullable, const_dropped }`, with `extent` `Exact` for `resource_return_alias` and `Interior` for `resource_return_interior`.

**Placement.** `p` MUST name a `resource_borrowed` parameter whose type is `ptr<mut, u8>` or `ptr<const, u8>`, and the result type MUST be one of those two. Every other placement — a custody-bearing base, a scalar, `ptr<mut, ptr<mut, u8>>`, `fnptr`, a scalar or `unit` result — rejects the whole module. `const_dropped` is set exactly when the base is `ptr<const, u8>` and the result `ptr<mut, u8>` (`strchr`: `const char *` in, `char *` out); it is recorded, not inferred from a name.

**Semantics.** A consumer MUST apply all of the following, and MUST NOT approximate any of them:

1. **Non-owning.** The result acquires, discharges and transfers nothing. A discarded derived result is exactly a discarded `resource_return_none()`.
2. **Releasing the derived result is not a release of the base.** The base's obligation stays exactly as it was. A consumer MUST NOT credit such a release, a consumption, a store, a removal, a reallocation or an escape (a `return`) of the derived result to the base. Handing an **interior** result that resolves to one identity to a release is a violation *candidate* (an invalid free: C11 7.22.3.3p2 requires the value an allocator returned). Every other such position — an exact alias, which *is* the base's value, an ambiguous or unresolved binding, and every non-release custody position — is a refusal.
3. **Releasing the base invalidates every pointer derived from it.** A later use of the derived result at any position of a contracted call other than an out slot, or a release or return of it, on a path where the base was released, is a violation candidate: the value of a pointer to an object whose lifetime has ended is indeterminate (C11 6.2.4p2).
4. **Nullness is the result's own.** A null test of a derived result MUST narrow only that result: it establishes nothing about the base's existence or nullness, and the base's nullness establishes nothing about it.
5. **The base must be denotable.** A consumer binding a derived result MUST be able to name what its base actual denotes; the C gateway admits a place, a place plus or minus one offset token (defined pointer arithmetic stays inside the same object, C11 6.5.6p8; since card W43-RESULT-ALIAS-RETURNED such an offset base is read at the **interior** extent whatever the declaration says, because `memcpy(p + 1, …)` returns `p + 1`, not `p`), a string literal and the address of a plain name (neither is a tracked identity), and refuses to bind a result over any other base expression.
6. **`const_dropped` is carried with the result**, so a later check can see that a write through it would write through a pointer the base declared `const`. The v1 vocabulary has no per-parameter read/write distinction, so no consumer in this repository yet refuses such a write through a borrowed position (`W43-RESULT-ALIAS-CONST-WRITE`).

**Verdict contract, unchanged.** A candidate becomes `Unsafe` only under the existing provisional-`Unsafe` policy and is never presented as an upstream defect: the analyzer does not know an interior pointer's offset, and an interior pointer that happens to sit at offset zero would be a valid release. Everything unproven stays `Unknown`. A body *defined* in the analysed unit is checked against a derived result in force for it (declared or inferred) by the rule of §C04.3.2, and falls back to the contract-free analysis when that check fails (card W43-RESULT-ALIAS-RETURNED; before it, such a body was always analysed contract-free). The checked extern-call route (`qcore-contract::extern_bridge`) has no derived-result constructor and refuses such a declaration rather than projecting it onto `resource_return_none()`.

**Versioning.** This is an addition to resource vocabulary version 1, exactly as the variadic data tail and the literal-argument premise were: the `resource_contract_v1` marker is unchanged, every module that loaded before loads to the same functions (the pinned corpus tables in `compiler/tests/qcore_contract_corpus.rs` are the authority), and a loader that predates the predicates rejects a module using them as an unsupported predicate rather than misreading it. No new evidence version is introduced: every published verdict already carries the exact trusted module texts in `source_assumptions` (and the Cycle 19 audit their digests), so the evidence identity names which declarations — and therefore which vocabulary — a verdict rests on. The Lean model is `QCore.DerivedResult`.

#### C04.3.2. Inferred derived results

Card W43-RESULT-ALIAS-RETURNED lets the gateway's W4 inference publish a derived result as an `InferredValidated` summary. The summary is `Derived { parameter: k, extent, nullable, const_dropped }`, exactly the record a declaration loads to. Callers therefore apply it through the rules of §C04.3.1 unchanged.

It is proposed only when the ordinary live-entry and removed-wrapper routes publish nothing, and only for a body with these properties, checked on a CFG that desugars a whole-call `return f(…)` of a derived result like any admitted return call:

1. **One borrowed base.** Formal `k` is bound to a *name-only* site: a target with no resource status. Every custody-bearing use of it therefore refuses. That covers a release, a consumption, an escape (including `return k;`), an `Owned` position, an aggregate owner and a bound read. The formal stays exactly as borrowed as the summary claims. No exit may observe a status or an unresolved effect on that site.
2. **Every exit is NULL or derived from `k`.** Each reachable exit returns either a null pointer constant or a derived binding over `k`'s site. At least one exit returns the latter. Any other exit refuses. That covers a fresh resource, an untracked or opaque pointer, no pointer, an ambiguous or mixed binding, and a pointer derived from another formal, a local, a global or a literal.
3. **The relation is read off the exits.**
   * `extent` is `Exact` only when every derived exit is exact.
   * `nullable` is set when some exit returns NULL or some derivation step may return NULL for a non-null base. This relative nullness is tracked apart from the base's own.
   * `const_dropped` is set when some step dropped `const`.
4. **The ordinary gate re-checks it.** The body must be fully modelled and defect-free under that entry. Every other formal's mode must be the one the ordinary least-binding search reads.

Each refused shape leaves the function without a summary, so its calls stay uncontracted: `Unknown`. The Lean model is `QCore.DerivedResultInference`. It proves the gate sound against the contract meaning, and proves that each refused shape is refused. Releasing an exact alias is still refused rather than credited to the base, on the ground recorded in card W43-RESULT-ALIAS-RETURNED's report.

### C04.4. Unsupported semantics and false declarations

Only normal C returns are represented by v1 postconditions. A C routine returning null or a negative integer has still returned normally; it has not taken a QCore typed-error exit. Error-specific ownership behavior can be modeled only where a supported mode explicitly represents it, such as nullable acquisition or the specified reallocation relation.

General success-dependent parameter modes, exceptional exits, `ensures any`, `ensures error(E)`, quantitative clauses other than the counted-user clauses of C04.7, effects, access permissions and synchronization are not in v1. Non-null pointer error sentinels such as `MAP_FAILED` are not representable. The scalar failure sentinel `-1` of a descriptor is, through the `descriptor` kind and its `sentinel`/`valid` validity words (§C04.6); no other scalar sentinel is, and nullability is never reinterpreted as one.

**Value-dependent parameter modes are not in v1.** A parameter mode is fixed per declaration; no clause conditions a *mode* on an argument's value, and C04.5's premise does not add one — it restricts where the whole declaration applies, leaving every mode exactly as written.

The loader rejects syntax that attempts to express unsupported semantics. It cannot discover that a syntactically admitted declaration is semantically false of a named C function. For example, merely writing a pointer-returning `mmap`-like name with `resource_return_owned(nullable)` does not enable sentinel validation. Applying that model to an API whose failure is non-null would be an invalid trusted assumption. A broader profile MUST model success tests, endpoint kinds, generations and failure residuals explicitly; it MUST NOT reinterpret v1 nullability silently.

### C04.5. The literal-argument premise

Where a real C entry point switches behavior on a scalar argument — `flb_hash_table_add`'s `val_size`, whose zero arm stores the caller's pointer while its positive arm copies into a fresh allocation — a declaration read off one arm would otherwise silently cover the other. One precondition closes that gap:

```qcore
requires resource_literal_argument(p, LITERAL);
```

`p` MUST name a parameter of the same declaration. `LITERAL` is a decimal integer with an optional leading minus and no suffix, alternate base, leading zero, or other spelling; a leading zero is an octal constant and `-0` is a second spelling of a value that already has one, so both are rejected and the declared literal is **canonical**: one spelling per value. The loader records the C token texts the spelling produces (`0` is `["0"]`, `-1` is `["-", "1"]`), so a consumer compares token vectors and never re-parses a number.

`p` MAY name a parameter of any type in the v1 vocabulary, including a pointer. The vocabulary is closed, so this is a decision over two cases rather than an open permission: a premise on a scalar parameter bounds a value-dependent arm, and a premise naming a pointer parameter and the literal `0` bounds a declaration to the call sites passing a **null pointer constant** (C11 6.3.2.3p3), which is a legitimate reading of a two-arm callee. A premise no call site can satisfy is vacuous in the *refusing* direction, so admitting the clause on any parameter cannot make a consumer more permissive.

The clause declares that **the whole declaration applies only at call sites whose actual at `p` is exactly the integer constant `LITERAL`**. A consumer that honors it MUST treat the callee as *undeclared* at every other call site, including one that does not supply an actual at `p` at all. The clause therefore MUST only withdraw call sites the declaration would otherwise have covered; it MUST NOT widen one, and it MUST NOT alter any mode. It states nothing about the callee's body: it is a bound on where a trusted assumption is believed, not evidence for it.

Total-or-reject applies in full. An unknown parameter name, a malformed literal, a second premise on the same parameter, and any other `requires` spelling each reject the whole module, so a typo cannot leave a declaration silently unrestricted. A declaration MAY carry more than one premise, at most one per parameter; all of them must hold at a call site for the declaration to apply.

Like the variadic data tail, the premise is not carried on the `ResourceFunctionContract` value. It is rederived from the retained trusted source texts (`literal_argument_premises`), so an environment with no declared sources declares no premises. That is the honest reading rather than a fail-closed one, so a consumer that acts on premises MUST refuse — not proceed unrestricted — when rederivation over sources that mention the clause fails; `crates/qcore-c-gateway/src/legacy_literal_premise.rs` implements exactly that refusal.

This clause supersedes the machine-parsed `qcore-premise` comment directive that card HASH-CONTRACTS carried in the module text. That carrier is **retired**: the loader MUST reject a module whose text still contains its introducer, so a leftover directive fails loudly rather than silently losing the restriction it declares. Card HASH-CLAUSE measured the two carriers' refusal sets equal over the generic fixture family and the real upstream body before removing the directive, and pinned the measured set.

#### C04.5.1. Which call-site spellings are the declared constant

A call site MAY spell the declared constant in more than one way. A consumer MUST decide the question **structurally, without evaluating the actual**, and MUST admit exactly the spellings that two erasures reduce to the declared token vector:

* **matched enclosing parentheses** — `(0)` and `((0))` are the same primary expression as `0`. Parentheses that do not match each other, as in `(0) + (0)` or the cast `(long) 0`, are not erased.
* **an integer suffix** from C11 6.4.4.1 — `0L`, `0u`, `0ULL`. A suffix fixes the constant's *type*; it does not change its value, and the digit run is untouched.

Every other spelling MUST be refused, because deciding it would require a capability this premise deliberately lacks:

| Spelling | What deciding it would need |
|---|---|
| `0 + 0`, `1 - 1`, `-(0)`, `-0` against `0` | arithmetic |
| `0x0`, `0b0`, `00` | a base; `010` is 8, so a leading zero MUST NOT be stripped |
| `(void *) 0`, `(long) 0` | reaching into a cast's operand |
| an `enum` constant, or any other identifier | symbol resolution |

An object-like **macro** is not among the refusals: a consumer that reads preprocessed text sees the expansion, so a macro naming the constant has already become the constant.

Because the admitted spellings are exactly the declared constant, honoring them is not a widening of the declaration: it is applying the declaration where the clause already says it applies. A consumer MUST NOT admit a spelling whose value it has not decided structurally, and MUST NOT compare numeric values.

### C04.6. Resource kinds and integer-carried identities

Card W43-DESCRIPTOR-RESOURCE-TRACKING added **kinds** to the custody positions of vocabulary version 1. Before it, every tracked resource had one kind, so a declaration of `fopen`/`fclose` as owned/released would have let `free(fopen(…))` and `fclose(malloc(…))` discharge, and a file descriptor — an `int` whose failure value is `-1`, not NULL — could not be declared at all (`W43-LIBC-DESCRIPTOR-LIFECYCLE`, `CT-CTWIN-DESCRIPTOR`).

**Vocabulary.** Two clauses name a kind:

```qcore
ensures normal resource_kind(p, KIND);
ensures normal resource_return_kind(KIND);
```

`KIND` is exactly one of `descriptor`, `stream` and `directory`, each the kind of a pair this card declares; no kind is added without a declared use (an `FTS *` kind waits on a readable `fts_open`, and a library-handle kind on the return route modelling `return (T *)handle;`, card W43-DESCRIPTOR-RESOURCE-TRACKING §3). The **heap** kind is what a position with no kind clause means, and it is spelled only by that absence: writing `heap` rejects the module, so every meaning has one spelling. Every declaration written before this card therefore loads to exactly the contract it loaded to before, with every position heap. Kinds load onto `ResourceFunctionContract` as `parameter_kinds` (non-heap entries only) and `result_kind`, both omitted from the serialized form and from the diagnostic text when heap, so a contract without a kind is byte-identical in every artifact.

**Placement.** A kind clause names a custody-bearing position:

| Position | Admitted kinds |
|---|---|
| `resource_owned`, `resource_consumed`, `resource_released` parameter | every kind |
| `resource_out` parameter | every pointer-carried kind (a descriptor has no out slot in this vocabulary) |
| `resource_return_owned` result | every kind |
| `resource_borrowed` parameter | none — a borrowed position touches no obligation |
| `resource_stored`, `resource_removed` parameter; `resource_return_reallocated`, a derived or a `none` result | none — the owner cascade and the replacement relation were specified for heap allocations |

**Integer-carried identities and sentinel validity.** `descriptor` is carried by an integer: its position MUST be typed `i32`, and its failure value is the sentinel `-1`. Its validity is spelled `sentinel` (the position may carry `-1`) or `valid` (it never does), the counterpart of `nullable`/`nonnull`, which a descriptor position MUST NOT use; conversely `sentinel`/`valid` on a heap or pointer-kind position rejects the module. Every other kind is carried by an admitted pointer and keeps `nullable`/`nonnull`. This is the one relaxation of C04.1's pointer-only custody rule: a scalar carries custody exactly when its position declares the `descriptor` kind.

```qcore
extern fn open(file: ptr<const, u8>, oflag: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(file);
    ensures normal resource_borrowed(oflag);
    ensures normal resource_variadic_data_tail();
    ensures normal resource_return_owned(sentinel);
    ensures normal resource_return_kind(descriptor);
};

extern fn close(fd: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(fd, valid);
    ensures normal resource_kind(fd, descriptor);
    ensures normal resource_return_none();
};
```

**Semantics.** A consumer MUST apply all of the following, and MUST NOT approximate any of them:

1. **A custody position touches only an obligation of its own kind.** Each identity has the kind of the position that created it (an acquisition result, an out slot, or a formal under the contract a body is checked against). A release of another kind discharges nothing; when its actual resolves to exactly one identity it is an `invalid_release` violation *candidate* (`free(fopen(…))`, `fclose(malloc(…))`, `close(fp)`), and the identity's own obligation stays open. Every other kind-mismatched custody position — owned, consumed, stored, removed, a reallocation's replaced parameter, or an ambiguous release — is a refusal.
2. **Validity narrows like nullness, from the declared value set.** A descriptor-returning call produces `-1` or a number `≥ 0`. A comparison of a tracked descriptor with `0` or `-1` narrows it exactly as far as that set allows: `fd < 0`, `fd <= -1` and `fd == -1` establish *invalid* on their true edge and *valid* on their false edge, `fd >= 0`, `fd > -1` and `fd != -1` the reverse (and the mirrored spellings the same). `fd > 0` establishes *valid* on its true edge only, and `fd <= 0` on its false edge only, because descriptor `0` is valid. An invalid edge carries no resource at that identity, as a null edge does; a valid edge makes the identity usable at a `valid` position.
3. **A null test of an integer-carried identity refines nothing.** `if (fd)`, `if (!fd)`, `fd == 0` and `fd != 0` decide nothing about validity, so neither edge removes or validates the identity. A body that abandons a descriptor on the `!fd` edge is reported as the leak candidate it is.
4. **A tracked integer is custody, not data.** Every route that treats a declared scalar as custody-neutral MUST ask, before admitting a value, whether it reads a name that may hold a tracked integer-carried identity, and MUST refuse when it does: a store into a member (typed or untyped base), an aggregate field, an array element, a location reached through a pointer, or an object that outlives the call (the scalar-source exemption of C10's publication rule does not cover it). The C gateway's single predicate is `descriptor_custody::value_carries_descriptor`. A copy into a plain name aliases the same identity; a `return` escapes it; arithmetic into another name (`n = fd + 1`) creates and aliases no identity; a comparison decides a branch and transfers nothing.
5. **A descriptor handed where no declaration states the effect is refused.** A tracked integer at a `Borrowed` position is a *use* only where a declared contract covers the callee. A validated inferred summary was derived with its integer formals untracked, so a helper storing its formal still earned `Borrowed`; the call is refused instead, for the reason C10 withdrew the borrowed publication launder for pointers.
6. **Use after release.** A non-heap identity used at a `Borrowed` position after it was released on some path reaching the use is a `use_after_release` violation *candidate* (`read(fd, …)` after `close(fd)`, `fflush(fp)` after `fclose(fp)`) — the `double_release` reading of the same status. The heap kind keeps the pre-existing borrowed-liveness refusal and is not accused.
7. **Inferred summaries stay heap.** A consumer inferring summaries MUST NOT infer a non-heap result or out slot, and MUST check a body's releases against the kinds of the contract it is validated against, so a body that releases a probed (heap) formal with a descriptor release validates no summary. A declared kinded contract checked against a body defined in the unit seeds its formals with the declared kinds.

**Verdict contract, unchanged.** A candidate becomes `Unsafe` only under the existing provisional-`Unsafe` policy and is never presented as an upstream defect. Everything unproven stays `Unknown`, and no rule here turns an `Unknown` into `Safe` by approximation: with no kinded declaration in force every site is heap, no name holds an integer identity, and every rule above is the identity (measured byte for byte on the Cycle 19 corpus). The checked extern-call route (`qcore-contract::extern_bridge`) has one custody kind and refuses a kinded declaration rather than projecting it onto the heap.

**Versioning.** This is an addition to resource vocabulary version 1, exactly as the variadic data tail, the literal premise and the derived results were: the marker is unchanged, every module that loaded before loads to the same functions, and a loader that predates the clauses rejects a module using them as an unsupported predicate (`resource_kind`, `resource_return_kind`) or nullability (`sentinel`, `valid`) rather than misreading it. No new evidence version is minted: every verdict carries the exact module texts in `source_assumptions`, which name the declarations — and so the vocabulary — it rests on. The Lean model is `QCore.KindedCustody`.

**Not in the vocabulary, deliberately.** A kinded stored/removed relation; a descriptor out slot (`pipe`, `socketpair` write an `int[2]`); a status-conditioned out slot (`getaddrinfo`'s list is valid on a zero return, which needs a status witness bound to the list); a command-dependent creator (`fcntl` creates a descriptor only for `F_DUPFD`/`F_DUPFD_CLOEXEC`, and one declaration carries one literal premise per parameter); a transfer that happens on success only (`fdopen` moves the descriptor into the stream only when it returns non-NULL); a pointer sentinel (`mmap`'s `MAP_FAILED`); a process stream released by `pclose`, not `fclose` (no audited unit calls `popen`). A library-handle kind (`dlopen`/`dlclose`) is also absent: its one corpus effect was a leak candidate refuted by reading, because the return route does not model `return (T *)handle;` as an escape (a pre-existing shape that reports the same false leak for a heap pointer), so the pair is withheld until that route is repaired.

### C04.7. Counted users

Card W45-FB-RETRY-LIFETIME adds a **quantitative** clause family (C09) to vocabulary version 1: a counted reference an object holds on itself through a counter member, such as a task's `users`.

**Vocabulary.**

```qcore
ensures normal resource_user_added(p, COUNTER);            // one user added; the caller owes its drop
ensures normal resource_user_dropped(p, COUNTER);          // one user dropped, no release check
ensures normal resource_user_dropped_checked(p, COUNTER);  // one user dropped, then the release check
ensures normal resource_user_release_check(p, COUNTER);    // the release check alone
ensures normal resource_user_neutral();                    // no user of any object changes
ensures normal resource_user_exit(p, RETURNED, DELTA);     // every return of RETURNED added DELTA users
```

`p` names a `resource_borrowed` pointer parameter: a user is not custody of the object, and the object's custody mode is unchanged. `COUNTER` names the member of the designated object that holds the count. `RETURNED` is a canonical integer (C04.5's spelling rules) and `DELTA` one of `added` (+1), `restored` (0) and `dropped` (-1), so a net count is never spelled as arithmetic. A parameter carries at most one operation, or exit clauses, never both; `resource_user_neutral()` stands alone; an exit clause requires an integer result. Every other placement rejects the whole module (grammar G05).

**Semantics.** A consumer MUST apply all of the following:

1. **An addition is an obligation, a drop discharges it.** Along a path, a formal's count is the sum of the operations applied to it. An operation is attributed to a formal only when its actual is that formal's name and no statement of the body rebinds or takes the address of it. An unattributed operation leaves every tracked count undecided, and is reported.
2. **A drop without the release check is owed a check.** A drop that takes the count below its entry value leaves the object possibly unreferenced; unless a later addition or a release check reaches the same path, the exit it reaches is a violation *candidate*. A drop that only rolls back an addition of the same path owes nothing of its own.
3. **The release check may destroy the object.** After `resource_user_dropped_checked` or `resource_user_release_check`, a further operation on the same formal is undecided.
4. **Exit clauses are checked, never assumed, where the body is defined.** Every clause of a function defined in the analysed unit is decided against its body per path: **refuted** when a path that returns the clause's value definitely has a known count other than the clause's (a provisional `Unsafe` candidate, never an upstream defect), **undecided** when such a path's count or returned value is not known (`Unknown`, with the blocker), **held** otherwise. An exit no clause covers must restore every tracked count. A caller applies a declaration whose function is defined in its unit only when those clauses hold there; otherwise the call is opaque.
5. **The frame is declared, and opacity is fail-closed (premise UC-FRAME).** A call's effect on counted users is read from its declaration; a declaration with no counted-user clause and no body in the unit changes no count; a body defined in the unit is neutral when it writes no declared counter member and calls nothing that is not neutral; a call with no contract and no body, or through an expression, is opaque and leaves every tracked count undecided. The premise is stated, not proved: a declaration that omits a counter change it performs is a false declaration (C04.4).

**Verdict contract, unchanged.** A refuted clause is a provisional `Unsafe` candidate under the existing policy; branch feasibility beyond the conditions a consumer decides (a comparison of a stored call result or of a literal-premise formal with a literal) is not proved. Every published result carries the exact module texts, so the evidence identity names the clauses it rests on. The checked extern-call route has no counted-user model and refuses such a declaration. The Lean model is `QCore.CountedUsers`.

### C04.8. Container publication

Card W45-FB-RETRY-LIFETIME. An intrusive insertion (`mk_list_add(&node->_head, &queue->pending)`) links an object into a container the caller reaches; the custody of the object passes to that container. `resource_stored` does not state this: destroying a list head destroys no member.

```qcore
ensures normal resource_published(entry, container);
ensures normal resource_unpublished(entry);
```

Both name `resource_borrowed` pointer parameters, which keep that mode for the ABI projection and for the admission of a member address; the container is a distinct parameter. A consumer that does not apply the clause MUST refuse the call, never read it as a borrow. A consumer MUST apply all of the following:

1. **Publish.** The entry names its object as `&B->F` or `B`. The container must be reached from memory the caller supplied: a formal the body never rebinds (`&P->F…` or `P`), or a local whose one write is its declaration's initialiser from such a formal (`T *L = P->F;`). When `B` denotes exactly one known, non-null, live identity with no unresolved effect, the identity is published: its custody is the container's and no exit owes it. Every other publication that could move custody refuses: an unnamed entry, any other container (a local object, a file-scope object, an object the body acquired), an unknown, ambiguous, derived or possibly-null binding, an identity that is not live, and an unbound formal (reported unresolved, which is also what makes a derivation bind it). An object that owns linked members or children is published with its owned objects withdrawn in both directions, and the call is refused.
2. **A published identity stays the container's.** Releasing it is a violation candidate (`release_after_escape`); transferring it again, a `return` included, refuses.
3. **Unlink.** `resource_unpublished` restores the body's custody only for an identity published on every path reaching the call and still published; every other unlink refuses, an untracked entry included.

A derived summary states custody only, so it is compared with a declaration's custody projection; the container and counted-user clauses are not part of that comparison. The Lean model is `QCore.ContainerPublication`.

## C05. Manifests and immutable assumptions

A resource manifest has the following existing version-1 shape:

```json
{
  "schema_version": 1,
  "modules": ["qcore/libc/memory.qv"]
}
```

Unknown JSON fields, unsupported versions and empty selections are rejected. Module paths are interpreted relative to the manifest's parent directory. Every path component must be a normal relative component; absolute paths, `.` and `..` components are rejected. This lexical check is not a canonical filesystem sandbox: it does not establish symlink confinement, provider isolation, or an operating-system access policy.

Duplicate file-list names and duplicate function contracts are errors. Successful loading retains the exact selected text map in `source_assumptions`. `check_source_assumptions` reloads that immutable text and compares the resulting function map. It does not reread mutable files and does not prove that a trusted external contract matches its C implementation.

Programmatically constructed environments can have no retained source map. Their lack of declared source text MUST NOT be reported as source-backed verification. A consumer using such an environment must identify its alternate assumption source and trust boundary.

The reviewed `cycle19-resource-contracts.json` selects nine modules and twenty-five resource-v1 declarations (grown deliberately by cards HASH-CONTRACTS and PROD-CONTRACTS; the pinned counts in `compiler/tests/qcore_contract_corpus.rs` are the authority). This narrow projection is intentional; it does not load the complete lifecycle/observation corpus into the custody table.

## C06. Declared, inferred and proposed contracts

`ContractTrust` currently distinguishes `Declared`, `InferredValidated` and `Proposed` records. Their meanings are different:

**Declared** contracts are explicit trusted assumptions. A source-derived result can be valid relative to them without proving their external implementation. **InferredValidated** summaries require the declared inference algorithm's independent reanalysis and compatibility checks. **Proposed** summaries are unchecked suggestions and MUST NOT justify Safe.

The schema tag alone is not evidence. The `ResourceContracts` function map does not attach and enforce a complete provenance/evidence bundle on every entry. Callers and certificate consumers MUST preserve how a summary was obtained and revalidate it where required.

The current inference in `crates/qcore-c-gateway/src/legacy_summary.rs` is conservative and bottom-up. It binds formals without silently assuming every nullable pointer is non-null, requires a Safe/diagnostic-free analysis, requires reachable exits and a compatible disposition across those exits, distinguishes fresh returned custody from a returned formal identity, and refuses unsupported hidden custody changes. Declared contracts take precedence; mismatches are conflicts rather than permission to overwrite a declaration.

Current inference supports the basic formal/return modes plus bounded direct
`Out`, `Stored`, `Removed`, `Reallocated` and nullable returns. A direct Removed
wrapper starts with the original child contained in its distinct nonnull owner.
Nullable preconditions are proposed one at a time and retained only after full
re-analysis validates all admitted alternatives, including prior nullable
children. A null child carries no custody; every nonnull exit must preserve its
original identity as directly live and detached, and preserve the owner.
Guarded nonnull removal calls and reordered direct forwarding can validate.
General removal/out-slot aliases, relational branch analysis, general recursive
fixed points and cost inference remain unsupported; zero/nonzero context-specific
recursion uses strictly lower-rank independently validated summaries. The W4 extension report records validation and pinned verdict parity. Recursive or otherwise unresolved dependencies MUST remain unresolved unless the implementation's actual validation establishes the needed summary; empty optimistic summaries are not evidence.

An inferred summary MUST be invalidated when the function, its relevant callees, compilation profile, contract environment or source-import coverage changes. In particular, an output parameter's old relationship MUST NOT survive reassignment to a different destination. The merged W4 extension has a regression for out-formal reassignment; the summary cannot keep the prior destination identity.

**Derivation scope (card W44-CORPUS-CALLEE-SUMMARIES).** Under a main-source selection, the definitions an inference considers MAY exceed the definitions the unit reports, but only by unselected definitions the selection reaches through direct calls that have **internal linkage on their own definition** (`static`, not `extern`; C11 6.2.2p3), so that every call to them in the unit runs the analysed body, and whose names are not reserved in every context (`__x`, `_X`; C11 7.1.3), which leaves C library header bodies to the libc provider. A definition of external linkage MUST NOT be admitted this way: a call to a function with an inline definition may run the external definition instead (C11 6.7.4p7). An admitted definition obtains a summary only through the same validation, never changes the reported scope, and is re-derived by a certificate check from the same selection. The Lean model is `QCore.CalleeSummaryScope`.

**Objects released through a formal (card W45-CALLEE-SUMMARY-CASCADE).** A validated inference MAY rest on a body that hands a member `B->F` of a formal `B` to a `resource_released(_, nullable)` position, provided the read's obligations pass (every base resolves, no base owns sub-custody and no read place is bound, so the released object is none the analysis tracks) and the release is the last use of that place: `B` is never copied, the place is released once and before that only null-tested, no loop or non-local jump can repeat the release, and `B` itself is released afterwards on every path, so the inference states `resource_released` for `B`. Such a summary states the formals' modes only, and the objects it releases through a formal are not in its vocabulary; therefore every call to a function defined in the unit whose contract in force is not a declaration, and whose body or a body it calls hands such a read to a call, MUST refuse an actual whose target owns sub-custody (a field link, a contained child or an indexed collection) before the summary is applied. A non-null release of such a read and a release of a member address MUST keep the refusal. The Lean model is `QCore.MemberRelease`.

### C06.1. Contracts awaiting human review

Card W45-CONTRACT-DEFERRED-REVIEW. A resource-v1 declaration, or one clause of it, MAY be marked as **awaiting human review**. `ContractTrust` gains a fourth status for it, `PendingHumanReview`. **Proposed** is not reused, for two reasons: a proposal awaits the *core's* validation and can be promoted by it, whereas only a person resolves a review, by editing the text; and C06 bars a proposal only from justifying `Safe`, whereas a pending contract MUST justify neither `Safe` nor `Unsafe`. A pending contract is not an assumption, not evidence and not an attestation, and no record of a sign-off exists: removing or changing the marker changes the module text, and so its digest, in the ordinary way.

**Syntax.** Contract-level, a requirement beside the resource-v1 selector:

```qcore
extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending("is the handle freed on the error path?", "owner");
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
```

Clause-level, wrapping one `ensures normal` clause in place:

```qcore
    ensures normal resource_review_pending(resource_released(h, nonnull), "reason");
```

The first string is the **mandatory** free-text reason; the second, optional, names a reviewer or owner. Each is one string literal, not blank, with no escape and no control character. A declaration carries one contract-level marker or any number of clause-level ones, each on a distinct clause. The grammar is [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1).

**Loading.** Total-or-reject (C04) applies in full. A missing or blank reason, a reason or owner that is not one plain string, a third string, a second contract-level marker, a contract-level marker beside a clause-level one, a marker wrapping a marker, a clause-level marker around the `resource_contract_v1` selector or a literal-argument premise (C04.5; those are deferred only by deferring the whole contract), a contract-level form under `ensures` and a clause-level form under `requires` each reject the **whole module**. A wrapped clause is read exactly as if written bare, so every C04 rule still applies to it. A marked declaration is therefore loaded, validated, digested and packaged like any other; its source text is retained (C05) and `check_source_assumptions` recomputes its status. The loader then **withholds** it: the environment records it under `pending_review`, with its module, the contract it states and every marker, and it is **never** an entry of the function map. A consumer that does not recognise the status therefore sees an uncontracted callee, which is the refusing direction.

**Meaning.** A pending declaration discharges nothing and accuses nothing. Every call site or obligation that would apply it MUST be held, never `Safe` and never `Unsafe`, under the reason code `awaiting-human-review`; since card W45-USER-CONTRACT-PLUGINS a verdict resting only on held calls is `Pending` (language specification §L21.2), and one with any other unresolved construct stays `Unknown` (§L21.1; subject: the analyser, so it never supports a provisional `Unsafe`). A clause-level marker holds exactly the call sites a contract-level one holds: a v1 contract is applied as one conjunction, no consumer applies part of one, and dropping only the deferred clause would let the remaining, possibly weaker, clauses decide the call. The clause the marker names is what a report attributes, not a narrower refusal. A function *defined* in the analysed unit under a pending name is analysed as an undeclared definition: the pending contract is neither its checked interface nor its accuser, and its callers stay held whatever its body or a W4 summary of it proves.

**Consumers.** The legacy-C gateway rewrites every call site of a pending callee to a marked undeclared name before any reader sees it (the LOADER-ABI technique), so W4 derivation, W4 validation and certificate rechecking reproduce the hold; the statement refuses with ``call to `NAME` whose declared contract awaits human review (awaiting-human-review)``; a projection summary names its held calls (`…, N call(s) held awaiting human review`) only when there is one, and classifies as `awaiting-human-review` ahead of every other category. The checked extern-call route (`source.extern-call.v1`) finds no admitted contract and refuses with `Q023` naming the review. Provider packages carry the text unchanged (`providers/package.py` pins bytes, not clauses), and every provider selection loads through this loader. The observation and lifecycle interpreters do not recognise the marker and reject it as they reject every other resource-v1 clause.

**Inheritance and inclusion (C15).** Elaboration copies a parent's clauses, markers included, renaming parameters by position; a string is not a word, so a reason and an owner are never renamed. A child that inherits a deferred contract is therefore deferred, and one that inherits a deferred clause is deferred for that clause. The inherited-declaration agreement check compares the markers too (scope, reason and owner), so a child can never shed its parent's deferral.

**Reporting.** An audit report lists every marker in a `pending_human_review` section — declaration, contract or clause scope and clause text, reason, owner, module and the SHA-256 of its retained text, and the analysed functions and recorded held-call refusals it accounts for — with explicit zeros when nothing is pending, and prints a one-line tally. A unit's `reason_code` carries `awaiting-human-review` where unit reasons are recorded. The Lean model is `QCore.ContractDeferredReview`.

### C06.2. Revision modes: pinned and flexible contracts

Card W45-CONTRACT-REVISION-MODE. A resource-v1 declaration, or one clause of it, says how it relates to revisions of the audited project. **Pinned is the default** when nothing is written.

**Syntax.** Contract-level, a requirement beside the resource-v1 selector; clause-level, a wrapper around one `ensures normal` clause, in place:

```qcore
extern fn malloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

    ensures normal resource_revision(resource_released(p, nonnull), pinned);
```

The grammar is [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). A clause without a marker takes the contract-level mode. A contract is applied at a call site as one conjunction (C06.1), so a declaration is **flexible exactly when its contract-level mode is flexible and no clause is marked pinned**; one pinned clause pins the whole contract, and a clause marked flexible in a pinned contract leaves it pinned. The report names each marked clause.

**Loading.** Total-or-reject (C04). A second contract-level marker (a duplicate, or a conflicting mode), a clause-level marker that restates the contract-level mode, a mode other than `pinned` or `flexible`, a marker wrapping a marker (a revision or a review marker, in either order), a clause-level marker around the selector or a literal-argument premise, a contract-level form under `ensures` and a clause-level form under `requires` each reject the **whole module**. A wrapped clause is read exactly as if written bare. The environment records the markers of every declaration that carries one (`ResourceContracts::declared_revisions`); a declaration absent there is pinned. `check_source_assumptions` recomputes them from the retained text.

**Pinned.** A pinned contract is **in force only where its contract text is the one it was pinned with and the audited source matches one of the identities it was reviewed against**. Those identities live in a generated **lock**, `contracts.lock`, never in the `.qv` (or legacy `.qcore`) text: one per provider crate, beside its `Cargo.toml`; a module outside every crate is covered by the nearest `contracts.lock` in its directory or above it, up to the first directory holding a `Cargo.toml` or a Git directory. A lock (format version 2) records, per pinned declaration, the SHA-256 of its normalised declaration text (its tokens joined by one space) and one or more **reviewed identities**. An identity is the function's **sites** — every top-level declaration and definition, each with its file, the SHA-256 of its normalised declaration text and, for a definition, of its body — together with every source revision it was reviewed at; revisions whose sites are equal share one identity, no revision appears in two identities, and there is no wildcard. Comments and layout do not count; every other change does, a renamed local and a reordered statement included. For a Git checkout the sites are read from the blobs of `HEAD`, never the working tree. A contract text that differs from the pinned one voids every identity. Format version 1 (one source per lock) is still read, each entry as one identity. Before any unit is analysed, every audit route of `quirin-audit` (`current`, `contracts`, the configured route) and the corpus arbiter compare each pinned declaration in force with its lock over the audited source. A mismatch — no lock (`no-lock`), no entry or no identity (`not-in-lock`), a changed contract text (`contract-changed`) or sites matching no identity (`source-changed`) — is **not in force**: the declaration leaves the function map and is recorded under `ResourceContracts::pin_mismatch` with its reason, and every call relying on it is held under the reason code `contract-pin-mismatch` (§L21.1; subject: the analyser, so it never supports a provisional `Unsafe`), never `Safe` or `Unsafe`. A person resolves a mismatch by reviewing the changed source and re-pinning, so since card W45-USER-CONTRACT-PLUGINS a verdict resting only on such held calls is `Pending` (language specification §L21.2); it was `Unknown`. The legacy-C gateway holds those call sites exactly as it holds a pending review's (C06.1): each is rewritten to a marked undeclared name, the statement refuses with ``call to `NAME` whose pinned contract is not in force: the audited source does not match its lock (contract-pin-mismatch)``, and a unit summary names the held calls after any held for review. A function defined in the unit under a withheld name is analysed as an undeclared definition. `check_source_assumptions` accepts a withheld declaration only when it is pinned and equal to its fresh load, so a pin evaluation can move a declaration out of force and do nothing else. A lock that does not parse, names another version or normalisation, lists unsorted or duplicate entries, or has an identity with no revision, a revision in two identities or two identities with equal sites is malformed, and the audit refuses.

**`quirin-audit pin`.** A lock is written only by `quirin-audit pin`, after human review, and never during an audit. `pin --source-root DIR` **adds or refreshes** the identity of that checkout's `HEAD` for every declaration of every crate on the load path (so new contracts are pinned with one command) and never drops an identity recorded at another revision; a declaration whose contract text changed starts again from this revision alone. `--function NAME` restricts it to the named entries; `--regenerate` rebuilds each lock from this revision alone; `--drop-revision REV` (no source read) removes the revision REV — spelled in full, as a prefix of at least seven characters, or `directory` — from every identity, and an identity left with no revision goes. `--check` never writes: for the checkout's revision it reports each entry as recorded there, in force through an identity reviewed at another revision, not in force, or stale, and exits 1 unless every entry is recorded there; `--check --revision REV` reads REV's blobs from the checkout's object store instead of `HEAD`'s. `--modules DIR` pins a directory of modules outside the providers; `--verify-contracts` (no source) checks that each lock lists exactly the crate's current declarations, modes and contract digests, and `providers/contracts.py check` runs it.

**Flexible.** A flexible contract binds by name and declared signature only (the declared/C ABI agreement of card LOADER-ABI still applies) and is in force at every revision. On a function whose body the run analyses, the body is checked against it as usual; the mode never skips that check. On a trusted function (no body in the audit) it never applies silently: it is listed as an explicit assumption of the run. Every flexible contract of the libc provider (`quirin-provider-libc`, all of whose contracts are flexible) that the run does not check against a body forms **one standing, named assumption**, `libc-c-standard-abi`: the C library behaves as the C standard and platform ABI specify. The C library's version is the build's provenance, not part of the assumption.

**Not inherited (C15).** A revision mode records how a text was reviewed against its own function's source; an inheriting declaration names a different function. Elaboration therefore drops the parent's contract-level marker and unwraps its clause-level markers, and a child is pinned unless its own extension clauses say otherwise.

**Reporting and change reports.** Each audit's contract sidecar carries a `contract_revision` section — the source identity, the locks read (path, SHA-256, revision), counts, every pinned contract in force, every mismatch with its reason, pinned and found sites and the held functions and call sites, the flexible contracts checked against a body, the flexible trusted assumptions, the standing libc assumption, and an inventory of every declaration's mode and status — and a one-line tally. `quirin-audit compare --before REPORT --after REPORT` reports the change between two runs (two revisions, two contract sets, or both): selected and missing sources, contracts in force added, removed and changed, pin mismatches new and resolved, flexible and standing assumptions, per-function obligations added and removed, and verdict moves in which **violation detected** (a move to `Unsafe`) is kept separate from **coverage lost** (a move from `Safe` to `Pending` or `Unknown`, with the reason codes of its refusals); a move between `Unknown` and `Pending` is classed on its own, and each run's verdict tallies, `Pending` included, are reported.

**Boundaries.** The pin is enforced where an audit evaluates it: the four routes above. A caller of the gateway API, the checked extern-call route and the other compiler examples apply the environment they are given, in which no pin was evaluated. The scanner is lexical: it never preprocesses, so a site covers every configuration's text, and it balances braces per conditional group. The Lean model is `QCore.ContractRevisionMode`.

### C06.3. Checked specifications

Card W45-FB-BACKLOG-CUSTODY. A resource-v1 declaration MAY carry the contract-level requirement `requires resource_body_checked;` beside the selector. It marks the declaration as a **specification of a body the audit analyses**, not an assumption about an external implementation. The marker changes no mode and adds nothing to the environment's serialized form; a consumer rederives the marked names from the retained texts (`declared_checked_specifications`), exactly as the variadic tail (C04) and the literal premise (C04.5) are rederived. The grammar is [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1).

**Composition with the other markers.** The marker is an ordinary contract-level requirement, read after the revision (C06.2) and review (C06.1) markers are peeled. A pinned or flexible revision marker and this marker compose independently: the revision mode decides whether the declaration is in force for the analysed revision at all, and this marker then decides, inside a unit, whether its body establishes it; a declaration out of force because unpinned is never a specification in that unit. A review marker withholds the declaration, and a withheld declaration is never a specification. No marker wraps `resource_body_checked`: a clause-level marker around it rejects the module.

**Meaning.** A consumer analysing a translation unit MUST apply a checked specification — at the defining body's entry and at every call site of that unit — only while all of the following hold, and MUST otherwise withdraw it from that unit's environment, as a declaration the ABI or mode gate contradicts is withdrawn (C04.1):

1. the unit defines the body inside its derivation scope (C06, W44-CORPUS-CALLEE-SUMMARIES);
2. the body is not a member of a recursive component of the unit's direct call graph, so no specification is validated through itself (the constraint-4 rule against an assume-guarantee fixed point);
3. the body **establishes** the specification: analysed under the specification's entry custody, with exactly the checked specifications still standing in force, it is fully modelled, reports no diagnostic, every reachable exit matches every clause (the W4 gate, `observations_match_contract`), and no validated summary of it conflicts with the declaration (C04.1's `SummaryConflict`).

Withdrawal is a descent: each round withdraws every specification the current set does not establish, until a round withdraws none; a round only ever validates against specifications still standing. A withdrawn specification is never trusted: callers see whatever the ordinary W4 inference validates from the body, or an uncontracted call, and the defining function's row carries the diagnostic `declared specification of `NAME` is not established: REASON`, so it is never `Safe`. A consumer that does not implement the marker applies the declaration as an ordinary trusted declaration, so a provider MUST NOT rely on the marker with a consumer that predates it; the C-gateway and every audit route built on it implement it.

**Verdict contract.** An established specification is a result about that body under the unit's other contracts, never about a revision or a unit the consumer did not analyse. A specification is bound by function name and declared signature only; a revision whose C declaration differs is withdrawn by the ABI gate, which is the fail-closed answer. The Lean model is `QCore.CheckedSpecification`.

### C06.4. Contracts their own bodies refute

Card W45-REFUTED-CONTRACT-WITHDRAWAL. Every declared contract of a function whose body a consumer analyses is checked against that body, whether or not it carries the C06.3 marker. The body **refutes** the declaration when (a) a counted-user clause of the function's own row — a declared clause or the implicit obligation of an uncovered exit (C04.7 rules 2 and 4) — is refuted, or (b) the validated summary W4 derives from the body contradicts a declared custody *duty* (C04.1's mode arm; `mode-parameter-contradiction`, `mode-result-contradiction`). A `SummaryConflict` that is a refinement inside one duty (nullability, an owner index, a replaced parameter) is **not** a refutation, and neither is an undecided clause, a body that does not validate, or the absence of a body: those leave the declaration **unchecked**, applied as the trusted assumption C06 describes.

**Meaning.** A refuted declaration MUST stay in force for its own body, so the defining row keeps the refutation (a provisional `Unsafe` candidate, never an upstream defect), and MUST be withdrawn from every caller: every call site outside the function's own body is rewritten to a marked undeclared name before any reader sees it, the statement refuses with ``call to `NAME` whose declared contract its own analysed body refutes (contract-refuted-by-body)``, and the call is `Unknown` (reason category `stale-evidence`, subject the analyser, so it never supports a provisional `Unsafe`). It is never `Pending`: a refutation is the tool's decision under stated premises, and no named human act resolves it — a changed declaration or body is decided again by the tool, not accepted on review. A name the C04.1 mode gate withdrew keeps that gate's key in its unit.

**Two phases.** A refutation is found only in a unit that defines the body; a caller may sit in any unit. An audit route therefore runs a **first phase** that analyses every unit under the loaded contracts — each unit already holds the calls to what its own bodies refute — and records each refutation on the defining function's row (`contract_refutations`). The **refutation table** is the union of those rows, keyed by function name (`ResourceContracts::refuted_by_body`, omitted from the serialized environment when empty). When the table is not empty, a **second phase** analyses every unit again with it recorded, holding every call site of its names in every unit; its rows are published. With an empty table the second phase would repeat the first exactly, and is not run. The table is a set, so it is independent of unit order, `--jobs` and grouping, and its canonical serialization has a SHA-256 (`table_digest`, over the empty table too). A result that composes units analysed separately MUST carry each unit's `contract_refutations` and MUST join the digest of the table it was analysed under to its identity, or refuse to compose across different digests.

**Reporting.** Every route publishes a `refuted_contracts` section in its sidecar — the table applied, its digest, the number of phases, the digest of the published rows' own refutations, and the held functions and call sites per name — with a one-line tally. The checked extern-call route has no body to check and is unaffected. The Lean model is `QCore.RefutedContractWithdrawal`.

## C07. Observation interpreter

The implementation is `crates/qcore-contract/src/contract_observation.rs`. Its input is source text, the selected function name, and a map of fact names to signed `i128` values. The selected declaration MUST be unique, be an extern function, and have a contract. It accepts `requires` clauses and `ensures normal` clauses only, and requires at least one normal postcondition.

It evaluates an integer/Boolean expression language, not the external resource predicate-call language. Supported values are decimal integers, unary-negated values, fact identifiers, and Boolean literals. The full signed i128 literal range is supported, including its minimum value; positive or negative overflow rejects (R07). Operators are parentheses, unary minus and `!`/`not`, multiplication, addition/subtraction, comparisons, conjunction and disjunction. Boolean truth is nonzero; Boolean results use zero or one. Arithmetic uses checked signed operations. Division, quantifiers, pointer dereferences, arbitrary calls, nested `old(...)`, user-defined functions, runtime heap inspection and language-level effect evaluation are not supported.

The precedence order is unary, multiplication, addition/subtraction, comparisons, conjunction, disjunction. Binary operators are left-associative in the parser. The detailed grammar and limits appear in the implemented-grammars document.

### C07.1. Eager evaluation and result categories

All clauses are parsed and evaluated before the interpreter classifies whether preconditions hold. Boolean operators are eager rather than short-circuiting. Consequently an unsupported expression, a missing fact or an arithmetic overflow is an error even when it occurs in a logically unused branch or a postcondition under a false precondition.

There are three semantically distinct outcomes:

| Outcome | Meaning |
|---|---|
| `Checked { postconditions }` | Every evaluated precondition is true and the specified number of supported postconditions are true for the supplied environment. |
| `OutsidePreconditions { predicate }` | The environment does not meet a precondition. This is neither satisfaction of the full contract nor a demonstrated postcondition violation. |
| Error | Invalid/unsupported input, missing fact, arithmetic failure, or a false postcondition after true preconditions, as identified by the error. |

The compatibility `check_contract_facts` helper maps an outside-preconditions outcome to an error rather than a checked result. Callers MUST not collapse precondition exclusion into a positive observation count or treat every error as a source defect.

Example policy:

```qcore
module contract;
extern fn observe(live_before: i64, live_after: i64) -> unit
contract {
    requires live_before >= 0;
    ensures normal live_after >= 0;
    ensures normal live_after == live_before;
};
```

The fact map, not the function signature, supplies the values. The signature is not an automatic runtime binding mechanism, and declared scalar widths are not independently enforced as narrowing constraints on all supplied `i128` facts by this evaluator. Extra facts do not prove anything unless consumed by the selected policy. Required facts must exist.

### C07.2. Fact provenance

An identifier such as `chunk_is_live` can evaluate true when the supplied map assigns it a nonzero value. That evaluation does not prove the identifier's intended English meaning. A source adapter may derive it from a checked model; an observation adapter may derive it from recorded events; an arbitrary caller can also supply it. These cases MUST remain distinguishable.

Pre-state facts such as `old_busy` are explicit snapshot values, not automatic `old(...)` heap semantics. Their association with the correct operation and object must be established by the adapter. The same requirement applies to source/build digests, event order, endpoint identity, process completion, diagnostic flags and sanitizer outcomes.

An observation claim MUST identify the fact producer, the observed artifact, the policy text, the applicable preconditions, and the scope of the recording. A successful interpreter result is a policy replay result, not a general source Safe certificate.

## C08. Source-bound and lifecycle adapters

The bounded native Boolean-field importer additionally supports a one-record
pointer function returning C `void`. Its contract MUST declare `unit` and name
the selected C record as its borrowed nominal parameter type. The explicit
`qcore.c_boolean_field_contract.v3` profile checks both initial field values,
requires `object_is_live`, `exclusive_object_access` and `field_is_boolean`, and
uses `field` / `old_field` in postconditions. It MUST NOT admit a scalar `result`
predicate. A bare return ends the path; fallthrough is valid for void only.
Integer-returning functions retain the v2 profile and mandatory explicit return.
Unit input version 3, the result-kind field and the input-digest namespace are
bound to the task. Unknown statements, mismatched ABIs and incomplete source
imports remain Unknown. The schema's numeric return slot is zero bookkeeping for
unit paths and is never a claimed native return value. Source/ABI import and live,
exclusive Boolean caller conditions remain explicit premises. See
[CMetrics provider evidence](reports/w5-cmetrics-unit-provider/README.md).

The repository contains specialized importers and wrappers rather than one complete lifecycle theorem prover. These adapters inspect restricted source or execute restricted models, bind a fact environment, and use selected contract-source acceptance rules. Sharing the predicate evaluator does not make the adapters interchangeable.

The ChunkIO frame/source adapters in `compiler/src/chunk_contract.rs` check particular imported summaries against exact parameter shapes, return shapes, field facts and live/exclusive premises. The generic C importer and source-policy machinery cover only their supported syntax and compilation scope. A caller-supplied live/exclusive premise is an assumption until caller analysis establishes it.

A source-bound contract result MUST identify the imported function or source unit, source/build identity, compilation dialect and preprocessing configuration, imported operations, excluded operations, callee contracts, caller premises and the check that binds the summary to that source. Restricting the analysis to main-source definitions or to a selected translation unit MUST be reflected in the claim.

A result-use analysis can establish that a return value was compared or checked; that does not establish that the external operation succeeded. Similarly, an observed field reset is not proof that a resource was discharged, and a named release function call is not an endpoint release unless its trusted contract or actual result semantics support that conclusion.

When source extraction cannot determine nested calls, pointer identity, conditional cleanup, control-flow exits or relevant loop dependencies, the affected property MUST remain Unknown. In particular, every nested lifecycle operation relevant to a requested obligation must be visited or conservatively accounted for. The merged W3 tests cover nested effects and other scanner gaps, and lifecycle source derivation remains incomplete instead of claiming sufficient import coverage.

Custody transfer requires sufficient identity information for its update.
A pointer that may select either of two resources cannot strongly release,
escape, store, detach or replace both. Detachment additionally requires the
exact contained disposition and a unique matching owner across the represented
alternatives. Aggregate cleanup must establish those conditions for each
contained descendant; a directly live alternative cannot disappear in cleanup.
Released or previously escaped custody cannot be consumed again. Unsupported
correlations retain unresolved effects and cannot produce Safe. Retained state
following an unresolved operation is not itself proof of a leak; independently
established leaks remain evidence. See the [transfer validation report](reports/w4-nullable-removal/README.md).

Generic Boolean frame helpers now use an explicit nominal C record and the
complete modeled-field inventory from a loaded profile. `evaluate_boolean_frame_contract`
checks a mutable `borrow<'life, mut, Record> use one` parameter, unit result,
`object_is_live` and `exclusive_object_access` premises, constant `field_NAME`
post-state facts, `writes_only_declared_fields` and `modeled_fields_are_preserved`.
The evaluator intrinsically rejects empty or modeled-field writes. Both frame
evaluators reject written fields omitted from the policy, even when a policy
negates the footprint predicate. Schema-only evaluation remains declared-model
evidence; the configured service additionally requires an opaque Clang import.

The source importer checks exact record/type spelling, direct parameter identity,
external helper prototypes, ordered field/type vectors and constant-only writes.
Every field modeled for that record is protected, including fields declared by
other libraries in the same profile. Caller composition rechecks the protected
inventory even for legacy helpers. These declared layout vectors do not prove
recursive nested C offsets or linker identity; actual cross-unit ABI equality
remains an outer-audit premise. See [frame helper evidence](reports/w5-configured-frame-helpers/README.md).

## C09. Custody, access, lifetime and quantitative clauses

Custody contracts state who remains responsible for a resource. Access contracts state who may read or write an identified footprint. Lifetime conditions state when identities remain valid. Quantitative clauses state credit, live-count or cost relations. These domains are related but not interchangeable. The first implemented quantitative clause family is the counted-user family of C04.7, whose exit clauses state a live-count relation per returned value; the first implemented lifetime relation between an object and a container is the publication of C04.8.

A complete resource contract SHOULD describe its identity-bearing pre-state, the operation's effects, each possible return/error exit, each exit's custody and access post-state, and the cleanup protocol. A complete concurrent contract additionally needs transfer and synchronization witnesses, task/quiescence conditions and any mutex invariant that actually covers the accessed footprint.

For each resource identity and exit, exactly one supported disposition must be established: retained by the current custodian, transferred to a specific new custodian or owner, released, or retained as a residual obligation with an explicit unresolved/poisoned state. A scalar balance or `net == 0` cannot substitute for this disposition. Releasing one allocation and leaking another of equal size is not balanced custody.

Credit may be returned or discarded only according to its axis policy. Live obligations MUST NOT be erased by error-region exit, cancellation, failed cleanup, loss of a pointer, parent deallocation, or a cost bound. All surviving obligations must be preserved in the corresponding exit state.

A postcondition about normal return says nothing about an unmodeled exceptional, cancellation or divergence exit. The proposed complete source language must reject or weaken claims when a promised exit family is not covered. Current strict consumers reject the exits they cannot express; generic structural parsing alone does not discharge them.

## C10. Cleanup and OS endpoint contracts

Cleanup must distinguish invocation, attempted release, observed return, actual release and residual custody. An OS endpoint is identified by its kind, acquisition generation and lifetime, not by a numeric descriptor or virtual address alone. Numeric reuse after an earlier epoch has ended is permitted and must not merge the epochs.

Parent memory, child allocations, descriptors and mappings are separate resources. Deallocation of every parent allocation does not imply that all descriptors were closed or mappings unmapped. A null return can coexist with a partially acquired endpoint left by a failed rollback. A non-null stored mapping field can contain a failure sentinel rather than a live mapping. A counter such as an up count is an abstract state observation, not independently a release witness.

The complete cleanup relation MUST select one of the following policies and represent every resulting obligation:

| Policy | Required failure post-state |
|---|---|
| Infallible release | Failure is impossible under the explicit modeled premises; the proof/assumption must justify that exclusion. |
| Consumes on attempt | The obligation is consumed regardless of result only when the API's supported semantics justify that fact. This is not a universal destructor rule. |
| Returns on failure | Failed release retains the same live obligation for retry, transfer or explicit residual handling. |
| Poisons on failure | A separate poisoned/indeterminate state records what remains known and what recovery operations are permitted. It is not equivalent to release. |

The general error-region and advanced-control models have different implemented boundaries; the language specification documents them. An abstract model that removes a live item before invoking a fallible destructor describes an attempt-consuming policy, not arbitrary C cleanup.

For external close, unmap, reopen and synchronization operations, a future profile MUST encode its actual supported failure semantics. It MUST NOT derive universal OS behavior from a wrapper that merely suppresses a selected syscall and returns a fixed error. Such an injection records a deliberately controlled operation that did not reach the kernel, with premises determined by that wrapper.

Any harness reclamation after a library's parent teardown MUST be recorded separately and MUST NOT be credited as library cleanup. Likewise, successful synchronization in a fixed profile or observed content equality is not a general durability theorem. Uncertain external post-states require conservative residuals/Unknown rather than assumed cleanup.

The current repository contains the separately rerun reopen-failure package in `docs/reports/cycle19-reopen-failures`: five fixed native processes, with descriptor residuals and harness reclamation kept distinct. That later package was absent from the reviewed archive. No raw source unit is promoted on its basis.

## C11. Verifier protocol v1 and the certificate firewall

The existing schema has protocol version 1 and evidence version 1. Plugin manifests, requests and responses are structured transport/configuration objects. Declared capabilities, target revisions, semantic profiles, required summaries, fact names, checker names and theorem names do not themselves provide a proof.

The reviewed host validates identities, versions, targeting and envelope consistency. An Unknown result stays nonpositive. An Unsafe response is downgraded to Unknown with `QP010` because the host has no supported counterexample checker for that claim. A proposed Safe response passes through the envelope checker but is ultimately rejected; the current checker has no admitted Safe evidence form, including the empty form. See `compiler/src/verifier_plugin.rs:737–836`.

Therefore protocol-v1 plugin results MUST NOT establish Safe or Unsafe solely from the provider's asserted verdict. A future implementation MUST NOT obtain certification by simply removing the final rejection or trusting a theorem-name string. It needs a versioned checker and independently replayable evidence for a precisely scoped claim.

The existing process boundary provides bounded framing/output and timeout-related process control. The manifest's requested permissions and declared resource limits MUST NOT be described as an operating-system sandbox without corresponding enforcement. In particular, plugin declarations alone do not prove filesystem or network confinement or all declared memory limits.

Source-level verifier scopes now enter bounded obligations/evidence dispatch. The v1 path remains non-certifying, and incomplete source imports remain Unknown. A successful plugin listing or manifest inspection does not discharge source obligations.

### C11.1. User-defined contracts and checker plugins

Card W45-USER-CONTRACT-PLUGINS. A user MAY write resource-v1 contracts in their own crate outside this repository, laid out like the `quirin-provider-*` crates (`quirin-provider-NAME`, `NAMESPACE`/`PUBLISHED` constants, `contracts/`, a `contracts.lock`), and list it in a **user load path**: a load-path manifest (`qcore_contract::load_path_manifest`) whose `extends` names a shipped one. The extended manifest is read in full and may not itself extend; the user's crates obey every load-path rule and are validated together with the shipped ones, so a user crate can depend on a shipped crate and can never take a shipped crate's name or namespace. `quirin-audit current|contracts --load-path USER.json --provider quirin-provider-NAME` selects the user's crate; every selected module is lowered as a `crate:module` reference. User contracts obey every rule of this specification — total-or-reject loading (C04), checked-never-trusted bodies, revision modes with the user's own lock and `quirin-audit pin` (which, on a user load path, writes the user's locks only; C06.2), and the review marker (C06.1). The SDK is `providers/quirin-provider-plugin` (the crate layout, `quirin-plugin.json`, the user load path, the template, checker packaging); `quirin-audit plugin views --write` computes the user crates' resource views with the loader.

**Collisions.** A selected user module that declares a function a shipped crate also declares MUST be refused, unless the user load path carries an explicit override naming the function, the user crate, the shipped crate and a reason. Every applied override is recorded in the selection's `user_load_path` record. Declared names are read lexically (`extern fn NAME` outside comments), which can only over-report a collision.

**Checker plugins.** A user MAY attach an out-of-process checker to a user module's contracts (`checkers`: crate, module, sorted functions, package directory). It is a process provider: a package verified as every package is (digest and embedded-template binding), run once per audit through `VerifierRegistry`, under the process transport's deadline, output bound, execution profile and quotas. It MUST NOT run in-process. The request is protocol v1 framing with evidence v2 over a task the core derives: one `parameter_mode_respected` obligation per bound function, bound to the module's text digest (semantic profile `quirin.user-contract-check.v1`). The firewall (`qcore_verifier_host::plugin_firewall`) classifies the answer:

| The plugin | Verdict |
|---|---|
| `Safe`, and the registry issued a core certificate (a trusted-core witness checker replayed the witness against the core's obligation inventory) | `Safe` |
| `Safe` with no evidence (`QP008`) or with evidence no core checker exists for (`QP016`) | `Pending` (`plugin-claim-uncertified`) |
| `Unsafe`, and a trusted-core counterexample checker validated its witness for obligations of the core's inventory | provisional `Unsafe` |
| `Unsafe` with no witness, or no counterexample checker (`QP010`) | `Pending` (`plugin-claim-uncertified`) |
| `Unknown` | `Unknown` |
| a gate (`QP001`–`QP005`), a spawn failure, crash, timeout, quota or output breach or protocol error (`QP006`), an invalid envelope (`QP007`), a package that does not verify, or evidence that does not bind or that the checker refutes (`QP009`) | `Unknown` |

`Pending` is the answer exactly where the claim was never checked, so a person can resolve it by supplying something checkable; a claim that was checked and failed, or that does not bind, is `Unknown`. The command line registers no trusted checker for user contracts, so every plugin `Safe` and `Unsafe` it sees is `Pending` today; the certified row is reached through the library route with a trusted checker supplied by its caller. A bound function's verdict is joined into every function whose control-flow graph applies its contract (a callee chain), and each unit into its functions' (language specification §L21.2). The contract sidecar carries a `plugin_checks` section (each binding's package identity, execution profile, limits, claim, code, verdict and reasons) and the console a one-line tally. The Lean model is `QCore.VerdictSemigroup`.

### C11.2. Audit subcommand plugins

Card W45-AUDIT-SUBCOMMAND-PLUGINS. `quirin-audit NAME ARGS…`, for a `NAME` that is no built-in subcommand, runs the executable `quirin-audit-NAME` out of process. It MUST be found only on the declared search path (`--plugin-path`, then `QUIRIN_AUDIT_PLUGIN_PATH`), never on `PATH`; a built-in name MUST run the built-in and never an external executable; an unknown name is a usage error. The arguments after `NAME` pass verbatim, and the environment is cleared except a documented allow-list and variables named by `--plugin-env`. The protocol is versioned JSON (`quirin-audit-plugin` version 1): one request on standard input (subcommand, arguments, invocation directory, output, report, load path and providers, the audit identity — revision, contract-set digest, refutation-table digest — the audit report and `.qo` checkpoints with their digests, and the quotas), one response on standard output (outcome, artifacts with SHA-256, diagnostics, verdict claims). The plugin runs in its own process group, its working directory the declared output, under a wall-time quota and byte quotas on standard output and on standard error (captured to a file); a breach kills the group. Every artifact MUST resolve inside the output with its declared SHA-256. This is containment, not an operating-system sandbox (C11).

A verdict claim carries the claim tuple of C13 and a proposed `Safe`, `Unsafe` or `Unknown`. It MUST enter the result only through the C11.1 firewall: a `Safe` counts only with a certificate a trusted-core checker accepts and is otherwise `Pending` (`plugin-claim-uncertified`, `QP008`/`QP016`); an `Unsafe` counts only with a validated witness and is otherwise `Pending` (`QP010`); an `Unknown` is `Unknown`. No checker accepts a subcommand plugin's certificate today, so no subcommand plugin reaches `Safe` or `Unsafe`. A spawn failure, timeout, quota breach, nonzero exit, malformed response, protocol-version mismatch, reported failure or unverified artifact makes the run `Unknown`, and a failed run's claims join nothing. Each result row is the join (§L21.2) of the host's verdict for the artifact (from an audit report), the run's and the claims about it; the join never lowers a host verdict. The report and `<report>.invocation.json` record the plugin's path, SHA-256, protocol version and exit, the request, the quotas, the verified artifacts and every claim with the plugin's identity. The route writes no `.qo`. The SDK is `crates/quirin-audit-plugin`, which links no core analysis crate; the Lean model is `QCore.SubcommandPlugins`.

## C12. Obligations/evidence integration and remaining requirements

A source-summary importer that accepts public profile records MUST enforce its
audited-file and required-witness rules even when the records did not pass through
the manifest loader. Every witness must name an audited file; duplicate file or
witness identities, malformed digests, empty construct descriptions and unusable
substring checks reject. Source bytes must match the declared digests and every
required witness must pass before a library certificate is returned. Missing
witnesses cannot be skipped to produce a partial certificate. The current generic
importer and manifest loader share these rules; this does not authenticate the
profile's declared semantics or the caller's source/revision provenance. See
[boundary evidence](reports/w2-source-profile-boundary/README.md).

The reviewed archive contained W2/W3 designs. The current repository has versioned profile loading, a v2 witness schema, bounded replay/dispatch, and a provider-independent host crate, documented in their execution reports. The following requirements govern those implementations and their remaining extensions; a design document alone never establishes completion.

The core MUST derive an obligation set from the requested source/property and record stable subject identities. A provider MUST NOT choose a smaller obligation set and then certify that smaller set as the whole request. Evidence MUST bind the exact source/build inputs, importer version/coverage, semantic profile, contract assumptions, obligation identities, relevant dependency summaries and checker/evidence version. Current observation replay also checks exact contract-module/function identity, task/obligation observed-run bounds, and the deferred ObservedRuns scope marker. Replay does not authenticate the supplied native run provenance; `observations_trusted` remains an explicit condition.

Every accepted claim MUST be checked by a supported independent checker or a verified proof-checking boundary. Unknown, malformed, unsupported, oversized, stale or mismatched evidence MUST NOT become Safe. An Unsafe claim needs a checked counterexample or an admitted proof of violation for the same subject and property. Bounded evidence must retain its bounds.

Coverage composition requires explicit set accounting. The union of accepted discharges must cover all requested obligations, with compatible assumptions and semantic profiles. Duplicate evidence is not extra coverage. An empty evidence bundle cannot discharge a nonempty request. Uncovered or unsupported obligations remain Unknown. Local proofs do not automatically compose through unverified wrappers or callees.

Lifecycle scans MUST recursively visit relevant expressions and distinguish callback/indirect-call uncertainty. Summary caches MUST include all semantic inputs that can change the conclusion; changed contracts, profiles, compilation flags, source selections, transitive assumptions or checker versions invalidate the affected cache entry.

The old v1 evidence path MUST remain non-certifying. Introducing a new accepted evidence format requires an explicit schema/version transition, compatibility tests, mutation tests and a documented migration. The r0.3 documentation does not itself perform that transition.

## C13. Claim identity and reporting

Each safety, contract or observation conclusion MUST be interpretable as a tuple:

```text
(artifact, property, semantic profile, scope, bounds,
 assumptions, coverage, evidence/checker identity, result)
```

The tuple is a reporting requirement, not a claim that every existing schema already serializes these exact field names. Until the implementation carries all relevant information, the public report must supply missing qualifications or refuse the stronger claim.

`Safe` means the specified property has been established within that tuple, not that the program is safe in all respects. `Unsafe` requires a supported violation argument. `Unknown` means that the available supported analysis did not establish the requested result; it is not evidence of a defect and is not proof of safety. `Pending` (card W45-USER-CONTRACT-PLUGINS) means the result was not decided and awaits a named human act — a review, a re-pin or a checkable certificate — and is likewise neither evidence of a defect nor proof of safety (language specification §L21.2). Parser acceptance, contract loading, observer `Checked`, abstract event checking and Lean theorem checking retain their own result categories rather than being relabeled indiscriminately.

Diagnostics SHOULD distinguish malformed syntax, unsupported vocabulary, missing premise, failed source import, observed policy violation, residual custody, incompatible summaries, rejected evidence and execution-environment failure. A LeakSanitizer containment failure, missing toolchain or unavailable source checkout is not a program-level safety verdict.

## C14. Migration and acceptance requirements

Existing valid resource-v1 modules keep their exact meaning. This revision does not broaden the loader to swallow lifecycle clauses, change nullable into arbitrary failure semantics, add a hidden sentinel policy, or infer new modes. Existing observation policies retain eager checked arithmetic and their outside-preconditions distinction. Current v1 plugin Safe refusal remains intact.

A conforming implementation change MUST include positive, negative and mutation tests at the relevant boundary. Resource-loader tests must cover whole-declaration rejection, duplicate/missing modes, invalid owner relations, pointer/scalar mismatches, manifest validation and immutable assumption replay. Observation tests must cover missing facts under apparently unused branches, arithmetic overflow, false preconditions, false postconditions and provenance-binding failures in the adapter. Concurrent/resource integration tests must cover destruction with outstanding authority, post-join events, incorrect mutex-footprint bindings and external failure residuals.

W2 profile data, W3 evidence v2, richer W4 inference, provider extraction and additional CT mappings MUST be reported independently. A specification requirement or a proposed regression does not turn an unmapped CT case into a mapped one. The current manifest has five mapped cases out of fifty. Current validation and the separately unexecuted archive review are distinguished in [VALIDATION.md](VALIDATION.md).

The original review was documentation-only; this integration also tracks separately validated Rust/Lean repairs. The advertised review probe package was not among the five supplied files. Current regression tests and evidence are linked by the acceptance ledger; no missing probe execution or archive reconstruction is claimed.

## C15. Contract inclusion and inheritance

Card CONTRACT-INHERIT. A contract module MAY take a declaration's contract from another declared contract instead of restating it:

```qcore
module contract;
#include "quirin-provider-libc:libc/string.qv";

extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
contract inherit libc::memcpy;
```

**Resolution happens before parsing.** Both forms are resolved against an explicit published load path by `qcore_contract::contract_inclusion` and elaborated into the contract text a person would have written by hand; that elaborated text is what every consumer parses and what the environment retains as its source assumption (C05). The structural parser MUST keep refusing both forms, so an unresolved inclusion never reaches a consumer as an extern declaration without a contract block. Every loader entry point that has no load path (file list, manifest, source map) MUST refuse inclusion syntax (`QI013`), never skip it.

**Includes.** `#include "CRATE:MODULE";` directives follow `module contract;` and precede every declaration. `CRATE` is the full Cargo package name of a contract crate; `MODULE` is a relative `[a-z0-9_/-]+` path ending in `.qv` or legacy `.qcore` under that crate's `contracts/` tree with nonempty segments, so absolute paths, `.`, `..`, backslashes and escapes are unrepresentable. A module of crate `A` MAY include a module of crate `B` only if `B` is `A` or one of `A`'s declared dependencies (no transitive visibility), and only a module in `B`'s declared published set — for own-crate includes too. Include cycles and a target included twice MUST be refused. No search order exists, so no first-match rule exists.

**Namespaces.** Every crate on a load path MUST be named `quirin-provider-NAME` or (card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING) `qcore-provider-NAME`; these are exactly the admitted prefixes, and no namespace is ever recorded without derivation. Its namespace is `NAME` with `-` read as `_` (`quirin-provider-fluent-bit` → `fluent_bit`, `qcore-provider-librdkafka` → `librdkafka`), it MUST be recorded beside the crate's contract list and equal the derived one, and no two crates on one load path MAY share a namespace — including one remainder under both prefixes (`quirin-provider-libc` and `qcore-provider-libc` both derive `libc`, and are refused together, `QI017`). The include string names the crate by its package name, which is what the dependency rule checks; `NS::` is the namespace derived from it. A namespace names the crate that owns a contract, never a bundle and never the functions' own naming prefix: nothing is re-exported, and Monkey's `mk_*` contracts, owned by `qcore-provider-libevent`, are `libevent::mk_mem_alloc`.

**Inheritance.** `contract inherit PARENT;` on a plain `extern fn` gives it the parent's contract. `PARENT` is `NAME`, resolved only among the module's own declarations, or `NS::NAME`, resolved only among the modules this module includes directly from the crate with namespace `NS`. Exactly one declaration MUST match; an unqualified name that only an included module declares, an unknown or unincluded namespace, two matching declarations and a child named like its qualified parent MUST each be refused. The parent MUST be a plain `extern fn … contract { … };` declaration. The child's parameter count, each parameter type (as a token sequence) and its result type MUST equal the parent's. Parameters are bound **by position**: the child's clauses are the parent's clauses, in order, with each word naming the `i`-th parent parameter replaced by the `i`-th child parameter. A child parameter name occurring in the parent's clauses as any other word (capture), and any parameter name occurring as a selector word, MUST be refused. Chains are admitted; an inheritance cycle MUST be refused. Clauses after `inherit PARENT` are not admitted in this version.

**The consumer's own reading.** The renaming preserves meaning for a consumer that resolves every clause word by parameter lookup. A consumer that reads some positions structurally MUST compare its reading of every inherited declaration with its reading of the parent and refuse a difference. The resource-v1 loader (`load_published_resource_contracts`) compares custody modes, the result mode, literal-argument premises, the variadic tail, the ABI projection and (card W45-CONTRACT-DEFERRED-REVIEW, C06.1) the review markers, and admits only equality. An admitted inherited contract is therefore exactly as strong as its parent under that consumer; it is never weaker.

**Trust and provenance.** An inherited contract is a trusted assumption of the same strength as a hand-written one. It is not a proof, and its parent being loaded elsewhere is not evidence. An environment that used inclusion MUST carry, per selected module, its identity and text as written, every module it reached by inclusion (crate, crate version, module path, SHA-256 and text), and each inheriting declaration's chain to the declaration whose contract is written out. The chain is part of the serialized environment, so a change to a parent's text changes the environment's identity even when the elaborated text does not change. `check_source_assumptions` MUST replay every elaboration from the retained texts alone and refuse any difference (`QI016`). A report's assumptions block carries the same provenance (`contract_inclusion`). An environment without inclusion serializes exactly as before.

**Packages.** A provider package whose contracts include another module MUST pin every module it reaches (crate, version, module, SHA-256) in its contract inventory's `includes` object and install the exact texts under `includes/CRATE/MODULE`; `providers/package.py verify` refuses an unpinned directive, a changed or missing parent, an unused pin and an unpinned `includes/` tree. `qcore_contract::bundle::ContractBundle` applies the same rule to an independently pinned inventory.

**The real load path and selections** (card CONTRACT-INHERIT-ADOPTION). Each `quirin-provider-*` crate records `NAMESPACE` and `PUBLISHED` beside `CONTRACTS`, and each executable `qcore-provider-*` crate records them in its `src/main.rs` (card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING; the conformance harness `qcore-provider-conformance` owns no contracts and is not a load-path crate). `python3 providers/contracts.py sync` writes `providers/contract-load-path.json` from Cargo metadata (package name, version, and the normal dependencies that are themselves load-path crates) and those constants, with every owned module listed and every published module's SHA-256 pinned; `check` refuses a stale file. Core consumers read it as data (`qcore_contract::load_path_manifest`), never by linking a provider crate, and MUST refuse (`QI014`) a malformed manifest, a module list that differs from the crate's contract directory, and a published module whose text is not its pin. A resource manifest of **schema 2** names that file as `load_path` and MAY select a module by `CRATE:MODULE` reference (the `#include` spelling) beside path entries; path entries MUST resolve and be retained exactly as in schema 1, a reference is elaborated and gated as above and retained under its `CRATE:MODULE` key, and one total-or-reject load covers both kinds. The corpus index `contract/corpus.json` schema 2 names the same load path so that a corpus consumer reads an inheriting module as its elaboration. Clause-count pins count elaborated clauses; the written inheritance is pinned separately.

The refusal codes are the loader-owned `QI001`–`QI020`, listed in [the diagnostics namespace page](agents/diagnostics.md). Implementation: `crates/qcore-frontend/src/contract_inclusion.rs` (directive scanner), `crates/qcore-contract/src/contract_inclusion.rs` (load path, resolver, elaborator, replay), `crates/qcore-contract/src/contract_loader.rs` (v1 loader). Lean model: `proof/QCore/ContractInherit.lean`. Rationale and evidence: [CONTRACT-INHERIT](reports/contract-inherit/README.md).
