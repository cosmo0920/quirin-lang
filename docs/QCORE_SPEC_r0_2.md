# QCore-C: Quantitative Types, Checked Concurrency, and Resource-Safe Error Unwinding

**Status:** Proposed research specification, revision 0.2; implementation and proof completion are not claimed.  
**Audience:** An implementation and verification LLM agent  
**Normative language:** “MUST”, “MUST NOT”, “SHOULD”, and “MAY” are requirements at their usual standards meanings.  
**Primary implementation target:** a small typed language that erases to straightforward C11.  
**Secondary target:** a conservative analyzer for existing C after the source-language checker and C emitter are stable.

---


## Revision 0.2 Reading Guide and Precedence

**Prepared:** 2026-09-05. **Profile:** `checked-concurrency-0.2`.

This is a consolidated proposed revision based on the supplied revision 0.1. Sections 0–37 retain the sequential design and its existing research/validation obligations. References there to “revision 0.1” designate the inherited baseline, not evidence of completed implementation. Sections 38–59 define the new concurrency profile and take precedence for concurrent programs. The uploaded source itself is unchanged.

The design adds identity-indexed quantitative access permissions, scoped tasks with explicit joins, synchronization-backed transfer, fixed-shape mutex invariants, quiescence-aware cleanup, concurrent resource bounds, and conservative C-analysis results. These are new proposals; the accompanying executable model is bounded experimental evidence, not a proof or a QCore/C implementation.

| Inherited sections | Revision 0.2 interpretation |
|---|---|
| §§1–2: objectives and non-goals | §§38, 45 define the supported race-free concurrency subset and exclusions. |
| §§3–4, 7–8: domains and structural rules | §§39–41 add access shares and custodians without conflating credit, live quantity, or usage. |
| §5: BNF | New alternatives are integrated below; their productions and three explicit syntax corrigenda are specified in §42. |
| §§10–12: exits, flow, and ownership | §§40–43, 47, 49 add parallel state, loans, and conservative overlap-epoch bounds. |
| §§14–17: error opening and cleanup | §46 additionally requires authorized cleanup and quiescent dependent tasks. No implicit blocking join destructor is added. |
| §§21–22: C lowering and analysis | §§45, 48, 50 add runtime handoff contracts, partial-order validation, and assumption-qualified analysis. |
| §§26–30, 35: research, roadmap, theorems, and acceptance | §§51–57 add concurrency-specific gates; inherited continuous Lean/no-`sorry` requirements remain mandatory. |
| §§31–34: implementation, solver, and trust | §§48–51 extend records and explicit assumptions. |
| §§36–37: references and error distinction | §§58–59 add primary sources and preserve the distinction; positive access authority also cannot be abandoned. |

**Source provenance:** `qcore_quantitative_types_spec(1).md`

```text
SHA-256: eadd3c37156e0dbf109dcb3311f3baa4a81a6abef1b2d0057b79f53ad068d3a7
```

`CHANGELOG_r0_2.md` records all assembly changes. `SPEC_r0_1_to_r0_2.patch` is a unified diff against that exact baseline.

---

## 0. Agent Execution Directive

Implement this design as a sequence of independently testable calculi and compiler passes. Do not begin with the full surface language, arbitrary dependent types, unrestricted C, resumable effects, or whole-program inference.

The central invariant is:

> A typed control-flow exit carries an explicit post-state. A recoverable error is not a permission to forget live resources. An error handler may abandon unused quantitative credits, but every live resource obligation must be destroyed, rolled back, or explicitly transferred.

The referenced prior discussion established the following constraints, which this specification preserves:

1. Usage multiplicity and resource quantity are distinct.
2. `where` clauses are restricted to plain ordered index values, not arbitrary program expressions.
3. The intended index class is conceptually `Idx + Ordered`.
4. Examples such as `quant<n where n := 30>` and `q.memory <= 4096` must have a precise meaning.
5. Quantities may be resource vectors rather than a single natural number.
6. The system is intended to check resource allocation and release.
7. The accepted source subset should lower to unsurprising C.
8. The same typed control-flow representation should later support conservative analysis of existing C.
9. Recoverable errors and exceptions must be represented in the type system.
10. A checked “quant opening” operation is permitted inside an error region.
11. Precision should be exposed selectively; ordinary code must not require pervasive full-QTT annotations.

The private conversation URL is not part of the formal input to the implementation. Where a detail was not recoverable from the available context, this document marks it as a design decision or research question instead of pretending it was previously settled.

---

## 1. Design Objective

QCore-C is a C-adjacent, ownership-aware, quantitatively annotated language with:

- first-order and restricted higher-order functions;
- algebraic data types;
- owned resources and borrows;
- restricted index refinements;
- usage grades;
- multidimensional resource quantities;
- typed recoverable errors;
- path-sensitive normal and exceptional exit states;
- checked error-region cleanup;
- proof erasure and lowering to C11.

The initial soundness goal is not “all C is safe.” It is:

> A closed, well-typed QCore program cannot duplicate, use after move, double-release, or silently leak a tracked live resource on any checked normal or recoverable-error path; its measured resource trace does not exceed its statically accepted upper bound; and its generated C preserves the behavior of the QCore reference interpreter for the supported subset.

---

## 2. Explicit Non-Goals for Revision 0.1

Revision 0.1 MUST NOT attempt to support:

- asynchronous exceptions;
- Unix signals as typed exceptions;
- `setjmp`/`longjmp`;
- thread cancellation;
- multi-shot or resumable effect handlers;
- unrestricted continuations;
- arbitrary pointer aliasing;
- untagged unions;
- arbitrary pointer arithmetic;
- data races;
- lock-free concurrency;
- arbitrary recursion without a declared measure;
- arbitrary nonlinear arithmetic in refinements;
- user-defined code execution during type checking;
- implicit proof of contracts for unknown foreign functions;
- complete inference for legacy C;
- derived-category or triangulated-category machinery in the compiler core.

These may become later research tracks. They are not prerequisites for the useful resource-safe C subset.

---

## 3. Semantic Separation of Concerns

The implementation MUST keep the following domains distinct.

### 3.1 Runtime usage multiplicity

A usage grade describes how many times a variable may or must be used by a term. It controls weakening and contraction.

Examples:

- erased: `[0,0]`;
- exactly once: `[1,1]`;
- affine: `[0,1]`;
- relevant: `[1,∞]`;
- unrestricted: `[0,∞]`.

Usage grades answer: **“How many times is this value demanded?”**

### 3.2 Static index values

An index is a pure, ordered, dimensioned nonnegative quantity used in types and constraints.

Examples:

- buffer length;
- byte count;
- descriptor count;
- iteration bound;
- stack depth.

Indices answer: **“What numeric parameter does the type depend on?”**

### 3.3 Quantitative credits

A credit is available capacity or potential. It is not itself a live external resource.

Examples:

- an allocation budget;
- prepaid gas;
- an analysis potential.

Credits answer: **“How much capacity remains available?”**

A strict credit is linear during normal computation, but a region-local remainder MAY be abandoned by a checked error opening.

### 3.4 Live resource obligations

A live obligation corresponds to an acquired resource that has identity and a destructor or rollback operation.

Examples:

- a heap block;
- a file descriptor;
- a socket;
- a lock guard;
- a transaction;
- a temporary file.

Live obligations answer: **“What must still be released, rolled back, or transferred?”**

A live obligation MUST NOT be weakened, including on an error path.

### 3.5 Resource cost summaries

A cost summary is an upper bound, not an owned token.

Examples:

- peak live heap bytes;
- total allocation bytes;
- CPU steps;
- I/O operations.

Costs answer: **“What resource bound can this computation reach or consume?”**

### 3.6 Effects

An effect row records qualitative behavior such as I/O, allocation, mutation, unsafe operations, and typed errors.

Effects answer: **“What classes of action or non-local exit can occur?”**

### 3.7 Exit states

Every control-flow exit has a post-state.

Examples:

- normal completion;
- function return;
- typed error;
- loop break;
- loop continue;
- unrecoverable panic.

Exit states answer: **“What resources and facts remain at this exact exit?”**

This separation is mandatory. In particular, do not encode ownership, memory footprint, control effects, and variable-use multiplicity as one overloaded number.

---

## 4. Static Domains

### 4.1 Kinds

The core uses the following kinds:

```text
Type
Usage
Dimension
Idx<D>
QuantMode
Quant<M>
Effect
Error
Region
Lifetime
```

`D` ranges over declared dimensions. `M` ranges over `credit`, `live`, and `cost`.

### 4.2 Dimensions

A dimension is a nominal unit.

Examples:

```text
Bytes
Count<Fd>
Count<Socket>
Steps
Nanoseconds
Depth
Unitless
```

The checker MUST reject addition or comparison across different dimensions unless an explicit conversion declaration exists.

### 4.3 `Idx + Ordered`

`Idx<D>` is a compiler-defined index kind, not a user-overloadable runtime interface in revision 0.1.

Every `Idx<D>` domain MUST provide:

- `0`;
- decidable equality;
- decidable partial order `<=`;
- addition;
- multiplication by a `Unitless` nonnegative constant or index;
- componentwise-compatible substitution;
- normalization to the supported solver theory.

The core solver theory is quantifier-free linear integer arithmetic over nonnegative integers. Therefore:

- addition is allowed;
- comparison is allowed;
- multiplication by a known constant is allowed;
- variable-by-variable multiplication is forbidden in revision 0.1;
- arbitrary division is forbidden;
- `min` and `max` MAY be elaborated into guarded constraints or retained as solver-supported terms;
- subtraction is represented as checked monus, never as unrestricted natural subtraction.

### 4.4 Usage grades

Use interval grades:

```text
U = { [l,u] | l ∈ Nat, u ∈ Nat∞, l <= u }
```

Operations:

```text
[l1,u1] + [l2,u2] = [l1+l2, u1+u2]
[l1,u1] * [l2,u2] = [l1*l2, u1*u2]
0U = [0,0]
1U = [1,1]
```

Precision weakening uses interval containment:

```text
[l1,u1] <=U [l2,u2]  iff  l2 <= l1 and u1 <= u2
```

Named surface grades:

```text
zero     = [0,0]
one      = [1,1]
affine   = [0,1]
relevant = [1,∞]
many     = [0,∞]
```

Revision 0.1 MAY implement only these five tags and saturating operations, while the formal model retains the interval interpretation.

### 4.5 Quant vectors

Let `A` be the finite set of declared resource axes. A quant vector is a finite map:

```text
q : A -> Idx<dimension(A)>
```

Missing fields equal zero.

Core operations:

```text
zero(q)
q + r               componentwise addition
join(q, r)           componentwise maximum
scale(k, q)          scalar multiplication
q <= r               componentwise order
project(q, axis)     field projection
monus(q, r)          checked subtraction, defined only if r <= q
```

There is no implicit subtraction and no implicit loss of exact credit or live quantity.

### 4.6 Resource identity ledger

Quant vectors are insufficient to release resources because a number such as `{fd_count = 2}` does not identify the two descriptors or their destructors.

The checker therefore maintains a separate identity ledger:

```text
Delta : ResourceId -> ResourceEntry
```

A resource entry contains at least:

```text
resource type
type indices
current typestate
footprint quant vector
owning variable or temporary
home error region
destructor
acquisition order
```

The aggregate live vector is derived from the identity ledger:

```text
live(Delta) = sum(entry.footprint for entry in Delta)
```

This equality is an invariant checked after every resource transition.

---

## 5. Normative BNF

The grammar below is strict BNF. `ε` denotes the empty production. The scanner supplies the lexical tokens below; Unicode escape policy and comment syntax are intentionally left to the implementation profile.

```bnf
<identifier> ::= <IDENTIFIER-TOKEN>
<index-identifier> ::= <IDENTIFIER-TOKEN>
<constant-index-identifier> ::= <IDENTIFIER-TOKEN>
<integer-literal> ::= <INTEGER-TOKEN>
<literal> ::= <INTEGER-TOKEN>
            | <FLOAT-TOKEN>
            | <STRING-TOKEN>
            | <CHAR-TOKEN>
            | "true"
            | "false"
<literal-pattern> ::= <literal>
```

`<index-identifier>` and `<constant-index-identifier>` are syntactically identifiers but are distinct elaboration classes. Name resolution MUST reject an ordinary runtime binder in either position.

### 5.1 Program and declarations

```bnf
<program> ::= <declaration-list>

<declaration-list> ::= ε
                     | <declaration> <declaration-list>

<declaration> ::= <dimension-declaration>
                | <axis-declaration>
                | <type-declaration>
                | <resource-type-declaration>
                | <error-declaration>
                | <constant-declaration>
                | <extern-function-declaration>
                | <function-declaration>
                | <concurrency-profile-declaration>

<dimension-declaration> ::= "dimension" <identifier> ":" "Idx" "+" "Ordered" ";"

<axis-declaration> ::= "quant" "axis" <identifier> ":" <axis-mode> "<" <dimension-reference> ">" ";"

<axis-mode> ::= "credit"
              | "live"
              | "cost.total"
              | "cost.peak"

<dimension-reference> ::= <identifier>
                        | "Count" "<" <identifier> ">"

<type-declaration> ::= "type" <identifier> <optional-type-parameters> "=" <type> ";"

<constant-declaration> ::= "const" <identifier> ":" <type> "=" <expression> ";"
```

### 5.2 Type and index parameters

```bnf
<optional-type-parameters> ::= ε
                             | "[" <type-parameter-list> "]"

<type-parameter-list> ::= <type-parameter>
                        | <type-parameter> "," <type-parameter-list>

<type-parameter> ::= <identifier> ":" "Type"
                   | <identifier> ":" "Idx" "<" <dimension-reference> ">"
                   | <identifier> ":" "Usage"
                   | <identifier> ":" "Lifetime"
                   | <identifier> ":" "Share"
```

### 5.3 Ordinary and resource types

```bnf
<type> ::= <atomic-type>
         | <named-type>
         | <tuple-type>
         | <array-type>
         | <slice-type>
         | <pointer-type>
         | <owned-type>
         | <borrow-type>
         | <quant-type>
         | <result-type>
         | <function-type>
         | <access-quant-type>
         | <task-type>
         | <mutex-type>
         | "never"

<atomic-type> ::= "unit"
                | "bool"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "isize"
                | "usize"
                | "f32"
                | "f64"

<named-type> ::= <identifier> <optional-type-arguments>

<optional-type-arguments> ::= ε
                            | "<" <type-argument-list> ">"

<type-argument-list> ::= <type-argument>
                       | <type-argument> "," <type-argument-list>

<type-argument> ::= <type>
                  | <index-expression>
                  | <usage-expression>
                  | <lifetime>
                  | <share-expression>

<tuple-type> ::= "(" <type-list> ")"

<type-list> ::= <type>
              | <type> "," <type-list>

<array-type> ::= "array" "<" <type> "," <index-expression> ">"

<slice-type> ::= "slice" "<" <type> "," <index-expression> ">"

<pointer-type> ::= "ptr" "<" <mutability> "," <type> ">"

<owned-type> ::= "own" "<" <type> ">"

<borrow-type> ::= "borrow" "<" <lifetime> "," <mutability> "," <type> ">"

<mutability> ::= "const"
               | "mut"

<lifetime> ::= "'" <identifier>

<quant-type> ::= "quant" "<" <quant-mode> "," <quant-expression> <optional-where> ">"

<quant-mode> ::= "credit"
               | "live"
               | "cost"

<result-type> ::= "result" "<" <type> "," <error-row> ">"

<function-type> ::= "fn" "(" <optional-parameter-type-list> ")" "->" <type> <optional-effects> <optional-quant-summary>

<optional-parameter-type-list> ::= ε
                                 | <parameter-type-list>

<parameter-type-list> ::= <parameter-type>
                        | <parameter-type> "," <parameter-type-list>

<parameter-type> ::= <type> "use" <usage-expression>
```

### 5.4 Resource type declarations

```bnf
<resource-type-declaration> ::= "resource" "type" <identifier> <optional-type-parameters> "{"
                                <resource-member-list>
                                "}"

<resource-member-list> ::= <resource-member>
                         | <resource-member> <resource-member-list>

<resource-member> ::= "repr" ":" <type> ";"
                    | "state" ":" <identifier> ";"
                    | "footprint" ":" <quant-expression> ";"
                    | "destructor" ":" <identifier> "noexcept" ";"
                    | "copy" ":" "forbidden" ";"
                    | "move" ":" "allowed" ";"
```

A resource type MUST have exactly one representation, one footprint, and one `noexcept` destructor in revision 0.1.

### 5.5 Error declarations

```bnf
<error-declaration> ::= "error" <identifier> <optional-type-parameters> "{"
                        <error-variant-list>
                        "}"

<error-variant-list> ::= <error-variant>
                       | <error-variant> <error-variant-list>

<error-variant> ::= <identifier> ";"
                  | <identifier> "{" <field-list> "}" ";"

<field-list> ::= <field>
               | <field> "," <field-list>

<field> ::= <identifier> ":" <type>

<error-row> ::= "{" <optional-error-type-list> "}"

<optional-error-type-list> ::= ε
                             | <error-type-list>

<error-type-list> ::= <type>
                    | <type> "," <error-type-list>
```

An error payload may contain owned resource values. Such an error value is itself non-copyable and must be handled linearly.

### 5.6 Functions and contracts

