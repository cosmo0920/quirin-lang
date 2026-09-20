# Cycle 18 identity-bearing timed-wait observation

This increment adds a sealed, identity-bearing observation checker for one
bounded execution: the positive-timeout branch of Monkey's `mk_event_wait_2`
on one Linux dispatcher. The accepted trace has exactly eleven records, from
base creation through callback entry and exit to event, stack argument and base
end. Every record carries the run, dispatcher-owned buffer and local sequence;
resource-bearing records retain base, event and callback-argument identities
and generations. The checker consumes the complete trace and rejects missing,
extra, reordered, spliced, wrong-generation and multi-context records. Because
this profile establishes no cross-context order, it rejects every supplied
synchronization edge.

The certificate has private fields, no deserializer and no public constructor.
Only the checker can create it; callers can read its immutable checked context
and record count. This route is separate from the frozen kind-only
`ObservedKind` compatibility refinement. That older comparison still filters
lifecycle records and has no identity or generation, so it cannot bind a
production callback execution to a particular resource generation.

The wire adapter accepts separate raw `observations.jsonl` and `bindings.json`
bytes. It denies unknown fields, bounds each input and the record count, checks
the raw observation digest, exact profile/path/API set, consistent per-source
hashes and then calls the semantic checker. The returned source map is still
untrusted until the composition layer independently loads and hashes those
sources. The adapter does not manufacture source authority.

The native harness includes the authenticated Monkey `mk_event.c` from Fluent
Bit revision `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and interposes its Libevent
calls. It calls the real source wrapper and observes actual base, event,
argument, callback target, callback flags, priority, return values and backend.
The callback writes only to a fixed-size thread-local buffer. It performs no
allocation, lock or stdio; the harness flushes after `event_base_free`. Pointer
values are the run-local identities. Generation 1 is assigned by the observer
to each of the three single-creation slots; reuse and later generations are
outside this profile.

The reproduced Linux run built and linked `libevent_core.a` from the actual
`third_party/libevent` revision
`d82464a277d0f42703702c4dfd9af6af38595a83`, with Clang 21.1.8 and address and
undefined-behavior instrumentation applied to both Libevent and the harness.
Leak checking also completed outside ptrace. The runtime backend was `epoll`.
The bundled Monkey Libevent was not linked. The runner emits
`run-manifest.json`, `bindings.json`, `observations.jsonl`, dependency files, a
link map and `source-compile-commands.json`; the latter compiles the original
Monkey source with the pinned Cycle 18 Libevent include paths before bundled
paths. It labels that object as a source-configuration witness, not as a linked
Monkey archive. The linked harness object contains the included Monkey source
and has its own digest.

Reproduce from the repository root, substituting private output directories:

```sh
python3 scripts/cycle18_timed_wait_observation.py \
  --monkey-inputs build/cycle19/c-gateway-wave-pin/monkey/qcore-monkey-inputs.json \
  --monkey-source build/cycle19/c-gateway-wave-pin/monkey/monkey-source \
  --monkey-build build/cycle19/c-gateway-wave-pin/monkey/monkey-build \
  --libevent-source third_party/libevent \
  --libevent-build /tmp/qcore-cycle18-libevent \
  --output /tmp/qcore-cycle18-observer
```

This is runtime evidence for one successful two-millisecond wait. It is not an
all-execution safety result: the source does not check `event_new` or
`event_add` failure before continuing. Allocation/registration failures,
zero/infinite waits, other callbacks, event persistence and deferral, multiple
dispatchers or bases, cross-thread activation, real synchronization edges,
non-Linux hosts and other Libevent backends remain `Unknown`. ASan/UBSan/LSan
coverage here applies only to this dedicated execution and instrumented pinned
archive. It does not establish TSan coverage, the existing Monkey all-paths
trace, aggregate Cycle 18 acceptance or production-wide safety. In particular,
the existing all-paths trace's `cb_event` record is not filtered to invent this
dedicated execution.

The focused checks pass: two core semantic tests, six strict wire tests, the
203-module Lean build and the Cycle 18 theorem audit. The Lean model proves
exact-trace acceptance, absence of invented synchronization, binding of the
callback to the expected event generation and sensitivity to a changed event
generation. It models the checker relation; it does not prove the C observer,
wire parser, compiler, allocator, Libevent or operating system.

Standing gate commands, exits, counts and private logs are recorded in
[`gate/README.md`](gate/README.md).
