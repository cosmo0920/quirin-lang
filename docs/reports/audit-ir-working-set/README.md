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

## Release measurement and pre-pass follow-up prediction

At source `62f10abda`, the complete three-worker release audit also fails:
**6,999,998,464-byte peak**, `Result=oom-kill`, **299.069 seconds**, zero swap.
The driver exits 1; stop/reset both exit 0. The worker is killed before final
`memory.events` and input-hash receipts, so those fields remain unavailable.
[memory-release-budget3.json](memory-release-budget3.json) owns the exact inputs,
launch and accounting. No completed report or memory acceptance is claimed.
A separate two-worker run of that same binary is in progress, explicitly a
baseline for the next follow-up rather than evidence about code not yet built.

Prediction before the follow-up implementation: `member_prepass::run` clones
all previously read `MemberFacts` at every fixed-point round into a local vector,
which stays in scope during the next selected-unit reads. Replace only this
vector with ordered references to the existing map values and declaration unit,
ending that borrow before mutation. `closure_tokens` already accepts references;
there is no need to retain a second owned copy. Preserve the same member order,
closure tokens, reading fixed point, exclusions, refusals and published facts.
This removes a concrete ownership duplicate; its memory share is not yet measured.

The same pre-follow-up release binary also fails with **two workers** at the
7 GB ceiling after **457.566 seconds**, `Result=oom-kill`, zero swap, driver
exit 1. Stop/reset exit 0. The [two-worker receipt](memory-release-budget2.json)
retains 57 partial cgroup samples: the largest sampled current charge is mostly
anonymous memory and the audit process dominates observed RSS, with no Clang
process active in that particular sample. This is not an allocator attribution
or an independent aggregate peak. The exact baseline binary is retained at
`private/audit-ir-working-set/before-prepass-borrow/quirin-audit`.

The follow-up now computes round closure tokens using ordered references to
`read.values()` followed by the declaration unit. That borrow ends before the
read map can grow; no owned copy of all previous unit facts spans the next
reads. All selection, exclusion, fixed-point and refusal code is unchanged.
Its fresh standing gate and measurement are recorded separately below.

The [follow-up standing gate](prepass-gate/gate.json) exits **0** with all
commands passing: **537 units / 550 summaries, 8,498 Rust passed, 0 failed,
15 ignored; 362 Lean audits; 374 Python tests**. The pre-pass focused controls
pass 5 tests (already included in that full count). Format, Clippy, the correct
census feature, submodules, Clang and both reevaluation examples also exit 0.
All 1,498 tracked Rust sources remain unchanged throughout. This is a new gate
for the follow-up; the original gate and failed probes remain frozen above.

The borrowed-pre-pass-facts release at `e8efc940e` still fails its two-worker
run: peak **6,999,998,464 bytes**, `Result=oom-kill`, **537.829 seconds**, zero
swap; driver exit 1 and stop/reset exit 0. The worker's final events/hash receipt
is unavailable. [Exact receipt](memory-release-prepass2.json). No runtime share
or reduction factor is attributed to the removed clone from these capped,
failed runs. The unchanged-input baseline binary is retained privately.

Prediction before page-reclaim implementation: wrap every executor work item,
including pre-pass facts reads and the sequential route, in the existing unit
cache lifetime boundary. After local buffers and cache references are released,
on Linux/glibc call `malloc_trim(0)` to request returning already free allocator
pages to the OS. Its local system declaration is `int malloc_trim(size_t)` in
`/usr/include/malloc.h`. It cannot release live allocations and supplies no
analysis evidence. Other platforms retain cache release without a native trim
claim. This is an attempted reclamation, not a guaranteed memory bound or a
replacement for reducing live facts. Preserve owned results, ordered errors,
unwinding and all verdict/source checks; measure the identical full workload.

The cache/page-reclaim candidate passes ordered controls 12 and current CLI 17,
and builds in release, but still fails the two-worker full audit at **861.567
seconds**, peak **6,999,998,464 bytes**, `Result=oom-kill`, zero swap. Stop/reset
exit 0. It completes the 1,419-unit pre-pass (1,038 read, 36 unread) before the
later audit fails. [Receipt](memory-release-reclaim2.json) identifies the exact
candidate snapshot/binary; a full gate for that candidate was still pending.
No completed budget acceptance is claimed. The candidate source file is retained
under its private build directory alongside the full tracked-Rust hashes.

Prediction before phase-admission implementation: before parallel dispatch,
release idle caches/pages and read the tightest available cgroup-v2 headroom
snapshot from the process's group and ancestors. Cap that phase's workers by a
3 GiB per-worker planning reserve, always permitting one for progress; keep the
user/config/census maximum and selected inventory. Retained pre-pass facts can
therefore reduce later analysis concurrency instead of overlapping it with two
new unit states. This remains an estimate, not a guaranteed memory bound.
Unavailable observations add no new cap and are identified as unavailable;
no task is omitted or accepted on that basis. Record bounded coordinator-phase
planning observations in invocation provenance, outside verdict identities.
Preserve ordered consumption/errors/panics and malformed-dispatch validation.

## Gated phase-admission implementation

The [final source gate](final-gate/gate.json) exits **0**: **537 units / 550
summaries, 8,500 Rust passed, 0 failed, 15 ignored; 362 Lean audits; 374 Python
tests**. Every standing command, format/Clippy and the correct census feature
passes. Focused parallel controls pass 14 tests and current CLI 17. The entire
1,498-file Rust snapshot is unchanged and matches the candidate's build snapshot.
The gate overlaps the separately limited memory run; its timing is disclosed as
shared-host timing. The source rules and Lean semantic definitions are unchanged.

Every executor item now ends optional Clang reuse; on Linux/glibc it requests
free allocator pages back after release, including unwinding. Active results
and leases remain owned. Before parallel dispatch, another reclamation precedes
the tightest finite cgroup/ancestor headroom snapshot. A 3 GiB worker estimate
can reduce that phase to one, without omitting selected tasks. Dispatch is
validated before this reduction. Configured/census maxima and semantic report
order remain unchanged. Current/configured invocation provenance carries a
bounded phase trace with observations/unavailability and an omission count;
serial requests are not included. An unavailable observation is not a memory
claim. Oversize single units can still fail: this is not enforced admission.
The run's final budget outcome is recorded separately when it completes.
