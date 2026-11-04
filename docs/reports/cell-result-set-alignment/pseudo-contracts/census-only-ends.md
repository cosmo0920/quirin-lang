# Pseudo contracts: the census-only ends

These cover `flb_input_collector.fd_timer`, `flb_sp_task.window.fd`, `flb_config.shutdown_fd`, and Monkey's `mk_server_worker_loop` `timeout_fd`, `mk_sched_worker.signal_channel_w` and `mk_server.lib_ch_start[1]`.

Evidence only. None of these is declared, and the third patch touches none of them: section (d) is "not touched" for each. The live set of each timer comes from `mk_event_timeout_create`, which is MISSING: `timerfd_create` is undeclared and RULE-38-VALIDITY-READING-RESULT-SET is still open. The class uses the proposed `[0, MAX]`.

```
// PSEUDO CONTRACT: evidence only, not loaded, not in force.
// flb_input_collector.fd_timer: live = MISSING (proposed [0, MAX]); absent = -1 (flb_input.c:2242, :3181)
//   test fd_timer > 0 (flb_input.c:3145): dead [MIN, 0] & [0, MAX] = 0 => NOT decided (WRONG):
//   a COLLECT_TIME timer on descriptor 0 is never destroyed; the collector is freed while registered
//   fixed: test != -1 (or >= 0): dead {-1} & [0, MAX] = {} => decided
// flb_sp_task.window.fd: absent = 0 (calloc, flb_sp.c:462); test > 0 (flb_sp.c:670) => WRONG (shared 0)
//   fixed: -1 at allocation, test != -1
// flb_config.shutdown_fd: absent = 0 (calloc); tests <= 0 (flb_engine.c:1497), > 0 (:1563) => WRONG
//   (a timer on 0 is created again and never destroyed); fixed: -1 at init, != -1
// mk_server_worker_loop timeout_fd (local, mk_server.c:502, never tested): test > 0 (:585) => WRONG
// mk_sched_worker.signal_channel_w (absent 0, memset) test <= 0 (mk_server.c:630): declared-policy
//   column exact; POSIX column WRONG (write end on 0 read as no channel)
// mk_server.lib_ch_start[1] (absent 0, mk_mem_alloc_z) test <= 0 (mk_server.c:648): the same
```

(c) Native evidence: none of these was observed on descriptor 0. Source reading only.

(d) Planned fix: not touched by the patch. Under the declared [1, MAX], `signal_channel_w` and `lib_ch_start` would read exact once their creator's written set is derived; the timers need RULE-38-VALIDITY-READING-RESULT-SET.
