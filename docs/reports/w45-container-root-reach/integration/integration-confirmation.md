# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `5fe036f2`. It is
merged onto main `eac26af7` (which includes the user's macOS test fix), after
all wave-45 cards through W45-AUDIT-CHECKPOINTS. Only one record conflicted:
* `docs/development/increments/wave-45.md`, where both paragraphs are kept.

No status move and no correction. The run used the integrator's own worktree,
`CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are in
[gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,937 passed / 0 failed / 11 ignored** over 322 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 205 audits |
| Python before matrix regeneration | 0 | 358 root Python tests |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 x build + verify, provider Python | 0 | 129 / 0 / 1 over 43 summaries; 15 package Python tests |
| release corpus run | 0 | -- |
| `baseline_identity.py` against `w45-fb-backlog-custody/integration/artifacts` | 1 | by design; this card moves the corpus |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 358 root Python tests (unchanged after regeneration) |

## Composition

**Card delta.** Against `w45-fb-backlog-custody/integration/artifacts`, the
card removes 12 leaves and adds 2. It moves no function and no unit.

**Removed (12).** 11 foreign-unlink refusals plus `mk_server_listen_exit`'s
discharged release.

**Added (2).** Unresolved-identity leaves in `mk_vhost_destroy`.

**Overlap: 0 leaves.** The card's own BEFORE reproduced the combined baseline
exactly, so none of its removals were already gone on the combined tree.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | PREV (`w45-fb-backlog-custody`) | combined |
|---|---|---|
| functions | 398 / 9 / 4,427 | 398 / 9 / 4,427 |
| units | 5 / 8 / 168 / 4 | 5 / 8 / 168 / 4 |
| leak | 34 | 34 |
| unknown_alias | 3,570 | **3,571** |
| unsupported | 41,325 | **41,314** |

No function or unit changed status; the card's foreign-unlink and
static-container rules only trade leaves within the diagnostics count.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
