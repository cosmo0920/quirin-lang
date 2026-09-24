# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `0d131503`. This card
had base `f8b41209` and was measured against `w43-libc-result-alias`. It was merged
onto main `25b43584`, after W43-LIBC-RESTRICT-PROTOTYPES, in the integrator's own
worktree with `CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are in
[gate.json](gate.json). Every exit was captured to a file, not piped, and the
private logs are in the integrator session's scratch `gate-integrate-returned/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,277 passed / 0 failed / 11 ignored**, 285 summaries |
| core fmt, clippy `-D warnings`; both reevaluation examples | 0 | clean |
| `check_lean_audits.py` | 0 | **173** audits |
| `unittest discover -s scripts/tests` (before regeneration) | 1 | only the derived-facts check: 172 vs 173 Lean audits (`AuditDerivedResultInference`) |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w43-libc-restrict-prototypes/integration/artifacts` | 1 | by design: this card moves the corpus |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate integration/gate.json`, then Python | 0 / 0 | 313 follow-ups; **330** tests |

## Merge

The merge had two real code conflicts with W43-INDIRECT-ACCESS-GENERAL, both in
the C gateway.

* **`legacy_c.rs`.** Each card added one field to `Parser`: `census_bindings` from
  the indirect card and `admit_derived_returns` from this card. The resolution
  keeps both fields and both initializers.
* **`indirect_census.rs`.** Each card appended its own census writer at the end
  of the file, and git interleaved them through their identical final lines. The
  resolution restores both writers as complete functions.

`qcore-c-gateway` compiles with and without the `indirect-census` feature.

The ledger was merged three ways: entry by entry for follow-ups, field by field
elsewhere. Main's corpus row was then re-derived from this run. The card retires
two follow-ups and registers one, so the count goes **314 → 313**. The card's own
report says 299 → 298, which was true on its branch.

## Composition

`(integration run − w43-libc-restrict-prototypes/integration/artifacts)` equals
`(card AFTER − w43-libc-result-alias/artifacts)` exactly:

* 2 return-expression leaves are removed;
* `XXH_memcpy` and `mk_string_casestr` move Unknown → Safe;
* no unit moves;
* no violation candidates.

The run is archived as the new combined baseline in [artifacts/](artifacts/),
captured in the integrator worktree
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`.
It has these totals:

| Measure | Value |
|---|---|
| Functions (Safe / provisional Unsafe / Unknown) | **274 / 84 / 4,476** |
| Units | 3 / 42 / 136 / 4 |
| leak | 271 |
| unknown_alias | 4,725 |
| unsupported | 47,798 |
| return-expression refusals | 496 leaves / 419 functions / 146 sole |
