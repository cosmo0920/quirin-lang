set pagination off
set confirm off
set $ac = 0
handle SIGABRT nostop print pass
handle SIGSEGV nostop print pass

# No fault: just count wrapper allocations to confirm two per fan-out chunk.
break sb_allocate_chunk
commands
  silent
  set $ac = $ac + 1
  printf "[pf] allocate #%d chunk=%p (no fault)\n", $ac, chunk
  continue
end

run
printf "[pf] control run returned; allocate_calls=%d\n", $ac
quit
