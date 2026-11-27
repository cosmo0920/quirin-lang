# C1 — valid progressing control (no fault) for cb_queue_chunks at 046accf6.
# Counts collector entries, yields (both return points: sb.c:606 and sb.c:724),
# up_force observations (sb.c:635, with the returned value) and queued chunks
# (sb.c:716, `total += size`). Stops after 10 collector yields.
set pagination off
set confirm off
set print address off
set $collector = 0
set $enter = 0
set $ret = 0
set $obs = 0
set $nonzero = 0
set $queued = 0

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
  if $ret >= 10
    shell date +"[wd] TIME summary %s.%N"
    printf "[wd] SUMMARY run=C1 enter=%d returns=%d up_force_obs=%d nonzero=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $queued, $flushes
    kill
    quit
  end
  continue
end

break sb.c:716
commands
  silent
  set $queued = $queued + 1
  continue
end

break sb.c:635
commands
  silent
  set $obs = $obs + 1
  if ret != 0
    set $nonzero = $nonzero + 1
  end
  printf "[wd] OBS up_force ret=%d obs=%d enter=%d returns=%d queued=%d\n", ret, $obs, $enter, $ret, $queued
  continue
end

run
printf "[wd] ENDED run=C1 enter=%d returns=%d up_force_obs=%d nonzero=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $queued, $flushes
quit
