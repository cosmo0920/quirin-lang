# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25 on merge commit `10b8c607`. This card
(base `d3cd8d81`) was merged onto main `65b5df37`, after CONTRACT-INHERIT-ADOPTION,
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING and CONTRACT-VIEW-PROJECTION-CHECK. The run used
the integrator's own worktree, `CARGO_INCREMENTAL=0`, a cold build, and Clang 21.1.8
([clang-version.txt](clang-version.txt)). Every exit was captured to a file, not piped.
Private logs are in the integrator session's scratch `gate-integrate-indirect/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,254 passed / 0 failed / 11 ignored**, 284 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **172** audits |
| `unittest discover -s scripts/tests` (before regeneration) | 1 | 330 run, 1 failure; see below |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 13 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 313 follow-ups carried; **330** tests |
| `build_docs_index.py --check` | 0 | index matches |

The first Python run failed only
`test_the_derived_facts_are_recomputed_rather_than_carried`, on Lean audits
171 vs 172. The merged `validation` section predated `AuditIndirectScalarAccess`.
After the section was regenerated from this gate, the suite passed.

## Merge

* **Ledger.** Merged three-way with base `d3cd8d81`: entry by entry for
  follow-ups, field by field for the other sections.
  * The `corpus` section takes main's `contract_modules` and the card's
    `cycle19_raw_units`.
  * Twelve entries were added and three retired: **304 → 313**. The card's own
    report says 297 → 306, which was true on its branch.
* **Increment records.** Main had `wave-41-continuation-2.md` (the naming and
  view cards) and this card opened `wave-43.md`. The records are consolidated:
  `wave-43.md` is the current record and holds all three paragraphs, newest
  first, and `wave-41-continuation-2.md` is removed. The CONTRACT-VIEW-PROJECTION-CHECK
  confirmation's remark about combining headers there is superseded by this.

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0) was compared
with the card's own new baseline, `docs/reports/w43-indirect-access-general/artifacts/`,
using `baseline_identity.py --root-alias <integrator>=<card worktree>` (exit 0):

* **diagnostics, chunk-contracts and source are byte-identical**. On the combined tree
  the card's movement holds exactly:
  * functions: 257/84/4,493 → **269/84/4,481**;
  * units: 2 → **3 Safe** (`cmt_time.c`);
  * leak: 272 → **271**;
  * no violation candidates.
* **source-effects** is identical to that baseline modulo checkout-path identity digests.
* **contracts** is byte-identical to the CONTRACT-VIEW-PROJECTION-CHECK integration run in
  this same checkout. The card changes no contract input, and the three contract cards'
  verdict-neutral changes compose with it.

This is now the corpus baseline for the next integrations (W43-LIBC-RESTRICT-PROTOTYPES,
W43-RESULT-ALIAS-RETURNED, W43-LIBC-PRINTF-FAMILY), whose own cards measured against
`w43-libc-result-alias`.
