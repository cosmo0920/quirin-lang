# Quiet C11 Emission, Composable Streams, and Logger Levels

Status: normative amendment to revision 0.3  
Date: 2026-09-11  
Affected surfaces: source standard modules, reference observations, C11
emission, native conformance tests, examples, and documentation

## 1. Purpose

Quirin programs must not acquire observable output merely because the compiler
selected a particular lowering route. Output is a source-level effect. The C11
backend previously wrote compiler-synthesized execution transcripts for
sequential custody, extern calls, tasks, mutexes, and events. Those transcripts
were useful while the corresponding models were brought up, but they are not
program output and must not remain in production C11 emission.

This amendment establishes three rules:

1. generated programs are quiet unless their source explicitly performs I/O
   or selects a diagnostic threshold;
2. I/O observations form a composable `Streams` value rather than one merged
   process transcript; and
3. `std.logger` has the six levels `trace`, `debug`, `info`, `warn`, `error`,
   and `fatal`.

This amendment changes no custody, cleanup, error, concurrency, or contract
admission rule. Removing an implicit observation must never promote an
`Unknown` result to `Safe` or discharge an obligation.

## 2. Terminology

### 2.1 Program output

Program output is a sequence of bytes requested by an admitted source event
from `std.io` or `std.logger`. System-trace output is separately enabled by an
explicit source `log_level` declaration on profiles that admit it. A future
standard module may introduce another stream only by defining its identity,
effect, ordering, and C lowering.

### 2.2 Compiler observation

A compiler observation is structured verification data derived from a checked
program or collected by a verification harness. Resource acquisition, release,
move, task creation, task collection, mutex operations, extern calls, event
dispatch, return values, and run-time contract checks are compiler
observations. They are not program output.
Their event route remains load-bearing and MUST NOT be deleted merely because
its former stdout rendering is removed.

### 2.3 Debug print

A debug print is any compiler-synthesized write to a program stream. This
includes, but is not limited to, the historical `QCORE_TASK_*`,
`QCORE_BUFFER_*`, `OK`, `ERR`, `call`, `raise`, `return`, mutex-event, and
event-loop transcript lines. A diagnostic written by the `quirin` compiler
process before a generated program runs is not a debug print under this
definition.

## 3. Normative quiet-emission rule

The following requirements are mandatory.

1. A generated C11 translation unit MUST write no byte to a process stream
   unless an admitted source event requests that write or an admitted
   `log_level` declaration explicitly enables the diagnostic sink.
2. A source module that imports neither `std.io` nor `std.logger` MUST emit no
   `fprintf`, `printf`, `fputs`, `puts`, `fwrite`, or `fflush` call for output.
3. Such a module MUST NOT include `<stdio.h>` solely for compiler
   instrumentation.
4. A generated run-time failure MUST be represented by the route's checked
   error path, a non-zero process status, or a trap as already specified. It
   MUST NOT implicitly log a message.
5. `std.io` and `std.logger` are the only owners of formatted stream writes. A
   custody, task, mutex, extern, event, or other backend MUST submit compiler
   observations to the typed diagnostic logger described in section 6; it MUST
   NOT format or write those observations directly.
6. `--debug-emit`, `--save-temps`, or an equivalent compiler option may expose
   generated C source. It MUST NOT change the generated program's observable
   behavior.
7. No default-off preprocessor branch containing ad-hoc formatted debug writes
   is a compliant substitute. The retained event route MUST call a typed logger
   interface. Its ordinary sink is the null sink, so ordinary emission neither
   formats nor writes the event.

Without `log_level`, exact stdout and stderr are determined by source I/O
events alone. With it, diagnostic records at or above the selected threshold
are additionally written to stdout by the revision-0.3 hosted adapter. Source
`log LEVEL ...;` records remain on stderr, so the two mechanisms compose
without conflating application logs with system traces.

## 4. The `Streams` observation algebra

### 4.1 Domain

`Streams` is a finite map from a stream identity to an ordered byte sequence.
Revision 0.3 defines two stream identities:

```text
stdout
stderr
```

