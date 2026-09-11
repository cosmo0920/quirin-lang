# Quirin language specification — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the `.qcore` extension and this file's name —
> and every such identifier in this document is unchanged by the branding.

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

> **Normative amendment (2026-09-11).** Generated programs are quiet unless
> source code explicitly performs I/O; stream observations compose pointwise;
> and `std.logger` provides `trace`, `debug`, `info`, `warn`, `error`, and
> `fatal`. The complete requirements and migration gates are in
> [Quiet C11 Emission, Composable Streams, and Logger Levels](design/QUIET_EMISSION_STREAMS_AMENDMENT.md).

**Revision date:** 2026-09-07, Asia/Tokyo.  
**Status:** Source-reviewed specification and implementation requirements. This is not a declaration of full compiler conformance.  
**Reviewed input:** `qcore-c.tar.gz`, SHA-256 `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.

## L01. Authority, scope, and reading order

This revision separates the QCore language, its implemented source profiles, its independently callable semantic models, and the contract interpreters used to analyze C. It replaces the interpretation of revision 0.2 as one uniformly implemented language. It does not rename the compiler package version or change any serialized protocol version.

The companion [contract specification](QCORE_CONTRACT_SPEC_r0_3.md) defines external custody contracts, observation policies, source-bound contracts, and verifier evidence. The [implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) specify the deliberately narrow input languages of the current consumers. The [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf) preserves the extended language target; acceptance by the structural parser does not establish conformance to that BNF or to its semantics.

This document and the companion contract specification take precedence over conflicting descriptions in `obsolete/QCORE_SPEC_r0_2.md`, `obsolete/qcore_quantitative_types_spec.md`, `obsolete/concurrency_revision.md`, and older cycle reports. Requirements in the earlier design that are not changed here remain design requirements, not evidence of implementation. Historical reports remain records of their own artifacts and execution dates.

`MUST`, `MUST NOT`, `SHOULD`, and `MAY` express requirements on an implementation claiming the relevant profile. A requirement can be unmet by the reviewed source; such a requirement is not a statement that the compiler already enforces it. The [review and acceptance ledger](reviews/r0_3/REVIEW.md) identifies observed boundaries and outstanding defects.

Original line-number references identify the reviewed archive and may have shifted. `reviews/r0_3/source_inventory.json` binds the supplied review files and the integration source anchors; the acceptance ledger links current tests and evidence. The archive itself and its advertised probe package were not supplied in this checkout.

The configured source-contract audit service now selects explicit packages and
snapshotted policies using sealed import identities ([configuration grammar](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g08-source-contract-service-configuration-v1)).
Selection supplies no evidence authority: the existing bounded Boolean-field
task and trusted witness replay remain mandatory. This orchestration increment
adds no language syntax, semantic profile or general lifecycle coverage.

## L02. Evidence vocabulary and profile selection

Every published capability claim MUST identify both a semantic profile and an evidence level.

| Evidence level | Meaning | What it does not establish |
|---|---|---|
| Structural | Tokens, delimiters, declarations, clause kinds, or names were accepted and retained. | Complete ordinary typing, contract truth, resource safety, or executable support. |
| Source-checked | A named source subset was translated into specified semantic checks. | Support for source forms outside that subset. |
| Model-checked | A checker accepted an explicitly supplied IR, event trace, state graph, or execution candidate. | A faithful extraction of that model from arbitrary source. |
| Source-bound | A model or summary is bound to identified source/build inputs and its stated import coverage. | Coverage of omitted callers, translation units, schedules, or configurations. |
| Observed | Predicates were evaluated on identified recorded executions. | Universal behavior, unobserved failure modes, or whole-program safety. |
| Mechanized | A theorem about an identified formal model has machine-checked evidence. | Rust/Lean correspondence or C refinement unless those are separately proved. |
| Proposed | The syntax, schema, rule, or integration is specified but not implemented in this snapshot. | Any completed acceptance gate. |

These levels are not a single ordering. An observed native execution and a mechanized abstract theorem answer different questions. Combining them requires an explicit relation between their subjects.

For documentation, this revision names the current boundaries as follows. These names are specification labels, not newly implemented CLI switches or wire fields.

| Profile label | Entry point or implementation | Snapshot boundary |
|---|---|---|
| `source.structural` | `parse_source_program`, `elaborate_source` | Broad structural syntax; selected name, usage-grade, and quantitative-index checks. |
| `source.exec12` | `compile_files`, `quirin check`, `quirin emit` | Flat artificial resource operations, literal returns, and restricted direct calls. |
| `source.tasks.copy.v1` | `compile_files`, `quirin check`, `quirin emit` | Linux C11 threads or the emitted macOS/Windows compatibility subset; finite scalar-copy leaf workers, explicit joins, checked creation/error paths; no resources, loans or shared source memory. |
| `source.extern-call.v1` | `compile_files` with an explicit `CompileOptions` extern contract selection | A flat sequential entry calling declared extern functions under admitted resource-contract v1 shapes; results are valid only relative to those declarations. No extents, no excluded C04 mode, no concurrent or error-region context. |
| `source.nonexec` | The same driver with library/contract modes | Entry-free checking of declarations and supported bodies; no standalone C artifact. |
| `index.closed` | `quant_index` | Closed, dimensioned natural expressions in credit/live types. |
| `model.sequential` | Ownership, usage, flow, typed-error, error-region and CFG APIs | Supplied semantic IR; not a general source elaborator. |
| `model.concurrent14` | `check_concurrent` | Explicit event model with the limitations in L16 and review findings R01–R03. |
| `model.c11-candidate` | `check_c11_execution` | An explicitly supplied weak-memory execution candidate. |
| `c.resource` | `analyze_unrestricted_c11` | Clang-gated conservative custody projection under trusted resource contracts. |
| `contract.resource.v1` | `load_resource_contract_*` | Total-or-reject external custody declarations. |
| `contract.observation` | `check_contract_observation` | Integer/Boolean predicates over a supplied fact environment. |
| `verifier.v1` | Registry and certificate firewall | Configuration and proposal transport; no accepted v1 Safe evidence. |
| `verifier.v2.observed` | Core inventory, dispatch, independent observation replay | Conditional recorded-run agreement under explicit trust; lifecycle source derivation remains incomplete. |

No consumer MAY infer a stronger profile from a file extension, successful parsing, the presence of a theorem name, or a historical cycle-completion label.

## L03. Semantic domains and judgments

QCore keeps the following domains separate.

**Usage demand** describes how often a binding is demanded. It does not identify an allocation or authorize a read. **Indices** are pure, dimensioned static values. **Credit** is spendable capacity. **Live custody** records resources for which some custodian remains responsible. **Cost** is a bound on work or occupancy, not a token. **Effects** describe permitted operations and typed failure possibilities. **Access authority** concerns identified memory or protocol footprints. **Exit states** preserve the domain values and ownership relations that remain on each exit.

A general design judgment has the shape

```text
Σ ; Γ ; U ; C ; L ; A ; K ⊢ e : T ! E ▷ X
```

Here `Σ` contains static declarations and trusted contracts; `Γ` contains runtime types and bindings; `U` is demand information; `C` is credit; `L` is the identity-indexed live-custody ledger; `A` is access authority; `K` is cleanup/task/mutex protocol state; `E` is the effect row; and `X` is an exit-indexed family of post-states. This notation specifies responsibilities, not an assertion that one current Rust data structure contains every component.

An implementation MUST NOT discharge one domain by changing another. Spending credit does not release an allocation. Consuming a value once does not prove exclusive access. Returning an error does not erase cleanup duties. An increased cost allowance does not create a resource. A pointer value or descriptor number is not an ownership witness.

## L04. Lexical and module rules

The current lexer recognizes ASCII identifiers beginning with a letter or underscore, decimal integer literals, decimal fractional literals, strings, character literals, and its enumerated punctuation. It skips whitespace, `//` comments, and non-nested `/* ... */` comments. Byte spans are half-open UTF-8 byte ranges. Recognition of a float or string does not imply that `source.exec12` evaluates it. See `crates/qcore-frontend/src/lexer.rs:1–282`.

### L04.1. The string-view amendment

String capabilities are provided by the opt-in bundled module `std.string`.
Each source file MUST declare `import std.string;` before its string type,
literal values, length queries, or slicing may be admitted. The import applies
to the whole file; it grants no buffer, output, formatting, or concurrency
capability and cannot be supplied by another file's imports. Quoted tokens in
existing declaration metadata remain structural and need no string import.
These string capabilities MUST NOT be implicitly available as built-ins.

The revision-0.3 design adopts immutable UTF-8 string views with semantic
representation `{ ptr, len }`: literals denote static bytes, `len(s)` is O(1),
and `s[low:high]` is a checked, allocation-free view derivation. Indices count
bytes and must fall on UTF-8 code-point boundaries. A view carries no ownership
by itself and cannot outlive or permit mutation of its backing storage. It is
not null-terminated, and no implementation may use `strlen` to recover its
length. The `{ ptr, len, cap }` shape is reserved for a future mutable byte
slice, whose append capacity has no semantic place in an immutable string.

This requirement is **implemented** by card `STRING-SLICE` (2026-09-11) as the
checked source profile `source.string.v1`, which owns `Q068`–`Q073` in §L21
below. `std.string` is registered as the ninth bundled module, the import is
gated per file exactly as `std.concurrency` and `std.buffer` are, a quoted token
in a function body is decoded to a well-formed UTF-8 payload, and `len(s)` and
`s[low:high]` are checked and lowered to the aggregate above. Every §2 admission
condition is decided **statically** — the first of the two arms the amendment
permits — because every bound in the profile is a check-time constant; the
profile therefore has no run-time failure arm, and a bound it cannot decide is
refused rather than deferred. The amendment reserves **no word**: `len` is
contextual (skipped only before `(`) because the pinned contract corpus
references a parameter of that name, and `string` is admitted only in a `let`
type ascription.

What it still does **not** do, each a named follow-on rather than an omission:
it does not convert `std.buffer` into text (`STRING-SLICE-BUFFER-BORROW`), does
not permit a string across a C04 or native-library boundary
(`STRING-SLICE-ABI`) or across an output-module boundary (`STRING-SLICE-IO`),
and supplies no equality, ordering, hashing or search. It is one bounded source
profile and is not revision-0.3 source-language conformance. The complete
amendment, including storage, slicing, UTF-8, ownership, C11 lowering, ABI
refusal, and evidence requirements, is
[STRING_SLICE_V1](design/STRING_SLICE_V1.md); the implementation evidence is
`docs/reports/string-slice/README.md`.

A module header is optional, unique, and precedes declarations:

```qcore
module executable;
module library;
module contract;
```

The three lines above are alternatives, not a valid three-header file. An absent header means executable mode. Contract and library modules do not select or synthesize a process entry. A function named `main` in those modes is an ordinary definition. Current non-executable modes do not produce C library objects or establish a public ABI.

Multiple files share one namespace. Executable mode wins if any file is executable; otherwise library wins if any file is a library; otherwise the compilation is a contract compilation. Combining files does not resolve user-file imports or introduce a namespace hierarchy; bundled capability imports remain file-local. Duplicate declarations remain errors. `--entry` without any executable input is rejected with `Q403`.

In executable mode the driver honors an explicitly selected entry, otherwise selects `main` or its documented single-function fallback. The emitted entry MUST have no parameters. Unsupported or ambiguous entry selection is an error. All supported source function bodies, including uncalled definitions, are submitted to the source checks.

The installed command is `quirin`; its owning package keeps the `qcorec` name, so from a checkout the package spelling selects it:

```sh
cargo run -p qcorec -- check path/to/module.qcore
cargo run -p qcorec -- emit -o output.c path/to/program.qcore
cargo run -p qcorec -- plugins list
```

The historical `cargo run -p qcore-compiler --bin qcorec` spelling is obsolete after the CLI split.

Bundled semantic modules require an explicit top-level `import` in each source
file. The current registry contains exactly seven names — `std.buffer`,
`std.concurrency`, `std.extern`, `std.extern.unsafe`, `std.fmt`, `std.io` and
`std.logger`:

```qcore
import std.concurrency;
concurrency checked;
```

```qcore
import std.fmt;
import std.logger;
```

Each module gates its own capability and no other. `std.fmt` names how a scalar
is rendered and opens no stream; `std.io` writes a rendered line to standard
output; `std.logger` writes a level-tagged rendered line to standard error and
provides the `flush` sink-handoff barrier. Holding one of the three never grants
another, and `std.concurrency` grants none of them: a threaded module has no
free output channel. `flush` names no rendering, so a file that logs only a
barrier needs no `std.fmt`.

The logger levels, in increasing order, are `trace`, `debug`, `info`, `warn`,
`error`, and `fatal`; their canonical tags are the corresponding uppercase
words in brackets. `error` is contextual after `log` and remains the error-type
declaration keyword elsewhere. Source-authored logger events are not filtered.
Compiler observations use the separate, default-null diagnostic logger defined
by the Quiet C11 Emission amendment and are not program output by default. An
entry may explicitly select `log_level LEVEL;` after importing `std.logger`;
maintained hosted profiles then render admitted system events at or above that
threshold on stdout. The declaration carries `Io`, is recorded and rechecked,
occurs at most once per entry, and changes no custody or control-flow judgment.
Source-authored `log` records remain unfiltered on stderr.

`std.buffer` (card `STD-BUFFER`) grants an **owned, extent-carrying byte
buffer** — `let b = bytes(8);`, `b[0] = 65;`, `let x = b[0];`,
`let n = extent(b);`, `drop b;` — and nothing else. It opens no stream, so a
module that both buffers and prints names it *and* the output modules; and
`std.io` never grants it, `std.concurrency` never grants it, and neither extern
module grants it. **A byte buffer is not a string**: the module carries no text,
no encoding and no literal, because none exists in the language (follow-ons
`STD-BUFFER-STRING-LITERAL` and `STD-BUFFER-RENDER`). The acquisition is spelled
`bytes` rather than `buffer` because `buffer` was measured to occur in binding
position in shipped fixtures, so the syntax was renamed rather than the pinned
surface edited — the `work` → `effort` precedent; the
*module* keeps the name `std.buffer`, a module name being a dotted import path
and never a reserved word. The release is the **existing** reserved `drop`,
because the operation is the ownership IR's own drop.

Import paths are dot-separated identifiers terminated by a semicolon. Imports
may occur between top-level declarations, after the optional module header, and
apply to the whole file. Repeated imports are idempotent. Unknown module names
reject with `Q014`; concurrency forms without their import reject with `Q015`,
including in uncalled bodies and non-executable modules. `Q015` is reused,
deliberately and without a new code, for the three `STD-MODULES` capabilities:
`print` without `std.io`, `log` or `flush` without `std.logger`, and a named
rendering without `std.fmt` each reject with `Q015` naming the missing module,
and likewise, without a new code, for card `STD-BUFFER`'s capability: acquiring,
indexing or measuring an owned byte buffer without `std.buffer` rejects with
`Q015`. `drop NAME;` is deliberately **not** gated — it is an established
statement with an established meaning, and gating it would make every existing
module that drops a resource depend on a module it never needed.
Each input file is checked before combining declarations: another file's import
cannot grant it concurrency or any of those three. Importing vocabulary does not
discharge operation checks or make an
unsupported profile executable. The task profile still requires its
`concurrency checked;` marker. User-file/package resolution, import aliases,
wildcards, reexports and implicit/transitive imports are not implemented.

## L05. Structural declarations versus complete typing

The structural AST recognizes dimensions, quantitative axes, type aliases, resource types, errors, constants, extern functions, functions, the concurrency marker, and verifier scopes. It retains function parameters, source spans, token sequences, selected type categories, quantitative-index types, and structured contract-clause envelopes.

Recognizing a type category such as `Owned`, `Borrow`, `Result`, `Task`, or `Mutex` is not complete type elaboration. Current typed declarations preserve the exact result annotation and span. The sequential executable driver admits explicit i32/unit scalar results, checks return shape, and rejects unsupported result forms. The separate ScopedOwnedTasksV1 profile also checks original Buffer identities in owned task results. Its public result profile distinguishes that scalar slot from a separate custody identity ledger; full owned-result and per-exit type elaboration remain open (R04). Axis declarations are now validated unconditionally, including unused declarations (R05).

A conforming source checker MUST validate every declared type and every axis declaration even if no function uses it. It MUST preserve and check parameter and result types, effect annotations, and every contract obligation that it claims to discharge. Unknown type names, invalid dimension references, unsupported semantic annotations, and ignored result-type mismatches MUST NOT receive a claim of complete type correctness.

Structural-only tools MAY retain expressions or declarations they cannot semantically interpret. Their result MUST distinguish retention from validation. A verification mode that promises a particular property MUST reject the request or return Unknown when a relevant construct is only structurally retained.

## L06. Static dimensions and the closed index profile

A dimension is nominal. `Bytes` and `Steps` are distinct even when their values happen to be equal. `Count<T>` is a nominal count dimension and does not itself prove that `T` is a live object or an allocation kind.

```qcore
dimension Bytes : Idx + Ordered;
quant axis memory_budget : credit<Bytes>;
quant axis open_files : live<Count<File>>;
quant axis allocations : cost.total<Count<Allocation>>;
quant axis maximum_bytes : cost.peak<Bytes>;
```

Axis declaration syntax supports all four modes. The implemented closed quantitative *type* resolver supports only `credit` and `live`; cost types are not enabled merely because a cost axis can be declared.

