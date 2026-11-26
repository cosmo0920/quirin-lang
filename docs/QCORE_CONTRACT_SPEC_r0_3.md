# Quirin contract specification — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the legacy `.qcore` alias, and this file's name —
> and those component identifiers are unchanged by the branding. Current contract
> files use `.qv`, runtime sources use `.qr`, and native ABI manifests use `.qrlib`;
> legacy suffixes remain warned compatibility aliases.

> **Repository integration:** 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with `main` in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

**Revision date:** 2026-09-07, Asia/Tokyo.  
**Status:** Source-reviewed contract semantics, consumer boundaries, and implementation requirements.  
**Input identity:** `qcore-c.tar.gz`, SHA-256 `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.

---

## Conceptual Overview & Table of Contents

In Quirin, a `.qv` contract file is **not** a universal theorem automatically proved across all compiler passes. Instead, the repository shares one outer module and declaration envelope across multiple deliberately narrow, fail-closed **contract consumers**. Each consumer accepts a closed syntactic and semantic vocabulary, enforces total-or-reject loading within its domain, and produces a strictly bounded artifact: a trusted external custody environment, a checked bounds/null table, a runtime loaded-code envelope, an evaluated integer observation, or a proposed formal hypothesis graph.

To read or author Quirin contracts accurately, four orthogonal dimensions must always be kept distinct:

1. **Syntactic Envelope vs. Semantic Consumer (`Part I`):** Parsing a `contract { ... }` block structurally proves nothing. A caller must select a specific interpreter (`resource_contract_v1`, `bounds_contract_v1`, `outer_contract_v1`, observation, or model replay), and any unsupported clause must either reject the module or yield `Unknown` for the requested property — never silently disappear.
2. **External C Custody & Protocol Semantics (`Part II`):** `resource_contract_v1` models how C functions acquire, borrow, retain, move, count, synchronize, and release tracked resource identities, callbacks, and outer plugin boundaries, paired with formal symbolic premises and hypothesis graphs that record every unproved assumption explicitly.
3. **Trust Standing, Source Pinning, & Bundle Governance (`Part III`):** A loaded contract may be `Declared` (a reviewed assumption pinned to exact C source blobs via `contracts.lock`), `InferredValidated` (derived bottom-up from an analyzed C body), `CheckedSpecification` (withdrawn unless proved against its C body), `PendingHumanReview` (withheld as `Pending`), or `Proposed` (evidence-only, never justifying `Safe`).
4. **Orthogonal Properties, Observations, & Claim Envelopes (`Parts IV–V`):** Memory/descriptor custody, array/offset bounds (`c.bounds.index-extent.v1`), null-dereference safety (`c.null.deref.v1`), OS endpoint cleanup, dynamic execution observations, and out-of-process plugin claims remain separate properties composed under the four-value lattice `Safe < Pending < Unknown < Unsafe` ([§L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)).

### Table of Contents

- **[Part I — Contract Foundations, Consumers, Envelopes, and Inclusion](#part-i--contract-foundations-consumers-envelopes-and-inclusion)**
  - [C01. Scope and authority](#c01-scope-and-authority)
  - [C02. Consumer selection and clause coverage](#c02-consumer-selection-and-clause-coverage)
  - [C03. Common source envelope](#c03-common-source-envelope)
    - [C03.1. Formal contract hypotheses, conditions, theorems, and effects](#c031-formal-contract-hypotheses-and-conditions-migration-in-progress)
  - [C15. Contract module inclusion (`#include`), load paths, and inheritance](#c15-contract-inclusion-and-inheritance)
    - [C15.1. Runtime source import boundary](#runtime-source-import-boundary)
   - [C15.2. Declared contract dependencies](#c152-declared-contract-dependencies)
- **[Part II — External Resource Contracts v1 (`C04`)](#part-ii--external-resource-contracts-v1-c04)**
  - [C04. Resource-contract v1: total-or-reject loading](#c04-resource-contract-v1-total-or-reject-loading)
  - **[Cluster C04-A — Signatures, Custody Modes, Return Dispositions, and Retention](#cluster-c04-a--signatures-custody-modes-return-dispositions-and-retention)**
    - [C04.1. Admitted C ABI projection](#c041-abi-projection)
    - [C04.2. Parameter custody modes](#c042-parameter-modes)
    - [C04.3. Result custody modes](#c043-result-modes) (`C04.3.1` Derived results · `C04.3.2` Inferred derived results · [`C04.3.3` The no-return exit](#c0433-the-no-return-exit) · `C04.3.4` Borrowed results · `C04.3.5` Checked same-item detach results)
    - [C04.4. Unsupported semantics and false declarations](#c044-unsupported-semantics-and-false-declarations)
    - [C04.5. The literal-argument premise](#c045-the-literal-argument-premise) (`C04.5.1` Call-site constant spellings)
    - [C04.12. FFI function definitions: `[| quant #c_ffi |]`](#c0412-ffi-function-definitions-quant-c_ffi)
    - [C04.13. Return-code dispositions](#c0413-return-code-dispositions) (`C04.13.1` Member out slots · [`C04.13.2` Packing a return-code disposition](#c04132-packing-a-return-code-disposition))
    - [C04.14. Retained pointers](#c0414-retained-pointers) (`C04.14.1` Guarded retentions · `C04.14.2` Result as holder · `C04.14.3` Holder paths & removal of one registration · `C04.14.4` Derived retention summaries)
    - [C04.15. Slot moves](#c0415-slot-moves)
    - [C04.16. Derived out slots](#c0416-derived-out-slots)
    - [C04.17. Reference counts](#c0417-reference-counts)
  - **[Cluster C04-B — Resource Kinds, Cell Protocols, and Concurrency/Lifecycle Obligations](#cluster-c04-b--resource-kinds-cell-protocols-and-concurrencylifecycle-obligations)**
    - [C04.6. Resource kinds and integer-carried identities](#c046-resource-kinds-and-integer-carried-identities) (`C04.6.1` In-place kinds · [`C04.6.2` Descriptor cells](#c0462-descriptor-cells) · [`C04.6.3` Declared cell protocols](#c0463-declared-cell-protocols) · [`C04.6.4` Callee frames over a declared cell protocol](#c0464-callee-frames-over-a-declared-cell-protocol))
    - [C04.7. Counted users](#c047-counted-users) · [C04.8. Container publication](#c048-container-publication) · [C04.9. The loop-owner obligation](#c049-the-loop-owner-obligation) · [C04.10. The thread-synchronization obligation](#c0410-the-thread-synchronization-obligation) · [C04.11. The paired lifecycle obligation](#c0411-the-paired-lifecycle-obligation)
    - [C04.21. Local conditional prerequisites and registered lifetimes](#c0421-local-conditional-prerequisites-and-registered-lifetimes)
    - [C04.22. The list-finiteness premise](#c0422-the-list-finiteness-premise)
  - **[Cluster C04-C — Named Premises, Symbolic Formulas, and Hypothesis Graphs](#cluster-c04-c--named-premises-symbolic-formulas-and-hypothesis-graphs)**
    - [C04.18. Named premises: overview and evolution](#c0418-named-premises)
    - [C04.18.9. Symbolic premise statements (formula version 1)](#c04189-symbolic-premise-statements-formula-version-1)
    - [C04.18.1](#c04181-evidence-only-hypothesis-graphs-version-1) · [C04.18.2](#c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2) · [C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3) · [C04.18.8](#c04188-explicit-entry-condition-roots-graph-version-4): Hypothesis graphs versions 1–4
    - [C04.18.3. Proposed premise flow](#c04183-proposed-premise-flow-flow-version-1-schema-201) · [C04.18.4. Symbolic lifecycle metadata](#c04184-symbolic-lifecycle-metadata-source-version-1)
    - [C04.18.5. Explicit guarded decomposition documents (version 1)](#c04185-explicit-guarded-decomposition-documents-version-1) · [C04.18.6. Guarded production source embedding](#c04186-guarded-production-source-embedding-source-version-1-bridge-version-2) *(adoption halted)*
  - **Cluster C04-D — Callback Protocols, Member-Path States, and Runtime Outer Envelopes**
    - [C04.19. Callback contracts](#c0419-callback-contracts) (`C04.19.1`–`C04.19.7`)
    - [C04.20. Outer contracts: code loaded at runtime](#c0420-outer-contracts-code-loaded-at-runtime) ([`C04.20.1` Runtime list callback candidate inventories](#c04201-runtime-list-callback-candidate-inventories))
- **Part III — Contract Governance, Trust Standing, and Bundle Lifecycle**
  - [C05. Manifests and immutable assumptions](#c05-manifests-and-immutable-assumptions)
  - [C06. Declared, inferred and proposed contracts](#c06-declared-inferred-and-proposed-contracts) ([`C06.1` Human review holds](#c061-contracts-awaiting-human-review) · [`C06.2` Revision modes & pins](#c062-revision-modes-pinned-and-flexible-contracts) · [`C06.3` Checked specifications](#c063-checked-specifications) · [`C06.4` Refuted contracts](#c064-contracts-their-own-bodies-refute) · [`C06.5` Per-verdict dependencies](#c065-per-verdict-contract-dependencies) · [`C06.6` Bundle supersession](#c066-contract-supersession-across-bundles))
- **Part IV — Orthogonal C Property Contracts: Bounds, Indices, Null Dereference, and OS Cleanup**
  - [C09. Custody, access, lifetime and quantitative clauses](#c09-custody-access-lifetime-and-quantitative-clauses) ([`C09.1` Index & extent contracts](#c091-reviewed-index-and-extent-contracts-planned) · [`C09.2` `quant checks`](#c092-quantified-index-checks-quant-checks) · [`C09.3` Null dereference & `quant null-deref`](#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref) · [`C09.4` C library result sets](#c094-declared-c-library-result-sets))
  - [C10. Cleanup and OS endpoint contracts](#c10-cleanup-and-os-endpoint-contracts)
- **Part V — Observation Policies, Source-Bound Adapters, Plugins, and Claim Reporting**
  - [C07. Observation interpreter](#c07-observation-interpreter) ([`C07.1` Eager evaluation](#c071-eager-evaluation-and-result-categories) · [`C07.2` Fact provenance](#c072-fact-provenance))
  - [C08. Source-bound and lifecycle adapters](#c08-source-bound-and-lifecycle-adapters)
  - [C11. Verifier protocol v1 and the certificate firewall](#c11-verifier-protocol-v1-and-the-certificate-firewall) ([`C11.1` User contracts & checker plugins](#c111-user-defined-contracts-and-checker-plugins) · [`C11.2` Audit subcommand plugins](#c112-audit-subcommand-plugins) · [`C11.3` Contributor records](#c113-contributor-records-in-a-composed-verdict))
  - [C12. Obligations/evidence integration and remaining requirements](#c12-obligationsevidence-integration-and-remaining-requirements)
  - [C13. Claim identity and reporting](#c13-claim-identity-and-reporting)
  - [C14. Migration and acceptance requirements](#c14-migration-and-acceptance-requirements)

---

# Part I — Contract Foundations, Consumers, Envelopes, and Inclusion

<a id="c01-scope-and-authority"></a>
## C01. Scope and authority

This specification accompanies the [Quirin language specification](QCORE_SPEC_r0_3.md). Together, these two documents supersede any earlier or conflicting claims that all `.qv` (and legacy `.qcore`) contract files share a single interpreter or carry uniform proof strength. Specifically, this specification governs:

1. **Contract meaning:** the exact mathematical or operational relation a contract clause denotes.
2. **Consumer authority:** which implemented consumer is permitted to read and interpret a given contract vocabulary, and what that consumer must reject.
3. **Trust boundaries:** which inputs enter an analysis as trusted declarations, which are derived from C or Quirin source bodies, which are evaluated over recorded runtime observations, and which require independently replayed certificates.
4. **Permissible claims:** what a published verification, observation, or audit result is allowed to state on the basis of those inputs.

This revision does not alter verifier protocol version 1, the `resource_contract_v1` marker version, trace serialization schemas, or Cargo package versions in the compiler workspace.

### Normative keywords and grammar authorities

Throughout this specification, `MUST`, `MUST NOT`, `SHOULD`, and `MAY` express normative conformance requirements for any implementation claiming support for the relevant profile. They are **not** assertions that every historical or current subsystem already satisfies the requirement in full; differences and open residuals are tracked in the [r0.3 review and acceptance ledger](reviews/r0_3/REVIEW.md).

Grammar authorities are partitioned deliberately:

- The **[implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)** define the exact, restricted input grammars accepted by current contract consumers (`G01`, `G05`, `G05b`–`G05l`, `G05-formula`, `G05-guarded`).
- The **[extended design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf)** records the broader target surface of the Quirin language; it is **not** the grammar of the external resource loader or any other specialized contract consumer.

### Ontological boundary of a contract

A contract is a relation between explicitly identified preconditions, an operation, and its possible exit states. Three foundational anti-conflation rules apply across every subsystem:

- **Loading is not proof.** Successfully parsing, elaborating, or loading a `.qv` module constructs an assumption or policy representation; it never proves that an external C implementation satisfies that declaration.
- **Naming is not truth.** Naming a proposition, premise, or hypothesis atom after a theorem or invariant does not establish its truth.
- **Provenance visibility under composition.** Whenever results are joined or composed across functions, translation units, or tools, the distinction between a *trusted declaration*, a *source-derived summary*, an *evaluated runtime observation*, and a *checked certificate* `MUST` remain explicitly visible in the output's assumptions and provenance records.

---

<a id="c02-consumer-selection-and-clause-coverage"></a>
## C02. Consumer selection and clause coverage

Quirin uses a single outer declaration and contract-block syntax across several distinct subsystems, while keeping each consumer's semantic vocabulary narrow and closed. Because syntactic acceptance is shared whereas semantic interpretation is specialized, a contract's meaning depends strictly on **which consumer is invoked**.

### Implemented contract consumers

| Consumer | Selection mechanism and accepted payload | Meaning of a successful run |
|---|---|---|
| **Structural source frontend** | Invoked on a `.qv` or `.qr` module containing declaration envelopes; retains `requires`, `ensures`, and `quant` clause spans and tokens. | Lexical, syntactic, and selected structural frontend checks succeeded. General contract predicates are **not** proved or enforced. |
| **External resource loader v1** | Explicit loader invocation (`contract_loader`); every declaration must be an `extern fn` carrying `requires resource_contract_v1;` and only admitted resource-v1 clauses (`C04`). | A complete, total-or-reject external custody environment (`ResourceContracts`) was constructed as a set of explicit assumptions. |
| **Observation interpreter** | Explicit invocation (`contract_observation`) supplying contract source text, a target function name, and an integer fact map (`C07`). | All relevant `requires` clauses evaluated to true and all supported `ensures normal` postconditions evaluated to true for that single supplied fact map. |
| **Chunk / source-bound interpreters** | Explicit Rust adapters (`C08`) that extract restricted C source summaries and bind them to selected frame or field policies. | The specifically imported C source facts satisfy that adapter's policy relative to its stated caller and ABI premises. |
| **Mechanism / lifecycle interpreters** | Explicit adapters (`C08`, `C12`) that supply or derive lifecycle traces for an identified abstract model or native execution. | Acceptance within that bounded model or recorded observation only — never a proof over arbitrary C source code. |
| **Verifier host v1** | Invoked with a resolved plugin manifest alongside request and response envelopes (`C11`). | Process transport, target binding, and envelope consistency checks passed; **no** protocol-v1 `Safe` certificate is accepted. |

*(Orthogonal property loaders introduced in later waves — specifically the bounds/null contract loader `bounds_contract_v1` in [C09.1](#c091-reviewed-index-and-extent-contracts-planned)–[C09.4](#c094-declared-c-library-result-sets) and the runtime loaded-code loader `outer_contract_v1` in [C04.20](#c0420-outer-contracts-code-loaded-at-runtime) — follow the exact same explicit-selection, disjoint-vocabulary discipline.)*

### Deliberate interpreter dispatch

A caller `MUST` select the intended contract consumer explicitly:

- Writing `requires resource_contract_v1;` inside a contract block is mandatory when feeding a module to the external resource loader, but its presence does **not** cause the ordinary `quirin check` command to dispatch arbitrary `.qv` or `.qr` files into the C custody analyzer.
- Current source-level verifier scopes enter bounded core obligation derivation and dispatch, while unsupported lifecycle source effects leave their obligations incomplete (`Unknown`).

### Explicit CLI validation of load-path modules (`--contract-load-path`)

For a resource-v1 contract module owned by a crate in a published load-path manifest ([C15](#c15-contract-inclusion-and-inheritance)), admission checking is invoked explicitly via `--contract-load-path`:

```sh
quirin check --contract-load-path providers/contract-load-path.json \
  providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv
```

This command enforces the following exact boundary:

1. **Module selection:** It selects one contract module owned by a crate on the load path (including an owned helper or test module that is not itself exported in `PUBLISHED`).
2. **Inclusion verification:** Every parent module reached via `#include` must belong to the `PUBLISHED` set of either the owning crate or one of its directly declared Cargo dependencies, and its file bytes must match the SHA-256 digest recorded in `providers/contract-load-path.json`.
3. **Meaning of success:** Passing validates that the elaborated module satisfies `resource_contract_v1` syntax, ABI projection, and inherited-clause agreement ([C15](#c15-contract-inclusion-and-inheritance)). It inspects **no** C function bodies and establishes **no** `Safe` verdict. Without `--contract-load-path`, any module containing `#include` or `contract inherit` is refused immediately by the structural parser.

### Clause-accounting invariant (no silent dropping)

Every verification or audit result `MUST` account for every contract clause relevant to the requested property:

- **Retention vs. discharge:** Editors, formatters, and structural AST tools may retain unparsed or uninterpreted clauses as syntax, provided their output never implies that those clauses were checked or discharged.
- **Mandatory refusal or scoping:** When a semantic consumer encounters an unsupported clause in a selected declaration, it `MUST` do one of three things:
  1. Reject the module or selection outright (total-or-reject loading);
  2. Return `Unknown` for the requested property; or
  3. Explicitly restrict its published claim tuple ([C13](#c13-claim-identity-and-reporting)) to a narrower, orthogonal property for which that clause is irrelevant (for example, custody ignoring `bounds_contract_v1` modules, or `BoundsContracts::for_bounds` stripping `quant null-deref` clauses that belong exclusively to `c.null.deref.v1`).
- **Prohibition on silent weakening:** Silently ignoring or deleting an unsupported clause while publishing the original full-contract or property claim is a hard conformance defect.

### Ordinary source-checker boundary (`Q021`)

In executable and library Quirin sources (`.qr`), the ordinary compiler checker rejects nonempty contract blocks on defined source functions with diagnostic **`Q021`** until source-level contract enforcement is implemented. Contract blocks on `extern fn` declarations remain structural envelopes unless an explicit external-call or audit consumer is selected. Neither structural retention nor `Q021` rejection proves arbitrary postconditions.

> **Implementation & Provenance.** Structural frontend check and `Q021` gate: `crates/qcore-frontend` and compiler check driver (review ledger item `R08`). Load-path CLI admission: `quirin check --contract-load-path` over `qcore_contract::load_path_manifest`.

---

<a id="c03-common-source-envelope"></a>
## C03. Common source envelope

Contract files share a common module header, function-attachment syntax, and clause-introducer envelope before any consumer-specific parser inspects their predicates.

### Module header (`module contract;`)

A contract module normally opens with the explicit mode declaration:

```qv
module contract;
```

In `contract` mode, the file has no executable entry point and the compiler never synthesizes a `main` function. Consumer requirements on the header are:

- **External resource loader v1 (`C04`):** `MUST` require `module contract;` at the top of every selected source.
- **Generic observation interpreter (`C07`):** Does not require `module contract;` so that historical compatibility policies continue to parse, but all new reusable observation and contract modules `SHOULD` declare `module contract;` explicitly.

### Function contract attachment and clause envelopes

A `contract { ... }` block attaches to an `extern fn` declaration (or, structurally, to a defined `fn` declaration). The structural parser lexes and retains each clause's expression as exact source text and UTF-8 byte spans:

```qv
module contract;

extern fn observe(result: i32) -> unit
contract {
    requires input_is_valid;
    ensures normal result >= 0;
};
```

In the declaration above, the predicates are shaped for the observation interpreter ([C07](#c07-observation-interpreter)), not `resource_contract_v1`. Which identifiers are in scope, what integer values they take, and whether `input_is_valid` is backed by source or runtime evidence are determined entirely by the consumer invoked on the module; structural parsing supplies none of those facts.

The shared clause envelopes recognized by the structural parser are:

| Clause envelope | Exit selector | Consumer scope and structural behavior |
|---|---|---|
| `requires EXPRESSION;` | *(entry — none)* | Precondition, vocabulary selector (`resource_contract_v1`, `bounds_contract_v1`, `outer_contract_v1`), governance marker, or named premise. |
| `ensures SELECTOR EXPRESSION;` | `normal`, `any`, or `error(E)` (with nonempty type text `E`) | Postcondition over the selected return path. Narrower consumers such as `resource_contract_v1` and `contract_observation` accept `ensures normal` only and reject `any` and `error(E)`. |
| `quant SELECTOR EXPRESSION;` | `normal`, `any`, or `error(E)` | Quantitative exit clause retained structurally; rejected by narrower consumers unless specifically admitted. |
| `quant checks EXPRESSION;` | *(collocated — none)* | Selectorless index-range check introduced for `bounds_contract_v1` ([C09.2](#c092-quantified-index-checks-quant-checks)); retained structurally and rejected by `resource_contract_v1`. |
| Specialized `quant` clauses (`G01p/c/t/e`, `G05d`, `G05g`, `G05i`, `G05k`, `G05l`) | *(form-specific)* | Formal propositions/conditions (`C03.1`), null/deref/live/cell/handoff states (`C04.6`, `C04.19`, `C09.3`), local conditional blocks (`C04.21`), list finiteness (`C04.22`), and outer envelopes (`C04.20`). Each is rejected outside its owning vocabulary. |

The structural parser rejects empty clauses (`requires ;`), missing semicolons, and unknown clause introducers. It does **not** embed a general theorem prover for retained expression text, and the external resource loader is **never** a fallback elaborator for Quirin language-level effects, `own<T>`, `borrow<...>`, usage grades, access shares, or task protocols.

<a id="c031-formal-contract-hypotheses-and-conditions-migration-in-progress"></a>
### C03.1. Formal contract hypotheses, conditions, theorems, and effects

Language specification requirement [§L19.1](QCORE_SPEC_r0_3.md#l191-formal-contract-hypotheses-and-quant-effects-required-extension) mandates replacing informal prose contract conditions with typed mathematical representations across all provider contract families. That repository-wide migration is currently in progress.

#### Implemented grammar forms vs. proposed discharge

Four bounded formal clause families are implemented at the grammar and representation layers ([implemented grammars `G01p`, `G01c`, `G01t`, `G01e`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)):

1. **`quant proposition`** (`G01p`): Typed first-order mathematical hypothesis atoms with explicit parameter sorts and quantifier domains ([C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3)).
2. **`quant condition`** (`G01c`): Explicit entry prerequisites and exit assertions ([C04.18.8](#c04188-explicit-entry-condition-roots-graph-version-4)).
3. **`quant theorem`** (`G01t`): Named formal theorem applications carrying all instantiated premises and the exact registered Lean model and audit identity.
4. **`quant effect`** (`G01e`): Explicit state-transition specifications exposing entry state, exit state, guard, transition relation, and frame.

While these representations are parsed, validated, and retained as formal obligations, **production proof discharge and general C source-effect interpretation remain proposed**. Declaring any of these four forms records obligations and grants **zero** automatic truth authority.

#### Semantics of `quant condition` (`G01c`)

The approved condition syntax is:

```qv
quant condition SELECTOR #id(PARAMETERS) = FORMULA;
```

Its selector and obligation rules are strict:

- **Entry vs. exit selectors:** `entry` designates a call prerequisite; `normal`, `error(E)`, and `any` designate exit assertions. Because C functions return error codes (such as `-1` or `NULL`) as ordinary C values rather than Quirin typed-error unwinding, a negative C return value is still selected by `normal`.
- **Partial correctness (no termination claim):** Exit conditions (`normal`, `error(E)`, `any`) assert that `FORMULA` holds on every matching **terminating** exit; they never assert that the C function terminates.
- **Separation from local guards:** `quant condition` clauses are distinct from the read-only member-flag bindings and `[[local]]` conditional prerequisite blocks of [C04.21](#c0421-local-conditional-prerequisites-and-registered-lifetimes).
- **Explicit parameter binding:** Relating formal parameters representing `before`, `after`, `result`, or execution `trace` states to actual C source states or abstract models remains an explicit input obligation.
- **Fail-closed consumer handling:** Any consumer that does not interpret a `quant condition` clause `MUST` reject the selection or return `Unknown` under [C02](#c02-consumer-selection-and-clause-coverage). An `entry` condition is **never** silently dropped, and an exit condition (`normal`, `error(E)`, `any`) is **never** assumed to hold at function entry.

#### Completeness and parity rules for provider migration

For a contract condition's migration from historical prose to formal syntax to count as complete:

1. **Preconditions** `MUST` expose their typed mathematical predicate and explicit quantifier domain.
2. **Theorem applications** `MUST` retain every instantiated premise alongside the exact registered Lean model and audit file identity.
3. **Effects** `MUST` expose entry/exit states, guard, transition relation, and frame.
4. **Retirement of prose:** Historical English prose may remain in comments or archive reports as human explanation, but prose strings cannot serve as the semantic representation of a migrated contract ([C04.18](#c0418-named-premises) enforces a global parse-time prose ban `QF001`–`QF005` across contract premise and guarded-symbol surfaces).
5. **Recorded correspondence:** Every migrated condition `MUST` track its replacement and correspondence status. Proposed graph refinements ([C04.18.2](#c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2)), symbolic lifecycle records ([C04.18.4](#c04184-symbolic-lifecycle-metadata-source-version-1)), and Lean theorem names do not by themselves prove equivalence to legacy prose or truth over C code; open source, environment, coverage, and theorem-input obligations remain visible.
6. **Parity before replacement:** Provider lock files (`contract-inventory.lock`, `contracts.lock`) require their standard sync and review steps, and every consumer must preserve its existing verdicts and semantics until its formal replacement demonstrates parity under the standing evidence gates. Syntactic conversion alone establishes no safety coverage.

> **Implementation & Provenance.** Grammars `G01p`, `G01c`, `G01t`, `G01e` in `crates/qcore-frontend` (`quant_proposition`, `quant_condition`); Lean models `QCore.QuantProposition`, `QCore.QuantCondition`, `QCore.BooleanConditionProfile`. Card `QUANT-INDEX-CHECKS` (`quant checks` envelope).

---

<a id="c15-contract-inclusion-and-inheritance"></a>
## C15. Contract module inclusion (`#include`), load paths, and inheritance

Many C libraries wrap or forward standard routines under project-specific names — for example, Fluent Bit's `flb_memcpy` forwarding to `libc`'s `memcpy`. Re-typing identical custody clauses by hand invites drift between wrapper and target contracts. Contract inclusion and inheritance allow a `.qv` module to import a published module from an explicit load path and copy a parent function's contract by positional parameter renaming:

```qv
module contract;
#include "quirin-provider-libc:libc/string.qv";

extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
contract inherit libc::memcpy;
```

### Pre-parse elaboration and fail-closed refusal (`QI013`)

Inclusion and inheritance are resolved **before** ordinary contract parsing by `qcore_contract::contract_inclusion`:

1. **Textual elaboration:** Against an explicit, digest-pinned load path, `#include` and `contract inherit` are elaborated into the exact standalone `contract { ... }` text an author would have written by hand. Every downstream consumer parses that elaborated text, and the loaded environment retains both the elaborated text and the full inclusion provenance ([C05](#c05-manifests-and-immutable-assumptions)).
2. **Structural parser refusal:** The ordinary structural parser `MUST` continue to reject both `#include` and `contract inherit`. Consequently, an unelaborated module can **never** slip past the parser into a consumer as an `extern fn` lacking a contract block.
3. **Entry points without a load path:** Any loader entry point invoked without an explicit load path (raw file lists, schema-1 manifests, in-memory source maps) `MUST` reject inclusion/inheritance syntax with diagnostic **`QI013`** and `MUST NOT` skip the directive or declaration.

### Module inclusion rules (`#include "CRATE:MODULE";`)

- **Placement:** `#include` directives `MUST` appear immediately after `module contract;` and before the first declaration in the file.
- **Path lexical safety:** In `"CRATE:MODULE"`, `CRATE` is the exact Cargo package name of a provider contract crate, and `MODULE` is a relative path matching `[a-z0-9_/-]+` ending in `.qv` (or legacy `.qcore`) with nonempty path segments under that crate's `contracts/` directory. Absolute paths, `.` or `..` segments, backslashes, and string escapes are lexically unrepresentable.
- **Direct dependency and publication visibility:** A module owned by crate `A` `MAY` include `"B:M"` if and only if:
  1. `B` is `A` itself, or `B` is a **direct** normal Cargo dependency of `A` (transitive dependencies are invisible); **and**
  2. `M` is listed in `B`'s declared `PUBLISHED` module set (this publication gate applies equally to same-crate includes `A:M`).
- **Acyclicity and uniqueness:** Include cycles and including the same target module twice `MUST` be refused. Because every `#include` names an exact `(CRATE, MODULE)` pair, there is no directory search path and no first-match shadowing rule.

### Provider crate naming and namespace derivation (`QI017`)

Every crate participating in a contract load path `MUST` have a Cargo package name beginning with one of two admitted prefixes:

- `quirin-provider-NAME` (library contract crates), or
- `qcore-provider-NAME` (executable provider crates; the test harness `qcore-provider-conformance` owns no contracts and is excluded from the load path).

From the package suffix `NAME`, the crate's contract namespace is derived deterministically by replacing `-` with `_`:

| Cargo package name | Derived `NS::` prefix | Example qualified parent |
|---|---|---|
| `quirin-provider-libc` | `libc` | `libc::memcpy` |
| `quirin-provider-fluent-bit` | `fluent_bit` | `fluent_bit::flb_malloc` |
| `qcore-provider-librdkafka` | `librdkafka` | `librdkafka::rd_kafka_conf_new` |
| `qcore-provider-libevent` | `libevent` | `libevent::mk_mem_alloc` |

Each crate records its `NAMESPACE` constant alongside its contract list, and the loader verifies that the recorded string equals the derived one. **No two crates on one load path may share a derived namespace**, even across the two package prefixes (for example, placing both `quirin-provider-libc` and `qcore-provider-libc` on the same load path is refused with **`QI017`**). Notice that a namespace always identifies the **owning provider crate**, never a C symbol prefix and never a re-export: Monkey's `mk_*` contracts live in `qcore-provider-libevent` and are therefore referenced as `libevent::mk_mem_alloc`.

### Contract inheritance rules (`contract inherit PARENT;`)

Writing `contract inherit PARENT;` on a plain `extern fn` copies `PARENT`'s contract clauses onto the child declaration under strict syntactic and positional checks:

1. **Name resolution:**
   - An unqualified `NAME` resolves **only** among declarations in the same module (never silently reaching into included modules).
   - A qualified `NS::NAME` resolves **only** among modules directly `#include`d by this file from the crate whose namespace is `NS`.
   - Zero matches, multiple matches, an unknown or unincluded namespace, or a child declaration bearing the same function name as its qualified parent `MUST` be refused.
2. **Signature compatibility:** The parent must be an `extern fn ... contract { ... };` declaration (inheritance chains are permitted, provided they contain no cycle). The child and parent `MUST` have the exact same parameter count, token-for-token identical parameter types, and token-for-token identical result type.
3. **Positional parameter substitution and capture prevention:** Clauses are copied in exact order, replacing every identifier token naming the `i`-th parent parameter with the `i`-th child parameter's identifier. To prevent accidental variable capture or grammar corruption:
   - If a child parameter name already appears in the parent's clauses as any non-parameter token (such as a predicate name, field name, or keyword), elaboration `MUST` refuse (capture error).
   - If any parameter name coincides with an exit-selector keyword (`normal`, `any`, `error`), elaboration `MUST` refuse.
   - Additional clauses after `contract inherit PARENT;` are not admitted in this version.
4. **Governance markers across inheritance:**
   - **Copied verbatim:** Custody modes, result modes, kinds, literal-argument premises (`C04.5`), loop-owner/thread-sync/lifecycle obligations (`C04.9`–`C04.11`), named premises and hypothesis-graph symbols (`C04.18`), and human-review hold markers (`C06.1`) are inherited by the child so a wrapper can never shed a precondition, premise, or review hold.
   - **Stripped on elaboration:** Revision modes (`pinned`/`flexible`, [C06.2](#c062-revision-modes-pinned-and-flexible-contracts)) and library version ranges (`resource_library_version`, [C06.2](#c062-revision-modes-pinned-and-flexible-contracts)) record how the *parent's* C function was reviewed against upstream source or documentation; because the child names a *different* C function, elaboration strips parent revision markers and unwraps clause-level revision markers, leaving the child `pinned` by default.

### Consumer semantic agreement check

Positional token renaming preserves semantics automatically for clauses whose identifiers are resolved purely through the declaration's parameter list. However, some consumers also inspect structural positions or side tables. Every consumer `MUST` compare its semantic reading of an elaborated child declaration against its reading of the parent declaration and refuse any discrepancy. In particular, `load_published_resource_contracts` checks exact agreement across parameter custody modes, result mode, ABI projection, literal-argument premises, variadic tail markers, and `C06.1` review-hold markers/dispositions, guaranteeing that an inherited contract is never weaker than its parent.

### Provenance, replay (`QI016`), packaging, and schema-2 manifests

An inherited contract is a **trusted assumption** of the exact same epistemic standing as a hand-written contract — its parent being loaded elsewhere is not a proof of the child's C body.

- **Serialized provenance and deterministic replay:** Any environment built with inclusion retains, for every selected module, its as-written text and identity, the full closure of included parent modules (`crate`, `crate version`, `module path`, `SHA-256`, and `text`), and every inheriting declaration's resolution chain back to the root written-out contract. Because the chain is part of the serialized environment, editing a parent's text changes the environment's identity even if the resulting elaborated text happens not to change. `check_source_assumptions` replays every elaboration purely from the retained texts and refuses any mismatch with **`QI016`**. Environments that use no inclusion serialize byte-for-byte identically to pre-`C15` environments.
- **Provider packages and bundles:** A packaged provider whose contracts `#include` external modules pins every reached parent module in its inventory's `includes` map and Vendors exact copies under `includes/CRATE/MODULE`. Both `providers/package.py verify` and `qcore_contract::bundle::ContractBundle` refuse unpinned directives, missing or modified parent files, unused pins, and unpinned files under `includes/`.
- **Generated load-path manifest (`QI014`) and schema-2 manifests:** Each `quirin-provider-*` library crate and executable `qcore-provider-*` crate (`src/main.rs`) declares `NAMESPACE`, `CONTRACTS`, and `PUBLISHED`. Running `python3 providers/contracts.py sync` combines Cargo dependency metadata with those constants to generate `providers/contract-load-path.json`, pinning every published module's SHA-256 (`contracts.py check` refuses stale manifests). Core crates read `providers/contract-load-path.json` strictly as data (`qcore_contract::load_path_manifest`, never linking provider crates) and refuse malformed manifests, directory/list mismatches, or SHA-256 mismatches with **`QI014`**. Resource manifests at **schema version 2** (and `contract/corpus.json` schema 2) name `load_path` and allow mixing relative file-path entries with `"CRATE:MODULE"` references under a single total-or-reject load. Inventory lock files count elaborated clauses, while written inheritance directives are pinned separately.

<a id="runtime-source-import-boundary"></a>
### C15.1. Runtime source import boundary

Quirin's executable/library `.qr` import system (`#import`) and its contract module inclusion system (`#include`) are strictly segregated and `MUST NOT` cross-pollinate:

1. **No runtime imports in contracts:** Relative runtime file imports such as `#import "./helper";` or `#import "../shared/helper.qr";` belong exclusively to the `.qr` runtime source-bundle resolver. They `MUST NOT` select `.qv` contract modules, load contract assumptions, or substitute for published `#include "CRATE:MODULE";` directives. Any contract source containing a runtime-file `#import` is refused.
2. **Per-file capability isolation:** Canonical bundled capability imports in `.qr` files (such as `#import std.extern;` and `#import std.extern.unsafe;`, alongside compatible bare standard-module imports) apply solely to the file that declares them. Neither a relative runtime `#import` nor another file's bundled imports can grant a caller the capability to invoke `extern` functions (`std.extern`) or waive unchecked `extern` declaration restrictions (`std.extern.unsafe`).
3. **CLI dispatch separation:** Running `quirin check --contract-load-path ...` invokes only the explicit contract-loader validation route ([C02](#c02-consumer-selection-and-clause-coverage)) and never enables `.qr` runtime dependency loading.

> **Implementation & Provenance.** Diagnostics `QI001`–`QI020` ([diagnostics namespace policy](agents/diagnostics.md)). Directive scanner: `crates/qcore-frontend/src/contract_inclusion.rs`; load-path manifest, resolver, elaborator, and replay: `crates/qcore-contract/src/contract_inclusion.rs` and `load_path_manifest`; loader integration: `crates/qcore-contract/src/contract_loader.rs`. Lean model: `proof/QCore/ContractInherit.lean`. Cards: `CONTRACT-INHERIT` ([report](reports/contract-inherit/README.md)), `CONTRACT-INHERIT-ADOPTION`, `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`.


<a id="c152-declared-contract-dependencies"></a>
### C15.2. Declared contract dependencies

Card QUIRIN-AUDIT-CONNECT (Part C; user decision of 2026-11-26). A resource-v1 declaration MAY state which other crates' declarations its contract relies on, with their identity:

```qcore
extern fn onig_new(reg: ptr<mut, u8>, pattern: ptr<const, u8>) -> i32
contract {
    requires resource_contract_v1;
    requires resource_contract_dependency("quirin-provider-libc", libc::malloc, since "0.1.0");
    requires resource_contract_dependency("quirin-provider-libc", libc::free, sha256 "<64 hex>");
    ...
};
```

**The clause.** A contract-level requirement (G05 `contract_dependency`): the crate's package name (the `#include` spelling), the declaration as `NS::NAME` with `NS` the crate's derived namespace (C15), and an identity: a version range over the crate's version (`since` inclusive, `removed` exclusive, read as `resource_library_version` reads them) and/or the SHA-256 of the target declaration's normalised text (`c-tokens-v1`, the `contract_sha256` its lock records, C06.2). At least one identity is stated; one clause per target. It adds no obligation and changes no other clause's reading. Loading is total-or-reject (G05). `libc` declarations stay owned by `quirin-provider-libc` (rule 10j, card W43-LIBC-CONTRACTS): a dependency on one names that crate and never re-declares it (`cx_refuse_libc_target_elsewhere`).

**The manifest.** The load-path manifest records each crate's declared dependencies (`contract_dependencies`, one entry per clause: dependent crate, function and module; target crate, namespace and function; `since`, `removed`, `sha256`), absent when a crate declares none so every other manifest is unchanged. `python3 providers/contracts.py sync` writes it for the shipped load path and `quirin-audit plugin views --write` for a user load path; `qcore_contract::load_path_manifest` rereads every module with the loader's own clause reader and refuses (`QI014`) an entry that differs, and refuses a load path whose declared dependencies form a cycle (`cx_cycle_holds`).

**Resolution.** After the pins (C06.2) and the library version ranges, the audit resolves every stated dependency of a declaration in force against the loaded environment and its load path (`qcore_schemas::connect::resolve`, `qcore_audit::declared_dependencies`). A dependency is **satisfied** exactly when its target crate is on the load path and is the dependent's crate or one of its declared dependencies, the target declaration is loaded, the crate's version lies in the range, the declaration's digest is the stated one, a libc target names `quirin-provider-libc`, and the target is not itself held (`cx_satisfied_is_every_check`, `cx_every_check_is_satisfied`). An unsatisfied dependency **holds** its declaration, a held declaration holds every declaration that depends on it (`cx_held_dependency_holds_dependents`), and a declaration on a cycle is held (`cx_cycle_holds`). The held set is the least fixed point of these rules over the **set** of dependencies, so it does not depend on their order, and a declaration that is not held reaches no held declaration and no cycle (`QCore.MeasureConnect`: `held_is_order_independent`, `resolution_is_acyclic`). A held declaration is withheld through the pin-mismatch table (C06.2): `dependency-changed`, every cause a changed digest or a target held `Pending`, is `Pending` (`contract-pin-mismatch`: a review and re-pin resolve it); `dependency-unresolved`, any other cause, is `Unknown` (`missing-premise`; the call site reads ``call to `NAME` whose declared contract dependency does not resolve (dependency-unresolved)``). Never `Safe`, never a provisional `Unsafe` (`cx_held_is_never_safe`). A declaration whose module lies on no load path has no owning crate, so every dependency it states is unresolved. A report's `contract_revision` section carries `contract_dependencies` (the graph with every target's identity, the cycles and every hold) only when some declaration states one, so every other report is byte-identical to before.

**Measurement and composition.** `quirin-audit measure` records each side's graph per route (`dependency_graph`); `measure chain` refuses a link whose graphs differ (`cx_refuse_dependency_graph`) and lists a dependency change beside a move it may explain, never as its attribution (`cx_dependency_change_listed`); `measure compose` resolves the bundles' declared dependencies and joins each hold into the functions resting on the held declaration.

---

# Part II — External Resource Contracts v1 (`C04`)

<a id="c04-resource-contract-v1-total-or-reject-loading"></a>
## C04. Resource-contract v1: total-or-reject loading

The external resource contract loader (`crates/qcore-contract/src/contract_loader.rs`) constructs a trusted C interface environment (`ResourceContracts` in `qcore-schemas`) from explicit file lists, an immutable map of source texts, or a versioned manifest ([C05](#c05-manifests-and-immutable-assumptions)).

Loading is **total-or-reject** across the entire selection:

1. **Nonempty module set:** The selected module list `MUST` be nonempty, and every selected module `MUST` begin with `module contract;` and contain at least one declaration.
2. **Extern-function exclusivity:** Every item in a selected module `MUST` be an `extern fn` declaration (alongside optional module-local `@`-enumerator aliases `alias #NAME = @C_NAME;` under [C04.19.5](#c0419-callback-contracts)). Function names `MUST` be unique across the selection unless resolved by pin-stamp bundle supersession on an audit route ([C06.6](#c066-contract-supersession-across-bundles)). The loader never skips observation policies, `resource type` definitions, constants, error declarations, or unsupported C functions; if any single declaration fails validation, **no partial `ResourceContracts` environment escapes**.
3. **Whole-declaration reconstruction:** Every `extern fn` declaration `MUST` satisfy four structural invariants simultaneously:
   - **Closed ABI signature:** Its entire parameter and return signature belongs to the admitted C ABI projection ([C04.1](#c041-abi-projection), or the `[| quant #c_ffi |]` struct projection of [C04.12](#c0412-ffi-function-definitions-quant-c_ffi)), with distinct parameter names and zero usage grades, generics, effect rows, or trailing tokens. The loader reconstructs the canonical declaration and compares it against the source token stream so a recognized prefix can never mask extra syntax.
   - **Mandatory version selector and closed preconditions:** It contains **exactly one** `requires resource_contract_v1;` clause. That marker is an interpreter and vocabulary version selector, not a caller-supplied Boolean runtime fact. Every other `requires` clause must belong to the closed set of admitted preconditions and governance markers: literal-argument premises ([C04.5](#c045-the-literal-argument-premise)), loop-owner ([C04.9](#c049-the-loop-owner-obligation)), thread-synchronization ([C04.10](#c0410-the-thread-synchronization-obligation)), paired lifecycle ([C04.11](#c0411-the-paired-lifecycle-obligation)), named premises and formal metadata ([C04.18](#c0418-named-premises)), union discriminants ([C04.19.2](#c0419-callback-contracts)), human-review holds ([C06.1](#c061-contracts-awaiting-human-review)), revision and library-version bounds ([C06.2](#c062-revision-modes-pinned-and-flexible-contracts)), checked-specification markers ([C06.3](#c063-checked-specifications)), or `[[local]]` conditional blocks ([C04.21](#c0421-local-conditional-prerequisites-and-registered-lifetimes)). Any other `requires` spelling rejects the module.
   - **Complete custody assignment:** Every parameter receives **exactly one** custody mode ([C04.2](#c042-parameter-modes), or one per exit class in [C04.13](#c0413-return-code-dispositions)), and the return position receives **exactly one** result mode ([C04.3](#c043-result-modes)), accompanied only by admitted `ensures normal` predicates and the specific `quant` state/cell/protocol clauses of `C04.6`, `C04.14.3`, `C04.19`, and `C04.21`. Predicate calls are flat identifier/comma forms (with admitted `when` / `where` guards where specified); nested calls, arithmetic, trailing commas, and general Boolean connectives are rejected.
   - **Type/mode compatibility:** Parameter types, result types, nullability or sentinel validity markers, owner references, and kind annotations satisfy all cross-clause compatibility rules specified below.

---

### Cluster C04-A — Signatures, Custody Modes, Return Dispositions, and Retention

<a id="c041-abi-projection"></a>
#### C04.1. Admitted C ABI projection

Resource-contract v1 projects C function signatures onto a small, closed set of opaque interface types. These projections govern which custody modes a parameter or result may carry, and are checked directly against Clang's AST of each analyzed C translation unit.

##### Closed parameter and result type vocabularies

| Category | Admitted `.qv` spellings | Allowed positions & custody eligibility |
|---|---|---|
| **Fixed-width & pointer-width scalars** | `bool`, `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64`, `usize`, `f32`, `f64` | Parameters and results. Carry **no** custody (`resource_borrowed` only), *except* `i32` with `descriptor` kind ([C04.6](#c046-resource-kinds-and-integer-carried-identities)) and `u32` by-value `thread_key` finalizers/uses ([C04.6.1](#c0461-in-place-kinds)). |
| **Depth-1 object pointers** | `ptr<mut, u8>`, `ptr<const, u8>` | Parameters and results. Erase the C pointee type (`void *`, `char *`, `struct T *`, `enum E *`); eligible for custody-bearing modes. |
| **Depth-2 pointer slots** | `ptr<mut, ptr<mut, u8>>`, `ptr<mut, ptr<const, u8>>` | Parameters and results. Represent C `T **` and `const T **`. Both carry custody, except plain `resource_out` ([C04.2](#c042-parameter-modes)), which requires `ptr<mut, ptr<mut, u8>>` exactly. |
| **Function pointer** | `fnptr` | Parameters and results. Deliberately **not** spelled `ptr<...>` because a function designator is never a resource identity; admits `resource_borrowed` only (plus callback clauses in [C04.19](#c0419-callback-contracts)). |
| **Variable argument list** | `va_list` | **Parameters only** (rejected in result position and inside `ptr<...>`). Projects compiler `__builtin_va_list`; admits `resource_borrowed` only. |
| **Void return** | `unit` | **Results only** (rejected as a parameter type). Projects C `void` return; requires `resource_return_none()`. |

Arbitrary named types (outside `[| quant #c_ffi |]` in [C04.12](#c0412-ffi-function-definitions-quant-c_ffi)), triple or deeper pointers, Quirin `own<T>`, `borrow<...>`, language-level `result<T, E>`, and transparent unions (such as glibc's `__CONST_SOCKADDR_ARG`) are outside this vocabulary and rejected.

##### Specific C type projection rules

1. **C enumerations (gateway rule 12u):**
   - Any C enumeration type (`enum TAG` or untagged `enum`, reached directly or through any `typedef` chain at a parameter or result) `MUST` be written in `.qv` as the exact fixed-width integer scalar (`i32`, `u32`, `i64`, `u64`, etc.) that **Clang assigns to that enumeration in the analyzed translation unit** (for example, `u32` for an enum with no negative enumerators on standard SysV/LLP64 targets, `i32` when a negative enumerator is present, a 64-bit scalar when values exceed `int`, or its C fixed underlying type).
   - No enum tag or enumerator identifier appears in the `.qv` type signature.
   - Consumers `MUST NOT` assume every C `enum` is `i32`, `MUST NOT` fold an `enum` onto pointer-width `isize`/`usize`, and `MUST` refuse (withdrawing the declaration for that unit) whenever Clang does not establish a unique standard integer type — including Clang launch failure, incomplete `enum` types, types unnameable at file scope, zero or multiple compatible standard integer types, or signedness disagreement.
   - The C gateway determines this per translation unit via a single appended compile probe (`sizeof`, sign of `(T)-1`, and `__builtin_types_compatible_p` against the ten standard C integer types), without injecting `--target`, `-fshort-enums`, or `-m32` (recorded boundary `ABI-ENUM-UNIT-FRONTEND-FLAGS`). Pointers to enums (`enum TAG *`) project as ordinary depth-1 object pointers.
2. **Const-qualified double pointers and parameter qualifiers (gateway rules 12v, W43-LIBC-RESULT-ALIAS):**
   - Nested pointer qualifiers `ptr<A, ptr<B, u8>>` read **outside-in**: `A` qualifies the outer pointee (the pointer slot itself) and `B` qualifies the innermost pointee.
   - Therefore, `ptr<mut, ptr<const, u8>>` is the exact projection of an unqualified pointer to a pointer to a `const` object: `const char **`, `const void **`, parameter-adjusted `const T *p[]`, or a `typedef` chain whose innermost pointee carries `const`. Because `char **` and `const char **` are incompatible types in C (C11 6.7.6.1p2, 6.7.3p10), neither `.qv` spelling admits the other.
   - A qualifier **on the inner pointer slot** — between the two stars (`char *const *`) or applied to a pointer `typedef` (`const str *` where `typedef char *str;`) — has no representation in v1 and `MUST` be refused.
   - By contrast, a top-level qualifier **after a parameter's final `*`** (`const char *__restrict fmt`, `void *const p`) qualifies only the callee's local parameter variable and is dropped by C for function type compatibility (C11 6.7.6.3p15); C consumers read the unqualified parameter type while keeping result declarators strict.
3. **Function pointers (`fnptr`) and `va_list`:**
   - `fnptr` `MUST NOT` be folded onto `usize` or `u64` even on architectures where data and function pointers share width: a consumer comparing `.qv` and C declarations `MUST` reject `fnptr` for any non-function-pointer C type and reject every other `.qv` spelling for a C function pointer. Like `ptr<mut, u8>`, `fnptr` erases the target function's own parameter/return types.
   - `va_list` names `__builtin_va_list` (underlying glibc's `va_list` / `__gnuc_va_list`) abstractly rather than its target-dependent lowering (an array-decayed pointer on x86-64 SysV vs. a struct on AArch64), and `MUST` be matched bijectively against that builtin only.

##### Translation-unit ABI and summary-duty agreement gates

 projections are **not** decorative: because custody modes are gated on projected `.qv` types, an unfaithful signature projection could smuggle a custody mode onto a C type incapable of carrying it. Two mandatory withdrawal checks protect every translation unit:

1. **C declaration ABI agreement (`LOADER-ABI`):** Whenever an analyzed translation unit contains a C declaration of a contracted function, the consumer `MUST` compare result type, parameter count, each parameter's projected type, and variadic status against the `.qv` declaration. Any mismatch **withdraws** the contract across the entire unit (at all call sites and as the checked interface of any C body defined there), leaving the callee uncontracted (`Unknown`). A consumer `MUST NEVER` "repair" a disagreement by preferring either the `.qv` text or the C header.
2. **Validated body-summary duty agreement (`LOADER-ABI-MODES`):** Whenever a consumer independently derives an `InferredValidated` custody summary ([C06](#c06-declared-inferred-and-proposed-contracts)) from the contracted function's own C body in the analyzed unit, it `MUST` compare coarse **custody duties** at every parameter and return position:
   - *Parameter duties:* `retained` (`resource_owned`, `resource_borrowed`), `transferred` (`resource_released`, `resource_consumed`, `resource_stored`), `delivered` (`resource_out`), and `restored` (`resource_removed`).
   - *Result duties:* `produced` (`resource_return_owned`, `resource_return_reallocated`) vs. *absent* (all non-acquiring result modes).
   - Any inequality in duty **withdraws** the declaration for that unit (`Unknown`). Finer distinctions *within* a single duty — `nullable` vs. `nonnull`, which parameter is `owner` in `stored`/`removed`, or which parameter is `replaced` in `reallocated` — do **not** trigger withdrawal on their own. Consumers `MUST NOT` require a derived summary to exist (external callees have no body in the unit) and `MUST NOT` substitute the derived summary when withdrawing a contradicted declaration, because a derived summary may omit non-custody obligations of the withdrawn declaration.

ABI projections remain opaque interface summaries: they do not prove C struct layout, calling conventions, allocation sizes, alignment, effective types, aliasing freedom, or thread safety.

> **Implementation & Provenance.** Loader ABI check: `crates/qcore-contract/src/contract_loader.rs`; gateway rules 12u, 12v. Lean models: `QCore.AbiEnumConstPointer`, `QCore.GnuBuiltinReader`, `QCore.PointerQualifierReader`. Cards: `LOADER-ABI`, `LOADER-ABI-MODES`, `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`, `W43-ABI-ENUM-AND-CONST-PP` (decision 2026-10-05; note that `rd_kafka_confval_set_type` at `rdkafka_conf.c:3763-3874` switches on `confval->valuetype` rather than its `valuetype` argument and therefore uses unguarded retention rather than `C04.14.1`), `W43-LIBC-RESULT-ALIAS`, `W44-LIBC-RESIDUALS`.

---

<a id="c042-parameter-modes"></a>
#### C04.2. Parameter custody modes

Every named parameter `p` in a plain `resource_contract_v1` declaration carries exactly one parameter custody predicate. In the table below, `n` is `nullable` or `nonnull` for pointer-carried resources (or `sentinel` / `valid` for `i32` descriptors under [C04.6](#c046-resource-kinds-and-integer-carried-identities)):

| Predicate | Custody semantics in the abstract resource model |
|---|---|
| `resource_borrowed(p)` | Imposes no callee custody precondition on `p` and transfers no tracked obligation through `p`. *(Not equivalent to Quirin's lifetime-checked `borrow<'a, ...>` type.)* |
| `resource_owned(p, n)` | Caller must supply live tracked custody at `p` (unless modeled NULL under `nullable`), and the caller retains that live custody after the call returns. |
| `resource_consumed(p, n)` | Caller supplies live custody at `p`, and the call transfers responsibility away from the caller. *(Transfer of custody is not proof of immediate deallocation or OS endpoint closure.)* |
| `resource_released(p, n)` | Trusted destructor contract: discharges the live resource obligation carried by `p`. A false `resource_released` declaration can unsoundly mask a resource leak. |
| `resource_out(p, n)` | Establishes a fresh tracked resource identity written through the output slot `p`. Requires `ptr<mut, ptr<mut, u8>>` (save for in-place kind initializers in `C04.6.1` and descriptor cells in `C04.6.2`). |
| `resource_stored(p, owner, n)` | Moves custody of `p` beneath the designated aggregate `owner` parameter (`owner` must be a distinct parameter with mode `resource_owned`). |
| `resource_removed(p, owner, n)` | Detaches `p` from containment beneath `owner` (`owner` must be a distinct `resource_owned` parameter) so `p` becomes directly held by the caller. |

##### Containment, nullability, variadic tails, and caller-side borrow liveness

1. **Identity-bearing parent/child containment:** In `resource_stored(p, owner, n)` and `resource_removed(p, owner, n)`, the relation tracks individual child identities indexed beneath `owner` — never a mere integer counter of stored children. Declaring that a parent destructor (`resource_released(owner, ...)`) cleans up stored children is a **trusted API assumption**, not a proof that an external C destructor actually traverses every child link.
2. **Scope of `nullable`:** Marking a pointer `nullable` states only that passing or receiving a modeled `NULL` pointer creates or requires no live resource obligation on that `NULL` branch. It does **not** model arbitrary integer error codes, `errno`, out-parameter failure states, or non-null error sentinels (`MAP_FAILED`).
3. **Variadic argument tails (`resource_variadic_data_tail` and `resource_variadic_borrowed_tail`):**
   - A `.qv` declaration of a C variadic function (`printf`, `open`, `snprintf`) lists only its fixed named parameters and `MAY` carry **at most one** variadic tail postcondition:
     - `ensures normal resource_variadic_data_tail();` — declares that the callee takes no custody of, and retains no pointer to, any trailing variadic argument. At every call site, the consumer `MUST` verify that every trailing actual is pure data carrying **no** tracked resource identity, and refuse (`Unknown`) otherwise.
     - `ensures normal resource_variadic_borrowed_tail();` — declares the data-tail guarantee **and** treats every trailing variadic position as `resource_borrowed`, allowing trailing actuals that name tracked identities to pass as borrowed (subject to the caller-side borrow-liveness check below) while refusing any trailing actual read neither as data nor as such a place.
   - Both flags are rederived from retained source texts (`variadic_data_tails`, `variadic_borrowed_tails`); an environment without retained source texts has neither. Neither clause checks `printf`/`scanf` format-string type agreement, which is not a custody property.
4. **Caller-side refusal on definitely-released borrows (`ALIAS-LOCALS-BORROW-LIVENESS`):**
   - Although `resource_borrowed(p)` states that the *callee* demands no custody, the C analyzer enforces an independent **caller-side refusal**: it `MUST NOT` certify (`Safe`) a function body that passes to a `resource_borrowed` position an actual whose projection has settled on a single tracked heap identity **already released on every reaching path**.
   - This outcome is published as `Unknown` (unresolved-identity diagnostic) and is **deliberately not `Unsafe`**, because `resource_borrowed` proves neither that the callee dereferences `p` nor that the path is dynamically feasible. Scalars, unresolved/ambiguous/possibly-null bindings, identities released on only some reaching paths, and `consumed` identities (whose ownership moved without declared destruction) are unaffected.

> **Implementation & Provenance.** Parameter modes and variadic tail rederivation: `crates/qcore-contract/src/contract_loader.rs`. Caller borrow-liveness model: `QCore.BorrowLiveness`. Cards: `O04-TRACE`, `CG26-VARIADICS`, `ALIAS-LOCALS-BORROW-LIVENESS`.

---

<a id="c043-result-modes"></a>
#### C04.3. Result custody modes

Every `resource_contract_v1` declaration `MUST` contain **exactly one** result custody predicate describing how the return value relates to tracked obligations:

| Result predicate | Custody and aliasing semantics |
|---|---|
| `resource_return_none()` | Returns no fresh, detached, or derived custody (used for `unit` and custody-free scalar or status returns). |
| `resource_return_owned(n)` | A non-null (or non-sentinel `descriptor`) return establishes a fresh tracked resource identity owned by the caller; `n` governs whether `NULL`/`-1` is possible. |
| `resource_return_reallocated(p, n)` | Models `realloc`-style replacement of incoming `resource_owned` parameter `p`: a `NULL` return leaves incoming `p` live and unconsumed; a non-null return consumes `p` once and establishes a fresh result identity. |
| `resource_return_alias(p, n)` | Exact derived result (`C04.3.1`): returns the exact pointer value passed at `p` (or `NULL` under `nullable`), owning nothing (`memcpy`, `strcpy`). |
| `resource_return_interior(p, n)` | Interior derived result (`C04.3.1`): returns a pointer inside the object `p` designates (or `NULL` under `nullable`), owning nothing (`strchr`, `memchr`). |
| `resource_return_borrowed_from(p, n)` | Parent-borrowed derived result (`C04.3.4`): returns an interior pointer or owned/lazily-created child of parent `p` (or `NULL` under `nullable`), owning nothing and invalidated by parent release or mutation. |
| `resource_return_removed(p, n)` | Checked same-item detach result (`C04.3.5`): detaches `p` from its checked parent and returns that **exact same** identity (or transfers nothing on `NULL`), creating no fresh site. |

For `resource_return_reallocated(p, n)`, an analyzer `MUST` preserve both outcome branches until a branch condition resolves whether the returned pointer is `NULL`. Treating an untested `realloc` return as having unconditionally freed `p`, or reducing reallocation to `net_allocations == 0` without replacing `p`'s identity, is unsound and forbidden.

A minimal pair of trusted heap allocator/destructor declarations illustrates `resource_contract_v1`:

```qv
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

Loading this module constructs a trusted external model of `allocate` and `release`; it does not verify their C implementations.

<a id="c0431-derived-results"></a>
##### C04.3.1. Derived results (`resource_return_alias` and `resource_return_interior`)

Standard C string and buffer routines frequently return either their exact input pointer (`memcpy`, `strncpy`) or an offset within that buffer (`strchr`, `strstr`). Modeling them as `resource_return_owned` would invent a bogus second free obligation, whereas modeling them as `resource_return_none` would lose track of the returned alias if the caller subsequently freed the base buffer or tried to `free()` an interior pointer.

Both predicates load into `ResourceReturnMode::Derived { parameter, extent, nullable, const_dropped }`, with `extent` set to `Exact` for `resource_return_alias(p, n)` and `Interior` for `resource_return_interior(p, n)`.

###### Placement rules (total-or-reject)

- Base parameter `p` `MUST` carry `resource_borrowed(p)` and have type `ptr<mut, u8>` or `ptr<const, u8>`.
- The declaration's return type `MUST` be `ptr<mut, u8>` or `ptr<const, u8>`.
- Any other placement (a custody-bearing base `p`, a scalar, a depth-2 pointer, `fnptr`, or `unit`) rejects the entire module.
- `const_dropped` is recorded automatically when `p` is `ptr<const, u8>` and the return type is `ptr<mut, u8>` (as in C's `char *strchr(const char *, int)`).

###### Mandatory consumer semantics

Every consumer interpreting `Derived` `MUST` enforce all six rules without approximation:

1. **Zero ownership:** Acquiring or discarding a derived result creates, transfers, and discharges no custody obligation (discarding it behaves identically to `resource_return_none()`).
2. **No base discharge through derived pointers:** Releasing, consuming, storing, detaching, reallocating, or returning (`return q;`) a derived pointer `MUST NEVER` be credited as discharging or transferring the base's custody obligation:
   - Passing a derived result of **`Interior`** extent that resolves to a single tracked identity to `resource_released` produces a provisional `Unsafe` violation candidate (`invalid_release`, because C11 7.22.3.3p2 requires `free` to receive the exact pointer returned by an allocator; it remains provisional because an `Interior` pointer at offset `0` would coincide with the base).
   - Passing an **`Exact`** alias to `resource_released`, passing an ambiguous/unresolved derived pointer to `resource_released`, or passing any derived pointer to any non-release custody-bearing position refuses (`Unknown`).
3. **Base release invalidates derived pointers:** On any path where the base identity has been released, a subsequent use of a pointer derived from it at any contracted call position (other than overwriting its variable via an out slot), or releasing or returning it, is a provisional `Unsafe` violation candidate (C11 6.2.4p2: a pointer to an object whose lifetime has ended has indeterminate value).
4. **Independent nullness narrowing:** Testing whether a derived result `q` is `NULL` narrows **only** `q`; it proves nothing about whether the base `p` is `NULL` or live, and testing `p` proves nothing about a `nullable` derived `q`.
5. **Denotable base actuals and offset degradation:** A consumer binds a derived result only when the base argument is a denotable place, `place ± offset` (one arithmetic offset token, C11 6.5.6p8), a string literal, or `&plain_name` (the latter two carrying no tracked identity); any other base expression refuses (`Unknown`). Whenever the base actual is `place ± offset`, the result is bound at **`Interior`** extent even if the callee declared `resource_return_alias` — because `memcpy(p + 1, src, n)` returns `p + 1`, not `p`.
6. **Tracked `const_dropped`:** Carried on the derived binding so future write-aware consumers can detect writes through const-stripped pointers (residual `W43-RESULT-ALIAS-CONST-WRITE`; resource-v1 has no read/write split on `resource_borrowed`).

###### Consumer and bridge boundaries

- In defining translation units, a C body with an active derived-result declaration is checked against it via `C04.3.2` and falls back to contract-free body analysis if validation fails.
- Quirin's `.qr` checked extern-call bridge (`qcore-contract::extern_bridge`) has no derived-result constructor and **refuses** derived-result declarations rather than unsoundly projecting them onto `resource_return_none()`.
- Adding derived results preserves `resource_contract_v1` marker compatibility and existing corpus digests (`source_assumptions` records exact module texts).

> **Implementation & Provenance.** Lean model: `QCore.DerivedResult`. Cards: `W43-LIBC-RESULT-ALIAS`, `W43-RESULT-ALIAS-RETURNED`.

<a id="c0432-inferred-derived-results"></a>
##### C04.3.2. Inferred derived results

When analyzing an uncontracted C helper in a translation unit, W4 summary inference can synthesize `ResourceReturnMode::Derived { parameter: k, extent, nullable, const_dropped }` as an `InferredValidated` summary, which callers then apply under the exact rules of [C04.3.1](#c0431-derived-results).

Inference proposes a derived-result summary **only** after ordinary live-entry and removed-wrapper inference publish nothing, and **only** when all four checks succeed on the body's CFG (desugaring whole-call `return f(...);` of derived callees):

1. **Purely borrowed base `k`:** Formal parameter `k` is seeded as a *name-only* site carrying no custody status. Consequently, any custody-bearing operation on `k` — releasing, consuming, escaping (including `return k;` directly), passing to `Owned`, using as an aggregate owner, or performing a bound custody read — immediately refuses, and no exit may observe any status or unresolved effect on `k`'s site.
2. **All exits return `NULL` or a derivation from `k`:** Every reachable `return` yields either a null pointer constant or a derived binding rooted at `k`'s site, and **at least one** reachable exit returns the latter. Returning a fresh allocation, an opaque pointer, void/scalar, a mixed/ambiguous binding, or a pointer derived from another formal, local, global, or string literal refuses.
3. **Conservative summary attributes from exits:**
   - `extent` is `Exact` if and only if **every** non-null exit has `Exact` extent (otherwise `Interior`, or `Borrowed` under `C04.3.4`).
   - `nullable` is `true` if any exit returns `NULL` or any derivation step along an exit can yield `NULL` for a non-null `k` (tracked independently of `k`'s own nullness).
   - `const_dropped` is `true` if any derivation step drops `const`.
4. **Full W4 re-validation:** Re-analyzing the body under that entry must model every statement without diagnostics, with all other formals receiving their ordinary least-binding inferred modes.

Any refused shape leaves the C function without a summary (`Unknown` at call sites).

> **Implementation & Provenance.** Lean model: `QCore.DerivedResultInference` (soundness against `C04.3.1` and completeness of shape refusals). Card: `W43-RESULT-ALIAS-RETURNED`.

<a id="c0433-the-no-return-exit"></a>
##### C04.3.3. The no-return exit (`resource_no_return`)

Functions such as `abort()`, `exit()`, and `_Exit()` never return to their caller because they **terminate the entire process** (or invoke undefined behavior), ensuring no stack frame of the calling thread ever resumes. By contrast, non-local control transfers (`longjmp`, `siglongjmp`, `setcontext`) and single-thread exits (`pthread_exit`, `thrd_exit`) also never return normally to their immediate caller, yet **the process continues executing while abandoning live resources on unwound stack frames**. Because C11 `_Noreturn` / `__attribute__((noreturn))` conflates process termination with non-local exits, Quirin never infers termination from C attributes alone; a contract declares it explicitly:

```qv
extern fn abort() -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_return_none();
    ensures normal resource_no_return();
};
```

###### Placement and `UNDECLARABLE_C_NAMES`

1. **Strict signature and sibling limits:** `ensures normal resource_no_return();` requires return type `unit` with `resource_return_none()`, requires **every** parameter to be `resource_borrowed`, and forbids counted-user, container, loop-owner, thread-synchronization, and lifecycle clauses alongside it. Violating any of these rejects the entire module.
2. **Permanently undeclarable C intrinsics and non-local exits (`UNDECLARABLE_C_NAMES`):** Regardless of clauses, the loader unconditionally rejects any declaration naming Clang call-like syntax extensions (`__builtin_choose_expr`, `__builtin_offsetof`), returns-twice functions (`setjmp`, `sigsetjmp`), or non-local/thread exits (`longjmp`, `siglongjmp`, `setcontext`, `pthread_exit`, `thrd_exit`), because giving them ordinary or terminating call semantics would mismodel their control flow.

###### Semantics and Clang cross-check

- **Vacuous normal exit, active preconditions:** Declaring `resource_no_return()` states that the normal-return post-state is unreachable (false), while all parameter preconditions (including caller borrow-liveness) still apply at the call site.
- **Text-rederived, never inferred:** Consumers rederive `no_return_callees` from retained module texts, so inferred or proposed summaries can never acquire it. A consumer that ignores `resource_no_return()` treats the call as an ordinary `resource_borrowed` call with a fallthrough edge — a sound path over-approximation.
- **Gateway lowering (rule 11o):** The C gateway lowers a call *statement* to a path terminator (zero CFG successors, zero exit obligation check on that dead branch) **only** when Clang's AST confirms the C declaration itself is `noreturn`; if Clang's declaration is not `noreturn`, the gateway refuses the call (`Unknown`).

> **Implementation & Provenance.** Gateway rule 11o; Lean model: `QCore.CallExits`. Card: `CG21-CALL-EXITS-AND-BOUNDARIES`.

<a id="c0434-borrowed-results"></a>
##### C04.3.4. Borrowed results (`resource_return_borrowed_from`)

Many C container and object APIs return a pointer to an internal sub-object, an owned child element (`cJSON_GetObjectItem`, `rd_kafka_admin_result_ret_groups`), or a child lazily allocated inside and owned by the parent (`rd_kafka_topic_partition_get_private`, `get_or_create_*`). Unlike a plain buffer offset (`resource_return_interior`), such a borrowed child handle becomes invalid not only when the parent is destroyed, but also if the parent container is mutated through another path (for instance, resizing a table or deleting keys), and passing the child pointer to `free()` cannot be accused as `invalid_release` because the child is often a standalone heap block owned by the parent.

```qv
extern fn child(parent: ptr<mut, u8>, key: ptr<const, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(parent);
    ensures normal resource_borrowed(key);
    ensures normal resource_return_borrowed_from(parent, nullable);
};
```

`resource_return_borrowed_from(p, n)` loads to `ResourceReturnMode::Derived { extent: Borrowed, ... }`.

###### Placement and `const` tracking

- Base `p` `MUST` be a `resource_borrowed` depth-1 pointer (`ptr<mut, u8>` or `ptr<const, u8>`).
- The return type `MAY` be any of the four object pointer projections: `ptr<mut, u8>`, `ptr<const, u8>`, `ptr<mut, ptr<mut, u8>>`, or `ptr<mut, ptr<const, u8>>` (admitting borrowed arrays of child pointers such as `const T **`).
- `const_dropped` is set when `p` is `ptr<const, u8>` and the outer pointee of the return type is `mut`.

###### Strengthened semantics over `C04.3.1`

All rules of [C04.3.1](#c0431-derived-results) apply, strengthened as follows:

1. **Refused, never accused, at custody positions:** Passing a `Borrowed`-extent result to *any* custody-bearing position (`resource_released` included) is **refused (`Unknown`), never accused as `Unsafe`**, because the child object may sit at the start of its own parent-owned heap allocation.
2. **Ended by parent release OR external parent mutation (gateway rule 12y, premise `P-BORROW-FRAME`):** A consumer ends an active borrow both when `p`'s parent is released and at any call or store that could mutate the parent outside the borrow's own alias family — specifically any call or store whose arguments or target places name the parent, an ancestor of the parent, or any name outside the borrow's family (the borrow, its copies, and sub-borrows taken from it), as well as any call carrying an unreadable actual. Any later use of an ended borrow refuses as **`interior-borrow-ended`** (`Unknown`, never `Unsafe`).
3. **Mutations through the borrow itself:** Writing or calling through a borrow `b` counts as mutating the parent *through `b`*: `b` itself stays valid, while every **sibling** borrow previously taken from the same parent is ended.
4. **Lazily created children:** A `get_or_create_*` call that allocates a child and links it into `parent` leaves ownership with `parent` (caller owes nothing and may not release it), and the call itself counts as mutating `parent` (ending earlier borrows of `parent`).
5. **Independent nullness and inference forwarding:** Result nullness follows `n` (`C04.3.1` rule 4), and W4 derived-result inference (`C04.3.2`) preserves `extent: Borrowed` whenever any exit returns a `Borrowed` pointer so wrappers never weaken mutation invalidation to plain `Interior`.
6. **Arena distinction:** Interim arena allocations (`cfl_arena_calloc` under `W47-ARENA-INTERIOR-READING`) intentionally keep `resource_return_interior(arena, nullable)` rather than `Borrowed`, because allocating a second block from an arena does not invalidate previously returned arena blocks; dedicated pool kinds remain tracked under `W47-ARENA-POOL-RESULT-MODE`.

###### Structural body validation (gateway rule 13a)

In a translation unit defining the C body of a `resource_return_borrowed_from` function, the declaration stands **only** if the body validates structurally: the base formal `p` is never reassigned and never has its whole address taken, and every `return` yields either `NULL`, a pointer reached from `p` via member reads (across casts, parentheses, offsets, and locals assigned solely from such reads), or a local assigned from a call whose result is stored into a member of a `p`-reached object before `return` (lazy child creation). Otherwise, the declaration is withdrawn in that unit.

> **Implementation & Provenance.** Gateway rules 12y, 13a; Lean model: `QCore.InteriorBorrow`. Card: `SLOT-MOVE-AND-INTERIOR-BORROW` (decision 2026-10-05).

<a id="c0435-checked-same-item-detach-results"></a>
##### C04.3.5. Checked same-item detach results (`resource_return_removed`)

Operations such as `cJSON_DetachItemViaPointer(parent, item)` unlink a specific child `item` already contained inside `parent` and return **that exact `item` pointer** to the caller as directly held custody. Neither `resource_return_owned` (which invents a brand-new allocation identity) nor `resource_removed` with `resource_return_none` (which leaves the returned pointer unbound) models this accurately.

```qv
extern fn cJSON_DetachItemViaPointer(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_premise(detach_list_representation, FORMULA);
    ensures normal resource_owned(parent, nonnull);
    ensures normal resource_removed(item, parent, nullable);
    ensures normal resource_return_removed(item, nullable);
};
```

###### Placement and caller preconditions

- **Exact clause pairing:** `resource_return_removed(item, n)` is admitted **only** alongside `resource_removed(item, parent, n)` (with identical nullability `n`) and `resource_owned(parent, nonnull)`.
- **Signature restrictions:** `parent`, `item`, and the result `MUST` all be `ptr<mut, u8>`, with `parent` and `item` distinct parameters; every other parameter must be `resource_borrowed`, and no container, counted-user, retaining, callback, variadic, or no-return clauses may appear.
- **Strict caller-state gate:** Before applying the detach or rebinding the call's destination variable, the caller `MUST` establish:
  1. A single live, settled `parent` identity (a `NULL` or nullable `parent` conservatively refuses even if the C implementation checks `!parent`); and
  2. Either a proved `NULL` `item` (when `n` is `nullable`, transferring nothing) **or** a single settled contained `item` identity whose **sole** containing owner is `parent`.
  - Borrowed-from provenance, non-`NULL` checks, list invariants, branch-joined owners, or undecided containment **never** suffice to prove membership. On success, `item`'s existing identity becomes the call's result and transitions from parent-contained to directly held — creating no fresh site, disturbing no other child of `parent`, and splitting no existing alias into a second obligation.

###### Physical unlink body check and `detach_list_representation` premise

- **Bounded physical template validation:** In any translation unit defining the function's C body, a physical AST adapter verifies the complete doubly-linked list unlink statement/control/write sequence (up to parameter renaming and zero-constant spellings) over an unambiguous complete `struct` whose `child`, `next`, and `prev` fields are same-record pointers: updating predecessor/successor links, updating `parent->child` and circular tail `first->prev`, clearing `item->next` and `item->prev`, and returning `item`. Any missing, extra, or misdirected write or unreadable record layout **withdraws** the declaration before callers or summary inference consult it.
- **Required named representation premise:** Validation also requires the exact named premise `detach_list_representation` carrying the canonical [C04.18.9](#c04189-symbolic-premise-statements-formula-version-1) formula (`legacy_detach::REPRESENTATION_STATEMENT`), which states the physical-to-abstract representation invariant at entry: `parent`'s finite acyclic `child`/`next` chain equals `parent`'s exact owned children, `first->prev` designates the list tail (itself for a singleton) while every subsequent node's `prev` designates its predecessor, `last->next == NULL`, all child identities are live and pairwise distinct from one another and `parent`, and no concurrent thread mutates those links during the call.
- **Epistemic boundary:** This is one of the narrowly specified semantic readers of a named premise: checking the C unlink template and requiring the premise **does not** prove `detach_list_representation` or make the C body unconditionally `Safe`; the premise remains an uncovered `DeclaredAssumption` on every dependent verdict and certificate (and is re-verified from retained evidence during independent legacy certificate replay). Consumers restricted to fresh or resource-free results (including initial whole-return/passed-result routes and the `.qr` FFI bridge) refuse `resource_return_removed` rather than coercing it.

> **Implementation & Provenance.** Gateway `legacy_detach`; Lean models: `QCore.CjsonRepresentationPremises`, `QCore.CjsonTypedAtoms`. Card: `CJSON-DETACH-SAME-ITEM` (decision 2026-10-18).

---

<a id="c044-unsupported-semantics-and-false-declarations"></a>
#### C04.4. Unsupported semantics and false declarations

Resource-contract v1 intentionally excludes constructs that cannot be represented faithfully by unconditional (or return-code-partitioned) interface predicates:

1. **Normal C returns vs. typed exceptions:** Apart from `resource_no_return()` ([C04.3.3](#c0433-the-no-return-exit)), v1 models only normal C function returns (`ensures normal`). A C function returning `NULL` or `-1` has returned normally in C, not unwound via a Quirin `error(E)` path. Error-conditional custody is expressible only through modes that explicitly encode it: `nullable` acquisition, `sentinel` descriptor acquisition (`C04.6`), `resource_return_reallocated` (`C04.3`), in-place status witnesses (`C04.6.1`), descriptor cells (`C04.6.2`), return-code dispositions (`C04.13`), or callback return-code hand-offs (`C04.19.5`).
2. **Excluded v1 constructs:** `ensures any`, `ensures error(E)`, quantitative axes other than counted users (`C04.7`), general memory effects and access shares, general concurrency synchronization (note that `C04.9`–`C04.11` are call-site refusal preconditions, not general synchronization logics), non-null pointer sentinels (`MAP_FAILED`), and integer failure sentinels other than `-1` on `i32` descriptors (`C04.6`) are rejected.
3. **No value-dependent parameter custody modes:** A parameter's custody mode is never conditioned on another argument's runtime value. `requires resource_literal_argument(p, LITERAL)` ([C04.5](#c045-the-literal-argument-premise)) restricts *which call sites the entire declaration applies to* without altering any mode, and `when X OP K` ([C04.14.1](#c04141-guarded-retentions)) guards only a `resource_retained_by` clause (or member-path state in `C04.19.6`), never a custody mode.
4. **Syntactic validity cannot detect false trusted declarations:** While total-or-reject loading catches unsupported syntax, the loader cannot detect when an external declaration lies about an unanalyzed C library function. For instance, declaring a `mmap`-like routine with `resource_return_owned(nullable)` would silently treat `MAP_FAILED` (`(void *)-1`) as a valid non-null allocation — an invalid trusted assumption that violates conformance. Broader profiles must model non-null sentinels, generations, and failure residuals explicitly rather than reinterpreting `nullable`.

---

<a id="c045-the-literal-argument-premise"></a>
#### C04.5. The literal-argument premise

Some C functions change their ownership contract completely depending on a constant flag or size argument — for example, `flb_hash_table_add(..., val, val_size)` takes ownership or stores the caller's pointer when `val_size == 0`, whereas for `val_size > 0` it allocates a fresh copy and borrows `val`. Writing an unconditional contract for one arm would unsoundly apply that arm's modes to call sites taking the other arm.

To bound a declaration strictly to call sites passing a specific compile-time constant, `resource_contract_v1` admits:

```qv
requires resource_literal_argument(p, LITERAL);
```

##### Syntax, canonical literals, and parameter types

- **Parameter binding:** `p` `MUST` name a parameter of the declaration; at most one `resource_literal_argument` may name a given `p`, though a declaration `MAY` carry premises on multiple distinct parameters (all of which must hold simultaneously at a call site). Any typo in `p`, duplicate on `p`, or malformed literal rejects the entire module.
- **Canonical decimal spelling:** `LITERAL` `MUST` be a canonical base-10 integer with an optional leading `-` and **no** integer type suffix, radix prefix (`0x`, `0b`), leading zero (`00`, `010` octal), or `-0`. Because every mathematical integer has exactly one valid `.qv` spelling, the loader stores the exact expected C token vector (`0` → `["0"]`, `-1` → `["-", "1"]`) so consumers compare tokens structurally without re-parsing numbers.
- **Admitted on both scalars and pointers:** `p` `MAY` have any type in the `C04.1` vocabulary:
  - On a scalar parameter, `LITERAL` selects a constant integer/flag arm.
  - On a pointer parameter (`ptr<...>` or `fnptr`), `LITERAL` `0` restricts the declaration to call sites passing a **null pointer constant** (C11 6.3.2.3p3), such as `pthread_key_create(&k, NULL)` (no destructor callback) or `PEM_read_bio_X509(bp, NULL, cb, u)` (fresh allocation arm rather than object-reuse arm).
  - Even an unsatisfiable literal premise (such as `1` on a pointer parameter) only withdraws all call sites, failing closed.

##### Whole-declaration call-site withdrawal and callback interactions

1. **Strict narrowing (withdrawal on mismatch):** `resource_literal_argument(p, LITERAL)` asserts that **the declaration applies only at call sites whose actual argument for `p` is structurally `LITERAL`**. At every other call site (including one missing `p`), a consumer `MUST` treat the callee as **uncontracted** (`Unknown`). The premise never widens coverage, never changes parameter/result modes, and proves nothing about the callee's implementation.
2. **Interaction with callback clauses (gateway rule 57):**
   - *At direct calls of synchronous or opaque-slot invokers:* A literal-argument premise `MAY` stand alongside synchronous callback invocations (`resource_callback_invoked`, `resource_callback_invoked_or_default`) and opaque callback-slot invocations (`resource_callback_slot_invoked`, [C04.19.3](#c0419-callback-contracts)), because neither retains state after the call returns. Placing it beside callback registrations, table dispatches, registries, plain member invocations, or other retaining clauses (`C04.14`) rejects the module. Because callback declarations are withheld from the ordinary function map, consumers `MUST` enforce literal premises on withheld declarations *before* any callback resolution: if the literal premise is unmet, the callee is uncontracted and its callbacks are not evaluated there.
   - *When the declared function is passed as a callback candidate:* A literal premise bounds direct call sites, whereas a callback candidate is invoked indirectly with arguments chosen inside the invoker. Consequently, any function whose declaration carries `resource_literal_argument` `MUST` be treated as having **no contract in force** whenever it appears in a callback candidate set (and all candidates are treated as uncontracted if the premise table is unreadable).
3. **Text rederivation and retired comment directives:** Like variadic tails, `literal_argument_premises` are rederived from retained module texts (and reported as `literal_argument` premises in [C04.18](#c0418-named-premises) / [C06.5](#c065-per-verdict-contract-dependencies)). If rederivation fails over source texts containing the clause, consumers `MUST` refuse (`crates/qcore-c-gateway/src/legacy_literal_premise.rs`) rather than proceeding unrestricted. The historical `// qcore-premise` machine-parsed comment directive is **retired**, and any module text still containing its introducer is rejected by the loader.

<a id="c0451-which-call-site-spellings-are-the-declared-constant"></a>
##### C04.5.1. Call-site constant spellings

Consumers `MUST` decide whether a C call-site argument matches `LITERAL` **purely structurally on preprocessed tokens, without arithmetic evaluation**.

###### Admitted structural erasures

1. **Matched outer parentheses:** `(0)` and `((0))` erase to `0` (unmatched internal parentheses such as `(0) + (0)` or `(long) 0` are not enclosing groups and do not erase).
2. **C11 6.4.4.1 integer type suffixes:** Trailing `u`, `U`, `l`, `L`, `ll`, `LL` combinations (`0L`, `0u`, `0ULL`) fix the C integer type without changing the decimal digit sequence and are erased.
3. **C11 6.3.2.3p3 `(void *) 0` null pointer constant (pointer parameters with `LITERAL == 0` only):** When `p` is pointer-typed (`ptr<...>` or `fnptr`) and `LITERAL` is `0`, consumers `MUST` additionally admit matched outer parentheses around an exact `(void *)` token cast applied to an operand that reduces to `0` under rules 1–2 — covering `((void *) 0)` (standard C `NULL` after preprocessing), `(void *)0`, `(void *)(0)`, and `(void *) 0L`.
4. **Object-like macros:** Because C consumers inspect preprocessed tokens, `#define FLAGS 0` has already expanded to `0` before structural comparison.

###### Mandatory structural refusals

Every other spelling requires capabilities deliberately excluded from structural token matching and `MUST` be refused (withdrawing the declaration at that call site):

| Refused C call-site actual | Why structural matching refuses it |
|---|---|
| `0 + 0`, `1 - 1`, `-(0)`, `-0` (against `0`) | Would require constant-expression arithmetic evaluation. |
| `0x0`, `0b0`, `00` | Alternate radices; stripping leading zeros would misread octal `010` (8) as `10`. |
| `(long) 0`, `(char *) 0`, `(const void *) 0`, `(void **) 0`, `(void (*)(void)) 0` | Casts other than exact `(void *)` on a `0` pointer premise (note `(const void *) 0` is a null pointer in C, not a C11 6.3.2.3p3 *null pointer constant*); all casts on scalar premises or non-zero literals are refused. |
| Enumerator identifiers or `const int` variables | Would require C symbol/value resolution rather than token identity. |

> **Implementation & Provenance.** Gateway `legacy_literal_premise.rs`, rule 57; Lean models: `QCore.NullPointerConstantPremise`, `QCore.CallbackLiteralPremise`, `QCore.OpensslBioReaders`, `QCore.CallbackCandidatePremise`. Cards: `HASH-CONTRACTS`, `HASH-CLAUSE`, `CURRENT-CONTEXT-NEW-TRIAGE` (D3, decision 2026-11-16), `OPENSSL-PEM-CALLBACK-ARM` (decision 2026-11-18), `OPENSSL-BIO-READER-INVOCATIONS`, `CALLBACK-CANDIDATE-LITERAL-PREMISE`.

---

<a id="c0412-ffi-function-definitions-quant-c_ffi"></a>
#### C04.12. FFI function definitions: `[| quant #c_ffi |]`

While C-audit contracts erase C record pointers to `ptr<mut/const, u8>`, Quirin `.qr` programs calling C routines across the checked FFI boundary sometimes pass or receive concrete C `struct` values or borrowed `const struct` pointers as pure data. To keep every foreign struct interface auditable inside `.qv` files without polluting `.qr` sources or widening the C custody map, an FFI declaration is marked on the preceding line with `[| quant #c_ffi |]`:

```qv
[| quant #c_ffi |]
extern fn qr_header_total(header: ptr<const, c"struct qr_header">) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(header);
    ensures normal resource_return_none();
};
```

##### Lexical placement, closed FFI vocabulary, and withholding

1. **Contract-only attribute (`Q033`):** `[| quant #c_ffi |]` consists of five existing lexer tokens claimed only immediately before `extern fn` in a `.qv` contract module. Because it precedes `extern fn`, readers that reconstruct unmarked declarations remain untouched. Writing `[| quant #c_ffi |]` in a `.qr` executable or library module is refused with **`Q033`**.
2. **Closed type grammar (total-or-reject):**
   - *Parameters:* Any `§G02t` scalar (`bool`, fixed-width integers, `f32`, `f64`), `c"struct TAG"` (by-value struct copy), or `ptr<const, c"struct TAG">` (read-only borrowed struct pointer).
   - *Result:* `unit`, any `§G02t` scalar, or `c"struct TAG"` (by-value struct copy returned to caller).
   - *Strict rejections:* `c"..."` must be the identifier `c` adjacent (no whitespace) to a string literal matching `struct TAG` exactly — unions, enums, typedef names, `ptr<mut, c"struct TAG">`, custody pointers (`ptr<mut/const, u8>`), `fnptr`, and `va_list` all reject the module. Conversely, any **unmarked** declaration mentioning `c"..."` rejects the module.
3. **Pure-data clause restriction:** The contract block `MUST` contain **only** `requires resource_contract_v1;`, `ensures normal resource_borrowed(p);` for each parameter, and `ensures normal resource_return_none();`. Structs cross this boundary strictly as custody-free data; review, revision, checked-specification, premise, or effect clauses reject the module.
4. **Withheld from the custody map:** Marked declarations are validated and **withheld** from `ResourceContracts`'s ordinary function map, so every non-FFI consumer treats the callee as uncontracted. Only the `source.ffi.v1` route rederives them (`contract_loader::ffi_declarations`); calling a struct-bearing `extern fn` from `.qr` without `[| quant #c_ffi |]` refuses with **`Q033`**.

##### Native `.qrlib` manifest linkage

A `.qv` contract states *what* a foreign function is assumed to do, whereas a generated **`.qrlib` native ABI manifest** ([implemented grammars `§G02d.2.1`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)) specifies *which native C symbol* links to it:

- The `.qrlib` generator emits one `contract fn NAME(C_TYPES) -> C_TYPE from MODULE export ...;` entry for every declaration (marked or unmarked) in its input `.qv` modules, projecting `§G02t` scalars to `<stdint.h>`/`float`/`double`, `c"struct TAG"` to `struct TAG`, and `ptr<const, c"struct TAG">` to `const struct TAG *`.
- `source.ffi.v1` admits a `.qr` call only when the file's `extern library NAME;` resolves the callee to a `.qrlib` entry carrying the **exact** contract module selection key and projected C signature ([§L14.8](QCORE_SPEC_r0_3.md)).
- Linkage proves symbol and C-prototype binding only, never that the C implementation satisfies the contract; every certificate for a `.qr` file invoking `[| quant #c_ffi |]` lists the declaration under the explicit unsafe-FFI assumption ([§L14.6, §L20](QCORE_SPEC_r0_3.md)).

> **Implementation & Provenance.** FFI rederivation: `contract_loader::ffi_declarations`; diagnostic `Q033`; grammar `§G02d.2.1`. Cards: `W48-QR-C-STRUCT-IMPORT` (decision 2026-09-30), `W48-QR-FLOATING-POINT`, `W48-QRLIB-LINKAGE`.

---

<a id="c0413-return-code-dispositions"></a>
#### C04.13. Return-code dispositions

Many C functions decide parameter custody or out-slot initialization according to their integer return code — for instance, a queue insertion `take(p)` that consumes `p` on `0` and leaves `p` with the caller on nonzero error, or `getaddrinfo` which populates `*res` if and only if it returns `0`. `resource_on_return` models return-code-partitioned postconditions directly:

```qv
extern fn take(p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));
    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));
    ensures normal resource_return_none();
};
```

##### Syntax and total-or-reject exit-class invariants

`ensures normal resource_on_return(OP, VALUE, PREDICATE);` asserts that `PREDICATE` holds on every normal return whose integer value `v` satisfies `v OP VALUE`:

- **Operators and constants:** `OP` is one of `eq`, `ne`, `lt`, `le`, `gt`, `ge`. `VALUE` uses the canonical decimal spelling of [C04.5](#c045-the-literal-argument-premise) and `MUST` fit in a C `int`.
- **Inner predicates:** `PREDICATE` is a single parameter mode: `resource_borrowed(p)` or `resource_{owned,consumed,released,out}(p, NULLABILITY)`.
- **Exit-class partition and agreement rules:** The distinct `(OP, VALUE)` pairs form the declaration's **exit classes**, which `MUST` satisfy all seven invariants or reject the module:
  1. *Exact integer partition:* The exit classes are pairwise disjoint and their union covers all integers (verified exactly at `VALUE - 1`, `VALUE`, and `VALUE + 1` for every class).
  2. *Complete per-class assignment:* Every parameter is either unconditional (one plain mode clause) or conditional (one `resource_on_return` clause in **every** exit class).
  3. *Uniform entry custody:* For any conditional parameter, either **every** class requires live entry custody with the exact same nullability (`owned`, `consumed`, `released`), or **no** class requires entry custody (`borrowed`, `out`).
  4. *Signed integer status return:* The return type is a signed integer (`i8`, `i16`, `i32`, `i64`, or `isize`) paired with unconditional `resource_return_none()`.
  5. *Non-degenerate per-class validity:* Combining each exit class with the unconditional clauses must pass every check of an ordinary `C04` declaration (including resource kinds). If all exit classes produce the exact same contract, the module is rejected (such a contract must be written without `resource_on_return`).
  6. *Disjoint from in-place kinds:* Never combined with an in-place kind ([C04.6.1](#c0461-in-place-kinds)), whose status return is already its own validity witness.
  7. *Sibling restrictions:* Admitted only alongside plain parameter/result modes, resource kinds, and revision markers; review markers, `resource_body_checked`, literal-argument premises, variadic tails, and counted-user, container, loop-owner, thread-synchronization, or lifecycle clauses reject the module.

##### Conditional out slots, call-site branching, body checks, and forwarding

1. **Conditional out slots:** Pairing `resource_out(out, ...)` in a success class with `resource_borrowed(out)` in a failure class means the failure path leaves `*out` **unwritten**. On the unwritten branch, the consumer passes the slot actual as data and afterwards treats the slot variable `NAME` as holding an untracked value; any actual other than `&NAME` refuses (except under [C04.13.1](#c04131-an-out-slot-handed-a-member-of-a-caller-supplied-object)).
2. **Withholding and branch-tested call semantics (gateway rule 11r):**
   - Dispositions load into `RelationalContract` (`crates/qcore-schemas/src/relational_post.rs`) and are **never** inserted into `ResourceContracts`'s plain function map, so unaware consumers see an uncontracted callee.
   - Route G applies an exit class **only** when an `if` condition tests the call directly against an integer constant (or tests a variable assigned in the immediately preceding statement). Each branch edge receives the unique disposition agreed upon by all exit classes reachable on that edge; if reachable classes on an edge disagree (or none is reachable), the edge refuses.
   - If the caller discards or fails to test the return value in that form, the call receives the **join** across all exit classes — which, because admitted classes always differ at some parameter, refuses as **`relational-untested`** (`Unknown`).
3. **Body validation and withdrawal:** In a translation unit defining the function's C body, an ordinary body summary must match every exit class, and an exit-specific summary (rule 11r) must match, for each returned integer literal, every exit class containing that literal. Any contradiction withdraws the declaration (`relational-withdrawn`); if the body produces no summary, the declaration stands as a `Declared` assumption.
4. **Return-code forwarding (gateway rule 12s):** When a C wrapper's every `return` is either a plain integer literal or `return f(ARGS);` of a single callee with an active disposition — and both functions declare the exact same return type so no C integer conversion occurs — the wrapper inherits `f`'s exit classes. Each class is validated by restricting the wrapper's CFG to exits in that class, and a function with its own active disposition is `Safe` only when every restricted body validates its class and the joint body emits no diagnostic.

<a id="c04131-an-out-slot-handed-a-member-of-a-caller-supplied-object"></a>
##### C04.13.1. Member out slots

Without adding new `.qv` syntax, gateway rule 72 specifies when a caller may pass the address of a struct member `&R->F...` (such as `return cmt_mpack_consume_string_tag(reader, &opts->ns);`) instead of a local `&NAME` to a `resource_out` parameter (either plain or under a `C04.13` disposition):

```qv
extern fn cmt_mpack_consume_string_tag(
    reader: ptr<mut, u8>,
    output_buffer: ptr<mut, ptr<mut, u8>>
) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_on_return(eq, 0, resource_out(output_buffer, nonnull));
    ensures normal resource_on_return(ne, 0, resource_borrowed(output_buffer));
    ensures normal resource_return_none();
};
```

A consumer `MAY` admit `&R->F...` at an out slot **only** when all four conditions hold (and `MUST` retain its refusal otherwise):

1. **Single-out, otherwise-neutral callee:** The callee declares **exactly one** out slot (either unconditional `resource_out`, or a `C04.13` disposition where every class is either `resource_out` or unwritten `resource_borrowed` at that slot and identical everywhere else), and substituting `resource_borrowed` for that slot yields the completely neutral contract (all other formals `resource_borrowed`, `resource_return_none()`, zero kind or effect clauses). Because the destination belongs to the caller's object rather than the current stack frame, neither success (fresh custody stored into caller's struct) nor failure (slot unwritten) leaves a frame obligation, so no return-code branch is needed in this frame.
2. **Caller-supplied root (`P-72-CALLER-OBJECT`):** The slot actual is `&R->F...` whose first step dereferences `R`, and `R` is caller-supplied — an unrebound formal, or a local variable whose every write stores such a root, an object-pointer cast of one, or a member read through one. Frame-allocated structs, array subscripts, pointer casts on the slot address, or roots written from other sources refuse.
3. **Untracked owner and overwrite safety (`P-CG11-CALLER-SLOT`):** Along the reaching flow, the struct holding `F` is untracked (no reaching path bound it), `R` satisfies rule 10v's owner premise, and no binding rooted at `R` holds a tracked target (so overwriting `*(&R->F)` cannot leak an obligation tracked by this frame). Any frame-tracked owner refuses (only `O04-B08` field links may overwrite those).
4. **No derived summary published:** The call treats the slot as data and transfers the delivered custody to the caller's object; critically, **any C body containing such a call publishes no W4 derived summary**, preventing outer callers from misreading `R` as a side-effect-free `Borrowed` parameter.

> **Implementation & Provenance.** `crates/qcore-schemas/src/relational_post.rs`; gateway rules 11r, 12s, 72. Lean models: `QCore.RelationalPost`, `QCore.RelationalForwarding`, `QCore.OwnedOutMemberSlot`. Cards: `CG23-RELATIONAL-POSTCONDITIONS`, `CALLEE-RETENTION-CLASS1`, `CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT`.

<a id="c04132-packing-a-return-code-disposition"></a>
##### C04.13.2. Packing a return-code disposition

At a typed Quirin library call boundary, an opt-in adapter `MAY` pack a `C04.13` signed `i32` return-code disposition into a movable, generation-indexed [quant variant](QCORE_SPEC_r0_3.md#l091-quant-variants) while keeping the external C symbol's scalar `int` ABI unchanged. The custody/empty subset is implemented on `source.quant-variant.v1` ([G02d.4](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d4-delayed-custody-results-sourcequant-variantv1)) paired with a separately selected `C04.13` contract; protocol/effect `using #effectName` adapters remain **Proposed**.

```qv
quant variant TakeResult(p) {
    Taken carries none;
    Retained(code: i32) carries custody(p);
}

extern fn take(p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_on_return(eq, 0, resource_consumed(p, nonnull));
    ensures normal resource_on_return(ne, 0, resource_owned(p, nonnull));
    ensures normal resource_return_none();
    quant variant on_return(eq, 0) TakeResult::Taken;
    quant variant on_return(ne, 0) TakeResult::Retained;
};
```

1. **Subject binding & scalar ABI preservation:** Accepts an ABI-checked 32-bit signed `i32` C return type, binds the variant's subject parameter (`p`) to a unique `extern fn` formal and its caller generation $\rho$, and exposes typed return `quant variant TakeResult(p)` (populating optional `code: i32` with the exact returned integer; consumers lacking this adapter reject mapped declarations).
2. **Partition & underlying contract agreement:** Requires one `quant variant on_return(...)` clause per declared variant tag over pairwise-disjoint, exhaustive canonical `C04.13` comparison classes matching the underlying relational keys (never rehabilitating a body-refuted/withdrawn contract; protocol-only adapters over plain contracts may declare their own total partition).
3. **Single-duty post-state derivation:** Applies the underlying `C04.13` disposition once and partitions its post-state per [§L09.1](QCORE_SPEC_r0_3.md#l091-quant-variants): `resource_consumed` yields an empty `Taken` package, whereas `resource_owned` moves the existing caller release duty into `Retained` without minting or discharging a second duty.
4. **Proposed protocol evidence (`using #effectName`):** Arms carrying `protocol(...)` must cite a unique [G01e](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g01e-typed-conditional-own-operation-effects) `#effectName` and prove entry/exit binding, guard, transition, and frame preservation for generation $\rho$ (unverified `G01e` effects remain unsupported).
5. **Single-call boundary lowering:** Evaluates arguments and calls the C ABI symbol **exactly once**, classifying its saved `i32` return into the internal tag and optional `code` payload without introducing ABI-visible C structs or altering existing-C `C04.13` immediate-test rules.

> **Implementation & Provenance.** `compiler/tests/quant_variant.rs`, `proof/QCore/QuantVariant.lean`; grammar [G02d.4](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d4-delayed-custody-results-sourcequant-variantv1).

---

<a id="c0414-retained-pointers"></a>
#### C04.14. Retained pointers

C APIs that interact with external buffers or context pointers fall into three distinct architectural shapes:

- **Shape (a) — Custody-taking inserts:** On success, ownership transfers to the container; modeled via `C04.13` return-code dispositions.
- **Shape (b) — Retained non-owning pointers (`C04.14`):** The caller **retains ownership** of `P`'s allocation, while a holder `H` stores an unowned pointer into `P`. Releasing `P` while `H` remains live and unreset leaves `H` holding a dangling pointer.
- **Shape (c) — Callbacks invoked over retained state:** Modeled via callback contracts ([C04.19](#c0419-callback-contracts)).

`resource_retained_by` and `resource_holder_reset` specify Shape (b):

```qv
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

##### Grammar, withholding, and outlives obligation

- **Placement (`G05`, total-or-reject):** `P` and any parameter holder `H` are distinct depth-1 `resource_borrowed` pointers with no kind (`H` may also be the keyword `global`, `result` under `C04.14.2`, or a holder path `FORMAL.FIELD...` under `C04.14.3`; a parameter named `global` rejects the module). Each `P` is retained at most once and each `H` reset at most once; the clauses stand beside plain parameter/result modes and revision markers (plus `C04.16` derived out slots and `C04.19` callbacks where admitted).
- **Withheld from the function map (`ResourceContracts::retaining`):** A retained pointer is **never** reduced to a plain `resource_borrowed` call and forgotten. The loader validates the declaration in full and withholds it from the ordinary function map so any consumer unaware of retention treats the callee as uncontracted (`Unknown`).
- **Outlives requirement and verdict standing:** The caller keeps custody of `P`, and a retention-aware consumer `MUST` prove that `P`'s object outlives `H` (or that `H` is destroyed, reset, or has its registration removed before `P` is released). Otherwise the verdict is `Unknown` (or `Pending` if gated solely on a human-resolvable hold), **never `Safe`**. Releasing `P` while held refuses as **`retention-released-while-held`** (`Unknown`, **never `Unsafe`**, because the caller alone does not prove `H` subsequently dereferences the dangling pointer).

##### Route-G outlives decision table (gateway rule 12t)

Route G discharges `resource_retained_by(P, H)` in exactly five cases, refusing all others with **`retention-undecided`**:

| Retained actual `P` / Holder `H` | Decision and recorded lifetime obligation |
|---|---|
| **Static storage at `P`** (string literal, or C11 6.4.2.2p1 `__func__` / `__FUNCTION__` / `__PRETTY_FUNCTION__` under `P-RETAINED-PREDEFINED`) | Discharged immediately for **any** holder (including `global`), because static storage outlives the entire program. |
| **Null pointer constant at `P`** (`0`, `NULL`, `nullptr`, or `((void *) 0)` after stripping matched parens and `(T *)` pointer casts) | Designates no object; discharged immediately with zero retention record for **any** holder (`global` and caller-supplied holders included). Variables or calls that *may* be null are **not** stripped here. |
| **Stack-frame holder `H = &L`** (`L` an automatic non-pointer, non-array local named only as whole `&L` actuals of retaining/resetting callees) | `L` dies when the current function returns: caller-supplied unrebound formals (with no custody in this body) outlive `L` immediately; tracked identities are recorded on `L` until `resource_holder_reset(&L)` or function return (releasing earlier refuses `retention-released-while-held`). |
| **Tracked heap holder `H`** | Retention is recorded on `H`'s identity until `H` is released or `resource_holder_reset(H)` runs. If `H` still holds retentions at function exit (returned, stored, or leaked), `H` may outlive the frame and the exit refuses (`retention-undecided`). |
| **Program-lifetime holder `H = global`** | Admits **only** static storage and null pointer constants; every other `P` is `retention-undecided`. |

Caller-supplied holders (formals) and unnameable actuals refuse (`retention-undecided`); retentions join by set union across control-flow merges and are **not** forwarded into W4 summaries. Retaining declarations obey `C04.1` ABI and `C06.2` pin gates (unvalidated against C bodies in general under `RETAINED-BY-BODY-CHECK`, save for the guard check of `C04.14.1`).

<a id="c04141-guarded-retentions"></a>
##### C04.14.1. Guarded retentions

When a function retains `P` only for specific integer or enum-projected selector values, `resource_retained_by` carries an integer comparison guard (gateway rule 12w):

```qv
ensures normal resource_retained_by(P, H) when X OP K;
```

- **Grammar (`G05`, total-or-reject):** `X` must be an integer parameter (`i8`–`i64`, `isize`, `u8`–`u64`, `usize`, including `C04.1` enum projections; never `bool`, float, pointer, `fnptr`, or `va_list`) distinct from `P` and `H`. `OP` is one of `==`, `<`, `>`, `<=`, `>=` (`!=`, `&&`, `||`, and multiple guards reject). `K` is a canonical decimal literal (`C04.5` spelling — never an enum identifier; cite the C enumerator in a comment) within `X`'s range that **nontrivially partitions** `X`'s type (at least one value in `X`'s type satisfies `X OP K` and at least one falsifies it; e.g. `x >= 0` on `u32` rejects).
- **Decided vs. cautious evaluation:**
  - *Decided actual:* When the call-site actual for `X` is a structural decimal constant (`C04.5.1` including unary `-`) or an unshadowed file-scope C enumerator whose integer value Clang states for the unit, and lies within `X`'s range, the retention applies if and only if `actual OP K` holds.
  - *Undecided actual:* Every other actual (variables, arithmetic, casts, out-of-range C conversions, shadowed/unreadable constants) takes the **cautious union** — treating **every** guarded retention clause as active, never choosing the favorable arm.
- **In-unit body validation:** In a unit defining the callee's C body, a guard stands only if `X` is never modified, the body contains no `goto`, and every mention of `P` outside a guard region (the braced `then`-block of `if (X OP K)` or a non-fallthrough `switch (X)` `case` whose constant satisfies `OP K`) is solely a null/equality test or dereference (never under `&`). Failing this check strips the guard in that unit, reverting to cautious unconditional retention. *(No shipped corpus declaration currently uses a guard; see `C04.1` provenance on `rd_kafka_confval_set_type`.)*

<a id="c04142-the-result-as-holder"></a>
##### C04.14.2. Result as holder

Constructors frequently allocate a fresh object or return a counted reference that retains an unowned pointer into one of its arguments (gateway rule 13e):

```qv
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

- **Grammar:** Writing `result` as the holder requires `resource_return_owned(nonnull)` (either a fresh heap object, or a returned reference via `resource_return_kind(reference)` under [C04.17](#c0417-reference-counts)); any parameter named `result` rejects the module.
- **Lifecycle of a `result` holder:** The call's destination must be a plain variable binding a single live identity (otherwise `retention-undecided`). Static literals and predefined `__func__`-family arrays (`P-RETAINED-PREDEFINED`) discharge immediately. Tracked or caller-formal arguments are recorded on the result's identity until either:
  1. A heap result holder is released; or
  2. The function drops the **last reference it holds on an acquired reference holder** (premise `P-RESULT-HOLDER-LAST-DROP` — dropping a borrowed reference family kept from a caller's object ends nothing, because the caller's own reference keeps the holder alive).
- If the result holder still holds any retention at function exit (because it was returned, stored, or leaked), the exit refuses (`retention-undecided`, **never `Safe`**); releasing tracked `P` while held refuses (`retention-released-while-held`).

<a id="c04143-holder-paths-and-the-removal-of-one-registration"></a>
##### C04.14.3. Holder paths and removal of one registration

Event-loop and collector APIs often retain an object (or an embedded event sub-object `&X->event`) inside a holder reached through a struct member path (`coll->evl`), and later unregister that single item without resetting the entire event loop:

```qv
extern fn start(coll: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(coll);
    ensures normal resource_borrowed(config);
    ensures normal resource_retained_by(coll, coll.evl);
    ensures normal resource_return_none();
};

extern fn mk_event_del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(event);
    quant live #loop: #holds(event) |-> #absent;
    ensures normal resource_return_none();
};
```

###### Grammar (`G05`, total-or-reject)

- **Holder paths:** A retention's holder `H` may be `FORMAL.FIELD[.FIELD]...`, where `FORMAL` is a depth-1 `resource_borrowed` unkinded parameter (which may be `P` itself, as in `coll.evl`), followed by one or more field identifiers. Position distinguishes `FORMAL.FIELD` (a retention holder rooted at a parameter; a struct tag there rejects) from `TAG.FIELD` (record member paths in `C04.6.2` and `C04.19`). Holder paths take no `C04.14.1` `when` guard.
- **Single-registration removal:** `quant live #H: #holds(P) |-> #absent;` requires distinct depth-1 `resource_borrowed` unkinded parameters `H` and `P` (any parameter named `holds` or `absent` rejects; at most one removal per `(H, P)` pair, never beside a retention of the same pair). Both clauses withhold the declaration from the function map, while their `resource_borrowed` positions still admit typed member reads, pointer reads, and `&X->f` sub-object actuals at the access gate (`RETAINING-CALL-MEMBER-ACTUAL`).

###### Path-holder stability and unmatched-removal poisoning

1. **Path holders (`P-HOLDER-PATH-STABLE`):** At a call site, a **path holder** denotes a member path beneath an unrebound formal of the caller accessed exclusively via member reads off that path (no write to, `&` of, or mutation of any prefix; the formal is never passed whole, copied, or rebound). Any other holder expression (locals, call results, ID lookups, mutable paths) refuses (`retention-undecided`). Retaining a caller formal, tracked identity, or sub-object interior `&X->f` on a path holder outlives the stack frame and therefore refuses at function exit unless ended before exit; releasing or transferring `X` while `&X->f` is held refuses just like releasing `X` directly.
2. **Exact removal and unmatched-removal poisoning (`RETENTION-UNMATCHED-REMOVAL-POISONS`):**
   - *Why unmatched removals must poison:* In Monkey's `_mk_event_del(loop, event)`, calling `mk_event_del` with a loop that does *not* actually hold `event` still clears `event`'s status field, causing a subsequent `mk_event_del` on the *real* holding loop to return early without unregistering `event` from epoll/kqueue!
   - *Poisoning rule:* Executing `quant live #H: #holds(P) |-> #absent;` removes the exact matching `(holder, object/member)` record **only if that record is not poisoned**, and simultaneously **poisons every other active retention record whose held object `P`'s actual might alias** (only two distinct tracked identities are guaranteed disjoint; caller formals, union members, or unkeyed actuals may alias and therefore poison matching candidates).
   - *Ending a poisoned record:* A poisoned record can **never** be cleared by a later removal; it ends **only** when its holder itself is destroyed (`mk_event_loop_destroy` on a tracked loop, tracked holder release, or `resource_holder_reset`; destroying a loop via a member actual remains refused at the custody access gate). Exiting with a poisoned record refuses as **`retention-removal-poisoned`**; releasing its object while held refuses as **`retention-released-while-held`**. Distinct holder paths are never assumed equal, and retentions are not forwarded into summaries (cross-function event-loop tracking lives in [C04.21](#c0421-local-conditional-prerequisites-and-registered-lifetimes)).

<a id="c04144-derived-retention-summaries"></a>
##### C04.14.4. Derived retention summaries

Without adding `.qv` syntax, gateway rule 70 derives path-retention summaries automatically for simple C setter bodies:

- **Body pattern:** When an uncontracted C body in the analyzed unit otherwise validates with all formals `resource_borrowed` and `resource_return_none()`, each store `F->m[.n...] = q` of an unrebound object-pointer formal `q` into a member of unrebound formal `F` (inside `*F`) is recorded as `resource_retained_by(q, F.m[.n...])`, alongside the set of **overwritten paths** of all admitted pointer stores into formal members (storing a `fnptr` records an overwritten path with zero retention).
- **In-unit application:** The derived retention summary is installed in that unit's retaining table (withheld from the plain function map; never overriding a declared contract; never exported across units). At each call site, rule 12t checks every derived retention **and** verifies that every overwritten path resolves to a stable path holder of the caller (`P-HOLDER-PATH-STABLE`, lifting rule 10v's `P-CG11-CALLER-SLOT` up one frame), refusing (`retention-undecided`, `Unknown`) otherwise. Callers do not forward retentions into their own summaries.

> **Implementation & Provenance.** Gateway rules 12t, 12w, 13e, 70. Lean models: `QCore.RetainedBy`, `QCore.RetentionNullConstant`, `QCore.RetentionGuard`, `QCore.ReferenceResult`, `QCore.RetentionPath`, `QCore.MemberStoreRetention`. Cards: `CALLEE-RETENTION-CLASS1`, `RETENTION-NULL-CONSTANT-ACTUAL`, `W43-ABI-ENUM-AND-CONST-PP`, `REFCOUNT-REFERENCE-RETURNING-DECLARATIONS`, `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION`, `RETENTION-UNMATCHED-REMOVAL-POISONS`, `CG10-MEMBER-STORE-RETAINED-BY`.

---

<a id="c0415-slot-moves"></a>
#### C04.15. Slot moves

Dynamic string and buffer builders such as `sds_cat_safe(&s, text)` and `cfl_sds_cat_safe(&s, text)` take a pointer slot `char **s`, potentially `realloc` the buffer `*s` points to, and write the new address back into `*s` (leaving the original buffer intact in `*s` if reallocation fails). `resource_slot_moved` is the custody counterpart of `CG24`'s `bounds_frame_moves`:

```qv
extern fn cat(slot: ptr<mut, ptr<mut, u8>>, text: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(slot);
    ensures normal resource_borrowed(text);
    ensures normal resource_slot_moved(slot, nullable);
    ensures normal resource_return_none();
};
```

##### Grammar, alias invalidation, and body checks

1. **Placement and withholding (`G05`):** `s` `MUST` be a `ptr<mut, ptr<mut, u8>>` parameter with `resource_borrowed(s)` and no kind, named by at most one slot clause, standing where a retention may (`C04.14`). `n` specifies whether `*s` may be `NULL` on entry (`nullable` means a `NULL` `*s` moves nothing). The declaration is **withheld from the function map** so unaware consumers never misread a moved slot as a benign borrow.
2. **Route-G slot-move decision (gateway rule 12x):**
   - *Local slot `&L` holding one simple live identity:* When `L` is bound to a single live tracked identity with no structural attachments (no field links, container membership, holder retentions, collections, pending replacements, or nullable markers), `L` **retains that identity and its single release obligation** (covering both moved success and unchanged failure in one obligation), while **every other variable aliased to that identity has its binding invalidated** — any subsequent read of an old alias refuses as **`slot-move-alias`** (`Unknown`, never `Unsafe`, since reallocation might not have moved the block).
   - *Definitely-`NULL` `L` under `nullable`, or pass-through formal slots:* A proved-`NULL` `L` under `nullable` is a no-op; forwarding a caller's formal slot used solely as whole slot-moving actuals succeeds with no local move.
   - *All other actuals or bindings:* Refuse (`slot-move-undecided`).
3. **Current scope boundaries and body validation (gateway rule 13a):**
   - Combining `resource_slot_moved` with `resource_on_return` is not yet admitted (`SLOT-MOVE-RETURN-CODE-SPLIT`; all currently measured slot movers return `void`). Struct-field reallocations through a depth-1 pointer (`flb_config_task_map_grow` reallocating `config->task_map`, `SLOT-MOVE-FIELD-SLOT`) and W4 inference of slot-move summaries (`SLOT-MOVE-SUMMARY-INFERENCE`) are excluded because caller aliases of struct fields cannot be enumerated locally.
   - In a unit defining the function's C body, the declaration stands only if every mention of `s` forwards `s` whole to a slot-moving parameter of a declared callee; bodies that write `*s` directly (such as `cfl_sds_cat_safe`) withdraw the declaration inside their defining unit while standing as pinned assumptions in caller units.

> **Implementation & Provenance.** Gateway rules 12x, 13a; Lean model: `QCore.SlotMove`. Card: `SLOT-MOVE-AND-INTERIOR-BORROW`.

---

<a id="c0416-derived-out-slots"></a>
#### C04.16. Derived out slots

Functions such as `strtol(nptr, &endptr, base)` and `rd_slice_reader0` deliver an unowned interior pointer derived from an input buffer `nptr` through an output pointer slot `endptr` (or leave `*endptr` untouched):

```qv
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

- **Grammar and withholding (`G05`):** `s` is `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>`, and `p` is a distinct depth-1 pointer, both `resource_borrowed` with no kind; `n` is the nullability of `s` itself (`strtol` accepts `endptr == NULL`). `const_dropped` is recorded when `p` is `ptr<const, u8>` and `s` is `ptr<mut, ptr<mut, u8>>` (exact C `strtol` signature!). Withheld from the function map; permitted alongside `resource_retained_by` (`cJSON_ParseWithLengthOpts`).
- **Gateway decision (rule 12z):**
  - Passing `&L` where `L` holds **no** tracked identity binds `L` to a possibly-`NULL` pointer derived from `p` at `Borrowed` extent (without a mutation-ending borrow record, since `*s` may retain its prior untracked value): `L` owns nothing (releasing `L` refuses), releasing `p`'s base object invalidates `L` (subsequent use refuses rather than accusing), and parent mutations do not end `L`.
  - Passing `&L` when `L` **already holds a tracked identity** refuses (`out-derived-undecided`) because overwriting `L` would leak that identity. Passing `NULL` under `nullable` or forwarding a caller's slot changes nothing locally; all other actuals refuse.
- **Body check (rule 13a):** In a defining unit, stands only when every mention of `s` is either `*s = EXPR;` (with `EXPR` reached from `p` per `C04.3.4`) or forwarding `s` whole to another `resource_out_derived` position; otherwise withdrawn in that unit.

> **Implementation & Provenance.** Gateway rules 12z, 13a; Lean model: `QCore.DerivedOutSlot`. Card: `SLOT-MOVE-AND-INTERIOR-BORROW` (absorbing `INTERIOR-OUT-SLOT-MODE`).

---

<a id="c0417-reference-counts"></a>
#### C04.17. Reference counts

Reference-counted C objects (`rd_kafka_toppar_keep0` / `rd_kafka_toppar_destroy0`, `rd_kafka_q_keep` / `rd_kafka_q_destroy0`, OpenSSL's `X509_up_ref` / `X509_free`) cannot be modeled soundly as custody-neutral `resource_borrowed` calls (which would let a missing drop read `Safe`), nor as ordinary per-pointer heap ownership (because `q = keep(p)` leaves `p` and `q` pointing to **the same object** with `+1` owed reference, and `keep(p);` as an expression statement still increments the reference count!).

Resource-contract v1 models reference counting through the pointer-carried **`reference`** resource kind ([C04.6](#c046-resource-kinds-and-integer-carried-identities)):

```qv
extern fn keep(p: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_kept(p);
    ensures normal resource_return_alias(p, nonnull);
};

extern fn drop_or_nothing(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_dropped(p, nullable);
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

##### Placement, kind isolation, and withholding

1. **Three admitted surface forms (`G05`, total-or-reject):**
   - **Keep (`+1`):** `ensures normal resource_reference_kept(p);` on a depth-1 `resource_borrowed` unkinded pointer `p`, paired with `resource_return_alias(p, nonnull)` or `resource_return_none()`.
   - **Drop (`-1`):** `ensures normal resource_reference_dropped(p);` or `ensures normal resource_reference_dropped(p, nullable);` on a depth-1 `resource_borrowed` unkinded pointer `p`, paired with `resource_return_none()`.
   - **Returned reference (`+1` fresh/looked-up):** `ensures normal resource_return_owned(n);` paired with `ensures normal resource_return_kind(reference);` (`rd_kafka_q_new0`, `rd_kafka_toppar_get2`).
   - Writing `resource_kind(p, reference)` on a parameter rejects the module, and passing a `reference` family to a heap `free()` triggers a kind-mismatch candidate/refusal (`C04.6` rule 1). A declaration carries at most one reference clause; `p` cannot simultaneously be retained, a slot, or a derived-out base.
2. **Withholding:** Any declaration carrying `resource_reference_kept` or `resource_reference_dropped` (or a returned reference paired with a retention) is **withheld from the function map** so unaware consumers see an uncontracted callee, never falling back to a custody-neutral `resource_borrowed` reading.

##### Per-object family semantics (gateway rule 13b)

Route G tracks references in **per-object families** (`count = keeps - drops` along each path):

1. **Borrowed vs. acquired families:**
   - Calling `keep(p)` where `p` is a plain name (or pointer cast) holding no tracked identity starts a **borrowed family** at `count = 1` bound to `p` and to the keep's destination (an external caller's reference keeps the underlying object alive prior to `keep`); calling `keep` on an active family increments its count (`+1`).
   - Calling a `resource_return_kind(reference)` constructor/lookup starts an **acquired family** at `count = 1` on its non-null branch (and nothing on its `NULL` branch).
   - Member reads, call expressions, other resource kinds, ambiguous/unknown/derived bindings, or storing a keep's result directly into a struct field (`o->f = keep(p)`) refuse (`reference-undecided`).
2. **Path balance and exit hand-offs:** At every function exit, each active family must either satisfy `count == 0` or have handed off **exactly one** remaining reference (`count == 1`) via `return p;`, storing into a field link of a fresh owner acquired by this function, `resource_stored` containment, or `C04.8` container publication (note that leaving a reference inside a local owner that is freed or retained does *not* discharge it, and dropping a family while a holder/field/container retains it refuses). An unhanded `count > 0` at exit is a provisional `Unsafe` **leak candidate** on refusal-free paths, and `Unknown` (`reference-unbalanced`) on paths with an earlier refusal (a refused statement mentioning a family's variable withdraws that family).
3. **Last drop vs. over-drop:**
   - The drop taking `count` from `1` to `0` is *possibly final* (the underlying C object may be freed if its global refcount hits zero), so the family is discharged and **all names bound to it are invalidated**: any subsequent read, keep, or drop of a borrowed family refuses as **`reference-dropped`** (`Unknown`, never `Unsafe`).
   - Dropping an unheld reference (an unbound formal belonging to the caller, a member read, or an already-handed-off family) refuses (`reference-drop-unheld`).
   - Dropping through a name invalidated by the last drop of an **acquired** family is an **over-drop** — every reference visible to this function on an object it obtained itself was already dropped, making it a provisional `Unsafe` **double-release candidate** on refusal-free paths (`Unknown` otherwise). Branch joins with disagreeing positive counts become undecided and refuse subsequent operations.
4. **NULL-accepting drops (`resource_reference_dropped(p, nullable)`):** Documented NULL-safe destructors (such as OpenSSL's `X509_free`, `EVP_PKEY_free`, `SSL_free`, `SSL_CTX_free`, `BIO_free`, `ENGINE_free`) treat a definitely-`NULL` actual as a no-op, and treat a variable bound to *one live family or `NULL`* (such as an untested nullable lookup result or branch join with `NULL`) as dropping that family cleanly, while still refusing unheld or invalidated names. Plain (non-`nullable`) drops refuse both `NULL` classes (`reference-drop-unheld`).
5. **Plain atomic counters, transfers, and interactions:** Low-level atomic primitives (`rd_atomic32_add`/`sub`) are never declared as `reference` operations; instead, named domain wrappers (`rd_kafka_toppar_keep0`) carry the contract so ordinary statistics counters on `rd_atomic32_t` never collide (`REFCOUNT-PLAIN-COUNTER-DECLARATION`). Inferred summaries remain heap-only (`REFCOUNT-INFERRED-REFERENCE-RESULT`). Slot moves (`C04.15`) on a reference family, keeps on borrowed/derived-out pointers (`C04.3.4`, `C04.16`), and combining reference clauses with `C04.13` return-code dispositions all refuse; counted users (`C04.7`) remain a separate formal-member mechanism.

##### Structural body validation (gateway rules 13c and 13d)

- **Keep and drop bodies (rule 13c):** In any unit defining a keep or drop body (including header `static inline` definitions), the declaration stands only if `p` is never reassigned or `&`-taken, is mentioned solely in member accesses, whole call actuals, or `NULL` tests, is never stored or returned except `return p;` in an alias-returning keep, has no `goto`, and touches `p` (via member access or forwarding to an identical reference position, e.g. `rd_kafka_q_destroy_owner` → `rd_kafka_q_destroy0`). For `nullable` drops, the body must also open with `if (!p) return;` or `if (p == NULL) return;` (`NULL`, `0`, `((void *) 0)`) or forward `p` whole to another `nullable` drop without member access. Failure marks the declaration **`reference-withdrawn`** in the retaining table (preventing W4 inference from replacing it!).
- **Returned-reference bodies (rule 13d):** In a defining unit, a `resource_return_kind(reference)` declaration stands only if the body has no `goto`, has at least one `return`, and every `return` yields a `NULL` constant (when `nullable`), a call to a declared alias-keep or returned-reference function (casts erased), or a non-address-taken, unescaped local assigned solely from `NULL`, such calls, or fresh heap allocations. Failure marks **`reference-withdrawn`**.

> **Implementation & Provenance.** `ReferenceClause::nullable` (`qcore-schemas`); gateway rules 13b, 13c, 13d. Lean models: `QCore.ReferenceCount`, `QCore.NullableReferenceDrop`, `QCore.ReferenceResult`, `QCore.LibraryVersionRange`. Cards: `REFCOUNT-RESOURCE-KIND`, `REFERENCE-DROP-NULL-ACCEPTING`, `REFCOUNT-REFERENCE-RETURNING-DECLARATIONS`.

---

<a id="cluster-c04-b--resource-kinds-cell-protocols-and-concurrencylifecycle-obligations"></a>
### Cluster C04-B — Resource Kinds, Cell Protocols, and Concurrency/Lifecycle Obligations

Heap pointers are only one class of resource managed by systems C code. File descriptors carried in `int` variables or struct fields, `FILE *` streams, `DIR *` handles, `dlopen` handles, stack-allocated `regex_t` and `pthread_key_t` objects, self-counted task references, intrusively linked list nodes, event-loop thread affinity, POSIX condition variables and thread joins, and strict `start`/`stop`/`destroy` context lifecycles all violate one or more assumptions of plain heap-pointer custody.

Cluster C04-B specifies how `resource_contract_v1` models kind-indexed and integer-carried resources (`C04.6`), in-place initializers (`C04.6.1`), struct/slot descriptor cells and cross-function cell protocols (`C04.6.2`–`C04.6.4`), counted object users (`C04.7`), intrusive container publication (`C04.8`), thread and lifecycle preconditions (`C04.9`–`C04.11`), scoped `[[local]]` conditional prerequisites and registered interior lifetimes (`C04.21`), and external intrusive-list finiteness premises (`C04.22`).

<a id="c046-resource-kinds-and-integer-carried-identities"></a>
#### C04.6. Resource kinds and integer-carried identities

Without resource kinds, every tracked custody obligation would inhabit a single undifferentiated pool: declaring `fopen`/`fclose` as owned/released would falsely allow `free(fopen(...))` or `fclose(malloc(...))` to discharge cleanly, and POSIX file descriptors — carried in `int` scalars whose failure sentinel is `-1` rather than `NULL` — could not be represented at all. Resource-contract v1 indexes custody obligations by **resource kind**.

##### Kind vocabulary, default heap representation, and placement

Two `ensures normal` clauses attach a non-heap kind to a parameter or return position:

```qv
ensures normal resource_kind(p, KIND);
ensures normal resource_return_kind(KIND);
```

- **Closed kind set:**
  - In `resource_kind(p, KIND)`: `KIND` `MUST` be one of `descriptor`, `stream`, `directory`, `dynamic_library`, `regex`, or `thread_key`.
  - In `resource_return_kind(KIND)`: `KIND` `MUST` be one of `descriptor`, `stream`, `directory`, `dynamic_library`, or `reference` ([C04.17](#c0417-reference-counts); note that `resource_kind(p, reference)` on a parameter rejects the module because parameter reference operations use `resource_reference_kept` and `resource_reference_dropped`, and in-place kinds `regex` / `thread_key` never appear on a return value).
  - No kind is added to the grammar without an audited corpus consumer (for example, an `FTS *` kind remains withheld until `fts_open` is modeled).
- **Heap spelled solely by absence:** Any custody-bearing position with **no** kind clause has the **heap** kind. Writing `heap` explicitly as a kind identifier rejects the module so every meaning has a single canonical spelling. In `ResourceFunctionContract`, `parameter_kinds` (storing non-heap entries only) and `result_kind` are omitted from serialized JSON and diagnostic output whenever heap, keeping unkinded contracts byte-for-byte identical to pre-kind artifacts.

| Contract position | Admitted non-heap kinds |
|---|---|
| `resource_owned`, `resource_consumed`, `resource_released` parameter | `descriptor`, `stream`, `directory`, `dynamic_library`; in-place kinds (`regex`, `thread_key`) on `resource_released` only ([C04.6.1](#c0461-in-place-kinds)) |
| `resource_out` parameter | Pointer-carried kinds (`stream`, `directory`, `dynamic_library`); in-place kinds (`regex`, `thread_key`) as status-witnessed initializers ([C04.6.1](#c0461-in-place-kinds)); `descriptor` **only** via the canonical lowering of a descriptor slot cell ([C04.6.2](#c0462-descriptor-cells)) |
| `resource_return_owned` result | `descriptor`, `stream`, `directory`, `dynamic_library`, `reference` (never an in-place kind) |
| `resource_borrowed` parameter | **None** (borrowed parameters carry no obligation), *except* an in-place kind's `use` position ([C04.6.1](#c0461-in-place-kinds)) |
| `resource_stored`, `resource_removed` parameter; `resource_return_reallocated`, derived, or `none` result | **None** — parent/child containment cascades and `realloc` replacement are specified exclusively for heap allocations |

##### Integer-carried identities (`descriptor`) and sentinel validity

`descriptor` is the sole integer-carried resource kind and the sole relaxation of [C04.1](#c041-abi-projection)'s pointer-only custody rule:

- **ABI type and failure sentinel:** Every `descriptor`-kinded parameter or result `MUST` have type `i32`, representing a POSIX file descriptor whose error sentinel is `-1` and whose valid handles satisfy `fd >= 0`.
- **Validity keywords (`sentinel` vs. `valid`):** Descriptor custody positions replace `nullable` / `nonnull` with **`sentinel`** (may equal `-1`) and **`valid`** (guaranteed `>= 0`). Using `nullable`/`nonnull` on a `descriptor` position, or using `sentinel`/`valid` on a heap or pointer-carried kind (`stream`, `directory`, `dynamic_library`, `reference`), rejects the entire module.

```qv
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

##### Mandatory consumer semantics

Every consumer interpreting resource kinds `MUST` enforce all seven rules without approximation:

1. **Kind-isolated custody discharge:** Every tracked resource identity permanently retains the kind of the site that introduced it (acquisition return, out slot, or checked entry formal). A `resource_released` call of kind `K1` never discharges an identity of another kind `K2` (`K2 != K1`): when the argument resolves to a single live identity of another kind, it produces a provisional `Unsafe` **`invalid_release`** violation candidate (`free(fopen(...))`, `fclose(malloc(...))`, `close(fp)`) while leaving the underlying obligation open. Every other kind mismatch (at `owned`, `consumed`, `stored`, `removed`, `reallocated`, or an ambiguous release) refuses (`Unknown`).
2. **Domain-accurate sentinel narrowing:** Because a `sentinel` descriptor call yields `-1` on failure or an integer `>= 0` on success, branch comparisons against `0` or `-1` narrow validity strictly within `{-1} ∪ [0, INT_MAX]`:
   - `fd < 0`, `fd <= -1`, and `fd == -1` (plus mirrored forms `-1 == fd`, etc.) establish *invalid* (no owed resource) on the true edge and *valid* (`>= 0`, usable at `valid` positions) on the false edge; `fd >= 0`, `fd > -1`, and `fd != -1` establish the exact reverse.
   - `fd > 0` establishes *valid* on its true edge **only** (and `fd <= 0` on its false edge **only**), because descriptor `0` (`STDIN_FILENO`) is a valid open descriptor on the complementary edge!
3. **Boolean and zero tests refine nothing on descriptors:** `if (fd)`, `if (!fd)`, `fd == 0`, and `fd != 0` neither validate nor eliminate a `sentinel` descriptor (`-1` is truthy in C, and `0` is a valid descriptor). Abandoning a descriptor along an `if (!fd)` early return is reported as the leak candidate it is.
4. **Tracked integers are custody, never custody-neutral data:** Before any analysis pass admits an integer expression as plain scalar data, it `MUST` query whether the expression reads a variable that may hold a tracked integer-carried identity (`descriptor_custody::value_carries_descriptor`), and **refuse (`Unknown`)** whenever it does — blocking stores into struct/union members (typed or untyped base), array elements, dereferenced pointers, or objects outliving the call ([C10](#c10-cleanup-and-os-endpoint-contracts)'s scalar-source exemption does not apply to tracked descriptors). Plain variable copies (`fd2 = fd`) alias the same identity; `return fd;` transfers/escapes it; arithmetic (`n = fd + 1`) creates and aliases no identity; relational comparisons branch without transferring custody.
5. **No descriptor laundering through inferred summaries:** Passing a tracked descriptor to a `Borrowed` parameter counts as a benign borrow **only** when the callee is governed by an explicit `Declared` contract. Because bottom-up W4 inference analyzes C helpers with integer formals untracked (so a helper that stashes `int fd` into a global or struct still infers `Borrowed`), passing a tracked descriptor to an `InferredValidated` summary `MUST` be refused (`Unknown`).
6. **Use-after-release candidates on non-heap kinds:** Passing a non-heap identity (other than `reference`, which follows [C04.17](#c0417-reference-counts)'s `reference-dropped` refusal) to a `resource_borrowed` position along a path where it has already been released produces a provisional `Unsafe` **`use_after_release`** candidate (`read(fd, ...)` after `close(fd)`, `fflush(fp)` after `fclose(fp)`). Plain heap pointers retain [C04.2](#c042-parameter-modes)'s caller borrow-liveness refusal (`Unknown`).
7. **Heap-only summary inference and kind-checked body validation:** Summary inference `MUST NOT` infer non-heap return or out-slot kinds. When checking a C body in a defining unit, entry formals are seeded with any declared kinds, and releasing a probed heap formal via a non-heap destructor refuses summary validation.

##### Consumer boundaries and deliberately excluded shapes

- **Verdict contract and `.qr` FFI bridge:** Candidates become `Unsafe` only under the provisional-`Unsafe` policy and are never presented as upstream defects. With zero kinded contracts active, every site is heap and behavior is byte-identical to unkinded v1. Quirin's `.qr` checked extern-call bridge (`qcore-contract::extern_bridge`) supports heap custody only and **refuses** kinded declarations rather than projecting them onto heap.
- **Deliberately outside `C04.6`:** Kinded `stored`/`removed` containers; multi-descriptor array out parameters (`pipe(int[2])`, `socketpair`); command-multiplexed creators (`fcntl` with `F_DUPFD`/`F_DUPFD_CLOEXEC`, where a single declaration allows at most one literal premise per parameter); success-conditional cross-kind transfers (`fdopen`, which consumes `fd` into `FILE *` *only* on non-`NULL` return); non-null pointer error sentinels (`mmap`'s `MAP_FAILED`); and `popen`/`pclose` process streams.

> **Implementation & Provenance.** `ResourceFunctionContract::{parameter_kinds, result_kind}`, `descriptor_custody::value_carries_descriptor`. Lean model: `QCore.KindedCustody`. Cards: `W43-DESCRIPTOR-RESOURCE-TRACKING`, `W43-LIBC-DESCRIPTOR-LIFECYCLE`, `CT-CTWIN-DESCRIPTOR`, `W44-LEAK-PIN-AUDIT`, `CG25-RESOURCE-KINDS`, `REFCOUNT-RESOURCE-KIND`.

<a id="c0461-in-place-kinds"></a>
##### C04.6.1. In-place kinds

Across `C04.6`, a resource kind's carrier takes one of three forms: a **pointer** (failure sentinel `NULL`), an **integer** (`descriptor`, failure sentinel `-1`), or **caller-owned storage** initialized in place and reporting failure solely through the call's **nonzero integer status return**. Resource-contract v1 defines two **in-place** kinds:

- **`regex`:** POSIX regular expressions (`regcomp(&preg, pattern, cflags)` initializes caller-allocated `regex_t`; `regfree(&preg)` frees its internal allocations).
- **`thread_key`:** POSIX thread-specific data keys (`pthread_key_create(&key, NULL)` initializes `pthread_key_t`; `pthread_key_delete(key)` finalizes it by value; `pthread_getspecific(key)` reads it by value).

###### Admitted clause pairings and signature rules

An in-place kind appears only in the following three parameter roles:

| Clause pair on parameter `p` | Carrier type of `p` | Operational meaning |
|---|---|---|
| `resource_out(p, nullable)` + `resource_kind(p, KIND)` | `ptr<mut, u8>` | **Initialize:** On return status `0`, caller object `*p` is initialized and owes finalization; on nonzero status, nothing is owed. |
| `resource_released(p, nonnull)` + `resource_kind(p, KIND)` | `ptr<mut, u8>` (or `u32` by value for `thread_key`) | **Finalize:** Discharges the live in-place obligation on the designated object. |
| `resource_borrowed(p)` + `resource_kind(p, KIND)` | `ptr<mut, u8>` or `ptr<const, u8>` (or `u32` by value for `thread_key`) | **Use:** Reads or updates the initialized object without altering its obligation (the sole kinded `resource_borrowed` form). |

- **Initializer restrictions (total-or-reject):** An initializer declaration `MUST` have return type `i32` with `resource_return_none()` (its status witness), `MUST` contain at most one initializer parameter, and `MUST NOT` transfer any other custody. Writing `nonnull` on an in-place initializer, `nullable` on an in-place finalizer, `owned`/`consumed`/`stored`/`removed` with an in-place kind, `resource_return_kind` with an in-place kind, or a kinded `resource_borrowed` parameter for any non-in-place kind rejects the entire module.

**A use defined on a failed object** (card CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE, gateway rule 74n). `ensures normal resource_kind_failure_use(p, KIND);` refines a use position: the call is defined on the object `p` designates whether its latest initialiser succeeded or failed, so long as no finaliser has run since (POSIX `regerror`, whose `errcode` is "the last non-zero value returned by regcomp() or regexec() with the given value of preg", The Open Group Base Specifications Issue 8; `regexec` on such an object is undefined and carries no clause). It stands only on a by-address use, `resource_borrowed(p)` + `resource_kind(p, KIND)` on a `ptr<const, u8>` or `ptr<mut, u8>`, with `KIND` the position's own in-place kind; on an initialiser, a finaliser, a by-value `thread_key` position, a position without that kind or a non-in-place kind it rejects the whole module, and so does a duplicate (grammar G05). It is a predicate the loader reads into the contract (`failure_uses`), not a premise statement. A consumer that certifies in-place uses (rule 6 below for a key; gateway rule 74i for every in-place kind) MUST admit a use carrying it where the object is bound, on every reaching path, to at most one site of its latest initialiser that no reaching path has finalised: before the status test, on the nonzero edge, or a join of those. It MUST still refuse the use before any initialiser (a path with no binding), and MUST read a use after the finaliser as rule 4's use-after-finalisation candidate. A use without the clause reads as before. A derived summary never states the clause, so a declaration carrying it is not compared with a body. The Lean model is `QCore.InPlaceFailureUse`.

###### General in-place consumer rules (rules 1–4)

1. **Bound to a single automatic object (`&NAME`):** For `regex` (and automatic `thread_key` under rule 6), an initializer or pointer-based finalizer actual `MUST` be `&NAME` where `NAME` is a single automatic, non-pointer, non-array variable of the enclosing function (and for `thread_key` by-value finalizer/use positions, the bare identifier `NAME`, never `&NAME`); every other actual expression refuses (`Unknown`). At a `use` position, passing an actual that names no tracked in-place object falls back to ordinary `resource_borrowed`.
2. **Status variable as validity witness:** The `i32` return status of an initializer `MUST` be captured into a plain, non-address-taken variable, which acts as the **witness** of `NAME` until that witness variable is overwritten. Testing the witness against `0` (`rc == 0`, `!rc`, `rc != 0`, `if (rc)`) resolves `NAME`: the zero branch **certifies** the live obligation, and the nonzero branch **removes** it. Discarding the initializer's return value, testing the call directly without storing into a plain witness variable, or invoking a finalizer before testing the witness refuses (`Unknown`).
3. **Tracked storage is custody, never raw data:** Mentioning a tracked in-place object anywhere *other* than as a whole `&NAME` (or by-value `NAME` for `thread_key`) at a matching in-place-kinded parameter of a declared callee — including struct assignment, member access, or passing `&obj` to an unkinded `memset(&obj, ...)` — refuses (`Unknown`).
4. **Violation candidates:** Applying a finalizer of a mismatched kind discharges nothing (`invalid_release` candidate when resolved), finalizing twice is `double_release`, using after finalization is `use_after_release`, and reaching function exit with a certified object unfinalized is a `leak` candidate.

###### Additional rules for `thread_key` (rules 5–6, gateway rule 11v (5) & (7))

The `libc` declaration of `pthread_key_create(&k, destructor)` carries `requires resource_literal_argument(destructor, 0);` ([C04.5](#c045-the-literal-argument-premise)) so destructor-registering calls stay uncontracted, and `pthread_setspecific` is intentionally withheld from the function map (gateway rule 12t) so mentioning a tracked key there triggers rule 3's refusal. For `thread_key`, rule 1's initializer target is partitioned into two admitted storage classes (every other target refuses):

5. **Static-duration key storage (`P-THREAD-KEY-STATIC`):** When the initializer actual is `&G` for an unshadowed object `G` of **static storage duration** (file-scope or block-`static`, excluding `_Thread_local`/`__thread`, `volatile`, and `_Atomic`), **no frame obligation is bound** under stated premise `P-THREAD-KEY-STATIC` (a destructor-free key created into static storage is process-lifetime and owes no `pthread_key_delete`). Its return status need not be witnessed, whereas calling `pthread_key_delete(G)` inside a function frame is **refused** because `G`'s lifetime outlives the frame.
6. **Certified-only use of automatic keys (`a_safe_verdict_used_only_certified_keys`):** When the initializer actual is `&k` for a unique automatic variable `k` of the function (where every reference to the identifier `k` designates that automatic object without file-scope shadowing), `k` is tracked under rules 1–4, **and** any by-value `use` (`pthread_getspecific(k)`) is admitted **if and only if `k` is certified on every reaching path** — bound to a single live initialization site whose witness test has already taken the `== 0` branch. Because POSIX leaves reading an uninitialized or failed `pthread_key_t` undefined behavior, using `k` before `pthread_key_create`, before testing its witness, or along the nonzero failure branch is **refused (`Unknown`)** (while using `k` after `pthread_key_delete(k)` remains rule 4's `use_after_release` candidate).

> **Implementation & Provenance.** Gateway rules 11v, 11v (5), 11v (7), 12t. Lean models: `QCore.ResourceKinds`, `QCore.ThreadKeyStatic` (`a_safe_verdict_used_only_certified_keys`). Cards: `CG25-RESOURCE-KINDS`, `CURRENT-CONTEXT-NEW-TRIAGE` (D3), `CG25-THREAD-KEY-AUTOMATIC-REVISIT`.

<a id="c0462-descriptor-cells"></a>
##### C04.6.2. Descriptor cells

Event-loop and channel APIs (such as Monkey's `mk_event_timeout_create`, `mk_event_channel_create`, and `mk_event_channel_destroy`) store created file descriptors directly into struct members (`ev->fd`) or caller `int *` slots (`*r_fd`, `*w_fd`), alias a struct's `fd` member to one end of a created pipe, and later close the descriptor while leaving its stale integer number sitting in `ev->fd`. Modeling these operations via opaque prose premises hides their per-descriptor obligations; **descriptor cells** express them compositionally in `quant live` syntax ([grammar `G05d`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks)).

A **cell** is a memory location holding a descriptor integer: either a struct member on a borrowed depth-1 pointer formal (`#P: TAG.FIELD`) or the `int` pointee of a slot formal (`#S: #pointee`). Just as a local variable is a name for a descriptor identity ([C04.6](#c046-resource-kinds-and-integer-carried-identities) rule 4), a cell is a **tracked name** of a descriptor identity, and the call's integer return code serves as its **witness**.

```qv
// 1. Member cell receives a fresh descriptor on success; untouched or closed on -1:
quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;
quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1;

// 2. Unconditional close of the descriptor named by the member (number left in place):
quant live #data: mk_event.fd |-> #stale(descriptor);

// 3. Slot cell receives a fresh descriptor on 0, aliased into the event's fd member:
quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;

// 4. Conditional descriptor release and loop-retention removal guarded by cell equality:
ensures normal resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd;
quant live #loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd;
```

###### Cell state values and canonical two-level elaboration

In descriptor-cell clauses, `K` is `descriptor` (`fresh`, `same`, and `pointee` are reserved `#`-form keywords; bare `#stale` without `(descriptor)` retains its heap meaning under [C04.19.6](#c0419-callback-contracts) so heap stays spelled by absence):

- **`#fresh(descriptor)`:** The call created one fresh `descriptor` identity named by the cell.
- **`#stale(descriptor)`:** The descriptor identity named by the cell was closed/released, while the cell itself still holds the old integer value (neither `#absent` nor zeroed `#released`).
- **`{#stale(descriptor), #kept}`:** On this return class, the cell names no live obligation created by the call (either untouched `#kept` or closed `#stale(descriptor)`).
- **`#same(S)`:** The member cell becomes a **second alias name** for the exact same `descriptor` identity delivered into `#fresh` slot `S` in the same return class (one underlying descriptor, owed once).

Every `quant live` cell shorthand elaborates into canonical `ensures normal` predicates (both levels — or any agreeing mixture — load to the exact same canonical representation, while any disagreeing overlap rejects the module; the binder's `ensures normal resource_borrowed(P);` appears in both):

| `quant live` shorthand | Canonical underlying predicates |
|---|---|
| `#P: T.F \|-> #fresh(K) when #result OP V` | `resource_member_kind(P, T.F, K)` + `resource_on_return(op, V, resource_member_out(P, T.F))` |
| `#P: T.F \|-> {#stale(K), #kept} when #result OP V` | `resource_member_kind(P, T.F, K)` + `resource_on_return(op, V, resource_member_stale_or_kept(P, T.F))` |
| `#P: T.F \|-> #stale(K)` | `resource_member_kind(P, T.F, K)` + `resource_member_released(P, T.F, stale)` |
| `#P: T.F \|-> #same(S) when #result OP V` | `resource_on_return(op, V, resource_member_alias(P, T.F, S))` (inheriting `S`'s kind) |
| `#S: #pointee \|-> #fresh(K) when #result == 0` | `resource_on_return(eq, 0, resource_out(S, nonnull))` + `resource_on_return(ne, 0, resource_borrowed(S))` + `resource_kind(S, K)` |

###### Total-or-reject grammar (`G05d`, `quant-live-cell-*`)

- **Binders and return witness:** Cell binders `P` and `S` `MUST` be unkinded depth-1 `resource_borrowed` pointers (for `#pointee`, `ptr<mut, u8>` verified against C `int *` at call sites). Witness guards `when #result OP V` require a signed integer return with `resource_return_none()` and `MUST` use a single guard pair across the declaration: either `!= -1` / `== -1` (descriptor-returning) or `== 0` / `!= 0` (status-returning). Each cell is specified either unconditionally or at most once per return class (never both); `#fresh` and `#same(S)` `MUST` be guarded, and `S` in `#same(S)` `MUST` be stated `#fresh` in that same return class.
- **Member-equality guard (`when #P: TAG.FIELD == #X`):** Permitted on `ensures normal resource_released(X, valid)` (where `X` is an `i32` formal carrying `resource_kind(X, descriptor)` and read as `resource_borrowed` in the base custody projection) and on `quant live #H: #holds(...) |-> #absent`. At most one such equality guard may appear per declaration, and if any `#holds` removal is guarded, every `#holds` removal on the declaration must carry that guard. Cells may coexist only with retentions, removals, and plain custody predicates; any violation rejects the module (`quant-live-cell-*`).

###### Call-site semantics and fail-closed refusals (gateway rule 38)

1. **Uniform Clang struct typing (including initial-member embedding):** A member cell `#P: TAG.FIELD` is admitted in a calling function **only** if every actual passed to that formal in the function has a single Clang pointer type (prior to implicit `void *` conversion) equal either to `struct TAG *`, or to `struct R *` whose unique complete definition in the translation unit declares its **first member** `m` as `struct TAG` (C11 6.7.2.1p15 initial-member pointer convertibility, reading `X` as `&X->m`). Unions, incomplete structs, multi-step nesting, mixed actual types, or unknown types refuse. Slot actuals `MUST` be `&L` or `&X->f[K]`.
2. **Witness resolution and alias staleness:** Creating a `#fresh` cell binds an *undecided* identity until testing its plain witness variable resolves the branch (live on the creation edge, stale on the failure edge). Closing any live name of an identity marks **every** alias name of that identity stale; closing a stale cell refuses (**`descriptor-cell-double-release`**); closing an undecided cell or closing through a key that may alias an unmatched recorded cell refuses (**`descriptor-cell-undecided`**).
3. **Member-equality guard evaluation:** `#P: TAG.FIELD == #X` evaluates to *true* when both sides name the **exact same live identity**, and *false* when they name **two distinct live identities** (two simultaneously open descriptors never share a file-descriptor number); all other states refuse (**`descriptor-cell-guard-undecided`**), leaving any guarded `#holds` removal unapplied and poisoning what it may designate ([C04.14.3](#c0414-retained-pointers)).
4. **Exit escape, owner release, and non-forwarding premise (`P-CELL-STATE-NOT-FORWARDED`):** Unless governed by an active cross-function cell protocol ([C04.6.3](#c0463-declared-cell-protocols)), reaching function exit with a cell that may name a live descriptor refuses (**`descriptor-cell-escapes`**, never `Safe`), and releasing an owner struct while one of its cells may name a live descriptor refuses (**`descriptor-cell-owner-released`**). Every declaration carrying cell states publishes rule premise **`P-CELL-STATE-NOT-FORWARDED`** (*"a state a call puts on a descriptor cell of an object this function does not track is not forwarded into its summary"*). Zero outcomes in `C04.6.2` produce `Unsafe`. Because Monkey's `mk_event.c` bodies forward through `loop->data` via `void *` casts unread by body checks, their pinned cell contracts stand as `Declared` assumptions (gateway rules 13i–13k).

> **Implementation & Provenance.** Gateway rule [38](agents/gateway.md#constraint-38); grammar [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks). Lean model: `QCore.DescriptorCells`. Cards: `MK-EVENT-DESCRIPTOR-QUANT` ([proposal](reports/mk-event-descriptor-quant/PROPOSAL.md), [report](reports/mk-event-descriptor-quant/README.md); decisions D1–D5, 2026-11-01), `MK-EVENT-FIRST-MEMBER-CELL` ([report](reports/mk-event-first-member-cell/README.md)).

<a id="c0463-declared-cell-protocols"></a>
##### C04.6.3. Declared cell protocols

In multi-function C subsystems (such as Fluent Bit's `flb_sched` and `flb_output_instance` channel pairs), one function initializes descriptor cells on a heap struct, helper functions inspect or use them, and a destructor later tests a sentinel field (`ch_events[0] != -1` or `ch_events[0] > 0`) and closes them. Without a cross-function invariant, `C04.6.2` refuses at the initializer's exit (`descriptor-cell-escapes`). A **declared cell protocol** ([grammar `G05i`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05i-declared-cell-protocols), gateway rules [38c](agents/gateway.md#constraint-38c) and [38f](agents/gateway.md#constraint-38f)) specifies the record-type invariant linking sentinel guards to live descriptor cells:

```qv
// Record-wide sentinel guard (all listed cells live iff ch_events[0] != -1):
quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
quant cell #s: flb_sched.ch_events[1] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
quant cell #s: flb_sched.event.fd     |-> #same(ch_events[0]) when #s.ch_events[0] != -1;

// Per-end sentinel guards with explicit C09.4 live/absent value sets:
quant cell #o: flb_output_instance.ch_events[0] |-> #fresh(descriptor) when #o.ch_events[0] > 0 live_set([1, MAX]) absent_set(0);
quant cell #o: flb_output_instance.ch_events[1] |-> #fresh(descriptor) when #o.ch_events[1] > 0 live_set([1, MAX]) absent_set(0);
quant cell #o: flb_output_instance.event.fd     |-> #same(ch_events[0]) when #o.ch_events[0] > 0;
```

###### Protocol invariants, per-end sentinels, and declared value sets

1. **Core invariant and placement:** Whenever a protocol guard holds on an instance of `struct TAG`, each governed `#fresh(descriptor)` cell names its own live descriptor and each `#same(TARGET)` cell aliases `TARGET`'s descriptor; when the guard is provably dead, no governed cell names a live descriptor. `quant cell` clauses stand inside any `extern fn` contract block alongside its ordinary clauses, adding nothing to that host function's own call effect while marking the host as a withheld lifetime declaration; if two declarations state *different* protocols for the same `struct TAG`, both are canceled and `TAG` has no protocol. Every row consulting a protocol publishes declared assumption **`P-CELL-PROTOCOL-DECLARED`** (rule 13i; no run-wide store census proves it).
2. **Record guards vs. per-end sentinels:**
   - A **sentinel guard** (`!= -1` or `> 0`) `MUST` test one of the `#fresh` cells itself; a **flag guard** (`== 1`) tests a separate integer flag field that is not a descriptor cell.
   - Alongside at most one record-wide guard, individual channel ends `MAY` declare **per-end sentinels** (rule 38f), where each `#fresh` end is guarded by its own cell (`#o.ch_events[i] > 0`) and any `#same` alias shares its target end's guard (tracked as one independent protocol fact per guard).
3. **Asymmetry of `> 0` vs. `!= -1` and `live_set` / `absent_set`:**
   - A sentinel's meaning is determined by the creator's possible return/written values and the holder's absent-value stores — **never** optimistically inferred from whatever test a caller wrote. Because standard POSIX creators (`pipe`, `open`, `eventfd`) can return descriptor `0` on success (observed natively when `stdin` is closed), an unqualified `> 0` guard's dead edge (`<= 0`) overlaps `0` and **leaves the cell undecided** rather than proving absence!
   - To resolve `> 0` sentinels when an explicit policy or creator contract bounds valid descriptors, a per-end sentinel's `#fresh` clause `MAY` append `live_set(SET)` followed by `absent_set(SET)` using [C09.4](#c094-declared-c-library-result-sets)'s integer-set syntax:
     - `live_set` bounds all values a creator writes into the cell; `absent_set` bounds all values the holder writes when no descriptor is open.
     - A test's dead edge proves *absent* iff it is disjoint from `live_set`, and its live edge proves *live* iff it is disjoint from `absent_set` (otherwise leaving the cell undecided). When omitted, `live_set` defaults to `int \ {-1}` and `absent_set` to the guard's dead set.
     - **Load-time rejections:** An `absent_set` intersecting the guard's live edge, a `live_set` intersecting `absent_set`, sets attached to `#same` or a record-wide guard, wrong clause order, or duplicate sets reject the module (`quant-cell-absent-set`, `quant-cell-live-set`, `quant-cell-set`).
     - **Creator grounding (`descriptor-cell-live-set-unproved`):** Creating into a cell with an explicit `live_set` is admitted **only** when the creator's pin-derived written set for that slot ([C09.4](#c094-declared-c-library-result-sets)) is active and contained within `live_set` (and stores into the sentinel are admitted only for values in `absent_set`). Declaring `live_set([1, MAX])` is an explicit policy bound (excluding `fd == 0`), attributed on every verdict it enables.

###### Body semantics under an active cell protocol (rules 38c, 38d, 38f)

- **Guard tests:** Testing `r->GUARD OP V` (or its logical complement) on a `struct TAG *` variable `r` holding no prior fact for that guard introduces the caller's live descriptor identities on the live edge, and records *absent* (no live cell) on the dead edge — *except* when the dead edge meets `live_set` (such as unqualified `> 0` meeting `fd == 0`), which leaves the end *undecided*.
- **Stores, creations, and closes:**
  - Stores may never overwrite a cell that may be live; sentinels may be written only with dead/`absent_set` values; `== 1` flags may be set to `1` only when all governed cells are live and cleared only when none are live.
  - Creations into protocol cells require the guard to be known dead (or the cell already tracked), and only a `!= -1` guard (or grounded `live_set` disjoint from dead values) is established directly by creation.
  - `close(r->CELL)` transitions that cell to `#stale` (gateway rule [38d](agents/gateway.md#constraint-38d) admits `r->CELL` through the indirect-access gate solely at a function-map `descriptor` release when resolved by rule 38c; copied integer locals are not cell names).
- **Exits, calls, and holder release:**
  - At function exits and calls that may reach `r`, a record satisfying its protocol is forwarded cleanly instead of refusing with `descriptor-cell-escapes`; a record violating its protocol (for instance, closed without resetting its sentinel) refuses with **`descriptor-cell-protocol-broken`**.
  - Releasing the holder struct `r` terminates its protocols when every governed cell is provably dead/closed, and refuses otherwise (emitting **`descriptor-cell-zero-undecided`** when an unqualified `> 0` dead edge left `fd == 0` undecided — a candidate refusal, never `Unsafe`). In the **release window** between closing an end without resetting its sentinel and freeing `r`, any call that may reach `r` refuses unless exempted by a callee frame ([C04.6.4](#c0464-callee-frames-over-a-declared-cell-protocol)). Joins never drop a potentially live cell, and nothing in `C04.6.3` produces `Unsafe`.

> **Implementation & Provenance.** Gateway rules [38c](agents/gateway.md#constraint-38c), [38d](agents/gateway.md#constraint-38d), [38f](agents/gateway.md#constraint-38f); grammar [G05i](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05i-declared-cell-protocols). Lean models: `QCore.DescriptorCellProtocol`, `QCore.CellProtocolPerEnd`, `QCore.CellResultSet`. Cards: `MK-EVENT-CELL-CROSS-FUNCTION` ([proposal](reports/mk-event-cell-cross-function/PROPOSAL.md), [report](reports/mk-event-cell-cross-function/README.md)), `CELL-PROTOCOL-PER-END-GUARDS`, `CELL-PROTOCOL-CLOSE-ACTUAL`, `CELL-RESULT-SET-ALIGNMENT` (decisions 2026-11-02 through 2026-11-04).

<a id="c0464-callee-frames-over-a-declared-cell-protocol"></a>
##### C04.6.4. Callee frames over a declared cell protocol

Destructors often close a struct's descriptor channels without writing `-1` back into the soon-to-be-freed struct, then invoke teardown helpers such as `mk_list_del(&ins->_head)` before calling `flb_free(ins)`. During that **release window**, the closed channel's sentinel still appears live, so calling `mk_list_del` (which can reach `ins` via `container_of` on `&ins->_head`) would ordinarily refuse with `descriptor-cell-protocol-broken`. A **callee frame** ([grammar `G05k`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05k-callee-frames-over-a-declared-cell-protocol), gateway rule [38g](agents/gateway.md#constraint-38g)) asserts that a specific callee touches no protocol member of `struct TAG`:

```qv
quant cell mk_list_del: flb_output_instance |-> #untouched;
```

- **Whole-type frame meaning:** `CALLEE` — together with every function `CALLEE` transitively calls — **neither reads nor writes** any member named by `struct TAG`'s declared cell protocol (the top-level member of every governed cell and guard) on **any** instance of `struct TAG`, regardless of how `CALLEE` reaches that instance (covering `container_of(&ins->_head, ...)` as well as direct pointers).
- **Placement and agreement:** Placed alongside `struct TAG`'s `quant cell` protocol clauses without altering the host declaration's own call contract. A record retains its callee frames only when its protocol stands and every declaration stating `TAG`'s protocol lists the exact same frame callees.
- **Checked vs. refuted vs. review-held standing:**
  - **Checked:** In a translation unit that defines `CALLEE` and every transitive callee it invokes using constructs fully modeled by rule 38g (2) with zero mentions of any protocol member of `TAG`, the frame is **checked** and preserves the record's cell-state fact across the call.
  - **Refuted:** If an analyzed body in that transitive closure names a protocol member of `TAG`, the frame is **refuted** and release-window calls refuse with **`descriptor-cell-frame-refuted`** (`Unknown`).
  - **Declared (held as `Pending`):** When `CALLEE` or a transitive callee lacks a defining body in the unit, the frame is treated as an unverified deferred-review contract ([C06.1](#c061-contracts-awaiting-human-review)), **never** a silent `Safe` premise: a release-window call admitted solely through an unchecked frame is held under **`awaiting-human-review`**, capping the caller's row at **`Pending`** (never `Safe`).
- **Non-discharge boundary:** Records that currently satisfy their protocol pass calls normally without consulting frames; frames never discharge an open descriptor cell (freeing the holder still requires every cell to be closed/dead), and nothing in `C04.6.4` produces `Unsafe`.

> **Implementation & Provenance.** Gateway rule [38g](agents/gateway.md#constraint-38g); grammar [G05k](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05k-callee-frames-over-a-declared-cell-protocol). Lean model: `QCore.CellProtocolReleaseWindowFrames`. Card: `CELL-PROTOCOL-RELEASE-WINDOW-FRAMES` ([report](reports/cell-protocol-release-window-frames/README.md); decision 2026-11-04).

---

<a id="c047-counted-users"></a>
#### C04.7. Counted users

Objects such as Fluent Bit retry tasks maintain a self-referential active-user counter in a struct member (`task->users`), distinct from heap pointer ownership ([C04.17](#c0417-reference-counts)): callers borrow the object pointer while incrementing, decrementing, and checking whether the counter has reached zero to trigger destruction. Resource-contract v1 models these quantitative member-counter protocols directly:

```qv
ensures normal resource_user_added(p, COUNTER);            // +1 user; caller owes a matching drop
ensures normal resource_user_dropped(p, COUNTER);          // -1 user, without running release check
ensures normal resource_user_dropped_checked(p, COUNTER);  // -1 user, followed by release check
ensures normal resource_user_release_check(p, COUNTER);    // release check alone (0 delta)
ensures normal resource_user_neutral();                    // call changes no user count on any object
ensures normal resource_user_exit(p, RETURNED, DELTA);     // returning RETURNED applied net DELTA
```

##### Syntax and placement (`G05`, total-or-reject)

- **Subject and member:** `p` `MUST` name a `resource_borrowed` pointer parameter (a user count is not pointer custody, and `p`'s custody mode stays `resource_borrowed`); `COUNTER` names the struct member holding the count.
- **Exit clauses:** `RETURNED` uses the canonical decimal integer spelling of [C04.5](#c045-the-literal-argument-premise) (requiring an integer return type on the declaration), and `DELTA` `MUST` be one of `added` (`+1`), `restored` (`0`), or `dropped` (`-1`) so net deltas are never written as arithmetic expressions.
- **Exclusivity:** Each parameter carries *either* at most one operation clause *or* `resource_user_exit` clauses (never both); `resource_user_neutral()` stands alone with no other counted-user clause. Any other placement rejects the entire module.

##### Mandatory consumer semantics

1. **Pathwise obligation accounting and strict formal attribution:** Along each CFG path, a formal parameter's net user delta is the sum of operations attributed to it. An operation is attributed to formal `p` **if and only if** the call-site actual is the bare identifier `p` and no statement in the function body ever rebinds `p` or takes `&p`. Any unattributed counted-user operation leaves **every** tracked count undecided (`Unknown`) and emits a diagnostic.
2. **Unchecked drops owe a release check:** Applying `resource_user_dropped` when the path's net count drops below its entry value (`0`) leaves the object potentially unreferenced without reclamation; unless a subsequent addition or release check occurs along that path before function exit, reaching that exit produces a provisional `Unsafe` violation candidate. (By contrast, an unchecked drop that merely rolls back an earlier `added` on the same path returns the delta to `0` and owes no release check.)
3. **Post-check invalidation:** Because `resource_user_dropped_checked` and `resource_user_release_check` may destroy the underlying object when its counter hits zero, any subsequent counted-user operation on that formal along the path is undecided (`Unknown`).
4. **In-unit body verification of exit clauses:** Whenever an analyzed translation unit defines the C body of a declared function, every `resource_user_exit` clause is checked path-by-path against the body (and every return value not covered by an explicit `resource_user_exit` clause is required to restore all tracked counts to `0`):
   - **Refuted:** A reachable path returning `RETURNED` has a known net count unequal to `DELTA` → provisional `Unsafe` candidate (never presented as an upstream defect).
   - **Undecided:** A path's net count or returned integer is unknown → `Unknown` (naming the blocker).
   - **Held:** Every matching path satisfies `DELTA`. Callers in the same unit may apply the declaration **only** when all clauses hold on the body; otherwise calls to it are treated as opaque.
5. **Declared frame and fail-closed opacity (premise `UC-FRAME`):** Under stated frame premise `UC-FRAME`, a contracted external function with no counted-user clauses and no body in the unit changes zero user counts, and an in-unit C body is neutral iff it never writes any declared `COUNTER` member and invokes only neutral callees. Any call lacking both a contract and a body — or dispatched indirectly — is **opaque** and leaves all tracked counts undecided (`Unknown`). Omitting an actual counter mutation from an external contract is a false declaration ([C04.4](#c044-unsupported-semantics-and-false-declarations)). Quirin's `.qr` checked extern-call bridge has no counted-user model and refuses declarations carrying these clauses.

> **Implementation & Provenance.** Grammar `G05`; Lean model: `QCore.CountedUsers`. Card: `W45-FB-RETRY-LIFETIME`.

---

<a id="c048-container-publication"></a>
#### C04.8. Container publication

Intrusive doubly-linked list insertions such as `mk_list_add(&node->_head, &queue->pending)` transfer ownership of `node` into a container reachable by the caller, yet cannot be expressed with `resource_stored(node, queue, ...)`: `node` is passed via an interior link-member address `&node->_head`, `queue` is borrowed rather than `Owned`, and freeing a bare `mk_list` head does not automatically free linked elements. Resource-contract v1 models intrusive publication and unlinking explicitly:

```qv
ensures normal resource_published(entry, container);
ensures normal resource_unpublished(entry);

// Element destroyer unlinking a specific intrusive list member before freeing:
extern fn mk_http_thread_destroy(mth: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_unpublished(mth, _head);
    ensures normal resource_released(mth, nonnull);
    ensures normal resource_return_none();
};
```

##### Base publication and unlinking rules

In standalone publication and unlink contracts, `entry` and `container` name distinct `resource_borrowed` pointer parameters (preserving `resource_borrowed` for ABI projection and `&B->F` member-address admission). Any consumer that does not implement `C04.8` `MUST` refuse calls to these declarations rather than treating them as benign borrows.

1. **Publish (`resource_published(entry, container)`):**
   - `entry` must denote its base object as `&B->F` or `B`.
   - `container` must be reached from **caller-supplied memory**: either an unrebound formal parameter (`&P->F...` or `P`), or a local variable `T *L = P->F;` initialized once from such a formal and never reassigned.
   - When `B` resolves to a single known, non-null, live identity with zero unresolved effects and zero owned linked members or contained children, `B` transitions to **published** (custody passes to the caller's container, discharging this frame's exit obligation).
   - Every other publication — an unnamed `entry`, a local/global/freshly-acquired `container`, an unknown/ambiguous/derived/nullable/dead `B`, an unbound formal (marking it unresolved so W4 derivation binds it), or an object owning linked members/children (withdrawing those owned objects in both directions) — **refuses (`Unknown`)**.
2. **Post-publication restrictions:** Once an identity is published, releasing it is a provisional `Unsafe` violation candidate (**`release_after_escape`**), and transferring it a second time (including `return B;`) refuses (`Unknown`).
3. **Standalone unlink (`resource_unpublished(entry)`):** Restores direct frame custody **if and only if** the target identity was published along **every** path reaching the unlink and remains published; every other standalone unlink (including untracked entries) refuses (`Unknown`).

*(Note: W4 derived summaries express parameter/return custody only, so custody-duty agreement in [C04.1](#c041-abi-projection) compares against a declaration's custody projection and ignores container and counted-user clauses.)*

##### Element destroyers, link-member verification, and `ElementsReleased` summaries (gateway rules 34, 43, 46)

As the sole placement exception to `resource_borrowed`, `resource_unpublished(p)` or `resource_unpublished(p, MEMBER)` `MAY` appear on the **same parameter `p` alongside `resource_released(p, nullable | nonnull)`** to declare an **element destroyer** that unlinks `p` (`mk_list_del(&p->MEMBER)`) and then frees `p`:

- **Grammar and co-occurrence:** Writing a `MEMBER` argument (`resource_unpublished(p, MEMBER)`) without `resource_released(p, ...)` on that same `p`, placing `resource_published` beside a non-borrowed mode, or combining `resource_unpublished` with any mode other than `resource_borrowed` or `resource_released` rejects the module. An element destroyer `MAY` also coexist with [C04.19.6](#c0419-callback-contracts) member-path state clauses when those states are the declaration's only retaining clauses (all other container + retaining combinations reject).
- **Operational order and admitted `entry` classes:** Consumers `MUST` apply the unlink **before** the release, admitting `p` in **only** three proved cases (refusing all others):
  1. *Published identity (rule 3):* `p` was published on every reaching path and is still published (admitted with or without `MEMBER`).
  2. *Foreign list-iteration element (gateway rules 10f(2) and 46):* Admitted **if and only if** the declaration specifies `resource_unpublished(p, MEMBER)` **and** every `container_of` binding of the iterated element names that exact `MEMBER`.
  3. *Unpublished frame-held object with self-initialized links (gateway rules 43 and 46):* `p` denotes one live, unpublished identity held by the frame and unlinked on zero reaching paths, admitted **if and only if** the declaration specifies `resource_unpublished(p, MEMBER)` **and** the calling body performed a checked self-initialization covering **both** link pointers (`prev` and `next`) of `p->MEMBER` (`mk_list_entry_init(&p->MEMBER)`) on every reaching path with no subsequent statement that could overwrite `p` (making `mk_list_del(&p->MEMBER)` a safe no-op before `free(p)`).
  - Consequently, an element destroyer written *without* `MEMBER` (`resource_unpublished(p)`) rejects both foreign iteration elements and unpublished self-initialized objects!
- **Derived `ElementsReleased` summaries (`resource_elements_released(#i)`, gateway rules 10f(3) and 43):** When an in-unit C loop/cleanup helper acquires every element custody (via foreign unlinks or inner `ElementsReleased` calls) strictly from containers rooted at its own formals, W4 publishes its derived summary accompanied by the side effect `ElementsReleased` at those exact formal indices (`resource_elements_released(#i)` in clause standing; never writable in `.qv` declarations, and withheld entirely if any non-formal container root is touched). Applying `ElementsReleased` forces the caller to **forget every object it previously published** (attaching an unresolved effect so any later unlink, release, or use of them refuses); any consumer that does not model `ElementsReleased` `MUST` refuse the call (**`has_container_effect`**), never reading it as a pure `Borrowed` call.

> **Implementation & Provenance.** Gateway rules 10f(2), 10f(3), 34, 43, 46. Lean models: `QCore.ContainerPublication`, `QCore.ElementDestroyer`, `QCore.ElementReleaseSummary`, `QCore.ElementLinkPremises`. Cards: `W45-FB-RETRY-LIFETIME`, `W48-SCHED-CLEANUP-PRODUCTION`, `LIST-ENTRY-RELEASE-IDENTITY`, `LERI-HELD-AND-LINK-MEMBER-PREMISES`.

---

<a id="c049-the-loop-owner-obligation"></a>
#### C04.9. The loop-owner obligation

Even when the OS kernel's underlying poll descriptor (`epoll_ctl`) is thread-safe, userspace event-loop structures — such as Monkey's priority-queue links and Fluent Bit collector `running` flags — are unsynchronized and may be read or mutated **only on the single dispatcher thread that owns that event loop**. Resource-contract v1 expresses this thread-affinity precondition with at most one clause beside the selector:

```qv
requires resource_loop_owner(p, read);
requires resource_loop_owner(p, mutate);
```

##### Syntax, rederivation, and per-unit fail-closed refusal

- **Total-or-reject placement:** `p` `MUST` name an admitted pointer parameter representing the input instance whose collectors attach to event loop `L` (`coll->evl`, resolving to worker loop `thi->evl` for threaded instances and engine loop `config->evl` otherwise), and the mode `MUST` be `read` or `mutate`. Duplicate clauses, unknown or non-pointer parameters, invalid modes, or wrapping the clause inside `resource_review_pending` rejects the **entire module** (deferral is permitted only at contract level under [C06.1](#c061-contracts-awaiting-human-review)). Inherited declarations copy their parent's obligation verbatim ([C15](#c15-contract-inclusion-and-inheritance)), and consumers rederive `declared_loop_owner_obligations` from retained module texts without altering serialized custody records.
- **Discharge solely by thread-entry census (gateway rule 10p):** A call site satisfies `resource_loop_owner(p, MODE)` **if and only if** every derived execution context reaching the caller runs on `owner(L)` (the unique dispatcher thread of the engine loop for non-threaded instances, or of the worker loop for threaded instances) with zero unresolved call-graph paths:
  - **Per-unit gateway routes** derive no thread identities and `MUST` treat every `resource_loop_owner` call as undischarged, refusing the call (`Unknown`) exactly like an uncontracted callee.
  - **The thread-entry census (rule 10p)** traces direct calls plus the bounded indirect hop classes (`A`, `S`, `V`, `V*`, `R`, `L`, `F` from `W47-TAIL-PAUSE-BINDINGS`); any other function-pointer hop, function-naming macro, or value-read struct field leaves reached functions unresolved. Either a resolved off-owner caller context or an unresolved/missing context withholds discharge (`Unknown`, never `Unsafe` and never an upstream defect claim).

##### Asynchronous pause/resume completion vs. collector registration

Public plugin-dispatch entry points (`flb_input_pause` / `flb_input_resume`) invoke arbitrary plugin callbacks and remain intentionally undeclared ([C04.4](#c044-unsupported-semantics-and-false-declarations), gateway rule 10j): in the census kernel, completion on a non-threaded instance means *"callback finished"*, whereas on a threaded instance it means only *"request enqueued"* (return codes alone never establish *"applied"*). Paired collector operations inside a single callback (such as `in_tail`'s pair) establish a joint post-state only at the callback boundary when both calls succeed on `owner(L)`, with `resume`'s running post-state additionally conditioned on the `ingestion_active` premise ([C04.18.9](#c04189-symbolic-premise-statements-formula-version-1)). By contrast, collector registration setters (`flb_input_set_collector_*`, modeled via ID-forwarded callback registrations in [C04.19.3](#c0419-callback-contracts)) and low-level `mk_event_add` / `mk_event_timeout_create` retentions ([C04.14.3](#c0414-retained-pointers)) carry no `resource_loop_owner` obligation.

> **Implementation & Provenance.** `declared_loop_owner_obligations`; gateway rule 10p. Lean model: `QCore.LoopOwner` (deriving for this route the single-dispatcher premise assumed in `QCore.EventDriven`). Cards: `W47-LOOP-OWNER-CONTRACTS`, `W47-LOOP-OWNER-INSTANCE-FLOW`, `W47-TAIL-PAUSE-BINDINGS`, `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION`.

---

<a id="c0410-the-thread-synchronization-obligation"></a>
#### C04.10. The thread-synchronization obligation

Condition-variable waits atomically release and reacquire a mutex while requiring a loop-rechecked predicate, condition signals publish no memory without holding the paired mutex, and `pthread_join` reclaims cross-thread retained storage only after a verified successful join. None of these invariants can be expressed by pointer custody modes alone. Resource-contract v1 attaches at most one synchronization precondition beside the selector:

```qv
requires resource_thread_sync(cond, mutex, cond_wait);
requires resource_thread_sync(cond, mutex, cond_timedwait);
requires resource_thread_sync(cond, cond_signal);
requires resource_thread_sync(cond, cond_init);
requires resource_thread_sync(cond, cond_destroy);
requires resource_thread_sync(thread, join);
```

##### Syntax, discharge conditions, and cross-thread reclamation

- **Total-or-reject placement:** `cond` and `mutex` `MUST` name distinct `ptr<mut, u8>` parameters (`cond_wait` and `cond_timedwait` name both; `cond_signal`, `cond_init`, and `cond_destroy` name `cond` only), and `thread` in `join` `MUST` name a `u64` or `usize` parameter (`pthread_t`). Duplicate clauses, invalid operations or parameter types, or clause-level review wrappers reject the **entire module**. Inherited declarations preserve the obligation ([C15](#c15-contract-inclusion-and-inheritance)), rederived via `declared_thread_sync_obligations`.
- **Per-unit refusal vs. rule 10q census discharge:** Per-unit resource routes derive no synchronization facts and `MUST` refuse every call carrying `resource_thread_sync` (`Unknown`). Only the synchronization census (gateway rule 10q) can discharge an operation:
  - **`cond_wait` / `cond_timedwait`:** Admitted only as the body of a loop that rechecks a predicate read under `mutex`, entered with `mutex` held (`cond_timedwait` may exit the loop on a nonzero timeout return without claiming predicate truth).
  - **`cond_signal`:** Admitted only while `cond`'s paired `mutex` is held.
  - **`cond_init` / `cond_destroy`:** Admitted only in a provably quiescent phase (prior to thread publication, after the final signal ordered under `mutex`, or after verified successful joins of all live threads).
  - **`join` and cross-thread storage release (C5):** `join` requires checking its `0` return code (except under the stop-path rule below). Crucially, storage that another thread may retain can be released **only** after a verified successful join of every live retaining thread (and recursively every child thread spawned by a live retaining thread); publication alone, timed-wait expiry, failed/unchecked joins, or unresolved thread identities grant nothing.
  - **Intentionally undeclared APIs:** `pthread_cond_broadcast`, C11 `cnd_*` (whose base `libc/threads.qv` custody contracts remain unchanged), reader-writer locks, try/timed mutex locks, and `pthread_create` (which retains an argument and spawns a callback, [C04.4](#c044-unsupported-semantics-and-false-declarations)) remain undeclared. All refusals yield `Unknown`, never `Unsafe`.

##### Stop-path join and non-null refinements (`P-POSIX-JOIN`, `P-DEREF`)

On teardown/stop paths (gateway rule 10q as amended), the census admits two bounded refinements under explicit named premises:

1. **Unchecked first join of a locally spawned thread (`P-POSIX-JOIN`):** An *unchecked* `pthread_join(t, ...)` counts as successful **only** when `t` is the **first** join along the path of a thread started on that path, and every POSIX `pthread_join` failure condition (`EINVAL` non-joinable/already-awaited, `ESRCH`, `EDEADLK` self/cycle) is derived impossible: `t` was created joinable, no `pthread_detach` or foreign thread's `pthread_join` may name `t`, and nothing on the joining thread aliases `t`'s handle afterward. A second join of an already-joined thread is undefined behavior in POSIX and never counts.
2. **Pre-publication dereferenced field non-nullness (`P-DEREF`):** The `NULL` branch of a struct-field check may be excluded when that field is unconditionally dereferenced earlier on every reaching path and written globally only by pre-publication stores.
3. **Unrepresentable stop patterns:** Observed C thread behaviors outside v1 (branch-conditional joins, unproved ignored join returns, early returns without joining) may be recorded as `resource_review_pending` declarations ([C06.1](#c061-contracts-awaiting-human-review)), remaining withheld (`Pending`/`Unknown`) and discharging nothing.

> **Implementation & Provenance.** `declared_thread_sync_obligations`; gateway rule 10q, constraint 27. Lean model: `QCore.CondvarJoin`. Cards: `W47-CONDVAR-JOIN-PUBLICATION`, `W47-CONDVAR-JOIN-STOP-PATHS`.

---

<a id="c0411-the-paired-lifecycle-obligation"></a>
#### C04.11. The paired lifecycle obligation

Subsystem contexts started and stopped in pairs — such as Fluent Bit's library-mode `flb_start(ctx)`, `flb_stop(ctx)`, and `flb_destroy(ctx)` — obey a strict temporal state machine that determines both when calling `stop` or `destroy` is legal and whether `stop` joins the background engine thread. Resource-contract v1 declares these roles with at most one clause beside the selector:

```qv
requires resource_lifecycle(ctx, start);
requires resource_lifecycle(ctx, stop);
requires resource_lifecycle(ctx, destroy);
```

- **Total-or-reject placement:** `ctx` `MUST` name a `ptr<mut, u8>` parameter; duplicate clauses, unknown or non-pointer parameters, invalid operations, or clause-level review wrappers reject the **entire module**. Inherited declarations copy the obligation ([C15](#c15-contract-inclusion-and-inheritance)), rederived via `declared_lifecycle_obligations`. (Note: this interpreted clause is completely distinct from the proposed `resource_lifecycle_v1` metadata record of [C04.18.4](#c04184-symbolic-lifecycle-metadata-source-version-1).)

##### Strict-pair context state machine (`L1`–`L7`)

Each context instance (keyed one per allocation site under stated premise `P-LIFECYCLE-INSTANCE`) begins in `created` upon assignment from its constructor and transitions across `{created, started-ok, start-failed, undecided, stopped, destroyed}`:

- **`start(ctx)`:** Returning `0` moves `created → started-ok`; returning nonzero moves `created → start-failed`; an untested/unknown return moves `created → undecided`.
- **`stop(ctx)`:** Moves `started-ok → stopped`.
- **The seven strict-pair clauses:**
  - **`L1` (No restart):** `start` is permitted **only** from `created` (restarting from `stopped` is refused unless body analysis proves the declared `start` supports restart; none in the corpus does).
  - **`L2` (Single join on first stop):** Calling `stop` from `started-ok` joins the background thread spawned by `start` **exactly once**.
  - **`L3` (No double stop):** `stop` is forbidden from `stopped`.
  - **`L4` (No stop after failed start):** `stop` is forbidden from `start-failed`.
  - **`L5` (No stop before start):** `stop` is forbidden from `created`.
  - **`L6` (Valid destroy states):** `destroy` is permitted **only** from `created`, `start-failed`, or `stopped` (and **nothing** is permitted after `destroy`).
  - **`L7` (Self-cleaned failed start):** Every nonzero-returning error path inside `start` leaves zero spawned threads unjoined (justifying `L6`'s admission of `destroy` directly from `start-failed`).
  - Calling `stop` or `destroy` when `start`'s return value is **`undecided`** leaves the precondition **not established** and refuses identically to a precondition violation.

##### Checked body clauses vs. caller preconditions

- **Caller preconditions (`L1`, `L3`–`L6`):** Restrict callers only and are in force by construction.
- **Body-verified clauses (`L2` on `stop`, `L7` on `start`):** Never trusted blindly! The path reader checks `stop`'s body (assuming `start` returned `0`) for `L2`, and `start`'s nonzero return paths for `L7`:
  - **In force:** Every followed path satisfies the clause with nothing unresolved (the sole state permitting callers to rely on it).
  - **Refuted:** Every followed path contradicts the clause (reported as a candidate upstream path, never a verified upstream defect).
  - **Held:** Some path is unresolved or mixed. Behavior of `stop`'s C body when invoked from caller-forbidden states (`created`, `start-failed`, `stopped`) lies outside the contract (observable natively, never read as a contract clause).
- **Consumer enforcement:** Per-unit resource routes derive no context state and refuse every `resource_lifecycle` call (`Unknown`). In the rule 10q census, any path violating `L1`/`L3`–`L6` or leaving `start` `undecided` refuses every downstream release reached by that path (uncreated or already-destroyed keys decide nothing; the release's verdict is the join over **all** paths, alongside an optional conforming-path report).

> **Implementation & Provenance.** `declared_lifecycle_obligations`; gateway rule 10q. Lean model: `QCore.CondvarJoin` (section `Lifecycle`). Cards: `W47-FLB-LIB-LIFECYCLE`, `W47-FLB-LIB-STRICT-PAIRING` (decision 2026-09-29), `W47-FB-DOUBLE-STOP-OBSERVATION`.

---

<a id="c0421-local-conditional-prerequisites-and-registered-lifetimes"></a>
#### C04.21. Local conditional prerequisites and registered lifetimes

In event-driven subsystems such as Fluent Bit's input collectors (`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`, surface S3), whether an operation owes single-thread loop ownership (`C04.9`) or relies on a concurrent worker protocol depends on an immutable configuration flag (`ins->is_threaded != 0`), while collector records embed interior `mk_event` fields registered into an event loop (`coll->event |-> coll->evl`) whose registration lifetime crosses function boundaries. Resource-contract v1 models both through **read-only member condition bindings**, **`[[local]]` conditional prerequisite blocks**, and **`quant registered` record-lifetime clauses**:

```qv
quant #threaded = (#ins.is_threaded != 0);
quant #threaded in {
    requires resource_premise(worker_protocol, #concurrent |-> #inert);
}: [[local]];
quant not #threaded in {
    requires resource_loop_owner(ins, mutate);
}: [[local]];
requires resource_premise(
    s3_condition_snapshot_stable,
    quant all #m in #condition_reads: #m in #entry_stable
);
quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;
```

##### Scoped `[[local]]` conditional blocks and entry-snapshot reuse

1. **Logical meaning and caller obligations:** A positive block `quant #C in { requires R; }: [[local]];` imposes `C ⇒ R`; a negated block `quant not #C in { requires R; }: [[local]];` imposes `¬C ⇒ R` (when `C` is unknown at a site, both branches remain feasible). At every external call boundary, inner `requires` clauses remain **caller obligations**, never facts minted by the annotation; exported summaries preserve conditional caller obligations while internal branch facts leave no global premise.
2. **Admitted surface and exclusive guarded withholding:** Inside a `[[local]]` block, only named premises (`C04.18`), loop-owner (`C04.9`), thread-sync (`C04.10`), paired-lifecycle (`C04.11`) prerequisites, and `quant registered` clauses are admitted. Nested `[[local]]` blocks, attributes other than `[[local]]`, conditional custody/`ensures`/review clauses, conditional `resource_contract_v1` selectors, and guarded mixtures with callback, reference-count, hand-off, or other structured effects are **refused**. Condition-bearing contracts are stored in an **exclusive guarded table** so unaware consumers and legacy raw effect prepasses withhold the entire declaration; `.qr` source checking continues to reject unchecked nonempty contracts (`Q021`).
3. **Atomic prerequisite identity and `s3_condition_snapshot_stable`:**
   - Within an analyzed C body, the current source consumer discharges **named entry premises only** (conditional loop-owner, thread-sync, and lifecycle obligations lacking independent proof remain `Unknown`).
   - A scoped named prerequisite is matched as an **atomic proposition**, never by interpreting English or deducing logical consequences: its identity is the tuple `(retained contract-evidence bundle, premise name, canonical C04.18.9 formula text, fully substituted condition object & member chain, polarity)`. Both declarations must belong to the **same checked environment** (matching text in unrelated evidence grants nothing, and the local `#threaded` alias identifier is alpha-local).
   - Reusing an entry conditional prerequisite at an interior call site additionally **requires** the explicit unconditional named premise `resource_premise(s3_condition_snapshot_stable, quant all #m in #condition_reads: #m in #entry_stable)` ([C04.18.9](#c04189-symbolic-premise-statements-formula-version-1) SP5 block; absence of writes is **never** inferred as a substitute for declaring this premise), **and** a checked snapshot showing zero mutations, intervening calls, object-identity ambiguity, or unsupported control flow between entry and use. Guarded obligations and `s3_condition_snapshot_stable` are **not** automatically covered by existing [C04.20](#c0420-outer-contracts-code-loaded-at-runtime) runtime loaded-code envelopes.

##### `quant registered` record lifetimes, poison, and bounded trace transport

1. **Record-local binder and run-wide validation:** `quant registered #B: TAG.INTERIOR |-> TAG.HOLDER;` names an interior member (`event`) and its holder path (`evl`) on `struct TAG`. Its binder `#B` is strictly clause-local (never supplying an object to outer conditions), and an unconditional `quant registered` clause leaves sibling custody/callback clauses intact for clause-grained readers (recording its named assumption whenever used for forwarding). Run-wide source checking classifies declarations into `validated`, `withdrawn`, or `unresolved` (plain W4 custody agreement never validates `quant registered`). Conditional type protocols require a checked per-record-to-host-instance correlation (a host formal's flag never governs arbitrary instances of `TAG`), and holder equalities must be derived from checked stores plus named stability premises (`P-HOLDER-PATH-STABLE`), never assumed from the condition.
2. **May/must registrations, pre-postcondition release checks, and poison:** `May` registrations and releases stay in summaries; a release is checked *before* subsequent callee postconditions; `must` endings must hold on **all** feasible exits; **poison survives unmatched removals**, and **only the record's own holder teardown ends a poisoned registration** (ending a holder *after* releasing the record never repairs the violation). Unread sources, missing home correlations, or unproved endings withhold admission (`Unknown`).
3. **Bounded ordered summary transport (`<= 4096` steps) and cross-unit imports:**
   - Ordered primitive lifetime traces (capped at `4096` steps; retained across a CFG join **only** when every exit produces the exact same trace) substitute exact source-qualified paths and replay against caller registration/poison state. Same-unit transport requires an internal-linkage (`static`) helper; cross-unit transport uses a dedicated effect-bearing import channel requiring independent source replay of the exact home custody contract, lifetime effects, expected home identities, and an explicit linked-resolution premise (missing/altered effects never degrade to a plain `Borrowed` summary; recursive home chains and guarded mixtures refuse).
   - **Removal-only helpers:** Acyclic single-path C helpers performing only removal calls and exits on unmodified nonvolatile/non-atomic formals retain ordered formal-index removal effects outside the borrowed map (branches, aliases, writes, releases, and mixed registration/removal classes refuse). Certificate issuance and replay always rederive local summaries and re-verify imported homes rather than trusting supplied inference tables.
4. **Direct-member holder frames (schema major `24`), immutable local copies, and collector source-audit imports:**
   - *Holder frames (schema major `24`):* Complete straight-line C bodies with zero stores/unread effects and a full root-use census establish bounded preservation frames for one direct struct member beneath an exact formal, composing bottom-up across internal callees and transferring across typed direct whole-record calls (kept outside `Borrowed` summaries and re-checked on certificate replay).
   - *Immutable local snapshots:* Single-assignment, non-address-escaped, uniquely declared typed local copies of collector pointers and loop snapshots in complete straight-line bodies reuse their formal-rooted path while staying in the holder census (runtime numeric collector IDs, record types, and allocation sets never prove pointer equality, and `P-HOLDER-PATH-STABLE` plus existing custody/alias-liveness obligations remain in force).
   - *Collector source-audit imports:* Separate from lifetime summaries, bounded source replay may validate first-match key lookup in one exact owner list and constructor-local instance/conditional-loop stores (with closed subobject initialization frames), without proving cross-call list invariance, ID uniqueness, persistent flag stability, caller config equality, nonnullness, or `Safe` promotion.

> **Implementation & Provenance.** Surface S3 (`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`), SP5 snapshot formula (`SP5-GLOBAL-PROSE-PREMISE-BAN`), schema major `24`. Cards: `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`, `EVENT-CALLER-REMOVAL`, `EVENT-ORDERED-LIFETIME`, `EVENT-HOLDER-IDENTITY`, `EVENT-COLLECTOR-SELECTION`, `SP5-GLOBAL-PROSE-PREMISE-BAN`.

---

<a id="c0422-the-list-finiteness-premise"></a>
#### C04.22. The list-finiteness premise

Bounding the iteration count of an intrusive doubly-linked list traversal (`mk_list_foreach`, `cfl_list_foreach`, and their `_safe` and `_r` reverse macros) by the list's entry length requires two facts: (1) at loop entry, the list is finite, circular, and free of concurrent link writers, and (2) nothing executed during the traversal appends or splices links in that list. Because (1) is a global data-structure invariant rather than a local function fact, it is stated explicitly in a standalone **list-finiteness premise** file ([grammar `G05l`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05l-list-finiteness-premises), gateway rule [48](agents/gateway.md#constraint-48)):

```qv
quant list flb_config.outputs |-> #finite;
```

- **Meaning (stated, never proved):** Asserts that whenever any traversal of `struct TAG`'s `MEMBER` list head (`struct mk_list` or `struct cfl_list`) begins, following links from the head reaches the head again in exactly `n + 1` steps (`n` its finite length) and no concurrent thread mutates its links during the traversal. No consumer derives this premise; it is recorded as an explicit trusted assumption on every loop bound resting on it.
- **Strict isolation from `.qv` contract bundles:** List-finiteness premises live **exclusively** in standalone `G05l` files selected via `QCORE_LIST_FINITENESS_PREMISES` — **never** inside `resource_contract_v1` declarations, serialized `ResourceContracts` environments, or provider contract bundles. Consequently, zero custody, bounds, or null-dereference verdicts can ever read or move because of a list-finiteness premise.
- **Transitive read-only link frame check (gateway rule 48):** Even with the premise supplied, a consumer records a traversal bound **if and only if** the loop's entire **transitive frame** (the loop body plus every function reachable from it) is defined and analyzed in the translation unit (or belongs to the consumer's fixed table of pure/read-only `libc` routines), contains zero indirect calls, and performs **zero writes that may reach a list link**. Unanalyzed external callees, callee contracts, or unread frame declarations establish nothing and leave a named loop-measure refusal.
- **Recorded fact (never a verdict or progress claim):** When established, the consumer records `<= |H|` iterations (`|H|` denoting list length at loop entry) as an auxiliary loop-measure row beside the custody verdict (gateway rule 44). It never alters any `Safe`/`Pending`/`Unknown`/`Unsafe` verdict and is never a liveness, termination, latency, or throughput claim.

> **Implementation & Provenance.** Gateway rules [44](agents/gateway.md#constraint-44), [48](agents/gateway.md#constraint-48); grammar [G05l](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05l-list-finiteness-premises); `QCORE_LIST_FINITENESS_PREMISES`. Lean model: `QCore.ListLengthBound`. Card: `W45-FB-LIST-FINITENESS-PREMISE` ([report](reports/w45-fb-list-finiteness-premise/README.md)).

---

<a id="cluster-c04-c--named-premises-symbolic-formulas-and-hypothesis-graphs"></a>
### Cluster C04-C — Named Premises, Symbolic Formulas, and Hypothesis Graphs

Many C safety properties rest on global, representation, or callback-closure invariants that cannot be proved locally inside a single translation unit or expressed solely through parameter custody modes — for example, that OpenSSL `CRYPTO_get_ex_new_index` callbacks do not release caller-held objects, that `cJSON`'s doubly-linked child chains are well-formed at entry, or that concurrent threads do not mutate Monkey's coroutine/list links during teardown. Rather than hiding those assumptions in silent code paths or informal English comments, Quirin requires contracts to surface them as **formal symbolic premises** and **evidence-tracked hypothesis graphs**.

Two foundational design invariants govern every construct in Cluster C04-C:

1. **Global ban on English prose strings (`QF001`–`QF005`):** Across the entire contract ecosystem — named premises, hypothesis atoms, guarded decomposition symbols and documents, outer-contract definitions, and JSON formal-binding manifests — **natural-language string literals are rejected fail-closed**. Every assumption must be written as a canonical formula over a closed, Lean-modeled symbolic vocabulary (`C04.18.9`) or as a typed mathematical `quant proposition` / `quant condition` (`C04.18.7`, `C04.18.8`). Historical frozen prose artifacts are readable solely by the test-only legacy adapter `tests/support/legacy_prose_premise.rs`, which is unreachable from any production binary or loader.
2. **Naming or decomposing a premise is never proof:** Recording a symbolic premise (`C04.18`), building a hypothesis graph (`C04.18.1`), attaching a neutral refinement or cryptographic binding manifest (`C04.18.2`), evaluating a proposed premise flow (`C04.18.3`), recording symbolic lifecycle metadata (`C04.18.4`), or replaying a guarded decomposition (`C04.18.5`, `C04.18.6`) **never discharges an unproved C obligation and never promotes `Unknown` or `Pending` to `Safe`**.

| Diagnostic | Retired prose construct | Enforcing reader & scope | Required symbolic replacement |
|---|---|---|---|
| **`QF001`** | `requires resource_premise(NAME, "PROSE");` (or any string literal inside a premise statement, including a quoted formula) | `qcore_frontend::premise_formula::premise_clause` across every parser and vocabulary (surfaced as `Q000` prefixed with `QF001` on compiler routes) | Unquoted `C04.18.9` symbolic formula (`G05-formula`) |
| **`QF002`** | `requires resource_hypothesis(ID, "PROSE");` (under any introducer) or any declared graph atom with an opaque statement | `qcore_frontend::premise_formula::hypothesis_clause` and `qcore_contract::hypothesis` source/wire validators | Typed `quant proposition #ID(PARAMS) = FORMULA;` (`G01p`, [C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3)); only generated `legacy_atom_*` erasures keep opaque text |
| **`QF003`** | String literals in `resource_guarded_symbol_v1` or `resource_guarded_decomposition_v1`, or any guarded symbol whose proposition is not the canonical printing of a typed-rendering formula | `qcore_frontend::guarded_decomposition::parse_source_clause`, `guarded_decomposition::replay`, and `guarded_metadata_bridge::render` | Unquoted `C04.18.9` typed-rendering formula (`G01p` over SP6/SP7/SP9) and unquoted `,`-separated `G05-guarded` clauses using `&&` ([C04.18.6](#c04186-guarded-production-source-embedding-source-version-1-bridge-version-2)) |
| **`QF004`** | `quant #outer [[contract]] NAME, "PROSE";` (any string literal in an outer-contract definition) | `qcore_frontend::premise_formula::outer_header` and `qcore_contract::outer_contract` | Unquoted `C04.18.9` symbolic formula over the SP8 block ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime)) |
| **`QF005`** | Prose descriptions or non-canonical/untyped fields in JSON binding manifests or flow-model `environment.variables` / `bindings` | `qcore_contract::formal_binding` across `hypothesis_bindings::resolve*` and `premise_flow::compile*` | Canonical JSON `G01p` typed parameter lists (bound parameters or SP6/SP7/SP9/SP10 rendered terms), 64-hex digests, and exact atom parameters/formulas ([C04.18.2](#c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2)) |

---

<a id="c0418-named-premises"></a>
#### C04.18. Named premises: overview and evolution

When an external declaration's soundness depends on an invariant that its parameter and return clauses do not express, the declaration states that assumption explicitly via `requires resource_premise(NAME, FORMULA);`:

```qv
extern fn rd_kafka_q_purge0(rkq: ptr<mut, u8>, do_lock: i32) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(do_lock);
    ensures normal resource_return_none();
};
```

##### Syntax, cross-vocabulary placement, and total-or-reject loading

The structural contract parser validates `resource_premise` in **every** module regardless of vocabulary:

- **Identifier and formula shape (`G05`, `G05-formula`):** `NAME` `MUST` be a lowercase ASCII identifier (`[a-z][a-z0-9_]*`) unique within the declaration (multiple `resource_premise` clauses with distinct `NAME`s are permitted on one declaration), the introducer `MUST` be `requires` (never `ensures`), and `FORMULA` `MUST` be a well-formed symbolic formula under [C04.18.9](#c04189-symbolic-premise-statements-formula-version-1). Any string literal (`QF001`), malformed formula, duplicate `NAME` on one declaration, or wrong argument count rejects the **entire module**.
- **Where `resource_premise` may appear by vocabulary:**
  1. **`resource_contract_v1`:** Admitted beside `requires resource_contract_v1;` on any `extern fn` declaration.
  2. **`bounds_contract_v1` ([C09.3](#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)):** Admitted **if and only if** the same declaration also contains at least one `nonnull_member` clause (of any qualifier kind: stable, scoped, or plain). The premise then attaches to those member facts and is listed on every `c.null.deref.v1` row consulting them (the index/bounds consumer never reads it). Writing `resource_premise` beside any other bounds or null clause (including `quant null-deref` without `nonnull_member`) rejects the module.
  3. **Quirin `resource type` model contracts** (such as librdkafka's `queue_destructor.qv` declaring `rd_kafka_q_destroy_final`, selected by zero resource views): Admitted on any `extern fn`, extracted via `module_premises`, and pinned by module clause count and digest without entering any C verdict table.

##### Semantic standing, inheritance, and lock pinning

- **Pure assumption metadata (with two explicit narrow exceptions):** Apart from the physical unlink representation gate `detach_list_representation` in [C04.3.5](#c0435-checked-same-item-detach-results) and the scoped `[[local]]` conditional prerequisites plus `s3_condition_snapshot_stable` in [C04.21](#c0421-local-conditional-prerequisites-and-registered-lifetimes), an ordinary `resource_premise` **alters no custody mode, neither enables nor withdraws any call site, and is uninterpreted by verdict consumers**. Consumers rederive `declaration_premises` and `declared_premises` from retained `source_assumptions` and publish them on every verdict row consulting the declaration ([C06.5](#c065-per-verdict-contract-dependencies)), alongside `literal_argument` (`p = LITERAL`, [C04.5](#c045-the-literal-argument-premise)) and rule-attached premises (such as gateway rule 13e's `P-RESULT-HOLDER-LAST-DROP` and `P-RETAINED-PREDEFINED` on `resource_retained_by(p, result)`).
- **Inheritance and `contracts.lock` binding:** [C15](#c15-contract-inclusion-and-inheritance) elaboration copies parent `resource_premise` clauses verbatim and forbids children from dropping them. Because the clause is part of the normalized declaration text, adding, deleting, or editing a premise changes the declaration's SHA-256 digest and places a pinned contract **out of force** (`contract-pin-mismatch`, [C06.2](#c062-revision-modes-pinned-and-flexible-contracts)) until re-reviewed and re-pinned.
- **Runtime outer-contract interaction ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime)):** Every named premise defaults to a **closed-world fact** in the runtime outer projection (degrading dependent rows to `Unknown` unless covered by `#premise NAME` in an active outer envelope, or explicitly exempted as a build/reading premise in `C04.20`).

##### Provider premise architectures and structural audit checks

Across the shipped and user provider crates, symbolic named premises partition residual external invariants cleanly from stated `C04.19` callback clauses:

1. **librdkafka (`qcore-provider-librdkafka`, 220 clauses across 12 names in 9 modules):**
   - `rk_plugin_callback_closure` (180 declarations, including `rd_kafka_crash`, `rd_kafka_dump0`, and model contract `rd_kafka_q_destroy_final`) bounds `rk_conf` callbacks: in the static scope, invoked slots close over the run's declared setter registrations and constructor defaults; in the runtime scope with loaded plugins, it rests on the pinned `rk_plugin_library` and `flb_external_plugin` outer envelopes ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime)) without falsely claiming no plugin is loaded.
   - `served_queue` (on `rd_kafka_q_purge0` and `rd_kafka_q_wait_result`) bounds op callbacks (`rd_kafka_toppar_op_serve`, open union stores on `rd_kafka_op_s.rko_op_cb`, and `rd_kafka_poll_cb`'s `rko_u.offset_commit.cb`) whose queue-reference neutrality is not yet closed by in-force candidate contracts.
   - Crate tests (`every_premise_is_a_formula`) enforce formula-only premises across the tree (load-time enforcement on pre-packaged artifacts is tracked under `SP2-SHIPPED-PROVIDER-PROSE-BAN`).
2. **OpenSSL 3.5.5 (`quirin-provider-openssl`, audited by its checker plugin):**
   - Documented function-pointer setters across OpenSSL 3.5 manual pages — plus three **source-derived** setters lacking 3.5 man pages, pinned to OpenSSL `3.5.5` release source (`since "3.5.5"`): `EC_KEY_METHOD_set_init` (6 `ec_key_method_st.ec_key_meth_*` slots including `ec_key_meth_finish`) and `ENGINE_set_pkey_meths` / `ENGINE_set_pkey_asn1_meths` (`engine_st.engine_pkey_meths` and `.engine_pkey_asn1_meths`, passing `NULL` at their first two arguments via `supplied_nullable`) — are declared as explicit `C04.19.3` opaque callback slots (covering password, cert, verify, info, message, security, session, client-cert, 44 handshake/extension/PSK/ticket/SRP/async/QUIC/X509_STORE slots, 14 `BIO` / `BIO_METHOD` slots in `bio_slots.qv`, 9 `X509_LOOKUP_METHOD` slots in `x509_lookup_slots.qv`, and 18 key/digest/cipher/engine/async teardown slots in `teardown_slots.qv`, including slot invocations beside reference drops on `SSL_free`, `SSL_CTX_free`, `BIO_free`, and `X509_STORE_free`, and through released parameters on `EVP_MD_CTX_free` and `PKCS12_free`; registry invocations never stand beside reference drops).
   - Symbolic premises cover only what `C04.19.3` cannot express as parameter-held setter slots, enforced mechanically by three checker-plugin audit tables:
     - **Writer-list completeness:** Every 3.5 manual page's `NAME` list must match its slots' declared writers and non-writers (and every source-derived slot must declare every field store in `3.5.5`).
     - **`EX_DATA_AUDIT` (`QCore.OpensslExDataPremise`):** Requires `openssl_ex_data_callbacks` on every declaration (30 total) whose documented behavior allocates, copies (`SSL`, `SSL_SESSION`, `EC_KEY`, `BIO` chain), or frees (directly or internally as replaced/temporary objects) an instance of any of the 15 `CRYPTO_get_ex_new_index` classes.
     - **`FREE_AUDIT` and `INNER_FREE_CLASSES` (`QCore.OpensslObjectFreesAudit`, `QCore.OpensslFreeTeardownSlots`, `QCore.OpensslInnerFreesAudit`):** Requires `openssl_builtin_teardown_functions` and `openssl_loaded_module_functions` on all 8 teardown-running frees and 14 inner-key-freeing calls (22 each); requires `EVP_PKEY_free`, `X509_free`, `ENGINE_free`, and `EVP_MD_CTX_free` to **omit** `openssl_unstated_callback_slots` because 100% of their teardown callbacks are stated slots; requires `X509_STORE_free`, `SSL_CTX_free`, `SSL_free`, and `PKCS12_free` (which can still run `X509_CRL_METHOD`'s free function written into a result by `X509_CRL_METHOD_new`, plus `SSL_set_quic_tls_cbs` dispatch table `crypto_release_rcd` on `SSL_free`) alongside the 9 non-handshake inner-freeing loaders/readers (`SSL_CTX_use_*`, PEM/`PKCS12` readers, verify-path loaders) to carry the free statement of `openssl_unstated_callback_slots` (and the 5 handshake/`SSL_shutdown` calls to carry the handshake + free conjunct); requires `BIO_free` and `SSL_free` to carry `openssl_builtin_bio_methods` and `openssl_bio_free_callbacks`, and `X509_STORE_free`/`SSL_CTX_free`/`SSL_free` to carry `openssl_builtin_lookup_methods`; and forbids free premises on `OPENSSL_sk_free` and `X509_new`.
3. **Fluent Bit, `libc`, and `libevent`:** `quirin-provider-fluent-bit` carries three symbolic premises (`untracked_handback`, `ingestion_active`, `synchronous_output_reachability`); `qcore-provider-libevent` carries seven symbolic scheduler premises on `mk_http_thread_destroy`; and `quirin-provider-libc` carries zero `resource_premise` clauses, expressing its two `nested_call_admission` assumptions as `resource_premise_all` conjunctions over eight typed `quant proposition` atoms ([C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3)).

> **Implementation & Provenance.** `qcore_frontend::premise_formula` (`QF001`), `declaration_premises`, `declared_premises`, `module_premises`. Lean models: `QCore.OpensslExDataPremise`, `QCore.OpensslObjectFreesAudit`, `QCore.OpensslFreeTeardownSlots`, `QCore.OpensslInnerFreesAudit`. Cards: `VERDICT-CONTRACT-DEPENDENCIES`, `LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER`, `NONNULL-MEMBER-UNIFIED-QUALIFIER`, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`, `OPENSSL-SSL-CTX-CALLBACK-SLOTS`, `OPENSSL-UNSTATED-CALLBACK-SLOTS`, `OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS`, `OPENSSL-BIO-METHODS-AND-CALLBACKS`, `OPENSSL-EX-DATA-REGISTRY`, `OPENSSL-FREE-SESSION-REMOVE-INVOCATION`, `OPENSSL-REMAINING-HANDSHAKE-SETTERS`, `OPENSSL-FREE-INDIRECT-CALLBACKS`, `OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS`, `OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS`, `OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS`, `OPENSSL-RELEASED-PARAMETER-SLOTS`, `SP1-SYMBOLIC-OPENSSL-PREMISES` through `SP5-GLOBAL-PROSE-PREMISE-BAN`.

---

<a id="c04189-symbolic-premise-statements-formula-version-1"></a>
#### C04.18.9. Symbolic premise statements (formula version 1)

Every `resource_premise(NAME, FORMULA)` statement (and every `outer_contract_v1` definition statement in [C04.20](#c0420-outer-contracts-code-loaded-at-runtime)) is written in **formula version 1** ([grammar `G05-formula`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-formula-symbolic-premise-statements)), a sorted first-order logic over a closed, Lean-modeled term vocabulary (`QCore.PremiseFormula.Model`).

##### Grammar (`G05-formula`), operator precedence, and sorts

Operators bind from **loosest to tightest** as follows (reusing existing Quirin tokens plus the single dedicated set-difference token `\` (`Backslash`); `-` is rejected inside formulas so set difference has one unambiguous spelling, and the `G01` source-operator gate admits `|` and `&&` inside premise formulas):

| Precedence (loosest → tightest) | Syntax | Associativity / grouping rule | Typing & logical meaning |
|---|---|---|---|
| **1 (loosest)** | `P -> Q` | Right-associative | Logical implication (`prop × prop → prop`). |
| **2** | `P && Q` / `E1 && E2` | Left-associative | Logical conjunction on `prop × prop → prop` or callback effects `eff × eff → eff`. |
| **3** | `F \|-> E` | Non-associative | Callback maps-to (`fn × eff → prop` or `set(fn) × eff → prop`): **every** run of callback `F` (or of every member of `F`) during the call satisfies effect `E` (vacuously true when `F` does not run). |
| **4** | `x in S` · `R1 == R2` | Non-associative | Set membership (`T × set(T) → prop`) and caller-resource-set equality (`set(res) × set(res) → eff`). |
| **5** | `S1 \| S2` · `S1 \ S2` | **Parentheses required** when mixing or chaining | Set union and set difference (`set(T) × set(T) → set(T)`). Mixing or chaining `\|` and `\` without `( ... )` rejects the module. |
| **6 (tightest)** | `( F )` · `quant all #x in S: P` · `#term` / `#term(A)` / `#term(A, B)` · `CONST` · `{}` | Atomic / prefix | Grouping; bounded universal quantification binding `#x` to the element sort of `S` in `P`; closed-vocabulary terms; registered constants; empty resource set `{}` (`set(res)`). |

- **Closed sort system:**
  - *Element sorts:* `obj` (a C object or record/slot cell), `cls` (an OpenSSL object class), `fn` (a callback function at an invocation site, or `#call` itself), `meth` (a `BIO_METHOD` or `X509_LOOKUP_METHOD` table).
  - *Set sorts:* `set(obj)`, `set(fn)`, `set(meth)`, `set(res)` (sets of caller-held resources).
  - *Logical/effect sorts:* `prop` (a proposition — top-level formula sort) and `eff` (a property of one execution run of a callback or `#call`).
- **Standard logical idioms in `G05-formula`:**
  - *Negation:* Expressed either as implication to falsum `P -> #false`, or as membership in a set difference `#x in S \ T` when `#x` ranges over `S`.
  - *Nullable pointer members as 0-or-1-element sets:* A pointer member is modeled as `set(obj)` (empty when `NULL`, singleton when non-`NULL`): `"NULL or P"` is `quant all #x in S: P`, `"is NULL"` is `quant all #x in S: #false`, and `"is non-NULL"` is `(quant all #x in S: #false) -> #false`.
  - *Properties of the declared call itself:* `#call` (sort `fn`) denotes the single execution of the declared function, so `#call |-> E` asserts that the call itself satisfies `E`.

##### Fail-closed validation, canonical normalization, and legacy erasure

- **Total-or-reject validation:** Unknown `#term`s or constants, arity or sort mismatches, missing argument lists, unbound `#x` variables, bound variables shadowing a registered `#term` or outer bound variable, top-level expressions of sort other than `prop`, over-long token streams, constants exceeding 64 bytes (save registered `CANDIDATES_<64-HEX>` constants in SP8), or string literals (`QF001` / `QF004`) reject the **entire module**.
- **Canonical identity and rendering:** Premise identity is `(NAME, canonical_printing(FORMULA))` — two whitespace/parenthesis spellings that parse to the same AST denote the exact same premise, and dependency records, reading-site texts, and `declared_premises` always emit the canonical printing (while the raw module SHA-256 continues to hash exact file bytes).
- **Backslash rule in legacy hypothesis erasure:** Every `resource_premise` elaborates to a generated `legacy_atom_*` node in the declaration's v1 hypothesis graph ([C04.18.1](#c04181-evidence-only-hypothesis-graphs-version-1)) carrying its canonical formula text. The graph validator permits `\` inside an opaque legacy atom **if and only if** the atom is a generated `legacy_atom_*` whose text is the exact canonical printing of a valid `G05-formula` (all other legacy rule/literal-argument records retain the strict no-backslash, non-blank, no-control-character rule, and user-declared opaque atoms are banned by `QF002`).

---

##### Premise-formula `VOCABULARY` layers (`SP1`–`SP5` and `SP8`)

Every `#term` admitted directly inside `G05-formula` (`VOCABULARY` in `qcore_frontend::premise_formula`, mirrored component-by-component with defaults in `QCore.PremiseFormula.Model`) belongs to one of the six blocks below. Adding any new term, constant, or sort is a normative specification change requiring a matching Lean model update.

###### Block `SP1` — Core resource & OpenSSL callback/teardown terms

Alongside the 28 terms below, `SP1` registers the **15 constants of sort `cls`** corresponding to OpenSSL 3.5's `CRYPTO_get_ex_new_index` classes: `BIO`, `DH`, `DSA`, `EC_KEY`, `ENGINE`, `EVP_PKEY`, `RSA`, `SSL`, `SSL_CTX`, `SSL_SESSION`, `UI`, `UI_METHOD`, `X509`, `X509_STORE`, and `X509_STORE_CTX`.

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#args` | `set(obj)` | Objects designated by the call's pointer arguments. |
| `#created` | `set(obj)` | Objects allocated by the call, directly or as internal temporaries. |
| `#copied` | `set(obj)` | Source objects copied/duplicated by the call. |
| `#freed` | `set(obj)` | Objects whose final reference the call drops — directly, transitively (owned sub-objects), or as replaced/temporary internal objects. |
| `#class(o)` | `obj → cls` | OpenSSL class of object `o`. |
| `#of(k, S)` | `cls, set(obj) → set(obj)` | `{ o ∈ S \| #class(o) = k }`. |
| `#ctx(S)` | `set(obj) → set(obj)` | The `SSL_CTX` associated with each `SSL` object in `S`. |
| `#bios(S)` | `set(obj) → set(obj)` | Read and write `BIO`s held by each `SSL` object in `S`. |
| `#chain(S)` | `set(obj) → set(obj)` | `BIO`s in `S` plus every `BIO` chained downstream (`bio_next`), transitively. |
| `#ssl_of(b)` | `obj → set(obj)` | Singleton `SSL` object held by a `BIO_f_ssl` `BIO` `b` (empty for all other objects). |
| `#slots(S)` | `set(obj) → set(fn)` | Every callback pointer slot called by the library on objects in `S` (including `SSL_CTX_callback_ctrl` servername/status/ticket-key arms, CT validation, not-resumable session, and session-ticket-ext callbacks). |
| `#stated` | `set(fn)` | Callbacks whose invocation is explicitly declared on this declaration via `C04.19` clauses. |
| `#ex_data(k)` | `cls → set(fn)` | `new`, `dup`, and `free` callbacks registered via `CRYPTO_get_ex_new_index` for class `k`. |
| `#ex_data_free(k)` | `cls → set(fn)` | `free` callbacks registered via `CRYPTO_get_ex_new_index` for class `k`. |
| `#method(b)` | `obj → meth` | Method table (`BIO_METHOD` or `X509_LOOKUP_METHOD`) of `BIO` or `X509_LOOKUP` `b`. |
| `#builtin` | `set(meth)` | Library-defined method tables never passed to a setter (`BIO_s_socket`, `BIO_s_mem`, `BIO_f_buffer`, other built-in `BIO_s_*`/`BIO_f_*`, and `X509_LOOKUP_file`/`_hash_dir`/`_store`). |
| `#fns(b)` | `obj → set(fn)` | Functions in `b`'s `BIO_METHOD` or `X509_LOOKUP_METHOD`, instantiated as run on `b`. |
| `#store(S)` | `set(obj) → set(obj)` | Certificate store (`X509_STORE`) held by each `SSL_CTX` in `S`. |
| `#lookups(S)` | `set(obj) → set(obj)` | Lookup instances (`X509_LOOKUP`) held by each `X509_STORE` in `S`. |
| `#teardown(o)` | `obj → set(fn)` | Callbacks executed when freeing `o` from its method tables, provider, engine, and held cleanup fields. |
| `#settable` | `set(fn)` | Callbacks writable via any setter declared in the declaration's provider crate. |
| `#library` | `set(fn)` | Library-defined functions installed without a setter call in the run: default `RSA`/`DSA`/`DH`/`EC_KEY`/`EVP_PKEY_ASN1`/`EVP_PKEY`/`EVP_MD`/`EVP_CIPHER`/`EC_METHOD` tables, built-in engines, and `default`/`base`/`legacy`/`null` provider dispatch tables. |
| `#loaded` | `set(fn)` | Functions from dynamically loaded external modules: providers outside `default`/`base`/`legacy`/`null` (including `FIPS` and `OSSL_PROVIDER_load` modules) and shared-object engines loaded via the `dynamic` engine. |
| `#in_turn(f, S)` | `fn, set(obj) → set(fn)` | Callbacks invoked transitively during a run of `f` on any object in `S`. |
| `#caller_held` | `set(res)` | Resources held by the caller at the start of the run. |
| `#takes(R)` · `#releases(R)` · `#retains(R)` | `set(res) → set(res)` | Subset of `R` that the run takes custody of, releases, or retains past its return. |
| `#inert` | `eff` | Defined macro: `#takes(#caller_held) == {} && #releases(#caller_held) == {} && #retains(#caller_held) == {}`. |

###### Block `SP2` — librdkafka & cJSON callback, assertion, queue, and list terms

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#false` | `prop` | Falsum (never holds; `P -> #false` is `¬P`). |
| `#rk_conf` | `set(obj)` | Every `rd_kafka_conf_t` (`struct rd_kafka_conf_s`), including embedded `rk->rk_conf`. |
| `#ran(x)` | `set(fn) → set(fn)` | Subset of `x` actually invoked directly or transitively during the call (including `rd_kafka_log_buf` → `log_cb`). |
| `#registered` | `set(fn)` | Run-wide declared setter registrations (`resource_callback_member_registered`, [C04.19.3](#c0419-callback-contracts)). |
| `#defaults` | `set(fn)` | Constructor-stated default callbacks (`quant live #result: TAG.FIELD \|-> f`, [C04.19.4](#c0419-callback-contracts)). |
| `#candidates` | `set(fn)` | Functions carrying an in-force candidate contract for their target member ([C04.19.7](#c0419-callback-contracts)). |
| `#loaded_registered` | `set(fn)` | Callbacks registered by runtime-loaded plugin code (`plugin.library.paths`, `flb_plugin_load`, `flb_plugin_proxy_create`, and transitive calls; empty in static scope). |
| `#enveloped` | `set(fn)` | Registrations admitted by pinned `rk_plugin_library` / `flb_external_plugin` outer contracts ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime)) via public setters meeting member candidate contracts. |
| `#punned_stores` | `set(fn)` | Callbacks written via `rd_kafka_anyconf_set_prop0`'s `_RK_C_PTR` type-punned offset store (`rdkafka_conf.c:2087-2088`, invisible to Clang member-store facts). |
| `#conf_copies` | `set(fn)` | Callbacks copied from the matching property of another config by `rd_kafka_anyconf_copy`. |
| `#hidden_string` | `set(fn)` | Callbacks written into `_RK_HIDDEN` callback properties via string-keyed `rd_kafka_conf_set`. |
| `#ut_handle_produce_response` | `set(fn)` | Singleton `{ ut_handle_ProduceResponse }`. |
| `#call` | `fn` | The declared function itself (its single execution is the call). |
| `#rd_calloc` · `#rd_malloc` · `#rd_kafka_toppar_new0` | `fn` | Those exact C callees (`rd.h:139-149`, `rd_kafka_toppar_new0`) as invoked during `#call`. |
| `#returns_result_of(x)` | `fn → eff` | Run returns, as its final value source, the result returned by an invocation of `x`. |
| `#asserts_result` · `#returns_nonnull` · `#failure_no_return` | `eff` | Run executes `rd_assert(res != NULL)` before returning; run's return value is non-`NULL`; run does not return normally on allocation failure. |
| `#audited_assertions` · `#deployed_assertion` · `#deployed_failure_call` | `prop` | Audited compile database omits `NDEBUG`; deployed binary preserves the analyzed assertion; deployed binary preserves its terminating (`no_return`) failure handler. |
| `#op_callbacks` | `set(fn)` | Op callbacks reachable during the call via `rd_kafka_op_handle`, `rd_kafka_op_call`, or `rd_kafka_op_destroy`. |
| `#served` | `set(obj)` | Singleton queue served by the call (`rkq` argument). |
| `#held_on(x)` · `#refs(x)` | `set(obj) → set(res)` | References to objects in `x` held by the caller at entry (`#held_on`) vs. held by anyone (`#refs`). |
| `#broker_monitor_cb` · `#monitor_add_registered` · `#coord_rkb_monitor_cb` · `#txn_coord_monitor_cb` | `set(fn)` | Callback in `rko_u.broker_monitor.cb`; callbacks registered across all `rd_kafka_broker_monitor_add` calls; `rd_kafka_coord_rkb_monitor_cb` (`rdkafka_broker.c:5497`); `rd_kafka_txn_coord_monitor_cb` (`rdkafka_txnmgr.c:3248`). |
| `#broker` | `set(obj)` | Broker designated by `rko_u.broker_monitor.rkb`. |
| `#handed_only(x)` · `#reads_only(x)` | `set(obj) → eff` | Every object argument passed to the run belongs to `x`; run mutates zero objects in `x`. |
| `#assign_cb` · `#assignor_add_registered` · `#range_assign` · `#roundrobin_assign` · `#cooperative_sticky_assign` | `set(fn)` | Assign callback invoked by `rd_kafka_cgrp_assignor_run`; callbacks registered via `rd_kafka_assignor_add`; built-in range, roundrobin, and cooperative-sticky assign callbacks. |
| `#metadata` · `#members` | `set(obj)` | Metadata (`rko_u.metadata.mdi`) and group members passed to the assignor. |
| `#op` · `#ack_batches` · `#ack_details` · `#first_add_lists` · `#nonempty` | `set(obj)` | Handed `rko` op; list at `rko_u.share_fetch_fanout.ack_batches`; lists returned by `rd_kafka_share_build_ack_details`; lists lazily created on first `rd_list_add`; lists containing `>= 1` element. |
| `#logq` · `#forwards(x)` · `#fwd_queue` · `#unforwarded` · `#last_released` | `set(obj)` / `set(obj) → set(obj)` | Client log queue `rk->rk_logq`; transitive `rkq_fwdq` closure during enqueue; queue ref acquired via `rd_kafka_q_fwd_get` and dropped via `rd_kafka_q_destroy` in `rd_kafka_q_enq1`; queues concurrently unforwarded or losing their last external ref during the call. |
| `#event_cb(x)` | `set(obj) → set(fn)` | Queue event callbacks enabled on queues in `x` during enqueue (`rd_kafka_q_cb_event_enable`). |
| `#cjson_allocate` · `#cjson_reallocate` · `#cjson_deallocate` · `#libc_hooks` · `#installed_hooks` | `set(fn)` | Handed `hooks->{allocate,reallocate,deallocate}`; default `{malloc, realloc, free}` (`cJSON.c:178-186`); hooks installed via `cJSON_InitHooks`. |
| `#fresh_or_null` | `eff` | Run returns a fresh unaliased allocation or `NULL`. |
| `#hook_allocated` · `#deallocate_releases` | `set(obj)` | Allocations returned by `hooks->allocate`/`reallocate` during the call; allocations freed when handed to matching `hooks->deallocate`. |
| `#cjson_arrays` | `set(obj)` | Live `cJSON_Array` items during the call. |
| `#children(x)` · `#owned(x)` | `obj → set(obj)` | Ordered finite child sequence of cJSON item `x`; exact owned-child custody set of `x`. |
| `#first(x)` · `#last(x)` · `#child(x)` · `#next(x)` · `#prev(x)` · `#next_reach(x)` | `obj → set(obj)` | First/last sequence element; `x->child`, `x->next`, `x->prev` (empty if `NULL`); positive transitive `next` closure from `x`. |
| `#pred(x, y)` | `obj, obj → set(obj)` | Elements immediately preceding an occurrence of `y` in `x`'s child sequence. |
| `#nodup(x)` | `obj → prop` | `x`'s child sequence contains no duplicate object identity. |
| `#live` · `#parent` | `set(obj)` · `obj` | Objects live at call entry; non-null `parent` argument object. |
| `#concurrent` · `#keeps_links(x)` | `set(fn)` · `obj → eff` | Concurrent steps run by other threads during the call; run writes neither `x->child` nor `next`/`prev` of any element in `#children(x)`. |

###### Block `SP3` — Fluent Bit chunk, collector, and output-plugin terms

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#result` | `set(obj)` | Object designated by the call's return value (empty for `NULL` or scalar returns). |
| `#interior(S)` | `set(obj) → set(obj)` | Objects owned directly or transitively by members of `S`. |
| `#chunk_name(S)` | `set(obj) → set(obj)` | ChunkIO `sds` name `ic->chunk->name` of each input chunk in `S`. |
| `#tracked` · `#claimed` | `set(obj)` | Objects tracked by the caller's resource projection; objects whose custody/lifetime is claimed by a clause of the declaration (excluding `resource_return_none`). |
| `#bound` · `#stored` | `set(obj)` | Objects bound to a caller variable/member/element at the call site; objects stored through a pointer or container outliving the statement. |
| `#faithful` · `#admitted` | `prop` | Declaration's result clause matches exact caller-acquired custody at the site; gateway admits the call under its all-`Borrowed` + `None`-return rule (O04-TRACE N6/N8). |
| `#config_of(S)` · `#ingesting` | `set(obj) → set(obj)` · `set(obj)` | Enclosing `struct flb_config` (`ins->config`) of input instances in `S`; configs with `is_ingestion_active` true when read. |
| `#collector` · `#event_registered` · `#set_running` | `set(obj)` | Collector matched by ID in `ins`'s collector list (`get_collector`); collectors registered on their event loop (`mk_event_add` / `mk_event_timeout_create`); collectors whose `running` flag the call sets true. |
| `#returns_zero` · `#production` | `prop` | Call returns `0`; call is dispatched by a running Fluent Bit engine's output path rather than a standalone test/harness. |
| `#in_tree` · `#synchronous` · `#proxy` · `#out_plugin(S)` | `set(obj)` / `set(obj) → set(obj)` | In-tree `plugins/out_*` `flb_output_plugin`s; plugins carrying `FLB_OUTPUT_SYNCHRONOUS`; external plugins registered via `flb_plugin_proxy`; plugin `ins->p` of each output instance in `S`. |

###### Block `SP4` — libevent / Monkey HTTP coroutine & scheduler terms

*(Reuses `#tracked` from `SP3`; record members and pointer slots are modeled as `obj` cells.)*

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#singleton(o)` | `obj → set(obj)` | `{ o }` (`quant all #x in #singleton(E): F` acts as `let #x = E in F`). |
| `#live_at_use` · `#released_only_by_call` · `#released` · `#ended` | `set(obj)` | Objects live at every read/write/release point during the call; objects released by nothing else before or after the call; objects released directly/transitively by the call; objects whose storage lifetime ends during the call. |
| `#inside(o)` · `#overlaps(S)` · `#stores(S)` | `obj → set(obj)` / `set(obj) → set(obj)` | Sub-objects embedded inside `o`'s allocation; objects overlapping storage of `S`; member cells of `S` written by the call. |
| `#interleaved` | `set(fn)` | Concurrent threads, signal handlers, or reentrant executions overlapping the call. |
| `#no_writes(S)` · `#no_releases(S)` · `#no_resumes(S)` | `set(obj) → eff` | Run writes no object in `S`; frees/deletes no object in `S`; switches to/resumes no coroutine in `S`. |
| `#unread_after` · `#unresumed_after` | `set(obj)` | Cells never read after the call returns; cells never used to resume a thread/coroutine after return. |
| `#mk_parent(o)` · `#thread_data(t)` · `#mk_callee(t)` | `obj → obj` / `obj → set(obj)` | `o->parent` (`mk_thread`); `MK_THREAD_DATA(t)` (`(char *)t + sizeof(struct mk_thread)`); coroutine `t->callee`. |
| `#co_deletable` · `#created_for(t)` · `#suspended` · `#co_active` | `set(obj)` / `obj → set(obj)` | Coroutines valid for `co_delete`; coroutines created for `t`; coroutines suspended during the call; running coroutine `co_active()`. |
| `#mk_session(o)` · `#mk_channel(s)` · `#mk_event(c)` · `#mk_type(e)` · `#mk_link(o)` | `obj → obj` | `o->session` (`mk_http_session`); `s->channel` (`mk_channel`); `c->event` (`mk_event`); scalar cell `e->type`; `mk_list` link cell `o->_head`. |
| `#lists(n)` · `#caller_list(n)` · `#circular_lists` · `#reciprocal` · `#nodes(l)` | `obj → set(obj)` / `obj → obj` / `set(obj)` | List heads whose cycle contains `n`; list head through which caller reaches `n`; valid circular `mk_list` heads; nodes satisfying `n->next->prev == n && n->prev->next == n`; all nodes (including head) of list `l`. |
| `#libco_params_key` · `#holds(c)` · `#held_at_return(c)` · `#sole_owned(c)` · `#getspecific_ok` · `#setspecific_ok` | `obj` / `obj → set(obj)` / `set(obj)` | Current thread's `mk_http_thread_libco_params` TLS slot; object held by cell `c` when read vs. at return; allocations solely owned by `c`; TLS slots where all `pthread_getspecific` / `pthread_setspecific` calls succeed. |
| `#aliases(t)` · `#stack_deregister(t)` | `obj → set(obj)` · `obj → set(fn)` | Cells still designating `t` or its storage at return (`request->thread`, `channel.thread`, `mk_thread` TLS key, etc.); `VALGRIND_STACK_DEREGISTER(t->valgrind_stack_id)` when `MK_HAVE_VALGRIND` is enabled (empty otherwise). |

###### Block `SP5` — S3 local-condition snapshot stability terms ([C04.21](#c0421-local-conditional-prerequisites-and-registered-lifetimes))

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#condition_reads` | `set(obj)` | Member cells read at entry by the declaration's `quant #NAME = (...)` condition bindings. |
| `#entry_stable` | `set(obj)` | Cells retaining their entry value at every guarded `[[local]]`-prerequisite use during the call. |

###### Block `SP8` — Runtime outer-contract statement terms ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime))

In `outer_contract_v1` definitions, every term is interpreted relative to the dynamically loaded code hosted by the contract's loaders (excluding instructions inside functions belonging to the audited program itself). In addition to the 36 terms below, `SP8` admits one registered constant family of sort `obj` (the sole constants permitted to exceed 64 bytes): **`CANDIDATES_<64-UPPERCASE-HEX>`**, naming the SHA-256 digest of a provider's `outer-candidates.json` (`CANDIDATE_INVENTORIES`, [C04.20.1](#c04201-runtime-list-callback-candidate-inventories)).

| Term | Signature / Sort | Exact model denotation |
|---|---|---|
| `#outer_code` | `set(fn)` | Functions in libraries opened by the contract's loaders (`rd_kafka_plugin_new`, `flb_plugin_load`, `flb_plugin_proxy_create`) plus all external/libc functions they invoke outside the audited program. |
| `#conf_registrations` · `#public_setter_values` · `#meets_candidate` | `set(fn)` | Callbacks stored by `#outer_code` into `rk_conf` callback members; values stored via public `rdkafka.h` setters (`rd_kafka_conf_set_*_cb`); callbacks satisfying their target member's declared candidate contract. |
| `#interceptors` · `#zero_appends(x)` · `#conf_lifecycle(x)` · `#client_lifecycle(x)` | `set(fn)` / `obj → set(fn)` | Interceptor methods registered into `rd_kafka_conf_s.interceptors`; elements appended via inventory `x`'s exact public setter and union arm on `0` return; elements under configuration vs. pre-init non-share client lifecycle in `x`. |
| `#on_live_conf` · `#from_on_new` · `#before_init` · `#on_share_consumer` | `set(fn)` | Interceptors installed on a live config, during an `on_new` run, prior to client init completion, or on a share consumer. |
| `#copies(x)` · `#valid_through_last` | `set(fn) → set(fn)` · `set(fn)` | `x` plus registrations copied via `rd_kafka_conf_dup` / `on_new`; registrations whose code and opaque context stay valid through final destruction of all holding configs/clients (including destroy callbacks; decision D3 B). |
| `#borrows_args` · `#borrows_handles` · `#no_global_custody` · `#balanced` | `eff` | Run (with all calls it makes) strictly borrows all pointer/reachable objects and scalar handles (`sockfd`), performs zero global custody changes, and frees all local acquisitions before returning. |
| `#stores_none(x)` · `#calls_none(x)` · `#accesses_only(x)` · `#calls_only(x)` | `set(obj/fn) → eff` | By its *own instructions*, run writes zero cells in `x`, calls zero functions in `x`, reads/writes only audited-program objects in `x`, or calls only audited-program functions in `x`. |
| `#rk_internal` · `#rk_public` · `#rk_nonpublic` · `#meets_published(x)` | `set(obj)` / `set(fn)` / `set(fn) → eff` | Internal librdkafka cells absent from `rdkafka.h`/`rdkafka_mock.h`; public vs. non-public librdkafka functions; every call to `x` satisfies published librdkafka documentation preconditions (decision D1 A). |
| `#cjson_init_hooks` · `#cjson_global_hooks` · `#cjson_link_cells` · `#cjson_handed` · `#allocator_hooks_premise` | `set(fn)` / `set(obj)` / `prop` | `{cJSON_InitHooks}`; `global_hooks` cells; `child`/`prev`/`next` cells of all `cJSON` items; `cJSON` trees passed by `#outer_code` to cJSON APIs; both statements of `cjson_allocator_hooks` hold on pre-load hooks. |
| `#flb_core_owned` · `#handed` · `#exported` | `set(obj)` · `set(obj)` · `set(fn)` | Fluent Bit core-owned cells of `flb_*_instance` and `flb_config`; loader-handed/reachable objects, exported-API returns, registered-callback args, and plugin-owned objects; symbols exported by Fluent Bit (`ENABLE_EXPORTS ON`). |
| `#provenance(x)` · `#typed_access` | `set(obj) → eff` · `eff` | Every dereferenced/stored/passed pointer derives from an object in `x`, and every access uses a C-compatible effective type with zero unrelated record-pointer casts (decision D2 B). |

---

##### First-order `G01p` typed-rendering tables (`SP6`, `SP7`, `SP9`, `SP10`, `MES`, `K2`, `K6`)

While `VOCABULARY` governs `G05-formula` premise statements, typed hypothesis atoms (`quant proposition`, [C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3)), guarded decomposition symbols ([C04.18.5](#c04185-explicit-guarded-decomposition-documents-version-1)–[C04.18.6](#c04186-guarded-production-source-embedding-source-version-1-bridge-version-2)), JSON environment bindings ([C04.18.2](#c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2)), and formal gateway admission/refusal conditions (rules `38k`, `79`, `81`) use `G01p` first-order predicate/function syntax validated against `RENDERED_VALUE_SORTS` and `RENDERED_TERMS` (`qcore_frontend::premise_formula::check_rendering` and `guarded_proposition`). None of these rendering blocks modify `VOCABULARY` or `G05-formula`.

###### Block `SP6` — Canonical `G01p` rendering of `VOCABULARY` terms (`QCore.CjsonTypedAtoms.render`)

A registered `VOCABULARY` term maps deterministically to a `G01p` parameter signature over value sorts `{obj, cls, fn, meth, res, run}`:

- **Element-sorted term (`obj`, `cls`, `fn`, `meth`):** Value parameter of that sort (`#parent: obj`), or `function<ARGS..., SORT>` when taking element arguments.
- **Set-sorted term (`set(S)`):** Predicate adding one trailing element parameter of sort `S` (`#children: predicate<obj,obj>`, where `#children(o, x)` means `x in #children(o)`).
- **Proposition term (`prop`):** Predicate over its element arguments (`#nodup: predicate<obj>`).
- **Effect term (`eff`):** Predicate adding one trailing `run` parameter (`#keeps_links: predicate<obj,run>`), paired with `#runs: predicate<fn,run>` (`#runs(#f, #r)` denotes `#r` is an execution run of `#f`), so `F |-> E` renders as:
  `forall (#f: fn) { (F(#f) => forall (#r: run) { (#runs(#f,#r) => E(#r)) }) }`
- *(Higher-order terms taking a `set(...)` argument have no `G01p` rendering.)* Under `QCore.CjsonTypedAtoms.render`, `cJSON_DetachItemViaPointer`'s nine `detach_*` typed atoms hold in an entry-induced model iff their `QCore.CjsonRepresentationPremises` definitions hold (`typed_atoms_are_the_definitions`), and their conjunction is equivalent to `detach_list_representation` (`detach_formula_is_the_nine_typed_atoms`).

###### Block `SP7` — `cJSON_DetachItemViaPointer` guarded-symbol terms (`QCore.CjsonGuardedSymbols.render`)

Evaluated over one selected call's entry/exit `Environment`, `Call` arguments, and result:

| Term | `G01p` sort | Meaning |
|---|---|---|
| `#item` · `#exit_parent` | `obj` | Object designated by `item` (meaningful when `!#item_null()`); `#parent` at call exit. |
| `#item_null` · `#parent_null` | `predicate<>` | Call's `item` or `parent` argument is `NULL`. |
| `#exit_live` · `#exit_nodup` · `#exit_erases` | `predicate<obj>` | `#live` and `#nodup` at exit; `#exit_erases(i)`: exit child sequence equals entry child sequence with first `i` erased. |
| `#exit_owned` · `#exit_children` · `#exit_child` · `#exit_first` · `#exit_last` · `#exit_next` · `#exit_prev` · `#exit_next_reach` | `predicate<obj,obj>` | Exit-state counterparts of the unprefixed `SP2` terms. |
| `#exit_pred` · `#exit_runs` | `predicate<obj,obj,obj>` · `predicate<fn,run>` | `#pred` at exit; `#runs` over the exit environment's interference trace. |
| `#admits_removed` | `predicate<>` | Gateway admits the call site under its `Removed` contract (`resource_owned`, `resource_removed`, `resource_return_removed`). |
| `#source_corresponds` | `predicate<>` | Independent assumption that actual C heap, complete interference trace, caller identity, and 9-coordinate `Context` match the model (`source_corresponds_is_never_derived`; always `Open`). |
| `#caller_refutes_membership` | `predicate<obj,obj>` | Positive caller evidence that `i` is **not** an owned child of `p` (`invalid_symbol_entails_invalid_membership`). |

- **cJSON guarded-symbol definitions (`guarded_symbols_are_the_definitions`):** `detach` = `#admits_removed()`; `environment` = `#source_corresponds()`; `failure_case` = `(#parent_null() || (#item_null() || (!#child(#parent,#item) && forall (#y: obj) { !#prev(#item,#y) })))`; `invalid_membership` = `((!#parent_null() && !#item_null()) && #caller_refutes_membership(#parent,#item))`; `membership` = `((!#parent_null() && !#item_null()) && (#live(#parent) && #owned(#parent,#item)))`; `original_list_unchanged` = entry/exit equality of `child`/`next`/`prev`; `remaining_list_valid` = the 9 `detach_*` formulas over `#exit_*` plus `#exit_erases(#item)` and empty exit `next`/`prev` on `#item`; `success_case` = the 9 `detach_*` formulas ∧ `membership` ∧ `#result(#item)`.
- **Explicit warrant-in-proposition boundary (`#caller_refutes_membership`):** By explicit design decision, `#caller_refutes_membership` embeds positive refutation warrant inside the formula rather than writing plain `!#owned(#parent,#item)`. It remains strictly fail-closed because: (1) it entails `!#owned(#parent,#item)`, (2) unproved membership never yields it (`absence_of_membership_is_not_refutation`, `caller_refutation_is_never_derived`), and (3) [C04.18.5](#c04185-explicit-guarded-decomposition-documents-version-1)'s `condition` rule independently requires both `Supported` invalidity and positively `Refuted` admission.

###### Block `SP9` — Plugin guarded-symbol sorts and terms (`QCore.PluginGuardedSymbols.render`)

`SP9` extends `G01p` rendering with function symbols (`function<S1,..., integer | S>`) and prefixes every sort and term with `plg_` (evaluating implicitly against one selected `QCore.PluginPremiseDecomposition.Environment`):

- **13 `plg_*` value sorts:** `plg_event`, `plg_plugin`, `plg_family`, `plg_action`, `plg_callback`, `plg_member`, `plg_context`, `plg_handle`, `plg_activation`, `plg_resource`, `plg_symbol`, `plg_digest`, `plg_abi`.
- **Trace & projection functions/predicates:** `#plg_in_trace: predicate<plg_event>`; event projections `#plg_action`, `#plg_plugin`, `#plg_function`, `#plg_member`, `#plg_opaque`, `#plg_handle`, `#plg_resource`, `#plg_activation`; integer event functions `#plg_time`, `#plg_result`, and `1`/`0` flags `#plg_public_setter`, `#plg_exact_candidate`, `#plg_pointer_borrowed`, `#plg_scalar_borrowed`, `#plg_local_resource`, `#plg_conf_live`, `#plg_from_on_new`, `#plg_initialized`, `#plg_share_consumer`, `#plg_config_registration`, `#plg_in_interceptor`.
- **Action & family constants:** Actions `#plg_invoke`, `#plg_register`, `#plg_intercept`, `#plg_acquire`, `#plg_release`, `#plg_retain`, `#plg_transfer`, `#plg_return_local`, `#plg_global_custody`, `#plg_close`, `#plg_destroy_callback`, `#plg_copy_reference`; families `#plg_rk`, `#plg_fluent_c`, `#plg_fluent_go`.
- **Environment & identity symbols:** `#plg_loaded: predicate<plg_plugin,plg_family>`, `#plg_loader_enabled: function<plg_family,integer>`, `#plg_actual_build` / `#plg_expected_build`, `#plg_actual_inventory` / `#plg_expected_inventory`, `#plg_actual_artifact` / `#plg_expected_artifact`, `#plg_required_symbol`, `#plg_resolved_address` / `#plg_expected_address`, `#plg_actual_abi` / `#plg_expected_abi`, `#plg_member_candidate`, `#plg_static_default`, `#plg_code_live`, `#plg_context_live`, `#plg_handle_for`, `#plg_ref_count`, `#plg_return_boundary`, `#plg_rk_proposed`, `#plg_fluent_proposed`.
- **Decomposition root & warrant symbols (`plugin.json`):** `#plg_ev: plg_event` (the reached `rk_conf` callback invocation of `rd_kafka_dump0`); `#plg_admits_callback_closure: predicate<>` (`admission_is_never_derived`); `#plg_trace_corresponds: predicate<>` (`trace_corresponds_is_never_derived`); and `#plg_refutes_registration: predicate<plg_event>` (warrant-in-proposition condition `unregistered`: entails absence of both public registration and static default, `a_case_excludes_refutation`, `absence_of_registration_is_not_refutation`, `registration_refutation_is_never_derived`). Notice `default_lifetime_valid` (`#plg_code_live(...) && #plg_context_live(...)`) intentionally omits plugin unload ordering (`lifetime_entails_default_lifetime`, `default_lifetime_does_not_give_the_unload_order`).

###### Block `SP10` — Typed JSON environment references (`QCore.FormalBindingEnvironments`, `QF005`)

Binding manifests ([C04.18.2](#c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2)) and flow models ([C04.18.3](#c04183-proposed-premise-flow-flow-version-1-schema-201)) represent each `environment.variables` entry either as a 64-lowercase-hex content digest or as a nonempty, strictly name-sorted canonical JSON array of `{"name","sort"}` `G01p` parameters drawn from bound atom parameters (whose unprefixed names like `#env: environment` stay local without reserving global names) or registered `SP6`/`SP7`/`SP9`/`SP10` rendered terms. Any prose description or non-matching quantifier/conclusion rejects with **`QF005`** (`every_reference_resolves`, `environment_vectors_agree`).

| Term | `G01p` sort | Meaning |
|---|---|---|
| `#bnd_source_corresponds` | `predicate<>` | Independent Open assumption that environment terms and bound Lean propositions correspond to actual C source and native execution (`binding_source_correspondence_is_never_derived`). |
| `#bnd_held_context` | `bnd_verdict_context` | The 9 independently held `HypothesisGraph.Context` coordinates (exact equality, zero cross-property transport). |
| `#bnd_resource_view` · `#bnd_null_view` · `#bnd_model_view` | `bnd_property_view` | Independently held `resource_contract_v1`, `bounds_contract_v1` null, or declared-model source view replayed for the graph. |
| `#plg_hook_install` · `#plg_hook_replace` · `#plg_cjson_link_write` · `#plg_cjson_call` · `#plg_core_write` · `#plg_internal_write` · `#plg_api_call` · `#plg_access` · `#plg_forge` | `plg_action` | Additional `PluginPremiseDecomposition.Action` constructors for cJSON hooks/links, core/internal stores, API calls, pointer accesses, and pointer forging. |
| `#plg_preexisting_hooks_valid` | `predicate<plg_plugin>` | Pre-load cJSON allocator hooks are valid (`Environment.preexistingHooksValid`). |
| `#plg_proper_list` · `#plg_is_rk_call` · `#plg_public_rk_symbol` · `#plg_api_precondition` · `#plg_handed_object` · `#plg_exported_api` · `#plg_forged` | `function<plg_event,integer>` | Corresponding Boolean `PluginPremiseDecomposition.Event` flags encoded as `1`/`0`. |

###### Gateway rule-formula rendering blocks (`MES`, `K2`, `K6`)

Three internal C-gateway rule families also validate their admission conditions and refusal reasons through `guarded_proposition` over dedicated prefixed blocks in `RENDERED_TERMS` / `RENDERED_VALUE_SORTS`:

1. **`MES` (`mes_*`, gateway rule [38k](agents/gateway.md#constraint-38k), `QCore.MemberHandoff`):** Formalizes caller-struct member-store custody hand-offs (`P->F... = V`), named `mes_refuse_*` refusals, and the stated row premise `mes_caller_slot`.
2. **`K2` (`k2_*`, gateway rule [79](agents/gateway.md), `QCore.TypeQueryPositions`):** Over sorts `k2_site`, `k2_event`, `k2_query`, and `k2_run`, defines `#k2_condition`, `#k2_older_refused`, `#k2_region`, `#k2_scalar_result`, `#k2_event_of`, `#k2_branch_event`, `#k2_object_read`, `#k2_unaliased`, `#k2_query_of`, `#k2_type_operand`, `#k2_variably_modified`, `#k2_run_at`, `#k2_name_star_sizeof`, and `#k2_declaration` (`type_query_positions::CONDITIONS`).
3. **`K6` (`k6_*`, gateway rule [81](agents/gateway.md), `QCore.AddressArrowPlace`):** Over sorts `k6_amp`, `k6_member`, and `k6_place`, defines `#k6_binds`, `#k6_old_binds`, `#k6_plain_binds`, `#k6_exempt`, `#k6_opens_group`, `#k6_name_arrow`, `#k6_selection_chain`, `#k6_group_arrow`, `#k6_outer_address`, `#k6_place`, `#k6_old_place`, `#k6_plain_place`, `#k6_root`, `#k6_refused`, `#k6_over_address`, and `#k6_over_member_address` (`address_arrow_place::FORMULAS`).


###### Measurement-connection block (`CX`)

(card QUIRIN-AUDIT-CONNECT; user decisions of 2026-11-26: every condition and refusal of `measure chain`, `measure compose` and the declared contract dependencies is a formula the premise-formula validator checks). Appended after the K6 block in the Rust table `RENDERED_TERMS` (`block: "CX"`) and, for its ten sorts, `RENDERED_VALUE_SORTS`: `cx_step` (a step of a chain), `cx_route`, `cx_row` (a function or unit row of a route), `cx_point` (a side of a chain), `cx_field` (an identity field a link compares), `cx_ident` (an identity value, or the unrecorded marker), `cx_verdict`, `cx_bundle` (a contract crate on a load path), `cx_fn` and `cx_dep` (a declared contract dependency). No premise formula names them, so the premise `VOCABULARY`, grammar, decision vectors and Lean `vocab` are unchanged. Every name is prefixed `cx_`. The 60 formulas (`qcore_schemas::connect::FORMULAS`; `crates/qcore-frontend/tests/connect_formulas.rs`) are checked by `guarded_proposition`, and every record names the conditions it applies by id. The Lean model is `QCore.MeasureConnect`: one definition per term (`cxTerms`) and one theorem per formula (`formula_*`, the formulas as written in `formulaTable`).

---

##### Canonical provider premise formulas and `.qv` examples

```qv
module contract;

// OpenSSL certificate-chain loader (stated security/teardown slots + ex_data & teardown premises):
extern fn SSL_CTX_use_certificate_chain_file(
    ctx: ptr<mut, u8>,
    file: ptr<const, u8>,
    type_: i32
) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(
        openssl_ex_data_callbacks,
        quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) |-> #inert
    );
    requires resource_premise(
        openssl_unstated_callback_slots,
        quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |-> #inert
    );
    requires resource_premise(
        openssl_builtin_teardown_functions,
        quant all #o in #freed: quant all #f in #teardown(#o): #f in #library -> #f |-> #inert
    );
    requires resource_premise(
        openssl_loaded_module_functions,
        quant all #o in #freed: quant all #f in #teardown(#o):
            #f in #loaded -> #f |-> #inert
            && quant all #g in #in_turn(#f, #freed | #args | #created): #g in #library | #loaded
    );
    ensures normal resource_borrowed(ctx);
    ensures normal resource_borrowed(file);
    ensures normal resource_borrowed(type_);
    ensures normal resource_return_none();
};

// Fluent Bit synchronous output enqueue:
extern fn flb_output_task_singleplex_enqueue(ins: ptr<mut, u8>, task: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(
        synchronous_output_reachability,
        (quant all #p in #in_tree: #p in #in_tree \ #synchronous)
        && (#production -> quant all #p in #out_plugin(#args): #p in #synchronous && #p in #proxy)
    );
    ensures normal resource_borrowed(ins);
    ensures normal resource_borrowed(task);
    ensures normal resource_return_none();
};
```

- **OpenSSL (`quirin-provider-openssl`, `QCore.PremiseFormula`):**
  - `openssl_ex_data_callbacks` (30 declarations): `quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) |-> #inert` (entailed by the handshake `openssl_unstated_callback_slots` formula whenever zero clauses state an `ex_data` callback, and not conversely).
  - `openssl_unstated_callback_slots` — *handshake + inner-free statement* (5 declarations): `#slots(#of(SSL, #args) | #ctx(#of(SSL, #args)) | #of(SSL_CTX, #args)) \ #stated |-> #inert && (quant all #l in #lookups(#store(#ctx(#of(SSL, #args)) | #of(SSL_CTX, #args))): #fns(#l) \ #stated |-> #inert) && (quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) \ #stated |-> #inert) && quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |-> #inert`.
  - `openssl_unstated_callback_slots` — *free statement* (`SSL_free`, `SSL_CTX_free`, `X509_STORE_free`, `PKCS12_free`, plus 9 inner-freeing loaders/readers): `quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |-> #inert` (note: once `EVP_MD_CTX_free` and `PKCS12_free` stated their slots through released parameters, the intermediate `quant all #o in #freed: #teardown(#o) \ #stated |-> #inert` form was retired from live declarations).
  - `openssl_builtin_bio_methods` (10 declarations, where `#ssl_of(#b) \ #args` captures a `BIO_f_ssl` `BIO`'s non-argument `SSL` object): `quant all #b in #chain(#of(BIO, #args) | #bios(#of(SSL, #args))): #method(#b) in #builtin -> #fns(#b) |-> #inert && quant all #f in #fns(#b): #in_turn(#f, #ssl_of(#b) \ #args) |-> #inert`.
  - `openssl_bio_free_callbacks` (2 declarations): `quant all #b in #of(BIO, #args) | #chain(#bios(#of(SSL, #args))): #b in #freed -> #ex_data_free(BIO) |-> #inert`.
  - `openssl_builtin_lookup_methods` (3 declarations): `quant all #l in #lookups(#of(X509_STORE, #freed)): #method(#l) in #builtin -> #fns(#l) |-> #inert`.
  - `openssl_builtin_teardown_functions` & `openssl_loaded_module_functions` (22 declarations each): as shown in the `.qv` block above.
- **libevent / Monkey (`qcore-provider-libevent`, `mk_http_thread_destroy`, `QCore.PremiseFormula.Libevent`):**
  - `scheduler_exact_parent_embedding`: `quant all #m in #args: #mk_parent(#m) in #live_at_use && #m in #thread_data(#mk_parent(#m)) && #m in #inside(#mk_parent(#m)) && #mk_parent(#m) in #released_only_by_call`.
  - `scheduler_tls_params_owned`: `#libco_params_key in #getspecific_ok && #libco_params_key in #setspecific_ok && quant all #p in #holds(#libco_params_key): #p in #live_at_use \ #overlaps(#tracked) && #p in #sole_owned(#libco_params_key) && #p in #freed \ #held_at_return(#libco_params_key)`.
  - `scheduler_callee_live`: `quant all #m in #args: quant all #c in #singleton(#mk_callee(#mk_parent(#m))): #c in #freed && #c in #co_deletable && #c in #created_for(#mk_parent(#m)) && #c in #suspended \ #co_active && #interleaved |-> (#no_resumes(#singleton(#c)) && #no_releases(#singleton(#c)))`.
  - `scheduler_borrowed_chain_live`: `quant all #m in #args: quant all #s in #singleton(#mk_session(#m)): quant all #ch in #singleton(#mk_channel(#s)): quant all #ev in #singleton(#mk_event(#ch)): #s in #live_at_use && #ch in #live_at_use \ (#released | #ended) && #ev in #live_at_use \ (#released | #ended) && #mk_type(#ev) in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)) && quant all #w in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)): #w in #singleton(#mk_type(#ev))`.
  - `scheduler_valid_membership`: `quant all #m in #args: quant all #n in #singleton(#mk_link(#m)): quant all #l in #singleton(#caller_list(#n)): #l in #lists(#n) && (quant all #k in #lists(#n): #k in #singleton(#l)) && #l in #circular_lists && (quant all #k in #nodes(#l): #k in #reciprocal) && #interleaved |-> #no_writes(#nodes(#l) | #singleton(#mk_callee(#mk_parent(#m))) | #singleton(#mk_session(#m)) | #singleton(#mk_channel(#mk_session(#m))) | #singleton(#mk_event(#mk_channel(#mk_session(#m)))) | #singleton(#libco_params_key))`.
  - `scheduler_no_later_alias_use`: `quant all #m in #args: quant all #a in #aliases(#mk_parent(#m)): #a in #unread_after && #a in #unresumed_after`.
  - `scheduler_valgrind_request_neutral`: `quant all #m in #args: #stack_deregister(#mk_parent(#m)) |-> (#no_writes(#tracked) && #no_releases(#tracked))`.
- **librdkafka (`qcore-provider-librdkafka`, 220 clauses across 12 names, `QCore.PremiseFormula.Librdkafka`):**
  - `rk_plugin_callback_closure` (180): `(quant all #f in #ran(#slots(#rk_conf)) \ #loaded_registered: #f in #registered | #defaults && #f in #candidates) && quant all #f in #loaded_registered: #f in #enveloped`.
  - `conf_property_writer` (18): `(quant all #f in #punned_stores: #f in #registered | #defaults | #conf_copies | #hidden_string) && quant all #f in #hidden_string: #f in #ut_handle_produce_response`.
  - `asserted_allocation` (9 across 4 variants): `#call |-> #returns_result_of(#rd_calloc) && #rd_calloc |-> #asserts_result && #audited_assertions && #call |-> #returns_nonnull` (5 `rd_calloc` wrappers, plus 1 via `#rd_kafka_toppar_new0` and 1 via `#rd_malloc`), and `#deployed_assertion && #deployed_failure_call && #call |-> #failure_no_return` (rule 37's `rd_calloc` and `rd_strdup`).
  - `served_queue` (2): `#op_callbacks |-> #takes(#held_on(#served)) == {}`.
  - `broker_monitor_callback` (1) & `assignor_callback` (1): exact registrar/callback closure (`#coord_rkb_monitor_cb` / `#txn_coord_monitor_cb`; `#range_assign` / `#roundrobin_assign` / `#cooperative_sticky_assign`), `#handed_only(#broker)` / `#reads_only(#metadata | #members)`, and zero reference effects on `#op` / handle / queue.
  - `share_ack_batches_nonempty` (1): `(quant all #l in #ack_batches: #l in #ack_details) && (quant all #l in #ack_details: #l in #first_add_lists) && quant all #l in #ack_batches: #l in #nonempty`.
  - `rk_log_enqueue_no_event_callback` (2): `(quant all #f in #event_cb(#forwards(#logq)): #false) && quant all #f in #ran(#event_cb(#forwards(#logq))): #false`.
  - `rk_log_enqueue_forward_not_last` (2): `quant all #q in #fwd_queue: (#q in #freed -> #false) && (#q in #unforwarded && #q in #last_released -> #false)`.
  - `cjson_allocator_hooks` (2), `cjson_list_invariant` (1: `quant all #a in #cjson_arrays: quant all #c in #child(#a): (quant all #l in #last(#a): #l in #prev(#c)) && ((quant all #p in #prev(#c): #false) -> #false)`), and `detach_list_representation` (1: 16 conjuncts matching `legacy_detach::REPRESENTATION_STATEMENT` and `QCore.CjsonRepresentationPremises`).
- **Fluent Bit (`quirin-provider-fluent-bit`, `QCore.PremiseFormula.FluentBit`):**
  - `untracked_handback` (1): `quant all #r in #result: #r in #interior(#args) && #r in #chunk_name(#args) && #r in #result \ (#tracked | #claimed) && (#faithful -> #r in #result \ (#bound | #stored)) && (#r in #result \ (#bound | #stored) -> #faithful) && (#admitted -> #r in #result \ (#bound | #stored))` (`untracked_admitted_faithful`).
  - `ingestion_active` (1): `quant all #g in #config_of(#args): (#g in #config_of(#args) \ #ingesting -> #returns_zero && quant all #c in #collector: #c in #collector \ (#event_registered | #set_running)) && (#g in #ingesting -> quant all #c in #collector: #c in #event_registered && #c in #set_running) && (#returns_zero -> quant all #c in #collector: #c in #set_running -> #g in #ingesting)` (`ingestion_core_entails_premise`).
  - `synchronous_output_reachability` (1): as shown in the `.qv` block above (`synchronous_output_not_in_tree`).

> **Implementation & Provenance.** `qcore_frontend::premise_formula` (`VOCABULARY`, `RENDERED_VALUE_SORTS`, `RENDERED_TERMS`, `check_rendering`, `guarded_proposition`), `qcore_contract::formal_binding`; grammar [G05-formula](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-formula-symbolic-premise-statements). Lean models: `QCore.PremiseFormula` (`.Model`, `.Outer`, `.Libevent`, `.Librdkafka`, `.FluentBit`), `QCore.CjsonTypedAtoms`, `QCore.CjsonGuardedSymbols`, `QCore.PluginGuardedSymbols`, `QCore.FormalBindingEnvironments`, `QCore.MemberHandoff`, `QCore.TypeQueryPositions`, `QCore.AddressArrowPlace`. Cards: `SP1-SYMBOLIC-OPENSSL-PREMISES` through `SP10-FORMAL-BINDING-DESCRIPTIONS`, `MEMBER-EFFECT-STORES` (rule 38k), `K2-NO-TYPED-ACCESS` (rule 79), `K6-DIRECT-POSITION-ACCESS` (rule 81).

---

<a id="c04181-evidence-only-hypothesis-graphs-version-1"></a>
#### C04.18.1. Evidence-only hypothesis graphs (version 1)

To track structured relationships among assumptions without conflating assumption bookkeeping with safety proof, every `resource_contract_v1` declaration elaborates an **evidence-only hypothesis graph** (`qcore_contract::hypothesis`).

##### Structured conjunctions (`resource_premise_all`) and retirement of prose atoms (`QF002`)

- **Retirement of `resource_hypothesis` (`QF002`):** Historical version-1 source files declared prose leaf atoms via `requires resource_hypothesis(ID, "STATEMENT");`. That clause head is **permanently retired**: `resource_hypothesis` under any introducer and with any payload rejects the **entire module** with **`QF002`** (`qcore_frontend::premise_formula::hypothesis_clause`), and the source/wire graph validators reject any user-declared atom carrying an opaque statement. User-declared atoms `MUST` be written as typed mathematical propositions (`quant proposition`, [C04.18.7](#c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3)); only compiler-generated `legacy_atom_*` nodes (elaborated from `resource_premise`, `literal_argument`, and rule-owned premises) carry opaque text. Frozen historical prose atoms are read exclusively via test-only `tests/support/legacy_prose_premise.rs`.
- **Structured conjunctions (`requires resource_premise_all(NAME, ID[, ID]*);`):** Introduces a named conjunctive premise over a nonempty, strictly increasing, duplicate-free list of declaration-local lowercase atom `ID`s (`legacy_` prefix is reserved for compiler elaboration; atom `ID`s and structured-premise `NAME`s occupy disjoint namespaces; contextual predicate introducing zero reserved source keywords). Missing child atoms, premise-to-premise edges, cycles, unreferenced declared atoms, duplicate premise names, or non-`requires` introducers reject the **entire module**. No disjunction (`any`), negation (`not`), threshold, evidence-acceptance, or independence combinators exist in graph v1 (and v1's empty typed-predicate registry rejected unknown predicates with zero opaque fallback).

##### Legacy erasure, digest independence, and strict wire canonicality

- **Exact legacy compatibility:** Every `resource_premise` and `literal_argument` elaborates to one `legacy_atom_*` preserving its exact legacy erasure, declaration SHA-256, `contracts.lock` pin, and semantic behavior; `resource_premise_all(NAME, id1, id2, ...)` erases to a declared premise with statement `all(id1,id2,...)`. [C15](#c15-contract-inclusion-and-inheritance) inheritance copies graph symbols unchanged under formal parameter renaming and enforces exact content agreement.
- **Independent graph digest and wire check:** Each graph carries its own versioned canonical-JSON SHA-256 alongside (never replacing) the declaration digest. Retained-text replay reconstructs and compares the complete graph, and wire readers reject noncanonical JSON encodings rather than repairing claims.

##### Nine-coordinate `Context`, disposition vs. warrant, and zero semantic discharge

1. **Exact 9-coordinate `Context` binding:** Every evaluated graph instance binds `(artifact, property, semantic profile, scope, bounds, assumptions, source identity, build identity, evidence version)` by **exact equality** against an independently held expected `Context`.
2. **Orthogonal disposition and warrant:** Each atom's **disposition** (`open`, `supported`, `refuted`) is strictly separate from its **warrant** (`derived`, `observed`, `declared`, `human-reviewed`, `provider-proposed`). Warrant labels never alter disposition and never grant certificate authority; reporting a conjunction requires compatible `Context` across every child atom, missing evidence stays `open`, and multi-argument relational propositions remain indivisible single atoms.
3. **Zero verdict promotion:** Graph v1 installs **no semantic consumer**: even `supported` + `derived` graph evidence discharges zero C safety obligations, and all published atoms start `open` with `declared` warrant. Gateway rule 13f's consultation closure (including validated W4 summaries and held/withdrawn names) lifts to per-row atom dependencies in audit sidecars (retaining graph definitions, dependent contexts, row counts, and `Safe`-row counts; stripping graph fields reproduces prior sidecars byte-for-byte). Runtime outer envelopes ([C04.20](#c0420-outer-contracts-code-loaded-at-runtime), rule 31) still key on legacy premise names and can only lower runtime verdicts.

> **Implementation & Provenance.** `qcore_contract::hypothesis`, `qcore_frontend::premise_formula::hypothesis_clause` (`QF002`); gateway rules 13f, 31. Card: `SP6-SYMBOLIC-HYPOTHESIS-ATOMS`.

---

<a id="c04182-neutral-premise-decomposition-and-explicit-binding-references-graph-version-2"></a>
#### C04.18.2. Neutral premise decomposition and explicit binding references (graph version 2)

Graph version 2 (public schema major **`20.0`**) allows a contract to attach a **proposed formal atom conjunction** alongside an existing `resource_premise` without claiming that the conjunction proves or replaces the original premise:

```qv
// qcore-premise-binding-manifest-v1 rd_kafka_dump0 rk_plugin_callback_closure <64-hex-sha256>
requires resource_premise(rk_plugin_callback_closure, FORMULA);
requires resource_premise_decomposition(
    rk_plugin_callback_closure,
    plugin_callback_candidates,
    plugin_loaded_artifacts,
    plugin_loaded_custody,
    plugin_loaded_lifetime,
    plugin_loaded_registration,
    plugin_loaded_unload,
    plugin_outer_bridge
);
```

##### Neutral `refinements` attachment and version-2 wire rules

- **Attachment semantics:** `requires resource_premise_decomposition(NAME, ATOM[, ATOM]*);` attaches a nonempty, strictly increasing list of non-legacy atom `ID`s to **exactly one** existing `resource_premise(NAME, ...)` on the same declaration. The original `resource_premise`, its generated `legacy_atom_*` singleton, and `DeclaredPremise` records remain completely untouched: no equivalence or implication edge is asserted between `NAME` and the proposed conjunction, and zero closed-world exemptions are granted.
- **Version 1 vs. version 2 encoding (`20.0`):** Declarations without refinements omit the `refinements` map and remain **graph version 1** (preserving exact prior bytes and SHA-256 digests). Nonempty `refinements` require **graph version 2** (version 2 with empty refinements, duplicate/missing/ambiguous/undeclared target `NAME`s, unused/missing atoms, or noncanonical JSON rejects; older readers refuse v2).
- **Separate composition (`compose_refinement` vs. `compose`):** Evaluates the proposed refinement conjunction separately from the legacy singleton under identical held-graph, 9-coordinate `Context`, and atom-digest checks. Neither `supported` nor `refuted` refinement metadata changes the legacy premise's disposition, static verdict, standing, or closed-world status (impact propagation over direct and validated-summary consultation closures requires exact `Context` equality and **never** constructs `Unsafe`).

##### Content-addressed binding manifests (`QF005`) and cross-property isolation

1. **Cryptographic binding manifests (`// qcore-premise-binding-manifest-v1 DECLARATION PREMISE SHA256`):** Comment metadata pinning a canonical version-1 JSON manifest that binds exact graph/atom digests, Lean model and audit paths/SHA-256s, qualified formal proposition IDs, typed `SP10` environment references or 64-hex content digests, exact `G01p` quantifiers and conclusions (rejecting any prose or mismatch with **`QF005`** across `hypothesis_bindings::resolve`, `resolve_bounds`, and `resolve_model`), and `source_binding: open`. The resolver replays source bytes and verifies hashes against **explicitly supplied** held graphs and artifact bytes (performing zero filesystem discovery); matching hashes establish **reference integrity only**, never derived truth or C source correspondence (`167` live manifests pinned, refreshed when `SP2` converted `VOCABULARY` formulas).
2. **Property-isolated `bounds_contract_v1` / null refinements (`PLUGIN-PREMISE-DECOMPOSITION`):** Same-name `resource_premise_decomposition` attachments are also admitted on `bounds_contract_v1` declarations (whereas `resource_premise_all` remains rejected there), adding zero solver facts or interpreted null/bounds clauses. Source replay, held graphs, manifests, and all 9 `Context` coordinates remain strictly property-specific: shared C function names never allow joining resource and null graphs, null publication follows only consulted `nonnull_member` through-records, declared `resource type` models replay without invented C source pins, and outer-envelope catalogs bind their own loader sources and candidate inventories independently. For registered resource/null declaration overlaps, neutral metadata admission reruns the full bounds loader and legacy-erasure check first and verifies that atom statements, refinement children, and targeted legacy erasures match the independently replayed resource declaration without transporting evidence dispositions or `Context`.

> **Implementation & Provenance.** `qcore_contract::hypothesis_bindings` (`resolve`, `resolve_bounds`, `resolve_model`), `qcore_contract::formal_binding` (`QF005`), public schema `20.0`. Lean model: `QCore.FormalBindingEnvironments`. Cards: `PLUGIN-PREMISE-DECOMPOSITION`, `SP2-SYMBOLIC-LIBRDKAFKA-PREMISES`, `SP10-FORMAL-BINDING-DESCRIPTIONS`.

---

<a id="c04187-typed-mathematical-atoms-and-model-entry-obligations-graph-version-3"></a>
#### C04.18.7. Typed mathematical atoms and model entry obligations (graph version 3)

Graph version 3 introduces first-order typed mathematical hypothesis atoms ([grammar `G01p`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)):

```qv
quant proposition #detach_nodup(#parent: obj, #nodup: predicate<obj>) = #nodup(#parent);
```

- **Sole user-declared atom form (`QF002`):** Following the retirement of `resource_hypothesis`, `quant proposition #ID(PARAMETERS) = FORMULA;` is the **only** legal user-declared hypothesis atom. Unlike historical string atoms (which coalesced identical duplicates), **each `#ID` is defined at most once per declaration — repeating `#ID` rejects the module even when byte-identical**. Value sorts, relation/function signatures, operands, and quantifier domains are explicit; integer arithmetic and comparisons denote unbounded mathematical integers (so C width/truncation bounds must be stated explicitly), and binding formal parameters to C states remains an explicit input obligation rather than a parser output.
- **Graph v3 `requirements` and fail-closed loader gate:** Hypothesis groups (`resource_premise_all`), refinements (`C04.18.2`), flows (`C04.18.3`), and lifecycle records (`C04.18.4`) may reference formal `#ID`s, and a v3 graph may combine generated opaque `legacy_atom_*` nodes with formal `quant proposition` atoms. In declared-model contracts, **ungrouped** formal atoms act as standalone entry obligations listed in the graph's sorted `requirements` array. Crucially, the strict `resource_contract_v1` and `bounds_contract_v1` loaders **reject declarations carrying direct ungrouped formal entry obligations** until an interpreted C source valuation is supplied — they **never** install a C summary by silently dropping a formal entry obligation! Model inspection APIs mint zero `Supported`/`Refuted` assignments, legacy implications, C state transitions, or verdicts.
- **Provider formalizations:** Covers `quirin-provider-libc`'s 8 reading atoms (`QCore.LibcReadingPremises`), `cJSON_DetachItemViaPointer`'s 9 `SP6`-rendered `detach_*` atoms (`QCore.CjsonTypedAtoms`), `rd_kafka_dump0`'s 7 `plugin_*` atoms, and CMetrics's 161 formal clauses across map/opts/typed-setter/metric-setter models (`QCore.CmetricsHypotheses`, splitting the 105 former prose prerequisites and repeated getter bundle into 9 distinct obligations per getter while preserving `NULL`/out-of-range no-op guards, separating count arithmetic from target size/loop bounds, modeling finite truncated floating-point conversions including negative fractions that truncate to unsigned `0`, and quantifying complete-call interference exclusion). Source/ABI/backend and Rust/Lean correspondence remain explicit open premises.

> **Implementation & Provenance.** `crates/qcore-frontend/src/quant_proposition.rs` (`G01p`). Lean models: `QCore.QuantProposition`, `QCore.LibcReadingPremises`, `QCore.CmetricsHypotheses`, `QCore.CjsonTypedAtoms`. Cards: `SP6-SYMBOLIC-HYPOTHESIS-ATOMS` (discharging `SP2-DETACH-HYPOTHESES-TYPED-ATOMS`).

---

<a id="c04188-explicit-entry-condition-roots-graph-version-4"></a>
#### C04.18.8. Explicit entry condition roots (graph version 4)

Whereas a `quant proposition` atom (`C04.18.7`) may exist solely to participate in proposed refinement metadata, `quant condition entry` ([grammar `G01c`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), [C03.1](#c031-formal-contract-hypotheses-and-conditions-migration-in-progress)) declares a mandatory **call-entry prerequisite**:

```qv
quant condition entry #non_negative(n: integer) = n >= 0;
```

- **Unconditional root retention in graph version 4:** Graph version 4 records each `quant condition entry` clause's bounded formula as an `entry-condition` atom and keeps **every `entry-condition` atom in the canonical sorted `requirements` root list regardless of whether groups or refinements also reference it** (`QCore.QuantCondition` proves regrouping preserves every explicit entry obligation; ungrouped `quant proposition` atoms retain v3's root rule, and graph versions 1–3 retain their exact encodings and root rules). Canonical validation recomputes the complete sorted root list from scratch and rejects omitted, extra, or duplicate roots, wrong graph versions, or malformed formulas.
- **Exit assertions vs. entry roots and loader enforcement:** Exit conditions (`quant condition normal | error(E) | any`) remain in the typed condition view and **never** become entry hypotheses; neither declaring a condition nor attaching a graph label establishes its truth. Interpreted `resource_contract_v1` and `bounds_contract_v1` loaders reject unresolved roots and unsupported condition clauses before installing any summary (legacy erasure never authorizes dropping an entry root, and graph inspection adds zero runtime valuations or C permissions).
- **Bounded `G01c` Boolean source reader (`QCore.BooleanConditionProfile`):** Separately from external loaders, the bounded Boolean source reader retains caller assumptions, binds exact typed entry identities, and evaluates supported mathematical `normal` assertions across complete finite integer-domain paths (establishing conditional model properties without claiming general C source correspondence).

> **Implementation & Provenance.** `crates/qcore-frontend/src/quant_condition.rs` (`G01c`). Lean models: `QCore.QuantCondition`, `QCore.BooleanConditionProfile`.

---

<a id="c04183-proposed-premise-flow-flow-version-1-schema-201"></a>
#### C04.18.3. Proposed premise flow (flow version 1, schema 20.1)

`requires resource_premise_flow_v1(ENTRY, EXPRESSION);` records a declaration-local **proposed propositional flow expression** (`qcore_contract::premise_flow`, public schema **`20.1`**), admitted in `resource_contract_v1`, `bounds_contract_v1`, and declared `resource type` model views *after* their authoritative loaders and legacy-erasure checks succeed (leaving graph v1/v2 bytes and digests unchanged and installing zero semantic C consumers).

##### Syntax, structural limits, and 3-valued flow operators

- **Parenthesization, namespaces, and hard bounds:** Every binary operator must be fully parenthesized; leaves are existing graph atom `ID`s or `flow(ENTRY)` intra-declaration references (`ENTRY` and `ATOM` occupy disjoint namespaces). Duplicate `ENTRY` definitions (even byte-identical), missing references, cycles, non-root `->|` terminals, or references to terminal entries reject the declaration. All entries and both operands of every binary operator (including unchosen arms and unreferenced entries) are validated within strict per-declaration budgets: **`<= 256` entries, `<= 128` recursion depth, `<= 4,096` expanded nodes, and `<= 16,384` tokens per clause** (exhaustion rejects with zero partial output).
- **Operator disposition table (`Supported`, `Refuted`, `Open`):**

| Parenthesized flow operator | Logical reading | `Supported` iff | `Refuted` iff | Otherwise | Boundary note |
|---|---|---|---|---|---|
| `(A \|-> B)` | Conjunction (`A ∧ B`) | `A` and `B` both `Supported` | `A` or `B` `Refuted` | `Open` | Outside `flow_v1`, `\|->` retains its state/formula maps-to meaning. |
| `(A \|\|-> B)` | Sufficient disjunction (`A ∨ B`) | `A` or `B` `Supported` | `A` and `B` both `Refuted` | `Open` | Propositional sufficiency only — never skips C path safety checks. |
| `(A !-> B)` | Guarded exclusion (`A ∧ ¬B`) | `A` `Supported` **and** `B` positively `Refuted` | `A` `Refuted` or `B` `Supported` | `Open` | Missing or `Open` `B` **never** proves `¬B`. |
| `(A ->\| B)` | Root-only terminal (`A ∧ B`) | `A` and `B` both `Supported` (sets `terminal = true`) | `A` or `B` `Refuted` | `Open` | Root-only (no continuation); proves zero C termination and hides zero open prerequisites. |

##### Cryptographic model binding and `evaluate_proposed`

Each version-1 flow document binds exact module/source SHA-256, declaration, property view, graph, and an **explicitly supplied** formal model (verified against `SP10` typed environment/binding rules with **`QF005`** on prose across `premise_flow::compile`, `compile_bounds`, and `compile_model`; zero filesystem lookup). `evaluate_proposed` checks all 9 independently held `Context` coordinates, document/graph/model digests, exact atom digests, and duplicate/conflict-free evidence across every branch (`derived` opaque and `provider-proposed` warrants are rejected; `observed`, `declared`, and `human-reviewed` warrants apply to proposed flow logic only, establishing zero legacy prose truth, standing, `Safe`, or `Unsafe` verdicts, while refutation propagation tracks direct/validated-summary dependents within the exact same `Context` without synthesizing `Unsafe`).

> **Implementation & Provenance.** `qcore_contract::premise_flow` (`compile`, `compile_bounds`, `compile_model`, `evaluate_proposed`), schema `20.1`. Card: `SP10-FORMAL-BINDING-DESCRIPTIONS`.

---

<a id="c04184-symbolic-lifecycle-metadata-source-version-1"></a>
#### C04.18.4. Symbolic lifecycle metadata (source version 1)

`resource_lifecycle_v1` records symbolic creation/introduction (`|^+`) and destruction/retirement (`|^-`) intent on abstract symbolic resources or declared graph atoms (`qcore_contract::contract_lifecycle::replay`):

```qv
requires resource_lifecycle_v1(create_entry, |^+ resource(slot_token));
requires resource_lifecycle_v1(retire_hyp,   |^- hypothesis(detach_nodup));
```

- **Lexical and structural rules (total-or-reject):** `|^+` and `|^-` are single atomic tokens (internal spaces reject); admitted solely under `requires` in `resource_contract_v1`, `bounds_contract_v1`, and declared `resource type` model views after property-specific graph validation (unknown/malformed `resource_lifecycle_v*` heads reject without fallback, while interpreted [C04.11](#c0411-the-paired-lifecycle-obligation) `resource_lifecycle(ctx, start|stop|destroy)` is outside that `v*` family and keeps its exact semantics). `ENTRY` and target `ID` match `[a-z][a-z0-9_]*` (`<= 128` bytes), `<= 256` entries per declaration, `<= 4,096` source bytes per clause, zero duplicate `ENTRY`s (even identical), and every entry is validated even if unused.
- **Disjoint target namespaces:**
  - `hypothesis(ID)` `MUST` name an already-declared atom in the same declaration's hypothesis graph (without inventing quantifiers/correspondence or relaxing unreferenced-atom and legacy-erasure checks).
  - `resource(ID)` names a literal symbolic identifier — **never** a C lvalue, parameter, allocation site, or custody fact (preserved literally without parameter renaming across [C15](#c15-contract-inclusion-and-inheritance) inheritance; container unlinking and ownership transfer never imply symbolic destruction).
- **Pure record standing:** `|^+` proves zero existence, freshness, liveness, or hypothesis support; `|^-` frees nothing, alters zero dispositions, proves zero negations, and performs zero dependency invalidations; no create-before-retire or linear-use ordering is asserted (allowing repeated target operations under distinct `ENTRY` IDs). `contract_lifecycle::replay` reruns the full selected loader and returns source-SHA-bound records (never an evidence receipt), adding zero wire fields to graph/flow schemas and altering zero verdicts.

> **Implementation & Provenance.** `qcore_contract::contract_lifecycle::replay`.

---

<a id="c04185-explicit-guarded-decomposition-documents-version-1"></a>
#### C04.18.5. Explicit guarded decomposition documents (version 1)

A standalone **`guarded_decomposition_v1;`** document ([grammar `G05-guarded`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), inspected via `qcore_contract::guarded_decomposition::replay`) specifies a structured proposed obligation decomposition over six explicit symbol roles (`environment`, `prerequisite`, `operation`, `condition`, `case`, `postcondition`), where every symbol's proposition is the canonical printing of a `C04.18.9` typed-rendering formula (`QF003` rejects prose or non-canonical propositions; `operation` denotes proposed admission eligibility, never observed execution, and listed graph-atom dependencies assert zero equivalences).

##### Four obligation shapes and 4-valued obligation status

| `G05-guarded` clause shape | Lowered logical obligation | Crucial distinction & fail-closed rule |
|---|---|---|
| `(environment ∧ prerequisite₁ [∧ ...]) \|-> operation;` | `admit(operation) → (environment ∧ all prerequisites)` | **Necessary** condition on admission (unlike `flow_v1`, true prerequisites never suffice to admit `operation`!). Positive `operation` with a `Refuted` prerequisite is `Refuted`; missing prerequisites keep it `Open`. |
| `invalidity !-> operation;` | `invalidity → ¬admit(operation)` | `Satisfied` requires **both** `Supported` `invalidity` **and** positively `Refuted` `operation` (missing/`Open` evidence or failed proofs supply neither). |
| `operation \|\|-> case₁ / case₂ [/ ...];` | `admit(operation) → (case₁ ∨ case₂ ∨ ...)` | Retains **every** declared case and dependency without claiming C-path exhaustiveness or mutual exclusion; one `Supported` case never masks another case's `->\|` check. |
| `case ->\| postcondition;` | `case → postcondition` | Evaluated and returned separately for **every** case; never an entry prerequisite and never a C termination claim. |

- **Obligation status vs. proposition disposition:** Positively `Refuted` antecedent → **`Inactive`** (never a safety proof); `Open` antecedent or active `Open` conclusion → **`Open`**; `Supported` antecedent + `Supported` conclusion → **`Satisfied`**; `Supported` antecedent + `Refuted` conclusion → **`Refuted`**.
- **Total-or-reject validation, 9-coordinate evaluation, and forward-only propagation:** Every `operation` has **exactly one** entry guard (`1–32` prerequisites) and **one** alternatives clause (`2–32` cases); every `case` belongs to one `operation` and has **one** `->|` terminal; zero duplicate clauses, prerequisites, cases, or unsorted/dangling atom dependencies; budgets are **`<= 65,536` document bytes, `<= 4,096` bytes per statement/proposition, `<= 256` clauses/symbols/dependencies per binding, and `<= 128`-byte `[a-z][a-z0-9_]*` identifiers**. `replay` reruns the full resource/bounds/declared-model loader without stripping unsupported clauses and verifies module, declaration, source SHA-256, graph SHA-256, and readable document SHA-256. Assignment evaluation enforces all 9 `Context` coordinates and rejects duplicate/conflicting/`Derived`/`provider-proposed` warrants across all branches. Impact propagation carries recheck requests strictly forward (`environment`/`prerequisite`/`condition` → `operation` → **all** `case`s → `postcondition`s in the same `Context`), never backward from terminals, and adds zero production semantic consumers, truth dispositions, or `Unsafe` verdicts.

> **Implementation & Provenance.** `qcore_contract::guarded_decomposition::replay`; grammar `G05-guarded`. Lean models: `QCore.GuardedContractApplications`, `QCore.CjsonGuardedSymbols`, `QCore.PluginGuardedSymbols`. Cards: `SP6-GUARDED-SYMBOL-PROSE`, `SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS`.

---

<a id="c04186-guarded-production-source-embedding-source-version-1-bridge-version-2"></a>
#### C04.18.6. Guarded production source embedding (source version 1; bridge version 2)

Inline `.qv` clauses allow a declaration to embed one `C04.18.5` guarded decomposition document and its complete symbol table directly inside `contract { ... }`:

```qv
requires resource_guarded_symbol_v1(environment, environment, #source_corresponds());
requires resource_guarded_symbol_v1(detach, operation, #admits_removed());
requires resource_guarded_decomposition_v1(
    entry_guard,
    (environment && detach_nodup) |-> detach,
    invalid_membership !-> detach,
    detach ||-> success_case / failure_case,
    success_case ->| remaining_list_valid,
    failure_case ->| original_list_unchanged
);
```

##### Unquoted symbolic syntax (`QF003`), loader checks, and `guarded_metadata_bridge` v2

1. **String-free inline clauses (`QF003`):**
   - In `requires resource_guarded_symbol_v1(ID, ROLE, FORMULA[, DEPENDENCY]*);` (`0+` strictly sorted atom dependencies; `ROLE` in `{environment, prerequisite, operation, condition, case, postcondition}`), `FORMULA` `MUST` be an unquoted `C04.18.9` `G01p` typed-rendering formula over `SP6`/`SP7`/`SP9` terms and rendered value sorts, stored as its canonical printing (`qcore_frontend::quant_proposition::print`).
   - In `requires resource_guarded_decomposition_v1(ENTRY, CLAUSE, ...);`, the document is written as unquoted `,`-separated `G05-guarded` clauses spelling conjunction `&&` (rendered by `guarded_decomposition::readable` into version-headed `;`-terminated lines with `∧` to compute the readable document SHA-256).
   - Any string literal anywhere in either clause (under any introducer) or any non-canonical proposition rejects the **entire module** on every route with **`QF003`** (`qcore_frontend::guarded_decomposition::parse_source_clause`, `replay_inline`, and `guarded_metadata_bridge::render`; historical prose texts for `cJSON_DetachItemViaPointer`'s 17 symbols and `plugin.json`'s 14 symbols are lifted solely inside test-only `tests/support/legacy_prose_premise.rs`).
2. **Whole-source validation and `guarded_metadata_bridge` version 2:** Resource, bounds, and declared-model loaders validate the complete symbol map and document against the declaration's original hypothesis graph before admitting the metadata (bounds/null view checks compare guarded maps alongside flow/lifecycle maps while keeping pure null views separate; `replay_inline` re-verifies whole-source SHA-256, graph, and metadata). The hash-held **`guarded_metadata_bridge` version 2** operates on strictly **one resource module and one declaration**, verifying exact `BASE`/`TIP` whole-source SHA-256s, deterministic insertion immediately after the target declaration's last standalone LF clause, byte-for-byte preservation of all other bytes, and exact equality of original graph maps, ABI maps, premise erasures, and every `ResourceContracts` field upon private-constructor recovery (refusing missing/extra/changed modules, generic hash waivers, or clause stripping; bridge v1 checks only its original insertion language).

##### Standing: Production adoption halted by user direction

> **Adoption halted (`GUARDED-CONTRACT-ADOPTION-SUPPLEMENT`, status: `deferred; not launched`).** Production adoption of inline guarded contract metadata is **explicitly halted at the user's request** ([STATUS.md](STATUS.md), [supplemental restart card](reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md)):
>
> - During the frozen `BASE` (`31c5ed500d9d4a1448a623b839e1644adb19efd1`) / `TIP` (`552773f0c64e2e6a3c7d4b752c61a7d26130975a`) native verification campaign, five of the six required route measurements (`BASE` system/slice/corpus and `TIP` system/slice) completed before `TIP` corpus was intentionally interrupted by the user halt; **no complete comparison or native acceptance exists**, and subsequent `main` merges constitute a separate verification scope.
> - Resuming any production adoption requires **explicit human authorization**, restoration or fresh scoping of the pinned runtime capsules, official `contracts.lock` and `contract-inventory.lock` repinning, and zero unexplained differences across all six route outputs and comparators.
> - While the syntax, `QF003` prose ban, `replay_inline` validator, and Lean conditional recovery models remain tested in-tree, **inline guarded metadata carries zero production semantic, lifecycle, verdict, or acceptance authority**, and source universals, deployed callbacks, executable lifecycles, and C termination remain `Open`.

> **Implementation & Provenance.** `qcore_frontend::guarded_decomposition::parse_source_clause` (`QF003`), `qcore_contract::guarded_decomposition::replay_inline`, `qcore_contract::guarded_metadata_bridge` (v1 & v2). Cards: `SP6-GUARDED-SYMBOL-PROSE`, `SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS`, `GUARDED-CONTRACT-ADOPTION` ([supplemental card](reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md)).

---

### Cluster C04-D — Higher-Order Callback Contracts and Runtime-Loaded Outer Contracts

Higher-order control flow in systems C breaks first-order call-graph modularity in two distinct ways:

1. **In-program indirect dispatch (`C04.19`).** Functions accept raw function pointers, dispatch through arrays of operations, install destructors or event handlers into long-lived holder records, or store callbacks inside struct fields and opaque library contexts (`SSL_CTX`, `rd_kafka_op_s`, `flb_input_collector`). Treating a `fnptr` argument or struct field as a benign borrowed scalar would silently miss every resource acquisition, release, or ownership transfer performed by the target function.
2. **Out-of-program dynamic loading (`C04.20`).** Plugin loaders (`dlopen`-backed C shared objects, Go proxy plugins, runtime interceptor libraries) hand internal configuration or context pointers across an executable boundary to code absent from the static compile database. Any closed-world invariant established over static translation units—such as which callbacks can be registered in a struct field or whether a pointer member stays non-null—can be invalidated by dynamically loaded code unless explicitly bounded by a reviewable outer contract.

Cluster C04-D specifies the fail-closed contract vocabularies, candidate-closure rules, body checks, and static/runtime projection boundaries that govern both mechanisms.

<a id="c0419-callback-contracts"></a>
#### C04.19. Callback contracts

##### Conceptual model of higher-order C callbacks

Resource-contract v1 models indirect calls over the core's `callback_trace` protocol (`QCore.IndirectCalls`, gateway rule 11p), which enforces three temporal invariants on callback lifecycles:

* **Registration requires liveness:** registering a callback into a context or holder requires a live holder with no prior teardown.
* **Invocation requires active registration:** invoking a registered callback requires both that the registration remains in force and that the passed context is live.
* **Release requires prior unregistration:** releasing a context or holder requires any active callback registration on it to be unregistered first (and any elements owned by a releasing registry to be released before the holder is destroyed).

Across C libraries, higher-order callback patterns fall into three structural Dispatch Models, all unified under the C04.19 clause family:

1. **Direct and table-driven synchronous invocation:** The caller passes a `fnptr` parameter (`each(reader, cb, ctx)`), an optional nullable `fnptr` with built-in fallback (`PEM_read_bio_PrivateKey`), or a pointer to a constant dispatch table (`T`), and the callee invokes it zero or more times *during that call only* without retaining the pointer afterward.
2. **Path-tracked holder registries and element custody:** A constructor (`list_new(size, free_cb)`) or initializer (`msgpack_packer_init`) stores a callback inside a tracked holder `H`, later calls add elements (`list_add(rl, elem)`), invoke the registered callback over elements or arguments (`resource_callback_registry_invoked`), drop element custody, unregister the callback, and conditionally free `H` depending on whether `H` was heap-constructed or initialized in place (`rd_list_new` vs. `rd_list_init`).
3. **Struct-member and opaque-slot callbacks (`C04.19.1`–`C04.19.7`):** A callback is stored inside a named struct field (`rd_kafka_op_s.rko_op_cb`, `rd_kafka_conf_s.dr_msg_cb`) or an incomplete opaque library record (`ssl_ctx_st.cert_cb`, `evp_md_ctx_st.*`). Because path-local tracking cannot follow arbitrary heap stores across queues and worker threads, these invocations are resolved against a **run-wide closed candidate set** (narrowed where available by per-path constructor state), composed with union-discriminant teardown arms, return-code conditional ownership hand-offs, and per-member release/move states.

```qcore
module contract;

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

##### Actual-argument descriptors (`A…`)

In every callback invocation clause, the positional argument list `A…` specifies, argument by argument, what the callee passes to the callback at each invocation:

| Descriptor token | Admitted clause forms | Meaning at the callback's parameter position |
|---|---|---|
| `PARAM` (a parameter name of the enclosing declaration) | All invocation forms | The caller's own actual argument passed at `PARAM`, forwarded unchanged to the callback. |
| `_` | All invocation forms | An internal value supplied by the callee (an integer index, an interior pointer computed into an object handed to the callee, or a scratch buffer owned by the callee) that is non-null whenever a candidate requires a live object. |
| `supplied_nullable` | Wherever `_` is admitted (not in opaque slots unless `_` rules admit; rejected as a parameter name) | A callee-supplied value documented as **possibly NULL** (`QCore.CallbackSuppliedNull`). Candidates requiring `resource_owned(nonnull)` at this position are refused (`callback-supplied-null`); only `resource_borrowed` or `resource_owned(nullable)` is admitted. Imposes no requirement on the outer call, and forwarding matches it only against `supplied_nullable`. |
| `element` | `resource_callback_registry_invoked` only | Each element currently held by the registry holder `H`. This is the sole position where a callback candidate may declare `resource_released` (acting as the container's element destructor). |
| `supplied_owned` | `resource_callback_invoked` and `resource_callback_member_invoked` only (at most one per list; [C04.19.1](#c04191-an-object-the-callee-hands-its-callback)) | A live, callee-owned object whose custody the callee hands over to the callback on each invocation (such as a dequeued operation record). |

##### Summary of callback clauses, roles, and effects

The table below covers both the core C04.19 registry/synchronous clauses and the struct/slot extensions specified in `C04.19.1`–`C04.19.6`:

| Clause | Role | Custody and lifecycle effect | Body-check vs. assumption standing |
|---|---|---|---|
| `resource_callback_invoked(F, A…)` | Synchronous direct dispatch | During the call only, function pointer `F` is invoked zero or more times with `(A…)`; nothing retains `F` after return. | Checked against defining C body (rule 13y); mismatch withdraws (`callback-withdrawn`). |
| `resource_callback_invoked_or_default(F, A…)` | Synchronous dispatch with NULL fallback | Behaves as `resource_callback_invoked(F, A…)` when `F` is non-null. When the actual is a literal null pointer constant, `F` is never called and the callee executes its own default behavior described by the declaration's remaining clauses. | Withdrawn in any unit defining the function (rule 13y, `callback-withdrawn`); never forwarded by rule 15w. |
| `resource_callback_table(T, A…)` | Synchronous vtable/array dispatch | During the call only, function pointers read by the callee from the array designated by `T` are invoked zero or more times with `(A…)`. | Pin-governed declared assumption (rule 13i). |
| `resource_callback_registered(F, H)` | Holder callback installation | After return, holder `H` (either an in-place borrowed pointer parameter or `result` for a constructor) retains `F` (or no callback if `F` is a null constant), replacing any prior registration on `H`. | Parameter holder `H`: checked for `H->M = F;` stores (rule 13y). Constructor `result`: declared assumption (rule 13i). |
| `resource_callback_registry_invoked(H, A…)` | Holder callback invocation | During the call, whatever callback `H` currently holds (if any) is invoked zero or more times with `(A…)`. | Pin-governed declared assumption (rule 13i). |
| `resource_callback_unregistered(H)` | Holder callback clearing | After return, `H` holds no callback registration. | Pin-governed declared assumption (rule 13i). |
| `resource_element_added(E, H)` | Registry element insertion | `E`'s target becomes an element of `H`: custody transfers into `H` if `H`'s active callback releases `element`, otherwise `H` retains `E` under C04.14. | Pin-governed declared assumption (rule 13i). |
| `resource_elements_dropped(H)` | Registry element clearing | After return, `H` holds no elements (requires any owned elements to have been released by invocation first). | Pin-governed declared assumption (rule 13i). |
| `resource_released_if_constructed(H)` | Conditional holder destruction | Releases `H` if and only if `H`'s active registration originated from a `result` constructor (e.g., `rd_list_new` setting `RD_LIST_F_ALLOCATED`), and leaves `H` live if initialized in place (`rd_list_init`). | Pin-governed declared assumption (rule 13i). |
| `resource_callback_member_registered(H, TAG.FIELD, F)` | Struct-member setter / constructor registration (`C04.19.3`, `C04.19.4`) | Contributes `F` (or `#absent` / default function designator on `result`) to the run-wide candidate closure (and per-path constructor state) for `TAG.FIELD`. | Declared assumption (except `#absent` constructor body check in rule 14e, and `#id` forwarding check in rule 12n). |
| `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)` | Struct-member callback dispatch (`C04.19.3`) | Invokes candidates from the run's closed store set for `TAG.FIELD` (or decided constructor state) zero or more times with `(A…)`. | Pin-governed declared assumption (rule 13i). |
| `resource_callback_slot_registered(P, TAG.FIELD, F)` / `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)` | Opaque library-record callback dispatch (`C04.19.3`) | Registers or invokes callbacks on an incomplete/opaque C struct (`SSL_CTX`, `EVP_MD_CTX`, `BIO_METHOD`) over the run's closed setter registrations under `P-OPAQUE-SLOT-WRITERS`. | Pin-governed declared assumption (rule 13i). |

##### Grammar, placement, and function-map withholding

All callback clauses obey total-or-reject loading under grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1):

* **Parameter types and uniqueness:** `F` MUST name a `fnptr` parameter, and any single `fnptr` parameter MAY be named by at most one invocation or registration clause. `T`, `H`, and `E` MUST name depth-one pointer parameters declared `resource_borrowed` with no resource kind.
* **Reserved actual identifiers:** In `A…`, a parameter actual MUST name a parameter other than the invoked callback itself. If a declaration has a formal parameter named `_`, `element`, `supplied_nullable`, or `supplied_owned`, every callback clause on that declaration is rejected. Using `result` as a holder requires `ensures normal resource_return_owned(nonnull);` and is rejected if any parameter is named `result`.
* **Per-declaration co-occurrence restrictions:**
  * An element `E` MUST NOT be its own holder (`E != H`) and MUST NOT also appear in a `resource_retained_by` clause.
  * A declaration carries at most one clause of each callback keyword per position, and MUST NEVER carry both `resource_callback_registered(F, H)` (for a parameter `H`) and a registry use/unregistration clause on `H` in the same contract.
  * Callback clauses MAY stand wherever a retention clause may stand (C04.14) and MAY stand alongside retentions (for example, `msgpack_packer_init` both retains `data` and registers `callback` in `pk`).
  * Callback clauses MUST NEVER stand beside a slot-move or derived-out-slot clause (C04.15–C04.16), and MAY stand beside a literal-argument premise (C04.5) *only* when every callback clause is a synchronous invocation.
* **Co-occurrence beside reference-counting clauses (`QCore.OpensslFreeInvocations`):** By default, callback clauses alongside reference-count clauses (C04.17) are rejected. As the sole exception, an opaque slot invocation (`resource_callback_slot_invoked`, [C04.19.3](#c04193-a-registration-carried-by-the-object)) MAY stand beside reference clauses when **every** reference clause on the declaration is a reference drop and **every** callback clause is `resource_callback_slot_invoked` (e.g., OpenSSL teardown routines that decrement a reference count and invoke session-removal slots on the final drop). A reference keep, plain member invocation, synchronous invocation, table dispatch, registration, or registry clause beside any reference clause still rejects the module. At a call site of such a drop+slot declaration:
  1. The consumer MUST evaluate what each slot candidate requires of the dropped object *before* applying the reference drop (in the C gateway, the callback route executes before the reference route), including the requirement that the caller hold the live object prior to the drop.
  2. Because only the final reference drop executes the slot at runtime and static call analysis does not decide whether a given drop is the last, the consumer MUST NOT apply any candidate effect that mutates or depends on the dropped object's post-state (such as member-path states from [C04.19.6](#c04196-a-release-or-move-out-at-a-member-path) or constructed-state readings from [C04.19.4](#c04194-a-constructors-result-state)); if a candidate carries such an effect on the dropped object, the call MUST be refused.
* **Withholding from the plain function map:** Every declaration carrying any callback clause is **withheld from `ResourceContracts`'s ordinary function map** alongside retaining declarations. Consequently, any consumer that does not implement higher-order callback semantics sees an uncontracted callee and fails closed to `Unknown`—**no callback is ever silently treated as a benign borrowed pointer**.

##### Normative consumer obligations (gateway rules 13u–13x)

Any consumer interpreting callback contracts MUST enforce all five of the following rules:

1. **Closed candidate-set resolution (`callback-undecided`).**
   * For a `fnptr` actual, the consumer resolves a closed set of target functions over Clang's AST (rule 11p): a direct function designator, a definitively resolved local variable, or an entry from a constant dispatch table. Where a single source line expands to multiple Clang call expressions for the same callee (such as glibc's `__glibc_const_generic` macro generating one `bsearch` call per `_Generic` association), every call on that line is inspected and the resolution stands only when all of them agree.
   * For a table dispatch `resource_callback_table(T, A…)`, `T` resolves to a closed set either as a constant static table or from every function designator in the braced initializer of an automatic local array, provided the function body names that array *only* as whole-table actual arguments to declared table dispatchers (string literals and null entries in those initializers are never called, under premise `P-CG22-DEFINED-POINTER-VALUE`).
   * A literal **null pointer constant** passed to `resource_callback_registered` registers no callback (empty candidate set). Passed to `resource_callback_invoked_or_default`, a literal null pointer constant invokes nothing and reduces the call to its remaining custody clauses with no candidate check; this exemption applies **only** to a syntactic null pointer constant at the call site, never to a local variable or formal parameter that might hold `NULL`.
   * In all other positions—passing null to `resource_callback_invoked`, or passing an unresolved formal parameter, a pointer-dereferenced struct member, a cast expression, or a function call's return value—the candidate set is **open** and the call MUST be refused (`callback-undecided`).
   * Because no body check can prove what a callee's fallback code does when `F` is null, in any translation unit that defines a function carrying `resource_callback_invoked_or_default`, that declaration is withdrawn (`callback-withdrawn`, rule 13y), and formal-forwarding (rule 15w) never forwards a formal to or from such a declaration.

2. **Loop-safe identity composition and parameter requirement folding (`QCore.CallbackRequirementFold`, `QCore.CallbackSuppliedNull`).**
   * Every function in the resolved candidate set MUST have a contract in force: a declaration in the function map, an independently validated in-unit W4 summary (including unselected `static` internal-linkage helper definitions reached by rule 4a's derivation scope when passed as callback designators), or an imported cross-unit summary under rule 4c's link premise. Any candidate lacking a contract in force refuses the call (`callback-uncontracted`).
   * Because an invocation runs **zero or more times**, each candidate's effect on caller-visible custody MUST be idempotent—specifically, the **identity** on the caller's resource state: exact parameter-to-actual arity match, every parameter `resource_borrowed` or `resource_owned` with no resource kind, `resource_return_none()`, and no container or counted-user clauses. Any candidate that acquires, consumes, stores, or releases caller state (with the two explicit exceptions of registry `element` destructors below and `supplied_owned` in `C04.19.1`) refuses the call (`callback-effectful`). **A consumer MUST NEVER publish `Safe` by assuming an unknown or effectful callback is benign.**
   * `resource_owned` on a callback candidate is a *liveness/non-nullness requirement*, not a state-consuming effect:
     * At `_` or `element`, the callee satisfies `resource_owned` with the live object it supplies.
     * At `supplied_nullable`, the callee may pass `NULL`; hence a candidate requiring `resource_owned(nonnull)` is refused (`callback-supplied-null`), whereas `resource_borrowed` or `resource_owned(nullable)` succeeds without constraining the caller's arguments.
     * At a `PARAM` actual where the outer declaration declares `PARAM` as `resource_borrowed`, the outer call itself is strengthened at `PARAM` to `resource_owned(n)`, where `n` is `nullable` if and only if *every* candidate's requirement on that position is nullable—matching a direct call to the candidates.
     * At a `PARAM` actual where the outer declaration does **not** merely borrow `PARAM` (gateway rule 74j, across all invocation forms including member and slot invocations): an `owned`, `consumed`, `released`, or `stored` outer mode retains its outer custody mode, remaining `nullable` only when both the outer mode and every candidate requirement on that position allow null; an `out` or `removed` outer position holds no incoming caller object to satisfy a candidate's requirement and MUST refuse the call (`callback-requirement-unfolded`).
   * Under these rules, zero, one, or arbitrarily many invocations of any mix of admitted candidates compose to the identity on custody state after strengthening the call's entry requirements.
   * **Registry destructor exception:** At an `element` position of `resource_callback_registry_invoked`, candidates MAY declare `resource_released`—modeling a container's element-freeing callback.

3. **Path-sensitive holder registries (`callback-registry-undecided`).**
   * A `resource_callback_registered(F, H)` call records the candidate set on the current CFG path when `H` names a trackable holder: either `&L` for an automatic, non-pointer, non-array local `L` that appears in the function body *only* as whole `&L` actual arguments to holder-naming callees, or a single tracked live resource identity (including a heap constructor's `result`).
   * Any subsequent `resource_callback_registry_invoked`, `resource_callback_unregistered`, `resource_element_added`, `resource_elements_dropped`, or `resource_released_if_constructed` call on `H` requires a registration to be in force on *every* control-flow path reaching the call. Path joins preserve a registration only when arriving paths agree; a disagreeing join, an unregistered path, or an untrackable holder (such as a formal parameter or struct member) refuses the call (`callback-registry-undecided`).
   * When `resource_callback_registry_invoked(H, element)` executes: if every registered candidate is the identity, element custody is unchanged; if every registered candidate releases `element`, every element whose custody `H` took is released; any mixed or other candidate set refuses the call.

4. **Element custody and container lifetime (`callback-element-custody-undecided`).**
   * Adding an element (`resource_element_added(E, H)`) to a **releasing** registry backed by a tracked heap identity places `E` into `H`'s containment (returning or escaping `H` carries `E` with it, and invoking `H`'s registry releases `E`).
   * Adding an element to a **releasing** stack-frame registry (`&L`) MUST be refused (`callback-element-custody-undecided`), because an automatic frame struct has no transferable resource identity to own dynamic child allocations.
   * Adding an element to a registry with **no callback** (null registration) or an **identity callback** retains `E` by `H` under C04.14's retention decision table.
   * Plainly releasing `H` while its registry still owns unreleased elements MUST be refused (never treated as implicitly freeing elements), and executing `resource_elements_dropped(H)` while owned elements have not been released MUST likewise be refused.

5. **Flag-conditioned self-release.**
   * `resource_released_if_constructed(H)` applies after the registry clauses on the same call: if the registration that initialized `H` came from a `result` constructor, `H` itself undergoes an ordinary `resource_released`; if `H` was registered in place via a parameter holder, `H` is not released.

##### Body validation vs. pinned assumptions (rule 13y) and call-graph ordering (rule 13u)

* **Body validation (rule 13y):** When an analysed translation unit defines a function carrying callback clauses:
  * `resource_callback_invoked(F, A…)` stands only if the C body uses formal `F` *exclusively* as the callee of direct calls (`F(…)` or `(*F)(…)`) whose actual arguments positionally match `A…` (for a `PARAM` word: that exact formal parameter, never rebound and never address-taken; for `_`: any expression that does not mention `F`).
  * `resource_callback_registered(F, H)` on a parameter holder `H` stands only if the C body uses `F` *exclusively* as the right-hand side of member stores `H->M = F;` (occurring at least once), with neither `F` nor `H` ever rebound.
  * If a defining body fails this validation (or if the declaration carries `resource_callback_invoked_or_default`), the declaration is **withdrawn** in that translation unit: every call to it in that unit is refused (`callback-withdrawn`), and W4 summary inference MUST NOT substitute an inferred summary in its place. Note that rule 13y validates *how the callee uses the function pointer*, whereas the callback's own resource behavior comes from each candidate's contract.
  * Every other callback clause—`resource_callback_table`, registry operations, element custody, and constructor `result` registrations—is a pinned declared assumption (C06.2) with no body-evidence coverage (rule 13i).
* **Bottom-up summary ordering (rule 13u):** Passing an in-unit function designator to a synchronous invocation or table dispatch creates a dependency edge in W4's call graph so that the callback candidate is summarized *before* the caller that passes it.
* **Explicitly out-of-vocabulary shapes:** Passing value-returned streams (`pb_write` over `pb_ostream_from_buffer`) and returning retained values out of a holder (`pthread_getspecific`) remain outside C04.19 and fail closed.

**Implementation & Provenance.** Gateway rules 11p, 13i, 13u–13z, 15w, and 74j (`crates/qcore-contract/src/contract_loader.rs`, `crates/qcore-c-gateway/`); grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean models: `QCore.IndirectCalls`, `QCore.CallbackContracts`, `QCore.CallbackSuppliedNull`, `QCore.CallbackRequirementFold`, and `QCore.OpensslFreeInvocations`. Cards: `CG22-CALLBACK-REGISTRY-CONTRACT`, `CG22-CALLBACK-CANDIDATE-CONTRACTS`, `CALLBACK-SUPPLIED-NULL-ACTUAL`, `CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD`, `OPENSSL-PEM-CALLBACK-ARM`, and `OPENSSL-FREE-SESSION-REMOVE-INVOCATION`.

<a id="c04191-an-object-the-callee-hands-its-callback"></a>
##### C04.19.1. An object the callee hands its callback

Event-queue dispatchers—such as librdkafka's `rd_kafka_q_serve` family—dequeue an internally owned heap object (an `rd_kafka_op_t *`) and transfer ownership of that object to the invoked callback on every iteration. Because the object never belonged to the outer caller before the call, the outer `serve` call remains custody-neutral on the caller's state, yet safety requires proving that the callback actually accepts custody of (or destroys) the handed object rather than leaking or merely borrowing it.

```qcore
extern fn serve(q: ptr<mut, u8>, cb: fnptr, opaque: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_callback_invoked(cb, _, supplied_owned, opaque);
    ensures normal resource_return_none();
};
```

##### Syntax and placement rules

* The actual-argument keyword `supplied_owned` MAY appear in the argument list `A…` of `resource_callback_invoked` or `resource_callback_member_invoked` ([C04.19.3](#c04193-a-registration-carried-by-the-object)) only.
* At most **one** `supplied_owned` token is permitted per argument list; it is rejected in `resource_callback_table`, `resource_callback_registry_invoked`, and `resource_callback_slot_invoked`.
* Any declaration that declares a C formal parameter named `supplied_owned` rejects all callback clauses.

##### Normative consumer rules (gateway rule 12m)

* **Caller-state neutrality:** Because the handed object originates inside the callee, invoking `serve` leaves the outer caller's tracked identities unchanged (subject to normal requirements on the other positions in `A…`).
* **Mandatory candidate acceptance (`callback-supplied-kept`):** Every candidate in the callback's closed set MUST take custody of the `supplied_owned` parameter:
  * **Unconditional take:** Admitted immediately when the candidate's contract in force declares `resource_consumed(p, n)` or `resource_released(p, n)` at that parameter position.
  * **Conditional return-code hand-off ([C04.19.5](#c04195-a-hand-off-by-return-code)):** Admitted if and only if the candidate states a guarded `quant live #p: #owner |-> #callback when #result OP C;` clause *and* the invoking dispatcher's C body passes the **caller branch check** (verifying that when the callback declines ownership via its return code, the dispatcher retains or frees the object, and when the callback takes ownership, the dispatcher never touches it again); otherwise the call is refused (`callback-handoff-unbranched`).
  * **Refusal on borrow or unbranched conditional disposition:** Any candidate that merely borrows or requires liveness at that position (`resource_borrowed` or `resource_owned` without a matched C04.19.5 hand-off), or that relies on an ordinary C04.13 return-code disposition, leaves the callee-supplied object unclaimed and MUST be refused (`callback-supplied-kept`).
* **Standing:** `supplied_owned` is a pin-governed declared assumption (rule 13i) and is not inferred from C bodies.

**Implementation & Provenance.** Gateway rule 12m; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean models: `QCore.OpCallbacks` and `QCore.ReturnCodeHandOff`. Cards: `CG22-OP-CALLBACKS-AND-SERVED-QUEUES` and `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`.

<a id="c04192-a-release-chosen-by-a-discriminant"></a>
##### C04.19.2. A release chosen by a discriminant

Tagged-union operation records in C (such as `rd_kafka_op_t`, which switches on `rko->rko_type & ~RD_KAFKA_OP_FLAGMASK`) embed distinct owned pointers inside different union arms (`rko_u.offset_commit.partitions`, `rko_u.leaders.eonce`, etc.). Releasing the outer record releases child pointers only for the active tag arm. C04.19.2 specifies discriminant-switched release clauses so that consumers neither assume all union arms are freed nor fail to track child releases when the tag is statically known from construction.

```qcore
extern fn op_free(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_discriminant(rko, rko_type, 1610612736);  // switch (rko->rko_type & ~FLAGMASK)
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_arm_released(rko, rko_u.offset_commit.partitions) when rko_type == 6;  // RD_KAFKA_OP_OFFSET_COMMIT, rdkafka_op.h:93
    ensures normal resource_arm_empty(rko) when rko_type == 17;
    ensures normal resource_arm_empty(rko) otherwise;  // default: break;
    ensures normal resource_return_none();
};
```

##### Grammar and total-or-reject rules

* **Discriminant header:** `requires resource_discriminant(P, FIELD, MASK);`
  * At most one per declaration, permitted *only* alongside arm postconditions.
  * `P` MUST name a depth-one pointer parameter with no resource kind that the same declaration releases (`resource_released`).
  * `FIELD` is a dot-separated member path (`a` or `a.b`) naming the tag field inside `P`'s pointee.
  * `MASK` is a canonical non-negative decimal literal (C04.5 spelling) specifying the flag bits cleared before switching (`switch (P->FIELD & ~MASK)`); `0` means no mask is applied.
* **Per-arm postconditions:**
  * `ensures normal resource_arm_released(P, PATH) GUARD;` states that under `GUARD`, the destructor releases the object linked at member path `PATH` of `P` (`PATH` MUST NOT equal `FIELD`).
  * `ensures normal resource_arm_empty(P) GUARD;` states that under `GUARD`, the destructor releases no child member paths of `P`.
  * `GUARD` is either `when FIELD == VALUE` (`==` is the sole admitted operator; `VALUE` MUST be a canonical decimal literal—never a symbolic identifier, with its C enumerator name and header line documented in a comment—and any `VALUE` with bits intersecting `MASK`, i.e. `(VALUE & MASK) != 0`, rejects the module) or `otherwise` (matching every masked tag value not named by any `when` arm).
  * Each distinct guard (`when FIELD == VALUE` or `otherwise`) MAY carry either a single `resource_arm_empty(P)` clause or one or more distinct `resource_arm_released(P, PATH)` clauses—never both empty and released on the same guard, and never the same `PATH` twice under one guard.
* **Function-map withholding:** Declarations carrying discriminant/arm clauses stand wherever callback clauses may stand and are withheld from the plain function map so unaware consumers fail closed.

##### Semantics: constructor-decided tags vs. cautious join

For any masked discriminant value `v`, its active arm is the set of clauses guarded by `when FIELD == v`, or, if none mentions `v`, the clauses guarded by `otherwise`. If neither mentions `v` (because `otherwise` was omitted), `v` has **no declared arm**—never an implicitly empty one.

At a call releasing `P`:

1. **Known discriminant ([C04.19.4](#c04194-a-constructors-result-state)):** When `P` was produced by a declared constructor with `resource_discriminant_set` and its tag state remained intact across the run's stores (`P-OPCTOR-STORES-SEEN`), each reachable raw tag value `raw` is masked as `raw & ~MASK` and selects its corresponding arm; the consumer joins only over those reachable arms.
2. **Unknown discriminant (cautious join):** For every other object, the consumer computes the **cautious join** across *every* possible value:
   * If the declaration has **no `otherwise` arm**, unenumerated tag values have no arm and the call MUST be refused (`arm-undeclared`).
   * For a tracked child object linked at member path `PATH` of `P`:
     * If **every** reachable arm (`otherwise` included) releases `PATH`, the child object is released along with `P`.
     * If **some** reachable arms release `PATH` and others do not, child cleanup is ambiguous and the call MUST be refused (`arm-undecided`).
     * If **no** reachable arm releases `PATH`, the child link is untouched by arm dispatch and obeys ordinary shallow-release rules (`P-B08-SHALLOW`, refusing if a live owned child would be orphaned).

##### Body validation (gateway rule 12o)

In any translation unit that defines the declared destructor, the arm clauses are checked against Clang's AST of its body:

* The body MUST contain **exactly one** `switch` statement whose controlling expression reads `P->FIELD` (when `MASK == 0`) or `P->FIELD & E` where `E` constant-folds to `~MASK`.
* Reading the `switch` body case group by case group—accumulating stacked `case` labels and fall-through paths until terminated by `break` or `return`, with enumerator constants folded from the unit's enum declarations—the checker collects each value's *payload paths*: member paths rooted at `P` passed as call arguments (looking through `&`, casts, and array-subscript bases) whose first field segment matches the first segment of at least one declared arm path.
* Every `when FIELD == VALUE` arm's declared paths MUST equal that case value's extracted payload paths, and every unlabelled value falls into `default:` (or empty if no `default:` exists), whose extracted payload paths MUST equal the `otherwise` arm.
* Any mismatch withdraws the declaration in that unit (`arm-withdrawn`, refusing every call in that unit). Note the trust boundary: rule 12o proves *which member paths each `switch` arm passes to helper calls*; that those helper calls actually release the objects at those paths remains a pinned declared assumption.

**Implementation & Provenance.** Gateway rule 12o; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean model: `QCore.OpCallbacks`. Cards: `CG22-OP-CALLBACKS-AND-SERVED-QUEUES` and `OP-CONSTRUCTOR-DECLARATIONS`.

<a id="c04193-a-registration-carried-by-the-object"></a>
##### C04.19.3. A registration carried by the object

When a C subsystem stores function pointers inside heap records that travel through queues, event loops, or multi-step lifecycles (`rko->rko_op_cb`, `rk->rk_conf.dr_msg_cb`, `flb_input_collector.cb_collect`, OpenSSL's `SSL_CTX`), the call site that invokes or destroys the record cannot see the lexical site where the callback was assigned. C04.19.3 resolves such indirect calls sound-by-construction via a **run-wide closed store set** over the program's translation units, supplemented by typed setter registrations, constructor defaults, ID-forwarded collector registrations, and opaque library callback slots.

```qcore
extern fn op_destroy(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s.rko_op_cb, _, _, rko);
    ensures normal resource_return_none();
};
```

##### Member-invocation and setter-registration syntax

* **Member invocation:** `ensures normal resource_callback_member_invoked(OBJ, TAG.FIELD, A…);`
  * States that during the call, the function pointer stored at dotted member path `FIELD` beneath `struct TAG` in `OBJ` is invoked zero or more times with `(A…)`.
  * `OBJ` MUST be either a depth-one pointer parameter with no kind declared `resource_borrowed` or `resource_released` (where the callback field is read before `OBJ` is freed), or the keyword `supplied_owned` (allowed only when the same declaration hands a callback a `supplied_owned` object under `C04.19.1`, meaning the member callback on each callee-supplied object may run).
  * At most one `resource_callback_member_invoked` clause may appear per `(OBJ, TAG.FIELD)` pair.
* **Member setter registration:** `ensures normal resource_callback_member_registered(P, TAG.FIELD, F);`
  * `P` is a depth-one pointer parameter (`resource_borrowed`, no kind) and `F` is a `fnptr` parameter (`resource_borrowed`, named by no other clause; at most one registration per `(P, TAG.FIELD)`).
  * At a call site, it leaves the caller's local custody state unchanged; for the run-wide candidate closure, every call to this setter in any unit counts as storing its `F` actual into `TAG.FIELD` (resolved via rule 11p; taking the setter's own function address opens the member set).
* **Constructor default registration ([C04.19.4](#c04194-a-constructors-result-state)):** `quant live #result: TAG.FIELD |-> f;` (or `resource_callback_member_registered(result, TAG.FIELD, f)`) records function designator `f` in `TAG.FIELD`'s candidate set.

##### Closed-world candidate resolution (`P-CG22-MEMBER-CLOSED-WORLD`)

Under premise `P-CG22-MEMBER-CLOSED-WORLD`—which states that every translation unit capable of storing into the member belongs to the analysed run and no non-function scalar/byte-pattern store is type-punned into the function pointer—the consumer computes the least candidate set closed under all stores across Clang's ASTs:

1. **Admitted closed stores:**
   * Direct member assignments (`X->F = V` or `X.a.b = V`) and file-scope record initializer lists (for a record defined once at file scope whose initializer entries map cleanly onto declared fields, with omitted fields contributing null/nothing):
     * A function designator adds that function to the set.
     * A null pointer constant adds nothing.
     * Reading another function-pointer member (`rko_serve = orig_destq->rkq_serve`) adds the source member's candidate set (propagated to fixpoint).
     * A function-pointer formal parameter of a function whose own address is never taken in any unit adds every actual passed at that parameter across calls (scoped to the defining unit for `static` functions, run-wide for external functions).
   * Calls to declared `resource_callback_member_registered` setters and constructor defaults (`C04.19.4`).
2. **Operations that open the set (`callback-member-open`):**
   * Storing any unclassified expression (a general variable, a call's return value, a cast, or a non-`fnptr` formal).
   * Taking the address of the member (`&X->F`).
   * Unmappable initializer lists (records containing anonymous members, struct tags redefined locally inside functions, or sibling field initializers performing struct/array copies or function calls), opaque whole-object assignments from another type or call, or `memcpy`-family writes into the record.
   * Any translation unit whose AST fails to load (preprocessing, Clang, or JSON failure) opens *every* member, unless excluded by verified scope rules below. A readable unit containing no stores to the member contributes the empty set.
   * Declaring a `TAG.FIELD` that is not an actual function-pointer member of a `struct TAG` defined in the run is never decided as closed.

##### Record keys, embedded structs, and union layout aliasing

* **Innermost named record keying:** Every member path is canonicalized to its **innermost named `struct` tag**. Thus `rk->rk_conf.dr_msg_cb = V` (written through an embedded `struct rd_kafka_conf_s` field) and `conf->dr_msg_cb = V` (written through a pointer) normalize to the same key `rd_kafka_conf_s.dr_msg_cb`. Consequently, a typed struct copy `rk->rk_conf = *conf` copies only candidates already counted under `rd_kafka_conf_s.dr_msg_cb`, whereas an opaque write (`memcpy` or untyped overwrite) to an enclosing struct opens its embedded members. File-scope initializer lists of an outer struct delegate nested braced lists or typed values to the embedded struct's key.
* **Union layout disambiguation:** Members inside a `union` share storage unless separated by target byte layout:
  * A member inside a named union `U` is keyed `U.*@ARM_PATH`, and inside an anonymous union under `TAG.PATH` is keyed `TAG.PATH.*@ARM_PATH` (for example, `rd_kafka_op_s.rko_u.offset_commit.cb` normalizes to `rd_kafka_op_s.rko_u.*@offset_commit.cb`).
  * When the run's pre-pass extracts Clang's exact byte layout (`__builtin_offsetof` and `sizeof` per unit for all named keys, function-pointer fields, and transitively embedded value records), two union places alias **only if their byte ranges overlap**:
    * Direct member stores alias a union callback slot only when their byte offsets overlap.
    * Stores into a record type `R` held by value inside the union alias the slot only where one of `R`'s byte placements inside the union overlaps the slot.
    * Opaque writes to a record type `R` are refined by target expression (`UNION-OPAQUE-RECORD-WRITES-BY-TYPE`): an opaque write directly into a union path opens only overlapping byte ranges; an opaque write to a separate standalone variable (local, global, by-value formal, or a member path beneath a pointer to a struct that the layout tables prove does not embed the union) **never** aliases into the union; writes through pointers to `R`, array elements, initializer lists, or unmodeled paths conservatively check all placements of `R` in the union.
  * `memcpy`-family calls into a union place and converting the address of a union place to another pointer type (except byte-fillers like `memset`) opaquely open the **entire** union, as do arrays of records, missing layout entries, or unions whose layout differs between two translation units.

##### Run-wide pre-pass, non-C exclusions, checkpoints, and empty sets

* **Pre-pass execution (`MEMBER-PREPASS-OTHER-ROUTES`):** On the pinned route (across its full compile database plus analysed Monkey units), the `current` route, and the no-build route, a member pre-pass runs before per-unit custody analysis:
  * Any C unit whose preprocessed text contains no identifier token belonging to any invoked member's transitive closure graph is safely skipped.
  * Every remaining C unit (including data-only units via `CALLBACK-MEMBER-DATA-ONLY-UNITS`) has its full Clang AST scanned.
  * Non-C units (C++, assembly) or failed Clang units that mention a closure token open every member, with two audited exceptions:
    1. **Standalone C++ units (`CALLBACK-MEMBER-NON-C-UNITS`):** A C++ unit whose preprocessed text is self-contained (using no precompiled headers or C++ modules) opens *only* those members whose closure-graph tokens actually appear in its preprocessed text.
    2. **Verified out-of-program units (`member-facts-scope.json`):** Units listed in the profile's `member-facts-scope.json` with machine-rechecked exclusion evidence at the audited revision—either `tokens-absent` (raw unit text, Clang-resolved headers, and named generator inputs contain no final member token, with all unresolved `#include`s accounted for as generated) or `not-linked` (named link commands link the static archive without linking the wrapper object). Any exclusion whose evidence fails re-verification against the current sources is dropped and the unit is scanned normally.
* **Fold and checkpoint transport:** With the pre-pass active, each unit decides `resource_callback_member_invoked` against the run's pre-pass set using C04.19's loop-safe composition, and the final run fold retains `Safe` or `Pending` rows resting on that declaration if and only if the run set is closed (demoting to `Unknown` otherwise, never promoting). `.qo` checkpoints serialize the pre-pass member facts and their digest into checkpoint identity so the checkpoint composer folds them identically; a checkpointed run invoking a member without stored pre-pass facts refuses composition. Without the pre-pass (standalone single-unit gateway API calls), a unit decides over its local unit set (where an empty local set is refused as `callback-member-open`), and the fold checks that the run's closed set is contained in the unit's.
* **Closed empty run set (`QCore.AppRkCallbacks`):** Over the run-wide pre-pass set, a **closed empty set** proves that zero callbacks are ever registered into that field anywhere in the program; the invocation is therefore the identity (`a_closed_empty_run_set_invokes_nothing`).
* **Plugin-closure and type-punned setter premises:** Declarations invoking application `rk_conf` callbacks carry `requires resource_premise(rk_plugin_callback_closure, FORMULA);` (closing static sets over run registrations and defaults while routing dynamic plugin registrations to `C04.20`), and librdkafka config setters additionally name `conf_property_writer` to account for `rd_kafka_anyconf_set_prop0`'s type-punned `_RK_C_PTR` byte-offset store.

##### Registration forwarded through an integer ID (`QCore.RegistrationById`)

Event loops such as Fluent Bit's input-collector API allocate a callback record internally, link it into a list on an instance, store the callback, and return an integer handle ID rather than the record pointer:

```qcore
ensures normal resource_callback_member_registered(#id, flb_input_collector.cb_collect, cb_collect)
    where #id = #result in ins.collectors;
```

* **Loader rules (total-or-reject):** `#id` MUST be a fresh `#`-prefixed binder matching no parameter name, bound by `where #id = #result in FORMAL.FIELD`; `FORMAL` MUST be a depth-one `resource_borrowed` pointer parameter with no kind; `F` (`cb_collect`) MUST be a `fnptr` parameter named by no other clause; and the function's return type MUST be an integer with `resource_return_none()`.
* **Semantics and body check (rule 12n / 15w discipline):** At call sites, caller custody is unchanged and the run-wide closure records `F`'s actual as stored into `TAG.FIELD`. In a unit defining the setter, the declaration stands **only** if its body (or a single in-unit helper receiving `F` and `FORMAL` whole) allocates one local `struct TAG *r` once via an owned-result constructor, publishes `&r->H` once into `&FORMAL->FIELD`, stores `F` once into `r->CB_FIELD` with no other access to that member, and returns `r->ID` or a negative constant (with `r` used only for member accesses, a null test, and that return); otherwise the declaration is withdrawn (`callback-withdrawn`). Later ID-lookup calls (`flb_input_collector_start` / `_pause`) remain undeclared until ID resolution is modeled.

##### Opaque library callback slots (`QCore.OpaqueCallbackSlots`)

Libraries such as OpenSSL hide struct definitions behind incomplete forward declarations (`typedef struct ssl_ctx_st SSL_CTX;`), making `TAG.FIELD` invisible to Clang's record tables in client builds. Opaque slot clauses represent these hidden fields explicitly:

* **Syntax:** `resource_callback_slot_registered(P, TAG.FIELD, F)` and `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)`, where `P`/`OBJ` is a depth-one `resource_borrowed` pointer parameter with no kind (`OBJ` may also be `resource_released`, below), `TAG.FIELD` is a single field identifier beneath `TAG`, `A…` contains only non-callback parameters or `_`, and at most one slot or member clause appears per `(OBJ, TAG.FIELD)`.
* **Three strict admission gates:** A consumer decides `resource_callback_slot_invoked` **only** when:
  1. **Run-wide facts are present** (without pre-pass facts, `callback-member-open`);
  2. **No unit in the run defines `struct TAG`** (if `struct TAG` is complete in any unit, code could store into `p->field` directly without calling setters, so the call refuses); and
  3. **At least one declared setter registers `TAG.FIELD`** (an unregistered slot key has undeclared writers and refuses even when the run set is empty).
  *(Plain `resource_callback_member_invoked` is unchanged: it still refuses if no unit defines `struct TAG`, preventing an uncompiled library's members from being decided from setters alone.)*
* **Premise `P-OPAQUE-SLOT-WRITERS` and shared/indirect slot keys:** Because C forbids member access on incomplete types, client code can populate an opaque slot only via library functions. Shared slots copied internally across holders share one canonical key (e.g., `SSL_CTX_set_cert_cb` and `SSL_set_cert_cb` both register `ssl_ctx_st.cert_cb` because `SSL_new` inherits `SSL_CTX` settings). Holders may also reach the invoking object indirectly across setter tables (`BIO_meth_set_*` on `bio_method_st.*` reaching `SSL` via `BIO_new`/`SSL_set_bio`; `x509_store_st.*` via `SSL_CTX_set_cert_store`; `x509_lookup_method_st.*` via `X509_STORE_add_lookup`), using setter-specific key names rather than generic words like `free` or `init` (since key segments participate in pre-pass token filtering). Built-in library methods or internal non-`fnptr` writers (`SSL_CTX_enable_ct`) require explicit library-value premises (`openssl_builtin_lookup_methods`, etc.) or forbid stating the slot (`QCore.OpensslHandshakeSlots`, `QCore.OpensslBioSlots`, `QCore.OpensslRemainingSetters`, `QCore.OpensslFreeIndirectCallbacks`). For OpenSSL, automated manual-page checks (`QCore.OpensslUnstatedSlots`) enforce that every writer on a cited manpage's `NAME` list is declared (or classified as a non-writer/macro alias), and undocumented setters are never declared.
* **Opaque slot invocation through a released parameter (`QCore.OpensslReleasedSlots`):** `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)` MAY take an `OBJ` parameter declared `resource_released(OBJ, …)` (such as `EVP_MD_CTX_free` invoking its 12 teardown slots on `ctx`, or `PKCS12_free` invoking its 9 key slots on `a`). Such a declaration MAY carry *only* its basic custody clauses, named premises (C04.18), and `resource_callback_slot_invoked` clauses—any retention, reset, slot move, reference clause, discriminant/arm, constructed state, hand-off, member-path state/helper, descriptor cell clause, container effect, literal-argument premise, return-code class, or other callback clause alongside it rejects the module. At a call site, slots and candidate `resource_owned(n)` parameter requirements are evaluated *before* the release (folding non-null requirements into `OBJ`'s release mode); any candidate carrying post-release effects on `OBJ` (member-path states, deferred constructed-state reads, or registry uses) refuses the call (`callback-beside-release`).

**Implementation & Provenance.** Gateway rules 12n and 13i; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean models: `QCore.OpCallbacks`, `QCore.AppRkCallbacks`, `QCore.RegistrationById`, `QCore.OpaqueCallbackSlots`, `QCore.OpensslUnstatedSlots`, `QCore.OpensslHandshakeSlots`, `QCore.OpensslBioSlots`, `QCore.OpensslRemainingSetters`, `QCore.OpensslFreeIndirectCallbacks`, and `QCore.OpensslReleasedSlots`. Cards: `CG22-OP-CALLBACKS-AND-SERVED-QUEUES`, `CALLBACK-MEMBER-DATA-ONLY-UNITS`, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`, `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION`, `UNION-OPAQUE-RECORD-WRITES-BY-TYPE`, `CALLBACK-MEMBER-NON-C-UNITS`, `MEMBER-PREPASS-OTHER-ROUTES`, `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION`, `OPENSSL-SSL-CTX-CALLBACK-SLOTS`, `OPENSSL-UNSTATED-CALLBACK-SLOTS`, `OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS`, `OPENSSL-BIO-METHODS-AND-CALLBACKS`, `OPENSSL-REMAINING-HANDSHAKE-SETTERS`, `OPENSSL-FREE-INDIRECT-CALLBACKS`, and `OPENSSL-RELEASED-PARAMETER-SLOTS`.

<a id="c04194-a-constructors-result-state"></a>
##### C04.19.4. A constructor's result state

Heap constructors in systems C frequently allocate an operation or configuration record, initialize its tag discriminant (optionally OR-ing internal allocation flags), and either leave its callback field zeroed (`#absent`), set a library-default callback function, or store a caller-supplied `fnptr`. Recording this initial state on `result` allows downstream type-switched destructors ([C04.19.2](#c04192-a-release-chosen-by-a-discriminant)) to select the exact union arm rather than the cautious join, and allows member-callback dispatchers ([C04.19.3](#c04193-a-registration-carried-by-the-object)) to bypass an open run-wide closure when a freshly constructed object is known to hold `#absent` or a specific callback.

```qcore
extern fn rd_kafka_op_new0(source: ptr<const, u8>, type: u32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(source);
    ensures normal resource_borrowed(type);
    ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0);
    quant live #result: rd_kafka_op_s.rko_op_cb |-> #absent;
    ensures normal resource_return_owned(nonnull);
};

extern fn rd_kafka_op_new_cb(rk: ptr<mut, u8>, type: u32, cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(type);
    ensures normal resource_borrowed(cb);
    ensures normal resource_retained_by(rk, result);
    ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 536870912);
    ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, cb);
    ensures normal resource_return_owned(nonnull);
};
```

##### Lexical design of `|->` and `#absent`

* **Maps-to operator (`|->`):** Quantitative member-state clauses spell the mathematical maps-to arrow (`↦`) as `|->` (matching Z, Isabelle, and TLA+ ASCII conventions). A bare `->` is rejected with diagnostic `quant-live-maps-to` naming `|->` because `->` collides with C pointer-member syntax and `.qr` return-type arrows; `>->` is likewise unused because it denotes an injective function (`↣`). The lexer distinguishes `|->` from both `|>` and `||`.
* **Value keyword `#absent`:** `#absent` denotes the absence of a registered callback, kept distinct from C `NULL`. Under the unified `#` sigil rule ([C04.19.5](#c04195-a-hand-off-by-return-code)), writing a bare `absent` without `#` rejects the module (`quant-live-absent`, naming `#absent`).

##### Grammar and total-or-reject rules (G05 and G05d)

Every constructor result-state clause names the returned object via `result` (or `#result`), which MUST be backed by `ensures normal resource_return_owned(nonnull);` on the same declaration (`quant-live-unbacked` otherwise). Any formal parameter named `result` or matching a `#`-form reserved keyword (`absent`, `owner`, `callback`, `released`, `moved`, `stale`, `kept`) rejects the module. `TAG.FIELD` MUST be a dotted struct-member path (`struct TAG`'s field path; C-style `result->rko_op_cb` or any other expression rejects with `quant-live-member`).

Three constructor result-state forms are admitted:

1. **Discriminant initialization:** `ensures normal resource_discriminant_set(result, TAG.FIELD, P, FLAGS);`
   * `P` is an integer parameter of the constructor; `FLAGS` is a canonical non-negative decimal constant (`0` when no extra flag bits are set, or e.g. `536870912` for `RD_KAFKA_OP_F_FLASH`). At most one per declaration (identical repetitions merge).
2. **Low-level member callback state:** `ensures normal resource_callback_member_registered(result, TAG.FIELD, V);`
   * `V` is one of: (a) a `fnptr` parameter declared `resource_borrowed` and referenced by no other callback clause, (b) `#absent`, or (c) a **function designator**—a single identifier naming no parameter and not `result`, `NULL`, or `nullptr`, stating the member's default callback function.
3. **Quantitative maps-to shorthand:** `quant live #result: TAG.FIELD |-> V;` and `quant live #result;`
   * These are the sole `quant` clauses admitted inside `resource_contract_v1` constructor contracts (aside from return-code hand-offs and member-path states below). On the right-hand side of `|->`, `V` MUST be either `#absent` or a function designator (`quant-live-value` for anything else, such as `NULL`, `nullptr`, or a parameter name). For example:
     `quant live #result: rd_kafka_conf_s.log_cb |-> rd_kafka_log_print;`

Each `TAG.FIELD` on `result` may receive at most one state (an agreeing overlap merges; a conflicting second state rejects). Declarations carrying these clauses stand where callback clauses may stand and are withheld from the plain function map.

##### Two-level canonical elaboration

`quant live #result: TAG.FIELD |-> V;` (for `V` equal to `#absent` or a default function designator `f`) is an exact two-level shorthand that elaborates in the loader into two canonical parts:

1. `quant live #result;` — asserting that the returned identity is live (matching C09.3's liveness projection and backed by `resource_return_owned(nonnull)`), and
2. `resource_callback_member_registered(result, TAG.FIELD, V);` — asserting the initial callback state of `TAG.FIELD` (and, when `V` is a function `f`, contributing `f` as a candidate store to `TAG.FIELD`'s run-wide closure).

Contract authors may write the shorthand, the decomposed pair, or an agreeing combination; all elaborate to a single canonical clause set with identical meaning (`QCore.OpCallbacks`: `the_shorthand_and_its_parts_mean_the_same`, `the_shorthand_is_live_and_absent`, `the_shorthand_entails_quant_live`, `both_levels_are_one_canonical_set`; `QCore.AppRkCallbacks`: `a_stated_default_is_a_candidate`, `the_function_shorthand_and_its_parts_mean_the_same`, `both_function_levels_are_one_canonical_set`).

##### Per-path propagation, stability (`P-OPCTOR-STORES-SEEN`), and body validation (rule 14e)

* **Call-site propagation:** At a constructor call, the consumer attaches to the freshly created identity on the current path: (a) the exact discriminant value `actual | FLAGS` whenever `actual` constant-folds (non-constant actuals record no discriminant), and (b) each stated member's callback set (resolved via rule 11p for a `fnptr` actual or function designator, or the empty set for `#absent`). Control-flow joins retain a fact only when every incoming path carries the identical fact. At a later `C04.19.2` release, a preserved discriminant selects only its reachable masked arms; at a later `C04.19.3` member invocation whose run-wide closure would refuse, a preserved member state decides the invocation locally (`#absent` invokes nothing). When no per-path state survives, consumers fall back to `C04.19.2`'s cautious join and `C04.19.3`'s run-wide closure.
* **Run-wide store stability (`P-OPCTOR-STORES-SEEN`):** Per-path constructor states are sound to read at a later site only if intermediate mutations across the program cannot silently overwrite them. Over Clang's ASTs of all analysed and data-only units in the run, a stated member remains stable **only** while every store into it falls into one of four classified forms:
  1. A store into a pointer that the storing function obtained *exclusively* from direct calls to owned-result constructors (initializing its own fresh object, never mutating another activation's object) or into an automatic local struct;
  2. Storing a null pointer constant into a function-pointer member;
  3. A constant bitwise flag-OR `P->F |= E` on a discriminant; or
  4. A **closed case-rewrite** `P->F = C` inside a `case` group of a `switch` testing that same `P->F` (masked or unmasked), with `P` never rebound except to null—the set of decided discriminant values is closed under the rewrite relation (for example, `rd_kafka_broker_op_serve` rewrites `PARTITION_LEAVE` to `PARTITION_JOIN`, both of which map to `rd_kafka_op_destroy`'s `otherwise` arm).
  Sub-object writes affect only fields nested beneath them. Any unclassified store, whole-record overwrite, or unit with missing AST facts voids stability for that member; a function that itself writes the member reads no prior state on it; and the run's fold demotes any `Safe` or `Pending` row relying on a voided constructor member to `Unknown` (`constructed-state-open`).
* **Body check for `#absent` (rule 14e):** In a unit defining a constructor that states `#absent`, the state stands only if its Clang body returns a single local variable on every `return` path, initializes that local once from a zeroing allocator (`calloc` or `rd_calloc`), uses the local only as the base of member stores, in `return`, and inside `sizeof`, never writes to any `#absent` member, and writes any stated discriminant once from its parameter OR-ed with `FLAGS`. Failing this check withdraws the state in that unit while leaving the clause a declared assumption elsewhere (rules 13i–13k). Function-designator defaults, `resource_discriminant_set` without `#absent`, and `fnptr` registrations are declared assumptions.

**Implementation & Provenance.** Gateway rules 12o, 13w, and 14e; grammars [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1) and [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks). Lean models: `QCore.OpCallbacks` and `QCore.AppRkCallbacks` (built over `QCore.DerefDecomposition`). Cards: `OP-CONSTRUCTOR-DECLARATIONS`, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`, and `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`.

<a id="c04195-a-hand-off-by-return-code"></a>
##### C04.19.5. A hand-off by return code

Operation handlers in event-driven C (such as librdkafka's `rd_kafka_op_cb_t` callbacks returning `rd_kafka_op_res_t`) often accept or decline custody of a supplied operation dynamically via their return code: returning `RD_KAFKA_OP_RES_KEEP` or `HANDLED` means the callback took or freed `rko`, whereas returning `RD_KAFKA_OP_RES_PASS` leaves `rko` owned by the invoking serve loop. Such a conditional hand-off is sound **only** when the invoker's C body explicitly branches on that exact enumerator constant and treats custody consistently on each arm.

```qcore
alias #PASS = @RD_KAFKA_OP_RES_PASS;

extern fn rd_kafka_txn_op_send_offsets_to_transaction(
    rk: ptr<mut, u8>,
    rkq: ptr<mut, u8>,
    rko: ptr<mut, u8>
) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == @RD_KAFKA_OP_RES_KEEP;
};
```

##### Sigil conventions, module aliases, and grammar (G05, G05d)

* **Sigils:** `#` prefixes formal/result **binders** (`#rko`, `#result`, `#i`) and **value/domain keywords** (`#absent`, `#owner`, `#callback`, `#released`, `#moved`, `#stale`, `#kept`), whereas the clause-kind introducer immediately following `quant` remains bare (`live`, `deref`, `checks`, `null-deref`). `@NAME` denotes a C enumerator constant resolved through Clang.
* **Module-local enumerator aliases:** `alias #NAME = @C_NAME;` MAY appear at top level inside a `module contract;` file only (explicit executable or library headers reject it at parse time), at most once per `NAME`. `NAME` MUST NOT collide with any `#` reserved keyword (`absent`, `owner`, `callback`, `result`, etc.) or with any parameter name of any declaration in the module (`alias-collision`). Direct `@C_NAME` spellings and `#NAME` aliases elaborate to the identical canonical value, while the alias text participates in the declaration's contract digest.
* **Hand-off clause syntax (total-or-reject):**
  `quant live #P: #owner |-> #callback when #result OP C;`
  * `P` MUST name a depth-one pointer parameter declared `ensures normal resource_owned(P, nonnull);` on the same declaration (providing the liveness half; otherwise `quant-live-unbacked`).
  * The function's return type MUST be a signed/unsigned integer or C02 enum projection (e.g., `u32` for `rd_kafka_op_res_t`) with `resource_return_none()`.
  * `OP` MUST be `==` or `!=`, and `C` MUST be an `@C_NAME` or declared `#ALIAS` (undeclared aliases, raw integer literals, bare words, or relational operators `<`, `<=`, `>`, `>=` reject the module with `quant-live-handoff-*`).
  * At most one hand-off clause is permitted per declaration; it stands where callback clauses may stand and withholds the declaration from the plain function map.
  * Any declaration carrying a `#`-sigil clause rejects parameters named after reserved keywords; declarations without `#` clauses may keep parameter names such as `callback`. A co-occurring `resource_callback_member_invoked` clause on the same declaration may reference the handed formal `P` (`resource_owned`), and no other owned formal.

##### Clang enumerator resolution and lock pinning (rule 13l)

* Per translation unit, Clang resolves `@C_NAME` to an enumerator belonging to the candidate function's **own C return enum type**, and every unit in scope that declares the candidate MUST resolve it to the same constant; unresolved or mismatched-type names refuse every use. On the pinned audit route (after the member pre-pass), any unit declaring the candidate where `@C_NAME` fails to resolve rejects the run's contract selection.
* `quirin-audit pin` locates the top-level C `enum` declaration defining each `@C_NAME` (used directly or via an alias) and adds it to the declaration's reviewed lock identity (`site kind: enumeration`, digesting the entire `enum` statement), refusing if no enumeration defines the name.

##### Semantics and consumer obligations

After calling the function, custody of the object designated by `P` belongs to the **callback** if and only if `#result OP C` holds, and remains with the **caller** otherwise (`QCore.ReturnCodeHandOff.owner_after_is_the_guard`).

1. **As a callback candidate — the caller branch check (rule 12m):**
   * At a `supplied_owned`, `_`, or `PARAM` callback position (`C04.19`, `C04.19.1`, `C04.19.3`), a hand-off candidate is admitted **only** when the invoking function's C body passes the **caller branch check**:
     * Starting at the invoker, the checker traces the callback formal (for parameter dispatch) or the handed object (for member dispatch) forward through direct calls to every indirect call site that invokes it.
     * **Exact local copy rule (`QCore.CandidateComposition.an_exact_copy_calls_a_member_value`):** An indirect call through a variable `v` (formal or automatic local) whose address is never taken and that is written exactly once as `v = o->m` or `T v = o->m;` (pure lvalue read, no type conversion, `o` an unmodified formal—such as `rd_kafka_op_handle`'s `callback = rko->rko_serve`) is recognized as an invocation site of `o->m`.
     * At each invocation site, the return value is partitioned into the constants the C body compares it against plus the complement ("other"). Along every CFG path from the call to function exit (propagating returned status codes back into caller call sites up to the frame where the handed object was locally created):
       * Every return-code class **admitted by the hand-off guard** (where `#owner |-> #callback`) MUST NOT reference the handed object again prior to variable rebinding or a `noreturn` call, and at the outer invoker MUST NEVER return a handed parameter (`an_admitted_branch_never_touches_a_taken_object`).
       * At least one function on the traced chain MUST explicitly compare the return value against the guard's own constant `C` (`an_unbranched_guard_never_admits`).
       * No site along the chain may store the callback's return value through an implicit or explicit type conversion (`a_converted_result_never_admits`).
     * Any unread body, conflicting readings of a function, escaping callback formal, non-exact member copy, converted result, or unmodeled construct fails the check and refuses the call (`callback-handoff-unbranched`).
   * When admitted, `P` counts as **taken** at a `supplied_owned` position and **kept by the invoker** (on the non-taken branch) at `_` or `PARAM`. If the candidate also declares its own callback invocations, it composes under [C04.19.7](#c04197-a-candidate-with-its-own-invocations) when all of them are member invocations, and reads as uncontracted otherwise.
2. **At a direct call (`QCore.HandOffDirect`):**
   * A consumer evaluating a direct call to a hand-off declaration splits the caller's custody of `P` across the caller's branch on the return value (read via C04.13's discipline: tested directly in the controlling condition or assigned in the immediately preceding statement and compared against a non-negative integer or Clang-resolved result-enum constant):
     * Edges reachable only when the guard **holds** treat the actual as **taken** (`resource_consumed`: any subsequent use by the caller is refused).
     * Edges reachable only when the guard **fails** keep the actual live (`resource_owned(P, nonnull)`).
     * Any CFG edge reachable from both `eq C` and `ne C` refuses.
   * The split requires the caller branch check to pass on the calling body (explicit comparison against `C`, no conversion on the stored result, and no reference to `P` along the taken arm). Touching `P` on the taken arm refuses (`callback-handoff-unbranched`, never reported as a bug finding); any unsplit direct call refuses (`callback-handoff-direct`).

**Implementation & Provenance.** Gateway rules 12m and 13l; grammars [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1) and [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks). Lean models: `QCore.ReturnCodeHandOff`, `QCore.CandidateComposition`, and `QCore.HandOffDirect`. Cards: `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`, `HANDOFF-CANDIDATE-OWN-CALLBACKS`, and `HANDOFF-DIRECT-CALL-READING`.

<a id="c04196-a-release-or-move-out-at-a-member-path"></a>
##### C04.19.6. A release or move-out at a member path

Before an operation or subsystem record is destroyed, an invoked op-callback or a direct teardown function frequently frees, steals (moves to another owner and zeroes), or leaves a dangling pointer in specific nested struct/union fields (`rko_u.leaders.eonce`, `rko_u.leaders.partitions`, `mth->sched`). If a subsequent union-switched destructor ([C04.19.2](#c04192-a-release-chosen-by-a-discriminant)) also runs on the record, clearing a pointer (`NULL`) prevents double-free, whereas leaving a freed pointer non-null (`#stale`) triggers a double-free if an arm frees it again. C04.19.6 models these field-level post-states, composes them with union teardown arms and direct calls, and verifies multi-frame static teardown helpers.

```qcore
extern fn rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb(
    rk: ptr<mut, u8>,
    rkq: ptr<mut, u8>,
    rko: ptr<mut, u8>
) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == @RD_KAFKA_OP_RES_KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.eonce |-> #released when #result != @RD_KAFKA_OP_RES_KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when #result != @RD_KAFKA_OP_RES_KEEP;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.opaque |-> {#stale, #kept} when #result != @RD_KAFKA_OP_RES_KEEP;
};
```

##### Member-path state syntax, value lattice, and guards (G05, G05d)

The general clause form is:
`quant live #P: TAG.PATH |-> V [GUARD];`

* **Subject formal `P`:** A depth-one pointer parameter with no resource kind declared `resource_borrowed`, `resource_released(P, nonnull | nullable)` (for direct-call and helper destroyers), or `resource_owned(P, nonnull)` when `P` is the formal handed by a co-occurring `C04.19.5` hand-off.
* **Path `TAG.PATH`:** A dotted member path beneath `struct TAG`. At most one state clause may appear per path of `P`, and no stated path may be a prefix or extension (above or below) of another stated path on `P`.
* **Value keywords and 2-element sets (`V`):** Only the four atomic keywords below and the three 2-element uncertainty sets `{#released, #kept}`, `{#moved, #kept}`, and `{#stale, #kept}` are admitted (any other set or identifier rejects the module; all four words are `#`-form reserved keywords):

| State value `V` | Meaning on return (within `GUARD`, or unconditionally when unguarded) |
|---|---|
| `#kept` | The object at `TAG.PATH` is untouched and remains linked (default for all unnamed paths). |
| `#released` | The object held at `TAG.PATH` was released, and the member field itself was cleared (`NULL` or an emptied list). |
| `#moved` | Ownership of the object held at `TAG.PATH` transferred to another holder (never the caller, leaving caller custody like `resource_consumed`; destination documented in comments only), and the member field was cleared. |
| `#stale` | The object held at `TAG.PATH` was taken (released or transferred), **without clearing the member field** (which still holds the dangling pointer). |
| `{V', #kept}` (`V'` in `{#released, #moved, #stale}`) | Non-deterministic branch join: `V'` occurred on some internal paths and `#kept` on the remaining paths. |

* **Admitted `GUARD` forms:**
  1. **Result guard (`when #result OP C`):** Permitted *only* alongside a `C04.19.5` hand-off on the same formal `P`, and MUST be the **exact logical complement** of that hand-off's guard (identical constant `C`, opposite operator `==` vs. `!=`), partitioning all return codes cleanly between "callback took `P` whole" and "caller retains `P` with modified fields."
  2. **Scalar parameter guard (`when #X OP K`):** Permitted with or without a hand-off. `X` is a plain integer parameter other than `P` (`i8`–`i64`, `isize`, `u8`–`u64`, `usize`, with C04.14.1 bounds), `OP` is one of `==`, `!=`, `<`, `>`, `<=`, `>=`, and `K` is an in-range canonical decimal literal (optional leading `-`, non-trivially partitioning `X`'s range), `@NAME`, or `#ALIAS` (resolved via Clang in `X`'s enum per unit under rule 13l). When the guard fails, `TAG.PATH` reads `#kept`.
* **Co-occurrence and withholding:** Declarations carrying member-path states are withheld from the plain function map. Beside basic custody predicates and named premises, they admit only a hand-off (`C04.19.5`), or—for a **helper destroyer** (`resource_released(P, …)` with no hand-off)—`resource_callback_member_invoked(P, …)` clauses on that same `P`, or `resource_member_helper` (below).

##### Composition at callback invocations with union destroy arms (rule 30 & rule 74p)

When a callback invocation (`C04.19`, `C04.19.3`) passes a **caller parameter** at `P`'s position (note: `supplied_owned` and `_` pass callee-owned objects where field states do not affect caller links, and parameter guards `when #X OP K` at callback invocations always evaluate to `{V, #kept}`), for every tracked child object the caller linked at a member path of that parameter, each candidate's post-state at that path is composed *before* the invoking destructor's `C04.19.2` arm join (`every` arm releases the path, `some` arms do, or `none` do):

| Candidate leaves path as | Destructor arms: `every` releases | Destructor arms: `some` release | Destructor arms: `none` release |
|---|---|---|---|
| `#kept` (or path unnamed) | Released once by arm (`gone`) | Refused (`arm-undecided`) | Stays linked (`stays`) |
| `#released` or `#moved` | `gone` (member was cleared to `NULL`, so arm safely no-ops) | `gone` (cleared when taken) | `gone` |
| `#stale` | **Refused** (double release by arm on non-cleared pointer) | **Refused** (double release) | **Refused** (callback runs 0 or more times) |
| `{#released, #kept}` or `{#moved, #kept}` | `gone` on both branches (either callback took & cleared, or arm freed) | **Refused** | **Refused** (undecided) |
| `{#stale, #kept}` | **Refused** | **Refused** | **Refused** |
| Unstated (missing/withdrawn contract, or path nested below a stated path) | **Refused** | **Refused** | **Refused** |

Outcomes join across all candidates in the closed set (narrowed by decided constructor registrations from `C04.19.4` where applicable): identical outcomes stand, any refusal dominates, and `gone` joined with `stays` refuses. `gone` releases the linked child object exactly once and drops its field link (`QCore.MemberPathRelease`: `gone_leaves_exactly_once`, `a_gone_join_is_gone_for_every_candidate`).

* **Evaluation order when the outer call releases `P` (`QCore.CallbackReleasedEffectOrder`, rule 74p):** When the invoking declaration itself declares `resource_released(P, …)` (for plain member invocations, synchronous invocations passing `P`, or deferred constructed-state reads—*not* opaque slots, which refuse per `C04.19.3`), a consumer MAY apply `resource_released(P)` first and compose candidate member-path states immediately afterward, provided the outer release leaves `P`'s field links and constructed state intact and `gone` consumes only a still-live linked child. Thus if the outer release already freed `P` or a contained child, attempting `gone` on the same identity refuses rather than double-releasing, matching C's runtime order (callback before free).

* **At a table dispatch (`QCore.CallbackTableMemberPathStates`, rule 13v amended by card CALLBACK-TABLE-MEMBER-PATH-STATES):** `resource_callback_table` is a callback invocation like the others: a consumer MUST compose its candidates' states at the dispatch exactly as at a synchronous invocation, joined over the candidates, and MUST NOT drop them. The callee may call several entries, in any order; the join is sound for any non-empty sequence of the candidates' readings, because a `gone` join takes the object once and clears the member on every reading, and candidates that disagree refuse. As at every form, `gone` under no arm reads the dispatch as invoking a candidate.

##### Composition at direct calls (`QCore.MemberPathDirectCall`, rule 30)

* **Hand-off exclusion:** Directly calling a declaration that combines member-path states with a `C04.19.5` hand-off is refused (`member-path-direct`, explicitly naming the co-occurring clause).
* **Plain direct-call destroyer (`compose_direct`):** At a direct call (which executes once with no trailing union arms), each parameter guard `when #X OP K` is evaluated against the call's constant/enum actual at `X` (holding → `V`; failing → `#kept`; non-constant/unresolved actual → cautious join `{V, #kept}`, `an_unknown_value_gives_the_join`). Each tracked child linked by the caller at a member path of `P`'s actual is then updated:
  * `#released` or `#moved` → **gone** (child released once, field link dropped).
  * `#kept` (or unnamed path) → **stays** linked (subject to `P-B08-SHALLOW` if `P` itself is released).
  * `#stale` → child is marked **taken once** while its dangling field link is retained; any subsequent release or hand-off of that child—including when the caller aliased `P` and the child to the **same** tracked identity—refuses (`member-path-double-release`, `a_release_after_taken_is_a_double_release`, `one_identity_refuses_the_second_end`).
  * Any `{V', #kept}` set (`a_set_never_decides_at_a_direct_call`), a link not backed by one live object, a path above or below a stated path, or states withdrawn by the unit's body check refuses (`member-path-undecided` / `member-path-unstated`).

##### CFG body validation (rule 30) and static teardown helpers (rules 50 & 51)

1. **In-unit body validation (rule 30):** When a unit defines the function:
   * **Contradiction check (withdraws states):** Any non-null pointer store into a stated path, any whole-record overwrite of a struct containing a stated path, any store into or address-taking of a `#stale` or `#kept` path, or a formal pointee type mismatch against `struct TAG` **withdraws** the declaration's member-path states in that unit. (Note: when `P` is `resource_released`, post-call field values are unobservable, so `#released`/`#moved` validate only if the body explicitly clears the field before freeing `P`, remaining declared assumptions otherwise.)
   * **Bounded CFG abstract fixpoint:** The body (including `goto`s, labels, null-test branch refinements, and non-recursive `static` / `static inline` unit helpers passed `P` or `&path` within a bounded call depth, tracking per-return outcomes across `return f(…)`) tracks each stated path over `{untouched, untouched-and-null-tested, null, anything}`. Inside followed helpers, pointer stores to the link field named by the declaration's own `resource_unpublished(P, LINK)` (C04.8, e.g., `mk_list_del(&mth->_head)` clearing `_head.prev`/`next`) are recognized as that unlink rather than aborting the walk, whereas view copies into locals, unsequenced side effects on `P`, unmodeled constructs, or unstated pointer stores in helpers end the walk. If at every guard-admitted exit `#released`/`#moved` read `null`, `{V', #kept}` saw a null store with no `anything`, and `#stale`/`#kept` read `untouched`, body evidence `30` is recorded; otherwise (absent a contradiction) the states stand as declared assumptions (rules 13i–13k).
2. **Stated `#stale` value handed to a declared `static` helper (`QCore.MemberValueHelper`, rule 50):**
   * **Helper destroyer declarations:** `resource_callback_member_invoked(P, TAG.FIELD, A…)` MAY stand beside unguarded member-path states on a `resource_released(P, …)` formal with no hand-off.
   * **Rule 50 body evidence:** In the outer destroyer's own frame (depth 1, non-recursive), when a path `M` stated `#stale` is read while untouched (`P->M` or a single-assignment, non-escaped local copy of `P->M`) and passed to a `static` / `static inline` unit function whose helper-destroyer contract binds one unguarded released formal of matching `struct TAG`, passes its own rule-30 contradiction check, and has **closed empty run-wide sets** on all of its member invocations, the walk composes the take of `M`. If every guard-admitted exit takes `M` **at most and at least once** (`established_iff_taken_once`) with no escape or uncomposed call, body evidence `50` is recorded alongside `30`.
3. **Cross-unit helper composition and direct calls to helper destroyers (`QCore.DestroyerHelperComposition`, rule 51):**
   * Because callers in other translation units cannot inspect the outer destroyer's C body, the contract may publish the helper link explicitly via at most one clause:
     `ensures normal resource_member_helper(P, TAG.PATH, H);`
     where `P` is released by the declaration, `TAG.PATH` is stated unguarded `#stale`, `H` is a distinct helper function, and the outer declaration carries no callback or hand-off clauses. Rule 50 validates that only `H` takes `TAG.PATH` (covered by evidence `50`, else a declared assumption).
   * **Consumer obligation at a direct call with `resource_member_helper`:** After applying the outer destroyer's own member-path states: (1) every `resource_callback_member_invoked` on `H` is checked over the **run's** member facts and MUST resolve to a **closed empty set** (refusing `callback-member-open` if open, or `member-path-helper` if non-empty, missing run facts, or `H` is not an in-force helper destroyer); (2) `H`'s member-path states are then applied to the caller's live object linked at `TAG.PATH` via `compose_direct`. The caller row records a dependency on `H` so the run fold checks `H`'s members (`the_composition_carries_the_footprint`). Callback invocations never compose `H`.
   * **Direct call to a helper destroyer itself:** A helper destroyer (carrying both member invocations and member-path states on released `P`) no longer refuses `member-path-direct` at a direct call: the consumer first evaluates its member invocations (`C04.19.3`) and then applies its member-path states (`compose_direct`). As a *callback candidate*, it remains uncontracted ([C04.19.7](#c04197-a-candidate-with-its-own-invocations)).

**Implementation & Provenance.** Gateway rules 30, 50, 51, and 74p; grammars [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1) and [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks). Design rationale: [OP-CALLBACK-MEMBER-PATH-RELEASE prediction](reports/op-callback-member-path-release/PREDICTION.md). Lean models: `QCore.MemberPathRelease`, `QCore.CallbackReleasedEffectOrder`, `QCore.MemberPathDirectCall`, `QCore.MemberValueHelper`, and `QCore.DestroyerHelperComposition`. Cards: `OP-CALLBACK-MEMBER-PATH-RELEASE`, `CALLBACK-MEMBER-RELEASED-EFFECT-ORDER`, `FB-TEARDOWN-DESTROY-DECLARATIONS`, `MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS`, `MEMBER-PATH-DIRECT-REFUSAL-TEXT`, `SCHEDULER-DESTROY-MEMBER-STATES`, and `RULE50-CONSUMER-COMPOSITION`.

<a id="c04197-a-candidate-with-its-own-invocations"></a>
##### C04.19.7. A candidate with its own invocations

In layered event dispatchers (such as `rd_kafka_op_handle` invoking `rko->rko_serve` while itself serving as a callback candidate), a callback candidate's own contract may carry `resource_callback_member_invoked` clauses alongside its basic custody clauses and an optional `C04.19.5` return-code hand-off. C04.19.7 introduces no new surface syntax; instead, it defines how a consumer composes such a **nested member-invoking candidate**.

##### Composition semantics and consumer obligations (rule 13u, 12m, 12n)

A candidate declaration qualifies for nested composition **only** if its non-custody clauses consist exclusively of `resource_callback_member_invoked` clauses and an optional `C04.19.5` hand-off (any candidate declaring synchronous formal invocations, table dispatches, registries, setters, or its own `C04.19.6` member-path states remains uncontracted as a candidate).

1. **Inner member-invocation resolution:** Every `resource_callback_member_invoked` clause on the candidate is decided over the **run-wide pre-pass closed set** (routes lacking the pre-pass MUST refuse nested composition because the run fold checks only declarations listed directly on a row, and composed candidates are not listed there). Each inner target in that member's closed set is checked with the *candidate itself* acting as the invoker—running `C04.19.5`'s caller branch check on the candidate's C body when an inner target uses a return-code hand-off, and treating a closed empty run set as invoking nothing.
2. **Parameter liveness strengthening:** Starting from the candidate's declared parameter modes, any `resource_borrowed` formal of the candidate that an inner candidate requires to be live (`resource_owned(n)` at a forwarded parameter actual) is strengthened to `resource_owned(n)`.
3. **Outer hand-off verification:** If the candidate declares a `C04.19.5` hand-off, that hand-off is verified against the *outer* invoker's body after completing inner composition.
4. **Inner member-path state restriction:** If an inner candidate declares `C04.19.6` member-path states on a formal that the outer invocation binds to a **caller parameter**, composition is refused; at `_` or `supplied_owned` (objects never owned by the outer caller), inner member-path states have no effect on caller custody.
5. **All-or-refuse and least-fixed-point cycle rejection (`QCore.CandidateComposition`):**
   * The candidate counts as contracted if and only if **every** own member invocation decides (`a_composed_candidate_is_contracted_iff_each_own_invocation_decides`); any inner failure refuses the outer call with `callback-own-invocation-refused`, naming both the outer invocation and the inner refusal reason.
   * Composition uses a **least fixed point** (matching rule 15w, never coinduction): encountering a candidate already on the active resolution stack—directly or through mutual indirect recursion—refuses immediately (`a_cycle_never_supports_itself`).
* **Standing:** Nested composition relies on the candidate's and inner targets' contracts in force; member invocations remain declared assumptions (rule 13i).

**Implementation & Provenance.** Gateway rules 12m, 12n, and 13u. Lean model: `QCore.CandidateComposition`. Card: `HANDOFF-CANDIDATE-OWN-CALLBACKS`.

<a id="c0420-outer-contracts-code-loaded-at-runtime"></a>
#### C04.20. Outer contracts: code loaded at runtime

##### Conceptual model: static closed world vs. dynamically loaded code

Many production C daemons—such as Fluent Bit (`flb_plugin_load` for `.so` plugins, `flb_plugin_proxy_create` for Go plugins) and librdkafka (`rd_kafka_plugin_new`)—support optional runtime plugin loading. Statically, when no external plugin is loaded, closed-world properties (such as which callbacks can ever be registered on `rk_conf` or whether `flb_input_instance.p` stays non-null) are decidable over the audited repository's translation units. At runtime, once a dynamic loader passes a pointer `P` across `dlopen`, arbitrary external code can reach `P` and any transitively reachable C structs.

Rather than either falsely claiming static `Safe` verdicts hold for arbitrary third-party plugins or permanently degrading static builds to `Unknown`, `outer_contract_v1` hosts an explicit **outer contract** on the **loader function declarations** that pass internal pointers to external code, and publishes **two verdicts from a single audit run**:

* **Static scope:** Assumes no dynamic plugin loader is invoked; publishes the standard static verdicts unchanged alongside exact per-row closed-world dependency records ([C06.5](#c065-per-verdict-contract-dependencies)).
* **Runtime scope:** Projects each row under one or more active outer-contract scenarios (`flb_external_plugin`, `rk_plugin_library`, or `all`). Closed-world facts reachable from a loader's handed pointer degrade from `Safe` to **`Pending`** (`outer-contract` naming `NAME@DIGEST`) when explicitly vouched for by an in-force outer contract's **envelope**, or to **`Unknown`** (`outer-uncovered`) when left uncovered, stale, or unpinned. Human reviewers resolve `Pending` either by auditing the specific loaded plugin against the outer contract's symbolic formula and envelope, or by deploying without runtime plugins (the static scope).

```qcore
module contract;

extern fn flb_plugin_load(path: ptr<mut, u8>, ctx: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] flb_external_plugin, FORMULA;
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
    quant #outer #loaded: flb_input_instance.p |-> #kept;
};

extern fn flb_plugin_proxy_create(dso_path: ptr<const, u8>, kind: i32, config: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] flb_external_plugin;
    quant #outer where #config in #loaded [[runtime]];
};
```

##### Vocabulary isolation, syntax, and total-or-reject loading ([G05g](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05g-outer-contract-v1))

`outer_contract_v1` is a dedicated third contract vocabulary, completely disjoint from `resource_contract_v1` and `bounds_contract_v1` (both of which reject outer modules; provider crates register outer modules in a separate `outer_view`). Every declaration in an outer module MUST obey total-or-reject rules:

1. **Selector:** Exactly one `requires outer_contract_v1;`.
2. **Contract definition vs. citation header:** Exactly one header:
   * **Defining loader:** `quant #outer [[contract]] NAME, FORMULA;` (defined at most once across a selection). `FORMULA` MUST be a well-formed symbolic formula over C04.18.9's SP8 vocabulary (legacy prose string literals are rejected at parse time on every route with `QF004`; malformed formulas reject the module).
   * **Citing loader:** `quant #outer [[contract]] NAME;` (no formula and **no envelope lines**; cites a contract defined elsewhere in the same selection—citing an undefined contract rejects the selection; provider probes load all outer modules of a crate together).
3. **Runtime Escape Condition(s):** At least one `quant #outer where #P in #loaded [[runtime]] [when #result OP N];`:
   * `#P` binds a pointer parameter of the loader (each parameter bound at most once); `#loaded` is a `#`-prefixed value keyword denoting the set of objects handed to loaded code (bare `loaded` rejects).
   * The optional scalar guard `when #result OP N` is admitted only when the loader returns an integer type.
4. **Envelope lines (defining declaration only, unique keys):**
   * `quant #outer #loaded: TAG.FIELD |-> #kept;` — asserts that loaded code never stores into `TAG.FIELD`. Covers `stable:TAG.FIELD` (rule 14x), `member:TAG.FIELD` (rule 14t), `stated_members` (rule 14e), and `run_set_members` (rule 12n).
   * `quant #outer #loaded: TAG.FIELD |-> #registered;` — asserts that loaded code writes `TAG.FIELD` only via declared setter APIs using callbacks that satisfy `TAG.FIELD`'s candidate contract (or C04.20.1's runtime list inventory). Covers `run_set_members` (rule 12n).
   * `quant #outer #loaded: #premise NAME;` — asserts that the named premise `NAME` continues to hold in the presence of loaded code. Covers `premise:NAME`.

##### Closed-world facts vs. non-closed-world exceptions

Every conclusion drawn because static analysis observed all stores or registrations in the run is classified as a **closed-world fact** in a row's `contract_dependencies` ([C06.5](#c065-per-verdict-contract-dependencies)):

* **Closed-world facts requiring envelope coverage when reached:**
  * Scoped stable members (`stable:TAG.FIELD`, rule 14x) and declared non-null members (`member:TAG.FIELD`, rule 14t);
  * Run-wide callback member sets (`run_set_members`, rule 12n, `P-CG22-MEMBER-CLOSED-WORLD`);
  * Constructor-stated member states (`stated_members`, rule 14e, `P-OPCTOR-STORES-SEEN`); and
  * **Every** named premise (`C04.18`), *unless* explicitly listed below as a build/reading/tracking premise (any newly added premise defaults to closed-world until explicitly exempted).
* **Exempted non-closed-world premises:** `asserted_allocation`, `nested_call_admission`, `literal_argument`, `P-CELL-STATE-NOT-FORWARDED` (rule 38, superseding retired `mk_event_descriptors`), `untracked_handback`, `ingestion_active`, `synchronous_output_reachability`, and rule 13e's two premises.

##### Reachability (`outer_record_graph`), verdict join, pinning, and publication

* **Clang AST record reachability:** For each loader's `#P` binder, `qcore_c_gateway::outer_record_graph` builds a conservative reachability graph from Clang's AST of the loader's defining unit under its compile command:
  * Typed pointers, embedded structs, and arrays of structs add forward edges to their target records; an embedded struct also adds a **reverse `container_of` edge** to every enclosing struct that embeds it. Function pointers add no edges.
  * **Universal fallback (reaches every record):** Encountering `void *`, a forward-declared incomplete struct, an anonymous struct named by location, an unclassified typedef pointee, a held/unpinned loader, or a loader lacking a definition site, compile database entry, or readable unit conservatively marks *every* record reachable. (Forged integers-to-pointers and unrelated pointer casts are excluded by the outer contract's symbolic statement, which is what the `Pending` review checks.) Named premises are treated as reachable by every active outer contract.
* **Runtime verdict calculation:** Under an active set of outer contracts, each closed-world fact on a row contributes:
  * Nothing, if it is a member fact on a record *not* reached by the contract's loaders;
  * `Pending` (`outer-contract` naming `NAME@DIGEST`), if reached, every hosting loader is pinned in force, and an envelope line covers the fact; or
  * `Unknown` (`outer-uncovered`), if reached and either the loader is not in force or no envelope line covers the fact (an uncovered reached fact in *any* active contract yields `Unknown` even if another active contract covers it).
  The row's runtime verdict is the §L21.2 join (`Safe < Pending < Unknown < Unsafe`) of its static verdict and all fact contributions—never promoting any verdict.
* **Pinning (`outer-contracts.lock`) and `NAME@DIGEST`:** `quirin-audit pin --outer --source-root DIR` binds each loader declaration to its C definition sites in `outer-contracts.lock` (checked alongside resource and bounds locks by `pin --verify-contracts`). `DIGEST` is the SHA-256 over every hosting loader's name and canonicalized text digest, so modifying any definition, citation, runtime condition, formula, or envelope line alters `NAME@DIGEST` and invalidates prior review.
* **Projection output (`QCORE_OUTER_PROJECTION=PATH`):** When set on the pinned, configured, or no-build routes, static reports remain byte-identical while a standalone projection report is written to `PATH` containing selection digests/pins, contract definitions/envelopes/standings, loader reachability graphs, static tallies, and per-scenario (`flb_external_plugin`, `rk_plugin_library`, `all`) custody, null, and combined tallies plus every moved row and reason. When unset, zero outer files are read or written.

##### Production outer contracts and SP8 symbolic statements

* **Fluent Bit (`flb_external_plugin`) and librdkafka (`rk_plugin_library`):**
  * `providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv` defines `flb_external_plugin` on `flb_plugin_load` and cites it on `flb_plugin_proxy_create` (binder `#config`, pinned at `66910c10` and `046accf6`).
  * `providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv` defines `rk_plugin_library` on `rd_kafka_plugin_new` (binder `#conf` when `#result` indicates success).
  * Both envelopes carry `#registered` lines for the 17 `rk_conf` callback members and their embedded paths, `#kept` lines for the broker stable member, five rule-14t members, and two op members, and `#premise` lines for eight named premises including `cjson_allocator_hooks` and `cjson_list_invariant` (forbidding loaded code from calling `cJSON_InitHooks`, mutating global hooks, or corrupting cJSON `child`/`prev`/`next` links). Only `flb_external_plugin` vouches for keeping the four Fluent Bit instance `p` descriptors (`rk_plugin_library` intentionally omits unrelated Fluent Bit promises).
* **Symbolic statement formulas (SP8):** With `H` denoting `CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147` (the SHA-256 constant of `qcore-provider-librdkafka/outer-candidates.json`) and `W(#a)` denoting the 12 list-well-formedness conjuncts of `detach_list_representation` generalized to `#a` (over `#child`, `#first`, `#children`, `#pred`, `#next`, `#prev`, `#next_reach`, `#last`, `#nodup`):
  * **`rk_plugin_library` (`RK`):**
    `(quant all #f in #conf_registrations: #f in #public_setter_values && #f in #meets_candidate) && (quant all #f in #interceptors: #f in #zero_appends(H)) && (quant all #f in #interceptors: #f |-> (#inert && #borrows_args && #borrows_handles && #no_global_custody && #balanced)) && (quant all #f in #copies(#conf_registrations | #interceptors): #f in #valid_through_last) && (quant all #f in #interceptors: (#f in #conf_lifecycle(H) -> #f in #on_live_conf) && (#f in #client_lifecycle(H) -> #f in #from_on_new && #f in #before_init && #f in #client_lifecycle(H) \ #on_share_consumer)) && #outer_code |-> (#calls_none(#cjson_init_hooks) && #stores_none(#cjson_global_hooks) && #stores_none(#cjson_link_cells) && #stores_none(#rk_internal) && #calls_none(#rk_nonpublic) && #meets_published(#rk_public)) && (quant all #a in #cjson_handed: W(#a)) && #allocator_hooks_premise`
  * **`flb_external_plugin`:**
    `#outer_code |-> (#stores_none(#flb_core_owned) && #accesses_only(#handed) && #calls_only(#exported) && #provenance(#handed) && #typed_access) && (RK)`
    Because Fluent Bit plugins using librdkafka inherit every librdkafka and cJSON obligation, `flb_external_plugin` logically entails `rk_plugin_library` (`QCore.PremiseFormula.Outer.fb_entails_rk`).
* **Formula role vs. inventory citation:** Analysis consumers do not interpret the inner terms of `FORMULA` to prove plugin code; however, its canonical printing enters `NAME@DIGEST`, and `qcore_audit::outer_candidates` extracts every `CANDIDATES_<HEX>` constant from the formula AST (`outer_contract::candidate_inventories`) to bind and verify C04.20.1's candidate inventory.

**Implementation & Provenance.** Gateway rule [31](agents/gateway.md#constraint-31); grammar [G05g](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05g-outer-contract-v1); `qcore_c_gateway::outer_record_graph`, `qcore_audit::outer_candidates`. Design and faithfulness reports: [OUTER-CONTRACT-DESIGN](reports/outer-contract-design/README.md) and [SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS](reports/sp8-outer-contract-symbolic-statements/README.md). Lean models: `QCore.OuterContract` and `QCore.PremiseFormula.Outer`. Cards: `OUTER-CONTRACT`, `MK-EVENT-DESCRIPTOR-QUANT`, and `SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`.

<a id="c04201-runtime-list-callback-candidate-inventories"></a>
##### C04.20.1. Runtime list callback candidate inventories

Dynamic interceptor systems (such as librdkafka's `rd_kafka_interceptor_*` lists stored in `rd_list_t` containers) register multiple callbacks into runtime lists rather than single function-pointer fields. Static list-callback invocation remains unsupported (`Unknown`), but for the runtime outer projection (`C04.20`), an optional version-1 trusted inventory file `outer-candidates.json` (placed beside a provider crate's `contracts/` directory) provides a machine-pinned review surface for `#registered` list paths.

##### Inventory binding, contents, and neutral-custody obligations

* **Cryptographic citation:** Read *only* when `QCORE_OUTER_PROJECTION` is selected. Every defining outer formula whose envelope carries `#registered` list paths MUST cite the exact SHA-256 of `outer-candidates.json` via the uppercase SP8 constant `CANDIDATES_<DIGEST>` (replacing the retired `outer-candidates:DIGEST` prose token). A `#registered` list path without a matching inventory citation, or a citation naming a missing, malformed, ambiguous, or unbound inventory, refuses runtime selection.
* **Recorded metadata per list:** Exact canonical list key and embedded-struct aliases, element struct type, callback union arm, C function signature, public registration setter, success return value (`0`), allowed registration lifecycle phase, duplicate-registration semantics, callback/opaque-context lifetime requirements, and candidate custody restrictions. Distinct interceptor lists remain distinct keys even when their element records share a C callback union.
* **Mandatory human-review obligations on loaded list callbacks:**
  1. **Neutral custody:** Every pointer argument and every object reachable from it is strictly borrowed—no release, retention, ownership transfer, or global custody side effect. Scalar OS handles (including `sockfd`) are likewise borrowed (no `close`, `dup`, or transfer), and any internal temporary allocations must be balanced before returning. Documented writes into borrowed output buffers are permitted without waiving other API preconditions. Any callback or transitive callee violating neutral custody is unsupported.
  2. **Lifetime and lifecycle discipline:** Both the loaded code segment and any opaque context pointer must remain valid through the final possible invocation (including client/config destruction and registrations copied by `on_new`). Client-level interceptor registrations may occur *only* inside `on_new`, prior to initialization completion, on non-share-consumer clients; configuration-level registrations require a live configuration. Rejected or duplicate registrations do not append elements (except `on_conf_destroy`, which permits duplicates).
  *(These conditions are explicit human-review obligations for loaded plugins—never inferred from C types and never treated as proving the setter itself is custody-neutral.)*

##### Source-identity pinning and static/runtime boundary

* **Audited source pins:** `outer-candidates.json` embeds relative file paths and exact byte-level SHA-256 digests of the reviewed library sources at the audited revisions (verified against `HEAD` Git blobs for repository inputs or disk files for directory inputs). Any source-digest mismatch places every defining outer loader citing that inventory **out of force**, causing its envelope to cover nothing (`Unknown`). Source pins record review provenance and are not automatic proofs of C semantics.
* **Strict non-promotion boundary (`QCore.OuterListCandidates`):** `outer-candidates.json` never enters the static callback registry, never turns a C list into a `fnptr`, never decides static list invocations (which stay `Unknown`), and never promotes a runtime verdict above `Pending`.

**Implementation & Provenance.** `qcore_audit::outer_candidates`; grammar [G05g](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05g-outer-contract-v1) (SP8). Lean model: `QCore.OuterListCandidates` (distinct list identities, source-pin gate, lifecycle obligation, non-promotion; Rust/model correspondence and source review remain rule-28 premises). Cards: `OUTER-CONTRACT` and `SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`.


# Part III — Contract Governance, Trust Standing, and Bundle Lifecycle

Loading a `.qv` file into a compiler data structure proves syntax, not truth. In real C verification campaigns, contracts sit at different stages of lifecycle maturity and provenance: some are hand-audited assumptions pinned to specific Git commits of a third-party C library, some track a moving release-candidate branch, some are flexible standard-ABI specifications (`libc`), some are deferred awaiting human review, some are machine-inferred summaries of internal C helpers, and some are actively refuted when checked against their own C function bodies.

Part III specifies how contract selections are packaged into immutable assumption manifests (`C05`), how trust standing and body validation govern verdict soundness (`C06` and `C06.1`–`C06.5`), and how overlapping contract bundles compose deterministically via cryptographic pin timestamps (`C06.6`).

<a id="c05-manifests-and-immutable-assumptions"></a>
## C05. Manifests and immutable assumptions

##### Resource manifest format and lexical path validation

A version-1 resource manifest is a JSON object naming an explicit, non-empty list of contract modules:

```json
{
  "schema_version": 1,
  "modules": ["qcore/libc/memory.qv"]
}
```

*(Schema version 2 extends this format with a `load_path` reference and `CRATE:MODULE` selectors to support cross-crate `#include` and `contract inherit`; see C15.)*

The manifest loader enforces strict total-or-reject validation:

* **Envelope and uniqueness checks:** Unknown JSON properties, unsupported `schema_version` numbers, empty `"modules"` arrays, duplicate module entries in a file list, and duplicate function declarations across selected modules (except where resolved by timestamped bundle supersession under [C06.6](#c066-contract-supersession-across-bundles)) are errors that reject the entire load.
* **Relative lexical path rule:** Module file paths are resolved relative to the manifest file's parent directory. Every path segment MUST be a normal relative path component; absolute paths, `.` segments, and `..` parent-traversal segments are rejected.
* **Explicit security boundary (not an OS sandbox):** This path validation is a purely lexical check on manifest strings. It MUST NOT be described or relied upon as a canonical filesystem sandbox: it does not enforce symlink confinement, crate/provider process isolation, or operating-system access control.

##### Immutable `source_assumptions` retention and replay

* **Snapshot retention:** Upon successful loading, `ResourceContracts` stores the exact map of loaded module texts in `source_assumptions`.
* **Deterministic assumption recheck (`check_source_assumptions`):** Calling `check_source_assumptions` re-parses and reloads that retained immutable text map in memory and verifies that the reconstructed function map, review holds, revision modes, and elaborations match the active environment. It intentionally **never** re-reads mutable files from disk, and passing `check_source_assumptions` proves internal environment integrity—**not** that an external C implementation satisfies its declared contract.
* **Programmatically constructed environments:** Test harnesses or programmatic API callers may construct `ResourceContracts` values in memory with no retained `source_assumptions` text map. Such environments MUST NOT be reported as source-backed contract verification; any consumer accepting one MUST explicitly disclose its alternate assumption source and trust boundary.
* **Reviewed Cycle 19 narrow projection:** The historical `cycle19-resource-contracts.json` manifest selects 9 modules and 25 `resource_contract_v1` declarations (expanded deliberately under cards `HASH-CONTRACTS` and `PROD-CONTRACTS`, with exact clause and declaration counts pinned in `compiler/tests/qcore_contract_corpus.rs`). This narrow projection is intentional and does not import the broader observation or lifecycle contract corpus into the custody table.

**Implementation & Provenance.** `crates/qcore-contract/src/contract_loader.rs`, `qcore_schemas::ResourceContracts`, `compiler/tests/qcore_contract_corpus.rs`. Cards: `HASH-CONTRACTS` and `PROD-CONTRACTS`.

<a id="c06-declared-inferred-and-proposed-contracts"></a>
## C06. Declared, inferred, and proposed contract standing

##### Trust categories and provenance discipline

`ContractTrust` classifies contract records into four distinct epistemic standings:

1. **`Declared`:** Explicit human-authored contract assumptions (subject to revision-pin, library-version, checked-specification, and body-refutation gates in `C06.2`–`C06.4`). A source analysis result can be sound *relative to* declared assumptions without proving the external implementation of unanalysed callees.
2. **`InferredValidated`:** Machine-derived function summaries produced by the bottom-up source inference engine and independently re-analysed to confirm diagnostic-free `Safe` compatibility across all reachable exits.
3. **`Proposed`:** Unchecked candidate suggestions (such as heuristic, assistant-generated, or unvalidated candidate summaries). **`Proposed` records MUST NEVER justify a `Safe` verdict.**
4. **`PendingHumanReview` ([C06.1](#c061-contracts-awaiting-human-review)):** Declarations or clauses explicitly marked in `.qv` source as awaiting human sign-off or rewrite. Unlike `Proposed` (which awaits automated core validation), `PendingHumanReview` can be resolved *only* by a human editing the contract file, and **MUST justify neither `Safe` nor `Unsafe`**.

A schema enum tag alone is never self-authenticating evidence: `ResourceContracts`'s raw function map does not embed a cryptographic proof bundle on every entry, so audit pipelines and certificate verifiers MUST preserve how each summary was obtained and re-derive or re-validate summaries on replay.

##### Unified contract standing and verdict-effect reference

The following table summarizes every contract standing state across `C06.1`–`C06.6`, whether the contract enters the active function map, how call sites and defining bodies are treated, and the resulting four-value verdict ceiling (`Safe < Pending < Unknown < Unsafe`, language [§L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)):

| Contract standing | Function-map status | Effect on caller call sites | Effect on defining C body (if analysed) | Verdict effect & reason code |
|---|---|---|---|---|
| **Declared — Pinned in force** (`C06.2`) | Active in map | Discharges/imposes declared custody & requirements; recorded in `contract_dependencies.declarations`. | Checked for refutation (`C06.4`) and per-clause body standing (`C06.5`: `validated`, `partly-validated`, or `declared-assumption`). | Admits `Safe` (relative to pinned declaration). |
| **Declared — Tip-tracked in force** (`C06.2`) | Active in map (`tip-in-force`) | Applied as declared; recorded under `assumptions.tip_tracked` (`tip-tracked:FN`) with mandatory RC notice. | Checked identically to any in-force declaration (`C06.3`, `C06.4`, `C06.5`). | Admits `Safe` (with explicit unreleased-branch assumption; never claims reviewed pin). |
| **Declared — Flexible in force** (`C06.2`) | Active in map | Applied by name & ABI at any revision; listed as explicit run assumption (or `libc-c-standard-abi` for `quirin-provider-libc`). | Checked against defining body whenever present in the audit; never skips body checking. | Admits `Safe` (under explicit flexible/libc assumption). |
| **InferredValidated** (`C06`, W4) | Active in unit summary table | Applied to callers within unit (or imported under rule 4c); transitive dependencies tracked (`C06.5`). | Established directly from the defining body's `Safe`, diagnostic-free analysis; re-derived on certificate replay. | Admits `Safe` (invalidated on source/profile/callee change; guards member-release sub-custody). |
| **Checked Specification (`resource_body_checked`)** (`C06.3`) | Active in unit iff established; else withdrawn | Applied if non-recursive unit body establishes all clauses; if withdrawn, callers fall back to W4 summary or uncontracted call. | Analysed under specification's entry custody via monotone withdrawal descent; failure records diagnostic on defining row. | `Safe` when established; defining row never `Safe` when withdrawn. |
| **Proposed** (`C06`) | Never trusted | Cannot discharge obligations. | Requires full independent validation before promotion to `InferredValidated`. | **Never `Safe`** (`Unknown`). |
| **Awaiting Human Review (`resource_review_pending`)** (`C06.1`) | Withheld (`pending_review`) | Rewritten to marked undeclared call; held (`awaiting-human-review`). Second pass checks for masked non-hold refusals. | Analysed as an undeclared function; callers remain held regardless of body analysis. | **`Pending`** if sole obstacle (neither `Safe` nor `Unsafe`); **`Unknown`** if non-hold refusals also exist. |
| **Pin Mismatch / Tip Unresolved / Library Version Changed** (`C06.2`) | Withheld (`pin_mismatch`) | Rewritten to marked undeclared call; held (`contract-pin-mismatch`). Second pass checks for masked non-hold refusals. | Analysed as an undeclared function. | **`Pending`** if sole obstacle (resolvable by human review + `quirin-audit pin`); **`Unknown`** if other refusals exist. |
| **Library Version Out-of-Range, Unreadable, or Ambiguous** (`C06.2`) | Withheld (`pin_mismatch`) | Held with version refusal (`library-version-out-of-range` / `-unreadable` / `-ambiguous`). | Analysed as an undeclared function. | **`Unknown`** (`stale-evidence`; tool decision, never `Pending`, `Safe`, or `Unsafe`). |
| **Body-Refuted Contract** (`C06.4`) | Active on own body; **withdrawn from all callers** (`refuted_by_body`) | Rewritten across all units (phase 2) to marked undeclared call (`contract-refuted-by-body`). | Kept in force on defining body so the defining row retains its refutation (provisional `Unsafe` candidate). | Callers: **`Unknown`** (`stale-evidence`, never `Pending` or `Unsafe`). Defining body: provisional `Unsafe` / non-`Safe`. |
| **Bundle Supersession vs. Bundle Conflict** (`C06.6`) | Strictly newest `pinned_at` active; ties/unstamped cut (`contract_conflict`) | Superseded entries transparently replaced by newest text. Conflicted entries rewritten to marked undeclared call (`contract-conflict`). | Superseded uses winning contract; conflicted analysed as undeclared. | Strictly newest: normal standing. Conflict: **`Unknown`** (`contract-conflict`, never `Pending`, `Safe`, or `Unsafe`). |

##### Conservative bottom-up W4 summary inference (`legacy_summary.rs`)

The C gateway's summary inference (`crates/qcore-c-gateway/src/legacy_summary.rs`) derives `InferredValidated` contracts bottom-up over the direct call graph:

* **Core soundness invariants:** Inference binds formal parameters without optimistically assuming nullable pointers are non-null, requires a completely modelled `Safe` analysis with zero diagnostics, requires at least one reachable return exit and mutually compatible custody dispositions across all reachable exits, distinguishes newly allocated return custody from returning an incoming formal's identity, and refuses hidden or unmodeled custody mutations. Explicit `Declared` contracts always take precedence: a discrepancy between a body's inferred summary and its declared contract is recorded as a summary conflict (subject to `C06.4` and `C06.5` rule 13m), never as permission to overwrite the declaration.
* **Supported parameter/return shapes:** Alongside basic `Borrowed`, `Owned`, `Consumed`, `Released`, and fresh/none returns, inference supports bounded direct `Out`, `Stored`, `Removed`, `Reallocated`, and nullable return modes:
  * **Direct `Removed` wrappers:** Validation seeds the initial state with the child resource contained inside a distinct non-null owner formal. Nullable parameter preconditions are proposed one formal at a time and kept *only* after full re-analysis validates every admitted null/non-null combination (including previously admitted nullable children). A null child carries zero custody; every non-null exit must leave the original child identity directly live and detached while preserving the owner live. Guarded non-null removal helpers and parameter-reordered direct forwarders can validate.
  * **Unsupported inference shapes:** General aliased removal/out-slots, relational cross-variable branch tracking, general recursive fixed points, and quantitative cost inference are unsupported and remain unresolved (`Unknown`). Bounded zero/non-zero context-specialized recursion relies strictly on lower-rank, independently validated summaries; empty optimistic fixed-point seeds are forbidden.
* **Summary invalidation:** Every inferred summary MUST be invalidated whenever the function's AST, any relevant callee contract/summary, compilation flags/profile, contract environment, or source-import coverage changes. In particular, reassigning an output-slot formal parameter (`out = &other;`) MUST NOT retain the parameter's entry destination identity (tracked as a regression guard on W4 out-formal reassignment).
* **Derivation scope for unselected internal-linkage helpers (`QCore.CalleeSummaryScope`):** Under main-source translation-unit selection, inference MAY summarize unselected helper definitions reached via direct calls **if and only if**:
  1. The helper's own definition has **internal linkage** (`static`, without prior `extern`; C11 §6.2.2p3), guaranteeing every call in the translation unit executes that exact analysed body; and
  2. Its identifier is not reserved in all contexts (`__x` or `_X`, C11 §7.1.3), leaving standard-library header implementations to `quirin-provider-libc`.
  External-linkage `inline` definitions MUST NEVER be summarized this way because the linker may choose an external out-of-line definition instead (C11 §6.7.4p7). Admitted `static` helpers undergo full W4 validation, do not alter the unit's reported function denominator, and are re-derived from source during certificate verification.
* **Sub-custody guard for member releases through a formal (`QCore.MemberRelease`):** A function body that passes a struct member `B->F` of formal `B` to a `resource_released(_, nullable)` callee (`free(b->buf); free(b);`) can validate a `resource_released` summary for `B` **only when**: (a) all base pointers resolve with no sub-custody and `B->F` is unbound in tracked state, (b) `B` is never copied, (c) `B->F` is freed at most once (preceded only by null checks, outside loops and non-local jumps), and (d) `B` itself is freed afterward on every path. Because the resulting summary signature mentions only `B` and cannot express child-field releases, **every caller** invoking an in-unit non-declared function whose body (or transitive callee) performs such a member release MUST refuse any actual argument that owns tracked sub-custody (field links, contained children, or indexed collections) *before* applying the summary. Non-null member releases and releasing `&B->F` remain refused.

**Implementation & Provenance.** `crates/qcore-c-gateway/src/legacy_summary.rs`, `qcore_schemas::ContractTrust`. Reports: [W4 nullable removal](reports/w4-nullable-removal/README.md). Lean models: `QCore.CalleeSummaryScope` and `QCore.MemberRelease`. Cards: `W44-CORPUS-CALLEE-SUMMARIES` and `W45-CALLEE-SUMMARY-CASCADE`.

<a id="c061-contracts-awaiting-human-review"></a>
### C06.1. Contracts awaiting human review

##### Purpose and separation from `Proposed`

When drafting external C library contracts—whether by human inspection or automated tooling—an author may know the intended clause shape while needing human verification of subtle C edge cases (such as error-path cleanup or hidden pointer retention). C06.1 introduces `ContractTrust::PendingHumanReview` via `resource_review_pending`.

`Proposed` is intentionally **not** reused for this role for two foundational reasons:

1. **Who resolves it:** A `Proposed` summary awaits *automated core validation*, whereas a `PendingHumanReview` marker can be resolved *only* by a human editing the `.qv` source file (which alters the module's text and SHA-256 digest in the ordinary, auditable way—there is no out-of-band sign-off database or tool-minted approval).
2. **Symmetric non-decision (`Pending`):** Whereas `Proposed` is barred only from justifying `Safe`, a `PendingHumanReview` declaration discharges nothing **and accuses nothing**—it MUST justify neither `Safe` nor `Unsafe`.

##### Contract-level and clause-level syntax and dispositions ([G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1))

A declaration may carry **either** one contract-level review marker under `requires` **or** one or more clause-level wrappers around distinct `ensures normal` clauses:

```qcore
// 1. Contract-level deferral (mandatory reason + optional owner/reviewer):
extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending("is the handle freed on the error path?", "owner");
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

// 2. Clause-level deferral (acceptable as written once verified):
    ensures normal resource_review_pending(resource_released(h, nonnull), "verify nonnull requirement");

// 3. Clause-level deferral marked as requiring a contract rewrite before acceptance:
    ensures normal resource_review_pending(
        resource_borrowed(data),
        "retained by handle, not borrowed",
        rewrite_before_acceptance
    );
```

* **String literal rules:** The first string argument (`reason`) is **mandatory**; the second string argument (`owner`) is optional. Both MUST be single, non-blank string literals containing no escape sequences and no control characters.
* **Disposition categories:** Adding the trailing keyword `rewrite_before_acceptance` (after `reason`, or after `reason, owner`) classifies the marker as `must-be-rewritten-before-acceptance`—recording structurally that simply unwrapping the inner clause would be unsound because the body behaves differently from the placeholder clause. Omitting the keyword classifies the marker as `acceptable-as-written-after-review`. Categories are determined strictly by syntax, never by parsing prose inside `reason`, and both yield `Pending` when they are the sole obstacle.
* **Total-or-reject loading rejections:** The entire module is rejected if a marker has a missing/blank/non-literal reason, three string arguments, a misspelled or misplaced disposition token, two contract-level markers, a contract-level marker alongside a clause-level marker, nested markers, a clause-level wrapper around `resource_contract_v1` or a literal-argument premise (`C04.5`, which can be deferred only at contract level), a contract-level form inside `ensures`, or a clause-level form inside `requires`. Wrapped inner clauses undergo full C04 semantic validation as if written bare.

##### Withholding, non-masking `Pending` semantics, inheritance, and reporting

* **Withholding from the function map:** Loaded pending declarations are stored in `ResourceContracts::pending_review` (retaining module, full contract, markers, and `source_assumptions` text) and **omitted from the active function map** so unaware consumers fail closed.
* **Whole-conjunction holding and non-masking second pass:**
  * Both contract-level and clause-level markers hold **every call site** of the function under `awaiting-human-review`: because a v1 contract operates as an indivisible conjunction, dropping only a single deferred clause could allow weaker remaining clauses to misclassify the call.
  * When all unresolved obligations on a function are held calls, the function's verdict is **`Pending`** (§L21.2)—**provided** the function would carry *no other refusal reason* if the held declarations were placed in force as written (gateway rule 10i's second pass; `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`). If placing the held contract in force exposes an independent refusal (e.g., an unmodeled statement or invalid argument), that refusal is preserved and the verdict is **`Unknown`**, preventing a held call from masking unrelated analysis failures.
  * If the analysed unit *defines* a function whose contract is pending review, its body is analysed as an undeclared function, and callers remain held regardless of any W4 summary of its body.
* **Consumer behavior:** The C gateway rewrites calls to pending names to marked undeclared identifiers prior to W4 derivation, validation, and certificate replay, emitting ``call to `NAME` whose declared contract awaits human review (awaiting-human-review)``. `source.extern-call.v1` rejects with `Q023`; observation and lifecycle interpreters reject the resource-v1 clause. Under C15 inheritance, markers (scope, reason, owner, and disposition) copy verbatim without parameter renaming on strings, and child agreement checks forbid shedding a parent's deferral or disposition.
* **Audit reporting:** Reports include a `pending_human_review` section (with explicit zeros when empty, counting `rewrite_markers`) and attach a deterministic `review_holds` array (sorted by callee name and clause order) to every function row recording a held call; publishing a hold for a callee lacking a recorded marker fails the audit run.

**Implementation & Provenance.** `crates/qcore-contract/src/contract_loader.rs`, `crates/qcore-c-gateway/`; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean models: `QCore.ContractDeferredReview` and `QCore.ReviewHoldStanding.ConcreteReason`. Cards: `W45-CONTRACT-DEFERRED-REVIEW`, `W45-USER-CONTRACT-PLUGINS`, `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`, and `W47-REVIEW-HOLD-CONCRETE-REASON`.

<a id="c062-revision-modes-pinned-and-flexible-contracts"></a>
### C06.2. Revision modes: pinned and flexible contracts

##### Revision binding philosophy and `.qv` syntax

External C code changes across upstream commits: a helper that borrowed a buffer in v2.1 may take ownership or free it on error in v2.2. To prevent silent contract rot across upstream upgrades, **every `resource_contract_v1` declaration is `pinned` by default** unless explicitly marked `flexible`.

```qcore
extern fn malloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

// Clause-level override inside a flexible contract (pins the entire conjunction):
    ensures normal resource_revision(resource_released(p, nonnull), pinned);
```

* **Conjunction rule:** A clause without a wrapper inherits the contract-level mode. Because a contract applies at call sites as one conjunction, a declaration is **flexible if and only if its contract-level mode is `flexible` and zero clauses are marked `pinned`**. A single `pinned` clause pins the whole declaration.
* **Total-or-reject loading:** Duplicate/conflicting contract-level revision markers, clause-level markers restating the contract-level mode, modes other than `pinned`/`flexible`, nested revision/review wrappers in either order, clause wrappers on `resource_contract_v1` or literal premises, and swapped `requires`/`ensures` placements reject the whole module. Under C15 inheritance, parent revision markers are stripped so every inheriting child defaults to `pinned` for its own target function unless the child itself specifies otherwise.

##### Pinned contracts and `contracts.lock` (formats v1–v3; v4 in `C06.6`)

A pinned contract is **in force** only when its normalized `.qv` declaration text (tokens joined by single spaces) matches its lock entry *and* the audited C source matches a reviewed source identity in `contracts.lock` (stored one per provider crate beside `Cargo.toml`, or in the nearest ancestor directory up to `Cargo.toml`/`.git` for standalone modules; never inside `.qv` files):

* **Reviewed site identities:** Each lock entry records the normalized `.qv` SHA-256 and one or more **reviewed identities**. Each identity captures every lexical top-level declaration and definition **site** of the C function across the target project (file path, normalized prototype SHA-256, and—for definitions—normalized body SHA-256; comments and whitespace ignored, identifier renames and statement reorderings detected, braces balanced per preprocessor conditional group without running `cpp`) paired with the Git revisions reviewed at those exact sites (for Git inputs, read strictly from `HEAD` object blobs, never dirty working trees). Malformed, unsorted, duplicate, or revisionless v2 locks refuse the audit.
* **Mismatch holding (`contract-pin-mismatch`):** Before analysing any unit, all four audit routes (`current`, `contracts`, configured, and corpus arbiter) verify locks against the audited source. Any discrepancy—`no-lock`, `not-in-lock`, `contract-changed`, or `source-changed`—removes the declaration from the function map into `ResourceContracts::pin_mismatch` and holds every call under `contract-pin-mismatch` (`Pending` when sole obstacle and unmasked under rule 10i's second pass; `Unknown` if other refusals appear; never `Safe` or `Unsafe`), resolved by human review of the C diff followed by `quirin-audit pin`.

##### Tip identities for unreleased branches (`QCore.ContractTipMode`)

When a contract targets an upstream fix residing on an active release-candidate branch rather than a frozen tag, its lock entry (in **lock format v3**, adding a top-level `tip` list alongside `pinned` and `flexible`; mutually exclusive with static reviewed revisions so old commits cannot keep it alive after the branch advances) tracks the branch **tip**:

* **Recorded metadata:** Function, module, contract digest, `repository` (`name`, informational `url` that is never network-fetched, and local `path`), full Git `ref` (`refs/heads/BRANCH`), optional human-facing `release_label`, and `reviewed` commit + non-empty site digests.
* **Read-only Git resolution:** Prior to unit analysis, the audit locates the repo via `QUIRIN_TIP_REPOSITORIES` (`NAME=PATH` list; malformed syntax refuses the audit) or recorded `path`, invokes strictly read-only Git commands (`rev-parse`, `merge-base --is-ancestor`, `ls-tree`, `cat-file`—never writing or fetching), verifies the current tip descends from `reviewed`, and compares the tip's blob sites (including relative paths) against the audited source:
  * Matching contract digest + ancestor-valid resolved tip + identical sites → **in force** as `tip-in-force` (never `pinned-in-force`), recorded as an explicit run assumption `assumptions.tip_tracked` (`tip-tracked:FUNCTION`) because the branch may have advanced past `reviewed`.
  * Missing repo/ref, non-fast-forward history rewrite, unreadable blob, or missing site → held (`tip-unresolved` → `contract-pin-mismatch`). Differing sites → held (`source-changed`).
* **Mandatory RC notice:** Across `pin`, `pin --check`, `pin --verify-contracts`, console tallies, `contract_revision.tip_notices`, and `compare`, a presentation-only notice (`info` when in force/resolvable, `warning` otherwise) names the function, repo, ref, release label, and commit, stating explicitly that the contract **targets a future, unreleased release (a release candidate), not a released version**.

##### `quirin-audit pin` CLI workflow

Locks are written exclusively by `quirin-audit pin` after human review, never during audit runs:

| Flag / invocation | Behavior |
|---|---|
| `pin --source-root DIR` | Adds or refreshes `HEAD`'s site identity across all load-path crates without dropping other revisions (resets to `HEAD` alone if `.qv` text changed). |
| `--function NAME` / `--modules DIR` | Restricts pinning to named functions, or pins standalone modules outside provider crates. |
| `--regenerate` | Rebuilds locks from the current `--source-root` revision alone (preserving tip entries as written). |
| `--drop-revision REV` | Removes `REV` (`>= 7`-char prefix, full hash, or `directory`) from all identities without reading C source, pruning empty identities. |
| `--check [--revision REV]` | Read-only verification against `HEAD` (or `REV` blobs); exits `1` unless every entry is recorded at that revision (or valid via tip). |
| `--verify-contracts` | Source-free check (used by `providers/contracts.py check`) verifying every lock matches current crate declarations, modes, and digests. |
| `--track-tip REF` / `--untrack-tip` | Converts `--function NAME` into a v3 tip entry at `REF`'s commit (with optional `--repository-name`, `--repository-url`, `--release-label`), or converts it back to a reviewed `HEAD` identity. |

##### Flexible mode, standing `libc` assumption, and change comparison

* **Flexible mode:** Binds at any source revision by function name and declared C ABI signature (`LOADER-ABI` still withdraws ABI mismatches). Analysed bodies are still checked against the contract; unanalysed external functions are listed as explicit run assumptions. All flexible contracts in `quirin-provider-libc` without analysed bodies roll up into one standing assumption: **`libc-c-standard-abi`** (that the C standard library obeys ISO C and platform ABI; glibc/musl version is build provenance, not part of the assumption).
* **Sidecar and `compare`:** `contract_revision` records locks, sites, mismatches, assumptions, and inventory; `quirin-audit compare --before A --after B` diffs sources, active contracts, pin mismatches, obligations, tip movements, and verdict transitions—strictly separating **violation detected** (moves to `Unsafe`) from **coverage lost** (`Safe` → `Pending`/`Unknown`) and `Pending` ↔ `Unknown` shifts.
* **Enforcement boundary:** Pin evaluation runs on the four `quirin-audit` routes; raw library callers of the gateway API or `source.extern-call.v1` consume whatever `ResourceContracts` environment they pass without evaluating lock files.

##### External library version ranges (`QCore.LibraryVersionRange`)

Contracts for prebuilt/header-consumed C libraries (such as OpenSSL) may constrain the semantic library versions for which they hold:

```qcore
requires resource_library_version(openssl, since "1.1.0");
requires resource_library_version(openssl, since "1.1.0", removed "4.0.0");
requires resource_library_version(openssl, documented "1.0.2");
```

* **Bounds and `library-versions.json`:** `since` (inclusive) cites the official manual's `HISTORY` addition version; `documented` (inclusive, never an invented `since`) records the oldest checked manual series when no addition version is documented; `removed` is exclusive (`MAJOR.MINOR[.PATCH]` component-wise comparison). Each provider crate declares its probe header and decimal integer macros in `library-versions.json` (e.g., `openssl/opensslv.h` with `OPENSSL_VERSION_MAJOR`/`MINOR`/`PATCH`). Version ranges are not inherited under C15.
* **Compile-database header probing:** Both `quirin-audit pin` (recording `library_versions` in the lock) and audit runs preprocess `#include <HEADER>` via Clang across every distinct `-I`/`-isystem`/`-iquote`/`-idirafter` configuration in the run's compile database (overridable via `QCORE_LIBRARY_VERSION_COMPILE_DATABASE`, recorded as provenance). Unreadable databases or unreproducible header-search flags (`-nostdinc`, `--sysroot`, `-isysroot`, `-iprefix`, `-iwithprefix`, `-I-`, `--include-directory`, `-iframework`, `-F`) **never** fall back to Clang's default system paths—they mark the version unreadable and refuse pinning.
* **In-force vs. `Pending`/`Unknown` holds:** A versioned declaration is in force iff the probed version is **readable**, **unambiguous** across all include configurations, **within** `[since/documented, removed)`, and **equal** to the lock's pinned version. If it differs *only* from the pinned lock version (`library-version-changed`), calls are held as `contract-pin-mismatch` (`Pending`, resolved by review + re-pin); if it is `library-version-out-of-range`, `library-version-unreadable`, or `library-version-ambiguous`, calls are held as **`Unknown`** (`stale-evidence`, never `Pending`).

**Implementation & Provenance.** `crates/qcore-audit/`, `crates/qcore-contract/`; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean models: `QCore.ContractRevisionMode`, `QCore.ContractTipMode`, and `QCore.LibraryVersionRange`. Cards: `W45-CONTRACT-REVISION-MODE`, `CONTRACT-PIN-TIP-MODE`, `OPENSSL-PROVIDER-PLUGIN`, and `LIBRARY-VERSION-PROBE-ROUTE-DATABASE`.

<a id="c063-checked-specifications"></a>
### C06.3. Checked specifications

Unlike external library contracts (which state assumptions about unanalysed binaries), an internal project contract carrying `requires resource_body_checked;` beside `requires resource_contract_v1;` declares an **in-unit specification that the function's own C body MUST prove**.

##### Marker composition and monotone withdrawal descent (`QCore.CheckedSpecification`)

* **Marker layering:** `resource_body_checked` adds no serialized schema fields (consumers re-derive it from retained `source_assumptions` via `declared_checked_specifications`) and cannot be wrapped by clause-level markers. Revision modes (`C06.2`) and review holds (`C06.1`) peel off first: an unpinned or review-held declaration is already out of force and never acts as a specification.
* **Three mandatory establishment conditions:** Within an analysed translation unit, a checked specification remains in force (both at its own entry and for callers in that unit) **if and only if**:
  1. **In-scope body:** The unit defines the function's C body within its derivation scope (`C06`);
  2. **Non-recursiveness:** The function does not belong to any cycle in the unit's direct call graph (preventing circular assume-guarantee self-justification under constraint 4); and
  3. **Full discharge:** Analysed under its declared entry custody—using *only* checked specifications that themselves remain standing—the body is completely modelled, emits zero diagnostics, matches every declared clause at every reachable exit (`observations_match_contract`), and produces no `SummaryConflict` against W4 inference (C04.1).
* **Greatest-fixpoint withdrawal descent:** Validation runs iteratively: each round withdraws every `resource_body_checked` contract that fails condition (1), (2), or (3) under the currently standing set, repeating until reaching a fixed point where all remaining checked specifications establish themselves cleanly.
* **Failure and consumer boundaries:** A withdrawn specification is never trusted—callers in the unit see only whatever independent W4 summary validates from the body (or an uncontracted call), and the defining function's row records ``declared specification of `NAME` is not established: REASON`` (never `Safe`). An established specification vouches only for that analysed unit/revision (mismatched C signatures at other revisions fail closed at the ABI gate). Providers MUST NOT rely on `resource_body_checked` with legacy consumers predating C06.3 that ignore the marker; the C gateway and all `quirin-audit` routes enforce it.

**Implementation & Provenance.** `crates/qcore-contract/src/contract_loader.rs` (`declared_checked_specifications`), `crates/qcore-c-gateway/`; grammar [G05](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1). Lean model: `QCore.CheckedSpecification`. Card: `W45-FB-BACKLOG-CUSTODY`.

<a id="c064-contracts-their-own-bodies-refute"></a>
### C06.4. Contracts their own bodies refute

Even when a contract is *not* marked `resource_body_checked`, whenever a translation unit happens to define the C body of a declared function, the consumer checks that body against the declaration. If the body actively **contradicts** its own contract, continuing to trust that contract at call sites in the same or other translation units would be unsound.

##### Refutation criteria vs. benign refinements

A defining C body **refutes** its declared contract when either:

1. **Counted-user refutation:** A declared counted-user clause or implicit uncovered-exit live-count obligation on the function's own row is refuted (C04.7 rules 2 and 4); or
2. **Custody duty contradiction:** The independently validated W4 summary of the body contradicts a declared parameter or return custody *duty* (C04.1's mode gate: `mode-parameter-contradiction` or `mode-result-contradiction`, e.g. freeing a `resource_borrowed` parameter or leaking a `resource_released` one).

By contrast, a **refinement within the same duty** (such as stricter/looser nullability, owner-index specificity, or parameter replacement), an undecided clause, an unmodelled body that fails W4 validation, or the absence of a defining body is **not** a refutation—those leave the declaration standing as an unchecked `Declared` assumption.

##### Asymmetric caller withdrawal and deterministic two-phase audit (`QCore.RefutedContractWithdrawal`)

* **Keep on body, withdraw from all callers:**
  * On the **defining body itself**, the refuted declaration stays in force so the defining function's row preserves the refutation diagnostic (a provisional `Unsafe` candidate, never presented as an upstream bug).
  * For **every caller** outside that body, the declaration MUST be withdrawn: call sites are rewritten to marked undeclared names and refuse with ``call to `NAME` whose declared contract its own analysed body refutes (contract-refuted-by-body)``.
  * The caller's refusal is **`Unknown`** (`stale-evidence`, subject: analyser—so it never backs a provisional `Unsafe` and is **never `Pending`**, because refutation is a tool finding rather than a human sign-off hold).
* **Deterministic two-phase cross-unit propagation:** Because function `f` may be defined in `a.c` and called in `b.c`, every audit route executes:
  1. **Phase 1:** Analyses all translation units under the loaded contracts (already withdrawing intra-unit calls to functions refuted in the same unit) and collects `contract_refutations` from defining rows into the order-independent **refutation table** (`ResourceContracts::refuted_by_body`, hashed via canonical `table_digest` even when empty).
  2. **Phase 2 (when `refuted_by_body` is non-empty):** Re-analyses all units with `refuted_by_body` populated so every caller in every unit withdraws the refuted names, publishing Phase 2's rows. (When the table is empty, Phase 2 is skipped since its output would be identical.)
  Separate/checkpointed unit results MUST record `contract_refutations` and bind `table_digest` into their composition identity (refusing to compose mismatched digests). Audit sidecars report `refuted_contracts` (applied table, digests, phase count, and held call counts).

**Implementation & Provenance.** `crates/qcore-c-gateway/`, `crates/qcore-audit/`, `qcore_schemas::ResourceContracts::refuted_by_body`. Lean model: `QCore.RefutedContractWithdrawal`. Card: `W45-REFUTED-CONTRACT-WITHDRAWAL`.

<a id="c065-per-verdict-contract-dependencies"></a>
### C06.5. Per-verdict contract dependencies

To make the trust footprint of every `Safe` verdict transparent to reviewers and downstream consumers (such as `C04.20`'s outer-contract projection), every published legacy-C function row records `contract_dependencies` and every audit sidecar publishes a clause-grained `body_standing` ledger. Both are pure **evidence**: no static verdict reads them, and stripping them leaves verdicts byte-for-byte unchanged.

##### Per-row consultation tracking and closed-world/null dependency lists

Each row's `contract_dependencies` captures a sound superset (never omitting a consulted contract) of every lookup performed while analysing that function—its own entry contract, every callee name spelled in its AST or applied on its solved CFG, and transitively all dependencies of every consulted in-unit W4 summary—partitioned into:

* `declarations` (in force at consultation), `held` (review or pin holds), `withdrawn` (refuted or body-withdrawn), and `summaries` (in-unit W4 summaries);
* `run_set_members` (non-empty list of `TAG.FIELD` keys from consulted `C04.19.3` run-set member invocations, rule 12n) and `stated_members` (non-empty list of constructor-stated `TAG.FIELD` keys from `C04.19.4`, rule 14e), consumed as closed-world facts by `C04.20`; and
* `null_premises` (`null-deref:FUNCTION:LINE:VARIABLE` entries from the function and its consulted summaries whenever custody relied on `C09.3` decision 1's non-null alias reading, rule 14n—preventing such rows from ever being tallied as "resting on nothing").

##### Clause-grained `body_standing` (`QCore.BodyClauseStanding`)

Body checks validate *specific clauses*, never an entire declaration by proximity. A declaration's individual clauses—and the exact rules capable of validating each—are partitioned as follows:

| Clause family on a declaration | Body-validation evidence capable of covering it |
|---|---|
| Parameter custody modes + kinds, and return custody mode + kind (the *custody projection*) | Equal W4 summary, C06.3 entry-custody analysis, or **rule 13m** direct entry check (`13m`, below)—even when container/counted-user clauses are also present (covering the custody projection only). |
| Return-code exit classes (`C04.13`) | Gateway rule `11r`. |
| Guarded retention predicates (`C04.14`) | Gateway rule `12w`. |
| Slot moves (`C04.15`), derived out slots (`C04.16`), and function-map borrowed results | Gateway rule `13a`. |
| Reference keeps and drops (`C04.17`) | Gateway rule `13c` (keeps/drops) and rule `13d` (returned references). |
| Member-path states and helper takes (`C04.19.6`) | Gateway rules `30` and `50`. |
| Container (`C04.8`), counted-user (`C04.7`), unguarded `resource_retained_by`, holder resets, named premises (`C04.18`), variadic tails, `resource_no_return()` | **Uncovered** (always `declared-assumption`; structural checks on neighbour clauses never validate adjacent `resource_borrowed` formals). |
| Caller-side domain bounds: literal-argument premises (`C04.5`), loop-owner / thread-sync / lifecycle prerequisites (`C04.9`–`C04.11`) | *Caller obligations* (not body claims; excluded from the body-clause denominator). |

Across all units in the run that examined a defining body of the declaration (requiring at least one such unit and zero body failures):

* **`validated`:** Every clause of the declaration was covered in every examining unit.
* **`partly-validated`:** All examining units succeeded on what they checked, and *some*—but not all—clauses were covered (explicitly listing `uncovered_clauses`, each remaining a declared assumption).
* **`held`:** Withheld for human review (`C06.1`) or pin mismatch (`C06.2`).
* **`declared-assumption`:** No body was analysed in the run, or an examining unit failed to validate it. **A declared assumption is never labelled `validated`.**

The sidecar roll-up partitions `Safe` rows into those resting on *no declarations*, *only fully `validated` declarations*, *`partly-validated` declarations (with zero declared assumptions)*, *`null_premises`*, and *`declared-assumption` declarations* (highlighting the top 20 assumptions by dependent `Safe` count).

##### Direct entry-custody check for nullability refinements (`QCore.DeclaredEntryCheck`, rules 13m–13n)

Because W4 inference seeds bound pointer formals as `nonnull` and never weakens them, a nullable destructor wrapper such as `my_free(p) { free(p); }` declared `resource_released(p, nullable)` produces a W4 summary (`nonnull`) that differs from its declaration without violating any custody duty. Whenever a function-map declaration is its own pure custody projection (no container or counted-user clauses), is in force, is neither refuted (`C06.4`) nor a withdrawn `C06.3` specification, and differs from W4 *only* by a mode-compatible nullability refinement (`LOADER-ABI-MODES-REFINEMENT`), the evidence collector analyses the body directly under the **declaration's** entry custody on the reported CFG. If every exit matches the declaration and finishes `Safe` with zero diagnostics, evidence `13m` is recorded (validating all parameter and result clauses) instead of a body failure, without altering verdicts or consultation sets.

**Implementation & Provenance.** Gateway rules 13f–13n and 14n (`crates/qcore-c-gateway/`, `crates/qcore-audit/`). Lean models: `QCore.ContractDependencies`, `QCore.BodyClauseStanding`, and `QCore.DeclaredEntryCheck`. Cards: `VERDICT-CONTRACT-DEPENDENCIES`, `OUTER-CONTRACT`, `NULL-SAFETY-PROPERTY`, `VERDICT-BODY-STANDING-CLAUSE-SCOPE`, and `DECLARED-ASSUMPTION-BODY-CHECKS`.

<a id="c066-contract-supersession-across-bundles"></a>
### C06.6. Contract supersession across bundles

When modular contract bundles are combined, two bundles may declare contracts for the same C function as an updated bundle supersedes an older one. Rather than failing on load-order-sensitive duplicate errors, **lock format v4** resolves multi-bundle declarations deterministically via reviewed RFC 3339 **pin timestamps**.

##### Lock format v4, `pinned_at` timestamps, and monotonic regeneration

* **Format v4 and `pinned_at`:** Every lock entry (`pinned`, `flexible`, or `tip`) in a v4 lock carries `pinned_at` formatted strictly as UTC `YYYY-MM-DDTHH:MM:SSZ` (whose ASCII lexicographic order equals chronological order), alongside a lock-wide `fluent_bit_revision` binding. Legacy v1–v3 locks remain readable as unstamped history, while `pin --verify-contracts` (and `providers/contracts.py check`) requires v4 locks with valid stamps on all provider crates.
* **Strict stamp monotonicity (`PIN-REGENERATE-STAMP-MONOTONICITY`):** Stamps reflect human review events written by `quirin-audit pin`, never filesystem `mtime` or load-time clocks. Whenever `pin` updates, changes mode on, or `--regenerate`s an entry that already existed on disk, the new `pinned_at` MUST be strictly later than the replaced disk entry's stamp: an explicit user-supplied stamp that is not strictly later refuses the entire `pin` run (writing zero locks), and a default wall-clock stamp that is not later automatically advances to `previous_stamp + 1s`.

##### Maximum-stamp selection vs. `contract-conflict` residue (`QCore.ContractSupersession`)

For any C function declared in two or more selected resource modules:

1. **Unique newest text wins (`strictly_newest_wins`):** A declaration's active stamp is its lock entry's `pinned_at` *only* when that lock entry matches the declaration's current normalized text. If **every** competing declaration has a valid active stamp and all declarations sharing the maximum `pinned_at` have **byte-identical normalized text**, that newest text is **in force** (if byte-identical texts appear in multiple modules at the winning stamp, the lexicographically least module key retains it). Every older/other copy is **superseded**: its declaration span is overwritten with ASCII spaces while preserving newline positions (keeping byte offsets, spans, and digests of sibling declarations in that module unchanged; modules left empty are dropped), and `check_source_assumptions` replays the cut texts. Because maximum over a total order is associative and commutative, bundle union is completely independent of module load order.
2. **Unresolved tie or missing stamp → `contract-conflict` (`Unknown`):** If two declarations share the maximum stamp with *different* texts (`equal-stamps`), or if *any* competing declaration lacks a stamp (`missing-stamp`), has a bad timestamp (`malformed-stamp`), has a text mismatch against its stamp (`stale-stamp`), or is unpinned (`unpinned`—including any loader call invoked without lock stamps), **zero declarations of that function enter force**. All copies are excised into `ResourceContracts::contract_conflict`, and every call site refuses with ``call to `NAME` whose contract bundles conflict: their pin stamps do not decide which declaration is in force (contract-conflict)`` → **`Unknown`** (analyser-subject `contract-conflict`, never `Pending`, `Safe`, or `Unsafe`).
3. **C15 inclusion and vocabulary boundaries:** Excising a declaration from a module elaborated via C15 `#include`/`inherit` refuses the entire selection (preserving byte-exact inclusion replay). Supersession applies exclusively to `resource_contract_v1`; `bounds_contract_v1` and `outer_contract_v1` retain strict duplicate rejection.
* **Sliding 3-revision lock window (`providers/pin-lock-window.json`):** Provider crates maintain at most three lock sets: the current live set in `providers/<crate>/` plus up to two historical sets archived byte-for-byte under `providers/pin-lock-sets/<revision>/`, managed via `pin --window-add` (refusing a fourth), `pin --window-retire`, and `pin --window-rotate`.

**Implementation & Provenance.** `crates/qcore-contract/`, `crates/qcore-audit/` (`crates/qcore-audit/tests/pin_regenerate_stamps.rs`), `providers/pin-lock-window.json`. Lean model: `QCore.ContractSupersession` (`strictly_newest_wins`). Cards: `CONTRACT-SUPERSESSION` and `PIN-REGENERATE-STAMP-MONOTONICITY`.

---

# Part IV — Orthogonal C Property Contracts: Bounds, Indices, Null Dereference, and OS Cleanup

Resource custody (`resource_contract_v1`, Part II) proves who owns and releases tracked resource identities, but intentionally makes zero claim about array/pointer bounds, null-pointer dereferences, C library integer return ranges, or partial OS syscall rollback. Part IV specifies the orthogonal contract vocabularies, quantitative type groundings, body/caller checks, and four-value verdict rules governing those separate C properties.

---

<a id="c09-custody-access-lifetime-and-quantitative-clauses"></a>
## C09. Custody, access, lifetime and quantitative clauses

### Orthogonal semantic domains

Four distinct contract domains govern systems C safety, and none may silently substitute for another:

| Domain | Core question answered | Representative contract clauses |
|---|---|---|
| **Custody** | Who remains responsible for releasing or transferring a tracked resource identity? | `resource_acquired`, `resource_borrowed`, `resource_released`, `resource_retained_by` (`C04`) |
| **Access & Bounds** | Who may read or write an identified byte/element footprint, and does the extent cover the index? | `bounds_extent`, `bounds_frame_*`, `quant checks P, Idx #i: ...` (`C09.1`, `C09.2`) |
| **Existence & Lifetime** | Does a pointer name an existing non-NULL identity whose lifetime remains valid? | `quant null-deref #B`, `quant live #B`, `nonnull_member`, `resource_published_to` (`C09.3`, `C04.8`) |
| **Quantitative Accounting** | How do credit, live-user counts, or cost bounds evolve across exits? | `resource_users_*` (`C04.7`), `quant<credit \| live, ...>` ([§L08](QCORE_SPEC_r0_3.md#l08-credit-live-obligations-cost-and-access-quantities)) |

### Exit-by-exit identity conservation

A complete resource contract `SHOULD` specify its identity-bearing pre-state, operational effects, every reachable return/error exit, each exit's custody and access post-state, and its cleanup protocol (plus transfer/synchronization witnesses, task quiescence conditions, and covering mutex invariants for concurrent code):

- **Identity-grained accounting (never scalar `net == 0`):** On every exit and for every tracked resource identity, **one exact disposition** `MUST` be established — retained by the current custodian, transferred to a specific new custodian/owner, released, or held as an explicit residual/poisoned obligation. Releasing one allocation while leaking another of equal byte size is **not** balanced custody.
- **No silent erasure of `live` obligations:** Credit may be returned or discarded only when permitted by its axis policy. Live obligations `MUST NOT` be erased by error-region exit, cancellation, failed cleanup, lost pointers, parent struct deallocation, or cost bounds.
- **Unmodeled exits fail closed:** A `normal`-return postcondition states nothing about exceptional, cancellation, or non-terminating exits; strict consumers reject exits they cannot represent rather than assuming safety.

---

<a id="c091-reviewed-index-and-extent-contracts-planned"></a>
### C09.1. Reviewed index and extent contracts (`c.bounds.index-extent.v1`)

#### Property separation and vocabulary isolation

`c.bounds.index-extent.v1` is a separately selected existing-C safety property (`QCORE_BOUNDS_PROPERTY`) backed by the dedicated `bounds_contract_v1` loader ([G05b](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05b-bounds-contract-v1)). Neither `resource_contract_v1` nor `index.closed` (`§L06`) is widened by bounds contracts, and a custody `Safe` verdict never advertises an array or pointer bounds guarantee. When an audit requests both custody and bounds, both per-property verdicts and their reasons remain individually visible alongside their joined lattice result (`Safe < Pending < Unknown < Unsafe`, so `custody: Safe` $\sqcup$ `bounds: Unknown` = `Unknown`).

#### Consolidated `bounds_contract_v1` capabilities (gateway rules `11s`, `11e`, `12r`, `14f`–`14j`)

1. **Extent and index clauses (`CG16-INDEX-AND-OFFSET-EXTENTS`):**
   - `bounds_extent(P, EXT, elements | bytes)` binds pointer parameter `P` to an accessible element count or byte length (`EXT` a positive decimal literal or scalar parameter).
   - `bounds_index(IDX, LIMIT)` constrains scalar index parameter `IDX` to `0 <= IDX < LIMIT`.
   - `bounds_global_extent(G, EXT, elements | bytes)` and `bounds_return_extent(EXT, elements | bytes)` state external storage and return extents; `bounds_preserves_extents()` preserves caller extents across calls; `bounds_review_pending("REASON", ...)` places the declaration on human-review hold (`Pending` when sole obstacle).
2. **Closed mutation frames and derived return relations (`CG24-ALIAS-EXTENT-FRAME-VOCABULARY`, rule `11e`):**
   - Closed frames (`bounds_frame_reads(P)`, `bounds_frame_writes(P)`, `bounds_frame_moves(P)`) apply to pointer and slot (`T **`) parameters independently of custody modes:
     - **`reads`:** preserves all caller bounds facts;
     - **`writes`:** invalidates scalar facts on address-taken locals passed to `P` while preserving extent facts on written objects;
     - **`moves`:** models in-place reallocation (such as `cfl_sds_cat_safe`), additionally invalidating facts not established by source.
   - Derived return relations (`bounds_return_derived(P, OFF, LEN, UNIT)` over one or more candidate bases) state that the result equals `P + OFF` with `LEN` units, checked against `P`'s extent when known (contradictions report and yield no fact). Analysed bodies that write through a `reads` parameter or return a mismatched base/offset **refute** their declaration (`C06.4`).
3. **Provider `bounds_view`, mandatory pinning, and two-phase refutation (`CHUNKIO-META-LEN-BOUNDS-CONTRACTS`, `CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS`):**
   - Provider crates list bounds modules in their generated `bounds_view` (disjoint from `resource_view`). Under `QCORE_BOUNDS_PROPERTY`, audit routes load all `bounds_view` modules on the load path before unit analysis.
   - Every bounds declaration (except `C09.4` libc `result_set` declarations) is **pinned** in `bounds-contracts.lock` via `quirin-audit pin --bounds` (supporting branch-tip tracking as in `C06.2`, verified by `pin --verify-contracts`). Unpinned or stale declarations (`no-lock`, `not-in-lock`, `contract-changed`, `source-changed`) are held under `contract-pin-mismatch` (`Pending` at most — discharging nothing and accusing nothing).
   - Phase 1 checks every defined body (including header-defined inline helpers such as ChunkIO's `cio_file_st_get_meta_len`, `cio_file_st_set_meta_len`, `cio_file_st_get_hash`, `cio_file_st_infer_content_len`, and `cio_file_st_set_content_len`) against its declaration, publishing C06.5-shaped clause-coverage evidence and withdrawing any body-refuted declaration across all callers in Phase 2.

#### Normative access-bounds rules and verdict mapping

- **Element vs. byte arithmetic and one-past formation:**
  - An element index fact means $0 \le i < N$ (`Idx<N>` in conceptual prose is not `§L06` `Idx + Ordered` syntax, and runtime C values never enter `index.closed`).
  - A byte access of width $w$ at offset $o$ into an extent of $B$ bytes `MUST` establish $0 \le o$, $0 < w$, and $o + w \le B$ under target-accurate C signedness, integer promotions, and width arithmetic (never assuming absence of C undefined behavior to prove bounds).
  - Forming a one-past pointer ($i = N$) and dereferencing it are distinct obligations: $i = N$ never satisfies element access, and an empty extent ($N = 0$) admits zero accesses.
- **Source precedence over user contracts:** Fixed-array extents (`int arr[4]`) and layout facts derived directly from Clang AST/target evidence need no human review, and a user declaration `MUST NOT` enlarge a source-established object (`arr[7]` on `int arr[4]` is detected without a user contract and cannot be overridden by claiming 8 elements).
- **Four-value bounds verdict table:**

  | Analysis outcome for `c.bounds.index-extent.v1` | Published verdict |
  |---|---|
  | Every inventoried subscript, `*`, `->`, pointer formation, pointer difference/comparison, and call precondition is discharged under in-force premises | `Safe` *(for bounds only)* |
  | Held declarations (`bounds_review_pending` or `contract-pin-mismatch`) are the sole obstacle, and putting them in force leaves zero unresolved obligations | `Pending` *(naming held clauses and required human act)* |
  | Unresolved extent/index/alias/preservation step, unread C construct, contradicted declaration, or hypothetical violation under a held declaration | `Unknown` *(retaining specific reason codes)* |
  | Reachable out-of-bounds access independently validated under the recorded target and in-force premises | `Unsafe` *(provisional bounds finding)* |

**Open residuals (`CG16-BOUNDS-CONTRACT-TRANSPORT`, `CG20`):** `#include`/`inherit` and `.qo` checkpoint transport for bounds modules, symbolic index ranges beyond `[0, f)`, VLA/flexible-array extents (`CG17`), and callback-spanning extent relations.

**Implementation & Provenance.** `crates/qcore-contract/src/bounds_loader.rs`, `crates/qcore-core/src/index_extent.rs`, `crates/qcore-c-gateway/src/index_extent.rs`. Lean: `QCore.IndexExtent`. Cards: `CG16-INDEX-AND-OFFSET-EXTENTS`, `CG24-ALIAS-EXTENT-FRAME-VOCABULARY`, `CHUNKIO-META-LEN-BOUNDS-CONTRACTS`, `CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS`.

---

<a id="c092-quantified-index-checks-quant-checks"></a>
### C09.2. Quantified index checks: `quant checks`

#### Concept and collocated check placement

Within a `bounds_contract_v1` declaration ([G05c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05c-quantified-index-checks), gateway rule `12l`), `quant checks` states a **universally quantified element-index envelope** over a pointer parameter — simultaneously acting as a body check on the callee and an extent obligation on every caller:

```qcore
module contract;

extern fn cio_file_st_get_meta_len(map: ptr<mut, u8>) -> u16
contract {
    requires bounds_contract_v1;
    bounds_frame_reads(map);
    quant checks map, Idx #i: 22 <= #i && #i < 24;
};
```

`quant checks P, Idx #i: PRED;` sits inside the contract block with no exit selector (preserving `quant normal ...` for postconditions and keeping `<`/`<=` out of parameter type lists; `Idx #i` is the clause's local integer binder, and `&&`/`||` are admitted by the operator gate inside `quant checks` clauses). At most one `quant checks` clause may appear per pointer parameter.

#### Exact interval denotation (`QCore.IndexQuant`) and body/caller rules

- **Universal quantifier and literal connective semantics:**
  - `#i` ranges universally over mathematical integers counting elements of `P`'s C pointee type. Comparisons (`==`, `<`, `>`, `<=`, `>=`) place `#i` alone on one side against a **bound term** (canonical decimal literal, scalar parameter entry value, or `+`/`-` with at most one scalar parameter added once), combined with `&&` (tighter) and `||`.
  - `||` is interpreted **literally** in disjunctive normal form (`QCore.IndexQuant.mem_reading`): writing `#i < 24 || 0 <= #i` denotes $\mathbb{Z}$ and is **rejected at load time as unbounded** (never silently rewritten to `&&`). Constant predicates denoting the empty set also reject the module.
  - Admitted fact representations cover (1) bounded constant interval unions and (2) symbolic prefixes `0 <= #i && #i < f` (equivalent to `bounds_extent(P, f, elements)`); other symbolic parameter forms load cleanly and yield `Unknown` (never `Safe`).
- **Three-part checking discipline:**
  1. **Callee body check:** Seeds `P` with the minimal contiguous extent required by callers and tests every `P[k]`, `*P`, `P->f` (and constant-offset aliases `P + c`) against the admitted set: if every possible index value lies **inside**, normal extent checking proceeds; if **disjoint** (no value admitted), the body **refutes** its declaration (withdrawing it from callers under `C06.4`) and marks the access `Unknown`; otherwise `Unknown`.
  2. **Caller precondition check:** Verifies that the actual argument's established extent covers both the minimum and maximum admitted indices (`Unsafe` only when definitely reachable under in-force premises; `Unknown` if symbolic set is empty or actual extent is unknown).
  3. **Callee escape:** Passing `P + c` to a further callee succeeds only when that callee's own in-force constant `quant checks` set, shifted by `c`, is a subset of the caller's admitted set (`Unknown` otherwise, never a refutation).

**Implementation & Provenance.** `crates/qcore-contract/src/bounds_loader.rs`, `crates/qcore-core/src/index_quant.rs`, `crates/qcore-c-gateway/src/index_extent.rs`. Lean: `QCore.IndexQuant` (`mem_reading`). Card: `QUANT-INDEX-CHECKS`.

---

<a id="c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref"></a>
### C09.3. Null dereference: the property `c.null.deref.v1` and `quant null-deref`

#### 1. Property scope and dominating-dereference transfer

`c.null.deref.v1` (`QCORE_NULL_PROPERTY`, gateway rules `14k`–`14z`, `15q`–`15u`, `39`) is an orthogonal existing-C analysis property published alongside custody and bounds. It inventories every **evaluated** pointer dereference (`*p`, `p->f`, `p[i]`, pointer atomics) and every call-site actual passed to a `quant null-deref` formal, while excluding unevaluated contexts (`sizeof`, `_Alignof`, `typeof`, unselected `_Generic` arms) and pure address formations (`&B->G`, `&B[i]`, `&*E`, array decay; indirect calls via function pointers remain a named residual).

- **Moving the NULL question (`CG10` resolution):** Whenever every path reaching a use of `B` (or `&B->F` / `B ± n` per C11 §6.5.6p8) has already executed a real read or write dereferencing `B` without reassigning `B` in between, the later check is marked **`moved`** to that dominating dereference, concentrating review on the single origin check.
- **Custody bridge (`null-deref:FUNCTION:LINE:VARIABLE`, rule `14n`):** When custody analysis encounters a dereference through an unwidened, target-free, possibly-NULL path binding, it reads the base as non-NULL *conditional on* the explicit dependency `null-deref:FUNCTION:LINE:VARIABLE` recorded in `null_premises`, `contract_dependencies`, and the `Safe` roll-up — while `c.null.deref.v1` independently reports whether that dereference check is proved, held, a candidate, or a finding.
- **Sources of non-NULL vs. possibly-NULL:**
  - *Non-NULL:* `&obj`, array decay, string literals, function designators, `errno` address (C11 §7.5p2), in-force `resource_acquired(..., nonnull)` or `nonnull_result()`, in-unit bodies whose every return is non-NULL, entry `quant null-deref #B;` (declared or inferred), surviving NULL-guard branches (`if (p)`, `p != NULL`, `&&`/`||`/`?:`, `__builtin_expect`, early-exit/no-return guards), `assert(p)` (recording premise `NDEBUG` absent), and dominating dereferences.
  - *Possibly-NULL:* nullable allocations/lookups, uncontracted pointer returns, unannotated pointer formals, memory loads (`s->next`, `*pp`, globals, address-taken locals; forgotten across calls and pointer-capable stores, preserved across non-char scalar stores under C11 §6.5p7 effective-type premise), `NULL` constants, and uninitialized locals. Loops iterate to a fixed point (`Unknown` if unconverged); `goto` target labels forget path facts.

#### 2. Quantitative type grounding (`QCore.NullDerefGrade`) and 3-part dereference decomposition (`QCore.DerefDecomposition`)

`quant null-deref #B;` is grounded formally as the contract-level projection of a **[§L07 usage grade](QCORE_SPEC_r0_3.md#l07-usage-grades)** composed with **[§L08 access shares](QCORE_SPEC_r0_3.md#l08-credit-live-obligations-cost-and-access-quantities)** and **[§L09 resource identity](QCORE_SPEC_r0_3.md#l09-ownership-identity-and-borrowing)** (distinct from `§L06` static index dimensions):

- **Body dereference grade:** Along each terminating path, dereferencing formal `B`'s entry value (or passing it to an in-force `quant null-deref` formal) adds $1$ to `B`'s demand until the first NULL test on `B` or reassignment of `B` closes the account. Joining terminating paths via interval hull $[\min, \max]$ yields `relevant` ($[1, \infty]$) iff **every** terminating path dereferences `B` before testing or reassigning it, and lower bound $0$ (`many`) otherwise.
  - **In-unit inference (rule `14p`):** Computed bottom-up (cutting recursive cycles), any body with grade `relevant` on `B` infers `quant null-deref #B;` for callers in the same unit (and exports inferred clauses and all-non-null return facts across units for unique external definitions via post-pass travel without weakening rows).
  - **One-sided callee validation:** Unlike linear consumption, borrowing a read share spends nothing; thus a declared `quant null-deref #B;` on a body with lower bound $\ge 1$ reports `necessary`, and with lower bound $0$ reports `stronger-than-necessary` (sound, never rejected).
- **Three-part dereference decomposition and two-level surface syntax ([G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks)):**
  Dereferencing `*B` or `B->f` decomposes into three orthogonal `quant` projections loaded through `bounds_contract_v1` (`crates/qcore-contract/src/deref_parts.rs`):

  | Surface spelling | Canonical elaborated parts | Owning system & theorem boundary (`QCore.DerefDecomposition`) |
  |---|---|---|
  | `quant deref #B;` *(shorthand)* | `quant null-deref #B;` + `quant checks B, Idx #i: #i == 0;` + `quant live #B;` | Elaborates before consumers run; agreeing explicit parts merge idempotently (`quant-deref-conflict` on disagreement); pin `text_sha256` hashes written text while `canonical_sha256` hashes sorted canonical parts. |
  | `quant null-deref #B;` | Existence `(a)` alone | `checks(B, #i == 0)` mathematically entails `null-deref(B)` (unused automatically until `NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`), whereas the converse fails (one-past pointers are non-NULL yet out of bounds). |
  | `quant checks B, Idx #i: #i == 0;` | Extent `(b)` alone | Read exclusively by `BoundsContracts::for_bounds`. |
  | `quant live #B;` | Liveness `(c)` alone | Independent of `(a)` and `(b)` (dangling pointers fail liveness). Admitted on audit routes **only** as a checked restatement backed by an in-force `resource_owned` parameter in `resource_contract_v1` (`quant-live-unbacked` / `quant-live-contradicts-custody` otherwise; `resource_borrowed` is insufficient). |

#### 3. Member/result `nonnull_*` declarations, scoped invariants, and candidate triage

All non-NULL contract clauses in `bounds_contract_v1` ([G05e](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05e-null-property-member-and-result-declarations), [G05f](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05f-scoped-member-invariants)) use the strict `nonnull_` prefix (zero legacy aliases):

| Clause in `bounds_contract_v1` | Meaning, validation fold, and premise rules |
|---|---|
| `ensures normal nonnull_result();` | Every return path of defining bodies is non-NULL (`validated` when all examined bodies prove it; **withdrawn** run-wide if any body can return NULL). |
| `ensures normal nonnull_result_range(LO, HI);` | Integer return lies in `[LO, HI]`; narrows call-result bounds during path triage and rule `38e` pin derivation (`validated` or withdrawn against examined bodies). |
| `requires nonnull_member(TAG, FIELD);` | Hosted on `struct TAG`'s constructor: `FIELD` is non-NULL wherever read. Checked against all analysed stores and initializer lists (omitted field = NULL store; withdrawn run-wide if any store is not proved non-NULL; at best `partly-validated` due to `construction` premise). |
| `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];` | **Object-state scope (`QCore.NullMemberScope`):** Once `FIELD` becomes non-NULL on an existing `struct TAG`, no subsequent assignment, struct copy, or `memset`/`memcpy` sets it back to NULL (fresh initializer lists are exempt; `whole-object` untyped writes remain a premise). Allows `V->FIELD != NULL` on tracked `V` to survive calls and stores (`stable:TAG.FIELD`). Members reset to NULL on teardown/unlink (`mk_list.next`, `rkbuf_rkb`) are ineligible. |
| `requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];` | **Function-entry scope:** `P->FIELD` is non-NULL on entry (lasting the whole body if `FIELD` is `[[stable]]`, else until the first call), generating a `call-precondition` check on `ACTUAL->FIELD` at every direct caller. |
| **Plugin-premised member clauses (`QCore.NullMemberPremise`)** | Any `nonnull_member` variant may attach `requires resource_premise(NAME, ...);` (such as librdkafka's `rk_plugin_callback_closure`) when external plugin callbacks could mutate the struct outside analysed units; dependent null rows record `premise:NAME`. |

- **Path-sensitive candidate triage (`QCore.NullTriage`, `QCore.NullTriagePaths`, rules `14r`–`14w`, `15q`–`15u`, `39`):**
  - Candidates are narrowed **only by checked knowledge**: call integer results carry type/contract/summary **bounds** rather than free achievable values; exact linear relations (`x ± k`, difference/sum intervals) prune infeasible branches, while inexact conditions over-approximate both arms (never producing false `Unsafe` witnesses).
  - If exhaustive loop/label-complete exploration reaches zero NULL dereference states, the check is proved (`path-search:infeasible`, tracking every consulted declaration so withdrawal restores candidacy).
  - If an **exact** feasible entry path reaches the dereference with NULL (correlating single-read lookup helpers with path guards per rule `39`), the check becomes a **provisional `Unsafe` finding** (`null-on-allocation-failure` or `null-on-normal-path`, published with path, condition, and source excerpts, and **never presented as an upstream defect**).
  - Row verdict summary: `Safe` iff all checks are `proved`, `moved`, or `excluded` with zero unread forms and zero withdrawn dependencies; `Pending` iff held clauses are the sole obstacle; `Unsafe` iff a straight-line entry violation or validated exact-path `finding` exists; otherwise `Unknown` listing all surviving candidates.

**Implementation & Provenance.** `crates/qcore-contract/src/deref_parts.rs`, `crates/qcore-core/src/null_deref.rs`, `crates/qcore-core/src/null_triage.rs`, `crates/qcore-core/src/null_member_scope.rs`, `crates/qcore-c-gateway/src/null_deref.rs`. Lean: `QCore.NullDeref`, `QCore.NullDerefGrade`, `QCore.DerefDecomposition`, `QCore.NullTriage`, `QCore.NullTriagePaths`, `QCore.NullMemberScope`, `QCore.NullMemberQualifier`, `QCore.NullMemberPremise`. Cards: `NULL-SAFETY-PROPERTY`, `NULL-DEREF-QTT-GROUNDING`, `NULL-CANDIDATE-TRIAGE`, `NULL-WITNESS-GUARD-CORRELATION`, `NULL-MEMBER-INVARIANT-SCOPE`, `NONNULL-MEMBER-UNIFIED-QUALIFIER`, `LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER`, `NULL-TRIAGE-UNRESOLVED-PATHS`.

---

<a id="c094-declared-c-library-result-sets"></a>
### C09.4. Declared C library result sets and written sets

#### 1. Flexible libc `result_set` algebra (`QCore.ResultSet`, grammar [G05j](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05j-c-library-result-sets))

To state exact integer return domains for standard C / POSIX functions (such as `open`, `pipe`, and `timerfd_create` returning `-1 | [0, MAX]`) **without** loosening `resource_contract_v1` custody rules, `bounds_contract_v1` admits flexible C-library result sets in `quirin-provider-libc`:

```qcore
module contract;

extern fn pipe(pipedes: ptr<mut, i32>) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_revision(flexible);
    ensures normal result_set(-1 | 0);
    ensures normal written_set(pipedes[0], [1, MAX]) when #result == 0;
    ensures normal written_set(pipedes[1], [1, MAX]) when #result == 0;
};
```

- **Set operators and canonicalization:** Over the C return type's `[MIN, MAX]` integer universe, atoms `k`, `[a, b]` (`a <= b`), and `MIN`/`MAX` combine using ASCII programming-language operators at C precedence: `&` (intersection $\cap$, tightest), `^` (symmetric difference $\triangle$), and `|` (union $\cup$, loosest). Out-of-type literals, reversed bounds, or any subexpression/set denoting $\emptyset$ rejects the module (`result-set-empty`). Equivalent expressions normalize to maximal sorted `|`-joined intervals (`-1 | [0, MAX]` $\mapsto$ `[-1, MAX]`) for `canonical_sha256`, while `text_sha256` pins exact written text.
- **Libc-only flexible standing:** `result_set` requires `requires bounds_revision(flexible);` and is admitted in force **only** from `quirin-provider-libc` (held on any other crate) under standing assumption `libc-c-standard-abi`, when no in-unit C body defines the same symbol. Out-of-force sets read as the full `[MIN, MAX]` universe (`out_of_force_reads_the_universe`). `result_set` and pinned `nonnull_result_range` remain separate clauses (at most one per declaration).

#### 2. Conditional `written_set` clauses and pin-time AST derivation (`QCore.WrittenSetDerivation`, rules `38e`–`38f`)

- **`written_set(SLOT[K], SET) when #result == C`:** Pure value-set postcondition stating that when `#result == C`, the `int` stored at `SLOT[K]` (`SLOT` shorthand for `SLOT[0]`) belongs to `SET` (for example, `[1, MAX]` on `pipe` under the declared non-stdin policy, consumed by rule `38f`'s descriptor-cell `#fresh` creation check in [C04.6.2](#c0462-descriptor-cells)).
- **Pin-time derivation (`quirin-audit pin --bounds --derive-results`):**
  - For pinned audited wrappers, `pin --bounds --derive-results` analyses the defining Clang AST across **every** compile-database configuration of the source file, deriving both `nonnull_result_range` and per-return-class `written_set` sets from constants, in-unit helper derivations, and in-force `quirin-provider-libc` `result_set`/`written_set` declarations (via direct `return G(..., P, ...)` forwarding or branch-narrowed `ret = G(..., A, ...); *P = A[k]` stores; loops, jumps, or address-taken narrowed locals refuse derivation).
  - Pinning succeeds **only** if every derived set across all build configurations lies inside the declared set, recording the derived sets, helper definition sites, and consulted libc digests in `bounds-contracts.lock` (holding as `contract-changed` if any dependency drifts).

**Implementation & Provenance.** `crates/qcore-contract/src/bounds_loader.rs`, `crates/qcore-core/src/result_set.rs`, `crates/qcore-core/src/written_set.rs`, `crates/qcore-audit/`. Lean: `QCore.ResultSet`, `QCore.WrittenSetDerivation`. Cards: `CELL-GUARD-RESULT-RANGE-CHECKED`, `CELL-RESULT-SET-ALIGNMENT`, `CELL-RESULT-SET-WRITTEN-SET-DERIVATION`.

---

<a id="c10-cleanup-and-os-endpoint-contracts"></a>
## C10. Cleanup and OS endpoint contracts

### Epoch-distinct OS endpoint identities

Cleanup verification `MUST` distinguish destructor invocation, attempted release, observed return code, actual kernel/allocator release, and residual custody:

- **Generation identity over raw descriptors:** An OS endpoint is identified by `(kind, acquisition generation, lifetime)`, never by a numeric file descriptor or virtual address alone; numeric reuse after an earlier epoch closes `MUST NOT` merge distinct lifetimes.
- **Disjoint parent/child/descriptor obligations:** Freeing a parent struct neither closes embedded file descriptors nor unmaps `mmap` regions; a `NULL` constructor return may coexist with a leaked partial endpoint from a failed rollback; and a non-null mapping field may hold `MAP_FAILED` rather than a live mapping.

### Mandatory four-policy failure taxonomy

Every complete cleanup contract `MUST` select an explicit failure post-state policy and preserve all resulting obligations:

| Cleanup policy | Required failure post-state & justification boundary |
|---|---|
| **Infallible release** | Failure is impossible under explicit modeled premises that `MUST` be justified on the claim. |
| **Consumes on attempt** | Obligation is consumed regardless of return code (valid *only* where the specific OS/library API guarantees descriptor retirement on `close` error — never assumed as a universal destructor rule). |
| **Returns on failure** | Failed release retains the exact live obligation for retry, caller hand-off, or explicit residual accounting. |
| **Poisons on failure** | Transitions the obligation into an explicit poisoned/indeterminate state recording permitted recovery actions (never treated as released). |

- **Fault-injection & harness-reclamation discipline:** Syscall-interposition wrappers that suppress reaching the kernel prove controlled wrapper premises, not universal OS kernel semantics. Any cleanup performed by a test harness *after* library teardown (such as in `docs/reports/cycle19-reopen-failures`) `MUST` be recorded as harness reclamation, never credited to the audited library or used to promote raw source units.


---

# Part V — Observation Policies, Source-Bound Adapters, Plugins, and Claim Reporting

Part V specifies the runtime observation interpreter (`C07`), bounded C source-to-policy adapters (`C08`), the out-of-process verifier protocol v1 and plugin certificate firewall (`C11`), core obligation/evidence v2 integration (`C12`), canonical 9-tuple claim reporting (`C13`), and migration/acceptance invariants (`C14`).

---

<a id="c07-observation-interpreter"></a>
## C07. Observation interpreter

### Scope and expression domain (`contract_observation`)

Implemented in `crates/qcore-contract/src/contract_observation.rs` (grammar [G06](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g06-observation-predicate-grammar)), the observation interpreter evaluates a pure integer/Boolean postcondition policy for a selected `extern fn` against an explicitly supplied environment mapping fact names to signed `i128` values:

- **Admitted declaration shape:** The target name `MUST` identify a unique `extern fn` carrying a `contract { ... }` block with zero or more `requires` clauses and **at least one** `ensures normal` clause (`ensures any`, `ensures error(...)`, and `quant` clauses are rejected by this consumer).
- **Checked `i128` arithmetic and operator precedence:**
  - Atoms are decimal integer literals across the full signed `i128` range (`-2^127 ..= 2^127 - 1`, `R07`), `true`/`false` literals, fact identifiers, and parenthesized expressions.
  - Precedence from tightest to loosest (left-associative binary operators): (1) unary `-`, `!`, `not`; (2) `*`; (3) `+`, `-`; (4) `==`, `!=`, `<`, `<=`, `>`, `>=`; (5) `&&`, `and`; (6) `||`, `or`. Non-zero values are truthy; Boolean operators yield `0` or `1`; arithmetic overflow rejects.
  - Division, quantifiers, pointer dereferences, calls, nested `old(...)`, heap inspection, and effect evaluation are unsupported and rejected.

<a id="c071-eager-evaluation-and-result-categories"></a>
### C07.1. Eager evaluation and result categories

All `requires` and `ensures normal` clauses are parsed and **eagerly evaluated** (non-short-circuiting `&&` / `||`) *before* classifying precondition applicability. Consequently, an unsupported construct, missing fact identifier, or signed `i128` overflow inside any branch or postcondition fails immediately even when a precondition evaluates to `false`.

| Interpreter outcome | Normative meaning |
|---|---|
| `Checked { postconditions }` | Every `requires` clause evaluated true, and all `postconditions` ($\ge 1$) `ensures normal` clauses evaluated true for the supplied fact map. |
| `OutsidePreconditions { predicate }` | All clauses evaluated without error, and at least one `requires` clause (`predicate`) evaluated false — **neither** contract satisfaction **nor** a postcondition violation. |
| `Error(...)` | Malformed/unsupported syntax, missing fact, `i128` overflow, or a false `ensures normal` postcondition under true preconditions. |

*(Note: Legacy helper `check_contract_facts` maps `OutsidePreconditions` to an error; callers `MUST NOT` count excluded runs as positive `Checked` observations or present evaluation errors as upstream source defects.)*

```qcore
module contract;

extern fn observe(live_before: i64, live_after: i64) -> unit
contract {
    requires live_before >= 0;
    ensures normal live_after >= 0;
    ensures normal live_after == live_before;
};
```

*(Note: Values come exclusively from the supplied `i128` fact map; the `extern fn` parameter list does not bind runtime arguments or enforce narrower scalar widths on fact entries.)*

<a id="c072-fact-provenance"></a>
### C07.2. Fact provenance

Evaluating `chunk_is_live != 0` in a supplied map proves **only** that the caller passed a non-zero integer under that string key — never its English meaning. Snapshot names such as `old_busy` are explicit adapter-supplied integers, not automatic pre-state heap snapshots. Every published observation claim `MUST` distinguish model-derived facts, recorded runtime trace facts, and caller-supplied maps, and `MUST` bind the fact producer, observed artifact, policy text, preconditions, and recording scope. An observation `Checked` outcome is a **policy replay result**, never a general static source `Safe` certificate.

---

<a id="c08-source-bound-and-lifecycle-adapters"></a>
## C08. Source-bound and lifecycle adapters

Rather than claiming a monolithic C lifecycle theorem prover, Quirin provides specialized, explicitly scoped source-bound and model adapters that extract restricted C AST summaries and check them against targeted contract profiles:

- **Boolean-field and frame-helper C adapters (`qcore.c_boolean_field_contract.v2` / `.v3`, `evaluate_boolean_frame_contract`):**
  - `v2` checks integer-returning functions with mandatory explicit `return`; `v3` (`W5-CMETRICS-UNIT-PROVIDER`) extends support to single-record pointer functions returning C `void` (`-> unit` in `.qv`, forbidding scalar `result` predicates, admitting bare `return;` and `void` fallthrough, and using zero solely as internal return-slot bookkeeping). Both require explicit `object_is_live`, `exclusive_object_access`, and `field_is_boolean` caller premises and check `field` / `old_field` postconditions.
  - Configured Boolean frame helpers (`W5-CONFIGURED-FRAME-HELPERS`, [G08](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g08-source-contract-service-configuration-v1)) validate `borrow<'life, mut, Record> use one -> unit` signatures, constant `field_NAME` post-state assignments, `writes_only_declared_fields`, and `modeled_fields_are_preserved` across the **entire** modeled-field inventory of `Record` (including fields declared by sibling libraries in the same service profile), backed by opaque Clang AST import. Cross-unit C linker/offset ABI identity remains an explicit outer-audit premise.
- **ChunkIO and lifecycle source extraction boundaries (`chunk_contract.rs`):**
  - Every source-bound result `MUST` record the imported function/unit, source and build digests, C dialect/preprocessor flags, imported vs. excluded operations, callee contracts, caller premises, and summary-binding check.
  - Observing a return-value comparison does not prove an external call succeeded; observing a struct field reset (`p->fd = -1`) does not prove the underlying resource was released; and unresolved nested calls, ambiguous pointer aliases (such as a pointer selecting between two live resources, `W4-NULLABLE-REMOVAL`), conditional cleanups, or loop dependencies `MUST` remain `Unknown`.

---

<a id="c11-verifier-protocol-v1-and-the-certificate-firewall"></a>
## C11. Verifier protocol v1 and the certificate firewall

### Non-certifying protocol v1 baseline

Verifier protocol v1 (`evidence_version: 1`, `verifier_plugin.rs`) defines bounded out-of-process JSON framing, target/digest matching, and timeout/output limits, **not** an OS kernel sandbox (manifest permission/memory declarations do not enforce filesystem, network, or kernel RSS confinement on their own) and **not** a trusted proof source:

- Under bare protocol v1 (`evidence_version: 1`), `Unknown` stays `Unknown`, `Unsafe` without a supported counterexample checker is downgraded to `Unknown` (`QP010`), and any proposed `Safe` response is **rejected** because v1 carries no replayable `Safe` certificate format.

---

<a id="c111-user-defined-contracts-and-checker-plugins"></a>
### C11.1. User-defined contracts and checker plugins

Users `MAY` author external `resource_contract_v1` crates (`quirin-provider-NAME` with `NAMESPACE`, `PUBLISHED`, `contracts/`, and `contracts.lock` via SDK `providers/quirin-provider-plugin`) and register them in a **user load-path manifest** whose `extends` field names a shipped manifest (single-hop extension only; user crates may depend on shipped crates and may never shadow a shipped crate name or namespace).

- **Selection & collision override:** Audited via `quirin-audit current|contracts --load-path USER.json --provider quirin-provider-NAME` (`quirin-audit pin` on a user load path writes user locks only; `quirin-audit plugin views --write` computes `resource_view`). Any user declaration naming an `extern fn` also declared by a shipped crate is **refused** unless `USER.json` records an explicit collision override (`function`, `user_crate`, `shipped_crate`, `reason`), published in `user_load_path`.
- **Out-of-process checker plugins & the Certificate Firewall (`qcore_verifier_host::plugin_firewall`):**
  User modules `MAY` bind out-of-process checker packages (`quirin.user-contract-check.v1`, protocol v1 framing with **evidence v2** over core-derived `parameter_mode_respected` obligations). Every plugin response passes through the certificate firewall (`QCore.VerdictSemigroup`):

  | Plugin response & core verification status | Published firewall verdict |
  |---|---|
  | `Safe` + replayable witness **accepted by a registered trusted-core certificate checker** against the core's obligation inventory | `Safe` *(library route only when caller supplies a trusted checker; CLI registers none today)* |
  | `Safe` with empty evidence (`QP008`) or uncheckable evidence (`QP016`) | `Pending` (`plugin-claim-uncertified`) |
  | `Unsafe` + counterexample witness **validated by a trusted-core counterexample checker** | Provisional `Unsafe` |
  | `Unsafe` without a validated witness / checker (`QP010`) | `Pending` (`plugin-claim-uncertified`) |
  | `Unknown` | `Unknown` |
  | Gate refusal (`QP001`–`QP005`), process crash/timeout/quota/framing error (`QP006`), invalid envelope (`QP007`), unverified package, or **refuted/mismatched witness (`QP009`)** | `Unknown` |

  Bound function verdicts propagate into callers along solved CFG callee chains and unit roll-ups via the [§L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition) join, recorded in the contract sidecar's `plugin_checks` ledger.

---

<a id="c112-audit-subcommand-plugins"></a>
### C11.2. Audit subcommand plugins

Invoking `quirin-audit NAME ARGS...` for a non-built-in `NAME` executes `quirin-audit-NAME` out of process (`crates/quirin-audit-plugin`, `QCore.SubcommandPlugins`):

- **Search-path & process containment:** Resolved **exclusively** on `--plugin-path` then `QUIRIN_AUDIT_PLUGIN_PATH` (never `PATH`; built-in subcommands can never be shadowed). Runs in its own process group with a scrubbed environment (allow-list + `--plugin-env`), working directory set to the declared output directory, stdout/stderr byte quotas, and an analysis wall-time quota (excluding declared gate-slot wait marker time up to a bounded wait limit, `MEASURE-QUOTA-AND-INCREMENTAL`).
- **Protocol & firewall join:** Exchanges versioned JSON (`quirin-audit-plugin` v1) carrying audit/checkpoint/contract/refutation digests and verifies declared output artifact SHA-256 digests inside the output directory. Proposed verdict claims pass through the **exact `C11.1` firewall** (yielding `Pending` `plugin-claim-uncertified` for uncertified `Safe`/`Unsafe` since no subcommand certificate checker is registered today, and `Unknown` discarding claims on any process/quota/digest/protocol failure), joined (`§L21.2`) with host verdicts in `<report>.invocation.json` without writing `.qo` checkpoints or lowering any host verdict.

---

<a id="c113-contributor-records-in-a-composed-verdict"></a>
### C11.3. Contributor records in a composed verdict

Whenever a function, unit, checkpoint, or report row composes host and plugin verdicts (`W48-PLUGIN-VERDICT-PRESERVATION`, `qcore_audit::plugin_verdicts`, `QCore.VerdictSemigroup`), **every contributor's individual record `MUST` be preserved**:

- **Recorded fields:** Contributor identity (`host`, subcommand run, or checker plugin package ID/version/digest), the plugin's raw pre-firewall `answer` (proposed verdict, firewall classification/code, and plugin diagnostics), and its post-firewall contributed verdict and reasons (`contributions`, `host_status` + `plugin_links`, `host_verdict`, and `<report>.components.json` provenance components).
- **Join integrity & deciding-link attribution:** Every composed row's published verdict `MUST` equal the `§L21.2` join of its recorded contributor verdicts (readers `MUST` reject rows violating this equation), and when a checker plugin raises a unit above `host_verdict`, the unit's published reason `MUST` be that deciding link's typed firewall reason while retaining `host_verdict` intact. `.qo` checkpoints preserve contributor records byte-for-byte.

---

<a id="c12-obligationsevidence-integration-and-remaining-requirements"></a>
## C12. Obligations/evidence integration and remaining requirements

Core obligation derivation and witness replay (`evidence_version: 2`) obey five strict soundness invariants across all importers and verifiers:

1. **Core-owned obligation denominator:** The trusted core derives the complete obligation set and stable subject IDs from the requested artifact and property; a provider `MUST NOT` shrink the obligation set and certify that subset as the whole request, nor can an empty bundle discharge a non-empty obligation set.
2. **Audited-file & witness completeness (`W2-SOURCE-PROFILE-BOUNDARY`):** Public profile importers enforce audited-file SHA-256 digests, unique file/witness IDs, non-empty construct witnesses, and 100% required-witness satisfaction even when invoked outside the manifest loader (`observations_trusted` remains an explicit condition on native observation replay).
3. **Independent replay before `Safe` / `Unsafe`:** Every `Safe` discharge requires replay by a versioned trusted-core checker; every `Unsafe` claim requires a validated counterexample or proof of violation for the exact subject and property; malformed, oversized, stale, or mismatched witnesses stay `Unknown`.
4. **Explicit set coverage & cache completeness:** Discharges compose only across compatible semantic profiles and assumptions; summary caches `MUST` key on all semantic inputs (contracts, profiles, compile flags, source digests, transitive summaries, and checker versions).
5. **Non-certifying v1 retention:** Legacy protocol/evidence v1 paths remain permanently non-certifying; any new certifying evidence schema requires explicit version bumps, mutation tests, and documented migration.

---

<a id="c13-claim-identity-and-reporting"></a>
## C13. Claim identity and reporting

Every published safety, contract, bounds, null-dereference, or observation conclusion `MUST` be interpretable as a complete **9-tuple**:

```text
(artifact, property, semantic profile, scope, bounds,
 assumptions, coverage, evidence/checker identity, result)
```

| Result value | Normative reporting meaning (`Safe < Pending < Unknown < Unsafe`, [§L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)) |
|---|---|
| **`Safe`** | All obligations for the named `property` and `scope` are discharged within `bounds` under the explicitly listed `assumptions` and `evidence/checker identity` — never a blanket claim that the program is defect-free in unmodeled domains. |
| **`Pending`** | Undecided solely because a named, human-resolvable act remains (`contract-review-pending`, `contract-pin-mismatch`, `outer-contract`, or `plugin-claim-uncertified`) — non-`Safe`, retained in the denominator, and neither proof of safety nor evidence of a bug. |
| **`Unknown`** | Available supported analysis did not establish the property (unsupported syntax/vocabulary, missing contract, failed import, residual custody, refuted witness, or environment failure) — retained in the denominator, never omitted or promoted. |
| **`Unsafe`** | Supported, reachability-validated violation for the exact `artifact` and `property` (existing-C audit `Unsafe` rows are **provisional** analyzer findings and are never presented as upstream defects). |

---

<a id="c14-migration-and-acceptance-requirements"></a>
## C14. Migration and acceptance requirements

- **Backward semantic stability:** Valid `resource_contract_v1` modules keep their exact total-or-reject meaning (never silently absorbing lifecycle/bounds clauses, weakening `nullable` into arbitrary error semantics, or inventing hidden sentinel rules); `contract_observation` retains eager checked `i128` evaluation and `OutsidePreconditions`; protocol-v1 `Safe` rejection remains enforced.
- **Boundary testing mandate:** Every contract, loader, or consumer change `MUST` ship with positive, negative, and mutation tests at its exact boundary (whole-declaration rejection, duplicate/conflicting clauses, owner/kind mismatches, lock-pin & inclusion replay, eager overflow/missing-fact observation errors, concurrent loan/guard violations, and external failure residuals).
- **Evidence separation:** Specification prose, loader acceptance, Lean model theorems, native fault/sanitizer runs, and CT acceptance-matrix mappings are reported independently (at initial r0.3 integration on 2026-09-07, 5/50 CT cases were mapped; current live CT mapping and gate measurements are maintained in [`STATUS.md`](STATUS.md) and [`VALIDATION.md`](VALIDATION.md)).
