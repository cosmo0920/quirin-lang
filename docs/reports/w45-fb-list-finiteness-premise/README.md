# W45-FB-LIST-FINITENESS-PREMISE: a list-length bound under a declared premise

Card W45-FB-LIST-FINITENESS-PREMISE, base `591e5523f` (`origin/main`), the
named follow-up W45-FB-ENGINE-PROGRESS-MEASURE recorded for the census's 87
list-length loops (premise PL). It adds **gateway rule 48**
([rule](../../agents/gateway.md#constraint-48)), a **declared list-finiteness
premise** (contract specification
[C04.22](../../QCORE_CONTRACT_SPEC_r0_3.md#c0422-the-list-finiteness-premise),
grammar [G05l](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05l-list-finiteness-premises))
and the Lean module `QCore.ListLengthBound`.

**What it is.** A list-length bound fact, recorded beside rule 44's loop fact
and the resource verdict: a list traversal over a declared head runs at most
`|H|` iterations, `|H|` the list's length at loop entry. **What it is not.** No
verdict moves, nothing reads the fact, and there is no progress, termination
of the body, latency or throughput claim. A list may still be long.

## 1. Prediction

[PREDICTION.md](PREDICTION.md), committed at `0abf79a23` before any
implementation code: no custody verdict moves on any route, and 8 to 14 of
the 87 loops gain a bound. **Measured: 17** (§4). Every loop the prediction
named gained one; it missed `input_chunk_collect_output_references`'s six
loops (their callee `flb_chunk_route_plugin_matches` is defined in
`flb_input_chunk.c`, not in another unit as predicted) and four read-only loops
it did not name (`flb_engine_get_chunk_backpressure_percent`,
`sched_timer_coro_get_id`'s inner loop, `sched_timer_coro_get_by_id`,
`flb_sched_timer_coro_get`). The refusal groups were as predicted, except that
link writes (34) outnumber undefined callees (21): the precedence reports a
link write first when a frame has both.

## 2. Mechanism

* **The premise** (`crates/qcore-contract/src/list_finiteness.rs`). A file of
  `quant list TAG.MEMBER |-> #finite;` lines, named by
  `QCORE_LIST_FINITENESS_PREMISES`, loaded total-or-reject (a rejected file
  declares nothing). It states that the list headed at that member is finite
  and well formed when a traversal begins and that no other thread writes its
  links meanwhile. It is never proved, and it is never a contract module: no
  resource-v1 loader, provider package, profile contract set or bundle digest
  reads it, so no custody, bounds or null verdict can depend on it, and no
  provider lock moves. The Fluent Bit file is
  [`contract/profiles/fluent-bit/list-finiteness.qv`](../../../contract/profiles/fluent-bit/list-finiteness.qv),
  22 heads read from the census's PL and PC rows.
* **The check where its frame is established**
  (`crates/qcore-c-gateway/src/list_length_bound.rs`). On Clang's typed AST of
  the unit's preprocessed text, a loop gets a bound only when it is rule 5l's
  header family over `&R->M` (`R` a stable local of type `struct TAG *`),
  `TAG.M` is declared, and the loop's **frame** — its body and every function
  it can call, transitively — is read in the unit and writes no list link.
  Every function body of the unit is summarised (direct callees, indirect
  calls, opaque constructs, and the first write whose target may hold a link:
  a record, a list pointer or a character object), and the frame is composed
  once the whole unit has been read, so a callee defined after its caller is
  read. A callee the unit does not define refuses, except eight read-only C
  library functions (`strlen`, `strcmp`, …); so does every link-writing inline
  primitive (`mk_list_add`, `mk_list_del`), so this card admits no insertion
  and no unlink at all. Ten named refusals, in a fixed precedence.
* **Fail-closed choices.** A declared but unread frame, a callee's contract
  and a callee's declared effects establish nothing (the rule names the
  callee); a head reached through a member chain (`&ic->in->config->outputs`)
  or a formal list pointer is `head-not-member`; a rejected premise file is
  `premise-not-declared` with the loader's error.
* **The selection.** Only with rule 44's `QCORE_LOOP_MEASURE_PROPERTY` set.
  Each loop then carries a `list_bound` field (`qcore-list-length-bound/1`)
  and the summary a `list_length_bound.outcomes` tally. Without the premise
  variable the rows and summary are rule 44's byte for byte (the field is
  omitted), and rule 44's own fields never change either way
  (`tests/list_length_bound.rs`, `qcore-audit/tests/loop_measure_selection.rs`).

## 3. Lean

`QCore.ListLengthBound` with [`proof/AuditListLengthBound.lean`](../../../proof/AuditListLengthBound.lean)
(24 audited theorems, no placeholders, no project axioms), registered through
`scripts/gen_lean_roots.py`. The links are a successor function per test; the
declared premise is `WellFormed (links 0) h n` (following the links from the
head meets it again first after `n + 1` steps); the frame check is
`FrameKeepsLinks` (every body leaves the links unchanged).

* A link-preserving frame fixes the links, the cursor is the walk from the
  head, and the safe form's cursor is the plain one
  (`a_link_preserving_frame_fixes_the_links`, `cursor_is_the_walk`,
  `safe_cursor_is_the_plain_cursor`).
* Every run has at most `n` iterations, plain or safe, the loop meets the head
  after exactly `n`, and a prefix of a run is a run (`run_length_bounded`,
  `safe_run_length_bounded`, `the_loop_exits_after_the_length`,
  `the_full_run_has_the_length`, `a_prefix_of_a_run_is_a_run`); `n - j` is
  positive while running and decreases by one
  (`measure_positive_while_running`, `measure_decreases_by_one`).
* Both premises are necessary: a body appending one node per iteration keeps
  every state finite and well formed yet never exits
  (`an_inserting_body_never_exits`, `every_inserted_state_is_well_formed`),
  and a cycle missing the head never exits and has no length
  (`an_ill_formed_list_never_exits`, `the_ill_formed_list_has_no_length`).
* The decision admits only shapes meeting every gate
  (`an_admitted_shape_meets_the_premises`, with the refusal theorems); the
  vectors equal the Rust `LIST_DECIDE_VECTORS` (`decide_vectors_hold`, checked
  from Rust by `the_decide_vectors_hold_and_match_the_lean_table`); recording
  facts keeps the verdict (`recording_a_fact_keeps_the_verdict`,
  `recording_facts_keeps_the_verdict`).

The bridge from the syntactic frame check to `FrameKeepsLinks` is a stated
premise (trust boundary 28): Clang's AST read faithfully, no symbol
interposition, C's effective-type rule, and the library table's functions
writing no object.

## 4. The slice's 87 list-length loops

**Measured on this host**, on the card's own preparation of Fluent Bit
`046accf6` (the census's and the `fluent-bit-system` profile's revision;
`quirin-audit prepare`, private `private/w45-list/attempt-1/`, compile database
sha256 `7d7dea55…`), because this host holds none of the live profiles'
prepared inputs or baselines. The brief named upstream `90978ac6`; that is the
tail-pause profile's export, not the census's revision, so the system
selection at `046accf6` was used. Route: `quirin-audit current` over
`system-selection.json` with the eight-home link map, null and outer
projections, `QCORE_LOOP_MEASURE_PROPERTY` and the premise file (private
driver `private/w45-list/run-system.sh`, release binary of `9ec4d7417`, run
`private/w45-list/system-tip2-declared/`). The join
([scripts/join.py](scripts/join.py), output [loops.json](loops.json)) matches
each census loop by file, function and original line and refuses to publish
an unmatched loop: every loop matched, and the static and shared variants
agree on every loop.

| Outcome | Loops |
|---|---|
| **bounded** | **17** |
| `frame-writes-links` | 34 |
| `frame-callee-undefined` | 21 |
| `head-not-member` | 12 |
| `not-a-list-traversal` | 1 |
| `opaque-control` | 1 |
| `frame-indirect-call` | 1 |

Over the whole slice (229 loop rows, both variants) the tally is 32 bounded,
60 `frame-writes-links`, 42 `frame-callee-undefined`, 22 `head-not-member`,
69 `not-a-list-traversal` (counted, `while` and `do` loops), 2
`opaque-control`, 2 `frame-indirect-call`. Rule 44's own tally is unchanged
(30 measured, 199 refused, identical by reason to the base run). This run is
also the first slice run of rule 44's selection on any host; the rest of
`W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN` (the three live profiles' arbiter)
stays owed.

**Why 70 loops stay without a bound**, by named reason:

* **Link writes (34).** The frame unlinks or links: `mk_list_del`
  (destroyers, `_safe` cleanups, the scheduler's request moves) or
  `__mk_list_add` (`sb_append_chunk_to_segregated_backlogs`,
  `sb_segregate_chunks`, `flb_task_create`). Admitting them needs PL's clause
  "a `_safe` traversal removes at most the current node" checked, which this
  card does not do. Four are not list primitives: `metrics_append_input` and
  `flb_output_upstream_ha_set` reach a character write in
  `msgpack_pack_map`/`flb_strndup`, `tasks_start` writes an `unsigned char`
  status in the loop, and `flb_output_init_all` writes a `struct mk_list *`
  field (`tls_config_map`).
* **Undefined callees (21).** Fluent Bit's logger (`flb_log_print`),
  `flb_chunk_route_plugin_matches` in `flb_task.c`, ChunkIO
  (`cio_chunk_get_content_size`), `flb_input_chunk_destroy`, `flb_input_name`,
  `flb_input_chunk_down`, `__errno_location`, `strstr` (not in the library
  table) and `flb_engine_dispatch`.
* **Heads (12).** Nine heads are reached through a member chain
  (`&ic->in->config->outputs`, `&context->cio->streams`, …), whose
  intermediate pointer the frame could rewrite; three are a formal list
  pointer or a local list (`flb_engine_destroy_tasks`' `tasks`,
  `flb_output_flush_finished`'s `list`, `schedule_request_promote`'s
  `failed_requests`).
* `sched_timer_coro_get_id`'s outer `while` (not the family; its census bound
  `|list| + 1` rests on another argument), `flb_sched_timer_coro_cleanup`
  (inline assembly in `flb_coro_destroy`) and `flb_output_pre_run` (the
  plugin's `cb_pre_run` callback).

| Id | Site | Function | Census premise | Variants | Outcome | Detail |
|---|---|---|---|---|---|---|
| EP-L001 | `plugins/in_storage_backlog/sb.c:142-146` | `sb_destroy_backlog` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L002 | `plugins/in_storage_backlog/sb.c:155-174` | `sb_allocate_backlogs` | PC | 1 | `head-not-member` | the head `&context->ins->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L003 | `plugins/in_storage_backlog/sb.c:185-193` | `sb_destroy_backlogs` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L004 | `plugins/in_storage_backlog/sb.c:203-209` | `sb_find_segregated_backlog_by_output_plugin_instance` | PL | 1 | **bounded** | flb_sb.backlogs |
| EP-L005 | `plugins/in_storage_backlog/sb.c:222-246` | `sb_remove_chunk_from_segregated_backlog` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L006 | `plugins/in_storage_backlog/sb.c:255-259` | `sb_remove_chunk_from_segregated_backlogs` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L007 | `plugins/in_storage_backlog/sb.c:341-352` | `sb_append_chunk_to_segregated_backlogs` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `__mk_list_add` |
| EP-L008 | `plugins/in_storage_backlog/sb.c:399-483` | `sb_segregate_chunks` | PL | 1 | `head-not-member` | the head `&context->cio->streams` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L009 | `plugins/in_storage_backlog/sb.c:408-482` | `sb_segregate_chunks` | PL+PF | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `__mk_list_add` |
| EP-L010 | `plugins/in_storage_backlog/sb.c:512-520` | `sb_get_releasable_output_queue_space` | PL | 1 | **bounded** | sb_out_queue.chunks |
| EP-L011 | `plugins/in_storage_backlog/sb.c:551-573` | `sb_release_output_queue_space` | PL+PW+PF | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L013 | `plugins/in_storage_backlog/sb.c:615-721` | `cb_queue_chunks` | PL | 1 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L015 | `src/flb_input_chunk.c:569-731` | `flb_input_chunk_release_space` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L016 | `src/flb_input_chunk.c:947-959` | `flb_input_chunk_has_other_routes` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L017 | `src/flb_input_chunk.c:1143-1159` | `flb_input_chunk_release_space_compound` | PC | 2 | `head-not-member` | the head `&output_plugin->config->inputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L018 | `src/flb_input_chunk.c:1223-1247` | `flb_input_chunk_find_space_new_data` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L019 | `src/flb_input_chunk.c:1270-1291` | `flb_input_chunk_has_overlimit_routes` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L020 | `src/flb_input_chunk.c:1354-1364` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L021 | `src/flb_input_chunk.c:1366-1381` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L022 | `src/flb_input_chunk.c:1388-1394` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L023 | `src/flb_input_chunk.c:1409-1419` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L024 | `src/flb_input_chunk.c:1421-1436` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L025 | `src/flb_input_chunk.c:1439-1445` | `input_chunk_collect_output_references` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L042 | `src/flb_input_chunk.c:2643-2667` | `flb_input_chunk_destroy_corrupted` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L043 | `src/flb_input_chunk.c:2729-2753` | `flb_input_chunk_destroy` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L044 | `src/flb_input_chunk.c:3204-3236` | `memrb_input_chunk_release_space` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L046 | `src/flb_input_chunk.c:3742-3783` | `flb_input_chunk_ring_buffer_collector` | PC | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L048 | `src/flb_input_chunk.c:3994-4024` | `flb_input_chunk_update_output_instances` | PC | 2 | `head-not-member` | the head `&ic->in->config->outputs` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L049 | `src/flb_storage.c:170-341` | `metrics_append_input` | PC | 2 | `frame-writes-links` | a write to a character object (`unsigned char`) in `msgpack_pack_map` |
| EP-L050 | `src/flb_storage.c:283-300` | `metrics_append_input` | PL | 2 | `frame-callee-undefined` | `cio_chunk_get_content_size` is called in the frame and the unit does not define it |
| EP-L051 | `src/flb_storage.c:632-635` | `flb_storage_input_destroy` | PL+PF | 2 | `frame-callee-undefined` | `flb_input_chunk_destroy` is called in the frame and the unit does not define it |
| EP-L052 | `src/flb_storage.c:649-658` | `storage_contexts_create` | PC | 2 | `frame-callee-undefined` | `flb_log_print` is called in the frame and the unit does not define it |
| EP-L054 | `src/flb_storage.c:854-862` | `dlq_stream_total_size` | PL | 2 | `frame-callee-undefined` | `cio_chunk_get_content_size` is called in the frame and the unit does not define it |
| EP-L056 | `src/flb_task.c:169-179` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L057 | `src/flb_task.c:181-196` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L058 | `src/flb_task.c:203-209` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L059 | `src/flb_task.c:224-234` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L060 | `src/flb_task.c:236-251` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L061 | `src/flb_task.c:254-260` | `task_collect_output_references` | PC | 2 | `frame-callee-undefined` | `flb_chunk_route_plugin_matches` is called in the frame and the unit does not define it |
| EP-L062 | `src/flb_task.c:325-337` | `flb_task_retry_create` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L063 | `src/flb_task.c:407-413` | `flb_task_retry_count` | PL | 2 | **bounded** | flb_task.retries |
| EP-L064 | `src/flb_task.c:426-432` | `flb_task_retry_clean` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L065 | `src/flb_task.c:485-493` | `flb_task_running_count` | PC | 2 | **bounded** | flb_config.inputs |
| EP-L066 | `src/flb_task.c:487-492` | `flb_task_running_count` | PL | 2 | **bounded** | flb_input_instance.tasks |
| EP-L067 | `src/flb_task.c:516-541` | `flb_task_running_print` | PC | 2 | `frame-callee-undefined` | `flb_input_name` is called in the frame and the unit does not define it |
| EP-L068 | `src/flb_task.c:521-540` | `flb_task_running_print` | PL | 2 | `frame-callee-undefined` | `flb_log_print` is called in the frame and the unit does not define it |
| EP-L069 | `src/flb_task.c:524-535` | `flb_task_running_print` | PL | 2 | `frame-callee-undefined` | `flb_log_print` is called in the frame and the unit does not define it |
| EP-L073 | `src/flb_task.c:786-824` | `flb_task_create` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L074 | `src/flb_task.c:848-873` | `flb_task_create` | PC | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `__mk_list_add` |
| EP-L075 | `src/flb_task.c:912-920` | `flb_task_destroy` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L076 | `src/flb_task.c:933-937` | `flb_task_destroy` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L077 | `src/flb_task.c:967-971` | `flb_task_queue_destroy` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L078 | `src/flb_task.c:973-977` | `flb_task_queue_destroy` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L079 | `src/flb_engine.c:100-108` | `engine_has_fluentbit_logs_input` | PC | 2 | **bounded** | flb_config.inputs |
| EP-L081 | `src/flb_engine.c:143-147` | `flb_engine_destroy_tasks` | PL+PF | 2 | `head-not-member` | the head `tasks` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L082 | `src/flb_engine.c:165-192` | `flb_engine_reschedule_retries` | PC | 2 | `frame-callee-undefined` | `__errno_location` is called in the frame and the unit does not define it |
| EP-L083 | `src/flb_engine.c:167-191` | `flb_engine_reschedule_retries` | PL | 2 | `frame-callee-undefined` | `__errno_location` is called in the frame and the unit does not define it |
| EP-L084 | `src/flb_engine.c:178-190` | `flb_engine_reschedule_retries` | PL | 2 | `frame-callee-undefined` | `__errno_location` is called in the frame and the unit does not define it |
| EP-L085 | `src/flb_engine.c:202-210` | `flb_engine_flush` | PC | 2 | `frame-callee-undefined` | `flb_engine_dispatch` is called in the frame and the unit does not define it |
| EP-L086 | `src/flb_engine.c:290-305` | `flb_engine_get_chunk_backpressure_percent` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L090 | `src/flb_engine_dispatch.c:254-348` | `tasks_start` | PL+PF | 2 | `frame-writes-links` | a write to a character object `status` (`unsigned char`) in the loop |
| EP-L091 | `src/flb_engine_dispatch.c:282-341` | `tasks_start` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L092 | `src/flb_engine_dispatch.c:381-446` | `flb_engine_dispatch` | PL+PF | 2 | `frame-callee-undefined` | `flb_input_chunk_down` is called in the frame and the unit does not define it |
| EP-L093 | `src/flb_engine_dispatch.c:456-464` | `flb_engine_dispatch` | PL+PF | 2 | `frame-callee-undefined` | `flb_log_print` is called in the frame and the unit does not define it |
| EP-L097 | `src/flb_scheduler.c:183-221` | `schedule_request_promote` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L098 | `src/flb_scheduler.c:224-228` | `schedule_request_promote` | PL | 2 | `head-not-member` | the head `&failed_requests` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L100 | `src/flb_scheduler.c:362-368` | `flb_sched_request_invalidate` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L101 | `src/flb_scheduler.c:379-385` | `flb_sched_request_invalidate` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L102 | `src/flb_scheduler.c:422-439` | `sched_timer_coro_get_id` | PL | 2 | `not-a-list-traversal` | a `while` loop is not a list traversal |
| EP-L103 | `src/flb_scheduler.c:424-430` | `sched_timer_coro_get_id` | PL | 2 | **bounded** | flb_sched.timer_coro_list |
| EP-L104 | `src/flb_scheduler.c:452-457` | `sched_timer_coro_get_by_id` | PL | 2 | **bounded** | flb_sched.timer_coro_list |
| EP-L105 | `src/flb_scheduler.c:827-831` | `flb_sched_destroy` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L106 | `src/flb_scheduler.c:834-838` | `flb_sched_destroy` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L107 | `src/flb_scheduler.c:841-846` | `flb_sched_destroy` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L108 | `src/flb_scheduler.c:849-853` | `flb_sched_destroy` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L109 | `src/flb_scheduler.c:924-928` | `flb_sched_timer_cleanup` | PL | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L110 | `src/flb_scheduler.c:943-947` | `flb_sched_timer_coro_cleanup` | PL | 2 | `opaque-control` | inline assembly in `flb_coro_destroy` |
| EP-L111 | `src/flb_scheduler.c:1002-1007` | `flb_sched_timer_coro_get` | PL | 2 | **bounded** | flb_sched.timer_coro_list |
| EP-L113 | `src/flb_output.c:158-164` | `flb_output_pre_run` | PC | 2 | `frame-indirect-call` | an indirect call (`int`) in the loop |
| EP-L114 | `src/flb_output.c:652-672` | `flb_output_exit` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L115 | `src/flb_output.c:700-706` | `flb_output_get_instance` | PC | 2 | **bounded** | flb_config.outputs |
| EP-L116 | `src/flb_output.c:742-745` | `flb_output_flush_finished` | PL+PF | 2 | `head-not-member` | the head `list` is not `&R->M` with `M` a list member of `struct TAG` |
| EP-L117 | `src/flb_output.c:776-787` | `flb_output_new` | PL | 2 | `frame-callee-undefined` | `strstr` is called in the frame and the unit does not define it |
| EP-L118 | `src/flb_output.c:802-813` | `flb_output_new` | PL | 2 | `frame-callee-undefined` | `strstr` is called in the frame and the unit does not define it |
| EP-L119 | `src/flb_output.c:1445-1461` | `flb_output_plugin_property_check` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `prev` (`struct mk_list *`) in `mk_list_del` |
| EP-L120 | `src/flb_output.c:1537-1922` | `flb_output_init_all` | PL+PF | 2 | `frame-writes-links` | a write to a list pointer `tls_config_map` (`struct mk_list *`) in the loop |
| EP-L121 | `src/flb_output.c:2082-2085` | `flb_output_upstream_ha_set` | PL | 2 | `frame-writes-links` | a write to a character object (`char`) in `flb_strndup` |

## 5. Arbiter comparison

`quirin-audit measure --declared PREDICTION.md` over the `fluent-bit-system`
route ([measure-inputs.json](measure-inputs.json): the live route's selection,
profile and link map over the card's own preparation), **result `match`**,
status `complete`, exit 0 ([record](arbiter/native-comparison.json); private
run `private/w45-list/measure-3/`):

* **0 moves**, 0 unit moves, 0 unmatched rows; status deltas Safe 0, Unknown
  0, Unsafe 0 (448 function rows: 107 Safe, 337 Unknown, 4 Unsafe on both
  sides; 15 unit rows: 13 Unknown, 2 Unsafe).
* Not byte-identical, for environment reasons only: the four
  `review_holds` content changes (`flb_output_exit` and `flb_storage_create`,
  each variant) and the `compile_configuration` unit fields spell each side's
  own checkout path (`private/w45-list/base-src` vs `private/w45-list/tip-src`)
  in a module path, and the report, components, null and outer files carry
  the analyzer revision in their evidence versions and the ids hashed from
  them. The chunk-contract, source-effect and null-summary files are
  byte-identical.

**How it was run, and what was not.** The `rev:` sides need a private mount
namespace, which this host refuses (`unshare: write failed /proc/self/uid_map:
Operation not permitted`, first attempt `private/w45-list/measure-1/`). The
sides are therefore binaries built from `git archive` snapshots of the base
`591e5523f` and of `438fef384` (the card's code at its final state; private
`private/w45-list/{base,tip}-src/`), with `QCORE_AUDIT_BUILD_COMMIT` set to
each revision. A second attempt (`private/w45-list/measure-2/`) used the live
worktree as the tip side and was refused, correctly, with exit 101 because the
tree changed during the run. The `eight-home` corpus and the
`fluent-bit-tail-pause` slice were **not** measured: their prepared inputs
(`build/cycle19`, the tail-pause export of upstream `90978ac6`) and published
baselines do not exist on this host. The prediction's `none` for those routes
rests on construction: with `QCORE_LOOP_MEASURE_PROPERTY` unset the new code
returns before doing anything, and no custody, contract or provider code
changed. The integrator's arbiter on a provisioned host is owed as before
(`W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`).

A direct comparison of the selection runs agrees
([scripts/statuses.py](scripts/statuses.py)): the base run with rule 44 alone,
the tip with an empty premise file and the tip with the Fluent Bit premises
publish the same 448 function statuses, and rule 44's tally is identical.

## 6. Validation

The scoped card gate, `python3 scripts/gate_scope.py --base 591e5523f`, on
`438fef384` through the host toolchain script (`private/toolchain/env.sh`,
copied verbatim into the worktree's `private/w45-list/env.sh` because the
worktree guard refuses a command naming the primary checkout's path; it points
at the same system Clang 21.1.8 and Lean 4.35.0), mode full (the diff reaches
`qcore-c-gateway`): **8 steps passed, exit 0** ([gate.json](gate.json),
private run `private/gate-scope/20261113T151231Z-1981602/`).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 631 units, 673 summaries; 9,450 passed, 0 failed, 17 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`, all) | 0 | 430 audits |
| Python | 0 | 761 tests |

No provider step: `providers/` is untouched and the premise file is no
provider contract. The commits after `438fef384` add only this report's
evidence files (README, arbiter record, gate record).

## 7. Follow-ups

* `W45-FB-LIST-FINITENESS-PREMISE` is retired: the declared premise, its
  frame check, the bound fact and Lean parity are this card's, and the
  measured statement for the 70 unbounded loops is §4.
* `W45-FB-LIST-UNLINK-CURRENT-FRAME` (new): admit a frame whose only link
  write is the unlink of the current node of a `_safe` traversal (PL's second
  clause), the reason for most of the 34 `frame-writes-links` loops.
* `W45-FB-LIST-HEAD-MEMBER-CHAIN` (new): admit a head reached through a
  member chain whose intermediate pointers the frame writes nowhere (9 loops).
* `W45-FB-LIST-EXTERNAL-CALLEE-FRAMES` (new): a checked frame for callees in
  other units (the logger, `flb_chunk_route_plugin_matches`, ChunkIO) — read
  from their defining units, never declared (21 loops).
* Unchanged: `W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN` (the three live
  profiles' arbiter), `W45-FB-MEASURE-ESCAPED-BOUND-PREMISE`,
  `W45-FB-MEASURE-SHIFT-PRECONDITION`.
