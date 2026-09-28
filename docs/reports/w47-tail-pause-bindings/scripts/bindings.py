#!/usr/bin/env python3
"""Write and check the W47-TAIL-PAUSE-BINDINGS census (`bindings.json`).

Usage: bindings.py EXPORT_ROOT PREVIOUS_TEST_FILE OUT_JSON

EXPORT_ROOT is a `git archive` export of Fluent Bit 90978ac6 (tree
8d1f3cd7...); PREVIOUS_TEST_FILE is `git show 90978ac6~1:tests/runtime/in_tail.c`.
Every binding and negative case names a file, a line and an `anchor`: a
substring that must occur on that line. The script refuses (exit 1) when an
anchor is not on its line, so the census cannot silently drift from the
source it names. It reads source text only; it executes and proves nothing,
and a binding is a location plus a derived thread attribution, never a
verdict.
"""

import hashlib
import json
import sys
from pathlib import Path

COMMIT = "90978ac6a59e96d19621108f63e115eb4cd1c576"
TREE = "8d1f3cd78a00fcfb4095a39d3d0b3e7eaac80be4"
PARENT = "cb8d69ffc4bb7f01c5a75b6a5d73a2ab86e186ff"
PINNED = "046accf6aded7906f66a1baf0202da349e521736"
CORPUS_PIN = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
PREV = "90978ac6~1"  # the parent's copy of tests/runtime/in_tail.c

# Thread identities (the sketch's actors), derived from the call graph back to
# the thread entry at 90978ac6.
THREADS = {
    "T": "test/API caller thread: the runtime test's main thread (tests/runtime/in_tail.c), which calls flb_start/flb_stop and, in the threaded test and the previous non-threaded test, flb_input_pause/resume",
    "E": "engine thread: mk_utils_worker_spawn(flb_lib_worker) at src/flb_lib.c:969 -> pthread_create at lib/monkey/mk_core/mk_utils.c:318 -> mk_utils_worker_entry -> flb_lib_worker -> flb_engine_start (src/flb_engine.c:1120), which owns config->evl (set at src/flb_engine.c:1150)",
    "I": "input worker (threaded=true only): flb_input_thread_instance_init -> flb_tp_thread_start (src/flb_thread_pool.c:119) -> flb_worker_create -> mk_utils_worker_spawn(step_callback) -> input_thread (src/flb_input_thread.c:318), which owns thi->evl (flb_engine_evl_set at src/flb_input_thread.c:337)",
}

# Existing Quirin carriers a later card could map a binding onto. "none" means
# no current kernel, Lean model or contract carries the binding's obligation.
K_C14 = "concurrency.rs (Cycle 14: sequenced-before, spawn/join and mutex release/acquire edges over a supplied trace) / QCore.Concurrency"
K_REACTOR = "event_reactor.rs (dispatcher per base, L04/L05, explore_activation_orders; cross-thread use and synchronization are supplied premises) / QCore.EventDriven (sameBaseSingleDispatcher is a Bool premise)"
K_HANDOFF = "callback_task_handoff.rs (callback and deferred-work lifetime over a supplied operation sequence) / QCore.CallbackTaskHandoff"
K_TASK = "task_execution (spawn/join ledger of the QCore task route; no C binding) / QCore.TaskExecution"
K_MUTEX = "QCore.MutexInvariant (checked mutex traces for the QCore task route, initially-unlocked non-recursive mutexes only; constraint 27 refuses condition variables)"
K_WEAK = "weak_memory.rs (Cycle 17: one supplied C11 execution graph) / atomic_publication.rs (one closed release/acquire publication shape)"
C_MUTEX = "contract: libc pthread_mutex.qv (custody of mutex storage only, not mutual exclusion or protected data)"
C_WRAP = "contract: monkey wrapper_paths.qv declares _mk_event_add/_del/_timeout_* for the Unix/Libevent backend only (syntax-preserved specifications, outside the resource manifest); nothing for the epoll backend"
NONE = "none"


def b(bid, element, contracts, file, line, anchor, function, threads, carrier, note, prev=False):
    return {
        "id": bid,
        "sketch_element": element,
        "contracts": contracts,
        "file": file,
        "line": line,
        "anchor": anchor,
        "function": function,
        "revision": PREV if prev else COMMIT,
        "threads": threads,
        "carrier": carrier,
        "note": note,
    }


NT = "non-threaded input"
TH = "threaded input"

