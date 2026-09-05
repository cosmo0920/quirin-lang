# Cycle 18: Libevent Event-Driven Concurrency Verification

**Status: Bounded implementation and native platform matrix complete; aggregate acceptance pending**

Cycle 18 defines and verifies an event-driven concurrency profile modeled on
Libevent. It builds on Cycle 17's native backend and C11 memory relation; it
does not claim that an event loop makes arbitrary callbacks sequential or that
QCore-C verifies every Libevent API automatically.

The implementation must pin an exact Libevent release or commit, build
configuration, threading mode, and event backend. The verified artifact is the
combination of the QCore-C event model, the adapter for that pinned build, and
the checked application callback summaries.

## Verification boundary

The initial accepted profile covers:

- event-base creation, configuration, dispatch, loop-once/nonblocking modes,
  loop break, loop continue, and timed loop exit;
- initialized, pending, active, running, finalizing, and freed event states;
- read, write, timeout, signal, persistent, edge-triggered, and explicit
  activation flags where the selected backend supports them;
- event construction/assignment, add/reschedule, blocking and `EV_FINALIZE`
  nonblocking deletion, priority assignment, pending inspection, explicit
  finalization, and free;
- callback arguments containing owned, borrowed, shared, and quantitative
  resources under explicit callback summaries;
- cross-thread base notification after supported Libevent thread setup;
- socket bufferevents, evbuffers, immediate and deferred callbacks,
  thread-safe locking, unlocked callbacks, watermarks, and close-on-free; and
- callbacks that enqueue Cycle 17 scoped tasks, provided their task and event
  lifetimes are joined before teardown.

DNS, HTTP, RPC, OpenSSL, evconnlistener, custom memory/lock callbacks,
fork-reinitialization, and private Libevent APIs begin as `Unknown`. Each needs
a separate profile extension before it can contribute to a safety theorem.

## 18A: Event-reactor IR and executable semantics

Add an identity- and generation-indexed event IR with records for bases,
events, file descriptors, callback arguments, priority queues, active reasons,
deferred callbacks, and loop-control requests. The transition system includes:

```text
construct/assign -> add/reschedule -> activate -> callback enter/exit
                                      |              |
                                      +-> delete ----+
callback exit -> persistent re-pend or one-shot idle -> finalize/free
```

Readiness, timeouts, signals, and ordering within one priority are
nondeterministic inputs. Higher-priority activation constrains which queue is
selected, but does not prove fairness, happens-before, or resource ownership.
Recursive `event_active` and zero-delay timers must remain executable without
silently assuming progress for other events.

The reference interpreter explores bounded activation choices and produces a
partial-order trace containing lifecycle transitions, callback entries and
exits, application resource accesses, lock/atomic edges, file-descriptor
operations, and teardown.

## 18B: Lifetime, custody, and callback checking

Treat every event, base, bufferevent, descriptor, and owned callback argument
as an identity with explicit custody. Reject stale generations, double free,
use after base/event free, duplicated descriptor-close authority, callback
arguments that outlive their borrows, and teardown with a running callback or
unaccounted persistent event.

Deletion removes pending and queued activation. Model ordinary blocking delete,
`EV_FINALIZE` nonblocking delete, and explicit finalization separately. A
nonblocking deletion does not establish that a callback already running on
another execution context has returned; completed finalization or another
quiescence token is required before destroying callback-owned state.
Event-base destruction accounts separately for Libevent-owned event storage
and application-owned callback arguments.

Callback summaries declare:

- trigger reasons and base identity;
- read/write footprints and quantitative effects;
- ownership and borrow modes for the callback argument;
- events or bufferevents that may be added, deleted, activated, or freed;
- task, lock, atomic, error, and cleanup effects; and
- whether re-entry, deferral, persistence, or post-return execution is
  possible.

## 18C: Concurrency and happens-before model

Prove mutual exclusion only for ordinary callbacks dispatched by the same
event base while it has one dispatcher. Multiple bases may dispatch in
parallel. Immediate bufferevent or evbuffer callbacks may execute in the API
caller's context; deferred callbacks run through the associated base.

