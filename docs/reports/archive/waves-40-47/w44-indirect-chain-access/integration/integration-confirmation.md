# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-26, on merge commit `06dff404`. It is
the first wave-44 integration onto main `c1e69bb5`, and the merge had no
conflicts. The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and
Clang 21.1.8, and its figures are in [gate.json](gate.json).

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,420 passed / 0 failed / 11 ignored**, 290 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index, Lean audits, Python | 0 | clean; 178 audits; 330 tests |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w43-indexed-resource-collections/integration/artifacts` | 1 | by design: this card moves leaves |
| composition check v2 ([composition-check.txt](composition-check.txt)) | 0 | **composes**: 74 leaves removed, 49 added (two exchanges share one line), no function or unit moves |
| `build_matrix.py --gate`, then Python | 0 / 0 | 352 follow-ups |

## Integrator decision recorded

The brief required non-null evidence for every dereferenced link. The card
required it only where the analyzer can bind a pointer: the root, and the first
member place under a tracked object. A link inside an untracked object gets no
nullness check.

The integrator accepts this reading, for two reasons:

* it is exactly W43-INDIRECT-ACCESS-GENERAL's treatment of an unbound base;
* a NULL-link dereference is outside the custody property this projection
  decides.

The strict reading would turn the 24 removals into refusals. The user may
overrule this decision.

The combined run is archived as the new baseline under [artifacts/](artifacts/),
captured in the integrator worktree.
