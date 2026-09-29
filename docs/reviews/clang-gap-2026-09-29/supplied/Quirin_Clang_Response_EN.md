# Quirin vs. Clang C-to-LLVM IR — Full English Response

Yes, that understanding is correct. **In Quirin, contracts can supply semantics about resources, external functions, layouts, and other matters that cannot be determined directly from C source. It would therefore be inappropriate to conclude that every piece of information retained by Clang but not by Quirin represents an implementation gap.**

The important distinctions in this comparison are as follows.

| Situation | Assessment |
|---|---|
| An external function's semantics are unknown, but can be expressed using a supported call form and contract vocabulary | **An existing contract can supply them. This is not a parser deficiency.** |
| A contract exists, but cannot be applied in a condition, nested call, or complex reference target | **A gap in expression analysis, contract application, or effect composition** |
| A contract and checker exist, but information from C source does not reach the checker | **A missing connection to an existing checker** |
| The property to be proved is not included in the current contract vocabulary or verification model | **An extension needed to broaden the guarantee** |
| Numerical information or LLVM code-generation machinery unrelated to custody auditing | **Not counted as a defect** |

Using these distinctions, the comparison contains **a 30-row standard C operator matrix and 112 related entries**. The number 112 is not an unimplemented-feature count or a bug count. It includes partial support, precision limits, missing connections, and properties outside the guarantee.

**Materials containing the full inventory and implementation references:**

[Detailed report: Quirin–Clang operation and semantic-model gaps](Quirin_Clang_Gap_Analysis_EN.md)  
[Quirin source evidence with original line numbers](Quirin_Source_Evidence_EN.md)  
[Machine-readable inventory of all 112 entries and 30 operator rows](gap_inventory_EN.json)

The comparison used the attached Quirin archive as its baseline. LLVM's operation and AST taxonomy was pinned to main-branch commit `992ca79c83b0882465a202883109bd4cd167ec00`, as retrieved during the original analysis. Local Clang 17 successfully generated ASTs and LLVM IR for 16 functions, but **the Quirin Rust tests were not run**. Findings about Quirin are based on a static comparison of implementation branches, rejection conditions, and expected outcomes in existing tests. This comparison does not count every LLVM target intrinsic or C++ feature as a missing C feature. See the report's scope section and LLVM references L02–L04.

## 1. The Relevant Comparison Is the Semantic Transformation After Parsing, Not Simply “C Parser vs. emit-llvm”

First, `-emit-llvm` is not an independent C parser. It selects an LLVM output mode. Clang performs parsing and semantic analysis through Sema, then translates expressions, reference targets, calls, control flow, and other constructs from the resulting AST into LLVM IR. [LLVM references: L01, L04.]

```text
Clang:
C → preprocessing / Parser / Sema → typed AST → CodeGen → LLVM IR

Quirin:
C → Clang validation / preprocessing / typed AST
  → projection into the semantics relevant to the audit
  → contract application / state transitions / evidence checking
```

On the Quirin side, two routes must also be distinguished.

| Label | Main implementation | Role |
|---|---|---|
| **G: general C custody-audit route** | `legacy_c.rs`, `typed_access_path.rs`, and supporting readers | Constructs CFGs and resource operations from token analysis and typed-AST assistance |
| **T: bounded typed-source importer** | `typed_source.rs` | Passes typed values, expressions, reads/writes/calls/guards, and related information into source profiles |
| **Individual core verification models** | `disjoint_footprint.rs`, `weak_memory.rs`, etc. | Check the models, contracts, and evidence supplied to them |

**T is not another name for the entire Typed Auditor.** Also, the presence of a verification capability in the core does not necessarily mean that general C source is connected to it.

Ignoring this distinction leads to incorrect claims such as “Quirin has no subtraction,” “no loops,” “no typed intermediate representation,” or “no atomics.”

*Quirin evidence: Q01–Q02 and Q09. In particular, `typed_source.rs:34–157` already contains representations of typed expressions and operations.*

## 2. Differences Confirmed at the Operator Level

Here, “supported” means **the relevant reader can read that expression shape**. It does not mean acceptance for arbitrary types or positions, or proof of numerical safety.

The Clang-side comparison set comes from the AST operation definitions. [LLVM references: L02–L03.]