Libevent thread initialization and object locks authorize supported
cross-thread library operations. They do not grant application data
permissions. Model lock-held and `BEV_OPT_UNLOCK_CALLBACKS` invocation as
different transitions, and require application mutex, atomic, or custody
evidence for conflicting payload accesses. Cross-thread activation/wakeup
orders queue visibility only through documented synchronization edges.

Compose callback events with Cycle 17 vector clocks and C11 executions. Reject
or report `Unknown` for conflicting callbacks on different bases, callbacks
and worker tasks, unresolved callback targets, and configurations built with
locking disabled.

## 18D: Libevent importer and runtime adapter

Extend the legacy-C importer with summaries for the accepted `event2/event.h`,
`event2/thread.h`, `event2/buffer.h`, and `event2/bufferevent.h` surface. Resolve
callback targets and callback-argument aliases interprocedurally. Preserve
`Unknown` for function pointers or wrappers whose lifecycle and effects cannot
be proved.

Build a thin observation adapter against the pinned upstream version. It must
record event identity/generation, activation reasons, callback boundaries, and
synchronization without introducing a global logger lock that serializes the
program. Use per-dispatcher buffers and merge traces by explicit ordering
edges.

## 18E: Lean model and theorem obligations

Add a `QCore.EventDriven` model and prove:

1. every transition preserves well-formed base and event lifecycles;
2. a callback begins only with a live argument and its declared authority;
3. ordinary callbacks on one base never execute concurrently;
4. every conflicting cross-base or worker access is ordered or rejected;
5. event and descriptor close authority is unique and generation-safe;
6. persistent reactivation and deferred invocation preserve custody;
7. accepted deletion, finalization, and base teardown are quiescent;
8. errors and callback-triggered cleanup discharge every owned resource once;
9. progress is qualified by possible I/O, timeout, signal, or explicit loop
   blocking rather than stated as unconditional liveness; and
10. every observed execution of the pinned adapter refines an allowed event-IR
    trace under the selected backend contract.

The trust manifest must name the Libevent version, headers, configuration,
backend, OS adapter, C compiler, sanitizer availability, foreign callback
summaries, and any runtime observation hooks.

## Test and platform matrix

The executable corpus must include:

- one-shot and persistent read/write/timer events;
- simultaneous equal- and mixed-priority activations;
- edge-triggered drain and missed-drain failures;
- add, reschedule, blocking/nonblocking delete, finalize, and free from inside
  callbacks;
- deletion before dispatch and deletion racing with cross-thread activation;
- loop break, continue, exit-on-empty, and timed exit;
- recursive activation and starvation-prone traces;
- immediate, deferred, thread-safe, lock-held, and unlocked bufferevent
  callbacks;
- buffer watermarks, close-on-free, backpressure, and pending output at free;
- callback/task handoff, typed errors, cancellation, and cleanup;
- allocation, registration, wakeup, and backend-selection failures; and
- unresolved callback/alias/configuration fixtures that must remain `Unknown`.

Run bounded model exploration independently of the native tests. Differential
tests use nonblocking loopback sockets or socket pairs and a controlled clock
where supported. Exercise select and poll plus epoll on Linux, kqueue on a
native supported BSD/macOS host, and select/IOCP profiles on native Windows.
Run AddressSanitizer and UndefinedBehaviorSanitizer for lifetime defects and
ThreadSanitizer where supported for cross-context races. Libevent debug modes
supplement but do not replace the QCore-C checks.

## Diagnostics

```text
Q700  event/base lifecycle transition is invalid or stale
Q701  event, base, bufferevent, or callback argument used after free
Q702  callback argument outlives its ownership or borrow contract
Q703  file-descriptor or close-on-free custody is duplicated
Q704  cross-thread Libevent operation lacks supported thread initialization
Q705  callback access conflicts across bases, callers, or worker tasks
Q706  deletion, finalization, or teardown lacks callback quiescence
Q707  persistent event or deferred callback leaves an owned obligation
Q708  priority, readiness, or timing is used as synchronization evidence
Q709  callback target, Libevent API, backend, or build profile is unknown
Q710  bufferevent callback mode or lock contract is violated
Q711  loop exit leaves unaccounted event or callback custody
Q712  observed Libevent trace does not refine the checked event model
```

