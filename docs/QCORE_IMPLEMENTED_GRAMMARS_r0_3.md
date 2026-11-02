# Quirin implemented input grammars — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the legacy `.qcore` alias and this file's name —
> and those component identifiers are unchanged by the branding. Current contract inputs use `.qv`, executable/library inputs use `.qr`, and native ABI manifests use `.qrlib`; legacy names remain warned compatibility aliases.

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

Historical `.qcore`/`.qc` census counts below describe their original checkpoints. Current files may have `.qv` or `.qr` names while preserving those recorded source bytes and observations.

**Source scope:** supplied archive review, reconciled with the implementation records in the [acceptance ledger](reviews/r0_3/REVIEW.md).  
**Purpose:** distinguish exact consumer inputs from the broader [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf).

The grammars below use EBNF: quoted tokens are literals; `{ X }` means repetition; `[ X ]` means optional; `|` means alternatives. `IDENT` is an ASCII identifier and `NAT` is an unsigned decimal token. Lexical recognition alone does not determine semantic admissibility. Historical line references name the reviewed files; the integration source inventory separately binds available current files.

## G01. Structural source envelope

```ebnf
module_header = "module", ("executable" | "library" | "contract"), ";" ;
source        = [module_header], {module_import | declaration} ;
module_import = "import", IDENT, {".", IDENT}, ";" ;
contract      = "contract", "{", {clause}, "}" ;
clause        = "requires", NONEMPTY_EXPRESSION_TOKENS, ";"
              | "ensures", selector, NONEMPTY_EXPRESSION_TOKENS, ";"
              | "quant", selector, NONEMPTY_EXPRESSION_TOKENS, ";" ;
selector      = "normal" | "any" | "error", "(", NONEMPTY_TYPE_TOKENS, ")" ;
```

`declaration` and the two uppercase token-sequence placeholders above deliberately denote structural parser regions, not a claim of full expression/type checking. The parser tracks balanced regions and clause spans. The relevant implementation is `crates/qcore-frontend/src/frontend.rs`, particularly declaration validation, contract parsing, parameter parsing and `elaborate_source`.

Recognized declaration categories are concurrency profile, verifier scope, dimension, axis, type alias, resource type, error, constant, extern function and function. Broad statement/type classification includes forms that the source compiler will subsequently reject. The structural grammar is therefore not a sound shortcut for discovering executable capabilities by trying tokens until parsing succeeds.

A header may occur at most once and only before declarations. The default is executable mode. A contract module is entry-free; it is not automatically an observation policy or a resource table. Multiple input modes combine as executable first, library second, contract last. Each file still has its own header constraint, and declarations share one namespace.

Task syntax now has structured records in `frontend_tasks.rs`: `spawn S worker(...)`
retains ordered `copy`, `move`, `read ... share ...` and `write` place captures;
`join move H` retains its handle place. An immediately following `?` is retained.
Name resolution identifies a live scope, a source worker with matching arity,
capture roots and the handle binder. Typed statements preserve new binder IDs
after resolving their initializers. Place/share syntax is not proof of capture
types, permissions or live handles. These records do not extend G02's sequential executable profile. The separate
G02a path now checks and executes its supported task subset; the earlier
[task foundations record](reports/w6-source-task-foundations/README.md) remains historical.

Bundled semantic modules require an explicit top-level `#import` in each source
file. The current registry contains `std.concurrency`:

```qcore
#import std.concurrency;
concurrency checked;
```

Bundled import paths are dot-separated identifiers terminated by a semicolon. Imports
may occur between top-level declarations, after the optional module header, and
apply to the whole file. Repeated imports are idempotent. Unknown module names
reject with `Q014`; concurrency forms without their import reject with `Q015`,
including in uncalled bodies and non-executable modules. Each input file is
checked before combining declarations: another file's import cannot grant it
concurrency. Importing vocabulary does not discharge operation checks or make an
unsupported profile executable. The task profile still requires its
`concurrency checked;` marker. Runtime file imports are implemented by the source-bundle resolver below.
Package resolution, import aliases, wildcards and reexports remain unsupported,
and capabilities never transfer transitively. Bare `import` remains a bundled
module compatibility spelling; a contract module takes another
contract module's declarations through the separate `#include` directive of
G05a, resolved by the contract loader against a published load path and never
by this front end.

### G01p. Typed contract propositions (graph version 3)

`quant proposition` is contextual inside a function contract. The structural
parser retains kind, text and span. Typed model inspection and graph replay
parse and check the complete definition. A declared model's standalone formulas
are entry obligations; they are not established by declaration. The strict
resource/bounds loaders refuse unbound standalone entry obligations rather than
installing a summary that ignores them. Grouped hypothesis metadata retains its
existing evidence-only role. Theorem and effect representations are in
G01t/G01e below.

```ebnf
proposition = "quant", "proposition", "#", id, "(", [parameters], ")",
              "=", formula, ";" ;
parameters  = parameter, {",", parameter} ;
parameter   = "#", id, ":", sort ;
sort        = id | "predicate", "<", [ids], ">"
              | "function", "<", ids, ">" ;
ids         = id, {",", id} ;
formula     = "#", id, "(", [arguments], ")"
            | term, ("==" | "!=" | "<" | "<=" | ">" | ">="), term
            | "!", formula | "(", formula, ")"
            | "(", formula, ("&&" | "||" | "=>"), formula, ")"
            | ("forall" | "exists"), "(", "#", id, ":", id, ")",
              "{", formula, "}" ;
arguments   = term, {",", term} ;
term        = sum ;
sum         = product, {("+" | "-"), product} ;
product     = primary, {"*", primary} ;
primary     = "#", id | INTEGER | "-", primary
            | "#", id, "(", [arguments], ")" | "(", term, ")" ;
```

An `id` is 1–128 lowercase ASCII letters/digits/underscores beginning with a
letter. `predicate` and `function` are type constructors, never value sorts.
`predicate<...>` lists ordered argument sorts (`predicate<>` is nullary).
`function<A, B, R>` has arguments A/B and result R; `function<R>` is nullary.
All relation/function symbols are explicitly bound parameters. Calls require
exact arity and value sorts; equality requires equal value sorts. Free names,
duplicate parameters, active binder shadowing, higher-order quantification,
prose strings, implicit predicates/functions and unsupported operators refuse.

The value sort `integer` denotes mathematical unbounded integers. INTEGER is
0 or a nonzero-leading decimal with at most 128 digits. Arithmetic and order
require integer operands; `!=` lowers to negated equality. Functions/relations
receive no C interpretation by their names. Target widths, sizes, constants,
field identities, floating conversion and backend meanings require explicit
bindings. Arithmetic does not wrap or assert that corresponding C arithmetic
is defined. Other quantifier sorts may be empty; no finite or nonempty domain
is inferred. Every source/model interpretation and actual premise remains open
until checked for the exact environment. Missing interpretations are not
negation evidence.

Limits: 65,536 source bytes per definition; 64 parameters; 16 call arguments;
256 combined formula/term AST nodes and depth 32 (root depth zero). Source
nesting and parser work are additionally bounded (131,072 parsing steps;
backtracking never replenishes work fuel). Schema validation rechecks identifier,
sort, arity, node and depth limits. One definition creates one graph atom with
its `#` binder as ID. `legacy_` is reserved; duplicate formal IDs refuse.

A graph with formal atoms uses version 3. It records ungrouped formal IDs in
its canonical sorted `requirements` list, so standalone model prerequisites
cannot disappear. This field is omitted when empty; versions 1 and 2 retain
their exact original encoding and refuse formal atoms or requirements. Group,
refinement and lifecycle references retain their existing rules. Exact formal
ASTs and requirements participate in graph digests and source replay.
Interpretation/type checking alone supplies no theorem warrant or verdict.

Implementation: `qcore_frontend::quant_proposition`,
`qcore_schemas::quant_proposition`, `qcore_contract::hypothesis`.
`quant_proposition_inventory` inspects specified model files and emits source/
graph identities and entry requirements; it grants no C call permission.

### G01c. Explicit entry and exit conditions

User decision 2026-10-26: `quant condition` uses an explicit selector:

```ebnf
condition = "quant", "condition", condition-selector,
            "#", id, "(", [parameters], ")", "=", formula, ";" ;
condition-selector = "entry" | "normal" | "any"
                   | "error", "(", error-type, ")" ;
```

Signatures, terms, formulas and their bounds are G01p's. The selected typed
reader also bounds an error-type label to 1,024 bytes, nonblank and free of
control characters. The structural selector retains the existing error-type
source text; no error hierarchy or subtype matching is inferred.

`entry` states a prerequisite at function entry. `normal` states an assertion at
every normal return. `error(E)` selects exactly the matching typed error exit;
`any` selects every terminating normal/error exit. None asserts termination.
A negative C return code is a normal return. All before/after/result/trace
projections are explicit typed parameters requiring actual source bindings.
A missing condition interpretation is not negation evidence. Declarations,
artifact identities and theorem names alone establish none of these assertions.

`qcore_contract::quant_condition::declaration` inspects every typed condition
and rejects duplicate IDs across condition/proposition/effect/theorem clauses.
Structural parsing and generic inclusion/views preserve selectors and bodies.
Strict resource/bounds/local-condition consumers refuse the new clauses.
Hypothesis-only graph inspection retains explicit entry conditions as distinct
`entry-condition` atoms in graph version 4. Every such atom is always present
in canonical sorted `requirements`, even when metadata groups/refinements also
reference it. Grouping cannot discharge an entry assertion. Versions 1–3 retain
their prior bytes and rules; a v4 entry atom under an older version refuses.
No graph inspection supplies the required valuation. Exit conditions are
validated without treating those guarantees as antecedents. Whole-condition consumers must use the typed
condition view. No interpreted C summary is installed by this interface.

The bounded Boolean source-contract reader additionally accepts exact entry
role definitions for modeled-object liveness, caller-exclusive modeled access,
and `old_field` (unit) or `old_busy`/`old_lock` (i32) membership in `{0, 1}`.
`qcore_contract::boolean_condition_profile` owns the exact typed signatures and
formulas; matching an identifier alone is insufficient. Its `modeled(env,o)`
interpretation includes every object mapped by the chosen source profile, and
`during(env,e)` includes the selected call interval. Liveness and exclusivity
are caller assumptions over that interpretation, not extracted C facts.

For `normal` assertions this reader binds integer parameters only to its
existing entry/exit ghosts (`old_*`, unprefixed fields, i32 `result`, and helper
`field_*` writes). It accepts equality/inequality with integer literals,
addition, subtraction and negation under checked i64 evaluation. Unsupported
sorts, functions, operators, selectors or missing valuations refuse. Declaring
an unused exit variable does not satisfy the existing requirement to constrain
all exit fields/result/helper writes. Entry roles, signatures, both Boolean
prestates, companion fields and complete source-path validation remain required.
Every typed entry definition's canonical SHA-256 joins the task's explicit
assumptions, alongside the entire source identity. This bounded consumer creates
no generic graph support or ordinary C-call permission.

The introducer and selectors are contextual ordinary words; no globally
reserved word or lexer token is added. This form is distinct from the existing
`quant #guard = [...]` bindings and
local-condition blocks. Implementation: `qcore_schemas::quant_condition`,
`qcore_frontend::quant_condition`, `qcore_contract::quant_condition`;
semantics/audit: `QCore.QuantCondition` / `AuditQuantCondition.lean`.

### G01t. Typed theorem instantiation obligations

Inside a function contract, the contextual form is:

```ebnf
theorem = "quant", "theorem", "#", id, "(", [parameters], ")",
          "using", reference, "(", [symbols], ")",
          "requires", "(", [formula, {",", formula}], ")",
          "yields", formula, ";" ;
symbols = "#", id, {",", "#", id} ;
reference = word, {".", word} ;
```

Parameters, terms and formulas follow G01p. A reference consists of nonempty ASCII
identifier segments (letters/underscore first, letters/digits/underscores
thereafter), at most 128 bytes each and 1,024 bytes overall, preserving case. It identifies the
requested theorem; a spelling supplies no proof. Arguments are declared value,
predicate or function symbols, at most 64. There are at most 32 prerequisites.
A complete application has at most 65,536 source bytes, 64 parameters and 256
combined formula/term nodes, depth 32. The selected reader consumes all bytes.

`qcore_contract::quant_theorem::instantiate` accepts an explicitly supplied
`TheoremSchema` and application. It checks both typed signatures, exact reference,
argument sorts/arity and exact substituted prerequisites/conclusion. Substitution
is conservative: a bound-name capture is refused; different binder spellings
are not automatically equated. Model/audit references require well-formed paths
and SHA256 values. No file, artifact contents, proof or registry authority is
verified by those checks. The sealed, serialize-only result retains all premises,
conclusion and artifact identities with `model_proof_binding` and `source_binding`
**open**. No result can establish plugin suitability or discharge a premise.

Structural parsing preserves the clause and generic views render it. Strict
resource/bounds and local-condition consumers refuse applications. Implementation:
`qcore_schemas::quant_theorem`, `qcore_frontend::quant_theorem`,
`qcore_contract::quant_theorem`; conditional semantics/audit:
`QCore.QuantTheorem` / `AuditQuantTheorem.lean`.

### G01e. Typed conditional own-operation effects

```ebnf
effect = "quant", "effect", selector, "#", id, "(", [parameters], ")",
         "from", "#", id, "to", "#", id,
         "when", formula, "reads", "(", [arguments], ")",
         "writes", "(", [arguments], ")", "frame", formula,
         "=", formula, ";" ;
selector = "normal" | "any" | "error", "(", word, ")" ;
```

Parameters/formulas/terms follow G01p. Entry and exit names are distinct bound
values of sort `state`. Each footprint term has sort `object`. Reads and writes
may overlap for read-modify-write operations. Each list has at most 64 terms;
structurally identical duplicates within a list refuse. This is no alias proof.
The complete definition has at most 65,536 bytes, 64 parameters and 256 combined
formula/term nodes, depth 32 (footprint roots conservatively checked at depth 2).
Guard, frame and transition are typed formulas over the same explicit signature.

An effect is a conditional relation for the operation's own step. Its declaration
establishes no occurrence, atomicity, memory order, synchronization, whole-call
frame or source binding. Interference requires separate events and footprints.
Structural parsing preserves selector/text/span; generic inclusion/views render
it. Strict resource/bounds and local-condition consumers refuse it. No effect is
installed as a call summary. Implementation: `qcore_schemas::quant_effect`,
`qcore_frontend::quant_effect`; semantics/audit: `QCore.QuantEffect` /
`AuditQuantEffect.lean`.

### G01a. Runtime import bundle

```ebnf
standard_import = [ "#" ], "import", identifier, { ".", identifier }, ";" ;
runtime_import = "#", "import", quoted_relative_runtime_path, ";" ;
```

Canonical usage is `#import std.io;`, `#import "./helper";` and
`#import "../shared/helper.qr";`. Quoted paths contain no escapes or control
characters and name relative `.qr` files, with exactly `.qr` appended when
omitted. Top-level directives resolve recursively relative to their own source
file, without package, directory or legacy-suffix fallback.
`inspect_source_imports` returns mode/header/import metadata only;
`parse_source_program` rejects unresolved runtime directives. The verification
driver resolves the graph before parsing its derived checking projection.

`compile_files` and `compile_files_with_registry` use supplied snapshots only,
all as roots. `compile_source_bundle_with_registry` separates roots from the
available pool and explicitly selects snapshot-only or filesystem loading;
the CLI selects filesystem loading. Snapshot precedence, modes, identity,
bounded ordering and retained evidence follow [Runtime source imports](QCORE_SPEC_r0_3.md#runtime-source-imports).
Unmarked/library dependencies contribute declarations but never executable
root intent. Missing/ambiguous/cyclic/wrong-mode/bounded-out resolution refuses
at Q014 (existing filename incompatibility Q403). No code is allocated.
Capability checks are per file, including extern calls whose declaration comes
from another file. Imports do not broaden execution profiles or supported call
compositions. Contract inclusion remains the separate G05a loader.

## G02. Source execution profile

The following is the supported canonical body syntax, not a claim that every possible structurally accepted spelling of a declaration has ordinary type semantics:

```ebnf
body          = "{", {statement}, "}" ;
statement     = acquisition | constant | check | direct_call | drop | move | return ;
constant      = "let", IDENT, [ ":", "i32" ], "=", [ "-" ], NAT, ";" ;
acquisition   = "let", IDENT, "=", "qcore_acquire", "(", RESOURCE_ID,
                ",", BOOL, ",", ERROR_NAME, ")", ";" ;
check         = "qcore_check", "(", BOOL, ",", ERROR_NAME, ")", ";" ;
direct_call   = FUNCTION_NAME, "(", ")", ";" ;
drop          = "drop", RESOURCE_BINDING, ";" ;
move          = "move", RESOURCE_BINDING, ";" ;
return        = "return", [ ["-"], NAT ], ";" ;
BOOL          = "true" | "false" ;
```

The optional `i32` annotation on an integer literal binding is checked against
the lowered value. Mismatched or unsupported explicit annotations and malformed
local heads refuse at Q020. `qcore_acquire` publishes a custody identity without
a nominal result type; annotations on its binding, including `own<T>`, refuse
until the operation supplies that evidence. Error-region nesting does not bypass
this check. Local usage annotations retain Q100.

A parameter annotation is checked too (card `W48-QR-SCALAR-TYPES-R2`). This
body path supplies no argument to any parameter — a direct call names a
zero-parameter function and the emitted entry has none — so a parameter's type
must be one of the §G02t scalar table (`isize`/`usize` only with a recorded
target) and every other annotation (`Widget`, `fn() -> i32`, `own<T>`,
`borrow<T>`, …) refuses at Q020. Before the card no parameter annotation was
read at all (the named boundary B-1 of `docs/reports/remaining-v03`).

An `extern fn` declaration no body calls is read too (card
`W48-QR-SCALAR-TYPES-R3`, on every route of an executable or library module):
its parameters and result must be in the §C04.1 ABI vocabulary, and anything
else — `Widget`, `own<T>`, a `quant<…>` parameter — refuses at Q020 (§G02t,
"Every extern declaration is read").

`RESOURCE_ID` must fit `u32`. `ERROR_NAME` must be declared; when no error declarations exist the driver provides its default `Failure` case. Resource bindings must satisfy ownership and cleanup checks. A `move` transfers to the abstract result payload; it does not simply release an external endpoint. A unit result uses bare return, represented as zero in the reduced ABI. An i32 result requires its signed literal; the separate custody payload is explicitly identified in compilation result metadata.

`qcore_acquire` and `qcore_check` take literal Boolean controls in this profile. They are model operations, not general malloc calls or arbitrary runtime conditions. General expressions, argument-taking calls, indirect calls, calls through a shadowing local, calls to extern declarations, nested source control flow, source lambdas and source threading are not accepted by this driver body path. Local `use` annotations are explicitly rejected here rather than ignored. A bounded extern call is accepted by the separate `source.extern-call.v1` path of §G02d, which is selected before this one and does not widen it: this body path still refuses every call whose target is not a zero-parameter source function.

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

<a id="g02s-pure-scalar-functions-sourcescalar-callsv1"></a>

## G02s. Scalar functions and explicit output (`source.scalar-calls.v1`)

`SOURCE-SCALAR-CALLS-V1` adds the executable subset below. This profile is
selected after specialized profiles and before the legacy standard-IO route
when retained tokens contain the function-style `print(` form. Otherwise a
nonliteral return/local, an annotated local, or an argument-taking statement
call to a declared source function selects it. Sequential resource intrinsics
retain their previous route. Card `W48-QR-SCALAR-TYPES` adds one more door: a
module with `std.io` line statements **and** the scalar-call syntax above
reaches this route when the `source.std-io.v1` lowering refuses it; every module
that lowering admits keeps that route byte for byte. It adds no tokens or
reserved words and does not widen the other profiles.

```ebnf
function = "fn", NAME, "(", [parameter, {",", parameter}], ")", "->", result,
           ["effects", "{", [effect, {",", effect}], "}"], ["contract", "{", "}"],
           block ;
effect = "io" | "throws", "<", "ArithmeticError", ">"
       | "partial_eq" | "partial_ord" ;                        (* each once; §G02t *)
block = "{", {statement}, end, "}" ;
end = "return", [expression], ";"
    | "raise", (NAME | "ArithmeticError", "::", VARIANT), ";"
    | "match", call, "{", arm, arm, "}" ;                      (* one Ok, one Err *)
arm = "Result", "::", "Ok", "{", "value", ":", NAME, "}", "=>", block
    | "Result", "::", "Err", "{", "error", ":", NAME, "}", "=>", block ;
VARIANT = "Overflow" | "DivisionByZero" | "OutOfRange" ;
parameter = NAME, ":", SCALAR ;
result = SCALAR | "unit" ;
SCALAR = "bool" | "i8" | "i16" | "i32" | "i64" | "isize"
       | "u8" | "u16" | "u32" | "u64" | "usize"
       | "f32" | "f64" ;                                        (* §G02t *)
statement = "let", NAME, [":", SCALAR], "=", expression, ";"
          | call, ["?"], ";"
          | "print", "(", expression, ")", ";"                  (* i32 only *)
          | "print", RENDERING, OPERAND, ";"                    (* §G02g's reader *)
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "flush", ";" ;
expression = sum, [CMP, sum] ;                                  (* non-associative *)
sum = product, {("+" | "-"), product} ;
product = cast, {("*" | "/" | "%"), cast} ;
cast = unary, {"as", SCALAR} ;
unary = "-", unary | NAT | FLOAT | "true" | "false" | NAME | call, ["?"]
      | "(", expression, ")" ;
CMP = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
call = FUNCTION_NAME, "(", [expression, {",", expression}], ")" ;
```

Every parameter, local and result has one type of the §G02t table (a result
may also be `unit`); unit calls cannot occupy a scalar position. The typing,
arithmetic, conversion and rendering rules of the table are §G02t's; a program
that names only `i32` types and uses none of the forms above that card added
checks, runs and emits exactly as before it.
Every definition is checked, including uncalled bodies. Calls must be direct
and the entire graph acyclic. Locals are immutable, nonshadowing within each
function, and initializers resolve before binding. All functions end with one
explicit return. Operands/arguments evaluate left to right; binary operators
associate left. Literal magnitudes must fit their type, including -2147483648
for `i32`.

`print(EXPRESSION);` evaluates one i32 expression and writes its decimal value
followed by one newline to standard output. The source file must declare
`#import std.io;`; `std.fmt` is neither required nor consulted because this form
has no rendering selector. An absent effect row infers `io` from a direct print
or transitively from a called function. `effects {io}` explicitly permits it,
while `effects {}` explicitly excludes it and therefore refuses any direct or
transitive IO. The one other row entry is `throws<ArithmeticError>` (§G02t,
card `W48-QR-SCALAR-TYPES-R2`), never inferred; every other effect or error
name is outside this profile. Output follows
ordinary statement order and the left-to-right argument order above. The
builtin form stays `i32`-only and is refused in a module that uses any §G02t
form (a non-`i32` type, a comparison, a conversion, `bool` or a line
statement): such a module renders through `print dec|hex X;`.

The line statements are `source.std-io.v1`'s own (§G02g), read by that route's
own reader over the statement's tokens, so a line one route admits and the
other refuses is not expressible, and a pipe sink (`x |> print dec;`, §G02h)
desugars to one. They may stand in any function; a line derives `io` exactly
as a builtin print does. `OPERAND` is an integer literal (an `i32`) or a scalar
name this function bound; `borrow NAME` refuses at `Q028` because this route
has no custody model, and `log_level` refuses at `Q030` because this route
installs no system-trace adapter. The per-stream line trace is published
beside the entry's value, as `source.std-io.v1` publishes its own.

Arithmetic is checked: overflow and zero division refuse the closed entry at
check time, and a subexpression whose leaves are all literals is folded while it
is typed, so a constant fault refuses even in a function no entry calls (a
library module included). MIN/-1 division and remainder both refuse. Division
truncates toward zero. Emitted C retains calls and guards arithmetic, exiting 86
on a fault;
a fault writes nothing to either stream, and a source program without a print
writes nothing. A library check makes no claim that every possible argument is
overflow-free. An executable entry has no parameters and returns `i32` or
`unit`. Library modules check definitions but publish neither a closed
observation nor standalone C.

The C backend exposes file-scope fragments plus an entry-body contribution.
Those fragments contain no entry-point definition; the shared
`c_program::assemble` step emits exactly one `main`. Merging fragments uses the
established keys for the common IO include and prelude, so the prelude is
deduplicated before assembly. This is emission composition, not general
cross-profile source-call composition: a scalar function still cannot call a
function owned by another execution profile.

Limits are 256 functions, expression/call depth 64, 16,384 IR expression nodes,
16,384 tokens per function, and 100,000 evaluated expression steps. Parser nesting
and each left-associated binary chain also have a limit of 64. Exceeding any
limit refuses; no truncation is performed. Control flow, non-IO effects,
ownership, usage annotations, generic/indirect calls, and cross-profile call
composition remain outside this subset. Unsupported ordinary forms use Q020;
nonempty contracts retain Q021.
[Implementation and evidence](reports/source-scalar-calls-v1/README.md).

**Records across calls (card `W48-QR-USABLE-RECORDS`).** A module that declares
a record type (§G07.13), holds no buffer and declares a function besides its
entry reaches this route; every such module was refused before (the buffer
route models its entry only, `Q051`), and a record module with one function
keeps the buffer route byte for byte. A record of scalars — and of records of
scalars — is a plain value here:

```ebnf
parameter = NAME, ":", ( SCALAR | RECORD ) ;
result    = SCALAR | "unit" | RECORD ;
statement = "let", NAME, [":", ( SCALAR | RECORD )], "=", expression, ";" | … ;
unary     = … | RECORD, "{", [NAME, ":", expression, {",", NAME, ":", expression}], "}"
          | unary, ".", NAME, {".", NAME} ;              (* a field path *)
```

* A call **copies** a record argument in and a `return` copies a record result
  out: no custody is involved, and a callee building a new record leaves the
  caller's as it was. A parameter is a record by value only — `own<R>`,
  `borrow<R>`, `ptr<const, R>` and `move p` refuse at `Q020`.
* A construction names every declared field exactly once, in any order; the
  checker types each value at its field's type and orders them as declared.
  A field path names declared fields, each but the last a record. A record
  has no arithmetic, comparison, conversion or rendering; read the field.
* The record declarations are read by §G07.13's own reader; a type holding an
  owned `bytes` field anywhere refuses by name (`W48-QR-OWNED-RECORD-ACROSS-CALL`),
  a field write refuses (its rows admit `io` and the typed error, and a write
  derives `mut`: `W48-QR-SCALAR-CALLS-FIELD-WRITE`), and a function whose row
  permits `throws<ArithmeticError>` returns a scalar or `unit`
  (`W48-QR-RECORD-THROWS-RESULT`), each at `Q020`.
* The emitted C declares one `struct qcore_record_NAME` per record, a held
  record before its holder, and passes and returns it by value.

<a id="g02t-fixed-width-scalar-types"></a>

## G02t. Fixed-width scalar types

Card `W48-QR-SCALAR-TYPES` makes the §G05 `scalar_abi` table first-class values
of `.qr` sources. It is implemented on `source.scalar-calls.v1` (§G02s) and
rendered through `std.fmt`/`std.io`/`std.logger` (§G02g); every other position
keeps its existing refusal (table below). The definitions are
`crates/qcore-core/src/scalar_types.rs`, stated in Lean as `QCore.ScalarTypes`.

| Type | Values | C11 | `dec` | `hex` |
|---|---|---|---|---|
| `bool` | `false`, `true` | `bool` (`<stdbool.h>`) | `false` / `true` | refused |
| `i8` `i16` `i32` `i64` | two's-complement range of the width | `int8_t` … `int64_t` | signed decimal | `0x` + 2/4/8/16 digits |
| `u8` `u16` `u32` `u64` | `0 ..= 2^w - 1` | `uint8_t` … `uint64_t` | unsigned decimal | `0x` + 2/4/8/16 digits |
| `isize` | signed range of the recorded width `W` | `ptrdiff_t` | signed decimal | `0x` + `W/4` digits |
| `usize` | `0 ..= 2^W - 1` | `size_t` | unsigned decimal | `0x` + `W/4` digits |

* **Typing.** No implicit conversion exists anywhere. An arithmetic or
  comparison operand pair has one type; two different types refuse ("mixed-type
  operands"), and so does a `let`, argument or return whose type differs from
  the expression's. An integer literal takes its type from its context and
  defaults to `i32`, which is what keeps every existing program's typing
  unchanged; it must be a value of that type (the minimum included), else it
  refuses.
* **Checked arithmetic.** `+ - * / %` and unary `-` are defined on the ten
  integer types and compute the exact mathematical result, which must be a
  value of the type; otherwise the operation faults. A zero divisor faults, and
  so do `MIN / -1` and `MIN % -1` of a signed type. Division truncates toward
  zero. Unary `-` is `0 - x`, so on an unsigned type it succeeds only at `0`.
  There is no wrapping operation. On this route every operand of an executable
  entry is a check-time constant of its closed evaluation and every
  literal-only subexpression is folded while it is typed, so the fault is
  decided at check time and refuses (`Q020`); the emitted C still tests the
  operands against the type's limits **before** each operation, with explicit
  range tests rather than compiler builtins, and exits 86 on a fault.
* **Conversions.** `EXPR as TYPE` is the one spelling. It is **total** when
  every value of the source type is a value of the target (a widening), and
  then it is a plain cast; otherwise it is **checked** — the same number at the
  new type, or the fault when the number does not fit — never a truncation or a
  reinterpretation. There is no conversion to or from `bool`. `as` binds
  tighter than every binary operator and looser than unary `-`. The word `as`
  was already reserved, and a scalar type name after it is not collected as a
  reference by name resolution; the constraint-14 census
  (`docs/reports/w48-qr-scalar-types/census_spelling.py`) found no `as` followed
  by a scalar type name in any of the 625 tracked `.qr`/`.qv`/`.qrlib` files.
* **`bool`.** Comparisons produce `bool`. `bool` has `==` and `!=`, no ordering,
  no arithmetic, no conversion and no `hex` rendering.
* **`isize`/`usize` and the recorded target.** Their width is the builder's
  recorded target (`quirin --recorded-target FILE`, a closed three-key record:
  `pointer_width` 32 or 64, `triple`, `compiler`; total or reject). A program
  naming either type with no record refuses; nothing guesses a width. With one,
  the program is checked for exactly that width, the result carries it as a
  named assumption (`recorded target: …` among the declared assumptions), and
  the emitted C `_Static_assert`s `SIZE_MAX`, `PTRDIFF_MIN` and `PTRDIFF_MAX`
  against it, so a C compiler for another width refuses to build it. A program
  naming neither is unaffected by a record.
* **Rendering.** `print dec|hex X;` and `log LEVEL dec|hex X;` render every
  type: `dec` the number itself, `hex` the type's own bit pattern (two's
  complement for a negative value) in upper-case digits zero-padded to a
  quarter of the width, which for `i32` is the established `0x000000FF`. A
  `bool` renders `true`/`false` under `dec` and refuses `hex`.

**The typed arithmetic error** (card `W48-QR-SCALAR-TYPES-R2`). The builtin,
payload-free `ArithmeticError` has three variants: `Overflow` (an exact result
outside the type, `MIN / -1` and `MIN % -1` included), `DivisionByZero` (a zero
divisor of `/` or `%`) and `OutOfRange` (a conversion whose value does not fit).
It is the task profile's error discipline (§G02a) on this route:

* **Permission.** A function raises it only when its row lists
  `throws<ArithmeticError>`; the permission is never inferred, so an absent row
  permits no error and `effects { throws<ArithmeticError> }` alone excludes
  `io`. Any other error name in a row refuses.
* **Where it is raised.** In a function with the permission, an operation
  whose operands are not all literals — a parameter, a local, a call result —
  raises its fault's variant instead of faulting. A fault decided from
  literals alone (the fold of §G02s) is refused at check time whatever the row
  permits, and a function without the permission keeps the route's closed-entry
  refusal. Each is the same fault: `QCore.ScalarTypedErrors.static_fault_variant`
  and `static_fault_is_dynamic_raise` prove the fold's refusal is the raise the
  dynamic path would take.
* **Consumption.** A call of a function that may raise must be consumed:
  `CALL?` propagates the error unchanged and needs the caller's permission
  (`CALL?;` stands as a statement for a `unit` callee), and
  `match CALL { Result::Ok { value: v } => { … } Result::Err { error: e } => { … } }`
  runs the arm the outcome names — exactly one arm of each, in either order,
  over a callee with a scalar result. Every block ends in `return`, `raise` or a
  `match`, so no arm falls through; an arm's binding is in scope in that arm
  only. `raise e;` re-raises a bound error and `raise ArithmeticError::VARIANT;`
  raises one explicitly, both under the permission. An unconsumed call, and `?`
  or `match` on a callee that cannot raise, refuse.
* **Emission.** A function with the permission returns its error tag as an
  `int` (`0` success; `Overflow` 1, `DivisionByZero` 2, `OutOfRange` 3) and its
  value through a trailing pointer. Every operation in it — `i32` included —
  calls a `q_scalar_try_*` helper that tests the operands before computing and
  returns the tag instead of exiting 86, and the function returns that tag at
  once; `match` branches on it. An entry that propagates the error publishes it
  as its error outcome (`ArithmeticError::VARIANT` with its tag) and exits with
  status `80 + tag` after the lines it already wrote. A function without the
  permission emits exactly what it emitted before.
* **Reservation.** No word is reserved. `match`, `raise`, `Result`, `value` and
  `error` are the task profile's; `ArithmeticError` is skipped by name
  resolution only where `::` follows it, and the constraint-14 census found it
  in no tracked source file.

**Annotations every route admits** (card `W48-QR-SCALAR-TYPES-R2`). Every
route checks every annotation it admits. The sequential route checks
parameters against this table (§G02). The six routes that model their entry
only — `source.std-io.v1`, `source.buffer.v1` (with its record and loop
forms), `source.string.v1`, `source.socket.v1`, `source.http.v1` and
`source.event.v1` — used to admit any other function unread: its parameter,
result and local annotations and its body. Each now refuses such a function at
its own existing "models the entry only" code (`Q030`, `Q051`, `Q068`, `Q080`,
`Q090`, `Q058`). A module with `std.io` lines, scalar-call syntax and a helper
reaches this route instead, which checks every function.

**Positions.** Widened by card `W48-QR-SCALAR-TYPES`: parameters, results,
annotated and inferred locals, literals, arithmetic, comparisons and
conversions on `source.scalar-calls.v1`, its line statements, and the `|>`
sinks that desugar to them. Widened by card `W48-QR-SCALAR-TYPES-R3` (below):
the record fields of `source.buffer.v1` and that route's whole value
environment (§G07.13, §G02k), and extern results as values on the extern and
composed extern + io routes (§G02d, §G02i). Widened by card
`W48-QR-SCALAR-TYPES-R4` (below): task worker parameters and results, `copy`
captures, value statements in every task function, and the composed tasks + io
operands (§G02a, §G02j). Widened by card `W48-QR-SCALAR-TYPES-R5` (below):
loop bounds and indices (§G07.12) and the value statement on
`source.buffer.v1` (§G02k). Widened by card `W48-QR-SCALAR-TYPES-R6` (below):
the `Buffer` payload and the `protected` payload (§G02b, §G02c), the composed
epoch route's task half (§G02f), the fixed-width worker outcome in the F02
observable alphabet, and the whole tier on the composed string/buffer route
(§G07.9). Every other position that takes `i32` keeps its existing refusal
and is a named follow-up: an error variant's payload, which stays the `i32`
Buffer (§G02bc, `W48-QR-TYPES-ERROR-PAYLOAD`), and an executable entry result
other than `i32`/`unit` (C's `main` returns `int`; the entry's value is
published, never returned; a task entry's likewise). A typed `let` beside an `std.io` line is
scalar-call syntax, so such a module reaches this route and needs no change
to `source.std-io.v1`. A conversion written on one of those routes was
refused at `Q013` before this card (name resolution collected the type name);
it now reaches that route's own reader and is refused at that route's own code.

**The buffer route's typed environment** (card `W48-QR-SCALAR-TYPES-R3`,
§G07.13, §G02k). Every name `source.buffer.v1` binds to a scalar has exactly
one §G02t type: a literal `let`, a byte read, an `extent` and a loop index are
`i32`, as before; a read of a record field has the field's declared type; a
conversion has its target type; a copy bind keeps its source's type. Every
position checks the type it takes — an index, a stored byte, a lend bound and
a loop bound take an `i32` and refuse a name of any other type (`Q052`, "`x`
is a `u8` and this position takes an `i32`"); a field value takes exactly its
field's type; a rendering operand takes any type and renders it at that type.
The route had **no arithmetic** — it never had an expression — so the one
operation this card gave it is the conversion statement (card
`W48-QR-SCALAR-TYPES-R5` adds the value statement, below) `let NAME = OPERAND as TYPE;`
(one operand: an `i32` literal or a bound name). Every value on this route is a
check-time constant, so a conversion that does not fit is refused at check
time (`Q053`, the range condition) and the emitted C calls a helper whose
operand test — §G02t's own, shared with `source.scalar-calls.v1` — is a
witness that aborts, the route's discipline. Nothing converts to or from
`bool` (`Q052`), and `isize`/`usize` in a field or a conversion need the
recorded target (`Q051`), which the route receives and publishes as the named
assumption.

**Extern results as values** (card `W48-QR-SCALAR-TYPES-R3`, §G02d, §G02i).
A bound extern scalar result has its declared result's §G02t type, a literal
binding is an `i32`, and the extern route gains the **value statement**, one
§G02t operation per statement over bound values and literals:

```ebnf
value_statement = "let", NAME, "=", value_expr, ";" ;
value_expr = operand, ("+" | "-" | "*" | "/" | "%"), operand
           | operand, ("==" | "!=" | "<" | "<=" | ">" | ">="), operand
           | "-", NAME
           | operand, "as", SCALAR ;
operand    = NAME | ["-"], INTEGER | "true" | "false" ;
```

A literal operand takes the other operand's type (`i32` when both are
literals). A value is **known** when its number is a literal, a declared
`succeeds INTEGER` outcome, or a statement over known values; a statement over
known values is folded at check time — a fault refuses (`Q024`), naming the
variant the typed path would raise. A statement reading a value known only at
run time (an undeclared result) that can fault — arithmetic, negation, a
conversion that is not total — is admitted only when the entry's row declares
`throws<ArithmeticError>`, and then takes the typed path: the emitted program
calls §G02t's `q_scalar_try_*` helper and, on a nonzero tag, logs the raise and
exits `80 + tag`. A comparison and a total conversion never fault. A declared
outcome is now read at any width and range-checked against the declared
result (`Q024`); with no recorded target, an `isize`/`usize` outcome is carried
unchecked as before and any statement computing with it refuses. At a
declared scalar parameter an argument is a value of exactly the parameter's
type, and a literal argument is range-checked against it (`Q024`): there is no
implicit conversion at the extern boundary either. A value statement calls
nothing and is observed as nothing in the call trace. On the composed
extern + io route a rendering reads a known value of **every** §G02t type at
that type (before the card, an `i32` only); a value known only at run time
still refuses (`Q043`, "no model computed"), and `isize`/`usize` render only
with a recorded target (`Q043`).

**Task workers and `copy` captures** (card `W48-QR-SCALAR-TYPES-R4`, §G02a,
§G02j). On the scoped-task route a worker parameter and a worker result may be
any type of the table, a local annotation any type (a literal after it takes
its type, range-checked), and a `copy` capture copies a value of any type into
a parameter of exactly that type — there is no conversion at the boundary, and
the transferability judgment classifies the other ten types as one duplicated
kind beside `i32` (`TransferKind::FixedScalar`, `QCore.Transferability`). A
task entry's result stays `i32`, `unit` or its error. Every task function —
the entry and every worker — gains the extern route's **value statement**
(the grammar above, read by the same reader and decided by the same decision,
`qcore_core::scalar_values`): a statement over values known at check time (a
literal, a copy of a known value, a statement over such values) is folded and
a fault refuses (`Q020`); a statement reading a value known only at run time —
a parameter, which is what a capture is inside the worker, a joined outcome, a
buffer read — that can fault is refused without the function's
`throws<ArithmeticError>` (`Q300`) and with it lowers to a checked step whose
error edge is the route's **ordinary error exit**: the scope exits, then
`raise` of the classified variant. So a raise is an exit like any other, and a
worker that still holds an owned Buffer there is refused (`Q600`).
`ArithmeticError` is a builtin error type of the route, payload-free, beside
`SpawnError` and `AllocError`, and like them may not be redeclared (`Q013`); a
worker's `ArithmeticError` is an ordinary typed worker error, which `join`
collects and the creator consumes with `?` (under its own permission) or
`match` (`QCore.ScalarTasks.worker_error_reaches_creator_unchanged`). The
emitted C widens the task value by a union holding one member per fixed-width
type the module names, calls §G02t's `q_scalar_*` / `q_scalar_try_*` helpers,
and logs a fixed-width worker result under the name `=TYPE`; a module that
names none of the table's other types and performs no value statement emits
exactly the bytes it always did. A composed rendering reads a value of any type
at its type, in the parent or in a worker; `bool` has no `hex` (`Q050`). A
fixed-width worker outcome has no spelling in the F02 observable alphabet, so
such a program sits outside F02 relational comparison (`reference_trace`
refuses it) and its native evidence is the byte comparison of the maintained
trace, whose sink prints `QCORE_TASK_RESULT TASK TYPE NUMBER`.

**Typed loops and the buffer route's value statement** (card
`W48-QR-SCALAR-TYPES-R5`, §G07.12, §G02k). On `source.buffer.v1` a `for` bound
may be a value of any integer type of the table, and the **index has the
bound's type**: the low end is the literal `0`, which takes the bound's type as
every literal takes its context's, so a range has one type and no mixed-type
range is expressible. A literal bound is an `i32`, so a bound of another type
is a name of the lowering's constant tier (§G07.12), which now also holds a
conversion or a value statement over the tier's own values, folded by the
shared decision (`qcore_core::scalar_values`) under the recorded target. The
bound is decided at every type by the loop's own rules, in `LOOP-V1`'s order —
a `bool` bound, an `isize`/`usize` bound with no recorded target, a negative
bound, a zero bound and a bound above 4096 refuse (`Q098`), and the count must
be a value of the index type — and the budget is unchanged. Because the count
is at most the ceiling and a value of the type, every index value
`0 … count-1` and the final step's `count` are values of it: the emitted
`i += 1` never overflows or truncates, and the loop runs exactly `count` times
(`QCore.ScalarLoopBounds.the_loop_runs_exactly_count_times`); a `u64` near its
maximum and a signed bound near `MIN` or `MAX` are refused by the ceiling or
the sign rule before any index exists. The checker re-derives the count from
the bound's value in its own typed environment and binds the index at its type
per iteration; a typed index is refused at an `i32` position (`Q052`) and is
converted with `as` to be one. The emitted loop is
`for (T i = 0; i < BOUND; i += 1)` at the type, against the bound's own C
object; a loop over a literal, a literal `let` or an `extent` is exactly the
`i32` loop it was.

The route also gains the **value statement** of the extern and task routes —
`let NAME = A OP B;`, `A CMP B` or `-NAME`, one operation, read by the same
reader and decided by the same decision (its `as` form is the route's existing
conversion). Every value on this route is known at check time, the loop index
once per iteration, so every statement is folded: a fault refuses (`Q053`,
naming the variant the typed path would raise), a literal outside its type
refuses (`Q053`), mixed operand types and `bool` arithmetic, negation or
ordering refuse (`Q052`), a target width with no record refuses (`Q051`), and
a chained expression refuses (`Q051`). Nothing on the route raises, so it needs
no `throws<ArithmeticError>`. The result is a value of its type: renderable,
convertible, copyable, a loop bound when its operands are of the constant
tier, and refused at an `i32` position unless it is an `i32`. The emitted C
calls §G02t's shared `q_scalar_*` helpers, whose fault test is a witness the
checker proved never taken, and makes each comparison through a two-parameter
function. Card R5 refused the tier on the composed string/buffer route
(§G07.9) by name (`Q077`); card R6 admits it there (below).

**Typed payloads** (card `W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-PAYLOADS`,
§G02b, §G02c). The payload type is part of a Buffer's type:

```ebnf
buffer_type = ("own" | "borrow" | "lend"), "<", "Buffer", ["<", SCALAR, ">"], ">" ;
protected   = "let", NAME, "=", "protected", (NAT | SCALAR), ";" ;
```

`own<Buffer>` is `own<Buffer<i32>>` — every Buffer before the card, with its
bytes unchanged — and `own<Buffer<T>>` holds one value of any §G02t type `T`
(likewise `borrow<…>` and `lend<…>`). The payload type is named at
acquisition by the initializer: `qcore_buffer_new(VALUE)` holds a payload of
`VALUE`'s own type (an `i32` literal, or a binding of any type); a whole
read yields a value of the payload type and a whole write takes exactly one,
a literal ranged against it. Types are compared exactly at every boundary: a
capture, a lend and a lease deliver a Buffer of the parameter's payload type
and nothing converts. A range write lease's byte extent — and an owner's byte
read's — is the payload type's **size** (`bool` 1, `i8`/`u8` 1, … `i64`/`u64`
8, `isize`/`usize` the recorded width over 8, never guessed), not a fixed
four; a lease past it refuses at the capture (`Q600`), and so does a byte
index past it. A byte **write** into a `bool` payload refuses (`Q600`): a byte
other than 0 or 1 is not a `bool`, so a `bool` payload's byte view is
read-only. An error variant's payload stays `own<Buffer>`, the `i32` payload;
`V(own<Buffer<T>>)` refuses at the declaration (`Q300`,
`W48-QR-TYPES-ERROR-PAYLOAD`). The `WRITE-LEND-TASKS` exclusivity holds over
typed extents — no byte has two write authorities, and every leased byte is a
payload byte (`QCore.ScalarPayloads`). The emitted C holds a typed payload in
its own member beside the historical `int32_t`, `_Static_assert`s each payload
type's size, and logs a typed payload event under a `=TYPE` name through an
adapter arm installed only where needed; its trace line is the historical one
with `TYPE NUMBER` for the number (`QCORE_BUFFER_ALLOC 0 1 U16 258`), so every
`i32`-payload program keeps its bytes and its F02 trace. The F02 alphabet has
no typed payload event, so such a run is refused by the reference encoding
rather than narrowed (`W48-QR-F02-TYPED-PAYLOADS`). `protected TYPE` declares
a mutex-protected payload of the type's size (`protected i32` is
`protected 4`, byte for byte); the route has no value position, and a write
into a `protected bool` stores the fill's low bit, so the cell is a `bool`
after every write. `isize`/`usize` payloads need the recorded target, which
the result names.

**The composed epoch route's task half** (card `W48-QR-SCALAR-TYPES-R6`,
`W48-QR-TYPES-EPOCH-TASKS`, §G02f). The task half of `source.epoch.tasks.v1`
admits the whole task-route tier — worker parameters and results of every
type, `copy` captures of any scalar, typed payloads and the value statement —
under the recorded target the builder records. The epoch's derived shape reads
no scalar type (a `copy` of any type is `copy;`; a result of an owned Buffer of
any payload type is retained), every byte figure stays declared, and a
worker's `ArithmeticError` is the join-failure edge the recovery plan already
carries. A creator's raising statement is a creator exit, admitted by the
scope rule only with no child of the scope outstanding (`ScopeBusy`
otherwise), so it abandons no credit (`QCore.ScalarEpochTasks`).

**Fixed-width outcomes in F02** (card `W48-QR-SCALAR-TYPES-R6`,
`W48-QR-F02-FIXED-OUTCOMES`). The F02 observable alphabet gains the
fixed-width worker outcome, parsed from `QCORE_TASK_RESULT TASK TYPE NUMBER`
(`true`/`false` for `BOOL`; a number outside its type, and an unknown type
name, do not parse). A run whose worker returns one is encoded and related,
not refused; a trace without one relates exactly as before, and no pinned
digest moves (`QCore.TaskTraceFixedOutcomes`).

**The composed string/buffer route** (card `W48-QR-SCALAR-TYPES-R6`,
`W48-QR-TYPES-COMPOSED-STRING`, §G07.9). A composed module — one holding a
buffer — reads a top-level conversion and value statement with
`source.buffer.v1`'s reader, and its loops (any integer index type, typed
bounds of the lowering's tier) and their bodies' conversions and value
statements as before; all of them are decided by that route's runner over its
typed environment, under the recorded target. A typed name renders at its type
at the top level and in a body, and refuses at an `i32` position of this route
(a slice bound, a composed piece) by name (`Q077`); every other refusal is the
buffer route's own under this route's `Q077`. A typed statement stores no byte,
so the borrow window is decided exactly as for an `i32` program
(`QCore.ScalarComposedString`). The emitter keeps one typed environment for
the whole entry and defines each helper once.

**Emitted C under `-Werror`** (card `W48-QR-SCALAR-TYPES-R6`). A scalar binding
of `source.buffer.v1` that nothing after it in its block reads — a loop bound
whose `i32` loop is written against its literal count, a body binding no later
body statement reads — is followed by `(void) NAME;`, as the string route
already did (`W48-QR-LOOP-BOUND-NAME-UNUSED-C`); every value-statement
comparison on the scalar, extern and task routes is the shared two-parameter
function `q_scalar_CMP_T`, so a value compared with itself builds
(`W48-QR-INLINE-COMPARISON-WERROR`).

**Every extern declaration is read** (card `W48-QR-SCALAR-TYPES-R3`,
`W48-INERT-EXTERN-DECLARATION-TYPES`). An executable or library module's
`extern fn` declarations are read whether or not a body calls them: each
parameter must be a §G02t scalar or one of the three custody pointers of
§C04.1, and the result that or `unit`; anything else refuses at `Q020`,
naming the parameter or the result. The check runs after the admitting route's
own passes, so every refusal given before keeps its bytes. A contract module is
exempt: its declarations are the contract surface itself, in the resource
contract language's wider vocabulary, read by the contract loader.

**Floating point: `f32` and `f64`** (card `W48-QR-FLOATING-POINT`;
semantics and normative limitations in specification §L14.7). The table gains
two types, outside the integer table: `ScalarType::parse` still reads the
eleven above, and a position admits a float only by reading its types with
`parse_admitting_floats`, so every position this card did not widen keeps its
refusal.

| Type | Values | C11 | `dec` | `hex` |
|---|---|---|---|---|
| `f32` | every IEEE 754 binary32 bit pattern | `float` | shortest round-trip, ECMAScript layout | C hex float, normalised |
| `f64` | every IEEE 754 binary64 bit pattern | `double` | shortest round-trip, ECMAScript layout | C hex float, normalised |

```ebnf
FLOAT    = DIGITS, ".", DIGITS, [EXPONENT] | DIGITS, EXPONENT ;  (* one token *)
EXPONENT = ("e" | "E"), ["+" | "-"], DIGITS ;
effect   = ... | "partial_eq" | "partial_ord" ;                 (* each once *)
```

* **Lexing.** `DIGITS . DIGITS` was already the lexer's `Float` token; card
  `W48-QR-FLOATING-POINT` extends it with an exponent written directly after
  the digits, taken only when a digit follows the `e` and its sign, so `1.5e`
  and `1e+` lex as before (a literal and the name `e`). The census
  (`docs/reports/w48-qr-floating-point/census_float_spelling.py`) found no
  tracked file spelling an exponent form.
* **Literals.** A float literal takes its context's float type, `f64` when
  there is none (a literal-only operand of `as` to a float type takes that
  type), and is read once at it; one that rounds to ±inf or a nonzero one that
  rounds to zero refuses. An integer literal is never a float, a float literal
  never an integer. `-` is unary minus, exact on every float. There is no
  literal for NaN or an infinity (write `0.0 / 0.0`, `1.0 / 0.0`) and no
  hexadecimal float literal.
* **Operations.** `+ - * /` and unary `-` are IEEE (round to nearest, ties to
  even) and never fault; `%` refuses. `f32` and `f64` do not mix. The emitted C
  performs each operation in its own helper (`q_scalar_add_f64`), so no C
  expression holds two float operations.
* **Comparisons** need their effect in the declared row — `partial_eq` for
  `==`/`!=`, `partial_ord` for the orderings — never inferred; on the scalar
  route a caller declares every comparison effect its callees derive. Integer
  and `bool` comparisons are unchanged.
* **Conversions.** `as` from an integer or `f64` to `f32`, and from an integer
  to `f64`, rounds to nearest, ties to even, and is total; `f32 as f64` is
  exact; a float to an integer truncates toward zero and faults (`OutOfRange`)
  for NaN, ±inf and a value outside the target, tested in C against exact
  hexadecimal thresholds before the cast.
* **Renderings.** `dec` prints the shortest decimal that reads back to the same
  value at its type, laid out as ECMAScript's `Number::toString` (`1.5`,
  `100000000000000000000`, `1e+21`, `0.000001`, `1e-7`) with `-0` for negative
  zero; `hex` prints `0x1.8p+1`, `0x1p-1074`, `-0x0p+0`; a NaN renders `nan`
  and the infinities `inf`/`-inf` under both. The emitted renderers
  (`q_float_dec_T`, `q_float_hex_T`) use no C library formatting.
* **Environment.** A program holding a float includes `<float.h>` and
  `<string.h>`, `_Static_assert`s the binary32/binary64 parameters and
  `FLT_EVAL_METHOD == 0`, `#error`s under fast-math or finite-math-only, and
  names the IEEE premise among its assumptions.

| Position | Floats |
|---|---|
| §G02s parameters, results, locals, literals, arithmetic, comparisons, conversions, renderings, records (card `W48-QR-USABLE-RECORDS`) | admitted |
| §G02d extern declarations (`AbiType` `f32`/`f64`, `float`/`double`), literal arguments, declared outcomes (`succeeds 1.5`, compared bit for bit), `let NAME = FLOAT;` (`f64`), value statements; §G02i rendering | admitted |
| §G02d.1 unchecked declarations | admitted (the `Q033` vocabulary now has them) |
| §G02u `struct!` fields (`float`/`double` members), `#c_ffi` parameters and results, literals, outcomes, helpers | admitted |
| §G02a/§G02j worker parameters, results, `copy` captures, value statements, record leaves (card `W48-QR-TASK-RECORD-CAPTURES`); §G02v closures; §G02f the epoch route's task half | admitted |
| §G07.13 records of `source.buffer.v1`, its conversions and value statements, loop bodies (§G07.12); the composed string route (card `W48-QR-FLOAT-BUFFER-PAYLOADS`, below) | admitted |
| §G02b `Buffer<f32>`/`Buffer<f64>` and §G02c `protected f32`/`protected f64` payloads (card `W48-QR-FLOAT-BUFFER-PAYLOADS`, below) | admitted |
| a loop bound | refused on every route, by name (`Q098`) |
| the `.qrlib` manifest's C declaration spellings (`double`, `float`) and the header reader's projection | admitted (card `W48-QRLIB-LINKAGE`, both manifest versions) |

**Floats on the byte-buffer route and in payloads** (card
`W48-QR-FLOAT-BUFFER-PAYLOADS`; limitations §L14.7 items 14–19).

```ebnf
EFFECT  = "io" | "alloc" | "mut" | "partial_eq" | "partial_ord" ;  (* source.buffer.v1, source.string.v1 *)
stmt    = ... | "let", NAME, "=", ["-"], FLOAT, ";"               (* an f64 *)
        | "let", NAME, "=", ["-"], FLOAT, "as", SCALAR, ";" ;      (* read once at a float SCALAR *)
field   = NAME, ":", (SCALAR | "f32" | "f64" | "bytes" | RECORD) ;
payload = "Buffer", "<", (SCALAR | "f32" | "f64"), ">" ;
protected = "let", NAME, "=", "protected", (NAT | SCALAR | "f32" | "f64"), ";" ;
```

* **`source.buffer.v1`** reads its record declarations, conversions and value
  statements with the float-admitting readers (§G07.13, §G02k): a field may be
  `f32`/`f64` and takes a float literal (read once at the field's type) or a
  name of exactly its type; `as` converts to and from floats; a value
  statement takes float literals and names; `let NAME = FLOAT;` binds an
  `f64`. Loop bodies read the same statements. Every value on the route is a
  check-time constant, so every statement is folded by the shared decision: a
  float → integer conversion that does not fit (NaN and ±inf included) is
  refused at check time (`Q053`), and the emitted C computes each operation
  again through §G02t's helpers and renders through the language's own
  renderers, under the float environment of this section. A float comparison
  needs `partial_eq`/`partial_ord` in the entry's row (`Q052` otherwise); the
  certificate publishes the comparison effects its statements used. A float
  loop bound — a float name, directly or through the lowering's constant tier
  — is refused by name (`Q098`) before its number is read. A float at an
  `i32` position (an index, a stored byte, a lend bound) is refused (`Q052`).
* **The composed string route** reads the same statements through the buffer
  route's reader, under its own entry row's `partial_eq`/`partial_ord`, and
  refuses each at its own `Q077`.
* **`Buffer<f32>`/`Buffer<f64>`** (§G02b): the payload's lease and byte extent
  is 4 or 8 bytes (`size_bytes`), a byte view is the value's IEEE object
  representation — every byte pattern is a value, so a byte write into a float
  payload is admitted — and whole writes take exactly the payload's type (an
  integer literal refuses, `Q020`). The payload event is logged as its bit
  pattern, copied with `memcpy`, and traced `F32 BITS`/`F64 BITS`, every NaN as
  `nan`, through a sink branch installed only in a program holding a float
  payload.
* **`protected f32`/`protected f64`** (§G02c) protect 4 and 8 bytes — `protected
  4`/`protected 8` to the model, byte for byte — and the emitted C asserts the
  IEEE environment.

**Diagnostics.** No code is allocated. Every new refusal on this route is a
widened condition of `Q020` (type, literal-range, mixed-type, `bool`,
conversion, recorded-target and fault refusals, and card
`W48-QR-SCALAR-TYPES-R2`'s typed-error refusals); a line statement's shape keeps
`source.std-io.v1`'s `Q030`, a `borrow` operand its `Q028`, and a missing
import the gate's `Q015`. The annotation rule above widens the sequential
route's `Q020` and each entry-only route's own code.

<a id="g02u-imported-c-structs-sourceffiv1"></a>

## G02u. Imported C structs (`source.ffi.v1`)

Card `W48-QR-C-STRUCT-IMPORT`, card 1 of the `.qr` language track, on five
user decisions (2026-09-29/30): the import line, the file-wide trust mark, the
import form, the `#c_ffi` marker on the contract side (contract spec §C04.12),
and output only through `std.fmt`/`std.io`/`std.logger`.

```ebnf
ffi_module    = "#import", "std.ffi.unsafe", ";", { import },
                { struct_import | extern_decl | entry | helper } ;
helper        = "fn", NAME, "(", [ NAME, ":", ( SCALAR | NAME ), { ",", … } ], ")",
                [ "->", ( "unit" | SCALAR | NAME ) ], [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
                "{", { stmt }, "return", [ value ], ";", "}" ;   (* W48-QR-USABLE-RECORDS *)
struct_import = "struct", "!", "{", "type", NAME, "=", C_STRUCT, "from", STRING, ";",
                "fields", "{", field, { ",", field }, [ "," ], "}", "}" ;
C_STRUCT      = "c", STRING ;          (* no byte between; the text is exactly `struct TAG` *)
field         = NAME, ":", SCALAR ;    (* §G02t; 1 to 32 fields, names distinct *)
extern_decl   = "extern", "fn", NAME, "(", [ param, { ",", param } ], ")", "->", FFI_RESULT, ";" ;
param         = NAME, ":", ( SCALAR | NAME | "ptr", "<", "const", ",", NAME, ">" ) ;
FFI_RESULT    = "unit" | SCALAR | NAME ;   (* NAME: an imported struct *)
entry         = "fn", NAME, "(", ")", "->", "unit", "effects", "{", EFFECT, { ",", EFFECT }, "}",
                "{", { stmt }, "return", ";", "}" ;
stmt          = "let", NAME, "=", NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
              | "let", NAME, "=", NAME, ".", NAME, ";"
              | [ "let", NAME, "=" ], NAME, "(", [ value, { ",", value } ], ")",
                [ "succeeds", outcome ], ";"
              | NAME, ".", NAME, "=", value, ";"                (* a listed field *)
              | "let", NAME, "=", NAME, ";"                     (* a copy *)
              | print_stmt | log_stmt | "flush", ";" ;          (* §G02g *)
assign        = NAME, ":", value ;
value         = [ "-" ], INTEGER | "true" | "false" | NAME ;
outcome       = [ "-" ], INTEGER | "true" | "false"
              | NAME, "{", assign, { ",", assign }, [ "," ], "}" ;   (* literal values only *)
```

* **The import enables it, and marks the whole file.** `#import std.ffi.unsafe;`
  is spelled like every bundled import and granted by no other module (the
  `std.extern.unsafe` precedent, §G02d.1): a `struct!` in a file without it
  refuses at `Q032`. A call still needs `#import std.extern;` (`Q022`), and a
  line its output module. Every result from the file holds under the
  **unsafe-FFI assumption** — that the imported layouts are faithful and each
  `#c_ffi` function behaves as its contract states — which the certificate
  names with every imported struct and every marked function called; the
  verdict is at best "Safe under the unsafe-ffi assumption" and never plain
  `Safe` (language specification §L14.6, §L20). This is not the `Unsafe`
  verdict.
* **Route selection.** `source.ffi.v1` is selected **before every other
  route**, by the import or by a `struct!`: an FFI file reaches no route that
  would publish without the assumption, and a declaration or statement this
  route does not model refuses (`Q026`) instead of reaching one.
* **The layout is the compiler's, never the source's.** The listing names
  fields and `.qr` types; no offset, size or alignment is written. The header
  resolves against the declaring file's own directory (one candidate path, no
  search). The builder's recorded target (`--recorded-target`) must name the
  running Clang — its first `--version` line as `compiler`, its `-dumpmachine`
  as `triple` — and that compiler's typed AST and record layout
  (`qcore_c_gateway::ffi_layout`) give each member's written and canonical
  type and its offset. A listed field must be a direct member whose canonical
  type is the canonical type of the field's §G02t C11 spelling under the same
  compiler (`int64_t` and `ptrdiff_t` are both `long` on LP64, so `i64` and
  `isize` both read a `long`; a `long long` reads as neither), and the listing
  must follow the layout order — that is how a listed field's offset is
  compared, since the listing states none. An unreadable header or context, no
  recorded target, another compiler, no complete `struct TAG` (an unknown tag),
  a union, a bit-field, a flexible array member, an anonymous member, any
  attribute on the record or a member (`packed`, `aligned`, `#pragma pack`),
  and a listed member with no §G02t equivalent refuse at `Q063`; a listed field
  that is not a member, of another type or out of layout order refuses at
  `Q064`.
* **Unlisted members are opaque bytes.** A read or a construction naming one
  refuses at `Q051`; a member of any type may stay unlisted. A construction
  zeroes the object before storing the listed fields.
* **Records are §G07.13's.** A construction supplies every listed field exactly
  once (`Q051`), each value of exactly the field's type (`Q052`; a literal
  range-checked, `Q053`); a read binds a value of the field's type. Card
  `W48-QR-USABLE-RECORDS`: `RECORD.FIELD = VALUE;` writes a **listed** field
  with a value of exactly its type and derives `mut` (an opaque or unknown
  member, `Q051`); `let q = r;` copies a record or a scalar. Nesting an
  imported struct stays `W48-QR-FFI-NESTED-AND-ARRAY-FIELDS`.
* **`.qr` helper functions (card `W48-QR-USABLE-RECORDS`).** A function other
  than the entry takes §G02t scalars and imported records **by value**
  (a pointer parameter refuses, `Q026`) and returns `unit`, a scalar or a
  record; an absent effect row is inferred, an explicit one must admit what
  the body does (`Q029`). A helper is checked at **every** call, with the
  values it is given — records copied — by the entry's own statement rules;
  the call graph is acyclic and at most 16 deep, and a helper the entry never
  reaches refuses (`Q026`). A helper call declares no outcome (`Q025`). The
  emitted C defines each as `static RESULT qr_fn_NAME(…)`, by value, and a
  declared outcome that disagrees inside one exits 70 through `exit`.
* **The mark is a file's.** In a bundle of several files, a function in a file
  that does not itself import `std.ffi.unsafe` refuses (`Q032`): no code is
  checked under the unsafe-FFI assumption in a file that never took it.
* **Calls.** A declared callee is admitted against the explicit contract
  selection, by name, parameter names and types. A position carrying an
  imported struct — by value or `ptr<const, NAME>`, a result by value — is
  admitted **only through a `[| quant #c_ffi |]` declaration** whose types name
  the same C tag; through an unmarked one it refuses at `Q033`. An unmarked
  custody-free scalar contract (every parameter borrowed, no result custody)
  may be called as well. A pointer result, `ptr<mut, …>` over a struct and any
  other type refuse at `Q024`; a signature disagreeing with its contract at
  `Q027`; an `unsafe extern fn` at `Q034`.
* **Linkage (card `W48-QRLIB-LINKAGE`).** Every admitted callee resolves
  through a **generated** `.qrlib` (§G02d.2.1) that its declaring file binds
  with `extern library NAME;` (§G02d.3): no binding, a binding to a v1
  manifest (`Q065`), a callee the bound manifests do not bind (`Q066`), an
  entry naming another contract module or another C signature — struct
  positions compared by tag (`Q041`), a manifest header whose digest is not
  the header the `struct!` read, for a struct the callee carries (`Q064`), and
  a slice that is not total (`Q047`) refuse. The trusted premise "the builder
  links the library that supplies each callee" is gone: the builder keeps only
  the include path (`BuilderIncludes`), and each called function carries a
  linkage premise — symbol X from artifact Y@sha256 (static), or stub
  Y@sha256 plus runtime premise R (dynamic), under ABI premises Z — which the
  certificate must name for every call (`Q067`, the tripwire) and the verdict
  lists as `linked:NAME`. It says which code is linked, never that it is safe:
  the unsafe-FFI assumption is unchanged.
* **Declared outcomes.** A C result is a value with no number until the source
  declares one: `succeeds INTEGER` on a scalar result, `succeeds NAME { … }`
  with every listed field's literal on a struct result. The emitted C compares
  the real result and exits 70 before anything reads it. Rendering a value no
  outcome determined refuses at `Q043`.
* **Emission.** The C `#include`s the header as written (the builder puts its
  directory on the include path), uses the real `struct TAG`, never re-declares
  it, and asserts `sizeof`, each listed `offsetof` and each listed member's type
  (`_Generic`), so a header whose layout drifted after checking does not build.
  Each callee is declared from its contract signature, so a header declaring it
  otherwise makes the unit ill-formed. A `const` pointer argument is `&r_NAME`,
  a by-value one `r_NAME`.

Census (constraint 14, `docs/reports/w48-qr-c-struct-import/census_ffi_spelling.py`):
no tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` file contained `struct !`,
`c"…"`, `[|`, `|]`, `c_ffi` or a `std.ffi` import before the card; no word is
reserved. Card 4's capture list `|copy x, …|` lexes as single bars and is
untouched: `[|` and `|]` form only with an adjacent `[` or `]`.

<a id="g02r-resource-aware-functions-sourceresource-callsv1"></a>

## G02r. Resource-aware byte helpers (`source.resource-calls.v1`)

`source.resource-calls.v1` is a closed, executable extension for direct calls
that carry `std.buffer` byte storage across source function boundaries. The
type spelling is deliberately `bytes`: task execution's `Buffer` is a distinct
fixed four-byte resource and is refused on this route. Whitespace inside the
angle brackets is insignificant.

```ebnf
parameter = NAME, ":", ("i32"
          | "own", "<", "bytes", ">", "use", grade-one
          | "borrow", "<", "bytes", ">"
          | "lend", "<", "bytes", ">") ;
grade-one = "one" | "1" | "[", "1", ",", "1", "]" ;
result = "unit" | "i32" | "own", "<", "bytes", ">" ;
argument = scalar-operand | "move", NAME | "borrow", NAME
         | "lend", "(", NAME, ")" ;
call = FUNCTION_NAME, "(", [argument, {",", argument}], ")" ;
return = "return", ";" | "return", scalar-operand, ";"
       | "return", "move", NAME, ";" ;
```

Functions may also use the flat `source.buffer.v1` acquisition, extent, byte
load/store and drop statements, plus `let NAME = call;` or an unbound unit
call. Calls are direct. A local or parameter that shadows a helper name makes
the apparent call indirect and therefore refuses. The graph must be acyclic,
and every declared helper must be reachable from the selected entry; this v1
profile does not claim universal checking of an uncalled public library.

An owned argument transfers its one live slot and leaves the caller name stale.
A read loan permits whole-buffer reads; a write loan permits reads and writes.
Loans exist for one call and cannot be returned, stored, subloaned or relended.
The same owner may occur more than once in one call only when every occurrence
is a read loan. Each helper derives the minimum extent required of each byte
parameter from its checked accesses and transitive calls; every call site must
prove the actual extent meets that minimum. Owned results must be bound and
are transferred back to the caller. All custody operations are projected onto
the existing ownership checker, and the flattened byte events are checked by
the existing standard-buffer checker and certificate recheck.

The derived effect row contains `alloc` and/or `mut`, transitively. An absent
row infers them. An explicit row must contain all derived effects; no other
effect is supported. An explicit row uses commas between effects, permits no
trailing comma, appears at most once and consumes the rest of the header;
residual header tokens refuse. Allocation failure exits with status 86 and has no error
payload in this profile. Therefore an allocation or allocation-performing call
is admitted only when its failure cannot strand another live owner in the
caller or callee. This is an allocation-failure cleanup condition, not general
typed-error support.

Limits are 256 functions, 256 parameters per function, call depth 64, 16,384
source IR statements, and 32,768 expanded items. An expanded item is one
visited statement, emitted buffer event, or generated ownership operation,
including literal-argument binds and loan wrappers. All limits refuse rather
than truncate. The expanded-item bound applies while flattening the call DAG,
so repeated high-arity calls cannot cause exponential work under the depth
bound. Branches, loops,
records, extern calls, task `Buffer`, source contracts, local usage forms other
than `own<bytes> use one`, borrowed results, computed extents and indices, and
cross-profile calls remain outside this subset.

The profile allocates no diagnostic code. Ordinary signature, call-graph and
shape refusals use Q020; an empty contract block is an unsupported header at
Q020 and nonempty source contracts retain Q021; invalid usage
grades use Q100. Buffer statement shape uses Q051, extent obligations Q053,
inherited custody refusals Q054, certificate recheck failures Q056 and missing
transitive effects Q057. The Lean module proves bounded decision lemmas for
alias compatibility, affine slots, extent composition, acyclicity, transitive
effects and zero-owner allocation failure. It is not a proof of the complete
frontend-to-C translation.

[Implementation and evidence](reports/resource-helper-functions/README.md).

## G02a. Scoped-copy task execution

`source.tasks.copy.v1` uses the ordinary compiler commands and requires exactly
one `concurrency checked;` marker. All definitions are checked. The entry has no
parameters; leaf workers accept named i32 parameters and return i32/unit —
since card `W48-QR-SCALAR-TYPES-R4`, parameters and results of every §G02t type
(§G02t, "Task workers and `copy` captures"; the entry's result stays i32/unit).
Local scalar bindings are immutable. A return may use an i32 literal or live
scalar slot; unit uses bare return or a live unit result slot. Explicit supported effect rows permit
`spawn`, `join`, `may_block` and `throws<E>`; every actual effect/error must be
permitted. Payload-free error variants preserve their exact `E::Variant` tags.

```qcore
#import std.concurrency;
concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let collected = join move h;
        let y = collected?;
        return y;
    }
}
```

Creation succeeds or raises the distinguished SpawnError dynamically. `join`
collects the worker's typed Result; `?` is a separate propagation operation.
`let y = join move h?;` combines those operations. A bare join may discard only
an infallible unit result. Scope exits, returns and propagation reject live
handles or unconsumed Results, including the failure edge of a later spawn.
There is no automatic join. Sequential spawn/join pairs are supported.
An infallible `?` consumes the Result and extracts its scalar value without
inventing an error exit; the core requires an empty declared error set.

Explicit recovery uses exactly one of each canonical arm, in either order:

```qcore
match spawn s worker(copy x) {
    Result::Ok { value: h } => { let y = join move h?; return y; }
    Result::Err { error: failure } => { return 0; }
}
```

The scrutinee may instead be `join move h` or an existing Result binding.
Scrutinees are whole expressions without `?`. Each arm binds one local name;
its checked body may return, raise or continue with outer identities. Arm
bindings do not escape. `raise failure;` rethrows a bound error with its exact
tag; `raise E::Variant;` remains supported. Handled creation errors need no
`throws<SpawnError>` permission unless propagated. Spawn itself requires its
spawn effect. Prior live siblings remain obligations on every arm and must be
explicitly joined before exit. Other patterns, guards, arm values, arbitrary
match scrutinees and general branching remain unsupported. Explicit matching
of an infallible Result conservatively checks both arms; only infallible `?`
uses the empty-error extraction rule.

Captures must be `copy` of direct immutable i32 local/parameter names (since
card `W48-QR-SCALAR-TYPES-R4`, of any §G02t type, into a parameter of exactly
that type). General expressions, assignments, loops, general branches,
resources, loans, move/read/write captures, mutexes, externs, globals, user
contracts and task-bearing workers reject in every definition; the one
expression form admitted, by the same card, is §G02t's value statement — one
operation per `let`, with `ArithmeticError` raised through the ordinary error
exit. Entry-free task modules are not admitted yet.

**Records across tasks** (card `W48-QR-TASK-RECORD-CAPTURES`; §G07.13, §G02b,
§G02j). A task module may declare record types and use them in every task
function — construct, read, write, pass to a worker, return from one:

```ebnf
field     = NAME, ":", ( SCALAR | owned | RECORD ) ;   (* §G07.13's declaration *)
owned     = "own", "<", "Buffer", [ "<", SCALAR, ">" ], ">" ;
param     = NAME, ":", RECORD                  (* a record of scalars *)
          | NAME, ":", RECORD, "use", "one" ;  (* a record that owns a Buffer *)
result    = RECORD ;
capture   = ( "copy" | "move" ), NAME, [ path ]           (* a record or a sub-record *)
          | ( "copy" | "move" ), NAME, path               (* one scalar or owned field *)
          | "read", NAME, path, "share", SHARE
          | "lend", NAME, path, LOW, "..", HIGH ;
stmt      = "let", NAME, "=", RECORD, "{", assign, { ",", assign }, [ "," ], "}", ";"
          | "let", NAME, "=", NAME, [ path ], ";"      (* a field read, or a copy *)
          | NAME, path, "=", VALUE, ";"                 (* a field write, or a refill *)
          | "drop", NAME, [ path ], ";"
          | "return", [ "move" ], NAME, ";" ;           (* a record result *)
task_effect = … | "mut" ;                             (* in a module declaring a record *)
```

* **A record is its leaves.** A record binding is one task slot per scalar or
  owned-`Buffer<T>` leaf, depth-first in declaration order (card
  `W48-QR-USABLE-RECORDS`' leaf order); a record parameter is one worker
  parameter per leaf; a record capture is its leaves' captures. So every
  custody question about a record is the unedited checker's question about an
  owned leaf. A `bytes` field is `source.buffer.v1`'s byte buffer, which crosses
  no spawn here (`Q500`, `WRITE-LEND-TASKS-EXTENT`); a record type in a composed
  epoch module refuses (`Q500`, `W48-QR-TASK-RECORD-EPOCH`).
* **Crossing a spawn.** The transferability judgment's aggregate kind decides a
  whole-record capture from its leaves: a record of scalars crosses by `copy`
  (duplicated), a record that owns by `move` (custody of every owned leaf
  transferred, its scalars duplicated); `move` of a record of scalars, `copy`
  of one that owns (naming the owned leaf and the contract it lacks), `write`
  (by name, before any leaf is asked) and every sharing mode (a record shares
  no protocol whole) refuse at `Q600`, as does a record into a parameter of
  another record type (types are nominal). A successful creation moves a
  record whole: on that path it is used no more (`Q600`); a failed creation
  restores it exactly. A record moved into another record at construction is
  used no more either.
* **Field captures** are the existing captures of one leaf: `copy r.f` of a
  scalar field, `move r.buf` of an owned one (leaving a **hole**, used before a
  refill at `Q600` by the checker on that path), `read r.buf share N` and
  `lend r.buf LOW..HIGH` with the loan and lease rules over that leaf. The
  one-leased-resource-at-a-time bound is **per resource** since this card: two
  fields — two resources — may be leased to two workers at once; two leases of
  one field still may not share a byte, and a read share and a write lease
  still do not mix. A sub-record of scalars crosses by `copy`; a sub-record that
  owns crosses field by field (`W48-QR-NESTED-OWNED-SUBRECORD-MOVE`).
* **Results.** `return r;` returns a record of scalars and `return move r;` one
  that owns; `join` collects it whole and it is unpacked into fresh leaf slots
  where it is taken (`?` or the `Ok` arm). The entry's result stays `i32`/`unit`.
* **Operations in any task function**: construction (every field once; a scalar
  field a literal of its type or a name of exactly its type, an owned field a
  Buffer of its payload type moved in, a record field a record of its type
  copied or moved in), a field read (a scalar copied, an owned field moved out),
  `let q = r;` (a record of scalars; one that owns has no whole-move spelling,
  `W48-QR-RECORD-WHOLE-MOVE`), a field write (a fresh value of exactly the
  field's type; at an owned field the **refill**, admitted exactly into a hole,
  `OverwriteOfLiveField` otherwise) which derives `mut` — admitted in the row of
  a module that declares a record, and required (`Q300`) —, `drop r.buf;`, and
  `drop r;` of every owned leaf (a hole refuses), `qcore_buffer_read(r.buf)` and
  `qcore_buffer_write(r.buf, v)`. A record left in different states by two arms
  of a `match` that both continue is unsettled and used no more
  (`W48-QR-TASK-RECORD-ARM-MERGE`).
* **Emission.** A record result travels in the task result's `leaves` array
  (installed only in a module that returns a record); its trace is
  `QCORE_TASK_RESULT TASK RECORD N` followed by one result line per leaf. The
  F02 observable alphabet has no record outcome, so such a run is refused by
  the reference encoding rather than flattened (`W48-QR-TASK-RECORD-F02`).
  See [the card's report](reports/w48-qr-task-record-captures/README.md).

**Closures** (card `W48-QR-CLOSURES`, §G02v). A spawn may write its worker in
place, `spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }`; the capture
list uses exactly this section's capture spellings, and the closure is lifted
into a hidden named worker before any rule above applies.

The native driver supports Linux C11 threads, macOS through an emitted pthread
adapter, and Windows through an emitted winpthreads-derived C11 thread subset;
it explicitly rejects other targets. Its start record survives until successful native join; failed
join aborts without reclaiming possibly accessed storage. These rules do not
establish general C11/shared-memory or cross-platform refinement. See the
[recovery execution record](reports/w6-task-recovery/README.md).

## G02v. Closures with explicit capture lists

Card `W48-QR-CLOSURES` (user decision of 2026-09-29: an explicit capture list,
lifted into a hidden named worker; no implicit capture). On the scoped-task
routes (§G02a–§G02bc and the composed route §G02j) a spawn may write its worker
in place:

```ebnf
spawn      = "spawn", SCOPE, closure ;
closure    = captures, "->", TYPE, [ "effects", "{", EFFECTS, "}" ], block ;
captures   = "||"                                     (* no capture: one token *)
           | "|", capture, { ",", capture }, "|" ;
capture    = G02a_capture, [ "as", NAME ] ;           (* a named worker's spellings *)
G02a_capture = ( "copy" | "move" ), PLACE
           | "read", PLACE, "share", SHARE
           | "lend", PLACE, LOW, "..", HIGH
           | "write", PLACE ;                         (* refused by the judgment *)
```

```qcore
thread scope lanes {
    match spawn lanes |copy bonus, move job, lend pixels 0..2| -> Job effects { mut } {
        job.count = bonus;
        let low = qcore_buffer_write(pixels, 0, 17);
        return move job;
    } {
        Result::Ok { value: first } => { … }
        Result::Err { error: failure } => { … }
    }
}
```

* **The lift.** The parser lifts each closure into a **hidden worker** named
  `ENCLOSING#closureN` — the enclosing function's name and the closure's
  ordinal there in source order; `#` is no identifier character, so no source
  name collides — declared immediately before the enclosing function (a
  closure's own closures before it), with one parameter per capture, the
  written result type and effect row, and the closure's body. The spawn is an
  ordinary spawn of that worker with the written captures, in either spawn
  form: `let h = spawn … { … }?;` or `match spawn … { … } { arms }`. The
  closure's tokens stay where they are written; a statement that contains a
  closure reads its capture list and skips its header and body, which are the
  hidden worker's.
* **Parameter names and types.** A capture binds the place's root name, a field
  capture's last segment (`copy r.f` binds `f`), or the name after `as`. The
  **type** is the captured place's type at the spawn site, decided by the task
  route there: a scalar's type, a record's record type, an owned Buffer's
  `own<Buffer<T>> use one` (for `move`), its `borrow<Buffer<T>>` (for `read …
  share`) and its `lend<Buffer<T>>` (for `lend …`). That is the only type the
  transferability judgment could admit for the place and mode, so the judgment
  decides each capture exactly as for a named worker, and the lifted program
  checks and emits byte for byte as its hand-written twin — each closure a
  named worker before the enclosing function (`QCore.Closures`).
* **No implicit capture.** The hidden worker's scope is its capture binders and
  the module's globals (record, error and function names). A body naming any
  other name of the enclosing scope is refused (`Q013`), naming it and saying
  the closure does not capture it. A place captured twice, or two captures
  bound to one name, is refused (`Q011`); `as NAME` resolves a clash.
* **Required, not inferred.** The result type is required (`Q000` without
  `->`); the effect row follows the worker rules, so a closure that raises
  `ArithmeticError` declares `throws<ArithmeticError>`, and the error reaches
  the creator through `join` unchanged.
* **Spawn targets only.** A capture list anywhere but directly after
  `spawn SCOPE` — a closure stored in a binding, passed to a worker or called —
  is refused (`Q038`, `Q000`; `W48-QR-CLOSURE-VALUES`). A capture list never
  touches `[` or `]` (`Q000`): `[|` and `|]` are the quant delimiters.
* **Worker rules unchanged.** A closure that spawns — another closure in its
  own scope, or anything else — is a task-bearing worker and refused by the
  first-order leaf rule, at the closure's span; a closure has no name, so it
  cannot recur. Closures are lowered by the scoped-task and composed task
  routes only: on an epoch route (`W48-QR-CLOSURE-EPOCH`), the mutex route
  (`W48-QR-CLOSURE-MUTEX`), the field-footprint route (`W48-QR-CLOSURE-ROUTES`)
  and the FFI route (`W48-QR-TASK-FFI-STRUCT-WORKER`) a closure refuses at
  `Q600` by name.
* **Lexical.** No reserved word is added. `|` and `||` are claimed only
  directly after `spawn SCOPE` (and the closing `|` of that list); `as` is
  claimed only after a capture inside a closure's list. The census
  (`docs/reports/w48-qr-closures/census_closure_spelling.py`) found none of
  these shapes in any tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` file.

See [the card's report](reports/w48-qr-closures/README.md) and
[`examples/threads`](../examples/threads/README.md#closures).

## G02b. Owned-buffer task execution

`ScopedOwnedTasksV1` extends G02a with one concrete builtin type:
`own<Buffer>`. Each Buffer is a malloc-backed allocation with a fresh generation
and one i32 payload — or, since card `W48-QR-SCALAR-TYPES-R6`, one payload of
any §G02t type, `own<Buffer<T>>` (§G02t, "Typed payloads"). Its resource identity is separate from source
slot and scalar identities. This is not arbitrary `own<T>` elaboration.

```qcore
#import std.concurrency;
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> {
    return move buffer;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
```

Owned parameters require exactly `use one`. Captures must use `move` for owned
Buffer parameters and `copy` for i32 parameters, with exact arity and no duplicate
owned capture. Successful creation consumes the original parent bindings;
failed creation restores them, without altering prior siblings. A later join
returns custody through a fresh Result/value binding; it never revives the
original moved binding. Reading does not consume ownership; `drop name;` and
`return move name;` consume it. Repeated reads require the same live owner.

Card `W48-QR-TASK-RECORD-CAPTURES`: an owned Buffer may also be a **field** of a
record (§G02a, "Records across tasks"). A record that owns is a parameter
spelled `NAME: RECORD use one` and crosses by `move` — each owned leaf an owned
capture of this section, reserved, committed or rolled back with the others in
one creation — and a worker hands it back with `return move NAME;`, `join`
returning custody of every owned leaf at once. Duplicate owned captures, a
moved field used before a refill, and a record moved while one of its leaves
is lent or leased refuse by this section's rules, per leaf.

`qcore_buffer_new(I32_OPERAND)` requires `alloc` in the root or a leaf worker. It must either
be propagated with `?` under `throws<AllocError>`, or directly matched using the
same canonical Result arms as G02a. A failed allocation produces no Buffer or
new generation. Earlier owners must be released on that failure path. Local
annotations may be i32, unit or `own<Buffer>` and must match the initializer;
local quantitative annotations remain unsupported. `qcore_buffer_read(NAME)`
requires one live owned binding and returns i32. The builtin function names and
AllocError cannot be shadowed by source declarations.

Workers may read/drop owned inputs and locally allocated Buffers, return exactly
one remaining input or fresh local, or drop all of them before returning
scalar/unit or raising a payload-free error. The core derives this relation from
checked exits and applies exact caller capture identities or a checked fresh
result; return annotations alone cannot create custody. Multiple owned inputs
are supported. Owned root-entry results reject, including in uncalled definitions.
Workers remain nonrecursive task-free leaves. Every scope/return/error exit must
account for owned bindings, Results and handles; there is no implicit cleanup
or join in this profile. Local move aliases and general assignment are not
part of the current source grammar.

The native and reference allocation identity is `task * 2^32 + ordinal`, with
task in `0..2^32-1` and a per-task successful-allocation ordinal in `1..2^32-1`.
Root task identity is zero. Failed allocation consumes no generation; native
worker scheduling cannot change another task's counter. Native execution aborts
if these bounds are exhausted. Task-local fault scripts select allocation
attempts by successful runtime task identity. See the
[worker-allocation record](reports/w6-worker-allocation/README.md).

The finite task checker rejects unreachable blocks. Owned Result analysis uses
actual checked worker exits rather than inventing resource-bearing alternatives:
explicit matches with an impossible owned-result arm, and propagation from an
owned-return worker with an unused nonempty error row, may therefore reject.
Use infallible `?` for the empty-error owned-return case. General branch reasoning
and dead-code validation remain open.

Native worker buffer observations are retained privately and printed by the
parent after successful join. The transcript is a normalized collection order,
not a wall-clock schedule. C11 publication/join, allocator correctness and
source/core/native correspondence remain explicit refinement boundaries.
Lean model lemmas and fixed native pointer/sanitizer observations are separate
evidence. Owned graphs/error payloads, shared memory, mutexes, nested
workers, general control flow and targets other than Linux, macOS and Windows remain unsupported.
One bounded borrowing form is admitted by G02bb below; every other loan form
remains unsupported.
See [the owned task report](reports/w6-owned-source-tasks/README.md).

## G02bb. Scoped read loan

`ScopedReadLoansV1` extends G02b with one bounded borrowing form: a **scoped
read loan** of one parent-owned Buffer to one child, returned by the creator's
join. It is selected whenever any checked function has a `borrow<Buffer>`
parameter; a module that lends nothing keeps its G02a/G02b profile unchanged.

```qcore
#import std.concurrency;
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s reader(read buffer share FULL) {
            Result::Ok { value: handle } => {
                let during = qcore_buffer_read(buffer);
                let seen = join move handle?;
                let after = qcore_buffer_read(buffer);
                drop buffer;
                return seen;
            }
            Result::Err { error: failure } => {
                let restored = qcore_buffer_read(buffer);
                drop buffer;
                return restored;
            }
        }
    }
}
```

A loan parameter is spelled exactly `NAME: borrow<Buffer>`, with **no** usage
grade: the borrower may read it any number of times and may do nothing else
with it. Grading loan reads is not part of this profile. The only capture that
produces one is `read PLACE share FULL`, against a place whose type is
`own<Buffer>`. A `share` expression names how much read authority the borrower
receives; `share FULL` is the whole of it. Fractions are card T05's grammar,
G02bc below.

A read loan is not a value. It is refused as a function result, as an entry
parameter, as a copy source, as a move source, as a `drop` operand and as a
re-loan source; `qcore_buffer_read(NAME)` is the only operation that accepts
one. A worker cannot lend onward for two independent reasons: a loan is not an
`own<Buffer>` place, and a spawned worker is a first-order leaf.

While a loan is outstanding, the lender **keeps read authority** over its own
Buffer and loses every exclusive authority over it — mutation, release, move
and re-loan are all refused, naming the loan rather than a custody transfer.
The bounds are structural: at most one loan per creation, at most one
lent *resource* at a time, and one creation may not both move and lend the
same resource. It may move one Buffer and lend another.

Creation failure restores the lender exactly and opens no loan; the reservation
window before that outcome is known grants nobody anything, read included.
Only the creator's successful join closes a loan. A child that completed but
has not been joined still holds its loan. A path that never joins its borrower
cannot leave the scope: the scope-busy rule reaches that first, and the exit
rule's own loan check is a redundant backstop.

Native emission passes the same pointer for a move and for a loan and differs
only in what the caller keeps: a moved capture is consumed, a lent one is not.
The typed `TaskLoan` diagnostic event is submitted only after a creation that
actually returned a child, and `TaskReturn` only after a collection that
actually returned. Concurrent reads by lender and
borrower are admitted because nothing may write a loaned resource; that is a
checker rule, not a dynamic measurement — no race detector covers this backend,
because ThreadSanitizer does not intercept C11 `thrd_create`.

Write loans, loans of anything but a Buffer, loans across nested workers,
loans held past a scope and targets other than Linux, macOS and Windows remain unsupported. See
[the read-loan report](reports/remaining-t04/README.md). Fractional and
multi-reader shares are G02bc.

## G02bc. Counted read shares and the checked Buffer write

`ScopedReadSharesV1` extends G02bb by *counting* the lent authority. Each
`read PLACE share S` capture transfers `S` out of a per-resource budget of
`FULL` (1024, the same unit as the concurrency model's `FULL_ACCESS`), so one
allocation may be lent to at most two borrowers whose shares sum to no more
than `FULL`. It is selected when some capture lends a share other than `FULL`,
or when the module writes a Buffer; a module that does neither keeps its
G02a/G02b/G02bb profile unchanged.

```qcore
#import std.concurrency;
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(1)?;
    let prepared = qcore_buffer_write(buffer, 42);   // exclusive authority
    thread scope s {
        match spawn s reader(read buffer share 512) {
            Result::Ok { value: first } => {
                match spawn s reader(read buffer share 512) {
                    Result::Ok { value: second } => {
                        let during = qcore_buffer_read(buffer);
                        let a = join move first?;
                        let b = join move second?;
                        let restored = qcore_buffer_write(buffer, 7);
                        let after = qcore_buffer_read(buffer);
                        drop buffer;
                        return after;
                    }
                    /* … creation-failure arms elided … */
                }
            }
        }
    }
}
```

A share expression is `FULL`, a decimal literal, a parenthesised share, or a
sum of shares. It must evaluate to a positive value at most `FULL`; zero, an
over-full value and a named share are represented refusals, and a name that
does not resolve is refused earlier still by name resolution. This profile has
no declared share constants.

The budget is over the authority handed **out**. Lending transfers `S` from the
resource's unlent authority to the borrower; the outstanding total may never
exceed `FULL`, and an over-allocation is refused as a whole — no partial share
is ever granted. At most two holders exist at once; a third is refused by the
declared bound even when authority remains. The owner's own read is *not*
budgeted: it is retained across committed lending exactly as G02bb retains it,
and is sound only because a write demands the reconstructed exclusive
authority. The separation-logic reading, in which the owner's read is itself a
consumed fraction, is not this profile.

`qcore_buffer_write(BUFFER, VALUE)` evaluates to `unit`, takes one exact
`own<Buffer>` name and one i32, and demands exclusive authority. It is
therefore refused for the whole window in which any share is outstanding —
including after only one of two borrowers has been joined — and a borrower
cannot reach it at all, because a read loan is not an `own<Buffer>` binding.
Exclusive authority, and with it the write, the release and the move, return
only when the *last* share comes back: one join out of two reconstructs
nothing.

Two borrowers of one allocation are two holders of one identity. The backend
hands both the same pointer, so the allocation's live bytes are counted once,
and lending never consumes the lender's binding — only its release does.
The typed `ResourceWrite` diagnostic event is submitted by an admitted write;
the loan and return event meanings are unchanged from G02bb.

Disjoint slicing, write shares, more than two simultaneous readers, several
lent resources at once, nested workers and targets other than Linux, macOS and Windows remain
unsupported. See [the read-share report](reports/remaining-t05/README.md).

## G02bc. Tagged error payload

`ScopedErrorPayloadV1` extends G02b with one bounded failure-edge transfer: an
explicitly tagged error variant carrying exactly one owned Buffer out of one
child, through its completion capsule and its creator's join. It is selected
whenever any checked function declares an error payload; a module whose error
variants are all payload-free keeps its G02a/G02b/G02bb profile unchanged.

```qcore
#import std.concurrency;
concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
    match qcore_buffer_new(7) {
        Result::Ok { value: scratch } => {
            let seen = qcore_buffer_read(payload);
            drop payload;
            drop scratch;
            return seen;
        }
        Result::Err { error: exhausted } => {
            raise Recovery::Failed(payload);
        }
    }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                match join move handle {
                    Result::Ok { value: seen } => {
                        return seen;
                    }
                    Result::Err { error: failure } => {
                        let recovered = qcore_error_payload(failure);
                        let value = qcore_buffer_read(recovered);
                        drop recovered;
                        return value;
                    }
                }
            }
            Result::Err { error: refused } => {
                drop buffer;
                return 0;
            }
        }
    }
}
```

A payload variant is declared exactly `VARIANT(own<Buffer>);` inside an
`error NAME { … }` declaration. Scalar payloads, `borrow<Buffer>` payloads,
empty parameter lists and several payload fields are each refused at the
declaration. A declared error row is **uniform**: either every variant carries a
payload or none does, so a joined error always holds exactly one Buffer and the
parent's take is total. A mixed row is a represented refusal, not a widening.

`raise NAME::VARIANT(BUFFER);` moves one owned Buffer into the raised error, and
`raise NAME::VARIANT;` stays the payload-free spelling. The declaration decides
which is required, in both directions. `AllocError` and `SpawnError` never carry
a payload — the runtime raises them after a failure that acquired nothing — and
the task entry may not declare a payload row at all, because it has no external
custodian, exactly as it may not declare an `own<Buffer>` result.

`let NAME = qcore_error_payload(ERROR);` is the only way a payload leaves an
error. The error keeps its tag and loses its payload, so a second take is
refused, and until the take happens the bound error may neither be discarded at
an exit nor re-raised. A taken payload is an ordinary owned Buffer with the
ordinary duty: forgetting it is a leak on the path that forgot it.

A worker's error exit must account for every identity it acquired: locals it did
not carry must already be released, and the completion names exactly the one
identity the outcome carries. A creation that failed transfers nothing. A second
join of the same handle is refused, so the payload can never be collected twice.
One Buffer may be a normal owned result on one path and a payload on another;
the checker keeps the two exits apart.

Two core gates cannot be reached from this source surface, because the uniform
row rule and the entry rule refuse earlier: propagating a payload-carrying
Result with `?` from a function that also declares a builtin failure, and
re-raising a bound error that still holds its payload. Both are checked on
mutated IR so that widening the surface later cannot silently admit them.

Native emission carries the payload in the result value alongside the error tag
and clears every stale copy as it moves, submitting a typed `TaskResult` event
whose error tag and optional payload generation distinguish a payload-bearing
result from a payload-free error.

Several payloads, recursive owned graphs, arbitrary error ABIs, payloads on
nested workers, loans inside payloads and targets other than Linux, macOS and Windows remain
unsupported. See [the error-payload report](reports/remaining-t06/README.md).

## G02c. Guarded fixed-footprint mutex

`CheckedMutexGuardsV1`, profile id `source.mutex.guard.v1`, lowers a lexical
guard onto the `CheckedMutexInvariantV1` model checker
(`crates/qcore-core/src/mutex_invariant.rs`). It is selected by the presence of
a `mutex guards` declaration, ahead of the scoped-task routes. It composes with
none of them: a module using the scoped-task `spawn`/`join` expressions is
refused. Its own bounded child-task form is a nested `thread scope` block
(card T03-F).

```qcore
#import std.concurrency;
error Fault { Broken; }
fn main() -> unit {
    thread scope s {
        let buf = protected 4;          // one shared resource, 4 bytes
        let m = mutex guards buf;       // protected footprint fixed here
        with lock m as g {
            open g;                     // the invariant may now be broken
            write buf with g;
            close g;                    // restored, by a declared predicate
            let b = borrow buf with g;
            read buf with g;
            drop b;                     // the borrow dies inside its section
        }
        with lock m as g2 { read buf with g2; }
        let f = fact buf;               // a guard-scoped refinement fact
        with lock m as g3 {
            assert f with g3;
            open g3;
            read buf with g3;
            use f;
            close g3 unchanged;         // restored, and *checked*: no write
        }
        thread scope c {                // one child task, joined at `}`
            with lock m as gc { read buf with gc; }
        }
        thread scope d share buf under m {   // CT46's declared crossing
            with lock m as gd { read buf with gd; }
        }
    }                                   // scope exit destroys m, then buf
}
```

```ebnf
module      = { error_declaration } , entry ;
entry       = "fn" , NAME , "(" , ")" , "->" , "unit" , "{" , scope , "}" ;
scope       = "thread" , "scope" , NAME , "{" , { outer } , "}" ;
outer       = "let" , NAME , "=" , "protected" , ( NAT | SCALAR ) , ";"
            | "let" , NAME , "=" , "mutex" , "guards" , NAME , ";"
            | "let" , NAME , "=" , "mutex" , "rank" , NAT , "guards" , NAME , ";"
            | "let" , NAME , "=" , "fact" , NAME , ";"
            | ( "read" | "write" ) , NAME , ";"
            | "wait" , NAME , ";"
            | "use" , NAME , ";"
            | "thread" , "scope" , NAME , "{" , { child } , "}"
            | "thread" , "scope" , NAME , "share" , NAME , "under" , NAME ,
              "{" , { child } , "}"
            | "with" , "lock" , NAME , "as" , NAME , "{" , { inner } , "}" ;
inner       = "open" , NAME , ";"
            | "close" , NAME , [ "unchanged" ] , ";"
            | "grow" , NAME , "guards" , NAME , ";"
            | ( "read" | "write" ) , NAME , "with" , NAME , ";"
            | "let" , NAME , "=" , "borrow" , NAME , "with" , NAME , ";"
            | "drop" , NAME , ";"
            | "assert" , NAME , "with" , NAME , ";"
            | "use" , NAME , ";"
            | "wait" , NAME , ";"
            | "thread" , "scope" , NAME , "{" , { child } , "}"
            | "thread" , "scope" , NAME , "share" , NAME , "under" , NAME ,
              "{" , { child } , "}"
            | "raise" , NAME , "::" , NAME , ";" ;
child       = (* any `outer` or `inner` form except a declaration and except
                 `drop`; a child inherits the enclosing critical section
                 lexically and holds none of its authority *) ;
```

`mutex guards R;` declares the protected footprint syntactically, and the only
way to change it afterwards is card CT34-GROWTH's `grow` statement below, under
its own rule; a dynamic protected shape is otherwise not expressible rather than
being detected later. Creation consumes the declaring task's `FULL_ACCESS` share over
`R`, so a second `mutex guards R;` cannot obtain it (`Q617`) and an unguarded
access to `R` has no fallback authority (`Q611`). A guard obtained over one
generation authorizes only that generation's footprint (`Q610`).

Guard names bind only inside their own `with lock` body, so a guard-derived
name mentioned after the block is `Q013`, not a concurrency diagnostic. A write
requires the invariant to be declared broken first, and both the normal block
end and a `raise` require it restored and every guard-derived borrow released
(`Q613`, `Q614`, `Q615`). `close g;` supplies a *declared* witness, so a
certificate covering it carries `InvariantPredicateDeclared`;
`close g unchanged;` supplies the **checked** witness and carries none.
`DeclaredFootprintShape` is on every certificate from this route, and
`GuardFootprintAuthority` and `SynchronizedCustodyEdges` never are. Unless the
module declares a lock rank (card CT33-RANKS below), a nested `with lock` lowers
to an explicit unsupported-construct event and is refused at `Q622`; `defer`
inside a critical section, and every other statement outside the grammar above,
is refused at `Q600`. Scope exit destroys each generation and then each
resource, under the model's quiescence rules.

Card T03-F's three completions, each bounded by a stated rule:

* **`close g unchanged;`** is admitted only when every statement since this
  guard's `open` was a read or a fact use — the shape the lowering can decide.
  Any other shape is refused at `Q600` and never silently downgraded to the
  declared witness.
* **`let f = fact r;` / `assert f with g;` / `use f;`** are guard-scoped
  refinement facts. Each assertion mints a fresh model identity, so relying on
  a fact after its guard was released is `Q616`, relying on one that was never
  asserted is `Q600 UnknownFact`, and a child task relying on its parent's fact
  is `Q616`.
* **A nested `thread scope c { … }`** is one child task, created where the
  block opens and joined where it closes. The child receives no share and no
  custody, so a protected footprint is reachable to it only through its own
  acquisition (`Q611` otherwise); naming the enclosing guard is `Q612`; and a
  child block written *inside* a critical section is refused because its join
  lands while the creator holds the guard (`Q600 JoinWhileGuardHeld`). A child
  declares no resource, generation or fact, and may not release a borrow it did
  not take (`Q600`). **`wait m;`** announces an acquisition and, if never
  performed, blocks the generation's destruction (`Q619`).

Card T03-GUARD-JOIN adds one opt-in form to that last rule and changes nothing
else about it:

* **`thread scope c under g { … }`**, written inside `g`'s own section, lowers
  its join to the core's `GuardLocalJoin` rather than `Join`. `under` names the
  enclosing section explicitly; outside one, or naming a different guard, it is
  a lowering refusal (`Q600`) or an unresolvable name (`Q013`). Admission is the
  core's decision, not the grammar's: the child must have been created strictly
  inside this same section, the section must be the only one held, with its
  invariant closed and no live borrow, and the child *and every task under it*
  must have named no mutex, guard or guard-scoped fact anywhere in the trace and
  hold no share or custody. Anything else keeps the same
  `Q600 JoinWhileGuardHeld` refusal, so a plain `thread scope c { … }` inside a
  section is unchanged. Because a child in this profile holds no share and no
  custody, a guard-local child cannot touch a declared resource at all: the
  admission is about the *join*, not about giving the child authority. It adds
  no premise to the certificate — every condition is checked — and each admitted
  join is named on it. `under` is reserved. See
  [the T03-GUARD-JOIN record](reports/remaining-t03-guard-join/README.md).

Card CT33-RANKS adds the ranked declaration and nothing else to the grammar:

* **`let m = mutex rank N guards r;`** declares `m`'s position in a lock order.
  `N` is one decimal literal in `0 ..= 4294967295`; any other spelling is a
  lowering refusal (`Q600`). Declaring at least one rank turns on the core's
  opt-in discipline for the whole module, and two things follow. Every mutex the
  module declares must then be ranked, or the module is refused at `Q625` — a
  partly ranked module has no order, so it is not checked over its ranked part.
  And a nested `with lock` becomes expressible: you may nest exactly where you
  have declared in which order, and the core then refuses any acquisition made
  while holding an equal-or-higher rank (`Q624`), which is what makes a
  lock-order cycle writable and refused rather than inexpressible. Nesting buys
  the right to hold two guards, not to conflate them: the inner body's frame
  carries only the inner section, so naming the enclosing guard from inside is
  `Q600`, and a nested acquisition invalidates the decidable region behind
  `close g unchanged;`. A task re-locking a generation it holds keeps its
  existing `Q600 DuplicateLockCapability`, since the rank test runs last and
  never displaces an existing refusal. Declaring no rank leaves every module
  written before this card byte-identical, including the `Q622` nesting refusal.
  A passing rank check is not a deadlock-freedom proof — see
  `docs/QCORE_SPEC_r0_3.md` for exactly what it does and does not establish.
  `rank` is reserved. See
  [the CT33-RANKS record](reports/remaining-ct33-ranks/README.md).

Card CT34-GROWTH adds one statement and nothing else to the grammar:

* **`grow g guards r;`**, written inside `g`'s own section, adds `r` to the
  protected footprint of the generation `g` guards. It names the guard, and the
  core reads the generation off it, so a growth can never reach a mutex its
  holder does not hold. Admission is the core's decision, not the grammar's: the
  invariant must be open at the growth (`Q626`); `r` must be protected by no
  live generation, this one included (`Q617`); the growing task must be the
  generation's creator and hold the full share over `r` (`Q621`); and the growth
  must move the generation's declared invariant to an identity it has never
  carried (`Q627`), which the lowering mints. A growth is a change to the
  protected region, so `close g unchanged;` is refused in a section that grew
  (`Q600`) rather than downgraded, and the section must close by declaration.
  Growth is permanent: `r` is behind the guard from then on — an unguarded
  access to it becomes `Q611` — the originally declared footprint stays
  protected, and there is no shrink or replacement form. A refused growth leaves
  the generation's shape, its declared identity and the ledger untouched, so the
  section's own `close` then names an identity the generation never took; that
  fallout is reported rather than repaired. Growth adds no premise beyond the
  `InvariantPredicateDeclared` the new identity carries, and each admitted
  growth is named on the certificate. The unconditional `Q618` fixed-shape
  refusal is unchanged and has no source form. `grow` is reserved. See
  [the CT34-GROWTH record](reports/remaining-ct34-growth/README.md).

Native emission uses one thread and C11 `mtx_t` with real heap footprints on
Linux, macOS and Windows (the latter two through the emitted compatibility layer);
the reference execution of the checked event list must reproduce the same
observation. There is no condition variable, no try/timed lock and no
reader/writer lock in this profile. A module whose trace
carries task structure (`spawn`, `join`, a custody transfer or an announced
acquisition) is checked but has **no native rendering**: the backend emits one
thread, so the module is published with no C rather than with C the backend did
not produce. See [the T03 record](reports/remaining-t03/README.md) and
[the T03-F record](reports/remaining-t03-f/README.md).

## G02d. Bounded extern calls

`source.extern-call.v1`. Selected when a source file declares an `extern fn`
and a function body calls it, after the concurrent routes: a module that also
spawns or locks stays with those routes and keeps their existing refusals.

```ebnf
extern_module = "import", "std.extern", ";",
                { error_decl | extern_decl | entry } ;
extern_decl   = "extern", "fn", NAME, "(", [ param, { ",", param } ], ")",
                "->", ABI_TYPE, ";" ;
entry         = "fn", NAME, "(", ")", "->", "unit",
                [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
                "{", { extern_stmt }, "}" ;
extern_stmt   = acquisition | scalar_binding | scalar_literal | plain_call
              | value_statement                      (* §G02t, card W48-QR-SCALAR-TYPES-R3 *)
              | "return", ";" ;
acquisition   = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                "succeeds", BOOL, "else", { disposition },
                "raise", ERROR_NAME, ";" ;
disposition   = CALLEE, "(", [ arg, { ",", arg } ], ")" ;
scalar_binding = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                 [ "succeeds", [ "-" ], NAT ], ";" ;
scalar_literal = "let", NAME, "=", [ "-" ], NAT, ";" ;
plain_call    = CALLEE, "(", [ arg, { ",", arg } ], ")", ";" ;
arg           = [ "-" ], NAT | NAME | "move", NAME | "out" ;
EFFECT        = "io" | "alloc" | "mut"
              | "throws", "<", "ArithmeticError", ">" ;  (* card W48-QR-SCALAR-TYPES-R3 *)
```

Card `W48-QR-SCALAR-TYPES-R3` makes the value namespace **typed** (§G02t,
"Extern results as values"): a bound result has its declared result's type, a
`value_statement` computes one §G02t operation over bound values and literals,
a declared `succeeds INTEGER` is a value of the result's type at any width, and
an argument at a declared scalar parameter is a value — or a literal in range —
of exactly that parameter's type. `throws<ArithmeticError>` in the row is the
permission a statement over a run-time value needs to raise; it derives no
effect.

Two of those forms are card `STD-IO-COMPOSE`'s and both stay inside the value
namespace card `PIPE-SCALAR-RESULT` opened. `scalar_literal` binds a number
written at the site: it calls nothing, so it derives no effect, admits no
argument, opens no obligation and can carry no custody in any position — the
narrowest widening that lets a rendered value originate in the module that
renders it (`EXTERN-SCALAR-LITERAL`). The optional `succeeds` tail on
`scalar_binding` is the site's **declared outcome**, the same concept the
acquisition adapter already spells with that word: `succeeds BOOL` declares
which arm a nullable owned return takes and `succeeds INTEGER` declares which
number a scalar result carries. The two are told apart by the literal alone, so
no malformed adapter tail can be read as the other and `Q025` keeps its exact
shape; no reserved word was added. Neither control is believed by the emitted
program, which compares the real result and exits 70 on disagreement.

A bound value whose number the source did **not** declare is still a value: it
may be passed at a declared scalar parameter exactly as before. What it may not
do is be *rendered*, which is a `source.std-io-extern.v1` rule (§G02i) and not
one of this route's — this route renders nothing.

`ABI_TYPE` is exactly the resource-contract v1 projection of §G05. The callee's
contract is **not** written here: it comes from an explicit `CompileOptions`
selection loaded by the same total-or-reject loader, and an inline
`contract { … }` block on a called extern declaration is refused (`Q027`) rather
than enforced. The source declaration's parameter names, parameter types and
result type must equal the selected module's, or the call is refused.

Callable C04 shapes are exactly: scalars by value (data only), `resource_borrowed`
on `ptr<mut, u8>` or `ptr<const, u8>`, `resource_consumed(p, nonnull)` on
`ptr<mut, u8>`, `resource_return_none()` with a `unit` or scalar result,
`resource_return_owned(nullable)` returning `ptr<mut, u8>`, and — card
`EXTERN-F1` — `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` plus
`resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` beneath a distinct
`resource_owned(owner, nonnull)` parameter on `ptr<mut, u8>` — and, card
`EXTERN-MODES`, `resource_released(p, nonnull)` on `ptr<mut, u8>` plus
`resource_return_reallocated(p, nullable)` returning `ptr<mut, u8>` over a
subject declared `resource_owned(p, nonnull)` on `ptr<mut, u8>`. Contract spec
§C04.2 requires a stored parameter's owner to be a distinct `Owned` parameter
and §C04.3 requires a reallocation subject to be `Owned`, so `resource_owned` is
callable in exactly those two designated positions and nowhere else.
`resource_removed`, a nonnull owned return, a nonnull reallocated return, a
reallocation subject that is out of range, not `Owned`, nullable or on any other
pointer, a reallocated result on any other type, a reallocation beside an out
slot, a nullable released parameter, a release on any other pointer, a nonnull
out slot, an out slot on any other ABI type, two
out slots, an out slot beside an owned return, a store beside either, a nullable
stored parameter, a store on a `const` pointer, a store beneath itself or
beneath a non-`Owned` parameter, a nullable or `const`-pointer declared owner, a
`resource_owned` parameter that owns nothing, a nullable consumed parameter, a
custody mode on a scalar, a borrowed `ptr<mut, ptr<mut, u8>>` and a consumed
`ptr<const, u8>` are each refused by their own name (`Q024`).

`resource_removed` is refused on a measured ground rather than by omission: a
removal names a child this profile stopped tracking when it was stored, because
`EXTERN-F1` records the residual beneath a declared owner as a premise and not
as a modeled owner-to-child index, so admitting it would mint a caller identity
out of a premise. `resource_released(p, nullable)` is refused for the reason the
nullable consumed mode is: a nullable release discharges nothing when the
modeled value is null (§C04.2), and this profile does not model that value.

`out` marks the position of the declared out slot; it names a parameter
position, never a binding, and is reserved. The `let` binder names the identity
the callee writes into that slot. `move NAME` at a declared store removes the
binding from the body exactly as a consumption does, so every later use is
refused; where the custody went is declared by the contract, not by the call
site, and is recorded on the certificate as a premise. Card `EXTERN-MODES` adds
no argument syntax and no reserved word: `move NAME` also spells a declared
release and a declared reallocation subject, and which of the four departing
modes ran comes from the declaration alone. The certificate keeps them apart —
a release is published under `releases`, a replacement as one `(replaced,
established)` pair — because contract spec §C04.2 warns that a false release
declaration can invalidate a safety conclusion while a false consumption only
relocates an obligation.

A reallocation is the one departing argument an acquiring call may carry.
Contract spec §C04.3 states both of its arms — a null failure preserves the
incoming custody, a success replaces it exactly once — so the subject stays on
the error arm's live list and must be dispositioned there like any other live
identity. Every other departing mode beside an acquisition stays refused, for
the reason that refusal exists: an ordinary declaration constrains only its
normal exit.

`succeeds BOOL` is the fixture's literal outcome control, exactly as in
`qcore_acquire`/`qcore_check`: a controlled model operation, not a claim about
when a production allocator fails. Both arms are exercisable, and the emitted C
compares the real result's nullness against the declared control and exits with
status 70 if they disagree — for an out slot the emitted local starts `NULL`, so
a callee that never wrote it is observed as the failure arm rather than
believed. The `else … raise ERROR;` arm is mandatory for each acquisition
source, an owned nullable return and an out slot alike (`Q025`).

Card `EXTERN-ERR-REL` gives that arm its dispositions. Between `else` and
`raise` the handler names one call per still-live identity, and the arm is
admitted only when it *is* this acquisition's established cleanup edge: the
sequential lowering `qcore_core::cleanup_lowering` releases every identity live
before a failing acquisition in reverse acquisition order, and the arm must
release the same identities, each exactly once, in that order. This is a
rendering, not a second rule — an extern-acquired identity is released only by
calling a declared `resource_consumed` callee, and the profile cannot invent
one. A disposition that acquires, that stores, that discharges other than
exactly one identity, that names an identity not live on the arm, that runs in
another order, or whose derived effect row the entry's declared row does not
admit is refused (`Q025`), as is an identity that reaches the raise still live
— the v1 abandonment refusal, narrowed to the undispositioned case and not
weakened. An acquiring call that also moves custody out is refused there too,
because a declaration constrains only its normal exit. The arm is a path: a
disposition never removes a binding from the success path. That is what makes a
store reachable — it needs two simultaneously live identities, the child and its
declared owner, which the second acquisition's arm now accounts for.

Card `PIPE-SCALAR-RESULT` adds `scalar_binding` and one meaning to a bare
`NAME` argument, and adds no reserved word. `let NAME = CALLEE(ARGS);` binds a
**non-acquiring call's scalar result** to a local, and a bound scalar may then
be passed at a declared scalar (data) parameter. The admission is exactly:
the callee establishes fresh custody through none of the three acquisition
sources, and its declared result is a custody-free scalar of the §C04.1 ABI
vocabulary. `unit` is refused — there is no value — and every pointer result is
refused, including a pointer declared `resource_return_none()`, because such a
result is an address the declaration says nothing about (`Q024`, naming the
declared result). Binding an acquiring call's result refuses at `Q025`: an
acquisition has exactly one spelling, the mandatory adapter, and this card adds
no second one.

Scalars and identities are **separate namespaces**. A bound scalar is admitted
at a declared data parameter and at no other mode; a resource binding is refused
at a data parameter (`Q024`, "a declared scalar parameter carries data, never
custody"); a scalar binding removes nothing from and adds nothing to the ledger,
so an unread one is not a leak; and a `let` whose name already means the other
kind of thing is refused rather than resolved by lookup order — measured, the
reachable refusal is name resolution's own `Q012` (duplicate local binding), and
the lowering keeps its `Q026` arm as an unreachable fail-closed backstop. The
certificate publishes `(callee, value)` per binding in trace order, beside the
custody fields and never inside them. A **scalar literal** binding
(`let n = 42;`) is deliberately *not* added here and stays refused; it is
follow-on `EXTERN-SCALAR-LITERAL`, and `STD-IO-COMPOSE` is the card that needs
it. The emitted C declares the result at its ABI spelling, discards it to
`void` so the translation unit stays warning-clean, and prints an observation
that names the callee, the value and the arguments —
`value CALLEE vN ARG…`.

Card `EXTERN-SCALAR-OUTCOME` (trace half) appends the number to that line, and
only where a `succeeds INTEGER` outcome is declared: `value CALLEE vN ARG… -> N`.
A reference interpreter still cannot compute a C function's result; what it
prints is the *declared* number, and the emitted program compares the real
result against that same number and exits 70 **before** reaching the
observation write, so a run only ever publishes the line having established
that the declared number is the returned one. A binding with no declared
outcome keeps the card-`PIPE-SCALAR-RESULT` line byte for byte, because nothing
in either model computes a number for it — that remainder is deliberate, not an
omission. The ordering of the comparison and the write is load-bearing rather
than incidental.

The derived effect row is `{io}`, plus `alloc` exactly when the call establishes
fresh custody through any of the three acquisition sources and `mut` exactly
when some parameter is a `mut` pointer; the entry's declared row must admit it, and an
absent row therefore admits no extern call at all. Length
and extent are not transported, so no declaration here constrains how many bytes
a callee touches. Custody liveness, leaks and resource flow are checked by the
established sequential passes over the same lowering, not by a second extern
code. Emission declares each callee from the ABI table and calls it directly;
which object file supplies the symbol is the builder's premise. See
[the extern-call record](reports/extern-call-v1/README.md) and
[the out/stored extension](reports/extern-call-f1/README.md),
[the error-arm release record](reports/remaining-extern-err-rel/README.md) and
[the released/reallocated extension](reports/remaining-extern-modes/README.md).

### G02d.1. Unchecked linkage

Card `UNSAFE-LINKAGE`. An external library with **no contract at all** may be
linked, under an explicit recorded waiver. This widens G02d's *declaration* form
and nothing else: the statement grammar, the argument forms and every refusal
above are unchanged, and without the waiver an uncontracted callee still refuses
at `Q023`.

```ebnf
unsafe_module   = "import", "std.extern", ";",
                  "import", "std.extern.unsafe", ";",
                  { error_decl | extern_decl | unsafe_extern_decl | entry } ;
unsafe_extern_decl = "unsafe", "extern", "fn", NAME,
                     "(", [ param, { ",", param } ], ")",
                     "->", SCALAR_ABI_TYPE, ";" ;
SCALAR_ABI_TYPE = "bool" | "i8" | "i16" | "i32" | "i64" | "isize"
                | "u8" | "u16" | "u32" | "u64" | "usize" | "unit" ;
```

Both imports are required and neither implies the other: `std.extern` is the
call vocabulary and grants no waiver, `std.extern.unsafe` is the waiver and
grants no call. The waiver import is required by the *declaration* (`Q032`),
not by a call to it — declaring an uncontracted symbol is itself the trust act,
so a file that links one and never calls it still names the module. `unsafe`
was already a reserved source word (it introduces the `unsafe { … }` statement),
so this form adds none.

Parameters take `SCALAR_ABI_TYPE` minus `unit`, and the result takes it whole;
**every pointer spelling is refused in every position** (`Q033`), including
`ptr<const, u8>`, and so is any spelling outside the §G05 ABI table. The
contract is waived; the ABI is not. The admitted signature therefore carries no
custody in either direction, which is why an uncontracted call cannot change the
caller's ledger whatever the callee does. A waiver may not sit beside a
checkable declaration (`Q034`): not an inline `contract { … }` block, and not a
name the selected contract environment also declares — there the contracted
route wins whether or not the two signatures agree.

The derived effect row of an uncontracted callee is `{io, alloc, mut}`
unconditionally, because nothing is known about it, so an entry calling one
declares the whole row. A module may mix contracted and uncontracted callees; a
module all of whose callees are uncontracted needs no contract selection at all,
which is what `quirin check` and `quirin emit` can compile today.

Every certificate names each uncontracted callee, with its declared ABI, as an
unchecked-linkage premise, on the same channel that carries a trusted declared
contract; a published certificate missing one is refused (`Q035`). See
`docs/design/UNSAFE_LINKAGE_V1.md` and
[the unchecked-linkage record](reports/wave18-unsafe-linkage/README.md).

### G02d.2. The native-library manifest

Cards `UNSAFE-LINKAGE-ARTIFACT`, `NATIVE-LIBRARY-CONTRACT-SLICE` and
`NATIVE-LIBRARY-HEADER`. A **manifest** is not source: it is a separate
declarative file a builder selects (`quirin --native-library MANIFEST`,
`CompileOptions::native_libraries`), and it adds no form to G02d's or G02d.1's
source grammar and no reserved word. What it changes is the *premise* a call
publishes: a G02d.1 waiver moves from "symbol X, unchecked" to "symbol X,
unchecked, from artifact Y@hash under ABI premises Z", and — card
`NATIVE-LIBRARY-CONTRACT-SLICE` — a G02d **contracted** call gains a second
premise beside its declared-contract one, "contracted X per contract C, from
artifact Y@hash under ABI premises Z". The second premise **stacks with** the
declared-contract premise and never replaces it: one says what the code does,
the other which code it is.

```ebnf
manifest     = marker, { field | entry } ;
marker       = "native_library", "manifest", "v1", ";" ;
field        = ( "library" | "version" | "artifact" ), TOKEN, ";"
             | "hash", "sha256:", 64 * LOWER_HEX, ";"
             | "data_model", "lp64", ";"
             | "unwinding", "abort", ";"
             | "allocator_custody", "closed", ";"
             | "thread_posture", "single_caller", ";"
             | "header", TOKEN, ";"                        (* card `NATIVE-LIBRARY-HEADER` *)
             | "header_hash", "sha256:", 64 * LOWER_HEX, ";" ;
entry        = waived_entry | contract_entry ;
waived_entry = "unchecked", "fn", NAME,
               "(", [ C_ABI_TYPE, { ",", C_ABI_TYPE } ], ")",
               "->", C_ABI_TYPE, ";" ;
contract_entry
             = "contract", "fn", NAME,                     (* card `NATIVE-LIBRARY-CONTRACT-SLICE` *)
               "(", [ C_ABI_TYPE, { ",", C_ABI_TYPE } ], ")",
               "->", C_ABI_TYPE, "from", MODULE_REF, ";" ;
C_ABI_TYPE   = "_Bool" | "int8_t" | "int16_t" | "int32_t" | "int64_t"
             | "intptr_t" | "uint8_t" | "uint16_t" | "uint32_t"
             | "uint64_t" | "uintptr_t" | "void"
             | C_POINTER ;                                 (* a waived entry refuses C_POINTER *)
C_POINTER    = "uint8_t *" | "const uint8_t *" | "uint8_t **" ;
MODULE_REF   = TOKEN ;   (* the contract selection's own key, compared exactly *)
```

`#` begins a comment; statements are `;`-terminated and may span lines. The
eight identity and ABI-premise fields are mandatory and each may be stated once.
The two **header-binding** fields (card `NATIVE-LIBRARY-HEADER`) are optional
**as a pair**: both, or neither. One without the other is a `Q039` missing-field
rejection, on the same ground as every mandatory field — an identity missing a
field is not a weaker identity but an unusable one — and naming neither is the
default, which leaves every manifest written before that card byte-valid and
byte-unchanged. Entry types are
the **C** spellings of the exported header declaration, not the QCore ones, so
the agreement check compares a projection rather than a string. The pointer
spellings `uint8_t *`, `const uint8_t *` and `uint8_t **` are recognised
(the space before `*` optional) and then **refused in a contract-less entry**,
which is a rejection with its own name rather than an unreadable spelling: a
library's pointer symbols belong in a **contract slice entry**, and card
`NATIVE-LIBRARY-CONTRACT-SLICE` is where they now go. The two entry kinds are
told apart by their keyword and never by their shape: a `contract fn` without
its `from MODULE` tail, and an `unchecked fn` carrying one, are each a named
rejection rather than a reading of the other.

A `contract fn` entry binds its symbol to a declared contract **module
reference** — the contract selection's own key, compared exactly and never
normalised. The contract machinery is unchanged: the manifest selects and
attributes, it states no mode and no effect of its own, and the contract is
loaded, admitted and bridged by the unedited resource-contract v1 route.
The binding is total in **both** directions (`Q047`): every entry must resolve to
a declared contract in the module it names, and every declaration of a module the
slice names must be bound by an entry. The slice is derived from the entries
themselves rather than declared by a second statement, because a second spelling
would be a second place the slice could be wrong.

Loading is total or reject (`Q039`), the selection's binding must be a function
(`Q040`), a **waived** declaration the manifest contradicts is refused (`Q041`),
a name the manifest declares contract-less may not also be reached through a
declared contract (`Q042`), a contract slice binding must resolve in both
directions (`Q047`), a **contracted** declaration the manifest contradicts is
**withdrawn for the unit** and its call sites fall to `Q023` (`Q048`), and a
published certificate must carry both stacked premises of every manifest-bound
contracted callee (`Q049`). Selecting no manifest is the default and leaves every
G02d and G02d.1 program byte-identical.

A `header` field binds the library's **exported header** and `header_hash` its
digest (card `NATIVE-LIBRARY-HEADER`, spec §L14.4). Where they are present, every
entry's transcribed declaration above is read out of that header's own text and
compared against it, for both entry kinds alike, rather than believed: the
header's text is supplied to the checker (`CompileOptions::native_library_headers`;
`quirin` resolves the reference against the manifest's own directory), its digest
is measured over exactly those bytes and **verified** against `header_hash`, and
a declaration outside the bounded reader subset refuses the manifest by name
(`Q063`). A transcription the header contradicts refuses (`Q064`) — the `Q041`
disposition, not `Q048`'s, because the weaker admissible reading here is this
same manifest without a header and that is accepted. Declarations the manifest
does not name are ignored: a header is the library's whole exported surface.

An `include SPELLING from REFERENCE HASH` statement declares one text that
header's own `#include` lines may be read from (card
`NATIVE-LIBRARY-HEADER-DECORATION`, spec §L14.4.1). The statement is repeatable,
requires a `header` to read the texts into, and names three things: the
`#include` operand exactly as the header writes it (`<event2/visibility.h>` or
`"local.h"`, delimiters included), the reference the text is resolved and
supplied under — the same namespace as `header`, and `quirin` resolves it against
the manifest's own directory — and its `sha256` digest. Declaring none is the
default and is the wave-22 behaviour byte for byte.

What the statement does **not** say is the point of the card. It says nothing
about what the text contains, and in particular it does not declare that any name
may be erased. The reader **derives** from the declared texts' own `#define`s
that an export-decoration macro is erasable — every definition object-like, every
body in the reader's own published vocabulary of type-neutral decorations, never
`#undef`ed, and defined on every path through the conditional structure, which no
condition is evaluated to establish. A name that is not derivable is not erased
and its declaration keeps the `Q063` refusal it already had. An `#include` the
manifest does not declare is not followed; a declared text that is not supplied,
whose digest does not match, or that nothing reaches refuses at `Q063`, as does a
supplied text no manifest declares once a header is bound.

See `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`,
`docs/design/NATIVE_LIBRARY_HEADER_V1.md`,
[the manifest record](reports/wave19-unsafe-linkage-artifact/README.md),
[the contract slice record](reports/wave20-native-library-contract-slice/README.md)
[the header record](reports/wave22-native-library-header/README.md)
and [the decoration record](reports/native-library-header-decoration/README.md).

#### G02d.2.1. The generated manifest (v2)

Card `W48-QRLIB-LINKAGE` (user decisions, 2026-09-30). `.qrlib` is **the** C
linkage manifest of the FFI route (§G02u), and a v2 manifest is **generated on
every build, before linking**, never written by hand: `quirin qrlib generate`
(or the library API `qcore_c_gateway::linkage::generate`) derives it from the
artifact's bytes and the build's declared inputs, `quirin qrlib check` and
`quirin build` regenerate it and refuse a stored manifest that is not byte for
byte the regenerated one as **stale**, and `quirin build` (the `LinkPlan` API)
hashes every pinned file again immediately before handing the linker exactly
the pinned artifact. The v1 grammar above loads unchanged, and `float`/`double`
join `C_ABI_TYPE` in both versions (`W48-QR-FLOAT-QRLIB-SPELLING`).

```ebnf
manifest_v2   = "native_library", "manifest", "v2", ";",
                { field_v2 | entry_v2 } ;
field_v2      = ( "library" | "version" | "artifact" | "target" ), TOKEN, ";"
              | "hash", DIGEST, ";"
              | "linkage", ( "static" | "dynamic" ), ";"
              | "format", ( "elf" | "macho" | "coff" ), ";"
              | "input", TOKEN, DIGEST, [ "as_needed" ], ";"        (* repeatable *)
              | "runtime", RUNTIME, ";"                              (* repeatable *)
              | "data_model", ( "lp64" | "llp64" ), ";"
              | "unwinding", "abort", ";" | "allocator_custody", "closed", ";"
              | "thread_posture", "single_caller", ";"
              | "header", TOKEN, ";" | "header_hash", DIGEST, ";" ;
RUNTIME       = "soname", TOKEN, "in", TOKEN
              | "install_name", TOKEN, [ "current_version", TOKEN ]
              | "dll", TOKEN, "os_build", TOKEN ;
entry_v2      = "contract", "fn", NAME, "(", [ C_TYPE, { ",", C_TYPE } ], ")",
                "->", C_TYPE, "from", MODULE_REF, "export", EXPORT, ";" ;
EXPORT        = TOKEN, [ "version", TOKEN ], ( "global" | "weak" ), [ "ifunc" ],
                [ "dll", TOKEN ], "in", TOKEN ;
C_TYPE        = C_ABI_TYPE | "float" | "double"
              | "struct", TAG | "struct", TAG, "*" | "const", "struct", TAG, "*" ;
DIGEST        = "sha256:", 64 * LOWER_HEX ;
```

* **What is derived.** `hash` is the sha256 of the file the link names
  (`artifact`); an `ld` script is followed and every file it names is an
  `input` with its own digest, in script order, `AS_NEEDED` kept; `format`,
  `target` and `data_model` come from the recorded target (`llp64` for 64-bit
  COFF); each entry's `export` clause is what the artifact's own symbol table
  says of the symbol — its object-level name (`_sqrt` on Mach-O), its default
  ELF version, `global` or `weak`, `ifunc` for an `STT_GNU_IFUNC`, the DLL a
  COFF import names, and the file (and archive member) that defines it; a
  dynamic link's `runtime` premises are the shared objects' `DT_SONAME`s, a
  stub's install name and current version, or each imported DLL with the
  declared OS build. `header_hash` is measured over the declared header.
* **What is declared.** `library`, `version`, `linkage`, the three ABI
  premises (never defaulted), the header, the contract modules — **every**
  declaration of each becomes one entry, so the slice is total by
  construction — and the Windows OS build.
* **Linkage means the exported symbol table** (the view `nm` gives, read by
  the card's own ELF, Mach-O and COFF/PE readers, never by `nm`). A bound
  symbol must be exported and defined in the pinned artifact: missing
  (stripped), undefined, local-only, doubly defined, defined only under a
  hidden version, not listed by an archive's index, imported by ordinal only,
  or — on Windows — not exported by name by the runtime DLL or exported as a
  forwarder each refuses generation by name. A defined weak symbol is admitted
  and recorded `weak`.
* **Static and dynamic.** `static` pins the code (an ELF, Mach-O or COFF
  archive, or a script naming archives) and states no `runtime`; `dynamic`
  pins the link-time stub (an ELF shared object or a script naming them, a
  `.tbd` text stub — read for the target's slice, `arm64e-macos` standing in
  for an unlisted `arm64-macos` — or a dylib export trie, a COFF import
  library) and states
  at least one `runtime` premise, because the OS owns the library the program
  loads. A script naming both kinds, the linkage kind the artifact is not, a
  static link resolving to an import and a dynamic COFF link resolving to code
  are refused.
* **Refusals.** Each is a `Q039` load refusal by its own name: a v2 statement
  in a v1 manifest or `include` in a v2 one, an `unchecked fn` in a v2
  manifest, a v2 entry without its `export` clause, a malformed `export`,
  `input` or `runtime`, a static manifest with a `runtime`, a dynamic one
  without, and a missing `linkage`/`format`/`target`. Struct spellings are
  refused in a v1 manifest.

§L14.8 states the linkage limitations. See
[the card record](reports/w48-qrlib-linkage/README.md).

### G02d.3. The source library binding

Card `NATIVE-LIBRARY-SOURCE-BINDING`. §G02d.2's manifest is selected by the
*builder*; until this card, resolution was by **symbol name across the whole
selection**, so whichever selected manifest happened to name a symbol supplied
that symbol's premise and no source file said so. This adds the one form a
source file needs to say it:

```ebnf
library_binding = "extern", "library", NAME, ";" ;
```

It is **import-adjacent**: it stands at file scope beside the `import` lines, it
declares no name, and it grants no capability — it states which manifest-bound
library *this file's* declared extern surface is resolved against. It mints **no
reserved word** (constraint 14): `extern` has introduced a declaration since
§G02d and `library` has been a source word since `module library;`; what is new
is the two-word sequence `extern library` at declaration position, which was a
parse error before this card. `library` stays an ordinary identifier everywhere
else.

`NAME` is a selected manifest's own `library` field, compared **exactly** and
never normalised — two spellings that normalise together are two different
selections.

The binding is **opt-in**. A file that declares none keeps the selection-wide
resolution byte for byte, which is every program written against §G02d.2 before
this card. A file that declares one or more resolves only against the manifests
it named, and:

* a name no selected manifest declares, a name two of them declare, a repeated
  binding, or a binding with nothing selected each refuse the module (`Q065`) —
  nothing resolves implicitly and nothing falls back;
* a symbol this file declares that a selected manifest it did **not** bind
  binds refuses (`Q066`). It is not silently handed the weaker premise: that
  would let a file stop its certificate naming a build by binding a different
  library, which is `Q041`'s disposition read from the other side;
* a symbol **no** selected manifest binds is unaffected and keeps exactly the
  premise it had — absence of evidence is not evidence, and the two absences are
  different;
* a published certificate must carry a source-binding premise for every
  `extern library` the module declares (`Q067`).

Resolution is **per input file**: the *declaring* file's binding decides a
declaration, on card `UNSAFE-LINKAGE`'s own ground that declaring a symbol is
itself the trust act. A binding in another compilation input grants a file
nothing and takes nothing away, which is constraint 13's discipline applied to a
binding.

A binding is resolved **wherever it is written**, before route selection, so a
module carrying one and no extern surface is refused if it does not resolve
rather than carrying a declaration nothing looked at. The check is guarded on the
file having written a binding: a module that writes none never causes the
selection to be loaded, so a manifest that does not load cannot refuse a module
that never mentioned one.

The premise a bound module publishes names the spelling the file wrote, the
manifest that declared it, and that manifest's artifact identity — `library@hash`
— beside the four ABI premise fields. It **stacks** with every callee premise and
replaces none: the callee premises say what each symbol was read as, and this one
says which build this file declared its extern surface to come from. A module that
binds a library and calls nothing it exports still carries it, because the binding
is a trust act whether or not a call site consumed it.

See [the source binding record](reports/native-library-source-binding/README.md).

Card `W48-QRLIB-LINKAGE`: on the FFI route (§G02u) the binding is **required**
for a file that calls C, and it must resolve to a generated (v2) manifest
(§G02d.2.1); a file with no binding (`Q065`, widened), a binding resolving to a
v1 manifest (`Q065`) and a callee no manifest the file bound binds (`Q066`,
widened) refuse. The other routes keep the opt-in rule above byte for byte.

## G02e. Flat-epoch quantitative bound

`source.epoch.bound.v1`. Selected before every other concurrent route by the
`epoch` marker on a thread scope, because an epoch scope is also a
`thread scope` and a module declaring a quantitative epoch must not be handed to
a route that models no accounting. Lowers onto the unedited `FlatEpochBoundV1`
checker (`qcore_core::epoch_accounting`, diagnostics `Q630`–`Q641`).

```ebnf
epoch_module = "import", "std.concurrency", ";", entry ;
entry        = "fn", NAME, "(", ")", "->", "unit", "{", epoch_scope, "}" ;
epoch_scope  = "thread", "scope", NAME, "epoch", "{", { header | worker }, "}" ;

header       = cell | allocation | overhead | latency_claim ;
cell         = "let", NAME, "=", "share", NAT, ";" ;
allocation   = "bound", NAT, "effort", NAT, ";" ;
overhead     = "overhead", "scope", NAT, "thread", NAT, [ "measured", NAT ], ";" ;
latency_claim= "bound", "effort", "as", "latency", ";" ;

worker       = "thread", "scope", NAME, "{", { clause }, "}" ;
clause       = "read", NAME, ";" | "write", NAME, ";"
             | "peak", NAT, ";" | "effort", NAT, ";"
             | "credit", NAT, "effort", NAT, ";"
             | "retained", NAT, ";"
             | "join", "(", ( "move" | "drop" | "close" ), ")", ";"
             | error_arm
             | worker ;
error_arm    = "error", "(", ( "move" | "drop" | "close" ), ")", ";"
             | "error", "(", "move", NAT, "effort", NAT, ")", ";" ;
```

Two axes are declared: `memory` carries every byte figure and `effort` carries
the profile's work quantity. `allocation` and `overhead` each appear exactly
once; `credit` appears exactly once per worker; `peak`, `effort`, `retained` and
the join clause at most once each. Every `NAT` is read at `u64` and an
out-of-range literal is refused, never wrapped.

The join clause is the creator's act, written in the child's block:
`join(move)` transfers the completed child's result to the creator,
`join(drop)` releases it there, and `join(close)` is the representable claim
that the result stopped being live at completion — refused by the core at
`Q634`. A worker with no join clause lowers to an unjoined task (`Q635`). The
parentheses are load-bearing: a bare `join` is claimed by the scoped-task
profile's expression scanner, so this profile's join is always written
`join(...)`.

Shapes the profile does not model become *program content* the core refuses
rather than grammar the lowering hides: `write NAME;` is
`UnsupportedEpochShape::SharedMutation` and a nested worker block is
`NestedTask` (both `Q632`); a shared entry naming something other than a
declared cell keeps an identity the storage list does not contain (`Q631`); a
repeated `read` of one cell in one worker is not deduplicated (`Q630`); and
`overhead … measured RUNS;` offers a native measurement in place of the declared
figures, which is `Q640`. Everything outside this grammar — a clause word the
profile does not have, a missing header, a second declaration, a `spawn`/`join`
task expression, `join(close)` with nothing retained — is refused at `Q600`.

`bound effort as latency;` is a request, not a grant: the lowering records it
and the driver puts it to `epoch_wall_clock_bound`, which always refuses
(`Q639`). This profile has no schedule model, so no quantity it derives is ever
a time.

`error_arm` declares what the **creator's own error edges** do with the credit
granted to that worker (card `CT23-CREDIT-ERROR-PATH`,
`qcore_core::epoch_credit_recovery`). It reserves no new word: `error`, `move`,
`drop`, `close` and `effort` are all already reserved. On **this** route it is
always refused, at `Q630`, because a declared worker is not a creation the
creator could fail to make — the refusal comes from the core's own band, not
from this grammar pretending the word is unknown. Its meaning is given in §G02f,
where the creator's error edges exist and are derived.

Every quantity here is a **declared premise** of `FlatEpochBoundV1`
(`DeclaredTaskFootprints`, `DeclaredRuntimeOverhead`,
`DisjointSharedStorageCells`, `NoScheduleModel`); nothing on this route imports
a peak, a result, a credit or an overhead figure from C, from an allocator or
from a measured run. The route is **check-only**: an admitted epoch is a checked
accounting statement about a declared shape, so it emits no C, runs no reference
execution and implies no native run. See
[the epoch source record](reports/remaining-t08-epoch-src/README.md).

## G02f. Flat-epoch bound over a checked task region

`source.epoch.tasks.v1`. Selected before G02e by an `epoch` marker **and** the
scoped-task profile's own `concurrency checked;` declaration: a module carrying
both is accounting for a real task region, not for declared worker shapes. The
entry's first statement is an epoch *plan*; the rest of the entry is an ordinary
scoped-task region. No diagnostic code is added by this route.

```ebnf
composed_entry = "fn", NAME, "(", ")", "->", result, "{", plan, { statement }, "}" ;
plan           = "thread", "scope", NAME, "epoch", "{", { header | row }, "}" ;
header         = allocation | overhead | latency_claim ;   (* G02e's, minus `cell` *)
row            = "thread", "scope", WORKER, "{", { clause }, "}" ;
clause         = "move", ";" | "copy", ";" | "read", ";"
               | "peak", NAT, ";" | "effort", NAT, ";"
               | "credit", NAT, "effort", NAT, ";"
               | "retained", NAT, ";"
               | [ "join", "(", ( "move" | "drop" | "close" ), ")", ";" ]
               | [ error_arm ] ;                            (* G02e's *)
```

`WORKER` names a source function the region spawns. Rows naming `WORKER`, in
declaration order, correspond one-to-one with that worker's `TrySpawn` sites in
the checked task IR, in source order; a declared row with no matching creation
and a creation with no declared row are both refused, per worker. A row's
ordered `move;`/`copy;`/`read;` clauses must equal that creation's ordered
argument operands, and `retained BYTES;` is required exactly when the spawned
worker's checked result type is an owned Buffer.

The `read;` clause (card `REGION-JOINS-LOAN`, wave 30, discharging
`T08-COMPOSE-LOAN` deliberately) declares a **counted read share** — the task
route's own `read PLACE share N` at the spawn argument. Like `move;` and
`copy;` it is place-free, and it also spells no share: the lent place and the
share are the checked task region's, not the plan's, to name. The wave-15
refusal reason ("a read loan's residency is shared with the lender, which this
flat accounting does not represent") dissolved rather than being overridden:
the lender on this route is always the **creator** — a spawned worker is a
leaf function and lends nothing — and creator-scope bytes are outside the
epoch's allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan contributes no
epoch byte and no shared cell. Share arithmetic, the write exclusion while a
share is out, the refusal of a `drop` of the lent owner (`LoanOutstanding`)
and the share's return at the join stay the **unedited** task ledger's own
rules on the task half. A `read NAME;` clause — the shared-cell read — still
has no referent here (`T08-COMPOSE-SHARED`).

The `join` clause is **optional here**: the creator's disposal is derived from
the checked IR (the handle's joins, the outcome's eliminations, and whether the
bound value is released), and a written clause must equal what was derived. The
task count, each task's identity and `joined` are derived and cannot be
declared at all.

No byte or work figure is derivable: this backend's `Buffer` carries no extent
(`TryAllocateBuffer` takes an initial scalar value, not a size) and the profile
has no cost model, so `peak`, `effort`, `credit`, `bound` and `overhead` stay
declared premises exactly as in G02e.

The `error_arm` clause is **optional**, at most once per row, and declares what
the creator's own error edges do with that row's grant
(card `CT23-CREDIT-ERROR-PATH`, checked by `EpochCreditRecoveryV1` beside the
bound): `error(move);` returns the whole grant to the epoch,
`error(move BYTES effort UNITS);` returns that much and spends the rest,
`error(drop);` spends all of it, and `error(close);` is the representable claim
that it is neither returned nor spent — refused at `Q635`. An elided clause is
`error(drop);`, so every module written before the clause existed keeps a
byte-identical certificate and byte-identical emitted C. The *edges* are
**derived**, never declared: every `TrySpawn` carries a `spawn_error` successor,
and a join edge exists exactly where a `TryOutcome` eliminates that creation's
join outcome, which is to say where the child can actually raise. Restoring more
than the grant is refused at `Q636`. Restoration does **not** reduce
`granted_credits` and therefore relaxes no `Q637`: an epoch whose recovery path
re-grants restored credit is `CT23-RESTORE-REUSE`. See
[the error-path record](reports/ct23-credit-error-path/README.md).

**The region epilogue** (card `REGION-CREDIT-JOINS`, wave 28) is the composed
entry's one optional `error region` statement — CT22's scenario, *"quant
opening discards parent-local unused credit after joins"*, made expressible:

```ebnf
region_epilogue = "error", "region", NAME, "{",
                    { "credit", NAT, ";" | "effort", NAT, ";"
                    | "let", NAME, "=", "qcore_buffer_new", "(", NAT, ")", "?", ";"
                    | "drop", NAME, ";"
                    | "move", NAME, ";" },
                    "qcore_check", "(", "false", ",", ERROR, ")", ";",
                  "}",
                  "catch", "(", ERROR, ")",
                  [ "quant", "open", NAME, "as", BINDING ],
                  "{", { "discharge", BINDING, ";" }, "}" ;
```

It is admitted as a direct epilogue of the entry's task scope, **after every
spawn and join**, or as a direct statement of the exact `Result::Ok` arm of a
direct task-scope `match spawn` (`CT24-CLEANUP-TASK-COMPOSITION`). The epilogue's
ordering is derived from checked task-IR spans and refused at `Q600` when false.
The success-arm form may precede its join: the existing task ledger refuses a
cleanup release while a child holds that owner's share (`LoanOutstanding`),
while independent-owner cleanup is admitted. Other nested placements remain
refused. Published cleanup quantities and releases in this arm are conditional
on reaching it; spawn-failure recovery is checked separately by the task route. Since card
`REGION-JOINS-DUTIES` (wave 29, deliberately superseding the wave-28 duty-free
premise) the body admits the task route's **own** acquisition and release
beside the parent's quantities, so the sealed pack carries real cleanup duties
(`Destroy`, one per unreleased owner) beside the remainder, and the opening's
`discharge` destroys the owners in reverse acquisition order — CT25's duty
half. The region **erases into its straight-line twin**: the split keeps each
acquisition and body release at its own byte offsets in the task half and
writes one `drop NAME;` per remaining duty into the blanks after them, so the
**unedited** scoped-task route compiles, custody-checks, emits and
reference-executes the destruction the model decided; a duty-free region
still blanks completely and keeps the emitted C, source map and reference
observation byte-identical with and without it (constraint 15l, the wave-28
measurement unchanged). Both published figures are re-derived before
publication: `abandoned_credit` against the reader's quantity fold and
`released` against its duty fold, over the model's own cleanup trace. Shape
refusals are `Q302` and an undeclared error is `Q300`, per §L21 — including a
release of a name the region did not acquire, an owner colliding with an
entry binding, an erasure that does not fit inside the statement it erases
(`REGION-JOINS-ERASURE-FIT`), and the sequential `qcore_acquire` spelling,
which names no task-route resource; every pack obligation keeps the model's
own `Q303`–`Q309`. Two simultaneously live owners refuse at the task route's
existing custody `Q600` — the second acquisition's `?` failure edge discards
the first, the erased program's real leak path (`REGION-JOINS-MULTI-OWNER`).

Since card `REGION-JOINS-LOAN` (wave 30) the body also admits the **adoption**
`move NAME;`: the region takes a live entry owner's cleanup duty into the
pack — the same sealed `Destroy`, an acquisition from the pack's side — so an
**acquired** owner's name is the region's own while an **adopted** owner's
name is the entry's own (a name the entry does not bind refuses at `Q302`, as
does one owner taken twice through either shape). The adoption erases
completely — the owner's acquisition already stands in the entry's own text —
and the discharge's `drop NAME;` is written into the blanks at the region's
source position (after every join for the direct epilogue), where the
**unedited** task route decides everything custodial: an adopted
scalar refuses at that route's `Q500` ("drop requires an owned buffer"), a
drop of a still-lent owner at its `Q600` (`LoanOutstanding`), and an
acquisition beside an adopted owner keeps the `REGION-JOINS-MULTI-OWNER`
boundary. With a worker capturing a counted read share of the adopted owner,
this is **CT25's whole scenario** — cleanup destroys owner after joins *and
loan expiry* — because the share returns at the join under the unedited task
ledger, and the wave-28 after-joins rule (`Q600`) orders the erased release
after every join and therefore after the expiry.
See [the composition record](reports/region-credit-joins/README.md),
[the duty record](reports/region-joins-duties/README.md) and
[the loan record](reports/region-joins-loan/README.md).

Refused at `Q600`, each with a named follow-on: shared storage cells and
shared-cell `read NAME;`/`write` clauses (`T08-COMPOSE-SHARED`; the read-loan
refusal `T08-COMPOSE-LOAN` is **discharged** by card `REGION-JOINS-LOAN`
above), a mutex generation anywhere in the composed body
(`T08-COMPOSE-GUARD`, ranked or not), a joined owned result the creator hands on
rather than releasing or keeping (`T08-COMPOSE-RESULT-REUSE`), a plan that is
not the entry's first statement, a second epoch scope, and a spawn or join
whose span follows the region epilogue (`REGION-CREDIT-JOINS`). The profile's
own refusals still come from the core's `Q630`-`Q641` band. Emission and
reference execution are the scoped-task route's own, byte for byte; the epoch
and the region epilogue are check-only on top. See
[the composition record](reports/remaining-t08-compose/README.md).

## G02g. Rendered lines: `std.fmt`, `std.io` and `std.logger`

`source.std-io.v1` (card `STD-MODULES`) is the whole implemented surface of the
three output modules. A file selects it by using one of its three statements.

```ebnf
module    = { import }, { error_decl | entry } ;
import    = "import", ( "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
            "{", { stmt }, "}" ;
stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "log_level", LEVEL, ";"
          | "flush", ";"
          | "return", ";" ;
RENDERING = "dec" | "hex" ;
LEVEL     = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
OPERAND   = [ "-" ], INTEGER | NAME | "borrow", NAME ;
EFFECT    = "io" | "alloc" | "mut" ;
```

```qcore
#import std.fmt;
#import std.io;
#import std.logger;

fn main() -> unit effects { io } {
    let total = 42;
    print dec total;            // stdout: 42
    print hex 255;              // stdout: 0x000000FF
    log warn dec total;         // stderr: [WARN] 42
    flush;                      // sink handoff; writes no line
    return;
}
```

**The operand rule is the effect discipline.** A *bare* `OPERAND` is an integer
literal or a name the entry bound with `let` to an integer literal. Neither form
can denote an owned buffer, a share or a handle, so a bare rendering cannot
consume, release, store or retain custody of anything — it is not that the
checker refuses those uses, it is that the profile cannot express them. A name
that is not one of the entry's integer bindings is `Q028`; a `let` bound to
anything but an integer literal is `Q030`. The derived effect row is
exactly `{Io}` for a program that writes and empty for one that does not; it is
never `Alloc` and never `Mut`. The entry's declared `effects { … }` row must
admit the derived one (`Q029`).

**The borrowed render** (card `STD-FMT-BORROW`) is the one operand form that
names a resource. `borrow NAME` renders that resource's **model identity** — the
number the emitted C's own call trace spells `rN` — and nothing else: no byte of
the object, no address, no extent, no descriptor, because no model a rendering
can reach carries one. It is admitted only where a checked custody model both
supplied the identity **and** holds it live at that statement, which
`source.std-io.v1` never does: this route has no custody model, so every
`borrow NAME` written on it refuses at `Q028` — structurally, for want of a
model, rather than by a rule the route states. §G02i is where the form is
reachable, and the liveness rule is that route's join.

Card `W48-QR-SCALAR-TYPES`: the three line statements are also read, by the
reader above, on `source.scalar-calls.v1`, where their operand is a scalar of
any §G02t type and `dec`/`hex` render it at its own type. A module reaches that
route only when this route's lowering refuses it and it uses scalar-call
syntax (§G02s), so every module this route admits keeps it byte for byte.

`borrow` was **already** a reserved source word (the checked mutex-guard
profile's), so the form adds none and moves no pinned surface. The marker is
also what keeps the pipe out: `x |> print dec` desugars to `print dec x;` with
no marker, and a sink stage naming an extra selector word refuses at `Q036`
(§G02h), so no pipeline can spell a borrowed render in this revision.

Each module gates its own capability: `print` needs `std.io`, `log` and `flush`
need `std.logger`, and naming a rendering needs `std.fmt`. `flush` names no
rendering, so a logger-only file that never renders needs no `std.fmt`. A
missing import is `Q015` naming the module.

The severity is spelled `fatal`, not `error`: `error` is the reserved
error-type declaration keyword, and this card renamed its own syntax rather than
edit the pinned declaration surface, exactly as `work` became `effort` in G02e.
Eight words are newly reserved — `print`, `log`, `flush`, `dec`, `hex`, `info`,
`warn`, `fatal`. An **unreserved** word in a rendering or level position is
collected as a referenced name and refuses at `Q013` before this route's own
`Q030` message; the refusal is correct and fail-closed, and only the message is
less specific.

The route emits C11 and has a reference execution, compared **per stream**: the
run's stdout against the reference's stdout projection and its stderr against
the stderr projection. **No order between the two streams is modeled or
claimed.** `flush` emits `fflush` and produces no line — sink handoff, not
durability: no `fsync` is performed and no crash survival is claimed. There is
no format-string language, no string or character value, no file or descriptor,
and no per-record source logger filter. `log_level LEVEL;` is the separate,
at-most-once system-trace adapter selection described by the quiet-emission
amendment; it derives `Io`, requires `std.logger`, and does not filter
source-authored `log` records. A `print` inside a `thread scope` or beside an
extern call stays with the more specific route and keeps its refusal. See
[the module record](reports/wave17-std-modules/README.md).

## G02k. An owned byte buffer: `std.buffer`

`source.buffer.v1` (card `STD-BUFFER`) is the whole implemented surface of the
byte-buffer module. A file selects it by using one of its four constructs. It is
the **source-level** tier of the two `std.buffer` tiers
[`STD_BUFFER_V1.md`](design/STD_BUFFER_V1.md) §1 distinguishes; the
native-library-backed tier of `NATIVE_LIBRARY_LINKAGE_V1.md` §6 is untouched by
it and neither retires the other.

```ebnf
module    = { import }, { error_decl | entry } ;
import    = "import",
            ( "std.buffer" | "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
            "{", { stmt }, "}" ;
stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "let", NAME, "=", "bytes", "(", INTEGER, ")", ";"
          | "let", NAME, "=", "extent", "(", NAME, ")", ";"
          | "let", NAME, "=", NAME, "[", OPERAND, "]", ";"
          | NAME, "[", OPERAND, "]", "=", OPERAND, ";"
          | "drop", NAME, ";"
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "flush", ";"
          | "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}"
          | "let", NAME, "=", OPERAND, "as", SCALAR, ";"     (* card W48-QR-SCALAR-TYPES-R3 *)
          | "return", ";" ;
OPERAND   = [ "-" ], INTEGER | NAME ;
BOUND     = INTEGER | NAME ;
EFFECT    = "io" | "alloc" | "mut" ;
```

**The conversion row and the typed environment are card
`W48-QR-SCALAR-TYPES-R3`'s** (§G02t, "The buffer route's typed environment"):
every scalar name this route binds has one §G02t type, `i32` for every source
this section lists, and every position below that takes an integer takes an
`i32`. The conversion is not a buffer construct, so it selects no route.

**The `for` row is `source.loop.v1`'s** (card `LOOP-V1`, §G07.12), admitted here
and on no other route in v1. It is the **only** nested block this profile reads:
every other one still refuses at `Q051`. Its own refusals carry `Q097`–`Q099`;
its body's statements are the rows above, read by this profile's own reader.

```qcore
#import std.buffer;
#import std.fmt;
#import std.io;

fn main() -> unit effects { io, alloc, mut } {
    let b = bytes(8);           // acquire eight zeroed bytes; the obligation opens
    b[0] = 65;                  // write one byte at a checked index
    let n = extent(b);          // the declared length, as an ordinary integer
    let first = b[0];           // read one byte
    print dec n;                // stdout: 8
    print dec first;            // stdout: 65
    drop b;                     // release; the obligation closes
    return;
}
```

**The extent is transported.** `bytes(N)` lowers to an ownership acquisition
whose footprint is `N` on a byte axis, so the declared length lives in the
custody model's own ledger for the resource's whole lifetime and `extent(b)`
reads it back out of there rather than out of a side table. This is the
capability `ExternAssumption::NoExtentTransport` names as missing, supplied **on
the source side only**: a declared C04 contract still cannot say how many bytes
a callee touches, and this profile does not change that.

**Custody is inherited, not reimplemented.** The lowering hands the event
sequence to the unedited cycle-3 ownership checker, which owns every custody
refusal at `Q054`: a leak at the normal exit, a double release, an access after
release, a rebound live name. This profile adds only the rules about *indices*,
because nothing in the language had ever indexed anything.

**Every index, length and stored byte is a check-time constant** — an integer
literal or a name bound to one, which includes a name bound by `extent(…)` or by
a byte read. A non-constant operand refuses at `Q052`. The rule is load-bearing
rather than conservative: this route's evidence is a byte comparison between a
reference execution and a native run, and a value the reference interpreter
cannot compute is one it cannot predict a line for. `Q053` is the bounds and
range rule — an index outside `[0, extent)`, a stored byte outside `0..=255`, an
extent outside `1..=4096` — decided **before emission**, so an out-of-range
access is a refusal and not a run-time trap. `Q055` refuses a buffer at a
position it may not cross: a rendering operand (bare or `borrow`), an index, or a
stored byte.

The derived effect row carries `Alloc` and `Mut`, where `source.std-io.v1`'s is
exactly `{Io}` because its operand grammar admits no resource. §G02d derives
those two as well, but from a **declared contract**'s projections — `Alloc` when
the contract says the callee acquires — whereas here they follow from an
acquisition and a store this profile checks and discharges itself. An
acquisition and a release derive `Alloc`, a store derives `Mut`, a read derives
nothing, and the declared `effects { … }` row must admit each (`Q057` — this
profile's own code, not a reuse of §G02g's `Q029`, because §L21 forbids a second
semantic pass reusing one and this row can carry effects `Q029` never reports).

**Two words are newly reserved: `bytes` and `extent`.** The release is the
**existing** `drop` — the operation is the ownership IR's own drop, so a new
word would have been a synonym — and the byte accessors are the **existing** `[`
and `]`, consumed elsewhere only in usage-interval positions (§G04) and never in
a statement body, so `b[0] = 65;` needs no parser change at all. The acquisition
is spelled `bytes` and **not** `buffer`: `buffer` was measured at 555
occurrences across the source-side corpus, in *binding* position in shipped
fixtures (`let buffer = qcore_buffer_new(1)?;` in
`examples/threads/shared_readers.qr`), so reserving it would refuse them and
this card renamed its own syntax instead — the `work` → `effort` (§G02e)
precedent. The *module* keeps the name `std.buffer`.

The route emits **self-contained** C11 — `calloc`, `free` and two bounds-checked
accessors defined in the emitted file, with no extern declaration, no library
and no boundary — and has a reference execution, compared per stream exactly as
§G02g's is. `calloc` rather than `malloc` so a read before a write has a defined
answer. The emitted bounds comparison and allocation check **abort**, and both
are *witnesses*: every index is a constant the checker already proved in range,
so the comparison is provably never taken, and if one ever fired that would be a
defect in the checker. Every claim on this route is therefore
**partial-correctness**: if the emitted program produces an observation at all,
it is the reference observation; an aborting run produces none.

**No string, and no crossing.** There is no text, no encoding, no literal and no
concatenation — a byte buffer is not a string (follow-ons
`STD-BUFFER-STRING-LITERAL`, `STD-BUFFER-RENDER`). A module that buffers *and*
extern-calls stays with §G02d and keeps that route's refusal; one that buffers
*and* spawns stays with the concurrent route and keeps its `Q600`; one that
buffers and prints reaches this route, whose line statements are §G02g's own.
See [the module record](reports/wave22-std-buffer/README.md).

## G02i. Composed extern calls and rendered lines

`source.std-io-extern.v1` (card `STD-IO-COMPOSE`). Selected when a module both
calls a declared `extern fn` (§G02d) and writes one of the standard-IO
statements (§G02g) — the conjunction of those two routes' own selectors, and
nothing else.

```ebnf
composed_module = "import", "std.extern", ";",
                  { "import", STD_IO_MODULE, ";" },
                  { error_decl | extern_decl | entry } ;
composed_stmt   = extern_stmt | io_stmt ;   (* §G02d and §G02g, verbatim *)
```

**This route adds no statement.** Every statement of a composed entry is lowered
by one of the two existing lowerings, unedited, chosen by the statement head
alone: `print`, `log` and `flush` are the standard-IO model's, `return;` is the
single return *both* models require, and everything else is the extern model's.
Each lowering runs with the other's statements skipped, so neither ever sees a
statement it does not model and neither refusal surface moves. A module using
one feature set is not composed and keeps that route byte for byte.

Route selection places this composition **after every concurrent route and
before both routes it composes**. A module that prints and spawns, or
extern-calls and spawns, therefore keeps its concurrent route's existing
refusal: this card composes two *sequential* models and no other pair.

What the composition decides, and all it decides, is **which value a rendering
reads**. The extern model owns the value namespace, so the operand names of
`print`/`log` resolve against its bindings — a bound scalar, never a resource
identity, which has no operand form at all (§G02g's effect discipline is
structural and is unchanged here). The operand must be bound by an earlier
composed step, must be of a §G02t type (card `W48-QR-SCALAR-TYPES-R3`; before
it, the one ABI width `int32_t` the two renderings were total on), and
must be a value whose number the source **determined**: a `scalar_literal`, or a
`scalar_binding` carrying its declared `succeeds INTEGER` outcome. A bound call
result with no declared outcome is refused at `Q043`, because rendering it would
publish a line no model computed.

Card `STD-FMT-BORROW` adds one rule to that join and no code: a `borrow NAME`
operand resolves against the extern model's **custody** namespace — never its
value namespace — and is admitted only where that model's own published ledger
holds the identity live at the rendering's composed step. An identity never
established (including one whose acquisition took its declared null arm), or one
that departed by consumption, release, storage or reallocation before the
rendering, is refused at `Q043` rather than rendered. The rendering opens no
obligation and discharges none: the composed ledger is still exactly the extern
projection's, and the identity is as live after the line as before it. A
rendering is not a *publication* in §L21's constraint-10 sense — the destination
is a byte stream rather than an object of the custody graph, the bytes carry a
copy of the identity number rather than the address, and no model operation
reads a stream back — so no second reachable path to the resource exists. The
emitted program reads nothing out of the object: the identity is a number the
checker determined, and what the borrow buys is the checked precondition, not a
run-time read.

The composed program's derived effect row is the **union** of the two models'
rows and must be admitted by the one row the entry declares. Its observation is
per stream: standard output carries this route's `print` lines *and* the extern
route's synthesized call trace, in one real order on that one stream, and no
order between stdout and stderr is modeled — the boundary §G02g already stated.

## G02j. Composed scoped tasks and rendered lines

`source.std-io-tasks.v1` (card `COMPOSE-CONCURRENCY`) is selected for a module
that uses **both** the scoped-task forms (§G02c) and the line-producing
statements (§G02g), and that uses no extern call — the conjunction
`has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`, and nothing else.
It is selected **after** the composed-epoch, epoch and mutex routes and
**before** the task route it composes, so every other concurrency exclusion is
unchanged: a module with an epoch scope or a mutex generation keeps that model's
refusal, a module that extern-calls and spawns keeps the task route's `Q600`,
and a module using one of the two composed feature sets falls through to that
route byte for byte.

The grammar adds **no statement and no reserved word**. The three line-producing
statements are §G02g's, read by that route's own parser; the scoped-task forms
are §G02c's, lowered by that route's own lowering. One thing is widened: a task
function's `effects { … }` row admits `io`, and *only* on this route — off it,
`io` is still the unsupported task effect that refuses at `Q020`. The entry must
declare it whenever the composed program writes at all, and since card
`OBS-REORDERING` a **worker** that writes must declare it in its own row too.

```ebnf
task_stmt   = ... | io_stmt ;          (* §G02c plus §G02g's three, verbatim *)
task_effect = "alloc" | "spawn" | "join" | "may_block"
            | "throws", "<", NAME, ">"
            | "io" ;                   (* this route only *)
```

**Placement.** A line stands in the **parent's** body, or — since card
`OBS-REORDERING` — in the body of a function this entry spawns as a worker.

In the parent it may stand anywhere: before any `spawn`, between two `join`s,
inside a `match` arm on the creation or allocation outcome, or after every join.
The reason is a measurement of the emitted runtime rather than a convention:
every *trace* write an emitted task program performs is performed by the
parent's thread, because a worker's trace events are appended to that worker's
private array and replayed by the parent at the join. So a parent line's
position relative to every trace line is determined, and such a program's
streams are compared **byte for byte**.

In a **worker** it is performed by that worker's own thread, concurrently with
the parent's writes and with its siblings'. Card `COMPOSE-CONCURRENCY` refused
that and named its own repair — the comparison-up-to-an-admitted-reordering
capability — and card `OBS-REORDERING` landed it
(`qcore.obs-reordering.v1`, `docs/design/OBS_REORDERING_V1.md`). A worker line
is admitted, and placing one is what **declares** the relation: that program's
streams are compared window by window and emitter by emitter, where a window
boundary is a point of the run at which no worker is live and an emitter is the
thread that wrote the bytes. Two extra conditions come with it, both `Q050`:

* the worker's **own** `effects { … }` row must admit `io` — the effect
  discipline is per function, and a worker body is a function body;
* a line in a function that is neither the entry nor a spawned worker is refused
  outright: nothing in this profile makes such a function run, so its line has
  no thread to attribute it to.

A program that places **no** worker line declares nothing, and its comparison is
unchanged: a relation whose windows each have one emitter admits exactly one
stream, its own. All four published `examples/threads` programs are of that
kind.

**Operands.** A rendering operand is an integer literal or a name the *task*
model bound to an `i32` — since card `W48-QR-SCALAR-TYPES-R4`, to a value of
any §G02t type, rendered at its type (§G02t; `bool` has no `hex`, `Q050`):

```ebnf
OPERAND     = [ "-" ], INTEGER | NAME     (* no `borrow NAME` on this route *)
            | NAME, path ;                (* a scalar field of a record, W48-QR-TASK-RECORD-CAPTURES *)
```

Card `W48-QR-TASK-RECORD-CAPTURES` adds the field operand: `print dec r.f;` or
`log LEVEL dec r.f;` renders a **scalar** field of a record the function holds
at the field's type, in the parent or in a worker, read out of the field's
leaf slot; a sub-record, an owned field and a whole record refuse (`Q050`).
The operand is read by this route alone, so every other route's §G02g grammar
is unchanged.

Unlike §G02g's own `let`-bound literals and §G02i's declared outcomes, the number
is **computed by the task model** at the rendering's own step — the value a
`qcore_buffer_read` read out of a live buffer, or the outcome a `join`
collected — so the reference execution reads it out of the slot the task model
wrote. Whether the name may be read at that position is decided by
`check_task_module`, over an ordinary copy of the scalar inserted at exactly that
position; the composition states no liveness rule of its own. A name denoting an
owned Buffer, a read loan or a task handle is refused, and `borrow NAME` is
refused on this route entirely (§G02i is where a borrowed render is admitted,
over a model that publishes a ledger the join can read).

The composed observation is per stream, exactly as elsewhere: standard output
carries this route's `print` lines *and* the synthesized task trace, on that one
stream, and no order between stdout and stderr is modeled. That stream's order
is one real order when every line on it is the parent's — which it is unless a
worker `print`s — and otherwise is the declared reordering relation's.
Removing this route's own lines from standard output leaves exactly the task
route's rendering of the same module, which is checked over the four published
`examples/threads` programs and proved in general as
`QCore.ComposedTasks.erasing_the_source_lines_leaves_the_task_trace`.

## G02h. The pipe operator, and the operator token family

`source.pipe.v1` (card `PIPE-OPERATOR`) is a **parse-time desugaring, not a
profile**: a file does not select it, and no route knows it exists. A
well-formed pipeline is rewritten into the tokens of the statement a reader
would have written without it, before the delimiter map is built and before any
parsing, so name resolution, custody, effects, linearity, concurrency,
whole-program checking, reference execution and C emission all see an ordinary
statement. A piped program and its hand-written twin are `syntax_equivalent`
and emit byte-identical C11.

```ebnf
stmt       = [ "let", NAME, "=" ], OPERAND, ( "|>", STAGE )+, ";" ;
OPERAND    = token, { token } ;            (* verbatim: `x`, `-7`, `move b` *)
STAGE      = [ "move" ], STAGE_BODY ;
STAGE_BODY = "print", [ RENDERING ]        (* a G02g sink; no selector renders text *)
           | "log", LEVEL, [ RENDERING ]   (* a G02g sink; no selector renders text *)
           | "drop"                        (* the ownership IR's own release *)
           | NAME, [ "(", [ arg, { ",", arg } ], ")" ], [ ADAPTER ] ;  (* a G02d call *)
ADAPTER    = "succeeds", BOOL, "else", { disposition }, "raise", NAME ;
```

| Written | Desugars to |
|---|---|
| `total \|> print dec;` | `print dec total;` |
| `total \|> log fatal dec;` | `log fatal dec total;` |
| `s \|> print;` | `print s;` (card `STRING-SLICE-IO`) |
| `s[0:4] \|> log info;` | `log info s[0:4];` (card `STRING-SLICE-IO`) |
| `b \|> drop;` | `drop b;` |
| `x \|> f;` and `x \|> f();` | `f(x);` |
| `b \|> f(7);` | `f(b, 7);` |
| `move b \|> release;` | `release(move b);` |
| `8 \|> alloc succeeds true else raise E \|> move free;` | `let ⟨t⟩ = alloc(8) succeeds true else raise E;` then `free(move ⟨t⟩);` |
| `let r = x \|> f succeeds true else raise E;` | `let r = f(x) succeeds true else raise E;` |

**A chain is a statement sequence** (card `PIPE-CHAIN`). A pipeline of `n`
stages desugars to `n` statements: each non-final stage binds its result to a
hygienic temporary the next stage receives, and the final stage keeps the
statement form the reader wrote around the whole pipeline. Custody flows through
the temporaries under the unedited checkers — by the time any of them runs a
temporary is an ordinary binding — so a chain that drops a live result refuses
at the pre-existing `Q202`, and a value consumed mid-chain cannot be used after
the pipeline.

**"Produces a value" is decided by stage kind**, syntactically and with no
contract knowledge in the desugaring: a call stage produces a value, and a
statement stage (`print`, `log`, `drop`) or an operandless statement (`flush`)
does not. Card
`PIPE-CHAIN` decided it by the presence of `succeeds`, because the acquisition
adapter was then the only `let`-with-call the language admitted; card
`PIPE-SCALAR-RESULT` adds the other one — a non-acquiring call's scalar result
(§G02d) — so the keyword no longer separates the two. Whether a particular
callee's declared result is bindable is the **route's** question, answered from
the declaration and reported at that stage's own span: a `unit`-returning callee
in a non-final position still refuses, at `Q024` naming its result rather than
at `Q037` naming the pipe. This is what makes `x |> render |> scale`
expressible, which card `PIPE-CHAIN` measured as the first of the two boundaries
under the brief's `value |> render |> log`; the second, a module that both calls
an extern function and prints, is unchanged and remains `STD-IO-COMPOSE`.

**A model intrinsic is a call stage** (card `PIPE-INTRINSIC-STAGE`). Card
`COMPOSE-CONCURRENCY` filed the brief's literal `buffer |> read |> render |> log`
as needing "a pipeline stage for a task intrinsic, which is a pipe-grammar
change". It is not, and the census is the reason: the source language's model
intrinsics fall into two shapes, and this grammar already decides both.

| Shape | Intrinsics | Where | As a stage |
|---|---|---|---|
| **call-shaped**, `NAME(ARGS)` | `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload` | §G02b task routes and every route that composes them (`ComposedStdIoTasksV1`, `ComposedEpochTasksV1`) | **admitted** by the `NAME [ "(" … ")" ] [ ADAPTER ]` alternative above, with no table entry of its own |
| **call-shaped**, `NAME(ARGS)` | `qcore_acquire`, `qcore_check` | the sequential route | **admitted**, likewise |
| **keyword-shaped** | `spawn S W(ARGS)?`, `join move H?`, `share N`, `read N`, `write N`, `grow`, `open`, `close`, `bound`, `overhead` | their own routes | **refused**: their operands are not a parenthesized argument list, so no call stage produces them, and the closed statement table does not name them |
| **keyword-shaped** | `drop N` | every route with custody | **admitted** by the `"drop"` alternative above (card `PIPE-DROP-STAGE`), and it is the only keyword-shaped head that is |

So `buffer |> qcore_buffer_read |> log info dec;` is the statement sequence
`let ⟨t0⟩ = qcore_buffer_read(buffer); log info dec ⟨t0⟩;` under the ordinary
chain rule, emitting byte-identical C against that hand-written pair. An
intrinsic whose result the surrounding statement discards, or whose result the
next stage's route cannot bind, refuses at **that route's** own code and span —
`qcore_buffer_write` returns `unit` and refuses mid-chain at `Q050`, not at
`Q037` — which is the rule §G02d already states for a `unit`-returning callee.
A chain whose intermediate stage *allocates* binds an owned resource to a
temporary no reader can name; unless the chain **ends in a release** it leaks and
refuses at the route's pre-existing leak diagnostic.

**One keyword-shaped head is a stage: the release** (card `PIPE-DROP-STAGE`).
`drop` names the statement table's third entry and its only *releasing* one, so
`b |> drop;` is `drop b;` and `7 |> qcore_buffer_new? |> drop;` is the pair
`let ⟨t0⟩ = qcore_buffer_new(7)?; drop ⟨t0⟩;` — the allocating chain above,
spellable end to end, with the pipeline releasing the temporary it invented. A
releasing entry emits by the same rule as a rendering one and binds no result by
the same rule, so it is **last-stage-only** without a rule of its own: a released
value cannot feed a later stage because there is no value to feed (`Q037`), and a
bound pipeline cannot end in one (`Q036`). What a *later statement* does with a
released binding is the checker's unchanged business — `b |> drop; b |> drop;` is
the ordinary double release. Every other keyword-shaped head keeps the refusal it
had, on a criterion stated in `crates/qcore-frontend/src/pipe.rs`'s stage table:
an entry earns its place only when the statement form is `HEAD SELECTOR* OPERAND`
with closed selectors **and** some stage can feed it a value no reader can name.

Card `STRING-SLICE-IO` made each entry's selector count an **inclusive range**
rather than a number — `print` takes 0 or 1, `log` takes 1 or 2 — because a
rendered string names no `std.fmt` rendering (§G07.9). The pipe decides nothing
about which arity a route admits: it emits the head, the selector words the
reader wrote and then the operand, and the route's own reader refuses a shape it
does not model at its own code and span. `x |> print;` on `source.std-io.v1`
therefore refuses at `Q030` naming the missing `dec`/`hex` rather than at `Q036`
naming the pipe — a refusal moved from the pipe to the route, never an
acceptance, and every pipeline this table accepted before that card desugars to
the same tokens because every one of them wrote the maximum.

The same card makes "no later pass can tell a pipe was written" true of **every**
route. The sequential route recovered each statement's tokens by re-lexing its
source bytes rather than reading the program's own post-desugaring stream, which
for a pipeline recovers the pipeline: the operand sits left of the callee in the
bytes and right of it in the statement, so that route's own `qcore_acquire`
reader saw the operand where it wanted the callee name. Reading the parsed
stream is identical on every pipe-free file — it is the lex of that same source —
and it is what admits the two sequential intrinsics above.

**Two spellings name the value a previous stage produced**, because a hygienic
temporary has no name a reader could write: `|> move STAGE` consumes it,
mirroring `move OPERAND |> STAGE` for the first stage (where `move` on the stage
is refused, one spelling per meaning); and a bare `move` inside a stage's adapter
tail names it, so an acquisition's error arm can dispose of the value piped into
it. Without the second, `Q025` would cap a chain at one acquisition.

**Hygiene is enforced by refusal.** Temporaries are named `qcore__pipeN`; a file
that needs them and also writes that prefix is refused at `Q036` rather than
silently α-renamed, so no diagnostic can name a binding the reader did not
write. A file whose pipelines are all single-stage generates none and is
untouched.

**Insertion is at the stage's first operand position** — Elixir-style
first-argument insertion, chosen because QCore is subject-first and has no
currying, so there is no partial application for a last-argument rule to
compose with. For a parenthesized call that position is immediately after `(`,
before any argument already written. For a statement stage the head and its
closed selector words are not arguments, so the operand follows them; both v1
statement stages take exactly one operand, so "first operand position" and
"append" coincide and no selector-arity table is needed.

**Attribution.** Every token the reader wrote keeps its own span, unmoved, so a
refusal inside a desugared statement points at the bytes that were typed: an
operand fault at the operand, a stage fault at *that stage*. Only `let`, `=`,
`;`, `(`, `)`, `,` and the temporaries are synthesized, each anchored zero-width
at the `|>` it came from — at that pipe's start for the first stage and at its
end for a later one, which is what keeps consecutive statements' byte spans
strictly disjoint where a `|>` abuts its neighbours, since a statement's
synthesized terminator sits at the next pipe's start and `|>` is two bytes wide.
The one exception is recorded rather than hidden: in a **multi-stage** bound
pipeline the `let NAME =` the reader wrote sits to the left of every stage, so
keeping its spans would make the last statement's span swallow the ones before
it; the binder is re-anchored at the final `|>` with the reader's own name
preserved. A one-stage bound pipeline keeps its binder tokens where they were.

**Refusals.** `Q036` a pipeline shape this profile does not model — an empty
side, a stage that is not a name, a sink stage of the wrong selector arity, an
unbalanced call stage, a malformed or value-less `let NAME =` binder, an
assignment pipeline (follow-on `PIPE-ASSIGN`), `move` on the first stage, a bare
`move` whose incoming value is not a binding, or a file that needs pipeline
temporaries and writes the `qcore__pipe` prefix itself; `Q037` a stage that
cannot receive the operand — `flush`, which takes none, or a **sink** in a
non-final position, since only a call stage binds a result;
`Q038` an operator token used where this revision gives it no meaning — one of
the five operator spellings `|`, `&`, `^`, `||`, `&&` **outside an `if`
condition** (see the table below). Card `BITWISE-CONDITIONS` gave the three
single-byte spellings a meaning in that one position, so the gate is now
positional for all five and no longer holds any of them as a pure reservation.
Dropping the `;` of the last statement in a body is a parse failure (`Q000`)
before the pipe pass runs.

### The operator token family

Two-byte spellings are tried before every one-byte spelling, so tokenization is
longest-match at a fixed width of two. The vertical-bar-family spellings are
pairwise distinct byte pairs (or a single byte), so their order in the table is
immaterial and the resolution is decided by width alone.

| Spelling | Token | Status |
|---|---|---|
| `[\|` / `\|]` | `QuantOpen` / `QuantClose` | **unchanged** — the G04 quant-index delimiter alias |
| `\|>` | `PipeInto` | **landed** — desugared away before parsing (§G02h above) |
| `\|\|` | `PipePipe` | **logical or** — one token; admitted in an `if` condition and lowered there (§G02i) |
| `&&` | `AndAnd` | **logical and** — same, binding **tighter** than `\|\|` |
| `\|` | `Pipe` | **bitwise or** — admitted only as a parenthesized comparison operand in an `if` condition (§G02i); `Q038` elsewhere |
| `&` | `Ampersand` | **bitwise and** — same position, binding **tighter** than `^` |
| `^` | `Caret` | **bitwise xor** — same position, between `&` and `\|` |

**This table supersedes card `PIPE-OPERATOR`'s, by explicit user direction**
(card `OR-AND-LOWERING`). That card had decided the single `|` was logical or
and `&&` logical and, with `&` held for binary use and `||` refused as a
misspelling of `|`. The retable inverts the or half: **both** logical operators
are double-character and short-circuit, and **all three** single characters are
freed for a future bitwise wave. That also answers the open note wave 18 left —
bitwise or had no spelling left — by giving it `|` back.

`[|` is matched at the `[`, so `[|>` is always `[|` then `>` and never `[` then
`|>`, and `[||]` is the two quant delimiters and never an or-operator. `|>` wins
over `||` and over `|`, so the ambiguity wave 18 recorded (`a |> b` against a
hypothetical `a | > b`) was inherited by **`BITWISE-CONDITIONS`**, which
measured it and recorded the answer: `|| >` is unambiguous, and a bitwise `|`
adjacent to a `>` must be spaced — but the adjacency can never produce a wrong
*acceptance*, only a refusal, because `>` has no left operand inside an integer
expression. `(1 | 2) > 0` is the spaced spelling and it parses; `1 |> 2` is a
pipeline in a condition and refuses at `Q045`.
`|||` is `||` then the reserved `|`; `&&&` is `&&` then the reserved `&`.

**Two deliberate pin updates, neither changing any program's verdict.** A bare
`|` was a lex error before card `PIPE-OPERATOR` and a `Q038` refusal after it; it
stays a `Q038` refusal outside a condition, with the message naming the position
it *is* admitted in rather than claiming the token. A bare `^` was a lex error
(`unexpected character '^'`) and became a token whose occurrences refuse at
`Q038` — the same move one card later, for the same reason: an actionable
message. A bare `&` refused downstream in whichever route claimed the module and
now refuses at the gate. In all three cases the retable itself left the set of
accepted programs unchanged and moved only the layer and the message; card
`BITWISE-CONDITIONS` then *widened* the set, in one position and by one
grammar production each (§G02i). The census behind the retable
is in [the card record](reports/wave19-or-and-lowering/README.md): `[|`, `|]`,
`||`, `&` outside comments and `^` appear in **no** `.qcore` file at all, and
`|>` only in the four pipe fixtures.

## G02i. The source branching construct

```ebnf
stmt     = "if", "(", cond, ")", block, [ "else", ( block | stmt ) ] ;
cond     = conj, { "||", conj } ;                 (* 1, left-associative  *)
conj     = neg,  { "&&", neg  } ;                 (* 2, left-associative  *)
neg      = "!", neg | cmp ;                       (* 3, right-associative *)
cmp      = int, [ op, int ] ;                     (* 4, non-associative   *)
int      = xor, { "|", xor } ;                    (* 5, left-associative  *)
xor      = and, { "^", and } ;                    (* 6, left-associative  *)
and      = atom, { "&", atom } ;                  (* 7, left-associative  *)
atom     = "(", cond, ")" | "true" | "false"
         | [ "-" ], INTEGER | NAME ;
op       = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

The grammar is **two-sorted**: levels 1–4 produce a *condition* and levels 5–7
an *integer*, `atom` produces either, and one parser reads both — so a sort
mismatch is a refusal rather than a second parse. There is no coercion in either
direction: `if (mask & 4)` and `if (true | false)` are both refused (`Q045`).

`source.branch.v1` (card `IF-STATEMENT`), on the **sequential route only**. The
statement shape is the one the parser has always accepted; what the card added
is that the condition's bytes are now *recorded* rather than discarded, so a
semantic pass can read them. `else if` flattens into one statement with one
condition per header, and lowers to a right-nested branch. Nesting an `if`
inside an arm is admitted — it is the shape a short-circuit lowering needs.

**No new reserved word and no new token.** `if`, `else`, `true` and `false` were
already reserved source words, and `==`, `!=`, `<=`, `>=`, `<` and `>` were
already single tokens. The census across all 369 `.qcore`/`.qc` files (317 of
them source-side, where wave 18 counted 294) found `if`
as a word in four files — three of them `//` comment text quoting C inside
`providers/quirin-provider-fluent-bit/contracts/fluent_bit/`, and one an archived probe the standing gate does
not replay — and `else` in sixteen, every one of them the extern adapter's
`succeeds BOOL else … raise ERROR` tail, which is a `;`-terminated statement and
never follows a `}`.

**The condition is decidable at check time**, exactly as the `succeeds` and
`passes` literal outcome controls beside it are, because the translation
certificate compares one reference observation, one CFG observation and one
observation parsed from the compiled program's own output. Every leaf is closed
over literals, so evaluation stays state-free however the leaves are combined.

**The connectives (card `OR-AND-LOWERING`).** Card `IF-STATEMENT` shipped the
tier with one leaf and no connectives, naming the successor card that would add
them. `||` and `&&` are now admitted **in a condition and nowhere else**, and
they are a pure desugaring onto this construct:

```text
if (A || B) T else E   ==>   if (A) { T } else if (B) { T } else { E }
if (A && B) T else E   ==>   if (A) { if (B) { T } else { E } } else { E }
```

The arms are lowered **once** and their operation vectors duplicated into nested
branches, so a connective adds no statement, no acquisition and no checker rule;
`qcore_core` is byte-unchanged by the card, and the emitted C is byte-identical
to the hand-lowered twin's. Short-circuit is structural rather than asserted —
the right operand is the condition of a branch nested inside an arm of the left
operand's — though it is not *observable*, because every leaf is a pure literal
comparison; `IF-RUNTIME-CONDITION` is what would make it so. Every connective
level runs the same four-model custody join, so a connective cannot launder a
`Q046` disagreement: `if (A || B) { drop a; }` is the conditional leak exactly as
`if (A) { drop a; }` is.

**The negation and the integer tier (card `BITWISE-CONDITIONS`).** Unary `!` is
eliminated by an **arm swap**: `if (!C) { A } else { B }` is the branch a reader
writes as `if (C) { B } else { A }`, so `!!A` is literally `A`'s own lowering,
the emitted C is byte-identical to the hand-swapped twin's, and no checked model
sees a negation. The three bitwise operators `|`, `^` and `&` are admitted in
**one** position — a parenthesized comparison operand — over signed 32-bit
literals and names bound to one. A `NAME` atom is a **check-time constant
binding**, `let NAME [: i32] = INTEGER;`, admitted at the top level of a sequential
entry body and substituted for its value here; a name that denotes a resource
identity is refused, so no custody-bearing value gains a position in this tier.

**Precedence is declared, and the rule for declaring it is stated once: a
precedence is declared only where it agrees with C, and where this tier's two
sorts would force a different ranking than C's, the unparenthesized spelling is
refused rather than silently redefined.** Levels 1, 2, 5, 6 and 7 are C's own
order and are declared: `&&` tighter than `||`, `&` tighter than `^` tighter
than `|`, all left-associative, parentheses group. Levels 3 and 4 are not — C
ranks `!` tighter and `|`/`^`/`&` looser than the comparisons, while a tier in
which a comparison yields a condition and a condition is never an integer
operand must rank them the other way — so `!A < B` and `A | B == C` are
**refused** (`Q045`) and `!(A < B)` and `(A | B) == C` are the spellings. A
comparison is non-associative, so `1 < 2 < 3` is refused too.

Two measured consequences are recorded rather than designed. For a *homogeneous*
chain the lowering is associative on the nose — `(A || B) || C` and
`A || (B || C)` emit identical C, and so do the two groupings of `1 | 2 | 4` —
so each associativity declaration is load-bearing only where operators of
different levels mix. And a negation over two *identical* arms is unobservable,
for the same reason a swap of equals is: `!` is load-bearing exactly when the
arms differ.

`|>` is **not** in the tier and is never given a precedence relative to anything
in it: a condition mixing them is refused at `Q045`.

**The branch is nonetheless real.** Both arms are validated in full, both are
emitted as basic blocks, and the emitted C computes the comparison itself —
`if (INT32_C(3) < INT32_C(5)) { goto …; } else { goto …; }` — rather than
receiving the decided literal, so the reference evaluator and a C compiler are
cross-checked by the translation-validation triangle and by a sanitized native
run. Only the *observation* is single-valued; follow-on `IF-RUNTIME-CONDITION`.

**The custody join is agreement between the arms that survive it, never
widening.** Both arms are validated from the same incoming state, and an arm
that **returns** discharged its obligations at its own exit and does not reach
the join at all (card `IF-ARMS`); the join compares the arms that fall through,
and where there are two of them it admits only equal ordered live lists and equal
transferred sets. **Four** models check that agreement — the cleanup lowering,
ownership (`OwnershipOp::Branch`), typed errors (`ErrorOp::Branch`) and resource
flow (`Flow::join_branches`) — and a fifth, usage (`UsageExpr::Branch`), joins
the arms' *demands*. Only `SourceOp::Branch`, in the cleanup lowering, is new:
the other four were all already shipped, each with its own mismatch error.

### The arm statement subset

| Arm statement | Admitted | Ground |
|---|---|---|
| `qcore_check(BOOL, Error);` | yes | custody-neutral |
| `drop NAME;` / `move NAME;` | yes | the join's own subject |
| a nested `if` | yes | the shape `\|\|`/`&&` lower onto |
| an empty arm; `if` with no `else` | yes | the missing arm is the empty arm |
| `return …;` | **yes** (card `IF-ARMS`) | an exit from an arm carries exactly the obligations the sequential route imposes at any exit, so the returning arm discharges at its own exit and leaves the join |
| a zero-argument call `f();` | **yes** (card `IF-ARMS`) | a source call emits no C at all in this subset; it composes the callee's summary flow into the caller's, and a *conditional* call is admitted only when that composition is the identity |
| `let NAME = …;` | no — `Q044` | an arm-local binding has no position after the join |
| `let n = qcore_acquire(…);` | no — `Q103` | two arms leave different resource-identity ledgers; `IF-ARM-ACQUIRE` |
| `while`, `match`, `defer`, `raise`, … | no | each keeps its own code; `while` inside an arm is still `Q500` |

**The continuation moves with the exit.** Four of the five models cannot express
"this arm exited", so the driver does not hand them a one-sided join: the
statements written *after* a branch one arm of which returns are lowered inside
the arm that survives to reach them, which is where they run. The rewrite is a
**move**, never a duplication — the continuation is lowered exactly once — so the
shape that would need it in two arms (an early return in one arm of a nested
branch whose sibling also falls through) is refused rather than served by
duplicating acquisitions, calls and demands.

**Refusals.** `Q044` a shape this profile does not model — inconsistent block and
condition counts; a `let` inside an arm; a branch with one returning arm and no continuation for the
survivor to reach, or one whose continuation still reaches no `return`
(`IF-ARM-RETURN-NESTED`); statements after a branch every arm of which returns;
or a conditional call whose callee's summary flow is not neutral
(`IF-ARM-CALL-FLOW`). `Q045` a
condition outside the tier — a chained comparison, an arithmetic expression, a
shift, a complement, a pipeline, an empty condition, an unbalanced parenthesis,
a connective missing an operand, an integer where a condition is required or a
condition where an integer is required, an **ungrouped** bitwise comparison
operand or `!` operand, a name bound to a resource identity, or a non-integer
operand.
`Q046` two surviving arms do not reach the join in the same custody state.
`Q202` an arm exit with an owned resource still live — the same condition, and
the same code, as the program's own exit, reported at the exit that leaks. A name
written in a condition that is bound to nothing is collected as a referenced
name and refuses earlier at `Q013`. An operator token written **outside** a
condition refuses at `Q038` before the tier is consulted — all five of `|`,
`&`, `^`, `||` and `&&`. An acquisition
inside an arm — or in a continuation hoisted into one — is refused by the
pre-existing ownership join at `Q103`, inherited unedited and only re-attributed
(`IF-ARM-ACQUIRE`). Every other route keeps its own refusal, measured: `Q030` on
`source.std-io.v1`, the extern route's nested-block refusal on
`source.extern-call.v1`, and `Q500` on the copy-scalar task profile — and neither
a connective in the condition nor a widened arm statement changes which refusal
answers. See
[the branch record](reports/wave19-if-statement/README.md),
[the connective record](reports/wave19-or-and-lowering/README.md),
[the integer-tier record](reports/wave20-bitwise-conditions/README.md),
[the arm record](reports/wave20-if-arms/README.md) and
[the design note](design/SOURCE_BRANCH_V1.md).

## G02k. The checked event surface

Card `STD-EVENT`. Design record: [`design/STD_EVENT_V1.md`](design/STD_EVENT_V1.md).
Example page: [`../examples/events/README.md`](../examples/events/README.md).

```ebnf
module    = { import }, entry ;
import    = "import", ( "std.event" | "std.logger" | "std.io" | "std.fmt" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "i32",
            [ "effects", "{", effect, { ",", effect }, "}" ], "{", { stmt }, "}" ;
stmt      = "event", "open", ";"
          | "log_level", level, ";"
          | "event", "close", ";"
          | "endpoints", NAME, NAME, ";"
          | "watch", NAME, NAME, interest, ";"
          | "unwatch", NAME, ";"
          | "wake", NAME, [ byte ], ";"
          | "dispatch", "ready", NAME, ";"
          | "let", NAME, "=", "dispatch", "ready", NAME, ";"
          | "dispatch", "idle", ";"
          | "close", NAME, ";"
          | "print", rendering, operand, ";"
          | "log", level, rendering, operand, ";"
          | "flush", ";"
          | "return", [ "-" ], INTEGER, ";" ;
interest  = "read" | "write" ;
byte      = INTEGER (* 0 through 255, refused rather than truncated *) ;
rendering = "dec" | "hex" ;
operand   = INTEGER | NAME ;
level     = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
effect    = "io" | "alloc" | "mut" ;
```

**The value vocabulary is exactly one form** (card `COMPOSE-EVENT-LINES`).
Until that card this grammar had no `let` and no scalar operand at all: every
name it resolved denoted an endpoint or a registration, a custody name. It now
has `let NAME = dispatch ready E;` and nothing else — the byte that turn's
callback drained off the endpoint, which is the byte this program's own `wake`
delivered. A custody name is still not an operand: rendering one refuses at
`Q074`, and `borrow NAME` refuses there too, because this route publishes no
live custody identity for a render (`STD-FMT-DESCRIPTOR`). The derived effect
row is still exactly `{Io}`, and `Alloc` and `Mut` are still unreachable — now
because the one value form binds a byte rather than because there is no value
form.

The line-producing statements are `source.std-io.v1`'s own, read by that route's
own reader and rendered by that route's own emitter, on the `source.buffer.v1`
precedent; each is gated by its own module (`print` by `std.io`, `log` and
`flush` by `std.logger`, a rendering by `std.fmt`) exactly as it is there. Every
line in `examples/events` is spelled as a `|>` pipeline — `value |> log info dec;`
— which desugars to the statement form and emits the same C byte for byte, so
that is a rule about how the examples read rather than a restriction on the
route.

The original eight reserved words are `event`, `endpoints`, `watch`, `unwatch`,
`wake`, `dispatch`, `ready`, and `idle`; the quiet-emission amendment adds
`log_level`; card `COMPOSE-EVENT-LINES` adds **none** — `let` is the language's
own binding word, and `print`, `log`, `flush`, `dec`, `hex` and the six levels
were reserved by `STD-MODULES` and by the amendment. Four existing ones are reused rather
than respelled: `open`, `close`, `read`, `write`. The context pair is spelled
`event open;` / `event close;` for exactly that reason. The standing constraint
14 census over every `.qcore`/`.qc` fixture and every inline source string found
no collision: the corpus occurrences of `event` are ghost-field names inside
`contract { … }` clauses, never a statement head and never a binding.

`log_level LEVEL;` may occur once, with or without a live event context. It
sets the typed diagnostic sink threshold for this emitted event program. If it
is absent the sink is null; if present, admitted event records at or above the
level are rendered on stdout by the hosted system-trace adapter. It projects to no reactor operation and changes
no custody or readiness judgment.

**The custody half is the existing model.** Each statement projects onto
`qcore_core::event_reactor::EventOperation`s and the projection is handed to
that model's own checker unmodified; a refusal at `Q060` carries its `L00`–`L12`
diagnostic verbatim. This card is what made that model reachable from source at
all.

| Statement | Projection |
|---|---|
| `event open;` / `event close;` | `CreateBase` / `FreeBase` |
| `endpoints A B;` | `CreateSocketPair` (success branch) |
| `watch E D INTEREST;` | `Assign` naming the descriptor, then `Add` — the custody edge |
| `unwatch E;` | `Delete` then `FreeEvent`, retiring the registration's cookie |
| `wake D;` / `wake D BYTE;` | *(none — the reactor models no byte flow; the payload is a value and never a trace record)* |
| `let V = dispatch ready E;` | the same five operations `dispatch ready E;` projects to; the binding names the drained byte |
| `print` / `log` / `flush` | *(none — a source line is program output, not a compiler observation)* |
| `dispatch ready E;` | `BeginDispatch`, `Activate`, `CallbackEnter`, `CallbackExit`, `EndDispatch` |
| `dispatch idle;` | `BeginDispatch`, `EndDispatch` |
| `close D;` | `CloseSocket` (success branch) |

**The determinism boundary is this route's own.** A certificate compares one
observation, so v1 waits only on readiness the program itself induced with
`wake D;`, and each turn declares what it reports — a declaration the emitted
program verifies against what `poll(2)`/`WSAPoll` really reported, exiting
non-zero on disagreement. A turn at which two or more registrations are ready is
refused **by name** at `Q061`: `poll(2)` reports a set, and the callback order
within one ready set is the implementation's rather than one the program
induced. Every registration carries priority `0` so that a priority order is not
smuggled in as an undeclared ordering premise. A `write` interest is accepted
and checked and makes the module emit no C at all, on the `accept_check_only`
precedent.

Diagnostics: `Q058` shape, `Q059` an unbound custody name, `Q060` the reactor
model's own refusal, `Q061` the determinism boundary, `Q062` the effect row,
`Q074` the composed line and value vocabulary's own refusals (second band of the
same pass, card `COMPOSE-EVENT-LINES`, relocated at integration from `Q065`), and `Q015` for a missing
`#import std.event;` or a line-producing statement's own module.

**The composition moves none of the five codes above.** A line erases to no
reactor operation and no ledger transition —
`QCore.ComposeEventLines.erasing_the_lines_leaves_the_reactor_projection` and
`…_leaves_the_run_unchanged` — so `Q061`'s two-member refusal, the
declared-readiness rule and the `write`-interest check-only arm answer exactly as
they did. A program that names a drained byte carries a second named platform
premise, `EVENT-V1-STREAM-BYTE-ORDER`, and the emitted program checks it: a byte
that is not the one the model predicted exits non-zero rather than rendering a
line no model computed. Both streams of an executable module are compared byte
for byte, which is this route's own measurement — one accepted profile, one
thread, so every record of the derived `qcore.obs-reordering.v1` relation is a
barrier and the relation is strict.

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
quant_type = "quant", "<", (scalar | vector), [where_clause], ">"
           | "quant", "[|", (scalar | vector), [where_clause], "|]" ;
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

`quant[| ... |]` is a delimiter alias for `quant< ... >` over the identical interior grammar: `[|` and `|]` are single lexer symbols, both spellings resolve to the same closed-index type, and the opener fixes the required closer, so a mixed pair such as `quant[| ... >` is refused, never repaired. A bare `|` remains a lexer error. The alias exists because `<`/`>` collide visually with where-clause comparison operators; it adds no semantic surface.

`quant<cost,...>` is rejected by this resolver as a future total/peak profile. `quant<access,...>` is skipped by this closed-index parser for separate handling; skipping it is not source-level access verification. The axis environment is now built and validated unconditionally, including modules without quantitative type occurrences (R05).

Source: `crates/qcore-frontend/src/quant_index.rs`, `frontend.rs:1519–1548`. A supported canonical example is:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;
extern fn keep(q: quant<n where n := 5> use one) -> unit;
extern fn hold(q: quant[|n where n := 5|] use one) -> unit;
```

## G05. Resource loader v1

```ebnf
resource_module = "module", "contract", ";", {alias_declaration}, resource_fn,
                  {resource_fn | alias_declaration} ;
  (* card SUPPLIED-OWNED-RETURN-CODE-HANDOFF, contract specification C04.19.5 *)
alias_declaration = "alias", "#", IDENT, "=", "@", IDENT, ";" ;
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
            | "ptr", "<", "mut", ",", "ptr", "<", "mut", ",", "u8", ">", ">"
            | "ptr", "<", "mut", ",", "ptr", "<", "const", ",", "u8", ">", ">" ;
  (* the last: card W43-ABI-ENUM-AND-CONST-PP, a pointer to a pointer to const,
     contract specification C04.1 *)
resource_clause = "requires", "resource_contract_v1", ";"
                | "requires", literal_argument_premise, ";"
                | "requires", contract_review_marker, ";"
                | "requires", "resource_body_checked", ";"
                | "requires", named_premise, ";"
                | "requires", hypothesis_atom, ";"
                | "requires", hypothesis_all, ";"
                | "requires", hypothesis_decomposition, ";"
                | "requires", premise_flow, ";"
                | "requires", discriminant, ";"
                | "ensures", "normal", resource_predicate, ";"
                | "ensures", "normal", arm_predicate, ";"
                | "ensures", "normal", return_code_disposition, ";"
                | "ensures", "normal", clause_review_marker, ";"
                | "requires", contract_revision_marker, ";"
                | "ensures", "normal", clause_revision_marker, ";" ;
hypothesis_atom = "resource_hypothesis", "(", premise_name, ",", STRING, ")" ;
hypothesis_all = "resource_premise_all", "(", premise_name, ",", premise_name, {",", premise_name}, ")" ;
hypothesis_decomposition = "resource_premise_decomposition", "(", premise_name, ",", premise_name, {",", premise_name}, ")" ;
named_premise =
    "resource_premise", "(", premise_name, ",", STRING, ")" ;
  (* card VERDICT-CONTRACT-DEPENDENCIES, contract specification C04.18:
     premise_name is [a-z][a-z0-9_]*, the string one plain non-blank line *)
contract_revision_marker =
    "resource_revision", "(", revision_mode, ")" ;
clause_revision_marker =
    "resource_revision", "(", resource_predicate, ",", revision_mode, ")" ;
revision_mode = "pinned" | "flexible" ;
contract_review_marker =
    "resource_review_pending", "(", review_note, ")" ;
clause_review_marker =
    "resource_review_pending", "(", resource_predicate, ",", review_note, ")" ;
review_note = STRING, [",", STRING], [",", "rewrite_before_acceptance"] ;
  (* reason, then optional owner, then the optional disposition of card
     W47-REVIEW-HOLD-CONCRETE-REASON, contract specification C06.1 *)
literal_argument_premise =
    "resource_literal_argument", "(", IDENT, ",", integer_literal, ")" ;
integer_literal = ["-"], DIGITS ;
resource_predicate = "resource_borrowed", "(", IDENT, ")"
  | ("resource_owned" | "resource_consumed" | "resource_released" | "resource_out"),
    "(", IDENT, ",", nullability, ")"
  | ("resource_stored" | "resource_removed"),
    "(", IDENT, ",", IDENT, ",", nullability, ")"
  | "resource_return_none", "(", ")"
  | "resource_return_owned", "(", nullability, ")"
  | "resource_return_removed", "(", IDENT, ",", nullability, ")"
  | "resource_return_reallocated", "(", IDENT, ",", nullability, ")"
  | "resource_no_return", "(", ")"   (* card CG21, contract spec C04.3.3 *)
  | counted_user_predicate
  | container_predicate
  | retention_predicate
  | slot_predicate
  | reference_predicate
  | callback_predicate
  | "resource_return_borrowed_from", "(", IDENT, ",", nullability, ")" ;
    (* card SLOT-MOVE-AND-INTERIOR-BORROW, contract spec C04.3.4 *)
nullability = "nullable" | "nonnull" ;
(* card W45-FB-RETRY-LIFETIME, contract specification C04.7 and C04.8 *)
counted_user_predicate =
    ("resource_user_added" | "resource_user_dropped"
     | "resource_user_dropped_checked" | "resource_user_release_check"),
    "(", IDENT, ",", IDENT, ")"              (* parameter, counter member *)
  | "resource_user_neutral", "(", ")"
  | "resource_user_exit", "(", IDENT, ",", integer_literal, ",",
    ("added" | "restored" | "dropped"), ")" ;
container_predicate =
    "resource_published", "(", IDENT, ",", IDENT, ")"   (* entry, container *)
  | "resource_unpublished", "(", IDENT, ")" ;
(* card CG23-RELATIONAL-POSTCONDITIONS, contract specification C04.13 *)
return_code_disposition =
    "resource_on_return", "(", relation_op, ",", integer_literal, ",",
    disposition_predicate, ")" ;
relation_op = "eq" | "ne" | "lt" | "le" | "gt" | "ge" ;
disposition_predicate = "resource_borrowed", "(", IDENT, ")"
  | ("resource_owned" | "resource_consumed" | "resource_released" | "resource_out"),
    "(", IDENT, ",", nullability, ")" ;
(* card CALLEE-RETENTION-CLASS1, contract specification C04.14 *)
retention_predicate =
    "resource_retained_by", "(", IDENT, ",", (IDENT | "global" | "result"), ")",  (* retained, holder *)
    [retention_guard]
  | "resource_retained_by", "(", IDENT, ",", holder_path, ")"   (* card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, C04.14.3 *)
  | "resource_holder_reset", "(", IDENT, ")" ;
holder_path = IDENT, ".", member_path ;          (* a formal of the declaration, then its members *)
(* card W43-ABI-ENUM-AND-CONST-PP, contract specification C04.14.1 *)
retention_guard = "when", IDENT, guard_op, integer_literal ;
guard_op = "==" | "<" | ">" | "<=" | ">=" ;
(* card SLOT-MOVE-AND-INTERIOR-BORROW, contract specification C04.15, C04.16 *)
slot_predicate =
    "resource_slot_moved", "(", IDENT, ",", nullability, ")"            (* slot *)
  | "resource_out_derived", "(", IDENT, ",", IDENT, ",", nullability, ")" ;  (* slot, base *)
(* card REFCOUNT-RESOURCE-KIND, contract specification C04.17 *)
reference_predicate =
    ("resource_reference_kept" | "resource_reference_dropped"), "(", IDENT, ")" ;
(* card CG22-CALLBACK-REGISTRY-CONTRACT, contract specification C04.19 *)
callback_predicate =
    "resource_callback_invoked", "(", IDENT, { ",", callback_actual }, ")"   (* fnptr, actuals *)
  | "resource_callback_table", "(", IDENT, { ",", callback_actual }, ")"     (* table, actuals *)
  | "resource_callback_registered", "(", IDENT, ",", (IDENT | "result"), ")"  (* fnptr, holder *)
  | "resource_callback_registry_invoked", "(", IDENT, { ",", (callback_actual | "element") }, ")"
  | ("resource_callback_unregistered" | "resource_elements_dropped"
     | "resource_released_if_constructed"), "(", IDENT, ")"                  (* holder *)
  | "resource_element_added", "(", IDENT, ",", IDENT, ")"                  (* element, holder *)
  | "resource_callback_member_invoked", "(", (IDENT | "supplied_owned"), ",", IDENT, ".", member_path,
    { ",", callback_actual }, ")"                                            (* object, TAG.FIELD, actuals *)
  | "resource_callback_member_registered", "(", IDENT, ",", IDENT, ".", member_path, ",", IDENT, ")"
                                                  (* OP-APP-RK-CALLBACKS-VIA-POLL-CB: holder, TAG.FIELD, fnptr *)
  | "resource_callback_member_registered", "(", "#", IDENT, ",", IDENT, ".", member_path, ",", IDENT, ")",
    "where", "#", IDENT, "=", "#", "result", "in", holder_path ;
                                                  (* FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, C04.19.3: by id *)
callback_actual = IDENT | "_" | "supplied_owned" ;
(* card CG22-OP-CALLBACKS-AND-SERVED-QUEUES, contract specification C04.19.2 *)
member_path = IDENT, { ".", IDENT } ;
discriminant = "resource_discriminant", "(", IDENT, ",", member_path, ",", integer_literal, ")" ;
arm_predicate =
    ( "resource_arm_released", "(", IDENT, ",", member_path, ")"
    | "resource_arm_empty", "(", IDENT, ")" ),
    ( "when", member_path, "==", integer_literal | "otherwise" ) ;
(* card OP-CONSTRUCTOR-DECLARATIONS, contract specification C04.19.4 *)
result_state_predicate =
    "resource_discriminant_set", "(", "result", ",", IDENT, ".", member_path, ",",
        IDENT, ",", integer_literal, ")"                    (* TAG.FIELD, P, FLAGS *)
  | "resource_callback_member_registered", "(", "result", ",", IDENT, ".", member_path, ",",
        (IDENT | "#", "absent"), ")" ;                      (* TAG.FIELD, F or a function *)
result_state_clause = "quant", "live", "#", "result",
    [ ":", IDENT, ".", member_path, MAPS_TO, ("#", "absent" | IDENT) ], ";" ;   (* G05d *)
(* card SUPPLIED-OWNED-RETURN-CODE-HANDOFF, contract specification C04.19.5 *)
handoff_clause = "quant", "live", "#", IDENT, ":", "#", "owner", MAPS_TO, "#", "callback",
    "when", "#", "result", ("==" | "!="), ("@", IDENT | "#", IDENT), ";" ;   (* G05d *)
(* card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, contract specification C04.14.3 *)
removal_clause = "quant", "live", "#", IDENT, ":", "#", "holds", "(", IDENT, ")", MAPS_TO, "#", "absent", ";" ;   (* G05d *)
(* card OP-CALLBACK-MEMBER-PATH-RELEASE, contract specification C04.19.6 *)
path_fate = "#", ("released" | "moved" | "stale" | "kept") ;
member_path_clause = "quant", "live", "#", IDENT, ":", IDENT, ".", member_path, MAPS_TO,
    ( path_fate | "{", "#", ("released" | "moved" | "stale"), ",", "#", "kept", "}" ),
    [ "when", "#", "result", ("==" | "!="), ("@", IDENT | "#", IDENT)
    (* card FB-TEARDOWN-DESTROY-DECLARATIONS, decision 2 of 2026-10-16 *)
    | "when", "#", IDENT, ("==" | "!=" | "<" | ">" | "<=" | ">="),
        ( [ "-" ], integer_literal | "@", IDENT | "#", IDENT ) ], ";" ;   (* G05d *)
(* card MK-EVENT-DESCRIPTOR-QUANT, contract specification C04.6.2; USER DECISION 2026-11-01 *)
cell_kind = "descriptor" ;
cell_value = "#", "fresh", "(", cell_kind, ")" | "#", "stale", "(", cell_kind, ")"
    | "{", "#", "stale", "(", cell_kind, ")", ",", "#", "kept", "}" | "#", "same", "(", IDENT, ")" ;
witness_guard = "when", "#", "result", ( "!=", "-", "1" | "==", "-", "1" | "==", "0" | "!=", "0" ) ;
member_equality = "when", "#", IDENT, ":", IDENT, ".", member_path, "==", "#", IDENT ;
cell_clause = "quant", "live", "#", IDENT, ":", ( IDENT, ".", member_path | "#", "pointee" ), MAPS_TO,
    cell_value, [ witness_guard ], ";" ;   (* G05d *)
guarded_release = "ensures", "normal", "resource_released", "(", IDENT, ",", "valid", ")",
    member_equality, ";" ;   (* G05 *)
guarded_removal = "quant", "live", "#", IDENT, ":", "#", "holds", "(", IDENT, ")", MAPS_TO, "#", "absent",
    member_equality, ";" ;   (* G05d *)
member_cell_predicate = "resource_member_kind", "(", IDENT, ",", IDENT, ".", member_path, ",", cell_kind, ")"
    | "resource_member_released", "(", IDENT, ",", IDENT, ".", member_path, ",", "stale", ")"
    | "resource_on_return", "(", ( "eq" | "ne" ), ",", ( "0" | "-", "1" ), ",",
        ( "resource_member_out" | "resource_member_stale_or_kept" ), "(", IDENT, ",", IDENT, ".", member_path, ")"
        | "resource_member_alias", "(", IDENT, ",", IDENT, ".", member_path, ",", IDENT, ")", ")" ;   (* G05 *)
```

The descriptor cells of card MK-EVENT-DESCRIPTOR-QUANT (contract specification C04.6.2; USER DIRECTION 2026-11-01, "We must decompose the premise into quant like composed clauses."; USER DECISION 2026-11-01, "I approve the changes of D1-D5.") carry static conditions of their own. A `cell_clause`'s binder names a depth-one pointer parameter, `resource_borrowed`, with no kind (`quant-live-cell-binder`); with `#pointee` it is a `ptr<mut, u8>` slot. Its `cell_kind` is `descriptor` (`quant-live-cell-kind`); a `#fresh` and a `#same` value take a `witness_guard` (`quant-live-cell-guard`), every guard of the declaration is of one pair (`!= -1` / `== -1` or `== 0` / `!= 0`) over a signed integer result with `resource_return_none()`, and a cell is stated for every return or per class, one state per class (`quant-live-cell-duplicate`); `#same(S)` names a slot stated `#fresh` in the same class (`quant-live-cell-value`). Every cell has one kind, from its values or a `resource_member_kind` / `resource_kind` clause, and a kind clause names a stated cell. The lower level (`member_cell_predicate`, and a slot's `resource_on_return(eq, 0, resource_out(S, nonnull))`, `resource_on_return(ne, 0, resource_borrowed(S))` and `resource_kind(S, descriptor)`) elaborates to the same canonical set as the shorthand; an agreeing overlap merges and a disagreeing one rejects. A `member_equality` names a borrowed depth-one pointer and an `i32` formal released under it; a declaration states one, a `guarded_release`'s formal states `resource_kind(X, descriptor)` and no other mode, and a guarded removal guards every removal of its declaration (`quant-live-cell-equality`). Cells stand beside retentions, removals and plain predicates only, and withhold the declaration from the function map; `fresh`, `same` and `pointee` join the reserved keywords of a `#` form. Every violation rejects the whole module.

The hand-off and its surface (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF, contract specification C04.19.5; USER DECISIONS 2026-10-15) carry static conditions of their own. `#` marks binders and value keywords (`#absent`, `#owner`, `#callback`), `@IDENT` a C enumerator; the lexer reads `@` as one symbol (`Symbol::At`; before the card a lexer error, and the census found no `@` and no `alias` in code position in any tracked source file). An `alias_declaration` stands only in a contract module (an explicit executable or library header is a parse error; a headerless staged view is checked against the file's own mode), once per name, its name no reserved keyword (`absent`, `owner`, `callback`, `result`) and no parameter of any declaration in the module (`alias-collision`); it is part of the digest of every declaration spelling `#NAME`. A `handoff_clause`'s binder names a depth-one pointer parameter that the declaration states `resource_owned(P, nonnull)` (`quant-live-unbacked` otherwise), its constant an `@` name or a module alias (`quant-live-handoff-alias` for an undeclared one; a number, a bare word, an ordering and any other shape: `quant-live-handoff-constant`, `-guard`, `-owner`, `-binder`, `-shape`); the result is an integer or enumeration type with `resource_return_none()` (`quant-live-handoff-result`); one per declaration. A declaration carrying any `#` form (a hand-off or a result state) rejects a parameter named like a reserved keyword. A member invocation's object may be the handed formal (`resource_owned`), never another owned formal. The clause stands where a callback predicate may and withholds the declaration from the function map. `#absent` is the one spelling of the result state's value at both levels; a bare `absent` rejects naming `#absent` (`quant-live-absent`). An `@` name's resolution (an enumerator of the candidate's own result type) is Clang's, per unit and after the pinned route's pre-pass, never the loader's (C04.19.5). Every violation rejects the whole module.

The holder paths, removals and id-forwarded registrations of card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (contract specification C04.14.3, C04.19.3; USER DECISIONS 2026-10-16, "Tracked holder path", "R3: quant state", "Result + list") carry static conditions of their own. A `holder_path` stands only as a retention's holder: its first `IDENT` names a depth-one pointer parameter of the declaration, `resource_borrowed`, with no kind (it may be the retained parameter); a record tag there is an unknown parameter and rejects the module; it takes no `retention_guard`; a parameter is retained at most once across both holder forms. A `removal_clause`'s binder and its `#holds` operand name distinct depth-one pointer parameters, `resource_borrowed`, with no kind; one removal per pair, never beside a retention of the same pair; a parameter named `holds` or `absent` rejects the module beside one; any other shape after `#holds` (another value, `->`, a second operand) rejects (`quant-live-holds-shape`). An id-forwarded registration's first operand `#IDENT` is bound by its own `where` clause to `#result` (another binder or value rejects) over an integer result with `resource_return_none()`, the binder naming no parameter; `in` takes a `holder_path` whose formal is a depth-one pointer parameter, `resource_borrowed`, with no kind; its `fnptr` and member follow the setter form's conditions. All three stand where a retention may and withhold the declaration from the function map. Every violation rejects the whole module.

The member-path states of card OP-CALLBACK-MEMBER-PATH-RELEASE (contract specification C04.19.6; USER DECISION 2026-10-15, "B: state + set") carry static conditions of their own: the binder names a depth-one pointer parameter stated `resource_borrowed` or (card FB-TEARDOWN-DESTROY-DECLARATIONS, a destroyer's own object) `resource_released` with no kind, or the formal the declaration's hand-off hands (`resource_owned(P, nonnull)`) (`quant-live-path-binder`); the left of `|->` is a dotted `TAG.FIELD` (`quant-live-path-member`; a bare `->` rejects naming `|->`, `quant-live-maps-to`); the right is one of the four value keywords or exactly the set `{V, #kept}` with `V` one of `#released`, `#moved`, `#stale` (any other set, `{#kept, V}` included, rejects: `quant-live-path-set`; any other value, `#absent` included: `quant-live-path-value`); a result guard stands only beside a hand-off of the same formal and is its complement, the same constant with the other operator (`quant-live-path-guard`); a parameter guard (card FB-TEARDOWN-DESTROY-DECLARATIONS, contract specification C04.19.6 amended; USER DECISION 2026-10-16, "Yes, allow it") names a plain integer parameter other than the binder (`i8` through `i64`, `isize`, `u8` through `u64`, `usize`) with one of the six comparisons and a canonical decimal in that parameter's range that partitions it, an `@` enumerator or a module alias (`quant-live-path-guard`, `quant-live-path-guard-constant`, `quant-live-handoff-alias`), and stands with or without a hand-off; one state per path of a formal, never a path below another stated one (`quant-live-path-duplicate`). The keywords `released`, `moved`, `stale` and `kept` join the reserved keywords of a `#` form (a parameter or an alias so named rejects). The clauses stand only beside a hand-off and plain predicates (a retention, a callback or arm clause, a result state, a slot or reference clause beside them rejects) and withhold the declaration from the function map. Every violation rejects the whole module.

The result-state predicates of card OP-CONSTRUCTOR-DECLARATIONS (contract specification C04.19.4; USER DECISIONS 2026-10-14, A1, B1 and C) carry static conditions of their own: the object is the word `result`, which requires `resource_return_owned(nonnull)` (`quant-live-unbacked`); a parameter named `result`, `absent`, `owner` or `callback` rejects the module (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the reserved keywords of a `#` form); the member is a dotted `TAG.FIELD`; `resource_discriminant_set`'s `P` names an integer parameter and its `FLAGS` is a canonical non-negative decimal, at most one per declaration; `resource_callback_member_registered`'s `F` names a `fnptr` parameter whose mode is `resource_borrowed` and which no other clause of the declaration invokes or registers, or is the value keyword `#absent` (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF; a bare `absent` rejects), or (card OP-APP-RK-CALLBACKS-VIA-POLL-CB, USER DECISION 2026-10-14, D1) is an identifier naming no parameter, a function designator, the stated default; the same word with a parameter in place of `result` (card OP-APP-RK-CALLBACKS-VIA-POLL-CB, Q2) is a setter's registration, read with the callback predicates: the holder names a depth-one pointer parameter, `resource_borrowed`, with no kind, `F` a `fnptr` parameter no other clause names, one registration per holder and member, and `result` there is the result state above; a member carries one state (an agreeing overlap merges, a different second state rejects). `result_state_clause` and (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF) `handoff_clause` are the only `quant` forms resource v1 admits (G05d, "Result states"). They stand where a callback predicate may and withhold the declaration from the function map. Every violation rejects the whole module.

The callback predicates (card CG22-CALLBACK-REGISTRY-CONTRACT, contract specification C04.19) carry static conditions of their own: the first `IDENT` of `resource_callback_invoked` and `resource_callback_registered` names a `fnptr` parameter (whose only mode is `resource_borrowed`), and one callback is named by at most one invocation or registration; a table, a holder and an element name depth-one pointer parameters (`ptr<mut, u8>` or `ptr<const, u8>`) whose mode is `resource_borrowed` and which carry no kind; a `callback_actual` `IDENT` names a parameter of the declaration other than the invoked callback, `_` a value the callee supplies, and `element` (a registry invocation only) each element the holder holds; a declaration with a parameter named `_` or `element` rejects every callback predicate, and one with a parameter named `result` rejects a `result` holder, which further requires `resource_return_owned(nonnull)`; an element is not its own holder and is never also a retained pointer; a declaration carries at most one predicate of each keyword per position, and never both a registration in a parameter holder and a registry predicate on the same holder; and a callback predicate stands where a retention predicate may, beside retention predicates, but never beside a slot or a reference predicate. Every violation rejects the whole module. A declaration carrying one is withheld from the function map with the retaining declarations (C04.14).

The predicates of card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (contract specification C04.19.1-C04.19.3; USER DECISION 2026-10-14) carry static conditions of their own. `supplied_owned` is an actual of `resource_callback_invoked` and `resource_callback_member_invoked` only, at most once per actual list; a declaration with a parameter named `supplied_owned` rejects every callback predicate. A member invocation's object is a depth-one pointer parameter with no kind whose mode is `resource_borrowed` or `resource_released`, or `supplied_owned` (only where some clause of the declaration hands a callback that object); its member is `TAG.FIELD` with at least one `.`; one clause per object and member. A `discriminant` names a depth-one pointer parameter with no kind that the declaration releases, a member path and a canonical decimal mask; a declaration carries at most one, and exactly when it carries arm predicates. Each arm names the discriminated parameter; its guard is `when FIELD == VALUE` with the discriminant's member path and a canonical decimal `VALUE` (`==` only, no name, no value the mask clears) or `otherwise`; a guard carries one `resource_arm_empty` or released paths, never both and never one path twice, and a path is never the discriminant itself. Arm predicates and the discriminant stand where a callback predicate may and withhold the declaration from the function map. Every violation rejects the whole module.

The reference predicates (card REFCOUNT-RESOURCE-KIND, contract specification C04.17) carry static conditions of their own: the named parameter is a depth-one pointer (`ptr<mut, u8>` or `ptr<const, u8>`) whose mode is `resource_borrowed` and which carries no kind; a declaration carries at most one reference predicate; a keep's result is `resource_return_alias(p, nonnull)` on the same parameter or `resource_return_none()`, a drop's is `resource_return_none()`; the referenced parameter is never also retained, a slot or a derived out slot's base; a reference predicate stands where a retention predicate may; and the kind `reference` is spelled by `resource_return_kind(reference)` only beside `resource_return_owned` (a returned reference) and never by `resource_kind` on a parameter (the module is rejected). Every violation rejects the whole module. A declaration carrying one is withheld from the function map with the retaining declarations (C04.14).

The slot predicates and the borrowed result (card SLOT-MOVE-AND-INTERIOR-BORROW, contract specification C04.3.4, C04.15, C04.16) carry static conditions of their own: a moved slot names a `ptr<mut, ptr<mut, u8>>` parameter, a derived out slot a `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>` parameter, each `resource_borrowed` with no kind; a derived out slot's base is a distinct depth-one pointer parameter, `resource_borrowed`, with no kind; a slot is named by at most one slot predicate and is never also a retained pointer; a slot predicate stands where a retention predicate may (beside plain parameter and result predicates, revision markers and retention predicates), and a declaration carrying one is withheld from the function map with the retaining declarations (C04.14). `resource_return_borrowed_from(p, n)` loads to the derived result at the `Borrowed` extent: `p` names a depth-one `resource_borrowed` pointer parameter and the result is a depth-one pointer or `ptr<mut, ptr<mut, u8>>` / `ptr<mut, ptr<const, u8>>` (an array of the parent's children); every other placement rejects the module. A slot predicate inside `resource_on_return` is not admitted (an unsupported predicate). Every violation rejects the whole module.

A `retention_guard` (card W43-ABI-ENUM-AND-CONST-PP, contract specification C04.14.1) stands only on `resource_retained_by` (a guard on any other predicate, and a second `when`, reject the module; a clause-level revision marker wraps a guarded clause as it wraps a bare one); `when` is reserved in a clause expression. Its `IDENT` names an integer parameter of the declaration (`i8` through `i64`, `isize`, `u8` through `u64`, `usize`; never `bool`, a float, a pointer, `fnptr` or `va_list`) other than the clause's retained pointer and holder; its `integer_literal` follows the canonical-literal rule below and lies in that parameter's range; and the guard partitions the range (some value satisfies it and some does not). `!=` and Boolean combinations are not admitted. Every violation rejects the whole module.

The retention predicates (card CALLEE-RETENTION-CLASS1, contract specification C04.14) carry static conditions of their own: the retained parameter and a holder parameter are distinct depth-one pointer parameters (`ptr<mut, u8>` or `ptr<const, u8>`) whose mode is `resource_borrowed` and which carry no kind; the holder is either such a parameter, the word `global`, or (card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, contract specification C04.14.2) the word `result`, which requires the declaration's result to be `resource_return_owned(nonnull)` (a heap object or, with `resource_return_kind(reference)`, a returned reference); a declaration with a parameter named `global` rejects the clause, and one with a parameter named `result` rejects a `result` holder; a parameter is retained at most once and a holder reset at most once; and a retention predicate stands only beside plain parameter and result predicates and revision markers (a return-code disposition, a review marker, the checked-specification marker, a literal premise, a variadic tail, the no-return exit and a counted-user, container, loop-owner, thread-synchronization or lifecycle clause each reject the module beside it). Every violation rejects the whole module. A declaration carrying one is loaded and validated in full and then withheld from the function map (C04.14); inheritance (G05a) compares its retentions.

The return-code dispositions (card CG23-RELATIONAL-POSTCONDITIONS, contract specification C04.13) carry static conditions of their own: the distinct `(relation_op, integer_literal)` pairs are pairwise disjoint and cover every integer; the literal is canonical and fits a C `int`; a parameter carries either one ordinary mode or exactly one disposition per class; the entry requirement of a conditional parameter agrees across classes; the result is `resource_return_none()` over a signed integer type; classes that all state the same contract, a disposition beside an in-place kind, and a disposition beside a review marker, the checked-specification marker, a literal premise, a variadic tail or a counted-user, container, loop-owner, thread-synchronization or lifecycle clause, reject the module. Every violation rejects the whole module.

The counted-user and container predicates (card W45-FB-RETRY-LIFETIME) carry static conditions of their own: every parameter they name is a `resource_borrowed` pointer parameter; a publication's container is a distinct parameter with no container predicate of its own; a parameter carries at most one counted-user operation and at most one container predicate; `resource_user_neutral()` appears at most once and never beside another counted-user predicate; an exit clause requires an integer result type, its `integer_literal` follows the canonical-literal rule above, one parameter and returned value carry at most one clause, and a parameter carries either an operation or exit clauses, never both. Every violation rejects the whole module.

Additional static conditions are mandatory: exactly one marker, exactly one mode per parameter, exactly one result mode, unique names, compatible pointer/scalar types, valid distinct owned parent references, an owned reallocation source, and — for `literal_argument_premise` — a parameter name the same declaration declares (of any admitted type, pointer included: see contract specification C04.5), at most one premise per parameter, and a `DIGITS` run the lexer produces as a single integer token with no leading zero, which excludes suffixed, alternate-base and octal spellings; `-0` is likewise rejected, so the declared literal is canonical. These conditions cannot be represented by context-free productions alone. Signature usage annotations, extra declarations, unsupported predicates and other exit selectors are rejected atomically, not ignored. See contract specification C04–C06 and `crates/qcore-contract/src/contract_loader.rs`.

The whole-declaration tail clauses `resource_variadic_data_tail()` and (card CG26-VARIADICS) `resource_variadic_borrowed_tail()` take no argument; a declaration carries at most one of the two (a second, or both, rejects the module), and either requires the analysed C declaration to be variadic (contract specification C04.1, C04.2).

`resource_no_return()` (card CG21-CALL-EXITS-AND-BOUNDARIES) appears at most once, only on a `unit` result with `resource_return_none()` and `resource_borrowed` on every parameter, and never beside a counted-user, container, loop-owner, thread-synchronization or lifecycle clause; a declaration of a name in `UNDECLARABLE_C_NAMES` (`contract_loader.rs`: the Clang extension nodes spelled like calls, the returns-twice calls and the non-local exits) is rejected whatever its clauses. See contract specification C04.3.3.

`literal_argument_premise` (card HASH-CLAUSE) states where the declaration is applied, not what it means: a consumer treats the callee as undeclared at every call site whose actual at the named parameter is not exactly that integer constant. It never widens a declaration and never changes a mode. Which call-site *spellings* are that constant is fixed by contract specification C04.5.1 (card HASH-REFINE): matched enclosing parentheses and an integer suffix are erased structurally, and every spelling needing arithmetic, a base, a cast or symbol resolution is refused.

The review markers (card W45-CONTRACT-DEFERRED-REVIEW, contract specification C06) carry their own static conditions. The first `STRING` is the mandatory reason and the second the optional reviewer or owner; each is one lexer string literal, not blank, with no backslash escape and no control character, so each has one spelling. A declaration carries either one `contract_review_marker` or any number of `clause_review_marker`s, never both and never two contract-level markers. A `clause_review_marker` wraps exactly one `resource_predicate` (the kind clauses and `resource_variadic_data_tail()` included), which is then subject to every condition above as if written bare; it cannot wrap a marker, the `resource_contract_v1` selector or a `literal_argument_premise`, and it is admitted only under `ensures normal`, as the contract-level form is only under `requires`. Every violation rejects the whole module. A marked declaration is loaded and validated in full and then withheld (C06); inheritance (G05a, C15) copies its markers with its clauses.

The revision markers (card W45-CONTRACT-REVISION-MODE, contract specification C06.2) have their own static conditions. A declaration carries at most one `contract_revision_marker`; a second one is a duplicate (the same mode) or a conflict (the other mode). A `clause_revision_marker` wraps exactly one `resource_predicate`, read as if written bare, and may not restate the contract-level marker's mode. It cannot wrap a marker of either kind, and a `clause_review_marker` cannot wrap it; it cannot wrap the selector or a `literal_argument_premise`, which take the contract's mode; and it is admitted only under `ensures normal`, as the contract-level form is only under `requires`. A `revision_mode` other than `pinned` or `flexible` is malformed. Every violation rejects the whole module. With no marker a declaration is pinned. Inheritance (G05a, C15) does not copy a parent's revision markers: elaboration drops the contract-level form and unwraps the clause-level one.

The named premise `requires resource_premise(NAME, "STATEMENT");` (card VERDICT-CONTRACT-DEPENDENCIES, contract specification C04.18) names an assumption the declaration rests on: `NAME` a lowercase identifier, the statement one plain, non-blank string with no escape and no control character, each name at most once per declaration; any other shape, a repeated name and an `ensures` spelling each reject the whole module. It changes no mode, is rederived from the retained texts (`declaration_premises`), is part of the declaration's text and so of its contract digest and pin, and inheritance (G05a) copies it, compared by the agreement check. Since card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER the shape check belongs to the contract format: `crates/qcore-frontend/src/frontend.rs` (`premise_clause`) refuses, at parse, any other shape, an introducer other than `requires` and a repeated name in one declaration, in every module. A resource-v1, bounds-v1 (G05f, beside a member clause of any kind only) or Quirin model contract is rejected whole. A model contract's premises are read by `contract_loader::module_premises`.

The conjunction-only graph extension (CONTRACT-HYPOTHESIS-GRAPH-V1, C04.18.1) admits `requires resource_hypothesis(ID, STRING);` and `requires resource_premise_all(NAME, ID {, ID});` in resource-v1 declarations. Both heads and IDs remain ordinary `Word` tokens: the reserved-word census adds zero words and zero lexer token variants. Statements use C04.18's plain-string restrictions. IDs are lowercase identifiers, with `legacy_` reserved. Conjunction references are nonempty, unique and strictly increasing; only atom references are admitted. Unused/missing/conflicting atoms and any typed predicate reject the whole module. Equal duplicate atoms coalesce. There is no `any` or `not` grammar and no graph clause under a scoped S3 prerequisite or FFI vocabulary. The bounds-v1 exception is the separately bounded same-name extension below; `resource_premise_all` remains unavailable there. Inheritance preserves graph symbols, independent of formal renaming. All graph records remain evidence-only; C04.18.1 defines exact context, digest, replay and erasure rules.

C04.18.2 additionally admits the ordinary-word contextual head `requires resource_premise_decomposition(EXISTING_NAME, ATOM {, ATOM});`. It resolves exactly one declared legacy premise in the same declaration, creates no named premise, and retains that premise's exact singleton/erasure. Only nonlegacy atom children are admitted; malformed, repeated, missing or noncanonical attachment data reject. Nonempty attachment maps use graph2; unchanged v1 bytes/digests remain canonical. Proposed conjunction composition is separate from original proposition truth. The plain module-comment binding marker and canonical explicitly supplied manifest are metadata outside the expression grammar, with strict version/content-address/reference checks under C04.18.2. No operators, reserved words or typed predicates are added. The PLUGIN-PREMISE-DECOMPOSITION extension also admits the same atom and same-name attachment heads in bounds-v1 declarations. The complete bounds parser still validates every ordinary clause; graph metadata introduces no interpreted BoundsClause or solver fact, and `resource_premise_all` remains refused there. Replay uses a separately held bounds/null source view and retains exact original premise erasure and semantic clause identity. Resource loading continues to refuse bounds vocabulary. Explicit manifest integrity uses that property-specific replay; a null row can attach only through its actual consulted member-fact/premise dependency records. The declared resource-type model view is replayed separately and makes no C ABI claim. Unattached graphs retain exact canonical v1 bytes; a nonempty proposed refinement uses v2 and older v1-only readers refuse it. The extension reuses schema20 graph/binding records without adding fields or operators.

The checked-specification marker `requires resource_body_checked;` (card W45-FB-BACKLOG-CUSTODY, contract specification C06.3) takes no argument, occurs at most once and only as a contract-level requirement; a second one, an argument, and an `ensures` spelling each reject the whole module. It changes no mode: the loaded contract is the one the same declaration without it loads, and which declarations carry it is rederived from the retained texts (`declared_checked_specifications`), as the variadic tail and the literal premise are. Inheritance (G05a) copies it with the other clauses, and the inherited-declaration agreement check compares it. It composes with the revision and review markers: it is read after they are peeled, neither may wrap it, and a declaration a review marker withholds, or an unpinned declaration out of force, is never a specification.

The grammar describes tokens, not required whitespace. Apart from the one `clause_review_marker`, `clause_revision_marker` and `return_code_disposition` wrappings, it provides no general predicate-call nesting. It also does not turn an admitted trusted declaration into proof that a C function obeys it.

`resource_return_removed(item, n)` (C04.3.5) names a matching
`resource_removed(item, parent, n)` parameter and a distinct nonnull owned
parent, with matching nullability and mutable depth-one heap pointer types.
Every other parameter is borrowed; richer effect/clause families and variadic
tails are refused. The result retains the item's identity. The physical-body
reader additionally requires the exact explicit `detach_list_representation`
premise and complete bounded list layout/unlink frame; neither its syntax nor
its premise establishes caller membership.


### G05-flow. Proposed premise expressions

C04.18.3 adds one contextual clause, also admitted in the explicit bounds and resource-type model views after full vocabulary validation:

```ebnf
premise_flow = "resource_premise_flow_v1", "(", flow_id, ",", flow_expr, ")" ;
flow_expr = atom_id | "flow", "(", flow_id, ")"
          | "(", flow_expr, flow_operator, flow_expr, ")" ;
flow_operator = "|->" | "||->" | "!->" | "->|" ;
```

IDs match `[a-z][a-z0-9_]*`. Every binary operation requires its own parentheses; no precedence or associativity is inferred. `|->` reuses MapsTo only inside this clause; elsewhere its state-clause meaning is unchanged. The lexer uses longest matching for three additional symbols `||->`, `!->`, `->|`; the contextual heads remain ordinary words. `flow(ID)` resolves only a same-declaration entry; atom IDs resolve only existing graph atoms. Full reference validation rejects missing IDs, duplicate entries, cycles, excessive expansion and terminal composition below the root, including through references. Every branch and entry is checked. The exact limits, three-valued truth tables, positive-refutation requirement, explicit discharged terminal condition, source/model/Context replay and evidence-only authority boundary are C04.18.3. These records add no graph field or premise, and inherited graph/flow identifiers are literal metadata independent of formal parameter renaming. The source-language expression grammar, scoped S3 and FFI vocabularies do not gain general flow expressions.


### G05-lifecycle. Typed symbolic lifecycle clauses

C04.18.4 admits the following source-version1 metadata through the existing resource, bounds and explicit resource-type-model views:

```ebnf
lifecycle_clause = "resource_lifecycle_v1", "(", entry_id, ",",
                   lifecycle_operator, lifecycle_target, ")" ;
lifecycle_operator = "|^+" | "|^-" ;
lifecycle_target = ("resource" | "hypothesis"), "(", target_id, ")" ;
```

The enclosing source clause is `requires lifecycle_clause;`. Operators are atomic by longest match and remain unsupported in ordinary expressions, flow-v1 expressions and other contract vocabularies. No untyped operand, target path, nested lifecycle expression or trailing token is admitted. IDs are literal metadata with the exact bounds/reference/inheritance rules of C04.18.4. These forms retain typed symbolic intents only; they introduce no executable lifecycle or proof authority. Existing maps-to, flow, pipe and arithmetic spellings keep their identities.

### G05-guarded. Standalone readable guarded decomposition

C04.18.5 defines this separate version1 document. It is **not** an ordinary
source program or a `.qv` contract clause; `.qgd` is used for the two explicit
report-owned applications. No source-language word or lexer token is added.

```ebnf
guarded_document = "guarded_decomposition_v1", ";", guarded_clause, { guarded_clause } ;
guarded_clause = "(", environment_id, "∧", prerequisite_id,
                   { "∧", prerequisite_id }, ")", "|->", operation_id, ";"
               | condition_id, "!->", operation_id, ";"
               | operation_id, "||->", case_id, "/", case_id, { "/", case_id }, ";"
               | case_id, "->|", postcondition_id, ";" ;
```

IDs match `[a-z][a-z0-9_]*`. Only surrounding whitespace is insignificant;
operator spellings are atomic, and the conjunction symbol is exactly `∧`.
Every symbol is resolved against an explicitly supplied typed proposition map
bound to a fully replayed property-specific source graph. No nesting, precedence,
flow reference, lifecycle operation, implicit case or trailing token is supplied.
All operations/cases must be completely paired with guards, alternatives and
separate terminals. Exact role/reference/size limits, admission-direction
lowering, positive-refutation and Context checks are C04.18.5. Existing G05-flow,
G05-lifecycle and maps-to meanings are unchanged.

### G05-guarded-source. Explicit guarded source embedding

C04.18.6 extends the contract metadata vocabulary explicitly. Existing contextual
flow-v1/lifecycle-v1 expressions retain their meanings. The ordinary lexer
retains the raw content of a quoted multiline literal; this extension refuses
backslash escapes rather than applying JSON/C escape decoding.

```ebnf
guarded_symbol = "requires", "resource_guarded_symbol_v1", "(", ID, ",", role,
                 ",", RAW_STRING, { ",", graph_atom_id }, ")", ";" ;
guarded_source = "requires", "resource_guarded_decomposition_v1", "(", entry_id,
                 ",", RAW_GUARDED_DOCUMENT_STRING, ")", ";" ;
role = "environment" | "prerequisite" | "operation" | "condition" | "case" | "postcondition" ;
```

`RAW_GUARDED_DOCUMENT_STRING` contains the complete G05-guarded versioned document,
including its final semicolon. One document and its complete role/proposition map
are required per participating declaration. The source clause limit is131,072
bytes; embedded document/statement/identifier/symbol/dependency bounds remain
C04.18.5's. Every `resource_guarded_` head is reserved to this total-or-reject
reader, including unsupported versions. No executable lifecycle or source truth
is introduced. The sealed bridge version2 is separately bounded to one resource
module/declaration; it is not a general source eraser.

## G05a. Contract inclusion and inheritance

Card CONTRACT-INHERIT. A directive layer read by
`crates/qcore-frontend/src/contract_inclusion.rs` and resolved by
`crates/qcore-contract/src/contract_inclusion.rs` **before** any consumer
parses the module. The structural parser of G01 does not admit either form:
it refuses them with a `Q000` parse error naming the contract loader, so an
unresolved inclusion never reaches a consumer.

```ebnf
contract_module   = "module", "contract", ";", {include_directive},
                    declaration, {declaration} ;
include_directive = "#", "include", STRING, ";" ;
(* STRING content: CRATE ":" MODULE, with
   CRATE  = [a-z0-9][a-z0-9_-]*                       (the Cargo package name)
   MODULE = SEGMENT {"/" SEGMENT} (".qv" | ".qcore"), SEGMENT = [a-z0-9_-]+ *)
inheriting_fn     = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
                    "->", TYPE, "contract", "inherit", parent, ";" ;
parent            = [IDENT, "::"], IDENT ;            (* one qualifier level *)
```

`#` is the existing attribute-marker token and `::` the existing `ColonColon`
token; `include` and `inherit` are **contextual** (read only after `#` at a
contract module's top level and only directly after `contract`), so no
reserved word is added and both remain ordinary identifiers elsewhere. The
constraint-14 census found no `#` in code position, no `#include`, no
`inherit` and no non-block `contract` tail in any of the 571 tracked Quirin
sources, archived probes included.

Static conditions (each a `QI` refusal, contract specification C15): the
directives precede every declaration of a `module contract;` module; the
included crate is the includer's crate or a declared dependency and the module
is in its published set; no include cycle or repeated include; the parent is
the one own declaration named `NAME`, or the one declaration named `NAME` in
the directly included modules of the crate whose namespace is `NS`; the parent
is a plain `extern fn … contract { … };`; arity, parameter types and result
type agree; the positional renaming captures no word; no inheritance cycle; no
clause after `inherit PARENT`. The elaborated text replaces each directive by
blanks and each `inherit PARENT` by the parent's clause block under the
renaming; resource-v1 then loads it through G05 unchanged and additionally
requires the child's loaded record to equal the parent's.

The include string carries the **package name** (`quirin-provider-libc`), which
the dependency rule checks; the qualifier carries the **namespace** derived from
it (`libc`: the package name without `quirin-provider-`, `-` read as `_`). Card
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING admits exactly one second prefix,
`qcore-provider-` (`qcore-provider-libevent` → `libevent`, so Monkey's
`mk_mem_alloc` is written `libevent::mk_mem_alloc`); one remainder under both
prefixes is one namespace and is refused (`QI017`).

Card CONTRACT-INHERIT-ADOPTION adds the selection form. A schema-2 resource
manifest (`cycle19-resource-contracts.json`) is JSON, not QCore source:

```ebnf
manifest = "{", '"schema_version": 2', ",", '"load_path": ', PATH, ",",
           '"modules": [', entry, {",", entry}, "]", "}" ;
entry    = PATH | REFERENCE ;          (* a REFERENCE contains ":"; a PATH may not *)
REFERENCE = CRATE ":" MODULE ;         (* exactly the include target above *)
```

`PATH` is relative with no traversal and resolves as in schema 1; the load path
is the generated `providers/contract-load-path.json`. Schema 1 (paths only, no
`load_path`) is unchanged. The first adopting modules are
`fluent_bit/resource_memory.qv` (`flb_malloc`, `flb_calloc`, `flb_free`
inherit `libc::malloc`, `libc::calloc`, `libc::free`) and
`fluent_bit/resource_string.qv` (`flb_strdup`, `flb_strndup` inherit
`libc::strdup`, `libc::strndup`).

## G05b. Bounds contract v1

Card CG16-INDEX-AND-OFFSET-EXTENTS, contract specification
[C09.1](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned).
A separate consumer (`crates/qcore-contract/src/bounds_contract.rs`) over the
G01 envelope, selected by its own marker, loaded total-or-reject:

```ebnf
bounds_module = "module", "contract", ";", bounds_fn, {bounds_fn} ;
bounds_fn     = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
                "->", result_abi, "contract", "{", bounds_clause,
                {bounds_clause}, "}", ";" ;
bounds_clause = "requires", "bounds_contract_v1", ";"
              | "requires", bounds_pre, ";"
              | "requires", "bounds_review_pending", "(", review_note, ")", ";"
              | "requires", "bounds_review_pending", "(", bounds_pre, ",", review_note, ")", ";"
              | "ensures", "normal", bounds_post, ";"
              | "ensures", "normal", "bounds_review_pending", "(", bounds_post, ",", review_note, ")", ";" ;
bounds_pre    = "bounds_extent", "(", IDENT, ",", count, ",", unit, ")"
              | "bounds_index", "(", IDENT, ",", count, ")"
              | "bounds_global_extent", "(", IDENT, ",", DIGITS, ",", unit, ")" ;
bounds_post   = "bounds_return_extent", "(", count, ",", unit, ")"
              | "bounds_preserves_extents", "(", ")"
              | "bounds_frame_reads", "(", IDENT, ")"
              | "bounds_frame_writes", "(", IDENT, ")"
              | "bounds_frame_moves", "(", IDENT, ")"
              | "bounds_return_derived", "(", IDENT, ",", count, ",", count, ",", unit, ")" ;
count         = IDENT | DIGITS ;
unit          = "elements" | "bytes" ;
```

Card CG24-ALIAS-EXTENT-FRAME-VOCABULARY (gateway rule
[11e](agents/gateway.md#constraint-11e)) added the four `bounds_frame_*` and
`bounds_return_derived` postconditions and the *slot* parameter type
`ptr<mut, ptr<mut, u8>>` (a C `T **`), under the same selector; every text
CG16 accepted keeps its meaning. Their static conditions, each rejecting the
whole module: a frame clause names a pointer or slot parameter; a declaration
that states any frame states exactly one for **every** pointer and slot
parameter (a closed frame), and a slot parameter appears only in a frame
clause (never in `bounds_extent` or a relation), so a declaration with a slot
states a frame; `bounds_preserves_extents()` and a `bounds_frame_moves` exclude
each other; `bounds_return_derived` names a pointer parameter as its base, its
offset and length are `count`s, it needs a pointer result, it excludes
`bounds_return_extent`, its base is not declared `bounds_frame_moves`, and
there is at most one relation per base (several bases make a multi-base
relation).

`parameter`, `result_abi` and `review_note` are G05's. Static conditions,
each rejecting the whole module: exactly one `bounds_contract_v1` per
declaration; `bounds_extent` names a pointer parameter and `bounds_index` a
scalar one; a `count` IDENT names a scalar parameter of the same declaration,
and `DIGITS` is canonical (no sign, suffix, base or leading zero);
`bounds_global_extent` names a C object, not a parameter;
`bounds_return_extent` needs a pointer result; at most one clause per
parameter, object, return and `bounds_preserves_extents`; one contract-level
marker, or clause-level markers, never both, never a marker around a marker
or around the selector; only `requires` and `ensures normal`. No inclusion or
inheritance (G05a) exists for this vocabulary. Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS
(gateway rule [14j](agents/gateway.md#constraint-14j)) keeps it so: the loader
refuses `#include` and `contract inherit`, the generated bounds view leaves
such a module out, and a load path naming one in a bounds view refuses the
selection. Card QUANT-INDEX-CHECKS adds the
quantified index check [G05c](#g05c-quantified-index-checks). The meanings are C09.1's;
`elements` counts the C pointee type the audited declaration states.
`resource_*` and `bounds_*` predicates are disjoint, so each loader rejects the
other's modules. The reserved words were absent from every tracked contract
module when they were introduced (the census test
`the_reserved_spellings_are_unused_by_tracked_contracts`). Card
CHUNKIO-META-LEN-BOUNDS-CONTRACTS (gateway rule
[12r](agents/gateway.md#constraint-12r)) ships the first tracked bounds
module, `providers/qcore-provider-chunkio/contracts/chunkio/meta_len_bounds.qv`,
in its crate's `bounds_view` on the load path (never its resource view); the
census now exempts a module the bounds loader accepts and still finds the
words in no other tracked module. A bounds module is a `.qv` contract module
like any other, so the compiler's frontend gate also reads it: the `&&` and
`||` of a G05c predicate are the predicate's own connectives, admitted by the
operator gate (`Q038`) inside a `quant checks` clause and nowhere else in a
contract module (before the card a tracked quant check failed the contract
corpus's compiler regression on its `&&`). Pins: C06.2's lock format, in the
crate's `bounds-contracts.lock`.

## G05c. Quantified index checks

Card QUANT-INDEX-CHECKS, contract specification
[C09.2](QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks),
gateway rule [12l](agents/gateway.md#constraint-12l). One more `bounds_clause`
of G05b, read by `crates/qcore-contract/src/bounds_quant.rs` (decisions:
`qcore_core::index_quant`):

```ebnf
bounds_clause = … | quant_check ;
quant_check   = "quant", "checks", IDENT, ",", "Idx", "#", IDENT, ":", q_or, ";" ;
q_or          = q_and, {"||", q_and} ;
q_and         = q_primary, {"&&", q_primary} ;
q_primary     = "(", q_or, ")" | q_side, q_relation, q_side ;
q_side        = "#", IDENT | q_term ;
q_term        = ["-"], q_operand, {("+" | "-"), q_operand} ;
q_operand     = DIGITS | IDENT ;
q_relation    = "==" | "<" | ">" | "<=" | ">=" ;
```

The structural parser (G01) reads `quant checks` as its own clause kind
(`SourceContractClauseKind::QuantCheck`, no exit selector) and retains the text
after it; before this card the spelling was a parse error, so no previously
accepted text changes meaning. Static conditions, each rejecting the whole
module with a named diagnostic: the subject `IDENT` is a pointer parameter
(`ptr<mut, u8>` or `ptr<const, u8>`; not a scalar, a slot or another name,
`quant-check-subject`); the binder is exactly `Idx #VAR`
(`quant-check-binder`, `quant-check-shape`); every `#` names `VAR`, which stands
alone on exactly one side of every comparison (`quant-check-variable`); a
comparison is not chained (`quant-check-chained`); `!=`, `!`, `*` and every other
operator are refused (`quant-check-operator`); a `q_operand` is a canonical
`DIGITS` (no sign, suffix, base or leading zero) or a scalar parameter of the
declaration, never an enumeration constant, macro or global, and a term names
at most one parameter with coefficient one and takes no parentheses
(`quant-check-term`); an end that overflows the integer model is refused
(`quant-check-overflow`); at most 512 predicate tokens, 64 nested groups and 64
ranges in the disjunctive normal form (`quant-check-size`); every range bounds
the variable from below and from above (`quant-check-unbounded`); a constant
predicate admits some index (`quant-check-empty`); at most one quant check per
formal (`duplicate bounds clause for quant P`) and none on a formal that also
states `bounds_extent` (`quant-check-with-extent`). An unbalanced group is
refused by the structural parser before the reader. The clause is part of the
declaration digest. Review markers are contract-level only (a clause-level
`bounds_review_pending(…)` wraps a predicate call, which this clause is not).
`resource_*` consumers reject the clause kind.

## G05d. Null-dereference checks

Card NULL-SAFETY-PROPERTY (USER DECISION 2026-10-10, decision 2), contract
specification
[C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref),
gateway rules [14o](agents/gateway.md#constraint-14o) and
[14p](agents/gateway.md#constraint-14p). One more `bounds_clause` of G05b,
read by `crates/qcore-contract/src/bounds_contract.rs`:

```ebnf
bounds_clause   = … | null_deref | deref | live ;
null_deref      = "quant", "null", "-", "deref", "#", IDENT, ";" ;
deref           = "quant", "deref", "#", IDENT, ";" ;   (* NULL-DEREF-QTT-GROUNDING *)
live            = "quant", "live", "#", IDENT, ";" ;    (* NULL-DEREF-QTT-GROUNDING *)
```

The structural parser (G01) reads `quant null-deref` as its own clause kind
(`SourceContractClauseKind::QuantNullDeref`, no exit selector) and retains
the text after it; before this card the spelling was a parse error (`null` is
no exit selector), so no previously accepted text changes meaning. Static
conditions, each rejecting the whole module with a named diagnostic: the
clause is exactly `#` and one `IDENT` (`quant-null-deref-shape`); the `IDENT`
is a pointer parameter of the declaration (`ptr<mut, u8>` or
`ptr<const, u8>`; not a scalar, a slot or another name,
`quant-null-deref-parameter`); at most one per formal (`duplicate bounds
clause for null-deref P`). The clause is part of the declaration digest
(`quant null-deref #P`) and is pinned with the declaration
(`bounds-contracts.lock`). Only the null-dereference property reads it: the
bounds consumer reads the environment without it (a declaration and a module
left with no clause leave the bounds identity), and `resource_*` consumers
reject the clause kind, as they do `quant checks`. A declaration holding only
null-dereference checks still carries `requires bounds_contract_v1;`, and its
signature obeys G05b (no floating type; a slot only with a closed frame).

**Grounding** (card NULL-DEREF-QTT-GROUNDING, USER DECISION 2026-10-11: "We
need grounding."). The production is spelled with `quant` because it is a
quantitative form, but not the §L06 one that G05c reads. It is the
contract-level projection of a §L07 usage grade, read with §L08–§L09 access
and identity ([language §L07](QCORE_SPEC_r0_3.md#l07-usage-grades),
"Null-dereference demand"; [contract
§C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref),
"Grounding"). `quant null-deref #B;` reads as "`B`'s dereference demand has
grade `relevant`": every terminating path uses `B`'s pointee before any NULL
test, so every caller must pass an existing identity (non-NULL). The
production adds no type, dimension, axis, grade literal or predicate. Its
`#IDENT` binder names a formal as G05c's `Idx #IDENT` names an index variable,
and the two clauses stay separate kinds (`QuantNullDeref`, `QuantCheck`), each
read by one consumer only.

**Two levels** (card NULL-DEREF-QTT-GROUNDING; USER DECISION 2026-10-11:
"Two levels"; [contract
§C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref),
"Two levels"). Two more clause kinds, `SourceContractClauseKind::QuantDeref`
and `QuantLive`. Before this card both spellings were parse errors (`deref`
and `live` are no exit selectors), so no previously accepted text changes
meaning. The bounds contract v1 loader elaborates every written form into
canonical parts (`crates/qcore-contract/src/deref_parts.rs`):

| Written | Canonical parts |
| --- | --- |
| `quant deref #B;` | `quant null-deref #B;` · `quant checks B, Idx #i: #i == 0;` · `quant live #B;` |
| `quant null-deref #B;` | the existence part alone (unchanged) |
| `quant checks B, Idx #i: P;` | itself (G05c) |
| `quant live #B;` | the liveness part alone |

Static conditions, each rejecting the whole module with a named diagnostic:
the clause is exactly `#` and one `IDENT` (`quant-deref-shape`,
`quant-live-shape`); the `IDENT` is a pointer parameter
(`quant-deref-parameter`, `quant-live-parameter`); at most one shorthand per
formal (`duplicate bounds clause for deref P`) and at most one written
liveness part (`duplicate bounds clause for live P`). A shorthand's part and a
written part for the same formal are merged when they agree (equal pointers
and, for the extent part, an equal predicate, the bound variable's spelling
aside) and are otherwise refused (`quant-deref-conflict`). A `bounds_extent`
beside a shorthand's extent part is refused as well (`quant-deref-conflict`;
G05c's `quant-check-with-extent`). The parser refuses `quant WORD #IDENT;`
with a word that names no form (`quant-shorthand-unknown`). The pinned digest
(`text_sha256`) covers each line as written (`quant deref #P`, `quant live
#P`), so a shorthand and its decomposition pin differently. `canonical_sha256`
covers the signature and the sorted canonical part set, so the two have one
canonical digest. Resource v1 and every other consumer reject both clause
kinds. On an audit route, a liveness part is admitted only where the
function's resource v1 declaration in force states `resource_owned` at that
position (`quant-live-unbacked`, `quant-live-contradicts-custody`).

**Result states in resource v1** (card OP-CONSTRUCTOR-DECLARATIONS, contract
specification [C04.19.4](QCORE_CONTRACT_SPEC_r0_3.md#c04194-a-constructors-result-state),
gateway rule [14e](agents/gateway.md#constraint-14e); USER DECISION
2026-10-14, C, and the maps-to token). Beside `quant live` and `quant deref`,
a constructor's resource v1 declaration may state its result's member state
in maps-to notation:

```ebnf
result_state_clause = "quant", "live", "#", "result",
    [ ":", IDENT, ".", member_path, MAPS_TO, ("#", "absent" | IDENT) ], ";" ;
handoff_clause = "quant", "live", "#", IDENT, ":", "#", "owner", MAPS_TO, "#", "callback",
    "when", "#", "result", ("==" | "!="), ("@", IDENT | "#", IDENT), ";" ;
MAPS_TO = "|->" ;   (* one lexer token, qcore_frontend::lexer::MAPS_TO *)
```

`|->` is the ASCII maps-to `↦`: `->` was rejected because it collides with C
member access in contract text and with the `.qr` return-type arrow, `>->`
because Z and set-theory notation read it as `↣`, an injective function;
`|->` is the standard ASCII for `↦` (Z, Isabelle, TLA+). It is one lexer
token (`Symbol::MapsTo`), matched before every two-byte spelling, so `|>`
stays the pipe and `||` the logical or (and `||->` is `||` then `->`); before
the card the same bytes lexed as `|` then `->`, which no reader admitted.
`absent` is a reserved word, distinct from C's `NULL`; since card
SUPPLIED-OWNED-RETURN-CODE-HANDOFF (USER DECISION 2026-10-15) it is the value
keyword `#absent`, its one spelling, and a bare `absent` rejects the module
naming `#absent` (`quant-live-absent`). Static conditions,
each rejecting the whole module with a named diagnostic: the binder is
`#result` (`quant-live-result-shape`; a formal is a bounds-module liveness
part, never a resource v1 one); the left of `|->` is a dotted `TAG.FIELD`
(`quant-live-member`; `result->rko_op_cb` and a bare `->` reject with
`quant-live-maps-to`, which names `|->`); the right is `#absent` or, since card OP-APP-RK-CALLBACKS-VIA-POLL-CB (USER
DECISION 2026-10-14, D1: "the right-hand side is either `absent` or a function
designator. Anything else rejects the module"), one identifier naming a
function: never a parameter of the declaration, `result`, C's `NULL` or
`nullptr`, a number or an expression (`quant-live-value`). A function value
is the member's stated default (`quant live #result: rd_kafka_conf_s.log_cb
|-> rd_kafka_log_print;`), and its lower level is
`resource_callback_member_registered(result, TAG.FIELD, f)`. **Two levels:** the clause is the shorthand of `quant
live #result;` (the liveness part, restating `resource_return_owned(nonnull)`)
and the member's state (`#absent` or the function), written alone as
`resource_callback_member_registered(result, TAG.FIELD, #absent)` (or `…, f)`;
`QCore.AppRkCallbacks.the_function_shorthand_and_its_parts_mean_the_same`); the loader
elaborates every spelling into these canonical parts
(`crates/qcore-contract/src/contract_constructed_state.rs`), so the shorthand,
its decomposition and both together give one `ConstructedState`
(`crates/qcore-contract/tests/op_constructor_clause.rs`). The pinned digest
covers the text as written. Every other `quant` form still rejects in
resource v1, but for the hand-off and the member-path states below.

**Hand-offs in resource v1** (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF,
contract specification [C04.19.5](QCORE_CONTRACT_SPEC_r0_3.md#c04195-a-hand-off-by-return-code),
gateway rules [12m](agents/gateway.md#constraint-12m) and
[13l](agents/gateway.md#constraint-13l); USER DECISIONS 2026-10-15). A
formal's binder in a `quant live` clause states a hand-off by return code
(`handoff_clause`, G05): `quant live #rko: #owner |-> #callback when #result
!= @RD_KAFKA_OP_RES_PASS;`. It elaborates into the formal's liveness part
(`quant live #rko;`, backed by `resource_owned(rko, nonnull)`) and the
guarded transfer; the alias and the `@` name read identically. `quant live
#P;` alone with a formal stays the result-state reader's rejection
(`quant-live-result-shape`: a formal's liveness part belongs to a bounds
module). The static conditions are G05's.

**Member-path states in resource v1** (card OP-CALLBACK-MEMBER-PATH-RELEASE,
contract specification [C04.19.6](QCORE_CONTRACT_SPEC_r0_3.md#c04196-a-release-or-move-out-at-a-member-path),
gateway rule [30](agents/gateway.md#constraint-30); USER DECISION
2026-10-15, "B: state + set"). A formal's binder followed by a member path
states what the call leaves there (`member_path_clause`, G05): `quant live
#rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when
#result != @RD_KAFKA_OP_RES_KEEP;`. A `#P: #owner …` clause is a hand-off, a
`#result: …` clause a result state, any other `#P: …` a member-path state
(`crates/qcore-contract/src/contract_member_path.rs`). The static conditions
are G05's.
The neutral bounds-v1 metadata production additionally reuses G05's atom and attachment productions (PLUGIN-PREMISE-DECOMPOSITION):

```ebnf
bounds_hypothesis_metadata = "requires", (hypothesis_atom | hypothesis_decomposition), ";" ;
```

It is validated against the same declaration's existing named bounds premise,
creates no new named premise or interpreted bounds/null clause, and has no
solver semantics. `hypothesis_all` is not admitted in this production. The
property-specific replay and exact compatibility conditions are G05's.

## G05e. Null-property member and result declarations

Card NULL-CANDIDATE-TRIAGE, contract specification
[C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref),
gateway rules [14t](agents/gateway.md#constraint-14t) and
[14u](agents/gateway.md#constraint-14u). Two more clauses of G05b, read by
`crates/qcore-contract/src/bounds_contract.rs` (`BoundsClause::NonnullMember`,
`BoundsClause::NonnullResult`):

```ebnf
bounds_pre  = … | "nonnull_member", "(", IDENT, ",", IDENT, ")" ;
bounds_post = … | "nonnull_result", "(", ")" ;
```

Static conditions, each rejecting the whole module: the first `IDENT` of
`nonnull_member` is a C struct tag, never a parameter of the
declaration, and the second a member name (`nonnull-member-shape`); at most one
per (tag, member) per declaration; `nonnull_result()` is an `ensures
normal` postcondition of a declaration with a pointer result
(`nonnull-result-pointer`), at most one. Both words were absent from every
tracked contract module when introduced (the reserved-spelling census
exempts modules the bounds loader accepts). The card introduced them as
`null_member_nonnull` and `null_result_nonnull`; card NULL-MEMBER-INVARIANT-SCOPE
renamed them on the user's decision of 2026-10-11 (one `nonnull_` scheme,
`nullable_` reserved for clauses stating a value may be NULL; contract
specification C09.3, "Naming"). There is no alias: the old spellings are
refused as any unknown bounds clause is. Since card
NONNULL-MEMBER-UNIFIED-QUALIFIER `nonnull_member` is G05f's `member_clause`,
whose plain kind is this clause; a named premise may stand beside it. The clauses are part of the
declaration digest and pinned with it. Only the null property reads them:
`BoundsContracts::for_bounds()` removes them like `quant null-deref`, and
`resource_*` consumers reject the module. A declaration beside a
resource-v1 declaration of the same function is a registered
`null_deref_beside` relation (it states only the selector and null-property
clauses).

Card NULL-TRIAGE-UNRESOLVED-PATHS (gateway rule
[15q](agents/gateway.md#constraint-15q)) adds one postcondition:

```ebnf
bounds_post = … | "nonnull_result_range", "(", signed, ",", signed, ")" ;
signed      = ["-"], DIGITS ;
```

Static conditions, each rejecting the whole module: the declaration has a
scalar result (`nonnull-result-range-scalar`); both bounds are canonical
decimal literals (no suffix, base or leading zero, never `-0`, inside the
64-bit range: `nonnull-result-range-shape`) and `LO <= HI`
(`nonnull-result-range-empty`); at most one per declaration. The word was absent
from every tracked contract module when introduced. The clause is part of
the declaration digest and pinned with it; only the null property reads it
(`for_bounds()` removes it), and `resource_*` consumers reject the module.
The first tracked use is `providers/quirin-provider-cfl/contracts/cfl/null_deref.qv`
(`cfl_list_size`, in the crate's new `bounds_view`).
## G05f. Scoped member invariants

Card NULL-MEMBER-INVARIANT-SCOPE, contract specification
[C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)
("Scoped member invariants"), gateway rules
[14x](agents/gateway.md#constraint-14x)–[14z](agents/gateway.md#constraint-14z);
one clause since card NONNULL-MEMBER-UNIFIED-QUALIFIER (gateway rule
[14t](agents/gateway.md#constraint-14t); USER DECISION 2026-10-16, decision 3,
verbatim: "As shown above", "Migrate, one spelling", "Yes, all kinds"). G05e's
member clause takes a qualifier, read by
`crates/qcore-contract/src/bounds_contract.rs` (`member_clause`,
`member_qualifier`):

```ebnf
bounds_pre       = … | member_clause ;
member_clause    = "nonnull_member", "(", IDENT, ",", IDENT, ")"                       (* plain, 14t *)
                 | "nonnull_member", "(", IDENT, ",", IDENT, ")", member_qualifier      (* [[stable]], 14x *)
                 | "nonnull_member", "(", IDENT, ",", IDENT, ",", IDENT, ")", member_qualifier ;
                                                                                         (* [[scope]], 14z *)
member_qualifier = "where", "quant", "#", "scope", "[", "[", ("stable" | "scope"), "]", "]" ;
```

The three kinds load to exactly the facts the three spellings before the card
loaded to: plain `BoundsClause::NonnullMember`, `[[stable]]`
`BoundsClause::NonnullMemberStable`, `[[scope]]`
`BoundsClause::NonnullMemberScope` (`QCore.NullMemberQualifier`:
`the_merged_surface_is_equivalent_to_the_old_forms`). Static conditions,
each rejecting the whole module with a named diagnostic:

* the first `IDENT` is a C struct tag, never a parameter of the declaration,
  and the second a member name; any other argument shape
  (`nonnull-member-shape`);
* the third `IDENT` is admitted only with `[[scope]]`
  (`nonnull-member-parameter-needs-scope`), `[[scope]]` only with it
  (`nonnull-member-scope-needs-parameter`), and it names a pointer parameter
  of the declaration (`nonnull-member-scope-parameter`);
* the qualifier is exactly `where quant #scope [[WORD]]`
  (`nonnull-member-qualifier-shape`), `WORD` `stable` or `scope`
  (`nonnull-member-qualifier-unknown`);
* the clause is a `requires`, never an `ensures` (`nonnull-member-requires`);
* at most one plain clause and one `[[stable]]` clause per (tag, member), and
  one `[[scope]]` clause per (parameter, tag, member), per declaration;
* the retired words `nonnull_member_stable` and `nonnull_member_scope` (the
  card's migration rewrote every tracked declaration: librdkafka 67, Fluent
  Bit 14) are rejected in every position, a held one included, with a
  diagnostic naming the form to write (`nonnull-member-retired-spelling`).
  There is no alias. Both words stay in `RESERVED_SPELLINGS` (17), so no other
  vocabulary takes them.

The clauses are part of the declaration digest and pinned with it; the text
the digest covers renders the qualifier as written above, so spacing does
not move it. Only the null property reads them:
`BoundsContracts::for_bounds()` removes them, and `resource_*` consumers
reject the module. A declaration beside a resource-v1 declaration of the same
function is a registered `null_deref_beside` relation. A plain or
`[[stable]]` clause is a type-level declaration hosted, like G05e's member
declaration, on the record's constructor; a `[[scope]]` clause belongs to the
function it is written on. The null property's evidence labels keep their
kind names (`nonnull_member_stable; …` in an assumption, `nonnull_member_scope(TAG,
FIELD, P)` in a scope or check subject), byte for byte: they are not surface
spellings.

Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (contract specification C04.18
"Every vocabulary", C09.3 "Premised member facts"), extended by card
NONNULL-MEMBER-UNIFIED-QUALIFIER ("Yes, all kinds"): G05's named premise is a
bounds precondition only beside a member clause, of any kind
(`BoundsClause::Premise`):

```ebnf
bounds_pre = … | named_premise ;   (* only in a declaration that also states
                                      a member_clause of any kind *)
```

A declaration with a premise and no member clause rejects the module
(`premise-beside-member`). The same applies beside `quant null-deref` or any
bounds clause. A premise under `ensures`, a malformed one and a repeated name
reject it at parse, in every vocabulary. No reserved spelling is added:
`resource_premise` is G05's.

## G05g. Outer contract v1

Card OUTER-CONTRACT, contract specification
[C04.20](QCORE_CONTRACT_SPEC_r0_3.md#c0420-outer-contracts-code-loaded-at-runtime),
gateway rule [31](agents/gateway.md#constraint-31); USER DECISIONS 2026-10-17,
D1 "B: loader envelope" and D2 "Yes, as proposed". A module whose every
declaration selects `outer_contract_v1` is read by
`crates/qcore-contract/src/outer_contract.rs`. The frontend reads a clause that
begins `quant #outer` as its own kind (`SourceContractClauseKind::QuantOuter`,
expression from `#outer`); every other consumer rejects it.

```ebnf
outer_declaration = "extern", "fn", IDENT, "(", parameters, ")", "->", type,
    "contract", "{", outer_clause, { outer_clause }, "}", ";" ;
outer_clause      = "requires", "outer_contract_v1", ";"
                  | "quant", "#", "outer", outer_form, ";" ;
outer_form        = "[[", "contract", "]]", NAME, [ ",", STRING ]
                  | "where", "#", IDENT, "in", "#", "loaded", "[[", "runtime", "]]",
                    [ "when", "#", "result", ( "==" | "!=" ), [ "-" ], INTEGER ]
                  | "#", "loaded", ":", envelope ;
envelope          = IDENT, ".", IDENT, { ".", IDENT }, MAPS_TO, "#", ( "kept" | "registered" )
                  | "#", "premise", IDENT ;
NAME              = lowercase identifier, [a-z][a-z0-9_]* ;
STRING            = one plain string: not blank, no escape, no control character ;
```

Static conditions, each rejecting the whole selection with the named
diagnostic: exactly one `requires outer_contract_v1;` (and no other
`requires` or `ensures`, `outer-clause-foreign`); exactly one header
(`outer-contract-missing`, `outer-contract-repeated`, `outer-contract-shape`,
`outer-statement`); at least one runtime condition (`outer-runtime-missing`),
each binding a distinct pointer formal (`outer-binder-parameter`,
`outer-binder-pointer`, `outer-runtime-repeated`, `outer-runtime-shape`), a
guard only on an integer result (`outer-guard-result`); `#loaded`,
`#kept` and `#registered` written with `#` (`outer-loaded-keyword`,
`outer-envelope-keyword`); a dotted member path (`outer-envelope-member`) and
one of the two values (`outer-envelope-value`); no repeated envelope key
(`outer-envelope-repeated`); envelope lines only on the defining declaration
(`outer-envelope-citation`); across the selection one definition per contract
(`outer-contract-defined-twice`) and no citation without one
(`outer-contract-undefined`). The digest of a declaration covers every clause as
written (`text_sha256`), and the pin (`quirin-audit pin --outer`) binds it to
the loader's body.

## G05h. Local condition and registered-lifetime clauses

Card FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME S3, C04.21. These alternatives
extend `resource_clause`; the resource vocabulary selector stays unconditional.

```ebnf
condition_binding = "quant", "#", IDENT, "=", "(", "#", IDENT, ".",
                    member_path, "!=", "0", ")", ";" ;
condition_block = "quant", ["not"], "#", IDENT, "in", "{",
                  local_clause, {local_clause}, "}", ":", "[[", "local", "]]", ";" ;
local_clause = "requires", local_prerequisite, ";" | registered_clause ;
local_prerequisite = named_premise
  | "resource_loop_owner", "(", IDENT, ",", ("read" | "mutate"), ")"
  | "resource_thread_sync", "(", IDENT, [",", IDENT], ",",
      ("cond_wait" | "cond_timedwait" | "cond_signal" | "cond_init" | "cond_destroy" | "join"), ")"
  | "resource_lifecycle", "(", IDENT, ",", ("start" | "stop" | "destroy"), ")" ;
registered_clause = "quant", "registered", "#", IDENT, ":", IDENT, ".",
                    member_path, MAPS_TO, IDENT, ".", member_path, ";" ;
```

The binding root is a depth-one pointer formal of the host declaration; the
source reader must prove the final member is a nonvolatile integer. Both
registered paths must use the same record. Bindings are unique and used; one block per polarity is accepted. No implicit/free
record binder escapes a registered clause. The parser rejects nested blocks,
empty blocks, other attributes and unsupported inner clauses; each local
clause is limited to 64 KiB and each member path to 64 segments. Existing
prerequisite loaders validate their arguments rather than accepting opaque
text. Current source discharge for active prerequisites is the exact guarded
named-entry-premise subset; other active obligations remain Unknown. These
local contexts are distinct from G05g's runtime/static outer scenarios.

## G05i. Declared cell protocols

Card MK-EVENT-CELL-CROSS-FUNCTION (USER DECISION 2026-11-02, verbatim: "B as
recommended"), contract specification
[C04.6.3](QCORE_CONTRACT_SPEC_r0_3.md#c0463-declared-cell-protocols), gateway
rule [38c](agents/gateway.md#constraint-38c). One more `resource_clause`
alternative, read by `crates/qcore-contract/src/local_condition.rs` (`cell`,
`cell_protocols`):

```ebnf
cell_clause = "quant", "cell", "#", IDENT, ":", IDENT, ".", cell_path, MAPS_TO,
              ( "#", "fresh", "(", "descriptor", ")" | "#", "same", "(", cell_path, ")" ),
              "when", "#", IDENT, ".", cell_path, cell_guard, ";" ;
cell_path   = cell_segment, { ".", cell_segment } ;
cell_segment = IDENT, [ "[", DECIMAL, "]" ] ;      (* canonical: no leading zero *)
cell_guard  = "!=", "-", "1" | ">", "0" | "==", "1" ;
```

The structural parser (G01) reads `quant cell` as its own clause kind
(`SourceContractClauseKind::QuantCell`). Before this card it was a parse error,
so no previously accepted text changes meaning.

Static conditions. Each one rejects the whole module (`quant-cell-*`):

- **Binder.** The binder is no reserved `#` word, and one record uses one
  binder.
- **Guard.** The guard's binder is the clause's own. Every clause of a record
  states the same guard.
- **Kind.** `#fresh` takes exactly `descriptor`.
- **Cells.** Each cell is stated once. At least one cell is `#fresh`. A
  `#same` target is a `#fresh` cell of the same record.
- **Guard and cells.**
  - A `!= -1` or `> 0` guard is a `#fresh` cell.
  - A `== 1` guard is no cell.
- **Size.** A path has at most 64 segments.

The clauses of one declaration load to one canonical `CellProtocol` per record
(`LoopLifetimeContract::cells`, omitted when empty). The host becomes a
withheld lifetime declaration, as with G05h's registered clauses. Across
declarations, `ResourceContracts::cell_protocols` keeps a record's protocol
only when every declaration states the same one.

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

### G06a. Bounded observed timed-wait contract consumer

`quirin-audit observed-wait` uses the structural contract parser to load exactly
the selected declarations for `_mk_event_loop_create`, `_mk_event_wait_2`,
`_mk_event_wait_with_flags`, `_mk_event_loop_destroy` and `cb_wait_2_timeout`
from `monkey/wrapper_paths.qv` and `monkey/composition.qv`. The consumer
requires the supported extern signatures, parameter usage/ownership, return
types and exact `requires`/`ensures normal` clauses, with no body, `unsafe` or
effects block. The contract-mode `wait_source_policy.qv` declares those
expectations over derived declaration facts. Missing, duplicate, added or
changed selected clauses refuse.
It checks the named predicates against typed C paths and derives native
obligations; these identifiers do not become arbitrary executable predicates
in the G06 arithmetic interpreter. Contract acceptance alone emits no safety
verdict. One complete identity-bearing observation and event-kernel replay are
required for the separate bounded observed-composition artifact. Source:
`crates/qcore-verification/src/monkey_wait_projection.rs`; bounds and remaining
paths: [observed integration](reports/cycle18-observed-integration/README.md).

### G06b. Typed executable policy predicates

`check_typed_contract_facts` extends G06 for an explicitly selected policy
consumer. A fact is either an `i128` integer or UTF-8 text. Its primary grammar
additionally admits double-quoted string literals. Literals decode the existing
`source.string.v1` escapes before comparison. Text supports same-type `==` and
`!=` only; text ordering, arithmetic and direct Boolean use reject, and there
is no text/integer coercion. Boolean operators still evaluate both operands.
The legacy numeric API and G06 grammar remain unchanged.

The selected declaration must have the exact lexed header
`extern fn NAME() -> unit contract`, ignoring comment trivia. Parameters,
generics, effects, unsafe qualifiers, bodies and non-unit results reject.
Only `requires` and `ensures normal` clauses are supported, with at least one
postcondition. Duplicate facts, missing facts, unsupported syntax, type errors,
false preconditions, false postconditions and overflow all reject.

The typed consumer bounds source text at 64 KiB, facts at 256, function/fact
names at 256 bytes and text values at 4096 bytes. The predicate limits remain
512 non-EOF tokens and depth 64. These are consumer limits, not new general
language-conformance claims.

`contract_fact_literal(source, function, fact) -> Result<ContractFact, String>`
reads exactly one standalone `ensures normal FACT == LITERAL` clause from a
strict policy declaration. It accepts text and signed integer literals and
rejects missing, duplicate exact or conditional forms. Reading a literal does
not establish any supplied fact or discharge the other clauses; callers still
check the complete selected policy against independently derived facts.

The timed-wait consumers use `monkey/wait_source_policy.qv` for source and
contract-shape acceptance and `monkey/wait_identity_policy.qv` for context,
digest, bound and observed-profile relationships. Policy bytes are bound into
the resulting context. Source extraction and runtime observations supply facts;
the desired literals in a policy cannot manufacture the corresponding observed
values. See [extraction and parity](reports/cycle18-contract-extraction/README.md).

## G07. Extended grammar and conformance

`QCORE_DESIGN_GRAMMAR_r0_3.bnf` collects the nineteen BNF blocks from revision 0.2, adding the current optional module header. It preserves the intended full language vocabulary, including source constructs without driver lowering. Its lexical terminal placeholders are `IDENTIFIER-TOKEN`, `INTEGER-TOKEN`, `FLOAT-TOKEN`, `STRING-TOKEN` and `CHAR-TOKEN`.

That target grammar is neither a parser generator input already used by the compiler nor the grammar of every policy consumer. In particular, retained policy token expressions and resource predicate calls are defined at their consumer boundary above; their structural acceptance is not a new general-purpose contract proof rule. Future implementation work MUST keep the profile grammars, type rules, lowering coverage and negative tests synchronized instead of conflating the target with today's supported syntax.

## G07.9. Immutable UTF-8 string views v1 (`source.string.v1`)

The revision-0.3 [string-view amendment](design/STRING_SLICE_V1.md) requires an
explicit, file-local `#import std.string;` for string capabilities. Card
`STRING-SLICE` registers that module and implements this consumer grammar
(diagnostics `Q068`–`Q073`, spec §L21), card `STRING-SLICE-IO` adds the two
rendered-text statements marked below (`Q075`–`Q076`, the same pass's second
band), and card `STRING-CONCAT` adds the composition, the `std.buffer` bridge
and the embedded buffer statements (`Q077`–`Q079`, the third band):

```ebnf
module    = { import }, { error-decl | entry } ;
import    = "import", ( "std.string" | "std.buffer" | "std.fmt"
                      | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ], "{", { stmt }, "}" ;
stmt      = "let", NAME, [ ":", "string" ], "=", STRING-TOKEN, ";"
          | "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "let", NAME, "=", "len", "(", NAME, ")", ";"
          | "let", NAME, [ ":", "string" ], "=", NAME, "[", OPERAND, ":", OPERAND, "]", ";"
          | "let", NAME, [ ":", "string" ], "=", "concat", "(", PIECE, { ",", PIECE }, ")", ";"
                                                    (* STRING-CONCAT *)
          | "let", NAME, [ ":", "string" ], "=", "borrow_utf8", "(", NAME, ")", ";"
                                                    (* STRING-CONCAT *)
          | "let", NAME, [ ":", "string" ], "=", "borrow_utf8",
            "(", NAME, ",", OPERAND, ",", OPERAND, ")", ";"
                                                    (* CT07-SLICE-PERMISSIONS *)
          | BUFFER-STMT                             (* STRING-CONCAT *)
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "print", TEXT, ";"                      (* STRING-SLICE-IO *)
          | "log", LEVEL, TEXT, ";"                 (* STRING-SLICE-IO *)
          | "log_level", LEVEL, ";"
          | "flush", ";"
          | "return", ";" ;
OPERAND   = [ "-" ], INTEGER | NAME ;
TEXT      = STRING-TOKEN | NAME | NAME, "[", OPERAND, ":", OPERAND, "]" ;
PIECE     = TEXT | RENDERING, "(", NAME, ")" ;
EFFECT    = "io" | "alloc" | "mut" ;
```

`BUFFER-STMT` is `source.buffer.v1`'s own statement grammar (§G02k), **read by
that route's own reader** (`source_std_buffer::buffer_statement`), so a byte
write this route admits is byte for byte the statement that route would have
produced and a shape one accepts and the other refuses is not expressible. A
composed module is one that **acquires** a buffer: a module with no `bytes(N)`
in its entry embeds no buffer statement at all, which is what keeps every
program written before card `STRING-CONCAT` on the code path it was on.

`STRING-TOKEN` is the existing lexer's retained string token; this grammar adds
no token and **reserves no word**. `len`, `concat` and `borrow_utf8` are
contextual — each is a clause word only where it is immediately followed by
`(` — and `string` appears only in a type ascription. `dec` and `hex` need no
rule, having been reserved words since `STD-MODULES`. A quoted token in
*declaration metadata*, such as a `verify scope` entry's plugin version or
profile, is not a statement and acquires no import requirement.

**Composition (`concat`) is static.** Every `PIECE` is check-time-known, so the
compiler computes the result's bytes and it becomes a new `static const` payload
of the emitted program: no allocation, no run-time copy, no custody, and a
length that is the sum of the pieces' lengths and is itself a check-time
constant. A `RENDERING "(" NAME ")"` piece is the **text of a bound integer**,
produced by the same rendering function `print dec NAME;` renders with, so the
two spellings of a number cannot disagree. Two pieces are the minimum: a
composition of one is that piece, and is refused.

**The bridge (`borrow_utf8`) is checked twice.** Its UTF-8 validity is decided
**statically**, out of the buffer's own write history — `source.buffer.v1`
admits only constant bytes at constant indices — and the buffer may not be
written or released inside the borrow's **window**, which runs from the borrow
to the last statement that reads the view or anything derived from it. Both
directions are refusals at check time; there is no run-time scan and no failure
arm. A borrow contributes an ordinary `Read` to the cycle-3 ownership operation
list, so borrowing a released buffer is that checker's own refusal rather than a
rule this grammar adds.

**A borrow lends a range** (card `CT07-SLICE-PERMISSIONS`). The three-argument
form names the bytes `[LOW, HIGH)`, with both bounds resolved by the **same**
operand rule a slice bound is and checked against the extent the acquisition
declared (`0 <= low <= high <= extent`). The arity is **one or three and
nothing between**: a range needs both of its ends, so a two-argument spelling
has no reading and keeps refusing by shape. The one-argument form *is* the
three-argument one at `[0, extent)` — not an abbreviation of it but the same
rule at its degenerate bound, measured as an identical certificate and
byte-identical emitted C. What the range changes is the window's **content**
rather than its extent: a write is refused when its byte lies inside a live
loan and **admitted when it lies outside every one of them**, so two disjoint
permissions over one buffer are expressible and their disjointness is decided
(`max low < min high`) rather than assumed. A release is still refused for any
live loan however narrow, because it ends the storage all of them address, and
the UTF-8 decision is made on the lent bytes — so a range whose end splits a
code point is refused by that decision rather than by a new one. There is no
**write** lend: the owner keeps the write authority over every byte it did not
lend.

A bound is a check-time constant, so `0 <= low <= high <= len(s)` and both UTF-8
code-point boundaries are decided before emission and this profile has no
run-time failure arm. That is as true of an inline `TEXT` slice as of a bound
one: both are decided by one predicate, so a slice cannot be admitted at a
rendering operand where the same slice written as a `let` would be refused.

The two statement families above are separated by a **closed word set** and not
by a lookahead: `RENDERING` is `dec` or `hex`, so a statement naming one renders
a scalar and a statement naming neither renders text. A string therefore takes
**no** selector — `dec` and `hex` are total functions from an `i32` to digits
and a `string` is already the bytes — and a selector written in front of one
refuses at `Q072`, where it refused before the composition existed. What reaches
the stream is the view's exact UTF-8 payload, framed by the sink's own line
discipline: no re-encoding, no escaping, no terminator, no newline beyond the
one every line carries, and an embedded `0` written as the ordinary byte it is.

Both pipe spellings are the same statements (§G02h): `s |> print;` is
`print s;` and `s[0:4] |> log info;` is `log info s[0:4];`.

**Route selection is unmoved by the text forms.** A *constructed* string at an
operand — a quoted literal or an inline slice — selects this route; a bare `NAME`
does not, so `print bogus;` in a `source.std-io.v1` module keeps that route's own
refusal, and a bare name is admitted as text only inside a module already on
this route.

What remains outside every grammar in this document, each a named follow-on:
**freezing** a `std.buffer` value into an owned string (the borrow landed; the
freeze needs a custody component the view has nowhere to carry), transporting a
string at an extern/native-library ABI, comparing or searching strings, a
single-index access into a string, a composition with a piece the compiler
cannot evaluate, and the separate mutable `{ptr, len, cap}` byte slice.

## G07.10. A checked HTTP/1.1 message subset (`source.http.v1`)

Card `HTTP-QUIRIN-V1` registers `std.http` and implements this consumer grammar
(diagnostics `Q090`–`Q096`, spec §L21); card `HTTP-V1-COMPUTED-LENGTH` widens a
component position to a value the entry **computes**, allocating no code and
reserving no word. It is **an executed HTTP/1.1 subset with the boundary named**,
never HTTP/1.1 conformance: what is outside it is listed at the end of this
section and in `examples/http/README.md`.

```ebnf
module    = { import }, entry ;
import    = "import", ( "std.http" | "std.socket" | "std.string"
                      | "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ], "{", { stmt }, "}" ;
stmt      = "let", NAME, "=", "request", STRING-TOKEN, STRING-TOKEN, STRING-TOKEN, ";"
          | "let", NAME, "=", "response", STRING-TOKEN, INTEGER, STRING-TOKEN, ";"
          | "header", NAME, STRING-TOKEN, VALUE, ";"
          | "content", NAME, VALUE, ";"
          | "let", NAME, "=", STRING-TOKEN, ";"        (* a bound string view *)
          | "let", NAME, "=", "concat", "(", VALUE, { ",", VALUE }, ")", ";"
          | "let", NAME, "=", "len", "(", NAME, ")", ";"
          | "send", NAME, NAME, ";"                    (* a message or a view *)
          | "recv", NAME, NAME, ";"                    (* verified against one *)
          | SOCKET-STMT ;                              (* §G's socket surface *)
VALUE     = STRING-TOKEN                               (* this module's syntax *)
          | NAME                                       (* text this entry bound *)
          | ( "dec" | "hex" ), "(", NAME, ")" ;        (* a bound integer's text *)
EFFECT    = "io" | "alloc" | "mut" ;
```

`SOCKET-STMT` is `source.socket.v1`'s own statement grammar, **read by that
route's own reader** (`source_socket::socket_statement`), so a transport
statement this route admits is byte for byte the statement that route would have
produced and a shape one accepts and the other refuses is not expressible.

**Four reserved words** — `request`, `response`, `header`, `content` — each a
clause word at a statement head or in a `let` initialiser and never a reference.
`send`, `recv`, `let`, `close` and the standard-IO line words are reused rather
than respelled. The census that chose them rejected `body` (8 occurrences in
QCore *source* position, as a binding name in shipped fixtures), `message` and
`payload`; RFC 9110 §6.4 renamed the body concept **content**, so the admitted
word is also the specification's current one.

**Every component is check-time-known** — a quoted token, or a value composed
from check-time-known pieces — which is what makes a family of RFC 9112 facts
decidable before emission and refused by name: the version is exactly `HTTP/1.1`
(`Q091`), a method and a field name are `token`s and a target is origin-form
(`Q091`, `Q092`), no component may carry a `CR` or an `LF` (`Q092`), a request
carries exactly one `Host` field (`Q092`, RFC 9112 §3.2), and a declared
`Content-Length` **equals the content's byte count** (`Q093`).

**A computed component decides exactly the same facts** (card
`HTTP-V1-COMPUTED-LENGTH`). A `VALUE` is resolved to bytes *before* any gate
runs, so `Q092`'s `CR`/`LF` exclusion and `Q093`'s length equality are the
unchanged functions above and never learn which spelling produced the bytes: a
`concat` piece carrying a framing byte refuses at `Q092` exactly as a written one
does, and a `dec(len(body))` that is not the content's byte count refuses at
`Q093`. The composition is **static** — the compiler holds the bytes, so the
result is a `static const` payload of the emitted program with no allocation, no
run-time copy and a length that is itself a check-time constant (constraint
15h) — and a composition of fewer than two pieces, or one above the 4096-byte
payload bound, refuses at `Q090`. `concat` and `len` are `source.string.v1`'s
**contextual** clause words read through that route's own predicates, `dec` and
`hex` have been reserved since `STD-MODULES`, and `has_source_http` is unchanged,
so this widening adds no reserved word and moves no route selection. The **field
name** stays a written token (`HTTP-V1-COMPUTED-FIELD-NAME`).

**The framing is constructed, not spelled.** No source program writes a CRLF:
the compiler writes one after the start line, one after each field and one to end
the head, and because no admitted component may carry one, every CRLF in an
admitted head is one the compiler put there.

**Import composition.** `std.http` grants the message vocabulary and nothing
else: it opens no socket (a program that speaks HTTP names `std.socket` too),
constructs no `string` — a quoted token at a method, target, field name, field
value or content position is this module's own syntax and no value can name it,
which is the case `STRING_SLICE_V1.md` §2 excludes — and writes no line. A
`let NAME = STRING-TOKEN;` or `let NAME = concat(…);` binding **does** construct
a string value a later statement names, and takes `std.string`'s own gate, as
does `let NAME = len(NAME);`. A `dec(NAME)` written *at a component* is this
module's syntax on the same terms as a quoted token there and takes neither that
gate nor `std.fmt`'s — the answer `STRING-CONCAT` already gave for a `dec` piece
inside a composition, inherited rather than decided again.

`source.socket.v1` itself also gains a string payload here
(`send NAME STRING-TOKEN;` and `recv NAME STRING-TOKEN;`, the socket card's
follow-on `SOCKET-STRING-PAYLOAD`), gated by `std.string` for the same reason.

What remains outside this grammar, each a named follow-on: a component whose
bytes the compiler **cannot** evaluate (`STRING-SLICE-CONCAT-RUNTIME` — the point
at which the `Content-Length` equality would stop being decidable), a computed
**field name** (`HTTP-V1-COMPUTED-FIELD-NAME`), a slice of a bound view beside
HTTP (`HTTP-V1-STRING-COMPOSE`), reading a message out of unknown input (`HTTP-V1-PARSE`), chunked transfer coding (`HTTP-V1-CHUNKED`), persistent
connections and pipelining (`HTTP-V1-PERSISTENT`), `100 (Continue)`
(`HTTP-V1-EXPECT-CONTINUE`), an external peer (`SOCKET-EXTERNAL-PEER`), TLS
(`HTTP-V1-TLS`), a body that is a `std.buffer` extent
(`HTTP-V1-BUFFER-CONTENT`), `obs-text` in a field value (`HTTP-V1-OBS-TEXT`), and
a rendered text line inside this route (`HTTP-V1-TEXT-TRANSCRIPT`).

## G07.11. A checked socket as a reactor-registered event source

Card `SOCKET-EVENT-COMPOSE`. This is not a new route: it is `source.socket.v1`'s
own grammar (§L21's `Q080`–`Q089` band) widened by the five statements of the
reactor vocabulary, which are `source.event.v1`'s own words (§G02k) and **add no
reserved word at all**.

```ebnf
socket-stmt    = (* every §G's socket statement, unchanged *)
               | "event", "open", ";"
               | "event", "close", ";"
               | "watch", NAME, NAME, INTEREST, ";"
               | "unwatch", NAME, ";"
               | "dispatch", "ready", NAME, ";"
               | "dispatch", "idle", ";" ;
INTEREST       = "read" | "write" ;
```

Route selection: a module using **both** vocabularies is a `source.socket.v1`
module; one using only the reactor's stays `source.event.v1`'s; one that also
buffers, constructs a string or declares an HTTP message declines the composed
guard and keeps the refusal it already had (`EVENT-BUFFER-PAYLOAD`,
`HTTP-V1-EVENT-COMPOSE`). `wake` and `endpoints` are deliberately **not** in the
grammar above: readiness here is induced by `connect`, `send` and `sendto`, and
descriptors come from `tcp`, `udp` and `accept`, so neither statement has
anything to do. A `write` interest is checked in full and makes the module
emit no C, which is §G02k's own `accept_check_only` arm.

The custody relation is a **loan**: `watch E S …;` is an
`OwnershipOp::BorrowShared` of `S` to `E` running to the matching `unwatch E;`,
so a close while registered and a `bind`/`listen`/`connect` on a watched socket
are refused by the *unedited* ownership checker while a `send`, a `recv` and an
`accept` are admitted. Spans are **lexical**, so registrations retire in the
reverse of the order they are taken; a crossed pair is refused by name
(`SOCKET-EVENT-CROSSED-SPANS`). A dispatch turn **drains nothing** — it observes
level-triggered readiness, and the entry's own transport statements do the work.

What remains outside this grammar, each a named follow-on: a source-level
callback body (`EVENT-CALLBACK-BODY`), an executable write interest
(`SOCKET-EVENT-WRITE-READY`), a turn over a two-member ready set
(`OBS-REORDERING`), crossed registration spans
(`SOCKET-EVENT-CROSSED-SPANS`), a registration on a socket of another program
(`SOCKET-EXTERNAL-PEER`), and composition with `source.http.v1`
(`HTTP-V1-EVENT-COMPOSE`).
## G07.12. The checked bounded counted loop (`source.loop.v1`)

Card `LOOP-V1` implements this consumer grammar (diagnostics `Q097`–`Q099`, spec
§L21); card `LOOP-UNROLL-BUDGET` adds the `attr` production and the three-layer
budget below, **allocating no code** — each of its refusals is a condition of
`Q097` or `Q098`. It is a **control shape**, not a bundled module: it needs no
`import`, exactly as `source.branch.v1`'s `if` does, and a route admits it the
way a route admits `if`. **v1 admits it on `source.buffer.v1` only**; every
other route keeps the refusal it already had (`LOOP-ROUTE-WIDENING`), and a
clause list reaching one of those routes is refused at that route's own code.

```ebnf
stmt   = [ attr ], "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}" ;
attr   = "[|", "#", "unroll", ",", "where", "#", "bound", "<", INTEGER, "|]" ;
BOUND  = INTEGER | NAME ;
```

* **`BOUND` is a check-time constant** the *lowering* can compute: an integer
  literal, a name bound by a literal `let`, or a name bound by `extent(NAME)` —
  and, since card `W48-QR-SCALAR-TYPES-R5`, a name bound by a conversion or a
  value statement over such values, of any integer type of §G02t, the index
  then taking the bound's type (§G02t, "Typed loops"). A bound read out of a
  byte, a field or a copy is refused (`Q098`, `LOOP-BOUND-FROM-BYTE`), and so
  is a `bool` bound, an `isize`/`usize` bound with no recorded target, a bound
  of zero, a negative bound, a bound above 4096, and an entry whose unrolled
  event total is above 8192.
* **The bound is held to a three-layer budget** (card `LOOP-UNROLL-BUDGET`).
  Layer one is the **absolute ceiling**, 4096, which no declaration may raise.
  Layer two is the **default budget**, 256: a `for` with no `attr` may count at
  most that many iterations, and one above it refuses at `Q098` with the clause
  list that would license it **written into the message**. Layer three is the
  `attr` itself — one statement-adjacent clause list licensing the single `for`
  that follows it.

  The declaration is **checked, never trusted**. The checker compares the
  loop's own check-time bound against the condition and refuses a loop that
  violates it, naming both numbers; `#bound < N` is strict, so `N` itself is not
  licensed. A declared `N` that would license a bound above the ceiling refuses
  **before** this loop's bound is looked at, which is what keeps the ceiling a
  ceiling rather than a default with a longer spelling. Because `#bound < N`
  licenses bounds up to `N - 1`, the largest condition the construct admits is
  `#bound < 4097`, and that is the one that licenses a traversal of the largest
  buffer `source.buffer.v1` admits. A condition **no** bound can satisfy —
  `#bound < 1`, `#bound < 0` — refuses from the clause list alone, before any
  loop is read, and names no bound.

  The attribute raises the **per-loop bound and nothing else**: the entry's
  unrolled-event budget of 8192 is untouched, so a nest of licensed loops is
  still refused by what the checking route actually costs. A clause list outside
  the form above, one before a statement that is not a `for`, a second one in a
  row and a dangling one all refuse at `Q097`; an *unterminated* one is a `Q000`
  parse failure. A clause list the reader could not read suppresses the budget
  decision, so one mistake produces one refusal.

  **The attribute erases.** It governs the checker's budget and appears nowhere
  in the emitted C: an attributed and an unattributed program of the same
  admitted bound emit byte-identical C, and the licence changes no published
  value in the certificate either.
* **The low end is the written-out `0`.** A range with another low end refuses
  at `Q097` rather than being silently re-based (`LOOP-RANGE-LOW`).
* **`NAME` is a scalar binding scoped to the body**, taking `0, 1, …, BOUND-1`
  in order and read-only inside it. It is not visible after the loop, and
  neither is any other name the body binds — the `IF-STATEMENT` rule for arm
  bindings. A body binding that shadows one the entry already made refuses at
  `Q097` rather than deciding which of two values survives the loop.
* **The body's statements are the admitting route's own**, read by that route's
  own reader, so a shape one admits and the other refuses is not expressible.
  Nesting a `for` inside a `for` is admitted; `break`, `continue` and `return`
  inside a body are refused by name at `Q097`.
* **One iteration's net custody effect must be the identity** (`Q099`). A body
  that acquires and releases is admitted; one that acquires without releasing,
  or releases a resource it did not acquire, is refused.

**Zero new tokens and one reserved word.** `..` is two existing `Dot` tokens read
positionally; `for` is reserved (census-clean at zero occurrences outside `//`
comments across all 418 `.qcore`/`.qc` files); `in` is a **contextual** clause
word and deliberately **not** reserved, because the constraint-14 census found it
as a reference inside the pinned contract corpus.

**Checked by exact bounded unrolling, emitted as a real `for` loop.** The two
halves are asymmetric on purpose: the checker needs each iteration's exact
values because the certificate publishes them and the route's evidence is a byte
comparison, while the emitted C needs to be the program a C reader would have
written — and emitting the loop makes the native run a *check* of the unrolling
rather than a restatement of it.

## G07.13. The first source-level aggregate (`source.record.v1`)

Card `AGGREGATES-V1` implements this consumer grammar. It **allocates no
diagnostic code**: every refusal is a condition of the admitting route's
existing `Q051`–`Q057` band (spec §L21), widened and stated there rather than
taken silently, under the `Q-NAMESPACE-DECISION` ice box and on the
`SOCKET-EVENT-COMPOSE` / `SOCKET-INET-ADDRESSES` precedent. It **reserves no
word** either. Like `if` and `for` it is a language construct and not a bundled
module: it needs no `import`. **v1 admits it on `source.buffer.v1` only**; every
other route keeps the refusal it already had (`AGGREGATE-ROUTE-WIDENING`).

```ebnf
decl    = "type", NAME, "{", field, { ",", field }, [ "," ], "}" ;
field   = NAME, ":", ( SCALAR | "bytes" | RECORD ) ; (* SCALAR: §G02t; RECORD: W48-QR-USABLE-RECORDS *)
path    = ".", NAME, { ".", NAME } ;                (* W48-QR-USABLE-RECORDS *)
stmt    = "let", NAME, "=", NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
        | "let", NAME, "=", NAME, path, ";"
        | NAME, path, "=", VALUE, ";" ;             (* the field write, W48-QR-USABLE-RECORDS *)
assign  = NAME, ":", OPERAND ;
OPERAND = [ "-" ], INTEGER | NAME                  (* `source.buffer.v1`'s own *)
        | "true" | "false" ;                       (* at a `bool` field *)
```

* **The declaration head is the already-reserved `type`**, claimed at the one
  shape a `{` after the name gives it; `type NAME = …;` is the alias it always
  was. `resource type NAME {` is claimed by an earlier branch and never reaches
  it. The constraint-14 census found **zero** bare `type NAME {` occurrences in
  the tree and **rejected `record`** as a spelling: an archived probe fixture is
  the single line `record R { }`, pinned to `Q000` by two archived reports.
* **Card `W48-QR-SCALAR-TYPES-R3`: a scalar field is any §G02t type.** Its
  value is a literal range-checked against the field's type (the minimum
  included; `Q053`), `true`/`false` at a `bool` field, or a bound name of
  **exactly** that type — no implicit conversion (`Q052`); a field read binds a
  value of that type into the route's typed environment (§G02t), which every
  position then checks. `isize`/`usize` fields need the recorded target
  (`Q051`). An `i32` field reads its literal with the route's own `i32` reader,
  so every record declared before the card checks and emits byte-identically.
  Each member is emitted at its type's `<stdint.h>`/`<stdbool.h>`/`<stddef.h>`
  spelling.
* **Every scalar field is a plain value, and a record of scalars is therefore
  a plain copyable value.** (Before card `W48-QR-SCALAR-TYPES-R3`, every such
  field was an `i32`.)
  It contributes no ownership operation, opens no obligation, derives no effect,
  and is refused at every custody position by the admitting route's existing
  "this is not a buffer" rule. An owned or view field is a new custody shape and
  a separate card (`AGGREGATE-OWNED-FIELD`, `AGGREGATE-VIEW-FIELD`): the cycle-3
  ownership IR addresses an owner by a plain **name** and has no notion of a
  path, so `p.buf` is not something the checker can be asked about.
* **Type formation decides distinctness.** A record declares 1 to 32 fields and
  no two may share a name (`Q051`; the field bound is `Q053`). A repeated name is
  refused rather than resolved, because which of two declarations a read answers
  to would otherwise depend on the order a reader walks the list in.
* **Construction is total**: every declared field is supplied exactly once, by
  name, never by position. A missing, undeclared or repeated field refuses at
  `Q051`. Totality is what makes a field read total on the declared names.
* **A field value is the admitting route's own operand** — an integer literal or
  a name that route bound to a check-time-constant integer — read by that route's
  own reader, so a value one admits and the other refuses is not expressible. A
  buffer at a field position is that route's existing `Q055`.
* **A field read binds an ordinary scalar** of the admitting route, so rendering,
  indexing and storing a field needs **no new operand form**. A record at a
  position that takes an integer refuses at `Q052`, naming the field read that
  would fix it.
* **Card `W48-QR-USABLE-RECORDS`: writes, nesting, and calls.** A field whose
  type is another record the module declares (either declaration order) holds
  that record by value: a record of scalars is copied in, one that owns a
  buffer anywhere below is moved in (the ownership IR's owner path, one level
  before the card, is now a path of any depth, and the owned leaves are
  released depth-first in declaration order). The declaration graph is
  acyclic (a record that contains itself refuses, `Q051`), at most 8 deep and
  256 leaves (`Q053`). Every field-path form takes a path (`o.inner.buf[0]`,
  `drop o.inner.buf;`, `extent(o.inner.buf)`); a path through a field that is
  not a record refuses (`Q052`). `RECORD.PATH = VALUE;` writes a scalar field
  with a value of exactly its type (a literal range-checked, `Q053`; a name
  of another type, `Q052`) and derives `mut`; at an owned field it is the
  **refill**, moving a buffer into a hole only (over a live field, `Q054`). On
  a record that owns, a read or write of a scalar field asks the IR that the
  record is live, so after `drop f;` both are a use after drop (`Q054`). A
  whole sub-record write and a whole read of a sub-record that owns refuse by
  name (`Q051`). A record of scalars crosses `.qr` calls on
  `source.scalar-calls.v1` (§G02s); this route stays entry-only.
* **Card `W48-QR-TASK-RECORD-CAPTURES`: the scoped-task route.** A task module
  declares records whose owned fields are `own<Buffer<T>>` — the task route's
  own resource — and a record crosses a spawn and a `join` as its leaves
  (§G02a, "Records across tasks"). `AGGREGATE-TASK-CAPTURE`'s by-name refusal is
  lifted.
* **Name resolution needed no new rule.** A field name sits before a `:` at a
  construction and after a `.` at a read, and both positions were already drained
  by `lexical_references`; the type name is an ordinary reference and resolves
  because every declaration's name is a global. A construction of an undeclared
  type therefore refuses at `Q013`.
* **The `.` adjacency is pinned.** A field access is `Word Dot Word` and cannot
  collapse into a float, because the lexer starts a `Float` only at a digit —
  `127.0.0.1` is still `Float Dot Float`. A *numeric* field path (`p.1.2`) is one
  float token after the `Dot` and refuses by name at `Q051`.
* **Each declared type emits one C struct**, with one member per field in
  declared order, and a construction emits one designated initializer naming
  every field. Member and tag names are prefixed (`f_`, `qcore_record_`) so a
  QCore name that is a C keyword still emits a compilable struct.

## G08. Source-contract service configuration v1

This JSON format configures the audit service; it adds no QCore source syntax.
The base fields below are required; `frame_helpers` is optional on each binding.
Unknown fields reject:

```json
{
  "schema_version": 1,
  "packages": ["packages/example"],
  "bindings": [
    {"library": "example", "provider": "example.boolean", "contract": "policy.qv"}
  ]
}
```

Relative package and policy paths resolve against the configuration directory.
There must be 1–256 distinct nonempty package paths and 1–4096 bindings with
unique nonempty library names and valid provider IDs. Configuration is bounded
at 1 MiB, each UTF-8 policy at 8 MiB, and retained policy bytes together at
32 MiB. Policy bytes are snapshotted once. Package identities, manifest target
support and the existing Boolean-field checker capability must validate before
dispatch. The sealed import supplies the model-library identity; configuration
cannot manufacture it or expand import coverage. Missing bindings/packages,
incompatible targets and rejected witnesses fail without provider fallback.

`SourceContractPolicies::load` loads policies for direct comparison without
requiring installed packages. `SourceContractRegistry::load` additionally verifies
and registers explicit process packages. Its `verify` method accepts an opaque
Clang-imported summary and returns a checked certificate plus contract evaluation
only after existing trusted replay succeeds. Source: `crates/qcore-audit/src/source_contract.rs`.
This implementation supports the existing bounded Boolean-field semantic profile;
it is not a general contract solver. See the [execution record](reports/w5-configured-source-contracts/README.md).

A binding may additionally declare `"frame_helpers": [{"source": "reset.c",
"function": "reset_metadata", "record": "widget", "contract": "reset.qv"}]`.
Omission means no helper selection. At most 256 helpers are allowed per binding,
with unique C function names; function/record identifiers are at most 128 ASCII
bytes. Source paths must be contained relative paths within the selected library
root. Policy paths resolve from the configuration directory and their snapshots
share the existing 32 MiB aggregate policy budget.

`import_configured_frame_helpers` requires exactly one selected compilation entry
per source, canonical source containment, an absolute compilation directory, and
strict preservation of supported preprocessing options. Unrepresented options
reject. Source reads are bounded at 8 MiB each and retained imported helper source
at 32 MiB per call; this is not a new bound on transient Clang preprocessing.
`import_configured_frame_helpers_compatibility` explicitly preserves the existing
pinned audit's historical option projection; there is no automatic fallback.
`configured_frame_helper_policies` in the audit sidecar records snapshotted
selection metadata, not proof that every configured helper was executed.

### Version 2: pinned contract bundle selection

Version 1 retains string policy paths. Version 2 accepts those paths and, for
binding or frame-helper `contract`, this strict object:

```json
{
  "manifest": "policy-bundle/contracts.json",
  "sha256": "<64 lowercase hexadecimal digits pinned by the consumer>",
  "owner": "qcore.cmetrics",
  "path": "contracts/cmetrics/counter_reset.qv"
}
```

`manifest` resolves relative to the configuration, while `path` is an exact key
in the existing provider inventory format (`schema_version: 1`, `plugin_id`,
`contracts` mapping paths to SHA-256). The consumer independently supplies the
manifest hash and expected owner. Loading validates the inventory hash, owner,
version, unique keys, exact recursive file set, and every file hash before
exposing immutable UTF-8 policy snapshots. Asset keys must be forward-slash paths
under `contracts/`, end in `.qv` or legacy `.qcore`, and contain no empty, dot, parent, backslash
or colon component. Symlink manifests, asset directories/files and special files
reject. Manifest parents are resolved as the explicitly selected location;
protect that directory from concurrent mutation during loading.

Limits: 1 MiB manifest, 1–4096 files, 8192 tree entries, 32 directory levels,
8 MiB per nonempty UTF-8 file and 32 MiB per complete bundle. Selected policy
copies also consume the service's existing 32 MiB aggregate snapshot budget.
Unknown fields, versions, paths, mismatches and failures reject without fallback.
Version 1 refuses bundle objects; unsupported service versions still reject.

`qcore_contract::bundle::ContractBundle::load` needs no executable or process
registry. A bundle identifies supplied policy bytes, not predicate truth,
authorship, provider capability or source authority. Existing source import,
policy-derived task identities and independent witness replay remain unchanged.
`SourceContractRegistry` still requires its selected executable packages; bundle
ownership does not implicitly select or register a process. The CMetrics source
configuration is the pilot; existing compatibility adapters are retained.

## Configured C audit composition

`qcore-audit` schema-1 configuration now accepts an optional `source_contracts`
object with `configuration`, `source_rules` and explicit `libraries` entries
(`library`, checkout-relative `source_root`). All are required within that object;
unknown fields reject. The exact configuration and helper-resolution rules are
in the [audit command documentation](../crates/qcore-audit/README.md#configured-source-contract-processes).
This input selects existing bounded Boolean source import and evidence-v2 replay;
it does not extend QCore source grammar or promote the C resource verdict.

## G09. Concurrent field-footprint questions

`source.bitfield-concurrency.v1` is a property-only checker. It requires
`#import std.concurrency; concurrency checked;`, explicit-layout records, and
one `unit` entry with effects `{spawn, join, mut}`. The entry declares one
object with `let OBJECT = TYPE;`, then one task scope, then `return;`.

```ebnf
layout_field = NAME, ":", ("field", "<", OFFSET, ",", SIZE, ">"
             | "bits", "<", UNIT, ",", BIT_OFFSET, ",", BIT_WIDTH,
               ",", BYTE_OFFSET, ",", BYTE_SIZE, ">") ;
```

The scope contains at least two `let H = spawn S WORKER(write OBJECT.FIELD)?;`
statements before any join, followed by one `let R = join move H?;` for each
handle in spawn order. Each used worker has one `i32` parameter, effect `{mut}`,
and exactly `write PARAM; return;`. Other declarations/statements refuse.
`bits` and `field` are contextual, not newly reserved words.

The checker derives layouts and field handles for `DisjointFootprintsV1`.
Same-unit bit-fields refuse at `Q661` regardless of disjoint bit ranges. Layout
truth remains a declared ABI premise. Admission establishes only pairwise
field independence in a **successful spawn/join window**; it does not establish
task failure recovery or lifetime safety. No C or reference execution is
published. See [the bounded profile and controls](reports/ct10-source-layout/README.md).

## G10. Existing C: generic selections

The existing-C resource gateway reads preprocessed C. A C11 generic selection
(§6.5.1.1) inside a function body is admitted by one route, *selection
invariance*, over the gateway's token view (string and character literals are
one token):

```ebnf
selection   = "_Generic", "(", control, ",", association, { ",", association }, ")" ;
association = ( type_name | "default" ), ":", expression ;
expression  = { "(" }, { pointer_cast }, core, { ")" } ;
pointer_cast = "(", IDENT, { IDENT }, "*", { "*" | qualifier }, ")" ;
qualifier   = "const" | "volatile" | "restrict" | "__restrict" | "__restrict__" ;
core        = IDENT, "(", arguments, ")" | IDENT | other_expression ;
```

A selection is admitted only when every association's `expression` has the
same `core` token for token after its enclosing parentheses and pointer casts
are removed; at most one association is `default`. A cast is removed only
when its operand is a whole parenthesised group, a direct call or a single
name. If any cast was removed, `core` must be a direct call `IDENT ( … )`
whose C declaration in the same unit returns an object pointer. An admitted
selection is lowered to `core` (parenthesised unless it is a call or a
name); its `control` is dropped, since C does not evaluate it. Selections
nested in an association are lowered first. Every other selection is left as
written and refuses where it did before, normally as a call to an
uncontracted function `_Generic`.

The admitted forms include glibc 2.43's C23 const-preserving macros
(`__glibc_const_generic`: `memchr`, `strchr`, `strrchr`, `strpbrk`, `strstr`,
`bsearch` and the wide-character searches), in any position: a statement, a
declaration initializer, a condition, a return or a call argument. The
lowered call then meets the ordinary classifier and its declared contract
exactly as the same call spelled without a selection. The Clang-resolved route
(reading the compiler's selected association) is not implemented.
See [the card report](reports/w44-c11-generic-selection/README.md) and
`QCore.GenericSelection`.


## G11. Existing C: indexed child/next lookup

The gateway's bounded read-only class matches this complete preprocessed body,
with alpha-renamed function, formal, local and named record/alias names:

```c
static T *F(const T *P, U N) {
    T *C = 0;
    if (P == 0) { return 0; }
    C = P->child;
    while ((C != 0) && (N > 0)) { N--; C = C->next; }
    return C;
}
```

`static` and the parent's leading `const` are optional. `T` is a complete,
unique named nonunion `struct TAG` or its unique defining-struct alias; both
members are unqualified single-level pointers to the same record. `U` resolves
to an unsigned integer class; opaque, pointer, Boolean, enum, floating, volatile
or atomic types refuse. Only zero and its parenthesized/void-pointer NULL forms
are normalized. Added calls/writes, different guards/steps/returns, deeper
pointers, qualified links and ambiguous/anonymous records refuse. Both original
source-bound typed reads must be present under the retained Clang context.
The original typed reads and alias/base/chain obligations remain intact. An
inference-only name-only site with no resource/status or conflicting facts may
be viewed as untracked at its source-bound read; only the cursor's checked
non-NULL while-body point supplies its narrowing. Real tracked sites keep
their original obligations, and a released peer prevents any borrowed view.
The inferred result is Borrowed,parent0,nullable,const_dropped, validated by
ordinary derived-result inference; it never becomes a fresh allocation.

Only an assigned direct `D = F(P, DECIMAL);` with decimal0–255 receives the
private caller identity descriptor; this bound avoids target-width truncation
for every admitted unsigned parameter. A literal has no redundant leading
zero, suffix, cast or sign. Nested/whole-return calls and other indices retain
the ordinary borrowed reading. Exact identity additionally needs must-held
physical links and one settled contained child uniquely owned by a live parent.
The caller actual and stored-link owners must resolve to the callee's complete
canonical defining record tag. Same field spelling or site is insufficient;
opaque caller type aliases withhold refinement. Link facts retain that tag,
and a differently typed write invalidates overlapping owner facts.
An absent link means Unknown; explicit NULL differs from absence. The source
bridge currently proves exact stored-head and empty-head cases. General
successor source closure, object-key lookup and Removed wrapper transport are
not implemented by this class. See [gateway rule13a](agents/gateway.md#constraint-13a)
and [the report](reports/detach-lookup-membership/README.md).

## G12. Existing C: checked call-free condition composition

The resource gateway can retain its complete Boolean plan for a controlling
expression made from existing call-free leaves, parentheses, `!`, `&&`,
`||` and the ordinary `A ? B : C` conditional, with C's existing precedence
and short-circuit placement. The plan is
retained when an older admission already applies or when at least one node is
an existing checked `IndirectScalarAccess`. This includes, for example:

```c
if ((p->fd != -1) && (p->map != NULL)) { /* existing supported statements */ }
```

Every leaf must already classify completely. Retention adds no leaf grammar,
cast, qualifier, pointer value, scalar value or call-effect rule. The original
read, alias/lifetime and per-hop obligations remain at their original positions;
unresolved or released bases cannot be made safe by retaining the plan. A
failed leaf refuses the complete plan and restores the classification probe.
Descriptor and switch guards remain. The same existing graph assembly places
admitted tests in `if`, `while`, `do` and `for` controls; it does not admit a new
value-expression position or infer scalar outcomes.

This closes a composition gap for older checked field readers, including the
Linux `(cf->fd != -1) && (cf->map != NULL)` residency predicate and a header
helper's two-pointer NULL conjunction. It supplies no callback neutrality,
ownership transfer or richer return transport. Existing source summaries still
require their ordinary complete body derivation and caller checks. See
[gateway rule5h](agents/gateway.md#constraint-5h),
[the card report](reports/w48-bottom-up-batch4/README.md), and the bounded
`QCore.CallFreeConditionReads` model. C extraction and Rust/model
correspondence remain explicit review premises.