| Operation / form | G: custody route | T: `typed_source.rs` |
|---|---|---|
| Unary `+`, `~` | Present in the scalar reader | **Absent** |
| Unary `-`, `!` | Present | Expression representations exist |
| Address-of `&` | Limited support in typed paths, actual arguments, etc. | Expression representation exists, subject to place/profile restrictions |
| Dereference `*` | Limited typed scalar-access support | Expression representation exists, subject to type/profile restrictions |
| Prefix/postfix `++`, `--` | Scalar/pointer steps and limited stepped accesses | **Limited to standalone statements for selected integer types** |
| `sizeof` | Checks resource neutrality for fixed types and restricted operands | **Only restricted forms involving fixed profile types** |
| `_Alignof` | Supports fixed scalar queries and related forms | No dedicated support |
| Binary `+`, `*`, `/`, `%` | Readable as scalar expressions; some pointer-step support also exists | Expression representations exist |
| Binary `-` | Present in the scalar reader and limited pointer steps | **Absent** |
| `<<`, `>>` | Present in the scalar reader | **Absent** |
| `<`, `>` | Present | Expression representations exist |
| `<=`, `>=` | Present in the scalar reader | **Absent** |
| `==`, `!=` | Scalar comparisons and checks involving NULL, etc. | Expression representations exist |
| Bitwise `&`, `\|` | Present in the scalar reader | Expression representations exist |
| Bitwise `^` | Present in the scalar reader | **Absent** |
| `&&`, `\|\|` | Condition CFG support, but not every expression position | **Short-circuiting is limited to conditions** |
| `?:` | Limited support in top-level condition contexts and related forms | **Not supported as a general expression** |
| `=` | Separate rules for assignment, conditions, Out, fields, etc. | **Limited to statements** |
| `+= -= *= /= %= <<= >>= &= ^=` | Limited rules for scalar statements, pointer steps, etc. | **Absent** |
| `\|=` | Scalar-statement rules exist | **Limited to `int` statements** |
| Comma operator `,` | Conservative scanning and individual forms exist, but not general expression semantics | **Absent** |
| Function calls `()` | Contracted direct calls plus limited nesting/dispatch | **Only prescribed direct-callee forms** |
| Indexing `[]` | Limited typed-access, collection, and related forms | An `Index` representation exists |
| Member access `.`, `->` | Typed paths and limited tracked stores | **Limited to selected record fields** |
| Casts `(type)` | Rules for scalar casts, restricted pointer returns, etc. | **Only permitted CastKinds** |

**The clearest operator gaps in T are the following.**

```text
Binary operators:
    -  <=  >=  ^  <<  >>

Unary operators:
    +  ~

Compound assignments:
    +=  -=  *=  /=  %=  <<=  >>=  &=  ^=
```

In addition, T has no general branches for `FloatingLiteral` or `CharacterLiteral`. These differ from its integer- and string-literal support.

However, **adding these operators to the match allowlist alone would not complete the work**. Downstream consumers must also agree on whether an expression is custody-neutral, numerically evaluated, or a source of obligations.

*Quirin evidence: Q11, `typed_source.rs:582–796`; Q12, `:798–960`; Q16, `typed_access_path.rs:950–1182`.*

## 3. Expression and Effect-Composition Gaps That Remain Even When Contracts Exist

This is the most important category in the comparison.

Clang's expression lowering handles not only value computation but also short-circuiting, assignment results, conditional evaluation, and reads/writes. Quirin has substantial support for individual cases, but this is not yet a common mechanism across every expression position. [LLVM reference: L05.]

| Missing composition | Current state | Required processing |
|---|---|---|
| **Ownership acquisition inside a condition** | G's condition desugaring rejects Owned, Reallocated, and descriptor results | Construct the acquired identity, assignment, NULL/sentinel branches, and success/failure states together |
| **Nested calls with non-identity effects** | Inner calls are largely restricted to result `None`, parameters `Borrowed`, etc. | Apply the inner acquisition/release/transfer, then pass its result to the outer call |
| **Calls nested two or more levels** | Support centers on one-level whole-argument forms | Recursively elaborate expressions such as `f(g(h(p)))` |
| **Calls occupying only part of an argument** | Forms such as `f(g(p) + 1)` fall outside the restricted rules | Compose the call result with subsequent operations |
| **Ordering of sibling calls and call/read combinations** | Identity-effect restrictions and conservative rejection are used | Check every permitted order, or commutativity established from contract footprints |
| **Value-producing `&&` and `\|\|`** | Supported as condition CFGs | Preserve short-circuiting and result values in assignments, returns, and actual arguments |
| **Value-producing `?:`** | Restricted support in condition contexts | Merge only the selected branch's effects and the resulting alias/custody state |
| **Assignments, prefix/postfix updates, and comma expressions used as values** | Depend on statement handling and individual shapes | Separate value production, updates, evaluation counts, and sequencing |
| **General `_Generic` selection** | G uses invariance when every association has the same core | Use Clang's actual selection result and do not evaluate unselected branches |
| **GNU statement expressions** | Internal-assignment scanning and special forms are separate from general semantics | Compose the inner scope, final value, internal calls, and escape into the outer scope |
| **Evaluated/unevaluated boundaries such as `sizeof`** | Limited non-VLA forms are treated as resource-neutral | Distinguish operands that are evaluated under VLA semantics |

