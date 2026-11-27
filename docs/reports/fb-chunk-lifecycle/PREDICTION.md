# FB-CHUNK-LIFECYCLE: prediction

Committed before the semantic change. Base `76c1f9cfc`. The card tip before
this file is `ca098c30b` (the base probe).

## What was known when this was written

**Base probe** (`ca098c30b`, `tests/static_constant_formal.rs`, run at base):
`static void subject(int destroy) { void *p = q_alloc(); if (destroy)
q_free(p); }` is a provisional `Unsafe` whatever its callers pass: every call
`subject(1)`, `subject(1); subject(1)`, a parenthesised `subject((1))`, a
zero, mixed or variable actual, an escaped address, and no caller at all.

**Base system slice** (`private/fcl/run-system-base`, release binary of a
`git archive` of `76c1f9cfc`, `contract/profiles/fluent-bit/system-selection.json`
over Fluent Bit 046accf6, the opt-in user load path): 114 Safe / 0 Pending /
329 Unknown / 5 Unsafe, the priority-0 tip figures. Unit `sb.c` is `Unsafe`
through one row, `sb_remove_chunk_from_segregated_backlog`, whose leaves are
exactly:
- `live obligation acquired at plugins/in_storage_backlog/sb.c:226` (the
  `destroy == 0` exit);
- `indirect memory access is outside the resource projection` (the
  `backlog->ins->fs_backlog_chunks_size` accounting, FB-BACKLOG-ACCOUNTING);
- `declared specification ... is not established` (rule 5l).

**Candidate census** (`scripts/candidate_census.py` over the raw checkout,
macro names `FLB_TRUE`/`rd_true`/... read as 1/0): nine static functions
whose integer formal is spelled only in `if (NAME)` / `if (!NAME)` and whose
every in-file call passes one truth. Besides `sb.c`, the ones in audited
libraries are `rdkafka_conf.c::rd_kafka_anyconf_set_modified` (`is_modified`,
one call passing `1`), `flb_http_client_http1.c::parse_term` (both trims,
both calls passing `FLB_TRUE`) and `ne_systemd_linux.c::str_ends_with`
(`caseless`, every call `FLB_TRUE`); the others are tests and examples.
Header `static inline` bodies were not censused.

## Planned change

New gateway rule 90 (`legacy_static_constant_formal.rs`, one hook in
`build_function_cfg_with`): for a `static` definition whose identifier occurs
only as its declarator, file-scope prototypes and direct calls, and whose
integer formal every call fixes to a decimal constant of one truth and the
body spells only in `if (NAME)` / `if (!NAME)`, each branch on the formal
keeps the arm that truth selects, and the row carries the premise
`static-call-sites-closed`.

## Predicted

- **System slice:** `sb_remove_chunk_from_segregated_backlog` `Unsafe ->
  Unknown`. The leak leaves (FLB_TRUE preprocesses to `1`); the indirect
  access and the withdrawn specification stay, so the row is not `Safe`.
  Unit `sb.c` `Unsafe -> Unknown` (every other row of it is Safe or
  Unknown). Totals 114 / 0 / 330 / 4. No other system row moves; the
  callers of the function (`sb_remove_chunk_from_segregated_backlogs`,
  `sb_destroy_backlog`, `sb_release_output_queue_space`, `cb_queue_chunks`)
  keep their uncontracted-call leaves, because the specification and the W4
  summary still need the body to report nothing.
- **Opt-in current** (`--monkey-linked`): no status move. The candidate rows
  that are on the route (`rd_kafka_anyconf_set_modified`, possibly
  `parse_term`) lose at most the leaves of their pruned arm and keep their
  indirect-access refusals.
- **No Safe -> non-Safe move**, and no Unknown -> Unsafe move: the rule only
  removes edges.

```measure-prediction
route corpus
none
route system
move plugins/in_storage_backlog/sb.c::sb_remove_chunk_from_segregated_backlog Unsafe -> Unknown
delta Unsafe -1
delta Unknown +1
route slice
none
```

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host
(no `build/cycle19` inputs or baselines); the system slice above is run with
`current --selection` from the prepared inputs instead.
