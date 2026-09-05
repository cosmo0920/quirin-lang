# Cycle 19 Fluent Bit library and chunk validation

## Scope and source identity

This is a local verification run. It does not add, require, or modify a CI
workflow.

The adjacent Fluent Bit checkout was audited at:

```text
66910c10a4d7eafa810b84229cf2dcf7b0f26f97
v5.1.1-127-g66910c10a
```

The audit rejects another `HEAD` and tracked changes under the selected
library and chunk source trees.

The targets are the top-level C translation units for CMetrics and CTraces,
every built C translation unit under librdkafka 2.15.0 and ChunkIO, and Fluent
Bit's central `src/flb_input_chunk.c` wrapper that are present in the generated
`build/compile_commands.json`. Cycle 19 also audits the non-dependency Monkey
translation units in a separate build with `MK_EVENT_LOOP_LIBEVENT=ON`.
Generated and platform sources absent from these builds are not included. The
three generated librdkafka OpenTelemetry tables and empty `ctr_utils.c` have no
discoverable source function body; they still pass the standalone
preprocessing and syntax gate.

The bundled librdkafka version is identified by `RD_KAFKA_VERSION` as 2.15.0.
The ChunkIO import is v1.5.5, introduced into this Fluent Bit history by
`b3a19e516ae6173566e6442d083c4261edbc5bd2`.

The Monkey tree at that Fluent Bit revision is:

```text
b40a80d97588a29a3906398e087dea4084523410
Monkey v1.8.9 import (Fluent Bit commit 1f10023cfbb25115eadb6c16498c2003c7a9c2bb)
```

Monkey bundles a Libevent source snapshot under
`lib/monkey/mk_core/deps/libevent`. The seven source-contract files trusted by
the Cycle 18 model are byte-identical to the separately pinned
`third_party/libevent` checkout at
`d82464a277d0f42703702c4dfd9af6af38595a83`. This equality is checked by the
audit; it is not a claim that every packaging, test, or generated file in the
two trees is identical.

## Gateway changes

The unrestricted-C gateway now has an explicit dialect gate for `c11`,
`gnu99`, `gnu11`, and `gnu17`. C11 remains the default. Arbitrary compiler
arguments are not accepted by the verifier.

The Cycle 19 19A rerun replaces the audit's name-only allocation annotations
with explicit parameter/result contracts. Known allocators return nullable
owned custody, null-tolerant destructors say so explicitly, and `rd_realloc`
has preserve-on-failure/replace-on-success semantics. Recognized null tests
refine those alternatives on separate CFG edges. A contract mismatch remains
`Unknown`; the checker does not assume that an allocation succeeded merely
because a later call consumes its result.

The reduced importer now preserves `for`, `do`/`while`, `switch`/`case`
fallthrough, labels, cleanup `goto`, `break`, and `continue` as explicit CFG
edges. Unresolved targets and calls embedded in control expressions remain
`Unknown`. Aggregate contracts can store children under a unique owner,
cascade destruction/transfer, and restore direct custody through checked
removal; wrong-owner removal is a custody conflict.

The library audit extracts only include paths, preprocessor definitions, and
the allowlisted language dialect from each compilation-database entry. When a
command has no explicit `-std`, the audit records GNU17, matching the GNU C
default used by the local GCC 15 build. Definitions containing unresolved CMake
or Make `$` substitutions are not forwarded. Because the production gateway
preprocesses through standard input, the audit also restores the main source
file's directory as the first quoted-include search location. This is required
for Monkey's `mk_event.c`, which includes its selected backend implementation.

The gateway can also restrict proof scope to functions defined in the main
source file. Functions introduced by expanded headers remain available to
Clang for parsing and type checking, but their bodies do not block the selected
source-file projection. Function discovery is file-scope aware so block macros
such as `cfl_list_foreach(...) { ... }` are not mistaken for function
definitions.

Returned local allocations are modeled as ownership transfers. This closes the
simple factory case while leaving call-in-return expressions fail-closed.

For the larger librdkafka units, the projection now tokenizes a preprocessed
translation unit once and reuses that token stream for every selected
function. A malformed reduced-parser body no longer discards successfully
projected sibling functions: the aggregate verdict remains `Unknown`, with a
partial summary. When the first raw-source function is compiled out by the
selected definitions, source-scoped analysis uses the first surviving selected
definition. Conditional C++ linkage blocks are ignored for raw C function
discovery. Translation units without a function body have a public syntax-only
gateway and cannot receive a resource-safety certificate.