For example:

```c
void *p;

if ((p = malloc(n)) != NULL) {
    free(p);
}
```

What is missing here is not a contract describing the semantics of `malloc` or `free`.

It is **the transformation of the entire expression**:

```text
Apply malloc's contract
  → create a new resource identity
  → bind it to p
  → branch on NULL / nonnull
  → free only on the successful branch
  → merge the states
```

The attached version already includes a test that checks that an acquisition inside a condition is not classified as Safe and that the acquisition call is not hoisted out of the condition.

Similarly:

```c
sink(release_and_status(p));
```

Even when both contracts exist, the inner release effect must be composed with the outer call.

By contrast:

```c
sink2(f(p), g(q));
```

also raises the issue of sibling-argument evaluation order. **The order obtained by simply traversing the AST from left to right must not be adopted as an order guaranteed by C.**

*Quirin evidence: Q19, `legacy_condition_desugar.rs`; Q20, `variadic_nested.rs`; Q14, `typed_source.rs:1315–1410`; Q21, `legacy_generic_selection.rs`; Q47, tests for Owned results inside conditions.*

## 4. Remaining Gaps in Memory and Reference-Target Operations

Here, a contract may describe an object's shape or ownership relationships, but the question is whether **an actual C reference target can be matched to the corresponding object in that contract**.

Clang handles lvalues, aggregates, record layouts, and related concerns through distinct mechanisms. Accordingly, comparing Quirin only on whether it “can read a pointer” is insufficient. [LLVM references: L06–L07, L13.]

| Area | Remaining difference |
|---|---|
| **Mixed tracked paths** | Paths such as `p->items[i].field`, mixing embedded records, indexing, and dereferencing, are not generalized as tracked places. Some are currently accepted when the root is never-bound. |
| **General pointer-valued stores** | `s->p = q`, `*slot = p`, and `slots[i] = p` are not handled uniformly. Limited aggregate stores, Out, and collections already exist. |
| **Field stores through caller-owned objects or aliases** | Current aggregate stores impose strong conditions, such as a local owner acquired in the current function. |
| **Storage involving nested fields, arrays, multilevel pointers, and function pointers** | Generalizing store destinations beyond simple `BASE->FIELD` is incomplete. |
| **Readback from `void *` fields** | Certain stores are supported, but pointee/view information is insufficient for typed readback of the tracked value. |
| **Union loads/stores** | Typed access rejects union owners. The core footprint model also explicitly excludes unions. |
| **Bit-field reads/writes** | C-side typed access rejects them. **The core already has a storage-unit model**, so this includes a missing connection. |
| **Whole-aggregate assignment/copy** | Not generalized as field-wise alias/custody transfer. |
| **General and designated initializers** | T's `InitListExpr` is restricted to forms such as two elements of `struct timeval`. |
| **Compound literals** | Anonymous-object identity, lifetime, and address escape are not handled generally. |
| **VLAs and flexible array members** | Connections among dynamic extents, actual evaluation of bound expressions, and scope lifetime are incomplete. |
| **Bounds of computed indices** | Reading an index is different from proving `0 <= i < count`. |
| **Exact offsets in pointer arithmetic** | Pointer steps exist, but do not generally compute byte offsets as element width × index. |
| **Pointer differences, ordering, and one-past pointers** | Same-array/object conditions and the distinction between formation and dereference ranges are not generally verified. |
| **Pointer–integer conversions and tagged pointers** | There is no general model of cast kinds, widths, information loss, and provenance. |
| **Alignment** | The connection from types/layouts/derived addresses to alignment obligations for general C accesses is incomplete. |
| **Allocation generations and detailed subobject aliases** | Release/realloc tracking already exists. The core generation model is not connected to every C place. |
| **Non-aliasing proofs between separate allocations** | The footprint prototype explicitly rejects disjointness between different allocation identities. |
| **Effective type, type punning, and restrict** | Some routes assume them, some reject relevant forms, and some models are not connected. T also rejects the `restrict` qualifier. |
| **Volatile accesses** | Custody invariance is different from guaranteeing access counts and ordering. |
| **Global/static/thread-local storage** | T rejects global references and locals with storageClass. There is no common model for all storage durations. |
| **Uninitialized values, padding, and object representation** | General read safety does not follow from the current custody verdict Safe. |
| **Byte semantics of memcpy/memmove/memset** | Custody contracts can be written, but do not guarantee every property of overlap, initialized regions, and effective type. |

