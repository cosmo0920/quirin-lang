# Wave-47 condvar-join integration

**Branch** `integrate/w47-condvar`: `main` `27854776` (W47-LOOP-OWNER-COROUTINE-HOP) + merge `1b91394b`
(W47-CONDVAR-JOIN-PUBLICATION).

**Merge `1b91394b`.**

```
Merge W47-CONDVAR-JOIN-PUBLICATION onto main after the coroutine hop

No code conflicts; the card's census builds against main's census API and
its 35 census tests pass on the merged tree. Records: rule 10q beside
main's 10p amendment, router and STATUS rows (main's 10p row kept, the
card's 10q row added), wave-47.md newest first, matrix (main + the card's
update_ledger.py: 547 -> 550), INDEX.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

No code conflicts. Parents: `27854776` (main after the coroutine hop) and `17268cf7` (W47-CONDVAR-JOIN-PUBLICATION
gate records, matrix and index).

## Gate

Full gate measured in `private/w47-int-g/gate-1b91394b`, every exit captured to a file.

| Step | Exit | Count |
|---|---:|---|
| submodule | 0 | |
| `run_test_binaries.py` | 0 | 348 units, 361 summaries; 6,240 passed / 0 failed / 11 ignored |
| clean-dev, fmt, clippy | 0 | |
| reevaluation probe / followup | 0 | |
| `check_lean_audits.py` | 0 | 223 |
| `scripts/tests` (root Python) | 0 | 365 |
| docs index | 0 | |
| provider cargo test | 0 | 129 passed / 0 failed / 1 ignored |
| provider clean-dev / fmt / clippy / contracts-check | 0 | |
| package build, 5 package verifies (chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 | |
| provider-python | 0 | |
| clang-version | 0 | Ubuntu clang version 21.1.8 (6ubuntu1) |
| release build (`quirin-audit`) | 0 | |
| corpus arbiter | 0 | |
| system slice | 0 | |
| tail-pause slice (`w47-tail-pause-bindings/scripts/run-current.sh attempt-1 int-1b91394b`) | 0 | |

`cycle20_verifier_plugins` passed cleanly (20 passed / 0 failed) inside the single `run_test_binaries.py` pass, so
no rerun of the known load-flake test was needed.

## Composition ([corpus](compose-corpus.log), [system](compose-system.log), [slice](compose-slice.log))

Base for all three: `docs/reports/w47-loop-owner-contracts/artifacts/after/<scope>` (the round-1 baseline, byte-identical
to main's coroutine hop and the figure the condvar card measured from), composed against itself and against the card's
`docs/reports/w47-condvar-join-publication/artifacts/after/<scope>` (AFTER).

- **Corpus:** COMPOSES, 0 mismatches, 0 functions/leaves touched by both cards. Merged functions: 526 Safe / 10
  Unsafe / 4,298 Unknown (matches the expected 526/10/4,298). Merged units: 6 Safe / 8 Unsafe / 167 Unknown / 4
  data_only. Diagnostic counts: leak 36, summary_conflict 1, unknown_alias 7,936, unsupported 35,737.
- **System:** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 31 Safe / 2 Unsafe / 201 Unknown (matches
  the expected 31/2/201). Merged units: 7 Unknown / 1 Unsafe. Diagnostic counts: counted_user 4, unknown_alias 1,020,
  unsupported 2,888.
- **Slice (tail-pause):** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 24 Safe / 1 Unsafe / 245
  Unknown (matches the expected 24/1/245). Merged units: 12 Unknown / 1 Unsafe. Diagnostic counts: counted_user 1,
  unknown_alias 1,009, unsupported 2,857.

All three scopes compose cleanly against the card's own AFTER measurement with no function or leaf claimed by both
the base and the card, and no mismatch — the condvar-join card's changes are additive over the round-1 baseline with
no analyzer regression.

## Figures (merged, this integration)

| Scope | Safe | Unsafe | Unknown | Units |
|---|---:|---:|---:|---|
| Corpus (functions) | 526 | 10 | 4,298 | 6 / 8 / 167 / 4 (Safe/Unsafe/Unknown/data_only) |
| System (functions) | 31 | 2 | 201 | 7 Unknown / 1 Unsafe |
| Slice, tail-pause (functions) | 24 | 1 | 245 | 12 Unknown / 1 Unsafe |
