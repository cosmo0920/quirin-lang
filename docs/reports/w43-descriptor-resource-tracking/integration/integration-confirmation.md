# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `e7094948` plus the
pin commit `87fa29e7`. The card's base was `f8b41209` with main `0a07cb57` merged in,
and it was measured against `w43-indirect-access-general`. It was merged onto main
`58359503`, after W43-LIBC-PRINTF-FAMILY, in the integrator's own worktree with
`CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are in [gate.json](gate.json).
Private logs are in the integrator session's scratch directories
`gate-integrate-descriptor/` and `gate-integrate-descriptor-2/`.

## Gate

The first run, `gate-integrate-descriptor/` on `e7094948`, had **1 failure**:
`every_qcore_contract_source_parses_round_trips_and_compiles` expected 109 corpus
modules and found 110. W43-LIBC-RESTRICT-PROTOTYPES added `libc/regex.qcore` and
this card added `libc/dirent.qcore`, and the merge kept one of the two 109 pins.
The fix is in `87fa29e7`, and the core gate was rerun in full as
`gate-integrate-descriptor-2/`:

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,325 passed / 0 failed / 11 ignored**, 288 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **175** audits |
| Python before regeneration | 1 | only the derived-facts check (Lean 174 vs 175) |
| `build_matrix.py --gate`, then Python | 0 / 0 | 329 follow-ups; **330** tests |

The provider and package steps and the corpus run come from the first run. No
file they read changed, and every step exits 0:

* provider tests 124 / 0 / 0 over 39 summaries;
* the isolated mutation control;
* `contracts.py check`;
* 5 × package build + verify;
* 13 package Python tests;
* the release corpus run.

## Merge

**Gateway (`legacy_c.rs`, 7 hunks).** This card, W43-RESULT-ALIAS-RETURNED and
W43-INDEX-TRAVERSAL each added fields in the same places. The merge keeps all of
them:

* `EntryCustody` has both `derived_bases` and `site_kinds`.
* `TransferContext` has `derived_bases`, `traversal` and `site_kinds`.
* `observe_exit` takes both `scalars` and `kinds`.

The merge adds two integration rules, both fail-closed:

1. `admits_descriptor_read` refuses a statement that reads a tracked descriptor
   when it is classified as traversal's `PointerStep`. It is treated like the
   other custody-neutral scalar routes.
2. A published store whose source is an `Alias` or a `PointerStep` counts as
   custody when that source carries an integer identity, whatever its declared
   type.

**Test helpers.** The traversal and derived-result test helpers map the two new
diagnostic kinds the same way this card mapped the existing helpers:
`KindMismatchedRelease` → `invalid_release`, `UseAfterClose` → `use_after_release`.

**libc modules.** `stdio.qcore` and `socket.qcore` keep both sides' appended
declarations. On the first attempt, git treated the shared final clause of
`sprintf` as common, which left `sprintf` without a result mode. That was
restored, and main's existing declarations were checked to be unchanged.

**Pins.** 110 modules, 47 selected, 152 v1 declarations, 47 source assumptions.

**Ledger.** Merged by entry and by field. One entry was retired and three were
registered, taking the count from **326 → 329**. `CT-CTWIN-O` and `EXTERN-CALL-F`
carry the card's amendments. `corpus.contract_modules` was corrected to 110,
because both sides had independently written 109.

## Composition

The check was run with composition check v2 plus an explicit **explained-leaf**
list ([explained-leaves.json](explained-leaves.json)), and the combined run equals
the expectation exactly.

**Card delta:** 60 leaves removed and 43 added. `ctr_random_get` and its unit move
Unknown → provisional Unsafe.

**Explained (4 leaves).** These are the four `mk_err(..., path)` tail refusals
(`arguments beyond the declared parameters of mk_print`) in `mk_utils_register_pid`.

* `seed_call_arguments` in `borrowed_actual.rs` marks as risky every name passed
  at a call position that is not a declared `Borrowed` position.
* `path` is passed to `stat(path, …)` and to `open(path, …)`.
* `stat` is declared borrowed by W43-LIBC-RESTRICT-PROTOTYPES, which is in the
  previous baseline but not on this card's branch.
* `open` is declared borrowed by this card, which is not in the previous baseline.
* Only the combined tree declares both, so only there is `path` an untracked
  base, and its tail uses are data.
* The function stays Unknown.

**Combined result.** `ctr_random_get` moves Unknown → provisional Unsafe, together
with its unit. There are no Safe moves and no kind-mismatch or use-after-close
leaves.

The provisional Unsafe comes from the `fd > 0` guard, which lets descriptor 0 leak
on the fallback path. That is reachable only if `open` returns 0, which means
standard input was closed. It was not observed natively and is not an upstream
defect claim.

The combined run is archived as the new baseline in [artifacts/](artifacts/). It
was captured in the integrator worktree
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`.

| Measure | Value |
|---|---|
| Functions | **294 Safe / 85 provisional Unsafe / 4,455 Unknown** |
| Units | **3 Safe / 43 provisional Unsafe / 135 Unknown / 4 data-only** |
| `leak` | 271 |
| `unknown_alias` | 4,358 |
| `unsupported` | 45,848 |
