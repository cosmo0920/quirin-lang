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
