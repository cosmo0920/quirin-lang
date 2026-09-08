# QCore language specification — revision 0.3

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

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
| `source.exec12` | `compile_files`, `qcorec check`, `qcorec emit` | Flat artificial resource operations, literal returns, and restricted direct calls. |
| `source.tasks.copy.v1` | `compile_files`, `qcorec check`, `qcorec emit` | Linux C11 threads or the emitted macOS/Windows compatibility subset; finite scalar-copy leaf workers, explicit joins, checked creation/error paths; no resources, loans or shared source memory. |
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

A module header is optional, unique, and precedes declarations:

```qcore
module executable;
module library;
module contract;
```

The three lines above are alternatives, not a valid three-header file. An absent header means executable mode. Contract and library modules do not select or synthesize a process entry. A function named `main` in those modes is an ordinary definition. Current non-executable modes do not produce C library objects or establish a public ABI.

Multiple files share one namespace. Executable mode wins if any file is executable; otherwise library wins if any file is a library; otherwise the compilation is a contract compilation. This is not an import system or namespace hierarchy. Duplicate declarations remain errors. `--entry` without any executable input is rejected with `Q403`.

In executable mode the driver honors an explicitly selected entry, otherwise selects `main` or its documented single-function fallback. The emitted entry MUST have no parameters. Unsupported or ambiguous entry selection is an error. All supported source function bodies, including uncalled definitions, are submitted to the source checks.

The current command owner is the `qcorec` package:

```sh
cargo run -p qcorec -- check path/to/module.qcore
cargo run -p qcorec -- emit -o output.c path/to/program.qcore
cargo run -p qcorec -- plugins list
```

The historical `cargo run -p qcore-compiler --bin qcorec` spelling is obsolete after the CLI split.

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

A second profile, `CheckedMutexInvariantV1`, discharges the guard-footprint and synchronized-handoff premises inside its own bounded model over supplied traces. Every mutex generation declares a fixed protected footprint at creation and consumes the creator's full share over it, so a protected footprint has exactly one protector and no fallback authority; access authority is keyed to a freshly minted guard instance, not to a mutex name; and a custody transfer's synchronization witness is derived from the checker's own happens-before history and compared, never believed. Its certificates MUST NOT carry `GuardFootprintAuthority` or `SynchronizedCustodyEdges`; they retain `DeclaredFootprintShape` and, when a `close` supplies a declared predicate, `InvariantPredicateDeclared`. Condition variables, try/timed locks, reader/writer locks, upgradeable guards, nested protecting mutexes, recursive acquisition, lock ranks and composition with the task-completion capsule are represented refusals, not omissions. `TrustedExplicitTrace` is unchanged and is not retired by this profile.

The source driver does not lower general concurrency syntax. It lowers one bounded surface, `source.mutex.guard.v1` ([G02c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02c-guarded-fixed-footprint-mutex)): a lexical guard over one fixed-footprint generation, on one thread, with no task creation. Every lowered statement is checked by the model above and the driver re-derives the certificate before publishing it. Source-to-trace refinement, the real C layout of the declared intervals, the invariant predicate, deadlock freedom, contention and general native refinement remain outside that result. Fixed Windows execution through the emitted compatibility subset is recorded in [the Windows thread report](reports/windows-c11-threads/README.md).

The `concurrency checked;` marker is not evidence of race verification. The marker alone does not select evidence. The scoped-copy path independently checks its typed task IR; the supplied-event concurrency checker remains a separate API. A future supported source-concurrency profile MUST derive and validate its events, task transitions, access permissions, and synchronization witnesses rather than accepting producer-supplied truth flags.

## L17. Explicit C11 execution candidates

The weak-memory API accepts explicit events and relations such as reads-from, modification order, synchronization, sequential consistency order, and mutex order. Its checker and associated theorems concern that supplied candidate representation and its implemented axioms.

Accepting one candidate does not establish that every execution of a C program is race-free. Rejecting a candidate does not establish that the program is safe either. A whole-program claim requires a justified complete candidate set or another sound all-execution argument, including publication and reclamation conditions relevant to the property.

A provider or source importer MUST NOT manufacture a synchronization relation merely because it makes a candidate pass. Candidate relations must be derived or explicitly declared as assumptions, with the distinction preserved in the evidence record. A `volatile` qualifier is not an independently established synchronization witness in this specification.

## L18. Existing C and interprocedural summaries

The unrestricted-C gateway first invokes Clang with a declared dialect, include paths, and preprocessor definitions. It then projects supported behavior into a conservative resource model. `unrestricted` describes what can be presented to the production C front end, not what the resource analyzer can completely verify.

