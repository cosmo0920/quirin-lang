# Integration confirmation (combined tree)

Integrator re-run on 2026-09-24, on merge commit `6ae583ca`. This card was merged
onto main `a7a936d9`, after W43-LIBC-CONTRACTS, in the integrator's own worktree
with Clang 21.1.8 present ([clang-version.txt](clang-version.txt)). Every exit
was captured to a file rather than piped. Private logs are in the integrator
session's scratch `gate-integrate-inherit/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,176 passed / 0 failed / 11 ignored**, 277 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **168** audits |
| `unittest discover -s scripts/tests` (before regeneration) | 1 | 327 run, 1 failure; see below |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 5 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 294 follow-ups carried; **327** tests |
| `build_docs_index.py --check` | 0 | index matches |

The first Python run failed only
`test_the_derived_facts_are_recomputed_rather_than_carried` (Lean audits 167
vs 168). The merged `validation` section came from main and predates
`AuditContractInherit.lean`. Regenerating the section from this gate fixed
it, and the rerun passed all 327 tests. This is the same deferred-validation
pattern recorded by the Monkey integration.

## Ledger

The ledger was merged per entry, three ways, with base `fa7ab464`. The card's
three follow-ups (`CONTRACT-INHERIT-ADOPTION`,
`CONTRACT-INHERIT-EXTENSION-CLAUSES`, `CONTRACT-INHERIT-NON-V1-CONSUMERS`) were
added on top of main's 291, so the count went from **291 to 294**. The card's report
states 282 → 285, which was true on its branch.

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0) was
compared with `docs/reports/w43-libc-contracts/artifacts/` using
`baseline_identity.py --root-alias`, which also exited 0.

* **diagnostics, chunk-contracts and source are byte-identical.**
* **contracts** is identical after the checkout root is aliased to the
  baseline's worktree.
* **source-effects** is identical modulo the checkout-path identity digests.
  It is also **byte-identical to the W43-LIBC-CONTRACTS integration run in
  this same checkout**, so this card changes nothing there. That settles the
  53 source-effects leaves the card's own report left unattributed against the
  older W43 baseline. They predate this card and do not come from it.

No verdict, contract text or coverage moves. No real provider contract uses
`#include` or `inherit` yet; adoption is `CONTRACT-INHERIT-ADOPTION`.
