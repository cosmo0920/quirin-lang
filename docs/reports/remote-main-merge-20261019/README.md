# origin/main conflict resolution, 2026-10-19

Local parent: `f35b5cf4b596e1ad2f5bdec6a0c203e2fd987287`.
Incoming parent: `400cc7dea1731f1c53c46ccc7fe59f89ffb57001`.

The only conflict was `dimensions.named_followups` in the acceptance matrix.
The local branch discharged `LEXICAL-SCALAR-NOOP-ALIAS-READ` (805 → 804),
while origin/main registered `CONTRACT-HYPOTHESIS-GRAPH-V1` (805 → 806).
The combined array has 805 entries. The matrix generator independently
recomputed that dimension with the local card's existing gate receipt; this
checks bookkeeping and does not constitute a new corpus measurement.
The local discharge and incoming candidate are both retained. The matrix's
current update note explains the combined count; historical counts retain
their original branch context.

The incoming plan and documentation link are retained. Its new increment
paragraph moves from part 9 into the current part 10, following the increment
index's integration instructions. No analyzer, contract or proof source is
changed by this merge, and no safety or coverage movement is claimed.

Validation logs: `private/remote-main-merge-20261019/`.
The initial sandbox `git fetch origin` exited 255 because `.git/FETCH_HEAD`
was read-only. The host-access fetch succeeded (exit 0).
`git merge --no-commit --no-ff origin/main` exited 1 for the documented content
conflict; `git submodule update --init --recursive` subsequently exited 0.
The matrix generator exited 0; its output and exit are retained in the private
log directory. Standing-gate results follow.

| Command | Exit | Result / private log |
| --- | ---: | --- |
| `python3 scripts/run_test_binaries.py` (sandbox) | 101 | 519 units / 532 summaries; 8,221 passed, 107 failed, 15 ignored; `rust.log`; runner logs `private/test-binaries/20261019T061955Z-21/` |
| `cargo fmt --all -- --check` | 0 | `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | 343 audits; `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | 374 tests; `python.log` |

The sandbox run's native-test failures include LeakSanitizer's explicit
ptrace restriction. No sanitizer or test expectation was disabled; the Rust
gate was repeated with host access and fresh private logs.

The host-access `python3 scripts/run_test_binaries.py` exited **0**:
**519 units / 532 summaries; 8,328 passed, 0 failed, 15 ignored**.
Its console log and exit are `rust-host.log` and `rust-host.exit` in the
private directory above; per-unit logs and the machine-readable summary are
in `private/test-binaries/20261019T063044Z-2065826/`.
The host rerun accounts for all 107 first-run failures without changing a
source or expectation. All final standing-gate commands exited 0.
Counts agree with the local card's receipt, so no differing integration
measurement or new corpus gate is published. Provider sources and packages
are unchanged; no provider gate was required or run for this merge.

`git diff --check` passed, and the index has no unmerged entries.
The follow-up IDs were also compared with the two parents and their common
ancestor: both branches' additions and removals are preserved exactly.
