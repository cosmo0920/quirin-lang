# Claude Code project notes

@AGENTS.md

Current normative specification: `docs/QCORE_SPEC_r0_3.md` and its contract
companion. Follow `docs/reviews/r0_3/acceptance.json` for current repair status;
the supplied archive review is older than merged W2–W5 implementations.
Persistent audit setup: `docs/CYCLE19_BUILD_INPUTS.md`.

Claude-specific additions:

- The current Cycle 19 nested-teardown boundary is recorded in
  `docs/reports/cycle19-file-descriptor-failures/README.md` and the AGENTS.md status
  ledger. Its native memory and controlled failure observations bind concrete allocation/
  ring provenance and rollback. General caller/failure, filesystem and event-
  composition gates remain open. Unsupported backend close now has a symbolic
  partial-release classification with unresolved backend custody and `Unknown`;
  exact opaque identities survive conditional stream/context release. Four
  observed unsupported-type inputs retain empty streams and return null chunks
  with complete rollback; this is not a universal caller/reachability proof.
  Twelve real Linux close/unmap/delete calls additionally bind OS endpoints in
  exclusive temporary fixtures. Six native one-file constructor/teardown runs
  bind 66 allocation generations and preserve mapping/descriptor residuals after
  parent free under EIO injection. General filesystem callers, arbitrary OS
  failure behavior, full sync guarantees and durability remain open. Twelve
  filesystem allocation-failure/control processes account for 81 acquired
  generations and 73 recorded releases. Two UBSan null-argument terminations
  retain eight outstanding prefix generations; only the ten returning cases
  complete teardown and leak checking. Failed root/chunk-name copies and a
  stream returned without a directory remain distinct observed outcomes. Eight
  further constructor OS-failure/control processes release all 88 generations;
  a suppressed rollback close retains the original descriptor/inode after null
  chunk return and parent release. POSIX resize return values and errno remain
  separate. Seven existing-file profiles additionally release all 70 generations,
  preserving original inode/content, distinct corrupted/read-only errors and
  descriptor custody after suppressed chmod/rollback-close failures. These
  observations do not establish arbitrary kernel failure rules. Five further
  residency/transaction/lock sequences release 50 generations and seven OS
  epochs. Logical rollback leaves the mapped header length unchanged and down/up
  reloads appended bytes. Locked begin retries while direct write succeeds;
  general transaction, lock and production-caller guarantees remain open.
  Three sync-failure sequences release 30 generations and four OS epochs.
  Failed commit retains the transaction and retry clears it. Down continues
  cleanup after failed sync; unmapped commit succeeds without syncing. These
  injected observations do not establish general OS failure or durability rules.
  Four failed-down sequences release 40 more generations and seven OS epochs.
  Failed unmap preserves mapping/descriptor identity, up ring and counter; prior
  successful sync survives failure. Combined sync/unmap failure stays unsynced,
  with recovery through retry or parent teardown releasing the original epoch.
  Four descriptor-failure sequences release 40 further generations and acquire
  five OS epochs. Failed close moves the chunk down but retains its descriptor;
  down/up retries reject. A second suppressed close retains that descriptor after
  parent free, with harness reclamation recorded separately. Failed reopen creates
  no epoch; successful retry creates a fresh one. General reopen failure paths,
  caller provenance, aggregate custody and full event composition remain open.

- Before starting checker/architecture work, read `docs/GENERALIZATION_PLAN.md`
  and land code according to its target crate layering (§3) and invariants
  (§2); do not add new code to the monolithic grab-bag once a target crate
  exists.
- W4 is executed for parameter/return custody with validation
  (`crates/qcore-c-gateway/src/legacy_summary.rs`,
  `docs/reports/w4-custody-inference/README.md`). A call into an internal C
  function is modeled only through a declared `.qcore` contract or a summary
  that `observations_match_contract` validated against a fresh analyzer run;
  everything else stays `Unknown`. When touching that pass, do not let a
  summary be published without validation, do not seed a formal parameter with
  entry custody the body did not demand (that asserts non-nullness and deletes
  failure paths), and do not accept an assume-guarantee fixed point for a
  recursive component. The 2026-09-07 extension validates direct `Out` slots,
  `Stored` formals, `Reallocated` results and possibly-null owned returns
  (`docs/reports/w4-custody-extensions/README.md`). Reassigned out pointers,
  `Removed` inference and recursive components remain unsupported; the
  `qcore-c-gateway` relocation is executed with compiler compatibility reexports
  (`docs/reports/w4-gateway-crate/README.md`).
- Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass. If a pinned table (contract corpus counts,
  submodule revision, archived report) conflicts with your change, the change
  must update the pin deliberately and say so.
