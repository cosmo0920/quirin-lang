# RELATIONAL-REWRITE-LINE-ORDER: rule 11r's rewrite keeps the stream's lines in order

Card RELATIONAL-REWRITE-LINE-ORDER (wave 48, card-high, small). Gateway rule
**11r** amended in place (and its router row); no new rule id, no grammar or
contract-specification change, no Lean module (section 5). Base: local
`main` `1c7333db1`. The census (`244728756`) and the frozen
[prediction](PREDICTION.md) (`a97dad6ee`) were committed before any
implementation code; the fix is `8b5ae962f`.

## 1. Defect

Rule 11r rewrites `NAME = f(…); if (TEST)` into `if (TEST')`, `TEST` with
`NAME` replaced by `(NAME = f(…))`, for a callee with a return-code
disposition. The `if`, its parenthesis and the synthetic parentheses kept the
`if`'s line, ahead of the moved call's tokens on earlier lines.
`typed_place::occurrence` places a statement on its line by binary search
over the parser's stream and declines every statement of a stream whose lines
are not non-decreasing. The rewrite scans the whole unit's stream, so
wherever it fired, no statement of the unit had a typed occurrence, and
nothing said so.

## 2. Census

`crates/qcore-c-gateway/src/legacy_stream_line_order.rs` writes parity rows
(`QCORE_TYPED_PATH_PARITY`, parity runs only): one per stored-return rewrite
that fired, with its sites; one per rewrite stage of `build_function_cfg_with`
that fired (`_Generic` selection, literal premises, rule 11r, the hand-off
rewrite, the out-alias normaliser, and an unsorted input), with whether the
stream was sorted before and after; one per guard refusal.
[`census.py`](scripts/census.py) and [`census.sh`](scripts/census.sh) reduce
them ([BEFORE](artifacts/census-census-corpus.json),
[AFTER](artifacts/census-after-corpus.json), and the `-system` / `-slice`
files beside them).

