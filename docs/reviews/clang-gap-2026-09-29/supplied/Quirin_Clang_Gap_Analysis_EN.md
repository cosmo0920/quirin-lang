# Quirin vs. Clang C-to-LLVM IR: Gaps in Operations and Semantic Models

Analysis date: 2026-09-29 (Japan Standard Time)

## Conclusion

It is inappropriate to count all external semantics that contracts can supply as “information missing from the parser.” The principal gap identified here is that **the general machinery for applying and composing an existing contract across arbitrary C expressions, places, and control flow is incomplete**. This must be distinguished both from areas where a core checker already exists but has only limited connections to C source, and from numerical or memory semantics that become necessary only when extending the guarantee beyond custody.

Two points are particularly important: `typed_source.rs` already has typed expressions and read/write/call/guard/lifetime events, and **G and T support different sets of operators**. It is inaccurate to describe Quirin as a whole as having “no typed IR,” “no subtraction,” “no loops,” “no recursion,” “no field access,” or “no atomics.”

## 0. Scope, Reproducibility, and Limits of the Guarantees

- Quirin: the attached `quirin-lang-main(1).zip`. SHA-256: `7e6e2351bcb5b83caa249870bef82d8d6d5e84e42cbacd6093ce9e179ebea507`.
- LLVM: the operation/AST taxonomy was pinned to the main-branch commit observed during the analysis, `992ca79c83b0882465a202883109bd4cd167ec00`. Official Doxygen and the Internals Manual were also consulted for the structure of code generation. Doxygen is rolling documentation; it is not claimed to be the documentation generated from that exact pinned commit.
- A probe containing 16 C functions was compiled to both AST JSON and LLVM IR using local Clang **17.0.0**, and both compilations succeeded. This was not an execution test of the 2026 main branch.
- **The Quirin Rust build and tests were not run.** The original analysis environment had neither cargo nor rustc. The comparison statically inspected match branches, explicit rejection conditions, model definitions, and expected outcomes in existing fixtures. An existing test's expected outcome must not be confused with a result obtained by rerunning that test in this analysis.
- The unit of “coverage” is the taxonomy of standard C operators and expression/statement/declaration semantics, together with the processing that projects them into an audit. This does not mean exhaustive testing of all combinations of C programs, all target intrinsics, or all LLVM backends. C++-, Objective-C-, HLSL-, and CUDA-specific features are excluded. GNU/Clang/MS/OpenMP features are separated as extensions relevant only when adopted for C.
- The **112 entries below are neither a bug count nor a mandatory implementation count**. This is a classified comparison that also includes out-of-scope features, precision limits, and missing connections to existing models.

Evidence is provided as Q01–Q47 in the accompanying [Quirin_Source_Evidence_EN.md](Quirin_Source_Evidence_EN.md). Each excerpt includes its original path, line numbers, and SHA-256. `raw_boundary_mentions.json` is a string index used to support exploration; its number of matches must not be treated as a count of unimplemented features.

## 1. The Correct Layers to Compare

Clang's `-emit-llvm` is a mode that selects LLVM output, not an independent C parser. The relevant layers of the conventional Clang CodeGen pipeline are approximately as follows (L01–L14):

```text
C + compile options
  → preprocessor / parser / Sema
  → typed Clang AST
  → scalar / aggregate / lvalue / call / statement / atomic lowering
  → LLVM IR
```

Quirin uses Clang for the same initial stages, then projects a restricted subset of the resulting semantics according to the intended guarantee (Q01–Q17):

```text
G: custody auditing of existing C
  Clang validation and preprocessing + token reading + typed-AST assistance
  → LegacyInstruction / CFG
  → resource-contract application, state analysis, and summary revalidation

T: bounded source profiles using typed_source.rs
  typed Clang AST
  → SourceValue / SourceExpression / SourceOperation / source paths
  → consumers check guards, custody, extents, synchronization, and so on

core:
  individual models for scalars, footprints, concurrency, weak memory, etc.
  Note: a model's existence does not establish that arbitrary C can reach it.
```

**G and T do not accept the same source subset. T is not another name for the entire Typed Auditor; it is an importer used by some of its profiles.** Also, Clang's ability to emit IR for a C program is not proof that the program is free of use-after-free, races, overflow, or other safety problems.

## 2. What Contracts Can and Cannot Supply

| Situation | Are existing contracts sufficient? | Assessment |
|---|---|---|
| The acquire/release/borrow/consume/out/derived/realloc semantics of an external function are unknown, but its call shape is supported | Yes, within the existing vocabulary | The call can be audited under a `Declared` assumption without analyzing the function body |
| The call is recognized, but its result cannot be connected to a condition, nested expression, or unsupported place | Merely adding a contract is insufficient for the C source as written | F: extend frontend elaboration and contract application |
| Layout, extent, or synchronization can be declared and a checker exists, but there is no source adapter | A declaration alone does not connect the guarantee | R: connect C semantics to the checker's premises and events |
| The required property is absent from resource v1 or the relevant checker's vocabulary | Additional declarations in the existing vocabulary are insufficient | V: extend the vocabulary and validating consumer |
| An unsupported region is extracted into a helper and the entire helper is treated through a trusted contract | Sometimes possible | Whether the helper body satisfies that contract remains a separate assumption or proof obligation |
| An arithmetic value is irrelevant to custody | Discarding the value can be valid | This can be a sound abstraction and is not itself a defect |

`Declared` is an assumption; `InferredValidated` means the body has been rechecked against the summary; `Proposed` is unvalidated and cannot justify `Safe`. The attached version also has `PendingHumanReview`, which cannot justify either `Safe` or `Unsafe` (Q30). Do not confuse “expressible as a contract” with “verified to be true.”

## 3. Standard C Operator Support Matrix

“Expression-tree support” means the operation and permitted child shapes can be read. It does not mean support for arbitrary types or positions, or proof of arbitrary safety properties. G's scalar-neutral handling does not guarantee numerical evaluation. The table includes the spellings of all standard operators in the comparison (Clang taxonomy: L02).

