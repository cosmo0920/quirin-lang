# Cycle 19: unsupported backend close classification

The remaining nonnull `cio_chunk_close` path now has an explicit conditional
`Unknown` classification. For every integer stream type unequal to the two
source-probed backend constants, the source skips backend cleanup, unlinks both
embedded nodes, releases the nullable name and chunk, and decrements the saved
context's total counter. It does not change the up counter or establish backend
discharge or backing deletion. Constructor reachability remains a separate gate;
this conditional path is not a validated reachable violation.

The existing source interpreter now carries the symbolic class of all integers
excluding the memory and filesystem constants. It does not sample a third value
and generalize that execution. Comparisons with additional constants, unsupported
symbolic operations and truth tests that divide this class reject. Memory and
filesystem replay retain their original release requirements. The external audit
requires this classification to account for exactly its previously unclassified
close path, without changing the scope of either existing certificate.

The 12-clause QCore policy states live/disjoint allocation and parent provenance,
reciprocal exclusive lists, positive counters, dependency/ABI/free bindings and
absent wrapper obligations. It records only conditional partial effects. Four
Rust tests include 14 source mutations, changed source constants and false/unknown
policy mutations. Eight Lean theorems prove preservation of opaque backend
obligations and the up counter, exact total-counter subtraction, rejection of
zero/duplicate release and refusal to discharge unclassified custody. The Lean
model does not prove the Rust interpreter, native provenance or Clang semantics.

All 404 workspace tests, 46 contract modules, two audit-example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency audit
and the pinned external audit pass. Every prior source, diagnostic and contract
record is identical after removing the one added classification. Aggregate
verdicts remain 2 Safe, 98 provisional Unsafe and 81 Unknown across 185 units,
with four data-only units and zero gateway failures. No raw unit is promoted.
The first workspace run detected the new corpus entry at the wrong
position in the ordered expectation table; the entry was placed in its required
lexical position and the full gate rerun.

Reproduce from the repository root using the standing commands in
[the traversal report](../cycle19-teardown-traversal/README.md), with
`RUST_TEST_THREADS=1 cargo test --workspace` and these audit output arguments:

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-unsupported/source.json /tmp/cycle19-unsupported/diagnostics.json
```

Cycle 19 remains in progress. Opaque residual propagation through parent release,
native caller/failure provenance, full filesystem guarantees, aggregate source
custody, remaining wrapper lifecycle and full Monkey/Cycle 18 event composition
remain open. This classification does not close those gates.