The Rust representation may store complete lines while every admitted
revision-0.3 operation remains line-producing. This is an implementation
choice; the semantic value is the byte sequence obtained by joining those
records in order.

### 4.2 Identity and composition

The identity value, written `empty`, maps every stream to the empty sequence.
Composition, written `left ++ right`, is pointwise concatenation:

```text
(left ++ right)[stream] = left[stream] followed by right[stream]
```

Composition is associative and has `empty` as both a left and right identity.
It does not create an order between distinct streams. Consequently:

```text
stdout: ["a\n"] ++ stdout: ["b\n"] = stdout: ["a\n", "b\n"]
stdout: ["a\n"] ++ stderr: ["b\n"]
```

preserves order within each stream and states no relative order between the
two streams.

### 4.3 Source events

The existing source forms contribute these singleton values:

```text
print RENDERING OPERAND;             => stdout: [render(OPERAND) + "\n"]
log LEVEL RENDERING OPERAND;         => stderr: [tag(LEVEL) + " " + render(OPERAND) + "\n"]
flush;                               => empty, plus a sink-handoff event for stderr
return;                              => empty
```

A program's `Streams` value is the composition of its source events in program
order. Module and model composition MUST combine these values with the same
operation. No composition rule may inject a compiler observation.

### 4.4 C sinks

The hosted C11 backend maps `stdout` and `stderr` to the corresponding C
streams. A freestanding backend may bind them to target-supplied sinks. Sink
selection is a backend capability and must not change the `Streams` value.

## 5. `std.logger` levels

### 5.1 Grammar

The logger grammar is amended to:

```text
LEVEL ::= "trace" | "debug" | "info" | "warn" | "error" | "fatal"
log-statement ::= "log" LEVEL RENDERING OPERAND ";"
```

The level order is:

```text
trace < debug < info < warn < error < fatal
```

Source logger events are not implicitly filtered: source code explicitly asked
for those records. Compiler observations use a separate diagnostic logger sink
with an explicit threshold. Its default is `off`, which is stricter than
`fatal`; enabling it is a compiler/debug-build choice and is recorded in the
emission identity. Fan-out, buffering, and asynchronous delivery of
source-authored logger events require a later amendment.

### 5.2 Rendering

The canonical tags are:

```text
trace -> [TRACE]
debug -> [DEBUG]
info  -> [INFO]
warn  -> [WARN]
error -> [ERROR]
fatal -> [FATAL]
```

The remainder of a log line is unchanged.

### 5.3 The `error` keyword

`error` remains the error-type declaration keyword. In the token sequence
immediately following the `log` statement head it is also the logger level
`error`. This is contextual interpretation, not a new global identifier
category. An implementation MUST NOT rename this level to avoid the existing
keyword and MUST NOT reinterpret an error declaration as a log level.

`trace` and `debug` SHOULD likewise be treated as contextual logger words. If
the current name resolver requires them to be globally reserved during the
migration, the implementation must first census every maintained source
fixture and record that compatibility decision. The desired steady state is
contextual interpretation.

Malformed or unknown levels continue to use the StandardIo pass's existing
shape diagnostic. No diagnostic code is added or reassigned by this amendment.

## 6. Retained trace events and diagnostic logging

### 6.1 Event preservation

All structured trace-event constructors, reference relations, certificate
fields, and replay checks remain. This amendment changes their rendering path,
not their existence or meaning.

The default C11 lowering connects the diagnostic logger to a null sink. An
explicit verification or debug build may inject a typed in-memory sink. A
source profile may also admit `log_level LEVEL;`; that declaration installs the
profile's standard diagnostic adapter and threshold. The revision-0.3 hosted
adapter writes those selected records to stdout. The chosen sink and threshold are part of
the emission identity.

### 6.2 Level assignment

Fine-grained events are logged at `trace`. These include resource
acquire/release/read/write/move records, invariant open/close records, argument
and identity detail, and individual event-loop operations.

Lifecycle summaries are logged at `debug`. These include task spawn/join/result
records, mutex create/destroy records, extern-call entry/outcome records, and
whole-run outcome summaries.

Run-time failures retain their existing status or trap semantics. A diagnostic
logger may additionally receive them, but logging MUST NOT replace or weaken
the failure.