BINDINGS = [
    # --- thread identity and loop ownership
    b("B01", "thread E entry", ["C1", "C5"], "src/flb_lib.c", 969, "mk_utils_worker_spawn(flb_lib_worker, ctx, &tid)", "do_start",
      {"all": "T spawns E"}, K_TASK, "the only spawn of the engine thread; tid stored in config->worker at :973"),
    b("B02", "thread E entry (pthread_create)", ["C5"], "lib/monkey/mk_core/mk_utils.c", 318, "pthread_create(tid, &thread_attr, mk_utils_worker_entry, start)", "mk_utils_worker_spawn",
      {"all": "T (engine) or E (input workers via flb_worker_create)"}, NONE, "joinable thread; no pthread_create contract exists (libc provider declares none)"),
    b("B03", "loop ownership L(E): config->evl", ["C1"], "src/flb_engine.c", 1150, "config->evl = evl;", "flb_engine_start",
      {NT: "E"}, K_REACTOR, "the engine loop is created and published by E before any collector starts"),
    b("B04", "loop ownership L(I): thi->evl", ["C1"], "src/flb_input_thread.c", 255, "thi->evl = mk_event_loop_create(256);", "input_thread_instance_create",
      {TH: "E creates it (flb_input_init_all on E); I runs it from src/flb_input_thread.c:421"}, K_REACTOR, "created on E during input init, dispatched only by I: a pre-publication phase the sketch calls quiescent"),
    b("B05", "collector attachment coll->evl (threaded)", ["C1"], "src/flb_input.c", 2251, "coll->evl = thi->evl;", "collector_create",
      {TH: "I (for a threaded input, cb_init and cb_pre_run both run on I: src/flb_input_thread.c:369, :409)"}, NONE, "collector-to-loop attachment; no Quirin vocabulary names a loop owner"),
    b("B06", "collector attachment coll->evl (non-threaded) and the stated ownership comment", ["C1"], "src/flb_input.c", 2254, "Non-threaded inputs must always attach collectors to the owner", "collector_create",
      {NT: "E (cb_init and cb_pre_run run on E)"}, NONE, "the survey's flb_input.c:2231-2240 is this text at 66910c10 (2233-2240); at 046accf6 and 90978ac6 it is 2250-2258 (file identical at both)"),
    # --- collector register/start/pause/resume/delete/running
    b("B07", "collector register (time)", ["C1"], "src/flb_input.c", 2273, "int flb_input_set_collector_time(", "flb_input_set_collector_time",
      {NT: "E", TH: "I"}, NONE, "list insertion into ins->collectors at :2267 without a lock"),
    b("B08", "collector register (fd event)", ["C1"], "src/flb_input.c", 2294, "int flb_input_set_collector_event(", "flb_input_set_collector_event",
      {NT: "E", TH: "I"}, NONE, ""),
    b("B09", "collector start", ["C1"], "src/flb_input.c", 2335, "static int collector_start(", "collector_start",
      {NT: "E via flb_input_collectors_start (src/flb_engine.c:1392)", TH: "I via flb_input_thread_collectors_start (src/flb_input_thread.c:405)"}, NONE, "timer: mk_event_timeout_create on coll->evl; fd: mk_event_add"),
    b("B10", "collector running (read)", ["C1"], "src/flb_input.c", 2964, "int flb_input_collector_running(", "flb_input_collector_running",
      {NT: "E (revised test callback) or T (previous test)", TH: "T (threaded test poll)"}, NONE, "plain int read of coll->running; no atomic, no lock"),
    b("B11", "collector pause", ["C1", "C3"], "src/flb_input.c", 3158, "int flb_input_collector_pause(", "flb_input_collector_pause",
      {NT: "E (revised test) / T (previous test)", TH: "I"}, NONE, "timer: fd_timer=-1 then mk_event_timeout_destroy (result ignored); fd: mk_event_del, -1 on failure; running=FALSE only on the success path"),
    b("B12", "collector delete", ["C1"], "src/flb_input.c", 3197, "int flb_input_collector_delete(", "flb_input_collector_delete",
      {NT: "E", TH: "I"}, C_MUTEX, "the only collector-list mutation under config->collectors_mutex; not called in the sketch's paths"),
    b("B13", "collector resume", ["C1", "C3"], "src/flb_input.c", 3218, "int flb_input_collector_resume(", "flb_input_collector_resume",
      {NT: "E (revised test) / T (previous test)", TH: "I"}, NONE, "already-running returns -1 (logged error)"),
    b("B14", "is_ingestion_active early return", ["C3"], "src/flb_input.c", 3241, "if (config->is_ingestion_active == FLB_FALSE) {", "flb_input_collector_resume",
      {NT: "E / T", TH: "I"}, NONE, "returns 0 without resuming and without setting running: success at the API is not a running-state promise"),
    b("B15", "is_ingestion_active writers", ["C3"], "src/flb_engine.c", 1742, "config->is_ingestion_active = FLB_FALSE;", "flb_engine_stop_ingestion",
      {"all": "E (engine stop path, src/flb_engine.c:1477)"}, NONE, "also src/flb_engine.c:1670 (flb_engine_shutdown, E) and src/flb_config.c:302 (init); read on I by a threaded resume with no synchronization"),
    b("B16", "collector destroy", ["C1", "C5"], "src/flb_input.c", 3140, "int flb_input_collector_destroy(", "flb_input_collector_destroy",
      {NT: "E (flb_input_instance_destroy from flb_engine_shutdown)", TH: "E after flb_input_thread_instance_exit joined I"}, NONE, "deletes through config->evl, not coll->evl (:3146, :3150): for a threaded input the collector was attached to thi->evl, which I destroyed before exiting (binding residual, not a defect claim)"),
    b("B17", "engine dispatch of a collector", ["C1"], "src/flb_input.c", 3276, "int flb_input_collector_fd(", "flb_input_collector_fd",
      {NT: "E (flb_engine_handle_event, src/flb_engine.c:1002)"}, K_REACTOR, "linear scan of config->inputs x ins->collectors; runs cb_collect synchronously on E (in_tail is not a coroutine input)"),
    b("B18", "worker dispatch of a collector", ["C1"], "src/flb_input_thread.c", 133, "static int input_collector_fd(", "input_collector_fd",
      {TH: "I"}, K_REACTOR, ""),
    # --- mk_event (epoll backend of the production build)
    b("B19", "mk_event_add", ["C1"], "lib/monkey/mk_core/mk_event.c", 99, "int mk_event_add(", "mk_event_add",
      {"all": "caller's thread"}, C_WRAP, "delegates to _mk_event_add"),
    b("B20", "mk_event_add (epoll): priority-queue link removal", ["C1"], "lib/monkey/mk_core/mk_event_epoll.c", 149, "mk_list_del(&event->_priority_head);", "_mk_event_add",
      {"all": "caller's thread"}, NONE, "the userspace intrusive link the sketch names; epoll_ctl (:138) is thread-safe, this unlink is not"),
    b("B21", "mk_event_del", ["C1"], "lib/monkey/mk_core/mk_event.c", 132, "int mk_event_del(", "mk_event_del",
      {"all": "caller's thread"}, C_WRAP, "returns -1 on an unregistered event"),
    b("B22", "mk_event_del (epoll): priority-queue link removal", ["C1"], "lib/monkey/mk_core/mk_event_epoll.c", 180, "mk_list_del(&event->_priority_head);", "_mk_event_del",
      {"all": "caller's thread"}, NONE, ""),
    b("B23", "timeout create", ["C1"], "lib/monkey/mk_core/mk_event.c", 156, "int mk_event_timeout_create(", "mk_event_timeout_create",
      {"all": "caller's thread"}, C_WRAP, "timerfd backend (MK_HAVE_TIMERFD_CREATE) at mk_event_epoll.c:190"),
    b("B24", "timeout destroy", ["C1"], "lib/monkey/mk_core/mk_event_epoll.c", 341, "static inline int _mk_event_timeout_destroy(", "_mk_event_timeout_destroy",
      {"all": "caller's thread"}, C_WRAP, "_mk_event_del then close(event->fd); returns 0 regardless"),
    b("B25", "engine loop iteration over the bucket queue", ["C1"], "include/fluent-bit/flb_event_loop.h", 74, "#define flb_event_priority_live_foreach", "flb_event_priority_live_foreach (macro)",
      {NT: "E (src/flb_engine.c:1454)", TH: "I (src/flb_input_thread.c:422)"}, K_REACTOR, "pops _priority_head links the pause/resume paths unlink"),
    # --- public pause/resume
    b("B26", "flb_input_pause", ["C2"], "src/flb_input.c", 3074, "int flb_input_pause(", "flb_input_pause",
      {NT: "E (revised test callback; flb_engine_shutdown/stop_ingestion via flb_input_pause_all) or T (previous test)", TH: "T (threaded test) or E (flb_input_pause_all at shutdown)"}, NONE, "branches: already buffer-paused -> -1; threaded -> flb_input_thread_instance_pause; non-threaded -> flb_input_plugin_pause"),
    b("B27", "flb_input_pause shutdown clause", ["C2"], "src/flb_input.c", 3078, "ins->config->is_shutting_down == FLB_FALSE", "flb_input_pause",
      {"all": "as B26"}, NONE, "added by 058c2c3c7 (between 66910c10 and 046accf6): an already buffer-paused input passes on to the pause callback only while shutting down with FLB_INPUT_SHUTDOWN_FLUSH, which only in_emitter sets; for in_tail it reduces to the 66910c10 test"),
    b("B28", "flb_input_resume", ["C2"], "src/flb_input.c", 3100, "int flb_input_resume(", "flb_input_resume",
      {NT: "E or T", TH: "T"}, NONE, ""),
    b("B29", "synchronous plugin pause (non-threaded)", ["C2"], "src/flb_input.c", 3034, "int flb_input_plugin_pause(", "flb_input_plugin_pause",
      {NT: "the flb_input_pause caller's thread", TH: "I (handle_input_event)"}, NONE, "void cb_pause returns nothing: ret stays 0; the paused gauge is set on the caller's thread"),
    b("B30", "synchronous plugin resume (non-threaded)", ["C2"], "src/flb_input.c", 3054, "int flb_input_plugin_resume(", "flb_input_plugin_resume",
      {NT: "the flb_input_resume caller's thread", TH: "I"}, NONE, ""),
    b("B31", "threaded pause request: pipe write", ["C2", "C6"], "src/flb_input_thread.c", 542, "ret = flb_pipe_w(thi->ch_parent_events[1], &val, sizeof(val));", "flb_input_thread_instance_pause",
      {TH: "T or E"}, NONE, "returns 0 when ins->thi is NULL (:532) and 0 on a successful write: enqueue, not completion"),
    b("B32", "threaded resume request: pipe write", ["C2", "C6"], "src/flb_input_thread.c", 571, "ret = flb_pipe_w(thi->ch_parent_events[1], &val, sizeof(val));", "flb_input_thread_instance_resume",
      {TH: "T"}, NONE, ""),
    b("B33", "threaded request receive", ["C2"], "src/flb_input_thread.c", 53, "static inline int handle_input_event(", "handle_input_event",
      {TH: "I (src/flb_input_thread.c:475)"}, NONE, "pause/resume failures are logged only; no acknowledgement to the sender"),
    # --- tail
    b("B34", "in_tail pause callback", ["C2", "C3"], "plugins/in_tail/tail.c", 518, "static void in_tail_pause(", "in_tail_pause",
      {NT: "as B29", TH: "I"}, NONE, "pauses static, pending, docker/multiline flush and the fs backend; scan, watcher and rotated collectors are not paused"),
    b("B35", "in_tail resume callback", ["C2", "C3"], "plugins/in_tail/tail.c", 550, "static void in_tail_resume(", "in_tail_resume",
      {NT: "as B30", TH: "I"}, NONE, ""),
    b("B36", "backend dispatch", ["C3"], "plugins/in_tail/tail_fs.h", 45, "static inline void flb_tail_fs_pause(", "flb_tail_fs_pause",
      {"all": "as B34"}, NONE, "inotify when ctx->inotify_watcher, else stat"),
    b("B37", "inotify backend pause", ["C3"], "plugins/in_tail/tail_fs_inotify.c", 457, "void flb_tail_fs_inotify_pause(", "flb_tail_fs_inotify_pause",
      {"all": "as B34"}, NONE, "both flb_input_collector_pause results ignored (:459, :460)"),
    b("B38", "inotify backend resume", ["C3"], "plugins/in_tail/tail_fs_inotify.c", 463, "void flb_tail_fs_inotify_resume(", "flb_tail_fs_inotify_resume",
      {"all": "as B35"}, NONE, "both flb_input_collector_resume results ignored (:465, :466)"),
    b("B39", "Cfs registration", ["C3"], "plugins/in_tail/tail_fs_inotify.c", 432, "ret = flb_input_set_collector_event(in, tail_fs_event,", "flb_tail_fs_inotify_init",
      {NT: "E", TH: "I"}, NONE, ""),
    b("B40", "Cprogress registration", ["C3"], "plugins/in_tail/tail_fs_inotify.c", 444, "ret = flb_input_set_collector_time(in, in_tail_progress_check_callback,", "flb_tail_fs_inotify_init",
      {NT: "E", TH: "I"}, NONE, ""),
    # --- test path (90978ac6)
    b("B41", "test collector callback: disables itself, pause, snapshot, resume, publish", ["C1", "C2", "C3", "C4"], "tests/runtime/in_tail.c", 2475, "static int cb_tail_pause_test(", "cb_tail_pause_test",
      {NT: "E (dispatched by flb_input_collector_fd)"}, K_C14, "flb_input_collector_pause(test collector) at :2483 before taking test->mutex at :2484"),
    b("B42", "publication under the mutex", ["C4"], "tests/runtime/in_tail.c", 2496, "test->done = FLB_TRUE;", "cb_tail_pause_test",
      {NT: "E"}, K_MUTEX, "results written before done; pthread_cond_signal (:2497) while locked; unlock at :2498"),
    b("B43", "test collector registration on the owner", ["C1"], "tests/runtime/in_tail.c", 2515, "test->collector_id = flb_input_set_collector_time(ins, cb_tail_pause_test,", "cb_tail_pause_test_pre_run",
      {NT: "E (flb_input_pre_run_all at src/flb_engine.c:1314, before flb_input_collectors_start at :1392)"}, NONE, "10 ms timer; started by flb_input_collectors_start on E"),
    b("B44", "plugin copy and test-owned data", ["C5"], "tests/runtime/in_tail.c", 2556, "ins->p = &test.plugin;", "flb_test_inotify_pause_collectors",
      {NT: "T, before flb_start publishes the instance to E"}, NONE, "ins->p and ins->data (:2557) point into T's stack frame for the engine's lifetime"),
    b("B45", "await with deadline", ["C4"], "tests/runtime/in_tail.c", 2565, "ret = pthread_cond_timedwait(&test.cond, &test.mutex, &deadline);", "flb_test_inotify_pause_collectors",
      {NT: "T"}, NONE, "rechecks test.done in a loop; a timeout or error breaks out and the check at :2570 fails; results read under the mutex until :2579"),
    b("B46", "stop and join", ["C5"], "tests/runtime/in_tail.c", 2583, "ret = flb_stop(ctx->flb);", "flb_test_inotify_pause_collectors",
      {NT: "T"}, K_TASK, "a nonzero flb_stop exits the process without freeing (:2586)"),
    b("B47", "free after join", ["C5"], "tests/runtime/in_tail.c", 2588, "flb_destroy(ctx->flb);", "flb_test_inotify_pause_collectors",
      {NT: "T"}, NONE, "then fixture removal (:2591), condvar and mutex destroy (:2595, :2596)"),
    b("B48", "flb_start", ["C5"], "src/flb_lib.c", 1012, "int flb_start(flb_ctx_t *ctx)", "flb_start",
      {"all": "T"}, NONE, "do_start waits for FLB_ENGINE_STARTED on config->ch_evl (a pipe read on T after E's write at src/flb_engine.c:1039)"),
    b("B49", "flb_stop", ["C5"], "src/flb_lib.c", 1040, "int flb_stop(flb_ctx_t *ctx)", "flb_stop",
      {"all": "T"}, NONE, "status NONE/ERROR: joins and returns 0 ignoring the join result (:1058-1059); running: flb_engine_exit then returns pthread_join's result (:1077); ctx->status is written by E (:932) and read by T without synchronization"),
    b("B50", "engine join", ["C5"], "src/flb_lib.c", 1077, "ret = pthread_join(tid, NULL);", "flb_stop",
      {"all": "T joins E"}, K_TASK, "no pthread_join contract exists"),
    b("B51", "input worker join", ["C5"], "src/flb_input_thread.c", 607, "pthread_join(tid, NULL);", "flb_input_thread_instance_exit",
      {TH: "E joins I (flb_engine_shutdown -> flb_input_exit_all)"}, K_TASK, "result ignored"),
    # --- condvar handshake of the worker
    b("B52", "worker status publish", ["C4"], "src/flb_input_thread.c", 683, "static int input_thread_instance_set_status(", "input_thread_instance_set_status",
      {TH: "I"}, K_MUTEX, "status written and cond signalled under init_mutex"),
    b("B53", "worker status await", ["C4"], "src/flb_input_thread.c", 699, "static int input_thread_instance_get_status(", "input_thread_instance_get_status",
      {TH: "E (flb_input_thread_instance_init and flb_input_thread_instance_pre_run)"}, NONE, "waits in a loop on init_status == 0 under the mutex"),
    b("B54", "condvar re-init after unlock", ["C4"], "src/flb_input_thread.c", 715, "pthread_cond_destroy(&thi->init_condition);", "input_thread_instance_get_status",
      {TH: "E"}, NONE, "destroy/init at :715-716 after the unlock, then init_status read without the mutex at :719; ordered after the only signal by the mutex hand-off (candidate-satisfying under that premise); constraint 27 refuses condition variables"),
    # --- threaded test and the previous test
    b("B55", "threaded test: unsynchronized running poll", ["C1"], "tests/runtime/in_tail.c", 2440, "static int wait_tail_collectors_state(", "wait_tail_collectors_state",
      {TH: "T reads coll->running written by I"}, NONE, "50 iterations x flb_time_msleep(100)"),
    b("B56", "threaded test: pause request from T", ["C2"], "tests/runtime/in_tail.c", 2634, "ret = flb_input_pause(ins);", "flb_test_inotify_threaded_pause_collectors",
      {TH: "T"}, NONE, "success means the request was written, not applied"),
    b("B57", "threaded test: context read after start", ["C1"], "tests/runtime/in_tail.c", 2629, "tail_ctx = ins->context;", "flb_test_inotify_threaded_pause_collectors",
      {TH: "T reads ins->context written by I in cb_init"}, NONE, "ordered, if at all, by I's set_status (mutex) -> E -> the started pipe -> T"),
    b("B58", "fixture teardown helper: unlink before stop, stop result ignored", ["C5"], "tests/runtime/in_tail.c", 286, "unlink(ctx->filepaths[i]);", "test_tail_ctx_destroy",
      {"all": "T"}, NONE, "used by the threaded test (:2644) and by the previous non-threaded test; sleep(1) then flb_stop at :292 with its result ignored"),
    b("B59", "previous non-threaded test: pause from the test thread", ["C1", "C2"], "tests/runtime/in_tail.c", 2497, "ret = flb_input_pause(ins);", "flb_test_inotify_pause_collectors",
      {NT: "T, while E dispatches config->evl"}, NONE, "at 90978ac6~1 (identical blob at 046accf6); T mutates E's loop through in_tail_pause -> flb_input_collector_pause -> mk_event_del", prev=True),
    b("B60", "previous non-threaded test: running read from the test thread", ["C1"], "tests/runtime/in_tail.c", 2499, "TEST_CHECK(flb_input_collector_running(tail_ctx->coll_fd_fs1, ins) == FLB_FALSE);", "flb_test_inotify_pause_collectors",
      {NT: "T"}, NONE, "", prev=True),
]

