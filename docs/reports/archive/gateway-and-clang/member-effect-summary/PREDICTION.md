# MEMBER-EFFECT-SUMMARY prediction

> **USER DECISION 2026-11-05** (verbatim), answering MEMBER-ACCESS-PROJECTION's
> stop (member custody through a formal parent): "Option 1 (rec.)": a
> member-effect summary, with option 3's re-attribution done first as the
> verdict-neutral step.

Rows are relisted against `baseline:c8c7edf21`
(`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/` in the main
checkout), the newest published baseline. The tip also carries the round-3
cards merged at `25508ccaf` (integrate/r4: 38 corpus moves against that
baseline, recorded in docs/reports/member-access-projection/README.md §3);
they are theirs, not this card's.

## Revision 2 (current), committed before step 2's implementation

Step 1 (rule 38i, revision 1 below) is implemented at `8977ba29a` and moves
nothing. Step 2 adds the member-release half of the member-effect summary
(rule 38j):

- **Body check.** In a body, a member release `R(B->F)` at a `Released
  { nullable: true }` position, `B` a formal, is admitted (rule 5k's lowering,
  the position read `Borrowed`) under rule 5k's P1–P3 (the base never copied;
  the place's other occurrences null tests before the release; no loop, no
  `goto`/`setjmp`), now also when the body **keeps** `B` (no `R(B)` anywhere):
  the body then states, for `B`'s position, that it releases `F` (`stale`),
  or releases and then stores a null constant into `F` in the same block,
  nothing between (`cleared`). A call `g(.., B, ..)` or `g(.., &B->F, ..)` to
  a function whose member effect is in force counts as the release of every
  member it names, under the same premises (a cascade). A body with an effect
  and a release of `B` that is not rule 5k's P4 keeps its refusal.
- **Where the summary lives.** A body with a member effect gets **no**
  ordinary W4 summary (no record, no cross-unit export, no callback or
  indirect reading: each sees an uncontracted function, as before); its
  validated custody contract and its effect go to a separate environment
  table, installed for a function's parser under a reserved minted name read
  only by the direct-call route (the precedent is rule 12t's retained
  contracts). A direct whole-statement call to it is lowered through that
  contract only when the caller's own body check admits the call as the
  release of each member the effect names; otherwise the call keeps its
  refusal, under a name.
- **Declared form for bodiless callees**: `ensures normal
  resource_member_released(P, F, nullable|nonnull [, cleared]);` (contract
  specification and implemented grammar), loaded into the same separate
  table; a `nonnull` effect is never admitted at a call (nothing establishes
  an untracked member non-null).
- **Not in this step**: the store half (`P->F` takes a custody-carrying
  value), typedef-spelled bases (W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS),
  local (non-formal) parents.

### Rows (baseline `c8c7edf21`)

Found by [`scripts/candidates_step2.py`](scripts/candidates_step2.py) (an
Unknown row whose every leaf is the gate's refusal on an origin line spelling
`R(B->F);`, optionally under `if`), then each body read at the pinned
revision.

**Corpus (Fluent Bit 66910c10).** Seven candidates:

| row | body | predicted |
|---|---|---|
| `cmt_map.c::metric_release_storage` | four `free(metric->F)`, then four `metric->F = NULL;`, top level | Unknown -> Safe (`cleared` x4) |
| `cmt_metric.c::cmt_metric_exp_hist_snapshot_destroy` | `if (snapshot == NULL) return;`, then twice `if (s->F != NULL) { free(s->F); s->F = NULL; }` | Unknown -> Safe (`cleared` x2) |
| `cmt_opts.c::cmt_opts_exit` | five `if (opts->F) { cfl_sds_destroy(opts->F); }` (nullable release) | Unknown -> Safe (`stale` x5) |
| `mk_server/mk_lib.c::free_chunk_header` | `mk_mem_free(input->buffer); input->buffer = NULL;` (inherits `free`) | Unknown -> Safe (`cleared`) |
| `rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release` | typedef-spelled base `rd_kafka_mock_sgrp_record_state_t *state` | stays Unknown (W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS) |
| `rdkafka_offset.c::rd_kafka_offset_file_close` | typedef-spelled base `rd_kafka_toppar_t *rktp` | stays Unknown (same) |
| `mk_core/mk_memory.c::mk_ptr_free` | typedef-spelled base `mk_ptr_t *p` | stays Unknown (same) |