```bnf
<function-declaration> ::= "fn" <identifier> <optional-type-parameters>
                           "(" <optional-parameter-list> ")"
                           "->" <type>
                           <optional-where>
                           <optional-effects>
                           <optional-quant-summary>
                           <optional-contract>
                           <block>

<extern-function-declaration> ::= "extern" "fn" <identifier> <optional-type-parameters>
                                  "(" <optional-parameter-list> ")"
                                  "->" <type>
                                  <optional-where>
                                  <optional-effects>
                                  <optional-quant-summary>
                                  <optional-contract>
                                  ";"

<optional-parameter-list> ::= ε
                            | <parameter-list>

<parameter-list> ::= <parameter>
                   | <parameter> "," <parameter-list>

<parameter> ::= <identifier> ":" <type> <optional-index-alias> <optional-usage>

<optional-index-alias> ::= ε
                         | "as" "idx" <identifier>

<optional-usage> ::= ε
                   | "use" <usage-expression>

<optional-effects> ::= ε
                     | "effects" <effect-row>

<effect-row> ::= "{" <optional-effect-list> "}"

<optional-effect-list> ::= ε
                         | <effect-list>

<effect-list> ::= <effect-item>
                | <effect-item> "," <effect-list>

<effect-item> ::= <identifier>
                | "throws" "<" <type> ">"
                | "unsafe"

<optional-quant-summary> ::= ε
                           | "quant" <quant-summary-block>

<optional-contract> ::= ε
                      | "contract" "{"
                        <contract-clause-list>
                        "}"

<contract-clause-list> ::= ε
                         | <contract-clause> <contract-clause-list>

<contract-clause> ::= "requires" <constraint> ";"
                    | "ensures" <exit-selector> <constraint> ";"
                    | "quant" <exit-selector> <quant-relation> ";"
                    | <access-contract-clause>

<exit-selector> ::= "normal"
                  | "error" "(" <type> ")"
                  | "any"

<quant-relation> ::= <quant-expression> <relation-operator> <quant-expression>

<quant-summary-block> ::= "{"
                          <quant-summary-clause-list>
                          "}"

<quant-summary-clause-list> ::= ε
                              | <quant-summary-clause> <quant-summary-clause-list>

<quant-summary-clause> ::= "peak.live" <relation-operator> <quant-expression> ";"
                         | "total" <relation-operator> <quant-expression> ";"
                         | "normal.live" <relation-operator> <quant-expression> ";"
                         | "error" "(" <error-pattern> ")" "." "live" <relation-operator> <quant-expression> ";"
```

### 5.7 `where` clauses

```bnf
<optional-where> ::= ε
                   | "where" <where-item-list>

<where-item-list> ::= <where-item>
                    | <where-item> "," <where-item-list>

<where-item> ::= <constraint>
               | <identifier> ":=" <index-expression>

<constraint> ::= "true"
               | <index-expression> <relation-operator> <index-expression>
               | <quant-expression> <relation-operator> <quant-expression>
               | "(" <constraint> ")"
               | <constraint> "and" <constraint>

<relation-operator> ::= "=="
                      | "!="
                      | "<"
                      | "<="
                      | ">"
                      | ">="
```

The surface parser MAY accept `or`, but the elaborator MUST split it into separate path alternatives. The core constraint language remains conjunctive per path.

### 5.8 Index expressions

```bnf
<index-expression> ::= <index-atom>
                     | "(" <index-expression> ")"
                     | <index-expression> "+" <index-expression>
                     | <index-expression> "*" <index-constant>
                     | <index-constant> "*" <index-expression>
                     | "min" "(" <index-expression> "," <index-expression> ")"
                     | "max" "(" <index-expression> "," <index-expression> ")"
                     | "monus" "(" <index-expression> "," <index-expression> ")"
                     | "sizeof" "<" <type> ">"

<index-atom> ::= <integer-literal>
               | <index-identifier>
               | <quant-projection>

<index-constant> ::= <integer-literal>
                   | <constant-index-identifier>

<quant-projection> ::= <identifier> "." <identifier>
```

`<index-identifier>` refers only to an `Idx<D>` binder or a stable scalar reflected through `as idx`. It MUST NOT refer to an arbitrary runtime object.

### 5.9 Quant expressions

```bnf
<quant-expression> ::= <quant-atom>
                     | "(" <quant-expression> ")"
                     | <quant-expression> "+" <quant-expression>
                     | "join" "(" <quant-expression> "," <quant-expression> ")"
                     | "scale" "(" <index-expression> "," <quant-expression> ")"
                     | "monus" "(" <quant-expression> "," <quant-expression> ")"

<quant-atom> ::= "zero"
               | <identifier>
               | "{" <optional-quant-field-list> "}"

<optional-quant-field-list> ::= ε
                              | <quant-field-list>

<quant-field-list> ::= <quant-field>
                     | <quant-field> "," <quant-field-list>

<quant-field> ::= <identifier> "=" <index-expression>
```

### 5.10 Usage expressions

```bnf
<usage-expression> ::= "zero"
                     | "one"
                     | "affine"
                     | "relevant"
                     | "many"
                     | <integer-literal>
                     | "[" <index-expression> "," <usage-upper> "]"
                     | <identifier>

<usage-upper> ::= <index-expression>
                | "inf"
```

Symbolic usage intervals MAY be parsed but are out of scope for revision 0.1 inference. They require explicit annotations and solver support.

### 5.11 Blocks and statements

```bnf
<block> ::= "{" <statement-list> "}"

<statement-list> ::= ε
                   | <statement> <statement-list>

<statement> ::= <let-statement>
              | <let-index-statement>
              | <assignment-statement>
              | <expression-statement>
              | <drop-statement>
              | <move-statement>
              | <discharge-statement>
              | <guard-statement>
              | <if-statement>
              | <match-statement>
              | <while-statement>
              | <return-statement>
              | <raise-statement>
              | <break-statement>
              | <continue-statement>
              | <defer-statement>
              | <error-region-statement>
              | <unsafe-statement>
              | <thread-scope-statement>
              | <footprint-statement>
              | <with-lock-statement>

<let-statement> ::= "let" <pattern> <optional-type-ascription> "=" <expression> ";"

<optional-type-ascription> ::= ε
                             | ":" <type>

<let-index-statement> ::= "let" "idx" <identifier> "=" <scalar-expression> ";"

<assignment-statement> ::= <place-expression> "=" <expression> ";"

<expression-statement> ::= <expression> ";"

<drop-statement> ::= "drop" <place-expression> ";"

<move-statement> ::= "move" <place-expression> ";"

<discharge-statement> ::= "discharge" <identifier> ";"

<guard-statement> ::= "guard" <constraint> "else" "raise" <expression> ";"

<if-statement> ::= "if" "(" <expression> ")" <block> <optional-else>

<optional-else> ::= ε
                  | "else" <block>
                  | "else" <if-statement>

<match-statement> ::= "match" <expression> "{"
                      <match-arm-list>
                      "}"

<match-arm-list> ::= <match-arm>
                   | <match-arm> <match-arm-list>

<match-arm> ::= <pattern> "=>" <block>

<while-statement> ::= "while" "(" <expression> ")"
                      "invariant" <constraint>
                      "quant" "invariant" <quant-relation>
                      <optional-loop-bound>
                      <block>

<optional-loop-bound> ::= ε
                        | "bound" <index-expression>

<return-statement> ::= "return" <optional-expression> ";"

<optional-expression> ::= ε
                        | <expression>

<raise-statement> ::= "raise" <expression> ";"

<break-statement> ::= "break" <optional-label> ";"

<continue-statement> ::= "continue" <optional-label> ";"

<optional-label> ::= ε
                   | "'" <identifier>

<defer-statement> ::= "defer" <expression> ";"

<unsafe-statement> ::= "unsafe" <block>
```

### 5.12 Error regions and quant opening

```bnf
<error-region-statement> ::= "error" "region" <identifier>
                             <block>
                             <catch-clause-list>

<catch-clause-list> ::= <catch-clause>
                      | <catch-clause> <catch-clause-list>

<catch-clause> ::= "catch" "(" <error-pattern> ")"
                   <optional-quant-open>
                   <block>

<optional-quant-open> ::= ε
                        | "quant" "open" <identifier> "as" <identifier>

<error-pattern> ::= <pattern>
                  | "_"

<pattern> ::= <identifier>
            | <identifier> ":" <type>
            | "_"
            | <literal-pattern>
            | <variant-pattern>
            | <tuple-pattern>

<variant-pattern> ::= <identifier> "::" <identifier>
                    | <identifier> "::" <identifier> "{" <pattern-field-list> "}"

<pattern-field-list> ::= <pattern-field>
                       | <pattern-field> "," <pattern-field-list>

<pattern-field> ::= <identifier>
                  | <identifier> ":" <pattern>

<tuple-pattern> ::= "(" <pattern-list> ")"

<pattern-list> ::= <pattern>
                 | <pattern> "," <pattern-list>
```

### 5.13 Expressions

```bnf
<expression> ::= <primary-expression>
               | <unary-expression>
               | <binary-expression>
               | <call-expression>
               | <field-expression>
               | <indexing-expression>
               | <borrow-expression>
               | <move-expression>
               | <try-propagation-expression>
               | <constructor-expression>
               | <spawn-expression>
               | <join-expression>

<primary-expression> ::= <identifier>
                       | <literal>
                       | "(" <expression> ")"
                       | <unit-expression>
                       | <tuple-expression>

<unary-expression> ::= <unary-operator> <expression>

<unary-operator> ::= "-"
                   | "!"
                   | "*"

<binary-expression> ::= <expression> <binary-operator> <expression>

<binary-operator> ::= "+"
                    | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "=="
                    | "!="
                    | "<"
                    | "<="
                    | ">"
                    | ">="
                    | "and"
                    | "or"

<call-expression> ::= <expression> "(" <optional-argument-list> ")"

<optional-argument-list> ::= ε
                           | <argument-list>

<argument-list> ::= <expression>
                  | <expression> "," <argument-list>

<field-expression> ::= <expression> "." <identifier>

<indexing-expression> ::= <expression> "[" <expression> "]"

<borrow-expression> ::= "&" <expression>
                      | "&" "mut" <expression>

<move-expression> ::= "move" <expression>

<try-propagation-expression> ::= <expression> "?"

<constructor-expression> ::= <identifier> "::" <identifier>
                           | <identifier> "::" <identifier> "(" <optional-argument-list> ")"
                           | <identifier> "::" <identifier> "{" <optional-initializer-list> "}"

<optional-initializer-list> ::= ε
                              | <initializer-list>

<initializer-list> ::= <initializer>
                     | <initializer> "," <initializer-list>

<initializer> ::= <identifier> ":" <expression>

<scalar-expression> ::= <expression>

<place-expression> ::= <identifier>
                     | <place-expression> "." <identifier>
                     | "*" <place-expression>
                     | <place-expression> "[" <expression> "]"
```

---

## 6. Normative Meaning of `where`

### 6.1 Allowed terms

A `where` clause MAY mention only:

- `Idx<D>` variables;
- integer literals;
- declared index constants;
- quant-vector projections such as `q.memory`;
- `sizeof<T>` when statically known;
- the restricted index operators in the BNF;
- componentwise comparison of quant vectors.

It MUST NOT mention:

- heap reads;
- pointer dereferences;
- mutable variables;
- arbitrary function calls;
- user-defined overloaded comparison;
- floating-point values;
- closures;
- effectful computations;
- arbitrary structural fields from runtime objects.

### 6.2 Meaning of `:=`

Inside `where`, `n := e` is a definitional index binding.

It means:

```text
introduce fresh n and record n == e
```

It does not mean runtime mutation.

Example:

```text
quant<n where n := 30>
```

is accepted as scalar shorthand only when the expected axis and dimension are unambiguous. Its core elaboration is equivalent to:

```text
quant<credit, { expected_axis = n } where n := 30>
```

The scalar shorthand SHOULD be rejected when more than one axis is possible.

### 6.3 Meaning of projections

Given:

```text
q : quant<credit, Q>
```

and an axis declaration:

```text
quant axis memory : credit<Bytes>;
```

the expression:

```text
q.memory <= 4096
```

is a well-formed ordered index constraint of dimension `Bytes`.

The literal `4096` is dimension-polymorphic until constrained by the left operand.

### 6.4 Runtime-to-index reflection

A parameter declaration:

```text
size : usize as idx n
```

introduces:

- runtime value `size : usize`;
- ghost index `n : Idx<Unitless>` or a contextually declared dimension;
- singleton fact `value(size) == n`.

The reflected runtime variable MUST be immutable or converted to SSA form. Mutation creates a fresh index symbol.

A local statement:

```text
let idx n = size;
```

has the same effect for a stable unsigned scalar.

No arbitrary value is promoted into the index language.

---

## 7. Type Formation and Resource Metadata

### 7.1 Copyability

A type is `Copy` only if all of its runtime fields are copyable and it contains no:

- owned resource;
- live quant token;
- strict credit token;
- cleanup pack;
- mutable borrow;
- error payload containing any of the above.

Copying a non-`Copy` type is a type error.

### 7.2 Ownership

`own<T>` means the current variable has exclusive responsibility for the value.

Moving an owned value:

- invalidates the source place;
- transfers all resource identities and quant obligations reachable from the value;
- preserves the aggregate footprint;
- may change the cleanup order and region home.

Borrowing does not transfer ownership.

### 7.3 Resource footprint

For each resource value `x : own<R<i...>>`, the type declaration computes:

```text
footprint(x) : Quant<live>
```

The checker MUST prove that all footprint expressions are nonnegative and dimension-correct.

A resource footprint is an aggregate measure. The resource identity ledger remains authoritative for destruction.

### 7.4 Destructors

A revision 0.1 destructor MUST:

- consume the owned resource exactly once;
- be total for every valid typestate accepted by `drop`;
- be declared `noexcept`;
- not return an owned copy of the same resource;
- reduce the live ledger by exactly the resource footprint;
- leave no child resource obligations;
- have an explicit cost summary.

A fallible close operation must be split into:

1. an explicit fallible operation, such as `close_checked`;
2. a final `noexcept` destructor that chooses a fixed terminal policy and discharges ownership.

Cleanup during error unwinding MUST use only `noexcept` destructors or registered total rollback actions.

---

## 8. Quant Modes and Structural Rules

### 8.1 `quant<credit, q>`

Represents exactly `q` units of available capacity.

Allowed operations:

```text
split  : Credit(q1 + q2) -> (Credit(q1), Credit(q2))
merge  : (Credit(q1), Credit(q2)) -> Credit(q1 + q2)
spend  : (Credit(q + r), Demand(q)) -> Credit(r)
```

Rules:

- no contraction;
- no implicit weakening during normal computation;
- explicit transfer is allowed;
- a region-local remainder may be abandoned only through a checked error opening;
- outer credits not moved into the error region are not affected.

### 8.2 `quant<live, q>`

Represents an exact aggregate live obligation. In ordinary source code, live quantities SHOULD be derived from owned resources instead of manipulated directly.

Rules:

- no contraction;
- no weakening;
- no implicit subtyping to a smaller quantity;
- reduction only through verified destructor, rollback, or transfer;
- error opening exposes it for discharge, never for erasure.

### 8.3 `quant<cost, q>`

Represents an upper bound.

Rules:

- proof-erased;
- may be weakened upward: if `q <= r`, then `Cost(q) <: Cost(r)`;
- may be joined by componentwise maximum;
- cannot authorize allocation or resource use.

### 8.4 Checked monus

`monus(q, r)` is legal only with a proof of `r <= q`.

The result is exact `q - r`.

Failure to prove the order is a static error, not a saturating runtime behavior.

---

## 9. Function Types and Substitution

A function type conceptually has the form:

```text
forall indices.
  (x1 : A1 use u1, ..., xn : An use un)
  -> B
  effects E
  exits Omega
```

### 9.1 Usage substitution

If a function body uses parameter `x` with grade `u`, and argument `a` has usage context `gamma_a`, application contributes:

```text
u * gamma_a
```

If evaluating the function expression contributes `gamma_f`, total demand is:

```text
gamma_f + u * gamma_a
```

This rule generalizes pointwise over all arguments.

### 9.2 Index substitution

Instantiating a function replaces index variables in:

- argument types;
- result type;
- `where` constraints;
- resource footprints;
- exit summaries;
- cost summaries.

Substitution MUST preserve dimensions.

### 9.3 Resource-summary substitution

If a callee summary mentions `n`, calling it at index `m` substitutes `m` into every exit flow.

The caller then composes the instantiated flow with its current flow and ledger.

### 9.4 Evaluation order

QCore-C defines left-to-right evaluation for:

- function expression;
- arguments;
- tuple fields;
- struct fields;
- binary operands.

Generated C MUST introduce temporaries where necessary to preserve this order. It MUST NOT rely on unspecified C operand evaluation order.

---

## 10. Effects and Exit Maps

### 10.1 Effect rows

An effect row is a finite set containing items such as:

```text
io
alloc
mut
throws<E>
unsafe
```

Effects are qualitative and are not a substitute for quantitative summaries.

A handler removes the handled `throws<E>` alternatives and unions in its own effects.

### 10.2 Exit tags

The core exit tags are:

```text
normal
return
error<E>
break<label>
continue<label>
panic
```

`panic` is outside the recoverable-error guarantee. Revision 0.1 SHOULD lower panic to process termination and MAY skip cleanup.

### 10.3 Exit summary

The checker synthesizes:

```text
Omega : ExitTag -> finite set of ExitState
```

Each `ExitState` contains:

```text
path condition
value type, if any
ownership ledger
usage residuals
quant credit environment
live aggregate
resource-flow summary
effect row
cleanup-frame stack
```

The implementation may use a compact CFG dataflow representation rather than materializing this record literally.

### 10.4 Join

At a normal CFG merge:

- copyable value types must unify;
- owned ledgers must be equal up to alpha-renaming and equivalent typestate;
- moved/uninitialized states must agree;
- live vectors must match the owned ledgers;
- path conditions are kept as alternatives or safely weakened;
- peak and total costs join by componentwise maximum;
- usage intervals join by the least enclosing interval.

