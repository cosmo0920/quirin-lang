# W43-LIBC-CONTRACTS: trusted libc/POSIX contracts, measured first and placed only in the libc crate

Card W43-LIBC-CONTRACTS, 2026-09-23, on base `9edad50c` (main after
QUIRIN-PROVIDER-SHARED). After W43-RETURN-EXPRESSION, 315 Unknown functions
were blocked only by one uncontracted callee. This card measures which callees
of the whole uncontracted-call family are C library or POSIX functions,
declares the ones the v1 vocabulary can state exactly, and refuses the rest by
name. The declarations live only in `providers/quirin-provider-libc`, and a new
test in the standing gate enforces that.

This is **pure contract data**. The card changes no loader, checker, gateway or
backend code, so there is no semantic change and no Lean obligation. The Rust
changes are the libc crate's `CONTRACTS` index and deliberate test pins. Every
declaration is a trusted assumption about libc, as trust boundary 28 requires,
and each one is listed [below](#declared-contracts-and-their-assumptions).

## Headline

* **Inventory.** The baseline's uncontracted-call family has **2,721** distinct
  callees and 12,685 leaves. **132** of them are libc/POSIX functions: 128
  external glibc prototypes and 4 glibc header-inline bodies. Together they
  hold 3,422 leaves, block 1,192 function slots and are the only blocker of
  **45** Unknown functions. The rest are excluded: 2,462 are corpus-defined
  (262 sole-blocked), 69 come from other system libraries such as OpenSSL,
  curl, zlib and SASL, 13 are builtins and 45 are unresolved names. See
  [libc-inventory.md](libc-inventory.md) and [the next section](#1-inventory).
* **Declared: 55** in eleven new modules. 53 are the neutral element (every
  parameter borrowed, `resource_return_none()`). `strdup` returns
  `resource_return_owned(nullable)`, and `prctl` applies only under the literal
  premise `option == 15` (`PR_SET_NAME`). They decide 229 leaves and the sole
  blocker of 19 functions.
* **Refused: 77**, each for a named reason:
  * 21 return an alias (`memcpy`, `memset`, `strcpy`, `strchr`, ...).
  * 14 are faithful, but the declared-ABI reader **withdraws** them because a
    qualifier follows the `*`. This group includes the whole printf family,
    and it answers `LOADER-ABI-RESTRICT-QUALIFIER`'s measurement.
  * 6 have other reader shapes.
  * 17 create or release descriptors or handles.
  * 8 have an in-place lifecycle.
  * 2 do not return.
  * 3 invoke callbacks, 1 retains a pointer and 1 writes through a
    pointer-to-pointer.
  * 4 are header-inline bodies that the unit can check.
* **Arbiter.** BEFORE reproduces the W43-RETURN-EXPRESSION diagnostics **byte
  for byte**. AFTER removes 314 leaves and adds 27. The attribution script
  assigns all 341 changed leaves: 229 removed at a declared callee, 4 removed
  at the validated summary of a newly Safe function, 86 on the same analysed
  line as a decided call, and 22 elsewhere in a decided function. **22
  functions move from Unknown to Safe.** 0 functions move in any other
  direction, 0 unit statuses move, and the leak pin holds at **272**. The
  baseline moves to [artifacts/](artifacts/), and
  `w43-return-expression/artifacts/` is frozen as its predecessor.
* **Placement test.** `scripts/tests/test_contract_corpus.py` fails if any
  contract module outside `quirin-provider-libc` declares a libc/POSIX
  function. It checks the canonical corpus and every `.qcore` under
  `providers/`. The function names come from the generated
  `providers/quirin-provider-libc/libc-names.txt` (2,762 names) plus every
  name the crate itself declares. A synthetic negative control proves it can
  fail.
* **Follow-ups 285 → 294.** Ten residuals are registered, and
  `LOADER-ABI-RESTRICT-QUALIFIER` is retired against this card's measurement.

| Measure | BEFORE (W43-RETURN-EXPRESSION) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 223 / 84 / 4,527 | **245 / 84 / 4,505** | +22 / 0 / −22 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 49,371 | **49,094** | −277 |
| `unknown_alias` | 4,665 | **4,655** | −10 |
| `leak` / `summary_conflict` / `double_release` | 272 / 1 / 0 | 272 / 1 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole | 12,021 / 2,796 / 414 | 11,801 / 2,728 / 397 | −220 / −68 / −17 |
| Unknown functions blocked only by one uncontracted callee | 315 (235 callees) | 306 (233 callees) | −9 |
| return-expression leaves / functions / sole | 499 / 422 / 145 | 497 / 420 / 144 | −2 / −2 / −1 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 96 / 33 / 59 | **107 / 44 / 114** | +11 / +11 / +55 |

The uncontracted-call row counts Unknown functions, as the
[blocker census](blocker-census.txt) does. The inventory's 12,685 leaves span
all functions.

## 1. Inventory

**Method** ([`scripts/libc_inventory.py`](scripts/libc_inventory.py)). A
private run of the exact audit command captured each unit's line-marked
preprocessing through a transparent Clang wrapper. The run's diagnostics are
byte-identical to BEFORE, so the capture did not perturb it. Each unit was
then scanned with a small C lexer that tracks the current file and brace
depth. For every callee named by a `call to uncontracted function` leaf, the
lexer records every file-scope `NAME (` occurrence: a declaration, or a
definition when the parentheses close before `{`. `dpkg -S` names the Debian
package that owns each declaring system header.

A callee is **libc/POSIX** when all of these hold:

* a header owned by `libc6-dev` or `linux-libc-dev` declares it;
* no analysed unit carries a file-scope definition of it;
* no corpus text declares it.

Forty-three such callees also have a textual body somewhere in the pinned tree
that no pinned unit compiles, so they are classed `libc-with-tree-body` and
listed with those bodies. The bodies are Windows or non-C11 compatibility
code: `winpthreads.c`, `tinycthread.c`, `flb_dlfcn_win32.c`, `win32_glob.c`,
`flb_compat.h` Windows arms, and a `sockem.c` test interposer. They are
libc/POSIX in the audited build. The four glibc `static __inline` header
functions are classed `libc-inline`.

| Class | Callees | Leaves | Sole-blocked | Disposition |
|---|---:|---:|---:|---|
| `libc` + `libc-with-tree-body` | 85 + 43 | 2,823 | 45 | decided per callee ([table](libc-inventory.md)) |
| `libc-inline` (`__bswap_16/32/64`, `__uint64_identity`) | 4 | 599 | 0 | refused (header body; see below) |
| `corpus` | 2,462 | 8,849 | 262 | **excluded**: contracting a checkable corpus body would replace it with a trusted assumption |
| `other-system` (OpenSSL 55, curl 8, zlib 5, SASL 1) | 69 | 151 | 2 | excluded: not libc/POSIX, and outside the placement rule |
| `builtin` (`__builtin_va_*`, `__builtin_memcpy`, `_Generic`, `__asm__`, ...) | 13 | 193 | 2 | excluded: not functions |
| `unresolved` (callback parameters, protobuf-c `*__init` macros, `__atomic_*`) | 45 | 70 | 4 | excluded: no file-scope declaration found |

The corpus callees deliberately excluded include the largest sole blockers:
`LZ4_decompress_generic` (9), `cmt_mpack_unpack_array` (5),
`ctr_mpack_unpack_array` (5), `LZ4_compress_HC` and `LZ4_compress_HC_extStateHC`
(4 each), `rd_kafka_admin_result_ret_groups` (4), the LZ4 streaming entry points,
the cJSON item helpers, `add`, nanopb's `pb_write` and `rd_slice_reader0`. The
top 25 of each excluded class, with their declaring or defining file, are in
[libc-inventory.json](libc-inventory.json) under `excluded`. The full
2,721-row classification is in [inventory.json.gz](inventory.json.gz).

## 2. Declared contracts and their assumptions

Every declaration below is a **trusted assumption** about the C library. It
verifies no body. Each was read from the prototype the analysed units declare
after preprocessing (glibc on Ubuntu, x86-64, LP64), and each module's own
comment carries the prototypes, the custody reading and a "what no
declaration here says" list. Numbers in parentheses give functions blocked and
sole-blocked at the baseline.

| Module | Declarations | Assumption stated |
|---|---|---|
| `libc/string.qcore` | `strcmp` (8/6), `strlen` (80/0), `strerror_r` (3/0) | Reads its string arguments. `strerror_r` writes into the caller's buffer. None allocates, frees or retains an argument, and each returns a scalar. The `strerror_r` declaration is the **XSI** `int` form; the ABI gate withdraws it in the 113 units that declare the GNU `char *` form, and every call site is in an XSI unit. |
| | `strdup` (2/0) | Returns a fresh `malloc` allocation or NULL: `resource_return_owned(nullable)`. `free` from `libc/memory.qcore` releases it with the same allocator. |
| `libc/stdio.qcore` | `perror` (14/0), `fflush` (4/0), `remove` (1/0) | Reads a string, flushes a stream the caller keeps, or reads a path. Nothing is allocated, freed or retained. |
| `libc/stdlib.qcore` | `atoi` (3/0), `srand` (1/1), `rand_r` (2/0), `system` (1/0) | Reads a string, takes a scalar, updates the caller's seed value, or runs a command and waits for it. Nothing is allocated, freed or retained. |
| `libc/unistd.qcore` | `sleep` (4/1), `usleep` (2/0), `sysconf` (4/2), `getpagesize` (1/1), `geteuid`, `getegid`, `setsid` (1/0 each), `access` (2/0), `unlink` (3/0), `read` (3/0), `write` (1/0) | Takes scalars or reads and writes through caller buffers. `read`/`write` **use** a descriptor, neither create nor close one, and retain nothing. |
| `libc/sys_stat.qcore` | `fstat`, `mkdir`, `umask` (1/0 each) | Writes the caller's `struct stat` value, reads a path, or exchanges a scalar. |
| `libc/socket.qcore` | `setsockopt` (5/5), `send` (1/1), `recv` (1/1), `listen`, `sendfile`, `ntohl` (1/0 each), `htonl` (2/0) | Uses an existing descriptor. The kernel copies `optval` and `buf` and retains neither. The byte-order calls are scalar-only. |
| | `gai_strerror` (1/0) | Returns a pointer to a **static** message that the caller must not free and that aliases no argument: `resource_return_none()`, following the `__errno_location` precedent. |
| `libc/threads.qcore` | `mtx_lock`, `mtx_unlock` (4/0 each), `cnd_wait` (2/0), `pthread_rwlock_rdlock`, `pthread_rwlock_wrlock` (1/0 each), `pthread_rwlock_unlock` (2/0) | Uses the `libc/pthread_mutex.qcore` shape. The lock object stays the caller's on both sides of the call. The declaration says nothing about mutual exclusion; the constraint-27 model is untouched. |
| | `pthread_self` (1/0), `pthread_kill` (2/0), `pthread_setcancelstate`, `pthread_setcanceltype` (1/0 each), `pthread_attr_setdetachstate` (2/0), `sigemptyset` (4/0), `sigfillset` (3/0), `sigaddset` (1/0) | Takes scalars or writes scalar state through a caller pointer. |
| `libc/time.qcore` | `time` (4/0), `timegm` (1/0) | Stores or normalises a value through a caller pointer and returns a scalar. |
| `libc/math.qcore` | `ceil` (2/0), `floor`, `fabs` (1/0 each), `pow` (2/0) | Scalars only. |
| `libc/dlfcn.qcore` | `dlerror` (2/0) | Returns a pointer to library-owned message storage that the caller must not free: `resource_return_none()`. |
| `libc/process.qcore` | `prctl` (1/1) | Applies **only** where `option` is literally `15` (`PR_SET_NAME`), through `requires resource_literal_argument(option, 15)`. That option copies at most 16 bytes of the name and retains nothing, so the tail is `resource_variadic_data_tail()`. Every other option stays undeclared. |

**Declared-ABI agreement, measured before the arbiter.** Each module was run
through `cargo run -p qcore-c-gateway --example declared_abi_census` over the
185 preprocessed units ([abi-evidence.json](abi-evidence.json)). Every shipped
declaration agrees wherever a unit declares it, except `strerror_r` as noted.
Some units carry no declaration the reader recognises, for example `time` in
40 units and `sendfile` in 65; there the contract applies
(`LOADER-ABI-UNDECLARED`, the existing boundary). The arbiter confirms it: AFTER
contains no `whose declared contract was withdrawn` leaf for any name this card
declares.

**Nothing verified here.** A wrong assumption in this table can make a
`Safe` below wrong. These premises join the verdict's assumption set; they are
not facts about glibc that this repository proves.

## Refused callees

Seventy-seven libc/POSIX callees are refused. Each ground is a reason the v1
vocabulary, or the reader in front of it, cannot state the effect exactly.
None is over-approximated.

| Ground | Callees | Sole-blocked |
|---|---|---:|
| **result-alias**: the result, or a stored out-pointer, aliases an argument or retained storage; v1 has no "aliases parameter n" mode | `memcpy`, `memmove`, `memset`, `strcpy`, `strncpy` (return argument 1); `strchr`, `strrchr`, `strstr`, `strcasestr`, `memchr`, `memrchr` (point into it); `gmtime_r`, `localtime_r` (return argument 2); `realpath` (argument 2 or fresh); `pthread_getspecific` (retained); `strtol`, `strtoll`, `strtoull`, `strtod`, `getpwnam_r`, `getgrnam_r` (alias written through `T **`) | 13 |
| **abi-restrict**: faithful, but the reader withdrew the declaration at every call site because a qualifier follows the `*` | `printf`, `fprintf`, `snprintf`, `sprintf`, `stat`, `strftime`, `gettimeofday`, `getsockopt`, `getpeername`, `dlsym`, `cnd_timedwait`, `pthread_sigmask`, `regexec`, `regerror` | 2 |
| **abi-sockaddr-union** / **abi-enum-typedef** / **abi-va-list** | `bind`, `connect` / `getrlimit`, `setrlimit` / `vprintf`, `vsnprintf` | 0 |
| **descriptor-or-handle-lifecycle**: creates or releases a descriptor or handle; v1 custody is pointer-only and has one kind | `open`, `close`, `socket`, `accept4`, `fcntl`, `fopen`, `fclose`, `opendir`, `closedir`, `dlopen`, `dlclose`, `getaddrinfo`, `freeaddrinfo`, `fts_open`, `fts_close`, `glob`, `globfree` | 4 |
| **in-place-object-lifecycle**: init or destroy of a caller-owned object | `regcomp`, `regfree`, `mtx_init`, `mtx_destroy`, `pthread_rwlock_init`, `pthread_rwlock_destroy`, `pthread_attr_init`, `pthread_key_create` | 4 |
| **no-return** | `exit`, `abort` | 0 |
| **callback** / **retains-pointer** / **pointer-to-pointer-borrow** | `qsort`, `call_once`, `pthread_once` / `pthread_setspecific` / `backtrace` | 3 |
| **header-inline-body**: a `static __inline` glibc body the unit can check | `__bswap_16`, `__bswap_32`, `__bswap_64`, `__uint64_identity` | 0 |

The **abi-restrict** row is the measurement `LOADER-ABI-RESTRICT-QUALIFIER`
asked for. The first trial arbiter selected all 73 drafted declarations and
published **2,577** new `whose declared contract was withdrawn … (abi-unreadable-declaration)`
call-site leaves: 1,290 for `snprintf`, 1,192 for `fprintf`, 50 for `printf`
and 10 for `sprintf`, and so on. A five-unit probe fed to the same census tool
isolates the unread token. `const char *__restrict __format` is withdrawn;
`const char * __format` agrees. `project_declarator` counts only trailing
`*` words, so a parameter that **ends** in a qualifier after its `*` projects
to nothing. A declaration that is withdrawn everywhere decides nothing and
only renames leaves, so these fourteen were dropped before the final AFTER
run. Their readings remain in their modules' comments. The repair is a
faithful widening (C11 6.7.6.3p15 drops a parameter's top-level qualifiers
for compatibility), but it changes analyzer semantics, so it is registered
rather than done here (`W43-LIBC-POINTER-QUALIFIER-READER`). It is also why
`LIBC-PRINTF-FAMILY` stays open, and it means `memcpy`'s alias mode alone would
not make `memcpy` declarable.

**Two readings stricter than precedent, recorded here.** W37 declared a sort
that takes a comparator, but this card refuses libc's callback-invoking
functions because it cannot state the callback's effect. The existing corpus
already mixes allocators in its one kind-less custody class (for example,
`cfl_sds_destroy` and `free`); this card does not extend that mixing to
descriptors or `FILE *`. Both refusals are registered (`W43-LIBC-CALLBACK-RETAIN`,
`W43-LIBC-DESCRIPTOR-LIFECYCLE`).

## 3. Placement rule and its test

The user-directed rule is that libc contracts live **only** in
`quirin-provider-libc`. It is now mechanical:

* `providers/quirin-provider-libc/libc-names.txt` lists 2,762 names. Each is
  an identifier followed by `(` at file scope in a `libc6-dev` or
  `linux-libc-dev` header across all 185 pinned units. The generator is
  [`scripts/libc_names.py`](scripts/libc_names.py).
* `scripts/contract_corpus.py::libc_placement_violations(repository)` scans
  every canonical corpus module and every `.qcore` under `providers/`. It
  reports every `extern fn` declared outside the libc crate whose name is
  listed or is declared by the libc crate itself. A missing or empty list
  raises an error, so the check fails closed.
* `scripts/tests/test_contract_corpus.py`, which runs in the standing gate's
  Python suite, asserts three things:
  * the repository has **no** violations;
  * the list is non-trivial and covers every name the libc crate declares;
  * in a synthetic negative control, a missing list raises and an empty list
    raises. The same control then shows that each of three violations is
    reported: a listed name in a Fluent Bit module, a listed name in a loose
    provider `.qcore` (a commented-out declaration is ignored), and a name
    that is only libc-crate-declared.

The test is scoped to shipped contract trees. QCore test fixtures under
`crates/*/tests` and `compiler/tests` are programs, not contracts, and one of
them declares `malloc`. The name list is host-bound, which is registered as
`W43-LIBC-NAME-LIST-HOST-BOUND`. The crate's own `index_is_exactly_the_contract_tree`
unit test already rejects an unindexed module under `contracts/`.

## 4. Arbiter

One release binary serves both stages, because the card is declarative and the
audit reads the manifest at runtime: `cycle19_fluent_bit_libraries_audit`, sha256
`fba2ad21…0f6b`, built from base `9edad50c`. Both runs use
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
which runs the W42/W43 command shape.

| Run | Tree | Manifest | Exit | Window (UTC) | Record |
|---|---|---|---:|---|---|
| BEFORE | `9edad50c` | 33 modules | 0 | 06:50:19–06:55:24 | [before-run.json](before-run.json) |
| AFTER | `d5b90c6d` | 44 modules | 0 | 07:28:59–07:34:50 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

**BEFORE vs the baseline** ([before-identity.json](before-identity.json)):
diagnostics, chunk-contracts and source are **byte-identical** to
`w43-return-expression/artifacts/`. `contracts` and `source-effects` carry the
checkout-path and crate-path identities that QUIRIN-PROVIDER-SHARED and the
W43 integrator already itemised. They bear no verdict.

**AFTER vs BEFORE** ([after-identity.json](after-identity.json)):
chunk-contracts, source-effects and source are byte-identical. `contracts`
differs only in the manifest text and in 11 added selected-source entries, one
per new module: 0 removed and 0 changed. Two further AFTER-configured runs,
trial 2 before the implementation commit and the `-E -P` capture run,
reproduced the AFTER diagnostics byte for byte.

**Attribution** ([`scripts/attribute.py`](scripts/attribute.py),
[attribution-summary.json](attribution-summary.json), every row in
`attribution.json.gz`) exits 0 with `attribution_complete: true`. The leaf
`line` coordinate is resolved in each unit's analysed `-E -P` text, which
[`scripts/unit_texts.py`](scripts/unit_texts.py) pairs with its unit by
file-scope definitions. 180 units are paired; the 5 unpaired units project no
function. The first matching class wins:

| Class | Removed | Added | What it is |
|---|---:|---:|---|
| declared-callee | 229 | — | `call to uncontracted function X` with X declared here; exactly the 229 leaves the inventory lists for the 55 names |
| newly-safe-callee | 4 | — | `call to uncontracted function F` where F moved to Safe and now supplies a validated W4 summary: `rd_strcmp` (1, in `rd_strcmp2`) and `mk_socket_set_tcp_{fastopen,nodelay,reuseport}` (1 each) |
| same-line | 67 | 19 | The analysed line calls a declared or newly Safe name. **Removed:** 40 branch-condition refusals (`if (strcmp(…) == 0)`, `if (unlink(…))`), 22 lexical indirect-access refusals whose line's call is now contracted, 3 nested-call refusals and 2 return-expression refusals (`return (pthread_self() == thr);`). **Added:** 11 `nested call` refusals, where the now-modelled call's own argument is an uncontracted call (for example `ceil(log2(…))`, `srand(rd_seed())`, `strlen` inside another call), 7 `resource call with a formal embedded in an unmodeled expression` and 1 `unresolved resource identity` |
| same-function | 14 | 8 | Elsewhere in a function that has a decided leaf. These are statements that span lines, where the leaf's first line does not spell the call, and bindings that the old refusal had poisoned. `cio_chunk_open` and `cio_os_mkpath` are the two `strdup` callers. `strdup` now binds an owned result there, so `cio_chunk_open`'s seven `ch` alias leaves become two (`ctx`, `st`) and `cio_os_mkpath`'s `dup_dir` resolves. `ctr_random_get` and an OIDC unit test lose poisoned-binding alias leaves as well |
| **total** | **314** | **27** | net −287 = −277 `unsupported` −10 `unknown_alias` |

The same-function class is the weakest link, because it is anchored at
function rather than line level. It covers 22 leaves in 10 functions, and all
22 are listed by coordinate in `attribution.json.gz`. None of them is a leak,
and none is in a function that moved.

Every one of the **74** units whose fields change moves only its
`diagnostic_counts` and its count-summary `reason`, and both move by exactly
the leaf delta of the unit's own functions. No unit status, reason code or
other field moves. The top-level fields other than the counts are unchanged.

**The 22 functions that move Unknown → Safe.** Each is Safe **relative to the
declared premises above**, which is invariant 2's reading and not a proof of
the C.

| Function | Unit | Its only BEFORE blocker |
|---|---|---|
| `rd_kafka_DescribeConsumerGroups_cmp`, `rd_kafka_DescribeTopics_cmp` | `rdkafka_admin.c` | `strcmp` |
| `rd_list_cmp_str` | `rdlist.c` | `strcmp` |
| `rd_map_str_cmp` | `rdmap.c` | `strcmp` |
| `rd_strcmp` | `rdstring.c` | `strcmp` |
| `rbtree_compare` | `mk_mimetype.c` | `strcmp` |
| `rd_strcmp2` | `rdstring.c` | `rd_strcmp`, which is now Safe and summarised |
| `rd_kafka_mock_coord_str2type` | `rdkafka_mock.c` | two branch conditions calling `strcmp` |
| `thrd_is_current` | `tinycthread_extra.c` | `return (pthread_self() == thr);` |
| `cio_getpagesize` | `cio_utils.c` | `getpagesize` |
| `mk_utils_remove_pid` | `mk_core/mk_utils.c` | a branch condition calling `unlink` |
| `mk_utils_worker_rename` | `mk_core/mk_utils.c` | `prctl` (option `15`) |
| `mk_main` | `mk_lib.c` | `sleep` |
| `net_plain_read` / `net_plain_write` | `mk_net.c` | `recv` / `send` |
| `mk_socket_set_cork_flag`, `…_tcp_defer_accept`, `…_tcp_fastopen`, `…_tcp_nodelay`, `…_tcp_reuseport` | `mk_socket.c` | `setsockopt` |
| `mk_utils_get_system_core_count`, `mk_utils_get_system_page_size` | `mk_server/mk_utils.c` | `sysconf` |

Nineteen Unknown functions were sole-blocked by a declared callee. Eighteen of
them are in the table above. The nineteenth, `rd_kafka_srand`, **stays
Unknown**: its `srand(rd_seed())` now refuses on the nested call. That is the
remaining obligation the brief anticipated. Across all 169 functions that name
a declared callee, the outcome is 18 Unknown → Safe, 141 Unknown → Unknown and
10 provisional Unsafe → Unsafe. The last group gains or loses only non-leak
leaves ([where-they-went.json](where-they-went.json)). Four further Safe moves
come from lines rather than sole blockers, which gives the 22.

**Leak pin (constraint 7).** The pin is unmoved at **272**, and no leak leaf
is added or withdrawn. This card is the third declarative arbiter in a row to
leave the pin alone, after W36 and W37. Most of the declarations are the
neutral element, and the one acquisition, `strdup`, sits in two Unknown
functions that keep other blockers. The new baseline is
[artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json));
`docs/reports/w43-return-expression/artifacts/` is its frozen predecessor and
is not edited.

## 5. Pin changes, each deliberate

* `contract/corpus.json`: unchanged. The libc crate's `contracts/` was
  already a root, so the eleven modules are indexed without an edit.
* `cycle19-resource-contracts.json`: 33 → 44 modules. The eleven are inserted
  after `libc/pthread_mutex.qcore`.
* `compiler/tests/qcore_contract_corpus.rs`: corpus size 96 → 107, plus 55
  per-declaration clause pins in logical-path order. Each neutral declaration
  pins 2 clauses plus 1 per parameter; `strdup` pins 3 and `prctl` pins 5.
* `compiler/tests/contract_loader.rs`:
  * corpus module count 96 → 107;
  * (selected, v1 declarations, rejected) (33, 59, 207) → **(44, 114, 207)**;
  * two manifest `source_assumptions` pins 33 → 44;
  * the old-table parity list gains 54 all-borrowed entries and `strdup`'s
    owned-nullable entry.
* `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs`: the manifest
  `source_assumptions` pins 33 → 44.
* `scripts/tests/test_contract_corpus.py`: modules 96 → 107, and the
  `quirin-provider-*` share 24 → 35. The provider-owned count stays at 72,
  because `providers/contracts.py check` is unchanged.
* `providers/quirin-provider-libc/src/lib.rs`: `CONTRACTS` 3 → 14 entries.
  The disk-pinned unit test covers them.

No assertion is weakened. Every pin moved only by the eleven added modules.

## 6. Validation

Measured in this worktree (`agent-a2aa6b1173f195eb0`). The core and
provider gates ran on the implementation commit `d5b90c6d`. Later commits
change documents, the ledger, report scripts and evidence files only; no Rust,
Lean, contract or test source changes. Every step's exit was captured to a
file, never piped, by [`scripts/run_gate.py`](scripts/run_gate.py) with
`CARGO_BUILD_JOBS=6`, `RUST_TEST_THREADS=2` and debug info off. Logs are
worktree-private under `measure/`. The generator input is
[gate.json](gate.json).

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt), taken in this checkout) | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,136 passed / 0 failed / 12 ignored**, 272 summaries | `measure/gate/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **167** audits | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-libc-contracts/gate.json` | 0 | 294 follow-ups, 107 contract modules | `measure/build-matrix-2.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-check.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **319** tests | `measure/python-final-2.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/ptest.log` |
| `cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations --locked --offline -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned, inventories unchanged | `measure/provider-gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/pbuild.log` |
| `python3 providers/package.py verify measure/provider-gate/packages/<id>` (×5) | 0 ×5 | | `measure/provider-gate/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 5 tests | `measure/provider-gate/ppython.log` |

**Every count equals the base.** The core count 5,136 / 0 / 12, the 167 Lean
audits and the provider count 109 are exactly the integrator's figures at
`9edad50c`. This card adds no Rust test; it re-pins existing ones. The
`quirin-provider-libc` index test now covers 14 modules. Python goes from
317 to 319, which are the two new placement tests. No `P02-FLAKE` row failed
in the full runs, so the isolated package run above is the constraint-16
record, not a retry.

**Attempts that are not the gate, disclosed.**

* The first focused run of `contract_loader` failed 1 of 10 (exit 101). It
  stopped at `production_contracts_preserve_the_previous_resource_modes`,
  because the old-table parity list did not yet name the new declarations.
  The list was then extended, as §5 describes, and the rerun passed 10/10.
* A pre-final Python run failed 1 of 319 (exit 1): `dimensions.named_followups`
  was 285 against 294 entries before regeneration. It is the order-dependence
  the validation rule names, and the regeneration fixed it.
* A first `gate.json` recorded 321 Python tests. The measured count is 319,
  so the file was corrected and the matrix regenerated before the final
  suite ran.
* This session was paused by an account usage limit after both gates had
  completed. On resume, every step's recorded exit was checked; none was
  missing or cut short, so no step was rerun.
* The ledger, STATUS and report edits were made in the working tree while the
  core `cargo test` step was running. The Rust sources, contracts and manifest
  under test were already committed at `d5b90c6d`; the 319-test Python suite
  ran after every edit.

The trial arbiters (`measure/trial1/`, `measure/trial2/`) and the
wrapper-captured runs are measurements, not gate steps; each exited 0.

## 7. Ledger and status

* Registered ([followups.json](followups.json)):
  `W43-LIBC-POINTER-QUALIFIER-READER`, `W43-LIBC-ABI-READER-SHAPES`,
  `W43-LIBC-RESULT-ALIAS-NAMES`, `W43-LIBC-OUT-POINTER-ALIAS`,
  `W43-LIBC-DESCRIPTOR-LIFECYCLE`, `W43-LIBC-IN-PLACE-LIFECYCLE`,
  `W43-LIBC-NORETURN`, `W43-LIBC-CALLBACK-RETAIN`,
  `W43-LIBC-HEADER-INLINE-BODIES` and `W43-LIBC-NAME-LIST-HOST-BOUND`.
* Retired: `LOADER-ABI-RESTRICT-QUALIFIER`. Its discharge condition was
  exactly the measurement in §2, and the entry is preserved verbatim in
  [retired-followups.json](retired-followups.json).
* `LIBC-MEM-RESULT-ALIAS` and `LIBC-PRINTF-FAMILY` stay open. The first is
  widened by `W43-LIBC-RESULT-ALIAS-NAMES`; the second is blocked on
  `W43-LIBC-POINTER-QUALIFIER-READER`.
* Follow-ups go from **285 to 294**, and `dimensions.named_followups` is
  regenerated to match. `corpus.cycle19_raw_units` now points at this report's
  artifacts; its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* `docs/STATUS.md`'s corpus, contract-corpus and follow-up rows are updated
  from this arbiter's primary evidence.

## 8. For the integrator

* Constraint 7's leak-pin chain can record W43-LIBC-CONTRACTS as the third
  consecutive declarative arbiter to leave the pin at 272: 55 declarations and
  22 function moves, with no leak leaf added or withdrawn.
* Constraint 8's paragraph on the ABI reader can record the measured
  `__restrict` boundary: a qualifier **after** the `*` makes a parameter
  unreadable, and every glibc prototype with `__restrict` is withdrawn. This
  card leaves rule text to the integrator.
* The `contracts` artifact embeds this worktree's path in its source keys,
  as earlier isolated cards' artifacts did.

## 9. Remaining work

The registered residuals, in order of measured leverage:

1. The pointer-qualifier reader repair. It unlocks the fourteen faithful
   `__restrict` prototypes (2,238 leaves, 2,212 of them in the printf
   family).
2. A result-alias mode. It covers 13 sole-blocked functions, led by
   `memcpy` (8), and also needs the reader repair.
3. A no-return clause. `exit` and `abort` together hold 33 leaves.
4. Kind-indexed custody for descriptors and handles, and an in-place
   lifecycle mode. Together they cover 8 sole-blocked functions.

Outside libc, 262 of the 315 sole-blocked functions wait on **corpus-defined**
callees. Their lever is W4 summaries, which need Safe bodies, not
declarations.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> 9edad50c        # manifest at 33 modules
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <before> docs/reports/w43-return-expression/artifacts --worktree-alias
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> d5b90c6d         # manifest at 44 modules
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py <before>/resource-diagnostics.json <captured -E -P dir> <unit-texts>
python3 docs/reports/w43-libc-contracts/scripts/attribute.py <before> <after> <unit-texts> <out.json> providers/quirin-provider-libc/contracts/libc/{dlfcn,math,process,socket,stdio,stdlib,string,sys_stat,threads,time,unistd}.qv
python3 docs/reports/w43-libc-contracts/scripts/libc_inventory.py docs/reports/w43-return-expression/artifacts/diagnostics.json.gz <captured -E dir> <fluent-bit-clang-source> <inventory.json>
python3 docs/reports/w43-libc-contracts/scripts/decisions.py <inventory.json> libc-inventory.json libc-inventory.md <the eleven modules>
python3 docs/reports/w43-libc-contracts/scripts/libc_names.py <captured -E dir> providers/quirin-provider-libc/libc-names.txt
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-libc-contracts/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w43-libc-contracts/artifacts 30
```

The captured preprocessing comes from a private run with `QCORE_CLANG` set to a
transparent wrapper that tees Clang's `-E` or `-E -P` output. Its diagnostics
are byte-identical to the corresponding run without the wrapper. The private
raw outputs are in this worktree: `measure/before/`, `measure/after/`,
`measure/trial1/` (all 73 drafted declarations), `measure/trial2/` (the
shipped 55, before the commit), `measure/inventory/` (the wrapper, captures,
ABI census and tools) and `measure/unit-texts/`.
