# QCore library and contract modules

A `.qcore` file can declare its compilation intent before its declarations:

```qcore
module contract;

extern fn flb_free(ptr: ptr<mut, u8>) -> unit
contract {
    requires valid_input_extent == 1;
    ensures normal live_after == 0;
};
```

`module contract;` checks declarations and supported function bodies without
requiring, selecting, or generating `main`. It retains the structured contract
clauses for the appropriate semantic checker. It does **not** prove arbitrary
predicates or grant trust to an external function.

`module library;` has the same entry-free checking behavior and allows a file
of helper function definitions to be checked independently. A function named
`main` in either mode is an ordinary checked definition, never an implicit
process entry. These modes currently produce checked QCore modules, not C
library objects or a public C ABI; `qcorec emit` reports that boundary before
writing any output.

`module executable;` explicitly selects the existing executable behavior.
Files with no module declaration keep that behavior, including `--entry NAME`
and the existing single-function entry fallback. A missing or ambiguous entry
in an executable is still an error.

The module declaration is optional, unique, and must precede declarations.
Comments and whitespace may surround it. Unknown modes, missing semicolons,
repeated headers, and misplaced headers are rejected.

For multiple input files, an executable file selects executable compilation;
otherwise a library file selects library compilation; otherwise all files
are contracts. Contract declarations can therefore accompany libraries or an
executable. This is one shared module namespace, not a new import/name-mangling
system. Duplicate declarations remain errors. `--entry` on an entirely
library/contract compilation is rejected (`Q403`); include an executable file
to select an entry. Every supported function body still goes through the same
semantic checks in each mode.

```sh
cargo run -p qcore-compiler --bin qcorec -- check \
  contract/qcore/fluent_bit/memory.qcore
cargo run -p qcore-compiler --bin qcorec -- check \
  contract/qcore/language/quantified_indices.qcore
```

All files under `contract/qcore` now use `module contract;`; none needs a dummy
`main`. The Rust `CompiledModule` records `mode`; `entry`, `c_source`, and
`reference_observation` are `Option`s and are absent for non-executable modules.
Source maps remain empty when no executable C artifact exists.

# Closed quantitative `where` bindings

The source front end now resolves definitions inside quantitative types:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;

extern fn keep(q: quant<n where n := 5> use one)
    -> quant<credit, { chunks = total }
             where total := base + 2, base := 3, total <= 5>;
```

Both quantitative types resolve to five `Count<Chunk>` credits. Bindings are
local to the containing type. The same name can have another definition in a
different type; it does not become a runtime variable. Definitions may refer
to later definitions in the same `where` clause.

The implemented subset supports credit and live vectors, scalar shorthand
when exactly one credit axis exists, decimal natural constants, addition,
literal scaling, `min`, `max`, `monus`, and the six ordinary comparisons.
Comma or `and` separates clauses. `monus(a, b)` requires `b <= a`; arithmetic
must fit `u128`. Binder dimensions follow the axes in which they occur.
Repeated fields, duplicate/shadowing/cyclic bindings, unknown names, mixed
dimensions, false constraints, and overflow are rejected. Runtime calls,
pointer/field expressions, and nonlinear products are outside this subset.

The typed source IR retains resolved fields and local bindings as dimensioned
`IndexTerm`s. This is a closed-definition elaboration feature, not general
dependent function typing, symbolic inference from runtime values, or a proof
of quantitative transfers through arbitrary bodies. Cost total/peak types and
extensions to the separate legacy `quant<access, ...>` profile remain outside
this addition. The native regression fixture is
`contract/qcore/language/quantified_indices.qcore`.
