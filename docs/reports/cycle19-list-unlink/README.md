# Cycle 19: source-derived intrusive-list unlink

The pinned ChunkIO build now supplies complete typed paths for `mk_list_del`
and its `__mk_list_del` helper. Native contracts in
`contract/qcore/chunkio/list_unlink.qcore` check reconnection of both neighboring
link cells, clearing both entry links, the write footprint, exactly one helper
call and no object release. The checker expands the checked helper body at the
source-derived arguments; it does not assume effects from a callee name.

Both pointer-alias classes are replayed: distinct previous/next neighbors and
previous = next, as in a single element linked through one list head. Entry is
distinct from both. The admitted language contains only bound pointers, direct
link cells, reads/writes, null pointers and the checked helper call. It cannot
inspect numeric addresses or create other pointer identities; the two classes
therefore cover this bounded alias domain up to identity renaming. Missing
cells, wrong arguments, opaque/recursive calls, partial effects, extra writes
and unsupported control flow fail closed.

The certificate retains both source projections, exact QCore inputs and both
four-write traces. Live reciprocal membership, exclusive access and C/QCore
layout binding remain caller premises. Helper parameters are raw pointers with
cell-access premises so equal neighbor nodes are allowed without asserting two
exclusive whole-object borrows. Unlinking preserves allocation custody; it does
not itself free the containing chunk or prove an arbitrary list traversal.

All **354 workspace tests** and the 28-module native corpus pass. Tests mutate
neighbor identity, helper writes, null clearing, effect count, unsupported
branches/loops, field qualifiers, signatures/premises and every postcondition.
Formatting, warnings-denied Clippy, audit example tests, both reevaluation
probes, Lean build and Cycle 19 dependency audit pass. Six new Lean theorems
establish entry clearing, neighbor reconnection, singleton-head restoration
and the frame for other nodes. C/Clang, source-to-IR translation and the Rust
replay remain trusted boundaries; this is not a proof of arbitrary C.

The pinned audit records the certificate under `backend_teardown` with
`kind: list_unlink`. Generic resource inventory, the full Monkey sidecar,
all 96 scalar-function results, the memory-backend certificate and all four
counter certificates remain identical to the counter baseline. No whole-unit
verdict changes: 185 units; 4,688/4,775 projections; two Safe, 98 provisional
Unsafe, 81 Unknown, four data-only and zero gateway failures. Projection is not
safety coverage, and provisional Unsafe is not a confirmed upstream defect.

See `parity.json`, `chunk-contracts.json.gz`, validation logs and `evidence.json`.

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-unlink-source.json /tmp/cycle19-unlink-diagnostics.json
```

Cycle 19 remains in progress. Applying this result to both concrete embedded
chunk links, composing the backend and saved-context counter call in chunk
close, and the remaining filesystem/stream/context and Monkey composition
workstreams are still open. Native observation instrumentation was unchanged.
