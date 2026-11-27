set pagination off
set confirm off
set $collector = 0
set $enter = 0
set $ret = 0
set $ups = 0

break cb_queue_chunks
commands
  silent
  set $collector = 1
  set $enter = $enter + 1
  printf "[wd] ENTER cb_queue_chunks entercount=%d upsofar=%d retsofar=%d\n", $enter, $ups, $ret
  continue
end

break sb.c:724
commands
  silent
  set $ret = $ret + 1
  printf "[wd] RETURN cb_queue_chunks retcount=%d entercount=%d ups=%d\n", $ret, $enter, $ups
  continue
end

# Observe (do not fault) up_force calls from the collector.
break cio_chunk_up_force
commands
  silent
  if $collector == 1
    set $ups = $ups + 1
    printf "[wd] up_force call n=%d (enter=%d ret=%d) -- passthrough\n", $ups, $enter, $ret
  end
  continue
end

run
printf "[wd] CONTROL ended: enters=%d returns=%d up_force_calls=%d\n", $enter, $ret, $ups
quit
