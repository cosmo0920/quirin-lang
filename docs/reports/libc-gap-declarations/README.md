# LIBC-GAP-DECLARATIONS

Card 2 of round 13. The user ran `quirin-audit current` at Fluent Bit
`046accf6` and said (verbatim, 2026-11-05): "Now, we're still needed to fill
the gap of the missing contracts." Then: "Launch 1 and 2". This card covers
the libc and builtin names in that gap. Base `c8c7edf21`.

## 1. Census

**Source.** The run's report
(`build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`
in the main checkout), recomputed by `scripts/gap.py`. A name counts when the
report says `call to uncontracted function` for it, and it is either listed in
`providers/quirin-provider-libc/libc-names.txt` or spelled `__builtin_*`.

**Columns.**
- *pairs*: (function, callee) pairs.
- *only*: functions whose every diagnostic is an uncontracted call.

**Totals.** 64 names / 176 pairs. Eight C11 names have their body in the run:
`mtx_init` 4, `mtx_destroy` 3, `thrd_join` 3, `cnd_signal` 3, `cnd_init` 2,
`cnd_destroy` 2, `cnd_broadcast` 2 and `call_once` 2. They are librdkafka's
`tinycthread.c` definitions, so they are not a libc gap. That leaves
**56 names / 155 pairs**, the brief's figure.

**Already declared?** No provider or profile module declares any of the 56
with `extern fn`. `realloc` and `fread` appear in other providers only in
`//` comments. So no case passes to card 1 as "declared but not in force".

| Callee | pairs | only | Decision | Ground / form |
|---|---:|---:|---|---|
| `__builtin_alloca` | 41 | 1 | **open (user)** | frame storage; §4 |
| `pthread_getspecific` | 15 | 1 | declared | result owns nothing (`resource_return_none()`) |
| `pthread_setspecific` | 10 | 2 | declared | `resource_retained_by(pointer, global)` (C04.14) |
| `__builtin_memcpy` | 8 | 0 | **open (user)** | libc twin; §4 |
| `realloc` | 6 | 0 | **open (user)** | size-0 arm; §3 |
| `tolower` | 4 | 0 | declared | scalar |
| `pthread_key_create` | 4 | 3 | refused | integer key through an out pointer, plus a destructor callback (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `getaddrinfo` / `freeaddrinfo` | 4 / 4 | 1 / 1 | refused | the list is not heap: `free(res)` must not discharge it. Needs a list kind (W43-DESCRIPTOR-REFUSED-SHAPES) |
| `pthread_mutex_init` | 3 | 0 | refused | in-place init through a member or global (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `fread` | 3 | 0 | declared | a use of its stream |
| `__builtin_va_arg` | 3 | 2 | **open (user)** | rule 11w's destination; §4 |
| `__builtin_memmove` | 3 | 0 | **open (user)** | libc twin; §4 |
| `sigaction` | 2 | 0 | refused | registers a handler the kernel keeps and invokes (W43-LIBC-CALLBACK-RETAIN) |
| `pthread_attr_init` | 2 | 0 | refused | in-place lifecycle (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `connect` | 2 | 1 | refused | `__CONST_SOCKADDR_ARG` transparent union, withdrawn by the ABI reader (W43-LIBC-GETPEERNAME-SOCKADDR-UNION) |
| `abs` | 2 | 0 | declared | scalar |
| `toupper`, `sqrt` | 1 each | 0 | declared | scalar |
| `strcat` | 1 | 0 | declared | `resource_return_alias(dest, nonnull)` |
| `sigdelset` | 1 | 0 | declared | writes a caller signal set |
| `shutdown` | 1 | 1 | declared | a use of a descriptor; it releases nothing |
| `sendmsg` | 1 | 0 | declared | a use of a descriptor; ancillary descriptors are copied, never moved |
| `recvmsg` | 1 | 0 | refused | `SCM_RIGHTS` ancillary data creates descriptors no declaration states |
| `realpath` | 1 | 0 | refused | argument 2 or a fresh allocation; the one site uses the derived arm (W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED) |
| `readlink`, `lseek` | 1 each | 0 | declared | a path or descriptor use |
| `pthread_rwlock_init` / `_destroy` | 1 / 1 | 1 / 1 | refused | in-place, storage through a formal (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `pthread_mutex_destroy` | 1 | 0 | refused | finaliser of the refused init (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `pthread_create` | 1 | 0 | refused | runs a callback and retains `arg` (rule 10j; constraint 27) |
| `pthread_cancel` | 1 | 0 | declared | scalar (the `pthread_kill` precedent) |
| `pthread_attr_destroy` | 1 | 0 | refused | finaliser of the refused init |
| `posix_fallocate`, `ftruncate`, `msync` | 1 each | 1 each | declared | a descriptor or mapping use |
| `poll` | 1 | 0 | declared | reads and writes the caller's `pollfd` array |
| `munmap` / `mremap` | 1 / 1 | 1 / 1 | refused | mapping lifecycle with the pointer sentinel `MAP_FAILED` (W43-DESCRIPTOR-REFUSED-SHAPES) |
| `glob` / `globfree` | 1 / 1 | 0 | refused | in-place `glob_t` lifecycle plus an error callback (CG25-IN-PLACE-RESIDUAL-PAIRS) |
| `getrandom` | 1 | 1 | declared | writes the caller's buffer |
| `getpwnam_r` / `getgrnam_r` | 1 / 1 | 0 | declared | `resource_out_derived(result, resultbuf, nonnull)` and `resource_retained_by(buffer, resultbuf)` |
| `fts_open` / `fts_close` | 1 / 1 | 0 | refused | `char *const *` has no spelling (rule 12v), and there is no FTS kind (W43-DESCRIPTOR-REFUSED-SHAPES) |
| `chown`, `chmod` | 1 each | 0 | declared | a path use |
| `bind`, `accept4`, `accept` | 1 each | 1 / 1 / 0 | refused | sockaddr transparent union (W43-LIBC-GETPEERNAME-SOCKADDR-UNION) |
| `backtrace` | 1 | 0 | refused | writes non-custody pointers into a `void *[]` (W43-LIBC-OUT-POINTER-ALIAS) |
| `__builtin_prefetch`, `__builtin_nanf`, `__builtin_ctz`, `__builtin_ctzll` | 1 each | 0 / 1 / 1 / 1 | **open (user)** | value builtins; §4 |

**Summary.**

| Outcome | Names | Pairs |
|---|---:|---:|
| declared | 23 | 52 |
| refused | 24 | 38 |
| open, `realloc` | 1 | 6 |
| open, builtins | 8 | 59 |
| **total** | **56** | **155** |
