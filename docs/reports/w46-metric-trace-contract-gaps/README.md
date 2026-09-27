# W46-METRIC-TRACE-CONTRACT-GAPS

## Prediction, recorded before implementation

Base `9bce3986dd5703faa6d1fc2f485dc88a20902fdd`. The pinned Cycle 19
source is Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; the
system slice is `046accf6aded7906f66a1baf0202da349e521736`.

Adding custody-neutral scalar metric helpers and trace ID queries / scalar
span setters should reduce `uncontracted-callee` leaves in the relevant library
units. I predict no new validated Unsafe result and no increase in Safe due to
the many other blockers in encoder and decoder paths. Exact function/unit
movement is deliberately left open for the pinned corpus arbiter; this is a
prediction, not an after-the-fact claim. These contracts do not discharge a
caller-owned metric, ID, span, or nested resource. A returned trace ID buffer
is a member alias, and mutation that replaces reachable ownership is outside
the predicted neutral set.

The two CMetrics source/header files in scope have identical SHA-256 bytes at
both revisions. CTraces files differ by revision, so the selected functions
were checked in both before declarations were added. The reviewed boundary,
controls and integration notes follow.

## Reviewed source and declaration boundary

The two read-only source roots are
`build/cycle19/fluent-bit-clang-source` at `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`
and `private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source`
at `046accf6aded7906f66a1baf0202da349e521736`, both under the main checkout.
Each new non-libc declaration was pinned with `quirin-audit pin --source-root`
once per revision, with no handwritten lock identity. The selected CTraces
function bodies were compared textually at the two pins and match, though their
files as a whole changed. CMetrics `cmt_metric.c`, `cmt_metric.h` and
`cmt_math.h` are byte-identical across the pins (SHA-256
`43f5cffac5256bc2d9dfd83eec1dd301cc3f90e35337a34345e7cf78cbab1cb8`,
`01436d880675c755ebba53d361273446b9745077b2cc2f9aa20ca8089e7e86a4`,
`cb0e00fc3e2b3f675b66e87c4bf280f36007df6a7c65a18e0dbc6cf0ca99819c`).

`cmt_metric_get_value`, `get_value_type`, `get_int64_value`,
`get_uint64_value`, `has_start_timestamp`, and `get_start_timestamp` read
scalar fields, sometimes through other scalar helpers, and return scalars.
`set_int64`, `set_uint64`, `set_timestamp`, `set_start_timestamp`, and
`unset_start_timestamp` store scalar fields only. The two `cmt_math_*`
conversions copy bit patterns through a local union. This is thirteen new
CMetrics declarations in the existing `metric_values.qcore`. The prototypes
in `cmt_metric.h` and static-inline definitions in `cmt_math.h` agree in
parameter and result types: object pointers project to `ptr<mut, u8>`,
`uint64_t` to `u64`, `int64_t` to `i64`, `int` to `i32`, and `double` to `f64`.
The conversions have one shared inline definition, unlike the unselected
multi-backend `cmt_atomic_*` family. The declarations assert no atomicity,
ordering or floating-point value relation.

The metric accessors delegate to `cmt_atomic_load` and scalar setters to
`cmt_atomic_store`. All backend definitions in the pinned tree were inspected:
Clang and GCC use builtins on `uint64_t *`; generic uses a mutex and writes or
reads `*storage`; MSVC's two branches use a critical section or `_Interlocked*`.
The generic and MSVC initialization paths may change backend global state but
none adopts, frees, publishes, or returns the metric pointer or a reachable
member identity. This supports only the claimed custody vector. The atomic
functions themselves remain undeclared because their backend copies and
linkage are not a single safe name-keyed declaration.

`ctr_id_cmp` (`ctr_id.c:95-116`) only reads lengths and compares bytes;
`ctr_id_get_len` (`:118-121`) reads the ID buffer length and returns a scalar.
The selected `ctr_span_set_flags` and
`ctr_span_set_dropped_{events,links,attributes}_count` functions in
`ctr_span.c:315-348` store only scalar fields. Their prototypes in `ctr_id.h`
and `ctr_span.h` agree. `size_t` projects to `usize` under the already assumed
LP64 target; `uint32_t` to `u32`. The singular
`ctr_span_set_dropped_link_count` exists as a body but has no matching public
prototype; the declared plural function is the one selected.

Every new formal is `resource_borrowed` and every scalar or void result is
`resource_return_none()`. Borrowed means caller custody remains; a store
through a borrowed pointer is allowed only because the selected stores move no
reachable owned identity. All selected callees are defined in an analysed
corpus unit or a common inline header, so the declared ABI and duty withdrawal
gate remains in force. A declaration is a trusted source premise, never a
proof of the C body.