In particular, the interval model in `indexed_collection.rs` tracks **which elements carry release obligations**. It does **not** guarantee that an index is within an allocated region.

Also, `disjoint_footprint.rs` already checks ranges, bit-fields, and generations using layout contracts. However, that file explicitly states that integration with general production auditing is limited.

It would therefore also be incorrect to characterize all of this as “reimplement the verification theory from scratch.” **A substantial part of the work is delivering the correct C places and accesses to existing core checkers.**

*Quirin evidence: Q16–Q18, Q24–Q25, and Q33. Rejection expectations for `s->p=q` and union reads appear in Q36, `tests/access_path_statements.rs:59–84`.*

## 5. Missing Numerical Semantics — Not Necessarily Required for Custody

As you pointed out, **what is necessary depends on the property being audited**.

For example, if integer multiplication does not change resource state and its result does not affect an audited branch, extent, or contract condition, a custody audit may remain valid without computing the multiplication result.

However, checking bounds or size safety requires the following.

| Numerical-semantic category | Remaining processing |
|---|---|
| Machine meaning of types | Connect integer widths, signedness, ranks, and target-dependent type information to a common domain |
| Promotions and implicit conversions | Give downstream meaning to the integer promotions/usual arithmetic conversions determined by Clang |
| Integer casts | Sign extension, zero extension, truncation, and information loss |
| Integer arithmetic | Signed overflow and unsigned modular arithmetic |
| `/` and `%` | Division by zero and the signed-minimum/`-1` case |
| Shifts | Conditions involving shift counts, widths, and signedness |
| Comparisons and truth-value conversion | Mixed signedness, comparisons after conversion, and integer/pointer/FP truthiness |
| FP conversion and arithmetic | Integer–FP conversion, FP-width conversion, NaN, infinity, signed zero, and rounding |
| The FP environment | Rounding modes, exceptions, fenv, and assumptions such as fast-math |
| Constants and type queries | Actual values of enums, `sizeof`, `_Alignof`, and `offsetof` |
| Extended numerical types | `_Complex`, imaginary types, vectors/matrices, `_BitInt`, and fixed point |

Clang has separate implementations for scalar and complex lowering. Two expressions involving the same binary operator do not necessarily receive the same semantic processing. [LLVM references: L05, L08.]

A particularly important detail in Quirin is that **the core's `scalar_calls.rs` explicitly specifies checked I32 operations, not C wraparound or undefined-behavior semantics**.

Thus:

> The core has addition, subtraction, multiplication, and division  
> ≠ it can audit all C integer types and conversion rules.

Conversely:

> It does not model all numerical semantics of C  
> ≠ its custody audit is unsound.

What matters is **whether the abstraction that discards values preserves the resource property under analysis**.

*Quirin evidence: Q09–Q11, Q22–Q23, Q32, `scalar_calls.rs:1–80`, and Q44.*

## 6. Remaining Gaps in Control Flow, Function Calls, and Contract Vocabulary

### Control flow

Compared with Clang's statement lowering, the question is not just whether a CFG exists, but which source route constructs which edges and scopes. [LLVM reference: L09.]

