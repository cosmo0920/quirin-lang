# Ordered-lifetime landing gate

The merged source is `153fd9cc3`, integrating
`codex/event-ordered-lifetime` at `e8383f36200bb921369046381a8043fda762679f`
with published main `facdb5f4ec1806c38f09f953f4c5b2ae463386e5`.
The full merged source/provider gate passed every command (exit 0): Rust
8,842 passed / 0 failed / 15 ignored (573 units / 592 summaries, 24 filtered),
Lean 390 audits, Python 554 tests with no skips, provider Rust 133 passed /
0 failed / 1 ignored (43 summaries), both reevaluation probes, root/provider
fmt and Clippy, and all five provider package builds/verifications.

[Exact commands, exits, counts and private log paths](gate-steps.json) retain
the fresh merged run. The selection was reused from the card's full plan;
`validation_tip` identifies this run and every command executes in main.
Clang was captured in this worktree at the start of this gate. The figures
match the card gate, so no second flat matrix gate is introduced. Card-native
observations remain their own checkpoint; final merged publication is a fresh
three-route `land.py publish` on the committed landing records, compared with
published main facdb5f4. No outcome of that future command is inferred here.

The card amendments were applied with `--profiles-base facdb5f4` after a dry
run. FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME remains open. Matrix/index/contribution
records are regenerated, checked and committed before the required Python
rerun and final publication. Private landing logs are under
`private/event-ordered-landing-20261028/`; final baseline/publication and push
receipts are recorded there only on success. The first fresh landing gate exhausted disk space and is not validation. Its
logs remain in private/event-ordered-landing-20261028/disk-failed-gate and
disk-failed-driver.log. Disposable build outputs from this task's completed
caller/ordered cards were reclaimed. The fresh full gate here follows cleanup.
