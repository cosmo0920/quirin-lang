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
| `__builtin_va_arg` | 3 | 2 | existing follow-up | `va_arg` inside a typed store (CG26-VA-PROTOCOL-PRECISION); §4 |
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
| open, builtins | 7 | 56 |
| builtin owned by an existing follow-up (`__builtin_va_arg`) | 1 | 3 |
| **total** | **56** | **155** |

The script is [`scripts/gap.py`](scripts/gap.py). The ranked list it printed
for the run is [`census-046accf6.txt`](census-046accf6.txt).

## 2. The declarations

All 23 are in `quirin-provider-libc` and only there (the libc placement rule;
every name is in `libc-names.txt`, which is unchanged). Each is
`revision flexible` under the standing `libc-c-standard-abi` assumption. Each
module's new section quotes the audited host's prototype and the C standard,
POSIX or Linux text it states.

| Module | Names | Custody statement |
|---|---|---|
| `stdlib.qv` | `abs`, `tolower`, `toupper` | scalar in, scalar out |
| `stdlib.qv` | `strcat` | `resource_return_alias(dest, nonnull)` (C04.3.1). It stands outside `string.qv` because that module is published: its SHA-256 is pinned in `contract-load-path.json` and part of every inheriting contract's identity |
| `math.qv` | `sqrt` | scalar |
| `stdio.qv` | `fread` | a use of its stream (`fflush`'s statement) |
| `threads.qv` | `sigdelset`, `pthread_cancel` | `sigaddset`'s and `pthread_kill`'s statements |
| `threads.qv` | `pthread_setspecific` | `resource_retained_by(pointer, global)` (C04.14): withheld from the function map; rule 12t decides only static storage, so a heap or caller pointer is `retention-undecided` |
| `threads.qv` | `pthread_getspecific` | `resource_return_none()`: the caller owns nothing through the call. The relation to the retained object is not stated, so every custody use of the result is refused as an unresolved identity |
| `unistd.qv` | `lseek`, `ftruncate`, `posix_fallocate` | a use of a descriptor |
| `unistd.qv` | `readlink`, `chown`, `msync`, `getrandom` | path, mapping and buffer uses |
| `sys_stat.qv` | `chmod` | a path use |
| `socket.qv` | `shutdown`, `sendmsg`, `poll` | descriptor uses; `shutdown` does not close |
| `process.qv` | `getpwnam_r`, `getgrnam_r` | `resource_out_derived(result, resultbuf, nonnull)` (C04.16) and `resource_retained_by(buffer, resultbuf)` (C04.14) |

**Locks.**
- `providers/contracts.py sync` regenerated `contract-inventory.lock`. Nine
  modules' digests and counts changed; no other crate's lock changed.
- `quirin-audit pin --source-root <046accf6 checkout> --crate
  quirin-provider-libc --function …` wrote the 23 flexible entries in
  `contracts.lock`.
- `contract-load-path.json` is unchanged: no published module moved.

**Controls.** `crates/qcore-c-gateway/tests/libc_gap_declarations.rs` loads
the production modules byte for byte and analyses units that include the
audited host's glibc headers under `_GNU_SOURCE`. It has six tests:

1. Every declaration agrees with its prototype (LOADER-ABI) and admits a
   pass-through caller: `Safe`, no diagnostic.
2. With the card's sections cut off, each of those calls keeps
   `call to uncontracted function`.
3. The card's sections declare exactly the 23 names.
4. `fread` after `fclose` and `lseek` after `close` are use-after-close
   candidates. Uses before the close are `Safe`.
5. `pthread_setspecific` of a heap, caller or freed pointer is
   `retention-undecided`. A release of `pthread_getspecific`'s result, once
   or twice, is `Unknown`, never `Safe` and never accused.
6. A `getpwnam_r`/`getgrnam_r` buffer freed while the record lives is
   `retention-released-while-held`. A local buffer is `retention-undecided`.
   A release of `strcat`'s alias is not `Safe`; a release of its base is.

No route, loader or Lean definition changes. The readings are rules 12t
(`QCore.RetainedBy`), 12z (`QCore.DerivedOutSlot`) and C04.3.1
(`QCore.DerivedResult`), and the kinds of C04.6 (`QCore.KindedCustody`).

## 3. `realloc`: a user choice (not declared)

The brief asks for `realloc` with its failure keeping the old block. That is
`resource_return_reallocated(ptr, nullable)` beside `resource_owned(ptr,
nullable)`, librdkafka's `rd_realloc` statement. It is exact except for one
arm:

- **Size 0.** On glibc, `realloc(p, 0)` with `p` non-NULL frees `p` and
  returns NULL (glibc realloc(3)). C17 7.22.3.5 makes the result
  implementation-defined, and C23 makes the call undefined.
- **What the declaration would say.** A NULL result keeps the old block owned.
- **The consequence.** A caller that frees `p` on the NULL arm would be read
  as `Safe` while it double-frees on glibc.

No admitted clause restricts a declaration to a nonzero argument:
`resource_literal_argument` (C04.5) is an equality with one literal, and the
guard of C04.14.1 conditions only a retention. Under fail-closed the card does
not declare it. The six sites (`batches_append`, `batch_add_resource`,
`batch_add_scope`, `batch_add_metric`, `append_metric_to_timeseries`,
`cio_memfs_write`) each reallocate a member, so a declaration would also meet
the field-slot refusal (SLOT-MOVE-FIELD-SLOT) before deciding anything.

Options:

- **(A) Recommended: a nonzero-argument precondition.** For example
  `requires resource_argument(size, != 0);`, a C04.5-style premise. The
  declaration applies only where the actual is structurally decided nonzero
  and is undeclared elsewhere. This matches C23. It needs a loader clause,
  rule-12-style call-site reading and Lean parity. Most sites spell
  `(count + 1) * sizeof(T)`, so they stay undecided unless arithmetic is read.
- **(B) A declared policy.** Declare the reallocation reading and record size
  0 as outside the policy, like the user's `[1, MAX]` decision for `pipe`.
  It is optimistic on glibc for size 0, and every row it moves is attributed
  to the policy.
- **(C) A cautious failure arm.** A NULL result leaves the old block's custody
  undecided, so any later release or use is refused. It is exact for C17,
  but a caller that frees on failure is never `Safe`.
- **(D) Keep it undeclared.**

## 4. Builtins: decisions and a user choice

`__builtin_*` names are compiler intrinsics. They have no owning provider
crate, and the placement test does not list them. W48-CALLEE-DECISION-TABLE
class 6 ("compiler builtins without an owning crate") has waited on a user
decision since wave 48. Per name:

- **`__builtin_va_arg` (3 pairs).** Not a new choice. The `va_list` protocol
  is rule 11w. The three sites (`cmt_histogram_buckets_create`,
  `rd_kafka_mock_push_request_errors`,
  `rd_kafka_mock_partition_push_request_errors`) store a scalar `va_arg`
  into an array element or a member. CG26-VA-PROTOCOL-PRECISION already
  names exactly these three ("a scalar va_arg value inside a typed store");
  it is amended with the 046accf6 measurement.
- **`__builtin_memcpy` (8) and `__builtin_memmove` (3)**, all in
  librdkafka's `lz4.c`/`lz4hc.c`. GCC and Clang give these the semantics of
  the library functions they name, so the reading is settled. Where it lives
  is the class-6 choice:
  - **(1) Recommended.** The C gateway reads the call through the libc twin's
    declaration in force (`memcpy`, `memmove`), compared against Clang's
    builtin signature like any declaration (LOADER-ABI). No contract text
    is duplicated, and the libc placement rule is untouched.
  - **(2)** A data-only `quirin-provider-builtins` crate declaring
    `extern fn __builtin_memcpy(...) contract inherit libc::memcpy;`. It
    needs a new load-path crate and views.

  No function would change status: every site's function has other
  blockers (`LZ4_wildCopy8` 3, `LZ4_memcpy_using_offset` 1 and more).
- **Value builtins: `__builtin_ctz`, `__builtin_ctzll` (`snappy.c`),
  `__builtin_nanf` (`cJSON.c`) and `__builtin_prefetch` (`mk_plugin.c`).**
  Each takes and returns scalars, or a pointer it only reads. With either
  option above they are custody-neutral, and `find_lsb_set_non_zero`,
  `find_lsb_set_non_zero64` and `cJSON_GetNumberValue` would have no other
  blocker. `__builtin_ctz(0)` is undefined; that is a value property, not
  custody.
- **`__builtin_alloca` (41 pairs, 1 sole).** The brief's semantics: stack
  storage that must never be freed and never escape the frame. No declaration
  can state that.
  - `resource_return_none()` would refuse a `free` of it, as an unresolved
    identity. But it would never notice the escape: a returned or stored
    alloca pointer is untracked data. So it is not the stated semantics.
  - **(a) Recommended.** A gateway reading of the builtin as frame storage of
    the calling function. It carries no obligation. A release of it is an
    `invalid_release` candidate (C11 7.22.3.3p2: not an allocator's value).
    A `return` of it, a store into anything that outlives the frame, or
    hand-off to a retaining position is refused (`frame-storage-escapes`).
    Its non-NULL result is already the null property's
    (`null_deref/walk.rs`). It needs Lean parity (a `QCore.FrameStorage`
    model) and one measurement.
  - **(b)** Keep it refused.

## 5. Measurement and gate

The prediction is [PREDICTION.md](PREDICTION.md). The integrator runs the
measurement:

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6 --side tip=rev:<last code commit> --declared docs/reports/libc-gap-declarations/PREDICTION.md
```

**Card gate.** The command was `python3 scripts/gate_scope.py --base
c8c7edf21 --out private/gate`, at head `a3dc75a94`, and it exited 0. The
record is [gate/gate.json](gate/gate.json) and its printed selection is
[gate/selection.log](gate/selection.log). The scope was full, because the
diff reaches `qcore-c-gateway`. Lean: none.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| fmt | 0 | |
| Clippy | 0 | |
| Rust suite | 0 | 603 units: 9,167 passed, 0 failed, 16 ignored, 135 filtered |
| reevaluation probe | 0 | |
| reevaluation follow-up | 0 | |
| Python | 0 | 577 tests |
| provider package build | 0 | |
| provider package verify (all) | 0 | |
| provider package verify | 0 | |
| provider test | 0 | 133 passed, 0 failed, 1 ignored |
| provider fmt | 0 | |
| provider Clippy | 0 | |

**The earlier attempt** ([gate/attempt-1-gate.json](gate/attempt-1-gate.json),
head `fbca407e2`) failed only in `qcore-compiler::test::contract_loader`, 2
tests:
- the pinned v1 declaration count, 499 against the new 522;
- the previous-modes table, which lacked the new names.

`e6a5288d0` updates both pins: v1 declarations 499 -> 522, withheld
declarations 109 -> 112, and the 20 plain modes added to the table. Every
other step of that attempt exited 0.

**Measurement (integrator, 2026-11-05).** Run from the main checkout, tip
`rev:e6a5288d0` against `baseline:c8c7edf21`, with the command above, output
`private/measure/libc-gap-20261105T034105Z`; exit 1 (declared mismatch).
System and slice match (0 moves; slice has 6 content changes). Corpus: 5
moves, 0 unit moves, 0 unmatched, 65 content changes. The 4 declared moves
happened. One more row moved Unknown -> Safe that the prediction missed:
`librdkafka::rdstring.c::_rd_strcasestr`. At the base its only leaves
were "unmodeled call, aggregate or indirect access in for-loop condition"
(`rdstring.c:348`) and "... in do-while condition" (`:358`). The only call in
either condition is `tolower((int)*h)` / `tolower((int)*n)`. This card declares
`tolower` as `int -> int` with `resource_borrowed(c)` and no pointer
(§2). With the declaration the conditions are modelled and the row has no leaf.
Its declaration dependencies are `strcasecmp`, `strlen` and `tolower`. Its result is
`haystack` or `NULL`, as at the base. The move is justified. The prediction
counted only rows with a `call to uncontracted function` leaf and missed
callee calls that sit in loop conditions. No row moved to Unsafe.
[native-comparison.json](native-comparison.json) (sha256
`fd63c6d56f2a800d9cdbee5c83d5d6eade86e764841348bd05b23f03d5dd8982`).

## 6. Follow-ups

Registered:
- `LIBC-REALLOC-SIZE-ZERO`: §3's choice.
- `BUILTIN-FRAME-STORAGE`: §4's alloca reading.

Amended:
- `W48-CALLEE-DECISION-TABLE` (class 6 options, sized at 046accf6)
- `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`
- `W43-LIBC-CALLBACK-RETAIN`
- `W43-LIBC-OUT-POINTER-ALIAS`
- `W43-DESCRIPTOR-REFUSED-SHAPES`
- `W43-LIBC-GETPEERNAME-SOCKADDR-UNION`
- `CG25-IN-PLACE-RESIDUAL-PAIRS`
- `CG26-VA-PROTOCOL-PRECISION`

Each amendment adds the 046accf6 sizes and what this card discharged.
External libraries (OpenSSL, libcurl, zlib) appear in neither list: they are
not libc, and the round brief leaves them to their plugin crates.
