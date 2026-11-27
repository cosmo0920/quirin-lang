# Combined validation

Implementation base: `9a38004529180498c3d28532c2feb718ca198e7a`.
Report checkpoint before final gate recording: `ed4b4932`. The gate ran while
report-only files were finalized. Compiler, crates, proof, contracts, providers
and root scripts are unchanged from the base. No new semantic proof is claimed.
`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=4`; test runner and Lean use 4 jobs.

| Step | Command | Exit | Result |
|---|---|---:|---|
| Fresh-worktree submodules | `git submodule update --init --recursive` | 0 | initialized |
| clang | `clang --version` | 0 | measured in this worktree |
| rust | `python3 scripts/run_test_binaries.py --jobs 4` | 0 | 291 units / 301 summaries; 5,676 passed / 0 failed / 11 ignored |
| fmt | `cargo fmt --all -- --check` | 0 | clean |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | passed |
| followup | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | passed |
| lean | `python3 scripts/check_lean_audits.py --jobs 4` | 0 | 191 audits passed; build passed |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 351 tests passed |

Source census: 15/15 preprocessing commands exit 0. Retry and accounting
source-pin verification each exit 0. The isolated request-allocation harness
compile and execution each exit 0; its result is about the disclosed stubs and
exact function, not a production scheduler. Report JSON/Python syntax, report
index and `git diff --check` pass. The Python suite's `fatal: bad revision
not-a-real-task-context-base` is its intentional negative-control output, not
a failed gate. Each delegated investigation preserves its initial invalid
routing-topic query (exit 2) and corrected query (exit 0).

The optional Astra backlog experiment returned a cybersecurity safety rejection
before any execution evidence. It is not included in passed experiment counts.
Its source-level finding remains unvalidated.

## Private logs and archived evidence

The worktree is `/home/cosmo/.codex/worktrees/fb-system-audit-cards/qcore-c`.
Each step has a separate `.log` and `.exit` in
`private/fb-system-audit-cards/gate/`; `results.json` records exact commands,
exits and timings. Submodule logs sit one directory above. Individual Rust
binary logs and summary are in
`private/test-binaries/20260925T162214Z-3864512/` (summary.json).
`validation.json` records absolute paths for direct resumption. Census raw
preprocessing outputs live in `private/fb-system-audit-cards/census-run/`;
its source/command/digest inventory is archived in census/evidence.json.
The retry experiment's original agent logs are archived in retry/artifacts/;
its executable is omitted, and its generator recreates the experiment into a
fresh private directory. No provider-only gate was required by these changes.
No pinned-corpus arbiter was rerun: the baseline was read for coverage census,
and this work changes no analyzer or contract semantics.
