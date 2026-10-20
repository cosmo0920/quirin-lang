# W48 bottom-up batch 4 — checked call-free predicate composition

Base `2d8f6d34257db14031a2578ce14f941bee25fc19`; branch
`codex/w48-bottom-up-batch4`. Prediction commit `5cdf4b594` precedes semantic
implementation `b931ed61d`. The [prediction](PREDICTION.md) remains frozen.
The exact-base release analyzer was preserved before implementation; its
build HEAD `35aaecd3e` changes report text only. [Build provenance](base-binary.json)
discloses the HEAD and immutable binary digest.

## Selection and checked boundary

The prior ranked residual puts `rd_kafka_log0` first (score 3). Actual pinned
source confirms `rd_kafka_log_buf` allocates an op and log string, writes op
fields and either enqueues or calls `conf->log_cb`. `rd_kafka_q_enq1` recursively
forwards, stores serving callbacks and either transfers or destroys/replies to
the op. The neutral prefix-closed recursion theorem cannot justify that
positive effect. No neutral callback, consumed-op declaration or assumed
acyclic forwarding graph is introduced. Scheduler cleanup (score 2) waits on
TLS-retained `libco_params`, list unlink, `mk_thread_destroy`'s `cb_destroy`
dispatch and `co_delete` before frame release. These remain real dependencies,
not cheap one-leaf closures. The next score 1 cJSON object-detach wrappers return
a Removed result after a string-keyed getter. The getter traverses child/next
and string comparisons; the existing bounded array-lookup frame supplies
neither string lookup membership nor richer Removed return transport.

The next ready score 1 body is `cio_file_is_up`. Its actual Linux source expands
to `(cf->fd != -1) && (cf->map != NULL)`. Both leaves already have checked
readers, but the call-free plan was discarded when neither leaf needed the
newer Clang access-path reader or elaborator. A baseline synthetic control
reproduced Unknown with the exact branch refusal; production conformance is
measured separately under its own actual declarations. All readiness source
hashes are [recorded](readiness-sources.json).

The new Rust rule retains a complete, otherwise refused call-free condition
plan when one node is an existing `IndirectScalarAccess`. Each original leaf
instruction, alias/lifetime/per-hop obligation and branch decision remains at
its existing short-circuit/loop position. No leaf syntax, pointer identity,
scalar value, source summary or contract authority is assumed by retention.
An incomplete leaf still refuses the complete plan and restores the probe.
The descriptor and switch guards remain. Existing admissions keep their route.
The report-owned link map adds only `cio_file_is_up` as an explicit
selected-program home. Link resolution, type/calling-convention agreement and
non-interposition remain stated assumptions.

`QCore.CallFreeConditionReads` binds the 8 Boolean retention vectors and reuses
the established Boolean-tree desugaring theorem for checked-read events. For
an arbitrary read checker, the complete state and error trace agree; an
error emitted by an evaluated read is retained. The model proves no C
extraction or Rust correspondence theorem (boundary 28). Six focused controls
cover Boolean compositions, loops, leaks, double release, released bases and
refused volatile/atomic/write/uncontracted leaves. Every Safe fixture replays
its retained certificate. The literal 0 compound control stays conservative:
both scalar outcomes are represented, so it cannot erase a released read.

## Evidence and results

Validation and fresh paired corpus/system/slice runs are in progress under the
shared host serialization lock. Private root: `private/w48-bottom-up-batch4/`.
No gate or production outcome is claimed by this work-in-progress paragraph.

## Follow-ons

`W48-BOTTOM-UP-RESIDUAL-PLAN` remains open. Logger ownership/callback closure,
scheduler TLS/list/thread destruction, string-keyed lookup membership and
richer Removed return transport require their own checked boundaries. The
configured slice's unresolved `end` identities remain independently tracked.

## For the integrator

Integrate serially from this isolated branch; it never advances main. Preserve
prediction chronology, the exact build/analyzer/input/source identities and
all failed attempts. Keep conditional external homes separate from source
facts and caller-session replay occurrences separate from distinct C-body
conformance. Shared STATUS, part10 increment, acceptance generator/matrix and
report index need serial reconciliation with the premise cards. No provider
contract edits are made, so the conditional provider gate is inapplicable.
