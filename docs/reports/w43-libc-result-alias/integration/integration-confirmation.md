# Integration confirmation (combined tree)

Integrator re-run on 2026-09-24, on merge commit `ac241322`. This card was merged
onto main `dfe10c92` (after CONTRACT-INHERIT) in the integrator's own
worktree, with Clang 21.1.8 present ([clang-version.txt](clang-version.txt)).
Every exit was captured to a file, not piped. Private logs are in the integrator
session's scratch `gate-integrate-ra/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,197 passed / 0 failed / 11 ignored**, 278 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **170** audits |
| `unittest discover -s scripts/tests` (before regeneration) | 1 | 327 run, 1 failure; see below |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 5 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 297 follow-ups carried; **327** tests |
| `build_docs_index.py --check` | 0 | index matches |

The first Python run failed only
`test_the_derived_facts_are_recomputed_rather_than_carried`, on Lean audits
168 vs 170. The merged `validation` section came from main and predated this
card's `DerivedResult` and `PointerQualifierReader` audits. After regenerating
the section from this gate, the Python suite passed.

## Ledger

The ledger was merged per entry, three ways, with base `a7a936d9`. On top of
main's 294, three entries were retired (`LIBC-MEM-RESULT-ALIAS`,
`W43-LIBC-RESULT-ALIAS-NAMES`, `W43-LIBC-POINTER-QUALIFIER-READER`) and six
registered, giving **294 → 297**. The card's own report says 291 → 294, which was
true on its branch.

## Corpus arbiter

The integrator made a fresh release run (`w43-return-expression/scripts/run_audit.py`,
exit 0) and compared it with `docs/reports/w43-libc-result-alias/artifacts/`
using `baseline_identity.py --root-alias`, which also exited 0. Diagnostics,
chunk-contracts and source are **byte-identical**. Contracts is identical after
the checkout-root alias. Source-effects is identical modulo the checkout-path
identity digests.

The card's movement therefore holds on the combined tree with CONTRACT-INHERIT:

* functions go from 245/84/4,505 to **257/84/4,493**;
* no unit moves;
* the leak count stays at 272;
* no violation candidates appear in the corpus.

## Integrator note on the brief

The brief said releasing a derived result is never a release of the base. For
an exact alias that is wrong in C: `free(memcpy(b, …))` frees `b`. The card
refused that case to `Unknown` rather than accuse correct code, and registered
`W43-RESULT-ALIAS-EXACT-RELEASE`. The integrator accepts this deviation. An
interior-pointer release is still a violation candidate, as briefed.
