# Review audit-runner cancellation and failure observation

## Scope and result

This card addresses review priorities R1 and R2 in
`scripts/check_lean_audits.py`. The external review was an inspection lead, not
an authority; the implementation and tests were derived against the repository
runner itself.

The parallel audit run now owns a per-run `ChildSupervisor`. Its lock covers the
stopped-state check, `Popen`, and registration as one operation. Cancellation
therefore either prevents a child from being created or waits for an in-progress
spawn to register and includes that child in termination. Pending futures are
cancelled, running child trees are stopped, and `shutdown(wait=True,
cancel_futures=True)` collects every worker before the runner returns. The same
cleanup covers exceptions during future submission.

Audit failures are observed with `as_completed`, so a later failure does not
wait behind an earlier slow submission. Successful child output remains
buffered and is replayed in sorted audit order. A new supervisor is created for
each invocation, so stopped state cannot poison a later run.

## Deterministic regression coverage

`scripts/tests/test_check_lean_audits.py` adds these boundaries:

- a supervisor already stopped refuses before `Popen`, and the mocked `Popen`
  is asserted untouched;
- a controlled `Popen` pause drives the in-progress spawn/register race while a
  sibling fails, then waits beyond the escaped child's marker delay and proves
  that no marker appears;
- the invocation following that cancellation runs six real harmless Python
  children and observes all six markers;
- a later immediate failure interrupts an earlier five-second child within a
  1.5-second bound;
- failure on the third `pool.submit` collects the two already submitted
  workers within the same bound;
- the existing transcript test continues to assert audit-order output when
  completion order differs.

The existing timeout descendant-cleanup test and top-level interrupt-to-130
test remain green. The focused module ran 26 tests successfully.

## Semantic and Lean boundary

This is Python process orchestration around unchanged Lean commands. It changes
when failure is noticed and how children/workers are cancelled and collected;
it does not change a QCore verdict, proof statement, verification rule, audit
command, warning policy, allowed axiom set, or audited Lean module. No Lean
definition or theorem is therefore required by the semantic-parity rule. The
standing Lean audit still ran all 150 `Audit*.lean` files successfully.

## Standing gate

The final gate measured commit `c9216d0342637b6f113335368d15c6fd5a556108`
with `CARGO_BUILD_JOBS=2` and
`python3 scripts/check_lean_audits.py --jobs 2`. Private logs are under
`measure/review-audit-runner-c9216d03-escalated/`.

| Command | Exit | Result / private log |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned libevent `d82464a2`; `submodule.log` |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --no-fail-fast` | 0 | 4,884 passed, 0 failed, 10 ignored; 251 strict summaries over 249 targets; `cargo-test.log` |
| `CARGO_BUILD_JOBS=2 cargo fmt --all -- --check` | 0 | clean; `cargo-fmt.log` |
| `CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings` | 0 | clean; `cargo-clippy.log` |
| `CARGO_BUILD_JOBS=2 cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `probe.log` |
| `CARGO_BUILD_JOBS=2 cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | 150 audits; `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | 317 tests; `python.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8; `clang-version.txt` |

The first fresh-worktree submodule attempt, before escalation, exited 1 because
the sandbox could not create the submodule metadata directory under the parent
repository's `.git/worktrees`; it is preserved under
`measure/review-audit-runner-b4c2f7bd/`. An initialized-checkout sandbox gate
then completed all independent steps but its workspace test exited 101: 4,779
passed, 105 failed, 10 ignored over 251 summaries. The failures were emitted
native/sanitizer rows under the sandbox; 88 child diagnostics explicitly said
LeakSanitizer cannot work under ptrace, with other failures aggregating or
asserting over those failed child runs. The complete escalated run above
re-executed the same 251 summaries with 4,884/0/10. The failed run is retained
as environment evidence and is not presented as a successful gate.

No provider path or `crates/qcore-schemas` file changed, so provider count 122
is carried Unix evidence rather than re-measured. No constraint-16 row failed
in the successful full run, so no isolated retry is claimed. No coverage,
corpus, verdict, contract, CT, or conformance figure moves.

## Limitations

The regressions deterministically control Python children and the spawn call;
they do not reproduce operating-system scheduler interleavings exhaustively.
The supervisor serializes child creation briefly while holding its lifecycle
lock. Failure-log selection follows the first failure observed in completion
order; successful-run log replay is stable in audit order. Windows retains the
existing `taskkill` process-tree implementation and is exercised by CI rather
than this Linux measurement.

## Integrator reconciliation list

1. Amend `docs/agents/validation.md` Lean CI hygiene after its cleanup sentence
   with: “Parallel audit cancellation synchronizes stopped-state inspection,
   child spawn and registration; failures are observed in completion order,
   while successful logs are replayed in audit order, and every pending or
   running worker is collected before return.”
2. Do not update `docs/STATUS.md`: this card changes orchestration only and the
   authoritative coverage, verdict, corpus, CT and conformance figures do not
   move.
3. Record the card in the current increment at integration, newest first, with
   the final integrated gate figures. Do not copy this card's pre-integration
   commit identity if reconciliation changes the tree.
4. No acceptance-matrix entry is retired or added by this card; the review R1
   and R2 labels were external review priorities rather than registered matrix
   residuals.
