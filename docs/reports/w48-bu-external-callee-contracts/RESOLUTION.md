# Callee resolution and declaration review

The twelve refreshed preprocessing probes in `prepared-resolution.json` all
exit 0, as do the earlier twelve probes in `resolution-preliminary.json`. The
refreshed corpus `mk_iov.c` probe uses the selected standalone Monkey compile
database; the preliminary corpus probe used the embedded Monkey configuration
and does not establish the standalone configuration. `prepared-resolution-checks.json`
records the corrected configured-declaration checks.
Each uses the recorded compile command, with its compilation action replaced by
preprocessing, rather than guessing the platform from unpreprocessed source.
The Fluent Bit corpus source is `66910c10`; the system source is `046accf6`; the slice is
the actual archive-import commit `34155955` (tree `8d1f3cd7`). The original
`90978ac6` object is absent locally, so no ancestry check against it is claimed.

| Route | writev | thrd_current and mtx_destroy |
|---|---|---|
| Corpus | POSIX external declaration from sys/uio.h | system C11 threads.h, WITH_C11THREADS=1 |
| System | same configured external declaration | same configured system C11 declarations; these source units are not selected |
| Tail-pause | same configured external declaration | same configured system C11 declarations; these source units are not selected |

In pinned `lib/librdkafka-2.15.0/src/tinycthread.h:35`, WITH_C11THREADS selects
`<threads.h>`; bundled declarations are the other branch. `tinycthread.c:28–932`
wraps its implementations in `#if !WITH_C11THREADS`. Raw source discovery can
therefore list names whose definitions the configured analyzer does not see.
The fresh BASE and TIP native rows confirm zero analyzed functions and 30 raw
discovered names in this unit. The planner's raw home names do not establish
analyzed bodies. No bundled-body contract is
justified by these configured routes.

`writev` gathers bytes from caller-provided buffers to a descriptor and returns
a scalar byte count or failure, which can set errno. The custody statement is
borrowed fd/iovec/count and no resource result: neither vector nor reachable
buffers transfer custody or are retained. This does not promise purity, successful
output, a full write, errno preservation, buffer bounds, null safety, or freedom
from cancellation. The existing ABI reader must still accept the actual
`ssize_t (int, const struct iovec *, int)` declaration. Source:
[POSIX.1-2017 licensed reproduction](https://man7.org/linux/man-pages/man3/writev.3p.html).
Direct Open Group retrieval returned HTTP 403.

`thrd_current` returns the current thread identifier ([C11 N1570 §7.26.5.2](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)).
The configured host headers declare `typedef unsigned long int __thrd_t;`
and `typedef __thrd_t thrd_t;`; on the audited LP64 target this supports the
u64 scalar projection, with no custody result. A pointer-valued or incompatible
identifier ABI must withdraw it. This is no thread-creation, join, synchronization
or thread-lifetime claim and supplies no owning handle.

`mtx_destroy` destroys resources used by the mutex within caller-owned storage;
it does not free that storage (N1570 §7.26.4.1). No mutex resource kind exists in
the current custody vocabulary. The existing in-place kind binds an automatic
object addressed by `&NAME`, whereas this actual is a global mutex whose init
and destruction occur in different bodies. Borrowed would erase the lifecycle;
heap Released would free the wrong obligation. Decline a declaration and retain
Unknown under `CG25-IN-PLACE-RESIDUAL-PAIRS`, whose discharge condition already
requires a kind and storage/owner protocol with audited Lean and a measurement.
The configured C11 `thrd_success` is 0 on all three routes
(`configured-thread-success.json`); the older entry's bundled-tinycthread value
is not the active status ABI here. That removes no missing kind/global-owner
obligation. The earlier entry retains its dated checkpoint claims.

Both implemented declarations remain libc-owned, revision flexible declared
assumptions under the existing standard/ABI trust boundary. They do not prove
libc implementation bodies, source extraction, Rust–Lean correspondence or a
whole C program. They add no semantic effect kind or gateway admission rule;
all existing Lean audits remain required. The earlier contract-only W46 report
records the same existing-kind proof boundary explicitly.
