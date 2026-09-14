# Verifier host closed-input deadline repair

2026-09-14; base `5ab4ba9bc85a5de0e955261b570cc503a24f726b`.

## Finding and repair

The standalone host regression reproduced in isolation on the Linux execution
host. Printing the actual failure and temporarily observing the transport loop
localized it to the **third**, 1,000 ms, large-response arm: `Err(Timeout)`.
The 100 ms live-provider arm already returned `Io(Broken pipe)`. This corrects
the wave-33 ledger's attribution to that shorter deadline without rewriting
its frozen report.

The response repeats the request's 8 MiB ID. The host reads at most 8 KiB per
pipe per iteration, then used to sleep 1 ms unconditionally. Reading the ID
alone needs 1,024 reads and therefore at least 1,024 ms of requested sleep,
before overhead. A direct child can exit after writing the last bytes into the
pipe while the host still has unread bytes. At deadline, that exited-child
state correctly selects Timeout rather than the live-child Io arm. This is a
transport throughput defect, not evidence that the Io assertion should change.
A traced run outside the restricted execution environment passed: the symptom
is timing-sensitive across environments, but the unconditional backoff is
present in both.

The loop now sleeps only when the request byte count and both output byte
counts are unchanged. Deadline checks, one bounded read per output pipe per
iteration, output caps, incomplete-request rejection, error precedence, and
process-group cleanup are unchanged. An idle provider still backs off; a busy
provider still reaches the deadline check on every iteration. No source
semantics, verdict admission rules, or Lean definitions change. These are
executed transport observations, not a proof of OS scheduling or wall time.

## Regression evidence

`crates/qcore-verifier-host/tests/standalone_host.rs` keeps every original
assertion and all three deadlines. Its Io assertions now print the actual
failure. A new admitting control consumes the entire request before returning
the same large response, and must return that exact response within the same
1,000 ms process budget. The original partial-request arm must still return
Io, and descendant and live-process marker checks still establish cleanup.

The standalone target passed all 3 tests; the closed-input test subsequently
passed 5/5 isolated repetitions. No macOS execution is claimed. Scheduling
under arbitrary host starvation remains outside this observation.

## Validation

Raw logs are worktree-private under `measure/closed-input/`.
- Standalone target: exit 0, 3 passed; repeated regression: 5/5, all exit 0.
- Host crate `cargo test -p qcore-verifier-host --no-fail-fast`: exit 101,
  97 passed / 15 failed / 0 ignored. The six failures are in
  `execution_quota`, all reporting denied `unshare(CLONE_NEWUSER | CLONE_NEWNET)`
  in this restricted environment (`host.log`); transport regression passes.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`:
  exit 0 (`fmt.log`, `clippy.log`).
- Both reevaluation examples: exit 0 (`probe.log`, `followup.log`).
- `python3 scripts/check_lean_audits.py`: exit 0, 128 audits (`lean.log`).
- Full `cargo test --workspace --no-fail-fast` was started, encountered native
  LeakSanitizer environment failures (including its ptrace incompatibility
  diagnostic), and was interrupted with exit 130 when the user confirmed the
  fix and requested committing it. `workspace.log` is partial, not an aggregate
  passing gate. No replacement workspace total is claimed.
- User independently confirmed that the reported failure is fixed before
  requesting the commit; their platform and command were not specified.

## Remaining boundary

`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`'s reproduced throughput defect is repaired.
A complete workspace gate in an environment permitting the sanitizer and
namespace checks remains to be measured. The earlier archived gate stays
frozen; this report supersedes only its closed-input failure diagnosis.
