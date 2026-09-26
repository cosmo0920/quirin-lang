# W45-FB-ENGINE-PROGRESS: predictions

**Corpus and slice.** This card changes no analyzer, contract, provider or
Lean code. The prediction is no movement:

* the 185-unit corpus arbiter is not required, so it was not run;
* the slice at `046accf6` reproduces `main`'s archived slice with 0 differing
  function rows.

**Outcome:** the slice reproduced with 0 differing rows
([comparison](artifacts/slice-vs-w45-refuted-contract-withdrawal.txt)).

**Native runs.** Each gdb script's header states the source-reading
prediction. Each header was written before that script's first run, and the
headers are unchanged in substance since.

| Run | Prediction (script header) | Outcome |
|---|---|---|
| C1 | The collector yields every pass and the queues drain | 10 / 10 yields, 13 queued |
| E1 | Under a persistent `CIO_ERROR`, the collector enters once and does not return (the earlier observation at `864c3d97`) | 1 entry, 0 yields, 12,000 failed bring-ups |
| E2 | After one `cio_file_update_size` failure the descriptor stays open, and every later `up_force` fails at "file descriptor already exists" (cio_file.c:827-830) | 11,999 arm hits, 0 yields |
| C2 | After one `open` failure the next `up_force` succeeds and the queues drain | the 2nd bring-up succeeded, 13 queued, 10 / 10 yields |
| E3 | The −1 arm reschedules without spending an attempt, so `retry_limit` 3 does not end the retry | 12 dispatches, attempts 1 throughout, 0 drops |
| C3 | Attempts are spent 1, 2, 3, then "cannot be retried" | as predicted |

One prediction was not in any header: that the flush counter would read 0
during E1 and E2. The counter was added after the first E1 and E2 runs, which
had already shown no yield. Those earlier runs are listed as superseded in
[evidence.json](native/evidence.json).