| Operator / form | G: custody gateway | T: typed_source | Related entries | Evidence |
| --- | --- | --- | --- | --- |
| Unary + | Supported by the scalar reader | Unsupported | E02 | Q11 Q16 |
| Unary - | Supported by the scalar reader | Expression-tree support | N04 | Q11 Q16 |
| ~ | Supported by the scalar reader | Unsupported | E02 | Q11 Q16 |
| ! | Scalar/condition support | Expression-tree/branch support | E06 | Q11 Q14 Q16 |
| & (address-of) | Typed paths, actual arguments, etc., with restrictions | Expr::Unary exists; place/profile restrictions apply | M01–M05 | Q11 Q17 Q18 |
| * (dereference) | Typed scalar access, with restrictions | Expr::Unary exists; type/profile restrictions apply | M01–M25 | Q11 Q16 |
| Prefix ++ / -- | Limited scalar/pointer-step forms | Standalone statements for selected integer types only | E05 | Q12 Q24 |
| Postfix ++ / -- | Limited stepped-access support | Standalone statements for selected integer types only | E05 | Q12 Q24 |
| sizeof | Neutrality checks for fixed types/operands | Fixed profile types; childless forms only | E16 N11 M12 | Q11 Q22 Q23 |
| _Alignof | Fixed scalar queries and related forms | No dedicated support | E16 N11 | Q11 Q22 |
| * / % (binary) | Scalar-neutral reading | Expression-tree support, not numerical guarantees | N04 N05 | Q11 Q16 |
| + (binary) | Scalar/limited pointer steps | Expression-tree support, not bounds guarantees | M15 N04 | Q11 Q16 Q24 |
| - (binary) | Scalar/limited pointer steps | Unsupported | E01 M16 | Q11 Q16 Q24 |
| << >> | Scalar-neutral reading | Unsupported | E01 N06 | Q11 Q16 |
| < > | Limited scalar/condition reading | Expression-tree support | N07 | Q11 Q16 |
| <= >= | Limited scalar/condition reading | Unsupported | E01 | Q11 Q16 |
| == != | Scalar/NULL-related handling, etc. | Expression-tree support | N07 | Q11 Q16 |
| & \| (bitwise) | Scalar-neutral reading | Expression-tree support | N01–N03 | Q11 Q16 |
| ^ | Scalar-neutral reading | Unsupported | E01 | Q11 Q16 |
| && \|\| | Condition CFG support, not every value position | Short-circuit execution in conditions only | E06 | Q13 Q14 Q19 |
| ?: | Limited to top-level conditions and related forms | Unsupported as an expression | E07 | Q11 Q19 |
| = | Shape-specific rules for assignments, conditions, Out, fields, etc. | Statements only | E04 M02 | Q11 Q12 Q18 Q19 |
| += -= | Limited scalar/pointer-step forms | Unsupported | E03 | Q12 Q24 |
| *= /= %= <<= >>= &= ^= | Scalar-statement rules and related handling, with restrictions | Unsupported | E03 | Q12 Q16 |
| \|= | Scalar-statement rules and related handling | int statements only | E03 | Q12 |
| , (comma) | Conservative closure-based scanning exists, not general expression semantics | Unsupported | E08 | Q11 Q38 Q40 |
| [] | Limited typed reads/writes/collections and related forms | Index expression; profile restrictions apply | M01 M02 M14 | Q11 Q16 Q25 |
| . -> | Typed paths/limited tracked stores | Selected record fields only | M01–M10 | Q11 Q16 Q18 |
| Function call () | Contracted direct calls plus limited nesting/dispatch | Direct callees in prescribed forms only | E09–E12 K01–K14 | Q11 Q17 Q20 |
| (type) cast | Limited forms, including scalar casts and strict pointer-return handling | Six cast kinds plus LValueToRValue | E15 N02 N03 N08 M18 | Q11 Q16 Q40 |

## 4. Classification Codes

| Code | Meaning | How to interpret priority |
|---|---|---|
| F | Extend C AST/expression/place/control elaboration, contract application, or effect composition | Prioritize when broadening the usable scope of existing contracts |
| R | An adapter connecting C-source facts/contracts to an existing checker | Prioritize the connection rather than reimplementing the core |
| V | Extend a property's abstract domain, contract vocabulary, or consumer | Required only when that property is intended to be guaranteed |
| P | A conservative coverage, precision, or budget limit | A task for reducing `Unknown`; not automatically a soundness bug |
| O | Outside the current custody guarantee, or an unnecessary backend difference | Do not count as a defect or mandatory TODO |

## 5. Complete Classified Inventory

