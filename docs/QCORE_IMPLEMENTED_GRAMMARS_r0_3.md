# QCore implemented input grammars — revision 0.3

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

**Source scope:** supplied archive review, reconciled with the implementation records in the [acceptance ledger](reviews/r0_3/REVIEW.md).  
**Purpose:** distinguish exact consumer inputs from the broader [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf).

The grammars below use EBNF: quoted tokens are literals; `{ X }` means repetition; `[ X ]` means optional; `|` means alternatives. `IDENT` is an ASCII identifier and `NAT` is an unsigned decimal token. Lexical recognition alone does not determine semantic admissibility. Historical line references name the reviewed files; the integration source inventory separately binds available current files.

## G01. Structural source envelope

```ebnf
module_header = "module", ("executable" | "library" | "contract"), ";" ;
source        = [module_header], {declaration} ;
contract      = "contract", "{", {clause}, "}" ;
clause        = "requires", NONEMPTY_EXPRESSION_TOKENS, ";"
              | "ensures", selector, NONEMPTY_EXPRESSION_TOKENS, ";"
              | "quant", selector, NONEMPTY_EXPRESSION_TOKENS, ";" ;
selector      = "normal" | "any" | "error", "(", NONEMPTY_TYPE_TOKENS, ")" ;
```

`declaration` and the two uppercase token-sequence placeholders above deliberately denote structural parser regions, not a claim of full expression/type checking. The parser tracks balanced regions and clause spans. The relevant implementation is `crates/qcore-frontend/src/frontend.rs`, particularly declaration validation, contract parsing, parameter parsing and `elaborate_source`.

Recognized declaration categories are concurrency profile, verifier scope, dimension, axis, type alias, resource type, error, constant, extern function and function. Broad statement/type classification includes forms that the source compiler will subsequently reject. The structural grammar is therefore not a sound shortcut for discovering executable capabilities by trying tokens until parsing succeeds.

A header may occur at most once and only before declarations. The default is executable mode. A contract module is entry-free; it is not automatically an observation policy or a resource table. Multiple input modes combine as executable first, library second, contract last. Each file still has its own header constraint, and declarations share one namespace.

## G02. Source execution profile

The following is the supported canonical body syntax, not a claim that every possible structurally accepted spelling of a declaration has ordinary type semantics:

```ebnf
body          = "{", {statement}, "}" ;
statement     = acquisition | check | direct_call | drop | move | return ;
acquisition   = "let", IDENT, "=", "qcore_acquire", "(", RESOURCE_ID,
                ",", BOOL, ",", ERROR_NAME, ")", ";" ;
check         = "qcore_check", "(", BOOL, ",", ERROR_NAME, ")", ";" ;
direct_call   = FUNCTION_NAME, "(", ")", ";" ;
drop          = "drop", RESOURCE_BINDING, ";" ;
move          = "move", RESOURCE_BINDING, ";" ;
return        = "return", [ ["-"], NAT ], ";" ;
BOOL          = "true" | "false" ;
```

`RESOURCE_ID` must fit `u32`. `ERROR_NAME` must be declared; when no error declarations exist the driver provides its default `Failure` case. Resource bindings must satisfy ownership and cleanup checks. A `move` transfers to the abstract result payload; it does not simply release an external endpoint. A unit result uses bare return, represented as zero in the reduced ABI. An i32 result requires its signed literal; the separate custody payload is explicitly identified in compilation result metadata.

`qcore_acquire` and `qcore_check` take literal Boolean controls in this profile. They are model operations, not general malloc calls or arbitrary runtime conditions. General expressions, argument-taking calls, indirect calls, calls through a shadowing local, calls to extern declarations, nested source control flow, source lambdas and source threading are not accepted by this driver body path. Local `use` annotations are explicitly rejected here rather than ignored.

A direct call must resolve to a zero-parameter source function. For C emission, it must also have supported acyclic call structure and an interpreted observation with no resource trace, a normal result, and no owned payload. It is not enough that the function's aggregate net resource change is zero. Declared user effects and general contract predicates are not comprehensively checked by this slice.

