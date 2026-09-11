# Quiet emission and composable streams

This increment implements
[`QUIET_EMISSION_STREAMS_AMENDMENT.md`](../../design/QUIET_EMISSION_STREAMS_AMENDMENT.md).
It separates source-authored program I/O from compiler observations without
removing the structured trace routes on which reference execution,
certificates, and replay checks rely.

## Implemented boundary

The sequential, task, mutex, extern-call, and event C11 emitters submit typed
diagnostic events at `trace` or `debug` level. Ordinary emission selects a
threshold above `fatal` and a null sink, so these events produce no process
stream bytes and add no `<stdio.h>` dependency. A verification harness may
replace the typed sink and lower the threshold; formatting is not embedded at
event sites. Maintained hosted profiles additionally admit one explicit
`log_level LEVEL;` declaration, gated by `import std.logger;`. It installs the
shared system-trace stdout adapter and records the threshold in the checked
certificate. Existing run-time refusal, exit-status, and trap behavior is
unchanged.

`std.io` and source-authored `std.logger` remain the formatted I/O owners. The
reference value is now exposed as `Streams`, a per-stream ordered collection
with pointwise concatenation. Rust tests and Lean theorems pin associativity
and both identity laws. `std.logger` accepts, in order, `trace`, `debug`,
`info`, `warn`, `error`, and `fatal`, with uppercase bracketed tags. The
already-reserved `error` token is interpreted contextually after `log`.
The maintained-source fixture census found no binding named `trace` or
`debug`; the current resolver therefore reserves those two words during this
migration, as recorded in the amendment and `AGENTS.md`.

Examples receive no system-trace bytes unless their source selects them. The
event, thread, branching, pipeline, and declared-extern examples make that
choice explicit with `log_level trace;` and show the rendered records on
stdout. The unsafe-linkage example
uses source-authored `log trace` statements for the returned scalars rather
than exposing compiler observations. Native tests retain reference observations
and use status, sanitizer, heap/task hooks, source-I/O projection, or
typed-emission structure as appropriate. CT03 and CT17's live fixture
expectations require quiet stdout while retaining their reference models and
heap witnesses.

## Compatibility and superseded claims

This report supersedes earlier statements that compiler-synthesized
`QCORE_TASK_*`, `QCORE_BUFFER_*`, mutex, extern, sequential, and `QCORE_EVENT_*`
text is part of an emitted program's stdout/stderr transcript. In particular,
the byte-transcript portions of `TRACE-STDIO`, `COMPOSE-CONCURRENCY`,
`OBS-REORDERING`, `STD-EVENT`, and their dependent native example claims are no
longer current. Their structured event alphabets, causal relations, checked
models, and refusal conclusions remain current.

No frozen report or artifact was edited. Emitted-C text and therefore any
digest over it intentionally change. This increment does not re-pin the Cycle
19 corpus: the unrestricted-C gateway and its 272-leak baseline are outside
the changed source-language emitter routes.

## Validation

At the user's direction, the full standing per-change gate was not run for
this increment. Targeted Rust checks, Windows native transcript runs, formatter
checks, and the affected Lean builds/audits were run from the repository root.
Provider packaging was not rerun because no provider, schema, package template,
or provider protocol changed. The exact completed command results and this
boundary are recorded in the corresponding `DEVELOPMENT.md` increment entry.

The implementation remains bounded. A public compiler option for selecting a
diagnostic sink independently of source, a stable external diagnostic-event
ABI, additional named streams, and a freestanding sink binding are follow-on
work; ordinary portable C11 emission already has no diagnostic stdio
dependency.