### Expression Acceptance, Evaluation, and Effect Composition

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| E01 Binary - <= >= ^ << >> | T / F | All six operators are absent from the BinaryOperator allowlist in Importer::expression. G's typed-access scalar reader supports all six. | Extend T's expression tree and consumers. Do not describe G as lacking subtraction, comparisons, or shifts. | Q11 Q16 |
| E02 Unary + and ~ | T / F | T's UnaryOperator supports only & * - !. G's scalar reader accepts + - ! ~. | Extend the common unary representation and consumers. Numerical evaluation itself is unnecessary when only custody invariance must be established. | Q11 Q16 |
| E03 Compound assignment += -= *= /= %= <<= >>= &= ^= | T / F | T's CompoundAssignOperator supports only int \|=. G already has scalar compound assignments and some pointer steps. | Compose single evaluation of the left-hand side, the old-value read, operation-time conversions, writeback, and the expression result. | Q12 Q24 |
| E04 Assignment expressions used as values | G/T / F | T handles = in its statement branch, not as an Expr. G reads conditional assignments and other forms through shape-specific rules. A nested-assignment closure fix exists, but it is not full expression elaboration. | Handle x=(y=f()), return (x=f()), etc. through one evaluation/effect model. Closure detection alone must not be treated as execution semantics. | Q11 Q12 Q19 Q38 |
| E05 Value-producing ++/-- and prefix/postfix distinctions | G/T / F | T supports standalone statements for selected integer types. G handles pointer steps and limited stepped accesses. Old/new values and side effects are not composed uniformly across arbitrary expression positions. | Separate value production from update events. Replacing i++ with i=i+1 changes the value of the expression. | Q11 Q12 Q24 |
| E06 Value-producing &&/\|\| positions | G/T / F | G has condition CFGs. T preserves these operators in expression trees but rejects short-circuit expressions in ordinary evaluate, restricting them to branch_condition. | Merge result values in assignments, actual arguments, returns, and comparison operands without producing effects from the unevaluated side. | Q13 Q14 Q19 |
| E07 General ?: values, places, and ownership merges | G/T / F | G supports top-level ?: in condition contexts. T's expression handler has no ConditionalOperator. This is not general merging of all value/resource results. | Merge the selected branch's effects and the result's alias/custody state. Handle the GNU omitted-middle-operand form separately. | Q11 Q19 Q37 |
| E08 Comma operator | G/T / F | Closure handling conservatively detects nested assignments, but T has no comma BinaryOperator. Several position-specific readers in G also reject comma. | Represent sequencing explicitly: finish the left operand, evaluate the right operand, and return the right operand's value. | Q11 Q38 Q40 |
| E09 Ownership-producing calls in conditions | G / F | Condition desugaring rejects Owned/Reallocated/descriptor results. Even with a malloc contract, an acquisition in a condition does not directly enter this route. | Lower the new resource identity, assignment, NULL/sentinel branches, and success/failure states together. | Q19 Q29 Q47 |
| E10 Non-identity effects of nested calls | G / F | Nested calls require identity conditions such as result None and all parameters Borrowed. Owned/Released/Consumed/Out effects are not generally composed. | Apply existing contract effects from the inside out and connect values/aliases to the outer call's actual arguments. | Q03 Q20 |
| E11 Calls nested more than one level and calls within subexpressions | G / F | Support centers on a one-level nested call occupying an entire argument. Calls within nested calls and calls occupying only part of an argument are rejected. | Extend recursive expression elaboration to f(g(h(p))), f(g(p)+1), and similar forms. | Q20 |
| E12 Sibling calls and call/read combinations with unspecified order | G/T / F | G restricts multiple calls in one leaf to identity effects. T rejects combinations involving calls and multiple reads through reject_unsequenced. | Handle every permitted order or prove commutativity from contract footprints. Do not arbitrarily impose left-to-right evaluation. | Q14 Q19 Q20 |
| E13 Complex expression composition within condition leaves | G / F | Nested connectives, type operators, increment/decrement, expression callees, etc. are explicit rejections in condition plans. | Connect general expression elaboration to the existing condition CFG. Do not merely make a condition opaque and drop its calls. | Q19 |
| E14 FloatingLiteral / CharacterLiteral | T / F | T handles IntegerLiteral/StringLiteral but has no float/character-literal branches. G's typed scalar reader has those branches. | Preserve typed literals in T. Exact FP arithmetic need not be implemented simultaneously for custody-only purposes. | Q11 Q16 |
| E15 Coverage of cast kinds | T / F | T is limited to LValueToRValue plus BitCast/IntegralCast/NoOp/ArrayToPointerDecay/NullToPointer/FunctionToPointerDecay. | Handle IntegralToPointer/PointerToIntegral, ToVoid, the ToBoolean variants, integer–FP/FP-width conversions, atomic conversions, AddressSpaceConversion, etc. according to the intended property. | Q11 Q37 |
| E16 sizeof / _Alignof positions and operand shapes | G/T / F | G supports fixed scalar queries and limited sizeof(*p)/p[literal] forms. Substitution with 1 is a neutrality marker. T supports only childless sizeof of fixed profile types. | Distinguish unevaluated operands from operands evaluated because of VLA semantics, connecting to actual size/alignment only when needed. | Q11 Q16 Q22 Q23 |
| E17 The actually selected _Generic association | G/T / F | G uses selection invariance: all associations must have the same core. Reading Clang's selected GenericSelectionExpr result is explicitly unimplemented. T also has no dedicated branch. | For differing associations, use Clang's selected result without evaluating the controlling expression or unselected branches. | Q21 Q11 |
| E18 General offsetof / container_of handling | G/T / F/V | T has no OffsetOfExpr. Some G token shapes and existing container_of boundaries do not constitute a model of arbitrary offset calculation or reverse subobject derivation. | Use target/layout contracts when the offsetof value is needed; connect base/subobject provenance for container_of. | Q11 Q33 Q38 |
| E19 GNU statement expressions | G/T / F | T has no StmtExpr. G's closure/special-shape detection of internal assignments is distinct from general elaboration of a block's final value, lifetime, and exits. | Connect the inner scope, final expression value, internal calls, and escape to the outer scope. | Q11 Q37 Q38 |
| E20 A common opaque boundary for builtins/extension expressions | G/T / F/V | T supports a profile of direct CallExpr nodes. It does not interpret other AST kinds, such as ChooseExpr, BuiltinBitCastExpr, or vector/complex nodes, as ordinary contracted calls. | Do not automatically apply ordinary function contracts to all AST kinds. Add adapters for AST-specific effects or establish boundaries through contracted wrappers. | Q11 Q39 |

