# Cycle 19: native unsupported-type constructor outcomes

Four pinned, unmodified native executions pass `INT_MIN`, `-1`, `2` and `INT_MAX`
to stream creation and then attempt to open a chunk. Each stream retains the
requested type. Each chunk open returns null, preserves the caller's `-999`
error sentinel and releases its chunk name and object during rollback. The
surviving context and empty stream then tear down normally. Across the four
executions, all 24 allocation generations are released: eight during rollback
and sixteen during final teardown. The trace records no acquired backend
allocation; the surviving tree has zero total and up chunk counters.

This distinguishes the observed constructor results from the preceding
conditional close classification. Those four calls do not return a live chunk
that can enter the unsupported-close path. They do not prove that every possible
caller, mutated stream type, build profile or failure combination is excluded.
The conditional source classification remains `Unknown`, with opaque backend
custody preserved through parent release. No whole-unit violation is inferred.

The independent adapter checks exact allocator kinds, sizes, fresh generations,
nonoverlapping storage, rollback order, returned context/stream/type/chunk/error,
reciprocal empty rings, counters and complete nested release order. The shared
tree checker allows a non-memory stream only with zero chunks, so this extension
cannot interpret an unknown backend as memory. Floating-point/Boolean substitutes
for recorded integer fields reject in the new profile. An 11-clause QCore runtime
policy checks the derived facts. Three Lean theorems concern the abstract final
recorded-result/release-partition gate; they do not prove the observer, Python
adapter, compiler, or native constructor semantics.

All fourteen selected ChunkIO/CRC units and the observer are rebuilt with
ASan/UBSan. Leak checking passes outside ptrace. The four new cases emit no
stderr; the allocation-failure regressions retain expected ENOMEM logging.
The original five positive memory cases and ten constructor-failure/control cases
also pass all sanitizers using the extended shared adapter. Compilation commands,
source/dependency/compiler/binary/observer/checker hashes and traces are archived.
Pinned source/build, complete instrumentation and exclusive harness execution
remain trusted. The recorded ABI uses 64-bit pointers and 32-bit integers.

Reproduce from the repository root:

```sh
python3 scripts/cycle19_unsupported_constructor_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-unsupported-constructors
python3 scripts/cycle19_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-unsupported-positive-regression
python3 scripts/cycle19_constructor_failure_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-unsupported-failure-regression
python3 -B -m unittest discover -s scripts/tests -v
```

All 407 workspace tests, 47 contract modules, two audit-example tests, ten Python
tests (including 21 new trace mutations), formatting, warnings-denied Clippy,
both reevaluation probes, Lean build/dependency audit and the pinned external
audit pass. All prior source, diagnostic and contract evidence is identical.
Aggregate verdicts remain 2 Safe, 98 provisional Unsafe, 81 Unknown, four
data-only units and zero gateway failures across 185 units.

Run the standing Rust/Lean and pinned external audit gates from the
[preceding report](../cycle19-nested-unsupported/README.md). Cycle 19 remains in
progress: general caller/failure provenance, full filesystem guarantees,
aggregate source custody, wrapper lifecycle and full Monkey/Cycle 18 event
composition remain open. No raw unit is promoted.
