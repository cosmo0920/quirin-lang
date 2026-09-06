# Cycle 19: native replay contracts and expanded Monkey paths

On 2026-09-06, the recent callback-binding and descriptor-observation policies
were extracted into **20 executable native QCore contracts, containing 56
postconditions**. The expanded sanitized harness checks all eight imported
callback paths on supplied executions. The source audit additionally retains
37 symbolic paths from eight adapter wrappers, including allocation and
registration failure exits. Cycle 19 remains in progress.

## Native contracts are active checker inputs

- [`callback_binding.qcore`](../../../contract/qcore/monkey/callback_binding.qcore)
  defines 15 policies for callback scope, typed memory and symbol binding,
  complete replay, unique path selection, bounded send/errno/cleanup results,
  map-release identity, source-call arguments, ordered call intervals, injected
  failure labeling, and unique runtime-effect consumption.
- [`descriptor_observation.qcore`](../../../contract/qcore/monkey/descriptor_observation.qcore)
  defines five policies for socketpair outcomes, fresh descriptor generations,
  live identity use, successful close without an active I/O callback, and
  terminal discharge of bases, descriptors, events and dispatch obligations.
- [`composition.qcore`](../../../contract/qcore/monkey/composition.qcore)
  already contained the actual callback postconditions and general conditional
  descriptor-custody contracts. Those remain distinct from the narrower
  execution profile, which rejects ambiguous native close failure.

The first two modules declare validation helpers over interpreter-supplied
ghost facts, rather than new C resource APIs. Rust decodes records, maintains
state, resolves typed source expressions and supplies those facts to the shared
QCore predicate interpreter. The acceptance predicates above live in QCore;
the former Rust policy comparisons were replaced by native contract calls.
Missing declarations, unsupported predicates, false postconditions, and unmet
preconditions reject the attempted operation. Mutation regressions exercise
every extracted contract with both a false and an unknown predicate.

The interpreter still enforces concrete storage availability, representation
limits, source grammar, schema validity and executable state transitions.
Those implementation mechanisms are not claims that arbitrary QCore syntax or
arbitrary C has been formally verified. The APIs accepting custom rule text
produce execution reports only; the supplied policy must be retained with the
evidence. The normal APIs load the checked-in QCore modules. Build metadata
records both native rule hashes and the existing callback contract hash.

## Expanded callback evidence

| Callback | Executions | Exercised path indices | Paths with native call results | Injected failure paths | Checked callback postconditions |
| --- | ---: | --- | --- | --- | ---: |
| `cb_event` | 4 | 0, 1, 2, 3 | 0, 1, 2, 3 | None | 8 |
| `cb_timeout` | 3 | 0, 1, 2 | 0, 2 | 1 | 9 |
| `cb_wait_2_timeout` | 1 | 0 | 0 | None | 1 |

The original readable-channel execution is retained. Three additional
`cb_event` cases use actual Libevent `event_active` dispatch with write,
read/write, and timeout flags. These are controlled activations, not evidence
that the OS reported each readiness combination. The timer additionally sends
successfully to a live reader. One explicitly labeled `injected_failure` send
returns -1 with `ECONNABORTED`, exercising native cleanup without `perror`.
This is a controlled failure case, not an observed OS failure. The original
real broken-pipe send and cleanup are retained.

The result separately records `native_paths`, `injected_paths`, and
`injected_executions`; checked paths are their union. Origin labels, like the
snapshots themselves, are supplied by the trusted observer and are not
authenticated. No exercised path is universally proved for all inputs/heaps.
No static event or plugin certificate is issued, even when `unknown_paths`
is empty.

The shared Cycle 18/19 observation now has **84 records**, three `epoll` bases,
six descriptor acquisitions and closes, four descriptor-number reuses, and
one injected socketpair failure. The harness passes address, undefined-behavior
and leak sanitizers. Linked static archives retain their original build's
instrumentation. Expected stderr remains `socketpair: Too many open files`
and `write: Broken pipe`. The allocator regression still checks 145 executions.