## Refusal controls and formal parity

`metric_trace_contract_gaps.rs` loads the actual owner modules. Its accepting
fixture calls one metric scalar setter, one value getter, a trace ID length
query and a trace span scalar setter, then releases its own allocation. It
rechecks the resulting Safe certificate. Removing each representative
declaration restores its named uncontracted-call refusal. A separate control
keeps a leak when a borrowed setter is called on a caller-owned allocation,
and refuses a getter on a definitely released actual. Corpus tests pin all
nineteen declaration names and exact clause counts; the load-path selection
remains 50 modules, and the resource-v1 declaration count moves 183 → 202.

`QCore.MetricTraceContractGaps` models the selected arity vector and proves
its 19 entries retain caller custody and produce no resource. It also proves
the borrowed/owned duty relation, a releasing body contradicts a neutral
declaration, and absent body evidence keeps the declaration a premise. Its
five theorems are registered in `AuditMetricTraceContractGaps.lean` and in the
whole-project `QCore` audit import. Source-to-Lean correspondence remains a
reviewed premise.

## Residual boundary and integration

`ctr_id_get_buf` returns `cid->buf`, a member alias whose lifetime is tied to
the ID; declaring it fresh or aliasing it to `cid` would be false. `ctr_id_set`
replaces and frees that member and cannot use the neutral mode. Status, schema,
ID and attribute setters that allocate, retain or replace reachable ownership
stay uncontracted. `cmt_log_print` and the `ctr_mpack_consume_*` family can
reach callbacks and stay out. Encoder and decoder cleanup, callback
retention, and reachable child custody are not discharged by these scalar
declarations. `W36-CTRACES-FAMILY-FIRST` remains an integrator convention
decision: this card adds declarations to the existing sole CTraces module,
not a second module that settles its naming convention.

The integrator should regenerate current corpus and system slice measurements,
attribute every changed leaf and verdict, update `docs/STATUS.md`, the
acceptance matrix and one wave-46 increment paragraph from that primary
evidence, and decide the CTraces family convention. The card's own standing
gate results and any local arbiter probe are recorded below; main is advanced
only by the integrator.

## Card gate and retained attempts

The card used `CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/w46-contract-gaps/targets/metric-trace`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and
`CARGO_INCREMENTAL=0` to keep each worktree's build independent and off the
27-GB `/tmp` filesystem. The private per-step logs and exact commands/exits
are in `private/w46-metric-trace-gate/summary.json`; no output is piped.

The fresh worktree's required `git submodule update --init --recursive`
first failed (exit 1) because the sandbox could not create its `.git/worktrees`
module directory. The integrator then initialized it with a scoped local-cache
command (exit 0), yielding `third_party/libevent` at pinned `d82464a2`. A
repeat gate invocation exited 128 solely because the sandbox could not lock
that module's `core.worktree` config; `git submodule status --recursive`
still reported the correct pin. Both failed attempts are retained.

The initial sandboxed workspace runner completed all 323 units with 335
summaries: 5,916 passed, 107 failed, 11 ignored, exit 101. Of 46 failed
binaries, 42 logs contain a LeakSanitizer fatal error under this sandbox, two
contain `unshare(CLONE_NEWUSER | CLONE_NEWNET)` `EPERM`, one CT child returned
1 where 0 was expected without a captured cause, and one found a real stale
CMetrics source-policy inventory digest. The latter was corrected in
`contract/profiles/cmetrics-counter/source-contracts.json`; its focused Rust
test then passed (1/1). The runner logs and machine summary remain under
`private/test-binaries/20260927T011433Z-37/`. These 107 are not counted as a
green suite or all labelled environmental; the CT child awaits a rerun.

`cargo fmt --all -- --check` initially exited 1 after a late assertion edit;
`cargo fmt --all` applied the requested layout and the check reran exit 0.
`cargo clippy --all-targets -- -D warnings` exited 0; both reevaluation
examples exited 0; `scripts/check_lean_audits.py` exited 0 with 211/211 audits;
the script unit suite exited 0 with 364 tests. `providers/contracts.py check`,
`scripts/build_docs_index.py --check` and `quirin-audit pin --check` at both
reviewed revisions all exited 0. `providers/package.py build` exited 0 and
`verify` exited 0 for each of its five built packages. Provider Python tests
exited 0 with 15/15 tests. The sandboxed provider Cargo suite exited 101
after 36 pass / 1 fail in 18 summaries: the failing package-conformance row
requires a Linux confined profile unavailable in this sandbox. Provider fmt
and Clippy checks each exited 0. The nonzero suites are retained as attempts;
their unsandboxed reruns and final counts follow after scoped execution.
