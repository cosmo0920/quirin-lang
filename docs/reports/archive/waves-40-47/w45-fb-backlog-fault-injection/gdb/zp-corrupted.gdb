set pagination off
set confirm off
set $collector = 0
set $enter = 0
set $ret = 0
set $intercept = 0

break cb_queue_chunks
commands
  silent
  set $collector = 1
  set $enter = $enter + 1
  printf "[wd] ENTER cb_queue_chunks entercount=%d interceptsofar=%d retsofar=%d\n", $enter, $intercept, $ret
  continue
end

break sb.c:724
commands
  silent
  set $ret = $ret + 1
  printf "[wd] RETURN cb_queue_chunks retcount=%d entercount=%d intercept=%d\n", $ret, $enter, $intercept
  continue
end

# Force CIO_CORRUPTED (-3) from the collector's up_force: exercises the wrapper-removal path.
break cio_chunk_up_force
commands
  silent
  if $collector == 1
    set $intercept = $intercept + 1
    printf "[wd] INTERCEPT up_force -> CIO_CORRUPTED n=%d (enter=%d ret=%d)\n", $intercept, $enter, $ret
    return -3
    if $intercept >= 400
      printf "[wd] CAP reached corrupted: intercepts=%d returns=%d enters=%d\n", $intercept, $ret, $enter
      kill
      quit
    end
    continue
  else
    continue
  end
end

run
printf "[wd] CORRUPTED ended: enters=%d returns=%d intercepts=%d\n", $enter, $ret, $intercept
quit
