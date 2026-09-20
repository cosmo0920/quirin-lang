# Cycle 18 observed composition integration

Base: `a524f4d95eaafd0dc5e879b070b9cf4c0e1a5e08`.
Status: implementation in progress, no acceptance promotion.

Three GPT-5.6 Sol agents at medium reasoning implement independent composition
context admission, sealed timed-wait source projection, and identity-bearing
runtime observation. The integrator owns configured audit wiring, cross-module
review, final gate, evidence reconciliation and publication.

## First executable milestone

A dedicated observed execution of the pinned Monkey timed-wait path is bound
to its callback and wrapper source, compile/build/profile identities, observer
and linked libraries, exact run records and lifecycle/argument identities.
The checker consumes the complete dedicated trace. It never filters unrelated
callbacks out of the older combined replay or promotes supplied booleans.

A successful artifact describes only its named run, selected path, assumptions
and bounds. Unobserved allocation/registration failures remain Unknown. The
existing three-callback accounting and Cycle 18 aggregate acceptance stay open
until their own source/path and platform obligations are discharged.

## Subsequent capability work

`cb_event` still requires all-path payload identity/bounds and write authority.
`cb_timeout` requires self-teardown and separate event/map/descriptor custody.
General callback/task handoff needs enforced spawn/join/cancel semantics;
currently declared task/deferral flags do not establish those properties.
Native Libevent sanitizer coverage and the deployed observation adapter need
host-specific evidence; a Linux run cannot stand for kqueue or Windows IOCP.

The parent `MONKEY-CALLBACK-CYCLE18-COMPOSITION` remains registered and open.
Five narrower follow-ups record the next discharge boundaries:
`CYCLE18-TIMED-WAIT-FAILURE-PATHS`, `CYCLE18-CB-EVENT-PAYLOAD-BOUNDARY`,
`CYCLE18-CB-TIMEOUT-SELF-TEARDOWN`, `CYCLE18-CALLBACK-TASK-HANDOFF` and
`CYCLE18-NATIVE-PLATFORM-MATRIX`. Their acceptance-matrix entries state explicit
discharge conditions. No existing follow-up is retired by this observed run.

## Integration requirements

Reuse the existing generic event kernel and sealed reactor imports. Keep new
semantic work in the layered crates, with matching audited Lean. Context
identity equality is a binding check, not attestation of an observation producer.
Preserve all existing verdicts and pinned evidence unless a separately scoped
property is actually discharged. No change to full language conformance is
implied. Reports from each card supply the reconciliation list and exact gate
commands, exits, counts and private log paths.

## Contract boundary

The source projection consumes the existing `monkey/wrapper_paths.qcore` and
`monkey/composition.qcore` declarations. Their exact selected clauses and byte
digests are bound into the sealed projection and the observed-run context.
Primitive matching of these predicates against imported C operations remains
the trusted Rust implementation; accepting contract text alone proves nothing.
The callback's borrowed, no-resource-effect declaration is checked against its
sole-return source body before constructing a callback model.

The configured entry point is `quirin-audit observed-wait CONFIG.json`. It reads
the pinned source/profile inputs, source compilation database, contracts,
Libevent build configuration, native run manifest and exact observation/binding
payloads. It constructs a fresh context from these configured inputs and checks
the sealed projection, identity-bearing observation and complete event model.
The result is `observed_composition_checked`, not a universal `Safe` verdict.
The normal `quirin-audit CONFIG.json` route retains its existing behavior.

## Explicit trust and formal limits

The native producer supplies runtime observations and the selected artifacts'
linkage evidence. Hash comparisons bind those artifacts; they do not attest
the producer, host, executable execution or link process. The native harness
wraps Libevent calls and the original callback, records a fixed dispatcher-local
buffer, and flushes after teardown. Its activation observation is inferred from
the timeout flags at callback entry. Generation 1 describes a single lifetime,
not a general pointer-reuse detector. Fresh model names preserve the recorded
full-width native identities in separate singleton namespaces.

Interposed `event_new` registers the observer trampoline and dispatcher buffer
with Libevent; the trampoline forwards delivery to the original callback and
captured source argument. The trace records those logical source identities,
not the literal callback/argument pair stored inside the instrumented Libevent
event. Instrumentation transparency remains a premise; this is not an
uninstrumented production-execution certificate.

Lean modules model source obligations, context equality, exact identity-bearing
records, singleton identity renaming and the selected legal lifecycle. Native
truth, Clang import, cryptographic hashing and Rust/C/Lean implementation
refinement remain external boundaries. No theorem here establishes all-path
safety of the Monkey translation unit.

## Executed integration evidence

The fresh Linux x86_64 observer ran against the pinned Libevent source at
`d82464a277d0f42703702c4dfd9af6af38595a83`, compiling both the observer and
`libevent_core.a` with Clang address/undefined sanitizers and running with leak
detection enabled. [Native command and exit](native-run.json),
[artifact identities](native/run-manifest.json), [complete records](native/observations.jsonl)
and [bindings](native/bindings.json) are retained. Thread support is configured
in the build, but this harness neither links the pthread adapter archive nor
calls `evthread_use_pthreads`; the checked model is strictly single-dispatcher.
This run establishes no cross-thread safety or whole-Libevent sanitizer matrix.

The [configured audit](observed-config.json) produced a successful
[bounded composition report](observed-report.json), checking eleven records
and eleven event operations with the original full-width native identifiers
retained in its explicit mapping. The exact configured paths are host-local;
native objects, executable and build logs remain under
`private/cycle18-observed-integration/native/`.

The shipped [negative-control runner](check_negative_controls.py) changes
callback generation, removes a record, invents synchronization, substitutes
the backend, changes callback borrowing to ownership, and substitutes an
artifact digest. Payload hashes are recomputed so semantic mutants reach the
semantic gate. [All six controls](negative-controls.json) exited 1 and emitted
no composition report; each individual log is archived beside that record.
The Rust join test additionally covers source-summary/contract context swaps,
wrong bounds, changed raw bytes and an identity greater than `u32::MAX`.

Strict compile ingestion gained the exact native flags `-ffunction-sections`,
`-fdata-sections` and `-fsanitize=address,undefined`, transported to Clang.
A focused test checks both sanitizer preprocessor features. Paired `-MMD -MF`
dependency output switches remain in compile-task identity but are omitted
from the analysis invocation, like object output switches, so auditing cannot
overwrite retained dependency files; missing/duplicate pairs refuse. Other
sanitizer spellings still refuse. The first end-to-end attempt refused `-MMD`
before that explicit support was added; the second exited 0. An earlier WIP
compile failed before the agent modules were exported; neither failure is
counted as a passing gate.