| route | rule 11r fires | sites | unsorted before → after | functions built on the stream |
| --- | --- | --- | --- | --- |
| corpus | `cmt_decode_opentelemetry.c` (cmetrics) | 3: `get_or_create_external_metadata_kvlist` (`result = cfl_kvlist_insert_kvlist(…);` over three lines, then `if (result != 0)`), `clone_exemplars_to_kvlist` (two `result = cfl_kvlist_insert_*(…); if (result != 0)`) | yes → no | 78 (the unit's 40 rows and header definitions built as callees) |
| system | `src/flb_output.c` | 1: `flb_output_task_singleplex_enqueue` (`ret = flb_output_task_queue_enqueue(…); if (ret == -1)`) | yes → no | 46 |
| slice | nowhere | 0 | — | 0 |

Every other stage keeps the order wherever it fires (corpus: `_Generic` 36
functions, literal premises 4,744, the hand-off rewrite 244 functions at 2
sites in 1 unit; system 1 / 174 / 0; slice 3 / 367 / 0); no input stream is
unsorted. So rule 11r's rewrite was the only stage that broke the order on
the measured routes, and after the fix no stream is unsorted.

## 3. Fix and guard

* **Fix** (`legacy_relational_post::stored_returns_with`). Both
  stored-return rewrites keep the order, as HANDOFF-DIRECT-CALL-READING's
  `keep_line_order` did for its own: the `if`, its parenthesis, the condition
  tokens before `NAME` and the added parenthesis take `NAME`'s line, and the
  parenthesis closing the moved call takes the call's own closing line. The
  flag is gone (always on); a `Rewrite` value names the rewrite for the
  census. The rewritten tokens and their order are unchanged, so the C read
  is the same; only the `if` head's line attributions move.
* **Guard** (`legacy_stream_line_order::guard`, hooked in
  `build_function_cfg_unscoped` after every rewrite, on the stream the parser
  reads). **Variant chosen: both.** A stream whose lines are not
  non-decreasing is a **hard error** in debug and test builds (a panic naming
  the function and the first out-of-order pair, so any rewrite that breaks the
  order fails the tests that exercise it), and in release builds a **counted
  refusal**: the function's first instruction is `Unknown` with the
  description ``token stream out of line order after a rewrite: `A` on line N
  stands before `B` on line M, so no statement has a typed occurrence
  (stream-line-order)`` (the row is never `Safe` and the leaf is counted in
  every report), and a `stream-refusal` parity row. Release builds refuse
  rather than panic so that a disordered stream in some unit fails closed for
  that function instead of aborting a whole audit.

Tests: `legacy_relational_post`'s `a_line_ordered_rewrite_keeps_the_stream_sorted`
(both rewrites, every token's line checked) and
`a_rewritten_unit_keeps_the_other_statements_occurrences` (after the rewrite
of a test on a later line, two identical statements on a third line each have
their typed occurrence, ordinal 1 of 2); `legacy_stream_line_order`'s
`the_first_disorder_is_named`, `a_disordered_stream_is_a_hard_error_in_test_builds`
(`should_panic`, debug builds) and
`a_disordered_stream_is_a_counted_refusal_in_release_builds` (the release
answer taken through a test-only switch: the refusal is the first
instruction and the function is `Unknown`; the same body in order is `Safe`
with no refusal); `tests/relational_postconditions.rs`'s
`a_rewritten_unit_keeps_its_typed_occurrences` (Clang: a body with a
stored-return test on a later line and `s->in.x = 1; s->in.x = 1;` on one
line is `Safe`, rule 10t telling the two stores apart; the rewrite's own
reading over lines is still `Safe`). Every existing 11r, hand-off and CG10
test passes (`relational_postconditions` 14, `handoff_direct_call` 5,
`clang_gap_phase2` 14, `return_code_handoff` 6).

## 4. Measurement

BEFORE `census` (`f450284f…`, built at `244728756`: the base code plus the
census rows, which only reach the parity stream) and AFTER `after`
(`17e3319b…`, built at `8b5ae962f`), each `run.json` naming its binary,
every route with the null property ([summary](artifacts/summary.json),
scripts [`before.sh`](scripts/before.sh), [`build.sh`](scripts/build.sh),
[`measure-binary.sh`](scripts/measure-binary.sh)). The BEFORE corpus report
is main's published `1e76a508…`; the system and slice reports embed the
measuring worktree's path, so their digests are compared only within this
worktree.

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 938 / 9 / 3,887 | **938 / 9 / 3,887** | `1e76a508…` → `ff04247e…` | 707.2 → 747.3 |
| system | 93 / 4 / 351 | **93 / 4 / 351** | `1e62b6d3…` → `d76c5212…` | 195.5 → 246.4 |
| slice | 52 / 1 / 347 / 4 Pending | the same, byte-identical | `49220b8e…` | 152.4 → 195.9 |
| null | 1,113 / 1,435 / 2,355 / 18; 84 / 125 / 245; 71 / 103 / 232 | rows byte-identical on all three | | |

Wall times were measured once each on a shared host and are no performance
claim. Contract-dependency counts and the Safe roll-up are identical on all
three routes. **Guard refusals: 0** on every route.

**Every moving row** ([corpus](artifacts/compare-corpus.json),
[system](artifacts/compare-system.json), [slice](artifacts/compare-slice.json)),
attributed to the rewrite; no status move, so **no Safe change in either
direction** to read and no correction:

| route | row | status | leaf before → after |
| --- | --- | --- | --- |
| corpus | `get_or_create_external_metadata_kvlist` | Unknown | `unresolved resource identity entry_kvlist` 7206 → 7203 |
| corpus | `clone_exemplars_to_kvlist` | Unknown | `unresolved resource identity filtered_attributes` 7422 → 7421; `… array` 7437 → 7436 |
| system | `flb_output_task_singleplex_enqueue` (both occurrences of `src/flb_output.c`) | provisional Unsafe | `unresolved resource identity retry` 17844 → 17843 |

Each leaf belongs to the rewritten `if` statement, whose line was the `if`
token's; it now carries the assignment's line, where the call C evaluates
starts. No leaf came back or went away: neither unit
has a line whose identical statements need a typed occurrence to be told
apart, so restoring the occurrences changed no reading here. The guard now
keeps that from recurring silently anywhere.

**Prediction against outcome.** As predicted: no status move on any route,
moves confined to the two units, exactly the four predicted line moves, null
unchanged, no guard refusal, the rewrite firing at the same 3 + 1 sites with
every stream sorted after it. The predicted possibility of restored or new
leaves from typed occurrences did not occur.

## 5. Lean

None: no decision rule changes. The rewrite produces the same tokens in the
same order; only token line attributions of the `if` head move, and line
attributions decide nothing. The guard's release refusal is an `Unknown`
instruction composed by the existing CFG transfer and verdict join (`Unknown`
absorbs `Safe`, §L21.2, already proved); it fires on no measured row. Lean
audits stay 329.

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py): 787 → **786**
(equal to `dimensions.named_followups`). **Retired:**
`RELATIONAL-REWRITE-LINE-ORDER` (its DISCHARGE holds: the 11r rewrite keeps
the lines non-decreasing, measured on every route, every leaf and status
change attributed above). Nothing registered: the guard covers every rewrite
before the parser, so no other stream rewrite can leave the order broken
unseen.

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/) (filled after the gate run).

## Files likely to conflict

`crates/qcore-c-gateway/src/legacy_c.rs` (the census lines and the guard hook
in `build_function_cfg_unscoped`, the module declaration),
`legacy_relational_post.rs`, `legacy_handoff_direct.rs` (the `Rewrite`
argument), `tests/relational_postconditions.rs`, `docs/agents/gateway.md`
(11r), `docs/agents/README.md` (11r row), `docs/STATUS.md`,
`docs/development/increments/wave-48-part8.md`, the acceptance matrix,
`docs/reports/INDEX.md`.