Revision 0.1 MUST reject an owned-resource merge that would require inventing an optional ownership state. A later version may introduce explicit `option<own<T>>`.

---

## 11. Resource Flow Algebra

For each path and exit, maintain an internal flow summary:

```text
Flow = (net, peak, total)
```

where:

- `net` is an internal signed vector describing change in live amount;
- `peak` is the maximum live increase above path entry;
- `total` is cumulative resource consumption.

Signed quantities are internal only. Surface `Idx` values remain nonnegative.

### 11.1 Primitive acquisition

For footprint `q`:

```text
acquire(q) = (q, q, acquisition_cost(q))
```

and a new identity is inserted into the ledger.

### 11.2 Primitive release

For footprint `q`:

```text
release(q) = (-q, zero, release_cost(q))
```

and the matching identity is removed.

Release is valid only if the exact resource identity is live and owned.

### 11.3 Sequential composition

For:

```text
F1 = (d1, p1, t1)
F2 = (d2, p2, t2)
```

define:

```text
F1 ; F2 =
(
  d1 + d2,
  join(p1, d1 + p2),
  t1 + t2
)
```

The `d1 + p2` term is checked in the internal signed domain and converted to a nonnegative peak bound.

### 11.4 Conditional composition

The condition cost is sequenced before either branch. Mutually exclusive branch peaks and totals join by maximum.

Post-ledgers must satisfy the ordinary join rule.

### 11.5 Loops

Revision 0.1 requires:

- an explicit logical invariant;
- an explicit quant invariant;
- an explicit finite bound when total cost is claimed;
- exact preservation of the owned ledger across each iteration;
- zero net live-resource change per iteration.

For a body with `net = 0` repeated at most `k` times:

```text
loop.net   = 0
loop.peak  = body.peak
loop.total = scale(k, body.total)
```

General nonzero-net loop summaries are a later research item.

---

## 12. Core Ownership Operations

### 12.1 Variable read

Reading a `Copy` variable adds one usage demand and preserves the value.

Reading a non-`Copy` value without borrow or move is rejected.

### 12.2 Move

`move x`:

- requires `x` initialized and exclusively owned;
- consumes the source place;
- transfers all reachable ledger entries;
- counts as the one demanded use for an exact-one binding.

### 12.3 Shared borrow

`&x`:

- does not transfer ownership;
- creates a shared loan;
- prohibits mutation through the owner for the loan lifetime;
- may be copied if the borrowed type permits shared access.

### 12.4 Mutable borrow

`&mut x`:

- creates an exclusive loan;
- prohibits all competing access for the loan lifetime;
- is non-copyable;
- returns ownership unchanged when the loan ends.

The first executable prototype MAY use lexical non-overlapping borrows rather than full non-lexical lifetime inference.

### 12.5 Drop

`drop x`:

- invokes the declared `noexcept` destructor;
- consumes `x`;
- removes all resource identities owned by `x`;
- subtracts the exact footprint;
- records destructor cost.

Double drop and drop after move are rejected.

### 12.6 Return

At function return, every local live resource must be:

- moved into the return value;
- moved into an allowed output parameter;
- dropped;
- or carried in a typed error payload on an error exit.

No unaccounted local resource may remain.

---

## 13. Recoverable Errors

### 13.1 Error representation

`throws<E>` is surface syntax for a typed non-resumable exceptional edge.

The core may represent it as:

```text
result<T, E>
```

plus explicit CFG error edges.

This is a compilation choice; source semantics remain non-local and typed.

### 13.2 Raise

`raise e`:

- requires `e` to inhabit an error type allowed by the current effect row;
- creates no normal exit;
- preserves the current live ledger on the error edge;
- transfers any resources moved into the error payload from the local ledger to the payload;
- seals the nearest applicable error-region cleanup frame.

`raise` is not a weakening rule.

### 13.3 `?` propagation

`e?`:

- evaluates `e`;
- extracts the normal value on success;
- propagates the typed error on failure;
- preserves the callee’s declared error post-state;
- seals any current region-local obligations on the propagated edge.

---

## 14. Error Regions and Checked Quant Opening

### 14.1 Region frame

Entering:

```text
error region r { body } ...
```

creates a frame:

```text
Rr = (
  entry ledger,
  entry credits,
  acquisition log,
  registered rollback log,
  escaped identities,
  path-specific resource flow
)
```

Each newly acquired resource whose home is `r` appends a destructor action to the acquisition log.

Dropping or moving the resource removes or transfers that action.

### 14.2 Sealed cleanup pack

When an error exits the region body, the checker creates a path-specific sealed pack:

```text
CleanupPack<r, E, LiveDelta, CreditRemainder>
```

The pack contains compile-time evidence sufficient to:

- identify all still-live region-local resources;
- invoke their destructors in the required order;
- execute registered rollback actions;
- abandon region-local strict credits;
- prove the resulting region-local live quantity is zero.

The runtime representation MAY be zero-sized. The generated C may instead use labels and initialization flags.

### 14.3 Legality of `quant open`

`quant open r as cleanup` is legal only:

- inside a catch clause attached to region `r`;
- for the pack produced by that exact region;
- once;
- while the pack is still sealed;
- when every contained live obligation has a total `noexcept` discharge action.

Opening the wrong region or opening outside a catch is a static error.

### 14.4 Meaning of opening

Opening does not directly delete live quantity.

It changes access from:

```text
sealed obligations
```

to:

```text
a linear cleanup capability
```

The handler must consume that capability with:

```text
discharge cleanup;
```

or a future verified manual decomposition.

### 14.5 Meaning of discharge

`discharge cleanup`:

1. runs path-specific destructors in reverse acquisition order unless a resource type declares another verified ordering discipline;
2. runs registered rollback actions;
3. removes every discharged identity from the live ledger;
4. reduces the region-local live vector to zero;
5. abandons remaining region-local credits;
6. consumes the cleanup capability exactly once;
7. enters the remainder of the handler under the post-cleanup baseline state plus the error payload.

The cleanup sequence MUST NOT raise a recoverable error.

### 14.6 Handler exit rule

A catch handler MUST NOT complete normally, return, or rethrow while its cleanup capability remains live.

After discharge, the handler may:

- recover and return a normal value;
- raise the same or another typed error;
- move owned values from the error payload elsewhere;
- allocate new resources under the handler’s own enclosing region.

### 14.7 Catch without quant opening

A catch clause without `quant open` is legal only if every matching error edge has:

```text
region-local live delta == zero
and
region-local strict credit remainder == zero
```

### 14.8 Nested regions

Regions are stack-disciplined.

On propagation:

- the innermost matching region handles first;
- its cleanup pack must be discharged before an error can leave that handler;
- then the outer region receives an error edge with its own pack.

This gives predictable C lowering and avoids cross-region cleanup ambiguity.

### 14.9 Resources in error payloads

A resource moved into an error payload is removed from the region cleanup pack.

The handler owns that payload resource and must drop, transfer, recover with, or rethrow it.

This permits partial-result errors without double cleanup.

### 14.10 Why unrestricted error weakening is forbidden

The following rule is unsound and MUST NOT exist:

```text
on error, discard every remaining quant obligation
```

It would treat a live file descriptor or heap allocation as if it were unused potential.

Only credit remainder is discardable. Live obligations require destructor, rollback, or transfer evidence.

---

## 15. Formal Error-Opening Rule

Use the following rule as the intended core shape.

Let:

- `Delta0` be the region entry ledger visible to the handler after local cleanup;
- `S` be the sealed path-specific cleanup stack;
- `qL` be the aggregate live footprint of `S`;
- `qC` be the region-local strict credit remainder;
- `cleanup(S)` be a total, non-throwing transition from `Delta0 + S` to `Delta0`.

Then:

```text
DESTRUCTIBLE(S)
live(S) = qL
cleanup(S) : (Delta0 + S) -> Delta0
Gamma ; Delta0 ; c : CleanupPack<r,E,qL,qC> use one |- handler exits Omega
every exit in Omega consumes c
--------------------------------------------------------------------------------
Gamma ; Delta0 + S |- catch(e:E) quant open r as c { handler } exits Omega
```

`discharge c` is the unique primitive eliminator in revision 0.1.

The implementation should encode the “every exit consumes `c`” requirement through ordinary linear-use checking rather than a special ad hoc scan.

---

## 16. Error-Region Monotonicity and Outer Resources

Automatic region cleanup is responsible only for resources homed in that region.

An operation inside the body may modify an outer resource only if its error contract specifies the resulting ownership and typestate.

For revision 0.1, one of the following must hold on every error edge:

1. the outer resource is returned in the same typestate;
2. the outer resource is moved into the error payload;
3. a registered rollback action restores it;
4. the region contract explicitly exposes the changed post-state.

The compiler MUST NOT assume that arbitrary mutation can be reversed.

---

## 17. `defer` and Rollback Registration

`defer action;` registers a total cleanup action in the current region.

Revision 0.1 requirements:

- `action` captures owned values linearly;
- `action` is `noexcept`;
- `action` is run exactly once if still registered at discharge;
- explicit cancellation or transfer consumes the registration;
- defer order is LIFO.

`defer` is useful for foreign C APIs whose handles are not yet declared as native resource types.

The preferred long-term representation is still a resource type with a destructor.

---

## 18. Example: Allocation and Error Cleanup

```text
dimension Bytes : Idx + Ordered;
dimension FdCount : Idx + Ordered;

quant axis heap_bytes : live<Bytes>;
quant axis fd_count   : live<FdCount>;
quant axis alloc_ops  : cost.total<Unitless>;
quant axis peak_heap  : cost.peak<Bytes>;

resource type File {
    repr: i32;
    footprint: { fd_count = 1 };
    destructor: file_drop noexcept;
    copy: forbidden;
    move: allowed;
}

resource type Buffer[n: Idx<Bytes>] {
    repr: ptr<mut, u8>;
    footprint: { heap_bytes = n };
    destructor: buffer_drop noexcept;
    copy: forbidden;
    move: allowed;
}

error LoadError {
    OpenFailed { code: i32 };
    AllocateFailed { requested: usize };
    ReadFailed { code: i32 };
}

fn load(
    path: borrow<'p, const, CStr>,
    size: usize as idx n
) -> own<Buffer<n>>
where n <= 4096
effects { io, alloc, throws<LoadError> }
quant {
    peak.live <= { fd_count = 1, heap_bytes = n };
    error(_).live == zero;
}
{
    error region load_r {
        let file = open_file(path)?;
        let buffer = alloc_buffer(size)?;
        read_exact(&file, &mut buffer)?;
        drop file;
        return move buffer;
    }
    catch (e: LoadError) quant open load_r as cleanup {
        discharge cleanup;
        raise e;
    }
}
```

Required path behavior:

```text
open_file fails:
    cleanup pack = empty

alloc_buffer fails:
    cleanup pack = [file]

read_exact fails:
    cleanup pack = [buffer, file]
    discharge order = buffer, then file

normal:
    file is dropped
    buffer is moved to the return value
```

The declared peak live bound is:

```text
{ fd_count = 1, heap_bytes = n }
```

The normal return transfers the buffer obligation to the caller. Every handled error path has zero region-local live obligation after discharge.

---

## 19. Example: Exact Quant and `where`

```text
let budget:
    quant<credit, { memory = n } where n := 30>
    = credit_literal(30);
```

Scalar shorthand:

```text
quant<n where n := 30>
```

is accepted only when the expected quant mode, axis, and dimension are uniquely known.

Constraint example:

```text
fn bounded(
    q: quant<credit, Q> use one
) -> unit
where q.memory <= 4096
{
    ...
}
```

`q.memory` is an index projection. The `where` clause executes no runtime code.

---

## 20. Example: Resource-Carrying Error

```text
error FlushError {
    Retryable {
        file: own<File>,
        pending: own<Buffer<n>>
    };
    Fatal { code: i32 };
}
```

When raising `Retryable`, both resources are moved into the error payload and are removed from the local cleanup pack.

The catch handler receives a linear error value. It may retry, drop both values, or rethrow the error. Copying the error is forbidden.

---

## 21. Lowering to C11

### 21.1 Erasure

The C emitter erases:

- index variables;
- `where` proofs;
- usage proofs;
- quant cost proofs;
- zero-sized cleanup capabilities;
- type-level dimensions.

It retains runtime values, resource representations, tags, and any initialization flags needed for cleanup.

### 21.2 Errors

Typed errors lower to:

- a status/tag enum;
- a tagged payload union or out-parameter;
- explicit branch edges;
- cleanup labels.

No C++ exceptions are required.

### 21.3 Error region example

A source error region similar to the previous example may lower schematically to:

```c
struct load_result load(const char *path, size_t size)
{
    struct file file_value;
    struct buffer buffer_value;
    bool file_live = false;
    bool buffer_live = false;
    struct load_error err;

    if (!open_file(path, &file_value, &err)) {
        goto qerr_load_r;
    }
    file_live = true;

    if (!alloc_buffer(size, &buffer_value, &err)) {
        goto qerr_load_r;
    }
    buffer_live = true;

    if (!read_exact(&file_value, &buffer_value, &err)) {
        goto qerr_load_r;
    }

    file_drop(&file_value);
    file_live = false;

    buffer_live = false; /* ownership moved to result */
    return load_ok(buffer_value);

qerr_load_r:
    if (buffer_live) {
        buffer_drop(&buffer_value);
        buffer_live = false;
    }
    if (file_live) {
        file_drop(&file_value);
        file_live = false;
    }
    return load_err(err);
}
```

The exact generated representation may use distinct error labels per acquisition state and omit booleans. The checker’s path-specific pack is the source of truth.

### 21.4 Required C subset discipline

Generated C MUST:

- have explicit-width integer semantics where overflow matters;
- avoid signed overflow;
- avoid out-of-bounds pointer arithmetic;
- avoid unsequenced side effects;
- initialize tagged-union discriminants before reading payloads;
- preserve source evaluation order;
- call each generated destructor at most once;
- use explicit ownership-move markers in the emitter IR;
- not expose erased proofs to runtime behavior.

### 21.5 Typed IR before C

C SHOULD NOT be the sole proof-carrying intermediate representation.

The compiler SHOULD use:

```text
typed AST
-> typed CFG/SSA with ownership, indices, effects, and exit maps
-> cleanup-explicit CFG
-> C AST
-> C text
```

C erases the distinctions needed for the proof. It is an excellent interoperability and code-generation target, but a poor sole carrier of quantitative typing evidence.

### 21.6 Translation validation

Revision 0.1 should validate each emitted function by comparing:

- typed CFG exits;
- cleanup-explicit CFG exits;
- generated C control-flow labels;
- resource-acquire and resource-release events.

A later mechanized compiler may prove this translation.

---

## 22. Analyzing Existing C

The later C analyzer reuses the typed CFG and resource-flow engine, but ingestion is conservative.

### 22.1 Front-end pipeline

```text
preprocessed C
-> C AST
-> CFG
-> SSA or memory-SSA
-> alias abstraction
-> resource API recognition
-> path-sensitive exit analysis
-> modular summaries
```

### 22.2 Required annotations

Unknown foreign and legacy APIs require contracts describing:

- ownership of arguments;
- ownership of return values;
- possible error sentinels;
- acquisition footprint;
- release footprint;
- typestate transitions;
- whether cleanup may fail;
- callback effects;
- aliasing assumptions.

Example conceptual contract:

```text
extern fn malloc(
    size: usize as idx n
) -> result<own<HeapBlock<n>>, AllocError>
effects { alloc, throws<AllocError> }
contract {
    quant normal { heap_bytes = n } == { heap_bytes = n };
    quant error(AllocError) zero == zero;
}
```

### 22.3 Modular scale

Large programs MUST be analyzed through function summaries and call-graph strongly connected components.

Do not attempt unrestricted whole-program path enumeration.

Use:

- summary caching;
- bounded path splitting;
- widening for index intervals;
- conservative unknown effects;
- user contracts at subsystem boundaries.

### 22.4 Legacy-C precision boundary

The analyzer can soundly report potential leaks or unknown states, but cannot infer exact ownership in the presence of arbitrary:

- aliasing;
- pointer casts;
- function pointers;
- inline assembly;
- hidden allocator wrappers;
- concurrency;
- `longjmp`;
- unmodeled callbacks;
- union punning.

Unknown behavior MUST become an explicit top/unknown state, not an optimistic assumption.

### 22.5 Relationship to source compilation

The same CFG checker can serve both paths:

```text
QCore source -> richly typed CFG -> accepted or rejected -> C emission
C source     -> partially typed CFG -> conservative diagnostics
```

The first path aims at proof-like guarantees. The second path aims at useful sound over-approximation under declared assumptions.

---

## 23. Category-Theoretic Boundary

The following interpretation is useful and compatible with the design:

- types and contexts are objects;
- programs are morphisms;
- a typed error computation has a result shape analogous to `T(A + E)`;
- graded usage is modeled by semiring-indexed structure;
- destructors are effectful maps from a resource type to unit;
- handlers compose normal and error branches.

However, triangulated and derived categories are not required for:

- parsing;
- decidable refinement checking;
- ownership;
- resource identities;
- cleanup insertion;
- C lowering;
- legacy-C analysis.

Any analogy between error propagation and exact triangles or mapping cones is currently a semantic research hypothesis. It MUST NOT determine revision 0.1 syntax or typing rules.

A separate metatheory track may investigate whether a chain-complex or derived interpretation gives useful compositional laws for multi-stage rollback or fault localization. It should be retained only if it produces a concrete theorem, algorithm, or simplification.

