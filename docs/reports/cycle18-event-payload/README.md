# Cycle 18 `cb_event` payload boundary

`CYCLE18-CB-EVENT-PAYLOAD-BOUNDARY` gains a source-only projection and a
separate contextual replay checker. The source projection consumes
constructor-private typed-C `SourceSummary` values for `cb_event` and
`_mk_event_loop_create`; serialized or caller-authored operations cannot create
source authority. The contextual checker returns `Checked` for this bounded
property or `Unknown`. It does not publish `Safe`, compose the whole event
program, or change the default audit and observed-wait outcomes.

## Implemented boundary

`crates/qcore-verification/src/event_payload.rs` derives, hashes and checks:

- all four imported `cb_event` read/write guard alternatives;
- the exact event → map → context → fired-base/fired-index identity chain;
- exactly four writes on every path: `fired[i].fd`, `.mask`, `.data`, and
  `ctx->fired_count`, including the source values for `fd`, `data`, and the
  single increment;
- the successful `_mk_event_loop_create` allocation expression
  `sizeof(struct mk_event) * size`, the allocation result stored in
  `ctx->fired`, and the same `size` stored in `ctx->queue_size`;
- exact source and allocation summary hashes, contract hash, callback-model
  hash, write-authority hash, and dedicated allocation-evidence hash in the
  independently validated event-composition context;
- registration/callback argument identity, exact callback enter/exit run
  identity, unique source-path selection by flags, and total accounting of
  nonselected binding and observation records;
- the native allocation record's capacity, element size, allocation byte
  count, context size, and identities against complete before/after snapshots;
- a nonnegative signed `int` index below both capacity and `INT_MAX`, with
  checked offset, extent, and address arithmetic;
- exact reactor `CallbackSummary` authority: borrowed argument, read/write
  triggers, no deferral/tasks, and `PayloadAccessKind::Write` for exactly the
  context and fired payloads.

The executable policy is
`contract/qcore/monkey/event_payload.qcore`. It receives primitive indices,
capacities, sizes, offsets, extents, identities, field sets, model modes,
trigger sets, payload counts and concrete `PayloadAccessKind` values; it does not
accept caller-asserted “bounds valid” or “authority valid” Boolean facts.
Unsupported shapes, omissions, substitutions, arithmetic overflow, surplus
callback intervals and incomplete snapshots retain `Unknown`.

## Per-path account

| Read flag | Write flag | Derived writes | Contextual requirement |
| --- | --- | ---: | --- |
| taken | taken | 4 | exact run/snapshot/allocation/model join |
| taken | not taken | 4 | exact run/snapshot/allocation/model join |
| not taken | taken | 4 | exact run/snapshot/allocation/model join |
| not taken | not taken | 4 | exact run/snapshot/allocation/model join |

The return-only `cb_wait_2_timeout` is an explicit negative control and cannot
construct an event-payload projection. Source mutations to the `data` value or
allocation formula also refuse. Context regressions cover path omission,
negative and `INT_MAX` indices, storage/identity changes, write-authority
changes, queue-size changes, and allocation-evidence substitution.

## Evidence classes and limits

The typed source paths and allocator expression are **source-derived model
evidence**. The Rust regression imports the checked-in copies of the pinned
production callback/wrapper bodies through Clang, rather than constructing a
`SourceSummary`. Its contextual fixture exercises the same schema over the
previous all-path native trace; the test-added `queue_size` and allocation row
are fixture data and are **not new production native evidence**.

The dedicated native producer completed under ASan, UBSan and LSan and records
the real `mk_mem_alloc_z` allocation, `ctx->queue_size`,
`sizeof(struct mk_event)`, `sizeof(struct mk_event_ctx)`, and four dispatcher
executions (read, write, both and timeout/no-read-write). The focused Rust test
consumed the exact files from
`private/cycle18-priority1/native-payload/output-5/` and checked 4/4 source
paths, capacity 8 and element size 72. Their SHA-256 identities are:

