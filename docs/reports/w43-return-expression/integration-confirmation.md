# Integration confirmation (combined tree)

Integrator re-run, 2026-09-23, on merge commit `e469b8be`: this card merged
onto main `7e8d59ef` (after CT-SHARED-FIXTURE-DISCLOSURE and
W43-CHUNK-SOURCE-PROFILE), from the integrator's own worktree with Clang 21.1.8
present. Private logs: the integrator session's scratch `gate-integrate-re/`.

## Standing gate on the combined tree

Every step's exit was captured to a file, not piped; all exit 0.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5,117 passed / 0 failed / 12 ignored**, 272 summaries |
| fmt, clippy `-D warnings` | clean |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / 0 |
| `check_lean_audits.py` | **166** audits |
| `unittest discover -s scripts/tests` | **317** tests |
| `build_docs_index.py --check` | index matches |

The counts compose exactly: 5,103 Rust / 165 Lean at `7e8d59ef` plus this
card's 14 tests and one audit.

## Corpus arbiter against this card's archived baseline

`scripts/run_audit.py` with a fresh release build (binary
`c42d835d3a84a43b…`), exit 0, 05:00:58Z–05:05:51Z; then
`scripts/baseline_identity.py … --root-alias <integrator worktree>=<card worktree>`,
exit 0.

| Artifact | Result |
|---|---|
| diagnostics (verdict-bearing) | **byte-identical** |
| chunk-contracts | byte-identical |
| source | byte-identical |
| contracts | identical after the checkout-path alias |
| source-effects | differs only in identity digests |

The source-effects difference is 41 leaves by `scripts/json_leaf_diff.py` (the
same count this card measured for its own path-only difference). After
normalising `build_digest`, `configuration_digest`, `task_digest`,
`obligation_id` and digest-named keys, six leaves remain, and each is again a
digest: `evidence/coverage[0]` (`resource_contracts:<digest>`) and
`task/contract_sources/qcore:boolean-field-import:v1` on results 16, 24 and 63.
No verdict, kind or coverage payload differs. These identities hash the
checkout path; the card's archived and confirmation runs, taken in one worktree,
are byte-identical. The earlier W43 merges therefore move no corpus artifact.
