# B01–B04 integration

Implementation and integration record for the four Cycle-19 call-position cards,
requested 2026-09-20. Base: `8e633261fd216a76d4d25390b0dadeda891c59aa`.

## Scope and dispatch

B01 return expressions, B02 branch conditions, B03 loop/switch conditions, and
B04 nested call arguments are assigned to GPT-5.6 Sol agents at medium effort
in isolated worktrees. Three worker slots are available; B04 follows the first
completed worker. The user explicitly selected these four cards. The planning
recommendation to pair B02 with W42-INDIRECT-ACCESS is not an authorization to
claim that separate card complete: indirect-access blockers remain in scope
only where an individual B card explicitly models and measures them.

## Baseline reproduction

The integrator rebuilt the release audit binary from the unchanged base and
ran it against the existing pinned source, compile database and ChunkIO package.
The audit exited 0; its command, binary digest, source commit and timestamps are
in [baseline-run.json](baseline-run.json). Both verdict-bearing artifacts
(`diagnostics` and `chunk-contracts`) reproduce the wave-37 baseline byte for
byte. `source` is also byte-identical. The two remaining artifacts differ in
checkout/configuration identities: their complete structural comparison is
[baseline-artifact-diff.json](baseline-artifact-diff.json), including normalized
contract-module keys. No module content, selection or manifest entry changed.
The raw comparison is [baseline-byte-identity.txt](baseline-byte-identity.txt).

Private raw artifacts and logs are in
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/`.
These are measured baseline evidence, not an AFTER result or a gate for the
four implementations. Raw units remain unpromoted; provisional Unsafe findings
are not validated upstream defects. Lean rule models retain the extraction and
Rust correspondence premises of evidence rule 28.

## Integration state

Implementation, review, combined measurement and standing validation are pending.
