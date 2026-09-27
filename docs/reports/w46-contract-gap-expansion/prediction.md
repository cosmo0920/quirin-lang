# W46 contract gap expansion: pre-implementation prediction

Base: `9bce3986dd5703faa6d1fc2f485dc88a20902fdd`. The user requested
additional libc, Libevent, Monkey, CMetrics, CTraces, input-chunk and flushing
event-management contracts. Astra performs gap selection and integration;
three isolated Sol 6, medium-reasoning cards implement independent families.

The audit compares exact uncontracted-call diagnostics with tracked contract
declarations and reads candidate bodies at both Fluent Bit revisions
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and
`046accf6aded7906f66a1baf0202da349e521736`. Declaration absence, declaration
selection, unsupported ABI/effects and checked-but-unestablished specifications
are different gaps. The census is a diagnostic inventory, not a complete API
catalogue or a safety-coverage claim.

Expected additions are scalar queries, scalar updates with no reachable custody
change, and precisely modelled derived results. CMetrics scalar metric getters
and conversions, CTraces ID length/comparison, Monkey event metadata and Fluent
Bit chunk/output/task metadata are the initial candidates. libc additions must
not duplicate built-in handling without a stated benefit. All non-libc contracts
remain pinned; libc declarations remain explicit flexible ABI assumptions.

Prediction: some named uncontracted-call leaves disappear, but whole-function
moves depend on the remaining blockers and are not promised. No new resource
kind, generic analyzer admission, callback composition or production lifecycle
proof is planned. Leak/violation changes are not expected; any such change must
be attributed rather than absorbed. Existing refuted output obligations remain
refuted and their callers remain held. Corpus and system-slice counts remain
separate. Each implementation card records a narrower prediction before editing.

Open boundaries carried into this work: retained callbacks/data; backend-selected
event registration and waits; refcounted and in-place lifecycle operations;
CMetrics/CTraces encoder-owned subobjects; input-chunk remapping and buffer aliases;
flush coroutine/task handoff and destruction; existing OR-1/OR-7 refutations;
and the distinction between a reviewed source pin and a proof of a C body.
