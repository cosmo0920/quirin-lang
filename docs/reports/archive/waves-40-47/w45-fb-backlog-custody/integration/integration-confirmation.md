# Integration confirmation (combined tree)

The integrator merged W45-FB-BACKLOG-CUSTODY on `2736f30c`, onto main
`2f9131a8` (after all wave-45 cards through W45-CONTRACT-REVISION-MODE). The
merge is a fast-forward-able history graft (`2f9131a8` is an ancestor of the
card's tip `1605b0ff`) with no textual conflicts: 109 files changed, 76,834
insertions(+), 36 deletions(-).

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,877 passed / 0 failed / 11 ignored** over 317 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 201 audits (`AuditListIteration`, `AuditCheckedSpecification`, `AuditRefusedCallHandOn` added) |
| Python (root) | 0 | 358 tests |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 x build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 15 package Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w45-fb-retry-lifetime/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **COMPOSES** |
| `build_docs_index.py --check` | 0 | report index matches |

No Safe function moved to non-Safe.

## Composition

**Card delta.** Against `w45-fb-retry-lifetime/integration/artifacts` (the
card's own BEFORE on main `2f9131a8` reproduced it byte-for-byte), the card
removes 1,480 leaves and adds 99. It moves 14 functions and 5 units. No
overlap with an earlier integration and no transitive callee-summary leaves.

**The leak decrease 37 -> 34.** Four `MK-LIST-ADD-BORROWED-PUBLICATION` leaves
are withdrawn to Unknown by the refused-call hand-on fix
(`mk_rconf_read`, `mk_vhost_create`, `mk_vhost_set_property`,
`mk_vhost_set_single`, all provisional Unsafe -> Unknown), and one provisional
candidate is added: `mk_http_error`'s unclosed `open()` descriptor. This is not
a defect claim.

**Unknown -> Safe (9 functions):** `cio_memfs_scan_dump`, `cio_stats_get`,
`cio_stats_print_summary`, `cio_stream_get`, `cio_stream_size_chunks_up`,
`cmt_labels_count`, `is_file_included`, `mk_rconf_meta_get`, `mk_vhost_lookup`.

**Unknown -> provisional Unsafe (1 function):** `mk_http_error` (the added
leak above).

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `2f9131a8` (`w45-fb-retry-lifetime`) | combined |
|---|---|---|
| functions | 389 / 12 / 4,433 | **398 / 9 / 4,427** |
| units | 4 / 10 / 167 / 4 | **5 / 8 / 168 / 4** |
| leak | 37 | **34** |
| unknown_alias | 3,571 | **3,570** |
| unsupported | 42,702 | **41,325** |
| summary_conflict | 1 | 1 |

The combined run is archived as the new baseline under [artifacts/](artifacts/).
