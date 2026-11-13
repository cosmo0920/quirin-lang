# W45-FB-LIST-FINITENESS-PREMISE: prediction

Frozen before any implementation code. Base `591e5523f` (`origin/main`). Only
this file exists in the card's report directory at the freeze.

## 1. The mechanism

A **declared list-finiteness premise** (`quant list TAG.MEMBER |-> #finite;`,
one line per intrusive list head, in a premise file the run names through a
new environment variable `QCORE_LIST_FINITENESS_PREMISES`) states that the
list whose head is the `struct mk_list`/`struct cfl_list` member `MEMBER` of a
`struct TAG` object is finite and well formed whenever a traversal of it
begins, and that no other thread writes its links during the traversal. It is
stated, not proved.

The premise is **checked where its frame is established**: at a list
traversal of rule 5l's header family (`for (C = (H)->next; C != (H); C =
C->next)`, its `_safe` and `prev` forms) whose head `H` is `&R->MEMBER`, with
`R` a stable local of type `struct TAG *`, the loop's frame — its body and
every function the body can call — must be read in the unit and must write no
list link. A callee the unit does not define (other than a short table of
read-only C library functions), an indirect call, inline assembly or a write
that may reach a list link (a list-pointer, record or character lvalue)
refuses with a named reason. Where the frame is established, the loop
records a **list-length bound fact**: it runs at most `|H|` iterations, `|H|`
the list's length at loop entry. It is recorded beside rule 44's fact in the
loop-measure rows, only when both selections are set. No verdict reads it.

## 2. What moves

Nothing. The custody analysis, its contracts, its routes and its report are
untouched; the premise file is not a contract module of any profile's bundle,
and with either selection unset nothing new runs. Every route's report is
byte-identical to the base, components sidecar included.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## 3. The slice's list-length loops (both selections set)

Of the census's 87 list-length loops (`expressible_as: list-contract`, Fluent
Bit `046accf6`), predicted:

* **8 to 14 gain a list-length bound fact**, among them the read-only
  traversals `sb_find_segregated_backlog_by_output_plugin_instance` (sb.c:203),
  `sb_get_releasable_output_queue_space` (sb.c:512), `flb_task_retry_count`
  (flb_task.c:407), `flb_task_running_count`'s two loops (flb_task.c:485,
  487), `engine_has_fluentbit_logs_input` (flb_engine.c:100) and
  `flb_output_get_instance` (flb_output.c:700);
* **most of the rest refuse on a callee the unit does not define** (Fluent
  Bit's logger `flb_log_check`/`flb_log_print`, `flb_free`, the ChunkIO and
  routes-mask functions);
* a smaller group refuses on a **link write** in the frame (the inline
  `mk_list_add`/`mk_list_del` primitives), on a head that is not a declared
  member of a stable root (a formal list pointer such as
  `flb_engine_destroy_tasks`' `tasks`, a local list), and `sched_timer_coro_get_id`'s
  `while` loop (flb_scheduler.c:422) is not a list traversal.

The static and shared variants of each core file give the same outcome per
loop. No loop gains a fact without a declared premise, and no custody verdict,
bounds, null or rule-44 counted measure changes.
