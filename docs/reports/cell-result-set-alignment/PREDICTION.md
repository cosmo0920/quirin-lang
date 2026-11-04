# CELL-RESULT-SET-ALIGNMENT prediction

Committed before any implementation, on base `32169290d` (origin/main), after
the inventory census (`census/`, commit `725647541`). The rows are relisted
at `baseline:32169290d` (`private/baselines/32169290de211435f4741d10dc7e35ed625e8634/`
in the main checkout).

> **USER REQUEST 2026-11-04** (verbatim): "We need to write down a prompt to
> align the result set for our contracts."
>
> **USER DIRECTION 2026-11-04** (verbatim, on the user's patch): "we have an
> updated patch the previously specified place. We had decided to use the
> format and to check the boundary conditions. So, we need to follow the
> format of the patch to change the our contracts which are related to this
> patch."

## What the card will change

1. **One reading of every end guard (rule 38f, generalised).** A sentinel
   guard's edge decides from two value sets of the cell: the live set (the
   creator's) and the absent set (what the holder's stores write).
   - The dead edge decides "absent" only when its values meet the live set
     nowhere.
   - The live edge decides "live" only when its values meet the absent set
     nowhere.
   - Without a declared live set, the live set is rule 38's premise
     ("a live descriptor is never -1"), every `int` but -1. Without a declared
     absent set, the absent set is the guard's own dead set.
   - Under those defaults the reading is rule 38f's, case for case:
     - `!= -1`: both edges decide;
     - `> 0`: the live edge decides, and the dead edge holds 0, a possible
       live value, so it does not decide.
2. **`>= 0` / `< 0` end guards** (step 9: the user's patch format). G05i
   admits `>= 0` as a per-end sentinel, and rule 38c's test reading admits it
   with its complement `< 0`.
   - Without a declared live set, its dead edge `[MIN, -1]` meets the
     premise's live set (it holds -2 and below), so it does not decide.
   - With a declared live set `[0, MAX]` it decides.
   - Lean parity in `QCore.CellProtocolPerEnd` / `QCore.ResultSet`.
3. **The patch-format declarations are planned, not in force.** They are a
   module under this report that no route loads, read only by fixtures and
   by a store census over a patched scratch tree. The `> 0` declarations in
   `teardown_destroy.qv` stay as they are.
4. **Creator contracts.** Each one is written only where its set can be
   grounded (MISSING ones are listed in the inventory). The form for the live
   set of slot-created descriptors and the spelling of the absent set are
   pending the user (Q1, Q2 in the report). Whichever is chosen, no pinned
   `> 0` dead edge can decide: 0 is a live value.

## Rows the change can reach

The rows that read a declared protocol are the rows depending on the host
`flb_config_set_program_name` at `baseline:32169290d`, plus the `flb_config`
and `flb_sched` rows whose protocols stay withheld:

| Route | Row (units) | Baseline | Predicted |
|---|---|---|---|
| system | `flb_output_instance_destroy` (×2) | Unknown, 27 | **Unchanged.** `> 0` reads as before: the live edge's close is decided, and the dead edge leaves the end undecided (`descriptor-cell-zero-undecided` at the release). |
| slice | `flb_input_instance_destroy` (×2) | Unknown, 62 | **Unchanged**, for the same reason. |
| slice | `input_thread_instance_destroy` (×2) | Unknown, 8 | **Unchanged.** The flag group is not a sentinel, and the four `> 0` ends read as before. |
| slice | `flb_input_new`, `input_instance_channel_events_init`, `input_thread_instance_create` (×2 each) | Unknown, 178 / 12 / 30 | **Unchanged.** The `-1` stores are the `> 0` sentinel's dead value, and no creation establishes `> 0`. |
| system | `flb_engine_started` (×2) | Safe, 0 | **Safe, unchanged.** `flb_config` stays withheld. |
| system | `flb_engine_failed`, `flb_engine_start`, `flb_sched_destroy` (×2 each) | Unknown | **Unchanged.** No protocol of theirs is declared. |
| slice | `flb_create`, `flb_input_collector_destroy` (×2 each) | Unknown | **Unchanged.** Census-only ends, with no declaration. |

- **corpus.** It analyses none of these records.
- **No `Safe` move.** No pinned declaration gains a set that decides a `> 0`
  dead edge.
- **No `Unsafe` move.** Rules 38c and 38f never accuse.
- **No move away from `Safe`.**

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Where this could be wrong.** If the user's answer to Q2 adds explicit
absent sets to `teardown_destroy.qv`, the host's text changes. Rows that
already depend on it may then show a changed contract or assumptions digest
without a leaf change, which is a content change, not a status move. If a
generalised edge reading differs from rule 38f on some guard/value pair the
fixtures do not cover, a leaf at a destroyer could change. That is still
`Unknown`, since every such row keeps other leaves.