### Places, Memory, Declarations, and Initialization

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| M01 Tracked paths mixing embedded fields, indexing, and dereferencing | G / F/R | Typed access accepts mixed paths only when the root is never-bound. The tracked graph primarily supports direct -> field links. | Promote these to a common place representation built from DeclID/FieldID/Index/offset and connect it to actual arguments in existing contracts. | Q16 Q17 |
| M02 General pointer-valued stores | G / F | General s->p=q, *slot=p, and slots[i]=p are not supported. Limited aggregate_store/Out/collection handling already exists. | Distinguish value reads/stores, alias addition/overwrite, old obligations, and escape. | Q18 Q36 |
| M03 Field stores through caller-owned objects or aliases | G / F | aggregate_store requires conditions such as a local owner acquired in the current function and a simple base definition. Formals, aliases, and general owners are not treated equivalently. | Obtain evidence of the caller's frame and owner identity from contracts and connect it to the field-update transfer. | Q18 |
| M04 Storing through nested fields, arrays, function pointers, etc. | G / F | aggregate_store focuses on exact BASE->FIELD forms, with restrictions on nested embedded fields, arrays, multilevel pointers, function pointers, and related shapes. | Generalize stores around typed places. Distinguish this implementation work from bypassing it with an opaque helper contract. | Q18 |
| M05 Typed readback from void* fields | G / F/V | Storage into certain void* fields is supported, but reads from tracked void* fields are rejected because pointee-typing evidence is insufficient. | Add contracts and implementation that connect the post-cast view/pointee type to the stored identity. | Q18 |
| M06 Union loads/stores and overlapping members | G/T/core / F/V | Typed access rejects union owners. The core footprint model also marks UnionMember unsupported. Some address-taking of union-typed members exists through another route; not every union expression is impossible. | Represent overlapping storage and the view/effective-type boundary, or hide union operations inside a contract boundary. | Q16 Q33 Q36 |
| M07 Bit-field reads/writes | G/T/core / F/R | G's typed access rejects bit-fields. The core already represents bit-field storage-unit sharing under layout contracts. | Connect source fields to storage units and read/modify/write events. Different bit ranges must not automatically be treated as disjoint. | Q16 Q33 |
| M08 Whole-struct/union copies and assignments | G/T / F/V | There is no general field-wise alias/custody transfer for aggregate assignment. G even has a rejection fixture for h->value=h->value. | Track aggregate values field by field or treat a copy function through an explicit contract. | Q11 Q12 Q36 |
| M09 General aggregate and designated initializers | T/G / F | T's InitListExpr is limited to two elements of struct timeval. G's closure detection of bindings inside braces is not general initialization semantics. | Reflect designators, implicit zero-fill, array elements, and initial aliases in pointer fields. | Q11 Q12 Q38 |
| M10 Compound-literal object identity and lifetime | T/G / F/V | T does not support CompoundLiteralExpr. In G, it appears among rejected forms in certain positions. | Give anonymous storage an identity and block/file lifetime, and pass its address/value into contracts. | Q11 Q37 |
| M11 String/array initialization extents and terminators | G/T / V/R | String literals exist, but this is distinct from deriving byte extents, termination, and remaining zero-fill from general C initialization. | Connect to an initialization model when bounds/string guarantees are intended. This is unnecessary for custody-neutral use alone. | Q09 Q11 Q12 Q25 |
| M12 VLA allocation, evaluation, and release scope | G/T/core / F/V | T supports fixed storage only. The core footprint model explicitly excludes VariableLengthArray. G has an Unknown fixture for VLA sizeof side effects. | Represent actual evaluation of bound expressions, dynamic extents, and lifetime at scope exit. | Q12 Q33 Q36 |
| M13 Flexible array members | G/T/core / F/V | The core footprint model explicitly marks them unsupported. There is no general gateway connection deriving their allocated extents from C. | Combine allocation-size contracts with offsetof and element width to derive the remaining region. | Q18 Q33 |
| M14 Bounds of general computed indices | G/core / R/V | Collections track intervals of release obligations, not bounds. The footprint prototype also excludes ComputedIndex. | Connect a numerical domain/guards to extent contracts to verify 0<=i<count. | Q25 Q33 |
| M15 Byte offsets for general pointer arithmetic | G/T/core / R/V | G's steps track coarse derivation, such as positive literals, without evaluating offsets. T's preservation of Binary expressions also does not automatically prove bounds. | Introduce element-size×index, offset updates, and in-extent conditions only for properties that require them. | Q09 Q24 Q33 |
| M16 Same-object conditions for pointer subtraction and ordering | G/T / V/R | Some pointer truthiness/comparison and derived-pointer bookkeeping exist, but arbitrary pointer differences/orderings are not checked for the same-array condition. | Connect these operations to comparison obligations carrying provenance/extents. If a contract assumes the same allocation, display it as an assumption. | Q08 Q17 Q24 |
| M17 Distinguishing one-past formation from dereference | G/core / V/R | Interior/Exact/stepped handling is not a general source route that precisely classifies arbitrary one-past pointers. | Separate the permitted pointer-formation range from the load/store range. | Q08 Q24 Q29 Q33 |
| M18 Pointer–integer round trips and tagged pointers | G/T / F/V | T's cast allowlist lacks PointerToIntegral/IntegralToPointer. G also restricts cast-return forms. | Use a model preserving bit width, information loss, provenance, and masks, or a trusted opaque wrapper. | Q11 Q29 Q40 |
| M19 Alignment obligations for addresses/offsets | G/core / R/V | Declared layout contracts exist, but there is no general source connection comparing a C access's required alignment with its derived address. | Compare typed accesses, layouts, and offsets when alignment guarantees are intended. | Q16 Q33 |
| M20 General connections for allocation generations and stale aliases | G/core / R | G already tracks release/realloc and derived aliases. The core footprint model has generations, but they are not connected to all general C places. | Preserve existing liveness checks while connecting finer subobjects/views to generations. | Q08 Q29 Q33 |
| M21 Non-aliasing proofs across different allocations | core / V/R | The footprint prototype explicitly rejects disjointness across different allocation identities. Its design does not assume that different IDs are unconditionally disjoint. | Introduce an allocator model, actual object identity, or an explicit assumption. | Q33 |
| M22 Effective type, type punning, and restrict | G/T/core / V/F | Some G routes assume C effective-type rules and related conditions. T rejects the restrict qualifier. The footprint core excludes TypePunnedAlias. | restrict is not equivalent to atomic or volatile. Preserve type-view constraints or state them explicitly as assumptions at the guarantee boundary. | Q10 Q20 Q33 |
| M23 Counts and ordering of volatile accesses | G/T / F/V | T rejects volatile. G's custody-neutral handling does not guarantee volatile I/O ordering. | For MMIO auditing, model observable reads/writes as separate effects. Keep this separate from custody-only guarantees. | Q10 Q16 Q29 |
| M24 Global/static/thread-local storage | T/G / F/V | T rejects global references and locals with storageClass. Global-related closure handling in G is not a value model for all storage lifetimes. | Add identity/lifetime handling for thread-local objects, static initialization, and shared globals to the source profiles that need them. | Q11 Q12 Q39 |
| M25 Uninitialized values, padding, and object representation | G/T / V/O | T has LocalStart/Read, but a raw summary alone is not a safety proof. G's Safe is not a general guarantee of initializedness or safe padding access. | Add domains only when definition-before-read or object-representation guarantees are required. | Q09 Q12 Q45 |
| M26 Internal byte semantics of memcpy/memmove/memset | G/core / V/O | Resource contracts can express custody neutrality, exact aliases, etc., but not every byte-initialization, overlap, and effective-type property. The footprint core excludes OverlappingCopy. | Check transfer ranges, overlap, and type views only when claiming byte-level safety. Existing contracts can suffice for the external call's custody effects alone. | Q29 Q33 |

