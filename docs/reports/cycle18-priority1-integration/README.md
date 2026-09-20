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
