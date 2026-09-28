# Wave-47 in_tail resolution integration

**Branch** `integrate/w47-r6`, `HEAD` `4290a0cb` = `main` `fefb9199` + `W47-CONDVAR-JOIN-STOP-PATHS` (fast-forward
`7226d6ea`) + merge `1b185b55` (`W47-LOOP-OWNER-INSTANCE-FLOW` onto `W47-CONDVAR-JOIN-STOP-PATHS`) + merge `4290a0cb`
(`W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL` onto the in_tail resolution integration).

**Merge `1b185b55`.**

```
Merge W47-LOOP-OWNER-INSTANCE-FLOW onto W47-CONDVAR-JOIN-STOP-PATHS

Both cards add one review-held Fluent Bit contract module. Composed:
- contracts.lock: structural JSON union (55 base + 5 + 4 = 64 pinned,
  no shared entry changed on either side), ordered by function as the
  pin tool writes it;
- registration lists sorted (input_instance before lib_engine_thread) in
  contract-load-path.json, cycle19-resource-contracts.json and the
  provider lib.rs (the keep-both merge had fused two tuples);
- count pins to base + both: selected 56 -> 58, corpus 120 -> 122,
  quirin-provider modules 47 -> 49, v1 declarations 242 -> 251 with 9
  withheld for review; the instance-flow card's duplicate assertion block
  collapsed;
- rule router: the instance-flow 10p row and the stop-paths 10q row;
  STATUS rows by three-way row merge, corpus row to the combined figures;
  wave-47.md newest first; matrix (+ update_ledger.py: 555 -> 558).
Pin tests (contract_loader, qcore_contract_corpus, provider_selection,
neighbours, prod_contracts, scripts/tests corpus, providers/tests) pass.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Merge `4290a0cb`.**

```
Merge W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL onto the in_tail resolution integration

