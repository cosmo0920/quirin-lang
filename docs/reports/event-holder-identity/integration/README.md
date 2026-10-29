# Holder-identity landing gate

The merged source is `3ee2c5ac1`, integrating
`codex/event-holder-identity` at `dbc494f0e15f3b36153401bb7e5459951945ca14`
with published main `80c8aea989029c10b3dfbd0179715b7ea050262c`.
The full merged source/provider gate passed every command (exit 0): Rust
8,847 passed / 0 failed / 15 ignored (574 units / 593 summaries, 24 filtered),
Lean 391 audits, Python 554 tests with no skips, provider Rust 133 passed /
0 failed / 1 ignored (43 summaries), both reevaluation probes, root/provider
fmt and Clippy, and all five provider package builds/verifications.

[Exact commands, exits, counts and private log paths](gate-steps.json) retain
the fresh merged run. The selection comes from the fresh full landing plan;
`validation_tip` identifies this run and every command executes in main.
Clang was captured in this worktree at the start of this gate. The figures
match the card gate, so no second flat matrix gate is introduced. Card-native
observations remain their own checkpoint; final merged publication is a fresh
three-route `land.py publish` on the committed landing records, compared with
published main 80c8aea98. No outcome of that future command is inferred here.

The card amendments were applied with `--profiles-base 80c8aea98` after a dry
run. FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME remains open. Matrix/index/contribution
records are regenerated, checked and committed before the required Python
rerun and final publication. Private landing logs are under
`private/event-holder-landing-20261029/`; final baseline/publication and push
receipts are recorded there only on success. The preliminary matrix command used an absent wrapper path (exit 2);
the documented build_matrix.py command succeeded (exit 0) before the merge.
The command and correction are retained in setup-attempts.json. The initial full run later exhausted disk space before provider tests and provider Clippy could execute (exits 1 and 101, no provider test summaries). The original final/slot receipts were truncated during the failed writes. Exact commands, exits and durations were recovered from the retained driver transcript; counts were re-read from each original log. The failed attempt remains in failed-disk-gate.json, with initial-driver.log beside it. After removing the disposable root Rust incremental cache, only those affected provider steps were rerun under normal gate-slot isolation. gate-steps.json composes the original successful commands and the successful provider reruns on the identical clean merged source. provider-repair-gate.json retains those fresh commands and exits. No failed attempt supplies final gate credit.