---

## 24. Bidirectional and Multi-Pass Checking

The implementation SHOULD separate checking into these passes.

### Pass A: parsing and name resolution

Produces a scoped AST.

### Pass B: ordinary type elaboration

Resolves:

- nominal types;
- generics;
- patterns;
- function calls;
- error variants;
- borrow forms.

### Pass C: index elaboration

Builds:

- index binders;
- dimension constraints;
- path predicates;
- solver obligations.

### Pass D: usage analysis

Synthesizes variable-use intervals and checks declared usage grades.

### Pass E: ownership and typestate flow

Builds the identity ledger and checks move, borrow, drop, and transfer.

### Pass F: effects and exit maps

Builds explicit normal and error edges.

### Pass G: quant flow

Computes live, peak, total, credits, and function summaries.

### Pass H: error-region discharge

Constructs sealed cleanup packs and checks every `quant open` and `discharge`.

### Pass I: cleanup-explicit CFG lowering

Introduces labels, flags, and destructor calls.

### Pass J: C emission

Erases ghost content and prints C.

Keeping these passes separate makes failures attributable and permits gradual implementation.

---

## 25. Required Diagnostics

Use stable diagnostic identifiers.

```text
Q001  non-index term in where clause
Q002  dimension mismatch
Q003  unsupported nonlinear index expression
Q004  unproved checked monus
Q005  unsatisfied where constraint

Q100  non-Copy value copied
Q101  exact-one value not consumed
Q102  exact-one value consumed more than once
Q103  use after move
Q104  move while borrowed
Q105  conflicting borrow
Q106  branch ownership states do not join

Q200  drop of non-live resource
Q201  double drop
Q202  live resource escapes normal exit
Q203  live resource escapes error exit
Q204  destructor may throw
Q205  destructor footprint mismatch
Q206  resource ledger/live-vector inconsistency

Q300  raised error not in effect row
Q301  unhandled typed error
Q302  quant open outside matching catch
Q303  quant open refers to wrong region
Q304  cleanup pack duplicated
Q305  cleanup pack not discharged
Q306  catch without required quant open
Q307  rethrow with undischarged local cleanup
Q308  fallible rollback in error cleanup
Q309  resource both in error payload and cleanup pack

Q400  loop ownership invariant not preserved
Q401  missing finite bound for claimed total cost
Q402  nonzero live-resource net change per iteration

Q500  foreign function missing ownership contract
Q501  unsupported C control transfer
Q502  alias state became unknown
Q503  unsafe block lacks declared postcondition
```

Every resource diagnostic SHOULD include:

- acquisition site;
- current exit path;
- expected destructor or transfer;
- conflicting move/drop site;
- relevant index constraints.

---

## 26. Known, Designed, and Unknown

### 26.1 Established foundations

The following are established techniques and should be treated as reusable foundations rather than novel claims:

1. Semiring-graded usage contexts and usage-sensitive substitution.
2. Linear, affine, relevant, erased, and unrestricted usage disciplines.
3. Graded and quantitative dependent typing.
4. Typed algebraic effects and typed exceptions.
5. Ownership and refinement types for low-level code.
6. C-targeting languages based on linear or uniqueness types.
7. Flow-sensitive static analysis of C.
8. Resource-bound analysis based on potential and amortization.
9. The fact that multi-shot control effects interact dangerously with captured linear values.
10. Destructor-based cleanup as a way to reconcile recoverable errors with first-class resources.

### 26.2 Concrete design decisions in this document

The following are implementable combinations of known ideas, but the exact composition is a QCore-C design decision:

1. `where` restricted to dimensioned `Idx + Ordered` expressions.
2. `:=` as definitional index binding.
3. explicit separation of usage, credit, live obligation, cost, and effect.
4. multidimensional quant vectors plus a separate identity ledger.
5. path-sensitive exit maps as the central checking representation.
6. `quant open r as cleanup`.
7. a linear sealed cleanup pack.
8. error-only abandonment of region-local strict credit.
9. prohibition of live-obligation weakening on errors.
10. path-specific generated cleanup with LIFO default.
11. typed CFG as the shared substrate for source compilation and C analysis.
12. C11 lowering through explicit tags and cleanup labels.

These decisions require validation and may change without contradicting the established foundations.

### 26.3 Open research questions

The following are not yet established for this exact system:

1. A full preservation and progress proof for the combined language.
2. A resource-safety theorem covering ownership, quant vectors, credits, error packs, and borrows simultaneously.
3. Principal inference for usage, effects, resource vectors, and path constraints.
4. Scalable inference of precise peak memory for large programs.
5. General nonzero-net resource loops.
6. Polymorphic recursion with quantitative summaries.
7. Higher-order functions capturing owned values and typed errors.
8. Fallible destructors and double-error semantics.
9. Asynchronous cancellation.
10. Resumable or multi-shot effects with quantitative resources.
11. Concurrency and lock-order/resource-order checking.
12. Sound and precise analysis of unrestricted legacy C.
13. Translation correctness from full QCore-C to full C11.
14. A useful derived- or triangulated-category semantics.
15. Whether a single abstract algebra can express all live, peak, cumulative, credit, and identity behavior without harming implementation clarity.

The six questions still explicit in the Cycle 16 formal research boundary are
promoted into the planned Cycle 17 milestone in
`docs/ROADMAP_AFTER_CYCLE_9.md`. The broader list above remains the historical
research inventory; bounded results from Cycles 13–16 are not restated as
unresolved implementation work.

---

## 27. Prototype and Proof Roadmap

Each stage must have executable tests before the next stage begins.

Lean formal verification is a continuous requirement across the roadmap, not a
one-time activity deferred to Stage 8. Every development stage MUST extend the
Lean core with definitions corresponding to the executable semantics introduced
by that stage and machine-checked lemmas for its safety-critical acceptance
criteria. Both the implementation test suite and `lake build` MUST pass before
the next stage begins. `sorry` is not accepted as verification. Stage 8 remains
the integration milestone for the reduced end-to-end operational semantics and
its global metatheory.

### Stage 0: syntax-only core

Implement:

- lexer;
- parser;
- AST;
- pretty printer;
- parse/print/parse round-trip.

Exclude resources and errors.

Acceptance:

- all BNF productions have golden tests;
- invalid `where` expressions are rejected syntactically where possible.

### Stage 1: index and dimension checker

Implement:

- dimensions;
- `Idx`;
- `where`;
- `:=`;
- quant-vector syntax;
- linear arithmetic constraint generation;
- SMT-LIB emission or an equivalent decision procedure.

Acceptance:

- `q.memory <= 4096` typechecks only when `memory : Bytes`;
- variable-by-variable multiplication is rejected;
- definitional bindings normalize;
- contradictory constraints are reported.

### Stage 2: usage grades

Implement:

- interval or five-tag usage algebra;
- variable-demand synthesis;
- exact-one, affine, and many checking;
- usage substitution for function application.

Acceptance:

- exact-one variables are neither dropped nor duplicated;
- erased variables do not occur in runtime expressions;
- branch joins are correct.

### Stage 3: ownership without errors

Implement:

- `own`;
- lexical shared and mutable borrows;
- move;
- drop;
- resource identity ledger;
- footprints;
- normal-exit checking.

Acceptance:

- no use after move;
- no double drop;
- no normal-path leak;
- aggregate live vector equals ledger sum.

### Stage 4: resource flow and bounds

Implement:

- `Flow(net, peak, total)`;
- sequential composition;
- conditional join;
- function summaries;
- bounded zero-net loops.

Acceptance:

- instrumented runtime traces never exceed accepted static peak;
- composed summaries match direct interpretation on generated small programs.

### Stage 5: typed errors without cleanup packs

Implement:

- error declarations;
- effect rows;
- `raise`;
- `?`;
- exit maps;
- catch with zero local obligations.

Acceptance:

- unhandled errors are rejected;
- error post-ledgers are path-sensitive;
- branch and catch joins are validated.

### Stage 6: error regions and quant opening

Implement:

- region frames;
- acquisition logs;
- sealed cleanup packs;
- `quant open`;
- `discharge`;
- nested regions;
- error payload resource transfer.

Acceptance:

- error-only credit abandonment works;
- live obligations cannot be abandoned;
- cleanup pack is consumed exactly once;
- partial initialization is handled correctly;
- nested cleanup order is deterministic.

### Stage 7: cleanup-explicit CFG and C emitter

Implement:

- typed CFG;
- cleanup labels;
- initialization flags or edge-specific labels;
- tagged error ABI;
- C pretty printer;
- C compiler integration tests.

Acceptance:

- reference interpreter and emitted C agree on values, error tags, and resource traces;
- sanitizers report no generated-code errors on the supported non-MSVC test corpus;
- each runtime resource identity is released at most once.

### Stage 7.5: native Windows/MSVC conformance (postponed)

After the initial C emitter is stable, validate and adapt it for native MSVC:

- select and invoke the MSVC compiler through the Rust `cc` crate;
- use MSVC-appropriate C language, warning, and runtime-checking options;
- confirm the tagged error ABI and fixed-width representations on Windows;
- compile and execute the differential C corpus on native Windows runners;
- compare values, error tags, ownership transfers, and resource traces with
  both Rust interpreters;
- add Windows-specific generated-code regression cases where required.

Acceptance:

- emitted programs compile as C with warnings treated as errors under MSVC;
- the Windows binaries match both Rust interpreters on the differential corpus;
- available Windows runtime instrumentation reports no generated-code errors;
- every returned or locally owned allocation is released exactly once.

Stage 7.5 is a portability-validation cycle, not part of Stage 7 completion.
It is intentionally postponed until native Windows/MSVC execution is available.

### Stage 8: executable formal model

Formalize a reduced core in Lean, Agda, or Rocq:

- natural indices;
- usage grades;
- owned resource identities;
- acquire/drop;
- typed error;
- one error region;
- cleanup pack;
- small-step semantics.

Prove:

- progress up to typed error;
- preservation;
- no duplication of resource identities;
- no double release;
- region cleanup theorem.

### Stage 9: legacy C analyzer

Only after Stage 7 is stable:

- C parser integration;
- resource API contracts;
- CFG/SSA import;
- conservative alias abstraction;
- summary fixed points;
- diagnostics.

Acceptance:

- finds seeded leaks/double frees/error-path leaks;
- produces no false negative on the supported annotated subset;
- reports unknown rather than assuming safety outside the model.

---

## 28. Required Unit and UI Tests

### 28.1 Positive index tests

```text
where n := 30
where n <= 4096
where q.memory <= 4096
where sizeof<u64> * n <= limit
where max(a, b) <= cap
```

### 28.2 Negative index tests

```text
where *ptr <= 10
where object.field <= 10
where f(n) <= 10
where n * m <= 10
where bytes <= fd_count
where monus(4, 8) == 0
```

The last case fails statically because monus requires proof that `8 <= 4`; it does not saturate.

### 28.3 Positive ownership tests

- allocate, use, drop;
- move into return;
- move into struct;
- shared borrow then drop after borrow ends;
- mutable borrow with no alias;
- resource moved into typed error payload.

### 28.4 Negative ownership tests

- copy owned buffer;
- drop twice;
- read after move;
- move while shared borrowed;
- mutable and shared borrow overlap;
- one branch drops while the other preserves;
- local resource remains at return.

### 28.5 Positive error-region tests

1. acquisition fails before any resource exists;
2. second acquisition fails after first succeeds;
3. operation fails after two resources succeed;
4. nested regions;
5. handler recovers after discharge;
6. handler rethrows after discharge;
7. unused region-local credit is abandoned;
8. resource is moved into error payload and excluded from cleanup;
9. explicit drop before error removes item from pack;
10. moved return resource is excluded from normal cleanup.

### 28.6 Negative error-region tests

- live obligation silently discarded;
- quant opened outside catch;
- wrong region opened;
- pack discharged twice;
- pack not discharged;
- handler returns before discharge;
- handler rethrows before discharge;
- destructor declared fallible;
- resource appears in both payload and cleanup;
- outer resource irreversibly consumed without error contract;
- manual foreign cleanup can throw.

### 28.7 Resource-bound tests

- sequential acquisitions compute correct peak;
- acquisition then release then acquisition uses maximum, not sum, for peak;
- cumulative allocation uses sum;
- mutually exclusive branches use maximum;
- bounded zero-net loop scales total but not live peak;
- nested call summary substitution;
- `sizeof<T> * n` footprint;
- multi-axis vector comparison.

### 28.8 C differential tests

For every executable positive test:

1. run the reference interpreter;
2. record result/error and resource trace;
3. emit C;
4. compile C;
5. run C;
6. compare result/error and normalized trace.

Inject failures at every fallible call position to enumerate cleanup paths.

---

## 29. Property-Based Tests

Generate small typed programs and check:

1. **Alpha invariance:** renaming bound variables does not change summaries.
2. **Substitution law:** inferred usage after substitution matches semiring composition.
3. **Ledger/vector consistency:** `live(Delta)` equals the aggregate quant vector.
4. **No duplicate identity:** a resource ID appears in at most one owner or cleanup pack.
5. **No double release:** release events are unique per ID.
6. **Exit coverage:** every interpreter exit is represented in `Omega`.
7. **Peak soundness:** runtime peak is componentwise below the static bound.
8. **Cleanup restoration:** discharge restores the region baseline modulo declared error payload transfers.
9. **Erasure stability:** ghost-only changes do not alter runtime results.
10. **C agreement:** generated C trace agrees with the interpreter.
11. **Sequence associativity:** flow composition is associative on valid flows.
12. **Credit conservation:** before error opening, split/merge/spend preserve exact credit.
13. **Error credit rule:** only credits homed in the opened region may be abandoned.
14. **Move conservation:** moving changes owner and cleanup order but not live footprint.
15. **Branch conservatism:** merged bound dominates each branch bound.

---

## 30. Formal Theorems

### 30.1 Index decidability

For the revision 0.1 index fragment, well-formedness and entailment are decidable relative to the chosen QF-LIA solver.

### 30.2 Usage soundness

If a term checks under usage declarations, every runtime variable occurrence count lies within its accepted usage interval, modulo erased static occurrences.

### 30.3 Preservation

If:

```text
State |- e : A exits Omega
```

and `e` takes one operational step, the resulting expression and runtime resource state remain well typed under the corresponding updated static state.

### 30.4 Progress up to typed exit

A closed well-typed term is:

- a value;
- able to step;
- a declared typed error;
- or an explicit panic.

It is not stuck on ownership, cleanup, or index operations.

### 30.5 Resource identity integrity

For every reachable state:

- each live resource identity has exactly one owner, payload, or sealed cleanup pack;
- no released identity remains live;
- no live identity is represented twice.

### 30.6 Normal-exit resource safety

At a checked normal function exit, every resource acquired by the function is either:

- released exactly once;
- or transferred exactly once through the declared return/output.

### 30.7 Error-opening safety

For a matching region `r`, discharging its cleanup pack:

- releases every region-local live identity exactly once;
- releases no non-region identity;
- executes actions in the declared order;
- reduces the region-local live aggregate to zero;
- consumes the pack exactly once;
- abandons only region-local credit.

### 30.8 Peak-bound soundness

For every accepted execution path, the observed resource trace is componentwise bounded by the statically reported peak and total summaries.

### 30.9 C-lowering simulation

For the supported subset, each source step or source exit is simulated by a finite sequence of generated-C abstract-machine steps with equivalent observable:

- value or typed error;
- resource acquire/release trace;
- ownership transfer;
- external call sequence.

This theorem may initially be tested by translation validation before mechanization.

---

## 31. Suggested Repository Layout

```text
qcore/
  SPEC.md

  compiler/
    src/
      lexer.rs
      parser.rs
      ast.rs
      names.rs
      kinds.rs
      dimensions.rs
      index.rs
      constraints.rs
      usage.rs
      types.rs
      effects.rs
      ownership.rs
      resource.rs
      flow.rs
      exits.rs
      error_region.rs
      cfg.rs
      cleanup_lowering.rs
      c_ast.rs
      c_emit.rs
      diagnostics.rs
      interpreter.rs
      trace.rs
    tests/
      parse/
      index/
      usage/
      ownership/
      errors/
      quant_open/
      bounds/
      c_diff/
      ui/

  proof/
    QCore/
      Syntax.lean
      Index.lean
      Usage.lean
      Resource.lean
      Effects.lean
      ErrorRegion.lean
      Semantics.lean
      Preservation.lean
      ResourceSafety.lean
      CleanupSafety.lean

  c_analyzer/
    src/
      c_frontend.rs
      annotations.rs
      alias.rs
      summaries.rs
      scc.rs
      diagnostics.rs
    tests/
      fixtures/
```

The proof assistant file extension is illustrative. Use one proof assistant consistently.

---

## 32. Implementation Data Structures

A practical initial representation is:

```text
IndexTerm
  = Var(IndexId, Dimension)
  | Lit(BigNat, Dimension)
  | Add(IndexTerm, IndexTerm)
  | Scale(BigNat, IndexTerm)
  | Min(IndexTerm, IndexTerm)
  | Max(IndexTerm, IndexTerm)
  | Monus(IndexTerm, IndexTerm)
  | SizeOf(TypeId)

QuantVector
  = ordered map AxisId -> IndexTerm

Usage
  = Zero | One | Affine | Relevant | Many

ResourceEntry
  = {
      id,
      type_id,
      type_args,
      state,
      footprint,
      owner,
      home_region,
      destructor,
      acquisition_seq
    }

ExitState
  = {
      path,
      ledger,
      credits,
      usage,
      flow,
      effects,
      cleanup_frames,
      value_type
    }

CleanupFrame
  = {
      region_id,
      baseline_ledger_projection,
      actions,
      local_credits,
      escaped_resources
    }

CleanupAction
  = Drop(ResourceId)
  | Rollback(ActionId, captured_resources)
```

