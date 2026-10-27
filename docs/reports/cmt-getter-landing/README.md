# CMetrics getter landing checkpoint

The merged 808d completion is `f6591ceb4`; the measured integration tip is
`b403cd4deddaab711f446f383270363aca39a2af`, compared with published
`d4c43b5accdf126523be0089183f346c09f0c281`. The fresh three-route measure
command exited 0 with stable inputs/binary, no unmatched rows and no composition
conflicts. Exactly `cmetrics::cmt_map.c::cmt_map_metric_get` changes Unknown to
provisional Unsafe, adding one leak leaf and removing none. Corpus becomes
1,013 Safe / 3,811 Unknown / 10 provisional Unsafe (4,834 functions); its units
are 8 Safe / 165 Unknown / 8 provisional Unsafe / 4 data-only. System stays
101 / 343 / 4; slice stays 57 Safe / 342 Unknown / 4 Pending / 1 provisional
Unsafe. There is no Safe regression and no other resource leaf change.

All 4,921 corpus NULL dictionaries are identical. The NULL carrier's custody
and combined fields change only for the same getter, which explains the
profile's changed NULL-row digest without a NULL-property verdict movement.
System and slice verdict digests are unchanged. The outer document changes
only custody/combined Unknown and Unsafe tallies by one in the static view
and each existing scenario; its NULL tallies, contracts, loaders, selection
and envelope coverage are unchanged.

[Native receipt](../cmt-getter-ownership-closure/integration/native-receipt.json),
[complete comparison](../cmt-getter-ownership-closure/integration/native-comparison.json.gz),
[NULL attribution](../cmt-getter-ownership-closure/integration/null-attribution.json)
and [outer attribution](../cmt-getter-ownership-closure/integration/outer-attribution.json)
own this checkpoint. The receipt binds compressed complete raw report pairs,
including components, contracts, source effects, NULL and outer documents.
The historical card README's `native/receipt.json` link was absent from the
received tracked tree; this independent evidence is published beside that
checkpoint, without rewriting its historical claims. The retained historical
`native-witness.json` remains separate evidence about the public hash-zero
storage witness, not a fresh runtime experiment in this landing.

## Gate and publication

The full source gate runs on the merged code at `b403cd4de` using fresh private
logs `private/codex-cmt-landing-20261027/host-full/`. Root Rust: 8,832 passed,
0 failed, 15 ignored (571 units / 590 summaries). Lean: 388 audits. Python:
554 tests, no skips. Both reevaluation probes, root fmt/Clippy and all five
provider package builds/verifications exited 0. Provider Rust: 133 passed,
0 failed, 1 ignored (43 summaries); provider fmt/Clippy also
exited 0. [Gate](../cmt-getter-ownership-closure/integration/gate.json) and
[exact commands, exits, counts and private log paths](../cmt-getter-ownership-closure/integration/gate-steps.json)
record the complete run. The Clang recording was captured in this worktree
at the start of that gate. Records are committed before a fresh Python rerun
and mandatory `python3 scripts/land.py publish`; that command remeasures the
final committed revision and refuses any committed-profile mismatch.

The initial sandbox gate was terminated (exit 143) after native LeakSanitizer
fatal errors under sandbox process restrictions. Its 237 completed unit logs /
256 observed summaries are partial outputs, not validation evidence; no checks
were disabled. The first measurement preparation was terminated (143) before
auditing after a mistaken baseline argument. The corrected `native-2` run above
is the measurement. Private logs and the aborted-attempt receipt are retained
under `private/codex-cmt-landing-20261027/`.