| Item | Precise assessment |
|---|---|
| General `while`/`for`/`do` in T | **Missing.** T's `for` is limited to canonical scans and `do` to literal `0`. G has loop CFGs. |
| `switch`/`case`/`default` in T | **Missing.** G has them. |
| `goto`/labels/`break`/`continue` in T | **Missing.** G has them. |
| Refining cases using the switch value in G | Conservative processing creates possible edges to all cases. It is not refinement through numerical dispatch. |
| General branch correlations and infeasible-path elimination | Relationships beyond NULL, validity, distinctness, indices, and similar domains are limited. |
| General loop invariants | Specific collection/scan models exist, but not numerical/heap invariants for arbitrary loops. |
| General recursion | Countdown and neutral-frame handling exist. There is no general treatment of arbitrary ranks, multiple ranks, or structural recursion. |
| Computed goto and label addresses | Require a target-set model distinct from direct goto. |
| `setjmp`/`longjmp` | Ordinary call contracts alone do not express nonlocal-return edges and scope invalidation. |
| Cleanup attributes | General composition of implicit scope-exit calls across all exit edges is needed. |
| Inline asm, asm goto, and MS SEH | Require another model/boundary covering memory clobbers and control edges. |
| `noreturn` | T supports standalone terminal calls, but there is no common treatment across all expression positions and exit forms. |

Limits such as T's 64 paths, 512 operations, depth/node limits, and G's alias-set limit of 8 are **precision and coverage limits**. Returning Unknown when a budget is exceeded should not automatically be counted as a soundness defect.

*Quirin evidence: Q06–Q08, Q12–Q15, Q26–Q27, and Q39.*

### Function calls and contracts

| Item | Remaining difference |
|---|---|
| Indirect calls and function pointers | Match candidate callees, signatures, and applicable contracts |
| Callbacks | General relationships among capture, escape, repeated invocation, asynchronous execution, and unregister/teardown |
| Complex actual-to-formal substitution | Common handling that connects values or slots containing fields, indices, and aliases to the identities referenced by a contract |
| General read/write footprints | `Borrowed` does not imply arbitrary memory purity. Custody modes and frame effects are separate. |
| Conditional relationships among results and multiple resources | Nullable results, Out, and particular realloc rules already exist, but not arbitrary return-code×multiple-resource relations |
| General borrow lifetimes and extent expressions | Offset, length, and multiple-base relationships beyond `Derived` Exact/Interior |
| Arbitrary resource kinds | The current enum is Heap/Descriptor/Stream/Directory. Sentinels and release protocols for other handle kinds require further design. |
| Variadic tails | Data-only contracts exist, but not general semantics for variadic arguments containing arbitrary resources. |
| `va_start`/`va_arg`/`va_copy`/`va_end` | Specific source shapes exist, but not a general `va_list` state machine. |
| Format safety | A data-tail contract does not guarantee agreement between formats and types or bounds for `%s`. |
| Exit-dependent summary relationships | Express relationships beyond constraints such as the same disposition on every exit |
| Contract application across consumers | The same contract has different acceptance ranges in conditions, nested calls, fields, and dedicated profiles. |

Clang's call handling also deals with the ABI. However, Quirin already has **checks matching declared contracts against C signatures**. This is distinct from physical ABI lowering such as `byval`, `sret`, and register assignments. A custody auditor need not reimplement the latter. [LLVM reference: L10.]

*Quirin evidence: Q17–Q20, Q26, Q28–Q31, and Q41–Q42.*

## 7. Remaining Gaps in Atomics, Concurrency, and Parallelism

Here too, the comparison separates what already exists in the core from what is not connected to general C.

Clang generates code not only for atomic loads/stores but also for RMW and compare-exchange operations. That capability itself does not prove race freedom for an entire program. [LLVM reference: L11.]

| Item | Current state and gap |
|---|---|
| General C `_Atomic` accesses | T rejects the qualifier. A dedicated publication-source route and a core load/store model nevertheless exist. |
| Atomic RMW, exchange, and fetch operations | `weak_memory.rs` **does not represent RMW**. |
| Compare-exchange | A model is needed for success/failure orders, updates to expected, weak spurious failures, and related behavior. |
| Fences | `weak_memory.rs` **does not represent fences**. |
| Consume and dependencies | **Explicitly out of scope.** |
| Release sequences | Limited to C11 plain loads/stores and same-thread store sequences. |
| Checking all candidate executions | The current checker validates **one supplied execution graph**, not an exploration of all executions. |
| General C access → footprint → mutex invariant | Connections among existing models are limited. |
| General composition of publication and lifetime | Match initialization, publication, observation, use, and reclamation beyond specific publication shapes. |
| Atomic reclamation, ABA, etc. | Explicitly excluded from the footprint prototype. |
| Eventual observation, progress, and termination | Separate guarantees from publication safety. They do not follow from current successful checks. |

