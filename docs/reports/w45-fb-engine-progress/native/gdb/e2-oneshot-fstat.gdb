# E2 — zero-progress failure outcome from ONE transient failure. The first
# collector-originated cio_chunk_up_force reaches _cio_file_up, which opens the
# file (cio_file_native_open succeeds) and then calls cio_file_update_size
# (cio_file.c:853). gdb makes that one call behave as its failure arm
# (cio_file.c:912-916: fs_size = 0, return the non-CIO_OK result): it sets
# cf->fs_size = 0 and returns CIO_ERROR (-1), once. Nothing else is forced.
# Source reading predicts that _cio_file_up returns CIO_ERROR without closing
# the descriptor (cio_file.c:853-856), so every later up_force of that chunk
# fails at "file descriptor already exists" (cio_file.c:827-830).
# Counts hits of that arm (cio_file.c:828) and of "already mapped" (822).
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

break cio_file_update_size
commands
  silent
  if $in_up == 1 && $faults == 0
    set $faults = 1
    set var cf->fs_size = 0
    printf "[wd] INJECT one-shot cio_file_update_size -> CIO_ERROR (fstat failure arm) fd=%d path=%s\n", cf->fd, cf->path
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
  if $obs <= 6 || $obs % 1000 == 0
    printf "[wd] OBS up_force ret=%d obs=%d enter=%d returns=%d queued=%d fd_exists_arm=%d already_mapped_arm=%d\n", ret, $obs, $enter, $ret, $queued, $fd_exists, $already_mapped
  end
  if $obs >= 12000
    printf "[wd] CAP reached: up_force_obs=%d\n", $obs
    shell date +"[wd] TIME summary %s.%N"
    printf "[wd] SUMMARY run=E2 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d fd_exists_arm=%d already_mapped_arm=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $faults, $fd_exists, $already_mapped, $queued, $flushes
    kill
    quit
  end
  continue
end

run
printf "[wd] ENDED run=E2 enter=%d returns=%d up_force_obs=%d nonzero=%d injected=%d fd_exists_arm=%d already_mapped_arm=%d queued=%d engine_flushes=%d\n", $enter, $ret, $obs, $nonzero, $faults, $fd_exists, $already_mapped, $queued, $flushes
quit