NEGATIVE_CASES = [
    {"id": "NC1", "trace": "Old non-threaded test: T:start; E:dispatch || T:pause/resume",
     "bound_path": ["B59", "B26", "B29", "B34", "B37", "B11", "B21", "B22", "B25"],
     "expected": "reject", "obligation": "C1/C2: T lacks Own(L); flb_input_pause runs the plugin callback on T, which unlinks _priority_head links of E's loop",
     "reachable_in_pinned_tests": "yes, as a candidate: flb_test_inotify_pause_collectors at 90978ac6~1 (and at 046accf6, the same blob) pauses from T (tests/runtime/in_tail.c:2497 there)"},
    {"id": "NC2", "trace": "Revised test: E:callback, pause, snapshot, resume, publish; T:await, stop/join, free",
     "bound_path": ["B43", "B41", "B26", "B29", "B11", "B13", "B10", "B42", "B45", "B46", "B50", "B47"],
     "expected": "candidate-satisfying", "obligation": "C1-C5 under the stated premises: every collector access on E; publication under test.mutex; free only after a successful join",
     "reachable_in_pinned_tests": "yes: flb_test_inotify_pause_collectors at 90978ac6 (the intended trace; no model checks it)"},
    {"id": "NC3", "trace": "T:enqueue_pause; T:read Cfs.running",
     "bound_path": ["B56", "B31", "B55", "B10", "B33"],
     "expected": "reject", "obligation": "C1/C2: a successful enqueue is not completion; the running read is unsynchronized with I's write",
     "reachable_in_pinned_tests": "yes, as a candidate: flb_test_inotify_threaded_pause_collectors at 90978ac6 (tests/runtime/in_tail.c:2634-2636) polls coll->running from T (50 x 100 ms)"},
    {"id": "NC4", "trace": "E:publish done; T:free S; E:return from callback",
     "bound_path": ["B42", "B45", "B47"],
     "expected": "reject", "obligation": "C5: publication is not join",
     "reachable_in_pinned_tests": "no: the revised test frees only after flb_stop (B46) returned 0, which on its running path is pthread_join's result; the previous test's test-owned storage was not callback-retained"},
    {"id": "NC5", "trace": "T:wait timeout; T:free S; E:late callback",
     "bound_path": ["B45", "B46", "B47"],
     "expected": "reject", "obligation": "C5: timeout is not cancellation",
     "reachable_in_pinned_tests": "no: a timeout breaks the wait and takes the same flb_stop/join path; a failed flb_stop exits without freeing (tests/runtime/in_tail.c:2586). Residual: flb_stop's NONE/ERROR branch returns 0 without checking its join (src/flb_lib.c:1058)"},
    {"id": "NC6", "trace": "E:resume while ingestion disabled; T:assume running",
     "bound_path": ["B28", "B30", "B35", "B38", "B13", "B14"],
     "expected": "reject", "obligation": "C3's missing ingestion premise: flb_input_collector_resume returns 0 at src/flb_input.c:3241 without resuming",
     "reachable_in_pinned_tests": "no: neither test resumes after flb_engine_stop_ingestion; reachable in the engine only after stop_ingestion"},
    {"id": "NC7", "trace": "One backend collector fails to resume",
     "bound_path": ["B38", "B13", "B19", "B23"],
     "expected": "no-conclusion", "obligation": "C3: no paired-running conclusion; flb_tail_fs_inotify_resume ignores both results, so the partial state is not reported",
     "reachable_in_pinned_tests": "not driven: needs an mk_event_add or timerfd failure"},
    {"id": "NC8", "trace": "CV wakes before publication",
     "bound_path": ["B45", "B42"],
     "expected": "no-conclusion (wait again)", "obligation": "C4: the loop rechecks test.done under the mutex; no result is read without done",
     "reachable_in_pinned_tests": "allowed by POSIX (spurious wakeup); the revised test handles it; not an obligation failure"},
    {"id": "NC9", "trace": "Fixture removal before join (test_tail_ctx_destroy)",
     "bound_path": ["B58", "B49"],
     "expected": "reject", "obligation": "C5's test-specific fixture rule: files are unlinked before flb_stop and the stop result is ignored",
     "reachable_in_pinned_tests": "yes, as a candidate: the threaded test (tests/runtime/in_tail.c:2644) and the previous non-threaded test tear down through test_tail_ctx_destroy. Added by this census; not in the sketch's table"},
]

