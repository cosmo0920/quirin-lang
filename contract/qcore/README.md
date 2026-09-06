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