`Idx + Ordered` means that the relevant index operations are pure and dimension-respecting. It is not a runtime trait implementation supplied by arbitrary user code. No mutable field read, pointer dereference, allocator query, or ordinary function call may enter the closed index evaluator.

The `index.closed` expression vocabulary is decimal natural literals, local defined names, addition, one literal scaling factor per product, parentheses, and `min`, `max`, and checked `monus`. Comparisons are `==`, `!=`, `<`, `<=`, `>`, and `>=`. Clause separators are commas or `and`.

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;
extern fn reserve(q: quant<n where n := 5> use one)
    -> quant<credit, { chunks = total }
             where total := base + 2, base := 3, total <= 5>;
```

This is an index-elaboration example. It does not prove that `reserve` preserves or manufactures credit.

`quant[| ... |]` is an equivalent delimiter spelling of `quant< ... >` with the identical interior grammar and semantics; the opener fixes the required closer, so mixed pairs are rejected. The alias exists for readability where `<`/`>` would sit beside comparison operators; it introduces no new type.

Each `where n := e` is a type-local definition. Definitions can refer to later definitions in the same clause, but cycles, duplicate definitions, shadowing of forbidden visible names, unbound/runtime names, and incompatible dimensions are rejected. A binding must acquire a dimension from its use; a standalone unused binding with no dimension is not silently accepted as dimensionless. Type-local names do not become runtime locals or global constants.

Scalar `quant<e ...>` shorthand selects the sole credit axis in the environment. Zero or multiple credit axes make that shorthand ambiguous. Vector fields MUST be unique and have the requested axis mode. Omitted vector axes are not implicitly declared resources.

Arithmetic is checked in `u128`. `monus(a,b)` requires `b <= a` and returns `a-b`; it does not silently truncate a negative result to zero. Nonlinear multiplication is rejected, even when a human could evaluate both names to constants. Runtime-to-index reflection, general symbolic constraints, dependent function substitution, pointer extents, and access-quant types are outside this implemented resolver.

The implementation also bounds expression nesting, additive chains, dependency depth, and evaluation recursion. A limit failure is a rejection, not permission to approximate a term as zero. See `quant_index.rs:130–308,315–580` for the exact operational limits.

## L07. Usage grades

A usage grade is a closed interval `[lower, upper]` over nonnegative demands, with an optional infinite upper endpoint. The implemented finite endpoints are `u64`.

| Source grade | Meaning |
|---|---|
| `zero` | `[0,0]` |
| `one` | `[1,1]` |
| `affine` | `[0,1]` |
| `relevant` | `[1,inf]` |
| `many` | `[0,inf]` |
| A natural literal `n` | `[n,n]` |
| `[a,b]` or `[a,inf]` | The specified interval, with `a <= b` for finite bounds. |

Sequential demands add. Demand substitution multiplies intervals. A mutually exclusive branch join is the least enclosing interval, not the sum of the two branches. A declared grade admits an actual demand only when it contains the actual interval. Arithmetic overflow rejects the operation. Zero multiplication is zero even against an infinite upper bound.

The usage checker MUST validate a lambda's binder demand before scaling its free-variable captures by an enclosing usage. Zero outer use does not make an invalid lambda binder contract valid. This rule is implemented at the usage IR boundary; source lambdas and general closure emission are not part of `source.exec12`.

Unannotated source parameters use `many` in the current driver. Supported explicit resource uses and zero-argument function uses contribute demand. Locals receive distinct internal usage names so that shadowing cannot satisfy a parameter's obligation. Known failing synthetic operations remove later runtime demands; call continuations conservatively admit an early exit. Local source `use` annotations are rejected in the executable subset rather than ignored.

A `one` annotation constrains demand, not object lifetime. Resource destruction, implicit error cleanup, aliasing, and cross-thread authority remain separate checks. See `usage.rs`, `QTT_ALIGNMENT.md`, and `driver.rs:354–381,392–675`.

## L08. Credit, live obligations, cost, and access quantities

`quant<credit,q>` represents capacity that may be split, transferred, and spent according to its contract. Splitting MUST conserve each dimension. A profile may permit explicit abandonment of unused credit, but abandonment cannot discharge live custody.

`quant<live,q>` summarizes outstanding duties; it is not a substitute for an identity ledger. Equal totals do not permit replacing the duty for one allocation with a different allocation. Live duties MUST be released, rolled back, or transferred to an identified custodian.

A cost quantity is an upper bound, not transferable authority. `cost.total` accumulates work; `cost.peak` bounds occupancy or another specified peak measure. A parallel work bound MUST NOT be described as elapsed time or latency without a separate scheduling model.

Access shares authorize operations on a particular identity and footprint. They are neither demand counts nor credit balances. The design uses `FULL = 1024`; `model.concurrent14` implements that constant as `FULL_ACCESS`. A write to ordinary non-atomic memory requires exclusive authority under the relevant access model. Multiple reads may use conserved positive shares. Atomic operations require their own access and lifetime protocol; the fact that two accesses are atomic does not prove publication, reclamation, or resource-lifetime safety.

The current closed quant resolver is not a credit-flow checker for arbitrary source bodies. The usage checker, flow checker, identity ledger, and concurrency checker have distinct APIs and representations. An implementation MUST make conversions between their numeric ranges explicit and checked.

## L09. Ownership, identity, and borrowing

The extended language design distinguishes raw pointers, owned resources, and borrows. A raw pointer conveys a representation, not custody. Moving an owned resource transfers its unique responsibility and invalidates the old owning binding. Dropping requires the relevant destruction authority and no incompatible active borrow. Shared and mutable borrows constrain operations without transferring destruction responsibility.

An identity in the general design includes a lifetime generation. Reusing a numeric address, file descriptor, slot, or source variable does not revive the old identity. A bounded model with plain integer resource IDs MUST treat each ID as one unique lifetime, or import a separately verified generation mapping. It MUST NOT interpret address equality as generation equality.

Resource footprints may include memory, handles, protocol cells, or nested resource identities. Aggregate custody requires explicit parent-child relations. Releasing a parent object does not, by itself, discharge its child allocations, OS endpoints, or pending callbacks. Transfer and destruction of an aggregate may cascade only under a contract that accounts for those children.

The ownership IR includes checks for acquire, move, borrow, borrow completion, drop, and transfer operations. Its accepted `ResourceId` and `Footprint` values are supplied model inputs. Complete source-level borrow inference, arbitrary pointer alias analysis, C layout derivation, and Send/Sync derivation do not follow from that API.

## L10. Resource-flow algebra and loop bounds

A flow is `(Δ, R, P, T)`: exact signed net live change `Δ`, minimum entry quantity `R`, peak increase relative to entry `P`, and cumulative cost `T`. Quantities are componentwise. Let `[x]+ = max(0,x)` componentwise.

```text
Acquire(q,c) = ( q, 0, q, c )
Release(q,c) = (-q, q, 0, c )
F1 ; F2     = ( Δ1 + Δ2,
                max(R1, [R2 - Δ1]+),
                max(P1, [Δ1 + P2]+),
                T1 + T2 )