Use immutable/persistent maps in the checker where convenient. Resource identities should be stable across CFG blocks.

---

## 33. Solver Discipline

The checker MUST emit proof obligations rather than silently insert runtime checks, except for explicit `guard`.

Example:

```text
guard n <= 4096 else raise TooLarge { requested: size };
```

creates two paths:

```text
success path: n <= 4096
error path:   n > 4096
```

After the guard, calls requiring `n <= 4096` typecheck.

The solver interface MUST record:

- assumptions;
- queried conclusion;
- result: proved, disproved, or unknown;
- source spans.

`unknown` is a compile error for safety obligations unless inside an explicitly declared `unsafe` block with a postcondition.

---

## 34. Unsafe Boundary

`unsafe` does not disable all checking.

An unsafe block MUST declare or infer:

- input owned ledger;
- output owned ledger for every exit;
- quant delta;
- possible errors;
- aliasing postconditions.

The checker may trust these postconditions, but it still enforces their use at the surrounding safe boundary.

Foreign contracts and unsafe postconditions are part of the trusted computing base until independently verified.

---

## 35. Acceptance Criteria for Revision 0.1

Revision 0.1 is complete only when all of the following hold:

1. The strict BNF parses.
2. `where` admits only ordered index terms.
3. Dimensions are checked.
4. Usage grades reject duplication and loss according to declarations.
5. Owned resources have identities and exact footprints.
6. Normal paths cannot leak tracked resources.
7. Typed errors produce explicit exit states.
8. Error regions produce path-specific sealed cleanup packs.
9. `quant open` is legal only in the matching catch.
10. Credit remainder may be abandoned only during matching error discharge.
11. Live obligations are always destroyed, rolled back, or transferred.
12. Cleanup actions are `noexcept`.
13. Nested region cleanup works.
14. Peak and total bounds are checked against instrumented traces.
15. The supported subset emits readable C11.
16. Generated C and the reference interpreter pass differential tests.
17. The reduced formal core has at least preservation, identity integrity, and error-opening safety proofs.
18. The legacy C analyzer is not started until items 1–16 are stable.

---

## 36. Research Basis

The design should be read against the following primary foundations:

- Robert Atkey, *The Syntax and Semantics of Quantitative Type Theory*, LICS 2018, DOI 10.1145/3209108.3209189.
- Benjamin Moon, Harley Eades III, and Dominic Orchard, *Graded Modal Dependent Type Theory*, ESOP 2021 / arXiv 2010.13163.
- Andreas Abel, Nils Anders Danielsson, and Oskar Eriksson, *A Graded Modal Dependent Type Theory with Erasure, Formalized*, 2026, arXiv 2603.29716.
- Dominic Orchard, Vilem-Benjamin Liepelt, and Harley Eades III, *Quantitative Program Reasoning with Graded Modal Types*, ICFP 2019, DOI 10.1145/3341714.
- Edwin Brady, *Idris 2: Quantitative Type Theory in Practice*, ECOOP 2021, arXiv 2104.00480.
- Wenhao Tang, Daniel Hillerström, Sam Lindley, and J. Garrett Morris, *Soundly Handling Linearity*, 2023, arXiv 2307.09383.
- Sidney Congard, Guillaume Munch-Maccagnoni, and Rémi Douence, *Linear Effects, Exceptions, and Resource Safety*, ESOP 2026 / arXiv 2510.23517.
- Ethan Chu, Yiyang Guo, and Jan Hoffmann, *Handling Exceptions and Effects with Automatic Resource Analysis*, OOPSLA 2026 / arXiv 2603.02260.
- Michael Sammler et al., *RefinedC: Automating the Foundational Verification of C Code with Refined Ownership Types*, PLDI 2021.
- Liam O’Connor et al., *COGENT: Certified Compilation for a Functional Systems Language*, arXiv 1601.05520.
- Xavier Leroy et al., CompCert documentation and semantics-preservation development.
- Frama-C and Eva documentation for modular, path-sensitive analysis of C.

---

## 37. Final Design Decision on Errors

It is structurally clean to integrate errors, but only under the following interpretation:

```text
error region
+ path-sensitive exit ledger
+ sealed cleanup pack
+ checked quant opening
+ no-throw discharge
```

The phrase “allow quant release in an error region” must be split into two rules:

```text
unused credit:
    may be abandoned by matching checked error discharge

live obligation:
    may be opened for cleanup
    may not be silently abandoned
```

This distinction is the non-negotiable soundness boundary of the design.

---

## 38. Scope, Feasibility, and Guarantee

### 38.1 Design decision

Checked multithreading is feasible as a compositional extension of the quantitative checker **provided access authority is indexed by resource identity and footprint, and all inter-thread transfers have synchronization semantics**.

Usage grades and resource-count vectors alone MUST NOT be used as a race-freedom argument. Two pointers can each be used once while racing on the same allocation. Conversely, multiple reads can be safe without exclusive ownership in every reading thread.

The extension uses bounded, splittable access permissions, ownership/lifetime obligations, synchronization contracts, and existing path-sensitive exits. Fractional permissions and concurrent separation logic are established foundations [C1, C2]. The precise integration with QCore error packs and quantitative flow is a design proposal requiring its own proofs.

### 38.2 Supported revision 0.2 core

The mandatory core supports first-order scoped tasks, move transfer, scoped shared and exclusive borrowing, statically proven disjoint access footprints, and mutex-protected fixed ownership invariants. Every successful task creation has exactly one explicit join on every checked scope-exit path.

The first executable milestone MAY support only whole-object footprints, but it MUST describe that subset as such. Full profile completion requires the disjoint-field/range rules below.

The supported C analyzer subset consists of annotated or recognized equivalents of these operations. Unstructured but provably matched create/join control flow MAY be normalized into the same task graph; unsupported patterns remain unknown.

### 38.3 Separate safety properties

`memory-race-free` means there are no conflicting accesses to a modeled C memory location by different threads without a happens-before ordering, where at least one access is non-atomic. The C11 baseline is WG14 N1570 §5.1.2.4 [C3].

`resource-conflict-free` means tracked resource operations obey their declared access and typestate protocols, including release versus use and competing exclusive transitions. This is a QCore property: it is not identical to the C definition of a data race. A kernel-managed handle may be memory-safe yet violate an application-level operation protocol.

`lifetime-safe` means no checked operation uses a resource after its lifetime ends and no resource is destroyed while a checked borrower can still access it.

The checker MUST report these claims separately from deadlock freedom, termination, fairness, linearizability, transaction atomicity, determinism, and wall-clock bounds. No latter property follows automatically from permission conservation.

### 38.4 Intended theorem

For every execution of a closed, well-typed program in this profile, every modeled non-atomic access and resource operation is authorized; conflicting operations in different threads are ordered by the supported synchronization protocol; and destruction cannot overlap a checked outstanding access. Quantitative live/peak/total bounds hold for the axes and runtime overhead covered by the selected cost model.

The theorem is conditional on valid foreign contracts, faithful primitive/runtime implementations, sound alias abstraction, correct target lowering, and the absence of execution outside the declared model. A report MUST list these assumptions. A linked component with unknown access effects prevents a whole-program guarantee for potentially affected state.

Race freedom is not complete C undefined-behavior freedom. Bounds, alignment, initialization, arithmetic, object lifetime, and other sequential validity obligations remain separate. A whole-program C-safety claim must discharge those obligations too; a race-only report must state its assumptions about them.

Safety is a property of finite execution prefixes. A blocked program may still be safe. No claim of eventual cleanup is made for an execution that never reaches a checked exit. The no-leak statement applies to completed checked normal and recoverable-error exits, not process abort or nontermination.

### 38.5 Deferred features

The safe core excludes detached tasks, asynchronous cancellation, signals as typed errors, `longjmp`, thread-only panic that bypasses cleanup, unmodeled thread-local destructors/callbacks, shared-memory interprocess concurrency, device/DMA mutation, general condition variables, barriers, reentrant mutexes, reader/writer-lock upgrades, atomic-pointer publication, lock-free reclamation, and general lock-free algorithms.

User-visible atomics are not needed for initial conformance. A later atomic profile MUST specify operation ordering and lifetime/publication protocols independently. Merely making a pointer atomic does not grant authority to its pointee. Atomics used internally by a verified synchronization wrapper belong to that wrapper's trusted implementation.

## 39. New Static Domains and Quantitative Access

### 39.1 Kinds

Add compiler-defined kinds:

```text
ObjectId       // fresh abstract allocation/storage identity
Generation     // distinguishes successive lifetimes at reused addresses
Footprint      // a finite region of memory locations or protocol cells
Share          // bounded integer permission units; not a resource dimension
ThreadId
TaskId
MutexId
SyncEventId
AccessQuant
```

These are not ordinary runtime objects and are not user-executable index computations. Resource identity is not a byte count, numeric address, C variable name, or descriptor integer.

### 39.2 Permission units

Each compilation/proof profile fixes a positive integer `FULL`; the initial default is `1024`. All linked checked modules MUST agree on this value, or be explicitly normalized and rechecked.

```text
Share = { k : Nat | 0 <= k <= FULL }
Perm(P,k) = k / FULL access authority at every location in footprint P
```

`FULL` is a scale, not the number of permitted reads. A thread may read repeatedly while retaining a positive share. The finite scale limits the number of independently allocated nonzero read shares, not the number of ordinary local reference aliases.

Shares use exact integer arithmetic, never floating point. Share constraints remain linear integer constraints: `0 < k`, `k1 + k2 == k`, `k <= FULL`. Products/division of symbolic shares are excluded. This preserves the base QF-LIA solver discipline. A rational/share-algebra generalization is optional future work, not a prerequisite.

### 39.3 Surface representation

The optional explicit ghost spelling is:

```text
quant<access, footprint_name, k>
```

This is an identity-indexed access type. It is **not** an ordinary quant vector with a new axis. Existing `quant<credit,...>`, `quant<live,...>`, and `quant<cost,...>` retain their meanings and arities.

An access quant cannot be converted to bytes, spent as allocation credit, scaled as a cost bound, copied, abandoned, or created by a numeric literal. Its only introduction rules are resource initialization and checked authority transfer/splitting. Footprint aliases resolve to compiler-established places and bounds; naming an alias creates no permission.

Ordinary source code SHOULD use existing borrow syntax and explicit task captures. Most ghost tokens remain implicit in the typed CFG.

### 39.4 Two independent structures

```text
Delta : ResourceId -> unique destruction/lifecycle obligation
Pi    : PermissionId -> (footprint, share, custodian, origin)
```

`Delta` answers who is responsible for eventual release or transfer. `Pi` answers who may access which state now. A parent may retain a frozen destruction obligation while a child holds an access share.

An `own<T>` value does not regenerate access authority while its permissions are lent out. The authority associated with ownership must actually be present and usable.

### 39.5 Concurrency type properties

Add compiler-checked properties `Send(T)` and `Sync(T)`:

* `Send(T)` permits transferring ownership and responsibility for `T` to another thread, including its destruction and all reachable state.
* `Sync(T)` permits the supported shared-borrow operations on `T` from multiple threads, subject to the access ledger and wrapper protocol.

These are inspired by, but not claimed identical to, Rust's properties [C4]. `Copy(T)` implies neither property.

Plain immutable scalars and aggregates derive these properties structurally. Owned containers require verified rules for their contents, representation, and destructor. Raw pointers, unknown foreign handles, thread-affine resources, and unsynchronized interior mutation do not derive the properties automatically.

A mutex wrapper can be `Sync` when its protected payload is `Send`, even when that payload is not directly `Sync`. This requires the mutex invariant proof; it is not a rule for ordinary shared references. Guards are thread-affine and not `Send`.

## 40. Footprints, Aliasing, and Global Custody

### 40.1 Footprint representation

A memory footprint is a finite union of typed ranges:

```text
(ObjectId, Generation, memory_location_range)
```

A protocol footprint may instead name an abstract state cell, such as a file's logical offset or a transaction's transition state. A single operation may require several memory and protocol footprints.

Storage addresses alone are insufficient because addresses and handle integers can be reused. Every lifetime introduction gets a fresh abstract generation. Pointer copying preserves provenance and does not introduce a fresh object.

### 40.2 C-compatible granularity

Whole objects are the default conservative unit. A refinement to fields or array ranges is permitted only when layout and access analysis establish distinct C memory locations.

Adjacent nonzero-width bit-fields belonging to one C memory location MUST NOT be treated as independently writable cells. Union overlap, unproven packed layouts, whole-struct copies, byte-wise access to object representations, casts, and library operations touching a wider range require overlapping footprints or an unknown result. Cache-line sharing alone is a performance concern, not a reason to merge otherwise distinct C memory locations.

For ranges in one allocation, prove bounds and disjointness using index constraints. A split at `m` requires `0 <= m <= n` and produces `[0,m)` and `[m,n)`. A half-open empty range has no access authority and must not be used to justify a load/store.

### 40.3 Pointwise conservation

For every live access-controlled location `a`:

```text
sum(k_i for all permission fragments i covering a) == FULL
```

The sum includes fragments held by running tasks, blocked tasks, mutex invariants, pending start records, completed-task capsules, suspended owner/loan frames, and sealed cleanup state. Zero-share fragments are omitted.

The checker may use `<= FULL` as its immediate non-overlap validity test, but a successful checked transition MUST also account for the missing complement in some explicit custodian. No positive share may silently disappear.

A mutex invariant and its guard are alternative custodians, never simultaneous copies. A task handle's promise of future returned authority is not current usable authority and must not be counted twice.

### 40.4 Resource custody

A live resource identity has exactly one destruction custodian, which may be a local owner, an owned aggregate, a checked mutex container, a running child's frame, an error payload, or a completion capsule.

The global live vector is the sum over unique live resource entries. Splitting permissions, creating additional pointer views, or transferring custody does not add another resource footprint.

The task handle tracks the join obligation and names a future post-state. It does not give the parent concurrent ownership of the child's resources. Task runtime storage is a separate identified resource.

### 40.5 Stable facts and interference

SSA indices reflected from copied scalars remain historical facts. A snapshot of mutable shared memory is not a persistent assertion about its current contents.

A function summary MUST identify facts invalidated by exclusive loan transfer, mutex release, and external mutation. Reacquiring a mutex reintroduces only the mutex invariant and fresh checked snapshots, not arbitrary facts from a previous critical section. Unsynchronized repeated reflection of mutable memory is forbidden.

## 41. Permission Algebra and Access Rules

### 41.1 Partial separating composition

Write `Pi1 * Pi2` for composition when fragments refer to compatible provenance and their pointwise sums do not exceed `FULL`. This is a partial commutative composition, not quant-vector `join`.

```text
Perm(P,k1+k2) <-> Perm(P,k1) * Perm(P,k2)
    when 0 < k1, 0 < k2, k1+k2 <= FULL
```

Splitting consumes the original fragment. Merging consumes both input fragments and requires the same footprint, generation, compatible loan origin, and actual custody by the merging context. A promised future fragment cannot be merged.

### 41.2 Spatial splitting

```text
Perm(P union Q,k) <-> Perm(P,k) * Perm(Q,k)
    when P and Q are proved disjoint
```

Both fragments may have `k == FULL`, because they cover different locations. This does not split the allocation's destruction obligation. Freeing the allocation requires reclamation of authority over its entire destruction footprint.

### 41.3 Reads and writes

A non-atomic read requires positive authority at every accessed location and a view whose mutability permits reading. It preserves the authority.

A non-atomic write or read/modify/write operation requires full authority at every affected location and a mutable view. It preserves the authority. Full authority alone does not turn a `const` view into a mutable reference or override the base borrow rules.

In compact form:

```text
Live(P)   ReadableView(t,P)   Pi_t supplies positive share over P
---------------------------------------------------------------- READ
State_t |- read(P) : A       // Pi_t unchanged

Live(P)   MutableView(t,P)   Pi_t supplies FULL over P
---------------------------------------------------------------- WRITE
State_t |- write(P,v) : unit // Pi_t unchanged
```

Ownership-consuming operations additionally require the corresponding `Delta` entry. Their access footprints include any state read/written by the operation and destructor.

### 41.4 Destruction

Release requires all of the following:

```text
unique destruction obligation in current custody
full authority over the destructor's declared access/reclamation footprint
all dependent loans ended
no running/completed-unjoined task retaining such a loan
valid typestate and legal destruction thread
no sealed mutex invariant still owning the resource
```

The operation consumes the appropriate authority and `Delta` entry. A destructor that also touches shared global state needs separately checked authorization for that state.

### 41.5 Local shared-reference copying

The base rule allowing a `Copy` shared borrow remains valid only as copying a runtime view associated with an existing loan group. It does not duplicate a linear permission fragment.

Multiple local views may consult one task-local permission group. Returning that group to its lender requires every associated view and reborrow to have expired. Cross-thread copying is different: task creation MUST partition authority or invoke a verified synchronization-wrapper sharing rule. No `Copy` implementation may bypass that partition.

### 41.6 Branches versus parallel tasks

At mutually exclusive CFG alternatives, retain path alternatives or require compatible exact permission states; do not add their permission amounts. At parallel composition, separate and account for the authority present in both tasks at the same time; do not take their maximum.

A branch merge cannot strengthen access by forgetting a possible outstanding loan. Unknown equality/disjointness is not permission to accept.

## 42. Normative Grammar Additions and Explicit Corrigenda

These are grammar extensions to §5, not pseudocode. Every referenced base nonterminal retains its definition unless explicitly amended. New keywords are contextual. Names for new static kinds are elaboration classes, not runtime values.