### Numerical Semantics and Use of Clang Type Information: Gaps When Extending Guarantees

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| N01 Canonical machine scalar types | G/T / V/R | Clang Sema already performs type checking. T retains c_type strings and some casts, but this is not a common width/signedness/rank/target numerical domain. | Import Clang's results rather than reimplementing C typing. Connect LP64/LLP64 and related distinctions to contract/target identity. | Q02 Q09 Q10 Q32 |
| N02 Integer promotions and usual arithmetic conversions | G/T / V/O | T's IntegralCast preserves some conversion information. G's neutral expressions do not compute numerical values. Numerical properties after conversion are outside the guarantee. | Give conversions semantics in consumers requiring arithmetic guarantees. Permit their removal when custody is invariant. | Q11 Q16 Q32 |
| N03 Sign extension, zero extension, and truncation | G/T / V/O | Preserving the name IntegralCast is different from proving bit-level results or information loss according to C widths and signedness. | Add cast transfer functions based on the target type. | Q09 Q11 Q32 |
| N04 Signed overflow and unsigned modular arithmetic | G/T/core / V/O | G does not prove overflow freedom for all inputs. Core scalar_calls uses checked I32 and explicitly lacks wraparound/C-UB semantics. | Add overflow assumptions corresponding to source C or a bitvector domain. Do not equate existing core operations directly with C operations. | Q32 Q45 |
| N05 Exceptional domains of division/remainder | G/T / V/O | Acceptance of / and % is not proof of safety against division by zero or signed minimum divided by -1. | Connect guards, ranges, and type widths. Input constraints from call contracts can be used instead, but the obligations must be displayed. | Q11 Q16 Q32 |
| N06 Shift counts and signed-shift semantics | G/T / V/O | G reads shifts as scalar-neutral. T does not yet represent shifts. General checking of C count restrictions and related conditions is a separate issue. | Separately from E01, add count/range/signedness obligations for the intended guarantee. | Q11 Q16 Q32 |
| N07 Mixed-signedness comparisons and Boolean conversions | G/T / V/O | G's NULL/validity/some index refinements differ from arbitrary numerical comparison semantics. T retains comparison trees only. | Use post-conversion types and ranges. Distinguish pointer truthiness from integer truthiness as well. | Q08 Q11 Q17 |
| N08 Integer–FP and FP-width conversions | G/T / F/V/O | T lacks the relevant CastKind support. G's scalar-only cast reading does not prove FP ranges or rounding properties. | Implement representation support under E15 separately from numerical transfer functions. | Q11 Q16 |
| N09 IEEE floating point: NaN, infinity, signed zero, and rounding | G/T / V/O | These are outside the guarantees of generic custody auditing and symbolic source summaries. | Model them in source profiles requiring floating-point properties. Do not count them as custody-audit defects. | Q09 Q32 Q45 |
| N10 fenv, rounding modes, FP exceptions, and fast-math assumptions | G/T / V/O | The relevant resource contracts and scalar model are not a general domain for these properties. | Import them as environment effects/compiler settings, or state explicit assumptions/exclusions. | Q04 Q29 Q32 |
| N11 General evaluation of constant expressions, enums, and sizeof values | G/T / P/V | T uses a table of unique, already-evaluated enum constants. G's type-query marker 1 is not the actual size. | Use Clang-evaluated values and return Unknown outside the supported domain. Do not reuse markers in bounds or branch decisions. | Q10 Q22 Q23 |
| N12 _Complex / imaginary / __real / __imag | G/T / F/V/O | T lacks ImaginaryLiteral/complex casts. G's scalar-type classification excludes _Complex/_Imaginary. | Add paired representations and cast/operation adapters when complex values are needed. A custody-neutral wrapper can also establish a boundary. | Q11 Q44 |
| N13 Vector/matrix operations and shuffle/convert | G/T / F/V/O | T's expression/type/AST allowlists do not include vector/matrix-specific nodes. | Add lane/element operations only when supporting selected C extensions. Duplicating every ISA intrinsic is unnecessary. | Q11 Q12 |
| N14 Additional scalar types such as _BitInt and fixed point | G/T / F/V/O | T has type-profile restrictions. G's builtin scalar table is also not Clang's complete scalar-type system. | Extend the dialect, type widths, casts, and arithmetic domain together. | Q05 Q10 Q12 Q44 |

### Control Flow, Scopes, and Nonlocal Control

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| C01 General while/for/do in typed_source | T / F | T's for support is limited to canonical scans/early returns, and do to literal 0. There is no WhileStmt branch. G has general loop CFGs. | This is not evidence that G lacks loops. Connect backedges and invariants/summaries to T/profiles. | Q06 Q12 |
| C02 switch/case/default in typed_source | T / F | These are absent from T's statement allowlist. G already implements them. | Add case branching, value selection, and fallthrough to T, or use a common CFG. | Q06 Q07 Q12 |
| C03 goto/label/break/continue in typed_source | T / F | These are absent from T's statement allowlist. G has CFG construction and jump-label resolution. | Connect T's local lifetime/end handling to control targets. | Q06 Q07 Q12 |
| C04 Pruning reachable cases using switch values | G / P/V | G conservatively creates possible edges to all case entries. This is not numerical dispatch based on the switch value. | Refine cases through scalar path conditions when needed. Conservative Unknown results are not soundness bugs. | Q07 |
| C05 Arbitrary branch correlations and infeasible-path reduction | G/T / P/V | G has limited NULL/validity/distinct/index domains. T retains all branches and delegates guard proofs to consumers. | Add range/relational predicates only where required. Contract preconditions can also be used. | Q03 Q08 Q09 |
| C06 General loop invariants and quantified collection relations | G/T / P/V | Indexed collections/limited scans exist, but they are not numerical/heap invariants for arbitrary loops. | Connect invariant contracts, widening/narrowing, or loop summaries according to the target property. | Q12 Q25 |
| C07 Ranks and contexts for general recursion | G / P/V | Countdown partitions and neutral recursive frames already exist. These are not general semantics for arbitrary multiple ranks or heap-structural recursion. | Add inductive summaries, ranking, and SCC fixed points to the contracts under verification. Do not simply label recursion unimplemented. | Q26 Q27 |
| C08 Path/operation/node/alias budgets | G/T / P | T has limits such as 64 paths, 512 operations, depth 64, and 2,048 nodes. G's MAX_ALIAS_TARGETS is 8 with widening. | Report coverage/precision bounds. Improve them using shared DAGs/summaries rather than changing budget-exhausted results to Safe. | Q08 Q09 Q10 Q13 |
| C09 Computed goto and label addresses | G/T / F/V | G's statements cover direct Goto/Label. T lacks IndirectGotoStmt/AddrLabelExpr. | Represent target and edge sets or establish an explicit boundary around unsupported regions. | Q06 Q11 Q12 |
| C10 Nonlocal returns such as setjmp/longjmp | G/T / F/V | Ordinary ContractCall/Return models do not represent returns-twice behavior or return edges across stack frames. | Do not omit continuations merely because a custody contract exists. Define nonlocal edges and lifetime/invalidation behavior. | Q03 Q06 Q12 Q29 |
| C11 Cleanup attributes and hidden scope-exit calls | T/G / F/V | T rejects local-attribute forms. Native/specialized cleanup replay does not lower general C CleanupAttr. | Attach cleanup events to every scope exit and compose them with noreturn/error/goto. | Q12 Q42 |
| C12 Inline asm, asm goto, and MS SEH | G/T / F/V/O | The typed-root reader explicitly rejects GCCAsmStmt/MSAsmStmt. General control/expression allowlists do not lower SEH. | Model clobbers, memory effects, and control edges at a contract boundary, or reject them. | Q06 Q11 Q12 Q39 |
| C13 noreturn in expression positions and exits from unknown calls | T/G / F/V | T specially handles standalone terminal calls whose signatures carry noreturn. This is not general nested-call/arbitrary-attribute lowering. | Represent normal/abrupt/no-return exits explicitly in contract outcomes and compose them across all contexts. | Q12 Q13 Q29 |

