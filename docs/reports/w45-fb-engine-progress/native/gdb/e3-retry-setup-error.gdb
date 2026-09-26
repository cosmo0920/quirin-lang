# E3 — revisit after an error outcome (retry path). Persistent premise: the
# retried chunk cannot be brought up. gdb sets `ret = -1` at
# flb_engine_dispatch.c:115, right after `ret = flb_input_chunk_set_up(task->ic)`
# (line 114) in flb_engine_dispatch_retry, on every retry dispatch. That is the
# value cio_chunk_up returns when storage.max_chunks_up is reached
# (cio_file.c:837-841). Source reading: the -1 arm (lines 115-131) calls
# flb_task_retry_reschedule without spending an attempt, so retry_limit (3)
# never ends the retry. Counts dispatches (with the retry's attempts),
# reschedules, retry creations and the "cannot be retried" drop
# (flb_engine.c:783). Stops after 12 retry dispatches.
set pagination off
set confirm off
set print address off
set $d = 0
set $forced = 0
set $resched = 0
set $created = 0
set $dropped = 0
set $max_attempts = 0

break flb_engine_dispatch_retry
commands
  silent
  set $d = $d + 1
  if retry->attempts > $max_attempts
    set $max_attempts = retry->attempts
  end
  shell date +"[wd] TIME dispatch %s.%N"
  printf "[wd] DISPATCH_RETRY n=%d task_id=%d attempts=%d\n", $d, retry->parent->id, retry->attempts
  if $d >= 12
    printf "[wd] SUMMARY run=E3 dispatches=%d forced=%d reschedules=%d retry_creates=%d dropped=%d max_attempts=%d\n", $d, $forced, $resched, $created, $dropped, $max_attempts
    kill
    quit
  end
  continue
end

break flb_engine_dispatch.c:115
commands
  silent
  set var ret = -1
  set $forced = $forced + 1
  continue
end

break flb_task_retry_reschedule
commands
  silent
  set $resched = $resched + 1
  continue
end

break flb_task_retry_create
commands
  silent
  set $created = $created + 1
  printf "[wd] RETRY_CREATE n=%d\n", $created
  continue
end

break flb_engine.c:783
commands
  silent
  set $dropped = $dropped + 1
  printf "[wd] DROPPED (cannot be retried) n=%d\n", $dropped
  continue
end

run
printf "[wd] ENDED run=E3 dispatches=%d forced=%d reschedules=%d retry_creates=%d dropped=%d max_attempts=%d\n", $d, $forced, $resched, $created, $dropped, $max_attempts
quit