The executable driver admits explicit i32/unit result annotations and validates each return form. Signed i32 literals include both extrema; the signed value is range-checked in a wider representation. Other result annotations, nonliteral returns and scalar/type mismatches reject. Structural tooling preserves richer result syntax, but full owned-result/per-exit typing is still unsupported. A separate custody identity ledger retains the reduced model's transfer payload; it is not proof that an i32 annotation describes owned values.

An example within the canonical slice is:

```qcore
fn main() -> i32 {
    let item = qcore_acquire(1, true, Failure);
    drop item;
    return 0;
}
```

This example does not demonstrate arbitrary allocation ABI support. Source references: `compiler/src/driver.rs:390–610, 724–767, 775–932`.

## G03. Source parameter usage grades

```ebnf
usage_annotation = "use", grade ;
grade            = "zero" | "one" | "affine" | "relevant" | "many"
                 | NAT
                 | "[", NAT, ",", (NAT | "inf"), "]" ;
```

A natural grade denotes `[n,n]`. Bounds must fit `u64`; finite lower bounds must not exceed upper bounds. Named meanings are `[0,0]`, `[1,1]`, `[0,1]`, `[1,inf]` and `[0,inf]`, respectively. Symbolic grade expressions are not evaluated by this source parameter parser. The independent usage algebra has checked addition, multiplication, interval hull and containment semantics; that algebra does not broaden this surface grammar. See `frontend.rs:838–878` and `crates/qcore-core/src/usage.rs`.

## G04. Closed quantitative indices

```ebnf
quant_type = "quant", "<", (scalar | vector), [where_clause], ">" ;
scalar     = expr ;
vector     = ("credit" | "live"), ",", "{", field, {",", field}, "}" ;
field      = IDENT, "=", expr ;
where_clause = "where", where_item, {("," | "and"), where_item} ;
where_item = IDENT, ":=", expr | expr, relation, expr | "true" ;
relation   = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expr       = product, {"+", product} ;
product    = atom, ["*", atom] ;
atom       = NAT | IDENT | "(", expr, ")"
           | ("min" | "max" | "monus"), "(", expr, ",", expr, ")" ;
```

The product rule has an additional semantic restriction: one operand must parse to a literal node. Two names are rejected even if later definitions would resolve both to constants. A parenthesized literal may still be a literal node. An unparenthesized product chain is not a general multiplication expression under this grammar.

Scalar shorthand is credit-only and requires exactly one declared credit axis. Vector fields are nonempty, uniquely named, mode-compatible and dimension-checked. Every local definition is pure, fresh and closed; forward definitions are allowed, dependency cycles and unknown/runtime names are rejected. Constraints must evaluate true. Unreferenced definitions without an inferable dimension are not silently assigned a dimension.

Numbers and arithmetic use checked `u128` natural semantics. `monus(a,b)` requires `b <= a` and computes the checked difference; it does not saturate an invalid subtraction to zero. There is no free subtraction, division or nonlinear symbolic solver. Overflow and structural limits are errors, not wraparound or assumed inequalities.

The parser rejects atom nesting beyond its depth-32 condition and addition beyond its term-limit condition of 64 added terms. Resolution/evaluation impose additional depth/dependency guards in the same implementation. These are explicit implementation limits, not mathematical restrictions on natural numbers.

`quant<cost,...>` is rejected by this resolver as a future total/peak profile. `quant<access,...>` is skipped by this closed-index parser for separate handling; skipping it is not source-level access verification. The axis environment is now built and validated unconditionally, including modules without quantitative type occurrences (R05).

Source: `crates/qcore-frontend/src/quant_index.rs`, `frontend.rs:1519–1548`. A supported canonical example is:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;
extern fn keep(q: quant<n where n := 5> use one) -> unit;
```

## G05. Resource loader v1

```ebnf
resource_module = "module", "contract", ";", resource_fn, {resource_fn} ;
resource_fn = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
              "->", result_abi, "contract", "{", resource_clause,
              {resource_clause}, "}", ";" ;
