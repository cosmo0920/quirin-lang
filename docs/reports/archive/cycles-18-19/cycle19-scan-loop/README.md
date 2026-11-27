# Cycle 19: structured Monkey injection scan

Date: 2026-09-06. Status: bounded 19D source syntax increment complete;
Cycle 19 remains in progress.

All **15 selected wrappers now have source projections**. The prior 14 retain
81 flat symbolic paths. `_mk_event_inject` adds two structured branches: duplicate
prevention disabled, or a read-only scan with an early function return. These
are not an enumeration of every loop iteration/path and do not establish
termination, bounds, arithmetic validity, memory safety or custody.

## Structured loop semantics

`ScanLoop` preserves the induction local, initial value, per-iteration header
reads/test, duplicate-test reads, early-return value, and typed increment.
A false header continues after the loop. A true match returns from the entire
function before later fired-array writes and count increments. Otherwise the
increment is stored in the induction local and the next iteration begins.

Only the bounded syntax of an already declared integer local, direct initializer,
unit increment and one conditional early return with no calls/writes is accepted.
There is no bound on execution inferred from that syntax. Other loops, side
effects, unsupported control and escaped induction locals remain Unknown.
Dot access to a selected ordinary record is retained as a field of its explicit
address. Source C integer types and casts are preserved.

Loop values and per-iteration read symbols have their own checked scope. They
cannot escape into the outer path. The ordinary local evaluator deliberately
does not infer a final index; a later direct use remains Unknown. The callback
execution binder explicitly rejects structured scans until it has a loop model.

The source report advances to **schema v2**, with an explicit description of
scan control. Callback/descriptor native observation schemas are unchanged.
Consumers must interpret the structured operation, including early return from
the function, rather than flatten its nested reads into one ordinary path.

Four new Lean theorems describe a finite scan-observation prefix: no fuel stays
Unknown, false header exits, a match returns before another iteration, and an
unmatched active iteration preserves the next iteration. These do not prove
Clang/Rust import, C machine arithmetic, memory bounds or universal termination.

## Validation

All 342 workspace tests, audit-example tests, formatting, warnings-denied
Clippy, Lean build and Cycle 19 dependency audit pass. Mutation regressions
retain changed return values and reject decrement, writes/calls in the scan,
escaped induction state and unsupported source effects. One workspace attempt
encountered the previously recorded plugin-test `Text file busy` error; the
complete rerun passed.

The pinned resource inventory and native chunk sidecar remain byte-identical.
The three callback summaries and the other 14 wrapper projections are unchanged.
Generic counts remain 185 units, 4,688/4,775 projected functions, two Safe,
98 provisional Unsafe, 81 Unknown, four data-only and zero gateway failures.
These projections are not proved coverage or confirmed upstream defects.
Native observation instrumentation was unchanged and was not rerun.

Reproduce with the [chunk-helper report's commands](../cycle19-chunk-helper/README.md),
using `/tmp/cycle19-scan-source.json` and `/tmp/cycle19-scan-diagnostics.json`
as report paths. The source report, validation logs and hash manifest accompany
this report.

Remaining gates are 19B source identity/custody mappings; 19C complete chunk
lifecycle and nested teardown; 19D scan safety/invariants, full wrapper/map
provenance, failure custody and Cycle 18 event composition; and their 19E
formal composition. Importing every wrapper's syntax does not close those gates.