- Status language matters in this project: keep the Safe/Unsafe/Unknown
  verdict vocabulary and the "unpromoted" phrasing exactly as defined in
  AGENTS.md when editing docs, reports, or commit messages.
- Run the full standing per-change gate from AGENTS.md before reporting a
  task complete; report probe/example exit codes honestly (a nonzero probe
  exit is a finding, not a build failure).

## Reopen failure evidence (2026-09-07)

Five native reopen size-lookup/mapping failure sequences release all 50
allocation generations and six mapping epochs. Ten descriptor epochs yield eight
library closes and two residuals after parent release, reclaimed separately by
the harness. Failed size lookup retains an unmapped descriptor; failed mapping
stores MAP_FAILED, reports up-ring membership without incrementing the counter,
and retains the new descriptor. Fixed retries and already-synced behavior remain
observations, not general OS failure or durability guarantees. Evidence:
`docs/reports/cycle19-reopen-failures/README.md`. Raw units remain unpromoted.

- W2's bounded profile migration and full 185-TU parity are recorded in
  `docs/reports/w2-profile-manifests/README.md`; remaining H4/H5/H6 and H7
  orchestration work is explicit there. Profile data remains declared trust.

Frontend gates repaired (2026-09-07,
`docs/reports/w6-frontend-gates/README.md`): nonempty source function contracts
reject centrally with Q021 until enforced. Guard/thread-scope header names bind
only inside their bodies; nested concurrency lowering still rejects with Q500.
This closes the two W6 batch-2 frontend defects without claiming contract or
concurrency semantics are implemented. CT mapping remains 5/50.

- W3 bounded evidence-v2 replay/dispatch is recorded in
  `docs/reports/w3-evidence-v2/README.md`. Positive observation certificates
  describe recorded runs under `observations_trusted`; lifecycle source
  derivation stays incomplete until real effect/ABI import exists. The generic
  host is extracted in `crates/qcore-verifier-host` (record:
  `docs/reports/w3-verifier-host/README.md`); trusted derivation/replay remain
  in the compiler adapter pending kernel consolidation. V1 Safe refusal is unchanged.

- W2 H6 provider metadata now loads from profile packages; see
  `docs/reports/w2-provider-discovery/README.md`. This does not extract providers
  into processes or change the v1 Safe refusal.

Lean CI hygiene is implemented (2026-09-07): the pinned proof build, all theorem
audits and the complete QCore axiom closure are checked, with both reevaluation
examples in Linux/Windows CI. Local gates pass; hosted CI awaits a push.
See `docs/reports/w7-lean-ci/README.md`.

W7 generic Lean naming is complete for the bounded targets: DescriptorCustody,
EventComposition and busy_effect_preserves_lock, with historical aliases and
unchanged models (`docs/reports/w7-generic-lean-names/README.md`).

Boolean field profile routing landed (2026-09-07,
`docs/reports/w2-boolean-field-profiles/README.md`): typed C record/field and
Boolean operation routes come from loaded profile data, with exact declared
assumptions on summaries. An unrelated record fixture exercises both supported
models. Full 185-TU resource diagnostics and source projections are unchanged;
chunk reports add only three assumption maps. Frame-helper ABI generalization
and source-to-model identity premises remain open.

- The generic `qcore-audit` driver and retained Cycle19 compatibility adapter
  are documented in `docs/reports/w5-configured-audit/README.md`. No provider
  module is linked by the new driver; production evidence routes remain.

Monkey typed import profiles landed (2026-09-07,
`docs/reports/w2-monkey-typed-profile/README.md`): function roles, records,
sizeof bounds, exact callback signatures, callee bounds and callback effects
load from profile data. Malformed or unavailable declarations reject; wrappers
retain their existing opaque-call grammar. Full 185-TU diagnostics are unchanged;
18 syntax summaries now retain explicit declared profile assumptions. These are
syntax projections and trusted declarations, not proofs of source custody or
whole-program safety. General source/model binding and provider consolidation
remain open; configured auditing is recorded separately above.

Revision 0.3 implementation review (2026-09-07): resource/task lifetime checks,
unconditional axis validation, signed literal extrema and exact/maximum loop
custody are repaired at their stated model/source boundaries. Ordinary i32/unit
result compatibility is checked with an explicit separate custody payload;
full owned-result typing remains open. Concurrency certificates retain declared
mutex/synchronization premises; checked invariant/source authority remains open.
Observation replay rejects scope/module/bounds substitution. Current requirements
and bounded acceptance are in `docs/QCORE_SPEC_r0_3.md` and
`docs/reviews/r0_3/acceptance.json`; superseded specs live in `docs/obsolete/`.
