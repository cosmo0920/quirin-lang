set pagination off
set confirm off
set print address off
set $collector = 0
set $enter = 0
set $ret = 0
set $intercept = 0

# Mark when the backlog collector callback is entered
break cb_queue_chunks
commands
  silent
  set $collector = 1
  set $enter = $enter + 1
  printf "[wd] ENTER cb_queue_chunks entercount=%d interceptsofar=%d retsofar=%d\n", $enter, $intercept, $ret
  continue
end

# Detect the collector returning (yield). Break at the closing return of the callback.
break sb.c:724
commands
  silent
  set $ret = $ret + 1
  printf "[wd] RETURN cb_queue_chunks retcount=%d entercount=%d intercept=%d\n", $ret, $enter, $intercept
  continue
end

# Fault injection: persistent CIO_ERROR from up_force, but only once the collector is live.
break cio_chunk_up_force
commands
  silent
  if $collector == 1
    set $intercept = $intercept + 1
    printf "[wd] INTERCEPT up_force -> CIO_ERROR n=%d (enter=%d ret=%d)\n", $intercept, $enter, $ret
    return -1
    if $intercept >= 400
      printf "[wd] CAP reached: intercepts=%d collector-returns=%d collector-enters=%d\n", $intercept, $ret, $enter
      printf "[wd] VERDICT: cb_queue_chunks did NOT return between enter and cap under persistent CIO_ERROR\n"
      kill
      quit
    end
    continue
  else
    continue
  end
end

run
printf "[wd] program ended without hitting cap: enters=%d returns=%d intercepts=%d\n", $enter, $ret, $intercept
quit
