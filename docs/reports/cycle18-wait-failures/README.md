# Cycle 18 timed-wait failure paths

Base: `a54aa8ab10d10ae26232fc9662eb1f4d6b61ddb6`.

This card discharges `CYCLE18-TIMED-WAIT-FAILURE-PATHS` at a bounded
source-and-fault-model boundary. It does not promote the aggregate Cycle 18
composition, callback coverage, or full conformance.

## Executable boundary

The sealed Monkey projection now carries complete path counts in addition to
the previously selected success paths: four `_mk_event_loop_create` paths and
three `_mk_event_wait_2` paths. Those counts come from constructor-private
Clang-imported `SourceSummary` values whose complete canonical vectors are
accepted by `monkey/wait_source_policy.qcore`; a caller cannot supply an
operation list or a path count.

`monkey/wait_failure_policy.qcore` is the runtime authority for eight cases,
their outcomes, named properties and reasons. It has 8 declarations and 66
clauses. The Rust checker reads the outcome/property/reason literals from that
module and evaluates each complete actual fact vector against its declaration.
Native JSON contains no verdict. Every observation field is mandatory and
unknown fields refuse; missing false/zero facts cannot pass through defaults.

Every emitted row carries the verdict tuple: artifact, property, semantic
profile, scope, bounds, assumptions and evidence version. The success row is
`Unknown`, not an all-path `Safe` result. Timer registration failure is
`Unknown` because ignoring `event_add == -1` does not prove backend liveness or
timeout behavior. Context-allocation failure is `Unknown` because caller null
handling is outside this slice. The opaque initialization branch and the
caller-size/multiplication bound are separate source-model `Unknown` rows.

Three narrowly named properties are `Unsafe` under the explicit source
fixture/stub fault model: a nonnull context can contain a null event base; a
created event base is retained when fired-array allocation fails; and a null
timer creation result is forwarded to `event_add` and `event_free`. These are
validated reachable source behaviors for the stated fault outcomes. They are
not claims about how the real deployed Libevent build behaves, and are not
presented as upstream defects.

## Instrumented evidence and negative controls

The dedicated C harness includes the approved wrapper fixture verbatim and
stubs dependency results to drive context allocation, base creation, fired
allocation, timer creation and timer registration failures plus a success
control. The runner binds both the harness and included fixture digests,
compiles with Clang ASan/UBSan, enables LSan, requires the exact six-scenario
set, and records that dependency behavior is stubbed. The final run is
[recorded here](native-run.json); its semantically identical committed payload
is [failure-observations.json](failure-observations.json).

The focused Rust suite mutates missing scenarios, retained-base cleanup, null
timer arguments, failed-registration activation, a required field, source
revision, trace order and an injected caller verdict. Every mutant refuses. Existing source-projection
tests retain fixture acceptance and the production-input ignore boundary.

`QCore.WaitFailurePaths` mirrors the exact four/three source inventory, all
eight classified cases and the source-harness/model evidence split. Audited
theorems reject a missing creation path, refuse promotion of registration
failure, refuse treating one success as inventory discharge, and establish
that every `Unsafe` model row has instrumented harness evidence. The module has
no project axioms or placeholders.

## Focused validation

The final [focused gate](gate.json) records:

- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy -p qcore-verification --all-targets -- -D warnings`: exit 0.
- `cargo test -p qcore-verification --test wait_failure_paths --test monkey_wait_projection -- --nocapture`: exit 0; 7 passed, 0 failed, 1 ignored.
- `cd proof && lake build QCore.WaitFailurePaths && lake env lean AuditWaitFailurePaths.lean`: exit 0; seven audited theorems.
- final sanitizer harness outside ptrace: exit 0; six complete records.

Private final logs are under
`measure/cycle18-wait-failures-focused-final-3/` and
`measure/cycle18-wait-failures-native-5/`. Earlier unique attempts are retained:
native attempts 1 and 2 exited 1 on fixture warning diagnostics; attempt 3
exited 1 because LSan cannot run under ptrace; attempt 4 exited 0 but used the
superseded caller-supplied outcome schema. Focused attempt 1 recorded fmt 1,
Rust 101 and Lean 1 during initial integration; focused attempt 2 recorded
fmt/Rust 0 and Lean 1 before the module build step; focused-final recorded only
the Clippy type-complexity failure (101), with fmt/Rust/Lean 0. No failed run is
counted as passing evidence.

## Integrator reconciliation list

- Register `monkey/wait_failure_policy.qcore` in the global corpus pins. The
  new module contributes 8 declarations and 66 clauses; this card leaves the
  root-owned table unchanged.
- Import `QCore.WaitFailurePaths` from root-owned `proof/QCore.lean` and retain
  `AuditWaitFailurePaths.lean` in the aggregate audit.
- Wire configured-audit artifact selection to pass independently read failure
  observation bytes and the sealed `MonkeyWaitSourceProjection` to
  `check_wait_failure_paths`; do not replace configured hashing/snapshots with
  the public check.
- Retire `CYCLE18-TIMED-WAIT-FAILURE-PATHS` only after that configured join is
  integrated. Preserve registration, initialization reachability, size bounds,
  downstream null handling and successful-run all-path claims as `Unknown`.
- Update the root-owned acceptance matrix, status/increment record, corpus
  documentation and final reconciliation record. No existing frozen artifact
  was edited by this card.