### 6.3 Typed boundary

The C emitter MUST target a typed diagnostic logging interface rather than
constructing a printable line at each event site. Formatting, when enabled,
belongs to one logger implementation. The null implementation must be usable
without `<stdio.h>`.

## 7. Verification without program-stream instrumentation

Removal of debug prints does not remove the observations required to validate
a lowering.

1. Reference interpreters MUST continue to derive structured custody, task,
   mutex, extern, and event observations where their profiles require them.
2. Certificates MUST continue to carry every observation required by their
   existing recheck. Those observations are evidence records, not `Streams`.
3. Native tests MUST inspect semantic state through one or more of:
   typed in-memory observation hooks, injected allocator/task/platform hooks,
   explicit result/status checks, sanitizer findings, or a bounded self-check
   performed by the generated test program.
4. A native semantic-verification hook MUST be the typed diagnostic logger
   interface or an equally typed adapter over it. Its evidence MUST remain
   structured; a test MAY additionally compare a formatted transcript when it
   is specifically testing an explicit `log_level` adapter.
5. Ordinary generated programs retain calls to the typed diagnostic logger,
   whose default implementation is the null sink. They MUST NOT carry a test
   event store. A formatted diagnostic sink is present only when source
   explicitly selects `log_level` for a profile that admits it. A verification
   build must be requested explicitly by the test or verifier API, and its
   sink and threshold must be recorded in the evidence identity.
6. Tests MUST NOT use a formatted stdout/stderr transcript as the sole semantic
   evidence for compiler observations. Transcript tests for explicit
   `log_level` examples pin the adapter presentation, not the certificate.

This separation prevents a program from acquiring an I/O dependency merely
because its lowering is under test, while preserving independent native
evidence for resource and concurrency behavior.

## 8. Tests and examples

The migration is complete only when all of the following hold.

1. No maintained example receives a compiler-synthesized debug line by
   default. A system-trace example MUST contain an explicit `log_level`.
2. Every byte expected from an example is attributable to a `print`, `log`,
   future explicit stream statement, or explicit `log_level` in that example's
   source.
3. An example that intends to demonstrate visible output imports the module
   granting that output and declares the `io` effect.
4. Tests of source I/O compare the emitted process streams with the composed
   `Streams` reference value.
5. Tests of custody, tasks, mutexes, and extern calls retain structured checks.
   Example tests MAY additionally pin system-trace stdout only when that
   example contains an explicit `log_level` declaration.
6. Tests cover all six logger levels, their canonical tags, their order, and
   the contextual use of `error`.
7. A structural test rejects any direct or indirect stream write in a non-I/O
   emitted module.
8. Hosted and freestanding emission tests demonstrate that a non-I/O program
   carries no stdio dependency.

Frozen evidence archives are not edited. Superseding measurements and changed
C emission digests are published in a new report beside them.

## 9. Compatibility and versioning

This is an intentional observable-output change for executable modules that
previously received synthesized transcripts. Source programs did not request
those bytes, so they are removed rather than deprecated as language output.

The following remain stable:

- source custody and effect judgments;
- source-authored `print`, `log`, and `flush` behavior, except for the additive
  logger levels;
- certificate refusal behavior;
- existing diagnostic ownership; and
- C ABI projections for declared native functions.

The following must be re-measured:

- emitted-C byte digests;
- examples whose documented transcript included compiler observations;
- native/reference tests that parsed those observations; and
- observation-reordering evidence whose alphabet included synthesized trace
  records.

## 10. Acceptance conditions

The amendment is implemented only when:

1. repository search finds no compiler-synthesized formatted stream write at
   an event site outside the logger implementation, while every prior trace
   event still reaches the typed diagnostic logger;
2. a module with no admitted I/O emits C with no stdio output route;
3. composed modules emit exactly their source-authored `Streams` value;
4. all six logger levels parse, check, interpret, emit, and execute with their
   canonical tags;
5. the full workspace gate, provider gate where affected, Python checks, Lean
   audits, and both reevaluation probes pass; and
6. the new report states which prior transcript and digest claims this
   amendment supersedes without modifying a frozen archive.
