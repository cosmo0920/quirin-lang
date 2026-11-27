# E1 — zero-progress failure outcome, persistent premise (the W45-FB-BACKLOG-
# FAULT-INJECTION experiment re-run at 046accf6): every collector-originated
# cio_chunk_up_force (sb.c:633) returns CIO_ERROR (-1). Gated on the collector
# having been entered, so startup segregation is untouched. Counts as C1.
# Stops after 12000 up_force observations at sb.c:635.
set pagination off
set confirm off
set print address off
set $collector = 0
set $enter = 0
set $ret = 0
set $obs = 0
set $nonzero = 0
set $queued = 0
set $inject = 0

break sb.c:cb_queue_chunks
commands
  silent
  set $collector = 1
  set $enter = $enter + 1
  if $enter == 1
    shell date +"[wd] TIME first-collector-entry %s.%N"
  end
  printf "[wd] ENTER cb_queue_chunks enter=%d returns=%d obs=%d queued=%d\n", $enter, $ret, $obs, $queued
  continue
end

# Engine starvation: flush-timer dispatches (flb_engine_flush) after the
# collector was first entered. The storage_backlog collector runs on the engine
# thread, so while it does not return no other engine event is handled.
set $flushes = 0
break flb_engine_flush
commands
  silent
  if $collector == 1
    set $flushes = $flushes + 1
  end
  continue
end

break sb.c:606
commands
  silent
  set $ret = $ret + 1
  printf "[wd] RETURN(limit) cb_queue_chunks returns=%d enter=%d\n", $ret, $enter
  continue
end

break sb.c:724
commands
  silent
  set $ret = $ret + 1
  printf "[wd] RETURN cb_queue_chunks returns=%d enter=%d obs=%d queued=%d\n", $ret, $enter, $obs, $queued
  continue
end

break sb.c:716
commands
  silent
  set $queued = $queued + 1
  continue
end

break cio_chunk_up_force
commands
  silent
  if $collector == 1
    set $inject = $inject + 1
    return -1
    continue
  end
  continue
end

break sb.c:635
commands
  silent
  set $obs = $obs + 1
  if ret != 0
    set $nonzero = $nonzero + 1
  end
  if $obs <= 6 || $obs % 1000 == 0
    printf "[wd] OBS up_force ret=%d obs=%d enter=%d returns=%d queued=%d\n", ret, $obs, $enter, $ret, $queued
  end
  if $obs >= 12000
    printf "[wd] CAP reached: up_force_obs=%d\n", $obs
    shell date +"[wd] TIME summary %s.%N"
    printf "[wd] SUMMARY run=E1 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $inject, $queued, $flushes
    kill
    quit
  end
  continue
end

run
printf "[wd] ENDED run=E1 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $inject, $queued, $flushes
quit
