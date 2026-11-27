# Caller-removal landing gate

The merged source is `c265a31fadf7e69933fb106ce08408789944a26e`, integrating
`codex/event-caller-removal` at `49c77fad11936971f02e439f5ab476b8678ad51c`
with published main `9348f813444a25ef710ad6096cb88469913176fa`.
The full merged source/provider gate passed every command (exit 0): Rust
8,834 passed / 0 failed / 15 ignored (572 units / 591 summaries, 24 filtered),
Lean 389 audits, Python 554 tests with no skips, provider Rust 133 passed /
0 failed / 1 ignored (43 summaries), both reevaluation probes, root/provider
fmt and Clippy, and all five provider package builds/verifications.

[Exact commands, exits, counts and private log paths](gate-steps.json) retain
the fresh merged run. The selection was reused from the card's full plan;
`validation_tip` identifies this run and every command executes in main.
Clang was captured in this worktree at the start of this gate. The figures
match the card gate, so no second flat matrix gate is introduced. Card-native
observations remain their own checkpoint; final merged publication is a fresh
three-route `land.py publish` on the committed landing records, compared with
published main 9348f813. No outcome of that future command is inferred here.

The card amendments were applied with `--profiles-base 9348f813` after a dry
run. FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME remains open. Matrix/index/contribution
records are regenerated, checked and committed before the required Python
rerun and final publication. Private landing logs are under
`private/event-caller-landing-20261028/`; final baseline/publication and push
receipts are recorded there only on success. The initial matrix regeneration
used incorrect CLI/environment arguments and exited nonzero before the gate;
the corrected documented generator command passed before the merge commit.
