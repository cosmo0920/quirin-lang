set pagination off
handle SIGTERM nostop pass
break flb_output.c:892
commands
  silent
  set var instance->flags = instance->flags | 2048
  printf "C2: simulated FLB_OUTPUT_SYNCHRONOUS on output instance %p\n", instance
  continue
end
set $c2_done = 0
set $c2_task = 0
break flb_output.c:289 if $c2_done == 0 && retry != 0
commands
  silent
  # FAULT RUN ONLY: uncomment to simulate the allocation failure
  # set var queued_task = 0
  set $c2_done = 1
  set $c2_task = task
  printf "C2(retry): task %p users=%d queued_task=%p retry=%p\n", task, task->users, queued_task, retry
  continue
end
break flb_task_destroy if task == $c2_task
commands
  silent
  printf "C2(retry): task %p destroyed, users=%d\n", task, task->users
  bt 6
  continue
end
run
