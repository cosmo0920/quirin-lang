set pagination off
set confirm off
set $ac = 0
set $faultaddr = 0
set $firstaddr = 0
handle SIGABRT nostop print pass
handle SIGSEGV nostop print pass

# Force the SECOND wrapper allocation for the first fan-out chunk to fail.
break sb_allocate_chunk
commands
  silent
  set $ac = $ac + 1
  if $ac == 1
    set $firstaddr = chunk
    printf "[pf] allocate #1 chunk=%p (first wrapper, will be left charged/stale)\n", chunk
  end
  if $ac == 2
    set $faultaddr = chunk
    printf "[pf] allocate #2 chunk=%p first=%p -> FORCING NULL (drop 2nd wrapper)\n", chunk, $firstaddr
    return 0
  end
  continue
end

# Log when the physical chunk of the fan-out victim is closed/freed.
break cio_chunk_close
commands
  silent
  if ch == $faultaddr
    printf "[pf] cio_chunk_close CLOSING victim ch=%p delete=%d -> frees physical chunk; first wrapper (=%p) now stale\n", ch, delete, $faultaddr
  end
  continue
end

run
printf "[pf] run returned (program stopped or exited)\n"
bt
printf "[pf] end firstaddr=%p faultaddr=%p allocate_calls=%d\n", $firstaddr, $faultaddr, $ac
quit
