# Cycle 18 pinned Libevent validation

## Source identity

Libevent is recorded as a Git submodule at `third_party/libevent`. A fresh
checkout can reproduce the audited source with:

```sh
git clone --recurse-submodules https://github.com/cosmo0920/qcore-c.git
# Or, in an existing checkout:
git submodule update --init --recursive
```

The Cycle 18 model pins the resulting source exactly:

| Item | Evidence |
| --- | --- |
| Upstream | `https://github.com/libevent/libevent.git` |
| Repository path | `third_party/libevent` |
| Commit | `d82464a277d0f42703702c4dfd9af6af38595a83` |
| Description | `release-2.2.2-alpha-11-gd82464a2` |
| Commit date | 2026-08-29 |
| Branch at clone | `master`, tracking `origin/master` |

`cycle18_libevent_audit` requires that exact `HEAD`; a later upstream commit
is rejected until the pin and source contract are reviewed together. It also
rejects tracked modifications in the submodule, so the files being audited are
the files represented by the gitlink rather than a locally altered checkout.

## Audited upstream contracts

The Rust audit reads these pinned files and requires the listed behavior
markers:

| File | Contract checked |
| --- | --- |
| `include/event2/event.h` | blocking/nonblocking delete, finalization, explicit activation, and Windows IOCP-startup public APIs |
| `event.c` | one running loop per base, nonblocking-delete distinction, finalizing events ignoring activation, and active-queue processing |
| `bufferevent.c` | unlocked callbacks require deferral, locked/unlocked deferred dispatch, and thread-safe locking setup |
| `include/event2/thread.h` | pthread and Windows-thread initialization APIs |
| `include/event2/util.h` | socket-pair creation, portable socket close, and socket-error query APIs added by the Cycle 19 Monkey composition |
| `evutil.c` | socket-pair rollback and platform-specific close/error implementations used by those contracts |
| `epoll.c` | public epoll backend operation table and native wait path |
| `kqueue.c` | public kqueue backend operation table, native initialization, and native wait path |
| `win32select.c` | public WIN32 backend operation table and native `select` dispatch path |
| `event_iocp.c` | completion-port creation, association, completion wait, activation, and shutdown paths |
| `buffer_iocp.c` | overlapped evbuffer read and write launch paths |
| `bufferevent_async.c` | IOCP-backed asynchronous bufferevent construction and `ConnectEx` path |

The command and result are:

```text
$ cargo run --quiet --example cycle18_libevent_audit
verified Libevent release-2.2.2-alpha-11-gd82464a2 at d82464a277d0f42703702c4dfd9af6af38595a83
checked include/event2/event.h
checked event.c
checked bufferevent.c
checked include/event2/thread.h
checked include/event2/util.h
checked evutil.c
checked epoll.c
checked kqueue.c
checked win32select.c
checked event_iocp.c
checked buffer_iocp.c
checked bufferevent_async.c
```

This is a reproducible source-contract audit, not a proof of the upstream C
implementation.

The same audit is an integration test, so CI can use the normal workspace test
command after initializing submodules:

```sh
cargo test --workspace
```

For a focused failure signal, run
`cargo test -p qcore-compiler --test libevent_submodule`.

## Linux native build manifest

The out-of-tree build directory was `/tmp/qcore-cycle18-libevent`.  It was
configured with:

```sh
cmake -S third_party/libevent -B /tmp/qcore-cycle18-libevent \
  -DCMAKE_BUILD_TYPE=Debug \
  -DEVENT__DISABLE_OPENSSL=ON \
  -DEVENT__DISABLE_MBEDTLS=ON \
  -DEVENT__DISABLE_THREAD_SUPPORT=OFF \
  -DEVENT__DISABLE_TESTS=OFF \
  -DEVENT__DISABLE_REGRESS=OFF \
  -DEVENT__DISABLE_BENCHMARK=ON \
  -DEVENT__DISABLE_SAMPLES=ON \
  -DEVENT__ENABLE_VERBOSE_DEBUG=ON
cmake --build /tmp/qcore-cycle18-libevent --parallel 2
ctest --test-dir /tmp/qcore-cycle18-libevent --output-on-failure --parallel 2
```

The configuration selected Linux `EPOLL`, `SELECT`, and `POLL`, pthreads,
GNU C 15.2.0, and Libevent verbose debug mode.  OpenSSL and MbedTLS were
disabled.  The static/shared core, extra, pthread, and regression targets
built successfully.

Native test result:

```text
100% tests passed, 0 tests failed out of 84
Total Test time (real) = 766.12 sec
```

The generated matrix exercised timerfd/changelist/signalfd epoll variants,
select, poll, debug variants, and bufferevent rate-limit tests.  The checkout
remained clean after generation and execution; generated regression sources
are upstream-ignored artifacts.

## macOS native kqueue build manifest

The out-of-tree build directory was
`/tmp/qcore-cycle18-libevent-kqueue-default`. It was configured on macOS
26.6.2 (Darwin 25.6.0, arm64) with CMake 4.3.2 and Apple Clang 21.0.0:

