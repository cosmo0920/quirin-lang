set pagination off
set confirm off
handle SIGTERM nostop pass
break flb_output.c:892
commands
  silent
  set var instance->flags = instance->flags | 2048
  printf "C3: simulated FLB_OUTPUT_SYNCHRONOUS on output instance %p\n", instance
  continue
end
set $c3_done = 0
set $c3_task = 0
set $c3_forced = 0
break flb_output.c:330 if $c3_done == 0 && queued_task->retry != 0
commands
  silent
  set $c3_task = queued_task->task
  printf "C3: queued retry flush of task %p, users=%d (before forced flush failure)\n", $c3_task, $c3_task->users
  set $c3_done = 1
  continue
end
break flb_output_task_flush if task == $c3_task && $c3_forced == 0
commands
  silent
  set $c3_forced = 1
  printf "C3: forcing flb_output_task_flush(task=%p) to return -1 at entry (simulated flush-creation/pipe-write failure)\n", task
  return -1
  continue
end
break flb_task_destroy if task == $c3_task
commands
  silent
  printf "C3: task %p destroyed, users=%d\n", task, task->users
  bt 6
  continue
end
run
