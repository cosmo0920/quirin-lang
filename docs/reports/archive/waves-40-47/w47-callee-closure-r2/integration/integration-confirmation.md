# Wave-47 round-1 integration confirmation

Branch `integrate/w47-r1-typed-root`, HEAD `4b6b26ad` = main `3ba72320` +
`W47-ALIAS-RESIDUALS-CENSUS` + `W47-CALLEE-CLOSURE-R2` +
`W47-TYPED-ROOT-DECLARATION-READER` + two integrator decision commits.

## Card merges and conflict resolutions

- `063f0911` "Record W47-ALIAS-RESIDUALS-CENSUS standing gate" — committed
  directly as the round's base (no merge commit; first card on the branch).
- `eb4427aa` "Merge W47-CALLEE-CLOSURE-R2 onto the alias census" — resolved:
  `wave-47.md` (both paragraphs, newest first), reports `INDEX` (regenerated),
  acceptance matrix (census matrix + the card's `update_ledger.py`; follow-ups
  528 -> 532; regenerated, gate numbers carried until the merged gate runs).
- `ef037632` "Merge W47-TYPED-ROOT-DECLARATION-READER onto the round-1
  integration" — resolved: `STATUS` (round-1 figure rows kept pending the
  combined arbiter; the card's return-expression row; follow-ups 533),
  `wave-47.md` (newest first), reports `INDEX` (regenerated), acceptance
  matrix (round-1 matrix + the card's `update_ledger.py`: 532 -> 533;
  regenerated, gate carried).

## Integrator decision commits

- `7b15c6b8` "Record the user's arena decision: interior reading accepted as
  interim, pool mode registered" — `W47-ARENA-INTERIOR-READING` amended
  (stays open); `W47-ARENA-POOL-RESULT-MODE` registered; follow-ups 533 -> 534.
- `4b6b26ad` "Record the user's OpenSSL/libcurl decision: plugin crates
  composed through the verdict semigroup" — registers
  `W48-PLUGIN-VERDICT-PRESERVATION`, `W48-OPENSSL-CONTRACT-PLUGIN`,
  `W48-LIBCURL-CONTRACT-PLUGIN`; amends `W47-EXTERNAL-RESIDUAL-RANKING-R2`;
  follow-ups 534 -> 537.

## Gate table

| Step | Exit | Notes |
|---|---|---|
| submodule / clean-dev / fmt / clippy | 0 | core |
| tests (core workspace) | 0 | 6140 passed / 0 failed / 11 ignored, 354 summaries, 341 units |
| reevaluation-probe / reevaluation-followup | 0 | |
| lean | 0 | 217 Lean audits, no axioms beyond `propext`/`Classical.choice`/`Quot.sound` |
| python (root, scripts/tests) | 0 | 365 tests, OK |
| docs-index | 0 | |
| provider-tests | 0 | 129 passed / 0 failed, 43 `test result:` lines |
| provider-clean-dev / provider-fmt / provider-clippy / provider-contracts-check | 0 | |
| package-build + 5x package-verify | 0 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| provider-python | **1** | 24 tests, 1 known pre-existing failure: `test_dual_extension_same_stem_is_refused_before_manifest_selection` (fix card in flight) — allowed per gate brief |
| clang-version | 0 | present: Ubuntu clang version 21.1.8 (6ubuntu1) |
| release-build | 0 | |
| corpus arbiter | 0 | |
| system (Fluent Bit) slice | 0 | |

## Composition result

`compose3.py` on the corpus (base = `w47-alias-residuals-census` baseline,
A = `w47-callee-closure-r2` after, B = `w47-typed-root-declaration-reader`
after, merged = this gate's corpus run): **COMPOSES**, 0 mismatches, 0
functions/leaves touched by both cards.

`compose3.py` on the system slice (base = A = B = the alias-residuals-census
baseline system run, since neither later card re-ran or changed that slice):
**COMPOSES**, 0 mismatches.

The supplementary `baseline_identity.py` check of the merged corpus against
`W47-CALLEE-CLOSURE-R2`'s own `artifacts/after/corpus` (with a `--root-alias`
for the typed-root reader's worktree) did not complete: it raised
`KeyError('diagnostics')` because that script's `NAMES`-keyed manifest lookup
(`manifest[label]["file"]`) expects the older short-label manifest schema,
while the manifest these cards archived is keyed by full filenames. This is a
script/manifest-schema mismatch, not a corpus defect; it is recorded here
rather than fixed (out of the mechanical gate-helper's scope to edit scripts).

## Combined figures

- Corpus, merged functions: Safe 510 / Unsafe 10 / Unknown 4,314.
- Corpus, merged units: Unknown 167 / Safe 6 / Unsafe 8 / data_only 4.
- Corpus, merged diagnostic_counts: leak 36 / summary_conflict 1 /
  unknown_alias 7,443 / unsupported 36,209.
- System slice, merged unique functions: Safe 33 / Unsafe 2 / Unknown 199
  (not-projected count ignored per brief).
- System slice, merged units: Unknown 7 / Unsafe 1.
- System slice, merged diagnostic_counts: counted_user 4 / unknown_alias 985
  / unsupported 2,908.

Source: this integration's merged gate run
(`private/w47-int-c/gate-4b6b26ad/` in the integrator's worktree, not
archived; the durable copy is `docs/reports/w47-callee-closure-r2/integration/artifacts/{corpus,system}/`).
