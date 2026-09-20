# Cycle 18 priority-one integration

Base: `a54aa8ab`. Status: implementation in progress.

The user authorized the three priority-one follow-ups: timed-wait creation and
registration failures, all-path `cb_event` payload bounds/write authority, and
`cb_timeout` self-teardown with distinct event/map/descriptor custody. Three
GPT-5.6 Sol agents at medium reasoning implement isolated lanes; the root owns
integration, review, configured consumption, corpus pins, the combined gate,
reconciliation, ledger updates and publication.

Acceptance decisions belong in executable Quirin contracts over independently
derived facts. Generic extraction, arithmetic, hashing, parsing and operational
state transitions remain implementation machinery. Matching audited Lean models
are required for semantic changes; their correspondence to Rust remains reviewed,
not an extraction theorem. Ordinary `FreeEvent` refusal must not be waived to
admit running-callback teardown. Unproven paths remain Unknown; a native success
run does not establish universal safety and a provisional Unsafe is not an
upstream defect.

Each lane must identify the exact selected source paths, consumed artifacts,
context/bounds, negative controls and remaining host or provenance premises.
The integrator retires a tracked follow-up only against its actual discharge
condition and preserves all names previously tracked by it. Callback/task handoff
and expanded native platform acceptance are outside this priority-one scope.
Existing corpus/observed-wait outcomes are baseline evidence, not permission to
claim aggregate Cycle 18 completion.

## Integration review and native evidence

The integrator required a distinct current-callback deletion authorization and
reviewed every prior lifecycle operation against the new self-teardown state.
Ordinary delete, add, activation and free cannot resurrect or release that state
a second time. Callback frame lifetime remains separate from the released event
and map. The caller-owned peer descriptor is an explicit enclosing observation,
not a synthetic close appended to make the model pass.

The dedicated `QCORE_CB_EVENT_ONLY` harness mode executes the pinned Monkey
translation unit with real sanitized Libevent and the real sanitized Monkey
`mk_utils.c` support implementation. Four flag combinations exercise all four
`cb_event` paths, with actual typed snapshots, allocation call sizes, element
stride, queue size and backend observation. This producer performs callback
stdio and snapshots and changes timing; it is not the fixed-buffer timed-wait
observer. Runtime/linkage accuracy and exclusive storage ownership remain
explicit premises.

Native payload run 5 exited 0 under ASan/UBSan/LSan, with Linux epoll, capacity 8,
72-byte fired elements, 576-byte fired allocation and 32-byte context. Its files
and dependency inventory are under
`private/cycle18-priority1/native-payload/output-5/`; command output is
`private/cycle18-priority1/native-payload/run-5.log`. Earlier attempts are
retained: run 1 lacked the Monkey diagnostic support symbol; run 2 selected an
archive absent from this build; run 3's native binary succeeded but the runner
read the wrong event schema key and exited 1. Run 4 passed before the explicit
backend sidecar was added. These attempts are not final gate evidence.

## Delivered interfaces and bounded results

| Command | Executable policy | Integrated evidence |
|---|---|---|
| `quirin-audit wait-failures CONFIG.json` | 8 declarations / 66 clauses | Four creation and three wait source paths; six strict fault-harness observations plus two source-model limits; 3 scoped Unsafe and 5 Unknown cases |
| `quirin-audit event-payload CONFIG.json` | 5 declarations / 47 clauses | Four complete controlled `event_active` callback executions covering the four source paths; actual allocation/layout; explicit exclusive write authority |
| `quirin-audit timeout-teardown CONFIG.json` | 5 declarations / 38 clauses | Source projection of both teardown paths; one selected real failed-send execution; 15-operation event/map/descriptor model certificate |

All commands load the pinned production source rather than accepting a caller's
operation list. Configurations select observations and artifacts; manifests
identify those files without attesting execution. Typed role checks prevent a
file from being relabeled merely because it appears somewhere in an inventory.
Selected source and file snapshots are checked again before publication. Reports
cannot alias protected inputs and the new commands publish with atomic rename.

The payload checker returns a bounded property report, not an event-lifecycle
composition certificate. Every branch is exercised with controlled flags;
EV_TIMEOUT on the channel is a branch exercise, not natural timer/readiness
reachability. The abstract callback argument ID and native argument address
remain separate, with native registration identity checked without truncation.
The source has no general array-capacity guard; the report checks observed
indices against the actual allocation and a caller-declared write authority.

The teardown review added the map-write-endpoint equality after the lane's
handoff: a close of another live endpoint must refuse. Native pointer fields
must belong to the exact map object and have the declared C type. Ordinary
running-event free, generic nonblocking delete as self-free authorization,
post-free activation, resurrection, duplicate release and access after map
release remain refused. The caller's peer close is not credited to the callback.

