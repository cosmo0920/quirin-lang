"""W45-FB-ENGINE-PROGRESS: the hand classification of every slice loop.

Source reading at Fluent Bit 046accf6aded7906f66a1baf0202da349e521736. This
module is data. `census.py` joins it to the mechanical `loop_inventory.py`
output and refuses to publish if a loop is unclassified or a row names a loop
the inventory does not contain.

A row is keyed by (file, function, loop line). Fields:

role     where the loop runs at runtime (see ROLES)
entry    the engine event or caller chain that reaches it
cls      bounded | decreasing | yielding | unresolved (see CLASSES)
bound    the stated quantity (bounded), or the measure (decreasing)
premises premise ids (see PREMISES) the class depends on
progress the progress variable(s) the loop changes each iteration
blocking calls in the loop that can block the engine thread (I/O, locks)
retry    retry scheduling done in the loop, if any
express  how the bound could be stated for Quirin (see EXPRESS)
note     anything else the classification rests on
"""

ROLES = {
    "engine-loop": "the engine event loop itself (flb_engine_start)",
    "callback": "runs on the engine thread from a registered event: flush timer, collector, scheduler timer or request, output or input event",
    "ingestion": "runs inside an input's append path (called from input collectors on the engine thread, or from the ring-buffer collector)",
    "shutdown": "runs from the engine's STOP / grace-timer handling",
    "lifecycle": "start-up, configuration or exit; runs once per start or stop",
    "no-caller": "no caller found in src/ or plugins/ at the pin",
}

CLASSES = {
    "bounded": "iterations bounded by a stated quantity fixed at loop entry, under the listed premises",
    "decreasing": "a stated measure decreases strictly each iteration and is bounded below, under the listed premises",
    "yielding": "a non-terminating service loop that blocks (yields) each round",
    "unresolved": "no bound or measure holds without a premise that is not established (and, where marked, is violated under an injected fault)",
}

PREMISES = {
    "PL": "the traversed list is a finite, well-formed mk_list/cfl_list and the body (callees included) does not insert into it; a _safe traversal removes at most the current node",
    "PC": "the configuration's input and output instance lists are not modified while the engine runs",
    "PF": "a callee in the body does not free the saved successor of a _safe traversal (P-ILI-CALLEE-FRAME)",
    "PW": "at most one backlog wrapper per chunk per backlog queue (OB-3 of W45-FB-BACKLOG-CUSTODY)",
    "PR": "no producer thread writes the ring buffer while it is drained",
    "PU": "cio_chunk_up_force returns neither CIO_ERROR nor CIO_RETRY for a backlog queue head",
    "PE": "the exponent is non-negative (retry attempts >= 0)",
    "PRAND": "1 <= range <= RAND_MAX and rand() returns every value in [0, RAND_MAX] with positive probability",
    "PK": "the macro constants hold their pinned values: FLB_ENGINE_LOOP_MAX_ITER = 10 (flb_engine_macros.h:61), FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE = 1 (flb_engine.h:29)",
    "PN": "the loop's counter is not written in the body and its bound is not written in the body",
}

EXPRESS = {
    "measure-const": "a checked measure: counter against a literal or macro constant, counter not written in the body",
    "measure-var": "a checked measure: counter against a local, parameter or field not written in the body",
    "measure-u16": "a checked measure: counter against a uint16_t count read from chunk metadata (<= 65535)",
    "measure-shift": "a checked measure: a non-negative integer strictly decreased by a right shift; needs PE as a precondition contract",
    "list-contract": "a bound over a list length: needs a contract premise that the list is finite and not extended by the body's callees (PL); not derivable locally",
    "none": "no measure exists without an unestablished premise",
    "n/a": "a service loop; a bound is not the property",
    "once": "a do { } while (0) macro body; executes exactly once",
}

L = "list-contract"