### Function Calls and Contract Expressiveness/Application

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| K01 Indirect callees, function pointers, and general dispatch | G/T / F/V | T supports only prescribed decay forms referring to a direct FunctionDecl. G's specialized thread-entry dispatch census is not general function-pointer analysis. | Resolve candidate callees, common signatures, and contracts, then connect them to the call site. A contract cannot be applied without identifying the callee. | Q11 Q01 Q17 |
| K02 Callback capture, escape, and repeated invocation | G/T / F/V | Specialized callback/task source profiles exist, but resource v1 alone is not a common contract for arbitrary closure environments, invocation counts, and asynchronous lifetimes. | Make the relationship among capture at registration, borrowing during execution, and unregister/teardown explicit to the consumer. | Q01 Q29 |
| K03 Formal substitution for complex actual-argument places | G / F | Typed access positions and borrowed actuals have limited-path/never-bound conditions. They do not provide general call-contract application over mixed places. | Unify substitution from expression value to identity to formal, and from address-of-slot to Out destination. | Q16 Q17 Q18 |
| K04 General read/write footprints and frame effects | resource v1 / V | Borrowed/Owned and related modes describe custody. They are not a general frame model for arbitrary heap read/write sets or preserves clauses. | Add the required effect vocabulary and consumers. Do not mistake Borrowed for memory purity. | Q20 Q29 Q31 |
| K05 Arbitrary conditional relationships between results and multiple resources | resource v1 / V | Nullable Owned, specific Reallocated rules, Out, etc. already exist. A mode schema is not a logic for arbitrary return-code×multiple-resource postconditions. | Add relational postconditions for normal/failure branches where needed. Do not discard existing specialized realloc rules. | Q29 |
| K06 General borrow lifetimes and extent expressions for returned aliases | resource v1 / V | Derived consists of a parameter plus Exact/Interior and nullable/const_dropped. It does not express arbitrary multiple bases, numerical offsets/lengths, or borrow relationships spanning callbacks. | Put richer alias/extent/region relations in additional vocabulary and consumers. | Q29 |
| K07 Arbitrary resource kinds and handle representations | resource v1 / V | ResourceKind has four variants: Heap/Descriptor/Stream/Directory. It does not directly enumerate the distinct kinds of every Windows HANDLE, registry resource, or GPU resource. | Add required kinds, invalid sentinels, and release protocols, or explicitly abstract them. Avoid unjustified equivalence with an existing kind. | Q29 |
| K08 General semantics of variadic tails | G/T / F/V | G already supports declared resource_variadic_data_tail and validation of data-only actual arguments. T rejects variadic functions selected for analysis. | Extend only when generalizing vararg custody, promotions, and type traversal. Do not label all variadic support unimplemented. | Q15 Q20 |
| K09 General source semantics of va_start/va_arg/va_copy/va_end | G/T / F/V | VAArgExpr appears in typed-root address-use scanning but not in the general typed-expression importer. The two va_arg operations in a particular property-key reader are a specialized shape. | Model va_list state, extraction order, and requested types, or place an entire vararg helper behind a contract boundary. | Q11 Q39 Q41 |
| K10 Variadic format correctness | G / V/O | The data-tail contract explicitly does not guarantee agreement between format strings and actual-argument types or the extent of %s arguments. | Connect a separate checker when format safety is required. This is not mandatory for custody purposes. | Q20 |
| K11 Arbitrary exit-specific dispositions in summaries | G / P/V | Summary inference uses rules such as agreement of formal dispositions across all exits. It cannot generally express polymorphic exit relationships absent from existing modes. | Extend expressiveness under K05 together with source revalidation. | Q26 Q29 |
| K12 Consistent contracted-call handling across condition/nested/field consumers | G/T / F/R | Consumers permit different vocabularies and shapes. A contract recognized by one consumer may be rejected in another position. | Move toward common CallSummary instantiation, supported-clause declarations, and an application mechanism that preserves total-or-reject behavior. | Q17 Q19 Q20 Q31 Q42 |
| K13 C signatures versus the physical ABI | G/T / V/O | ABI-projection checking between declared contracts and C signatures already exists. This is not a machine ABI implementation for argument registers, byval, sret, etc. | Ordinary custody auditing can delegate this to Clang. Compare target-ABI results with contracts only when binary-interface guarantees are intended. | Q28 Q29 |
| K14 Matching summary evidence across translation units/builds | G/T / O | Input/build identity and rechecking already exist. Completeness of arbitrary whole-program link resolution was not demonstrated here; this entry is not a finding of a deficiency. | State matching conditions when extending cross-translation-unit guarantees. Do not characterize current rechecking as absent. | Q02 Q04 Q28 Q30 |

### Atomics, Concurrency/Parallelism Models, and Connections to C

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| W01 General source reads/writes of _Atomic | G/T/core / F/R | T rejects the _Atomic qualifier. G has specialized routes such as atomic publication, and the core has an atomic load/store model. | Route C AtomicExpr nodes, atomic casts, and accesses to the intended checker. Atomics as a whole are not unimplemented. | Q10 Q34 Q35 Q39 |
| W02 Atomic RMW, exchange, and fetch_* | core/source / V/R | weak_memory explicitly states that RMW is not represented. | Add indivisible read/write events, positions in modification order, and participation in release sequences. | Q34 |
| W03 compare_exchange success/failure/weak behavior | core/source / V/R | The plain load/store model has no CAS events. | Represent success/failure memory orders, updates to expected, spurious failure, and successful RMW as distinct states. | Q34 |
| W04 Thread/signal fences | core/source / V/R | weak_memory explicitly excludes fences. | Add fence/atomic-access relationships to the memory model. Treating fences as ordinary neutral calls cannot establish happens-before. | Q34 |
| W05 memory_order_consume and dependencies | core/source / V/R | consume is explicitly excluded. | Specify the adopted C memory model and compiler treatment, then represent dependencies or keep this explicitly out of scope. | Q34 |
| W06 Extended release sequences | core/source / V/R | The model is limited to C11 plain stores and same-thread store release sequences. | Match the chosen standard's rules, including RMW where applicable. Do not casually mix C11, C++20, and RC11. | Q34 |
| W07 Exploring all candidate executions and universal safety | core/source / V | weak_memory checks one supplied graph containing reads-from, modification order, synchronizes-with, and SC order. | Add execution generation, exhaustive exploration/abstraction, and justification of bounds. Success for one graph is not race freedom for all C executions. | Q34 Q46 |
| W08 Composing general access footprints with mutex invariants | core/source / R | The footprint prototype itself says it is not integrated into general production auditing and treats mutex-invariant composition as follow-on work. A limited source.socket.v1 route exists. | Connect actual C places to derived footprints and mutex-invariant evidence. | Q33 |
| W09 Generalizing publication and lifetime | source / R/V | The dedicated atomic-publication route checks happens-before conditions when an observation occurs. It is not a lifetime proof for arbitrary structures and all interleavings. | Derive initialization, publication, observation, use, and reclamation stages from source. | Q35 Q46 |
| W10 Atomic reclamation and ABA-related behavior | core/source / V/R | The footprint prototype explicitly excludes AtomicReclamation. | Do not claim lock-free reclamation safety from generations alone. Connect protocols such as hazard pointers/epochs to contracts and checkers. | Q33 Q34 |
| W11 Liveness, eventual observation, and termination | source/core / V/O | The publication-source guarantee does not establish that a flag is eventually observed or that execution terminates. | Introduce separate fairness/progress/termination obligations only when required. Distinguish them from custody and safety. | Q35 Q46 |

