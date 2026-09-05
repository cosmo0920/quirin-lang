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
| `include/event2/event.h` | blocking/nonblocking delete, finalization, and explicit activation public APIs |
| `event.c` | one running loop per base, nonblocking-delete distinction, finalizing events ignoring activation, and active-queue processing |
| `bufferevent.c` | unlocked callbacks require deferral, locked/unlocked deferred dispatch, and thread-safe locking setup |
| `include/event2/thread.h` | pthread initialization API |
| `epoll.c` | public epoll backend operation table and native wait path |

The command and result are:

```text
$ cargo run --quiet --example cycle18_libevent_audit
verified Libevent release-2.2.2-alpha-11-gd82464a2 at d82464a277d0f42703702c4dfd9af6af38595a83
checked include/event2/event.h
checked event.c
checked bufferevent.c
checked include/event2/thread.h
checked epoll.c
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

## QCore-C trust and limits

The trusted boundary for this run consists of the pinned headers and source
files above, CMake's generated configuration, Linux 7.0.0-30-generic x86-64,
GCC 15.2.0, pthreads, the selected EPOLL/POLL/SELECT backends, and application
callback summaries supplied to the Rust checker.  Rust `ObservedEvent` entries
are the observation-hook interface; the checker rejects reordered, duplicated,
or invented observable boundaries.

No native kqueue or Windows/IOCP result was available.  No ASan, UBSan, or
TSan result is recorded by this run.  The current repository supplies the
checked observation interface and refinement checker, but not a deployed
zero-global-lock C instrumentation hook for arbitrary Libevent applications.
Those gaps keep the aggregate Cycle 18 status short of full multi-platform
acceptance.