```sh
cmake -S third_party/libevent \
  -B /tmp/qcore-cycle18-libevent-kqueue-default \
  -DCMAKE_BUILD_TYPE=Debug \
  -DEVENT__DISABLE_OPENSSL=ON \
  -DEVENT__DISABLE_MBEDTLS=ON \
  -DEVENT__DISABLE_THREAD_SUPPORT=OFF \
  -DEVENT__DISABLE_TESTS=OFF \
  -DEVENT__DISABLE_REGRESS=OFF \
  -DEVENT__DISABLE_BENCHMARK=ON \
  -DEVENT__DISABLE_SAMPLES=ON \
  -DEVENT__ENABLE_VERBOSE_DEBUG=ON
cmake --build /tmp/qcore-cycle18-libevent-kqueue-default --parallel 2
ctest --test-dir /tmp/qcore-cycle18-libevent-kqueue-default \
  --output-on-failure --parallel 2 -R 'KQUEUE|kq-collision'
ctest --test-dir /tmp/qcore-cycle18-libevent-kqueue-default \
  --output-on-failure --parallel 2
```

Configuration selected `KQUEUE`, `POLL`, and `SELECT`, pthreads, and Libevent
verbose debug mode. `EVENT__HAVE_WORKING_KQUEUE` was defined. Each `__KQUEUE`
CTest entry set `EVENT_NOSELECT=1` and `EVENT_NOPOLL=1`, preventing fallback to
another backend. The build compiled `kqueue.c` into the static and shared core
libraries.

The focused native kqueue matrix passed:

```text
100% tests passed, 0 tests failed out of 21
Total Test time (real) = 191.47 sec
```

This included `test-test-kq-collision`, normal and debug regression runs, and
the generated signal-mode variants. The regression traces contained native
`kq_dispatch`/`kevent` activity.

The complete native backend matrix also passed:

```text
100% tests passed, 0 tests failed out of 65
Total Test time (real) = 589.48 sec
```

In addition to the forced-kqueue tests, the complete run exercised select,
poll, debug and generated signal-mode variants, and bufferevent rate-limit
tests.

CMake's optional `EVENT__FORCE_KQUEUE_CHECK=ON` probe was also investigated.
With CMake 4.3.2 its generated executable compiled and returned exit code 0,
but `check_c_source_runs` nevertheless left `EVENT__HAVE_WORKING_KQUEUE`
false. That configuration is unsuitable for validation because Libevent then
omits kqueue from its runtime backend table. The normal native macOS path sets
the feature explicitly, and the forced-backend CTest matrix above supplies the
runtime evidence.

## Windows native MSVC and IOCP build manifest

The out-of-tree build directory was `target/cycle18-libevent-msvc`. It was
configured on Windows 10.0.26200.9168 x64 with CMake 4.3.4, the Visual Studio
18 2026 generator, Windows SDK 10.0.26100.0, and native MSVC 19.51.36248:

```text
cmake -S third_party/libevent -B target/cycle18-libevent-msvc \
  -G "Visual Studio 18 2026" -A x64 \
  -DEVENT__DISABLE_OPENSSL=ON \
  -DEVENT__DISABLE_MBEDTLS=ON \
  -DEVENT__DISABLE_THREAD_SUPPORT=OFF \
  -DEVENT__DISABLE_TESTS=OFF \
  -DEVENT__DISABLE_REGRESS=OFF \
  -DEVENT__DISABLE_BENCHMARK=ON \
  -DEVENT__DISABLE_SAMPLES=ON \
  -DEVENT__ENABLE_VERBOSE_DEBUG=ON
cmake --build target/cycle18-libevent-msvc --config Debug --parallel 4
```

Configuration selected the `WEPOLL` and `WIN32` event backends and Windows
thread support. The core static/shared builds compiled `win32select.c`,
`event_iocp.c`, `buffer_iocp.c`, `bufferevent_async.c`, and
`evthread_win32.c`. IOCP is a completion dispatcher used by asynchronous
bufferevents and listeners rather than the name returned by
`event_base_get_method`; the focused regression group explicitly starts and
exercises that dispatcher while the event base uses the WIN32 backend.

The focused native IOCP command was:

```text
target/cycle18-libevent-msvc/bin/Debug/regress.exe \
  --verbose --timeout 120 iocp/..
```

All 21 IOCP tests passed. They covered the completion port, overlapped
evbuffers, asynchronous bufferevents, flush modes, watermarks, filters,
connection success/failure, nonblocking connection, listeners, and HTTP.

The complete generated Windows backend matrix was then run with:

```text
ctest --test-dir target/cycle18-libevent-msvc -C Debug \
  --output-on-failure --parallel 4
```

The result was:

```text
100% tests passed, 0 tests failed out of 44
Total Test time (real) = 326.86 sec
```

The complete run forced both WEPOLL and WIN32 variants, including verbose
debug variants, and ran the bufferevent rate-limit cases. The Windows/IOCP
platform result is recorded independently of the Linux and macOS runs.

## QCore-C trust and limits

The trusted boundary for these runs consists of the pinned headers and source
files above, CMake's generated configurations, Linux 7.0.0-30-generic x86-64
with GCC 15.2.0, macOS 26.6.2 arm64 with Apple Clang 21.0.0, Windows
10.0.26200.9168 x64 with MSVC 19.51.36248, pthreads or Windows threads as
applicable, the selected EPOLL/POLL/SELECT, KQUEUE/POLL/SELECT, and
WEPOLL/WIN32-plus-IOCP backends, and application callback summaries supplied
to the Rust checker. Rust `ObservedEvent` entries are the observation-hook
interface; the checker rejects reordered, duplicated, or invented observable
boundaries.

No ASan, UBSan, or TSan result is recorded for the pinned Libevent builds. The
current repository supplies the
checked observation interface and refinement checker, but not a deployed
zero-global-lock C instrumentation hook for arbitrary Libevent applications.
Those gaps keep the aggregate Cycle 18 status short of full acceptance.
