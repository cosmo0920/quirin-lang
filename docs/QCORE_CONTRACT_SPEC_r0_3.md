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

For a contract owned by a published load-path manifest, the resource-v1 admission check is explicit:

```sh
quirin check --contract-load-path providers/contract-load-path.json providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv
```

This selects one owned contract module, including an owned module that is not itself published. Each included parent must be published by an owned or directly declared dependency crate and must satisfy its recorded digest. The result validates trusted resource-v1 assumptions and inherited-clause agreement; it checks no C bodies and establishes no safety verdict. Without this option, unresolved inclusion still refuses at the structural parser.

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

The shared envelopes are `requires expression;`, `ensures selector expression;`, and `quant selector expression;`. Selectors are `normal`, `any`, and `error(E)` with nonempty type text. Card QUANT-INDEX-CHECKS adds a fourth, `quant checks expression;`, which carries no selector: the structural parser retains it, and only the bounds consumer reads it ([C09.2](#c092-quantified-index-checks-quant-checks)). Structural retention of `any`, a typed error, or a quantitative clause does not extend the narrower observation or resource-v1 vocabularies. Those consumers reject unsupported forms.

The source parser rejects empty clauses and unknown clause introducers. It does not implement a general theorem prover for the retained expression text. Language-level effects, `own`, `borrow`, usage annotations, access requirements and task protocols need their specified source/model elaborators; the external resource loader is not a fallback elaborator for them.

## C04. Resource-contract v1: total-or-reject loading

The implementation is `crates/qcore-contract/src/contract_loader.rs`. The public entry points load file lists, an immutable map of source texts, or a versioned manifest. The result is a `ResourceContracts` environment from `qcore-schemas`.

The selected source set MUST be nonempty. Every selected source MUST parse as a nonempty contract module. Every declaration MUST be an extern function, and function names MUST be unique across the complete selection. No partial environment escapes if a selected declaration fails. The loader does not skip observation modules, resource types, constants, errors, or unsupported lifecycle functions.

Every extern declaration MUST satisfy all of the following:

1. Its entire signature belongs to the admitted ABI projection, with unique parameter names and no usage annotations, generics, effects, extra signature syntax, or ignored tail tokens.
2. It has exactly one `requires resource_contract_v1;` marker. The only other admitted preconditions are the literal-argument premise of C04.5, the checked-specification marker of C06.3, the loop-owner obligation of C04.9, the thread-synchronization obligation of C04.10 and the named premise of C04.18; every other `requires` spelling is rejected.
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
ptr<mut, ptr<const, u8>>
```

**Card W43-ABI-ENUM-AND-CONST-PP (gateway rules 12u, 12v, 12w).** The user's
decisions, recorded verbatim from the card's brief:

> **USER DECISION 2026-10-05.**
> 1. **Integer projection for enum typedefs.** A parameter or result whose C type is an enum (tagged or untagged, through any typedef chain) is spelled in the contract as its **underlying integer type**, e.g. `i32` or `u32`. The reader takes that type from Clang's AST for the target (`fixedUnderlyingType` or the computed enum integer type), never assumes `int`, and requires an exact width and signedness match. If Clang does not state the type, refuse. No new type grammar; the contract does not record the enum's name.
> 2. **Shapes covered: enum typedefs and pointer-to-pointer-to-const** (`const char **`, `const void **`; today `pointer_projection` admits only `T **`). State the const-qualification rule precisely: which level is const, and what a `ptr<…>` spelling must say. Transparent unions (`__CONST_SOCKADDR_ARG`) and `va_list` stay registered, out of scope.
> 3. **Conditional clauses on an enum argument's value, in this card.** Some callees behave differently by an enum argument's value. `rd_kafka_confval_set_type` retains `valuep` only when `valuetype` is the PTR constant (5 functions alone). Add a guard, e.g. `when PARAM == CONST` (design the exact spelling; reuse the QUANT-INDEX-CHECKS comparison operators `==`, `<`, `>`, `<=`, `>=` and their evaluation model). A clause then applies only when the caller's actual is decided to satisfy the guard. An undecided actual gets the **cautious reading**: the union or join of every guarded clause, never the favourable one. Guards are over integer-typed (including enum-projected) parameters only, and constants are integer literals. Do not name enum constants: the integer projection carries no names, so state the value with its source citation in a comment. Requirements: total-or-reject grammar; guards must partition or cover: an uncovered value gets the cautious reading; Lean: the guard semantics, the cautious join, and the theorem that an undecided guard never yields Safe where the cautious reading would not; body check: where the body is available (W4 validation), each guarded clause is validated under its guard.

Decision 3's example does not hold of the body it names: the card read
`rd_kafka_confval_set_type` (`rdkafka_conf.c:3763-3874` at `66910c10` and
`046accf6`) and its retaining arm switches on `confval->valuetype`, the
holder's own stored type, never on the `valuetype` argument. Its declaration
therefore carries the cautious, unguarded retention; the guard (C04.14.1) is
in the vocabulary with no corpus declaration using it.

*Enumerations (rule 12u).* A consumer reading the C side MUST project an
enumeration type, at a parameter or a result, reached directly (`enum TAG`)
or through any typedef chain, untagged or tagged, to exactly one spelling:
the scalar of the width and signedness of the integer type **Clang states for
that enumeration in the analysed unit** (an enumeration without a negative
value is `unsigned int`, `u32`, on the audited target; one with a negative
value `int`, `i32`; a value beyond `int` a 64-bit type; a fixed underlying
type its own). It MUST NOT assume `int`, MUST NOT admit the pointer-width
`isize`/`usize` readings an ordinary `long` also has, and MUST refuse
(withdraw the declaration as unreadable) whenever Clang does not state the
type: a failed launch, an incomplete enumeration, a type the probe cannot
name at file scope, two or no compatible standard types, a signedness that
disagrees. The C gateway asks Clang through one appended probe per unit
(`sizeof`, the sign of `(T)-1`, and `__builtin_types_compatible_p` against
the ten standard integer types); the per-unit frontend request carries no
`--target`, `-fshort-enums` or `-m32`, the boundary `ABI-ENUM-UNIT-FRONTEND-FLAGS` records (as
CG20 recorded for plain `long`). A pointer to an
enumeration is an object pointer as before. The Lean model is
`QCore.AbiEnumConstPointer`.

*Pointers to pointers to const (rule 12v).* `ptr<A, ptr<B, u8>>` reads its
qualifiers from the outside in: `A` qualifies the pointee of the outer
pointer, which is the slot (the inner pointer object), and `B` qualifies the
innermost pointee. `ptr<mut, ptr<const, u8>>` is therefore exactly a pointer
to an unqualified pointer to a `const` object: `const char **`,
`const void **`, `const T *p[]` (adjusted), and a typedef chain whose
innermost specifiers carry the `const`. A qualifier on the slot — between the
two `*` (`char *const *`), on a typedef name that stands for a pointer
(`const str *` for `typedef char *str;`), or on a link of a typedef chain
that stands for a pointer — has no spelling and MUST be refused. `char **`
and `const char **` are distinct types (C11 6.7.6.1p2, 6.7.3p10), so neither
spelling admits the other. The new spelling carries custody as
`ptr<mut, ptr<mut, u8>>` does, except that `resource_out` still requires
`ptr<mut, ptr<mut, u8>>` exactly.

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

**Trailing arguments (cards O04-TRACE, CG26-VARIADICS).** A declaration of a C variadic function names only its named parameters. `ensures normal resource_variadic_data_tail();` declares that the callee takes no custody of, and retains no pointer to, any argument beyond them; a consumer MUST still establish at each call site that every trailing actual is data (it carries no tracked identity), and MUST refuse otherwise. `ensures normal resource_variadic_borrowed_tail();` declares the same and that every trailing position is `resource_borrowed`: a consumer MAY then read a trailing actual that names a tracked identity as a borrowed position (with the caller-side refusal below), and MUST refuse every trailing actual it reads neither as data nor as such a place. The borrowed tail implies the data tail; a declaration carries at most one of the two, both are rederived from the retained texts (`variadic_data_tails`, `variadic_borrowed_tails`), and an environment with no declared sources has neither. Neither says anything about a format string: argument/format agreement is not a custody fact.

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
| `resource_return_borrowed_from(p, n)` | The result is borrowed from the parent `p` designates (an interior pointer, a child it owns, or a child created in it), or NULL under `nullable`; it owns nothing and is ended by the parent's release or mutation. See §C04.3.4. |

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

#### C04.3.3. The no-return exit

Card CG21-CALL-EXITS-AND-BOUNDARIES added `ensures normal resource_no_return();`, a whole-declaration clause like the variadic data tail. It states that the call has **no normal exit** — its normal postcondition is false, so every other `ensures normal` clause holds vacuously and states only what the call requires of its actuals — and that the exit it takes instead is **program termination** (or undefined behaviour): no frame of the calling thread resumes. A non-local exit (`longjmp`, `siglongjmp`, `setcontext`, `pthread_exit`, `thrd_exit`) is not this exit: a frame's live resources are abandoned there while the program goes on. The C `noreturn` attribute cannot tell the two apart, so the kind is declared, never inferred.

**Placement.** The result MUST be `unit` with `resource_return_none()`, every parameter MUST be `resource_borrowed`, and no counted-user, container, loop-owner, thread-synchronization or lifecycle clause may stand beside it; anything else rejects the whole module. Independently of any clause, a declaration of one of the names the loader lists as undeclarable (`UNDECLARABLE_C_NAMES`: the Clang extension nodes spelled like calls, such as `__builtin_choose_expr` and `__builtin_offsetof`; the returns-twice calls, such as `setjmp`; the non-local exits above) is rejected: no v1 mode states their node or their exit, and an ordinary declaration would give the call the continuation of an ordinary call.

**Semantics.** The flag is not carried on the custody record; consumers rederive it from the retained trusted texts (`no_return_callees`), so an inferred or proposed contract never carries it. A consumer that ignores it applies the declaration as an ordinary borrowing call whose continuation C does not have: an over-approximation of the paths, never the unsound direction. The C gateway reads it (gateway rule 11o) and lowers a call **statement** as a path end — its modes applied as preconditions, no successor, no exit state — only where the call's own Clang declaration is `noreturn`; a declaration the Clang declaration does not establish is refused at the call. Versioning is as for C04.3.1: an addition to vocabulary version 1, marker unchanged. The Lean model is `QCore.CallExits`.

#### C04.3.4. Borrowed results

Card SLOT-MOVE-AND-INTERIOR-BORROW (gateway rules 12y and 13a), on the user's decision recorded here verbatim from the card's brief (it also governs C04.15 and C04.16):

> **USER DECISION 2026-10-05.**
> - **Class 2, slot-move custody.** This is the custody counterpart of CG24's `bounds_frame_moves`.
>   - After the call, the slot `T **s` still owns an object, possibly a new one.
>   - Every alias of the old pointer is invalid (a use is refused, never Safe).
>   - On failure the original stays owned by the slot.
>   - It composes with rule 11r's dispositions where the success/failure split is by return code.
>   - Members (W45 run sizes, functions decided alone): `sds_cat_safe` 4, `cfl_sds_cat_safe` 3, `format_metric` 3, `mk_header_prepare`, `rd_kafka_parse_Metadata0`, and two unit-test constructors (7 callees / 18).
>   - Read each body at both revisions; some may not be pure slot moves.
>   - Also: system `flb_config_task_map_grow` (a field reallocation). Include it only if the slot reading covers a field slot honestly; otherwise record it.
> - **Class 8, interior borrow of parent.**
>   - The result is borrowed from the parent: no release obligation for the caller.
>   - It is valid only while the parent lives and is not mutated through another path (a parent release or an invalidating mutation ends it, and a later use is refused).
>   - A write through it is a write to the parent.
>   - A lazily created child (`get_or_create_*`, `rd_kafka_topic_partition_get_private`) is created inside and owned by the parent.
>   - Relate it to the interim cfl_arena interior reading (memory `arena-pool-decision`; contract spec C04.3.1's derived-result rules) and do not contradict it.
>   - Members: `rd_kafka_admin_result_ret_groups` 4, `_resources` 3, `_topics` 3, cJSON `get_object_item` 2, `get_or_create_external_metadata_kvlist` 2, `rd_kafka_topic_partition_get_private` 2 (6 / 16).
> - **Also absorb `INTERIOR-OUT-SLOT-MODE`.** It is the same class-8 reading delivered through an out slot. A derived out-slot mode: the slot receives a pointer derived from parameter k, owning nothing, under C04.3.1's rules.
>   - Members: `rd_slice_reader0` 2.
>   - `cJSON_ParseWithLengthOpts` 2: its interior out slot plus its global holder, via rule 12t.
>   - The previously refused libc `strtol`/`strtoll`/`strtoull`/`strtod` (W43-LIBC-CONTRACTS): declare them in quirin-provider-libc if the mode covers them.
>   - Retire or amend that entry.

```qcore
extern fn child(parent: ptr<mut, u8>, key: ptr<const, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(parent);
    ensures normal resource_borrowed(key);
    ensures normal resource_return_borrowed_from(parent, nullable);
};
```

`resource_return_borrowed_from(p, n)` loads to the derived result of C04.3.1 at a third extent, `ResourceReturnMode::Derived { extent: Borrowed, … }`: the result is **borrowed from the parent** `p` designates — a pointer into it, a child object it owns (`get_object_item`, the admin result arrays), or a child the call creates and links into it (`rd_kafka_topic_partition_get_private`, a lazily created child). **Placement** is C04.3.1's, except that the result may also be `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>` (the parent's array of child pointers: `rd_kafka_admin_result_ret_groups` returns `const T **`); `const_dropped` is set when the base is `ptr<const, u8>` and the result's outer pointee is mutable.

**Semantics.** C04.3.1's rules apply, strengthened and never relaxed:

1. **No release obligation**, and no release: handing a borrowed result to any custody-bearing position is refused, never accused (it is never `invalid_release`, since the child may sit at its own allocation's start).
2. **Valid only while the parent lives and is not mutated through another path.** A consumer MUST end a borrowed result at the parent's release and at every operation that may mutate the parent through anything but the borrow itself: the C gateway ends it at every call or store whose actuals or written places name the parent, an ancestor of it, or any other name outside the borrow's family (its aliases and the borrows taken from them), and at every call with an actual it does not read (rule 12y, premise P-BORROW-FRAME: a call reaches only what its actuals reach). A later use of an ended borrow is **refused**, never accused (`interior-borrow-ended`).
3. **A write through it is a write to the parent.** A call or store through the borrow keeps the borrow, and ends every other borrow of the parent (its siblings).
4. **A lazily created child is created inside and owned by the parent**: the caller owes nothing, may not release it, and the call creating it is itself a mutation of the parent (rule 12y ends the parent's other borrows at every borrowed-result call).
5. **Nullness is the declaration's own** (C04.3.1 rule 4): `nonnull` makes the result non-null whatever the base's binding says.
6. **Forwarding.** The derived-result inference of C04.3.2 publishes a `Borrowed` summary when any exit returns a borrowed result, never a plain interior one, so a wrapper's callers apply these rules too.

**Relation to the arena reading.** The interim `cfl_arena_calloc` reading (`resource_return_interior(arena, nullable)`, W47-ARENA-INTERIOR-READING, the user's decision of 2026-09-28) is unchanged: an arena result stays an interior result of C04.3.1, invalidated by the arena's release, and the pool resource kind remains the target (`W47-ARENA-POOL-RESULT-MODE`). A borrowed result is the stricter reading of the same relation (it also ends at a mutation and is never accused); nothing here reads an arena result as borrowed.

**Checked against bodies (rule 13a).** In a unit that defines the declared function, the declaration stands only when the body validates it structurally: the base formal is never assigned and its whole address never taken, and every `return` is NULL or a pointer reached from the base by member reads (through casts, parentheses and offsets), through locals every assignment of which is so reached, or a local assigned from a call whose result the body stores into a member of a reached name before returning it (the lazily created child). A body that does not validate withdraws the declaration in that unit (every call there sees an uncontracted callee, and an inferred summary of the body governs). The Lean model is `QCore.InteriorBorrow`.

### C04.4. Unsupported semantics and false declarations

Only normal C returns are represented by v1 postconditions, apart from the no-return exit of C04.3.3. A C routine returning null or a negative integer has still returned normally; it has not taken a QCore typed-error exit. Error-specific ownership behavior can be modeled only where a supported mode explicitly represents it, such as nullable acquisition or the specified reallocation relation.

General success-dependent parameter modes (beyond the return-code dispositions of C04.13), exceptional exits, `ensures any`, `ensures error(E)`, quantitative clauses other than the counted-user clauses of C04.7, effects, access permissions and synchronization are not in v1; the loop-owner obligation of C04.9 and the thread-synchronization obligation of C04.10 are call-site preconditions, not synchronization clauses. Non-null pointer error sentinels such as `MAP_FAILED` are not representable. The scalar failure sentinel `-1` of a descriptor is, through the `descriptor` kind and its `sentinel`/`valid` validity words (§C04.6); no other scalar sentinel is, and nullability is never reinterpreted as one.

**Value-dependent parameter modes are not in v1.** A parameter mode is fixed per declaration; no clause conditions a *mode* on an argument's value, and C04.5's premise does not add one — it restricts where the whole declaration applies, leaving every mode exactly as written. The one value-conditioned clause is the guarded retention of C04.14.1 (card W43-ABI-ENUM-AND-CONST-PP), which conditions a `resource_retained_by` clause, never a mode.

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

`KIND` is exactly one of `descriptor`, `stream`, `directory`, `dynamic_library` and `regex`, each the kind of a pair a card declares; no kind is added without a declared use (an `FTS *` kind waits on a readable `fts_open`). Card CG25-RESOURCE-KINDS added `dynamic_library` (`dlopen`/`dlclose`, pointer-carried, failure value NULL), once the return route modelled `return (T *)handle;` (card W44-LEAK-PIN-AUDIT), and the in-place kind `regex` (`regcomp`/`regfree`, §C04.6.1). Card REFCOUNT-RESOURCE-KIND added the pointer-carried `reference` kind (§C04.17): a reference is taken only by `resource_reference_kept` and dropped only by `resource_reference_dropped`, `resource_return_kind(reference)` beside `resource_return_owned` is a returned reference the caller owes, and `resource_kind(p, reference)` on a parameter rejects the module. A use of a reference after its last drop is refused there, never accused, as the heap's is (rule 6 below does not apply to it). The **heap** kind is what a position with no kind clause means, and it is spelled only by that absence: writing `heap` rejects the module, so every meaning has one spelling. Every declaration written before this card therefore loads to exactly the contract it loaded to before, with every position heap. Kinds load onto `ResourceFunctionContract` as `parameter_kinds` (non-heap entries only) and `result_kind`, both omitted from the serialized form and from the diagnostic text when heap, so a contract without a kind is byte-identical in every artifact.

**Placement.** A kind clause names a custody-bearing position:

| Position | Admitted kinds |
|---|---|
| `resource_owned`, `resource_consumed`, `resource_released` parameter | every kind (an in-place kind: `resource_released` only, §C04.6.1) |
| `resource_out` parameter | every pointer-carried kind, and an in-place kind as its initialiser (§C04.6.1); a descriptor has no out slot in this vocabulary |
| `resource_return_owned` result | every kind except an in-place one |
| `resource_borrowed` parameter | none — a borrowed position touches no obligation — except an in-place kind's use (§C04.6.1) |
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

**Not in the vocabulary, deliberately.** A kinded stored/removed relation; a descriptor out slot (`pipe`, `socketpair` write an `int[2]`); a status-conditioned out slot (`getaddrinfo`'s list is valid on a zero return, which needs a status witness bound to the list); a command-dependent creator (`fcntl` creates a descriptor only for `F_DUPFD`/`F_DUPFD_CLOEXEC`, and one declaration carries one literal premise per parameter); a transfer that happens on success only (`fdopen` moves the descriptor into the stream only when it returns non-NULL); a pointer sentinel (`mmap`'s `MAP_FAILED`); a process stream released by `pclose`, not `fclose` (no audited unit calls `popen`). (The library-handle kind withheld here until the return route modelled `return (T *)handle;` was added by card CG25-RESOURCE-KINDS once card W44-LEAK-PIN-AUDIT repaired that route.)

#### C04.6.1. In-place kinds

Card CG25-RESOURCE-KINDS (gateway rule 11v). An **in-place** kind is initialised into storage the caller owns: the obligation lives in the object a pointer parameter designates, not in a returned pointer, and the initialising call reports failure only by its **status**. `regex` is the one in-place kind (`regcomp` compiles into the caller's `regex_t`, `regfree` frees what it allocated there). A kind's carrier is therefore one of three: a pointer (failure value NULL), an integer (`descriptor`, failure value `-1`) or caller storage (failure value a nonzero status).

**Positions.** An in-place kind stands on exactly three positions, each on a `ptr<mut, u8>` (or, for a use, `ptr<const, u8>`) that names the object:

| Clause pair | Meaning |
|---|---|
| `resource_out(p, nullable)` + `resource_kind(p, KIND)` | **initialise**: on a zero status the object is initialised and owes a finaliser; on a nonzero status nothing is owed |
| `resource_released(p, nonnull)` + `resource_kind(p, KIND)` | **finalise**: the obligation is discharged |
| `resource_borrowed(p)` + `resource_kind(p, KIND)` | **use**: the object is read or updated and its obligation is unchanged — the one kinded borrowed position |

An initialiser's result MUST be `i32` with `resource_return_none()` (the status), it moves no other custody, and a declaration has at most one initialiser. `nonnull` on an initialiser (an infallible one), `nullable` on a finaliser, an owned, consumed, stored or removed in-place position, and an in-place result each reject the whole module. A kinded borrowed position of a non-in-place kind still rejects.

**Semantics.** A consumer MUST apply all of the following:

1. **The obligation is bound to one automatic object.** An in-place actual is `&NAME` for `NAME` one automatic, non-pointer, non-array object of the function; any other actual of an initialiser or finaliser is a refusal. A use whose actual names no tracked object keeps the plain borrowed reading.
2. **The status is the sentinel.** The initialiser's status MUST be held by a plain, not address-taken name; it then witnesses the object until the name is written. A null test of the witness decides the object: the zero edge certifies the obligation, the nonzero edge removes it. An initialiser whose status no name holds (discarded, or consumed by the expression that tests it) is a refusal, and a finaliser before the status is decided is a refusal.
3. **Tracked storage is custody, never data.** A statement that mentions a tracked object other than as a whole `&NAME` actual at an in-place position of its kind of a declared callee — a struct copy, `memset(&obj, …)` at an unkinded borrowed position, a member read — is a refusal.
4. **Candidates** are those of §C04.6: a finaliser of another kind discharges nothing (`invalid_release` when it resolves), a second finaliser is `double_release`, a use after finalisation is `use_after_release`, and a certified object unfinalised at an exit is a `leak`.

The Lean model is `QCore.ResourceKinds`.

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

### C04.9. The loop-owner obligation

Card W47-LOOP-OWNER-CONTRACTS (gateway rule 10p). A collector or loop operation reads or mutates state that only the thread waiting on the loop may touch: Monkey's userspace priority-queue links and a collector's `running` flag are not protected by the kernel's own `epoll_ctl` thread safety. A resource-v1 declaration MAY carry, beside the selector, at most one

```qcore
requires resource_loop_owner(p, read);
requires resource_loop_owner(p, mutate);
```

`p` names an admitted pointer parameter: the input instance whose collectors attach to the loop `L` (`coll->evl`, which is `thi->evl` for a threaded instance and `config->evl` otherwise). `read` or `mutate` says what the operation does to loop-owned state. Any other spelling, a second clause, a non-pointer or unknown parameter, or a review marker wrapping the clause rejects the **whole module**; the obligation is deferred only by deferring the whole contract (C06.1). An inherited declaration carries exactly its parent's obligation (C15). The clause adds no custody mode and nothing to the environment's serialized form; a consumer rederives it from the retained texts (`declared_loop_owner_obligations`), as C04.5's premise is.

**Meaning.** The declaration applies at a call site only where the obligation is **discharged**: every derived context of the calling function runs on `owner(L)` for the mode it fixes — the single derived thread of the engine loop's dispatcher for a non-threaded instance, of the worker loop's dispatcher for a threaded one — and no path to the caller is unresolved. A consumer that derives no thread identity MUST treat the obligation as undischarged at every call and refuse the call as it refuses an uncontracted one; the gateway's per-unit resource route is such a consumer, so on it the clause can only refuse. The thread-entry census (rule 10p) is the consumer that can discharge it. Its derivation follows direct calls and the hop kinds of the W47-TAIL-PAUSE-BINDINGS census (A, S, V, V\*, R, L, F); every other function-pointer hop, a macro that names a function and a field read as a value leave the reached functions unresolved. One resolved context off its owner refuses the call; an unresolved or absent context withholds the discharge. A refusal is `Unknown`, never a validated reachable violation and never an upstream defect claim.

**Completion meanings and paired collectors.** The public pause/resume dispatches a plugin callback and stays undeclared (C04.4, rule 10j); its two completion meanings are the census kernel's: a non-threaded instance discharges "callback finished", a threaded one only "request enqueued", and no return code alone discharges "applied". Two collector operations of one callback (the tail backend's pair) state a joint state only at the callback boundary, only when both results are known to have succeeded on the owner, and a resume's running state only under an active-ingestion premise. Registration (`flb_input_set_collector_*`) and `mk_event_add`/`mk_event_timeout_create` retain a callback or the caller's event, which v1 cannot state, and stay undeclared.

The Lean model is `QCore.LoopOwner`; it derives, for this route, the single-dispatcher premise `QCore.EventDriven` otherwise takes as supplied.

### C04.10. The thread-synchronization obligation

Card W47-CONDVAR-JOIN-PUBLICATION (gateway rule 10q; constraint 27 as amended). A condition-variable wait releases and reacquires its mutex, a signal is not itself a memory-publication mechanism, and a join ends a thread only when its result says so; none of that is a custody fact. A resource-v1 declaration MAY carry, beside the selector, at most one

```qcore
requires resource_thread_sync(cond, mutex, cond_wait);
requires resource_thread_sync(cond, mutex, cond_timedwait);
requires resource_thread_sync(cond, cond_signal);
requires resource_thread_sync(cond, cond_init);
requires resource_thread_sync(cond, cond_destroy);
requires resource_thread_sync(thread, join);
```

A condvar and a mutex name `ptr<mut, u8>` parameters, distinct; a wait names both and every other condvar operation only the condvar; a joined thread names a `u64` or `usize` parameter (`pthread_t`). Any other spelling or operation, a second clause, a wrong parameter kind, or a review marker wrapping the clause rejects the **whole module**. An inherited declaration carries exactly its parent's obligation (C15). The clause adds no custody mode and nothing to the environment's serialized form; a consumer rederives it from the retained texts (`declared_thread_sync_obligations`).

**Meaning.** The declaration applies at a call site only where the obligation is **discharged**: a wait only as the body of a loop that rechecks a predicate read under the mutex the wait releases, with that mutex held (a timed wait's nonzero return may leave the loop, with no result claim); a signal only while the condvar's paired mutex is held; an `init` or `destroy` only in a quiescent phase (before publication, after the last signal is ordered by the mutex, or after a checked join of every live thread); a join only where its zero result is checked. A consumer that derives none of these MUST treat the obligation as undischarged at every call and refuse the call as it refuses an uncontracted one; the gateway's per-unit resource route is such a consumer. The census of rule 10q is the consumer that can discharge it, and it also decides C5: storage another thread may retain is released only after a checked, successful join of every live thread that retains it; publication, a timeout, an unchecked or failed join and an underived join identity grant nothing. `pthread_cond_broadcast`, C11 `cnd_*` (whose custody declarations in `libc/threads.qv` are unchanged), rwlocks, try/timed locks and `pthread_create` (a retained argument and a callback, C04.4) stay undeclared. A refusal is `Unknown`, never a validated reachable violation and never an upstream defect claim.

**Stop paths (card W47-CONDVAR-JOIN-STOP-PATHS, rule 10q as amended).** The census MAY count an *unchecked* join as successful only where it is the first join of a thread the path itself started and each POSIX error case is derived impossible under the named premise P-POSIX-JOIN (`pthread_join` returns 0 unless the target is not joinable, is already joined or awaited by another joiner, is the caller, or is in a join cycle through it): the thread was created joinable and no detach may name it, no other thread's join may name it, and nothing on the joining thread keeps its identifier. A second join of a thread already joined on the path is undefined behaviour and never counts. A null arm of a field test is excluded only where the field is dereferenced before the test on every path and written, anywhere, only by pre-publication stores (named premise P-DEREF). A thread a live retaining thread starts blocks the release until the parent is derived to join it. Observed thread behaviour of analysed bodies that v1 cannot state (a join on one branch, an ignored result, a return without a join) MAY be recorded as a declaration awaiting review (C06.1); it is withheld and discharges nothing.

The Lean model is `QCore.CondvarJoin`.

### C04.11. The paired lifecycle obligation

Cards W47-FLB-LIB-LIFECYCLE and W47-FLB-LIB-STRICT-PAIRING (gateway rule 10q as amended). A library whose context is started and stopped in pairs (Fluent Bit's `flb_start`/`flb_stop`/`flb_destroy`) states an API contract no custody mode says: which calls are permitted in which state, and what the first stop after a successful start does. A resource-v1 declaration MAY carry, beside the selector, at most one

```qcore
requires resource_lifecycle(ctx, start);
requires resource_lifecycle(ctx, stop);
requires resource_lifecycle(ctx, destroy);
```

`ctx` names a `ptr<mut, u8>` parameter, the context. Any other spelling or operation, a second clause, a wrong or unknown parameter, or a review marker wrapping the clause rejects the **whole module**. An inherited declaration carries exactly its parent's obligation (C15). The clause adds no custody mode and nothing to the environment's serialized form; a consumer rederives it from the retained texts (`declared_lifecycle_obligations`).

**The state machine: a strict pair** (amended by card W47-FLB-LIB-STRICT-PAIRING on the user's decision of 2026-09-29, which supersedes "a stop after the first has no effect"). A context is *created*, *started-ok* by a start that returned 0, *start-failed* by one that returned nonzero, left with its start's outcome *undecided* where the value is not derived on the path, *stopped*, or *destroyed*. The clauses: L1, a start only from created (a start after a stop is permitted only where a body reading derives that the declared start supports it; none does, so a restart is refused); L2, the first stop after a start that returned 0 joins the thread that start began, exactly once; L3, no stop after a stop; L4, no stop after a start that returned nonzero; L5, no stop of a never-started context; L6, a destroy only from created, start-failed or stopped; L7, a start that returns nonzero leaves no thread it began unjoined (the body fact behind L6's admission of start-failed). Nothing is permitted after a destroy. A stop moves started-ok to stopped; a stop or a destroy after an undecided start is **not established**, and refused like a violation.

**Checked, never trusted.** L1 and L3–L6 are preconditions on the caller: they only refuse, so they are in force by construction. L2 and L7 describe the declared bodies (L2 the stop's, run by the path reader after the declared start returned 0; L7 the start's own paths that return nonzero) and are **in force** only where every followed path satisfies them with nothing unresolved; **refuted** where every followed path contradicts them (the reader follows every arm it cannot decide, so only a contradiction on every path is a refutation); otherwise **held**. Only an in-force clause may be applied, and a refutation names a candidate upstream path, never a verified defect. What a stop's body does from a state the preconditions forbid is outside the contract: a consumer MAY report it as an observation and MUST NOT read it as a clause. A consumer that derives no context state MUST refuse every call of a declared function as it refuses an uncontracted one; the gateway's per-unit resource route is such a consumer. The census of rule 10q is the consumer that reads the states: a call whose context's derived state violates a precondition, or leaves one not established, on a path refuses every release that path reaches; a state it cannot derive (no creation on the path, a key whose instance was destroyed) decides nothing. A consumer MAY decide a release apart on its conforming paths (every history derived, none refused) as a report; the release's own verdict stays the join over every path. A context is named by one key per instance (P-LIFECYCLE-INSTANCE, stated), entering *created* where a creator's result is assigned to it.

The Lean model is `QCore.CondvarJoin` (section `Lifecycle`).

### C04.12. FFI function definitions: `[| quant #c_ffi |]`

Card W48-QR-C-STRUCT-IMPORT (user decision 4, 2026-09-30). The attribute marks an **FFI function definition** in a `.qv` contract module, written on the line before the declaration:

```qcore
[| quant #c_ffi |]
extern fn qr_header_total(header: ptr<const, c"struct qr_header">) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(header);
    ensures normal resource_return_none();
};
```

The marker is five tokens the lexer already produced (`[|`, `quant`, `#`, `c_ffi`, `|]`), claimed only at declaration position immediately before `extern fn`; it is not one of the declaration's own tokens, so every reader that reconstructs a declaration reads an unmarked one exactly as before. It belongs to the contract side so an auditor finds every FFI use in the `.qv` files: a runtime or library module carrying it is refused (`Q033`).

A marked declaration is read by its own, closed vocabulary, total or reject like the rest of C04:

* **Types.** A parameter is a §G02t scalar, `c"struct TAG"` (the struct by value) or `ptr<const, c"struct TAG">` (a borrowed `const` pointer); the result is `unit`, a §G02t scalar or `c"struct TAG"`. `c"…"` is the word `c` immediately followed by a string literal whose text is exactly `struct TAG`: a union, an enum, a typedef name, a space before the quote or any other spelling rejects the module, as do `ptr<mut, …>` over a struct, the custody pointers and `fnptr`/`va_list` (`f32`/`f64` are §G02t scalars since card W48-QR-FLOATING-POINT).
* **Clauses.** Exactly `requires resource_contract_v1;`, `ensures normal resource_borrowed(p);` once per parameter and `ensures normal resource_return_none();` — a struct crosses the boundary as **data**: a by-value struct is a copy, a `const` pointer is read and not retained, a returned struct is a copy the caller owns. Any other clause, marker (review, revision, checked specification) or suffix rejects the module.
* **Only a marked declaration may name a C struct.** An unmarked declaration spelling `c"…"` rejects the module by name.

A marked declaration is **withheld** from the environment's function map (the C06.1 precedent): every consumer that knows nothing of the marker sees an uncontracted callee, and `source.ffi.v1` alone rederives the marked declarations from the retained texts (`contract_loader::ffi_declarations`). A `.qr` call that takes or returns an imported struct is admitted only through a marked declaration; through an unmarked one it is refused (`Q033`). Every certificate of a file that calls one lists it, and the file's results hold under the unsafe-FFI assumption (language specification §L14.6, §L20).

**Linkage (card W48-QRLIB-LINKAGE, user decision 5).** A contract says what a function does; which code it is comes from a **generated** `.qrlib` (implemented grammars §G02d.2.1). The generator binds **every** declaration of each contract module it is given — marked or unmarked — as one `contract fn NAME(C_TYPES) -> C_TYPE from MODULE export …;` entry, projecting the declaration's types to C: a §G02t scalar to its stdint spelling (`float`/`double` for `f32`/`f64`), `c"struct TAG"` to `struct TAG`, `ptr<const, c"struct TAG">` to `const struct TAG *`. The FFI route admits a call only when the file's `extern library NAME;` resolves the callee to such an entry naming the same contract module (the selection key, compared exactly) and the same C signature (§L14.8). The contract's clauses are unchanged and still a declared assumption: a manifest proves which code is linked, never that the code behaves as the contract states.

### C04.13. Return-code dispositions

Card CG23-RELATIONAL-POSTCONDITIONS (gateway rule 11r). Many C entry points decide custody by the code they return: a `take(p)` that transfers `p` when it returns `0` and leaves it with the caller otherwise, a `getaddrinfo` whose out list exists exactly when it returns `0`. One clause states that relation:

```qcore
extern fn take(p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));
    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));
    ensures normal resource_return_none();
};
```

`resource_on_return(OP, VALUE, PREDICATE)` declares that `PREDICATE` holds on every normal return whose value `v` satisfies `v OP VALUE`. `OP` is one of `eq ne lt le gt ge`; `VALUE` follows C04.5's canonical spelling and must fit a C `int`; `PREDICATE` is one parameter predicate, `resource_borrowed(p)` or `resource_{owned,consumed,released,out}(p, NULLABILITY)`. The distinct `(OP, VALUE)` pairs are the declaration's **exit classes**. Total-or-reject applies:

* The classes are pairwise disjoint and together cover every integer (decided exactly on the breakpoints `VALUE - 1`, `VALUE`, `VALUE + 1` of every class).
* A parameter is either unconditional (one ordinary clause) or conditional (exactly one disposition per class); every class leaves no parameter without a mode.
* A conditional parameter's entry requirement agrees across classes: every class asks for live custody with the same nullability (`owned`, `consumed`, `released`), or none asks for custody (`borrowed`, `out`).
* The result is an unconditional `resource_return_none()` over a signed integer (`i8` … `i64`, `isize`).
* Each class, with the unconditional clauses, loads through every check of a plain declaration (kinds included). Classes that all load to the same contract are rejected: that declaration is written plainly.
* A disposition never stands beside an in-place kind (C04.6.1, gateway rule 11v): that kind's initialiser status is already its own witness, and one status is not read twice.
* A disposition stands only beside plain parameter and result predicates, kinds and revision markers: a review marker, the checked-specification marker, a literal premise, a variadic tail, and a counted-user, container, loop-owner, thread-synchronization or lifecycle clause each reject the module beside it.

**Out slots.** `resource_borrowed(out)` in one class and `resource_out(out, …)` in another states that the class leaves the slot **unwritten**: the call passes no custody through it there. A consumer passes the actual as data on that class and afterwards treats the slot's variable as holding a value it does not track; it refuses any actual other than `&NAME`.

**Meaning.** The declaration loads to a `RelationalContract` (`crates/qcore-schemas/src/relational_post.rs`), **never** an entry of the environment's function map (the C06.1 precedent), so a consumer that knows nothing of it sees an uncontracted callee. A consumer that honours it MUST apply a class only where the caller **decides** the returned value's class: the gateway does so for an `if` condition that tests the call where it stands against an integer constant, or a value stored by the statement immediately before such a test (rule 11r). Each edge of that test takes the one disposition every class it can reach agrees on; a test whose edge reaches classes that disagree, or none, refuses. A call whose value is not so tested keeps the **join** of the dispositions: since the classes of an admitted declaration disagree somewhere, that is a refusal (`relational-untested`). The retained texts reload to the same dispositions (`check_source_assumptions`).

**Checked against bodies.** Where the unit defines the declared function, its body's own summary decides the declaration: an ordinary summary must equal every class's contract, and an exit-specific summary (rule 11r) must equal, for each returned literal, the contract of every class containing it. A contradiction **withdraws** the declaration: every call site refuses (`relational-withdrawn`). A body that yields no summary leaves the declaration a declared assumption, as C06's rule for an ordinary declaration does. The Lean model is `QCore.RelationalPost`.

**Not in this clause.** A disposition conditioned on an *argument's* value (C04.4, C04.5), a pointer result's nullness as a class (the nullable result modes cover the one existing case), a retained-storage result, and a release of a set reachable from a parameter are not stated by it.

**Forwarding (card CALLEE-RETENTION-CLASS1, gateway rule 12s).** A body whose every `return` is a plain integer literal or `return f(ARGS);` of one callee with a disposition in force, whose result type is spelled exactly as that callee's (so C converts nothing between the two values), takes the callee's classes: each class is validated by the ordinary route on the body restricted to the exits returning a value in it, the forwarded call read through the decided test of that class. A function whose own disposition is in force (declared and not withdrawn, or inferred) has its own row decided per class: `Safe` only when every restricted body validates its class's contract and the joint body reports nothing. A declared disposition, like every declaration, is compared with the analysed unit's C declaration (C04.1) and evaluated against its pin (C06.2); a unit's withdrawals of other names never drop it. The Lean model is `QCore.RelationalForwarding`.

### C04.14. Retained pointers

Card CALLEE-RETENTION-CLASS1 (gateway rule 12t), on the user's decision recorded here verbatim from the card's brief:

> **USER DECISION 2026-10-05.** The decision is on `W48-CALLEE-DECISION-TABLE` class 1 ("callbacks and retained pointers"). The user chose **"split by shape"**. The three shapes: **(a) Custody-taking inserts → conditional custody transfer on success**, using the existing relational postconditions (CG23, gateway rule 11r, contract spec on return-code dispositions), no new vocabulary. **(b) A pointer retained in a holder → a new `retained_by(P, holder)` clause.** The caller **keeps** custody, but must establish that P's object outlives the holder (or that the holder is destroyed or reset before P is released). Otherwise the result is **Pending** (a named, human-resolvable reason) or Unknown, **never Safe**. Releasing P while a live holder retains it is a use-after-free hazard. Validate reachability before any Unsafe; it is provisional, never presented as an upstream defect. **(c) Callbacks over retained elements → CG22's callback contracts** (gateway rule 11p); where the callback contract vocabulary does not cover a member, record exactly what is missing, and size it.

Shape (a) is C04.13 with its forwarding; shape (b) is this clause; shape (c) is C04.19 (card CG22-CALLBACK-REGISTRY-CONTRACT).

```qcore
extern fn keep(state: ptr<mut, u8>, src: ptr<const, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_borrowed(src);
    ensures normal resource_retained_by(src, state);
    ensures normal resource_return_none();
};

extern fn reset(state: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_holder_reset(state);
    ensures normal resource_return_none();
};
```

`resource_retained_by(P, H)` declares that after the call the object `H` designates (or, for the word `global`, storage of static duration or a holder no caller object denotes: a file-scope object, a queue another thread drains) keeps a pointer into the object `P` designates. `resource_holder_reset(H)` declares that after the call `H` keeps nothing it kept before. Total-or-reject (C04): `P` and a holder parameter are distinct depth-one pointer parameters, `resource_borrowed`, with no kind; `P` is retained at most once and a holder reset at most once; a parameter named `global` rejects the clause; the clauses stand only beside plain parameter and result predicates and revision markers. The grammar is G05.

**Meaning.** The caller **keeps custody** of `P`'s object: the call takes none, and `P`'s mode stays `resource_borrowed`. But the object must outlive the holder, or the holder must be reset or destroyed before the object is released: a release of the object while a live holder keeps a pointer into it leaves the holder dangling, a use-after-free hazard the moment the holder reads it. **A retained pointer is never borrowed and forgotten**: the declaration is loaded and validated in full and then **withheld from the function map** (the C06.1 precedent, `ResourceContracts::retaining`), so a consumer that knows nothing of retention sees an uncontracted callee. A consumer that honours it MUST establish that the object outlives the holder; otherwise the call is `Unknown` (or `Pending` under a named, human-resolvable reason), **never `Safe`**. A release while held is refused, never accused: the hazard is real only if the holder reads the pointer afterwards, which the caller does not decide, so no `Unsafe` rests on it.

**What "outlives" is decided from (gateway rule 12t).** The route-G consumer decides exactly these, and refuses everything else with the reason `retention-undecided`:

* **Static storage.** A string literal outlives every holder.
* **The frame.** A holder spelled `&L`, for `L` one automatic, non-pointer, non-array object of the calling function that the body names only as a whole `&L` actual of a retaining or resetting callee, dies when the function returns, and nothing it holds is copied out. An object the caller of this function supplied (an unrebound formal this body holds no custody of) outlives it. An object this body tracks is recorded as held by `L` until `L` is reset or the function returns; a release of it before then is refused (`retention-released-while-held`).
* **A holder reset or destroyed.** A tracked heap holder releases what it holds when it is released; a `resource_holder_reset(H)` call drops what `H` holds. A tracked holder that still holds at an exit (returned, stored or leaked) may outlive the frame: the exit refuses.
* **The program-lifetime holder.** `global` keeps only static storage: any other `P` there is undecided.

A holder the caller supplied (a formal, a member reached through one) or an actual the consumer cannot name is undecided: the caller of *this* function decides the holder's lifetime, and a retention is not forwarded into a summary. Records join by union over paths. The C gateway's per-unit route is such a consumer; every other consumer sees an uncontracted callee. Not in this clause: a retention conditioned on an argument's value (an arm that copies and an arm that keeps; since card W43-ABI-ENUM-AND-CONST-PP the guarded form of C04.14.1), a retention beside a return-code disposition (cJSON's `add_item_to_object`), a pointer derived from `P` handed back through an out slot (since card SLOT-MOVE-AND-INTERIOR-BORROW, C04.16), and a callback the holder invokes over what it keeps (shape (c), C04.19).

**Checked, never trusted.** A retaining declaration is pinned (C06.2) and evaluated against its pin like every declaration, and compared with the unit's C declaration (C04.1); no inference replaces it in a unit that defines its body. Its body is not yet checked against the retention (`RETAINED-BY-BODY-CHECK`): it is a declared assumption (constraint 4). The Lean model is `QCore.RetainedBy`.

#### C04.14.1. Guarded retentions

Card W43-ABI-ENUM-AND-CONST-PP (gateway rule 12w), on the user's decision 3 recorded in C04.1:

```qcore
ensures normal resource_retained_by(P, H) when X OP K;
```

**Grammar** (G05, total-or-reject). A guard stands only on `resource_retained_by`. `X` names an integer parameter of the same declaration (`i8`–`i64`, `isize`, `u8`–`u64`, `usize`, enumeration projections included; not `bool`, a float, a pointer, `fnptr` or `va_list`), distinct from `P` and `H`. `OP` is one of QUANT-INDEX-CHECKS' comparisons (C09.2), `==`, `<`, `>`, `<=`, `>=`, evaluated over the mathematical integers; `!=`, `&&`, `||` and a second guard are not admitted. `K` is a canonical decimal literal (C04.5's spelling), never a name: the integer projection carries no enumeration names, so a declaration states the constant's value and cites its source in a comment. `K` lies in `X`'s range and the guard **partitions** the range: some value satisfies it and some does not (`x >= 0` on `u32` rejects). Every violation rejects the whole module.

**Meaning.** The clause holds exactly at the calls whose actual at `X` satisfies the guard: each guarded clause partitions `X`'s values into its arm (the clause holds) and the complement (it does not), so every value is covered. A consumer MUST apply it so:

* a call whose actual at `X` is **decided** — a decimal integer constant (C04.5.1's structural erasures: matched parentheses, an integer suffix; and a leading `-`), or a file-scope enumeration constant whose value Clang states for the analysed unit and which the calling function does not shadow — and lies in `X`'s range: the clause is in force when the guard holds and not in force when it fails;
* every other call (a variable, an expression, a cast, a value out of range that C converts, a shadowed or unread constant) is **undecided** and reads the **cautious** union: every guarded clause in force. A consumer MUST NOT read the favourable arm for an undecided actual.

**Body check.** Where the analysed unit defines the callee's body, a guarded clause stands in that unit only when the body **validates** it: every mention of `P` outside a guard region is a null or equality test or a dereference (never under `&`), where a guard region is the braced then-block of `if (X OP K)` with exactly the guard's comparison, or a `case` arm of `switch (X)` that no other label falls into and whose constant satisfies the guard; `X` is never written and the body has no `goto`. A guard the body does not validate is removed in that unit, which is the cautious reading at every call there. The check establishes only that the body cannot keep `P` when the guard fails; the retention itself stays the declared assumption C04.14 describes. In another unit the guard is a declared assumption like every clause, pinned (C06.2) after review.

**No corpus declaration carries a guard.** The decision's example, `rd_kafka_confval_set_type`, retains `valuep` on the holder's own stored type, not the argument (C04.1): it is declared with the cautious retention, and the in-unit check refuses a guard on that body shape. The Lean model is `QCore.RetentionGuard`.

#### C04.14.2. The result as holder

Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (gateway rule 13e), absorbing `RETAINED-BY-RESULT-HOLDER`, under the decision C04.14 records:

```qcore
extern fn make(owner: ptr<mut, u8>, name: ptr<const, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(owner);
    ensures normal resource_borrowed(name);
    ensures normal resource_retained_by(owner, result);
    ensures normal resource_retained_by(name, result);
    ensures normal resource_return_owned(nonnull);
    ensures normal resource_return_kind(reference);
};
```

**Grammar** (G05, total-or-reject). The holder of `resource_retained_by` may be the word `result`: the object the call returns. It requires the declaration's result to be `resource_return_owned(nonnull)` (a fresh heap object, or with `resource_return_kind(reference)` a returned reference, C04.17): only an object the caller owns is a holder whose end the caller can see. A parameter named `result` rejects a `result` holder; every other C04.14 rule applies (the retained parameter is a depth-one `resource_borrowed` pointer with no kind, retained at most once).

**Meaning.** After the call the returned object keeps a pointer into the object `P` designates; the caller keeps custody of `P`'s object and must show it outlives the returned object. The route-G consumer (rule 13e) decides it with rule 12t's table, the holder being the one live identity the call bound its plain destination to (any other destination is a holder the body cannot name: `retention-undecided`): a caller's object (an unrebound formal) or a tracked object is recorded as held by the result; a string literal or a predefined identifier (`__func__`, `__FUNCTION__`, `__PRETTY_FUNCTION__`: a static array, C11 6.4.2.2p1, premise P-RETAINED-PREDEFINED) outlives it. A record ends when the holder is released (a heap handle's destructor) or when the function drops the **last reference it holds on an acquired holder** (C04.17: every reference the function could see on an object it obtained itself; premise P-RESULT-HOLDER-LAST-DROP — a reference a callee took through a position declared `resource_borrowed` is that declaration's assumption, constraint 4). A drop of a borrowed family (a keep of a caller's object) ends nothing: the caller's reference keeps the object alive. A holder still holding at an exit — returned, stored, leaked — may outlive the frame: the exit refuses (`retention-undecided`), **never `Safe`**; a release of a tracked `P` while the result holds it is refused (`retention-released-while-held`), never accused. The Lean model is `QCore.ReferenceResult`.

### C04.15. Slot moves

Card SLOT-MOVE-AND-INTERIOR-BORROW (gateway rules 12x and 13a), on the user's decision recorded in C04.3.4: class 2, the custody counterpart of CG24's `bounds_frame_moves`.

```qcore
extern fn cat(slot: ptr<mut, ptr<mut, u8>>, text: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(slot);
    ensures normal resource_borrowed(text);
    ensures normal resource_slot_moved(slot, nullable);
    ensures normal resource_return_none();
};
```

`resource_slot_moved(s, n)` declares that the object `*s` designates may be moved (reallocated) by the call: **after the call the slot still owns exactly one object, possibly a new one; every alias of the old pointer is invalid; on failure the original stays owned by the slot.** `n` is the nullability of `*s` at entry: under `nullable` a NULL `*s` moves nothing. Total-or-reject (G05): `s` is a `ptr<mut, ptr<mut, u8>>` parameter, `resource_borrowed`, with no kind, named by one slot clause; the clause stands where a retention may (C04.14). The declaration is **withheld from the function map** with the retaining declarations, so a consumer that knows nothing of it sees an uncontracted callee: a moved slot is never read as borrowed.

**What the C gateway decides (rule 12x).** At a call, the slot actual is `&L`, a formal slot the calling body names only as whole actuals at slot-moving positions (the caller of this function owns what it designates: passed through, nothing moves here), or anything else (refused, `slot-move-undecided`). For `&L`, `L`'s binding decides: exactly one live identity this body tracks, held by no structure other than names (no field link, container, holder, collection, pending replacement or nullable marker) — the move: `L` keeps the identity and its one obligation, and **every other name bound to it is invalidated**: its binding is dropped and a later read of it is refused (`slot-move-alias`), never accused, since the object may not have moved; a definitely-NULL `L` under `nullable`: nothing; anything else: refused. Because the slot keeps the identity, the failure arm needs no rule of its own: the original is still owed by the slot, and the caller's single release discharges whichever object the slot holds.

**Composition with return-code dispositions.** The decision states that a slot move composes with rule 11r where the success/failure split is by return code (the success class moves, the failure class leaves the slot and its aliases as they were). No measured member has such a split (every declared slot mover returns `void`); a `resource_slot_moved` inside `resource_on_return` is therefore not admitted yet (an unsupported predicate, the refusing direction; `SLOT-MOVE-RETURN-CODE-SPLIT`).

**Not covered.** A field reallocated through a borrowed parameter (`flb_config_task_map_grow` reallocates `config->task_map`) is not a slot parameter: aliases of the old field value cannot be enumerated from the caller, so it is not declared (`SLOT-MOVE-FIELD-SLOT`). A body that forwards its slot to undeclared helpers (`format_metric`) gets no inferred slot-move summary (`SLOT-MOVE-SUMMARY-INFERENCE`).

**Checked against bodies (rule 13a).** In a unit that defines the declared function, the declaration stands only when every mention of the slot formal is a whole actual at a slot-moving position of a declared callee (the body moves exactly what its callees move); otherwise it is withdrawn in that unit. A body that stores through the slot itself (`cfl_sds_cat_safe`) is not read by the check, so there the declaration is withdrawn, and elsewhere it is a declared assumption (constraint 4). The Lean model is `QCore.SlotMove`.

### C04.16. Derived out slots

Card SLOT-MOVE-AND-INTERIOR-BORROW (gateway rules 12z and 13a), on the user's decision recorded in C04.3.4 (absorbing `INTERIOR-OUT-SLOT-MODE`).

```qcore
extern fn strtol(nptr: ptr<const, u8>, endptr: ptr<mut, ptr<mut, u8>>, base: i32) -> i64
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(nptr);
    ensures normal resource_borrowed(endptr);
    ensures normal resource_borrowed(base);
    ensures normal resource_out_derived(endptr, nptr, nullable);
    ensures normal resource_return_none();
};
```

`resource_out_derived(s, p, n)` declares that the call may write through the slot `s` a pointer derived from parameter `p`, owning nothing, under C04.3.1's rules, or leave the slot as it was; `n` is the nullability of `s` itself. Total-or-reject (G05): `s` is `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>`, `p` a distinct depth-one pointer, both `resource_borrowed` with no kind; `const_dropped` is recorded when `p` is `ptr<const, u8>` and `s` `ptr<mut, ptr<mut, u8>>`. The declaration is withheld from the function map with the retaining declarations; it may stand beside a retention (`cJSON_ParseWithLengthOpts` both retains `value` in `global` and writes into it through `return_parse_end`).

**What the C gateway decides (rule 12z).** `&L` with `L` holding no tracked identity: `L` becomes a pointer derived from `p`'s binding, possibly NULL. Because the slot may still hold its old (untracked) value, the binding is read at the `Borrowed` extent of C04.3.4 without a borrow record: it owns nothing (a release of it is refused), the release of `p`'s object ends it (a later use is refused rather than accused), and no mutation ends it. `&L` with `L` holding a tracked identity is refused (the write could lose it, `out-derived-undecided`); a NULL slot under `nullable` and a caller's slot passed through change nothing here; anything else is refused.

**Checked against bodies (rule 13a).** In a unit that defines the declared function, every mention of the slot formal is `*s = EXPR;` with `EXPR` reached from `p` (C04.3.4's reading), or a whole actual at a derived-out position; otherwise the declaration is withdrawn in that unit. The Lean model is `QCore.DerivedOutSlot`.

### C04.17. Reference counts

Card REFCOUNT-RESOURCE-KIND (gateway rules 13b and 13c), on the user's decision recorded here verbatim from the card's brief (it is also recorded in `W48-CALLEE-DECISION-TABLE`):

> **USER DECISION 2026-10-05.**
> **Class 3, reference counts → a new reference-count resource kind.**
> - Taking a reference ("keep") is +1 owned reference.
> - Dropping one ("destroy" or "drop") is −1.
> - Each function must balance the references it takes, or hand them off (return, store into a tracked owner, a declared transfer), exactly as with other owned resources.
> - A plain counter that is not a reference (e.g. a statistics counter incremented with `rd_atomic32_add`) needs its own declaration, and never inherits the reference reading.
> - **Rejected by the user:** treating keep/drop as custody-neutral. It is not fail-closed: a missing drop would read Safe. Never fall back to it.

```qcore
extern fn keep(p: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_kept(p);
    ensures normal resource_return_alias(p, nonnull);
};

extern fn drop(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_dropped(p);
    ensures normal resource_return_none();
};

extern fn lookup(key: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(key);
    ensures normal resource_return_owned(nullable);
    ensures normal resource_return_kind(reference);
};
```

**The resource kind.** `reference` is a resource kind (C04.6), pointer-carried: one reference a function holds on a reference-counted object. Its obligation is counted **per object**, not owned per pointer, because a keep returns its own argument: `q = keep(p)` leaves `p` and `q` denoting one object on which the caller holds one more reference, and `keep(p);` as a statement takes the reference all the same. A reference is therefore taken only by `resource_reference_kept` and dropped only by `resource_reference_dropped`; the one other position that spells the kind is a **returned reference**, `resource_return_owned(n)` with `resource_return_kind(reference)` (a constructor or lookup that hands its caller one reference: `rd_kafka_q_new0`, `rd_kafka_toppar_get2`), which is how a returned kept reference is owned by the caller. `resource_kind(p, reference)` on a parameter rejects the module, and a heap release of a reference is a kind mismatch (C04.6 rule 1).

**Placement** (total-or-reject, grammar G05). `p` names a depth-one pointer parameter (`ptr<mut, u8>` or `ptr<const, u8>`), `resource_borrowed`, with no kind; a declaration carries at most one reference clause; a keep's result is `resource_return_alias(p, nonnull)` on that parameter (the keep hands back its argument) or `resource_return_none()`, a drop's is `resource_return_none()`; `p` is never also retained, a slot or a derived out slot's base; the clause stands where a retention may (C04.14). A declaration carrying one is **withheld from the function map** with the retaining declarations: a consumer that knows nothing of references sees an uncontracted callee, so the borrowed reading of `p` — the custody-neutral reading the user rejected — is never what it applies.

**Semantics** (what a consumer that honours the clauses MUST apply; the C gateway's route G does, rule 13b):

1. **Which object.** The object a keep or drop applies to is the one its actual denotes: a plain name, or a pointer cast of one. A keep of a name bound to no tracked identity (an unrebound formal, an untracked local: an object some other reference keeps alive) starts a **family** — the references this function holds on that object — held once, bound to that name and to the keep's result; a keep of a name bound to a live family adds one. A returned reference starts an **acquired** family, held once and bound to the call's destination; its null edge holds nothing. Every other actual (a member read, a call, a tracked identity of another kind, an ambiguous, unknown, possibly-null or derived binding) is refused (`reference-undecided`), and a keep into a place that is not a plain name (`o->f = keep(p)`) is refused too.
2. **Balance.** Along a path a family's count is its keeps minus its drops. At every exit it must be zero, or exactly one reference must have been handed off: returned (`return p;`), or stored into a tracked owner (a field link to an owner the function itself acquired, escaping with it). A family that still owes a reference at an exit is a **leak candidate** on a path with no refusal, and a refusal (`reference-unbalanced`) on a path that has one, since the refused statement may have handed it on. A refused statement whose line reads the family's name withdraws it, as it withdraws any identity (the refused hand-on rule).
3. **The last drop.** A drop subtracts one; the drop that takes a family's count to zero is **possibly final** — `destroy` may free the object when the object's own count reaches zero, which the function cannot see — so the family is discharged and every name bound to it is invalidated: any later read, a keep or a drop included, is refused (`reference-dropped`), never accused, unless the function still holds another reference (a count above zero). A drop of a reference the function does not hold — an unbound formal (the caller's reference), a member read, a name the last drop of a kept family invalidated, a family already handed off or held by a structure — is refused (`reference-drop-unheld`, `reference-dropped`): consuming a caller's reference needs a declaration. A drop through a name the last drop of an **acquired** family invalidated is an **over-drop**: every reference the function could see on an object it obtained itself is already dropped, so the drop takes a reference nobody handed it. It is a violation candidate (published as a double release) on a path that refused nothing, and a refusal on any other path. Counts that two arriving paths disagree on are undecided, and every later operation on the family refuses.
4. **Verdicts.** Every candidate is provisional and never presented as an upstream defect; everything unproven is `Unknown`. No route here turns `Unknown` into `Safe` by approximation: a function that calls no keep is unchanged byte for byte.

**Returned references checked against bodies (rule 13d, card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS).** In a unit that defines the body of a declaration whose result is a returned reference, the declaration stands only when the body validates it structurally: the body has no `goto`, has at least one `return`, and every `return` hands back a null constant (only where the declaration is `nullable`), a call of a declared keep that returns its argument or of a declared returned reference (pointer casts erased), or a local (never a formal) every assignment of which is a null constant, such a call or a call of a declared heap allocation (a constructor's fresh object), whose address is never taken and which is never an actual at a declared release, consumption, store or drop. Otherwise the declaration is withdrawn in that unit: it moves to the retaining table marked withdrawn, every call there refuses (`reference-withdrawn`), and no inference reads the body in its place. The check establishes that every reference the body hands back is one it obtained and did not give away; that a fresh object's count starts at the one reference handed back, and that a declared non-null result is never NULL, stay declared assumptions pinned to the body (C06.2). A returned reference with no other clause stays in the function map; with a retention (C04.14.2) it is withheld like every retaining declaration.

**The transfer.** A callee that takes over its caller's reference (`ut_destroy_mock_queue` forwards it to `rd_kafka_q_destroy_owner`) is, from its caller's side, a drop: `resource_reference_dropped(p)` on a body rule 13c validates. The caller's reference leaves its family; on an acquired family a later drop is an over-drop candidate and a later read is refused. No separate transfer clause is needed by any measured member. The callee's own row does not yet read its parameter as the reference it took over (it refuses its own drop as `reference-drop-unheld`; `REFCOUNT-DROP-OWN-BODY-TRANSFER`).

**Inference.** A derived summary never publishes a reference result (C04.6 rule 7: inferred summaries stay heap), so a body that returns a kept reference (`rd_kafka_toppar_list_copy`) is `Safe` on its own row and its callers see an uncontracted callee (`REFCOUNT-INFERRED-REFERENCE-RESULT`).

**Plain counters.** The primitive a reference count is built on (`rd_atomic32_add`, `rd_atomic32_sub`, `rd_atomic32_set`; `rd.h:298,351` make `rd_refcnt_t` an `rd_atomic32_t` and `rd_refcnt_add0(R)` `rd_atomic32_add(R, 1)`) is never declared: a reference operation is declared on a named wrapper whose body names the object (`rd_kafka_toppar_keep0`, `rd_kafka_q_destroy0`, …), so after preprocessing a statistics counter on the same primitive is never read as a reference, and a reference operation spelled through the primitive is not read at all (it stays an uncontracted call). A plain counter's own declaration — custody-neutral only where Clang states the operand is not a reference count (the `rd_refcnt_t` field type) — is not written yet (`REFCOUNT-PLAIN-COUNTER-DECLARATION`).

**Interactions.** A slot move (C04.15) of a name bound to a family is refused (`slot-move-undecided`: a family is held by its count, not by a name). A borrowed result (C04.3.4) and a derived out slot (C04.16) own nothing and are never a family: a keep of one is refused. A family that a holder retains (C04.14), that is linked into an owner's field or stored or published into a container counts as held elsewhere: a drop of it is refused (as a release while held is), and at an exit only an escape (alone, or with the owner it is linked into), a stored containment or a publication hands a reference off, so a reference left in an owner that is freed or kept is still owed. No reference clause stands beside a return-code disposition (C04.13), so a family is never decided per class. Counted users (C04.7) remain a separate clause family on a formal's counter member; a reference family is per object on any name, with hand-offs.

**Checked against bodies (rule 13c).** In a unit that defines a keep or drop declaration's body (a header `static inline` body is defined in every unit that includes it), the declaration stands only when the body validates it structurally: `p` is never assigned and never has its address taken; it is mentioned only through member accesses, as a whole actual of a call, or in a null test; nothing stores or returns it except a keep's `return p;` (an alias keep returns exactly `p`, every other body returns nothing); the body has no `goto`; and it touches the object (a member access, or a whole actual at the same reference position of a declared callee: `rd_kafka_q_destroy_owner` forwards to `rd_kafka_q_destroy0`). Otherwise the declaration is marked withdrawn in that unit: every call there refuses (`reference-withdrawn`), and — unlike rule 13a's withdrawal — it is not dropped, so no inference reads the body in its place. The check establishes that the body hands back exactly its argument and keeps or stores nothing else of it; that the count arithmetic is the one declared stays a declared assumption (constraint 4), pinned to the body (C06.2). The Lean model is `QCore.ReferenceCount`.

### C04.18. Named premises

Card VERDICT-CONTRACT-DEPENDENCIES (user direction 2026-10-09, "handle as safe as possible"). A declaration MAY name an assumption it rests on that its clauses do not state, beside the selector:

```qcore
extern fn rd_kafka_q_purge0(rkq: ptr<mut, u8>, do_lock: i32) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(served_queue, "an op callback the call may run (rd_kafka_op_handle, rd_kafka_op_call, rd_kafka_op_destroy) takes over no reference the caller holds on the served queue");
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(do_lock);
    ensures normal resource_return_none();
};
```

**Loading.** Total-or-reject (C04). `NAME` is a lowercase identifier (`[a-z][a-z0-9_]*`) and `STATEMENT` one plain string literal, not blank, with no escape and no control character; a second premise of the same name on one declaration, any other argument shape, and the clause under `ensures` each reject the **whole module**. A declaration MAY carry several premises of distinct names. The grammar is [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1).

**Meaning.** The premise changes no mode and applies or withdraws nothing: the function map is the map the declaration has without it, and no consumer that decides a verdict reads it. It is rederived from the retained texts (`declaration_premises`, `declared_premises`), as the literal-argument premise (C04.5) and the checked-specification marker (C06.3) are, and published with every row that rests on the declaration (C06.5). Because the clause is part of the declaration's text, the declaration's contract digest — and so its pin (C06.2) — covers it: adding, removing or rewording a premise takes a pinned declaration out of force until it is reviewed and pinned again. Each literal-argument premise (C04.5) is reported as the premise `literal_argument` (`p = LITERAL`), and the premises a rule states for a clause are reported with it (rule 13e's `P-RESULT-HOLDER-LAST-DROP` and `P-RETAINED-PREDEFINED` for `resource_retained_by(p, result)`).

**Inheritance (C15).** Elaboration copies a parent's clauses, premises included, and the agreement check compares them, so a child never sheds its parent's premise.

### C04.19. Callback contracts

Card CG22-CALLBACK-REGISTRY-CONTRACT (gateway rules 13u–13z), on the user's decision of 2026-10-05 recorded in C04.14: class 1 shape (c), callbacks over retained elements, goes through callback contracts. The callback relation itself is the core's `callback_trace` (rule 11p, `QCore.IndirectCalls`): a registration needs a live context and no earlier teardown, an invocation the registration in force and the context live, the context's release a prior unregistration.

```qcore
extern fn each(reader: ptr<mut, u8>, cb: fnptr, context: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(context);
    ensures normal resource_callback_invoked(cb, reader, _, context);
    ensures normal resource_return_none();
};

extern fn list_new(size: i32, free_cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_borrowed(free_cb);
    ensures normal resource_callback_registered(free_cb, result);
    ensures normal resource_return_owned(nonnull);
};

extern fn list_add(rl: ptr<mut, u8>, elem: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_borrowed(elem);
    ensures normal resource_element_added(elem, rl);
    ensures normal resource_return_none();
};

extern fn list_destroy(rl: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_callback_registry_invoked(rl, element);
    ensures normal resource_elements_dropped(rl);
    ensures normal resource_callback_unregistered(rl);
    ensures normal resource_released_if_constructed(rl);
    ensures normal resource_return_none();
};
```

**The clauses.** An actual list `A…` names, position by position, what the callback receives: a parameter of the declaration (the caller's actual there, unchanged), `_` (a value the callee supplies: an index, a pointer it computes into an object it was handed, a buffer of its own) or, in a registry invocation only, `element` (each element the holder holds).

| Clause | Meaning |
|---|---|
| `resource_callback_invoked(F, A…)` | **Synchronous invocation.** During the call only, the function pointer `F` is called zero or more times with `(A…)`; nothing keeps it. |
| `resource_callback_table(T, A…)` | **Table dispatch.** During the call only, function pointers the callee reads from the array `T` designates are called zero or more times with `(A…)`. |
| `resource_callback_registered(F, H)` | **Registration.** After the call the holder `H` keeps `F` (a null `F`: no callback), replacing any earlier registration. `H` is a parameter (in place) or `result` (a constructor). |
| `resource_callback_registry_invoked(H, A…)` | During the call, the callback `H` keeps (if any) is called zero or more times with `(A…)`. |
| `resource_callback_unregistered(H)` | After the call `H` keeps no registration. |
| `resource_element_added(E, H)` | **Element custody.** `E`'s object becomes an element `H` holds: its custody passes to `H` when `H`'s registered callback releases its `element` argument, otherwise `H` only retains `E` (C04.14). |
| `resource_elements_dropped(H)` | After the call `H` holds no element. |
| `resource_released_if_constructed(H)` | **Flag-conditioned self-release.** The call releases `H`'s object exactly when the registration in force was made by a constructor (`result` holder), and nothing when it was made in place (`rd_list_new` sets `RD_LIST_F_ALLOCATED`, `rd_list_init`'s `memset` clears it). |

**Placement** (total-or-reject, grammar G05). `F` names a `fnptr` parameter; one callback is named by at most one invocation or registration. `T`, `H` and `E` name depth-one pointer parameters, `resource_borrowed`, with no kind. A parameter actual is a parameter other than the invoked callback; a declaration with a parameter named `_` or `element` rejects every callback clause; `result` requires `resource_return_owned(nonnull)` and rejects beside a parameter named `result`. An element is not its own holder and is never also retained; a declaration carries at most one clause of each keyword per position and never both a registration in a parameter holder and a registry clause on it; the clauses stand where a retention may (C04.14), beside retentions (`msgpack_packer_init` both retains `data` and registers `callback` in `pk`), never beside a slot or reference clause. A declaration carrying one is **withheld from the function map** with the retaining declarations: a consumer that knows nothing of callbacks sees an uncontracted callee, so **no callback is ever read as a benign borrowed pointer**.

**What a consumer MUST apply** (the C gateway's route G does, rules 13u–13x):

1. **Candidates.** A callback actual is decided only as a closed candidate set: rule 11p's resolution of the actual over Clang's AST (a function designator, a resolved local, a constant table; every Clang call of the callee on the statement's line is read, and the answer stands only when they agree, as glibc's `__glibc_const_generic` puts one `bsearch` call per `_Generic` association there), or, for a table, every designator of an automatic array's braced initializer when the body names the array only as whole table actuals of declared table dispatchers (string and null entries are never called: premise `P-CG22-DEFINED-POINTER-VALUE`). A null constant registers no callback; anywhere else, and for every other actual (a formal, a member read through a pointer, a cast, a call's result), the set is **open** and the call is refused (`callback-undecided`).
2. **Loop-safe composition.** Every candidate needs a contract in force (a declaration in the function map or a validated in-unit summary; otherwise `callback-uncontracted`), and that contract must be the **identity** on the caller's state: exactly as many parameters as actuals, each `resource_borrowed` or `resource_owned` with no kind, `resource_return_none()`, no container or counted-user clause (otherwise `callback-effectful`). `resource_owned` is a requirement (a live non-null object), not an effect: at a `_` or `element` position the callee meets it with what it supplies; at a parameter position the call itself is read with `resource_owned(n)` there (`n` nullable only when every candidate's is), exactly the requirement a direct call of the candidate with that actual carries. Then any number of invocations, of any candidates, is the identity, and the call reads as its own custody contract so strengthened. The one exception is an `element` position of a registry invocation, which may be `resource_released`: the registry's free callback. A callback releasing or keeping anything else is refused, including an object the callee supplies (`rd_kafka_q_serve*`'s ops). **Never `Safe` by assuming a callback benign.**
3. **Registries.** A registration is recorded on the path for a holder the body names: `&L` for `L` one automatic, non-pointer, non-array object named only as whole `&L` actuals of holder-naming callees, or one tracked live identity (a constructor's result included). A registry clause needs the registration in force on every path reaching the call (registrations join by agreement; a disagreeing join is undecided); otherwise, and for a holder the body cannot name (a formal, a member), the call is refused (`callback-registry-undecided`). An invocation over a registration whose candidates are all the identity changes nothing; over one whose candidates all release the element, it releases every element whose custody the registry took; any other registration is refused.
4. **Element custody.** An element added to a releasing registry held by a tracked identity is contained in it (C04's containment: the holder's escape carries it, and the registry's invocation releases it); added to a releasing frame registry it is refused (`callback-element-custody-undecided`: a frame object has no identity to own it); added to a registry with no callback or an identity callback it is retained by the holder under C04.14's decision table. A plain release of a holder whose registry still owns elements is refused, never read as releasing them; elements dropped without their release are refused.
5. **Self-release.** `resource_released_if_constructed(H)` releases the holder (an ordinary release, after the registry's own effects) when the registration in force before the call was a constructor's, and does nothing for an in-place one.

**Checked against bodies (rule 13y).** In a unit that defines the declared function, a synchronous invocation stands only when the body names `F` only as the callee of direct calls (`F(…)` or `(*F)(…)`) whose actuals match `A…` position by position (a parameter word: that formal, never rebound or address-taken; `_`: anything not naming `F`), and a registration in a parameter holder only when the body names `F` only as the value of stores `H->M = F;`, at least once, `F` and `H` never rebound. A body that does not validate withdraws the declaration in that unit (every call there refuses, `callback-withdrawn`; no inference reads the body in its place). The check establishes what the callee does with the pointer; the callback's own contract is the candidate's. Every other callback clause (a table, a registry use, element custody, a constructor's registration) is a declared assumption (constraint 4), pinned (C06.2), and no body evidence covers it (rule 13i).

**Summaries** (rule 13u). A designator a body hands a synchronous invocation or table dispatch is a call-graph edge for W4's bottom-up order, so the callback is summarised before its caller.

**Not in this vocabulary** (recorded in the card's report with sizes): an invocation through a member of an object the caller holds whose registration happened elsewhere (`rd_kafka_op_destroy`'s `rko_op_cb`, `pb_write` over a stream `pb_ostream_from_buffer` returned by value), a type-switched union release (`rd_kafka_op_destroy`), a callback that takes custody of an object the callee supplies (`rd_kafka_q_serve*`), a result returning what a holder retains (`pthread_getspecific`). The Lean model is `QCore.CallbackContracts`.

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

### C06.5. Per-verdict contract dependencies

Card VERDICT-CONTRACT-DEPENDENCIES (gateway rules 13f–13h). Every published legacy-C row carries `contract_dependencies`: the declarations its derivation consulted, split by their standing at the consultation (`declarations` in force, `held`, `withdrawn`), and the in-unit `summaries` whose own dependencies were added. The set is taken from the route's own consultations — every lookup of the row's own work whose name the row's body spells, every callee its solved CFG applies a contract of, its own name (its entry contract), and transitively those of every in-unit summary it consulted. It may over-approximate; it never omits a declaration a consulted call relied on.

The audit's contract sidecar carries a `contract_dependencies` section: per declaration its contract digest, mode and status (C06.2), its premises (C04.18), the rows and `Safe` rows that consulted it, and its **body standing** for the run. A declaration is `validated` only where every unit that examined one of its bodies validated it — an equal W4 summary, the body analysed under the declaration's entry custody matching every exit and `Safe` with no diagnostic, or a structural check of gateway rules 11r, 12w, 13a, 13c and 13d — and at least one did; one held for a human act (C06.1, C06.2) is `held`; every other is a `declared-assumption`: there is no analysed body, or one did not validate it, and the declaration stands as reviewed (this section's opening paragraph). A declared assumption is never labelled `validated`. The section ends with the reviewer's roll-up of the `Safe` rows (resting on no declaration, only on validated declarations, on a declared assumption) and the twenty declared assumptions most `Safe` rows rest on. Both the row field and the section are evidence: no verdict reads them, and removing them restores the report byte for byte. The Lean model is `QCore.ContractDependencies`.

**Named null premises** (card NULL-SAFETY-PROPERTY, gateway rule 14n, C09.3). A row whose derivation read a dereference through a target-free, possibly-null borrowed alias non-null (decision 1) also lists the premise `null-deref:FUNCTION:LINE:VARIABLE` in `contract_dependencies.null_premises` (its own and, through the consultation log, every in-unit summary's), and the roll-up counts the `Safe` rows resting only on a null premise and those carrying one; such a row is never counted as resting on nothing.

**What standing means: per clause** (card VERDICT-BODY-STANDING-CLAUSE-SCOPE, gateway rules 13i–13k). A piece of body evidence validates the clauses it compares or reads, never the whole declaration by itself. A declaration's clauses are each parameter mode with its kind, the result mode with its kind, each container (C04.8) and counted-user (C04.7) clause, each `resource_retained_by` (C04.14) and the guard of a guarded one, each holder reset, slot move (C04.15), derived out slot (C04.16) and reference keep or drop (C04.17), each exit class of a return-code disposition (C04.13), and the clauses that live only in the retained text: each named premise (C04.18), a variadic tail, and `resource_no_return()`. A literal-argument premise (C04.5) and the loop-owner, thread-synchronization and lifecycle obligations (C04.9–C04.11) bound where the declaration is believed — they are its callers' obligations — and state nothing a body must establish, so they are not clauses here. An equal W4 summary and the body analysed under the declaration's entry custody cover exactly the custody projection: every parameter mode and kind and the result mode and kind (a W4 summary equal to the custody projection of a declaration with container or counted-user clauses is recorded and covers those clauses only). Rule 11r covers every exit class; rule 12w the guards; rule 13a slot moves, derived out slots and the borrowed result of a declaration in the function map; rule 13c reference keeps and drops; rule 13d a returned reference. Nothing covers a container or counted-user clause, a retention, a holder reset, a named premise, a variadic tail or the no-return clause. A clause is `validated` for the run where the declaration's standing above would be `validated` and every unit that validated the declaration covered the clause there. The declaration's `body_standing` is then `validated` only when **every** clause is; `partly-validated` (its `uncovered_clauses` listed) when some but not all are; `held` as above; and a `declared-assumption` otherwise. An uncovered clause is a declared assumption, whatever evidence covered its neighbours: a structural check never validates the `resource_borrowed` formals beside the clause it reads. The roll-up counts a `Safe` row "only on validated declarations" only when every declaration it rests on is fully validated, and counts apart the rows resting on a partly-validated declaration and no declared assumption; every row so counted was so counted under the previous rule. The labels are evidence like the rest of the section: no verdict reads them. The Lean model is `QCore.BodyClauseStanding`.

**A refinement settled by the declaration's own entry check** (card DECLARED-ASSUMPTION-BODY-CHECKS, gateway rules 13m–13n). A unit's W4 summary that differs from the declaration of the same body is a body failure, with one exception. Where the difference contradicts no custody duty (the mode gate withdraws nothing: a nullability refinement, `LOADER-ABI-MODES-REFINEMENT`), the declaration is the function map's, is its own custody projection (no container or counted-user clause), is in force in the derivation as declared, and is neither refuted (C06.4) nor a withdrawn checked specification (C06.3), the body is analysed under the **declaration's** entry custody, on the CFG the reported run builds, with every refusal a reported row of it would carry. When every exit matches the declaration and the result is `Safe` with no diagnostic, the unit records evidence `13m` instead of the failure; `13m` covers exactly what `W4` and `entry` cover (every parameter and result clause). Otherwise the failure stands. This is the check W4 publishes every summary under, applied to the declaration directly: W4 seeds a bound formal non-null and never weakens it, so a nullable wrapper (`free(ptr)` under `resource_released(ptr, nullable)`) can be validated only this way. A declaration validated through it rests on its callees' declarations exactly as every W4-validated body does. The check is evidence only: it runs only while evidence is collected, writes no parity row and notes no consultation, no verdict reads it, and the defining row keeps its summary-conflict diagnostic. The Lean model is `QCore.DeclaredEntryCheck`.

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

**Disposition** (card W47-REVIEW-HOLD-CONCRETE-REASON, user decision 2026-09-29). A marker MAY end with the word `rewrite_before_acceptance`, after the reason and the owner:

```qcore
    ensures normal resource_review_pending(resource_borrowed(data), "retained, not borrowed: …", rewrite_before_acceptance);
```

It states that the text as written is not what the body does, so unwrapping it is not a resolution: the text must be rewritten before it can be accepted. Its category is `must-be-rewritten-before-acceptance`; a marker without it keeps the meaning above, whose resolution is a review and then unwrapping the text as written, and its category is `acceptable-as-written-after-review`. The category is read from this structure, never from the reason's prose. It changes no verdict: a function whose only open reasons are held calls stays `Pending` whatever their categories (§L21.2), and the second analysis of rule 10i still puts the held declaration in force as written. A misplaced, repeated or other word, or the word standing in for the reason, rejects the whole module (C04).

**Loading.** Total-or-reject (C04) applies in full. A missing or blank reason, a reason or owner that is not one plain string, a third string, a second contract-level marker, a contract-level marker beside a clause-level one, a marker wrapping a marker, a clause-level marker around the `resource_contract_v1` selector or a literal-argument premise (C04.5; those are deferred only by deferring the whole contract), a contract-level form under `ensures` and a clause-level form under `requires` each reject the **whole module**. A wrapped clause is read exactly as if written bare, so every C04 rule still applies to it. A marked declaration is therefore loaded, validated, digested and packaged like any other; its source text is retained (C05) and `check_source_assumptions` recomputes its status. The loader then **withholds** it: the environment records it under `pending_review`, with its module, the contract it states and every marker, and it is **never** an entry of the function map. A consumer that does not recognise the status therefore sees an uncontracted callee, which is the refusing direction.

**Meaning.** A pending declaration discharges nothing and accuses nothing. Every call site or obligation that would apply it MUST be held, never `Safe` and never `Unsafe`, under the reason code `awaiting-human-review`; since card W45-USER-CONTRACT-PLUGINS a verdict resting only on held calls is `Pending` (language specification §L21.2), and one with any other unresolved construct stays `Unknown`; since card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL that includes every reason the function would carry with the pending declaration in force as written, which the held call's refusal would otherwise mask (§L21.1; subject: the analyser, so it never supports a provisional `Unsafe`). A clause-level marker holds exactly the call sites a contract-level one holds: a v1 contract is applied as one conjunction, no consumer applies part of one, and dropping only the deferred clause would let the remaining, possibly weaker, clauses decide the call. The clause the marker names is what a report attributes, not a narrower refusal. A function *defined* in the analysed unit under a pending name is analysed as an undeclared definition: the pending contract is neither its checked interface nor its accuser, and its callers stay held whatever its body or a W4 summary of it proves.

**Consumers.** The legacy-C gateway rewrites every call site of a pending callee to a marked undeclared name before any reader sees it (the LOADER-ABI technique), so W4 derivation, W4 validation and certificate rechecking reproduce the hold; the statement refuses with ``call to `NAME` whose declared contract awaits human review (awaiting-human-review)``; a projection summary names its held calls (`…, N call(s) held awaiting human review`) only when there is one, and classifies as `awaiting-human-review` ahead of every other category. The checked extern-call route (`source.extern-call.v1`) finds no admitted contract and refuses with `Q023` naming the review. Provider packages carry the text unchanged (`providers/package.py` pins bytes, not clauses), and every provider selection loads through this loader. The observation and lifecycle interpreters do not recognise the marker and reject it as they reject every other resource-v1 clause.

**Inheritance and inclusion (C15).** Elaboration copies a parent's clauses, markers included, renaming parameters by position; a string is not a word, so a reason and an owner are never renamed. A child that inherits a deferred contract is therefore deferred, and one that inherits a deferred clause is deferred for that clause. The inherited-declaration agreement check compares the markers too (scope, reason, owner and disposition), so a child can never shed its parent's deferral or its disposition.

**Reporting.** An audit report lists every marker in a `pending_human_review` section — declaration, contract or clause scope and clause text, reason, owner, module and the SHA-256 of its retained text, and the analysed functions and recorded held-call refusals it accounts for — with explicit zeros when nothing is pending, and prints a one-line tally. A unit's `reason_code` carries `awaiting-human-review` where unit reasons are recorded. Since card W47-REVIEW-HOLD-CONCRETE-REASON each entry also carries its `category`, the section counts the `rewrite_markers` and the tally ends with that count; and every published function row that records a held-call refusal carries a `review_holds` member listing, for every held declaration in name order, each of its markers in clause order — declaration, module, scope and clause, the reason verbatim, owner, category and reason code — so a `Pending` row names the human act it awaits. A held callee the environment does not withhold, or withholds with no marker, fails the run rather than publish a hold with no reason. The member is additive: a row without a held call is byte-identical, and no status or count moves. The Lean model is `QCore.ContractDeferredReview`, and `QCore.ReviewHoldStanding.ConcreteReason` for the row's reasons.

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

**Pinned.** A pinned contract is **in force only where its contract text is the one it was pinned with and the audited source matches one of the identities it was reviewed against**. Those identities live in a generated **lock**, `contracts.lock`, never in the `.qv` (or legacy `.qcore`) text: one per provider crate, beside its `Cargo.toml`; a module outside every crate is covered by the nearest `contracts.lock` in its directory or above it, up to the first directory holding a `Cargo.toml` or a Git directory. A lock (format version 2) records, per pinned declaration, the SHA-256 of its normalised declaration text (its tokens joined by one space) and one or more **reviewed identities**. An identity is the function's **sites** — every top-level declaration and definition, each with its file, the SHA-256 of its normalised declaration text and, for a definition, of its body — together with every source revision it was reviewed at; revisions whose sites are equal share one identity, no revision appears in two identities, and there is no wildcard; the one identity that names no revision is a **tip identity** (below). Comments and layout do not count; every other change does, a renamed local and a reordered statement included. For a Git checkout the sites are read from the blobs of `HEAD`, never the working tree. A contract text that differs from the pinned one voids every identity. Format version 1 (one source per lock) is still read, each entry as one identity. Before any unit is analysed, every audit route of `quirin-audit` (`current`, `contracts`, the configured route) and the corpus arbiter compare each pinned declaration in force with its lock over the audited source. A mismatch — no lock (`no-lock`), no entry or no identity (`not-in-lock`), a changed contract text (`contract-changed`) or sites matching no identity (`source-changed`) — is **not in force**: the declaration leaves the function map and is recorded under `ResourceContracts::pin_mismatch` with its reason, and every call relying on it is held under the reason code `contract-pin-mismatch` (§L21.1; subject: the analyser, so it never supports a provisional `Unsafe`), never `Safe` or `Unsafe`. A person resolves a mismatch by reviewing the changed source and re-pinning, so since card W45-USER-CONTRACT-PLUGINS a verdict resting only on such held calls is `Pending` (language specification §L21.2); it was `Unknown`. Since card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL it stays `Pending` only when the function carries no other reason with the withheld declaration re-pinned as written (a body the change refutes, an argument the declaration does not admit): the held call no longer masks them. The legacy-C gateway holds those call sites exactly as it holds a pending review's (C06.1): each is rewritten to a marked undeclared name, the statement refuses with ``call to `NAME` whose pinned contract is not in force: the audited source does not match its lock (contract-pin-mismatch)``, and a unit summary names the held calls after any held for review. A function defined in the unit under a withheld name is analysed as an undeclared definition. `check_source_assumptions` accepts a withheld declaration only when it is pinned and equal to its fresh load, so a pin evaluation can move a declaration out of force and do nothing else. A lock that does not parse, names another version or normalisation, lists unsorted or duplicate entries, or has an identity with no revision, a revision in two identities or two identities with equal sites is malformed, and the audit refuses.

**Tip identities (card CONTRACT-PIN-TIP-MODE, user decision 2026-10-02).** A pinned declaration MAY instead track the **tip** of a branch: the contract is written for a fix that exists only on a branch (a release candidate), and the person who reviewed it asked that it follow that branch rather than one revision. The mode lives in the lock alone; the `.qv` text is unchanged and the declaration stays pinned. A lock entry holds either reviewed identities or exactly one tip identity, never both, so an earlier reviewed revision can never keep a tip contract in force after the tip moves. A lock holding a tip entry is **format version 3**: a top-level `tip` list, sorted and unique, beside `pinned` and `flexible` (a function is in at most one of the three); a lock without one is written as version 2, so every other lock is unchanged, and versions 1 and 2 are still read. A version-2 lock with a tip entry, or a version-3 lock without one, is malformed. A tip entry records the function, its module and contract digest; the **repository** — a `name` (letters, digits, `.`, `_`, `-`), an informational `url` that is never fetched, and the absolute local `path` where `pin` found it; the **ref**, a full refname (`refs/heads/BRANCH`); an optional free-text `release_label`, shown to people and read by nothing; and the **reviewed** record, the commit the person reviewed and its sites (never empty).

*Resolution.* Before any unit is analysed, the audit locates the repository — the environment variable `QUIRIN_TIP_REPOSITORIES` (entries `NAME=PATH` in the platform's path-list form; a malformed value refuses the audit) first, then the recorded `path` — and, with read-only Git commands only (`rev-parse`, `merge-base --is-ancestor`, `ls-tree`, `cat-file`), resolves the ref's current commit, requires the reviewed commit to be that commit or an ancestor of it, and reads the function's sites from that commit's blobs, never a working tree. It writes nothing to the repository and never fetches. The tip entry is **in force exactly when** its contract text is the pinned one, the tip resolved, and the resolved sites equal the audited source's sites (paths included; a copy vendored under another directory does not match). A repository not located, a missing ref, a tip that does not descend from the reviewed commit (a rewritten branch, or another repository at the path), an unreadable blob, or a tip with no site of the function is **not in force** under the reason `tip-unresolved`, the failure named; a resolved tip whose sites differ is not in force under `source-changed`, the detail naming the ref and commit. Nothing is in force by default. A tip entry not in force is held exactly as any pin mismatch is (`contract-pin-mismatch`, `Pending` under the held-call rule above, never `Safe` or `Unsafe`); its body, wherever analysed, is checked as C06.3 and C06.4 say for any declaration in force, and a review marker (C06.1) withholds it first.

*Reviewed versus tracked.* When the tip moves, a tip entry tracks a revision no person reviewed. Each audit's `contract_revision` section therefore records, for every tip entry, the repository, how it was located, the ref, the resolved commit and the tip's sites, the reviewed commit, and whether the tip moved and whether its sites changed since review; every tip entry in force is listed as a **named assumption** of the run (`assumptions.tip_tracked`, named `tip-tracked:FUNCTION`), as a flexible trusted contract is, and its inventory status is `tip-in-force`, never `pinned-in-force`. A verdict resting on it therefore never claims a reviewed pin.

*The notice.* Wherever a tip entry is loaded, pinned, checked or reported — `pin`, `pin --check`, `pin --verify-contracts`, each audit route's console tally, the `contract_revision` section (`tip_notices`, and each tip record's `notice`) and `compare` — the tool shows a notice that names the function, the repository, the ref, the release label when there is one, and the resolved commit, and states that the contract **targets a future, unreleased release (a release candidate), not a released version**. Its severity is `info` when the tip entry is in force for the audited source and `warning` when it is not, the notice then saying why (the sites differ from the tip's, or the tip cannot be resolved and how). Without an audited source (`--verify-contracts`) it is `info` when the tip resolves on the host and `warning` when it does not. The notice is presentation and evidence only: it is computed after every verdict, no verdict, status or count reads it, and a run with no tip entry is byte-identical to one before the card. The Lean model is `QCore.ContractTipMode`.

**`quirin-audit pin`.** A lock is written only by `quirin-audit pin`, after human review, and never during an audit. `pin --source-root DIR` **adds or refreshes** the identity of that checkout's `HEAD` for every declaration of every crate on the load path (so new contracts are pinned with one command) and never drops an identity recorded at another revision; a declaration whose contract text changed starts again from this revision alone. `--function NAME` restricts it to the named entries; `--regenerate` rebuilds each lock from this revision alone; `--drop-revision REV` (no source read) removes the revision REV — spelled in full, as a prefix of at least seven characters, or `directory` — from every identity, and an identity left with no revision goes. `--check` never writes: for the checkout's revision it reports each entry as recorded there, in force through an identity reviewed at another revision, not in force, or stale, and exits 1 unless every entry is recorded there; `--check --revision REV` reads REV's blobs from the checkout's object store instead of `HEAD`'s. `--modules DIR` pins a directory of modules outside the providers; `--verify-contracts` (no source) checks that each lock lists exactly the crate's current declarations, modes and contract digests, and `providers/contracts.py check` runs it. For tip identities: `pin --source-root DIR --function NAME --track-tip REF` (DIR a Git checkout holding REF; optional `--repository-name`, `--repository-url`, `--release-label`) records a tip entry reviewed at REF's current commit, replacing a reviewed entry; `--untrack-tip` (with `--function`) turns a tip entry back into a reviewed identity of the checkout's `HEAD`; every other run, `--regenerate` and `--drop-revision` keep tip entries as written, a changed contract text being reported and leaving the entry out of force until it is tracked again. `--check` reports a tip entry in force through the tip (allowed by its exit status) or not in force (exit 1), and `--verify-contracts` counts tip entries as pinned declarations; each prints the notice.

**Flexible.** A flexible contract binds by name and declared signature only (the declared/C ABI agreement of card LOADER-ABI still applies) and is in force at every revision. On a function whose body the run analyses, the body is checked against it as usual; the mode never skips that check. On a trusted function (no body in the audit) it never applies silently: it is listed as an explicit assumption of the run. Every flexible contract of the libc provider (`quirin-provider-libc`, all of whose contracts are flexible) that the run does not check against a body forms **one standing, named assumption**, `libc-c-standard-abi`: the C library behaves as the C standard and platform ABI specify. The C library's version is the build's provenance, not part of the assumption.

**Not inherited (C15).** A revision mode records how a text was reviewed against its own function's source; an inheriting declaration names a different function. Elaboration therefore drops the parent's contract-level marker and unwraps its clause-level markers, and a child is pinned unless its own extension clauses say otherwise.

**Reporting and change reports.** Each audit's contract sidecar carries a `contract_revision` section — the source identity, the locks read (path, SHA-256, revision), counts, every pinned contract in force, every mismatch with its reason, pinned and found sites and the held functions and call sites, the flexible contracts checked against a body, the flexible trusted assumptions, the standing libc assumption, and an inventory of every declaration's mode and status — and a one-line tally. `quirin-audit compare --before REPORT --after REPORT` reports the change between two runs (two revisions, two contract sets, or both): selected and missing sources, contracts in force added, removed and changed, pin mismatches new and resolved, flexible and standing assumptions, per-function obligations added and removed, tip entries (each run's standing, ref, resolved commit and notice, and whether the tip moved between the runs), and verdict moves in which **violation detected** (a move to `Unsafe`) is kept separate from **coverage lost** (a move from `Safe` to `Pending` or `Unknown`, with the reason codes of its refusals); a move between `Unknown` and `Pending` is classed on its own, and each run's verdict tallies, `Pending` included, are reported.

**Boundaries.** The pin is enforced where an audit evaluates it: the four routes above. A caller of the gateway API, the checked extern-call route and the other compiler examples apply the environment they are given, in which no pin was evaluated. The scanner is lexical: it never preprocesses, so a site covers every configuration's text, and it balances braces per conditional group. The Lean model is `QCore.ContractRevisionMode`, and `QCore.ContractTipMode` for tip identities.

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

### C09.1. Reviewed index and extent contracts (planned)

**Design decision, 2026-10-01; implementation pending under
[CG16-INDEX-AND-OFFSET-EXTENTS](plans/clang-gap-cards/cg16-reviewed-index-contracts.md).**
This section specifies a separate index/access-bounds property for existing C.
It does not add accepted syntax to resource-contract v1, extend `index.closed`,
or claim that a current custody certificate proves bounds. The requirements
below govern the new consumer when implemented; current consumers retain C02's
total-or-reject and property-scoping rules.

**Implementation status (card CG16-INDEX-AND-OFFSET-EXTENTS, 2026-10-01;
gateway rule [11s](agents/gateway.md#constraint-11s)).** Landed: the bounds
contract v1 consumer ([G05b](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05b-bounds-contract-v1),
`bounds_contract_v1` with `bounds_extent`, `bounds_index`,
`bounds_global_extent`, `bounds_return_extent`, `bounds_preserves_extents` and
its own `bounds_review_pending` marker, every declaration's text digested); the
route-G source consumer (`qcore_c_gateway::index_extent`) for the property
`c.bounds.index-extent.v1`, which derives fixed-array extents from Clang's
types, inventories every subscript, dereference, `->`, pointer formation,
pointer subtraction and ordering, and pointer-passing call, checks each
against `qcore_core::index_extent` (Lean `QCore.IndexExtent`), applies the
review-hold standing rule, validates reachability before `Unsafe`, withdraws a
return extent its analysed body refutes from the unit's callers, and re-derives
recorded evidence before admitting it; and the audit's explicit selection
(`QCORE_BOUNDS_PROPERTY`), which publishes bounds and combined rows beside an
unchanged custody report. Not landed at CG16, each a named residual of its
ledger entry (pins, placement and packaging, and the audit selection landed
since, below): C06.2 pins for bounds declarations (they bind by name and
signature; no pin hold is produced), inclusion, inheritance, packaging and
checkpoint transport of bounds modules, a bounds-module selection on the audit
routes (they run with an empty bounds environment), the cross-unit refutation
phase on the audit routes (the API takes the table), symbolic ranges beyond a
stable parameter relation (CG20), variable-length and flexible extents (CG17),
multi-base and derived relations (CG24), and bounds summaries of callee bodies.
Card QUANT-INDEX-CHECKS adds the quantified index check `quant checks P, Idx
#i: …;` to the same consumer (C09.2).

**Transport and pins (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS, 2026-10-04;
gateway rule [12r](agents/gateway.md#constraint-12r)).** Landed: bounds
modules live in their library's provider crate and are listed in the crate
record's generated `bounds_view` on the published load path (the owned modules
the bounds loader accepts; disjoint from the resource view, so no resource
selection loads one); with `QCORE_BOUNDS_PROPERTY` the pinned route, the
configured route and the contracts route load every bounds view on the load
path their resource manifest names before any unit is analysed. A bounds
declaration is **pinned** (C06.2's default; there is no flexible bounds
declaration): its reviewed identities live in the crate's
`bounds-contracts.lock` (C06.2's lock format, keyed by the declaration's
normalised text digest, which covers every clause and a quant check's
predicate), written only by `quirin-audit pin --bounds` and verified with the
resource locks by `pin --verify-contracts`; a declaration whose pin is not in
force (`no-lock`, `not-in-lock`, `contract-changed`, `source-changed`) is held
under the reason code `contract-pin-mismatch` exactly as a review marker holds
it (discharges nothing, accuses nothing, `Pending` at most), and a re-pin after
review is the act that resolves it. The first reviewed declarations are
chunkio's header accessors `cio_file_st_get_meta_len` and
`cio_file_st_set_meta_len` (`quant checks map, Idx #i: 22 <= #i && #i < 24;`
with a closed read, resp. write, frame), pinned at `66910c10` and `046accf6`.
A header-defined function has no row on the audit routes (they report each
unit's main-source definitions), so its declaration is applied at its callers
and its body is checked only where a consumer analyses it. Still not landed
(`CG16-BOUNDS-CONTRACT-TRANSPORT`): checkpoint transport, inclusion and
inheritance of bounds modules, the cross-unit refutation phase on the routes,
and tip tracking for bounds pins.

**Body evidence, refutation phase and tips (card
CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS, 2026-10-09; gateway rules
[14f](agents/gateway.md#constraint-14f)–[14j](agents/gateway.md#constraint-14j)).**
A header-defined function still has no row on the audit routes. Its body,
and every declared body a unit defines, is checked against its declaration in
each unit's first phase and published as **evidence for the declaration**
(C06.5's shape: a `Safe`, unrefuted body covers its quant checks and
preconditions; a frame clause only a positive frame check of its parameter; a
relation, a return extent, `bounds_preserves_extents()` and a global extent
nothing; `validated` only where every examining unit validated it and every
clause is covered in every one), with a roll-up of the bounds `Safe` rows; no
verdict reads it. A declaration a body refutes in one unit is withdrawn from
every unit's callers by the routes' second phase (C06.4's two phases). A
bounds declaration may track the tip of a branch exactly as C06.2 says for a
resource declaration. Inclusion and inheritance stay refused for bounds
modules, and a checkpointed run refuses the bounds selection, since a
checkpoint carries no bounds rows: both remain `CG16-BOUNDS-CONTRACT-TRANSPORT`.
The quant body check also reads a byte-type view of a quant object (every C
byte type has size 1). Further chunkio header accessors are declared:
`cio_file_st_get_hash` (`bounds_return_derived(map, 2, 4, bytes)`, a read
frame), `cio_file_st_infer_content_len` (`22 <= #i && #i < 24`, read) and
`cio_file_st_set_content_len` (`10 <= #i && #i < 14`, write), pinned at
`66910c10` and `046accf6`.

**Frames and derived relations (card CG24-ALIAS-EXTENT-FRAME-VOCABULARY,
2026-10-02; gateway rule [11e](agents/gateway.md#constraint-11e)).** Landed in
the same consumer and grammar: a **closed frame**, one of
`bounds_frame_reads(P)`, `bounds_frame_writes(P)` or `bounds_frame_moves(P)`
for every pointer parameter and every *slot* parameter (`ptr<mut, ptr<mut,
u8>>`, a C `T **`). Frames are not custody modes: a custody-borrowed argument
may still be written or reallocated. A read position keeps every bounds fact;
a write position withdraws the facts of the local whose address it receives
and keeps extent facts on objects it only writes; a move position (an in-place
reallocation, keeping or moving the base, such as `cfl_sds_cat_safe`'s slot)
also withdraws every fact on an object the source did not establish. A local
whose address appears only as a direct actual of resolved closed frames may
carry facts; a held frame tracks no local and keeps its other facts only as
held premises. A closed frame, and that the callee retains nothing derived
from a parameter beyond the call or its stated relation, is the
declaration's `Declared` premise. A **derived relation**
`bounds_return_derived(P, OFF, LEN, UNIT)`, on one base or several (the result
is one of them), states that the result is `P + OFF` with `LEN` units: when the
base's extent decides containment the result is inside that base, a base too
small contradicts the declaration (reported, no fact), and otherwise only the
declared view is used, after the call's preconditions are established. An
analysed body refutes its frame when it writes through a parameter declared
read or hands a parameter to a stronger frame position, and its relation when
it returns a source object or another base or offset; refuted declarations are
withdrawn from the unit's callers. Not landed: callback-spanning relations
(CG22's callees), a slot extent after reallocation, return-conditioned and
relation clauses for custody (W45-FB-BACKLOG-REFERENCE-VOCABULARY), and the
custody half of the in-place reallocation (W46-SDS-CAT-SAFE-IN-PLACE-REALLOC):
resource v1 is unchanged.

**Meaning and units.** A user MAY supply a contract describing an external
object's accessible extent or an external operation's index/extent relation.
The consumer MUST distinguish an element count from a byte extent and bind
each extent to an object or subobject, its base/offset, element type and width,
and the program state in which it is valid. An index-in-range fact means
`0 <= i < N` for that extent. Conceptual notation such as `Idx<N>` denotes this
relation only; it is not new Quirin syntax or the `Idx + Ordered` dimension
capability of language §L06. That capability alone proves neither an object's
extent nor a runtime index's range. Runtime values MUST NOT enter the closed
index evaluator by being named in a user contract.

For a byte access of width `w` at offset `o` into an extent of `B` bytes, the
consumer MUST establish `0 <= o`, `0 < w` and `o + w <= B`, with checked offset
and width arithmetic. Forming a one-past pointer and dereferencing it are
different obligations: `i == N` can satisfy an admitted formation rule but
never the element-access rule. An empty extent admits no element access.
The bounds consumer MUST NOT assume the absence of out-of-bounds undefined
behavior to establish the very bounds property being checked; a custody
profile's broader no-undefined-behavior premise is not a bounds proof.
Signedness, conversions, element-size multiplication and target-width overflow
MUST be interpreted under the recorded C target/profile, never by silently
substituting the closed natural-number evaluator for C semantics.

**Source facts and user assumptions.** Fixed-array extents and supported type
layout facts SHOULD be derived from the bound source/build evidence. A fact
derived and checked by the tool needs no human review merely because it is an
extent. Facts a user supplies about external storage or an external body's
behavior MUST enter this new consumer as review-held declarations until a
person resolves their review. Loading, naming, inference by an assistant, or a
plugin's unverified claim MUST NOT discharge that review. A reviewed
declaration is still a `Declared` assumption, not a proof of its C body.

Review state follows C06.1's distinction between review-held, declared,
inferred/validated and proposed facts. The new consumer MUST record the exact
clause, reason, optional reviewer/owner, disposition, retained text digest and
affected obligations. Resolution is an explicit edit of the held declaration
after human review, including rewriting a clause marked as requiring rewrite;
it is not a tool-minted approval or an authenticated sign-off. Reuse MUST bind
the exact contract, source/revision policy, build/target and evidence version;
a stale pin or changed clause cannot reuse the old admission. The resource-v1
review marker does not implicitly select or implement this new consumer.

A user declaration MUST NOT enlarge a source-established object or subobject,
erase a known offset, or override a checked contradiction. For example, a
contract claiming eight elements for an object established as `int arr[4]`
cannot make `arr[7]` in bounds. Conflicting premises MUST be reported and
withheld rather than making every obligation vacuously true. If a source body
is checked and refutes its index/extent declaration, dependent uses MUST
withdraw that declaration; merely re-approving it cannot discharge the
contradiction. A source-established violation independent of the declaration
retains its own evidence and verdict.

**Checking uses.** A precondition such as an index-in-range relation is an
obligation at every covered call site, not a fact made true by writing the
contract. A body may be checked relative to that precondition, but callers MUST
establish it and the result MUST retain that conditional scope. An external
postcondition may be used only as an in-force, reviewed assumption for the
specified exit after its call's preconditions are established. Every claimed
access MUST be checked against the actual base, index and extent at that
access; checking a declaration alone is insufficient. Writes, intervening
calls, aliases, resizing, release or reallocation that may invalidate those
relations MUST invalidate the affected facts unless their preservation is
established. A range fact confers no ownership, liveness, alignment,
initialization, effective-type or synchronization guarantee by itself.

**Verdicts and reporting.** The property uses the four-value composition of
language §L21.2 and the complete claim identity of C13:

| Situation for the requested bounds property | Result |
|---|---|
| All selected accesses and call-site obligations are discharged under recorded, in-force premises with complete scope coverage | `Safe` for bounds only |
| A named human act is the sole obstacle, and putting the held declarations in force as written leaves no other obstacle | `Pending`, naming the held clauses and the required act |
| An extent, identity, range, preservation step, relevant source form or analysis bound remains unresolved; or a declaration is contradicted | `Unknown`, retaining the specific reasons |
| A reachable out-of-bounds access is independently validated under the selected profile and in-force premises | `Unsafe` for that bounds violation |

A held declaration discharges nothing and accuses nothing. A hypothetical
violation obtained only by putting it in force is an `Unknown` standing reason,
not `Unsafe`. An independently validated violation elsewhere still joins as
`Unsafe`. Review alone MUST NOT turn a remaining unsupported or unproven access
into `Safe`. For a reachable `arr[7]` on a source-established `int arr[4]`, the
new property MUST detect the violation without requiring a user-written extent
contract; if reachability cannot be established, it MUST NOT publish a
validated violation. A proved-unreachable access is excluded only through
explicit checked reachability, not by dropping it from the inventory.

Custody and bounds results MUST remain individually visible. A request for
both joins both results and their reasons; custody-`Safe` plus bounds-`Unknown`
is not combined `Safe`. A custody-only result MUST state its narrower property
and MUST NOT advertise a bounds guarantee. Missing bounds support for a request
that includes bounds is `Unknown`, never an omitted row or an empty successful
scope. Diagnostic reasons, assumptions and certificates MUST preserve the
object/extent relation and every selected access or named import failure.

### C09.2. Quantified index checks: `quant checks`

**Card QUANT-INDEX-CHECKS, 2026-10-04 (user request: a `quant` constraint over
an index with `==`, `<`, `>`, `<=`, `>=`); gateway rule
[12l](agents/gateway.md#constraint-12l); grammar
[G05c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05c-quantified-index-checks).** A
bounds contract v1 declaration (C09.1) may state, for one pointer formal, which
indices the function uses through it:

```qcore
module contract;
extern fn cio_file_st_get_meta_len(map: ptr<mut, u8>) -> u16
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 24;
};
```

**Meaning (user decision 1).** `#i` is bound **universally**: every index the
function reads or writes through `map` satisfies the predicate. An index counts
elements of the C pointee type the audited declaration states (`elements` of
C09.1); `#i` ranges over the mathematical integers, and a C index value enters
through the source consumer's value reading, which applies C's conversions
under the recorded target; each judged access records its index expression's C
type. The predicate is comparisons `==`, `<`, `>`, `<=`, `>=` between the bound
variable, standing alone on one side, and a **bound term** (a canonical decimal
literal, a scalar formal's entry value, `+`/`-` of those, at most one formal and
added once), joined by `&&` (binding tighter) and `||`, grouped by parentheses.
A range is written with `&&`. `||` is admitted and read **literally**: the user's
spelling `#i < u || l < #i` admits every index when `l < u` and is rejected as
unbounded; it is never reinterpreted as `&&`. One bound variable per clause and
one clause per formal.

**The admitted set.** The predicate's interval reading (disjunctive normal form)
is exact over the integers (Lean `QCore.IndexQuant.mem_reading`). A set that is
**unbounded** (some range lacks a lower or an upper bound, decided from the form
alone) or, for a constant predicate, **empty** rejects the module; a symbolic set
found empty under a call's actuals decides nothing at that call. The facts carry
two forms: a bounded constant union, and the symbolic prefix `[0, f)` (`0 <= #i
&& #i < f`, the reading of `bounds_extent(P, f, elements)`); any other form with
a formal is accepted by the loader and yields **no fact** (a named Unknown
reason), never Safe.

**Checking uses.**

* **Body side.** The formal enters with the extent its callers must cover (from
  the least admitted index, or from `P` when that is not negative, to the
  greatest), and every element access, dereference and `->` through it,
  directly or through a copy or `P + k`, is first decided against the admitted
  set: **inside** (every value of the index admitted), then the ordinary extent
  decision follows; **outside** (no value admitted), the body **refutes** its
  declaration, which is withdrawn from the unit's callers (C06.4, rule 11s(7)),
  and the access is Unknown; otherwise the access is Unknown. An access is never
  in bounds on an undecided index. The seeded extent is a lower bound (the check
  says which indices are used, not how large the object is), so a decision *out
  of bounds* against it, a formation beyond it for instance, is undecided and
  never a violation.
* **Call side.** Each caller must establish that the actual's extent covers
  every admitted index under the actuals' values; the object being contiguous,
  that is the element accesses at the least and greatest admitted index. A
  violation is a call-precondition violation under rule 11s's reachability
  (`Unsafe` only when definitely reached under in-force premises; held premises
  accuse nothing); an unestablished extent, an unevaluated formal, an empty set
  or an unread form is Unknown.
* **Escape.** A checked formal passed to a callee is an access the body cannot
  see: it is decided inside only when the callee's own constant quant check on
  that position, shifted by the actual's constant offset, lies inside the
  caller's set; otherwise the pass is Unknown (never a refutation, since the
  callee need not use every index it admits).

Review, digests and verdicts are C09.1's: a contract-level
`bounds_review_pending(…)` holds the check (`Pending` when it is the sole
obstacle), the clause text is part of the declaration digest, a resolved check is
a `Declared` assumption, and an independently validated violation elsewhere
still joins as `Unsafe`.

**Placement (user decision 2) and the rejected alternatives.** The user
suggested a collocated check instead of an `ensures`-like syntax. The clause
`quant checks P, Idx #i: …;` was chosen: (1) it is collocated with what it
constrains, naming the formal as its subject, one clause per formal like
`bounds_extent(P, …)`; (2) it is a check of the body and an obligation of the
callers, so it has its own introducer and **no exit selector**, and the
selector-based `quant normal …` envelope of C03 keeps its structural,
`ensures`-shaped meaning; (3) every consumer already enumerates contract
clauses, so the resource-v1 loader refuses it and the structural parser retains
it (the spelling `quant checks` was a parse error before this card), and no
consumer can drop it silently. Rejected:

* a **parameter-collocated** `map: ptr<mut, u8> [| quant Idx #i: … |]`: the
  predicate would sit inside the parameter's type text, which every
  declaration reader reconstructs (resource v1's signature comparison,
  inclusion digests, the source checker), and its `<`/`<=` would break the
  angle-bracket depth by which the parameter list is split at commas; the
  `[| quant #c_ffi |]` precedent (C04.12) is a declaration-level marker kept out
  of the declaration's tokens, which a per-parameter predicate cannot be, and
  in type position the predicate would read as a refinement type, which it is
  not;
* **overloading `quant normal …`** with check meaning (an exit-selector,
  postcondition-shaped envelope);
* a **predicate call** such as `requires bounds_index_where(P, …)`: G05b's
  predicate calls are identifier/comma forms, and comparisons and connectives
  inside an argument would hide a second expression grammar.

`Idx #i` is the clause's binder only: it is not the language's `Idx + Ordered`
capability (§L06) and does not touch `index.closed`. Its `&&` and `||` are
the predicate's own connectives: the frontend's operator gate admits them
inside a `quant checks` clause and nowhere else in a contract module (card
CHUNKIO-META-LEN-BOUNDS-CONTRACTS; before it, a tracked module carrying a
quant check failed to compile at `Q038`). Pins, transport and audit selection
of bounds modules landed with card CHUNKIO-META-LEN-BOUNDS-CONTRACTS (C09.1's
status paragraph). Not landed (named residual): symbolic forms beyond
`[0, f)` (`CG16-BOUNDS-SYMBOLIC-RANGES`).

### C09.3. Null dereference: the property `c.null.deref.v1` and `quant null-deref`

**Card NULL-SAFETY-PROPERTY, 2026-10-10; gateway rules
[14k](agents/gateway.md#constraint-14k)–[14p](agents/gateway.md#constraint-14p);
grammar [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks).**

> **USER DECISION 2026-10-10.**
> 1. **Move the NULL question** (resolves `CG10-INTERIOR-ALIAS-NULLABILITY`).
>    `&B->F`, or any use of `B` as non-NULL, is read as non-NULL only when
>    every path to it already went through a real read or write via `B`, with
>    `B` not reassigned in between. Forming `&B->G` and `sizeof`/unevaluated
>    operands do not count as dereferences. The NULL question then moves to
>    that dominating dereference, and that dereference becomes a published
>    check in a new null-safety property. **Proved:** from a NULL test, a
>    non-NULL contract result or parameter, a checked allocation, or a further
>    dominating dereference up the chain. **Otherwise:** a NULL-dereference
>    candidate, validated for reachability before any Unsafe and never
>    presented as an upstream defect. A custody verdict relying on the
>    non-NULL reading lists a named assumption tied to the dereference site
>    until the null-safety check proves it. It appears in the per-verdict
>    dependency set and roll-up. User motivation: NULL dereference is one of
>    the most common bug sources, so the value is in surfacing candidates, not
>    only in freeing functions.
> 2. **Contract syntax** (the user called it "quite ideal"): `quant null-deref
>    #B;` A collocated check form, like `quant checks`. It means "this function
>    dereferences `B`; `B` must be non-NULL." **Caller side:** the actual for
>    `B` must be proved non-NULL, otherwise it is a candidate at the call site.
>    **Callee side:** the body check validates that the body dereferences `B`
>    without a prior NULL test. Inside the body, after the first dereference,
>    `B` and `&B->F` are non-NULL (decision 1). A body that tests `B` first does
>    not need the clause; it is reported as stronger than necessary, never as
>    wrong.

**The property.** `c.null.deref.v1` is a separately selected property of
existing C, beside custody and bounds (C09.1): the audit selects it with
`QCORE_NULL_PROPERTY` and publishes one null row per projected function beside
an unchanged custody report; a request naming custody and null publishes their
join (`Safe < Pending < Unknown < Unsafe`, a missing row `Unknown`). A custody
certificate claims nothing about null dereference, and a null row nothing
about custody.

**What is checked.** Every evaluated `*p`, `p->f` and `p[i]` read or write, an
atomic access through a pointer, and every actual at a callee's `quant
null-deref` position. Unevaluated operands (`sizeof`, `_Alignof`, a
`typeof`, a trait, an unselected `_Generic` association) and address formation
(`&B->G`, `&B[i]`, `&*E`, an array member's decay) are not dereferences. A call
through a function pointer is not inventoried (a registered residual).

**Sources of possibly-NULL.** The result of a callee whose resource contract
states an owned or reallocated *nullable* result (an allocation); a derived
nullable result (a lookup); any other pointer result no contract states; a
formal with no clause; a pointer read from memory (a member, an element,
`*pp`, a global, an address-taken local); a null pointer constant on some
path; an uninitialized local.

**Sources of non-NULL.** The address of an object, an array's decay, a string
literal, a function; a resource contract's non-null owned result (a listed
`declared:` assumption); an in-unit body whose every return is non-null; the
`errno` lvalue's address (C11 7.5p2); the function's own clause (declared, or
inferred below); a NULL test's non-null arm (`if (p)`, `!p`, `p != NULL`,
`p == NULL`'s other arm, `&&`, `||`, `?:`, `__builtin_expect`), including an
early exit or a no-return call on the other arm; `assert(p)`'s surviving arm
when the analysed text still holds the assertion, as the listed premise that
the deployed build does too (`NDEBUG` absent); and decision 1's dominating
dereference.

**Facts.** A local or formal whose address is never taken keeps its fact
across calls; a pointer `B ± n` is non-NULL exactly when `B` is (C11 6.5.6p8),
so an offset keeps the moved question; any other assignment replaces the fact
(the reassignment of decision 1). A fact on memory (`s->next`, a global, an
address-taken local) is forgotten at every call and at every store that could
write a pointer; a store of a non-character scalar keeps it under the listed
premise that it cannot modify a pointer object (C11 6.5p7). Paths join (a
value is non-NULL after a join only when it is on every arriving path); a loop
is iterated to its fixed point (a function whose loop does not reach one is
`Unknown`); a label a later `goto` reaches forgets every fact.

**Verdicts.** A check is *proved* (non-NULL on every path), *moved* (non-NULL,
the question moved to an earlier check of the same function, whose own
outcome stands in the inventory), *held* (discharged only by a held clause),
a *candidate* (possibly NULL, with its sources), a *violation* (NULL on every
path) or *excluded* (unreachable). The function is `Safe` only when every check
is proved, moved or excluded and no form went unread; `Pending` only when held
clauses are the sole obstacle; `Unsafe` only for a violation on the
straight-line prefix from entry (no branch, loop, call or label before it);
otherwise `Unknown`, with every candidate listed. A candidate is never
presented as an upstream defect. A row is `Safe` *conditional on* its own
preconditions when its scope names a declared or inferred clause.

**The clause `quant null-deref #B;`** (decision 2). Admitted in **bounds
contract v1 only** (G05d), not in resource contract v1: it is a check form
like `quant checks`, read by a separately selected property; resource v1
declarations feed custody's environment digest and certificates, which the
clause would change for a clause custody never reads, and the bounds
vocabulary already carries the check-form machinery (pins in the crate's
`bounds-contracts.lock`, C06.2's holds, the selection on the audit routes).
The bounds consumer reads the environment without these clauses: a
declaration left with none, and a module left with none, leave the bounds
identity (`BoundsContracts::for_bounds`), so no bounds row moves. The clause
text is part of the declaration digest, so a changed clause takes the pin out
of force (held: discharges nothing, accuses nothing). **Callee side:** `B` is
non-NULL at entry and the row is conditional on the clause; the body is
checked against it: `necessary` when every terminating path dereferences `B`
before any test or assignment of it, otherwise `stronger-than-necessary`,
never wrong (a stronger precondition is sound; it only asks more of callers).
**Caller side:** the actual is checked at the call (`call-precondition`); a
possibly-NULL actual is a candidate there; after the call the actual is
non-NULL with its question moved to that check. A held declaration's
obligation is `held` (Pending at most), and moves nothing. A declaration whose
parameters do not match the call's arguments or the definition is withdrawn
(`Unknown`).

**Inference** (rule 14p). In a unit, a body whose every terminating path
dereferences a formal (directly, through a copy or an offset, or by passing it
to a clause position) before any NULL test or assignment of it infers the
clause; the unit's callers carry the obligation, the body is `Safe`
conditional on it, and a body whose every return is non-null gives callers a
non-null result. Summaries are computed callees first; a recursive edge is
cut (no clause, no result). An inferred clause never leaves its unit, and a
caller in another unit sees nothing unless the clause is declared.

**Custody** (decision 1, rule 14n). Custody's dereference obligation reads a
dereference through a **target-free, unknown, possibly-NULL binding of the
identity derivation** in an unwidened state (what a pointer-valued path read
and rule 14b bind) as non-NULL; every other part of the obligation is decided
as for a non-NULL binding, and a tracked, derived, mixed, widened or known
binding keeps its refusal. The reading leaves the named premise
`null-deref:FUNCTION:LINE:VARIABLE`: on the function's summary
(`null_premises`), on the published row, in its `contract_dependencies` (its
own and, transitively, its in-unit summaries'), and in the Safe roll-up
(`resting_only_on_a_null_premise`, `carrying_a_null_premise`). The null row
says of each premise whether the null property proves its check. The binding
itself stays possibly NULL for custody (a release, a publication or an escape
through it decides as before).

**Grounding: a usage grade read with access and identity** (card
NULL-DEREF-QTT-GROUNDING; language specification
[§L07](QCORE_SPEC_r0_3.md#l07-usage-grades), with
[§L08](QCORE_SPEC_r0_3.md#l08-credit-live-obligations-cost-and-access-quantities)
and [§L09](QCORE_SPEC_r0_3.md#l09-ownership-identity-and-borrowing)).

> **USER DECISION 2026-10-11.** The user asked which of Quirin's type systems
> `quant null-deref` belongs to, since `quant` always refers to the
> quantitative types (the question as the card's brief records it). The
> decision, verbatim: "We need grounding." The user accepted the integrator's
> analysis, which this paragraph states normatively: `quant null-deref #B;`
> is the contract-level projection of a §L07 usage grade, read with §L08–§L09
> access and identity. It is not a §L06 index or dimension, and not a
> `credit`, `live` or `cost` axis.

*Placement.* `quant checks P, Idx #i: …` (C09.2) belongs to §L06: static
dimensions, `Idx + Ordered`, `index.closed`. `quant null-deref #B;` belongs to
§L07. It introduces no type, no dimension and no axis. The spelling stays as
a readable shorthand for the reading below, and its `#B` binder names a formal
as `Idx #i` names an index variable. The two forms stay distinct: `quant
checks` is an index predicate evaluated by the closed index vocabulary, while
`quant null-deref` has no index, no predicate and no arithmetic. The bounds
consumer never reads `quant null-deref`, and the null property never reads
`quant checks`.

*The use.* A dereference of `B` (every check of "What is checked" made through
`B`'s value) is a use of `B`'s pointee: an operation on an identity. It needs
an access share on that identity's footprint (§L08, `FULL = 1024`). A raw
pointer carries no custody (§L09), so the use needs a live identity to borrow
from. NULL has no identity, so no share exists for it. Non-NULL is therefore
the existence precondition of every such use.

*The grade of a body*, for each pointer formal `B`:

1. **A path's demand.** Along one terminating path, each dereference of `B`'s
   entry value adds one use (§L07: sequential demands add). The account
   closes at the first NULL test of that value: after it the body holds its
   own evidence, and later uses are proved by the test, not by the caller. It
   also closes at the first assignment of `B` (decision 1's reassignment):
   after it `B` names another value. `B ± n` is the same value (C11
   6.5.6p8) and adds nothing. Passing the value to a clause position of a
   callee whose clause is in force (declared and pinned, or inferred in the
   unit) is §L07 demand substitution: the parameter's `relevant` times the
   argument's `one` is `relevant`, so it adds a use. A held clause is not in
   force and adds nothing. Any other call position is `many` times `one`,
   lower bound 0. A path's demand is the exact interval `[n, n]`, where `n` is
   the number of uses before the account closes.
2. **The join.** The body's grade is the least enclosing interval of its
   terminating paths' demands (§L07: a mutually exclusive branch join is the
   least enclosing interval, not the sum). Its lower bound is the minimum and
   its upper bound the maximum (`inf` once a loop can repeat a use). A loop
   contributes its unrolled paths. A label that a later `goto` reaches counts
   as a path whose account closed at 0, because the walk forgets every fact
   there. A body with no terminating path has no grade.
3. **Only the lower bound is read.** The upper bound never enters an
   admission.

*The clause is the grade `relevant`.* "Dereferenced on every path before any
NULL test" is exactly a lower bound of at least 1: the grade `relevant`,
`[1, inf]`. A dereference on only some paths is `many`, `[0, inf]`. A lower
bound of 0 imposes no caller obligation, which is why the clause is inferred
(rule 14p) only when the body's grade is `relevant`. A body with no grade
infers nothing, so its formal stays possibly NULL in its own row.

*The moved question is sequential demand addition.* On a path where a use has
occurred (a demand of 1) and `B` is not reassigned, every later use, such as a
further dereference or `&B->F`, adds to a demand that already requires the
identity. The identity requirement of a sum is required as soon as one summand
requires it, and it is required once. The later use therefore inherits the
identity that the first use required. This is decision 1: the later check is
`moved` to the first.

*Caller obligation.* A declared `relevant` use admits an actual only if the
actual's identity exists, that is, only if it is non-NULL. A caller's
`call-precondition` check is discharged exactly when no value that the walk's
reading of the actual admits is NULL. A grade whose lower bound is 0 admits
every actual.

*Callee validation is one-sided.* The body check of a declared clause is
`relevant`'s admission of the body's grade. It is `necessary` exactly when the
body's grade has a lower bound of at least 1, otherwise
`stronger-than-necessary`. Here the clause departs from §L07, deliberately.
§L07 admission rejects a declared grade that does not contain the actual
interval (the usage checker refuses `one` on an unused parameter). The
clause's body check reports instead, and never rejects (decision 2: never
wrong). The departure is sound. The clause asks the caller for an existing
identity: a precondition, not a linear demand. A read's access share is
borrowed and conserved (§L08), not consumed. Asking for a share the body need
not use therefore spends nothing, and the callee's non-NULL entry reading
holds for every caller that meets the obligation.

**Decomposition: a dereference read through the `quant` family** (card
NULL-DEREF-QTT-GROUNDING).

> **USER DIRECTION 2026-10-11.** `quant null-deref` is read through the
> original `quant` family. The relations between the parts are derived from
> the proven models, never added as rules or axioms. Verbatim: "This can be
> derived from the proven lean. And it can be natural."

A dereference `*B` or `B->f` is safe only when three parts hold. Each part is a
projection of the same dereference use, and each is owned by an existing
system:

| Part | Clause | Owner |
| --- | --- | --- |
| (a) existence: `B` names an identity, so it is not NULL | `quant null-deref #B;` | §L07 grade and §L09 identity; the null-dereference property |
| (b) extent: `B`'s extent covers the dereferenced object | `quant checks B, Idx #i: #i == 0;` (element units of `*B`) | §L06, C09.1–C09.2; the bounds consumer |
| (c) liveness: the identity has not been released | `quant live #B;` | §L08 and the custody property |

Each entailment is a theorem of `QCore.DerefDecomposition`, derived from
`QCore.IndexExtent`, `QCore.IndexQuant`, `QCore.NullDeref`,
`QCore.NullDerefGrade`, `QCore.Usage` and `QCore.BorrowLiveness`. The only
modelling choice is the concrete pointer value: NULL, or a position in an
identity's object with the elements that remain from it and that identity's
custody status.

1. **`checks(B, #i == 0) ⊢ null-deref(B)`.** NULL has no extent, so a caller
   whose index-0 check is discharged, against a count that is sound for its
   actual, passes a value that is not NULL. `relevant` admits that value, and
   `QCore.NullDeref` may read it non-null (`checks_zero_entails_null_deref`,
   `checks_zero_entails_extent`). **No analysis uses this entailment.**
   Discharging a NULL obligation from a bounds fact is a later behaviour card,
   registered as `NULL-DEREF-FROM-INDEX-ZERO-BOUNDS` with this theorem as its
   basis.
2. **The converse fails.** A one-past-the-end pointer is not NULL, yet no
   sound count discharges its index-0 check
   (`null_deref_does_not_entail_checks_zero`). A non-NULL pointer with no known
   extent never discharges it either (`an_unknown_extent_is_never_in_bounds`).
   `quant null-deref` is therefore a separate, weaker projection and is not
   defined as `checks #i == 0`.
3. **The grade decides which parts are caller obligations.** A part's caller
   demand is §L07 demand substitution: the use's grade times the part's
   per-use demand, which is `one` when the callee's contract states the part
   and `zero` otherwise. With a lower bound of at least 1 (`relevant`), (a) is
   an obligation, and so is each of (b) and (c) wherever the contract states
   it. With a lower bound of 0 (`many`), no part is an obligation
   (`relevant_obligates_existence`, `relevant_obligates_a_stated_part`,
   `an_unstated_part_is_never_obligated`, `zero_lower_obligates_no_part`,
   `body_grade_decides_the_obligations`).
4. **Liveness is independent.** A released identity can still have (a) and (b),
   and a sound count can discharge its index-0 check, yet it fails (c), and
   custody refuses it (`existence_and_extent_do_not_discharge_liveness`,
   `a_released_identity_is_refused_although_it_exists_and_has_extent`,
   `each_part_fails_alone`). No theorem lets (a) or (b) discharge liveness.

**Two levels: the shorthand and its parts** (card NULL-DEREF-QTT-GROUNDING;
grammar [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks)).

> **USER DECISION 2026-10-11.** Contract authors may write either the
> shorthand form or the decomposed form, and both must mean the same thing.
> The user chose "Two levels".

| Written | Canonical parts |
| --- | --- |
| `quant deref #B;` | `quant null-deref #B;` `quant checks B, Idx #i: #i == 0;` `quant live #B;` |
| `quant null-deref #B;` | itself: the existence part (a) alone, its meaning and its pins unchanged |
| `quant checks B, Idx #i: P;` | itself |
| `quant live #B;` | itself: the liveness part (c) |

* **One canonical form.** The bounds contract v1 loader elaborates every
  shorthand into canonical parts before any consumer sees the declaration.
  Consumers, per-clause standing and the dependency sets see only canonical
  parts, so a shorthand, its decomposition and every mix give the same clause
  set and the same standing (`crates/qcore-contract/src/deref_parts.rs`).
* **Overlap.** A part that a shorthand gives and a part written explicitly for
  the same pointer are merged when they agree; the shorthand's spelling is
  kept, and the bound variable's spelling is not meaning. Disagreeing parts
  are refused (`quant-deref-conflict`): a different index check, or a
  `bounds_extent` beside the shorthand's extent part (one fact per pointer).
  A repeated shorthand is refused (`duplicate bounds clause for deref B`), as
  are two written parts with one key (the loader's duplicate rule, unchanged).
  An unknown `quant WORD #B;` is refused by the parser
  (`quant-shorthand-unknown`).
* **Vocabularies.** `quant deref` and `quant live` are admitted in **bounds
  contract v1 only**, beside `quant checks` and `quant null-deref`. Bounds v1
  is the one vocabulary that already carries collocated check forms, pins
  them (`bounds-contracts.lock`) and selects them on the audit routes, and it
  is the one loader that sees all three parts of one declaration together.
  Each canonical part reaches its own consumer. The bounds consumer reads the
  extent part: `BoundsContracts::for_bounds` removes the existence and
  liveness parts, and a declaration or module left with no clause leaves the
  bounds identity. The null-dereference property reads the existence part.
  Custody reads resource v1 only, and the resource v1 loader **refuses** both
  new forms, as it refuses every `quant` form, rather than ignoring a part it
  cannot read. The liveness part therefore reaches custody as a **checked
  restatement**. An audit route admits a bounds declaration stating `quant live
  #B;` only where the function's own resource v1 declaration in force puts `B`
  at a `resource_owned` position, with the same arity: the caller must retain
  a known live resource across the call. Otherwise the route is refused
  (`quant-live-unbacked`, `quant-live-contradicts-custody`). This follows the
  precedent of the checked relation `null_deref_beside`. `resource_borrowed`
  does not back it, because custody's `Borrowed` rule refuses only an identity
  released on every path, which is weaker than "not released".
* **Pins.** The pin digest stays over the text as written (`text_sha256`), so
  rewriting a shorthand as its decomposition changes the text and needs a
  re-pin. That is the conservative choice. Each declaration also carries a
  canonical digest (`canonical_sha256`) over its signature and its sorted
  canonical part set, so a reviewer can see when two texts mean the same. The
  bounds selection record publishes every canonical digest, and every backed
  liveness part, only when some declaration states a liveness part. An
  environment without the new forms writes the record it wrote before.
* **Lean.** Elaboration preserves meaning in the three-part model, and merging
  an agreeing overlap is idempotent (`QCore.DerefDecomposition`:
  `shorthand_means_its_parts`, `elaboration_preserves_meaning`,
  `merging_an_agreeing_overlap_is_idempotent`, `merge_merge`,
  `merge_preserves_meaning`, `shorthand_and_decomposition_are_one_canonical_set`,
  `a_mix_is_the_shorthands_canonical_set`, `one_canonical_set_one_meaning`).

Lean: `QCore.NullDeref` (`walk_sound`, `guard_sound`,
`moved_question_is_the_origins`, `no_safe_on_an_undecided_pointer`,
`caller_obligation_establishes_the_entry`, `an_inferred_clause_is_necessary`,
`a_relied_on_reading_is_listed`, `combined_safe_needs_null_safe`), audited in
`proof/AuditNullDeref.lean`, vectors bound to the Rust.
The grounding: `QCore.NullDerefGrade` (`relevant_accepts_body_iff_inferred`,
`necessary_iff_relevant`, `zero_lower_imposes_nothing`,
`a_path_without_a_first_dereference_infers_no_clause`,
`moved_question_is_sequential_addition`, `branch_join_matches`,
`caller_obligation_is_grade_admission`,
`stronger_than_necessary_is_still_sound`), audited in
`proof/AuditNullDerefGrade.lean`. Its `gradeVectors` are bound to
`qcore_core::usage` and `qcore_core::null_deref` by
`crates/qcore-core/tests/null_deref_grade.rs`. That the walk's `First` sets are
this abstraction of its paths is a review premise (trust boundary 28). The decomposition and the two levels: `QCore.DerefDecomposition`, audited in
`proof/AuditDerefDecomposition.lean`; the loader's correspondence to its
`elaborate` and `merge` is a review premise as well, pinned by
`crates/qcore-contract/tests/deref_parts.rs` and
`crates/qcore-c-gateway/tests/deref_parts_null.rs`.

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

## Runtime source import boundary

Runtime `#import "./helper";` and `#import "../shared/helper.qr";` belong to
the runtime source-bundle resolver. They MUST NOT select contract modules,
load contract assumptions or substitute for the published `#include` path.
A contract source containing a runtime-file import refuses. Canonical bundled
capability imports are `#import std.extern;`; bare standard-module imports
remain compatible. Neither a runtime import nor another file's bundled imports
grant a caller an extern-call capability or unchecked declaration waiver.
The caller requires its own `std.extern`; the file declaring an unchecked
extern requires its own `std.extern.unsafe`. CLI `check --contract-load-path`
retains its separate explicit contract-loader dispatch and does not enable
runtime dependency loading.
