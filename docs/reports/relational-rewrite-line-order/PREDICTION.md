# RELATIONAL-REWRITE-LINE-ORDER: prediction (frozen before implementation code)

Card RELATIONAL-REWRITE-LINE-ORDER (wave 48, card-high, small). Base: local
`main` `1c7333db1` (corpus 938 / 9 / 3,887 `1e76a508…`, system 93 / 4 / 351,
slice 52 / 1 / 347 / 4 Pending, ledger 787, Lean 329). Rule 11r amended in
place; no new rule id expected. The census code (commit `244728756`) writes
parity rows only and was measured first.

## 1. Census (binary `census`, `f450284f…`, built at `244728756`)

Every route with the null property; each `run.json`/digest file names the
binary. Custody totals and the corpus digest equal main's published BEFORE
(`1e76a508…`); the system and slice reports embed the measuring worktree's
path, so their digests (`1e62b6d3…`, `49220b8e…`) are this worktree's own
BEFORE, compared only with this worktree's AFTER. Null tallies equal the
published ones on all three routes (1,113 / 1,435 / 2,355 / 18; 84 / 125 /
245; 71 / 103 / 232).

| route | rule 11r rewrite fires | sites | stream left unsorted | functions built on it | report rows |
| --- | --- | --- | --- | --- | --- |
| corpus | 1 unit, `cmt_decode_opentelemetry.c` (cmetrics) | 3 (`get_or_create_external_metadata_kvlist`: `result = cfl_kvlist_insert_kvlist(…);` over three lines, `if (result != 0)`; `clone_exemplars_to_kvlist`: two `result = cfl_kvlist_insert_*(…); if (result != 0)`) | yes, all 3 | 78 (the 40 rows of the unit plus header definitions built as callees) | 40 (39 Unknown, 1 Safe `map_type_to_key`) |
| system | 1 unit, `src/flb_output.c` | 1 (`flb_output_task_singleplex_enqueue`: `ret = flb_output_task_queue_enqueue(…); if (ret == -1)`) | yes | 46 | 39 per occurrence (32 Unknown, 5 Safe, 2 Unsafe) |
| slice | never | 0 | — | 0 | — |

Every other rewrite stage of `build_function_cfg_with` keeps the stream
sorted wherever it fires (corpus: `_Generic` 36 functions, literal premises
4,744, the hand-off rewrite 244 with 2 sites in 1 unit; system 1 / 174 / 0;
slice 3 / 367 / 0), the out-alias normaliser never left one unsorted, and no
input stream is unsorted. So rule 11r's rewrite is the only stage that
breaks the order on these routes, and once it keeps the order the guard
below fires nowhere.

## 2. Fix and guard

* **Fix.** Rule 11r's rewrite keeps the stream's lines non-decreasing the
  way the hand-off rewrite does (`keep_line_order`): the `if`, its opening
  parenthesis and the condition tokens before the stored name take the
  line of `NAME` (the assignment's first token), the closing parenthesis
  after the moved call the line of the call's own closing parenthesis. The
  rewritten text is unchanged (the same C, the same tokens in the same
  order); only line attributions of the `if` head move.
* **Guard (variant: both).** After every rewrite, the stream the parser
  reads is checked: a stream that is not non-decreasing is a hard error
  (panic naming the function and the first out-of-order pair) in debug and
  test builds, and in release builds a counted refusal: the function's
  analysis begins with an `Unknown` instruction naming the disorder
  (`stream-line-order`), so the row is `Unknown` with that leaf, and a
  parity row counts it.

## 3. Predicted measurement

* **Status: no move on any route** (corpus 938 / 9 / 3,887, system 93 / 4 /
  351, slice 52 / 1 / 347 / 4 Pending). The typed occurrence is consulted
  only where two or more Clang statement nodes on one line carry the run's
  tokens (rule 10t's identical statements) and by rules 14a/14b's residual
  reading; I know of no row in either unit whose only blocker is such a
  statement. `map_type_to_key` (a `switch` returning string literals) stays
  `Safe`; the system unit's 5 `Safe` and 2 `Unsafe` rows keep their status.
  Any Safe → non-Safe change is a correction and is named, never adjusted
  away; any Unknown → Safe is read by hand.
* **Leaves move only in the two units.** Expected:
  1. **Line attribution at the rewritten sites.** A leaf the `if` statement
     carried on the `if`'s line moves to the assignment's line (the line
     where the call C evaluates starts). Candidates: corpus
     `get_or_create_external_metadata_kvlist` 7206 → 7203 (`unresolved
     resource identity entry_kvlist`), `clone_exemplars_to_kvlist` 7422 →
     7421 (`filtered_attributes`) and 7437 → 7436 (`array`); system
     `flb_output_task_singleplex_enqueue` 17844 → 17843 (`unresolved
     resource identity retry`).
  2. **Restored typed occurrences.** Leaves that were silently absent may
     come back and some refusals may become decisions, wherever a line of
     the two units (or of the header definitions built with them) holds
     identical statements; leaf counts in these rows may rise or fall.
* **Outside the two units:** no row moves, unless through a summary of a
  function of these units; any other moving row is unexplained and is
  reported as such.
* **Null property:** rows may move only in the two units, for the same
  reasons; tallies unchanged unless a status moves.
* **Guard refusals:** 0 on every route (the census finds no other stage
  that breaks the order).
* **Census after the fix:** the rule 11r rewrite fires at the same 3 + 1
  sites, every stream sorted after it.

## 4. Lean

None expected: no decision rule changes. The rewrite still produces the
same C; only token line attributions move. The guard adds a refusal that
never decides (an `Unknown` leaf composed by the existing join, where
`Unknown` absorbs `Safe`).
