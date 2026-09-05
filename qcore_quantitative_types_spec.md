# QCore-C: Quantitative Types, Resource Effects, and Checked Error Unwinding

**Status:** Research specification, revision 0.1  
**Audience:** An implementation and verification LLM agent  
**Normative language:** “MUST”, “MUST NOT”, “SHOULD”, and “MAY” are requirements at their usual standards meanings.  
**Primary implementation target:** a small typed language that erases to straightforward C11.  
**Secondary target:** a conservative analyzer for existing C after the source-language checker and C emitter are stable.

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
                           <optional-contract>
                           <block>

<extern-function-declaration> ::= "extern" "fn" <identifier> <optional-type-parameters>
                                  "(" <optional-parameter-list> ")"
                                  "->" <type>
                                  <optional-where>
                                  <optional-effects>
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

<primary-expression> ::= <identifier>
                       | <literal>
                       | "(" <expression> ")"

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
