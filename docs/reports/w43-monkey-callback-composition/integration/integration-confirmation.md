# Integration confirmation (combined tree)

Integrator re-run, 2026-09-23, on merge commit `ea800844`: this card merged onto
main `df60bc78` (after CT-SHARED-FIXTURE-DISCLOSURE, W43-CHUNK-SOURCE-PROFILE and
W43-RETURN-EXPRESSION), in the integrator's own worktree with Clang 21.1.8
present ([clang-version.txt](clang-version.txt)). Every step's exit was captured
to a file, not piped. Private logs: the integrator session's scratch
`gate-integrate-monkey/`.

| Step | Exit | Result |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | 0 | **5,136 passed / 0 failed / 12 ignored**, 272 summaries |
| fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **167** audits |
| `unittest discover -s scripts/tests` (first run) | **1** | 316/317: `test_the_derived_facts_are_recomputed_rather_than_carried` — committed `validation.lean_audits` 166 vs recomputed 167 |
| `build_matrix.py . --gate integration/gate.json` | 0 | derived sections regenerated; 284 follow-ups carried |
| `unittest discover -s scripts/tests` (after regeneration) | 0 | **317** tests |
| `build_docs_index.py --check` | 0 | index matches |

The first Python failure is real and expected from the step order: both this
card and W43-RETURN-EXPRESSION rewrote the matrix's derived `validation`
section, so the integrator deferred it during conflict resolution and
regenerated it from this gate ([gate.json](gate.json)), as the validation policy
requires Python to run after any matrix regeneration. This also replaces main's
previous `validation` record, which described W43-RETURN-EXPRESSION's isolated
gate (5,099) rather than the combined tree.

Counts compose exactly: 5,117 Rust / 166 Lean at `df60bc78` plus this card's
19 tests and one audit.

## Corpus arbiter

This card changes `qcore-verification`, which the corpus audit links, so a fresh
release run was taken (`docs/reports/w43-return-expression/scripts/run_audit.py`,
exit 0) and compared with the current baseline
`docs/reports/w43-return-expression/artifacts/` (`baseline_identity.py
--root-alias`, exit 0). The verdict-bearing diagnostics are **byte-identical**;
chunk-contracts and source are byte-identical; contracts is identical after the
checkout-path alias; source-effects differs only in checkout-path identity
digests, and all five run hashes equal the preceding combined-tree run's
(W43-RETURN-EXPRESSION `integration-confirmation.md`). This card therefore
moves no corpus artifact.
