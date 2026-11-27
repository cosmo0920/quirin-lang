# Cycle 19D: native callback binding and Lean models

On 2026-09-06, all three callbacks from the pinned forced-Libevent Monkey
adapter refined their recorded native executions and satisfied their native
QCore callback predicates under the supplied bindings. **This checks the
recorded executions; it does not issue a static event-composition certificate.**

| Callback | Checked executions | Observed source path index | Checked postconditions | Unobserved path indices (`Unknown`) |
| --- | ---: | ---: | ---: | --- |
| `cb_event` | 1 | 1 | 2 | 0, 2, 3 |
| `cb_timeout` | 1 | 0 | 3 | 1, 2 |
| `cb_wait_2_timeout` | 1 | 0 | 1 | None |

Path indices refer to the accompanying freshly imported source report. An
observed path index does not prove that path for every possible input or heap.
Even the no-effect callback's result assumes the recorded registration and
exclusive dispatch boundary.

## Binding and contract interpretation

A separate schema-version-1 JSONL sidecar binds every event creation to its
actual callback argument and native event pointer. Every callback execution
references its exact entry/exit sequence numbers in the **same** existing
48-record descriptor/event observation. The checker reuses the generation,
base, backend, dispatcher, thread, registration, and teardown checks from that
observation; it does not invent a second adapter execution for Cycle 18.

The hooks capture actual typed pre/post snapshots and intercept native calls.
`cb_event` binds argument → event map → context → bounded fired array, including
all slots in that array, and checks the fd/mask/data stores and count increment.
`cb_timeout` binds the map's writer and event, its stack send buffer contents
and length, errno-location result/read, and cleanup calls. Successful descriptor
close, event deletion, and event free must each consume exactly one matching
runtime effect in order. The map-free hook removes that map's snapshot cells;
subsequent accesses or duplicate release cannot refine. Freed maps are never
read by the post-snapshot hook.

Ordinary reads and writes are reconstructed from the imported source between
snapshots under exclusive access. They are **not** individually instrumented
memory accesses. Native snapshots are supplied evidence of live borrowed
storage; this does not establish allocation provenance or authenticate the
observer. Raw object addresses are scoped to event registrations and callback
intervals, not promoted to global allocation identities. Descriptor identities
retain their separately checked generations.

The Rust replay evaluator supports the required bounded typed symbols, locals,
casts, scalar expressions, guards, memory cells, arrays, and call effects. It
uses an LP64 scalar profile and rejects out-of-range arithmetic, absent or
released cells, wrong types, unsupported expressions, and incomplete traces.
A unique path must reproduce the entire retained post-state and call stream.
Unknown source imports or replay failures remain per-callback `Unknown` results;
malformed or inconsistent observation/binding envelopes are rejected.

The existing `contract/qcore/monkey/composition.qcore` clauses are parsed and
checked by the shared contract interpreter. The binder derives facts about
borrowed arguments, fired-array writes, preserved/released map custody,
permitted descriptor close, and no-effect callbacks. Borrowed argument liveness
is an observer precondition, not a proof obtained from pointer equality.
Changing a native predicate changes the verdict; unsupported predicates fail
closed. No duplicate Rust per-function postcondition table is introduced.

## Lean verification

`QCore.CallbackBinding` supplies executable abstract binding operations and
**20 audited theorems** covering:

- registration/observation scope equality and rejection of changed scope;
- unique refining path selection, required obligations, and explicit retention
  of unobserved paths;
- live typed read binding, exact symbol production, and absent-symbol rejection;
- typed writes to present cells, exact-cell updates, and preservation of other
  cells;
- live-object release, removal of every cell in that object, preservation of
  other objects, and rejection of subsequent reads, writes, or duplicate release;
- exact consumption of the next observed call record.

The module is imported by the normal Lean library build and `AuditCycle19.lean`.
The dependency audit contains no placeholders or project axioms. Its listed
standard dependencies are `propext` and `Quot.sound`.

These theorems verify the abstract binding models. They do not prove the Rust
interpreter, JSON decoder, Clang importer, native hook bridge, authentication,
full C semantics, or allocator provenance. The memory model uses mathematical
integers; the Rust interpreter additionally checks its bounded C scalar ranges.
The acceptance model requires already established guard, memory, call, and
contract results; it does not infer these results from booleans supplied by a
caller to the Rust API. Rust accepts private, freshly imported source summaries.

## Validation and evidence

All **318 workspace Rust tests** pass. Six new regression groups cover the real
observed callbacks; missing/duplicate/stale/cross-event bindings; changed memory,
flags, errno, buffers, and call order; missing/rebound runtime effects; source
and QCore predicate mutations; and per-callback source `Unknown` handling.
Formatting, Clippy with warnings denied, Lean build, and the Cycle 19 dependency
audit pass.

The sanitized native run still checks all 145 allocator executions and the
48-record descriptor trace, including four acquisitions/closes, two descriptor
number reuses, and one injected socketpair failure. Address, undefined-behavior,
and leak sanitizers pass for the harness and included code; linked archives
retain their original instrumentation. Expected stderr remains `socketpair:
Too many open files` and `write: Broken pipe`.

The pinned external audit remains at 185 units, **4,688/4,775 functions projected,
2 Safe, 98 provisional Unsafe, 81 Unknown, four data-only, and zero gateway
failures**. Provisional Unsafe results are not confirmed upstream defects.
Binding results remain separate from generic coverage counts.

- [Native binding result](monkey_binding_result.json), [binding sidecar](monkey_bindings.jsonl)
- [Imported source paths](monkey_callback_source.json), [pinned audit](source-audit.log)
- [Shared identity observation](monkey_observation.jsonl), [identity result](monkey_identity_result.json)
- [Build/source/hook metadata](metadata.json), [artifact and implementation hashes](evidence.json)
- [Rust tests](tests.log), [Clippy](clippy.log), [formatting](fmt.log)
- [Lean build](lean.log), [20-theorem dependency audit](lean-audit.log)
- [Allocator result](flb_mem_contract_result.json), [native stderr](monkey_observation.stderr)

## Reproduction and remaining gates

```sh
python3 scripts/cycle19_native_observations.py \
  --fluent-bit ../fluent-bit \
  --monkey-build /tmp/qcore-cycle19-monkey-build-codex-20260905 \
  --output /tmp/qcore-cycle19-binding-reproduction

cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cd proof
lake build
lake env lean AuditCycle19.lean
```

The native runner now invokes the pinned source audit and binding checker after
the identity check, records source/binding/contract hashes, and requires at least
one checked execution of each callback. The audit's optional fifth positional
argument accepts a directory containing `monkey_observation.jsonl` and
`monkey_bindings.jsonl`; it writes `monkey_binding_result.json` there. The runner
requires the previously documented disposable forced-Libevent build and an
environment supporting socketpair and sanitizers.

19D's supplied-execution callback binding is implemented. Five unobserved
callback paths, allocation/registration failure source imports, and complete
wrapper/map custody retain explicit `Unknown` reasons. No Cycle 18 event-program
composition or plugin `CoreCertificate` is produced. General 19B source identity
mapping and 19C lifecycle/teardown imports also remain open. Cycle 19 remains in
progress; generic projection warnings are not treated as its completion gate.
