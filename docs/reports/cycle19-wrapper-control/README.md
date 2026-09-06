# Cycle 19: wrapper assertion and terminal control

Date: 2026-09-06. Status: bounded 19D source-import increment complete;
Cycle 19 remains in progress.

The pinned audit retains **81 symbolic paths from 14/15 wrappers**, up
from 41 paths from ten. There are 7 explicit nonreturning paths. The
new wrappers are `_mk_event_add` (20 paths), `_mk_event_del` (12),
`_mk_event_timeout_create` (4), and `_mk_event_channel_create` (4).
`_mk_event_inject` remains Unknown because its variable-bound loop requires
an invariant or an explicit bounded execution profile.

## Preserved source boundary

The importer handles only literal-zero `do` macro conditions, retaining the
entire body and the false exit guard. Break, continue, nonliteral conditions
and unsupported statements even after an early return still fail closed.
Clang source-declared noreturn call statements produce a terminal path; following
calls/returns are absent. Terminal paths do not discharge resource obligations
and are not confused with successful teardown. Nested noreturn call expressions
remain Unknown. Lexical local-end markers are storage bookkeeping, not emitted
C destructor calls.

Wrapper direct calls now retain arbitrary exact source names, declarations and
arguments as **opaque** operations. This removes the wrapper callee name table
as a syntax bottleneck without assuming library semantics. Indirect calls still
fail closed; the callback-specific profile is unchanged. Compiler prediction
builtins are also opaque: their result guards require consumer semantics, and
no relationship to the predicted argument is assumed. List and event-reset
calls are retained but do not yet establish list/map custody.

Two-element array typedefs are resolved against an unambiguous source typedef
to int, preserving their source type spelling. Other element types, ambiguous
aliases and qualifiers fail closed. The execution binder rejects terminal paths
and aggregate storage until its model supports them.

Four Lean theorems establish the abstract terminal/return suffix boundary and
literal-zero macro control. They do not prove the Rust importer, compiler
builtin semantics, C declarations, call effects or universal C safety.

## Validation and next gates

All 337 workspace tests, audit-example tests, formatting, warnings-denied
Clippy, Lean build and Cycle 19 dependency audit pass. Mutation tests retain
terminal failures, reject hidden unsupported control and nested nonreturning
expressions, and reject wrong/qualified stack element types.

The pinned resource inventory remains byte-identical: 185 selected units,
4,688/4,775 projected functions, two Safe, 98 provisional Unsafe, 81 Unknown,
four data-only and zero gateway failures. The three callback summaries remain
identical. These are source projections, not new proved functions or upstream
defect claims. Native instrumentation was unchanged and was not rerun for this
source-only increment. Logs, source JSON and hashes accompany this report.

Reproduce using the commands in the [storage report](../cycle19-wrapper-storage/README.md),
with report outputs `/tmp/cycle19-wrapper-control-source.json` and
`/tmp/cycle19-wrapper-control-diagnostics.json`.

The remaining work is source-to-identity custody for 19B, ChunkIO/wrapper
lifecycle and nested teardown for 19C, the inject loop and full map provenance,
failure custody and Cycle 18 event composition for 19D, then 19E composition.
