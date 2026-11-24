# K7-RESIDUAL-CAUSES: gate and runs

## Card gate: stopped on the user's instruction

USER INSTRUCTION (2026-11-24, verbatim, through the coordinator): "We don't
need to run the each of K-related gates. We must run the merged K-related
tip's gate." The integrator runs one full landing gate on the merged
K1/K3/K5/K7 (+ SP10) tip, so this card's scoped gate was stopped.

| Attempt | Tree | Steps that ran | Outcome |
|---|---|---|---|
| `private/gate-1` (`python3 scripts/gate_scope.py --base 3ca85c7a6 --out private/gate-1`) | `51af41b6e` + untracked report files | submodules 0, fmt 0, **Clippy 101** (`unused import: strip` in `pointer_offset_place.rs`, a test-only import) | stopped by this card (its own PIDs) while the Rust suite queued; the import fixed in `91214d248` |
| `private/gate-2` (same command, `--out private/gate-2`) | `91214d248` | submodules 0, fmt 0, Clippy 0 (`cargo clippy --all-targets -- -D warnings`) | Rust suite still queued for an audit slot (position 1) when the instruction arrived; stopped by this card (its own PIDs 995573, 983975, 983970); Lean, Python and the rest never ran |

Selection (`--plan`): mode full (the diff reaches `qcore-c-gateway`), Lean
all (a proof root changed), Python always, provider gate no.

## Targeted checks run by the card

- `cargo test -p qcore-c-gateway --test k7_residual_causes` (the gate's
  test-build configuration, `scripts/test_link_config.py dynamic`,
  `scripts/test.sh fix1`): exit 0, 3 passed (`private/k7/fix1.log`).
- `cargo test -p qcore-c-gateway --lib -- pointer_offset_place`
  (`scripts/test.sh lib1`): exit 0, 4 passed (`private/k7/lib1.log`).
- `cargo clippy -p qcore-c-gateway --all-targets -- -D warnings` on
  `91214d248`'s tree: exit 0 (`private/k7/clippy-gw.log`).
- `cargo fmt -p qcore-c-gateway -- --check`: exit 0.
- `python3 scripts/gate_lean.py --module QCore.PointerOffsetPlace --audit
  AuditPointerOffsetPlace.lean`: exit 0, 1 module built, 1 audit passed (16
  `#print axioms`, `propext`/`Quot.sound` only) (`private/k7/lean3.log`;
  attempts 1 and 2 failed on proof steps, fixed before the commit).
- `python3 scripts/apply_amendments.py
  docs/reports/k7-residual-causes/integration-amendments.json --check`: exit 0.
- Not run: the Python suite (the stale matrix Lean-audit count, +1 for
  `AuditPointerOffsetPlace.lean`, is the integrator's regeneration).

## Unit-scoped reproduction

The base (`target-census`, census streams on) and tip (`target-tip`, report
only) runs of the 26 non-Monkey units holding a freed row
(`artifacts/selection.json`) waited about four hours for audit slots; their
outcome, if they finished before hand-back, is in
[artifacts/unit-compare.json](artifacts/unit-compare.json).