```

A branch join with equal exact net change takes the componentwise maximum of requirements, peaks, and totals. A branch join with different net changes MUST retain separate post-states or reject; it MUST NOT invent an optional live object by averaging counts. Numeric coverage also does not establish identity coverage.

The current implementation uses `u64` quantities and `i128` signed net changes and rejects checked arithmetic overflow. `Flow::repeat_exact(n)` computes exactly `n` compositions; `bounded_loop(n)` retains that exact meaning as a compatibility spelling. For a zero-net body, the existing bounded zero-net abstraction can conservatively retain the body's peak and requirement and scale total cost by a maximum count.

**Exact iteration evidence and upper-bound evidence are different contracts.** For nonzero net change, `F^n` is not the exit state of a loop that executes anywhere from zero to `n` times. The current `summarize_general_loop` distinguishes `Exact(n)` and `AtMost(n)`. Historical `Terminating(n)` and `Widened(n)` remain maximum-count evidence. Positive maxima with nonzero net reject; maximum zero yields zero flow. Exact counts and their source/control-flow justification remain supplied premises at this model API; numeric checking alone does not establish allocation identities. R06 is repaired at this bounded model boundary.

The revised requirement is:

* Exact-count evidence may produce `F^n` after the count and relevant control-flow assumptions are checked.
* At-most-count evidence for a nonzero-net body MUST preserve the possible net/identity exits or return Unknown/reject that summary. It MUST NOT turn a maximum allocation count into exact live custody.
* Cost and peak bounds may be widened conservatively over all admitted counts, but this widening MUST NOT erase the distinction between possible live-resource states.

A recursion summary similarly requires a checked fixed-point condition under explicit assumptions. The current source driver synthesizes restricted recursive summaries and refuses recursive C emission; this is not general user-written recursive-contract support. Termination claims are separate from partial-correctness resource claims and must identify their evidence.

## L11. Effects and exit maps

An effect row identifies permitted operations, including typed errors. A call's normal and error exits each carry their own ledger transition and effects. The absence of an error branch is a semantic claim that requires support, not a convenience for analysis.

The general exit family includes normal continuation, return, typed error, and the control exits admitted by a specific profile. The typed-error IR uses explicit normal/error maps; more advanced cancellation and cleanup outcomes live in separate models. Their existence does not mean all are elaborated from general source syntax.

A merge may join quantitative upper bounds but MUST preserve incompatible ownership alternatives. An error variant's resource payload remains owned. Matching an error, logging it, or replacing its tag does not release its payload.

A C function returning a negative integer, a null pointer, or an error sentinel has returned normally in the C calling convention unless an explicit adapter maps that return into a QCore typed error. The external contract consumer, not the integer sign alone, determines the appropriate relation.

`source.exec12` synthesizes an allocation effect allowance and artificial typed-failure operations. It does not derive complete user-declared effect rows and result types from source contracts. A consumer MUST NOT report those retained annotations as proved.

## L12. Error regions and checked quantitative opening

An error region records its entry ledger and an ordered set of cleanup responsibilities for resources acquired within the region. On an error edge, the region seals the path-specific cleanup pack. The pack identifies the originating region, error, baseline, pending cleanup actions, and any strict credit remainder represented by the profile.

Opening a pack creates a linear cleanup capability; it does not discard the pack's contents. `quant open` is valid only in the matching catch/region context. It MUST NOT select another region's pack, open a pack twice, or release an outer resource absent from the region's authority.

Discharge runs the authorized cleanup in its specified order and consumes the capability exactly once. Before the handler exits, every locally retained obligation MUST be discharged or explicitly transferred under the handler's exit contract. Active borrows and outstanding child-task authority must be resolved before destructive cleanup. A handler that cannot establish these conditions MUST NOT receive a resource-safe exit certificate.

Different failure paths may contain different cleanup packs. Their quantities MUST NOT be merged in a way that loses which resources actually exist. An error payload is separate from the region's cleanup pack unless an explicit transfer relation connects them.

The current error-region model supports path-specific packs, non-throwing destruction/rollback actions, payload movement, scalar strict-credit spending, opening, and discharge. Its `RegionOp` sequence is supplied as IR. General source error-region lowering is not part of `source.exec12`; its driver rejects those statements. See `error_region.rs:1–156` and the corresponding checking routines.

## L13. Fallible cleanup and cancellation

Every fallible release interface MUST state what happens to custody on success and on each relevant failure class. The design distinguishes at least these policies:

| Policy | Custody meaning |
|---|---|
| Infallible | The supported precondition excludes a cleanup failure; successful destruction consumes custody. |
| Consumes on attempt | Once the cleanup attempt occurs, the caller's custody is consumed even if an error is reported. |
| Returns on failure | Failed cleanup preserves or returns an identified obligation to the caller. |
| Poisons on failure | Failure produces an explicit poisoned/residual duty whose permitted operations are separately specified. |
| Indeterminate external outcome | The profile cannot establish whether the endpoint was released; it preserves an unresolved obligation and cannot claim complete cleanup. |

These are semantic policy categories, not new keywords supported by the current `.qcore` loader.

The existing advanced-control model removes a resource from its local live sequence when attempting its destructor and records destructor errors. It should therefore be read as an attempt-consuming abstract model, not as proof that arbitrary failing OS releases consume their resources. A returns-on-failure or poisoned cleanup policy requires additional state and validation.

A primary error or cancellation remains distinguishable from later cleanup errors. The advanced model records secondary failures rather than overwriting the original cause. Cleanup order is LIFO unless a different checked dependency order is specified. Cancellation is delivered at explicit safe points and at the modeled return boundary, not asynchronously at arbitrary instructions.

One-shot handlers may capture owned resources under their linear invocation contract. Resumable and multi-shot handlers cannot copy such ownership in the restricted model. This is an IR capability boundary, not source closure support.

## L14. The current executable source profile

The implemented source-to-C route is intentionally much smaller than the target language. Supported body operations are the flat forms below, subject to the semantic checks:

```qcore
module executable;
error Failure { Failed; }
fn main() -> i32 {
    let buffer = qcore_acquire(1, true, Failure);
    qcore_check(true, Failure);
    drop buffer;
    return 0;
}
```

`qcore_acquire` takes a literal resource ID, a literal Boolean success flag, and an error-type name. `qcore_check` takes a literal Boolean and an error-type name. These are controlled model operations; they are not declarations of a production allocator's complete behavior. The emitter realizes the model with generated C instrumentation and a tagged outcome.

`drop name;` destroys the corresponding modeled resource. `move name;` transfers it to the modeled result payload; it is not general assignment or arbitrary C pointer movement. A scalar i32 result requires an in-range signed integer literal; unit requires bare `return;`, represented as zero in the reduced ABI. Other scalar/result forms reject. `CompiledModule.result_profile` records the per-function scalar result and the separate custody payload model. Direct calls have zero arguments, name a source function, and discard the callee's result.

Emission additionally requires reachable callees to be observationally pure under the reduced model: no resource trace and a normal outcome with empty payload. Recursive calls and effectful callees are rejected for C emission. This check makes omission of a pure discarded-result call observationally acceptable only for this reduced observation model; it is not a general interprocedural backend.

Source `if`, `match`, loops, arbitrary assignment, general arithmetic evaluation, nonliteral returns, error-region bodies, source borrowing, source concurrency, and arbitrary extern calls are not executable through the `source.exec12` path. Structural recognition of those forms MUST NOT be reported as implemented lowering.

A *bounded* extern-call boundary is executable through the separate `source.extern-call.v1` path described later in this section. It does not widen `source.exec12`, which continues to refuse every call whose target is not a source function.

The result-literal parser now accepts the full signed i32 range, including `-2147483648`, by checking the signed result using a wider representation. Out-of-range literals reject. This R07 repair does not implement general result expressions or full owned-result typing.

The separate `source.tasks.copy.v1` path now lowers marked scoped-task source
through an opaque checked task IR and emits real threads on Linux, macOS and
Windows. Linux uses C11 threads directly; macOS uses an emitted pthread adapter
and Windows uses an emitted winpthreads-derived implementation of the required
C11 thread subset. It supports
immutable i32 copies, leaf workers with i32/unit results, dynamic creation
failure, explicit creator join, collected Results and separate `?` propagation.
The source execution profile and checked-artifact variant are explicit. Native
join failure aborts before environment reclamation. General capture authority,
shared memory, mutexes and owned payloads remain open. Explicit two-arm Result
handlers now handle spawn failure or collected worker errors, preserving any
prior sibling until its explicit join. Arm-local bindings cannot escape.
Infallible `?` consumes a checked empty-error Result without adding an
impossible error exit; a nonempty error row retains both alternatives. See [implemented grammar G02a](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02a-scoped-copy-task-execution)
and the [recovery evidence](reports/w6-task-recovery/README.md).

The additional `ScopedOwnedTasksV1` profile accepts affine `own<Buffer>` worker
parameters/results. A Buffer is an actual heap allocation containing one immutable
i32. Root and leaf-worker allocation may fail with AllocError. Successful move consumes its old
binding; failed creation restores the original bindings and prior sibling state.
A checked worker exit returns one original input or one fresh local, dropping
every other owned input and local allocation; scalar/unit/payload-free error
exits drop all of them. The parent must join and consume the Result
before using returned custody. An owned entry result has no external custodian
and rejects. These rules do not admit arbitrary resource types or owned errors.
See [G02b](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02b-owned-buffer-task-execution)
and [the bounded evidence](reports/w6-owned-source-tasks/README.md). Worker-local
generations use checked 32-bit task identities and nonzero 32-bit per-task
successful-allocation ordinals, preserving identity across native scheduling
orders. Explicit cleanup is required on every supported task error path; general
implicit worker cleanup and nested workers remain open. See the
[worker-allocation evidence](reports/w6-worker-allocation/README.md).

The separate `source.extern-call.v1` path makes a **call to a declared extern
function** executable at one bounded boundary. A file opts in with
`import std.extern;` (`Q022` otherwise, `Q014` for an unknown module name), and
the callee's contract MUST come from an explicit `CompileOptions` selection
loaded by the resource-contract v1 loader: there is no default environment, no
filesystem search and no fallback, and an inline source `contract { … }` on a
called extern declaration is refused rather than enforced. Of the C04 vocabulary
this path calls only scalars by value, `resource_borrowed` on an admitted
pointer, `resource_consumed(p, nonnull)`, `resource_return_none()` with a `unit`
or scalar result, `resource_return_owned(nullable)` whose null arm MUST be
branched at the call site into the existing typed-error exit,
`resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` and
`resource_stored(p, owner, nonnull)` on `ptr<mut, u8>`. An out slot is the
second admitted acquisition source and carries the same mandatory adapter: the
binding it establishes materializes only on the success arm. A store removes the
passed identity from the caller's ledger permanently, with no owned return in
the same call; that the declared owner keeps and eventually releases it is a
named premise on the certificate, discharged nowhere. Because contract spec
§C04.2 requires a stored parameter's owner to be a **distinct `Owned`
parameter**, `resource_owned` is admitted in exactly that position — nonnull,
on `ptr<mut, u8>`, as a loan the caller keeps — and refused everywhere else. A
declaration establishes at most one fresh identity: two out slots, an out slot
beside an owned return, and a store beside either are each refused by name, as
are `resource_released`, `resource_removed`, `resource_return_reallocated`, a
nonnull owned return, a nonnull out slot, a nullable stored parameter and a
nullable consumed parameter. Length and extent are not transported: C04
pointers are extentless `u8` projections, so no declaration on this path says
how many bytes a callee touches, and every certificate records that premise
along with each callee's declared contract and the builder's linkage premise.
The derived effect row is `{Io}`, plus `Alloc` exactly when the call establishes
fresh custody through either acquisition source and `Mut` exactly when some
parameter is a `mut` pointer; the caller's declared `effects { … }` row MUST
admit it. A call whose declared result is a **custody-free
scalar** and which establishes fresh custody through none of the acquisition
sources MAY bind that result to a local, `let NAME = CALLEE(ARGS);`, and a bound
scalar MAY then be passed at a declared scalar parameter (card
`PIPE-SCALAR-RESULT`). Scalars and identities are separate namespaces: a bound
scalar is admitted at a declared data parameter and nowhere else, a resource
binding is refused at a data parameter, a scalar binding removes nothing from
and adds nothing to the ledger — so an unread one is not a leak — and a `unit`
result, every pointer result and every acquiring callee keep the refusals they
had, so the mandatory adapter remains the one spelling of an acquisition.
Extern calls are admitted
in a flat sequential entry body only — task workers, `with lock` bodies, error
regions, nested blocks and the `source.exec12` driver body path keep their
existing refusals. A result reached through an extern contract is valid only
*relative to* that declaration (`ContractTrust::Declared`); it is never a proof
about the C implementation. See
[implemented grammar G02d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d-bounded-extern-calls),
the [extern-call evidence](reports/extern-call-v1/README.md) and the
[out/stored extension](reports/extern-call-f1/README.md).

### L14.1. Unchecked linkage

An external library that has **no contract at all** MAY be linked, under an
explicit recorded waiver and never otherwise (card `UNSAFE-LINKAGE`). Without
the waiver, an uncontracted callee keeps its `Q023` refusal exactly as above.

The waiver is **per declaration** and needs two independent, file-local opt-ins:
`import std.extern.unsafe;` (`Q032` otherwise) and the declaration form
`unsafe extern fn NAME(…) -> ABI;`. Neither extern module implies the other, so
a file that calls an uncontracted symbol names both. The gate is on the
declaration rather than on a call to it, because declaring an uncontracted
symbol is itself the trust act.

The waived thing is the **contract**, not the ABI. The declared spellings MUST
be in the §C04.1 vocabulary, and the admitted signature is scalars by value in
and a scalar or `unit` out; **every pointer, in every spelling and every
position, is refused** (`Q033`), `ptr<const, u8>` included. A pointer argument
hands unknown code a capability that no assumption makes checkable — nothing
bounds what it reads, nothing ends the loan at the return, `const` constrains
the declaration rather than the implementation, and nothing stops the callee
publishing it — so there is no premise text to record for it. The admitted
surface therefore carries no custody in either direction, which is why an
uncontracted call cannot change the caller's ledger whatever the callee does. A
`unit` parameter is refused, as is any spelling the ABI table does not contain.
A waiver MUST NOT sit beside a checkable declaration: an inline
`contract { … }` block on the waived declaration, and a name the selected
contract environment also declares, are each refused (`Q034`).

The derived effect row of an uncontracted callee is the **whole** modeled row —
`{Io, Alloc, Mut}` — unconditionally, because nothing is known about it; the
caller's declared `effects { … }` row MUST admit it, so the waiver is visible in
the entry's own signature and not only at the declaration.

Every certificate that names an uncontracted callee MUST carry it as a premise
by name and declared signature, on the same declared-assumption channel that
carries a trusted declared contract. There MUST be no path to a published
certificate that omits it: the premise is derived from the program's own callee
inventory, the certificate is republished only after an independent
rederivation, and a published inventory missing a required premise is refused
(`Q035`). A result reached through unchecked linkage is `Safe` **relative to
that waiver** and MUST be rendered as such; it is never a statement about the
callee. See
[implemented grammar G02d.1](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d1-unchecked-linkage),
`docs/design/UNSAFE_LINKAGE_V1.md` and the
[unchecked-linkage evidence](reports/wave18-unsafe-linkage/README.md).

### L14.2. Manifest-resident unchecked linkage

A waiver of §L14.1 MAY additionally be **resident in a library manifest**, so
that its premise names the artifact it was waived from rather than only the
symbol (card `UNSAFE-LINKAGE-ARTIFACT`; level L1 of the linkage spectrum in
`docs/design/UNSAFE_LINKAGE_V1.md` §1.1).

A `source.native-library.v1` manifest binds one library **artifact identity**
(library name, version, artifact file name, content hash) and four **ABI premise
fields** (data model, unwinding posture, allocator custody, thread posture) to a
list of symbols the artifact exports. An entry is either an explicitly marked
**contract-less** entry, which this section governs, or a **contract slice**
entry, which §L14.3 governs; the two are told apart by their keyword and never
by their shape, so a *dropped* contract can never be read as a waiver. A
manifest is loaded **total or reject**: an unknown field, an unknown field value,
a repeated or missing field, a malformed statement, a malformed entry of either
kind, a symbol bound twice across both kinds, a type spelling outside §C04.1 or a
contract-less entry declaring a pointer rejects the whole manifest (`Q039`). A
selection in which two manifests bind the same symbol is refused (`Q040`),
because the binding MUST be a function.

Selecting manifests is optional and MUST NOT change any program that selects
none: an unmanifested waiver keeps the §L14.1 premise unchanged, and the absence
of an artifact field in it is itself part of the trust statement — **no build
was named**. Where a manifest does name a waived symbol:

- the declared signature and the manifest's exported header declaration MUST
  agree, or the symbol is **refused** (`Q041`). The disagreement does not
  withdraw to the unmanifested premise: the only weaker admissible reading is
  the bare waiver, and reaching it by contradiction would reward a
  contradiction with a premise that says less;
- the **admitted surface does not move**. The closed §L14.1 table decides
  first and unchanged, so a manifest cannot admit a pointer for a
  contract-less entry: an artifact identity states *which code*, not what that
  code does with a capability;
- the certificate MUST carry the sharpened premise in place of the bare one,
  and every omission guard of §L14.1 applies to it unchanged — including the
  publication tripwire, which refuses a certificate that **downgraded** a
  sharpened premise to a bare one;
- a name the selected manifest declares contract-less MUST NOT also be reached
  through a declared contract (`Q042`). This is the mirror of `Q034`: where two
  declarations are visible they are compared, and the refusing direction wins.

The artifact identity is a **pin, not a probe**. The manifest's well-formedness
is checked; the artifact itself is neither read nor hashed at check time, and no
statement is made that the linker used the named build. What a certificate
therefore states is that the program was checked *on the declaration that this
symbol comes from this build*, which makes a substitution at link time a
detectable premise violation rather than an invisible one. The identity and the
four ABI premise fields are declared assumptions in the sense of §L21's verdict
rules, exactly as a declared contract is. See
`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` and the
[manifest evidence](reports/wave19-unsafe-linkage-artifact/README.md).

### L14.3. Manifest contract slices

A manifest entry MAY instead bind its symbol to a **declared contract** (card
`NATIVE-LIBRARY-CONTRACT-SLICE`; the trust artifact of level L3 of the linkage
spectrum in
`docs/design/UNSAFE_LINKAGE_V1.md` §1.1, design §4.1). This is where a library's
**pointer** symbols belong: §L14.1's waived surface admits no pointer in any
position, because a waiver has no modes in which to state what a pointer may be
used for, and a contract slice entry has exactly those modes in the unchanged
§C04 vocabulary.

The manifest **selects and attributes**; it creates no contract semantics. A
contract slice entry names a symbol, the artifact's exported header declaration,
and a declared contract **module reference** — the contract selection's own key,
compared exactly and never normalised. The contract itself is loaded, admitted
and bridged by the unchanged resource-contract v1 route.

The binding MUST be **total in both directions** (`Q047`): every entry MUST
resolve to a declared contract, in the module it names, of the selected contract
environment; and every declaration of a module the slice names MUST be bound by
an entry. A selection binding contracts with no contract environment selected is
refused for the same reason. The slice is derived from the entries themselves.

Where the binding resolves:

- the declared contract's §C04 ABI projection and the manifest's exported header
  declaration MUST agree. On disagreement the declared contract is **withdrawn
  for the analysed unit** (`Q048`): the name is not seeded into the admitted
  environment, every call site becomes an uncontracted callee and refuses at
  `Q023`, and the withdrawal itself is reported rather than performed silently.
  This is the §L14.1 disposition's *opposite*, from one rule: **withdraw only
  where the weaker admissible reading refuses.** At §L14.2 the weaker reading is
  the bare waiver, which is accepted, so a contradiction there refuses outright;
  here the weaker reading is the uncontracted callee, which refuses, so
  withdrawal is already the monotone direction and `LOADER-ABI`'s per-unit
  withdrawal is the right shape. A withdrawal can only turn an admission into a
  refusal;
- the certificate MUST carry **both** premises for that callee: the declared
  contract premise of §L14, unchanged, and an artifact premise naming the
  contract module and the build. The second **stacks with** the first and MUST
  NOT replace it — one states what the code does, the other which code it is —
  and a published certificate carrying only one of them is refused (`Q049`);
- the **admitted surface does not move**. The unchanged §C04 bridging table
  decides what may cross, first and alone, so a manifest binding admits no
  callee shape the contracted route refuses without one.

A waiver for a name the manifest binds to a contract needs no rule of its own:
`Q047` already implies that such a name is declared by the selected environment,
and §L14.1's `Q034` refuses a waiver for exactly that. The artifact identity
remains a **pin, not a probe**, exactly as in §L14.2, and both it and the four
ABI premise fields remain declared assumptions. See
`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4.1–§4.2,
[implemented grammar G02d.2](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d2-the-native-library-manifest)
and the
[contract slice evidence](reports/wave20-native-library-contract-slice/README.md).

### L14.4. The exported header a manifest binds

A manifest MAY name the **exported header** the library ships, and its digest
(card `NATIVE-LIBRARY-HEADER`; design
`docs/design/NATIVE_LIBRARY_HEADER_V1.md`). The two fields are optional **as a
pair**: both, or neither. Naming neither is the default, and a manifest that
names neither is unaffected by this level entirely — it admits the same symbols
and publishes the same premises as it did at §L14.2 and §L14.3.

Where a header is named, the declaration of **every** symbol the manifest binds
MUST be read from that header's own text and compared against the entry's
transcribed declaration, for both entry kinds alike. The reader works on the
header's token stream and MUST NOT require a typed C AST, so that derivation,
validation and certificate rechecking reproduce the decision (the reader
discipline of `docs/reports/remaining-loader-abi/README.md`). v1 reads a
**bounded subset** — comments, line splicing, a whole-file include guard, and
declarations at depth zero — and every shape outside it refuses **by name**:
a directive outside `if`/`ifdef`/`ifndef`/`elif`/`else`/`endif`/`define`/
`undef`/`include`, unbalanced conditionals, a symbol declared only inside a
conditional region, a symbol defined as a macro, a `static` declaration, an
empty (unspecified) parameter list, a variadic declaration, two depth-zero
declarations that project differently, a type this reader does not commit to,
and a type it **does** read for which the manifest's ABI vocabulary has no
spelling. All of these refuse the manifest at `Q063`.

The totality is **one-directional**, and deliberately the opposite of §L14.3's:
a bound symbol the header does not declare refuses, and a declaration the
manifest does not name is ignored. A contract module in a manifest's slice is
authored for that library, so a declaration in it that no entry binds is a
partial binding; a header is the library's entire exported surface, authored
without reference to any manifest.

A **disagreement between the manifest's transcription and the header's own text
refuses** (`Q064`) — the §L14.2 disposition, not §L14.3's, from the same one
rule: *withdraw only where the weaker admissible reading refuses.* The weaker
reading below a withdrawn header binding is the manifest **without** a header,
which is accepted, so withdrawing to it would reward a contradiction with a
premise that says less about the build. Both sides of this comparison are load
inputs besides, so there is no per-unit reading to withdraw.

The header digest is **verified**, unlike the artifact hash of §L14.2, which
remains a pin: the header is read at check time and its text decides admission,
so a certificate that did not name that text by content could not be rederived.
The residence premise therefore carries one verified identity beside three
declared ones, and a rendered premise MUST distinguish them. `sha256` remains
the only admitted algorithm.

### L14.5. The source file's own library binding

Card `NATIVE-LIBRARY-SOURCE-BINDING`. §L14.2 through §L14.4 make a *manifest*
say which build a symbol comes from; the selection itself is the **builder's**
act, and until this level the *source file* said nothing about it. Resolution
was by symbol name across the whole selection: whichever selected manifest
happened to bind a symbol supplied that symbol's premise.

A source file MAY declare `extern library NAME;` at file scope. The form is
import-adjacent — it declares no name and grants no capability — and it states
which manifest-bound library **this file's** declared extern surface is resolved
against. `NAME` is a selected manifest's own `library` field, compared exactly
and never normalised: the constraint of §L14.2's contract module reference, for
the same reason — two spellings that normalise together are two different
selections.

The level is **opt-in**. A file declaring no binding MUST resolve
selection-wide, exactly as at §L14.2–§L14.4, and its certificate MUST be
unchanged. A file declaring one or more MUST resolve only against the manifests
it named, and:

1. a name no selected manifest declares, a name two or more of them declare, a
   repeated binding of one name, and a binding made with no manifest selected
   each refuse the module (`Q065`). Nothing resolves implicitly and nothing
   falls back;
2. a symbol the file declares that a selected manifest it did **not** bind binds
   MUST refuse (`Q066`). It MUST NOT fall through to the weaker premise: that
   would let a file stop its certificate naming a build by binding a different
   library, which is §L14.2's disposition seen from the other side — a
   contradiction rewarded with a premise that says less;
3. a symbol **no** selected manifest binds MUST be unaffected. Absence of
   evidence is not evidence, and the two absences are different: one is a symbol
   nothing claims, the other is a symbol something claims and this file did not
   bind;
4. a published certificate MUST carry a source-binding premise for every
   `extern library` the module declares (`Q067`).

Resolution is **per input file**, and the *declaring* file's binding decides a
declaration — declaring a symbol is itself the trust act (§L14.1). A binding in
another compilation input grants a file nothing and takes nothing away, which is
§L12's file-local import discipline applied to a binding.

A binding MUST be resolved **wherever it is written**, not only where a route
reads it: a module carrying a binding and no extern surface at all is refused if
that binding does not resolve. The converse MUST also hold — a module that writes
no binding MUST NOT cause the manifest selection to be loaded at all, so that a
selection which does not load cannot refuse a module that never mentioned one.

The premise a bound module publishes names the spelling the file wrote, the
manifest that declared it, and that manifest's artifact identity beside its ABI
premise fields. It **stacks** with every callee premise and replaces none: the
callee premises say what each symbol was read as, and this one says which build
the file declared its extern surface to come from. A module that binds a library
and calls nothing it exports still carries it — the binding is a trust act
whether or not a call site consumed it. The identity remains a **stated pin**:
this level resolves a name to a declaration, and reads no file.

The binding is monotone in the refusing direction: every symbol a bound file
resolves, an unbound one resolved too, so a binding can remove a resolution and
can never add one.
#### L14.4.1. Declared include texts, and derived export decorations

A manifest that names a header MAY additionally declare the texts that header's
own `#include` lines may be read from (card
`NATIVE-LIBRARY-HEADER-DECORATION`), each by three parts — the `#include`
operand as the header writes it, the reference the text is read from, and its
digest. Declaring none is the default and is §L14.4 unchanged. An `#include`
whose operand the manifest does **not** declare MUST NOT be followed, exactly as
before; a declared text MUST be supplied, MUST measure the digest declared for
it before any of it is read, and MUST be reached by an `#include` of its
spelling — an unsupplied, substituted or unreached text refuses at `Q063`. Once
a header is bound, a text supplied for a reference no selected manifest declares
refuses at `Q063` as well, so the texts a decision rests on are exactly the
declared ones. A declared text is read under the same bounded subset as the
header itself and is spliced at most once, and the identities of all of them are
published beside the header's in the residence premise.

Where a manifest declares such a text, the reader MAY **derive** from the
declared texts' own `#define`s that a name is an *erasable declaration
decoration*, and erase it from the token stream. A manifest MUST NOT state which
names are erasable: the derivation is a reading of the declared texts, and a
declared erasure list would convert a reader boundary into an unchecked premise
about the header's own text. A name is derivable only when every one of the
following is read out of the declared texts:

1. every `#define` of it is **object-like**;
2. every one of those definitions' replacement lists lies in the reader's own
   published erasure vocabulary — decorations that change neither the declared
   type, the parameter vector nor the linkage name — or is a name already
   derived;