No code conflicts. Records: STATUS verdict-vocabulary row carries both
the fix's standing rule and the instance-flow deferred-declaration note;
follow-ups 555 -> 560 (instance-flow +3, the fix +3 and retires its own
entry, registered on the instance-flow branch); wave-47.md newest first;
matrix (+ the fix's update_ledger.py) and INDEX regenerated.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

No code conflicts in either merge.

## Gate

Full gate measured in `private/w47-int-i/gate-4290a0cb`, every exit captured to a file.

| Step | Exit | Count |
|---|---:|---|
| submodule | 0 | |
| `run_test_binaries.py` | 0 | 352 units, 365 summaries; 6,287 passed / 0 failed / 11 ignored |
| clean-dev, fmt, clippy | 0 | |
| reevaluation probe / followup | 0 | |
| `check_lean_audits.py` | 0 | 224 |
| `scripts/tests` (root Python) | 0 | 365 |
| docs index | 0 | |
| provider cargo test | 0 | 129 passed / 0 failed / 1 ignored |
| provider clean-dev / fmt / clippy / contracts-check | 0 | |
| package build, 5 package verifies (chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 | |
| provider-python | 0 | 24 passed |
| clang-version | 0 | Ubuntu clang version 21.1.8 (6ubuntu1) |
| release build (`quirin-audit`) | 0 | |
| corpus arbiter | 0 | |
| system slice | 0 | |
| tail-pause slice | 0 | |

`cycle20_verifier_plugins` passed cleanly (20 passed / 0 failed) inside the single `run_test_binaries.py` pass, so
no rerun of the known load-flake test was needed.

## Pin checks

`quirin-audit pin --check` for `quirin-provider-fluent-bit`, at the three trees the two cards used:

| Tree | Restricted to instance-flow's 4 functions | Restricted to stop-paths' 5 functions | Whole crate |
|---|---:|---:|---|
| `66910c10` (corpus source) | 0 (4 recorded) | 0 (5 recorded) | 0 (64 recorded) |
| `046accf6` (private clone) | 0 (4 recorded) | 0 (5 recorded) | 0 (64 recorded) |
| `90978ac6` (export) | 0 (4 recorded, identity `34155955`) | 0 (5 recorded, identity `34155955`) | **1** (13 recorded, 51 in force only through the `046accf6`/`66910c10` identity, 0 not in force, 0 stale) |

The whole-crate exit 1 at the `90978ac6` export is the documented pre-existing condition: every failing entry is
"in force through the identity reviewed at git `046accf6`/`66910c10`; git `34155955` not recorded" -- older
declarations that the export never carried its own identity for, not a regression from either card. Restricted to
each card's own function set (the check each card ran), the export is clean (exit 0) for both.

`quirin-provider-libc` has 0 pinned declarations (111 flexible entries per `pin --verify-contracts`), so
`pin --check` is exit 0 at all three trees with 0 recorded.

`pin --verify-contracts`: exit 0. Full crate tally: `qcore-provider-chunkio` 6/0, `qcore-provider-cmetrics` 23/0,
`qcore-provider-ctraces` 7/0, `qcore-provider-libevent` 14/0, `qcore-provider-librdkafka` 12/0,
`quirin-provider-cfl` 4/0, `quirin-provider-fluent-bit` 64/0, `quirin-provider-libc` 0/111,
`quirin-provider-mpack` 9/0, `quirin-provider-msgpack` 1/0 (pinned/flexible).

## Composition ([corpus](compose-corpus.log), [system](compose-system.log), [slice](compose-slice.log))

Base for all three scopes: `docs/reports/w47-condvar-join-publication/integration/artifacts/<scope>` (the previous
integration's merged baseline). A = the stop-paths card's own AFTER (`docs/reports/w47-condvar-join-stop-paths/artifacts/after/<scope>`);
the stop-paths card measured no corpus scope, so base stands in for A on corpus, as the card's own README records
(both cards' corpus is byte-identical to the base). B = the instance-flow card's own AFTER
(`docs/reports/w47-loop-owner-instance-flow/artifacts/after/<scope>`).

- **Corpus:** COMPOSES, 0 mismatches, 0 functions/leaves touched by both cards. Merged functions: 526 Safe / 10
  Unsafe / 4,298 Unknown (matches the expected 526/10/4,298 -- both cards' corpus is byte-identical to the base).
  Merged units: 6 Safe / 8 Unsafe / 167 Unknown / 4 data_only. Diagnostic counts: leak 36, summary_conflict 1,
  unknown_alias 7,936, unsupported 35,737.
- **System:** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 31 Safe / 2 Unsafe / 201 Unknown (matches
  the expected 31/2/201). Merged units: 7 Unknown / 1 Unsafe. Diagnostic counts: counted_user 4, unknown_alias 1,020,
  unsupported 2,888.
- **Slice (tail-pause):** COMPOSES, 0 mismatches, 0 touched by both. Merged functions: 24 Safe / 1 Unsafe / 244
  Unknown / 1 Pending. Merged units: 12 Unknown / 1 Unsafe. Diagnostic counts: counted_user 1, unknown_alias 1,009,
  unsupported 2,857.

The stop-paths card had 0 status moves on the slice; the instance-flow card moved `src/flb_lib.c::flb_input`
Unknown -> Pending there, appearing once (not both-touched) in the merged figures. This move is expected to remain
Pending: the review-hold fix card (`W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`) independently measured that this
site stays Pending under the fix, and the merged gate confirms it here (1 Pending, 0 both-touched).

## Census re-derivation

Rebuilt `quirin-source-analysis`'s `typed_dispatch_facts` and `loop_owner_census` examples (release) and the
`condvar_join_census` example (debug, via `docs/reports/w47-condvar-join-publication/scripts/census.sh`) from the
merged tree at `4290a0cb`, using the same prepared fluent-bit exports the cards used
(`private/w47-tail/attempt-1/prep` for the `90978ac6` export, `private/w45-fb/w46-external-callee-contracts/attempt-1`
for the `90978ac6~1` test file), read-only. All sub-steps exit 0.

**Typed loop-owner census** (`docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py` +
`docs/reports/w47-loop-owner-typed-dispatch/scripts/census.py`, all four sub-steps exit 0; 1,685 files read at both
revisions, matching the instance-flow card's own run):

| Reading | discharged | not discharged | unresolved | edges |
|---|---:|---:|---:|---:|
| `90978ac6` configured (fixed) | 8 | 130 | 10 | 33,573 |
| `90978ac6` raw (fixed) | 8 | 126 | 10 | 33,493 |
| `90978ac6~1` configured (previous) | 8 | 136 | 7 | 33,546 |
| `90978ac6~1` raw (previous) | 8 | 132 | 7 | 33,466 |

All four figures match the instance-flow card's own archived AFTER exactly (its README §4: `8/130/10` configured at
`90978ac6`, `8/136/7` configured at `90978ac6~1`). No site changes verdict or first reason between revisions: the
fixed test's 3 sites stay unresolved on the `flb_service_set` closure blocker; the old test's site stays not
discharged.

**Condvar/join census** (`docs/reports/w47-condvar-join-publication/scripts/census.sh`, build/fixed/previous all
exit 0):

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

These match the stop-paths card's own archived tallies (its README's "Whole census: no verdict moves at either
revision (fixed: releases 97 discharged...)"): discharged counts are identical between the two revisions at every
kind, confirming no verdict moved. Note: the not-discharged release count differs by 3 between the fixed test (134)
and the old test (131), not 5 as anticipated in the task brief's summary of the expected figures; this is reported
as a difference rather than a stop condition, since it reproduces the card's own archived numbers exactly rather
than diverging from them.

## Figures (merged, this integration)

| Scope | Safe | Unsafe | Unknown | Pending | Units |
|---|---:|---:|---:|---:|---|
| Corpus (functions) | 526 | 10 | 4,298 | 0 | 6 / 8 / 167 / 4 (Safe/Unsafe/Unknown/data_only) |
| System (functions) | 31 | 2 | 201 | 0 | 7 Unknown / 1 Unsafe |
| Slice, tail-pause (functions) | 24 | 1 | 244 | 1 | 12 Unknown / 1 Unsafe |
