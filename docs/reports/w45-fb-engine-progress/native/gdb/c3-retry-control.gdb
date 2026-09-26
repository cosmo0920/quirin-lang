# C3 — control for E3: the same retry fixture with no fault. Each retry
# dispatch brings the chunk up and flushes; the flush fails (closed port), so
# flb_task_retry_create spends an attempt, and retry_limit 3 ends the retry with
# the "cannot be retried" drop (flb_engine.c:783). Counts as E3; stops at the
# first drop or after 12 retry dispatches.
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
    printf "[wd] SUMMARY run=C3 dispatches=%d forced=%d reschedules=%d retry_creates=%d dropped=%d max_attempts=%d\n", $d, $forced, $resched, $created, $dropped, $max_attempts
    kill
    quit
  end
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
  shell date +"[wd] TIME dropped %s.%N"
  printf "[wd] DROPPED (cannot be retried) n=%d\n", $dropped
  printf "[wd] SUMMARY run=C3 dispatches=%d forced=%d reschedules=%d retry_creates=%d dropped=%d max_attempts=%d\n", $d, $forced, $resched, $created, $dropped, $max_attempts
  kill
  quit
end

run
printf "[wd] ENDED run=C3 dispatches=%d forced=%d reschedules=%d retry_creates=%d dropped=%d max_attempts=%d\n", $d, $forced, $resched, $created, $dropped, $max_attempts
quit
