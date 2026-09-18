# TASK-ROUTE-TSAN — ThreadSanitizer on the checked task route (wave 37)

**Headline.** The fault is **not QCore's**, and the measurement that settles it
is a fifteen-line C file containing no QCore code at all. ThreadSanitizer's
runtime ships an interceptor for `pthread_create` and **none for
`thrd_create`**; on glibc 2.34+ `thrd_create` reaches `start_thread` without
passing through that interceptor, so the new thread is never registered with
TSan and the first instrumented event in it faults at
`__tsan_func_entry+19` — `mov 0x18(%rdi),%rax`, a read of offset `0x18` of a
null `ThreadState *`. That is the entire content of `TASK-ROUTE-TSAN-SEGV`, and
it explains the address wave 36 recorded without having to attribute anything
to QCore's emission, allocator or mapping.

**The route now has TSan evidence.** Binding the two C11 thread primitives to
pthread through `-include` — changing **no emitted byte** — makes every subject
run clean, and the row is non-vacuous: mutating the emitted C so the two
workers write the *same* byte makes TSan name the race inside
`qcore_buffer_byte_write`. The write-lease card's C11 3.14 claim, which wave 36
could carry only by argument and by ASan/UBSan/LSan, is now backed by a
thread-race detector.

`TASK-ROUTE-TSAN-SEGV` is **retired**; two narrower residuals replace it.

## 1. The finding as inherited

`WRITE-LEND-TASKS-EXERCISE` (wave 36, `docs/reports/write-lend-tasks-exercise/README.md`
§5.4) measured that TSan killed its flagship, wave 31's held-only lease program
and the published `examples/threads/two_workers` at the identical address
(`0x18`) and pc, and concluded — correctly — that the failure was the route's
rather than any one program's. It recorded a fourth row, "a minimal
`thrd_create`/`thrd_join` program — **runs clean**", which is why the diagnosis
did not follow immediately: that row is a **false negative**, and §2.2 says
why.

That card declined to set the residual's priority. This card is that priority
being taken.

## 2. Diagnosis by measurement

### 2.1 The charter's hypothesis, refuted

The charter's stated first hypothesis was a fixed-address mapping colliding
with TSan's reserved shadow/app ranges — the shape a crash at one fixed address
across unrelated programs usually has. **Refuted, and cheaply.** A collision
with a reserved range faults at a *large* address; `0x18` is 24 bytes past
null, which the sanitizer itself names (`Hint: address points to the zero
page`). No `mmap` is reached: neither `os_endpoint` nor `MAP_FIXED_NOREPLACE`
appears anywhere in a program the task route emits — the emitted runtime's only
allocation primitives are `calloc`/`malloc`/`free` behind
`QCORE_TASK_CALLOC` / `QCORE_BUFFER_MALLOC`. The Linux mapping observations in
`CLAUDE.md`'s platform records belong to other subsystems and are untouched by
this card.

### 2.2 What it actually is

`measure/probe/min.c` — no QCore code, a `struct`, `thrd_create`, `thrd_join`,
and a thread body holding exactly one instrumented memory access:

```
ThreadSanitizer:DEADLYSIGNAL
==439589==ERROR: ThreadSanitizer: SEGV on unknown address 0x000000000018 (pc 0x598496bd4243 ...)
==439589==The signal is caused by a READ memory access.
==439589==Hint: address points to the zero page.
```

The identical address. Under `gdb` with `handle_segv=0`:

```
Thread 2 "min" received signal SIGSEGV, Segmentation fault.
0x0000555555630243 in __tsan_func_entry ()
#0  0x0000555555630243 in __tsan_func_entry ()
#1  0x000055555563e455 in body (p=...) at min.c:5
#2  0x00007ffff7ca3c7b in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:451
#3  0x00007ffff7d375cc in __GI___clone3 () at .../clone3.S:78
=> 0x555555630243 <__tsan_func_entry+19>:	mov    0x18(%rdi),%rax
```

`%rdi` is the `ThreadState *` `__tsan_func_entry` was handed; it is null,
because the thread was never registered. Two further measurements close it:

