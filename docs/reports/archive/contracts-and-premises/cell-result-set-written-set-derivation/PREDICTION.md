# CELL-RESULT-SET-WRITTEN-SET-DERIVATION prediction

Committed before any implementation, on base `c8c7edf21` (origin/main). The
rows are relisted at `baseline:c8c7edf21`
(`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/` in the main
checkout).

> **USER DECISIONS 2026-11-04** (verbatim, standing): "Basically, Q1(a) and
> Q2(i) But we must not handle descriptor 0 as a valid handle. So, we need to
> exclude the 0 descriptor like: written_set(pipedes, [1, MAX]); a live
> descriptor is in [1, MAX]"; "Write [1, MAX]"; Q3 "(α) live_set suffix";
> "Accept the leak". The user launched this card on 2026-11-04 (verbatim):
> "CELL-RESULT-SET-WRITTEN-SET-DERIVATION."

## What the card will change

1. **The slot derivation** (`quirin-audit pin --bounds --derive-results`). A
   pinned declaration's `written_set(SLOT[K], SET) when #result == C` is
   derived from the configured unit's body chain, as rule 38e derives a
   result range:
   - forwarding, `return G(.., P, ..)`: `P[K]`'s set is `G`'s own written set
     of that position for class `C` (a C library declaration's, or an in-unit
     helper's derived one);
   - a store `*P = fd[k]` after the C library call's failure exit (`ret =
     pipe(fd); if (ret < 0) return ...;`), with the returned local narrowed by
     each test on the path, so a `return ret` whose narrowed set excludes `C`
     constrains nothing;
   - anything else derives nothing, and nothing is pinned.
2. **The pinned creator declarations.**
   - `mk_event_channel_create`: `written_set(r_fd[0], [1, MAX])` and
     `written_set(w_fd[0], [1, MAX])` when `#result == 0`, in
     `monkey/result_range.qv`.
   - `flb_pipe_create`: `written_set(pipefd[0], [1, MAX])` and
     `written_set(pipefd[1], [1, MAX])` when `#result == 0`, in the Fluent Bit
     bounds view.
   - Both are pinned at 046accf6, 34155955 and 66910c10 with their lock
     records.
3. **The reading.** Rule 38f's creation check reads a creator's written set
   only for the class its `#fresh` state holds in (`== 0`).

## Every row moves only through pipe's declared written set [1, MAX]

The derived sets rest on `pipe`'s `written_set(pipedes[0|1], [1, MAX])`.
That set is the user's **declared policy**, not POSIX's set, which is
`[0, MAX]`. Descriptor 0 is observed natively on `ch_data`: `pipe2([0, 3])`
with standard input closed. Under POSIX's set the derivation would give
`[0, MAX]`. That is not inside the declared `live_set([1, MAX])`, so every
creation would keep refusing. **Every reading below is attributed to the
declared bound** (follow-up CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY).

## Rows the change can reach

At `baseline:c8c7edf21` exactly one row reaches the live-set creation check:
the slice's `input_instance_channel_events_init`. No other row on any route
names `descriptor-cell-live-set-unproved`. Every other creation into a cell
with a declared live set is refused at the access gate first. `flb_pipe_create`
has no resource or cell declaration, so no route reads its written sets, and
its callers stay `call to uncontracted function`.

| Route | Row (units) | Baseline | Predicted (cause) |
|---|---|---|---|
| slice | `input_instance_channel_events_init` (×2) | Unknown, 8 leaves | **Unknown.** The two creations at :1380 pass the live-set check, since the derived [1, MAX] lies in `live_set([1, MAX])`, which is the declared bound. They then refuse `descriptor-cell-undecided` in place of `descriptor-cell-live-set-unproved`: the path holds no dead reading of the record's ends at entry. The `protocol-broken` and `descriptor-cell-escapes` leaves at :1401 may come back as they were at `baseline:32169290d`. The row keeps other leaves: the uncontracted `flb_input_event_loop_get`, retention-undecided, and the access gate. |
| slice | `flb_input_channel_init` (×2) | Unknown | **Unchanged.** `flb_pipe_create` stays uncontracted for custody. |
| system, slice | `flb_output_init_all`, `input_thread_instance_create`, `flb_input_new` (×2 each) | Unknown | **Unchanged.** Their creations are refused at the access gate before the creation check. |
| corpus | none | | **Unchanged.** No corpus row reaches the creation check. Its Monkey units create into no cell with a declared live set (CELL-GUARD-RESULT-RANGE-CORPUS-MONKEY-IDENTITY). |

- **No status move on any route.** The one row that changes swaps one
  refusal for another and stays `Unknown`.
- **No `Unknown`->`Safe` move.**
- **No `Unsafe` move.**
- The libevent and Fluent Bit bounds locks and the bounds environment change.
  Earlier result-range landings showed no row content change from the
  environment alone, so none is predicted.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Where this could be wrong.** At :1401 the slice row could differ from its
`baseline:32169290d` content, since the other leaves changed between the two
baselines. That is still `Unknown`. A row whose cell operands carry the newly
in-force written sets could show a changed digest without a leaf change.
