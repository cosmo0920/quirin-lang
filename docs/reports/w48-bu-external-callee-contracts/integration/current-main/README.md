# Integration with advanced main

While the first merged-source gate and post-matrix check ran, another chat
merged and pushed result-constructor main `81df98c9d`. That source is merged
without conflicts into this integration; its earlier 8,601-test checkpoint
remains under the parent integration directory.

The full standing/provider gate is pending on the clean merged source.
The matrix is regenerated for 370 Lean audits while explicitly carrying the
prior gate provenance until this fresh combined-source run completes. No
combined native verdict counts are inferred.

The first refreshed attempt (`b47349336`) completed all 19 commands with two
Rust test failures (8,618 passed, 2 failed, 15 ignored) and one root Python
failure (373 of 374 passed). All provider commands, 370 Lean audits and the
other standing commands passed. `attempt-1/receipt.json` preserves its actual
exits and logs. The failures are stale rule-37 expectations: the loader mode
table, libc inheritance list, and symbolic-adoption preservation checkpoint.

The repair changes tests only. `pin-repair.json` and the tracked test fixture
pin the complete pre/post-rule-37 memory source hashes and four literal
source hunks. Historical symbolic erasure still compares every original byte;
current typed mode parity admits only the two reviewed checked non-null
results and verifies the exact checked-specification set. Extra bytes and a
changed checked-body marker are refused. The frozen evidence and production
contracts/code/Lean remain unchanged. A focused retry and complete standing
gate follow. Provider results retain their actual `b47349336` source, with
production input equality checked across this test-only repair.
