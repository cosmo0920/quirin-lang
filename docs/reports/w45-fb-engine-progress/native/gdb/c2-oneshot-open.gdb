# C2 — control for E2: ONE transient failure that leaves no residue. The first
# collector-originated cio_chunk_up_force reaches _cio_file_up, whose
# cio_file_native_open (cio_file.c:845) gdb makes fail once at entry (returns
# CIO_ERROR, the descriptor stays -1, as the open() failure arm
# cio_file_unix.c:410-413 leaves it). Nothing else is forced. Source reading
# predicts the next up_force of the same chunk succeeds and the queues drain.
# Counts as E2; stops after 10 collector yields.
set pagination off
set confirm off
set print address off
set $collector = 0
set $enter = 0
set $ret = 0
set $obs = 0
set $nonzero = 0
set $queued = 0
set $in_up = 0
set $faults = 0
set $fd_exists = 0
set $already_mapped = 0

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
    printf "[wd] SUMMARY run=C2 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d fd_exists_arm=%d already_mapped_arm=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $faults, $fd_exists, $already_mapped, $queued, $flushes
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

break cio_chunk_up_force
commands
  silent
  if $collector == 1
    set $in_up = 1
  end
  continue
end

break cio_file_native_open
commands
  silent
  if $in_up == 1 && $faults == 0
    set $faults = 1
    printf "[wd] INJECT one-shot cio_file_native_open -> CIO_ERROR fd=%d path=%s\n", cf->fd, cf->path
    return -1
    continue
  end
  continue
end

break cio_file.c:828
commands
  silent
  set $fd_exists = $fd_exists + 1
  continue
end

break cio_file.c:822
commands
  silent
  set $already_mapped = $already_mapped + 1
  continue
end

break sb.c:635
commands
  silent
  set $in_up = 0
  set $obs = $obs + 1
  if ret != 0
    set $nonzero = $nonzero + 1
  end
  printf "[wd] OBS up_force ret=%d obs=%d enter=%d returns=%d queued=%d fd_exists_arm=%d already_mapped_arm=%d\n", ret, $obs, $enter, $ret, $queued, $fd_exists, $already_mapped
  continue
end

run
printf "[wd] ENDED run=C2 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d fd_exists_arm=%d already_mapped_arm=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $faults, $fd_exists, $already_mapped, $queued, $flushes
quit
