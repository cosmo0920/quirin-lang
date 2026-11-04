# CELL-RESULT-SET-ALIGNMENT prediction

## Amendment (2026-11-04, committed before the declarations that move rows)

This section supersedes the sections below where they differ. They were
committed at `7f6d7209c`, before the user's decisions. The decisions,
verbatim:

- Q1/Q2: "Basically, Q1(a) and Q2(i) But we must not handle descriptor 0
  as a valid handle. So, we need to exclude the 0 descriptor like:
  written_set(pipedes, [1, MAX]); a live descriptor is in [1, MAX]".
- After the stdin-closed counter-evidence was put to the user: "Write
  [1, MAX]".
- Q3: "(α) live_set suffix".

The `>= 0` guard of step 9 is not added. Step 10, the third patch version,
went back to `> 0`.

**What moves rows.** The in-force per-end declarations in
`fluent_bit/teardown_destroy.qv` keep `> 0` and gain two suffixes:
- `live_set([1, MAX])`, the declared bound;
- `absent_set(...)`, read from the pinned stores: calloc's `0` for the
  `ch_*` ends, and `0 | -1` for `ingress_queue_channels`.

Under the live set [1, MAX], a `> 0` dead edge `[MIN, 0]` meets the live set
nowhere, so it decides "absent". **Every move below is attributed to that
declared bound, not to POSIX.** POSIX's set is [0, MAX], and descriptor 0 was
observed natively on `ch_data` (`pipe2([0, 3])` with stdin closed). The
creation check of Q3 (α) refuses a creation into such a cell unless the
creator's derived written set is in force inside the live set.

| Route | Row (units) | Baseline | Predicted (cause) |
|---|---|---|---|
| system | `flb_output_instance_destroy` (×2) | Unknown, 27 | **Unknown, 26.** `descriptor-cell-zero-undecided` at :638 goes. The `ch_events[0..1] > 0` dead edges now decide "absent" (the [1, MAX] bound). Every other leaf stays, :631's `protocol-broken` included. |
| slice | `flb_input_instance_destroy` (×2) | Unknown, 62 | **Unknown, 61.** `zero-undecided` at :1353 goes (the [1, MAX] bound). |
| slice | `input_thread_instance_destroy` (×2) | Unknown, 8 | **Unknown, 7.** `zero-undecided` at :226 goes (the [1, MAX] bound). |
| slice | `input_instance_channel_events_init` (×2) | Unknown, 12 | **Unknown.** The two creations at :1380 refuse `descriptor-cell-live-set-unproved`, in place of `descriptor-cell-undecided`, unless the creator's written sets are in force at the slice route. The escapes and `protocol-broken` at :1387/:1401 may change with the creation's refusal. |
| slice | `flb_input_new` (×2) | Unknown, 178 | **Unchanged.** The `-1` stores lie in `absent_set(0 \| -1)`. |
| slice | `input_thread_instance_create` (×2), system `flb_output_init_all` (×2) | Unknown | **Unchanged.** Their creations are refused at the access gate first. |
| system | `flb_engine_started` (×2) | Safe | **Safe, unchanged.** `flb_config` stays withheld. |

- **No `Safe` move.** Each moved row keeps leaves of its own, such as
  uncontracted calls, the access gate and unresolved identities.
- **No `Unsafe` move.**
- The host's text changes, so rows depending on
  `flb_config_set_program_name` show a changed contract digest.
- The re-derived `mk_event_channel_create` range rests on `pipe`'s new
  digest and stays `[-1, 0]` and in force.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

(No status move on any route; the content changes are the ones listed.)

## Original prediction (7f6d7209c)

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

```text
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