## Wrapper source failure paths

The importer retains function signatures, typed function references, symbolic
fixed-record `sizeof`, explicit casts, comparisons, arithmetic and returned
values. Calls remain opaque. Non-void fallthrough, unsupported statements on
unreachable paths, and unresolved escaped locals remain Unknown.

| Wrapper | Symbolic source paths |
| --- | ---: |
| `_mk_event_init` | 1 |
| `_mk_event_loop_create` | 4 |
| `_mk_event_loop_destroy` | 1 |
| `_mk_event_update` | 24 |
| `_mk_event_timeout_destroy` | 3 |
| `_mk_event_channel_destroy` | 2 |
| `_mk_event_wait_with_flags` | 1 |
| `_mk_event_backend` | 1 |

`_mk_event_loop_create` retains initialization failure, context-allocation
failure, fired-array-allocation failure and return of the context. The source
does not test the `event_base_new` result. Its fired-array failure path records
`mk_mem_free(ctx)` without inventing a base-free call. `_mk_event_update`
retains event-new and registration failures across all flag/type alternatives;
the source's old-event cleanup precedes creation, and a registration failure
does not gain an invented cleanup of the newly created event. These are
source-path facts, not discharged custody results or confirmed upstream defect
claims. No wrapper is promoted to Safe by this projection.

Seven wrappers remain explicitly Unknown: socketpair has an unsupported enum
reference; add/del contain assertion `do` statements; timeout-create,
channel-create and wait-2 contain unsupported local arrays/aggregates; inject
contains a loop. Every one of the 15 selected wrappers has a report entry.
The binder's separate Unknown about allocation/registration failure coverage
still applies: it accepts callback summaries, not these wrapper projections.

## Verification and remaining gates

All **323 Rust tests**, formatting, Clippy with warnings denied, Lean build,
the Cycle 19 dependency audit, sanitized native execution and pinned external
audit pass. Three additional Lean theorems preserve the distinction between
native and injected path evidence, bringing the binding module's audited
theorem count to 23. The audit has no placeholders or project axioms; its
binding theorems depend only on `propext` and `Quot.sound`; the full cycle audit
also lists standard `Classical.choice` for the existing descriptor-reuse theorem. It does not prove
the Rust interpreter, Clang bridge, native observer or full C semantics.

The generic pinned audit is unchanged: 185 units, 4,688/4,775 projected
functions, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only, and
zero gateway failures. Those figures are projection coverage, not proved
memory safety. Provisional Unsafe is not a confirmed upstream defect.

Remaining work includes source-to-identity custody mapping for 19B, ChunkIO and
Fluent Bit lifecycle/teardown imports for 19C, the seven unsupported Monkey
wrappers, source/runtime binding and custody discharge of wrapper failure
paths, complete event-map provenance, and full Cycle 18 event-program
composition. Exercised callback paths do not discharge those gates.

```sh
python3 scripts/cycle19_native_observations.py \
  --fluent-bit ../fluent-bit \
  --monkey-build /tmp/qcore-cycle19-monkey-build-codex-20260905 \
  --output /tmp/qcore-cycle19-native-rules-reproduction
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cd proof
lake build
lake env lean AuditCycle19.lean
```

The runner requires the previously documented disposable forced-Libevent build
and an environment permitting local sockets and LeakSanitizer. This run used
an approved execution outside ptrace sandboxing because LeakSanitizer cannot
operate under that sandbox. No upstream source files were modified.

Evidence: [binding results](monkey_binding_result.json),
[source and wrapper paths](monkey_callback_source.json),
[identity results](monkey_identity_result.json),
[shared observation](monkey_observation.jsonl),
[binding sidecar](monkey_bindings.jsonl), [metadata](metadata.json),
[pinned audit](source-audit.log), [Rust tests](tests.log),
[Lean dependency audit](lean-audit.log), and [hash manifest](evidence.json).

Saved text logs normalize trailing whitespace and final blank lines.
