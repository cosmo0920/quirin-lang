# gdb_joins.gdb -- independent cross-check of the interposer: break on every
# pthread_join call, print the thread argument (the first
# integer argument register on x86-64) and the two calling frames, continue;
# on a signal, print the backtrace.
set pagination off
set confirm off
set print thread-events off
set breakpoint pending on
break pthread_join
commands
  silent
  printf "GDBJOIN thread=%#lx\n", $rdi
  bt 3
  continue
end
run
printf "GDBEXIT\n"
info program
bt 8
