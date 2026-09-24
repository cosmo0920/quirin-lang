# Integration confirmation (combined tree)

Integrator re-run, 2026-09-24, on merge commit `b5f45079`. This card was merged
onto main `fa7ab464` (after WINDOWS-TEST-PORTABILITY), in the integrator's own
worktree, with Clang 21.1.8 present ([clang-version.txt](clang-version.txt)).
Every exit was captured to a file rather than piped. The private logs are in the
integrator session's scratch directory `gate-integrate-libc/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,139 passed / 0 failed / 11 ignored**, 274 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **167** audits |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 5 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 291 follow-ups carried; **319** tests |
| `build_docs_index.py --check` | 0 | index matches |

The card's own gate measured 5,136 / 0 / 12 over 272 summaries on `9edad50c`.
The difference comes from main's intervening WINDOWS-TEST-PORTABILITY commit,
which changed Rust tests and `#[ignore]` attributes. The difference was not
measured separately.

## Ledger

The follow-up ledger was merged per entry, three ways (base `9edad50c`). The card's
ten libc residuals were added, and `LOADER-ABI-RESTRICT-QUALIFIER` was retired.
Main had already retired three F02 follow-ups, so the combined count is
**282 → 291**. The card's report says 285 → 294, which was true on its own
branch. The `validation` section was regenerated from this gate rather than
merged.

## Corpus arbiter

This was a fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0)
compared with `docs/reports/w43-libc-contracts/artifacts/`
(`baseline_identity.py --root-alias <integrator worktree>=<card worktree>`, exit 0):

* **diagnostics, chunk-contracts and source are byte-identical**, so the verdict-bearing
  diagnostics are reproduced.
* **contracts** is identical once the checkout root is rewritten to the card
  worktree's path. The only differences are the absolute source paths.
* **source-effects** is identical after the checkout-path identity digests are
  normalised. Those digests are the `resource_contracts:<digest>` bound key,
  `coverage[0]`, `obligation_id`, `build_digest`, `task_digest`,
  `configuration_digest` and the `contract_sources` digest.

The card's movement therefore holds on the combined tree: 22 functions go from
Unknown to Safe, relative to the 55 declared libc/POSIX premises, with no unit
moves.
