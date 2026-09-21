# Quirin contract specification — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the `.qcore` extension and this file's name —
> and every such identifier in this document is unchanged by the branding.

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

**Revision date:** 2026-09-07, Asia/Tokyo.  
**Status:** Source-reviewed contract semantics, consumer boundaries, and implementation requirements.  
**Input identity:** `qcore-c.tar.gz`, SHA-256 `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.

## C01. Scope and authority

This document accompanies the [language specification](QCORE_SPEC_r0_3.md). Together they supersede conflicting claims that all `.qcore` contracts have the same interpreter or the same proof strength. It specifies what a contract means, which current consumer can interpret it, which inputs are trusted, and what a result may claim. It does not change protocol version 1, resource-contract marker version 1, trace schemas, or compiler package versions.

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

The result vocabulary adds `unit` to the parameter vocabulary. `unit` is not a parameter type. Arbitrary named types, general pointer nesting, `own<T>`, `borrow<...>`, and language-level `result<T,E>` are not accepted by this loader. The two real-valued scalars are admitted for readings a resource API in scope passes by value; they carry no custody, exactly as every other scalar does; `fnptr` was admitted on the same condition, for msgpack's `msgpack_packer_init` in the pinned Cycle 19 corpus.

**Agreement with the analysed source (card LOADER-ABI).** A consumer analysing a translation unit MUST compare a declaration's projections against that unit's own C declaration of the same function wherever it can read one, and MUST withdraw the declaration for that unit — at every call site, and as the interface any body defined there is checked against — when the two disagree in the result type, a parameter type, the parameter count, or a claimed variadic tail. Withdrawal makes the callee uncontracted, which is the refusing direction; a consumer may never repair a disagreement by preferring either side. The projections are not decoration: the mode admissibility rules of this section are keyed on them, so an unfaithful projection is what would buy a mode the C type could not carry.

**Agreement with a derived summary (card LOADER-ABI-MODES).** Where a consumer independently derives a *validated* custody summary from a contracted function's own body in the analysed translation unit, it MUST compare the declared custody duties against the derived ones and MUST withdraw the declaration for that unit, at every call site, when they contradict. A parameter's duty is `retained` for `resource_owned` and `resource_borrowed`, `transferred` for `resource_released`, `resource_consumed` and `resource_stored`, `delivered` for `resource_out` and `restored` for `resource_removed`; a result's duty is `produced` for an owned or reallocated result and absent otherwise. A contradiction is a duty inequality at some position and nothing else: nullability, a `resource_stored`/`resource_removed` owner parameter and a reallocated result's replaced parameter are refinements within one duty and MUST NOT withdraw on their own. A consumer MUST NOT require a derived summary before applying a declaration — absence of evidence is not evidence, and a callee defined in another translation unit is never compared — and MUST NOT install the derived summary in the declaration's place, because a derived summary may be weaker than the declaration it contradicts. Withdrawal makes the callee uncontracted, which is the refusing direction.

These are opaque interface projections. They do not check actual C layouts, ABI compatibility, pointee validity, allocation size, alignment, aliasing, or thread-safety. A scalar may carry a descriptor number as data, but it cannot carry descriptor custody in v1. Only `resource_borrowed` may apply to a scalar or to `fnptr`. Every other parameter mode requires an admitted pointer type; `resource_out` specifically requires `ptr<mut, ptr<mut, u8>>`. An owned or reallocated result requires an admitted pointer result.

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

### C04.4. Unsupported semantics and false declarations

Only normal C returns are represented by v1 postconditions. A C routine returning null or a negative integer has still returned normally; it has not taken a QCore typed-error exit. Error-specific ownership behavior can be modeled only where a supported mode explicitly represents it, such as nullable acquisition or the specified reallocation relation.

General success-dependent parameter modes, exceptional exits, `ensures any`, `ensures error(E)`, quantitative clauses, effects, access permissions and synchronization are not in v1. Non-null error sentinels such as `MAP_FAILED` and scalar failure sentinels such as `-1` are not representable as tracked endpoint acquisition outcomes by the existing nullability-only return mode.

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

## C05. Manifests and immutable assumptions

A resource manifest has the following existing version-1 shape:

```json
{
  "schema_version": 1,
  "modules": ["qcore/libc/memory.qcore"]
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

The repository contains specialized importers and wrappers rather than one complete lifecycle theorem prover. These adapters inspect restricted source or execute restricted models, bind a fact environment, and use selected `.qcore` acceptance rules. Sharing the predicate evaluator does not make the adapters interchangeable.

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

Custody contracts state who remains responsible for a resource. Access contracts state who may read or write an identified footprint. Lifetime conditions state when identities remain valid. Quantitative clauses state credit, live-count or cost relations. These domains are related but not interchangeable.

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

`Safe` means the specified property has been established within that tuple, not that the program is safe in all respects. `Unsafe` requires a supported violation argument. `Unknown` means that the available supported analysis did not establish the requested result; it is not evidence of a defect and is not proof of safety. Parser acceptance, contract loading, observer `Checked`, abstract event checking and Lean theorem checking retain their own result categories rather than being relabeled indiscriminately.

Diagnostics SHOULD distinguish malformed syntax, unsupported vocabulary, missing premise, failed source import, observed policy violation, residual custody, incompatible summaries, rejected evidence and execution-environment failure. A LeakSanitizer containment failure, missing toolchain or unavailable source checkout is not a program-level safety verdict.

## C14. Migration and acceptance requirements

Existing valid resource-v1 modules keep their exact meaning. This revision does not broaden the loader to swallow lifecycle clauses, change nullable into arbitrary failure semantics, add a hidden sentinel policy, or infer new modes. Existing observation policies retain eager checked arithmetic and their outside-preconditions distinction. Current v1 plugin Safe refusal remains intact.

A conforming implementation change MUST include positive, negative and mutation tests at the relevant boundary. Resource-loader tests must cover whole-declaration rejection, duplicate/missing modes, invalid owner relations, pointer/scalar mismatches, manifest validation and immutable assumption replay. Observation tests must cover missing facts under apparently unused branches, arithmetic overflow, false preconditions, false postconditions and provenance-binding failures in the adapter. Concurrent/resource integration tests must cover destruction with outstanding authority, post-join events, incorrect mutex-footprint bindings and external failure residuals.

W2 profile data, W3 evidence v2, richer W4 inference, provider extraction and additional CT mappings MUST be reported independently. A specification requirement or a proposed regression does not turn an unmapped CT case into a mapped one. The current manifest has five mapped cases out of fifty. Current validation and the separately unexecuted archive review are distinguished in [VALIDATION.md](VALIDATION.md).

The original review was documentation-only; this integration also tracks separately validated Rust/Lean repairs. The advertised review probe package was not among the five supplied files. Current regression tests and evidence are linked by the acceptance ledger; no missing probe execution or archive reconstruction is claimed.