3. it is never `#undef`ed;
4. it is defined on **every path** through the conditional structure preceding
   its first use, where a conditional group establishes definedness only if it
   wrote an `#else` and every one of its arms establishes it.

No condition is evaluated. Condition 4 is what makes the derivation a reading of
the text rather than an assumption about the configuration that will consume it:
whichever arm a consumer selects, the name is defined and its expansion is
erasable, so deleting the name **is** expanding it. A name that is not derivable
is not erased, and the declaration that carries it keeps the §L14.4 refusal it
already had. The same vocabulary erases a decoration written out in full, in
either the prefix or the suffix position of a declaration, so the two spellings
read alike; a decoration outside the vocabulary refuses the declaration by name
rather than being stepped over.

## L15. Lowering, observations, and translation certificates

Sequential lowering MUST preserve the checked resource identities, ordered acquisition/release/transfer behavior, and modeled normal/error payloads. The tagged C error representation must not bypass cleanup through unmodeled control transfers.

The current validation pipeline compares identified observations of source/IR and CFG execution, and its translation utilities can check generated C shape and supplied native observations. Such checks establish agreement for the selected model and inputs. They do not prove all compiler executions, all C undefined-behavior conditions, all targets, or general source refinement.

A translation certificate MUST identify the source model, CFG, emitted artifact where applicable, observation semantics, and supplied environment. A certificate for one phase MUST NOT be reused as proof of a different phase without a checked relation. A native run is an execution record, not a replacement for static coverage.

If a future concurrency backend permits independent operations to reorder, validation MUST compare the relevant partial order and observable outcomes, not insist on one incidental total instrumentation order. Conversely, a partial-order comparison MUST retain required lifecycle, synchronization, and cleanup edges.

## L16. Checked concurrency: required rules and current limitations

The design separates ordinary memory-race freedom, resource/protocol-conflict freedom, and lifetime safety. A checker must say which property it establishes. Lock-order checks do not, by themselves, establish freedom from all deadlocks. Atomic operations do not establish pointee lifetime, publication safety, fairness, or progress.

For a full access-capability profile, authority is indexed by lifetime identity and footprint. Pointwise shares across all custodians, parked mutex invariants, and completed-but-unjoined task capsules MUST be conserved. The unassigned complement must be explicitly outside the claim or accounted for as a reserve; it is not freely available to every task.

A spawn partitions or transfers authority and records a task handle. A completed child retains its result, error, and returned authority until join or another explicitly modeled collection protocol. Join consumes the handle and returns authority according to the child's outcome. Completion, join, and permission return MUST NOT be conflated with a mere sequence position in an input array.

A mutex protects a declared footprint/invariant. Holding an unrelated mutex MUST NOT create access authority. Unlock returns the protected authority to the mutex state. Facts about mutable protected memory must be invalidated or re-established across release and reacquisition.

Destruction requires unique release custody, a live generation, and quiescence with respect to all outstanding accesses, loans, callbacks, and tasks that can use that generation. No access or transfer may act on a destroyed generation. A joined task may not subsequently execute more events.

The current `model.concurrent14` checker uses interval footprints, fixed initial permissions, vector clocks, spawn/join edges, mutex release/acquire edges, and a custody table. Cross-task input order alone does not create happens-before. It checks conflicting accesses, lock order, live-generation use and active-task state. Access/transfer after destruction reject. Destruction requires custody, quiescence of other modeled authority and happens-before ordering of previous accesses. Join terminalizes a child only after its held locks and descendants are resolved; retained clock history does not preserve execution authority. R01/R02 regressions and reduced Lean lifetime models cover this boundary.

The certificate declares `TrustedExplicitTrace` and the initial-authority/unique-lifetime-ID premise, plus guard-footprint and synchronized-handoff premises when used. A held guard still relies on its supplied footprint binding, and a true synchronization flag still relies on declared real synchronization. These certificates do not prove faithful extraction from arbitrary source or all lifetime protocols.

A second profile, `CheckedMutexInvariantV1`, discharges the guard-footprint and synchronized-handoff premises inside its own bounded model over supplied traces. Every mutex generation declares its protected footprint at creation and consumes the creator's full share over it, so a protected footprint has exactly one protector and no fallback authority (a generation may later grow that footprint only under the bounded rule below, and may never shrink or replace it); access authority is keyed to a freshly minted guard instance, not to a mutex name; and a custody transfer's synchronization witness is derived from the checker's own happens-before history and compared, never believed. Its certificates MUST NOT carry `GuardFootprintAuthority` or `SynchronizedCustodyEdges`; they retain `DeclaredFootprintShape` and, when a `close` supplies a declared predicate, `InvariantPredicateDeclared`. Condition variables, try/timed locks, reader/writer locks, upgradeable guards, nested protecting mutexes, recursive acquisition and composition with the task-completion capsule are represented refusals, not omissions. `TrustedExplicitTrace` is unchanged and is not retired by this profile.

Lock ranks are no longer among those refusals. The profile carries one **opt-in** lock-rank discipline, off unless the program declares at least one rank, in which case: every mutex generation the trace creates MUST have a declared rank and every declared rank MUST name a generation the trace creates (`Q625`); and an acquisition by a task that already holds a live guard over a generation at an equal or higher rank MUST be refused (`Q624`), so the declared order is a strict one. The rank test is the last precondition of an acquisition, after every existing one, so the discipline never displaces a refusal the profile already issues. Certificates record the declared ranks verbatim and an independent recheck re-derives them; the discipline adds no assumption, because a rank is a declared position in an order rather than a claim about memory, layout or the scheduler. A passing rank check establishes only that no acquisition in the trace nests in non-increasing rank order. It is NOT a deadlock-freedom proof: this model has no blocking (an acquisition of a held generation is refused, not parked), so the wait-for graph a deadlock argument requires is not represented, and waits on anything other than these mutexes are outside the profile. A rank construct outside this discipline — a dynamic rank, a rank propagated through a call, a rank relation the model cannot decide — remains a represented refusal at `Q622`.

A proposal to change a generation's protected shape after creation MUST be refused at `Q618`; that refusal is unconditional and unchanged. Beside it the profile carries one **opt-in** protection growth, which a program requests explicitly and which is admitted only when all of the following hold: the growing task holds a live guard over the generation, and the generation is read off that guard rather than named separately; that guard's invariant is open (`Q626`), so the shape change lands in the window where the declared predicate need not hold and the section is then forced through a close before it may unlock or exit; the interval overlaps no live generation's protected shape, that generation's own included (`Q617`); the growing task is the generation's creator and holds the full share over the whole interval (`Q621`), because parking is all-or-nothing and destruction returns every parked share to the creator; and the growth moves the generation's declared invariant to an identity that generation has never carried (`Q627`), because a wider protected region is a different proposition. An admitted growth counts as a change to the protected region, so the checked no-write invariant witness MUST NOT be available to the section that grew. Growth is permanent for the generation: there is no shrink and no replacement, and the grown intervals are released to the creator with the declared ones at destruction. Certificates MUST name every admitted growth with its event, guard, interval and both invariant identities, and MUST record the effective protected shape; an independent recheck re-derives both. Growth adds no assumption: what the newly protected memory satisfies is the same `InvariantPredicateDeclared` premise, now attached to a named new identity, and the declared intervals remain `DeclaredFootprintShape`.

The source driver does not lower general concurrency syntax. It lowers one bounded surface, `source.mutex.guard.v1` ([G02c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02c-guarded-fixed-footprint-mutex)): a lexical guard over one fixed-footprint generation. Every lowered statement is checked by the model above and the driver re-derives the certificate before publishing it. Source-to-trace refinement, the real C layout of the declared intervals, the invariant predicate, deadlock freedom, contention and general native refinement remain outside that result. Fixed Windows execution through the emitted compatibility subset is recorded in [the Windows thread report](reports/windows-c11-threads/README.md).

That surface admits three further forms, each bounded by a decidable rule rather than by convention ([the T03-F record](reports/remaining-t03-f/README.md)). A `close` MAY select the checked no-write witness, and the lowering MUST admit that selection only for a region whose every statement since the matching `open` cannot write; any other region MUST be refused rather than downgraded to the declared witness. A refinement fact MUST be scoped to the guard under which it was established, so a use after that guard's release is a stale-fact refusal, and each fresh assertion MUST mint a fresh identity. A child task MAY be created and joined by a lexical block; it receives no share and no custody, so its access to a declared resource is mutex-mediated or refused, and its join is subject to the model's existing rule that no task joins while it holds a guard. A checked trace carrying task structure has no single-thread native rendering and MUST be published without emitted C rather than with C the backend did not produce. The scoped-task capture, owned-value and completion-capsule forms remain refused.

The `concurrency checked;` marker is not evidence of race verification. The marker alone does not select evidence. The scoped-copy path independently checks its typed task IR; the supplied-event concurrency checker remains a separate API. A future supported source-concurrency profile MUST derive and validate its events, task transitions, access permissions, and synchronization witnesses rather than accepting producer-supplied truth flags.

## L17. Explicit C11 execution candidates

The weak-memory API accepts explicit events and relations such as reads-from, modification order, synchronization, sequential consistency order, and mutex order. Its checker and associated theorems concern that supplied candidate representation and its implemented axioms.

Accepting one candidate does not establish that every execution of a C program is race-free. Rejecting a candidate does not establish that the program is safe either. A whole-program claim requires a justified complete candidate set or another sound all-execution argument, including publication and reclamation conditions relevant to the property.

A provider or source importer MUST NOT manufacture a synchronization relation merely because it makes a candidate pass. Candidate relations must be derived or explicitly declared as assumptions, with the distinction preserved in the evidence record. A `volatile` qualifier is not an independently established synchronization witness in this specification.

## L18. Existing C and interprocedural summaries

The unrestricted-C gateway first invokes Clang with a declared dialect, include paths, and preprocessor definitions. It then projects supported behavior into a conservative resource model. `unrestricted` describes what can be presented to the production C front end, not what the resource analyzer can completely verify.

Unsupported control flow, unresolved alias identities, missing contracts, and incomplete function projection preserve Unknown. An analysis of main-source definitions differs from an analysis of every preprocessed definition; that scope choice MUST travel with the result. A library-specific source importer may support a different subset from the generic gateway and must report its own coverage.

The fixed scalar type-query rule admits `sizeof` and `_Alignof` of a readable
scalar type name, and `sizeof` of one declared scalar variable, as unevaluated
data in the generic gateway's six expression positions and scalar assignments.
Its operand-class marker MUST NOT become a numerical size, alignment or branch
fact. A modeled assignment forgets the destination's old scalar value;
expression-position mutations beside a query remain unsupported by this rule.
Variable-length arrays, opaque types and compound operands are outside this
admission. The same token-derived decision governs summary validation and
certificate rechecking. See [the bounded implementation record](reports/wave23-c-operations/README.md).

External resource declarations are assumptions. W4 may infer an internal summary, but a candidate is published only after body re-analysis validates its entry assumptions, every modeled exit, formal disposition, result custody, and absence of unmodeled escape. The certificate checker re-derives summaries instead of trusting serialized labels.

The current W4 implementation supports bounded direct Out, Stored, Removed,
Reallocated and nullable-return inference with re-analysis and independent
rederivation. Immutable entry-prefix output aliases of the same explicit flat
pointer type may forward modeled Out calls or direct stores; casts, mutation,
shadowing, escapes and branch-local alias definitions remain unsupported. The
original sanctioned slot and every exit are independently validated, and source
certificates retain the original C text. Removed wrappers may admit null only after validating every newly
admitted path: the null child has no custody; the nonnull child starts contained
and ends detached/live with its original owner retained. Context-specific
zero/nonzero recursion requires strictly lower-rank validated calls. General
removal/out-slot aliases, relational branch analysis, general recursive fixed
points and cost summaries remain unsupported. Declared contracts take precedence; disagreement is a SummaryConflict, not a silent override. Unresolved recursive components do not acquire a self-justifying inferred summary. See the companion contract specification for the exact trust and mode rules.

## L19. Contract clauses and verifier scopes

A source function can retain `requires`, `ensures normal`, `ensures error(Type)`, `ensures any`, and `quant` clauses. The structural parser retains expression text and spans, not a universally typed predicate AST. The required interpretation depends on the selected consumer.

A contract-bearing declaration does not automatically install a legacy-C resource summary. Installation requires the strict resource-v1 loader and an explicit selection. A fact-policy checker does not establish arbitrary caller preconditions. A bounded source checker does not establish whole-program reachability. These boundaries are normative and are expanded in `QCORE_CONTRACT_SPEC_r0_3.md`.

A verifier scope selects provider configuration. It grants no ownership, access, or proof authority. Registry resolution checks IDs, versions, profiles, and capabilities. Current source scopes enter core obligation derivation and dispatch. Incomplete imports or unsupported effects cannot discharge their requested scope. Lifecycle derivation remains deliberately incomplete until a real effect/ABI importer is admitted. Invalid configuration is reported as `Q700`; dispatch uses the core Q7xx namespace.

Protocol v1 accepts no Safe proposal, including an empty or perfectly matching Boolean claim set. Unchecked Unsafe allegations become Unknown. W2 profile loading and bounded W3 evidence-v2/dispatch are now implemented and recorded in repository evidence, beyond the reviewed archive. V2 observation replay requires declared `observations_trusted = true` and observed-run bounds; explicit false is never overwritten. No first-party source safety is promoted.

## L20. Verdicts and claim boundaries

Every verification claim MUST identify

```text
(artifact, property, semantic profile, scope, bounds, assumptions,
 evidence version, evidence provenance)
```

This is a reporting requirement; the complete tuple is not already one universally implemented record across all current APIs.

**Safe** means that the named property is established for the identified subject, scope, and assumptions with complete coverage under the selected profile. **Unsafe** means a validated defect or counterexample under that profile. **Unknown** means the requested property was not established, including unsupported behavior, incomplete imports, missing premises, invalid evidence, or exhausted analysis bounds.

These words MUST NOT be repurposed to mean successful parsing, a green build, contract loading, or observed cleanup in a few fixtures. Rejection of malformed source is a compilation result, not necessarily a semantic counterexample to a well-formed program.

A partial report may retain validated local defects alongside an aggregate Unknown result. It must not erase uncertainty by counting provisional warnings as whole-program Unsafe certificates. Aggregate composition MUST account for every selected obligation; an empty provider coverage set cannot certify an uninspected unit.

## L21. Diagnostics and compatibility

The diagnostic ownership below reflects the merged W3/W7 migration. The prose revision does not itself change wire versions.

Ownership is allocated by *semantic pass*, not by crate: a pass owns a code range, and any crate that runs that pass emits from it. `qcore_frontend::frontend::source_diagnostic` is the authoritative pass → code map, and a code may not be reused by a second pass. Within a range, each individual code names one specific condition at its emission site.