SURVEY = [
    {"claim": "no thread/owner/happens-before clause in resource-v1", "result": "held",
     "evidence": "pthread_mutex.qv and threads.qv state custody only ('Nothing about mutual exclusion, lock ordering, ...'); no resource-v1 predicate names a thread, loop owner or happens-before edge"},
    {"claim": "condvars refused under constraint 27", "result": "held",
     "evidence": "docs/agents/concurrency.md constraint 27: condition variables are represented refusals of the mutex model. The libc provider still declares cnd_wait/cnd_timedwait as custody-only (borrowed) calls, which says nothing about wait semantics"},
    {"claim": "no pthread_create/join/cond contracts", "result": "held",
     "evidence": "no .qv/.qcore under providers/ or contract/ declares pthread_create, pthread_join or any pthread_cond_* function (they appear only in providers/quirin-provider-libc/libc-names.txt, the reserved-name list)"},
    {"claim": "event_reactor has dispatcher-per-base with L04/L05 but EventDriven.lean's sameBaseSingleDispatcher is a Bool premise", "result": "held",
     "evidence": "crates/qcore-core/src/event_reactor.rs: BeginDispatch records one dispatcher per base, L04ThreadInitialization, L05ConflictingAccess (the access's `synchronized` flag is supplied); proof/QCore/EventDriven.lean:131 `sameBaseSingleDispatcher : Bool`, assumed at :149 and :195"},
    {"claim": "none of the sketch's files is in any current selection", "result": "partly held",
     "evidence": "flb_input.c, flb_input_thread.c, flb_lib.c, plugins/in_tail/*, tests/runtime/in_tail.c and mk_event_epoll.c are in no selection; but src/flb_engine.c (the engine dispatch, flb_engine_start/shutdown and the is_ingestion_active writers) is in system-selection.json, and lib/monkey/mk_core/mk_event.c is in the 185-unit corpus through the forced-Libevent Monkey build (so the libevent backend, not epoll)"},
    {"claim": "flb_input.c:2231-2240 states loop ownership", "result": "held at 66910c10 only",
     "evidence": "the comment and assignment are at 2233-2240 at 66910c10 and at 2250-2258 at 046accf6 and 90978ac6 (flb_input.c is byte-identical at those two)"},
    {"claim": "the non-threaded test at 90978ac6~1 pauses from the test thread", "result": "held",
     "evidence": "tests/runtime/in_tail.c:2497 at 90978ac6~1 (blob 2f93a552, also the 046accf6 blob)"},
    {"claim": "the threaded test polls running unsynchronized (50 x 100 ms)", "result": "held",
     "evidence": "wait_tail_collectors_state, tests/runtime/in_tail.c:2440-2459 at 90978ac6: 50 iterations of flb_input_collector_running then flb_time_msleep(100)"},
    {"claim": "flb_tail_fs_inotify_pause ignores both return values", "result": "held",
     "evidence": "plugins/in_tail/tail_fs_inotify.c:459-460 (and resume :465-466)"},
    {"claim": "flb_input_collector_resume returns 0 without resuming when ingestion is inactive", "result": "held",
     "evidence": "src/flb_input.c:3241-3243; running stays FALSE"},
    {"claim": "input_thread_instance_set/get_status re-inits its condvar after unlock", "result": "held",
     "evidence": "src/flb_input_thread.c:712 unlock, :715-716 destroy/init, :719 unlocked status read"},
]