* `nm` on the built binary finds `___interceptor_pthread_create`,
  `__interceptor_pthread_create` and `__interceptor_trampoline_pthread_create`,
  and **no `thrd_create` interceptor of any kind**, while the binary's dynamic
  imports include `thrd_create@GLIBC_2.34`.
* The same program written against `pthread_create` (`minpthread.c`) runs
  **clean**, printing its result.

Toolchain: Ubuntu clang 21.1.8, Ubuntu GLIBC 2.43-2ubuntu2.4, x86-64.

**Why wave 36's minimal program ran clean.** `measure/probe/minempty.c` — a
`thrd_create` program whose body is `{ (void)p; return 0; }` — exits 0 under
TSan. Its body contains no memory access and no call, so clang instruments it
not at all: there is no `__tsan_func_entry` to fault. The minimal program was
too minimal, and the row it produced was a false negative about the toolchain
rather than a fact about QCore. Recorded here because the same trap is easy to
fall into again.

### 2.3 The fault reproduces on the route, at one pc

Each of the five subjects below, built `-fsanitize=thread` with no binding,
dies at **`0x18`** with pc **`0x555555630243`** — the same pc in all five, and
equal to `__tsan_func_entry+19` as symbolized above. Exit status 66 in every
case (`measure/probe/run1/*.bare.*`).

## 3. Fix or bound

There is **nothing in QCore to fix**. Emitting C11 `thrd_create` is correct and
deliberate: the route must also emit for MSVC, and `c11_threads_compat.h`
exists precisely to carry that name onto Windows and Apple. Changing the
emitted primitive to `pthread_create` would weaken portability and move every
published program's bytes to work around a gap in a sanitizer runtime. So the
answer is the other one the charter allows: **work around it at the harness,
and bound what the resulting row can say.**

`tests/support/thread_sanitizer.rs` carries `PTHREAD_BINDING`, a header
prepended through `-include` that replaces `thrd_create`/`thrd_join` at their
**call sites** with pthread-backed equivalents. `<threads.h>` is included
*before* the macros are defined, so the real declarations are never rewritten.
One binding covers both emitters, which reach the same two names from different
directions: the task runtime through its own documented
`#ifndef QCORE_TASK_CREATE` hook — whose default *is* `thrd_create` — and the
mutex-invariant backend through a literal call.

**Emitted C changed by zero bytes.** No file under `crates/qcore-core/src/`,
`compiler/` or `examples/` that contributes to emission is touched by this
card; the C the compiler produces is compiled verbatim and the binding arrives
entirely through a compiler flag, the same mechanism the existing
allocation-tracking harness uses for `QCORE_BUFFER_MALLOC`. There is therefore
no emitted-program byte diff to state, and §6 records the check that confirms
it rather than an argument that it must be so.

### 3.1 What the binding costs, stated rather than elided

The program TSan observes differs from the published program in **which symbol
spawns the thread**. It is not a different concurrency structure:

* glibc implements `thrd_create` by calling `pthread_create`'s implementation.
  This is not read from documentation — it is visible in the faulting backtrace
  in §2.2, where the frame below the unbound `thrd_create`'d worker is
  `start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:451`. Both
  builds reach the same `start_thread` on the same `clone3`.
* Every happens-before edge TSan reasons about here — thread creation and
  thread join — is therefore the same edge in both builds.
* The replacement bodies are not invented for the test. They are the
  pthread-backed `thrd_create`/`thrd_join` that the shipped
  `crates/qcore-core/src/c11_threads_compat.h` already compiles on Apple hosts,
  so every task program in this repository is already built this way on one
  supported platform.
* The binding's one representation assumption — that `thrd_t` and `pthread_t`
  are one representation — is a `_Static_assert` in the header, not a comment.

What the row therefore **cannot** speak for: the `thrd_create` call itself, the
Windows thread arm, and the Apple arm. Registered as
`TASK-ROUTE-TSAN-PTHREAD-BINDING`.

## 4. The TSan row and its non-vacuity

(to be written)

## 5. What TSan says about each subject

(to be written)

## 6. Gate

(to be written)

## 7. Residuals

(to be written)

## 8. Integrator reconciliation list

(to be written)
