# W45-FB-RETRY-LIFETIME: frozen prediction

Frozen before any analyzer, loader, contract or Lean change. At this commit
the only other new files are the card's measurement scripts.

## Bases

* Quirin: local `main` `74579af6`. The BEFORE arbiter run
  (`private/w45-fb-retry-lifetime/before-74579af6`, `--jobs 6`) reproduces
  `docs/reports/w45-parenthesised-callee/integration/artifacts/`:
  `resource-diagnostics.json`, `chunk-contracts` and `monkey-source.json`
  byte-identical, `contracts.json` identical after the root alias,
  `verdict_bearing_identical: true` (`source-effects` differs in checkout-path
  digests only, as in every earlier worktree run). Functions 390 / 12 / 4,432;
  units 4 / 10 / 167 / 4; leak 37.
* Fluent Bit: re-pinned by the user to `046accf6aded7906f66a1baf0202da349e521736`
  (upstream `master`). A slice BEFORE at `864c3d97` (attempt-1) reproduces the
  published W45-FB-SYSTEM-SELECTION report function for function and
  diagnostic for diagnostic. The slice BEFORE at `046accf6` (attempt-2, base
  binary): units 0 Safe / 3 provisional Unsafe / 12 Unknown; functions
  25 / 2 / 207 / 3 not projected (one more function,
  `flb_output_task_flush_error`, added upstream); leak 3, `unknown_alias` 661,
  `unsupported` 4,331. Every other moved row is an origin-line shift or an
  upstream body edit (report §1).

## Mechanisms to be added

1. **Container publication** (owned by this card). `mk_list_add` is declared to
   publish the object whose member it links into a container reached from the
   caller; `mk_list_del` retires the publication. A published body-acquired
   object is discharged into the container (no leak at exit); releasing it
   while published is a violation candidate; unlinking restores the body's
   custody. Publication into a container whose lifetime the body does not
   establish, or of an object it does not hold, refuses.
2. **Counted users.** Declared counter operations (`added`, `dropped` without a
   release check), a declared per-return exit clause, and a per-function check
   of each clause against the body: held, refuted (provisional `Unsafe`
   candidate) or undecided (`Unknown`, with its blocker).

## Forecast: the 185-unit arbiter

* **Leak pin:** 37 → between 31 and 37. The candidates are the four
  MK-LIST-ADD-BORROWED-PUBLICATION leaves still accused (`mk_rconf_read`,
  `mk_vhost_create`, `mk_vhost_set_property`, `mk_vhost_set_single`) and any
  other leaf whose object is linked with `mk_list_add` into a caller's list.
  No leak leaf is *added* by publication itself.
* **Functions leaving `Safe`:** between 0 and 6, each a correction: a body that
  publishes an object it does not own, returns a published object, or unlinks
  an object with `mk_list_del` that it cannot show published (previously an
  uncontracted call, so at most the ones whose only blocker moved).
* **Functions entering `Safe`:** at most 2, only where the false leak was the
  only diagnostic.
* **Provisional `Unsafe`:** falls by between 0 and 4 (a withdrawn false leak
  leaves `Unsafe`); may rise by at most 2 where a newly modelled unlink or
  publication exposes a release of a published object (`release_after_escape`),
  each classified by reading.
* **`unsupported`:** moves by fewer than 150 leaves, mostly same-position
  substitutions of `call to uncontracted function mk_list_del` by the
  unlink refusal.
* **Units:** at most one unit moves (`Unsafe` → `Unknown` in a Monkey unit
  whose only violation was a publication leak).
* The counted-user contracts touch no corpus unit: no corpus file calls
  `flb_task_users_*`, `flb_task_retry_destroy` or `flb_output_task_*`.

## Forecast: the system slice at `046accf6`

* `flb_output_task_queue_enqueue`: provisional `Unsafe` → `Unknown` (its leak
  at the return after `mk_list_add` is withdrawn; its other refusals stay).
* `sb_append_chunk_to_segregated_backlog`: its leak is withdrawn as well
  (`Unsafe` → `Unknown`) if its container root is a formal; recorded for the
  concurrent FB-BACKLOG-CUSTODY card, not claimed by it.
* **OR-1** (`flb_output_task_singleplex_enqueue` restores `task->users` on
  every `-1` return): **decided-refuted** on the enqueue-allocation arm
  (C2's shape), a provisional `Unsafe` candidate, under the synchronous-output
  premise; the `return flb_output_task_queue_flush_one(…)` exit undecided.
* **OR-7** (`flb_output_task_flush` returns `-1` exactly when it added no
  user): **decided-refuted** on the threaded arm (N2: the pool dispatch fails,
  the user is rolled back, and the function returns `0`), reachable for any
  threaded output without the synchronous premise; the non-threaded arms hold
  at the new revision.
* **OR-6** (`queued_task` not used after the flush fails): **Unknown**. It
  needs the identity of a container element recovered with `container_of`
  (`mk_list_entry_first`) and a callee summary that frees it (N3); neither is
  in scope.
* **OR-2**: not reached (Unknown, unchanged).
* The five slice functions otherwise stay `Unknown`: no function enters
  `Safe`.
