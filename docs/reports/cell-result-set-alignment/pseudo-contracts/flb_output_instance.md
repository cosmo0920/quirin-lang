# Pseudo contract: `flb_output_instance` (`ch_events[0..1]`, `event.fd`)

Evidence only. This file is not loaded and is not in force. It is not a patch and not an upstream report.

```
// PSEUDO CONTRACT: evidence only, not loaded, not in force.
// sets: live from libc/result_sets.qv `written_set(pipedes[k], [1, MAX]) when #result == 0`
//       (declared policy, USER DECISION 2026-11-04 "Write [1, MAX]"); POSIX column [0, MAX]
```

## (a) As written (046accf6; 66910c10 and 34155955 are the same up to line numbers)

```
// flb_output_instance.ch_events[k]  (046accf6)
//   creator = mk_event_channel_create(config->evl, &ins->ch_events[0], &ins->ch_events[1], ins)
//             flb_output.c:1545 (flb_output_init_all); slots <- pipe's descriptors
//   live    = [1, MAX] (declared) | POSIX [0, MAX]
//   absent  = 0 (flb_calloc, flb_output.c:822)
//   test    = ch_events[k] > 0   (flb_output.c:615 / :619, flb_output_instance_destroy)
//   declared: dead edge [MIN, 0] & [1, MAX] = {}             => decided absent
//             live edge [1, MAX] & 0 = {}                      => decided live      (exact)
//   POSIX:    dead edge [MIN, 0] & [0, MAX] = 0  (non-empty)   => NOT decided
//             (absent 0 is also a live value: no test of this end is exact)    (WRONG)
//   at the site: an end on descriptor 0 is never closed (leak), and its event registration stays.
```

Under the declared bound the analyzer reads it as decided. The release's `descriptor-cell-zero-undecided` (:638) goes, attributed to the bound. Under POSIX the fixture `posix_leaves_descriptor_zero_on_the_dead_edge` refuses with `descriptor-cell-zero-undecided`.

## (b) Fixed shape

```
// fixed: absent = -1 (store at :822 and after each close); test ch_events[k] > 0
//   dead edge [MIN, 0] & [1, MAX] = {} and [MIN, 0] & ([1, MAX] | -1) = -1 in absent => decided absent
//   live edge [1, MAX] & -1 = {}                                                   => decided live
//   verdict: the record keeps its protocol (fixture the_patched_shape_keeps_its_protocol: Safe)
```

## (c) Native evidence

This end was not observed on descriptor 0: `ch_data` holds 0 when stdin is closed (pinned binary 046accf6, `pipe2([0, 3])`). Its ends are 8 and up in every run (private/native/logs/pinned-closed.strace). The analysis here is a source reading for this end.

## (d) Planned fix (the user's third patch, sha256 c7cd578b…e385)

```
// after the patch: absent = -1 (flb_output.c:886-887 at 49592c4fd+v3, and after each close)
//                  live = [1, MAX]; test ch_events[k] > 0 (:645 / :650)
//   dead edge [MIN, 0] & [1, MAX] = {}  => decided   ;  live edge [1, MAX] & -1 = {} => decided
//   verdict: decided; the record keeps its protocol (planned module teardown_destroy_patch_v3.qv)
```

Hunk (quoted from the patch):

```
     if (ins->ch_events[0] > 0) {
         mk_event_closesocket(ins->ch_events[0]);
+        ins->ch_events[0] = FLB_UNINITIALIZED_SOCKET;
     }
...
+    instance->ch_events[0] = FLB_UNINITIALIZED_SOCKET;
+    instance->ch_events[1] = FLB_UNINITIALIZED_SOCKET;
```

Under POSIX's [0, MAX], the dead edge still holds descriptor 0. The -1 reset is inside the `> 0` block, so an end on 0 stays 0 and is never closed: the leak the user accepted ("Accept the leak"). `event.fd` is `#same(ch_events[0])`, so the patch has no test of its own.