### 42.1 New declarations and types

Add `<concurrency-profile-declaration>` to `<declaration>`, and the following productions:

```bnf
<concurrency-profile-declaration> ::= "concurrency" "checked" ";"

<access-quant-type> ::= "quant" "<" "access" "," <identifier> "," <share-expression> ">"

<share-expression> ::= "FULL"
                     | <integer-literal>
                     | <identifier>
                     | "(" <share-expression> ")"
                     | <share-expression> "+" <share-expression>

<task-type> ::= "task" "<" <lifetime> "," <type> "," <error-row> ">"

<mutex-type> ::= "mutex" "<" <type> ">"
```

Add `<access-quant-type>`, `<task-type>`, and `<mutex-type>` to `<type>`. `access` is deliberately not added to the old vector-valued `<quant-mode>` production. The access spelling is a separate type constructor.

A share identifier resolves only to a `Share` binder or compile-time share constant. Add `<identifier> ":" "Share"` to `<type-parameter>` and `<share-expression>` to `<type-argument>`. Expected-kind elaboration distinguishes a share argument from an ordinary index or usage argument. Parameterized share expressions generate bounded linear constraints in the permission solver; they cannot be silently substituted for dimensioned indices. Share preconditions use the separate contract clause below, not an extension of `where` to arbitrary ghost domains.

### 42.2 Statements and footprint aliases

Add these statement alternatives:

```bnf
<thread-scope-statement> ::= "thread" "scope" <identifier> <block>

<footprint-statement> ::= "footprint" <identifier> "=" <place-expression> ";"
                        | "footprint" <identifier> "=" <place-expression>
                          "range" "[" <index-expression> "," <index-expression> ")" ";"

<with-lock-statement> ::= "with" "lock" <place-expression> "as" <identifier> <block>
```

`footprint p = x;` names the currently resolved storage/protocol footprint of `x`. It is ghost-only. The range form is measured in array elements with target-aware lowering to memory locations. A runtime pointer alone cannot establish provenance or a valid footprint alias.

`thread scope s` introduces ghost lifetime `'s`. A guard bound by `with lock` has an inferred generative guard lifetime and can be used through its checked `value` view; users do not fabricate guard types or constructors.

### 42.3 Task expressions and captures

Add these expression alternatives:

```bnf
<spawn-expression> ::= "spawn" <identifier> <identifier>
                       "(" <optional-capture-list> ")"

<optional-capture-list> ::= ε
                         | <capture-list>

<capture-list> ::= <capture>
                 | <capture> "," <capture-list>

<capture> ::= "copy" <place-expression>
            | "move" <place-expression>
            | "read" <place-expression> "share" <share-expression>
            | "write" <place-expression>

<join-expression> ::= "join" "move" <place-expression>
```

The first spawn identifier is a live enclosing scope; the second resolves to a first-order checked function. Captures are evaluated left-to-right as reservation operations. Arbitrary capture expressions and closures are intentionally excluded from this revision.

`copy` requires a copyable, transferable value with no unauthorized shared access hidden inside it. `move` reserves exclusive ownership transfer. `read ... share k` creates or subdivides a shared loan group; `write` transfers an exclusive scoped loan.

The spawn expression returns a discriminated result. Immediate `?` propagation is legal only if its failure edge satisfies all outstanding-task obligations. Deferring discrimination requires a relational exit state; the initial implementation MAY require immediate `?` or `match` and reject deferred discrimination.

### 42.4 Contracts

Add to `<contract-clause>`:

```bnf
<access-contract-clause> ::= "access" <access-action> <place-expression> ";"
                          | "access" <access-action> <place-expression>
                            "range" "[" <index-expression> "," <index-expression> ")" ";"
                          | "thread" <thread-property> <identifier> ";"
                          | "share" "requires" <share-constraint> ";"

<share-constraint> ::= <share-expression> <relation-operator> <share-expression>
                     | "(" <share-constraint> ")"
                     | <share-constraint> "and" <share-constraint>

<access-action> ::= "read"
                  | "write"
                  | "release"
                  | "atomic"

<thread-property> ::= "requires_send"
                    | "requires_sync"
```

Add `<access-contract-clause>` to `<contract-clause>`. Contracts refer to declared parameters/globals with resolved provenance. These clauses summarize accesses; ownership/post-state clauses remain mandatory. `access atomic` is reserved and MUST be rejected outside a separately implemented atomic profile.

`share requires k > 0;` is a static precondition over the separate Share domain. The caller proves it and the callee may assume it; it cannot mint permissions or replace the access ledger. Base `where` restrictions are unchanged. Compiler-inferred access requirements refer to the footprints of parameters and globals, even when their ghost permission parameters are not written in the source.

`share requires k > 0;` is a static precondition over the separate Share domain. The caller proves it and the callee may assume it; it cannot mint permissions or replace the access ledger. Base `where` restrictions are unchanged. Compiler-inferred access requirements refer to the footprints of parameters and globals, even when their ghost permission parameters are not written in the source.

Complex synchronization summaries are initially compiler intrinsics or versioned external contract records (§48), not arbitrary user-written functions in `where`. Do not invent a general assertion language during implementation.

### 42.5 Base syntax corrigenda

The base examples use constructs not fully covered by their original productions. The consolidated revision makes the following corrections explicitly:

1. Function and extern declarations accept `<optional-quant-summary>` after `<optional-effects>` and before `<optional-contract>`.
2. `<pattern>` accepts `<identifier> ":" <type>` for the typed catch binder used by the examples.
3. Unit and tuples of at least two expressions are added to `<primary-expression>`:

```bnf
<unit-expression> ::= "(" ")"
<tuple-expression> ::= "(" <expression> "," <expression> <tuple-expression-tail> ")"
<tuple-expression-tail> ::= ε
                         | "," <expression> <tuple-expression-tail>
```

The consolidated document applies these changes; parser golden tests MUST cover them. Base snippets explicitly called conceptual or schematic remain so. These repairs do not constitute evidence that the full grammar has already been parsed or tested.

## 43. Scoped Tasks, Failure, and Join

### 43.1 Spawn precondition

Before successful spawn, the checker establishes a partition of the parent's owned entries, permission fragments, and strict credits into retained and transferred components. Borrowed resources are not duplicated in `Delta`.

The worker's parameter types, `Send`/`Sync` conditions, effects, entry access requirements, normal/error post-states, and quantitative summary MUST be known. Its global accesses must also be covered. Implicit capture of mutable globals is forbidden.

A child may not receive a guard, a thread-affine destructor obligation, an ancestor's task handle, or a reference whose storage can expire before the child's join.

### 43.2 Two-phase spawn and creation failure

`spawn` is a special resource-aware primitive, not an ordinary function that blindly consumes its argument expressions before failure is known.

**Reserve:** Validate captures, create a private stable start record, reserve ghost transfers, and suspend conflicting parent access. The environment is fully initialized before the runtime can start the child.

**Success:** Commit moves, credit partitions, and loans to the child; produce one `task<'s,T,E>` handle and one corresponding join obligation. The success edge invalidates moved parent places.

**Failure:** No worker execution has occurred. Destroy only private runtime-start storage, cancel the reservations, restore the captured parent places/permissions/credits/cleanup registrations, and return `SpawnError`. Previously running siblings are unaffected.

The runtime adapter MUST satisfy this all-or-nothing contract. If its API can report failure after starting a worker, it is not a valid adapter without an additional recovery protocol.

The special restoration includes usage residuals and SSA ownership versions, not a general rollback of arbitrary expressions. Restricting captures to places makes this tractable. A failure after successful creation is a worker result, never `SpawnError`.

### 43.3 Borrowed parameters

A child can repeatedly use a shared borrow according to its positive share and the referent's shared-access protocol. A child with an exclusive borrow can mutate the permitted footprint, while competing parent/sibling accesses are prohibited.

The owner's lifetime extends through successful join, not merely through the source worker call or an observed completion flag. Borrowed referents and captures must have stable addresses where their C representation requires it.

All borrowed capture authority is returned in the worker's completion post-state on **both** normal and typed-error exits. It cannot escape in a returned view, global, detached callback, or error payload. Owned values may be returned or carried in errors if their types and reachable contents are `Send`.

### 43.4 Task handles

A task handle is a built-in linear synchronization obligation with an explicit `join` eliminator. It is not an ordinary user resource with an automatically runnable total destructor.

It cannot be copied, dropped, detached, moved to a different task, forgotten, or put into a cleanup pack that promises ordinary no-throw discharge. Moving it between local variables in the creating task is allowed. Exactly one creator-context join consumes it.

The worker must join its own descendants before completion. These restrictions make the join graph a structured parent-child graph. They do not, by themselves, prevent deadlocks involving mutexes or external I/O.

### 43.5 Completion capsule

On worker completion, checked worker-local cleanup has run. Its return value or typed error, remaining returned owned values, loan-return evidence, and quantitative post-state are placed in a completion capsule.

The worker no longer uses the capsule. The parent cannot read it or regain its authority before a successful join. Resource identities in a capsule remain globally live, even after worker execution stops.

Thread-local finalizers and foreign callbacks capable of using captured state MUST be completed before the backend declares quiescence, or excluded by contract. A user-level “worker body finished” notification alone is insufficient [C5].

### 43.6 Join

```text
join move h : result<T, E>
```

Join has `may_block` behavior. On successful completion it consumes `h`, establishes the completion-to-parent synchronization edge, transfers the capsule's result/error and owned entries to the parent, and restores the recorded borrowed permissions and post-state facts.

Returning the worker's typed error as a result does not itself raise that error. The caller may propagate it only after required sibling joins and cleanup. This distinction permits deterministic collection of multiple worker outcomes without prematurely abandoning siblings.

A backend join failure does not prove termination or restore permissions. The initial safe wrapper handles an unexpected join failure by process-wide panic/abort without reclaiming possibly accessed storage. A recoverable join-error API that preserves the handle would require a separate retry/recovery protocol and is not part of this core.

No bound on join waiting time is inferred. Join MUST NOT be registered as a base `noexcept` total destructor. `noexcept` and “cannot block forever” are different facts.

### 43.7 Scope exit

Every checked edge leaving a thread scope—fallthrough, return, error, break, or continue—must have no outstanding task handle or child loan. All task results containing owned resources must also be accounted for by ordinary rules.

The checker does not silently detach, cancel, or auto-join. An error may be caught inside the scope so the handler can explicitly join already-created children before propagating. A second spawn failure after a first success therefore needs a path that joins the first task.

A `?` that would bypass a live sibling handle is rejected even if the usual sequential cleanup pack would otherwise be valid.

## 44. Mutexes and Protected Resource Invariants

### 44.1 Initial supported invariant

A built-in `mutex<T>` owns a stable mutex identity and a protected payload with a fixed ownership shape and reclamation footprint. Its invariant records the protected live identities, access permissions, valid typestate, and any declared stable scalar refinements.

The first profile permits mutation of values and protocol state while retaining the protected ownership shape. It does not permit replacing an unbounded owned graph, transferring resource ownership out of the mutex between tasks, or changing its live footprint. Such patterns require the dynamic-invariant extension and different cost accounting.

This subset is useful for counters, fixed buffers, state records, and modeled handle operations. It intentionally does not yet certify every lock-protected growable container.

### 44.2 Closed and open states

```text
closed mutex:
    mutex invariant holds the protected authority and custody
    shared mutex handles do not directly authorize payload access

successful lock:
    invariant authority is transferred to exactly one thread-bound guard
    all other handle holders still lack payload authority

unlock:
    guard's loans end and the invariant is re-established
    authority returns to the closed mutex
```

Lock acquisition is an operation of the synchronization wrapper, not an ordinary unprotected write to its implementation fields. The wrapper's proof/contract covers those internal fields.

### 44.3 Structured locking

```text
with lock m as g {
    // checked access through g.value
}
```

The operation either acquires the lock before entering the body or raises the modeled acquisition failure with no guard. Backend invariant violations may instead use the fixed process-panic policy. The exact adapter policy MUST be recorded.

Every checked exit from the body must return the protected invariant and expire guard-derived views before unlocking. The emitter inserts the unlock action on each such edge. Nested cleanup runs in a dependency-respecting order.

A resource-bearing error may not carry protected ownership or a guard-derived borrow out of the critical section. A copied scalar snapshot or independently owned local resource may escape under ordinary error rules.

### 44.4 Error invariant restoration

The mutex invariant can be temporarily opened inside a critical section, but must hold on all checked unlock edges. A typed error is not a reason to assume it restored itself. Code must roll back, expose a permitted invariant variant in a later profile, or reject the edge.

The initial invariant language is compiler-generated ownership/typestate/footprint structure plus supported index facts, not unrestricted separation-logic formulas. A generic `defer unlock` is insufficient when resources or facts needed by the invariant have been consumed.

### 44.5 Lock lifetime and destruction

Destroying a mutex requires unique ownership of the wrapper, full recovery of all handle-lifetime loans, no guard, no blocked/waiting task with a valid handle, and a closed valid invariant. Joining all scoped handle users is one way to establish this state.

Holding a payload guard does not authorize destruction of the underlying mutex. Guard ownership is thread-affine, and unlock is performed by the legal locking thread.

### 44.6 Blocking and lock order

The initial checker rejects joining a child while holding any tracked mutex guard. This prevents one common cleanup/parent-child deadlock pattern, but is not advertised as complete deadlock freedom.

An optional lock-order mode can require strictly increasing immutable lock ranks on every nested acquisition and propagate rank requirements through calls. Its theorem concerns cycles among the modeled mutex acquisitions only; it does not establish fairness, I/O termination, or absence of unrelated waits. Unknown lock aliases/ranks fail that optional proof rather than being guessed.

## 45. Synchronization and the C Memory Model

### 45.1 Ordering evidence

The concurrent IR records events and ordering relations, including sequenced-before within a task, successful start publication, mutex release/acquire handoff, and successful task termination/join.

For this core:

```text
HB = transitive closure of sequenced-before and supported synchronizes-with edges
```

All cross-thread authority transfers must be backed by the corresponding runtime ordering. Ghost reassignment without a runtime handoff is illegal. Pointer publication is distinct from merely calculating the pointer value.

C11 thread creation/join synchronization is specified in N1570 §7.26.5 [C3]. Mutex semantics must include applicable corrections to that baseline, including the mutex total-order clarification in DR 405 [C6]. A POSIX or Windows backend must separately justify equivalent abstract operations.

### 45.2 Why this excludes modeled data races

Suppose different threads have conflicting non-atomic accesses to an overlapping location. A writing thread must possess full authority. A competing reader needs positive authority, and a competing writer also needs full authority. Those simultaneous authorizations violate pointwise conservation.

If authority instead moved between the two accesses, the only permitted inter-thread transfers establish synchronization. The accesses are therefore ordered by the transfer's happens-before chain. The proof needs both the algebra and the transfer semantics; pointwise arithmetic alone is not the whole C11 argument.

### 45.3 Explicit non-evidence

`volatile`, sleeping, elapsed time, testing a plain “done” flag, a mutex at a different identity, or a successful allocation does not create the required ordering. A raw pointer cast, `const` qualifier, or descriptor-number inequality does not prove disjoint resource state.

A successful join orders the terminated child's effects; it does not retroactively make earlier parent/child conflicting accesses safe.

### 45.4 Atomic boundary

An atomic location must not be accessed non-atomically by bypassing its wrapper. Even a future atomic profile must track initialization, destruction, pointee lifetime, and resource ownership separately.

Release/acquire publication, compare-exchange ownership transfer, reference counting, hazard pointers, epochs, and ABA prevention require explicit protocols. None is certified by a numeric access fraction or an atomic-pointer annotation alone. The initial analyzer reports unsupported such protocols as unknown.

## 46. Recoverable Errors and Checked Quant Opening

### 46.1 Preserve the base credit/live distinction

A matching error discharge may abandon a region-local unused credit remainder. It may not abandon a positive access share, a live resource identity, a task handle, a guard invariant, or a return obligation for a borrowed capture.

A credit transferred to a child is no longer a parent-local remainder. Parent error handling cannot abandon it on the child's behalf.

### 46.2 Extended cleanup precondition

A concurrent cleanup pack records access dependencies in addition to the base action stack:

```text
CleanupPack<r,E,LiveDelta,CreditRemainder,AccessDependencies>
```

Opening for ordinary discharge requires a valid cleanup plan. For each destructor/rollback step, the plan proves its permission, loan, typestate, and thread-affinity preconditions at that point.

```text
NoOutstandingTaskInPack(S)
CleanupPlanAuthorized(Pi,S)
cleanup(S) : (Delta0 + S, Pi) -> (Delta0, Pi0)
EveryExitConsumes(cleanup_capability)
------------------------------------------------------------ QUANT-OPEN-CONCURRENT
matching catch may open and discharge S
```

The post-state need not make all global permissions zero; returned outer loans and mutex invariants remain valid. Only obligations local to the discharged region are eliminated or returned according to their rules.

### 46.3 Quiescence before destruction

For a resource borrowed by a child, joining that child and collecting its loan-return evidence is a prerequisite to destroying the referent. The compiler MUST NOT insert an ordinary destructor before that prerequisite.

The safe pattern is to catch within the still-live thread scope, join outstanding tasks, collect their results, and only then discharge resource cleanup or propagate errors. A region outside a thread scope cannot receive a checked error with unjoined tasks from inside it.

A completed-but-unjoined task does not satisfy the requirement. The parent still lacks the join event and returned authority.

### 46.4 Cleanup dependencies

The base LIFO discipline remains the default order among actions whose dependencies permit it. The checker additionally verifies edges such as:

