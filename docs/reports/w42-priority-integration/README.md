# Cycle 19 priority implementation

Requested 2026-09-20; implementation base
`f7b010debd018a3516f919224afcb3ca6682f3f9`. The integration branch is
`codex/cycle19-priority-wave`. Three implementation/measurement workers use
GPT-5.6 Sol at medium reasoning effort in isolated worktrees.

## Selected work

* W42-INDIRECT-ACCESS: bounded scalar-member stores through tracked live bases,
  paired with the already integrated B02 condition evaluation.
* W42-MONKEY-CALLBACK-COVERAGE: total accounting for the selected forced-Libevent
  callbacks, with explicit Unknown reasons where composed certificates are absent.
* W42-UNPROJECTED-DEFINITIONS: reproduce and classify the raw-definition versus
  projected-function difference and measure the unit reason distribution.

The [plan](../../plans/cycle19-custody-cards/README.md) additionally defines
W42-CHUNK-LIFECYCLE-CERT, including the nested ChunkIO teardown dependencies.
That certificate work and static-helper summaries remain queued. No dispatch
or syntax-admission count closes Cycle 19.

## Fresh baseline

The release audit rebuilt from the unchanged source and exited **0** against
the pinned source and forced-Libevent compilation database. All **five**
artifacts reproduce the B-series baseline **byte for byte**, including raw
contract keys. [Byte identities](baseline-byte-identity.json) and
[command, source commit and binary digest](baseline-run.json) bind the run.
Private logs and raw outputs are under
`/home/cosmo/Documents/GitHub/qcore-c/private/w42-priority/baseline/`;
the release build log is `private/w42-priority/baseline-build.log`.

This BEFORE measurement is not a gate or an AFTER result for the new work.
Raw units remain unpromoted; provisional Unsafe results are not validated
upstream defects. Lean gateway models retain the extraction and Rust-model
correspondence premises in evidence rule 28.

## Integration status

Implementation and review are in progress. Final changes, measured deltas,
gate results and reconciliation decisions must be recorded before completion.
