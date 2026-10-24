# Integration with advanced main

The complete standing/provider gate passes on clean combined source
`4d8d85976117f1eaaadd2fc37aef5a3bb172c259`, including CMetrics/main
`f00264193` and card `7d4ced73d`. All 19 commands exit 0:
8,645 Rust tests across 550 units / 563 summaries (0 failed, 15 ignored),
370 Lean audits, 374 root Python tests, 133 provider Rust tests (0 failed,
1 ignored), and 25 provider Python tests. Formatting, Clippy, both reevaluation
examples, inventory and all five package builds/verifies pass.

`gate.json` supplies these actual combined counts; `command-receipt.json`
retains commands, exits, source guards and hashed private logs. The adjacent
Clang recording comes from this exact run. The matrix uses this gate.
The card’s isolated 8,593-test gate and earlier 8,601-test integration gate
remain their own checkpoints. No combined native verdict count is inferred.

The first refreshed attempt on `b47349336` completed all 19 commands with
8,618 Rust passes, 2 failures and 15 ignored; root Python passed 373 of 374.
Provider commands, 370 Lean audits and the other standing commands passed.
`attempt-1/receipt.json` preserves those failures and logs. They were stale
rule-37 expectations in the loader mode table, libc inheritance list and
symbolic-adoption checkpoint.

The local test-only repair on `bc3a01fd6` passed the loader and corpus Python
checks but still failed symbolic graph equality (`attempt-2/receipt.json`).
Its `pin-repair.json` and literal fixture are historical at that commit; the
full standing gate was not launched. Concurrent main supplied the complete
repair, adopted in this merge: exact frozen byte erasure is preserved, graph
controls compare the same current semantic declarations, and checked result
modes require their checked-specification and named premise. The superseded
local fixture was removed. Current metadata, binding, flow and lifecycle
checks remain active. Every provider command was rerun in the final gate.

After matrix regeneration, all 374 Python tests pass on committed source
`ef909ffe2`, with the matrix hash unchanged (`post-matrix-python.json`).
`final-source.json` records the documentation-only difference from the fully
validated source and incorporates main’s later publication/planning records.
