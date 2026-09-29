# W48-QR-SCALAR-TYPES-R3: integration onto main

The card was gated alone at base `d3508e10`. While it ran, main moved to `e182d403`: CG00-CLANG-GAP-BASELINE and CG01-T-OPERATOR-CLOSURE landed from another session. It was merged onto that main as `integrate/w48-r3` `4cfc4d32`.

## Composition

- **Code:** merged without conflict.
- **Records:**
  - The follow-up ledger was merged structurally against `d3508e10`. Main retired 2 and registered 3; the card retired 3 and registered 4. Both sides recorded 621, but the merged ledger is **622** (620 − 5 + 7).
  - STATUS keeps both new rows.
  - The wave-48 increment log lists the card first.

## Combined gate

Script `merged-gate-r3.sh`; logs are in `private/w48-int-r3/gate-4cfc4d32/`, exits are copied in `exits/`, and figures are in `gate.json`.

| Step | Result |
|---|---|
| Submodule, fmt, clippy, both reevaluation probes | 0 |
| `run_test_binaries.py` | exit 101: 6,579 passed, 2 failed, 11 ignored (369 units, 382 summaries) |
| The 2 failures, rerun alone | exit 0 (20/20 and 10/10); see below |
| Lean | 228 audits (CG01's `AuditTypedSourceOperators` and the card's `AuditScalarPositions`) |
| Python | exit 1, then 0 (365 OK) after the matrix regeneration |
| Docs index | 0 |
| Provider gate | 0: 129 passed / 0 failed / 1 ignored; contracts check, package build and 5 verifies 0; provider Python 24 OK |

The two test failures were load-sensitive; the load average was about 30 during the run:
- `cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets` failed its wall-clock assert (`elapsed < 500ms`).
- `package_identity::a_removed_executable_is_refused_at_the_next_spawn` hit `Text file busy` (ETXTBSY) on spawn.

Both passed when rerun alone.

The Python failure was the matrix's stale `lean_audits` figure (227 against 228 on disk). The matrix was regenerated with `build_matrix.py . --gate integration/gate.json`, and the Python suite then passed.

## Results

- **Corpus:** `resource-diagnostics.json` is byte-identical (`ae17152b…`).
- **System and slice:** `resource-diagnostics.json` is identical to round 9's (`private/w47-int-l/gate-6cacfb1b`) after checkout paths are normalized: 0 differing lines each.
