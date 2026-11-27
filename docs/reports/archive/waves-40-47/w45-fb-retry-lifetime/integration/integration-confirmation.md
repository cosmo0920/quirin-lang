# Integration confirmation (combined tree)

The integrator ran this card's gate on merge commit `4dbab8e7` (two parents,
`74579af6` and `914233c6`), merged onto main `74579af6`, after all wave-45
cards through W45-FB-RETRY-FAULT-INJECTION. The merge is a clean fast-forward
of the card's own tip onto main with no conflicts: 130 files changed, 25,696
insertions, 148 deletions.

This is the first semantic Fluent Bit system card (P1.3, singleplex slice at
upstream master `046accf6`). It adds container-publication (spec C04.8, rule
10d: `resource_published`/`resource_unpublished` on monkey's
`mk_list_add`/`mk_list_del`) and counted-users (C04.7, rule 10e:
`resource_user_*` clauses) as checked provider contracts in
`quirin-provider-fluent-bit` (`task_users.qcore`, `output_singleplex.qcore`).
quirin-audit decides OR-1 and OR-7 as refuted: **provisional Unsafe
candidates** in the system slice (the singleplex path needs a synchronous
proxy plugin), **not upstream defect claims**. OR-6 stays Unknown.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,815 passed / 0 failed / 11 ignored** over 310 summaries |
| fmt, clippy `-D warnings` (core and provider) | 0 | clean |
| reevaluation probe and follow-up examples | 0 | clean |
| docs index | 0 | matches |
| `check_lean_audits.py` | 0 | 197 audits |
| root Python (`scripts/tests`), after matrix regeneration | 0 | 358 tests OK |
| Provider suite: tests, fmt, clippy | 0 | **124 passed / 0 failed / 0 ignored** over 39 summaries |
| `providers/contracts.py check` | 0 | clean |
| Isolated package mutation control, first attempt | **101** | 1 failed: `real_packages_preserve_unknown_and_reject_mutations` — `BrokenPipe` writing to a spawned provider's stdin, a host-load failure under concurrent cards |
| Isolated package mutation control, reruns ×3 | 0 / 0 / 0 | `real_packages_preserve_unknown_and_reject_mutations` passed 3/3 in isolation |
| 5 × package build + verify (`chunkio`, `cmetrics`, `ctraces`, `libevent`, `librdkafka`) | 0 | clean |
| Provider Python, first attempt | 1 | 1 failed: `test_the_real_workspace_manifest_is_current` — `AssertionError: 49 != 47`, a stale pin (the suite still pinned 47 resource-view modules while this card deliberately grew the Cycle 19 manifest to 49) |
| Provider Python, after pin fix (commit `b981c5c4`, test-only) | 0 | 15 tests OK |
| release corpus run | 0 | — |
| arbiter (`31-arbiter`) | 0 | archived as `arbiter-run/` |
| `baseline_identity.py` against `w45-parenthesised-callee/integration/artifacts` | 1 | by design; this card moves the corpus |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **COMPOSES** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 428 follow-ups; 358 tests |

Both failing first attempts were test-only issues, diagnosed and fixed by the
integrator directly on `integrate/fb-retry-lifetime`: the resource-view
manifest pin was updated to 49 (commit `b981c5c4`), and the package-mutation
control was confirmed load-sensitive by 3 clean isolated reruns. Neither fix
touches the analysis, so the arbiter run and its composition below stand
unaffected.

## Composition

**Card delta.** Against `w45-parenthesised-callee/integration/artifacts`, the
card removes 11 leaves and adds 18. It moves 1 function and no units.

**Overlap.** None: 0 of the card's removals were already gone on main.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main (`w45-parenthesised-callee` baseline) | combined |
|---|---|---|
| functions | 390 / 12 / 4,432 | **389 / 12 / 4,433** |
| units | 4 / 10 / 167 / 4 | 4 / 10 / 167 / 4 |
| leak | 37 | 37 |
| unknown_alias | 3,567 | **3,571** |
| unsupported | 42,699 | **42,702** |
| summary_conflict | 1 | 1 |

* **Safe → Unknown (correction):** `flb_input_chunk_create`. Its earlier Safe
  verdict rested on reading `mk_list_add` as a borrow, while the body both
  links `ic` into its input's list and returns it; the publication is now
  refused under the container-publication rule. This is a correction of a
  prior verdict, not a regression introduced by this card.

The integration run's leaf/function/unit delta equals the card's own delta
exactly: removed 11, added 18, `flb_input_chunk_create` Safe → Unknown, no
overlap and no transitive callers affected.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