## Reproduction

From the QCore-C root, CMetrics, CTraces, librdkafka, ChunkIO, the Fluent Bit
input-chunk wrapper, and the Monkey source-contract gate can be reproduced
with:

```sh
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- ../fluent-bit
```

The existing Linux Fluent Bit build auto-selects Monkey's epoll implementation;
it does not compile `mk_event_libevent.c`. To exercise the Libevent wrapper
rather than accidentally reporting epoll coverage, create an out-of-tree
profile from a disposable source copy:

```sh
monkey_source=$(mktemp -d /tmp/qcore-cycle19-monkey-src.XXXXXX)
monkey_build=$(mktemp -d /tmp/qcore-cycle19-monkey-build.XXXXXX)
cp -a ../fluent-bit/lib/monkey "$monkey_source/monkey"
cmake -S "$monkey_source/monkey" -B "$monkey_build" \
  -DCMAKE_BUILD_TYPE=Debug \
  -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
  -DMK_EVENT_LOOP_LIBEVENT=ON \
  -DMK_WITHOUT_BIN=ON \
  -DMK_WITHOUT_CONF=ON \
  -DMK_PLUGIN_DIRLISTING=OFF \
  -DMK_PLUGIN_MANDRIL=OFF \
  -DMK_TLS=OFF \
  -DEVENT__DISABLE_OPENSSL=ON \
  -DEVENT__DISABLE_MBEDTLS=ON \
  -DEVENT__DISABLE_TESTS=ON \
  -DEVENT__DISABLE_REGRESS=ON \
  -DEVENT__DISABLE_BENCHMARK=ON \
  -DEVENT__DISABLE_SAMPLES=ON
cmake --build "$monkey_build" --parallel 2
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit "$monkey_build/compile_commands.json"
```

The audit requires the `mk_event.c` command to contain
`MK_EVENT_LOOP_LIBEVENT` and byte-compares every selected copied `.c` file with
the adjacent Fluent Bit checkout before analysis. The process does not modify
Fluent Bit.

## Local result

| Slice | Built units | Data-only units | Gateway failures | Source functions | Projected functions | Projection coverage | Safe units | Unsafe units | Unknown units |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| CMetrics | 31 | 0 | 0 | 540 | 525 | 97.2% | 0 | 17 | 14 |
| CTraces | 17 | 1 | 0 | 274 | 264 | 96.4% | 1 | 3 | 12 |
| librdkafka 2.15.0 | 94 | 3 | 0 | 3,410 | 3,025 | 88.7% | 0 | 10 | 81 |
| ChunkIO | 13 | 0 | 0 | 126 | 122 | 96.8% | 0 | 5 | 8 |
| Fluent Bit input chunk | 1 | 0 | 0 | 66 | 64 | 97.0% | 0 | 1 | 0 |
| Monkey, forced Libevent | 29 | 0 | 0 | 359 | 346 | 96.4% | 1 | 18 | 10 |

All 185 selected translation units pass the production Clang gateway under
their gated GNU dialect and build definitions. Of 4,775 raw-source functions,
4,346 receive a partial resource summary. The four units without a discoverable
function body pass the same preprocessing and syntax gate but are kept outside
the Safe/Unsafe/Unknown resource-unit counts. Partial-result retention raises
the projected numerator for the already selected CMetrics, CTraces, and Monkey
slices without converting any incomplete unit into a `Safe` result.

Compared with the initial name-only-contract run, nullable contracts first
moved 15 resource translation units from provisional `Unsafe` to `Unknown`.
The explicit control-flow rerun recovers 51 additional function projections
and reclassifies eight units from `Unknown` to provisional `Unsafe`, for 54
Unsafe and 125 Unknown units overall; the two fully Safe units are unchanged.
These classifications remain conservative model results, not defect claims.

These numbers are projection results, not memory-safety coverage. In
particular, an `Unsafe` result at this stage is not evidence of an upstream
defect. The generic resource projection expresses direct owned parameters,
aggregate store/removal, and `realloc`. The audit now derives mechanism
profiles from pinned source contracts for librdkafka refcounts and typed
queues, CFL lists and arenas, and the nested ChunkIO teardown chain.

