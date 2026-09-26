set pagination off
handle SIGTERM nostop pass
set $c1_done = 0
break flb_scheduler.c:283 if $c1_done == 0
commands
  silent
  # FAULT RUN: simulate the allocation failure
  set var request = 0
  set $c1_done = 1
  printf "C1: timer %p linked, request %p\n", timer, request
  continue
end
break flb_sched_destroy
commands
  silent
  set $h = &sched->timers
  set $n = $h->next
  set $c = 0
  while $n != $h
    set $t = (struct flb_sched_timer *)((char *)$n - (long)&((struct flb_sched_timer *)0)->_head)
    printf "C1: timer %p type=%d data=%p\n", $t, $t->type, $t->data
    set $c = $c + 1
    set $n = $n->next
  end
  printf "C1: %d timers on sched->timers at teardown\n", $c
  continue
end
run
