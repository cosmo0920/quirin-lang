# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `5aea8197`. It is
merged onto main `b8fc7eae`, after W44-CORPUS-CALLEE-SUMMARIES,
W44-BRANCH-CONDITION-CALLS and W44-LEAK-PIN-AUDIT. Only records conflicted:
* the ledger was merged per entry (377 + 3 = 380, then 379 after the retirement below);
* the increment paragraph was prepended;
* the report index was regenerated;
* the `STATUS.md` corpus rows were re-derived from this arbiter.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang 21.1.8.
The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,503 passed / 0 failed / 11 ignored** |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 183 audits (`AuditAssignmentDestinationSuffix` added) |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| provider `cargo test` | 0 | **124 / 0 / 0** (rerun alone; see below) |
| provider fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-leak-pin-audit/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt), [explained-leaves.json](explained-leaves.json)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 379 follow-ups; 330 tests |

**Provider rerun, disclosed.** For this gate only, the integrator's runner ran
the corpus arbiter at the same time as the provider suite. Under that load one
configured-process test failed: `configured_audit::pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter`
reported 0 checked contracts / 4 Unknown instead of 2 checked. Its verifier
plugin runs under a 5 s process quota, and a quota overrun fails closed to
Unknown.

The test passed on its own. The whole provider suite, rerun with nothing else
running, passed 124 / 0 / 0, and that rerun is the recorded step. The runner no
longer overlaps these steps.

## Composition

**Card delta.** Against `w44-indirect-chain-access/integration/artifacts`, the
card removes 1,380 leaves and adds 191. It moves 3 functions and no units.

**Combined tree.** The combined tree adds 2 more leaves and moves one unit. Both
come from interactions that exist only on the combined tree:

1. **`mk_iov_init`'s summary is withdrawn (2 leaves).**
   * `mk_iov_init` is a `static inline` in Monkey's `mk_iov.h`. Its loop stores
     `iov->io[i].iov_base = NULL; iov->io[i].iov_len = 0;`.
   * Before this card, each store was misread as a binding of the undeclared
     names `iov_base` and `iov_len`. The body therefore validated, and
     W44-CORPUS-CALLEE-SUMMARIES' reached-static-helper route published a summary.
   * The repair refuses those stores, so the summary goes away. Two call sites
     report exactly what they report on the card's own base:
     * `mk_iov_create` (`mk_iov.c:63`): the call is uncontracted;
     * `mk_header_response_reset` (`mk_header.c:450`): the member-address
       argument is refused.
   * No verdict rested on that summary.
2. **`cmt_decode_msgpack.c`: provisional Unsafe → Unknown (unit).**
   * The leak audit had already withdrawn the leaks of this unit's other
     provisional-Unsafe functions.
   * This card withdraws `unpack_basic_type`'s two leaks. Both accused the
     misnamed binding `opts` in `(*map)->opts = calloc(…)`.
   * No Unsafe function remains, so the unit fails closed to Unknown.
   * Composition checker v3 gains an explained `unit_move`. It refuses any
     explained move to Safe.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `b8fc7eae` | combined |
|---|---|---|
| functions | 299 / 10 / 4,525 | **297 / 9 / 4,528** |
| units | 3 / 9 / 169 / 4 | **3 / 8 / 170 / 4** |
| leak | 21 | **19** |
| unknown_alias | 4,473 | 3,357 |
| unsupported | 45,412 | 45,343 |
| double / invalid release, use after release | 0 / 0 / 0 | 0 / 0 / 0 |

## Previously wrong verdicts

These are the soundness findings. Each verdict rested on the suffix misreading:

* **`LZ4_write16`, `LZ4_write32` (`lz4.c`): Safe → Unknown.**
  `((LZ4_unalign *) memPtr)->u16 = value` was lowered as a binding of the
  undeclared name `u16`, not as a store through `memPtr`.
* **`unpack_basic_type` (`cmt_decode_msgpack.c`): provisional Unsafe → Unknown.**
  Its two leaks accused a misnamed binding.
* **Descriptor guard.** `conns[fd].flags = 0` certified Safe at the card's base.
  The repair closes this too, and a unit test and an integration test pin it.

## Integrator decisions recorded

* **Retired `DESTINATION-SUFFIX-PLACE-SPELLING`**
  ([retired-followups.json](retired-followups.json)). Its discharge condition
  asked for a refused destination, measured by an arbiter; this card delivers
  that, and both `unpack_basic_type` leaves are withdrawn.
  `W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE` itself was never registered on main.
* **Adopted the card's gateway amendment as constraint 5i** in
  `docs/agents/gateway.md`, with its router row.
* **W44-INDIRECT-CALL-ARGUMENT-ACCESS must re-pin before it merges.** Its test
  `the_pre_existing_suffix_destination_hole_is_pinned` expects Safe. It must be
  re-pinned to Unknown before that card merges.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