The executable composition checker handles counted references,
intrusive-list/typed-queue/arena transfers, and evidence-gated publication,
handoff, and reclamation. It pins imported profiles to the audited Fluent Bit
revision and rejects mechanisms outside the declared bundled-library profile.
Per-function source paths are not yet translated into concrete identity-indexed
operations, so the generic projection can still report a local obligation as
leaked when custody moves through one of these mechanisms.
`ctr_version.c`, which is resource-neutral, is the single fully `Safe` CTraces
unit.

For Monkey, `tls_stub.c` is the single fully `Safe` unit. The 29 units are the
`mk_core`, `mk_server`, and selected `tls` library translation units; API test
executables, the server binary, plugins, and bundled dependencies are outside
this slice. The dependencies are governed separately by the Cycle 18
source-contract audit or remain future library-audit scope. `mk_event.c`
passes the gateway with the Libevent implementation expanded and projects all
14 source functions, provisionally as `Unsafe`; this is an ownership-model
result, not evidence of an upstream defect.

## librdkafka boundary

This stage follows the exact 94 C commands selected by Fluent Bit's local
build, including bundled compression, hashing, JSON, nanopb, OpenTelemetry,
threading, SSL, SASL, and unit-support translation units. It restores the
source-file quoted include location and forwards only the allowlisted dialect,
include paths, and resolved definitions. All 94 pass both preprocessing and
Clang syntax validation.

The resource profile recognizes `rd_malloc`, `rd_calloc`, `rd_strdup`, and
`rd_strndup` as nullable acquisitions and `rd_free` as a null-tolerant release.
`rd_realloc` now preserves the old identity on a null result and replaces it on
a non-null result. The 10 `Unsafe` and 81 `Unknown` unit results are still
projection classifications, not librdkafka defect reports. Librdkafka-specific
source contracts now establish refcount, typed-queue/intrusive-list,
queue-destructor, and mutex-mediated queue-handoff summary availability.
Per-call identity mappings, callbacks, atomic ownership publication, and
reclamation remain outside this audit run. Scalar atomic operations alone are
not treated as ownership-transfer evidence.

`tinycthread.c` contains 30 raw-source function bodies, but this build defines
`WITH_C11THREADS`, so none survive preprocessing. The unit remains fail-closed
as `Unknown (0/30)` rather than being called data-only or safe.

## Initial chunk boundary

The chunk evaluation deliberately spans both layers: all 13 built ChunkIO
units and Fluent Bit's `flb_input_chunk.c`. The wrapper uses 14 `cio_chunk_*`
operations covering open/close, up/down residency, locking, content access,
writes, size queries, and transaction rollback. Its operational lifecycle is
roughly create or map, move up/down, append or write, flush and lock, release,
then destroy and close. The gateway projects 64 of 66 raw wrapper functions;
the other two remain outside the projected summary under this build profile.

This is only the start of chunk evaluation. The generic resource contracts
recognize `flb_malloc`/`flb_calloc`, `flb_free`, SDS creation/destruction, and
ChunkIO open/close. They do not yet connect `struct flb_input_chunk` custody to
its `struct cio_chunk`, route mask, task, trace, list membership, memory/file
backend, busy flag, lock state, filesystem accounting, or deletion policy.
Consequently the wrapper stays `Unknown`, and none of these numbers constitutes
a chunk lifecycle, crash-consistency, routing, or concurrency certificate.

In particular, a future ChunkIO `Safe` mark must check the nested teardown
chain rather than attach a release annotation only to `cio_chunk_close`.
`cio_destroy` traverses and destroys streams, `cio_stream_destroy` closes all
linked chunks, and `cio_chunk_close` dispatches to the memory or file backend
before unlinking and releasing the chunk. `cio_memfs_close` and
`cio_file_close` discharge different nested resources. Conversely,
`cio_stream_delete` and `cio_chunk_delete` have delete-specific partial
post-states and are not interchangeable with complete object teardown.

## Monkey event-model boundary

The adapter's ten Libevent calls are now in the bounded event profile:

```text
event_add, event_base_free, event_base_loop, event_base_new,
event_del, event_free, event_new, evutil_closesocket,
evutil_socket_geterror, evutil_socketpair
```

