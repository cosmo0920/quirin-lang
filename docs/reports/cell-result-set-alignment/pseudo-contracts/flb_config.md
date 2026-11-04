# Pseudo contract: `flb_config` (`ch_data`, `ch_manager`, `ch_event.fd` at :607, `ch_notif`)

Evidence only. This file is not loaded and is not in force. The production text stays withheld (CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD). It is not a patch and not an upstream report.

## (a) As written (046accf6)

```
// PSEUDO CONTRACT: evidence only, not loaded, not in force.
// flb_config.ch_data[k]
//   creator = flb_pipe_create(config->ch_data)  flb_lib.c:181 (pipe on Linux)
//   live = [1, MAX] (declared policy) | POSIX [0, MAX];  absent = 0 (flb_calloc, flb_config.c:288)
//   test = if (config->ch_data[0])  i.e. ch_data[0] != 0   (flb_config.c:611/612, flb_config_exit)
//   declared: dead {0} & [1, MAX] = {} => decided; live [MIN,-1] | [1, MAX] & 0 = {} => decided (redundant)
//   POSIX:    dead {0} & [0, MAX] = 0 => NOT decided (WRONG): descriptor 0 read as absent
//   at the site: both ends leak when ch_data takes descriptor 0 (observed natively, (c))
// flb_config.ch_event.fd (#same(ch_manager[0]))
//   absent = -1 (MK_EVENT_ZERO, flb_config.c:297); test = if (config->ch_event.fd)  (:607)
//   dead {0}; live edge [MIN,-1] | [1, MAX] holds the absent -1 => NOT decided (WRONG on both columns)
//   at the site: the alias close of ch_manager[0] at :607, then :617/:618 closes it again (double close,
//   W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE); with the engine never started, close(-1).
// flb_config.ch_manager[0], ch_notif[0], ch_notif[1]
//   creator = mk_event_channel_create (flb_engine.c:1240; flb_lib.c:211)
//   absent = 0 (calloc); ch_notif[1] also -1 (flb_engine_failed)
//   tests  = ch_manager[0] > 0 (:617), ch_notif[0] > 0 (:625), ch_notif[1] > 0 && (:627),
//            ch_notif[1] <= 0 (flb_engine.c:1034, :1048)
//   declared: decided (exact);  POSIX: dead edge holds live 0 => NOT decided (WRONG): 2-fd leak, or a
//   skipped STARTED/FAILED write (flb_start waits) when a write end is on 0
```

## (b) Fixed shape

```
// absent = -1 at flb_config_init and after each close; tests > 0 (or != -1); no alias close
//   dead [MIN, 0] & [1, MAX] = {}; live [1, MAX] & -1 = {} => decided; verdict decided
```

## (c) Native evidence

These traces are recorded beside W43-DESC-FLB-CONFIG-DATA-PIPE-ZERO-GUARD and W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE. The runs were stdin closed, `-Y`, two SIGHUPs, then SIGTERM, traced with strace.

- **Pinned binary, 046accf6** (`private/native/logs/pinned-closed.strace`):
  - `ch_data` is `pipe2([0, 3])` (line 78). It is never closed, at either reload or at exit.
  - Teardowns show `close(13) = 0` then `close(13) = -1 EBADF`: the :607 alias, then :618.
  - No later channel takes 0.
- **Master 49592c4fd:** the same leak, and the same double close.
- **Patch v2 (`>= 0`):** ch_data is closed once, and after a reload descriptor 0 goes to the new epoll.
- **Patch v3 (`> 0`), `private/native/logs/v3-closed.strace`:**
  - fd 0, held by ch_data, is never closed. This is the leak the user accepted: USER DECISION 2026-11-04, verbatim, "Accept the leak".
  - The ch_manager double close is gone.
  - The 12 EBADFs left are timerfds, the same on every build.

## (d) Planned fix (third patch)

```
// ch_data: absent = -1 (flb_config.c:295-296 at 49592c4fd+v3; after each close :617/:621)
//   tests ch_data[0] > 0 (:615), ch_data[1] > 0 (:619)   — closed end by end with flb_pipe_close
//   declared: decided both edges; POSIX: dead [MIN, 0] holds 0 => NOT decided: an end on 0 stays 0 and is
//   never closed (the -1 store is inside the > 0 block) — observed natively, accepted by the user
// ch_manager / ch_notif: absent = -1 (:297-300; unconditional stores after the tests :632-634, :644-645)
//   tests ch_manager[0] > 0 (:625); ch_manager[1] > 0 && ch_manager[1] != ch_manager[0] (:628);
//         ch_notif[0] > 0 (:637); ch_notif[1] > 0 && … (:640)
//   the single tests: decided; the compound `&&` tests: the analyzer reads only an exact `r->G OP V`
//   test (rule 38c), so their close stays the access gate's refusal: NOT decided (test shape, not sets)
// :607: removed; ch_event.fd reset to -1 with the manager ends (:634) — no alias close, decided
// flb_engine.c untouched: ch_notif[1] <= 0 (:1037, :1051) now reads -1 (stored at init) as absent:
//   declared: decided (exact); POSIX: a live write end on 0 is read as "no channel" (race only)
```

Hunks (quoted):

```
-    if (config->ch_event.fd) {
-        mk_event_closesocket(config->ch_event.fd);
-    }
...
+    if (config->ch_data[0] > 0) {
+        flb_pipe_close(config->ch_data[0]);
+        config->ch_data[0] = FLB_UNINITIALIZED_SOCKET;
+    }
...
+    config->ch_manager[0] = FLB_UNINITIALIZED_SOCKET;
+    config->ch_manager[1] = FLB_UNINITIALIZED_SOCKET;
+    config->ch_event.fd = FLB_UNINITIALIZED_SOCKET;
```

Residual delta, as a suggestion only (the patch is not edited):
- The compound tests could be split the way the fixture `the_patched_unconditional_reset_keeps_its_protocol` reads them: `if (ch_manager[1] > 0) { if (ch_manager[1] != ch_manager[0]) close(...); }`.
- Under POSIX's set, a `>= 0` test (patch v2's) closes an end on descriptor 0.