```text
expire guard-derived views -> restore invariant -> unlock guard
expire local loans -> drop owner
join child (explicit, before ordinary discharge) -> drop borrowed owner
```

The initial implementation rejects a cleanup frame when reverse acquisition order cannot satisfy its dependencies; it need not invent a general cleanup scheduler. A later verified scheduler may use a dependency DAG with stable reverse-acquisition tie-breaking.

No blocking join is smuggled into a `noexcept` cleanup pack. This preserves the base requirement that ordinary cleanup eliminators are total and non-throwing on their accepted states.

### 46.5 Multiple errors

Worker typed errors remain ordinary, possibly linear, result values until collected. The parent can select a primary error by a declared deterministic policy, but must still discharge or transfer resources carried by other errors/results.

Discarding a secondary error is legal only when its payload is discardable. It must not lose owned resources or task obligations. Concurrent errors do not trigger asynchronous stack unwinding in another task.

### 46.6 Panic policy

The initial process-panic policy terminates the whole process and is outside the recoverable cleanup guarantee. It must not merely terminate one worker and leave other tasks running against reclaimed state. No error-opening theorem is claimed for OS termination or unmodeled asynchronous interruption.

## 47. Concurrent Quantitative Flow

### 47.1 Do not reuse conditional maximum

Mutually exclusive branches join costs by maximum. Concurrent computations may overlap, so their independent incremental peaks require addition in the conservative bound.

For an overlap epoch with unchanged shared footprint, no ownership handoff between participants during the epoch, disjoint participant-owned changing resources, and valid per-participant summaries `Fi = (di,pi,ti)`:

```text
epoch.net   = sum(di) + runtime_overhead_net
epoch.peak <= sum(pi) + runtime_overhead_peak
epoch.total <= sum(ti) + runtime_overhead_total
absolute_live_peak <= live_at_epoch_entry + epoch.peak
```

The entry union is counted once, including shared allocations. This is a conservative upper bound, not an equality of observed peaks. Backend overhead terms must be dimensioned and included exactly once. The overhead net is the signed change in separately modeled runtime live storage across the epoch; it may be set to zero only when that equality is proved. Storage already included in a participant summary or the entry baseline must not be charged again.

### 47.2 Scope of the simple formula

The formula applies to the structured, fixed-footprint-invariant core. It is not a universal law for arbitrary concurrently communicating resource flows.

A dynamic shared container, ownership-carrying channel, or transfer between running tasks can change which local baseline an allocation/release belongs to. Such a profile needs explicit global phase accounting or conserved allocation reservations that bound all interleavings. Applying the simple formula without its side conditions is forbidden.

### 47.3 Returned resources and accounting phases

A child returning an owned resource keeps that resource live in its completion capsule. Thread completion does not release it. At join, custody moves to the parent without changing the live vector. Subsequent phases use the resulting global baseline.

Transfer and permission-split events have zero net live footprint unless they also allocate distinct runtime storage. Child start records, result capsules, and thread handles need their own resource entries and release points.

### 47.4 Credits

Before fork, exact credits are partitioned among participants. Two tasks cannot each spend the original full budget. A shared dynamic credit pool requires a verified synchronized protocol and is not implicit in `credit.split`.

Credit and access splitting are different operations: one partitions future resource capacity, the other partitions authority over existing state. Neither implies the other.

### 47.5 Axes and unsupported bounds

Useful additional axes include outstanding task records, configured stack reservation, aggregate work steps, and synchronization operations. OS thread memory or stack usage is claimed only when the backend's model supplies a justified bound. It is not inferred from the presence of `thrd_create`.

Aggregate CPU work may sum across tasks. Wall-clock latency does not automatically become either the sum or maximum of their work bounds. Waiting and scheduling time are unknown without a separate timing model. Race-safety acceptance need not be blocked by an unclaimed timing axis, but a claimed bound must fail if its proof is unknown.

### 47.6 Required numerical checks

Two readers of one 4096-byte allocation still account for 4096 live bytes, not 8192. Two independent computations whose incremental peaks are 3072 and 5120 bytes require up to 8192 additional bytes, not 5120, absent a stronger non-overlap proof. Two tasks cannot each receive 100 units from a single 100-unit credit without an explicit capacity increase.

## 48. Conservative Analysis of Existing Multithreaded C

### 48.1 Extended ingestion pipeline

```text
C AST / CFG / memory SSA
-> provenance and conservative points-to abstraction
-> resource and synchronization API recognition
-> thread entry/callback resolution and may-happen-in-parallel relation
-> access footprints and memory/protocol effects
-> ownership, permission, lock and lifetime obligations
-> path-sensitive normal/error/task-completion summaries
-> modular fixed point and assumption-qualified diagnostics
```

The analyzer MUST resolve the actual selected platform/backend and contracts. Similar function names are not evidence of equivalent synchronization. Contracts may be supplied in versioned sidecar metadata where source annotations are impractical.

### 48.2 Minimum summary record

```text
ConcurrencySummary {
  parameter ownership and aliases;
  normal/error/post-completion ownership states;
  read/write/release/protocol footprints, including globals;
  required and returned permissions;
  acquired/released mutex identities and guard ownership;
  tasks created/joined, with creator and entry function;
  capture lifetime and escape conditions;
  publication and completion synchronization edges;
  callback targets and whether callbacks can run after return;
  thread-affinity / Send / Sync obligations;
  may_block and unsupported-control effects;
  quant summaries plus side conditions;
  trust/proof status, source spans, and contract version;
}
```

Foreign-resource operations are classified by actual protocol effects, not their names or C parameter constness. Reading a handle may mutate its logical state. Multiple descriptors may alias a shared underlying state cell while retaining distinct close obligations; a `dup`-like contract must represent both identities. Unknown sharing is not modeled as independent fresh resources.

### 48.3 Thread creation and joining in C

Recognized C/POSIX/Windows calls need contracts for failure, success, entry arguments, lifetime, and completion. A failed create must not produce a live join obligation. A failed join must not produce quiescence evidence. Unsupported detached or cancelled executions remain unknown.

Arrays/loops of thread handles require a bounded collection model or an invariant preserving the set of outstanding tasks. This is a separate inference milestone, not automatic from scalar handle support.

### 48.4 Alias and parallelism conservatism

Potential aliases are treated as overlapping until disjointness is proved. Holding `m1` while another thread holds `m2` does not protect a shared object merely because the variable names differ. Aliased lock identities must unify.

May-happen-in-parallel analysis must include all reachable thread entry functions and modeled callbacks. An unresolved callback that might access shared state invalidates the corresponding race-freedom claim.

### 48.5 Results and claim boundaries

For each property, return one of:

```text
PROVED_UNDER_ASSUMPTIONS
VIOLATION_WITNESS
POTENTIAL_VIOLATION
UNKNOWN
```

A rejected permission assignment is not automatically proof of an actual C race: it can be a conservative false positive or insufficient annotation. `VIOLATION_WITNESS` needs a validated feasible execution/trace under the modeled semantics. Timeout, unsupported control flow, unresolved aliasing, or a missing foreign summary is `UNKNOWN` or a potential violation, never proof of safety.

For whole-program reporting, every reachable relevant access must be covered. Unknown components are surfaced even when the analyzer proves individual modules. Existing contracts/unsafe blocks are listed as assumptions, not silently treated as verified implementations.

### 48.6 Relation to existing capability analyses

Clang documents compile-time capability checking, shared/exclusive access requirements, and guarded resources [C7]. This is useful evidence that annotation-driven checking is practical. It is not a proof that QCore's stronger ownership, cleanup, permission accounting, or C-ingestion obligations have been solved, and it does not replace QCore's analysis engine.

## 49. Checker Passes and Data Structures

### 49.1 Pass integration

Retain the separate base passes. Extend ordinary elaboration with `Send`/`Sync` and built-in concurrency types. Extend index analysis with a separate bounded-share constraint context. Extend ownership flow with custodians, frozen owners, and child-result states.

Add access-footprint elaboration before checking memory operations. Add task/lock protocol analysis before finalizing exit maps and cleanup legality. Concurrency, ownership, and alias facts may require a controlled fixed point in the C analyzer; they are not assumed available from a single superficial syntax pass.

Quant-flow computes overlap epochs only after the task graph and ownership-transfer side conditions are known. Cleanup lowering runs only after all permission and quiescence requirements are discharged.

### 49.2 Core records

```text
PermissionEntry {
  id, footprint, share, custodian, loan_origin, origin_span
}

Custodian = ThreadFrame(ThreadId)
          | SuspendedLoan(LoanGroupId)
          | MutexInvariant(MutexId)
          | PendingStart(TaskId)
          | CompletionCapsule(TaskId)
          | CleanupFrame(RegionId)

TaskEntry {
  id, creator, scope, worker, state,
  capture_reservations, owned_inputs, loan_inputs, credit_inputs,
  normal_post, error_posts, returned_loan_contract,
  completion_event, runtime_resources
}

TaskState = Reserved | Running | Completed | Joined

MutexEntry {
  id, owner, protected_invariant, state,
  handle_loans, guard_owner, runtime_contract, optional_rank
}

AccessEvent {
  thread, footprint, operation, provenance,
  authorization, synchronization_context, source_span
}
```

Extend `ExitState` with permission state, active/suspended loans, outstanding tasks, held guards, stable shared facts, and synchronization obligations. Extend `CleanupFrame` with an authorization/dependency plan. Preserve the original ownership/live-vector equality over unique resource identities.

`TaskEntry` summaries contain references to future ownership states, not duplicate live `ResourceEntry` objects in the parent's current ledger.

### 49.3 Solver interface

Every access/disjointness/share obligation records assumptions, conclusion, proof result, source spans, and the enabling protocol transition. Preserve proved/disproved/unknown distinctions.

No unsound widening may strengthen permissions. Widening can lose a proof and report unknown. Cached summaries are keyed by relevant type/contract versions, target layout, FULL, effect assumptions, and concurrency profile.

## 50. C Runtime and Lowering Contract

### 50.1 Target architecture

```text
typed concurrent CFG
-> task/lock/cleanup-explicit CFG
-> C AST + small backend synchronization ABI
-> C11 code and selected runtime adapter
```

Proofs, shares, footprint aliases, and ghost custody records erase. Real start records, thread handles, locks, result tags, storage-lifetime flags, and required synchronization do not erase.

The initial portable target may use `<threads.h>` where available. POSIX and native Windows backends are separate adapter implementations with separate conformance tests. The base postponed Windows/MSVC validation remains postponed until actually executed.

### 50.2 Start record

The emitter initializes the entire child environment before calling the backend. After successful creation, fields accessible by the worker are not concurrently modified by the parent. ABI handle output storage and worker-owned input fields must not overlap in an unprotected way.

A stack-allocated environment is allowed only when its stable storage outlives successful join on every checked path. Moving a C struct after giving its address to a child is forbidden unless the address is not observed or the representation is intentionally indirect.

On creation failure, the wrapper performs the declared rollback of reserved captures and internal storage. It must not read a successful child result on that path.

### 50.3 Completion record

The child writes its result/error exactly once after its own required cleanup. The parent reads it only after successful join. The completion record and owned result storage stay live until transferred or destroyed by the parent.

The runtime handles unexpected join failure without treating the result as initialized and without freeing the environment. A process-abort policy is acceptable at this out-of-model boundary; an optimistic “continue cleanup” policy is not.

### 50.4 Locks and cleanup

Lock/unlock calls are explicit IR operations with identity and invariant transfers. The optimizer must not move protected accesses outside their authorized region or erase ordering because the permission token itself is ghost-only.

All generated unlock/error edges are translation-validated. Guard lifetime and same-thread release constraints survive erasure through correct control flow and runtime API selection.

### 50.5 Concurrent translation validation

The sequential corpus still compares exact normalized traces where appropriate. Concurrent executions need not choose the same schedule or produce the same total event ordering.

The concurrent obligation is trace refinement or equality of permitted behaviors for a bounded model: preserve per-thread order, synchronization edges, ownership transfers, values/errors allowed by the source semantics, and quantitative bounds. Compare partial-order event graphs or replay a controlled schedule. Do not fail a correct implementation solely because independent events interleave differently.

A test logger must not accidentally serialize the whole program and hide races. Production-model instrumentation should use per-thread buffers or explicitly account for logging synchronization.

## 51. Formal Obligations

The following are theorem targets, not already-proven theorems. Lean development remains mandatory at every implementation stage, as in the base execution directive. No `sorry`, invented solver result, or testing-only substitution is accepted as proof completion.

### 51.1 Permission algebra

Prove validity of partial composition, pointwise conservation, split/merge inverses on valid fragments, spatial-split preservation, and impossibility of simultaneous full-write plus positive competing access on one location.

### 51.2 Concurrent preservation

A step of any task, lock protocol, start/complete/join transition, or checked cleanup preserves global resource identity uniqueness, permission conservation, valid custody, and compatible local typing.

### 51.3 Authorization and ordering

Every checked access has authority. Every inter-thread transfer that can enable a conflicting access is backed by an HB-producing protocol transition. Derive memory-race freedom for the modeled C-compatible footprint abstraction.

### 51.4 Lifetime and quiescence

Prove that destruction cannot occur with an outstanding dependent loan, and that a returned borrowed authority is usable by the parent only after a successful join. A merely completed task is insufficient.

### 51.5 Error preservation

Worker normal/error completion returns exactly the declared loans and accounts for owned payloads. Parent error propagation cannot bypass outstanding tasks. Error opening discards only eligible local credit and preserves access conservation for surviving state.

### 51.6 Mutex invariant transfer

Prove that the protected authority has exactly one custodian, guard-derived borrows cannot outlive unlock, and every checked unlock restores the invariant. Guard transfer to another task is impossible in the safe core.

### 51.7 Bounds

Prove the overlap-epoch peak bound under its stated side conditions, and prove that shared allocations and returned-result capsules are counted exactly once. Unsupported dynamic transfer patterns cannot use this theorem.

### 51.8 Progress qualification

Replace an unqualified concurrency progress claim with: a well-typed configuration is completed, can take a legal step, has a declared typed exit, has process panic, or is blocked at an explicitly modeled synchronization/I/O operation. It is not stuck due to a missing permission or invalid resource transition.

Deadlock freedom is not silently bundled into this theorem. A lock-rank theorem, when implemented, has its own hypotheses and scope.

### 51.9 Lowering and trust

Prove or translation-validate the reduced concurrent CFG's behavior against the selected C/runtime model. An SC interleaving model alone is not a proof for arbitrary C11 atomics. The initial theorem applies only to the synchronization-disciplined supported subset and its validated adapters.

Track solver soundness, foreign contracts, runtime adapters, C compiler assumptions, and alias abstraction explicitly in the theorem statement/trust manifest.

## 52. Established, Designed, and Unvalidated

| Category | Content | Status in this revision |
|---|---|---|
| Established foundation | Fractional read permissions/full write permission | Prior research [C1] |
| Established foundation | Resource invariants and compositional concurrent reasoning | Prior research [C2] |
| Established interface ideas | Transfer/share type properties and scoped task lifetimes | Primary Rust documentation [C4, C5] |
| Platform foundation | C data-race and synchronization semantics | WG14 baseline and corrections [C3, C6] |
| Practical precedent | Annotation-driven capability checking | Clang documentation [C7] |
| QCore design | Separate `AccessQuant`, bounded share scale, identity/custody ledgers | Proposed here |
| QCore design | Transactional place-only spawn reservations and explicit join | Proposed here |
| QCore design | Quiescence-aware quant opening and no implicit join destructor | Proposed here |
| QCore design | Fixed-footprint mutex invariant profile and overlap epochs | Proposed here |
| Empirical evidence | Supplied small permission model and executed tests | Limited to the report accompanying that model |
| Unvalidated integration | Full source checker, scalable C analysis, C lowering, continuous Lean proofs | Required work; not claimed complete |
| Later research | Dynamic resource invariants, lock-free protocols, scalable inference and richer sharing | Outside this revision's safe core |

The supplied model is deliberately smaller than the normative language. Its tests cannot establish compiler correctness, general race freedom, liveness, or any Lean theorem.

## 53. Implementation and Proof Roadmap

Preserve the base stage dependencies, especially stable ownership/error handling and typed CFG before extending the C analyzer. Concurrency work is not evidence that unfinished sequential stages are complete.

| Stage | Executable deliverable | Lean deliverable and acceptance gate |
|---|---|---|
| C0 | Whole-object permissions, FULL arithmetic, explicit split/merge, authorization checks | Partial-composition and exclusivity lemmas; exhaustive small-share tests |
| C1 | Structured task graph, move/read/write captures, spawn success/failure, explicit join | Reservation conservation, fork/join custody preservation, no early loan recovery |
| C2 | Child typed results, error-side loan returns, scope exits, cleanup dependency checking | Error preservation and quiescent destruction; every failure position tested |
| C3 | Built-in fixed-footprint mutex wrapper and structured guards | Invariant transfer, non-Send guard, unlock-edge correctness |
| C4 | Typed fields/ranges, proven disjointness, access-effect summaries | Spatial splitting and footprint soundness for the accepted layout subset |
| C5 | Concurrent flow epochs and explicit runtime cost axes | Conditional peak theorem, unique-resource accounting, credit partitioning |
| C6 | C11 runtime adapter and concurrent C emitter | Reduced operational simulation/translation-validation evidence; schedule-aware differential tests |
| C7 | Annotated C importer, task/lock summaries, conservative MHP and alias checks | Sound abstraction obligations; seeded positive/negative/unknown fixtures |

Every stage requires implementation tests and a successful `lake build` of its proof increment. Native Windows threading is an additional backend gate only after actual Windows execution; do not mark it complete from Linux tests.

