# Main integration

The user authorized merging and pushing after card completion. Main before
integration is `a7e5ed4cc`; card tip is `7d4ced73d`. Two pin-comment conflicts
are resolved preserving the existing 66-module view and the card’s
497-declaration count and all presence assertions. No production Rust or Lean
semantics are edited for this merge. `reconciliation.json` records the card’s
status, increment and two follow-up amendments. The isolated native checkpoint
remains historical; no combined native measurement is claimed.

The full standing/provider gate on clean merged source `b7c2b77e1283256b8b300be89184ba43e7035ba9`
passes every command (exit 0): 8,601 root Rust tests
(547 units / 560 summaries, 15 ignored),
369 Lean audits, 374 root Python tests, 133 provider Rust tests
(1 ignored) and 25 provider Python tests. Formatting, Clippy, both
reevaluation examples, inventory and all five package builds/verifies pass.
`gate.json` supplies the actual merged counts; `command-receipt.json` retains
commands, exits, source guards and hashed private logs. The card's isolated
8,593-test gate remains its own historical checkpoint. No combined native
verdict count is inferred from merging.
The initial matrix regeneration without its required repository argument
failed with IndexError; the corrected invocation `python3
docs/reports/remaining-d02/build_matrix.py .` succeeded. It carried the prior
gate provenance rather than inventing a combined measurement.

The earlier launch was interrupted while queued for the shared lock; no gate
command had started and no output directory existed on resume. The new run
uses fresh `integration-gate-1` outputs. The acceptance matrix is regenerated
with `--gate docs/reports/w48-bu-external-callee-contracts/integration/gate.json`;
the committed regenerated matrix receives a separate root Python check.
