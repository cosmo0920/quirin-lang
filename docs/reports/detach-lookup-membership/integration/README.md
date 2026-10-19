# Integration on main, 2026-10-19

The lookup card at `672ab45d1ff4930d87a47cb753844284e5dfa37c`
was merged after main had advanced to `60413ee78cb6e77bf2512391c57616a7dc30acc2`.
That intervening change added the contract-hypothesis-graph candidate and
documentation only. The integrated tree has no difference from the gated lookup
branch under `crates/`, `proof/`, `contracts/`, `providers/`, `scripts/`, the
changed gateway rule, or the changed language/grammar specifications. The
source and proof measurements in the card's `gate.json` therefore still apply;
no differing integration gate or new corpus measurement is claimed.

The matrix conflict was resolved by retaining the remote candidate and all
four lookup-card residual revisions: 805 unique named follow-ups. The part-10
increment retains the lookup, remote-merge, hypothesis-candidate and alias
paragraphs in newest-first order. The report index was regenerated to include
the lookup report. These actions discharge the card's three *For the integrator*
items; the four narrowed residuals remain open.

The first integrated report-index check exited 1 because the generated index
was stale. After `python3 scripts/build_docs_index.py` exited 0, the final
integrated commands all exited 0: matrix regeneration from the card gate,
374 Python tests, report-index check and the card residual inventory. Exact
commands, exits and private logs are in
`private/detach-lookup-integration-20261019/commands.json`.