parameter   = IDENT, ":", parameter_abi ;
parameter_abi = scalar_abi | pointer_abi ;
result_abi  = "unit" | parameter_abi ;
scalar_abi  = "bool" | "i8" | "i16" | "i32" | "i64" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "usize" ;
pointer_abi = "ptr", "<", "mut", ",", "u8", ">"
            | "ptr", "<", "const", ",", "u8", ">"
            | "ptr", "<", "mut", ",", "ptr", "<", "mut", ",", "u8", ">", ">" ;
resource_clause = "requires", "resource_contract_v1", ";"
                | "ensures", "normal", resource_predicate, ";" ;
resource_predicate = "resource_borrowed", "(", IDENT, ")"
  | ("resource_owned" | "resource_consumed" | "resource_released" | "resource_out"),
    "(", IDENT, ",", nullability, ")"
  | ("resource_stored" | "resource_removed"),
    "(", IDENT, ",", IDENT, ",", nullability, ")"
  | "resource_return_none", "(", ")"
  | "resource_return_owned", "(", nullability, ")"
  | "resource_return_reallocated", "(", IDENT, ",", nullability, ")" ;
nullability = "nullable" | "nonnull" ;
```

Additional static conditions are mandatory: exactly one marker, exactly one mode per parameter, exactly one result mode, unique names, compatible pointer/scalar types, valid distinct owned parent references, and an owned reallocation source. These conditions cannot be represented by context-free productions alone. Signature usage annotations, extra declarations, unsupported predicates and other exit selectors are rejected atomically, not ignored. See contract specification C04–C06 and `crates/qcore-contract/src/contract_loader.rs`.

The grammar describes tokens, not required whitespace. It provides no general predicate-call nesting. It also does not turn an admitted trusted declaration into proof that a C function obeys it.

## G06. Observation predicate grammar

```ebnf
predicate = or_expr ;
or_expr   = and_expr, {"or", and_expr} ;
and_expr  = comparison, {"and", comparison} ;
comparison = sum, {relation, sum} ;
sum       = product, {("+" | "-"), product} ;
product   = unary, {"*", unary} ;
unary     = ("-" | "!" | "not"), unary | primary ;
primary   = NAT | IDENT | "true" | "false" | "(", predicate, ")" ;
relation  = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

Reserved operator/literal words take their parser meanings rather than binding arbitrary facts. The interpreter uses signed `i128`, checked arithmetic, nonzero truth, eager evaluation and left-associated binary operators, including repeated comparisons. Thus `a < b < c` is not mathematical chained comparison syntax; write `(a < b) and (b < c)` to express that intended predicate.

A predicate is limited to 512 non-EOF tokens and the parser's depth-64 guard. Signed decimal literals are range-checked with sufficient magnitude capacity, including the full i128 minimum (R07). Out-of-range literals and checked arithmetic overflow reject. Negating an already minimal supplied fact still overflows; literal support does not authorize unchecked arithmetic.

Only `requires` and `ensures normal` envelopes are admitted by this consumer, with at least one postcondition. Function-call predicates, `old(...)`, `resource_*` calls, quant clauses, `&&`/`||`, typed errors and `any` exits do not extend this grammar. A missing fact or overflow errors even in `true or missing_fact`, and a malformed postcondition errors even when a precondition is false. Source: `crates/qcore-contract/src/contract_observation.rs`.

## G07. Extended grammar and conformance

`QCORE_DESIGN_GRAMMAR_r0_3.bnf` collects the nineteen BNF blocks from revision 0.2, adding the current optional module header. It preserves the intended full language vocabulary, including source constructs without driver lowering. Its lexical terminal placeholders are `IDENTIFIER-TOKEN`, `INTEGER-TOKEN`, `FLOAT-TOKEN`, `STRING-TOKEN` and `CHAR-TOKEN`.

That target grammar is neither a parser generator input already used by the compiler nor the grammar of every policy consumer. In particular, retained policy token expressions and resource predicate calls are defined at their consumer boundary above; their structural acceptance is not a new general-purpose contract proof rule. Future implementation work MUST keep the profile grammars, type rules, lowering coverage and negative tests synchronized instead of conflating the target with today's supported syntax.
