# Integration confirmation (combined tree)

Integrator re-run, 2026-09-23, on merge commit `7642d7de`: this card merged onto
main `8f40df36` (after every earlier W43 merge), in the integrator's own
worktree with Clang 21.1.8 present ([clang-version.txt](clang-version.txt)).
Every exit was captured to a file, not piped; private logs are in the
integrator session's scratch `gate-integrate-provider-shared/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,136 passed / 0 failed / 12 ignored**, 272 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **167** audits |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 5 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 285 follow-ups carried; **317** tests |
| `build_docs_index.py --check` | 0 | index matches |

The provider count of 109 is measured here; the integrator independently
recomputed the base's 123 from the base tree: the 59 evaluator tests in
`providers/common` ran 80 times because `source_effect.rs` was compiled into
four binaries and `runtime.rs` into seven, so 123 = 43 + 80 and
109 = 43 + 59 + 7 new.

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0)
compared with `docs/reports/w43-return-expression/artifacts/`
(`baseline_identity.py --root-alias`, exit 0): diagnostics, chunk-contracts and
source are **byte-identical**; source-effects has the same hash as the preceding
combined-tree run (checkout-path digests only). The contracts artifact differs
only in the **18** selected module paths: rewriting each
`providers/quirin-provider-*/contracts/<rest>` back to
`providers/shared/contracts/<rest>` reproduces the pre-move combined-tree run
(same worktree) byte for byte, and no former path remains. No verdict, contract
text or coverage moves.