| Namespace | Owning pass | Emitted by | Meanings in force |
|---|---|---|---|
| `Q000` | `NameResolution` (per input file) | `qcore-verification::driver` | An input file does not parse as a QCore declaration. |
| `Q001`, `Q016` | `Index` | `qcore-frontend` | `Q001` generic index/quant presentation; `Q016` dimension/axis declaration validity (checked unconditionally) and quant-index type resolution. |
| `Q010`–`Q015` | `NameResolution` | `qcore-frontend` | `Q010` duplicate declaration, `Q011` duplicate parameter, `Q012` duplicate local binding, `Q013` unknown name, `Q014` unknown imported module, `Q015` a capability used without its file-local import — concurrency, or one of the three `STD-MODULES` capabilities (`print` without `std.io`, `log`/`flush` without `std.logger`, a named rendering without `std.fmt`), the message naming the missing module. |
| `Q020`, `Q021` | `OrdinaryTypes` | `qcore-frontend`, `qcore-verification::driver` | `Q020` unsupported or ill-typed ordinary construct, including the emitted entry's parameter rule, the task route's source-contract gate, and an `effects { … }` row naming an effect the extern-call profile does not model; `Q021` the sequential whole-program gate refusing an unchecked source `contract { … }`. The two contract gates are route-specific and deliberately not unified — see below. |
| `Q022`–`Q027` | `ExternCall` | `qcore-frontend`, `qcore-verification::driver` | Admission and bridging of a call to a **declared** extern function (`source.extern-call.v1`). `Q022` an extern call in a file without its `import std.extern;`; `Q023` the callee has no admitted declared contract in the explicit selection (no selection, a selection that does not load, or an absent name); `Q024` the closed v1 bridging table refuses this contract mode, ABI type, argument form or argument count — including a stored argument that names the same identity as its declared owner, an error-arm disposition of the wrong arity or argument form, and — card `PIPE-SCALAR-RESULT` — a bound result whose declared result type is not a custody-free scalar (a `unit`, or any pointer, including one declared `resource_return_none()`), a resource binding passed at a declared scalar parameter, and a scalar argument this body never bound; `Q025` the mandatory acquisition adapter is missing, applied to a callee that establishes no fresh custody, names a declared non-error, or does not discharge live custody on its error arm. Card `PIPE-SCALAR-RESULT` adds one condition and no code: binding an *acquiring* callee's result to a scalar local refuses here, because an acquisition has exactly one spelling — the adapter — and the scalar-binding form is not a second one. The adapter is mandatory for each of the three admitted acquisition sources: a `resource_return_owned(nullable)` result, a `resource_out(_, nullable)` slot and a `resource_return_reallocated(_, nullable)` result (card `EXTERN-MODES`). Under card `EXTERN-ERR-REL` the error arm may carry explicit dispositions, and `Q025` covers every way one fails to be that acquisition's established cleanup edge: an identity live at the raise that no disposition releases (the retained abandonment refusal), a disposition that itself acquires or stores, one that discharges other than exactly one identity, one that names an identity not live on the arm, an order other than reverse acquisition order, and a disposition whose derived effect row the entry's declared row does not admit. An acquiring call that also moves custody out is refused there too, because a declaration constrains only its normal exit — with the single exception card `EXTERN-MODES` measures, a `resource_return_reallocated` subject, whose failure arm §C04.3 does state and which therefore stays live on the arm and must be dispositioned there; `Q026` an extern call in a context this profile refuses (a nested block, a non-entry function, or an entry with parameters), or — card `PIPE-SCALAR-RESULT` — a `let` whose name already means the other kind of thing in this body, a scalar binding shadowing a live resource binding or the reverse; that arm is an unreachable fail-closed backstop, since name resolution's own `Q012` catches every duplicate local binding before the lowering runs; `Q027` a declared input and the checked artifact disagree — an `extern fn` declaration against its selected contract, an inline `contract { … }` on a called extern declaration, a selection that no longer reproduces its own sources, or a published certificate against its independent rederivation. Card `STD-IO-COMPOSE` adds two conditions inside this band and no code: `Q025` also refuses a `let` that is neither an acquisition, a scalar-result binding nor a scalar **literal** binding (`let NAME = INTEGER;`, admitted by `EXTERN-SCALAR-LITERAL` — it calls nothing, so it derives no effect, admits no argument and opens no obligation), and the scalar-result binding admits an optional **declared outcome** tail, `succeeds INTEGER`, on the `succeeds BOOL` precedent — the site's literal outcome control, compared against the real result by the emitted C, which exits 70 on disagreement. No reserved word was added: the two tails are told apart by the literal alone, so a malformed adapter tail can never be read as the other one and keeps its own `Q025`. |
| `Q028`–`Q031` | `StandardIo` | `qcore-frontend`, `qcore-verification::driver` | Admission of the line-producing statements of `std.fmt`, `std.io` and `std.logger` (`source.std-io.v1`, card `STD-MODULES`). `Q028` a rendering operand that is not one of the entry's integer bindings — the refusal that keeps every resource unrenderable through a **bare** operand, since neither bare operand form has a constructor that could denote one. Card `STD-FMT-BORROW` adds one condition inside this band and no code: a `borrow NAME` operand for which no checked custody model published a live identity also refuses here, which on `source.std-io.v1` is *every* borrowed render, because that route has no custody model and its identity environment is therefore empty. The refusal is structural — the absence of a model rather than a rule this route states — and the two environments are read by two lookups that cannot reach each other, so supplying identities can never make a bare operand denote a resource and a borrowed operand can never resolve to a scalar. `borrow` was already a reserved source word (the checked mutex-guard profile's), so the form adds none. Whether a supplied identity is *live* is decided by the composing route's join at `Q043`, never here; `Q029` the entry performs `Io` but its declared `effects { … }` row does not admit it; `Q030` the flat entry shape this profile models is violated — a statement it does not model, a nested block, an entry with parameters, a `print`/`log`/`flush` outside the entry, an unknown rendering or level *whose word is reserved*, a `flush` with an operand, a missing or repeated return, or a malformed `effects` row; `Q031` a published certificate against its independent rederivation. An unreserved word in a rendering or level position is collected as a referenced name and refuses earlier at `Q013`; that is fail-closed and recorded, not `Q030`. |
| `Q032`–`Q035` | `UncheckedLinkage` | `qcore-frontend`, `qcore-verification::driver` | Admission of an extern declaration that has **no** contract at all, so an external library can be linked without one (§L14.1, card `UNSAFE-LINKAGE`). A separate pass from `ExternCall` on purpose: that pass decides whether a *declared contract* bridges to a call, while this one decides whether a signature may cross a boundary nothing checks, and records the premise that says so. `Q032` an `unsafe extern fn` declaration in a file without its `import std.extern.unsafe;` — the gate is on the declaration, not on a call to it; `Q033` the declared signature is outside the closed unchecked table: a pointer parameter or result in any spelling (`ptr<const, u8>` included, deliberately), a `unit` parameter, or a spelling the §C04.1 ABI vocabulary does not contain — the contract is waived, the ABI is not; `Q034` a waiver beside a checkable declaration: an inline `contract { … }` block on the waived declaration, or a name the selected contract environment also declares, in which case the contracted route wins whether or not the signatures agree; `Q035` a published certificate that does not carry an unchecked-linkage premise for an uncontracted callee its own program declares. `Q022`–`Q027` are unchanged: without the waiver an uncontracted callee still refuses at `Q023`. |
| `Q036`–`Q038` | `Pipe` | `qcore-frontend`, `qcore-verification::driver` | Admission of the pipe operator `\|>` (`source.pipe.v1`, card `PIPE-OPERATOR`) and, at `Q038`, the per-file **operator gate** deciding where each operator token may appear at all. The pass runs *before* every other one: a well-formed pipeline is desugared into the tokens of the equivalent unpiped statement at parse time, so no later pass can tell a pipe was written and this pass exists only to attribute the refusals of the forms that are **not** desugared away. Card `PIPE-CHAIN` extends the desugaring from one stage to a chain: a pipeline of `n` stages becomes a sequence of `n` statements, each non-final stage binding its result to a hygienic temporary the next receives, and the final stage keeping the statement form the reader wrote around the whole pipeline (which closes `PIPE-LET`). No diagnostic code moved and none was added. `Q036` a pipeline shape this profile does not model — an empty operand or stage, a stage that is neither a statement head nor a callee name, a sink stage naming the wrong number of selector words, a call stage whose parentheses are unbalanced, a malformed or value-less `let NAME =` binder, an assignment pipeline (follow-on `PIPE-ASSIGN`), `move` on the first stage (whose operand already has the language's own spelling), a bare `move` whose incoming value is not a binding, or a file that needs pipeline temporaries and itself writes the reserved `qcore__pipe` prefix (hygiene is enforced by refusal, never by silent renaming); `Q037` a stage that cannot receive the piped operand — `flush`, which takes none, or a **statement stage** (`print`, `log`, `drop`) in a non-final position, since only a call stage binds a result. Card `PIPE-SCALAR-RESULT` widened that condition without adding or moving a code: "produces a value" was decided by the presence of `succeeds`, because the acquisition adapter was then the only `let`-with-call the language admitted, and is now decided by **stage kind**, because a non-acquiring call's scalar result is bindable too. Whether a particular callee's declared result is bindable is the route's decision, so a `unit`-returning callee in a non-final position still refuses — at `Q024` naming its declared result rather than here; `Q038` an operator token used where this revision gives it no meaning. Card `OR-AND-LOWERING` **restated this condition through the §L21 process** rather than moving the code or adding one: under card `PIPE-OPERATOR` it read "a claimed-but-unlowered logical operator", because the two connectives had no branch construct to lower onto; `IF-STATEMENT` landed that construct and `OR-AND-LOWERING` lowers them, so what is left is the positional decision this gate has always made. Card `BITWISE-CONDITIONS` then **spent** the three reservations and made the condition unconditional: **all five** operator spellings `\|`, `&`, `^`, `\|\|` and `&&` are refused here **outside an `if` condition** and admitted inside one, where they are the connective tree and the integer tier `source.branch.v1` lowers (§G02i). That is the only position where there is anything to hold a Boolean or an integer value — there is still no expression tier of any kind elsewhere — and an occurrence *inside* a condition is not this pass's to refuse: the condition tier decides it at `Q045`. The three single-byte messages therefore name the position rather than a reservation (follow-ons `BITWISE-LET-INITIALIZER` and `BITWISE-STD-IO-OPERAND` for the two positions most obviously next). The gate reads the condition extents the parser recorded, so "condition position" means the parser decided so, never "an `if` appears to the left". Two token retables are recorded with this code and neither changed any program's verdict: `PIPE-OPERATOR` moved a bare `\|` from a lex error to this refusal, and `OR-AND-LOWERING` moved a bare `^` the same way while moving a bare `&` from its downstream route refusal to this one. Dropping the `;` of a body's last statement is a parse failure at `Q000` before this pass runs. Card `PIPE-INTRINSIC-STAGE` **restated the first sentence of this row through the §L21 process** and again moved and added no code. "The pass runs before every other one, so no later pass can tell a pipe was written" was true of the *token stream* and not of every reader of it: the sequential route recovered each statement's tokens by re-lexing its own source bytes, which for a pipeline recovers the pipeline rather than the statement it desugared to — the operand sits left of the callee in the bytes and right of it in the statement — so that route's `qcore_acquire` reader refused the very form the reader had written, at `Q500` naming the hand form. Reading the parsed program's own post-desugaring stream is identical on every pipe-free file, being the lex of that same source, and makes the sentence true of every route. The card also measured that a **call-shaped model intrinsic** — `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload`, `qcore_acquire`, `qcore_check` — is already a call stage by the `Q037` condition above, so `buffer \|> qcore_buffer_read \|> log info dec` needed no stage-table entry, no reserved word and no code; the keyword-shaped intrinsics (`spawn`, `join`, `share`, `grow`, `open`, `close`) stay unreachable because no call stage produces a statement whose operands are not a parenthesized argument list (§G02h). Card `PIPE-DROP-STAGE` then settled the one entry that card withheld, again **without adding, moving or changing a code**: `drop` is the closed statement table's third head and its only *releasing* one, so `b \|> drop;` is `drop b;` and an allocating chain may end in its own release. Its terminality is the `Q037` condition above **applied rather than extended** — a releasing stage binds no result, exactly as a rendering one does not, so a released value cannot feed a later stage because there is no value to feed — and a bound pipeline ending in one is the `Q036` condition above, likewise unchanged. The pipe states nothing about custody: what a *later statement* does with a released binding is the route's own checker, so a piped double release refuses exactly where the hand-written one does. No other keyword-shaped head joined, on a criterion recorded in the stage table itself: the statement form must be `HEAD SELECTOR* OPERAND` with closed selectors, **and** some stage must be able to feed it a value no reader can name — which is false for the mutex heads (their route has no call-shaped intrinsic), for the epoch plan clauses (their operands are check-time constants), and for `return` (a bound pipeline already names a chain's result), and is beside the point for `move`, which already spells this grammar's own consume prefix. |
| `Q039`–`Q042` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | Admission of a `source.native-library.v1` manifest selection and of the manifest-resident waivers it sharpens (§L14.2, card `UNSAFE-LINKAGE-ARTIFACT`; the band `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §7 reserved after `UNSAFE-LINKAGE` took `Q032`–`Q035` and `PIPE-OPERATOR` `Q036`–`Q038`). A separate pass from `UncheckedLinkage` on purpose: that pass decides whether a *signature* may cross a boundary nothing checks and its decision is unchanged here, while this one decides whether the **manifest** a waiver is resident in loads, binds and agrees — the trust object is the manifest, not the declaration. `Q039` a selected manifest does not load, total-or-reject: a missing marker, an unterminated statement, an unknown field, an unknown field value, a repeated or missing field, a malformed hash, a malformed entry of either kind — including a `contract fn` with no `from MODULE` tail and an `unchecked fn` carrying one, since the two kinds are told apart by their keyword and never by their shape — a malformed contract module reference, a type spelling outside the §C04.1 ABI projection, a contract-less entry declaring a pointer, a `unit` parameter in either kind, a symbol bound twice across both kinds, or a manifest binding nothing. Card `NATIVE-LIBRARY-CONTRACT-SLICE` **restated this condition list through the §L21 process** rather than moving or adding a code: `Q039` still means "a selected manifest does not load", and what changed is that a contracted entry is now a grammar this loader implements instead of one it declines; `Q040` two selected manifests bind the same symbol, so the binding would not be a function; `Q041` a waived declaration and the manifest that names it disagree in arity, a parameter or the result — a refusal, never a withdrawal to the weaker unmanifested premise; `Q042` a name the selected manifest declares contract-less is also reached through a declared contract, the mirror of `Q034`. Selecting no manifest is the default and changes nothing: `Q032`–`Q035` and `Q022`–`Q027` are unchanged, and an unmanifested waiver keeps its §L14.1 premise byte for byte. |
| `Q043` | `ComposedStdIo` | `qcore-frontend`, `qcore-verification::driver` | The **join** of the checked composition of `source.extern-call.v1` and `source.std-io.v1` in one entry (`source.std-io-extern.v1`, card `STD-IO-COMPOSE`). One code, not a band, and deliberately so: a composed module receives every `Q022`–`Q027` refusal from the extern pass at the extern pass's own site and every `Q028`–`Q031` refusal from the standard-IO pass at its own site, because the composition runs both routes' unedited lowerings and both routes' unedited checkers and only then checks where they meet. `Q043` therefore owns exactly three rules, none of which either composed model can state. (1) The composed order is an **interleaving**: every event of both projections appears exactly once, in that projection's own order, ending in the single `return;` both models require — a program whose order lost, duplicated or reordered an event is refused rather than checked, since a projection would then have been checked against a program the emitted C does not perform. (2) A rendering operand naming a scalar resolves against the **extern model's value namespace**, must be bound by an earlier composed step, must be of the one ABI width `source.std-io.v1` renders, and must be a value whose number the source **determined** — a literal binding, or a call binding carrying the declared outcome `succeeds INTEGER`. A bound call result with no declared outcome is not a renderable operand: rendering it would publish a line no model computed. A name that denotes a resource identity is not refused here at all when it is written as a **bare** operand, because neither bare constructor can carry one; it keeps the standard-IO route's own `Q028`. (3) The composed derived effect row is the **union** of the two projections' rows and must be admitted by the one row the entry declares. Card `STD-FMT-BORROW` adds a fourth rule inside this code and no new one: (4) a **borrowed** rendering, `print RENDERING borrow NAME;`, resolves `NAME` against the extern model's **custody** namespace and is admitted only where that model's own published ledger holds the identity live at the rendering's composed step, and only when the identity the standard-IO projection rendered is that identity. An identity this module binds no resource for, one never established (a declared null arm included), and one that departed by consumption, release, storage or reallocation before the rendering are each refused. The composition derives no liveness of its own: it reads the extern certificate's `live_after`, so it still owns no custody rule. What is rendered is the model identity and nothing else — never the object's contents, its address or an extent, none of which any model a rendering can reach carries, and the address in particular is not model-determined, so a line carrying one could not be compared against a reference run at all. The rendering moves nothing: the composed ledger is still exactly the extern projection's, the identity is as live after the line as before it, and a rendering is **not** a publication in the sense standing constraint 10 refuses, because its destination is a byte stream rather than an object of the custody graph, the bytes are a copy of the identity number rather than the address, and no model operation reads a stream back. A rendering also binds nothing: no `ValueId` is introduced for a borrowed render, so the identity cannot reach a declared scalar parameter or any other value position. Also the pass's generic presentation code, and the code of the composed certificate's own recheck failure — a composed publication rechecks all three certificates, each sub-certificate by that route's own recheck. Route selection runs this composition **after every concurrent route and before both routes it composes**, so a module that prints and spawns, or extern-calls and spawns, keeps its concurrent route's existing refusal, and a module using one of the two composed feature sets falls through to that route byte for byte. |
| `Q044`–`Q046` | `Branch` | `qcore-frontend`, `qcore-verification::driver` | Admission of the source branching construct `if (COND) { … } else { … }` (`source.branch.v1`, card `IF-STATEMENT`). A separate pass from `CEmission` on purpose: `Q500` refuses a construct the reduced C11 CFG *cannot encode*, and this construct **is** encoded — the typed CFG has carried two-way terminators since Cycle 7 and now carries `CfgTerminator::Branch` beside them. What this pass decides is whether a written branch is inside `source.branch.v1`. `Q044` a branch shape this profile does not model. Card `IF-STATEMENT` set the arm subset at `qcore_check`, `drop`, `move`, a nested `if` and the empty arm, refusing a `return` and a call; card `IF-ARMS` **admits both** and the code's condition narrows to the shapes the widening cannot place. (i) A `return` inside an arm is admitted because an exit from an arm carries exactly the obligations the sequential route imposes at any exit — `SourceOp::Return` demands an empty live list wherever it stands — so the returning arm discharges at its own exit and does not reach the join, which is then taken over the arms that survive it. Four of the five models that join a branch cannot express an exiting arm, so the statements written *after* the branch are lowered inside the arm that survives to reach them; that rewrite is a **move**, the continuation being lowered exactly once, and `Q044` refuses the shapes it cannot serve: a mixed branch with no continuation for the survivor to reach, a continuation that still reaches no `return`, and an early return in one arm of a nested branch whose sibling also falls through, which would need the continuation in two arms (follow-on `IF-ARM-RETURN-NESTED`). Statements after a branch every arm of which returns are unreachable and refused here rather than joined. An arm exit with an owned resource still live is **not** this code: it is the program exit's own condition, reported at `Q202` at the exit that leaks. (ii) A call inside an arm is admitted because a source call in this executable subset emits no C at all — it composes the callee's summary flow and effects into the caller's and demands the callee's usage — so it cannot make "the" observation ill-defined whichever arm it stands in. What it does raise is the **flat, positional** whole-program call sequence, which composes `segment · callee · segment · …` on every path; `Q044` therefore refuses a conditional call whose callee's summary flow is not neutral, because composing a neutral flow once is the identity and composing anything else would apply resource flow on paths that never perform the call (follow-on `IF-ARM-CALL-FLOW`). The arm's own custody flow is deliberately **not** split into that chain, so the four-model join still sees it. (iii) A `let` inside an arm — an arm-local binding has no position after the join (refused here by card `BITWISE-CONDITIONS`; widening it is arm-subset business). (iv) An `if` whose block and condition counts are inconsistent. Also the pass's generic presentation code; `Q045` a condition outside the decidable condition tier. Card `IF-STATEMENT` set that tier at one leaf — `true`, `false`, or a comparison of two signed 32-bit integer literals by `==`, `!=`, `<`, `<=`, `>` or `>=`. Card `OR-AND-LOWERING` **widened it to a connective tree** over the same leaves, again without moving or adding a code, and card `BITWISE-CONDITIONS` widened it a second time — unary `!`, the three bitwise operators inside a comparison operand, and names bound to integer literals — still without moving or adding a code, because the condition `Q045` decides has always been "a condition outside the decidable tier" and the tier grew rather than changed in kind. The grammar is now **two-sorted** and is spelled out in §G02i. Precedence is declared, not inherited from a parser accident, under one rule: **a precedence is declared only where it agrees with C, and where this tier's two sorts would force a different ranking than C's, the unparenthesized spelling is refused rather than silently redefined.** `&&` binds tighter than `\|\|`; `!` tighter than `&&`; `&` tighter than `^` tighter than `\|`; all are left-associative except prefix `!`, parentheses group, and a comparison is non-associative. C ranks `!` *tighter* and `\|`/`^`/`&` *looser* than the comparisons, so `!A < B` and `A \| B == C` are refused and `!(A < B)` and `(A \| B) == C` are the spellings. A `NAME` operand is a **check-time constant binding** — `let NAME = INTEGER;` at the top level of a sequential entry body — substituted for its value before any model runs; a name denoting a resource identity is refused here, so no custody-bearing value gains a position in the tier. A connective and a negation are eliminated **before** any checked model sees them — a negation by the **arm swap**, `if (!C) T else E` being `if (C) E else T` — `A \|\| B` is the right-nested branch and `A && B` the then-nested one that a reader would have written by hand, over the same two arms, so `qcore_core` is byte-unchanged and the emitted C is byte-identical to the hand-lowered twin's. Every level runs the same custody join, so a connective cannot launder a `Q046` disagreement. A call, a pipeline (`\|>` — never given a relative precedence; the mixture is refused), a shift, a complement, an arithmetic operator, a chained comparison, a name bound to nothing or to a resource, an integer where a condition is required or a condition where an integer is required, an ungrouped bitwise or `!` operand, a non-integer operand, an unbalanced parenthesis or an empty condition is refused. The tier is decidable at check time for the same reason the `succeeds` and `passes` literal outcome controls beside it are: the translation certificate compares one reference observation, one CFG observation and one observation parsed from the compiled program's own output, and a run-time-varying condition would leave “the” observation ill-defined (follow-on `IF-RUNTIME-CONDITION`); `Q046` the arms do not reach the join in the same custody state — the conditional leak, the conditional double-release, and the shape where one arm moves what the other releases. The join is **agreement between the arms that survive it, never widening**: both arms are validated from the same incoming state, an arm that returns discharged at its own exit and is not compared, and where two arms fall through the branch is admitted only if their ordered live lists and their transferred sets are equal. Card `IF-ARMS` changed that rule's *shape* without weakening it — the complete characterisation is that the only combination with no joined state is two surviving arms that disagree, so nothing that refused at the join before is admitted at it now. A name written in a condition that no `let` in the body bound is collected as a referenced name and refuses earlier at `Q013`; that is fail-closed and recorded, not `Q045`. An arm that acquires is refused by the pre-existing ownership join at `Q103` (two arms leave different resource-identity ledgers), which cards `IF-STATEMENT` and `IF-ARMS` both inherit rather than weaken; card `IF-ARMS` moves only that refusal's **span and wording** — onto the acquisition itself, naming the arm and the identity, and distinguishing an acquisition the reader wrote in an arm from one a hoisted continuation placed there — which is an attribution fix and not a code change. Follow-on `IF-ARM-ACQUIRE`. |
| `Q047`–`Q049` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | **Second band of the same pass** (§L14.3, card `NATIVE-LIBRARY-CONTRACT-SLICE`): the manifest's **contract slice** entries — the trust artifact of level L3 of the linkage spectrum (`docs/design/UNSAFE_LINKAGE_V1.md` §1.1), where a library's pointer symbols belong; L3's single-name *selection* stays follow-on `NATIVE-LIBRARY-SOURCE-BINDING`. The band is not contiguous with `Q039`–`Q042` because `Q043`–`Q046` were allocated between the two cards; a pass owning two ranges is the honest record of that order, and no shipped code was moved to make it look tidier. `Q047` the contract slice binding does not resolve against the selected contract environment, in either direction of design §4.1 — an entry whose symbol the environment does not declare, an entry naming a module other than the one that declares it (the reference is compared exactly, never resolved by bare symbol name), a declaration of a module in the slice that no entry binds, or a selection that binds contracts with no contract environment selected at all; `Q048` a **contracted** declaration and the manifest that binds it disagree in arity, a parameter or the result — the declared contract is **withdrawn for the analysed unit**, its name is not seeded into the admitted environment, and every call site falls to `Q023`, so this diagnostic and that refusal are both published. Deliberately the *opposite* disposition from `Q041`, from one rule: withdraw only where the weaker admissible reading refuses. `Q041`'s weaker reading is the bare waiver, which is accepted; this one's is the uncontracted callee, which refuses, so withdrawal is already the monotone direction and `LOADER-ABI`'s per-unit withdrawal (constraint 8) is the right shape. A withdrawal here can only turn an admission into a refusal; `Q049` a published certificate does not carry **both** stacked premises of a manifest-bound contracted callee — the declared-contract premise of `Q022`–`Q027`, unchanged, and the artifact premise naming the contract module and the build. The second stacks with the first and never replaces it, which is the one deliberate asymmetry with `Q041`'s level, where the sharpened waiver does replace the bare one. A waiver for a name the manifest binds to a contract has **no code of its own and needs none**: `Q047` implies the environment declares that name, and `Q034` already refuses a waiver for exactly that — measured, not assumed, because an unreachable diagnostic code is a claim a reader cannot check. Selecting no manifest, or one that binds no contract, is the default and changes nothing: `Q022`–`Q027` and `Q039`–`Q042` are unchanged and every unmanifested program is byte-identical. |
| `Q050` | `ComposedTasks` | `qcore-frontend`, `qcore-verification::driver` | The **join** of the checked composition of `source.std-io.v1` with the checked **task** route in one module (`source.std-io-tasks.v1`, card `COMPOSE-CONCURRENCY`). One code, not a band, and deliberately so: a composed task module receives every task-route refusal from the unedited task checker at that pass's own `Q600`, and every `source.std-io.v1` shape refusal from that route's own grammar at `Q030`, because the composition runs the task route's unedited lowering and `check_task_module` and reads the line-producing statements with `source.std-io.v1`'s own parser, and only then checks where they meet. A separate pass from `ComposedStdIo` on purpose: that pass joins two straight-line models whose rendered numbers are **declared** at the site, while this one joins a branching, concurrent model whose numbers the task model **computes** at the rendering's own step — which is why `StdIoProgram::scalars` cannot carry them and why the two compositions are not one pass. `Q050` owns exactly five rules, none of which either composed model can state. (1) **Placement.** A line stands in the **parent's** body, or in the body of a function the entry spawns as a worker, and nowhere else. *The parent half* is a measurement of the emitted runtime, not a guess about scheduling: every **trace** write an emitted task program performs is performed by the thread whose context identity is `0`, because a worker's trace events are appended to that worker's **private** array and replayed by the parent inside its `join` after a successful native join, and a worker never starts or collects a task because the lowering admits leaf workers only. So a line placed at any point of the parent has a determined position relative to every trace line, and the conservative reading — before any spawn, between joins, after all joins — is *subsumed*: every parent point is join-ordered, because a worker's output does not exist as bytes until its parent replays it. Such a program's streams are compared **byte for byte**, unchanged. *The worker half* was refused by card `COMPOSE-CONCURRENCY`, which named its own repair: a worker's line is written by that worker's own thread, concurrently with the parent's and with its siblings', so it needed "the F02 comparison-up-to-an-admitted-reordering treatment, or `STD-LOGGER-ASYNC`'s declared queue policy". Card `OBS-REORDERING` landed the first of those as `qcore.obs-reordering.v1` (`crates/qcore-core/src/obs_reordering.rs`, normative statement `docs/design/OBS_REORDERING_V1.md`), and a worker line is therefore admitted. **Placing one is what declares the relation**: the certificate reports `admits_reordering`, and that program's streams are compared window by window and emitter by emitter — a window boundary being a point of the run at which no worker is live, and an emitter being the thread that wrote the bytes, which is the *opposite* of F02's causal attribution and right for the opposite reason. A program that places no worker line declares nothing and keeps the byte comparison, which is that relation's own degenerate case rather than a second rule. A line in a function that is neither the entry nor a spawned worker stays refused: nothing in this profile makes such a function run, so its line has no thread to attribute it to and no window to place it in. (1a) **A worker that writes declares `io` itself.** The effect discipline of this model is per function and a worker body is a function body, so the worker's own `effects { … }` row must admit `io` in addition to the entry's. (2) **Position.** A line stands at a position of the block it was lowered into, and two lines in one block are performed in the order they were written — otherwise a projection would have been checked against a program the emitted C does not perform. (3) **Operand.** A rendering operand is an integer literal or a name the **task** model bound to an `i32` — a copied parameter, a `let`-bound literal, the number a `qcore_buffer_read` computed out of a live buffer, or the outcome a `join` collected. Whether that name may be *read* at that position is **asked of the task checker in the task model's own language**: an ordinary `BindScalar` copy of the scalar is inserted at exactly the rendering's position and `check_task_module` is re-run, so the composition owns no liveness rule and the must-fact bar of `BORROW-LIVENESS` is met by that checker's own all-path check. A name denoting an owned Buffer, a read loan or a task handle is refused here: neither operand form has a constructor that could carry one, so a rendering cannot consume, release, lend or retain custody. A `borrow NAME` rendering is **refused on this route** — `STD-FMT-BORROW` admits one where a model publishes a ledger the join can read, and the task ledger's parent/child custody edges, share arithmetic and join discipline are a different question with its own custody argument to make. (4) **Effects.** The derived row is `source.std-io.v1`'s own, over the composed projection: a program that writes or flushes performs `Io`, and the entry must declare it (and a writing worker its own row too, rule 1a). Off this route `io` is still the unsupported task effect it has always been, refused at `Q020`, so a spawning module that declares `io` and writes nothing is unchanged. Also the code of the composed certificate's own recheck failure. Route selection runs this composition **after** the composed-epoch, epoch and mutex routes and **before** the task route it composes, and its selector is the conjunction `has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`, so a module carrying an epoch scope or a mutex generation keeps that more specific model's refusal, a module that extern-calls **and** spawns keeps the task route's `Q600` (the exclusion `STD-IO-COMPOSE` recorded, deliberately not lifted here), and a module using one of the two composed feature sets falls through to that route byte for byte. |
| `Q051`–`Q057` | `StandardBuffer` | `qcore-frontend`, `qcore-verification::driver` | Admission of `source.buffer.v1` — the `std.buffer` bundled module's **owned, extent-carrying byte buffer** (card `STD-BUFFER`), the source-level tier of the two `docs/design/STD_BUFFER_V1.md` §1 distinguishes. A separate pass from `StandardIo` on purpose: that pass admits a closed set of statements with no resource and no custody at all — its operand type structurally cannot denote one — while this pass admits a resource acquisition, an indexed access into it and its release, so the two could not share a band without the range ceasing to describe what the pass decides. It is likewise separate from `Ownership`, whose `Q103` it deliberately does not emit even though **every custody refusal on this route is `qcore_core::ownership`'s**: the unedited cycle-3 checker is *inherited* here (the `PIPE-CHAIN` "inherit, don't reimplement" precedent) rather than run as the sequential resource-flow bundle runs it, so a reader following `Q054` back finds this route's lowering and not that bundle. The refusal is the same one; the attribution says which route asked. `Q051` the flat entry shape this profile models is violated — a statement outside the grammar, a nested block, a construct outside the entry, a parameterised entry, a malformed `bytes(…)`/`extent(…)`/byte access, a missing or repeated return, or a malformed `effects` row; `Q052` an operand that is not a **check-time constant** this entry bound, or a custody position naming no buffer. This is the decidability rule, and it is load-bearing rather than conservative: the route's evidence is a byte comparison between a reference execution and a native run, so a value the reference interpreter cannot compute is one it cannot predict a line for, and a run-time-varying index would leave *the* observation ill-defined (follow-on `STD-BUFFER-VARYING-INDEX`, whose missing capability is a relational domain over indices); `Q053` a bounds or range violation — an index outside `[0, extent)`, a stored byte outside `0..=255`, or a declared extent outside `1..=4096`. Each is a **refusal, not a run-time trap**: every index is a constant this checker decides before emission, and the emitted C's bounds comparison is a *witness* of that decision which is provably never taken. Out-of-range values are refused, never truncated; `Q054` a refusal reported by the inherited ownership checker — a leak at normal exit (`NormalPathLeak`), a double release (`DoubleDrop`), an access after release (`UseAfterDrop`) or a rebound live name (`BindingAlreadyExists`). Only names an acquisition binds contribute an operation to that checker, because it is asked about **resources**; a name that is not one is refused at `Q052` instead, which says the true thing; `Q055` a buffer at a position this route refuses to let it cross — a rendering operand (bare or `borrow`, the latter for `STD-FMT-BORROW`'s own structural reason: this route publishes no custody identity), an index, or a stored byte. Passing a buffer to a **declared extern callee** needs no code here and has none: route selection places this route *after* the extern route, so such a module keeps that route's existing refusal — measured, not assumed. That is the v1 answer to whether a buffer crosses the C04 boundary, and it is **no**, because a C04 pointer is an extentless `u8` projection and the crossing would discard the extent this route carries and replace it with a premise (follow-on `STD-BUFFER-EXTERN`, whose prerequisite is `NATIVE_LIBRARY_LINKAGE_V1.md`'s stage (b)); `Q056` the published certificate failed its independent rederivation; `Q057` the entry performs a derived effect its declared `effects { … }` row does not admit. `Q057` is deliberately **this pass's own code and not a reuse of `StandardIo`'s `Q029`**: §L21's allocation rule is that a code may not be reused by a second semantic pass, and the conditions differ besides — `Q029` reports an undeclared `Io` on a route that can derive nothing else, while `Q057` reports an undeclared `Alloc` or `Mut` as well. The route is selected after every concurrent route and after the extern route, and **before** `source.std-io.v1`, so a module that both buffers and prints reaches it and its line-producing events are that route's own — read by its reader, rendered by its emitter — while a module that only prints falls through byte for byte. A module that buffers and spawns never reaches it and keeps its `Q600`. The derived effect row carries `Alloc` and `Mut`. `source.extern-call.v1` derives those too, but from a **declared contract**'s projections — `Alloc` when the contract says the callee acquires — whereas here they follow from an acquisition and a store this route checks and discharges itself, so they are derived facts rather than consequences of a premise. |
| `Q050` | `ComposedTasks` | `qcore-frontend`, `qcore-verification::driver` | The **join** of the checked composition of `source.std-io.v1` with the checked **task** route in one module (`source.std-io-tasks.v1`, card `COMPOSE-CONCURRENCY`). One code, not a band, and deliberately so: a composed task module receives every task-route refusal from the unedited task checker at that pass's own `Q600`, and every `source.std-io.v1` shape refusal from that route's own grammar at `Q030`, because the composition runs the task route's unedited lowering and `check_task_module` and reads the line-producing statements with `source.std-io.v1`'s own parser, and only then checks where they meet. A separate pass from `ComposedStdIo` on purpose: that pass joins two straight-line models whose rendered numbers are **declared** at the site, while this one joins a branching, concurrent model whose numbers the task model **computes** at the rendering's own step — which is why `StdIoProgram::scalars` cannot carry them and why the two compositions are not one pass. `Q050` owns exactly four rules, none of which either composed model can state. (1) **Placement.** A line stands in the **parent's** body and nowhere else. That split is a measurement of the emitted runtime, not a guess about scheduling: every write an emitted task program performs is performed by the thread whose context identity is `0`, because a worker's trace events are appended to that worker's **private** array and replayed by the parent inside its `join` after a successful native join, and a worker never starts or collects a task because the lowering admits leaf workers only. So a line placed at any point of the parent has a determined position relative to every other line, and the conservative reading — before any spawn, between joins, after all joins — is *subsumed*: every parent point is join-ordered, because a worker's output does not exist as bytes until its parent replays it. A line inside a worker body would be written by that worker's own thread, concurrently with the parent's and with its siblings', and no checked model in this revision orders them, so the per-stream byte comparison this route rests on would compare against something that is not a function of the program; it is refused. Buffering worker lines the way the trace events are buffered is the recorded future path and needs the F02 comparison-up-to-an-admitted-reordering treatment, or `STD-LOGGER-ASYNC`'s declared queue policy. (2) **Position.** A line stands at a position of the block it was lowered into, and two lines in one block are performed in the order they were written — otherwise a projection would have been checked against a program the emitted C does not perform. (3) **Operand.** A rendering operand is an integer literal or a name the **task** model bound to an `i32` — a copied parameter, a `let`-bound literal, the number a `qcore_buffer_read` computed out of a live buffer, or the outcome a `join` collected. Whether that name may be *read* at that position is **asked of the task checker in the task model's own language**: an ordinary `BindScalar` copy of the scalar is inserted at exactly the rendering's position and `check_task_module` is re-run, so the composition owns no liveness rule and the must-fact bar of `BORROW-LIVENESS` is met by that checker's own all-path check. A name denoting an owned Buffer, a read loan or a task handle is refused here: neither operand form has a constructor that could carry one, so a rendering cannot consume, release, lend or retain custody. A `borrow NAME` rendering is **refused on this route** — `STD-FMT-BORROW` admits one where a model publishes a ledger the join can read, and the task ledger's parent/child custody edges, share arithmetic and join discipline are a different question with its own custody argument to make. (4) **Effects.** The derived row is `source.std-io.v1`'s own, over the composed projection: a program that writes or flushes performs `Io`, and the entry must declare it. Off this route `io` is still the unsupported task effect it has always been, refused at `Q020`, so a spawning module that declares `io` and writes nothing is unchanged. Also the code of the composed certificate's own recheck failure. Route selection runs this composition **after** the composed-epoch, epoch and mutex routes and **before** the task route it composes, and its selector is the conjunction `has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`, so a module carrying an epoch scope or a mutex generation keeps that more specific model's refusal, a module that extern-calls **and** spawns keeps the task route's `Q600` (the exclusion `STD-IO-COMPOSE` recorded, deliberately not lifted here), and a module using one of the two composed feature sets falls through to that route byte for byte. |
| `Q063`–`Q064` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | **Third band of the same pass** (§L14.4, card `NATIVE-LIBRARY-HEADER`): the **exported header** a `source.native-library.v1` manifest binds, read as a real `.h` instead of being believed as the manifest's transcription of one. The band is not contiguous with `Q039`–`Q042` or `Q047`–`Q049` because `Q043`–`Q046` and `Q050` were allocated between the cards; a pass owning three ranges is the honest record of that order, and no shipped code was moved to make it look tidier. `Q063` the header binding does not load — the named header is not supplied, its measured digest is not the declared one, it carries a preprocessing directive v1 does not interpret or conditional regions this reader cannot bracket, or the declaration of a symbol the manifest binds cannot be read: undeclared, declared only inside a conditional region v1 evaluates no condition of, defined as a macro, declared `static` (internal linkage is not an exported symbol of an artifact, which is a statement about the library rather than a reader boundary), declared with an empty parameter list (C11 6.7.6.3p14 specifies nothing, and a manifest entry always asserts a parameter vector), declared variadic, declared twice with readings that disagree, carrying a type this reader does not commit to, or carrying a type it **does** read for which the manifest's ABI vocabulary has no spelling (`fnptr`, `f32`, `f64` — reported apart from unreadable, because a vocabulary gap and a reader gap have different repairs); `Q064` a manifest entry's transcribed declaration and the exported header's own text disagree in arity, a parameter or the result — a **refusal**, deliberately the same disposition as `Q041` and the opposite of `Q048`, from the one rule all three rest on: withdraw only where the weaker admissible reading refuses, and here it is the manifest *without* a header, which is accepted. Both sides are load inputs, so there is no per-unit reading a withdrawal could apply to. A publication tripwire on the `Q049` model was considered and **not** allocated: the header identity is a field of the residence record `Q049` already requires, so a certificate carrying the residence and losing the header cannot be constructed, and an unreachable diagnostic code is a claim a reader cannot check. Card `NATIVE-LIBRARY-HEADER-DECORATION` extends `Q063` with §L14.4.1's **declared include texts** — one that is not supplied, whose measured digest is not the declared one, or that no `#include` reaches; and, once a header is bound, a text supplied for a reference no selected manifest declares — and with the decorations that card's derivation does **not** establish, which keep the refusal they already had. **No new code was allocated for it**: those are the same condition (the header binding does not load) reached by a new route, and a code that duplicated an existing one would make two names for one fact. Naming no header is the default and changes nothing: `Q039`–`Q042` and `Q047`–`Q049` are unchanged and every manifest written before these cards is byte-identical. |
| `Q058`–`Q062` | `Event` | `qcore-frontend`, `qcore-verification::driver` | Admission of the `std.event` statements (`source.event.v1`, card `STD-EVENT`; design record `docs/design/STD_EVENT_V1.md`). A separate pass from `Concurrency` on purpose: that pass owns the checked *task* and *mutex* profiles, whose subject is a thread, while this one admits a single-threaded reactor context and the custody edge a registration puts on a descriptor, and decides a **determinism** question no concurrency profile asks. The route's custody half is **not new**: the projected program is handed to the existing `qcore_core::event_reactor::check_event_program` unmodified, so a refusal at `Q060` is that model's own, attributed to the statement whose projection named it. This card is what makes that model reachable from source at all, retiring the recorded **U3 source-unreachability blocker**. `Q058` a violation of the flat entry shape this profile models, and the generic presentation code of the pass — one event context per entry opened before every other event statement and closed after all of them, every event statement in the entry, no nested block, exactly one `return INTEGER;` last, and a statement head the route does not model; also the code of the certificate's own recheck failure and of an entry-free module on this route. `Q059` a name that is not a live endpoint or registration of this entry — the route's grammar has **no value vocabulary at all**, so every name it resolves is a custody name and there is no operand form that could carry a scalar, a buffer or a handle. `Q060` the reactor model refused the projected program, carrying that model's own `L00`–`L12` diagnostic verbatim. `Q061` the **determinism boundary**: the declared readiness of a dispatch turn is not the one this program induced. A certificate compares one observation, so v1 waits only on readiness the program itself caused with `wake D;`, and `dispatch ready E;` / `dispatch idle;` declare what the turn reports — a declaration the emitted program then **verifies** against what `poll(2)`/`WSAPoll` actually reported, exiting non-zero on disagreement, so the named platform premise `EVENT-V1-LEVEL-TRIGGERED-READINESS` produces a failed run if it is false rather than a wrong verdict. Within `Q061` one thing is refused **by name**, with its ground in the message: a turn at which two or more registrations are ready. `poll(2)` reports a *set*, and the order a reactor runs one ready set's callbacks in is the implementation's rather than one the program induced — `event_reactor::explore_activation_orders` is documented "equal-priority events are deliberately nondeterministic", and every v1 registration carries priority `0` precisely so that a priority order is not smuggled in as an undeclared ordering premise. Covering such a turn waits on comparison up to an admitted reordering; this boundary does not depend on that capability and composes with it later. `Q062` the declared `effects { … }` row does not admit the derived one, which is exactly `{Io}` for any program that opens a context and empty otherwise — `Alloc` and `Mut` are unreachable by construction, for want of a value vocabulary rather than by a rule. A missing `import std.event;` is `Q015`, reused as `STD-MODULES` reuses it, with the message naming the module. A `write` interest is **accepted and checked** and makes the module emit no C and publish no reference observation (the `accept_check_only` precedent): the checking is identical, and only the execution is outside v1's boundary, because a write registration's readiness is a function of the kernel send buffer that no statement of this route induces. Route selection runs this route after every concurrent route and after the extern route, and **before** the standard-IO route, on the principle those routes state about each other: a module that both dispatches and prints stays here and keeps this route's refusal for the statements it does not model. The band is `Q058`–`Q062` because `Q050` was the highest code allocated on this card's base; a collision discovered at integration relocates the unshipped band per the wave-3 `T07`/`T09` precedent, and the code-to-condition mapping above is stable under such a shift. |
| `Q063`–`Q064` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | **Third band of the same pass** (§L14.4, card `NATIVE-LIBRARY-HEADER`): the **exported header** a `source.native-library.v1` manifest binds, read as a real `.h` instead of being believed as the manifest's transcription of one. The band is not contiguous with `Q039`–`Q042` or `Q047`–`Q049` because `Q043`–`Q046` and `Q050` were allocated between the cards; a pass owning three ranges is the honest record of that order, and no shipped code was moved to make it look tidier. `Q063` the header binding does not load — the named header is not supplied, its measured digest is not the declared one, it carries a preprocessing directive v1 does not interpret or conditional regions this reader cannot bracket, or the declaration of a symbol the manifest binds cannot be read: undeclared, declared only inside a conditional region v1 evaluates no condition of, defined as a macro, declared `static` (internal linkage is not an exported symbol of an artifact, which is a statement about the library rather than a reader boundary), declared with an empty parameter list (C11 6.7.6.3p14 specifies nothing, and a manifest entry always asserts a parameter vector), declared variadic, declared twice with readings that disagree, carrying a type this reader does not commit to, or carrying a type it **does** read for which the manifest's ABI vocabulary has no spelling (`fnptr`, `f32`, `f64` — reported apart from unreadable, because a vocabulary gap and a reader gap have different repairs); `Q064` a manifest entry's transcribed declaration and the exported header's own text disagree in arity, a parameter or the result — a **refusal**, deliberately the same disposition as `Q041` and the opposite of `Q048`, from the one rule all three rest on: withdraw only where the weaker admissible reading refuses, and here it is the manifest *without* a header, which is accepted. Both sides are load inputs, so there is no per-unit reading a withdrawal could apply to. A publication tripwire on the `Q049` model was considered and **not** allocated: the header identity is a field of the residence record `Q049` already requires, so a certificate carrying the residence and losing the header cannot be constructed, and an unreachable diagnostic code is a claim a reader cannot check. Naming no header is the default and changes nothing: `Q039`–`Q042` and `Q047`–`Q049` are unchanged and every manifest written before this card is byte-identical. |
| `Q065`–`Q067` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | **Fourth band of the same pass** (§L14.5, card `NATIVE-LIBRARY-SOURCE-BINDING`): the **source file's own** `extern library NAME;` binding, which stops manifest resolution being by symbol name across the whole selection and makes it a statement the file wrote. The band follows `Q063`–`Q064` immediately; the pass now owns four ranges, which is the honest record of the order the cards landed in, and no shipped code was moved to make it look tidier. The **spelling mints no reserved word** (constraint 14): `extern` has introduced a declaration since `Q022`–`Q027` and `library` has been a source word since `module library;`, and what is new is the two-word sequence at declaration position, which was a `Q000` parse failure before this card. `Q065` a file's binding list does not resolve against the selection — a library name no selected manifest declares, a name two or more of them declare (the binding would not be a function, the ground `Q040` refuses its own ambiguity on), one name bound twice in a file, or a binding made with no manifest selected at all. Total or reject, exactly as the manifest load, the header binding and the contract slice are: a selection that resolves partially would silently narrow what the file was told it bound; `Q066` a symbol this file declares is bound by a selected manifest this file did **not** bind. A **refusal**, not a fall-through to the weaker premise, and the disposition rule is `Q041`'s read from the other side: the weaker reading here is the file's premise *without* an artifact, which is accepted, so falling back would let a file stop its certificate naming a build by binding a different library. Its contracted mirror is the same code at the same condition, placed before the `Q042` gate. A symbol **no** selected manifest binds is untouched — absence of evidence is not evidence, and the two absences are different; `Q067` the publication tripwire on the `Q049` and `Q035` model: a certificate that lost a source-binding premise between derivation and publication is refused rather than published. Unlike `Q063`–`Q064`'s considered-and-declined tripwire, this one is not structurally impossible — the binding premise is carried beside the callee premises rather than inside a record another gate already demands. Declaring no binding is the default and changes nothing: `Q039`–`Q042`, `Q047`–`Q049` and `Q063`–`Q064` are unchanged, and every program written before this card resolves selection-wide and publishes the same certificate byte for byte. |
| `Q068`–`Q073` | `StandardString` | `qcore-frontend`, `qcore-verification::driver` | Admission of `source.string.v1` — the `std.string` bundled module's **immutable UTF-8 `{ptr, len}` string view** (card `STRING-SLICE`, normative statement `docs/design/STRING_SLICE_V1.md`). A separate pass from `StandardBuffer` on purpose, and the separation is the amendment's own §5: that pass admits an **owned, mutable, explicitly released** `{identity, extent, custody}` and every one of its refusals is either a custody rule or a rule about writing a byte, while this pass admits an **immutable, custody-free view of storage it does not own**, builds no ownership operation list at all, and decides two questions no buffer rule can state — whether a quoted token decodes to well-formed UTF-8, and whether a slice bound falls on a code-point boundary. It is likewise separate from `StandardIo`, whose `Q029` it does not reuse for `Q073`, on the rule that a code may not be reused by a second semantic pass. `Q068` the flat entry shape this profile models is violated — a statement outside the grammar, a nested block, a construct outside the entry, a parameterised entry, a malformed literal, `len(…)` or slice, a type ascription naming something other than `string`, a missing or repeated return, or a malformed `effects` row; also the generic presentation code of the pass, the code of an entry-free module on this route, and the code of the certificate's own failed rederivation. `Q069` a slice bound that is not a **check-time constant** this entry bound, or a view position naming no string. This is the decidability rule, and it is load-bearing rather than conservative for the same reason `Q052` is: the route's evidence is a byte comparison between a reference execution and a native run, so a value the reference interpreter cannot compute is one it cannot predict a line for. It is also what lets the profile take the **first** of the two arms amendment §2 permits — decide statically, rather than represent checked failure — totally, with no run-time failure arm anywhere (follow-on `STRING-SLICE-VARYING-INDEX`). Its not-a-constant arm is checked but **unreachable from a single source module**, because name resolution refuses an unbound name at `Q013` first; recorded as `STRING-SLICE-NOTASCALAR-REACH` and kept rather than deleted, on the `EXTERN-F1-STORE-REACH` precedent. `Q070` a quoted token this route cannot decode into a payload — an escape v1 does not define (`\\`, `\"`, `\n`, `\t`, `\r`, `\0` and `\xHH` are the whole set), a truncated `\x`, a trailing backslash, decoded bytes that are not well-formed UTF-8, or a payload longer than 4096 bytes. Each is a **refusal**: an ill-formed payload is refused at construction and the value never exists, which is how §3.4's "for the whole lifetime of the view" is discharged, and an over-long payload is refused rather than truncated. `Q071` a slice outside `0 <= low <= high <= len(s)`, or a bound that is in range but is not a UTF-8 code-point boundary. Both are decided at check time, so both are refusals and not run-time traps, and the emitted C's comparison is a *witness* of the decision that is provably never taken. A boundary violation is **never rounded**: rounding would silently slice a different string and not rounding would produce ill-formed text, and §2 forbids both. `Q072` a string at a position this route refuses to let it cross — a rendering operand (bare or `borrow`, the latter for `STD-FMT-BORROW`'s own structural reason: this route publishes no custody identity because it has none) or a slice bound. Passing a string to a **declared extern callee** needs no code here and has none: route selection places this route *after* the extern route, so such a module keeps that route's existing refusal — measured, not assumed. That is the v1 answer to §6's crossing question, and it is **no**, because no C04 pointer contract transports this aggregate or its extent (follow-on `STRING-SLICE-ABI`). The same construction answers §5: the route is selected *after* the buffer route, so a module that does both keeps `Q051` (follow-on `STRING-SLICE-BUFFER-BORROW`). `Q073` the entry performs a derived effect its declared `effects { … }` row does not admit. The derived row **carries neither `Alloc` nor `Mut` for any string operation** — §7 — and `{Io}` only where the entry also writes a line, which is the effect-system form of "zero-allocation, zero-copy". The route is selected after every concurrent route, after the extern route and after the buffer route, and **before** `source.std-io.v1`, so a module that both slices and prints reaches it and its line-producing events are that route's own — over **scalars only**, since this amendment does not make `std.io` or `std.logger` accept text (`STRING-SLICE-IO`) — while a module that only prints falls through byte for byte. The amendment **reserves no word**: `len` is skipped by name resolution only where it is immediately followed by `(` (a global reservation was measured unsafe against the pinned contract corpus, which references a parameter named `len` in an `ensures` clause), and `string` is admitted only in a `let` type ascription, which the reference collector already drains. The band was **allocated as `Q065`–`Q070`** on this card's base, where `Q064` was the highest code, and **relocated to `Q068`–`Q073` at integration** after card `NATIVE-LIBRARY-SOURCE-BINDING` merged first and took `Q065`–`Q067` as `NativeLibrary`'s fourth range. The wave-3 `T07`/`T09` precedent applied as it reads — the still-unshipped band moved, not the shipped one — and the relocation is a pure renumber: same order, same six conditions, same meanings, which is exactly what "the code-to-condition mapping above is stable under such a shift" is for. |
| `Q074` | `Event` | `qcore-frontend`, `qcore-verification::driver` | **Second band of the same pass** (card `COMPOSE-EVENT-LINES`): the composition of `source.event.v1` with the **source line vocabulary**, and with it the one value form this route has. The band is not contiguous with `Q058`–`Q062` because `Q063`–`Q064` were allocated between the two cards, and it is `Q074` rather than the `Q065` this card was written against because **the code was relocated at integration**: `NATIVE-LIBRARY-SOURCE-BINDING` merged first and holds `Q065`–`Q067` as the `NativeLibrary` pass's fourth range, and `STRING-SLICE`'s `StandardString` band took `Q068`–`Q073`. That is the wave-3 `T07`/`T09` precedent doing exactly what it exists for, and the condition below is the one this pass owned before the shift and after it: a relocation moves a number, never a rule. A pass owning two ranges is the honest record of that order, on the `NativeLibrary` precedent, and no shipped code was moved to make it look tidier. **What the composition adds.** A `source.event.v1` entry may carry `print RENDERING OPERAND;`, `log LEVEL RENDERING OPERAND;` and `flush;` — the `source.std-io.v1` statements, read by that route's own reader and rendered by that route's own emitter, so a shape one route accepts and the other refuses is not expressible — and `let NAME = dispatch ready E;`, which is the **same dispatch turn** `dispatch ready E;` is (same reactor projection, same determinism decision, same trace record) naming the byte that turn's callback drained off the endpoint. A `wake D BYTE;` declares the byte it delivers; `wake D;` delivers the route's own, so every program written before this card is byte-identical after it. **`Q074`** is the composition's own refusals and nothing else: a rendering operand that is not a value this entry bound — an endpoint or a registration name is a *custody* name and is refused here rather than rendered as a number; a `borrow NAME` rendering, because this route publishes no live custody identity for a render (the named follow-on is `STD-FMT-DESCRIPTOR`); a `let` that is not that one binding form; a line-producing statement whose shape `source.std-io.v1` refuses; and the model's own value/custody name collision, which no source program reaches because name resolution refuses a duplicate binding at `Q012` first and which is kept for the reason `EXTERN-F1`'s `resource_stored` gate is kept. **Nothing of `Q058`–`Q062` moves.** `Q061`'s two-member refusal, the declared-readiness rule, the `write`-interest `accept_check_only` arm and the reactor's own `Q060` answer exactly as they did: a line erases to no reactor operation and no ledger transition, which is stated as a theorem (`QCore.ComposeEventLines.erasing_the_lines_leaves_the_reactor_projection` and `…_leaves_the_run_unchanged`) rather than as a convention. The derived row stays `{Io}`; `Alloc` and `Mut` remain unreachable, now because the one value form binds a byte the program itself delivered rather than because there is no value form at all. A second named platform premise, **`EVENT-V1-STREAM-BYTE-ORDER`**, is carried **only** by a program that names a drained byte, and like the readiness premise it is *checked* by the emitted program — a byte that is not the one the model predicted exits non-zero rather than rendering a line no model computed. The reference observation of an executable module is the bytes the program writes, per stream: the source-authored lines always, and the retained event trace exactly where `log_level` installs the hosted adapter. Both streams are compared **byte for byte**, and that is this route's measurement rather than a convention: its one accepted reactor profile is single-threaded, so every record is written by the entry's own thread, every window of the derived `qcore.obs-reordering.v1` relation is a barrier and the relation is strict — the degenerate case of that capability rather than a second rule beside it. No reordering is declared, because this route needs none. |
| `Q063`–`Q064` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | **Third band of the same pass** (§L14.4, card `NATIVE-LIBRARY-HEADER`): the **exported header** a `source.native-library.v1` manifest binds, read as a real `.h` instead of being believed as the manifest's transcription of one. The band is not contiguous with `Q039`–`Q042` or `Q047`–`Q049` because `Q043`–`Q046` and `Q050` were allocated between the cards; a pass owning three ranges is the honest record of that order, and no shipped code was moved to make it look tidier. `Q063` the header binding does not load — the named header is not supplied, its measured digest is not the declared one, it carries a preprocessing directive v1 does not interpret or conditional regions this reader cannot bracket, or the declaration of a symbol the manifest binds cannot be read: undeclared, declared only inside a conditional region v1 evaluates no condition of, defined as a macro, declared `static` (internal linkage is not an exported symbol of an artifact, which is a statement about the library rather than a reader boundary), declared with an empty parameter list (C11 6.7.6.3p14 specifies nothing, and a manifest entry always asserts a parameter vector), declared variadic, declared twice with readings that disagree, carrying a type this reader does not commit to, or carrying a type it **does** read for which the manifest's ABI vocabulary has no spelling (`fnptr`, `f32`, `f64` — reported apart from unreadable, because a vocabulary gap and a reader gap have different repairs); `Q064` a manifest entry's transcribed declaration and the exported header's own text disagree in arity, a parameter or the result — a **refusal**, deliberately the same disposition as `Q041` and the opposite of `Q048`, from the one rule all three rest on: withdraw only where the weaker admissible reading refuses, and here it is the manifest *without* a header, which is accepted. Both sides are load inputs, so there is no per-unit reading a withdrawal could apply to. A publication tripwire on the `Q049` model was considered and **not** allocated: the header identity is a field of the residence record `Q049` already requires, so a certificate carrying the residence and losing the header cannot be constructed, and an unreachable diagnostic code is a claim a reader cannot check. Card `NATIVE-LIBRARY-HEADER-DECORATION` extends `Q063` with §L14.4.1's **declared include texts** — one that is not supplied, whose measured digest is not the declared one, or that no `#include` reaches; and, once a header is bound, a text supplied for a reference no selected manifest declares — and with the decorations that card's derivation does **not** establish, which keep the refusal they already had. **No new code was allocated for it**: those are the same condition (the header binding does not load) reached by a new route, and a code that duplicated an existing one would make two names for one fact. Naming no header is the default and changes nothing: `Q039`–`Q042` and `Q047`–`Q049` are unchanged and every manifest written before these cards is byte-identical. |
| `Q1xx` | `Usage`, `Ownership` | `qcore-frontend` | `Q100` usage grade, `Q103` ownership. |
| `Q2xx` | `ResourceFlow`, `CleanupLowering` | `qcore-frontend`, `qcore-verification::driver` | `Q200` resource flow, `Q202` cleanup lowering and reference interpretation. Card `IF-ARMS` **attributes** one of `Q202`'s existing conditions better without changing it: an exit that leaves an owned resource live is `LiveResourcesAtNormalReturn` wherever the exit stands, and an exit inside a branch arm is now reported at that exit's own span rather than at the function declaration — and rather than being pre-empted by a resource-flow join refusal about a net disagreement the exiting arm never had. Same code, same condition, better span; §L21's attribution rule, not a code change. |
| `Q3xx` | `TypedErrors`, `ErrorRegion` | `qcore-frontend` | `Q300` typed errors, `Q302` error region. |
| `Q4xx` | Driver invocation | `qcore-verification::driver` | `Q400` no input files, `Q401` missing or ambiguous entry, `Q403` `--entry` against a library/contract module. |
| `Q5xx` | `CEmission` | `qcore-frontend`, `qcore-verification::driver` | `Q500` a construct the reduced C11 CFG cannot encode. |
| `Q6xx` | `Concurrency` | `qcore-frontend`, `qcore-verification::driver` | `Q600` concurrency and scoped-task checking, and the carried-over structural refusal of every checked concurrency profile (a form its grammar cannot represent). The model profiles occupy disjoint sub-bands within the same pass: `Q610`–`Q627` checked mutex invariants (`CheckedMutexInvariantV1`, of which `Q624` an acquisition out of declared lock-rank order and `Q625` a module that declares ranks for only part of itself, both belonging to the opt-in lock-rank discipline and reached from source by `source.mutex.guard.v1`'s ranked declaration; and `Q626` a protection growth attempted outside an open invariant and `Q627` a protection growth that did not move the declared predicate identity, both belonging to the opt-in protection growth and reached from source by that profile's `grow` statement), `Q630`–`Q641` flat-epoch accounting (`FlatEpochBoundV1`, reached from source by `source.epoch.bound.v1`), `Q650`–`Q667` disjoint footprints (`DisjointFootprintsV1`) and `Q670`–`Q677` error-region task credit (`CheckedRegionTaskCreditV1`). Enumerating them here records existing codes; no code changed. |
| `Q7xx` | `PluginVerification` | `qcore-verification` | Core verifier-scope configuration and dispatch: `Q700` scope resolution failure, `Q701` obligations undischarged, `Q702` conflicting provider evidence or assumptions, `Q703` a provider allegation the core cannot validate. |
| `QP***` | Provider host | `qcore-verifier-host`, `qcore-verification` | Core provider-host, protocol, and evidence-firewall diagnostics (`QP001`–`QP016`). |
| `<plugin-id>.<local-code>` | — | Providers | Provider-owned diagnostics; exact selected plugin ID and one 1–128-byte ASCII local component (letters, digits, `_`, `-`). |

The quiet-emission amendment refines the `Q058` event-shape row above without
allocating or moving a code. An event entry may contain at most one
`log_level LEVEL;`, before, inside, or after its live event context. It requires
`std.logger`, derives `Io`, is recorded and independently rechecked, projects
to no reactor operation, and selects only the hosted system-trace stdout
adapter. Consequently “opened before every other event statement” and “closed
after all of them” apply to reactor statements, not to this entry-level logger
configuration. The same declaration is admitted by the maintained sequential,
standard-I/O, extern-composed, and task-composed hosted profiles under their
existing shape/import/effect codes.

Reserving all four of `Q700`–`Q703` corrects a documentation drift, not a code change: `Q702` and `Q703` have existed in `crates/qcore-verification/src/verify_dispatch.rs` since the W3 dispatch increment, and the code is the authority. An earlier revision of this row named only `Q700` and `Q701`, which was narrower than the implementation; `AGENTS.md` was already accurate.

The `ExternCall` pass is separate from `OrdinaryTypes` on purpose. `Q020` and `Q021` refuse *unenforced source* `contract { … }` blocks and keep exactly the behavior recorded below; the `ExternCall` pass consumes *explicitly selected declared* contracts through the resource-contract v1 loader and never enforces an inline block. Their coexistence is the reason `source.extern-call.v1` is not the inline source-contract consumer those two codes are waiting for.

`Q020` and `Q021` both refuse an unenforced source `contract { … }`, on the task and sequential routes respectively. They are recorded here as **intentionally route-specific and not to be unified** until a source-contract consumer lands: the two gates do not have the same rejection coverage (the task gate also refuses an empty `contract { }`, which the sequential route accepts), so collapsing them to one code would change what is rejected. A code change is semantic; unification is the consumer's work, not a diagnostic-cleanup change.

The Libevent-specific `Q700`–`Q712` table in revision 0.2 is a superseded design allocation, not a second active use of the same numbers. Libevent enum variants now use local L00–L12 codes, emitted under qcore.libevent; the mapping and regression tests are recorded with W3. Both response versions reject nested, empty or malformed local components with QP007 before verdict handling; see the [attribution report](reports/w7-diagnostic-attribution/README.md). Provider messages remain untrusted text. Other diagnostic cleanup remains incremental.

This specification revision does not change `PLUGIN_PROTOCOL_VERSION = 1`, `PLUGIN_EVIDENCE_VERSION = 1`, the resource manifest's `schema_version = 1`, the CLI's package version, or any stored observation schema. Future semantic changes require explicit versioning; changing the prose revision number is not wire compatibility work.

### L21.1 Reason categories for uncovered obligations

Every `Unknown` row carries a free-text `reason` today: `DerivationIncomplete.reason` on a `VerificationTask`, `ChunkSourceResult::Unknown { reason }` from a source importer, and the per-unit `"reason"` field of an audit report. Free text is not queryable and does not distinguish a defect in the analysed source from a failure of the machine analysing it. The following closed vocabulary is normative for classifying such a row. It is **implemented** (2026-09-08, card REASON-CODES): the vocabulary is published as `qcore_schemas::reason`, `DerivationIncomplete` carries a required `category` field named at construction, and audit-report units and source-contract functions carry a classified `reason_code` that fails closed to `null` rather than defaulting a category. The implementation constraints recorded below remain the rules for extending either carrier; the four archived 185-unit artifacts predate the field and move only at CORPUS-REPIN.

| Category | Means | Representative reasons in force |
|---|---|---|
| `unsupported-syntax` | The input is well-formed but contains a construct outside the modeled grammar. The analyser has no model, so the enclosing function stays `Unknown`. | `unmodeled expression CallExpr`, `unmodeled statement DeclStmt`, `unsupported assignment expression`, `unmodeled call, aggregate or indirect access in return expression`, `callback parameter type outside selected Unix profile` |
| `missing-premise` | A declared assumption or trusted input the analysis requires was refused or not supplied. | `caller refuses the observations_trusted assumption required for recorded observation replay`, `required observation hook … has no extern fn contract in the unit's trusted contract sources` |
| `incomplete-obligation-coverage` | A capability has no derivation, or only a partial one, in the current increment. The inventory is honest but not complete, so the capability cannot reach `Safe` through it. | `no core derivation for this capability in evidence v2.0`, `per-allocation-site custody derivation runs in the unrestricted-C gateway`, `source lifecycle effect/ABI import is unavailable` |
| `stale-evidence` | Evidence exists but is bound to a different identity — task digest, raw source digest, or build/profile/policy snapshot — than the subject under check. | witness rejected against a post-deletion task; a changed preprocessing input retiring source-effect witnesses |
| `provider-failure` | The selected provider was absent, failed to respond, returned a malformed or rejected response, or failed attribution at the firewall. | the `QP***` rejection paths |
| `residual-custody` | Analysis completed within its model but left an unresolved custody, containment or leak obligation. | ambiguous effects retaining unresolved custody; residual leaks depending on unresolved effects |
| `environment-unavailable` | The analysis could not run because a required tool, build tree or configuration input was missing or mismatched. | `audit requires the configured Clang executable …; compilation database selects /usr/bin/cc` |

Two rules bind any implementation of this vocabulary:

1. **An `environment-unavailable` row MUST NOT be relabeled as a source defect.** It is a distinct category precisely so that a missing Clang, an absent build tree, a mismatched compile database or an absent submodule can never be folded into `unsupported-syntax` or `residual-custody`, and can never contribute to a provisional `Unsafe`. It says nothing about the analysed source.
2. **Categories refine, never replace, the verdict contract.** A reason category is added alongside the existing `reason` text; it does not alter the source span, `property`, semantic profile, scope, bounds, assumptions, or obligation ID of any row, and golden data MUST show those unchanged across the migration.

Implementation is deferred, and the reason is structural rather than editorial: `DerivationIncomplete` is a field of `VerificationTask`, whose SHA-256 over canonical JSON travels in every evidence-v2 request and every witness, and the record is `deny_unknown_fields`. Adding a category field there changes every task digest, so every shipped provider package and every recorded witness would have to be regenerated in the same change — the opposite of additive. Adding the key to audit-report units instead would change all four pinned 185-unit artifacts byte for byte. Either belongs to its own increment, which must re-derive the pinned artifacts deliberately and say so.

## L22. Required conformance and regression evidence

The implementation matrix MUST track stages separately: parsing, semantic checking, reference execution, CFG/lowering, native execution, and proof. One checked IR test does not complete a source-language CT case. One passed platform run does not cover another target or backend.

The current CT manifest has eleven mapped cases—CT01, CT02, CT12, CT13, CT14, CT15, CT16, CT19, CT20, CT37, and CT50—and 39 unmapped cases. CT01 executes owned Buffer transfer/return/join/drop; CT02 rejects actual parent post-move access; CT12 executes creation failure with original Buffer restoration. CT13 has Linux native creation-recovery and retained-sibling evidence; CT15 has Linux native join-failure evidence; CT19 checks actual sibling-failure source; CT20 checks sibling join/result discharge before bound-error rethrow. CT16 executes native worker allocation/error cleanup. CT14 and CT37 force actual worker-body completion before creation returns, check retained completion storage and require explicit join/result consumption; pre-join parent use and discarded owned results reject. CT50 executes real unmap/remap at an identical address with distinct Buffer generations and rejects stale source bindings. The earlier CT16/CT37 sequential controls remain tested. Individual stage/target statuses remain authoritative; mappings do not imply fully completed end-to-end proofs. The current corpus contains 58 `.qcore` modules and a five-module resource-v1 selection; the original reviewed archive contained 56 modules.

The supplied archive review reported 39 Python evidence-validator tests and no Rust/Lean/native execution; its Libevent directory was empty. This repository integration has complete pinned inputs and separately executes Rust, Lean, Python and the configured audit. Current results are recorded in [VALIDATION.md](VALIDATION.md) and the acceptance ledger. Historical runs are never substituted for fresh checks.

Before claiming revision-0.3 conformance, the acceptance ledger requires regression coverage for destroyed-generation access, post-join events, mutex-bound authority, ordinary source result types, unconditional axis checking, exact-versus-at-most loop custody, the signed-minimum literal, complete contract-consumer selection, and evidence-version rejection. New work MUST preserve the existing fail-closed plugin firewall and must not silently promote Cycle 19 raw units.

## L23. Change record

Revision 0.3 adds explicit capability/evidence levels, documents the actual source-to-C subset, specifies separate interpreter grammars, clarifies module composition and closed indices, restores the full flow tuple including entry requirements, separates exact and bounded loop counts, and formalizes residual custody on cleanup failure. It narrows concurrency claims to the implemented event model, records new review gaps, preserves permanent v1 Safe refusal, and gives diagnostics one owner.

The 2026-09-11 string-view amendment adds an immutable UTF-8 `{ ptr, len }`
representation and an allocation-free slicing rule, implemented the same day by
card `STRING-SLICE` as the checked source profile `source.string.v1` (§L04.1,
diagnostics `Q068`–`Q073`). It widens exactly one profile and no other: mutable
`{ ptr, len, cap }` byte slices, buffer conversion, output composition, and
external ABI transport remain named follow-ons, and the crossing of a C04 or
native-library boundary remains refused.

The supplied review changed documentation only. This repository integration also tracks separately tested implementation repairs in the acceptance ledger. The supplied five files did not include the advertised probes or validation bundle; no execution of missing artifacts is claimed. Historical evidence is preserved.
