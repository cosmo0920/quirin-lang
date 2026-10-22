# AUDIT-IR-WORKING-SET

Base: `53e9b648eb05f8a3a2f2bfb94084d52f49a6422d`.
Parent: [AUDIT-IR](../../development/audit-ir-card.md).

Prediction before implementation: bounding ordered dispatch to a moving window
limits unconsumed unit results without changing task-order publication, first
error or panic position. Borrowed streaming serialization avoids cloning final
report rows and buffering the full encoded document, retaining exact pretty
JSON bytes, sorted fields and trailing newline. Ordinary entry validation can
move its CFG into the candidate and recover that same CFG for a refused entry;
derived-return fallback must retain its distinct ordinary CFG. None of these
changes alters certificate replay, contracts, verdict rules or `.qc` authority.

Add an optional `memory_budget_mib` configuration / `--memory-budget-mib` current
CLI setting. It caps requested concurrency using an explicit planning reserve
of 1,024 MiB for the coordinator plus 2,048 MiB per worker. This is a scheduling
estimate, not a hard RSS bound: a large single unit or retained published rows
may exceed it. Invalid or sub-minimum budgets refuse. Record budget, requested
and effective jobs; preserve census serialization. Hard-budget acceptance still
requires whole-cgroup measurement, including subprocesses, with no omitted units.

Acceptance: ordered bounded liveness and cancellation/panic tests, exact output
byte parity including optional evidence and writer errors, contract-entry
fallback parity, CLI/config budget validation, complete standing gate and a
fresh whole-job measurement. The previous four-job OOM receipt stays frozen.
Per-component AST/final-summary paging and two-phase product spooling remain
open unless this slice implements and measures them.

Implemented mechanisms: the executor shares one mutex/condition-variable state
for dispatch, completion and cancellation. The next worker may start only an
index in `[frontier, frontier + jobs)`; priority is applied within that range.
The frontier advances after the consumer releases its product. Both consumer
errors and unwinding stop and wake waiting workers. The bound covers products
waiting for publication, not rows the consumer intentionally retains for the
run's later refutation/member folds.

Report publication borrows rows, diagnostic counts and optional evidence through
a sorted map, writes pretty JSON into a buffered file writer and checks both
writes and final flush. The owned checkpoint-document API remains available.
Ordinary contract-entry analysis consumes its CFG and recovers the candidate's
unchanged CFG on a refused entry; derived-return validation keeps its distinct
ordinary fallback. Conflict and final-summary postprocessing still run.

The planning budget is optional. `--jobs` precedence is preserved, then the
budget caps it, and any active census still forces one job. The current CLI's
flag and JSON budget must agree; the configured JSON route accepts the same
field. With no budget the invocation record retains its previous shape. With a
budget it records both reserves and `memory_limit_enforced: false`. The example
7168 MiB is a binary-unit planning budget, not 7,000,000,000 bytes.

[gate.json](gate.json) records the complete standing gate: **537 Rust units,
550 summaries, 8,498 passed, 0 failed, 15 ignored**; **362 Lean audits**;
**374 Python tests**. Submodules, Clang, format, all-target Clippy and both
reevaluation examples exit 0. Focused controls pass parallel 12, streaming 1,
budget parsing 1, declared entry 8 and current CLI 17 tests; they are included
in the full total. All 1,498 tracked Rust files remain unchanged during the
campaign. Shared main `b5f74d79a` contains the separately committed Kafka
contract relocation; its production corpus pins pass in this combined gate.

The raw campaign driver exits 1 only because its optional census probe names
a nonexistent feature (`semantic-census-sinks`, exit 101). The corrected
`indirect-census` check exits 0; both commands and logs are retained. An earlier
focused probe's shell wrapper also exits 1 because `status` is read-only in
zsh; its accepted rerun exits 0. The first host-gate launch was not executed
because automatic approval review timed out; the one retry executed normally.
Exact commands, exits, source hashes and private paths are in the gate receipt.
No semantic rule changes; existing matching audited Lean definitions remain.

The release build and whole-job measurement follow separately; no new numeric
memory acceptance is claimed by the test gate. The previous failed dev-profile
measurement remains frozen in the admission report. Remaining work includes
bounded two-phase retained rows, component bodies retained during independent
replay, buffered whole-unit Clang output and per-component AST/CFG paging.