RESIDUALS = [
    {"id": "R1", "binding": "B16", "text": "flb_input_collector_destroy deletes through config->evl, not coll->evl; for a threaded input the collector's loop was thi->evl, already destroyed by I"},
    {"id": "R2", "binding": "B49", "text": "flb_stop's NONE/ERROR branch joins but returns 0 regardless of the join result; ctx->status is a plain field written by E and read by T"},
    {"id": "R3", "binding": "B34", "text": "in_tail_pause does not pause the scan, watcher and rotated collectors; C3 covers only the two backend collectors"},
    {"id": "R4", "binding": "B31", "text": "flb_input_thread_instance_pause/resume return 0 when ins->thi is NULL (no worker): the live-worker premise of C2 has no source witness"},
    {"id": "R5", "binding": "B11", "text": "flb_input_collector_pause ignores mk_event_timeout_destroy's result for time collectors"},
    {"id": "R6", "binding": None, "text": "the delta adds a process-wide tail file budget shared across inputs (tail_file_budget.c: budget_lock plus CFL atomics); the sketch excludes other inputs, so no C1-C6 obligation covers it"},
]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check(root, previous, entries):
    failures = []
    for entry in entries:
        path = previous if entry["revision"] == PREV else root / entry["file"]
        lines = Path(path).read_text(errors="replace").splitlines()
        line = lines[entry["line"] - 1] if 0 < entry["line"] <= len(lines) else ""
        if entry["anchor"] not in line:
            failures.append(f"{entry['id']}: `{entry['anchor']}` not on {entry['file']}:{entry['line']} ({entry['revision']})")
    return failures