### Input Conditions, Language Versions, Matching, and Extensions

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| I01 Explicit dialect selection | G / F/P | CCompilerDialect contains only C11/Gnu99/Gnu11/Gnu17. | To target strict C17, C23, C2y, etc., add flags, frontend identity, and syntax/semantic changes together. | Q05 |
| I02 Arbitrary target/ABI/semantic compilation flags | G / F/P | CFrontendFlag is an allowlist, not a gateway for arbitrary compiler arguments. | Add structured --target/-mabi and related options as needed. Include them in evidence identity rather than providing unrestricted passthrough. | Q04 Q05 |
| I03 Type-declaration profile coverage and canonicalization | G/T / F/P | Additional readers in both G and T do not give semantics to every Clang QualType. T is limited to signature/local type tables. | Organize typedefs, enums, atomics, arrays, function pointers, etc. around canonical type IDs. Leave C type-compatibility decisions to Clang. | Q10 Q12 Q15 Q28 |
| I04 Unique source-location-to-AST-node matching | G / P/F | Multiple readers require uniqueness of line/token shapes. Tests reject ambiguous candidates on the same line. | Move toward DeclID/SourceRange/AST-node identity. This does not imply that every one-line C program fails. | Q17 Q43 |
| I05 C23 bool/nullptr/typeof/constexpr and related features | G/T / F/P | There is no explicit C23 dialect, and T's typed-expression/declaration allowlists do not provide general support. Clang accepting some features as extensions in older modes does not mean all their semantics can be audited. | Register AST differences for each adopted dialect. Do not mechanically exclude a nullptr node from C merely because its name has a CXX prefix. | Q05 Q11 Q12 |
| I06 Extension nodes: Blocks, OpenMP, SIMD, SEH, etc. | G/T / F/V/O | Execution semantics of extensions absent from general allowlists do not automatically become contracted calls. | Define adapters/contract boundaries only for extensions adopted for C. Keep them separate from all of C++/Objective-C/HLSL. | Q06 Q11 Q12 Q39 |
| I07 Scope of preprocessing and build conditions | G / P | Preprocessing is already delegated to Clang; reimplementing all macro expansion is not a missing task. Different macro branches/targets imply different input semantics. | Distinguish guarantees for the selected build from guarantees over all configurations. The latter separately requires coverage of the configuration set. | Q01 Q02 Q04 Q05 |

### Differences from LLVM That Are Not Custody-Auditor Deficiencies

| ID / operation or mechanism | Scope / class | Current state and remaining gap | Required work and the contract boundary | Evidence |
| --- | --- | --- | --- | --- |
| X01 SSA conversion, phi construction, and mem2reg | auditor / O | G already has CFGs, and T has symbols/reads/writes/guards. LLVM-form SSA is not itself a necessary condition for resource safety. | Value merging and effect consistency are necessary; duplicating LLVM instructions is not. | Q03 Q06 Q09 |
| X02 Instruction selection, register allocation, and machine code | auditor / O | These belong to the backend after C-to-IR translation, not to Quirin's C auditor. | Do not include them as missing implementation tasks. | Q01 Q45 |
| X03 LLVM optimization, vectorization, and LTO | auditor / O | The amount of optimizer implementation is separate from the range of C semantics that can be audited. | Treat translation validation as a separate scope if correspondence to the executed binary is required. | Q01 Q45 |
| X04 DWARF, coverage instrumentation, and IR printing/bitcode | auditor / O | emit-llvm output formats and debug machinery are not prerequisites for contract application. | Source-location tracking is necessary; porting all debug code generation is not. | Q02 Q17 |
| X05 Reimplementing all of Clang Sema | auditor / O | A frontend entry point already uses Clang to parse and type-check. Duplicating checks such as C declaration compatibility is unnecessary. | Preserve or abstract the selected semantic information without losing relevant facts, and state audit assumptions explicitly. | Q01 Q02 Q28 |
| X06 Comparing against an assumption that emit-llvm proves absence of all C UB | auditor / O | Quirin's custody guarantee is not general C memory safety. LLVM IR generation likewise does not prove all dynamic safety properties of input C. | Do not conflate CodeGen semantic representation, Sema constraint checking, and sanitizer/checker safety guarantees. | Q09 Q32 Q45 |
| X07 Analyzing the bodies of all external implementations | auditor / O | The design already uses Declared contracts as explicit assumptions. Body analysis is not mandatory for opaque external functions. | Report contract applicability separately from evidence that the implementation satisfies the contract. | Q01 Q29 Q30 |

## 6. Representative Examples: Contract Gaps vs. Elaboration Gaps

### A. An acquisition contract can be written, but an Owned result inside a condition remains unsupported

```c
void *p;
if ((p = malloc(n)) != NULL) {
    free(p);
}
```

What is needed is not new semantics for malloc/free, but acquisition-result identity → binding to p → NULL/nonnull branching → release on only one branch → merging. G's condition desugaring rejects Owned results; existing fixtures also check that an acquisition in a condition is not judged Safe and that the call is not hoisted (Q19/Q47). Correct normalization to `p = malloc(n); if (p != NULL) ...`, or generalizing condition elaboration, is the appropriate direction. This does not contradict the design in which contracts can describe unknown bodies.

### B. Nested-call effects must be composed into the outer call

```c
sink(release_and_status(p));
```

Even if contracts exist both for release_and_status, which releases p and returns int, and for sink, G's current nested-call rules reject non-identity effects (Q20). This single-argument example can use the fact that the inner call precedes the outer call. By contrast, `sink2(f(p), g(q))` introduces the separate issue of sibling evaluation order, requiring both orders or a commutativity argument. Arbitrary AST enumeration order must not be treated as an order guaranteed by C.

### C. Opaque semantics of a pointer field and the source store are separate issues

```c
struct S { int *p; };
void set(struct S *s, int *q) { s->p = q; }
```

The general store in this body is an Unknown case in G's current fixtures (Q36). One route is to establish a boundary around set using a suitable trusted contract and audit the caller. However, proving from source that set's body satisfies that contract requires the store/owner/frame representations in M02/M03. These are two different questions.

### D. The sizeof marker 1 is not an actual size

`type_query.rs` and `sizeof_operand.rs` use a marker of 1 to classify unevaluated operands as resource-neutral tokens (Q22/Q23). This is not an incorrect implementation of sizeof. Conversely, reusing the same marker for allocation-size or bounds calculations would be incorrect. When bounds become part of the guarantee, connect to Clang target/layout information or explicit layout/extent contracts.

### E. A bit-field checker already exists

The core `disjoint_footprint` model handles shared bit-field storage units under layout contracts, but general C typed access rejects bit-fields, and the core's integration with general production auditing is also limited (Q16/Q33). This includes an R/F problem of delivering actual source places/accesses to an existing model, not simply “implement bit-field theory from scratch.”