The three named priority-one entries are discharged at these stated boundaries.
`MONKEY-CALLBACK-CYCLE18-COMPOSITION` retains registration/liveness, initialization
reachability, caller null/size handling, general allocation/authority provenance,
natural activation reachability and all-relevant-path composition. Task/deferral
handoff and host/platform expansion remain separately registered. Callback
accounting stays 3/3 accounted, 0/3 fully composed and 3 explicit Unknown; CT and
aggregate conformance are unchanged.

## Configured controls and retained evidence

Final payload run 6 adds typed artifact roles and labels controlled activation;
its native executable, source, support object, Libevent archive, build cache and
all dependency hashes are recorded in [the run manifest](payload-native/run-manifest.json).
Raw [observations](payload-native/observations.jsonl),
[bindings](payload-native/bindings.jsonl), [allocation](payload-native/allocation.json)
and [backend](payload-native/runtime.json) are archived beside it. Its command
log is `private/cycle18-priority1/native-payload/run-6.log`, exit 0. Run 5 remains
an earlier successful measurement, not the final manifest. The final configured
[payload report](payload-report.json) checks exactly four executions. All
[11 payload controls](payload-controls.json) refuse with exit 1, including
rehashed semantic mutants, artifact-role substitution, an extra otherwise valid
execution and report aliasing. The [shipped runner](check_payload_controls.py)
preserves every per-control log under `private/cycle18-priority1/payload-controls-3/`.

The selected [timeout report](timeout-report.json) is a fresh configured audit of
the lane's retained native bytes, with role metadata adapted explicitly; no
native rerun is implied. [Producer build metadata](timeout-native/metadata.json)
records the real command and linked archive hashes. Its native observations and
bindings are archived alongside it. The [seven configured controls](timeout-controls.json)
all exit 1; [their runner](check_timeout_controls.py) preserves logs under
`private/cycle18-priority1/timeout-controls/`. The first two exercise the new
map-descriptor and typed-object joins after recomputing producer hashes.

The [wait-failure report](wait-native/report.json) records the eight cases and
instrumented-source-model premises. The lane's nine configured negative controls
are retained in its card and worktree archive. Root configured wait and timeout
commands both exited 0 (`private/cycle18-priority1/{wait,timeout}-cli.log`).

[Legacy CLI parity](cli-parity.json) preserves one accepted timed-wait replay and
six refusing controls with identical lifecycle and identity mappings. This is
separate from the new bounded reports. Earlier focused failures are disclosed:
the first corpus-pin attempt had the correct entries in the wrong ordered-table
positions (exit 101); the corrected attempt passed all 13 tests. The first
hardened payload audit mistakenly interpreted retained Libevent source text as
a digest and refused; the corrected route snapshots the actual text bytes and
passes. None of these failed attempts is hidden in a successful count.

## Worktree preservation

[The sweep](worktree-sweep.json) removed the three completed clean delegated
worktrees only after final handoffs and a process/open-file check found no users.
234 private files (7,365,994 bytes) were copied and hash-verified under
`.claude/worktrees/.sweep-backups/cycle18-priority1-20260920/`. The timeout lane's
separate `/tmp/qcore-cycle18-timeout-*` source/build/native evidence is retained.
The external `next-wave-candidates-c-sources-18bd37` worktree remains untouched.

The release corpus run exited 0. [All five raw artifacts](corpus-bytes.json) are
byte-for-byte identical to the frozen W42 baseline; the [structured diff](corpus-diff.json)
also reports no change. [Run identity](corpus-run.json) records the current
binary, command, explicit environment and measured tip. This preserves 185
units (2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only), 4,834
projected functions and the 272-leak diagnostic baseline.

## Layering correction during the standing gate

The first complete workspace attempt reported a failure in
`workspace_dependencies_point_strictly_downward`: the initial payload audit
adapter had added a forbidden direct dependency from `qcore-audit` to
`qcore-core`. The dependency pin was kept intact. Callback-model construction
moved unchanged to `qcore-verification::event_payload::declared_event_payload_callback`,
whose name and documentation retain the caller-declared authority premise.
The corrected layering test passed all 11 checks. Two intermediate focused
compilation attempts while moving the helper failed (placement, then imports)
and are retained in `private/cycle18-priority1/layering-fix-{check,test}.log`;
`layering-fix-test-2.log` is the passing focused run. A separate complete gate
is required for the corrected tree; the original attempt is not counted as a
passing gate.
