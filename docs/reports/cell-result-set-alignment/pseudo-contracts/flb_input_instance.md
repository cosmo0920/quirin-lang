# Pseudo contract: `flb_input_instance` (`ch_events[0..1]`, `ingress_queue_channels[0..1]`)

Evidence only. This file is not loaded and is not in force. It is not a patch and not an upstream report.

## (a) As written (046accf6)

```
// PSEUDO CONTRACT: evidence only, not loaded, not in force.
// flb_input_instance.ch_events[k]
//   creator = mk_event_channel_create(evl, &ins->ch_events[0], &ins->ch_events[1], ins)  flb_input.c:1380
//   live    = [1, MAX] (declared policy) | POSIX [0, MAX]
//   absent  = 0 (flb_calloc, flb_input.c:394)
//   test    = ch_events[k] > 0   (flb_input.c:1311 / :1315, flb_input_instance_destroy)
//   declared: dead [MIN, 0] & [1, MAX] = {} => decided; live [1, MAX] & 0 = {} => decided   (exact)
//   POSIX:    dead [MIN, 0] & [0, MAX] = 0 => NOT decided                                      (WRONG)
// flb_input_instance.ingress_queue_channels[k]
//   creator = flb_pipe_create(ins->ingress_queue_channels)  flb_input_ingest.c:488 (pipe on Linux)
//   absent  = -1 (flb_input.c:549-550; flb_input_ingest.c:493-494); calloc's 0 superseded
//             (every return before :549 frees the instance directly). Declared absent_set(0 | -1).
//   test    = ingress_queue_channels[k] > 0   (flb_input.c:1319 / :1323)
//   declared: decided both edges (exact);  POSIX: dead [MIN, 0] holds live 0 => NOT decided (WRONG)
//   at the site: an end on descriptor 0 is never closed (leak).
```

Under the declared bound, `zero-undecided` at the release (:1353) goes, attributed to the bound. The POSIX fixture refuses.

## (b) Fixed shape

```
// absent = -1 stored at allocation and after each close; test > 0
//   dead [MIN, 0] & [1, MAX] = {}; live [1, MAX] & -1 = {} => both decided
//   verdict: decided (the_patched_shape_keeps_its_protocol; the_ingress_minus_one_stores_keep_the_protocol)
```

## (c) Native evidence

Not observed on descriptor 0 (`ch_data` holds 0 with stdin closed). Source reading only.

## (d) Planned fix (third patch)

```
// ch_events: absent = -1 (flb_input.c:401-402 at 49592c4fd+v3, and after each close); test > 0 (:1315 / :1320)
// ingress:   absent = -1 (master's own :553-554, still late but safe; after each close :1327 / :1332); test > 0 (:1325 / :1330)
//   dead [MIN, 0] & [1, MAX] = {} => decided; live [1, MAX] & -1 = {} => decided; verdict: decided
```

Hunks (quoted):

```
+        instance->ch_events[0] = FLB_UNINITIALIZED_SOCKET;
+        instance->ch_events[1] = FLB_UNINITIALIZED_SOCKET;
...
     if (ins->ingress_queue_channels[0] > 0) {
         mk_event_closesocket(ins->ingress_queue_channels[0]);
+        ins->ingress_queue_channels[0] = FLB_UNINITIALIZED_SOCKET;
```

The patch has no second -1 block in flb_input.c (re-checked at 49592c4fd+v3). A `!= -1` test would not need the ingress store moved up today: no path reaches the destroyer before it.
