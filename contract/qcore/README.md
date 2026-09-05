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
