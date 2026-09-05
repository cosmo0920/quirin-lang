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
`lib/monkey/mk_core/deps/libevent`. The five source-contract files trusted by
the Cycle 18 model are byte-identical to the separately pinned
`third_party/libevent` checkout at
`d82464a277d0f42703702c4dfd9af6af38595a83`. This equality is checked by the
audit; it is not a claim that every packaging, test, or generated file in the
two trees is identical.

## Gateway changes

The unrestricted-C gateway now has an explicit dialect gate for `c11`,
`gnu99`, `gnu11`, and `gnu17`. C11 remains the default. Arbitrary compiler
arguments are not accepted by the verifier.

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
| CMetrics | 31 | 0 | 0 | 540 | 521 | 96.5% | 0 | 16 | 15 |
| CTraces | 17 | 1 | 0 | 274 | 264 | 96.4% | 1 | 9 | 6 |
| librdkafka 2.15.0 | 94 | 3 | 0 | 3,410 | 2,984 | 87.5% | 0 | 10 | 81 |
| ChunkIO | 13 | 0 | 0 | 126 | 121 | 96.0% | 0 | 7 | 6 |
| Fluent Bit input chunk | 1 | 0 | 0 | 66 | 63 | 95.5% | 0 | 0 | 1 |
| Monkey, forced Libevent | 29 | 0 | 0 | 359 | 342 | 95.3% | 1 | 19 | 9 |

All 185 selected translation units pass the production Clang gateway under
their gated GNU dialect and build definitions. Of 4,775 raw-source functions,
4,295 receive a partial resource summary. The four units without a discoverable
function body pass the same preprocessing and syntax gate but are kept outside
the Safe/Unsafe/Unknown resource-unit counts. Partial-result retention raises
the projected numerator for the already selected CMetrics, CTraces, and Monkey
slices without converting any incomplete unit into a `Safe` result.

These numbers are projection results, not memory-safety coverage. In
particular, an `Unsafe` result at this stage is not evidence of an upstream
defect. The current resource projection does not yet express owned parameters,
aggregate/container custody, arena ownership, or `realloc`, so it can report a
local obligation as leaked when ownership actually moves into one of those
structures. `ctr_version.c`, which is resource-neutral, is the single fully
`Safe` CTraces unit.

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
`rd_strndup` as acquisitions and `rd_free` as release. It deliberately leaves
`rd_realloc` unmodeled because treating it as a fresh allocation would lose
the preserve-on-failure obligation. The 10 `Unsafe` and 81 `Unknown` unit
results are therefore projection classifications, not librdkafka defect
reports. Refcounted objects, typed queue macros, aggregate custody, callbacks,
thread handoff, atomics, and realloc semantics remain outside this stage.

`tinycthread.c` contains 30 raw-source function bodies, but this build defines
`WITH_C11THREADS`, so none survive preprocessing. The unit remains fail-closed
as `Unknown (0/30)` rather than being called data-only or safe.

## Initial chunk boundary

The chunk evaluation deliberately spans both layers: all 13 built ChunkIO
units and Fluent Bit's `flb_input_chunk.c`. The wrapper uses 14 `cio_chunk_*`
operations covering open/close, up/down residency, locking, content access,
writes, size queries, and transaction rollback. Its operational lifecycle is
roughly create or map, move up/down, append or write, flush and lock, release,
then destroy and close. The gateway projects 63 of 66 raw wrapper functions;
`append_to_ring_buffer` is one reduced-parser failure and two conditional
definitions do not survive this build profile.

This is only the start of chunk evaluation. The generic resource contracts
recognize `flb_malloc`/`flb_calloc`, `flb_free`, SDS creation/destruction, and
ChunkIO open/close. They do not yet connect `struct flb_input_chunk` custody to
its `struct cio_chunk`, route mask, task, trace, list membership, memory/file
backend, busy flag, lock state, filesystem accounting, or deletion policy.
Consequently the wrapper stays `Unknown`, and none of these numbers constitutes
a chunk lifecycle, crash-consistency, routing, or concurrency certificate.

## Monkey event-model boundary

The adapter calls seven Libevent APIs already accepted by Cycle 18:

```text
event_add, event_base_free, event_base_loop, event_base_new,
event_del, event_free, event_new
```

Three public descriptor utilities are not in the current Cycle 18 importer and
therefore remain `Unknown`:

```text
evutil_closesocket, evutil_socket_geterror, evutil_socketpair
```

Consequently, passing the C gateway does not produce a Cycle 18 event-safety
certificate for Monkey. The adapter uses `event_base_new`, so the runtime
Libevent backend is not pinned by the call itself, and the current importer
does not yet carry Monkey's `struct mk_event`/`struct ev_map` custody,
socket-pair close authority, nullable event acquisition, callback argument
aliases, or callback-to-fired-array effects into the event IR. The aggregate
event verdict remains fail-closed as `Unknown` until those contracts are
modeled and the selected backend is observed or constrained.

## Next resource-model gates

The next stage should be implemented and validated in this order:

1. import `for`, `switch`, `goto` cleanup, labels, `break`, and `continue` into
   the CFG rather than consuming them as unknown blocks;
2. distinguish owned, borrowed, consumed, and out-parameter function
   contracts;
3. add aggregate fields, typed queues, refcounts, and intrusive-list custody,
   including destructor summaries;
4. add nullable acquisition and path refinement;
5. model `realloc` as preserve-on-failure and replace-on-success;
6. model thread/callback transfer and atomic publication before evaluating
   librdkafka concurrency;
7. add a chunk-specific state model for memory/file residency, lock/busy/task
   custody, route accounting, transactions, and close/delete outcomes;
8. add CFL arena parent/child lifetime discharge; and
9. rerun the same pinned local audit before expanding beyond the central
   Fluent Bit chunk wrapper.

For the Monkey event slice, the corresponding gates are to add explicit
semantics for the three `evutil_*` calls, import the wrapper's event and
descriptor custody, refine nullable Libevent acquisitions and failed
registration paths, summarize `cb_event`, `cb_timeout`, and
`cb_wait_2_timeout`, and bind runtime backend evidence to the Cycle 18 build
profile before claiming event-model coverage.

Only functions whose resource obligations are fully discharged may contribute
to memory-safety coverage. `Unknown` and provisional `Unsafe` functions remain
outside the proved numerator.
