# QCore contract regression corpus

These native QCore sources are the durable, source-readable form of the
external-library contracts used by the compiler's Fluent Bit integration.
They are grouped by library and kept independent so each file can be checked
directly with `qcorec`.

The regression test in `compiler/tests/qcore_contract_corpus.rs` recursively
discovers every `.qcore` file, parses its function contracts into structured
clauses, and runs the complete compiler driver. It also pins the expected
contract-bearing declarations for each source, so deleting or accidentally
weakening a fixture is a test failure.

From the repository root:

```sh
cargo test -p qcore-compiler --test qcore_contract_corpus
cargo run -p qcore-compiler --bin qcorec -- check \
  contract/qcore/chunkio/import_composition.qcore
```

The clauses are retained in the source AST for future semantic passes. Today,
the driver checks their grammar and preserves their spans and expressions; it
does not yet use arbitrary contract predicates to mint a safety certificate.

Two contracts now have a bounded semantic check against actual C source:
`fluent_bit/busy_state.qcore` and `chunkio/lock_state.qcore`. Their function
names, preconditions, return relations, and frame conditions are defined here;
the Rust checker interprets these files rather than duplicating their
postconditions in Rust tables.

The source audit runs Clang, imports the complete supported function body,
and checks every `ensures normal` clause over all four entry combinations of
the Boolean ghost fields `busy` and `lock`. `old_busy` and `old_lock` denote
entry values; `busy` and `lock` denote exit values; `result` is the C integer
return. The bounded predicate language supports integer literals, these five
ghost names, addition, subtraction, parentheses, and `==`/`!=`. Unknown
predicates and unsupported clause kinds are rejected, not assumed true.

The live-chunk, exclusive-access, and Boolean-domain `requires` clauses are
caller assumptions. Source checking does not establish them at arbitrary call
sites. The imported body may access only the selected field; this establishes
the other field's frame condition in this bounded model. A negative C return
is still an `ensures normal` exit, not a QCore typed exception.

Run the semantic source checks with the existing Fluent Bit build database:

```sh
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- ../fluent-bit
```

Look for `qcore-contract ...: Checked` for both functions. The audit additionally
prints an explicit `Unknown` for every unmodeled function in `cio_chunk.c` and
`flb_input_chunk.c`. Unsupported source bodies never receive a field summary.
These two contract checks do not promote either translation unit to `Safe`.
The ordinary `qcorec check FILE.qcore` command continues to perform the compiler
and contract-syntax checks; use the audit above to bind these contracts to C.

Every contract source now begins with `module contract;` and compiles without
an artificial `main`. `module library;` similarly checks helper definitions
without a process entry. See [module modes and closed `where` bindings](../../docs/QCORE_MODULE_MODES.md)
for syntax, multi-file behavior, compiler API changes, and current boundaries.

`fluent_bit/memory.qcore` is also evaluated on executions of the actual
`include/fluent-bit/flb_mem.h`, compiled with Fluent Bit's include/define
profile and allocator observation hooks. Its five functions passed 145
recorded executions, including deterministic allocator failures. This is
runtime contract evaluation, separate from the two static Boolean-field
source checks above. See the [native evaluation report](../../docs/reports/cycle19-native-contracts/README.md)
for reproducible commands, assumptions, and exclusions.

The recent Monkey execution policies also have native semantic consumers:
`monkey/callback_binding.qcore` contains 15 replay rules and
`monkey/descriptor_observation.qcore` contains five descriptor identity rules.
These 20 contracts have 56 postconditions and are executed by the Rust binding
and observation checkers through the shared predicate interpreter. They cover
scope, typed memory/symbols, complete and unique replay, call results and order,
map release, injected-failure labeling, descriptor generations and terminal
custody. Mutating any of these contracts changes the corresponding checker
result; unsupported predicates fail closed.

These parameterless functions are validation helpers over interpreter-derived
ghost facts. Concrete C resource APIs and callback postconditions remain in
`monkey/composition.qcore`. Rust retains the schema decoder, expression and
state-transition implementation, representation bounds, and conservative source
grammar. Compiling a QCore file alone checks syntax; the native runner connects
these rules to recorded executions. The [native-rule report](../../docs/reports/cycle19-native-rules/README.md)
records eight exercised callback paths, the labeled injected failure, bounded
wrapper source projections and remaining composition gates.

## Declarative resource-projection contracts

`../cycle19-resource-contracts.json` explicitly selects five modules for the
legacy-C audit: `libc/memory.qcore`, `monkey/memory.qcore`,
`librdkafka/memory.qcore`, `fluent_bit/resource_memory.qcore`, and
`chunkio/resource_open_close.qcore`. These 19 declarations replace the audit's
Rust table. They are trusted resource-projection assumptions, separate from the
more detailed execution and lifecycle profiles above. In particular the
ChunkIO close declaration does not itself provide a nested teardown certificate.

The version-1 vocabulary begins with `requires resource_contract_v1`. Normal
postconditions use `resource_borrowed(p)`,
`resource_{owned,consumed,released,out}(p, nullable|nonnull)`, or
`resource_{stored,removed}(p, owner, nullable|nonnull)`. An aggregate owner must
be a distinct owned parameter. Each parameter has exactly one mode; out requires
a pointer-to-pointer. Results use `resource_return_none()`,
`resource_return_owned(nullable|nonnull)`, or
`resource_return_reallocated(p, nullable|nonnull)` with an owned input.

This is the existing abstract resource model: borrowed arguments have no
custody requirement, null reallocation failure preserves old custody, and
successful replacement invalidates it once. Pointer types are opaque ABI
projections rather than checked C layout declarations. Unsupported requirements,
effects rows, error/any/quant clauses, non-null error sentinels, unknown modes,
duplicates and incomplete mappings reject the entire selected environment.
Do not add these clauses to an observation module and expect either interpreter
to ignore clauses it cannot check.

`contract_loader::load_resource_contract_manifest` loads these files at runtime;
`load_resource_contract_files` and `load_resource_contract_sources` expose the
same strict loader for other selections and immutable snapshots. Certificates
retain the exact sources and reload them during independent checking. The audit
writes the same inputs to `RESOURCE_REPORT.contracts.json` alongside a requested
resource inventory; its optional sixth positional argument overrides the module
selection manifest. Missing or unloadable input terminates the audit as an
error, without reporting a safety result from a default environment.

## Source-validated chunk helper dependencies

The bounded chunk-field importer can compose a helper only after independently
checking its complete body as constant writes to ordinary int metadata fields,
with a matching external declaration, matching direct record field layout, and
the same borrowed parameter identity. The pinned audit supplies the source,
build and cross-unit ABI identity; the importer does not prove linker resolution
or complete ABI equivalence. Helper source text travels in the field summary
and native contract certificate. This is a bounded dependency check, not general
SCC/fixed-point custody inference.

`chunkio/lock_state.qcore` now includes `cio_chunk_unlock`. Its `field_error_n`
postcondition checks the source-validated `cio_error_reset` write on both exits.
Every helper-written field requires a `field_NAME` postcondition; absent or
unsupported ghost values reject the contract. Const borrows reject helper writes
as well as direct field writes. `ChunkState` models busy/lock lifecycle effects;
metadata writes remain in the source paths and native contract result.

When a resource inventory path is supplied, the pinned audit also writes
`RESOURCE_REPORT.chunk-contracts.json`, containing all 96 selected native
chunk-function results, exact checked QCore contracts and dependency sources.
Unknown results remain in that inventory. The generic resource report and its
Safe/Unsafe/Unknown classifications are separate and unchanged by these checks.
