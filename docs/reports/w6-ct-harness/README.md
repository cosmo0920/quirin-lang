# W6 CT conformance harness stand-up (2026-09-06)

Executes `docs/GENERALIZATION_PLAN.md` §W6 task 1 and maps an initial batch of
three CT cases. The CT01–CT50 language corpus (spec §55) is tracked by
[`docs/reevaluation/ct_manifest.json`](../../reevaluation/ct_manifest.json);
this record documents the harness design, exactly which cases moved off
`unmapped`, and what the mapped fixtures do and do not establish.

## Harness design

- **Test:** `compiler/tests/ct_corpus.rs` (integration test of
  `qcore-compiler`, the crate that owns `compile_files` and whole-program
  checking). Two tests:
  - `ct_manifest_and_fixture_set_agree_and_mapped_cases_execute` — loads the
    manifest, checks the CT01–CT50 identity/order/stage schema, and for every
    case enforces manifest/fixture agreement in both directions: a case marked
    `mapped` must have an executable fixture (missing fixture, missing
    `expected.json`, wrong `source_fixture` path, empty `semantic_profile`, or
    stage statuses that disagree with what the fixture executes all fail the
    test); a case marked `unmapped` must have **no** fixture directory (a
    fixture without a deliberate manifest advance fails the test); any other
    `mapping_status` vocabulary fails closed.
  - `ct_mapping_progress_is_pinned_and_counted` — pins the moving metric:
    3 mapped / 47 unmapped / 50 total. Unmapped cases are counted, never
    silently skipped.
- **Fixtures:** `compiler/tests/fixtures/ct/<ID>/case.qcore` plus
  `expected.json` (verdict `accept`/`reject`, optional entry, expected
  diagnostics as code + message substrings, expected reference-execution
  outcome and acquisition/release trace, executed stages, scope notes).
- **Execution is the real source pipeline** (reevaluation spec §5.1: no
  hand-constructed Rust events): each mapped case runs
  `compile_files` (parse → elaborate → synthesize semantics →
  `check_whole_program`; for executable modules also CFG construction,
  translation validation and the CFG reference interpreter, whose
  `ExecutionObservation` the harness compares against the pinned outcome and
  trace).
- **Namespace discipline:** CT remains the language corpus; no WM/VP/LC probe
  cases or per-library cases are admitted here.

## Cases mapped (3)

All three fixtures use the Cycle 12 executable source subset; each manifest
entry's `semantic_profile` records exactly what is and is not exercised.

| Case | Spec expectation | Fixture behavior | Result |
| --- | --- | --- | --- |
| CT02 | Parent uses buffer after successful move capture → Reject | `move buffer;` then `drop buffer;` — move capture modeled as ownership transfer out of the parent frame | Rejected: `Q202` `ResourceNotLive(1)` |
| CT12 | Create fails before any child starts → capture state restored; no task obligation | Failed owned acquisition raising declared `SpawnError` | Accepted; reference execution: outcome `SpawnError`, empty acquire/release trace (no obligation created) |
| CT16 | Worker typed error after partial local acquisition → clean locals, return loans, carry declared payload | One successful local acquisition, then a failing checked operation raising declared `WorkerError` | Accepted; reference execution: outcome `WorkerError`, trace `acquire:3, release:3` (local cleaned on the error path; the interpreter rejects any error exit with live resources) |

Per-stage manifest statuses: CT02 marks `parser`/`type_checker` executed;
CT12/CT16 additionally mark `reference_execution`/`cfg` executed.
`native_backend` and `proof` stay `unassessed` for all three.

## What this does not establish

- The three fixtures exercise the **sequential** ownership/typed-error/cleanup
  core only. Spawn/capture syntax, task-handle linearity, shares, mutexes and
  the concurrency backend are not exercised; those aspects of CT02/CT12/CT16
  are scoped out explicitly in each `semantic_profile` and stage table.
- Candidate CT50 (address/descriptor reuse) was **not** mapped: the executable
  subset rejects numeric resource-identity reuse outright
  (`Q202 DuplicateResource`), which is fail-closed but does not decide the
  case's "old capability cannot authorize new identity" property. CT13 was not
  mapped because the spec makes task handles linear obligations *without* an
  automatically runnable destructor (§43), so witnessing it via implicit
  resource cleanup would test the wrong mechanism.
- 47 cases remain `unmapped`. Per plan §W6 task 2, the resource/custody rows
  await W4, verdict/composition rows await W3, and the concurrency rows await
  source-level elaboration into the existing checkers.

## Standing gate (all run at this record's commit)

| Command | Exit |
| --- | --- |
| `cargo test --workspace` | 0 (410 tests pass, 0 fail, including the two new harness tests) |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 |
| `cd proof && lake build` | 0 |
