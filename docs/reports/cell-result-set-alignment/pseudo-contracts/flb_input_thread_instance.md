# Pseudo contract: `flb_input_thread_instance` (`ch_parent_events[0..1]`, `ch_thread_events[0..1]`)

Evidence only. This file is not loaded and is not in force. It is not a patch and not an upstream report.

## (a) As written (046accf6)

```
// PSEUDO CONTRACT: evidence only, not loaded, not in force.
// flb_input_thread_instance.ch_parent_events[k] / ch_thread_events[k]
//   creator = mk_event_channel_create(…, &thi->ch_parent_events[0], &thi->ch_parent_events[1], …)
//             (input_thread_instance_create, flb_input_thread.c:272 / :285)
//   live    = [1, MAX] (declared policy) | POSIX [0, MAX]
//   absent  = 0 (flb_calloc, flb_input_thread.c:235)
//   test    = > 0   (flb_input_thread.c:210, :213, :218, :221, input_thread_instance_destroy)
//   declared: both edges decided (exact);  POSIX: dead [MIN, 0] & [0, MAX] = 0 => NOT decided (WRONG)
//   at the site: an end on descriptor 0 is never closed (leak).
```

Under the declared bound, `zero-undecided` at `flb_free` (:226) goes, attributed to the bound.

## (b) Fixed shape

```
// absent = -1 at allocation and after each close; test > 0 => both edges decided; verdict decided
```

## (c) Native evidence

Not observed on descriptor 0 (the `dummy` input runs without a thread). Source reading only.

## (d) Planned fix (third patch)

```
// absent = -1 (flb_input_thread.c:245-248 at 49592c4fd+v3, and after each close); tests > 0 (:211, :215, :221, :225)
//   dead [MIN, 0] & [1, MAX] = {}; live [1, MAX] & -1 = {} => decided; verdict decided
```

Hunk (quoted):

```
+    thi->ch_parent_events[0] = FLB_UNINITIALIZED_SOCKET;
+    thi->ch_parent_events[1] = FLB_UNINITIALIZED_SOCKET;
+    thi->ch_thread_events[0] = FLB_UNINITIALIZED_SOCKET;
+    thi->ch_thread_events[1] = FLB_UNINITIALIZED_SOCKET;
```

The flag group (`notification_channels_initialized == 1`) is untouched by the patch.