## 7. Suggested Implementation Order

### Phase 1: Common expression elaboration and contract instantiation

Close the small operator gaps E01/E02/E03 while moving E04–E13 toward a common mechanism rather than multiplying shape-specific patches. Distinguish at least Value, Place, Read, Write, Call, Branch, Merge, and LifetimeStart/End. Apply a call contract exactly once, at the point where the name, type, and actual-argument identities have been established. Neither native QCore nor the existing T needs to be discarded.

```text
elaborate(expr, context)
  -> value/place
   + evaluation graph
   + effect/custody events
   + obligations/assumptions
```

Do not confuse sequenced-before with unspecified order. Normalization to A-normal form or similar representations must not introduce an order not guaranteed by C and then claim a universal source-level guarantee. Auditing one fixed compiler artifact is another possible policy, but its scope must be stated separately.

### Phase 2: Common typed places and tracked stores

Prioritize M01–M08/K03. Represent DeclID/FieldID, pointer views, indices, allocation/generation, and storage units so that owner/Out/Stored/Removed and related contracts can be applied to common places. Gradually adapt existing field-link and collection handling. This also reduces the limitations imposed by name strings and line-based matching.

### Phase 3: Connections to existing core checkers

Focus on M14–M21/W01/W08, connecting source-derived events and explicit contract premises to extent/layout/footprint/concurrency checkers. Do not count existing core results as results for general C. Publish the guarantees reachable through each route.

### Phase 4: Numerical and weak-memory extensions according to the intended guarantee

There is no need to reimplement the entire C numerical model first. For Fluent Bit, select from N01–N07/K05 according to priorities such as bounds/size overflow, pointer lifetime, and conditional ownership transfer. RMW/CAS/fences require explicit semantic additions to the existing weak-memory model; adding contracts alone does not produce verification of all candidate executions.

## 8. Units of Regression Verification

For every feature addition, test not only accepted cases but the following variations.

| Axis | Positive/comparison cases |
|---|---|
| Position | Standalone statement, condition, return, actual argument, nested expression, for clause |
| Effect | None/Borrowed, Owned, Released, Consumed, Out, Derived, Reallocated |
| Evaluation | Once/zero times under short-circuiting, unspecified left/right order, aliases of the same resource, volatile/atomic |
| Place | Local, formal, field, embedded field, array element, alias of the same owner |
| Type | Signed/unsigned widths, NULL, descriptor sentinels, void* views, enum/typedef |
| Boundary | No contract, pending, proposed, declared, validated, incompatible signature |
| Evidence | Changed original source, changed macro/target, changed contract digest, recheck |
| Verdict | Preservation of Safe/Unsafe/Unknown scope and assumptions |

The included probes confirm AST/IR generation on the Clang side; they are not Quirin acceptance tests. Quirin tests should be added through the existing gateway helpers and must identify the target source profile and consumer.

## 9. Included Files

| File | Contents |
|---|---|
| Quirin_Clang_Gap_Analysis_EN.md | This report |
| gap_inventory_EN.json | All classified entries and the standard C operator matrix |
| Quirin_Source_Evidence_EN.md | Q01–Q47 source excerpts with original paths and line numbers |
| Quirin_Clang_Response_EN.md | Full English translation of the preceding conversational response |
| metadata.json | Archive/excerpt SHA-256 values, comparison commit, and original execution environment |
| raw_boundary_mentions.json | Source string index; do not use its match count as an unimplemented-feature count |
| probes/clang_reference_probes.c | C probe containing 16 functions |
| probes/clang_reference_probes.ast.json | Typed AST generated by local Clang 17 |
| probes/clang_reference_probes.ll | LLVM IR generated by local Clang 17 |
| probes/probe_results.json | AST-kind/operator/cast inventory by function |
| translation_manifest.json | Translation provenance, preservation checks, and English-package checksums |

## 10. LLVM Primary Sources

The operation taxonomy is pinned to a commit; Doxygen is rolling documentation. This comparison concerns transformations performed by LLVM code generation. It does not interpret LLVM code generation as proof of all dynamic safety properties of C.

| ID | Source | URL | Purpose |
| --- | --- | --- | --- |
| L01 | Clang Internals Manual | https://clang.llvm.org/docs/InternalsManual.html | Parser/Sema/AST/CodeGen layering; rolling official docs |
| L02 | OperationKinds.def | https://github.com/llvm/llvm-project/blob/992ca79c83b0882465a202883109bd4cd167ec00/clang/include/clang/AST/OperationKinds.def | pinned operation taxonomy |
| L03 | StmtNodes.td | https://github.com/llvm/llvm-project/blob/992ca79c83b0882465a202883109bd4cd167ec00/clang/include/clang/Basic/StmtNodes.td | pinned AST taxonomy |
| L04 | CodeGenAction.cpp | https://github.com/llvm/llvm-project/blob/992ca79c83b0882465a202883109bd4cd167ec00/clang/lib/CodeGen/CodeGenAction.cpp | EmitLLVMAction / Backend_EmitLL |
| L05 | CGExprScalar.cpp | https://clang.llvm.org/doxygen/CGExprScalar_8cpp_source.html | scalar/casts/short circuit; rolling Doxygen |
| L06 | CGExpr.cpp | https://clang.llvm.org/doxygen/CGExpr_8cpp_source.html | lvalue/loads/stores; rolling Doxygen |
| L07 | CGExprAgg.cpp | https://clang.llvm.org/doxygen/CGExprAgg_8cpp_source.html | aggregate operations; rolling Doxygen |
| L08 | CGExprComplex.cpp | https://clang.llvm.org/doxygen/CGExprComplex_8cpp_source.html | complex operations; rolling Doxygen |
| L09 | CGStmt.cpp | https://clang.llvm.org/doxygen/CGStmt_8cpp_source.html | control flow and statement lowering; rolling Doxygen |
| L10 | CGCall.cpp | https://clang.llvm.org/doxygen/CGCall_8cpp_source.html | calls/ABI; rolling Doxygen |
| L11 | CGAtomic.cpp | https://clang.llvm.org/doxygen/CGAtomic_8cpp_source.html | atomic lowering; rolling Doxygen |
| L12 | CGDecl.cpp | https://clang.llvm.org/doxygen/CGDecl_8cpp_source.html | local/global storage and initialization; rolling Doxygen |
| L13 | CGRecordLayoutBuilder.cpp | https://clang.llvm.org/doxygen/CGRecordLayoutBuilder_8cpp_source.html | record and bitfield layout; rolling Doxygen |
| L14 | LLVM Language Reference | https://llvm.org/docs/LangRef.html | LLVM instruction/type/UB semantics; rolling docs |
| L15 | Clang Language Extensions | https://clang.llvm.org/docs/LanguageExtensions.html | GNU/Clang C extensions; rolling docs |