## Acceptance gates

Cycle 18 is complete only when:

- the accepted API/profile table is exhaustive for every recognized Libevent
  function, flag, callback mode, and backend configuration;
- executable model tests, generated schedules, negative fixtures, native
  differential tests, sanitizer runs, and platform-specific backend runs pass;
- unresolved callbacks and unsupported APIs/configurations yield `Unknown`;
- lifecycle, callback authorization, same-base exclusion, cross-context race
  freedom, quiescent teardown, and runtime trace-refinement theorems build with
  no placeholders or project axioms;
- Linux, a native kqueue host, and native Windows results are recorded rather
  than inferred from one another;
- the dependency and trust audit includes the pinned Libevent source and build
  manifest; and
- `cargo test --workspace`, strict formatting and Clippy, a placeholder-free
  `lake build`, and `lake env lean AuditCycle18.lean` all pass.

## Primary upstream behavior references

- [Libevent API documentation](https://libevent.org/doc/)
- [Working with an event loop](https://libevent.org/libevent-book/Ref3_eventloop.html)
- [Working with events](https://libevent.org/libevent-book/Ref4_event.html)
- [Locks and threading](https://libevent.org/libevent-book/Ref1_libsetup.html)
- [Bufferevents: concepts and basics](https://libevent.org/libevent-book/Ref6_bufferevent.html)

## Implemented bounded slice

- `compiler/src/event_driven.rs` implements generation-indexed event and
  bufferevent identities, lifecycle transitions, callback summaries, payload
  authority, descriptor-close custody, watermarks and pending-output cleanup,
  same-base callback exclusion, explicit cross-context ordering evidence,
  bounded priority schedule exploration, fail-closed API/profile
  classification, adapter-observation refinement, and the pinned-source audit.
- `compiler/examples/cycle18_libevent_audit.rs` verifies the exact upstream
  commit and source contracts used by the model.  It rejects another revision
  even if the expected symbol names happen to be present.
- `proof/QCore/EventDriven.lean` defines the checked lifecycle relation and
  proves generation safety, callback authorization, same-base exclusion,
  cross-context ordering, unique descriptor/event identities, custody through
  persistent/deferred callbacks, quiescent teardown, exactly-once cleanup,
  qualified progress, and observation refinement.
- `proof/AuditCycle18.lean` reports the theorem dependencies.  The lifecycle
  relation lemmas use Lean's standard `propext`; the aggregate safety theorems
  have no axiom dependencies.

This bounded slice does not turn unsupported Libevent calls into accepted
ones, infer arbitrary function-pointer targets, or infer ordering from
priority/readiness/trace order.  DNS, HTTP, RPC, OpenSSL, listeners, custom
allocator/lock callbacks, fork reinitialization, private APIs, and unknown
symbols remain `Unknown`.

## Pinned source and validation

The upstream checkout is pinned to commit
`d82464a277d0f42703702c4dfd9af6af38595a83`
(`release-2.2.2-alpha-11-gd82464a2`). The Linux build uses GCC 15.2.0, pthread
support, debug mode, OpenSSL disabled, MbedTLS disabled, and the detected
EPOLL, POLL, and SELECT backends. The native macOS build uses Apple Clang
21.0.0 with the same feature flags and the detected KQUEUE, POLL, and SELECT
backends. The native Windows build uses MSVC 19.51.36248, Windows threads,
the WIN32 event backend, and the IOCP regression profile. Exact commands,
audited source contracts, and native results are
recorded in `docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md`.

Application-specific foreign callback summaries, Libevent sanitizer evidence,
and a deployed per-dispatcher observation hook must still be recorded before
the full aggregate acceptance gate above can be called complete. Results are
recorded independently for Linux, macOS/kqueue, and Windows/IOCP rather than
inferred between platforms.

## Verification commands

```sh
git submodule update --init --recursive
cargo run --example cycle18_libevent_audit
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle18.lean
```