def main():
    root = Path(sys.argv[1])
    previous = Path(sys.argv[2])
    failures = check(root, previous, BINDINGS)
    if failures:
        print("\n".join(failures))
        return 1
    ids = {entry["id"] for entry in BINDINGS}
    for case in NEGATIVE_CASES:
        missing = [bid for bid in case["bound_path"] if bid not in ids]
        if missing:
            print(f"{case['id']}: unknown bindings {missing}")
            return 1
    files = sorted({e["file"] for e in BINDINGS if e["revision"] == COMMIT})
    document = {
        "schema": "w47-tail-pause-bindings/1",
        "scope": "a source-binding census of design sketch in_tail_pause_contracts.r1.md (SHA-256 58add9efed43576dc7ae95de8e56c55b45fe4915fcb5493b1f37977a851d51e0); locations and derived thread attributions only. No binding is a verdict, a verified race or an upstream defect claim; every reject is a candidate for a future checker.",
        "source": {
            "commit": COMMIT, "tree": TREE, "parent": PARENT,
            "parent_diff_sha256": "87800af96d47c9f4b4cea616ae85308a6449211136e0850ffaaee4c3296b4314",
            "pinned_base": PINNED, "corpus_pin": CORPUS_PIN,
            "previous_test_sha256": sha256(previous),
            "file_sha256": {f: sha256(root / f) for f in files},
        },
        "threads": THREADS,
        "bindings": BINDINGS,
        "negative_cases": NEGATIVE_CASES,
        "survey": SURVEY,
        "residuals": RESIDUALS,
    }
    with open(sys.argv[3], "w") as handle:
        json.dump(document, handle, indent=2)
        handle.write("\n")
    print(f"{len(BINDINGS)} bindings, {len(NEGATIVE_CASES)} negative cases, {len(files)} files: anchors hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
