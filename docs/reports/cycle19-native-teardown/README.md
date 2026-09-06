# Cycle 19: native constructor and memory teardown observations

Five single-threaded executions call the unmodified pinned ChunkIO constructors
and `cio_destroy`: an empty context, two empty streams, one memory chunk, six
memory chunks in two streams, and six chunks with metadata. The native path
constructs and releases 82 allocation generations and 13 chunks in total.
All fourteen selected ChunkIO/CRC translation units are rebuilt with ASan/UBSan,
libc allocation wrappers and compiler function hooks. Leak checking passes.
The filesystem-enabled build profile is preserved, but these cases exercise no
filesystem backend or OS cleanup operation. System libraries retain their build.

The observer records allocation generations, sizes and addresses, complete
context/stream/chunk/backend ownership snapshots, and embedded list sentinel,
node and reciprocal link addresses. The independent Python adapter checks fresh,
nonoverlapping owned storage, exact parent references, complete primary/up rings,
empty down rings and initial counters. It derives the entire nested destructor
event sequence from this tree, then compares every release, hook, unlink-state
snapshot and counter observation. No allocation may remain unowned or be freed
in the wrong order. Empty rings have no invented child calls. Null fields incur
no allocation-release obligation; the harness uses only the optional chmod field
on contexts and, in the final case, live metadata on chunks.

The adapter feeds five records to an 11-clause QCore runtime policy. Five Lean
theorems establish that the abstract exact-plan gate preserves generation order,
count and membership, rejects a changed sequence and has no duplicate releases.
They do not verify the Python adapter, native instrumentation, C compiler or
allocator. The pinned source/build, observer completeness and exclusive harness
execution remain trusted. Ring layout assumes the recorded 64-bit Linux ABI.
This evidence binds concrete caller/constructor/ring inputs for these five
executions only; it is not an exhaustive caller proof or a static certificate.

Validation: 400 workspace tests, 44 contract modules, two audit-example tests,
five Python tests (28 mutation subcases plus duplicate/reordered release and JSON
key checks), formatting, Clippy with warnings denied, both reevaluation probes,
Lean build and Cycle 19 axiom audit pass. Workspace tests use
`RUST_TEST_THREADS=1` to avoid the known process-fixture spawn race. The pinned
source, diagnostic and contract evidence is identical to the prior nested-cleanup
report: 185 units, 2 Safe, 98 provisional Unsafe, 81 Unknown, four data-only units
and no gateway failures. No raw unit is promoted.

Reproduce from the repository root:

```sh
python3 scripts/cycle19_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-native-teardown
python3 -B -m unittest discover -s scripts/tests -v
RUST_TEST_THREADS=1 cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
```

LeakSanitizer must run outside a ptrace sandbox. The native runner preserves the
read-only upstream checkout and records compilation commands, dependency hashes,
binary/observer/adapter/contract hashes and runtime results. `trace.jsonl` also
serves as the replay regression fixture. Pinned audit invocation is the same as
the preceding report, with `/tmp/cycle19-native` as output prefix.

Cycle 19 remains in progress. Allocation-failure paths, general caller provenance,
full filesystem sync and real OS guarantees, unsupported backend classification,
aggregate source custody, remaining wrapper lifecycle and full Monkey/Cycle 18
event composition remain open.
