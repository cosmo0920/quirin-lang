# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `fc3509ba`. The card
was merged onto main `ec70709a`, after CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, in the
integrator's own worktree with Clang 21.1.8 present ([clang-version.txt](clang-version.txt)).
Every exit was captured to a file rather than piped. The private logs are in the
integrator session's scratch directory `gate-integrate-view/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,229 passed / 0 failed / 11 ignored**, 283 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **171** audits |
| `unittest discover -s scripts/tests` (before regeneration) | 1 | 330 run, 1 failure; see below |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` (after `sync`) | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 13 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 304 follow-ups carried; **330** tests |
| `build_docs_index.py --check` | 0 | index matches |

The first Python run failed only
`test_the_derived_facts_are_recomputed_rather_than_carried`, because it expected
171 Lean audits and the merged `validation` section recorded 170. That section
predated `AuditContractViewProjection`. After the section was regenerated from
this gate, all 330 tests passed.

## Merge

* **Ledger.** The ledger was merged entry by entry, three ways, with base
  `f8b41209`. It kept main's 298 entries and added the card's six residuals,
  giving **304**. The card's own report says 299 → 305, which was true on its
  branch.
* **Increment record.** Both this card and the naming card opened
  `wave-41-continuation-2.md`. The two headers were combined, and this card's
  paragraph comes first because it is newer.
* **Load path.** `providers/contracts.py sync` was rerun after the merge.

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0) was
compared with `docs/reports/w43-libc-result-alias/artifacts/` using
`baseline_identity.py --root-alias`, which also exited 0. Diagnostics,
chunk-contracts and source are byte-identical.

All five artifacts, contracts and source-effects included, are **byte-identical
to the CONTRACT-INHERIT-QCORE-PROVIDER-NAMING integration run in this same
checkout**. The card changes no analysis input. The removed `source_identity`
copy and the `cio_chunk_close` parameter-type repair are outside the Cycle 19
resource selection.