# (file, function, line): dict(...)
ROWS = [
    # ---------------- plugins/in_storage_backlog/sb.c
    ("plugins/in_storage_backlog/sb.c", "sb_destroy_backlog", 142, dict(
        role="lifecycle", entry="cb_sb_exit -> sb_destroy_backlogs", cls="bounded",
        bound="|backlog->chunks| at entry", premises=["PL"], progress="list cursor",
        express=L,
        note="sb_destroy_backlogs unlinks the backlog from context->backlogs (sb.c:188) before this call, so sb_remove_chunk_from_segregated_backlogs (which walks context->backlogs) does not remove this queue's wrappers: the traversal is over an unchanged list. Source-reading candidate for the custody card (this queue's wrappers are not freed at exit); not a progress question.")),
    ("plugins/in_storage_backlog/sb.c", "sb_allocate_backlogs", 155, dict(
        role="lifecycle", entry="sb_segregate_chunks (engine start, flb_engine.c:1434; shutdown flush, flb_engine.c:1547)",
        cls="bounded", bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L,
        note="inserts into context->backlogs, not into the traversed config->outputs")),
    ("plugins/in_storage_backlog/sb.c", "sb_destroy_backlogs", 185, dict(
        role="lifecycle", entry="cb_sb_exit; sb_allocate_backlogs failure arm", cls="bounded",
        bound="|context->backlogs| at entry", premises=["PL"], progress="list cursor (current node unlinked, sb.c:188)", express=L)),
    ("plugins/in_storage_backlog/sb.c", "sb_find_segregated_backlog_by_output_plugin_instance", 203, dict(
        role="ingestion", entry="sb_release_output_queue_space <- flb_input_chunk_release_space_compound", cls="bounded",
        bound="|context->backlogs| (= |config->outputs| after sb_allocate_backlogs)", premises=["PL"], progress="list cursor; returns on match", express=L)),
    ("plugins/in_storage_backlog/sb.c", "sb_remove_chunk_from_segregated_backlog", 222, dict(
        role="callback", entry="collector cb_queue_chunks -> sb_remove_chunk_from_segregated_backlogs; ingestion eviction", cls="bounded",
        bound="|backlog->chunks| at entry; breaks after the first match", premises=["PL"], progress="list cursor", express=L)),
    ("plugins/in_storage_backlog/sb.c", "sb_remove_chunk_from_segregated_backlogs", 255, dict(
        role="callback", entry="collector cb_queue_chunks; ingestion eviction; exit", cls="bounded",
        bound="|context->backlogs|", premises=["PL"], progress="list cursor", express=L,
        note="the body removes wrappers from each queue, never a queue from context->backlogs")),
    ("plugins/in_storage_backlog/sb.c", "sb_append_chunk_to_segregated_backlogs", 341, dict(
        role="lifecycle", entry="sb_segregate_chunks", cls="bounded", bound="|context->backlogs|",
        premises=["PL"], progress="list cursor", express=L,
        note="returns -3 on the first failed insertion (the W45-FB-BACKLOG-FAULT-INJECTION partial fan-out premise); not a progress question")),
    ("plugins/in_storage_backlog/sb.c", "sb_segregate_chunks", 399, dict(
        role="lifecycle", entry="engine start (flb_engine.c:1434); shutdown flush when storage.backlog.flush_on_shutdown (flb_engine.c:1547)", cls="bounded",
        bound="|cio->streams|", premises=["PL"], progress="list cursor", express=L)),
    ("plugins/in_storage_backlog/sb.c", "sb_segregate_chunks", 408, dict(
        role="lifecycle", entry="as above", cls="bounded", bound="|stream->chunks| at entry",
        premises=["PL", "PF"], progress="list cursor (cio_chunk_close removes the current chunk)",
        blocking=["cio_chunk_up_force (open/fstat/mmap)", "cio_chunk_down (munmap/close)", "cio_chunk_close (unlink when deleting)"],
        express=L,
        note="a CIO_ERROR from cio_chunk_up_force here is not retried: the chunk stays down and `!cio_chunk_is_up` returns -3 (sb.c:431-433), ending segregation")),
    ("plugins/in_storage_backlog/sb.c", "sb_get_releasable_output_queue_space", 512, dict(
        role="no-caller", entry="none found", cls="bounded", bound="|backlog->chunks|; breaks when the running sum reaches required_space",
        premises=["PL"], progress="list cursor", express=L)),
    ("plugins/in_storage_backlog/sb.c", "sb_release_output_queue_space", 551, dict(
        role="ingestion", entry="flb_input_chunk_release_space_compound <- flb_input_chunk_find_space_new_data", cls="bounded",
        bound="|backlog->chunks| at entry; breaks when released_space >= *required_space",
        premises=["PL", "PW", "PF"], progress="list cursor; released_space (monotone)",
        blocking=["cio_chunk_close (unlink)"], express=L,
        note="sb_remove_chunk_from_segregated_backlogs frees the current wrapper in every queue; the saved successor survives only if it refers to another chunk (PW)")),
    ("plugins/in_storage_backlog/sb.c", "cb_queue_chunks", 611, dict(
        role="callback", entry="storage_backlog collector (flb_input_set_collector_time, 1 s; sb.c:769) on the engine thread",
        cls="unresolved", bound="none without PU",
        premises=["PU", "PL"],
        progress="total (grows by size > 0 on a queued chunk, sb.c:716); sum of queue lengths (decreases on every arm that calls sb_remove_chunk_from_segregated_backlogs); empty_output_queue_count (recomputed from 0 each pass, sb.c:613)",
        blocking=["cio_chunk_up_force (open/fstat/mmap)", "cio_chunk_down (munmap/close)", "cio_chunk_close", "flb_input_chunk_map"],
        yield_points=["sb.c:606 (return before the loop when total >= mem_limit)", "sb.c:724 (after the loop)"],
        express="none",
        decreasing_under="Under PU, every pass over a non-empty queue set removes at least one wrapper or ends with every queue empty, so the measure (sum of queue lengths) bounds the outer iterations by that sum + 1; total < mem_limit adds a second exit.",
        note="The CIO_ERROR / CIO_RETRY arm (sb.c:649-650) `continue`s without changing total, any queue or empty_output_queue_count, so a queue whose head keeps failing keeps the loop condition (sb.c:611-612) true. Observed natively: E1 (persistent CIO_ERROR) and E2 (ONE injected fstat failure, which leaves the descriptor open so every later up_force of that chunk returns CIO_ERROR at cio_file.c:827-830). Controls C1 and C2 yield. Top-ranked zero-progress candidate.")),
    ("plugins/in_storage_backlog/sb.c", "cb_queue_chunks", 615, dict(
        role="callback", entry="as above", cls="bounded", bound="|ctx->backlogs| per outer pass",
        premises=["PL"], progress="list cursor",
        note="the body removes wrappers from queues, never a queue from ctx->backlogs", express=L)),

    # ---------------- src/flb_input_chunk.c
    ("src/flb_input_chunk.c", "flb_input_chunk_release_space", 568, dict(
        role="ingestion", entry="flb_input_chunk_release_space_compound", cls="bounded",
        bound="pass_limit <= 2 (a literal: 1, or 2 for FLB_INPUT_CHUNK_RELEASE_SCOPE_LOCAL)", premises=["PN"],
        progress="pass", express="measure-const")),
    ("src/flb_input_chunk.c", "flb_input_chunk_release_space", 569, dict(
        role="ingestion", entry="as above", cls="bounded", bound="|input_plugin->chunks| at entry, per pass; breaks when released_space >= *required_space",
        premises=["PL", "PF"], progress="list cursor; released_space",
        blocking=["flb_input_chunk_destroy / flb_task_destroy (file close/unlink)"], express=L,
        note="the body destroys the current chunk or its task (task->ic is the current chunk); the saved successor must survive (PF)")),
    ("src/flb_input_chunk.c", "flb_input_chunk_has_other_routes", 947, dict(
        role="ingestion", entry="flb_input_chunk_prefers_physical_delete <- flb_input_chunk_release_space", cls="bounded",
        bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_input_chunk.c", "flb_input_chunk_release_space_compound", 1143, dict(
        role="ingestion", entry="flb_input_chunk_find_space_new_data", cls="bounded",
        bound="|config->inputs|; breaks when the remainder reaches 0", premises=["PC"], progress="list cursor; required_space_remainder",
        express=L,
        note="each iteration runs flb_input_chunk_release_space over another input's chunks: the work per ingestion is bounded by |outputs| x |inputs| x 2 x max|chunks|, with no batch limit")),
    ("src/flb_input_chunk.c", "flb_input_chunk_find_space_new_data", 1223, dict(
        role="ingestion", entry="flb_input_chunk_place_new_chunk", cls="bounded", bound="|config->outputs|",
        premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_input_chunk.c", "flb_input_chunk_has_overlimit_routes", 1270, dict(
        role="ingestion", entry="flb_input_chunk_place_new_chunk", cls="bounded", bound="|config->outputs|",
        premises=["PC"], progress="list cursor", express=L)),
] + [
    ("src/flb_input_chunk.c", "input_chunk_collect_output_references", line, dict(
        role="callback", entry="flb_input_chunk_map <- collector cb_queue_chunks (backlog restore)", cls="bounded",
        bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L))
    for line in (1354, 1366, 1388, 1409, 1421, 1439)
] + [
    ("src/flb_input_chunk.c", "flb_input_chunk_map", 1666, dict(
        role="callback", entry="collector cb_queue_chunks (sb.c:690)", cls="bounded",
        bound="direct_count <= 65535 (uint16_t stored count, flb_input_chunk.c:2256-2261)", premises=["PN"],
        progress="direct_index", express="measure-u16")),
    ("src/flb_input_chunk.c", "flb_input_chunk_map", 1702, dict(
        role="callback", entry="as above", cls="bounded", bound="direct_count <= 65535", premises=["PN"],
        progress="direct_index", express="measure-u16")),
    ("src/flb_input_chunk.c", "flb_input_chunk_map", 1716, dict(
        role="callback", entry="as above", cls="bounded", bound="direct_match_count <= |config->outputs|",
        premises=["PN", "PC"], progress="direct_match_index", express="measure-var")),
] + [
    ("src/flb_input_chunk.c", "flb_input_chunk_write_header_v2", line, dict(
        role="ingestion", entry="flb_input_log.c:297 and :1002 (log append with direct routes)", cls="bounded",
        bound="route_count (int parameter)", premises=["PN"], progress="index", express="measure-var"))
    for line in (1943, 2086, 2103, 2116, 2128, 2136)
] + [
    ("src/flb_input_chunk.c", "flb_input_chunk_get_direct_routes", line, dict(
        role="callback", entry="flb_task_create (flush timer) and flb_input_chunk_map (collector)", cls="bounded",
        bound="stored_count <= 65535 (uint16_t read from chunk metadata)", premises=["PN"], progress="index",
        express="measure-u16"))
    for line in (2286, 2329, 2369, 2430, 2450, 2483)
] + [
    ("src/flb_input_chunk.c", "flb_input_chunk_destroy_direct_routes", 2506, dict(
        role="callback", entry="flb_task_create; flb_input_chunk_map; get_direct_routes failure arms", cls="bounded",
        bound="route_count (int parameter)", premises=["PN"], progress="index", express="measure-var")),
    ("src/flb_input_chunk.c", "flb_input_chunk_destroy_corrupted", 2643, dict(
        role="ingestion", entry="input_chunk_get (flb_input_chunk.c:2878)", cls="bounded", bound="|config->outputs|",
        premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_input_chunk.c", "flb_input_chunk_destroy", 2729, dict(
        role="callback", entry="flb_task_destroy (output event, dispatch cleanup); release-space eviction", cls="bounded",
        bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_input_chunk.c", "memrb_input_chunk_release_space", 3204, dict(
        role="ingestion", entry="flb_input_chunk.c:3289 (memory ring-buffer mode)", cls="bounded",
        bound="|ins->chunks| at entry; breaks when released_space >= required_space", premises=["PL", "PF"],
        progress="list cursor; released_space", express=L)),
    ("src/flb_input_chunk.c", "flb_input_chunk_ring_buffer_cleanup", 3726, dict(
        role="lifecycle", entry="flb_input.c:1344 (input instance destroy)", cls="decreasing",
        bound="entries in the ring buffer (each successful read removes one)", premises=["PR"],
        progress="ring buffer read position", express="none",
        note="decreasing only while no producer writes (PR); holds when the input's thread has stopped before cleanup")),
    ("src/flb_input_chunk.c", "flb_input_chunk_ring_buffer_collector", 3742, dict(
        role="callback", entry="permanent scheduler timer (flb_engine.c:1419-1422) and the engine loop's ring-buffer flag (flb_engine.c:1640-1642)",
        cls="bounded", bound="|config->inputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_input_chunk.c", "flb_input_chunk_ring_buffer_collector", 3746, dict(
        role="callback", entry="as above", cls="unresolved",
        bound="none without PR: exits only when the ring buffer is empty or the input is paused",
        premises=["PR"], progress="ring buffer read position (one entry per iteration)",
        blocking=["input_chunk_append_raw / flb_input_log_append_processed (chunk writes, release-space eviction)"],
        express="none",
        decreasing_under="Under PR the entries in the ring buffer decrease by one per iteration, so the drain is bounded by the ring buffer's entry capacity.",
        note="threaded inputs write the ring buffer concurrently; there is no per-call batch limit, so a producer that keeps pace with the drain keeps this loop running on the engine thread (a workload premise, not a failure outcome). Not exercised natively by this card.")),
    ("src/flb_input_chunk.c", "flb_input_chunk_update_output_instances", 3994, dict(
        role="ingestion", entry="chunk append / size accounting", cls="bounded", bound="|config->outputs|",
        premises=["PC"], progress="list cursor", express=L)),

    # ---------------- src/flb_storage.c
    ("src/flb_storage.c", "metrics_append_input", 170, dict(
        role="callback", entry="storage metrics timer cb_storage_metrics_collect (5 s, flb_storage.c:382-384)", cls="bounded",
        bound="|config->inputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_storage.c", "metrics_append_input", 283, dict(
        role="callback", entry="as above", cls="bounded", bound="|i->chunks|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_storage.c", "flb_storage_input_destroy", 632, dict(
        role="lifecycle", entry="input instance destroy", cls="bounded", bound="|in->chunks| at entry",
        premises=["PL", "PF"], progress="list cursor (flb_input_chunk_destroy unlinks the current chunk)", express=L)),
    ("src/flb_storage.c", "storage_contexts_create", 649, dict(
        role="lifecycle", entry="flb_storage_create", cls="bounded", bound="|config->inputs|", premises=["PC"],
        progress="list cursor", express=L)),
    ("src/flb_storage.c", "sanitize_name_component", 794, dict(
        role="callback", entry="flb_storage_quarantine_chunk <- handle_dlq_if_available (output event)", cls="bounded",
        bound="min(out_sz - 1, strlen(in))", premises=["PN"], progress="i", express="measure-var")),
    ("src/flb_storage.c", "dlq_stream_total_size", 854, dict(
        role="callback", entry="flb_storage_quarantine_chunk (output event)", cls="bounded", bound="|st->chunks|",
        premises=["PL"], progress="list cursor", express=L)),

    # ---------------- src/flb_task.c
    ("src/flb_task.c", "map_get_task_id", 53, dict(
        role="callback", entry="flb_engine_dispatch (flush timer, per chunk) and task_alloc", cls="bounded",
        bound="config->task_map_size", premises=["PN"], progress="i", express="measure-var",
        note="called once per candidate chunk in flb_engine_dispatch, so a flush scans up to |in->chunks| x task_map_size entries")),
] + [
    ("src/flb_task.c", "task_collect_output_references", line, dict(
        role="callback", entry="flb_task_create <- flb_engine_dispatch (flush timer)", cls="bounded",
        bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L))
    for line in (169, 181, 203, 224, 236, 254)
] + [
    ("src/flb_task.c", "flb_task_retry_create", 325, dict(
        role="callback", entry="handle_output_event FLB_RETRY arm (flb_engine.c:746)", cls="bounded",
        bound="|task->retries| (one retry per output instance)", premises=["PL"], progress="list cursor", express=L,
        retry="spends an attempt (retry->attempts++, flb_task.c:356) or ends the retry at retry_limit (flb_task.c:328-333)")),
    ("src/flb_task.c", "flb_task_retry_count", 407, dict(
        role="callback", entry="handle_output_event FLB_OK arm", cls="bounded", bound="|task->retries|",
        premises=["PL"], progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_retry_clean", 426, dict(
        role="callback", entry="handle_output_event (all arms)", cls="bounded", bound="|task->retries|",
        premises=["PL"], progress="list cursor; returns after the first destroy", express=L)),
    ("src/flb_task.c", "flb_task_running_count", 485, dict(
        role="shutdown", entry="grace timer (flb_engine.c:1526)", cls="bounded", bound="|config->inputs|",
        premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_running_count", 487, dict(
        role="shutdown", entry="as above", cls="bounded", bound="|ins->tasks|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_running_print", 516, dict(
        role="shutdown", entry="grace timer (flb_engine.c:1535)", cls="bounded", bound="|config->inputs|",
        premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_running_print", 521, dict(
        role="shutdown", entry="as above", cls="bounded", bound="|ins->tasks|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_running_print", 524, dict(
        role="shutdown", entry="as above", cls="bounded", bound="|task->routes|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_create", 652, dict(
        role="callback", entry="flb_engine_dispatch (flush timer)", cls="bounded",
        bound="direct_output_count <= 65535 (uint16_t stored count)", premises=["PN"],
        progress="direct_output_index", express="measure-u16")),
    ("src/flb_task.c", "flb_task_create", 688, dict(
        role="callback", entry="as above", cls="bounded", bound="direct_output_count <= 65535", premises=["PN"],
        progress="direct_output_index", express="measure-u16")),
    ("src/flb_task.c", "flb_task_create", 704, dict(
        role="callback", entry="as above", cls="bounded", bound="stored_match_count <= |config->outputs|",
        premises=["PN", "PC"], progress="stored_match_index", express="measure-var")),
    ("src/flb_task.c", "flb_task_create", 786, dict(
        role="callback", entry="as above", cls="bounded", bound="|i_ins->routes_direct|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_create", 848, dict(
        role="callback", entry="as above", cls="bounded", bound="|config->outputs|", premises=["PC"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_destroy", 912, dict(
        role="callback", entry="output event (flb_task_users_dec), dispatch cleanup, release-space eviction", cls="bounded",
        bound="|task->routes| at entry", premises=["PL"], progress="list cursor (current route unlinked and freed)",
        express=L)),
    ("src/flb_task.c", "flb_task_destroy", 933, dict(
        role="callback", entry="as above", cls="bounded", bound="|task->retries| at entry", premises=["PL", "PF"],
        progress="list cursor (flb_task_retry_destroy unlinks the current retry)", express=L)),
    ("src/flb_task.c", "flb_task_queue_destroy", 967, dict(
        role="lifecycle", entry="output instance destroy", cls="bounded", bound="|queue->pending|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_task.c", "flb_task_queue_destroy", 973, dict(
        role="lifecycle", entry="output instance destroy", cls="bounded", bound="|queue->in_progress|", premises=["PL"],
        progress="list cursor", express=L)),

    # ---------------- src/flb_engine.c
    ("src/flb_engine.c", "engine_has_fluentbit_logs_input", 100, dict(
        role="lifecycle", entry="engine start", cls="bounded", bound="|config->inputs|", premises=["PC"],
        progress="list cursor", express=L)),
    ("src/flb_engine.c", "flb_engine_evl_init_private", 115, dict(
        role="lifecycle", entry="pthread_once", cls="bounded", bound="1 (FLB_TLS_INIT expands to do { } while (0))",
        premises=[], progress="none needed", express="once")),
    ("src/flb_engine.c", "flb_engine_destroy_tasks", 143, dict(
        role="lifecycle", entry="engine/input teardown", cls="bounded", bound="|tasks| at entry", premises=["PL", "PF"],
        progress="list cursor", express=L)),
    ("src/flb_engine.c", "flb_engine_reschedule_retries", 165, dict(
        role="shutdown", entry="engine STOP (flb_engine.c:1473)", cls="bounded", bound="|config->inputs|",
        premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_engine.c", "flb_engine_reschedule_retries", 167, dict(
        role="shutdown", entry="as above", cls="bounded", bound="|ins->tasks|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_engine.c", "flb_engine_reschedule_retries", 178, dict(
        role="shutdown", entry="as above", cls="bounded", bound="|task->retries|", premises=["PL"],
        progress="list cursor", express=L,
        retry="invalidates each retry's request and schedules it now (flb_sched_retry_now, 0 s); adds requests, not retries")),
    ("src/flb_engine.c", "flb_engine_flush", 202, dict(
        role="callback", entry="flush timer (flb_engine.c:984-988); STOP; grace timer", cls="bounded",
        bound="|config->inputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_engine.c", "flb_engine_get_chunk_backpressure_percent", 290, dict(
        role="callback", entry="flush timer -> flb_engine_adaptive_flush_update", cls="bounded",
        bound="|config->outputs|", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_engine.c", "handle_output_events", 909, dict(
        role="callback", entry="FLB_ENGINE_EV_OUTPUT event (flb_engine.c:1613-1618)", cls="bounded",
        bound="min(bytes read / 8, FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE) = at most 1", premises=["PK", "PN"],
        progress="index", blocking=["flb_pipe_r before the loop (the fd was reported readable)"],
        express="measure-const")),
    ("src/flb_engine.c", "flb_engine_start", 1450, dict(
        role="engine-loop", entry="flb_engine_start", cls="yielding",
        bound="none: the service loop; it leaves only by return (shutdown complete, flb_engine.c:1568; shutdown timer failure :1506; segregation failure :1550)",
        premises=[], progress="n/a",
        blocking=["mk_event_wait (flb_engine.c:1453): the yield point", "end-of-round cleanups"],
        yield_points=["mk_event_wait each round"], express="n/a",
        note="every engine callback in this census runs inside this loop's inner batch; a callback that does not return stops the loop (observed in E1/E2: 0 flush-timer dispatches)")),
    ("src/flb_engine.c", "flb_engine_start", 1454, dict(
        role="engine-loop", entry="flb_event_priority_live_foreach (flb_event_loop.h:74-100)", cls="bounded",
        bound="FLB_ENGINE_LOOP_MAX_ITER = 10 events per round", premises=["PK"],
        progress="__flb_event_priority_live_foreach_iter", blocking=["mk_event_wait_2(evl, 0) per iteration (non-blocking poll)"],
        express="measure-const",
        note="the bound counts events, not work: each event's handler runs to completion inside the iteration")),

    # ---------------- src/flb_engine_dispatch.c
    ("src/flb_engine_dispatch.c", "tasks_start", 254, dict(
        role="callback", entry="flb_engine_dispatch (flush timer)", cls="bounded",
        bound="|in->tasks| at entry; breaks when flush_in_flight >= flush_in_flight_limit", premises=["PL", "PF"],
        progress="list cursor", express=L,
        retry="a NEW task deferred by the in-flight bound keeps FLB_TASK_NEW and is revisited next flush (flb_engine_dispatch.c:266-278)")),
    ("src/flb_engine_dispatch.c", "tasks_start", 282, dict(
        role="callback", entry="as above", cls="bounded", bound="|task->routes| at entry", premises=["PL", "PF"],
        progress="list cursor (test-mode routes unlinked)", blocking=["flb_output_task_flush (pipe write)"], express=L)),
    ("src/flb_engine_dispatch.c", "flb_engine_dispatch", 381, dict(
        role="callback", entry="flb_engine_flush (flush timer)", cls="bounded",
        bound="|in->chunks| at entry; breaks when no task id is free", premises=["PL", "PF"],
        progress="list cursor", blocking=["flb_input_chunk_flush (chunk up: file I/O)", "flb_input_chunk_down"],
        express=L,
        note="a chunk whose flush yields no buffer or no tag is skipped with `continue` (flb_engine_dispatch.c:397-421) and revisited on every later flush: cross-invocation revisit row V3")),
    ("src/flb_engine_dispatch.c", "flb_engine_dispatch", 456, dict(
        role="callback", entry="as above", cls="bounded", bound="|in->tasks| at entry", premises=["PL", "PF"],
        progress="list cursor", express=L)),

    # ---------------- src/flb_scheduler.c
    ("src/flb_scheduler.c", "flb_sched_ctx_init", 38, dict(
        role="lifecycle", entry="scheduler init", cls="bounded", bound="1 (do { } while (0) macro)", premises=[],
        progress="none needed", express="once")),
    ("src/flb_scheduler.c", "flb_sched_ctx_init", 39, dict(
        role="lifecycle", entry="scheduler init", cls="bounded", bound="1 (do { } while (0) macro)", premises=[],
        progress="none needed", express="once")),
    ("src/flb_scheduler.c", "random_uniform", 106, dict(
        role="callback", entry="flb_sched_request_create <- handle_output_event retry arm; flb_task_retry_reschedule <- flb_engine_dispatch_retry",
        cls="unresolved", bound="none deterministic: rejection sampling",
        premises=["PRAND"], progress="none (a fresh rand() draw each iteration)",
        blocking=["flb_random_bytes before the loop (random device)"], express="none",
        decreasing_under="no measure; under PRAND each iteration exits with probability limit / (RAND_MAX + 1) >= 1/2, so the loop terminates with probability 1 and has geometric expected length",
        retry="computes the backoff delay (backoff_full_jitter)",
        note="range = min(cap, base * 2^n) - base + 1 depends on scheduler.cap and scheduler.base; range = 0 (cap = base - 1) divides by zero at :102 before the loop, and copies = 0 (limit = 0, condition always true) needs range > RAND_MAX, which int arithmetic reaches only by overflow. A grep of src/ found no ordering check between the two settings (source reading only)")),
    ("src/flb_scheduler.c", "schedule_request_promote", 183, dict(
        role="callback", entry="scheduler frame timer (flb_scheduler.c:606-609)", cls="bounded",
        bound="|sched->requests_wait| at entry", premises=["PL"],
        progress="list cursor (a promoted request is moved to another list)", express=L,
        retry="promotes waiting retry requests whose time has come; a request that cannot be scheduled is re-added to requests_wait (row V4)")),
    ("src/flb_scheduler.c", "schedule_request_promote", 224, dict(
        role="callback", entry="as above", cls="bounded", bound="|failed_requests|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "ipow", 237, dict(
        role="callback", entry="backoff_full_jitter <- flb_sched_request_create", cls="decreasing",
        bound="exp (non-negative) strictly decreases by `exp >>= 1`; at most 32 iterations for a 32-bit int",
        premises=["PE"], progress="exp", express="measure-shift",
        note="for a negative exp an arithmetic right shift stops at -1 and the loop does not end; the callers pass retry->attempts (>= 1 in the paths read)")),
    ("src/flb_scheduler.c", "flb_sched_request_invalidate", 362, dict(
        role="callback", entry="flb_task_retry_destroy (output event, retry limit); flb_engine_reschedule_retries", cls="bounded",
        bound="|sched->requests|; returns after the first match", premises=["PL"], progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "flb_sched_request_invalidate", 379, dict(
        role="callback", entry="as above", cls="bounded", bound="|sched->requests_wait|; returns after the first match",
        premises=["PL"], progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "sched_timer_coro_get_id", 422, dict(
        role="callback", entry="flb_sched_timer_coro_create <- timer_cb_coro_trampoline (permanent coroutine timers)", cls="bounded",
        bound="|timer_coro_list| + 1 outer iterations (only |list| ids can collide)", premises=["PL"],
        progress="id", express=L, note="also `id < UINT32_MAX`")),
    ("src/flb_scheduler.c", "sched_timer_coro_get_id", 424, dict(
        role="callback", entry="as above", cls="bounded", bound="|timer_coro_list|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "sched_timer_coro_get_by_id", 452, dict(
        role="callback", entry="FLB_ENGINE_EV_SCHED_CORO event (flb_scheduler.c:568)", cls="bounded",
        bound="|timer_coro_list|", premises=["PL"], progress="list cursor", express=L)),
] + [
    ("src/flb_scheduler.c", "flb_sched_destroy", line, dict(
        role="lifecycle", entry="engine shutdown", cls="bounded", bound=b, premises=["PL", "PF"],
        progress="list cursor", express=L))
    for line, b in ((827, "|sched->requests|"), (834, "|sched->requests_wait|"), (841, "|sched->timers|"), (849, "|sched->timers_drop|"))
] + [
    ("src/flb_scheduler.c", "flb_sched_timer_cleanup", 924, dict(
        role="engine-loop", entry="end of every engine round (flb_engine.c:1647)", cls="bounded",
        bound="|sched->timers_drop| at entry", premises=["PL"], progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "flb_sched_timer_coro_cleanup", 943, dict(
        role="engine-loop", entry="flb_sched_timer_cleanup", cls="bounded", bound="|sched->timer_coro_list_drop| at entry",
        premises=["PL"], progress="list cursor", express=L)),
    ("src/flb_scheduler.c", "flb_sched_timer_coro_get", 1002, dict(
        role="no-caller", entry="none found", cls="bounded", bound="|timer_coro_list|", premises=["PL"],
        progress="list cursor", express=L)),

    # ---------------- src/flb_output.c
    ("src/flb_output.c", "flb_output_prepare", 124, dict(
        role="lifecycle", entry="output setup", cls="bounded", bound="1 (do { } while (0) macro)", premises=[],
        progress="none needed", express="once")),
    ("src/flb_output.c", "flb_output_pre_run", 158, dict(
        role="lifecycle", entry="engine start", cls="bounded", bound="|config->outputs|", premises=["PC"],
        progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_exit", 652, dict(
        role="lifecycle", entry="engine shutdown", cls="bounded", bound="|config->outputs| at entry",
        premises=["PL", "PF"], progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_get_instance", 700, dict(
        role="callback", entry="handle_output_event; flb_output_flush_finished", cls="bounded",
        bound="|config->outputs|; breaks on match", premises=["PC"], progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_flush_finished", 742, dict(
        role="callback", entry="handle_output_event (non-threaded outputs)", cls="bounded",
        bound="|flush_list_destroy| at entry", premises=["PL", "PF"],
        progress="list cursor (flb_output_flush_destroy unlinks the current flush)", express=L)),
    ("src/flb_output.c", "flb_output_new", 776, dict(
        role="lifecycle", entry="configuration", cls="bounded", bound="|config->out_plugins|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_new", 802, dict(
        role="lifecycle", entry="configuration", cls="bounded", bound="|config->out_plugins|", premises=["PL"],
        progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_plugin_property_check", 1445, dict(
        role="lifecycle", entry="configuration", cls="bounded", bound="|ins->properties| at entry", premises=["PL", "PF"],
        progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_init_all", 1537, dict(
        role="lifecycle", entry="engine start", cls="bounded", bound="|config->outputs| at entry", premises=["PL", "PF"],
        progress="list cursor", express=L)),
    ("src/flb_output.c", "flb_output_upstream_ha_set", 2082, dict(
        role="lifecycle", entry="configuration", cls="bounded", bound="|ha->nodes|", premises=["PL"],
        progress="list cursor", express=L)),
]

# The one recursion cycle `loop_inventory.py` finds in the slice's direct call graph.
RECURSION = [
    dict(
        id="EP-R1",
        functions=["flb_output_task_flush", "flb_output_task_queue_flush_one", "flb_output_task_singleplex_flush_next"],
        file="src/flb_output.c",
        role="callback",
        entry="handle_output_event (synchronous outputs, flb_engine.c:634-637); tasks_start / flb_engine_dispatch_retry singleplex enqueue",
        cls="decreasing",
        bound="|queue->pending|: every flush_one moves the pending head to in_progress (flb_output.c:324-326) and every flush_next removes the in_progress head (flb_output.c:398-404), so the recursion re-enters only while pending is non-empty",
        premises=["PL"],
        express="list-contract",
        note="the recursion continues only on a failed flush (-1); its depth is bounded by the pending queue length at entry"),
]

# Cross-invocation revisits: the callback returns (yields), but the next
# invocation meets the same state. These answer the second half of the
# card's question and are not loops.
REVISITS = [
    dict(
        id="EP-V1", site="src/flb_engine_dispatch.c:114-131", function="flb_engine_dispatch_retry",
        trigger="flb_input_chunk_set_up(task->ic) returns -1 (cio_chunk_up fails, e.g. storage.max_chunks_up reached: cio_file.c:837-841)",
        effect="flb_task_retry_reschedule with the same retry->attempts; no attempt is spent, so retry_limit does not end the retry",
        cls="unresolved", premise="the chunk can never be brought up", observed="E3 (control C3)",
        express="a contract on the retry route: 'every reschedule spends an attempt or the premise is released' is not true at the pin"),
    dict(
        id="EP-V2", site="src/flb_engine_dispatch.c:101-111", function="flb_engine_dispatch_retry",
        trigger="config->flush_in_flight >= config->flush_in_flight_limit",
        effect="rescheduled without spending an attempt; the same bound defers NEW tasks in tasks_start (flb_engine_dispatch.c:273-278)",
        cls="unresolved", premise="in-flight flushes keep returning (handle_output_event decrements flush_in_flight, flb_engine.c:595-597)",
        observed="not exercised"),
    dict(
        id="EP-V3", site="src/flb_engine_dispatch.c:396-421", function="flb_engine_dispatch",
        trigger="flb_input_chunk_flush returns no buffer, or the chunk has no valid tag",
        effect="the chunk is skipped with `continue` and met again on every later flush; nothing removes it on this route",
        cls="unresolved", premise="the chunk's flush failure is persistent", observed="not exercised"),
    dict(
        id="EP-V4", site="src/flb_scheduler.c:193-228", function="schedule_request_promote",
        trigger="schedule_request_now fails (mk_event_timeout_create returns -1: out of descriptors or memory)",
        effect="the request is re-added to requests_wait and retried on the next frame",
        cls="unresolved", premise="the resource shortage is persistent", observed="not exercised"),
    dict(
        id="EP-V5", site="plugins/in_storage_backlog/sb.c:602-607", function="cb_queue_chunks",
        trigger="flb_input_chunk_total_size(in) >= mem_limit",
        effect="returns at once every collector period until queued chunks drain",
        cls="yielding", premise="outputs drain queued chunks", observed="not exercised",
        note="a permanent output failure keeps the backlog from draining without violating a safety property"),
]