Here, declaring through a contract that “this function releases a resource” or “this is the layout” is different from **being able to use that declaration to check every candidate concurrent execution**.

*Quirin evidence: Q33–Q35 and Q46. The model's scope is stated explicitly in `weak_memory.rs:1–8`.*

## 8. Restrictions on Input Conditions, Language Versions, and AST Matching

Beyond individual operations, other restrictions limit which C programs can reach the audit.

| Item | State in the attached version |
|---|---|
| Dialect | An explicit enum contains `C11`, `Gnu99`, `Gnu11`, and `Gnu17`. There is no explicit C23 or similar mode. |
| Compilation flags | An allowlist is used. This is not a gateway allowing arbitrary `--target`, `-mabi`, and other arguments. |
| Type profiles | Rather than a common semantic representation covering every Clang `QualType`, readers/profiles have their own permitted subsets. |
| AST matching | Some readers require unique line/token shapes. Tests reject ambiguous candidates on the same line. |
| Additional syntax such as C23 | Dialect, AST kinds, and type/initialization semantics need to be added together. |
| GNU/Clang/MS/OpenMP and other extensions | Each adopted extension needs an adapter. Ordinary call contracts are not automatically applied. |
| Macro/target configurations | A guarantee for one selected build differs from a guarantee for all configurations. |

This does not mean a preprocessor must be reimplemented. **Clang is already used for preprocessing and ordinary C syntax/type checking.**

Also, the same-line candidate restriction does not establish that “all C written on one line fails.” Other readers and fallbacks exist, so this must be assessed route by route.

*Quirin evidence: Q02, Q04–Q05, Q10–Q15, Q17, and Q43.*

## 9. What Is Not Counted as Missing

The following differences from LLVM are not defects in a custody auditor.

There is no need to port **LLVM-form SSA, `phi` instructions themselves, mem2reg, instruction selection, register allocation, machine-code generation, vectorization, LTO, DWARF, coverage instrumentation, or bitcode output** into Quirin.

Value/state merging and source-location tracking are of course needed, but they need not use the same representation as LLVM. LLVM IR's instruction system and the abstract operations required for resource auditing are not identical. [LLVM reference: L14.]

The following were also excluded from the “unimplemented” category because they already exist:

- Typed expressions and read/write/call/guard representations; CFGs, loops, direct goto, and switch in the general custody route.
- Short-circuit conditions, typed field/index accesses, limited pointer traversal, collections, function summaries, and limited recursion.
- Checks matching contracts to C declarations, contract trust distinctions, evidence rechecking, and bounded atomic/concurrency models.

Loading a contract is also distinguished from establishing that an implementation satisfies it. In addition to `Declared`, `InferredValidated`, and `Proposed`, the attached version has **`PendingHumanReview`**. This is a strength of the current design, not a missing feature.

## 10. What Should Be Central to the Implementation

The findings suggest the following order rather than reimplementing Clang CodeGen wholesale.

**First, a common expression elaborator and contract application mechanism.**  
Separate values, places, reads, writes, calls, branches, and merges so that the same semantic rules apply in conditions, returns, nested expressions, and actual arguments. Close the small operator gaps while preventing the proliferation of position-specific special cases.

**Second, common typed places.**  
Use `DeclID / FieldID / index / view / allocation identity / generation` as a foundation for applying existing Out, Stored, Removed, Derived, and related contracts to complex C reference targets.

**Third, connections to the existing core.**  
Before rebuilding extent, layout, footprint, mutex, or atomic models, correctly pass source-derived facts and contract assumptions to existing checkers.

Numerical-domain and weak-memory extensions should then be chosen according to **the properties that actually need to be guaranteed**. For Fluent Bit, size calculations, bounds, conditional ownership transfer, and callback lifetimes are possible early targets.

**In conclusion, the central gap in Quirin is not its strategy of abstracting C through contracts. It is that the machinery for consistently applying and composing its existing contracts and verification models across C's many expression positions, reference targets, and control-flow forms remains partial.**

---

**Reference key:** Q01–Q47 refer to the original source excerpts in [Quirin_Source_Evidence_EN.md](Quirin_Source_Evidence_EN.md). L01–L15 refer to the LLVM primary sources in Section 10 of [Quirin_Clang_Gap_Analysis_EN.md](Quirin_Clang_Gap_Analysis_EN.md). Source code, evidence identifiers, source line numbers, and the original analysis's execution limitations have been preserved; the English package does not represent a new audit run.