Later extensions have separate gates for dynamic task collections, condition variables, ownership-carrying queues, dynamic mutex ownership invariants, atomics/publication, and reclamation. Do not expand the accepted subset merely because a grammar token exists.

### 53.1 Cycle 18: Libevent event-driven verification

Cycle 18 is a planned verification profile built on the Cycle 17 native
concurrent backend and C11 weak-memory relation. Its complete implementation
and proof plan is normative roadmap material in `cycles/CYCLE_18.md`.

The profile models a Libevent-style reactor rather than equating an event loop
with sequential execution. An `event_base` has at most one active dispatcher,
but separate bases, worker threads, immediate callbacks, and cross-thread API
calls may execute concurrently. Readiness, timeout, signal, and equal-priority
activation order are nondeterministic. Priority constrains dispatch selection;
it does not establish happens-before, fairness, or ownership transfer.

The checked event state contains:

```text
EventBaseState =
  base identity + dispatcher custody + loop mode + priority queues
  + registered events + active callbacks + deferred callbacks

EventState =
  event identity + generation + base identity + trigger set + priority
  + persistence + lifecycle + callback summary + callback-argument custody

EventLifecycle = initialized | pending | active | running | finalizing | freed
```

Transitions cover construction/assignment, add/reschedule, readiness or
timeout activation, explicit activation, callback entry/exit, persistence or
one-shot removal, blocking deletion, `EV_FINALIZE` nonblocking deletion, loop
break/exit, explicit finalization, and free. Every transition preserves
generation validity, base membership, callback-argument lifetime,
file-descriptor custody, quantitative resource ledgers, and the Cycle 17
access-permission invariant. Deletion prevents a queued callback from starting;
only a documented blocking delete result or completed finalizer establishes
quiescence for a callback already running elsewhere. Freeing an event base must
account for callback arguments and owned application resources separately from
Libevent's internal event storage.

Cross-thread operations are accepted only under an initialized and supported
Libevent threading configuration. Internal base or bufferevent locking
protects the documented library object state; it does not automatically
authorize or order arbitrary application payload accesses. Immediate,
deferred, lock-held, and unlocked bufferevent callbacks have distinct checked
contracts. Events on multiple bases and callbacks that submit work to a task
queue use the ordinary Cycle 17 may-happen-in-parallel, mutex, atomic, and
custody rules.

The acceptance theorem requires lifecycle preservation, callback access
authorization, same-base dispatch exclusion, conflict ordering across bases
and workers, cancellation/finalization safety, quiescent teardown, and trace
refinement between annotated C, the event IR, and a pinned Libevent build.
Testing must include bounded schedule exploration, priorities, persistent and
one-shot events, recursive activation, loop break/exit, event deletion and
free during activation, cross-thread notification, bufferevent callback modes,
fault injection, sanitizers, and backend-specific differential runs. Unknown
callbacks, unsupported Libevent APIs or build configurations, and unresolved
callback-argument aliases must produce `Unknown`, never a safety certificate.

The upstream behavior baseline is the official
[Libevent API documentation](https://libevent.org/doc/), including the
[event-loop](https://libevent.org/libevent-book/Ref3_eventloop.html),
[event lifecycle](https://libevent.org/libevent-book/Ref4_event.html),
[threading](https://libevent.org/libevent-book/Ref1_libsetup.html), and
[bufferevent](https://libevent.org/libevent-book/Ref6_bufferevent.html)
contracts. Cycle 18 must pin an exact upstream release or commit and record the
enabled backend and thread configuration in every certificate.

## 54. Stable Diagnostics

Retain base identifiers and add:

```text
Q600  access lacks a positive permission
Q601  mutation requires full permission
Q602  permission over-allocation or unsupported composition
Q603  permission merge has mismatched identity/generation/origin
Q604  overlapping or unproved-disjoint footprints
Q605  destruction while a loan or task access is outstanding
Q606  non-Send value crosses a task boundary
Q607  non-Sync value shared between tasks
Q608  guard or thread-affine obligation crosses threads
Q609  task handle copied, dropped, detached, or escaped
Q610  checked scope exit has an outstanding task
Q611  spawn failure does not restore its reserved state
Q612  worker exit does not return borrowed capture authority
Q613  join fails to establish completion/quiescence
Q614  critical-section exit fails to restore invariant
Q615  guarded access uses the wrong mutex identity
Q616  guarded view or protected ownership escapes unlock
Q617  cleanup lacks access authority or dependency ordering
Q618  error opening attempts to abandon access authority
Q619  join attempted while holding a tracked mutex guard
Q620  unsupported atomic/publication/reclamation protocol
Q621  unknown parallel access, callback, or alias state
Q622  resource-bound composition lacks concurrency side conditions
Q623  shared allocation or task result counted more than once
Q624  stale shared-state refinement reused after interference
Q625  optional lock-order proof failed or is unknown
Q626  backend/profile/permission-scale incompatibility
Q627  permission reconstructed from a pointer, literal, or copied view
Q628  unsupported dynamic protected ownership shape
Q629  post-start task failure misclassified as creation failure
Q700  event/base lifecycle transition is invalid or stale
Q701  event, base, bufferevent, or callback argument used after free
Q702  callback argument outlives its ownership or borrow contract
Q703  file-descriptor or close-on-free custody is duplicated
Q704  cross-thread Libevent operation lacks supported thread initialization
Q705  callback access conflicts across bases, callers, or worker tasks
Q706  deletion, finalization, or teardown lacks callback quiescence
Q707  persistent event or deferred callback leaves an owned obligation
Q708  priority, readiness, or timing is used as synchronization evidence
Q709  callback target, Libevent API, backend, or build profile is unknown
Q710  bufferevent callback mode or lock contract is violated
Q711  loop exit leaves unaccounted event or callback custody
Q712  observed Libevent trace does not refine the checked event model
```

Diagnostics SHOULD show both relevant threads/sites, the overlapping footprint, permission amounts, current custody, missing synchronization or loan-return edge, and whether the issue is a proved violation, potential violation, or unknown analysis state.

## 55. Required Conformance Tests

The following is the required future compiler/analyzer corpus. It is **not** a statement that these tests have been implemented by the accompanying algebra model.

| ID | Scenario | Expected result |
|---|---|---|
| CT01 | Move an owned buffer to child; child returns it; parent joins and drops | Accept |
| CT02 | Parent uses buffer after successful move capture | Reject |
| CT03 | Parent and child share reads with complementary positive shares | Accept |
| CT04 | Parent writes while child retains a read share | Reject |
| CT05 | Two children each request FULL for the same memory | Reject |
| CT06 | Same pointer copied into differently named variables | Still same permission identity |
| CT07 | Two disjoint array halves mutated independently | Accept with bounds/disjointness proof |
| CT08 | Overlapping array ranges | Reject or analyzer potential/unknown |
| CT09 | Distinct ordinary struct fields | Accept only after layout/access proof |
| CT10 | Adjacent bit-fields in one memory location | Reject independent concurrent writes |
| CT11 | Full permission reconstructed while local shared aliases survive | Reject |
| CT12 | Create fails before any child starts | Capture state restored; no task obligation |
| CT13 | Second create fails after first succeeds | First task must still be joined |
| CT14 | Child finishes but parent has not joined | No restored parent authority |
| CT15 | Backend join failure | No quiescence, no environment reclamation |
| CT16 | Worker typed error after partial local acquisition | Clean locals, return loans, carry declared payload |
| CT17 | Worker error payload contains owned result | Join transfers it exactly once |
| CT18 | Worker error payload contains scoped capture borrow | Reject |
| CT19 | Parent `?` bypasses a live sibling task | Reject |
| CT20 | In-scope handler joins siblings before rethrow | Accept with all result obligations accounted |
| CT21 | Quant opening discards positive access permission | Reject |
| CT22 | Quant opening discards parent-local unused credit after joins | Accept |
| CT23 | Parent abandons child-owned credit during its own error | Reject |
| CT24 | Cleanup destroys an owner before child join | Reject |
| CT25 | Cleanup destroys owner after joins and loan expiry | Accept |
| CT26 | Two critical sections use the same mutex for the same payload | Accept |
| CT27 | Two different mutex identities allegedly guard the same payload | Reject without a valid composite protocol |
| CT28 | Guard is moved to a child or unlocked by another thread | Reject |
| CT29 | Typed error leaves a temporarily broken mutex invariant | Reject unless restored before unlock |
| CT30 | Guard-derived borrow escapes critical section | Reject |
| CT31 | Mutex destroyed with outstanding handle borrower/waiter | Reject |
| CT32 | Join while a guard is held | Reject in initial profile |
| CT33 | Lock order cycle in optional rank mode | Reject rank proof |
| CT34 | Protected owned graph grows/replaces identities | Unsupported in fixed-shape profile |
| CT35 | One shared 4096-byte allocation read by two tasks | Count 4096 once |
| CT36 | Concurrent incremental peaks 3072 and 5120 bytes | Bound at least 8192 plus modeled overhead |
| CT37 | Child returns allocation and finishes early | Allocation remains live in completion capsule |
| CT38 | Child credit splits total more than original credit | Reject |
| CT39 | Concurrent work bound used as wall-clock bound | Reject unsupported claim |
| CT40 | Atomic pointer points to prematurely freed memory | Not certified; unsupported lifetime/publication protocol |
| CT41 | `volatile` used as a completion flag | No synchronization proof |
| CT42 | Two descriptors share an underlying logical state | Preserve shared protocol-cell identity |
| CT43 | Unresolved callback may run after API return | Unknown; no whole-program proof |
| CT44 | Stack capture expires or environment moves before join | Reject |
| CT45 | Unknown destructor accesses shared global state | Require access contract or report unknown |
| CT46 | Known shared wrapper with non-Sync but Send payload | Accept only through verified mutex protocol |
| CT47 | Raw pointer is Copy but no transfer/share contract exists | Reject automatic cross-thread acceptance |
| CT48 | Stale heap refinement reused after mutex release/reacquire | Invalidate/re-establish, never assume |
| CT49 | Independent event order differs between C and interpreter | Validate partial order, not exact total trace |
| CT50 | New lifetime reuses old numeric address/descriptor | Old capability cannot authorize new identity |

Property tests MUST generate valid and invalid share partitions, alias-equivalent footprints, task success/error paths, and bounded interleavings. Exhaustive exploration is bounded evidence, not an unbounded proof.

C integration tests SHOULD include applicable race and memory sanitizers, but a clean sanitizer run is not proof of race absence. Instrumentation and test scheduler synchronization must be documented to avoid masking the tested behavior.

## 56. Illustrative Source Programs

These are intended surface examples for the grammar extension and predeclared library functions with the stated contracts. They are not claimed to have been compiled. Library-specific resource declarations are omitted, as in the base specification's schematic examples.

### 56.1 Concurrent reads and explicit join

```text
concurrency checked;

fn inspect(b: borrow<'b, const, Buffer<4096>>) -> usize
contract {
    access read b;
}
{
    return checksum(b);
}

fn two_reads(b: borrow<'b, const, Buffer<4096>>) -> bool
// The caller must provide a splittable shared permission group with FULL units.
// inspect and checksum are checked, non-throwing, read-only operations.
effects { throws<SpawnError>, spawn, join, may_block }
{
    thread scope s {
        let h = spawn s inspect(read b share 512)?;
        let local = checksum(b);
        let remote = join move h;
        let other = remote?;
        return local == other;
    }
}
```

The loan group for `b` is divided into 512 units for the child and 512 retained by the parent. Returning from the function returns the caller's borrowed authority; the function has not manufactured FULL merely by receiving a pointer. `remote` has no worker error alternatives in this example, so its `?` is a trivial success elimination; a real implementation may normalize it away.

The final equality compares copied scalar results and introduces no additional shared-memory access. A mutable caller may use the buffer again only after the returned shared loan has ended.

### 56.2 Forbidden write during a child read

```text
thread scope s {
    let h = spawn s inspect(read buffer share 512)?;
    overwrite(&mut buffer);   // Q601/Q105: child still holds a read share
    let result = join move h;
    consume_result(result);
}
```

Joining afterward does not repair the earlier conflicting mutation.

### 56.3 Mutex-protected updates

```text
fn increment(m: borrow<'m, const, mutex<u64>>) -> unit
{
    with lock m as g {
        g.value = checked_increment(g.value);
    }
    return ();
}
```

`checked_increment` must preserve the declared invariant on every normal/error path. Concurrent calls access the payload only through guards, not by treating shared access to the wrapper as writable access to `g.value`.

### 56.4 Cleanup order after an error

```text
// Conceptual CFG; not additional surface syntax.
create child borrowing buffer
parent operation fails
    -> handler INSIDE thread scope
    -> explicitly join child
    -> consume/drop/transfer child result or resource-bearing error
    -> expire returned loan group
    -> leave thread scope
    -> matching outer quant opening
    -> drop buffer and other authorized local resources
    -> abandon only eligible local credit remainder
    -> rethrow or recover
```

A compiler MUST reject the version that jumps directly from the parent failure to `drop buffer`.

## 57. Acceptance Criteria for Revision 0.2

Revision 0.2 is complete only when the sequential baseline gates required by the implemented subset are satisfied and all of the following hold:

1. The grammar additions and explicit corrigenda have parser/round-trip tests.
2. AccessQuant is distinct from vector quantities and from usage grades.
3. Permission and unique-resource ledgers preserve their separate invariants.
4. Provenance-aware whole-object and supported disjoint-footprint accesses are checked.
5. Captures, transactional creation failure, linear task handles, and explicit joins are implemented.
6. Both child normal and typed-error exits return scoped loan authority correctly.
7. No checked scope exit or error opening bypasses an outstanding task/loan.
8. Mutex invariant transfer, guard affinity, and all unlock edges are checked.
9. Runtime handoffs establish the required synchronization, including failed-operation distinctions.
10. Concurrent cost bounds include unique resources, completion capsules, and declared runtime overhead without using unsound max composition.
11. The required positive/negative/unknown corpus and bounded property tests pass.
12. C lowering is validated under schedule-aware comparisons for every claimed backend.
13. The reduced Lean development proves permission preservation, access exclusivity/ordering, quiescent destruction, and concurrent cleanup safety without `sorry`.
14. Analyzer reports distinguish proof, feasible witness, potential violation, and unknown; they expose all trusted assumptions.
15. No report equates this result with general race-condition freedom, deadlock freedom, lock-free correctness, or unrestricted legacy-C verification.

## 58. Added Primary Research and Platform Basis

These references support background mechanisms, not a completed proof of the proposed QCore combination. Existing §36 references are retained as the uploaded baseline bibliography; this revision does not independently revalidate every inherited entry.

**[C1]** John Boyland. *Checking Interference with Fractional Permissions*. SAS 2003, pp. 55–72. DOI: 10.1007/3-540-44898-5_4. The abstract describes partial read authority, full write authority, and structured-parallel semantics.

```text
https://link.springer.com/chapter/10.1007/3-540-44898-5_4
```

**[C2]** Viktor Vafeiadis. *Concurrent Separation Logic and Operational Semantics*. MFPS 2011. In particular, the operational soundness and resource-invariant treatment in §5. This is a foundation for proof organization, not a claim that its rules transfer unchanged to QCore.

```text
https://people.mpi-sws.org/~viktor/papers/mfps2011-cslsound.pdf
```

**[C3]** ISO/IEC JTC1/SC22/WG14. *N1570, Committee Draft, April 12, 2011*. See §5.1.2.4 for multithreaded execution/data races, §3.14 for memory locations, and §7.26 for threads. This is the public C11-era draft, not a claim to be the latest C standard.

```text
https://open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
```

**[C4]** The Rust Project. *The Rustonomicon: Send and Sync*. Transfer/share properties and important non-automatic cases. Consulted 2026-09-05.

```text
https://doc.rust-lang.org/nomicon/send-and-sync.html
```

**[C5]** The Rust Project. *std::thread::scope*. Scoped lifetimes and joining; documentation also distinguishes worker function completion from possible thread-local destructor activity. QCore specifies its own stricter quiescence contract rather than assuming identical semantics. Consulted 2026-09-05.

```text
https://doc.rust-lang.org/std/thread/fn.scope.html
```

**[C6]** WG14. *Clarification Request Summary for C11*, N2244, DR 405. Mutex lock/unlock ordering clarification.

```text
https://www.open-std.org/jtc1/sc22/wg14/www/docs/n2244.htm
```

**[C7]** LLVM Project. *Clang Thread Safety Analysis*. Capability environments, shared/exclusive requirements, and annotation-driven checking. Consulted 2026-09-05.

```text
https://clang.llvm.org/docs/ThreadSafetyAnalysis.html
```

## 59. Migration and Evidence Status

The source attachment remains unchanged. The consolidated revision retains its sequential chapters, applies only the documented banner/cross-reference/grammar changes, and appends this amendment with explicit precedence.

For existing sequential programs, inferred task-local full authority and ordinary borrowing should preserve acceptance except where the explicit grammar repairs clarify previously schematic syntax. Resource declarations used across threads additionally need Send/Sync and access-effect evidence; source-level compatibility is not permission to infer thread safety from old annotations.

The accompanying Python prototype is an executable experiment for a deliberately reduced permission/custody/synchronization model. Its validation report states exactly what ran. It is not the Rust compiler, a C front end, an implementation of this grammar, or a Lean proof. The normative acceptance criteria above remain outstanding until the actual implementation and proof artifacts satisfy them.

**Final decision:** implement checked concurrency through identity-indexed access quants, conserved ownership/custody, synchronization-backed transfers, and quiescence-aware exits. Do not implement it as an extra count in the existing resource vector, and do not let an error region erase a thread's outstanding access rights.