The Cycle 19 extension gives the three descriptor utilities explicit branch
and custody semantics. A successful `evutil_socketpair` creates two distinct
owned descriptors; failure creates none. `evutil_socket_geterror` is borrowed
and cannot revive a descriptor already closed in the trace.
`evutil_closesocket` discharges close custody only on success, so Monkey paths
that ignore a failed close retain an obligation. Duplicate closes and closing
an active callback descriptor are rejected. The pinned contract audit now also
checks `include/event2/util.h` and `evutil.c`, both byte-identical in Monkey's
bundled Libevent and the Cycle 18 checkout.

Recognizing all called API names still does not produce a Cycle 18 event-safety
certificate for Monkey. A new composition gate requires exact callback
summaries for `cb_event`, `cb_timeout`, and `cb_wait_2_timeout`; event-map
custody; the pinned Fluent Bit source revision; nullable base/event and
failed-registration paths; an accepted build;
event-program base creation matching that build; one matching dispatcher; a
matching observed backend; and an observation trace that refines the checked
event trace. The adapter uses `event_base_new`, so the
runtime backend is not pinned by the call itself. The current source audit does
not yet derive the complete wrapper trace, and no deployed observation was
captured in this run. The aggregate therefore remains fail-closed as `Unknown`.

## Bounded ChunkIO lifecycle result

Cycle 19 now includes a separate executable `chunk_lifecycle` checker. Its
bounded corpus covers memory- and filesystem-backed teardown, up/down
residency, list unlinking and counter balance, lock/busy/task/route custody,
transaction rollback, construction failure, and stream-delete partial states.
The model follows the concrete `cio_destroy` → `cio_stream_destroy` →
`cio_chunk_close` → backend-close chain.

The checker deliberately records a filesystem close with deletion enabled as
a delete request, not a confirmed deletion: `cio_file_close` returns `void` and
only logs native deletion failure. It separately records the observable result
of recursive stream-directory deletion. Memory-backed close never claims a
filesystem deletion.

The lifecycle checker is now behind an importer-composition gate. The gate
checks the pinned Fluent Bit revision, requires complete imported control flow,
requires the exact summary for every emitted lifecycle operation, and then
invokes the lifecycle checker. The unrestricted-C importer does not yet derive
all of those operations from the pinned source automatically. Accordingly, the
audit table above is unchanged and no raw ChunkIO or Fluent Bit unit is newly
marked `Safe`.

## Next resource-model gates

The checker-side custody and composition boundaries and the first pinned
library source profiles are implemented. Remaining work is per-path extraction
and real evidence, in this order:

1. map each audited typed-queue, intrusive-list, refcount, arena, destructor,
   and mutex-mediated handoff call to concrete custody identities;
2. require separate synchronization evidence for atomic publication and
   reclamation paths rather than deriving it from scalar atomic primitives;
3. derive the implemented ChunkIO operation trace from every selected wrapper
   path and retain `Unknown` for an incomplete path;
4. add CFL arena parent/child source mappings; and
5. rerun the same pinned local audit before expanding beyond the central
   Fluent Bit chunk wrapper.

For the Monkey event slice, the remaining evidence is a source-derived wrapper
operation trace satisfying the exact callback/failure/custody profile and a
deployed observation binding its dispatcher and selected backend to the Cycle
18 profile. The checker rejects either omission.

Only functions whose resource obligations are fully discharged may contribute
to memory-safety coverage. `Unknown` and provisional `Unsafe` functions remain
outside the proved numerator.


## Callback source-path increment (2026-09-06)

The current audit also imports the three included Monkey callbacks from the
forced-Libevent `mk_event.c` build. It emits four `cb_event` source paths, three
`cb_timeout` paths, and one `cb_wait_2_timeout` path. The optional third argument
to `cycle19_fluent_bit_libraries_audit` saves their typed access/call report.
The copied adapter and core layout/macro headers are explicitly byte-checked
in addition to the standalone translation units.

These are source projections, not callback or custody certificates. The audit
prints the remaining custody, failure-path, and runtime access refinement gates
for each imported callback. The native observation regression still checks the
48-record descriptor trace and 145 allocator executions, but it does not yet
bind the new symbolic payload/map/fired-slot accesses to those observations.
The table's generic projection and Safe/Unsafe/Unknown counts are unchanged.
See [the callback source report](../reports/cycle19-callback-source/README.md).