Their callers (`cmt_counter_destroy` and the other `cmt_*_destroy`,
`cmt_map_destroy`, `map_metric_destroy_unlocked`, `cmt_map_metrics_expire`,
`pack_metric`, `append_exp_histogram_metric_value`,
`cmt_exp_histogram_to_explicit`, `append_sample_to_metric`) all keep other
blockers on the base (`cfl_list_del` and `cmt_map_destroy` uncontracted,
unresolved identities, loop conditions): each loses its `call to
uncontracted function` leaf for the helper only where its own body check
admits the call, and **none moves**.

**System (046accf6).** No candidate. No move.

**Slice (34155955).** `plugins/in_tail/tail_fs_stat.c::flb_tail_fs_stat_remove`
(`if (file->fs_backend != NULL) { flb_free(file->fs_backend);
file->fs_backend = NULL; }`, `flb_free` inheriting `free`): Unknown -> Safe.
The slice's other gate-only rows are stores (`in->context = context`,
`file->fs_backend = fst`) and a returned descriptor close: no move.

```measure-prediction
route corpus
move cmt_map.c::metric_release_storage Unknown -> Safe
move cmt_metric.c::cmt_metric_exp_hist_snapshot_destroy Unknown -> Safe
move cmt_opts.c::cmt_opts_exit Unknown -> Safe
move mk_server/mk_lib.c::free_chunk_header Unknown -> Safe
delta Safe +4
route system
none
route slice
move plugins/in_tail/tail_fs_stat.c::flb_tail_fs_stat_remove Unknown -> Safe
delta Safe +1
```

No row may move to `Unsafe`; a move to `Safe` other than these is a finding
to explain before landing.

### The user's `current` run (Fluent Bit 046accf6), step 2

The same candidate reading over the run's report gives the same seven rows:
**4 functions newly Safe** (`metric_release_storage`,
`cmt_metric_exp_hist_snapshot_destroy`, `cmt_opts_exit`,
`free_chunk_header`), about 12 `member-release-through-formal` leaves of
step 1 gone with them, the three typedef-spelled bodies unchanged; no unit
or library changes status (each unit has other Unknown functions).

## Revision 1, committed before step 1's implementation on base `855b34ade`

```text
## Step 1: rule 38i, re-attribution only

Rule 5k is unchanged. The lexical gate's refusal `indirect memory access is
outside the resource projection` of a statement whose only deciding access
is member custody through a formal parent `P` (a formal the body never
rebinds) takes a name per premise, as rule 12k names an uncontracted callee:

| premise | statement | name |
|---|---|---|
| member release with the parent kept | `R(.., P->F, ..)`, `Released` at that position | `member-release-through-formal` |
| custody-carrying store into a member | `P->F = V`, `V` a pointer name or a custody-returning call | `member-store-through-formal` |
| an embedded member's address released | `R(.., &P->F[.G…], ..)`, `Released` at that position | `member-address-release-through-formal` |

The node keeps its writes and stays a non-held `Unknown` refusal, so no
verdict moves on any route:

measure-prediction
route corpus
none
route system
none
route slice
none

Rows: every corpus, system and slice row whose gate leaf has one of these
shapes changes that leaf's text only; its status and its leaf count are
unchanged (a line holding two refused statements that carried one gate leaf
can show two named leaves, as with rule 12k).

## The user's `current` run (Fluent Bit 046accf6), step 1

Estimated from MEMBER-ACCESS-PROJECTION's census of the run
(scripts/estimate_step1.py over census/premises.jsonl.gz, callee modes read
from every provider's .qv text; the census has no route answers, so this is
an estimate with the direction of its error stated):

| name | leaves | functions | functions whose every leaf it names |
|---|---:|---:|---:|
| member-release-through-formal | 72 | 53 | 4 |
| member-store-through-formal | 160 | 119 | 11 |
| member-address-release-through-formal | 1 | 1 | 0 |
| total | 233 | 164 | 15 |

- The indirect-access family loses about 233 of its 9,996 leaves to the three
  names. Over-estimate: the rule also asks that the formal is never rebound,
  that a stored name is one the lexical reader declares a pointer (a
  typedef-spelled pointer such as `sds` keeps the gate's text) and that a
  release reading admits with every actual borrowed; under-estimate: a leaf
  whose statement the census bound to two typed paths is not counted.
- No function, unit or library changes status.
```
