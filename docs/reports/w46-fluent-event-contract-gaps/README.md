# W46-FLUENT-EVENT-CONTRACT-GAPS

Base `9bce3986dd5703faa6d1fc2f485dc88a20902fdd`; prediction was committed
first (`0dbd85a5`). This card adds six source-pinned resource-v1 declarations
in three provider-owned modules. They are premises about caller custody and
counted users, not proof of the selected C implementation. The combined
185-unit corpus and Fluent Bit system slice are the integrator's measurement;
no production verdict movement is claimed from the focused fixtures here.

## Source-grounded declarations

Both pinned Fluent Bit checkouts were read: `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`
(`build/cycle19/fluent-bit-clang-source`) and
`046accf6aded7906f66a1baf0202da349e521736`
(`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source`).
The selected declarations were recorded and checked at **both** revisions:
six recorded, zero mismatched and zero stale per revision. The C ABI gate
withdraws a declaration on a different prototype; the source revision lock
withdraws it on a changed body. A defining unit's own result still depends on
that body's separate analysis.

| Contract | C reading and exact v1 effect |
|---|---|
| `flb_input_chunk_is_up` | `src/flb_input_chunk.c:3128-3131` / `:3168-3171` only returns `cio_chunk_is_up(ic->chunk)`. `lib/chunkio/src/cio_chunk.c:509-524` and `cio_file_is_up` read existing backend state. Borrowed chunk, scalar result, neutral task users. No residency transition. |
| `flb_task_get_route_status` | `include/fluent-bit/flb_task.h:226-246` walks existing `task->routes`, compares output pointer and returns a scalar status. Both inputs borrowed, neutral users. |
| `flb_task_set_route_status` | Same header `:248-267` walks the list and writes `route->status`; it never changes list membership or `task->users`. Task and output borrowed, scalar status argument borrowed, neutral users. |
| `flb_task_retry_count` | `src/flb_task.c:399-416` reads retry records and returns attempts or -1. Both pointer inputs borrowed, neutral users. |
| `flb_task_running_count` | `src/flb_task.c:477-496` reads inputs' task lists, `users` and retry list sizes and returns a scalar count. Config borrowed, neutral users. A query is not a user increment. |
| `flb_output_flush_id_get` | `src/flb_output.c:239-264` / `:242-267` reads, increments and wraps an ID on the output instance or thread instance and returns the prior scalar. The thread instance pointer returned by the TLS getter is local to this wrapper; neither pointer escapes to the wrapper's caller. Output borrowed, neutral users. This is a source-reviewed premise; the wrapper's own audit may remain Unknown because its raw TLS callee is undeclared. |

## Boundaries kept Unknown

The candidate list was narrowed after reading bodies transitively. No
contract was added for `flb_input_chunk_has_direct_routes` or
`flb_input_chunk_get_event_type`: `cio_meta_read` can enter
`cio_file_read_prepare -> mmap_file`, whose diagnostic arm calls the
configurable `cio->options.log_cb` through ChunkIO logging. This card cannot
bound that callback's custody or counted-user effects. The mapped metadata
view also publishes interior pointers through an out parameter. The existing
`flb_input_chunk_get_real_size` declaration's remap precedent does not prove
the callback neutral. This is a sharper boundary than the prediction's
provisional scalar-query list.

`flb_input_chunk_output_size_subtract` and
`flb_input_chunk_update_output_instances` reach `flb_warn`/`flb_trace` on
conditional paths. Fluent Bit logging can reach counters and a configured
metrics log callback, so this card makes no unconditional neutral claim for
those wrappers. The raw `flb_output_thread_instance_get` is undeclared:
its pointer aliases retained TLS storage, the same result-alias boundary as
`pthread_getspecific` (W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED). The scalar
flush ID wrapper does not publish that pointer.

The `flb_input_chunk_set_up`/`set_up_down` residency transitions can map or
reallocate member storage. `flb_input_chunk_release_lock` already has its
busy-state contract. Flush create/destroy/finished/dispatch can retain or
release flush/task resources and invoke callbacks. No borrowed/neutral
declaration was added for these lifecycle operations. Existing
`output_singleplex.qcore` and `task_users.qcore` remain unchanged. The
OR-1/OR-7 declarations are refuted by their defining bodies (provisional
Unsafe), while callers that depend on them stay Unknown; both observations
must be preserved. None of the named residuals is called a
validated upstream defect.

## Evidence and validation

The structured corpus pin deliberately moves 114 → 117 modules and adds six
declarations (46 pinned in `quirin-provider-fluent-bit/contracts.lock`, from
40). `providers/contracts.py sync` regenerated the load path. A Rust fixture
accepts a live borrowed scalar status call, refuses a call after release and
withdraws a contract whose C prototype disagrees. Lean's counted-user model
proves that a neutral helper preserves count and pending-release obligations,
while an opaque lifecycle call remains undecided; the C-to-Lean classification
and Rust correspondence remain the review premises of trust boundary 28.

Focused results before the standing gate: `qcore_contract_corpus` **4 passed**,
`retry_lifetime` **21 passed** (two new controls),
`providers/contracts.py check` exit **0**, Lean **210/210 audits** exit **0**,
and `quirin-audit pin --check` at each source revision **6 recorded / 0 not in
force / 0 stale**, exit **0**. An initial `providers/contracts.py sync` exit 1
correctly refused new declarations before their lock was pinned; after pinning,
`sync` exit 0. An initial focused corpus test exit 101 found this card's
misordered expected-row pins; row order was corrected and the focused run then
passed. Private target directory:
`/home/cosmo/Documents/GitHub/qcore-c/private/w46-contract-gaps/targets/fluent-events`;
builds set `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and
`CARGO_INCREMENTAL=0` to conserve disk. Standing and provider gates, commands,
exits, counts and private log paths are recorded below after the scheduled gate
slot. The integration arbiter is performed by the integrator after merge.

## Integrator list

- Update `docs/STATUS.md` module/declaration and corpus figures only from the
  combined arbiter, and add one newest-first wave increment paragraph.
- Reconcile `docs/reviews/r0_3/acceptance_matrix.json` against the measured
  results. Apply or decline each follow-up proposal in `followups.json` with
  source-grounded rationale; do not retire OR-1/OR-7 on this card's behalf.
- Run the combined corpus and system-slice before/after attribution, checking
  no unintended Safe or Unsafe movement and preserving Unknowns in the
  denominator. Keep the archived W46 external-callee report immutable.
- Resolve expected shared-file conflicts in `compiler/tests/qcore_contract_corpus.rs`,
  `providers/contract-load-path.json`,
  `providers/quirin-provider-fluent-bit/contracts.lock`, Lean audit registry,
  and corpus selection pins deliberately.

## Standing gate

Pending scheduled gate slot; fill from private step logs before completion.
