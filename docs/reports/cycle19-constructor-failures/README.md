# Cycle 19: native constructor allocation failures

The pinned, unmodified native memory path now runs with one injected ENOMEM at
each of nine allocation attempts, plus a successful control. The attempted
allocations are context storage, chmod copy, stream storage, stream name, chunk
storage, chunk name, memory backend, data buffer and initial metadata buffer.
There are 55 acquired generations across the ten executions: six releases during
constructor rollback and 49 through teardown of the surviving ownership tree.
All executions pass ASan, UBSan and leak checking. The original five positive
teardown scenarios also pass with the new optional allocation-failure hook.

| Failed attempt | Observed result | Rollback releases |
|---|---|---:|
| Context allocation | Null context | 0 |
| Chmod copy | Context succeeds with null chmod; children still constructed | 0 |
| Stream allocation | Null stream | 0 |
| Stream name | Null stream | 1 |
| Chunk allocation | Null chunk; error output retains harness sentinel | 0 |
| Chunk name | Chunk succeeds with null name | 0 |
| Backend allocation | Null chunk; error output is zero | 2 |
| Data buffer | Null chunk; error output is zero | 3 |
| Initial metadata | Metadata write returns -1; live chunk has no metadata | 0 |

The observer preserves native return and error behavior. It stops constructing
children after a null parent and always tears down the surviving context. A
successful chunk with a null name is only used for metadata and destruction;
this result does not establish safety of later name-consuming operations. The
missing chmod copy is observed on a memory-only context. No filesystem behavior
is inferred. Diagnostic output in `native.stderr` is the expected ENOMEM logging;
no sanitizer finding was emitted. These are bounded outcome observations, not
validated whole-unit defects or general API guarantees.

The adapter checks exact allocation attempts, injected failures, fresh identities,
nonoverlapping live storage, constructor rollback order, returned identities and
error/metadata outputs. It then invokes the shared reciprocal-ring, ownership
and exact nested teardown checker on the remaining live allocations. The null
context has a separate exact empty teardown trace. Missing/extra attempts, wrong
rollback frees, fabricated success/error values, aliased/null fields outside the
selected case, stale generations and incorrect terminal state reject.
The 11-clause QCore policy is runtime evaluation only. Two new Lean theorems
model the final partitioned release plan: rollback and teardown jointly conserve
its length and cannot release the same generation in both phases. The native
observer, allocator hooks, exclusive harness, compiler and Python adapter remain
trusted; their implementation is not proved by those abstract theorems.

All fourteen selected ChunkIO/CRC units and the observer are rebuilt with
ASan/UBSan. Library function instrumentation and linker allocation wrappers are
recorded with source/build/dependency/binary hashes; shared observer dependencies
are now included in the generated dependency manifest too. System libraries
retain their build. Failures occur only during construction, with one injected
failure per case; there are no combined failures, filesystem resources, metadata
replacement, realloc failure or concurrent callers in this profile.

Validation passes: 400 workspace tests, 45 contract modules, two audit-example
tests, seven Python tests (48 mutation subcases plus duplicate/reordered release
and JSON-key checks), formatting, Clippy with warnings denied, both reevaluation
probes, Lean build/Cycle 19 axiom audit, both native sanitizer runners and the
pinned external audit. Source, diagnostic and contract evidence remains identical
to the preceding audit. Aggregate verdicts remain 2 Safe, 98 provisional Unsafe,
81 Unknown, four data-only units and zero gateway failures across 185 units.

Reproduce the native run and regression checks from the repository root:

```sh
python3 scripts/cycle19_constructor_failure_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-constructor-failures
python3 scripts/cycle19_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-positive-regression
python3 -B -m unittest discover -s scripts/tests -v
```

LeakSanitizer requires execution outside ptrace. Run the remaining standing gates
from the [preceding report](../cycle19-native-teardown/README.md), using
`RUST_TEST_THREADS=1 cargo test --workspace`. This package archives both native
results, the failure trace fixture, validation logs, parity and hashes.

Cycle 19 remains in progress. General caller and other failure-path provenance,
full filesystem sync and real OS guarantees, unsupported backend classification,
general aggregate custody, remaining wrapper lifecycle and full Monkey/Cycle 18
event composition remain open. This matrix narrows the native failure observation
gap; it cannot close those broader gates or promote any raw unit.