Unsupported control flow, unresolved alias identities, missing contracts, and incomplete function projection preserve Unknown. An analysis of main-source definitions differs from an analysis of every preprocessed definition; that scope choice MUST travel with the result. A library-specific source importer may support a different subset from the generic gateway and must report its own coverage.

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
| `Q010`–`Q013` | `NameResolution` | `qcore-frontend` | `Q010` duplicate declaration, `Q011` duplicate parameter, `Q012` duplicate local binding, `Q013` unknown name. |
| `Q020`, `Q021` | `OrdinaryTypes` | `qcore-frontend`, `qcore-verification::driver` | `Q020` unsupported or ill-typed ordinary construct, including the emitted entry's parameter rule and the task route's source-contract gate; `Q021` the sequential whole-program gate refusing an unchecked source `contract { … }`. The two contract gates are route-specific and deliberately not unified — see below. |
| `Q1xx` | `Usage`, `Ownership` | `qcore-frontend` | `Q100` usage grade, `Q103` ownership. |
| `Q2xx` | `ResourceFlow`, `CleanupLowering` | `qcore-frontend`, `qcore-verification::driver` | `Q200` resource flow, `Q202` cleanup lowering and reference interpretation. |
| `Q3xx` | `TypedErrors`, `ErrorRegion` | `qcore-frontend` | `Q300` typed errors, `Q302` error region. |
| `Q4xx` | Driver invocation | `qcore-verification::driver` | `Q400` no input files, `Q401` missing or ambiguous entry, `Q403` `--entry` against a library/contract module. |
| `Q5xx` | `CEmission` | `qcore-frontend`, `qcore-verification::driver` | `Q500` a construct the reduced C11 CFG cannot encode. |
| `Q6xx` | `Concurrency` | `qcore-frontend`, `qcore-verification::driver` | `Q600` concurrency and scoped-task checking. |
| `Q7xx` | `PluginVerification` | `qcore-verification` | Core verifier-scope configuration and dispatch: `Q700` scope resolution failure, `Q701` obligations undischarged, `Q702` conflicting provider evidence or assumptions, `Q703` a provider allegation the core cannot validate. |
| `QP***` | Provider host | `qcore-verifier-host`, `qcore-verification` | Core provider-host, protocol, and evidence-firewall diagnostics (`QP001`–`QP016`). |
| `<plugin-id>.<local-code>` | — | Providers | Provider-owned diagnostics; exact selected plugin ID and one 1–128-byte ASCII local component (letters, digits, `_`, `-`). |

Reserving all four of `Q700`–`Q703` corrects a documentation drift, not a code change: `Q702` and `Q703` have existed in `crates/qcore-verification/src/verify_dispatch.rs` since the W3 dispatch increment, and the code is the authority. An earlier revision of this row named only `Q700` and `Q701`, which was narrower than the implementation; `AGENTS.md` was already accurate.

`Q020` and `Q021` both refuse an unenforced source `contract { … }`, on the task and sequential routes respectively. They are recorded here as **intentionally route-specific and not to be unified** until a source-contract consumer lands: the two gates do not have the same rejection coverage (the task gate also refuses an empty `contract { }`, which the sequential route accepts), so collapsing them to one code would change what is rejected. A code change is semantic; unification is the consumer's work, not a diagnostic-cleanup change.

The Libevent-specific `Q700`–`Q712` table in revision 0.2 is a superseded design allocation, not a second active use of the same numbers. Libevent enum variants now use local L00–L12 codes, emitted under qcore.libevent; the mapping and regression tests are recorded with W3. Both response versions reject nested, empty or malformed local components with QP007 before verdict handling; see the [attribution report](reports/w7-diagnostic-attribution/README.md). Provider messages remain untrusted text. Other diagnostic cleanup remains incremental.

This specification revision does not change `PLUGIN_PROTOCOL_VERSION = 1`, `PLUGIN_EVIDENCE_VERSION = 1`, the resource manifest's `schema_version = 1`, the CLI's package version, or any stored observation schema. Future semantic changes require explicit versioning; changing the prose revision number is not wire compatibility work.

### L21.1 Reason categories for uncovered obligations

Every `Unknown` row carries a free-text `reason` today: `DerivationIncomplete.reason` on a `VerificationTask`, `ChunkSourceResult::Unknown { reason }` from a source importer, and the per-unit `"reason"` field of an audit report. Free text is not queryable and does not distinguish a defect in the analysed source from a failure of the machine analysing it. The following closed vocabulary is normative for classifying such a row. It is a **specification, not yet an implemented field**; the implementation constraints are recorded below.

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

The supplied review changed documentation only. This repository integration also tracks separately tested implementation repairs in the acceptance ledger. The supplied five files did not include the advertised probes or validation bundle; no execution of missing artifacts is claimed. Historical evidence is preserved.
