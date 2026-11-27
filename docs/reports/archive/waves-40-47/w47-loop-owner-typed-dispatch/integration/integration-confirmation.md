# Wave-47 typed-dispatch integration

**Branch** `integrate/w47-typed-dispatch`: `main` `e56e0af9` (W47-CONDVAR-JOIN-PUBLICATION) + merge `70605056`
(W47-LOOP-OWNER-TYPED-DISPATCH).

**Merge `70605056`.**

```
Merge W47-LOOP-OWNER-TYPED-DISPATCH onto main after the condvar join

No code conflicts; 45 census tests pass on the merged tree. Records: the
card's 10p amendment placed inside 10p before main's 10q, the card's 10p
STATUS row with main's 10q row kept, wave-47.md newest first, matrix
(main + the card's update_ledger.py: 550 -> 551), INDEX.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

No code conflicts. Parents: `e56e0af9` (main after the condvar join) and `9e35573f` (W47-LOOP-OWNER-TYPED-DISPATCH
gate records, matrix and index).

## Gate

Full gate measured in `private/w47-int-h/gate-70605056`, every exit captured to a file.

| Step | Exit | Count |
|---|---:|---|
| submodule | 0 | |
| `run_test_binaries.py` | 0 | 349 units, 362 summaries; 6,250 passed / 0 failed / 11 ignored |
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
| tail-pause slice (`w47-tail-pause-bindings/scripts/run-current.sh attempt-1 int-70605056`) | 0 | |

`cycle20_verifier_plugins` passed cleanly (20 passed / 0 failed) inside the single `run_test_binaries.py` pass, so
no rerun of the known load-flake test was needed.

## Composition ([corpus](compose-corpus.log), [system](compose-system.log), [slice](compose-slice.log))

Base for all three: `docs/reports/w47-condvar-join-publication/integration/artifacts/<scope>` (the previous
integration's merged baseline), composed against itself three times, per the typed-dispatch card's own §6 claim
that the per-unit routes read no census and no typed fact, so none may move — the merged run is expected to equal
main's (pre-card) figures exactly.

- **Corpus:** COMPOSES, 0 mismatches, 0 functions/leaves touched by both cards. Merged functions: 526 Safe / 10
  Unsafe / 4,298 Unknown (matches the expected 526/10/4,298). Merged units: 6 Safe / 8 Unsafe / 167 Unknown / 4
  data_only. Diagnostic counts: leak 36, summary_conflict 1, unknown_alias 7,936, unsupported 35,737.
- **System:** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 31 Safe / 2 Unsafe / 201 Unknown (matches
  the expected 31/2/201). Merged units: 7 Unknown / 1 Unsafe. Diagnostic counts: counted_user 4, unknown_alias 1,020,
  unsupported 2,888.
- **Slice (tail-pause):** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 24 Safe / 1 Unsafe / 245
  Unknown (matches the expected 24/1/245). Merged units: 12 Unknown / 1 Unsafe. Diagnostic counts: counted_user 1,
  unknown_alias 1,009, unsupported 2,857.

All three scopes compose cleanly against the unchanged baseline with no function or leaf claimed by both composed
copies and no mismatch — the typed-dispatch card changes no per-unit verdict, so the merged run equals main's.

## Census re-derivation on the merged tree

Rebuilt `quirin-source-analysis`'s `typed_dispatch_facts` and `loop_owner_census` examples (release) and the
`condvar_join_census` example (debug, via `docs/reports/w47-condvar-join-publication/scripts/census.sh`) from the
merged tree at `70605056`, using the same prepared fluent-bit-dev exports the cards used
(`private/w47-tail/attempt-1/prep` for `90978ac6`, `private/w45-fb/w46-external-callee-contracts/attempt-1` for the
`90978ac6~1` test file), read-only.

**Typed loop-owner census** (`docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py` +
`scripts/census.py`, all four sub-steps exit 0): `same_census.py` against
`docs/reports/w47-loop-owner-typed-dispatch/artifacts/census` reports all four readings (`fixed`, `previous`,
`fixed-configured`, `previous-configured`) byte-identical (SHA-256 match) to the card's archive.

| `90978ac6` (fixed) | discharged | not discharged | unresolved |
|---|---:|---:|---:|
| Configured, after typed reading | 8 | 130 | 10 |

| `90978ac6~1` (previous) | discharged | not discharged | unresolved |
|---|---:|---:|---:|
| Configured, after typed reading | 8 | 136 | 7 |

Both equal the card's archived AFTER figures exactly (§4 of the card's README: `8 / 130 / 10` at `90978ac6`,
`8 / 136 / 7` at `90978ac6~1` configured).

**Condvar/join census** (`docs/reports/w47-condvar-join-publication/scripts/census.sh`, build/fixed/previous all
exit 0): SHA-256 of the regenerated `fixed.json` and `previous.json` match
`docs/reports/w47-condvar-join-publication/artifacts/census/{fixed,previous}.json.gz` exactly (decompressed). The
card's archived C4/C5 (`release`/`signal`/`wait`/`read`/`init`/`destroy`/`broadcast`) tallies therefore hold
unchanged on the merged tree:

| `90978ac6` (fixed) | discharged | not discharged | unresolved |
|---|---:|---:|---:|
| release | 97 | 134 | |
| signal | 5 | 4 | 3 |
| wait | 6 | 1 (+1 refused) | 6 |
| read | 8 | | 15 |
| init | 3 | | 9 |
| destroy | 1 | 1 | 11 |
| broadcast | | | 1 (+4 refused) |

| `90978ac6~1` (previous) | discharged | not discharged | unresolved |
|---|---:|---:|---:|
| release | 97 | 131 | |
| signal | 4 | 4 | 3 |
| wait | 5 | 1 (+1 refused) | 6 |
| read | 3 | | 15 |
| init | 2 | | 9 |
| destroy | 1 | | 11 |
| broadcast | | | 1 (+4 refused) |

Both revisions' census outputs are byte-identical to the cards' archives, so census re-derivation confirms the
typed-dispatch merge changed neither census's inputs nor outputs.

## Figures (merged, this integration)

| Scope | Safe | Unsafe | Unknown | Units |
|---|---:|---:|---:|---|
| Corpus (functions) | 526 | 10 | 4,298 | 6 / 8 / 167 / 4 (Safe/Unsafe/Unknown/data_only) |
| System (functions) | 31 | 2 | 201 | 7 Unknown / 1 Unsafe |
| Slice, tail-pause (functions) | 24 | 1 | 245 | 12 Unknown / 1 Unsafe |
