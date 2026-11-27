# W45-FB-BACKLOG-CUSTODY prediction (frozen before implementation)

Written 2026-09-26 on base `74579af6`, after the two BEFORE measurements
reproduced and before any analyzer, loader or contract change. It is not
edited after implementation starts; the report compares against it.

## What is measured BEFORE

* **Corpus arbiter** (`run_audit.py`, `--jobs 8`) reproduces
  `docs/reports/w45-parenthesised-callee/integration/artifacts/`:
  `resource-diagnostics.json`, `chunk-contracts` and `monkey-source.json`
  byte-identical, `contracts.json` identical after the root alias,
  `verdict_bearing_identical: true`. Functions 390 Safe / 12 provisional
  Unsafe / 4,432 Unknown; leak 37, unknown_alias 3,567, unsupported 42,699.
* **Shape census** of that report (`scripts/shape_census.py`, reading each
  leaf's original line): of 847 `for-loop condition` leaves, 269 sit on an
  intrusive circular-list header (`cfl_list_foreach` 155, `mk_list_foreach`
  56, `mk_list_foreach_safe` 31, `cfl_list_foreach_safe` 27); of 2,284
  `braced initializer` leaves, 326 sit on a `container_of` element read
  (`cfl_list_entry` 198, `mk_list_entry` 99, `*_entry_first`/`_last` 28,
  `container_of` 1). Every other braced shape (`RD_UT_ASSERT` 731, brace
  initialisers 315, call lines 540, …) is outside this card.
* **System slice at 864c3d97** reproduces the published figures (units
  0/3/12, functions 25/2/206/3). **Re-pinned to 046accf6** (user direction):
  units 0/3/12, distinct functions 25 Safe / 2 Unsafe / 207 Unknown / 3 not
  projected. `sb.c` is byte-identical between the two revisions; the whole
  difference is source change in `flb_engine.c`, `flb_engine_dispatch.c`,
  `flb_input_chunk.c` and `flb_output.c` (one new function,
  `flb_output_task_flush_error`), measured with the same base binary.

## The mechanisms, as planned

1. **Intrusive-list iteration** (gateway, new module). The preprocessed
   headers `for (C = (H)->next; C != (H); C = C->next)` and
   `for (C = (H)->next, N = C->next; C != (H); C = N, N = C->next)` (and the
   `prev` forms) are recognised on their original spelling and lowered to a
   modelled iteration: the header reads through `H`'s root with the typed-read
   obligation, the cursor(s) denote untracked list memory, both loop edges
   stay (the exit edge is kept). The body may not write the cursors. In the
   plain form, handing the current element's link node or the element to a
   call that could unlink or release it is refused; in the `_safe` form it is
   admitted under the stated premise that the body does not unlink the saved
   successor.
2. **Element identity** (`container_of`): the GNU statement expression
   `({ const typeof(((T *)0)->F) *__mptr = (P); (T *)((char *)__mptr -
   offsetof(T, F)); })` is admitted when `P` is an active cursor, a
   `(H)->next`/`(H)->prev` read, or a never-bound name, binding the
   destination to untracked memory.
3. **Checked specifications** (contract vocabulary, per the user's amendment):
   a declaration marked `requires resource_body_checked;` applies only where
   the unit defines its body and that body validates against it; otherwise it
   is withdrawn and the defining function reports why. It is never a trusted
   premise.

## Predictions

* **Corpus leaves.** Of the 269 list-header `for-loop condition` leaves,
  200–269 are withdrawn (the rest declined by the body-writes and
  member-address-publication guards). Of the 326 element-read braced leaves,
  220–326 are withdrawn. The `indirect memory access` leaves on the same
  header lines fall by a similar count. New refusals from the plain-form rule:
  10–60 leaves (plain `foreach` bodies that `mk_list_del`/free the element and
  `break`).
* **Exchanges.** Most withdrawn leaves are *replaced* by a later refusal in
  the same function (a branch condition, an uncontracted call, an alias), so
  `unsupported` falls by several hundred while `unknown_alias` may rise by
  tens (reads that now reach a flow check).
* **Function status.** Unknown → Safe: 0–15 functions (small iteration-only
  bodies). Safe → non-Safe: 0 predicted; any such move is a correction to
  name. Unknown → provisional Unsafe: 0–10, each an exposed leak or
  violation to classify by reading. The leak pin (37) may rise by 0–10 for
  the same reason (constraint 7: admitting an exit is the accusing
  direction).
* **Checked specifications and the corpus.** The new provider module names
  only `sb.c` functions, which are not in the 185-unit corpus, so the
  contracts move no corpus leaf; the corpus movement is the gateway
  mechanism's alone.
* **Slice (046accf6).** The `mk_list_foreach` refusals leave 10 of the 12
  `sb.c` slice functions; `sb.c` loses most of its 22 braced/for leaves.
  Predicted status: no slice function becomes Safe beyond today's two
  (`sb_allocate_chunk`, `sb_destroy_chunk`); the rest stay Unknown on branch
  conditions, uncontracted `cio_*`/`flb_*` calls and aliases.
* **Obligations.** OB-1 stays blocked on list publication (owned by
  W45-FB-RETRY-LIFETIME). OB-2 and OB-5 stay **Unknown**, not refuted: the v1
  vocabulary has no success-dependent reference relation between a wrapper
  and a chunk (OB-2) and no per-arm disposition for `flb_input_chunk_map`'s
  `chunk` (OB-5); both are registered as vocabulary residuals with their
  sizes. OB-3, OB-4, OB-6, OB-7, OB-8, OB-9 and OB-10 stay Unknown with named
  blockers. At most the two leaf helpers' checked specifications
  (`sb_allocate_chunk`, `sb_destroy_chunk`) are decided-holds.