| Native artifact | SHA-256 |
| --- | --- |
| `observations.jsonl` | `6595203bde2352e79e287e6e8dfe3697f6b14a0a9c51a7fa660ca195cb9715b2` |
| `bindings.jsonl` | `cb73e4d81db41eddd35ef6f3e161774b0af0858ffd177fb23606c6007edfb481` |
| `allocation.json` | `62b2a354847d6afacd6abbe4debf57a5f54d952837a83942a2f20122b198d549` |

These native facts remain producer-supplied observations. Hash equality
identifies inputs; it does not attest the observer, compiler, linkage or host.
C layout/Clang faithfulness and Lean-to-Rust correspondence remain explicit
premises.

`proof/QCore/EventPayloadBoundary.lean` proves the abstract four-path target
set, allocation extent relation, index/extent arithmetic, exact two-payload
write authority, and context-substitution refusals. The audited theorem family
models the rule; it is not an extraction or refinement proof for Rust or C.

## Focused validation

Private logs are under `measure/event-payload/attempt-{001,002,003}/` in the
delegated worktree. Final exits and counts are recorded after every process has
exited. The first clippy pass found three lint defects and exited 101; the
listed final clippy rerun is green after those defects were fixed.

| Command | Exit | Result | Log |
| --- | ---: | --- | --- |
| `cargo test -p qcore-verification --test event_payload -- --nocapture` | 0 | 5 passed | `attempt-002/event-payload-tests-final.log` |
| `QCORE_EVENT_PAYLOAD_NATIVE_DIR=.../output-5 cargo test -p qcore-verification --test event_payload configured_native_payload_artifacts_check_when_present -- --nocapture` | 0 | exact native artifacts: 1 passed | `attempt-002/native-payload-output5-test.log` |
| `cargo test -p qcore-verification --no-fail-fast` | 0 | 307 passed, 1 ignored | `attempt-001/qcore-verification-tests.log` |
| `cargo clippy -p qcore-verification --all-targets -- -D warnings` | 0 | clean | `attempt-003/clippy.log` |
| `cargo fmt --all -- --check` | 0 | clean | `attempt-003/fmt-check.log` |
| `lake build QCore.EventPayloadBoundary` | 0 | module built | `attempt-003/lean-build.log` |
| `lake env lean AuditEventPayloadBoundary.lean` | 0 | allowed axioms only | `attempt-003/lean-audit.log` |

The root integrator owns the combined standing gate, corpus pin, status,
acceptance matrix, increment record and configured-audit route.

## Integrator reconciliation list

1. Export `event_payload` from the merged `qcore-verification` module tree if
   the shared `lib.rs` cannot be taken directly.
2. Update the contract-corpus module/clause pins for
   `contract/qcore/monkey/event_payload.qcore` without weakening the pin.
3. Add `QCore.EventPayloadBoundary` to `proof/QCore.lean` if the aggregate build
   requires the explicit import; keep `AuditEventPayloadBoundary.lean` in the
   audit census.
4. Wire the configured audit's sealed production `cb_event` and
   `_mk_event_loop_create` summaries, validated context, exact dedicated native
   payloads, allocation record and reactor callback summary through the public
   two-stage API.
5. Archive the dedicated native command, exits, hashes and per-path output in
   this report directory. Only then decide whether the acceptance-matrix
   follow-up's full discharge condition is met; this delegated report alone
   does not retire it.
6. Preserve default-corpus and observed-wait outcomes unless the separately
   measured payload property justifies a distinct result.

## Integration evidence

The root's [combined integration report](../cycle18-priority1-integration/README.md)
owns the final dedicated native run, typed artifact-role binding, exact
four-execution limit, eleven configured refusal controls and frozen corpus/CLI
parity. Its `payload-native/` directory archives the command manifest, actual
allocation/backend records and complete observations/bindings. Controlled
`event_active` flags exercise source branches; this does not establish natural
readiness/timer reachability or full event-lifecycle composition.
