# CURRENT-MONKEY-LINKED-CONFIGURATION

Card base `a0ac2cae9` (FLUENT-BIT-LINK-PREPARE's tip). Brief: round 29.
It discharges `CURRENT-MONKEY-CONFIGURATION-NOT-LINKED`. User decision
(2026-11-08, verbatim): "Analyse linked config". The `current` route is to
analyse `lib/monkey` under Fluent Bit's own configuration, the one linked
into `bin/fluent-bit`, so that its homes can be derived. Before this card it
kept the 24 Monkey homes refused.

This commit holds the census (scope item 1). Later commits add the route
change, the checks and the records.

## Census

[`scripts/census.py`](scripts/census.py) reads the user's preparation at
Fluent Bit 046accf6 and writes nothing there. Output:
[`census.json`](census.json). Both databases are read:

- **standalone**: `monkey/monkey-build/compile_commands.json`, SHA-256
  `8c6779d0…61658075`. This is the forced-Libevent copy that the route reads
  through `--monkey-database`.
- **linked**: the `lib/monkey` entries of
  `fluent-bit-clang-production/compile_commands.json`, SHA-256
  `93e9e67e…ff99583`. This is the run's `--compile-database`, and its
  objects are what `bin/fluent-bit` links (FLUENT-BIT-LINK-PREPARE).

A unit is a Monkey unit by the route's own rule (`is_monkey_unit`): a `.c`
file under `mk_core/` but not `mk_core/deps/`, under `mk_server/`, or under
`tls/`. Each database has exactly one configuration per file. The census
preprocesses each command with Clang (`-E`, output under the worktree's
`private/`) and compares the results after rewriting each tree's roots. For
every top-level function definition in the unit's own text, or in a `.c`
file the unit includes textually (the event backend), it records whether the
preprocessed tokens are equal in both configurations, differ, or exist in
only one.

| Unit | Databases | Unit's functions equal / differ / only in one | What differs |
|---|---|---|---|
| `mk_core/mk_event.c` | both | 15 / 11 / 6 standalone-only | event backend: epoll (`mk_event_epoll.c`) when linked, Libevent (`mk_event_libevent.c` and the bundled `event2/` headers) when standalone. The 11 backend statics `_mk_event_{add,backend,channel_create,channel_destroy,del,inject,loop_create,loop_destroy,timeout_create,timeout_destroy,wait_2}` differ. `_mk_event_update`, `_mk_event_wait_with_flags`, `cb_event`, `cb_timeout`, `cb_wait_2_timeout` and `mk_event_libevent_socketpair` exist only in the standalone configuration |
| `mk_server/mk_lib.c` | both | 24 / 2 / 0 | `mk_lib_worker` and `mk_start` expand `mk_event_foreach` over the backend's `struct mk_event_ctx` (`events[i].data.ptr` for epoll, `fired[i].data` for Libevent) |
| `mk_server/mk_server.c` | both | 10 / 2 / 0 | `mk_server_loop_balancer` and `mk_server_worker_loop`: the same `mk_event_foreach` expansion |
| `mk_core/mk_file.c`, `mk_server/{mk_cache,mk_clock,mk_config,mk_fifo,mk_header,mk_http,mk_http_parser,mk_http_thread,mk_kernel,mk_mimetype,mk_net,mk_plugin,mk_scheduler,mk_socket,mk_stream,mk_user,mk_utils,mk_vhost,monkey}.c` (20 units) | both | every function equal (237 in all) | declarations only: the backend header (`mk_event_epoll.h` and `<sys/epoll.h>`, or `mk_event_libevent.h` and `event2/`) and therefore the layout of `struct mk_event_ctx`; no function text |
| `mk_core/{mk_iov,mk_memory,mk_rconf,mk_string,mk_utils}.c` (5 units) | both | every function equal (54 in all) | nothing: the whole preprocessed unit is token-equal |
| `tls/tls_openssl.c` | linked only | 27 linked-only | compiled under `MK_HAVE_TLS` (OpenSSL); it defines the `mk_tls_*` API |
| `tls/tls_stub.c` | standalone only | 5 standalone-only | the `MK_TLS=OFF` stub of `mk_tls_{enabled,exit,init,thread_init,transport}`. The linked configuration does not compile it: `tls/tls_openssl.c` defines those names there |

Totals: 30 files. 28 are in both databases, 1 is only linked and 1 is only
standalone. 5 of the 28 are whole-unit token-equal. 25 have equal text for
every unit function, and 3 compile different function text. All differences
come from the event backend and the TLS choice. No header inline function
differs in any unit.

How the commands differ, apart from paths:

- **Macros.** Every linked command adds Fluent Bit's global definitions
  (`FLB_*`, `JSMN_*`, `MPACK_EXTENSIONS=1`, `MSGPACK_EMBED_STACK_SIZE=64`,
  `NGHTTP2_STATICLIB`, `__FLB_FILENAME__=__FILE__`) and `MK_HAVE_REGEX` and
  `MK_HAVE_TLS`. The standalone `mk_core` commands add
  `MK_EVENT_LOOP_LIBEVENT`. The standalone `mk_server` units get it from the
  generated `mk_core_info.h`, which defines it. Both have the same
  `MK_HAVE_*` feature probes otherwise (accept4, backtrace, eventfd, memmem,
  memrchr, stat.h, sys/uio.h, timerfd_create, unistd.h, valgrind) and
  `MK_THREADS_POSIX`.
- **Includes.** The linked commands add Fluent Bit's include path, which
  names no Monkey header. The standalone commands add the bundled
  Libevent's `include/` (source and build). Monkey's generated headers
  (`mk_info.h`, `mk_env.h`, `mk_core_info.h`) come from each build's own
  `include/monkey`.
- **Other words.** Linked commands add a second `-Wall`, `-fPIC` and
  `-Wa,--noexecstack`. Standalone commands use `-fPIC` in `mk_server/` only
  and never `--noexecstack`. Both end with `-g -std=gnu17` (after
  `-std=gnu99`), and `-Wextra` is in both.
- **Plugins.** Neither configuration compiles a plugin among the route's
  units (`plugins/` is outside the Monkey unit rule in both). The linked
  database also compiles `plugins/{dirlisting,mandril}`, `mk_bin/`, `api/`
  and `deps/`. These are not Monkey units of the route in either
  configuration.
