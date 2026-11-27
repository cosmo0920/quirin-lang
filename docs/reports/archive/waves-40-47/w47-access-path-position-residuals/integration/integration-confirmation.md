# Wave-47 round-2b integration

**Branch** `integrate/w47-r2b`: `main` `2a0d37b0` (round 2a) + merge `2bf8b732` (W47-ACCESS-PATH-POSITION-RESIDUALS,
measured from the round-1 baseline) + `9135e7a1` (its surviving ledger amendment) + a restored pin.

**Merge.** One code conflict, `typed_access_path.rs` `Reader::path`: both admissions are kept (the card's address
target and rule 10m's `pointer_value`). Records: gateway 10n placed between 10m and 10o with its router row,
`wave-47.md` newest first, the plan's P3 note (both), STATUS (figures below), the matrix (the card's `update_ledger.py`
refuses as a whole because main already retired two of its three targets; the surviving amendment was applied
directly), the index.

## Gate

| Step | Exit | Count |
|---|---:|---|
| submodule, fmt, clippy, reevaluation probe / followup, docs index | 0 | |
| `run_test_binaries.py` at `9135e7a1` | 101 | one pinned test (below) |
| `run_test_binaries.py` after the restored pin | 0 | 344 units, 357 summaries; 6,172 passed / 0 failed / 11 ignored |
| `check_lean_audits.py` | 0 | 220 |
| `scripts/tests` | 0 | 365 |
| provider cargo test / fmt / clippy / `contracts.py check` / 5 package build+verify | 0 | 129 passed, 1 ignored |
| `providers/tests` | 0 | 24 OK |
| release build, corpus arbiter, system slice | 0 | |

**The failure was a composition effect.** The card re-pinned `&global_queue.items` at a `Borrowed` head as `Safe`
(rule 10n's address fallback). On the merged tree rule 10o counts the address taken of `global_queue` as a possible
binding, so the never-bound premise fails and the call is refused before the effect: `Unknown`, nothing accused —
exactly main's earlier pin, which is restored (`a_member_of_a_static_record_is_still_refused_before_the_effect`)
with a comment naming both rules.

## Composition ([corpus](compose-corpus.log), [system](compose-system.log))

- **System:** COMPOSES, 0 mismatches; the card moves no system function.
- **Corpus:** 527 Safe / 10 / 4,297 against 528 additive (516 + 12). Of the card's twelve gains eleven land;
  `rd_kafka_toppar_needs_query` (its unforecast summary gain) stays Unknown. No Safe of main is lost; no Unsafe or leak
  is added. 33 Unknown functions differ from the additive leaf sets, all in the stricter direction: the merged run keeps
  refusals the card alone removed (37 indirect-access, 7 `for`-condition, 2 branch-condition leaves, plus the
  uncontracted-call leaves of callers of `needs_query`); the six leaves the merged run lacks are alias leaves the card's
  admissions exposed, absent because those admissions do not happen. The address class is recorded under
  `W47-JOIN-AST-CONSERVATIVE-CLASSES`.

## Combined figures (new baseline: [artifacts/](artifacts/))

| Measure | Round 2a | Round 2b |
|---|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 516 / 10 / 4,308 | 527 / 10 / 4,297 |
| Units | 6 / 8 / 167 / 4 | unchanged |
| leak | 36 | 36 |
| unknown_alias | 7,746 | 7,927 |
| unsupported | 36,154 | 35,741 |
| System unique functions | 31 / 2 / 201 | unchanged |
| System unsupported / unknown_alias | 2,890 / 1,020 | 2,888 / 1,020 |
