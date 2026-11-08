# Gate time reduction: requirements

Status: requirements recorded 2026-11-05 by the integrator (record
GATE-TIME-REQUIREMENTS); no card has started. User request (verbatim,
2026-11-05): "We need to record the requirements of saving gating time tasks."
The source was a migration plan the user brought from another session; this
page keeps its requirements after checking each against `integrate/r3`
(`96a3efd3b`), and corrects the claims the tree no longer supports (§4).

Open work is tracked as the acceptance matrix's `named_followups`; each
requirement below names the entry that tracks it.

## 1. Measured cost (2026-11-05)

| What | Wall time | Source |
|---|---:|---|
| Full gate on `96a3efd3b` (landing) | 49 min: Rust suite 43 min 21 s (including the `rust-suite` slot wait), fmt 6 s, Clippy 53 s, probes 1 s, Lean 23 s, Python 67 s, providers 3 min 18 s | landing-r3 `private/gate-full-96a3efd3b/summary.txt` |
| Declared measurement, CFL-MPACK r2 (3 profiles, tip side) | 3 h 58 min | `private/measure/cfl-mpack-r2-20261105T023115Z`, `run.wall_time_ms` 14,284,164 |
| Declared measurement, LIBC-GAP (3 profiles, tip side) | 6 h 23 min | `private/measure/libc-gap-20261105T034105Z`, `run.wall_time_ms` 22,981,153 |

The two measurements ran concurrently with each other and with gates, so their
wall times include slot waits; the `--timeout-ms 36000000` quota counts those
waits too (R7).

Scoper probes (`python3 scripts/gate_scope.py --paths <p> --plan` at
`96a3efd3b`):

| Single changed path | Mode | Reason printed |
|---|---|---|
| `crates/qcore-c-gateway/tests/libc_gap_declarations.rs` (a new test file) | full | the diff reaches qcore-c-gateway |
| `providers/quirin-provider-libc/contracts/libc/stdlib.qv` (one contract) | full | the diff reaches qcore-c-gateway, qcore-contract (string-literal readers of `providers/`) |
| `proof/QCore/WrittenSetDerivation.lean` (one Lean module) | full | the diff reaches qcore-core |
| `compiler/tests/contract_loader.rs` | scoped | qcore-compiler, qcorec |

So a contract-only card or a Lean-only card runs the same 22-package Rust
suite as an analyzer change, and every landing waits hours for measurement.

## 2. Invariants every requirement keeps

- Fail-closed and the verdict contract (AGENTS.md hard invariants 1 and 2):
  no gate or measurement shortcut may publish a verdict a full run would not.
- A narrowed selection must be shown to miss no failing test: replay historical
  diffs and compare against the full selection before the narrowing is the
  default.
- Parity before deletion (invariant 3): a route moved out of the host keeps
  its pinned verdicts, with the diff archived under `docs/reports/`.
- Pinned expectations stay intentional (invariant 5): per-file
  `contract-inventory.lock` pins and the libevent submodule pin are unchanged
  in force; only where aggregate counts are read from changes.
- The landing (integrator) gate stays the full arbiter on the merged commit
  whenever analyzer, gateway or contract semantics change.

## 3. Requirements

**R1. Exact run-time readers.** The scoper attributes a run-time read to the
files the reader actually reads, not to every file under a named directory.
Directory walkers are declared, each with a test that the declaration matches
what the binary walks. Tracked by GATE-SCOPE-RUNTIME-READS-HEURISTIC.
Acceptance: the `.qv` probe in §1 no longer selects qcore-c-gateway or
qcore-contract on a string literal; the replay of R9 shows no missed failure.

**R2. Test-only edits stay scoped.** A diff confined to
`crates/<crate>/tests/<binary>.rs` (and its fixtures) selects that crate's
binary and its readers, not full mode, even for a crate in `FULL_CRATES`.
Tracked by GATE-SCOPE-FULL-CRATES-TEST-ONLY. Acceptance: the first probe in §1
is scoped.

**R3. Aggregate contract counts read from the locks.** Global counts that
every contract card edits (`compiler/tests/contract_loader.rs`:
`(66, 577, 370)`, retaining 113, pending review 11, relational 10, corpus
modules 147; `source_assumptions().len() == 66` in contract_loader.rs,
provider_selection.rs, neighbours.rs and prod_contracts.rs) are derived from
the providers' `contract-inventory.lock` files and the load-path manifest, so a
contract card touches only its provider crate and lock. The per-file lock
check in `qcore_contract_corpus.rs` stays. Tracked by
CONTRACT-COUNT-PINS-OUTSIDE-LOCKS. Acceptance: a declaration added to one
provider changes no file outside `providers/`; merging two contract cards no
longer conflicts on these literals (37 commits touched contract_loader.rs
since 2026-10-20).

**R4. Cross-tree includes leave the gateway's tests.** Lean-correspondence
and provider-text checks that `crates/qcore-c-gateway/tests/*.rs` read with
`include_str!` from `proof/` and `providers/` move to a leaf test target, so a
Lean or `.qv` edit does not select the gateway. Tracked by
GATEWAY-TESTS-CROSS-TREE-INCLUDES. Acceptance: the Lean probe in §1 selects
the Lean step and the leaf target, not full mode, unless a Rust reader of that
module is found.

**R5. Heavy corpus parity and mutation suites run only on their seam.** The
route-parity binaries (`unsupported_close_route_parity`,
`file_close_route_parity`, `filesystem_chunk_close_route_parity`) and the
composition-mutation binaries in qcore-verification are selected when a route
or composition seam changes, not for every change to their crate. Tracked by
W44-TEST-CRITICAL-PATH-ROUTE-PARITY (amended).

**R6. Card-gate stale records and provider lock.** A card adding a Lean audit
can pass its own gate (CARD-GATE-NEW-AUDIT-MATRIX-COUNT); a root crate change
that the provider workspace builds selects the provider lock check
(GATE-SCOPE-PROVIDER-LOCK). Both already tracked.

**R7. Measurement quota excludes slot waits.** `quirin-audit measure`'s
wall-time quota counts analysis time, not time spent waiting for a gate slot,
so a measurement queued behind a gate is not killed for waiting. Tracked by
MEASURE-SLOT-WAIT-OUTSIDE-QUOTA.

**R8. Incremental declared measurement.** A measurement re-audits only the
translation units whose source, contract or `.qo` input digests changed
against the published baseline, and joins the baseline's unchanged components
(`⊕`), with a check that the composed report equals a full run on a sample of
landings before it replaces the full run. Tracked by
MEASURE-INCREMENTAL-BASELINE-COMPOSITION; related W45-CHECKPOINT-PINNED-REPLAY-ROUTE,
W45-CHECKPOINT-GROUPS-IN-FLIGHT, CORPUS-DISPATCH-BY-SOURCE-SIZE.
Acceptance: a contract-only card's measurement finishes in a fraction of the
§1 times, with identical comparison output.

**R9. Selection replay evidence.** Before any of R1-R5 becomes the default, the
narrowed scoper is replayed over the diffs of recent landed cards (at least
LIBC-GAP-DECLARATIONS, CELL-PROTOCOL-RELEASE-WINDOW-FRAMES,
CFL-MPACK-EXTERNAL-DECLARATIONS, W45-AUDIT-SUBCOMMAND-PLUGINS) and the
selected test units are compared with the full selection's failing set at each
card's red commits. Discharges GATE-SCOPED-RUN-UNEXERCISED together with a
completed scoped landing.

**R10. Corpus routes behind the subcommand plugin boundary (longer term).**
The corpus-specific routes (`pinned_route.rs`, `current_route.rs`) and
source-analysis adapters move out of the `qcore-audit` host into an
out-of-process `quirin-audit` subcommand plugin, so host and gateway changes
stop rebuilding corpus code and corpus changes stop selecting the host.
Prerequisites: a plugin `Safe`/`Unsafe` claim is admitted only through
trusted-core certificate or witness replay (W45-SUBCOMMAND-CERTIFICATE-ROUTE),
host verdicts are read only from a report bound to a verified audit identity
(W45-SUBCOMMAND-HOST-REPORT-TRUST), with Lean parity for the admission rule;
then parity before deletion. Tracked by CORPUS-ROUTES-OUT-OF-HOST.

**R11. One audit cannot starve the host.** Added 2026-11-06 (user, verbatim,
on the integrator's question whether to register the gap: "Register it
(rec.)"). Observed on 2026-11-06: a `quirin-audit current` experiment
(CURRENT-RUN-WORKER-SPIN) spun 10 h 18 min in `audit/3` with zero I/O; a
card's `rust-suite` job had closed `audit/admission` to drain the audit slots
and waited 10.1 h for that one slot, so every queued audit (a declared
measurement, two checkpoint runs, a census probe) waited behind it. Holders
were found only by matching `/tmp/quirin-gate/*` inodes against
`/proc/locks`. Requirements: an audit slot job carries a stated wall-clock or
progress deadline and is reported (never silently killed) when it stops
making progress; a `rust-suite` drain has a bounded wait after which it
reopens admission and reports the blocking holder; `gate_slot.py` can list
live holders, waiters, their wait time and their CPU/I/O progress; peak RSS
above the admission budget is reported while running, not only at exit.
Tracked by GATE-SLOT-ADMISSION-STARVATION.

**R12. Slots are served in arrival order; a queue does not kill a measurement.**
Added 2026-11-08 (integrator's record of the 2026-11-07/08 incident). The audit
slots were not first come, first served: a measurement's audit
(SCALAR-STATEMENT-FORMS, `private/measure/scalar-forms-20261107T072754Z`)
waited 7.5 h, then 15.5 h, while audits and `rust-suite` drains that asked
hours later were admitted first; the measurement then died with
`plugin-timeout: exceeded the wall-time quota of 36000000 ms (36000000 ms of
declared slot waits excluded, limit 36000000 ms)`, because R7's exclusion was
capped at the quota. Requirements: a waiter that arrived earlier is admitted
before a later one of the same class, a `rust-suite` drain respecting the
audits queued ahead of it, with crashed waiters, restarts and mixed memory
budgets handled and no daemon; a measurement's slot wait is excluded from its
analysis quota without being capped at it, under a separate stated overall wall
bound with a much larger default and an override, both recorded; `status` shows
each waiter's queue position. Tracked by GATE-SLOT-FIFO-ORDER and
MEASURE-SLOT-WAIT-EXCLUSION-CAP (both discharged by card GATE-SLOT-FIFO-ORDER,
[report](../../reports/gate-slot-fifo-order/README.md)).

Suggested order: R1, R2, R3 and R7 first (scoper and test changes, no
semantics); R9 alongside; R4 and R5 next; R8 after them; R10 last.

## 4. Corrections to the source plan

- `--reader-granularity file` is already the default (`scripts/gate_scope.py`,
  `default="file"`); the remaining defect is the string-literal attribution of
  directories (R1).
- The full gate measured 49 min at this landing, not 3-6 hours; the hours are
  in declared measurement (§1), which is why R7 and R8 are requirements.
- The aggregate pins are `(66, 577, 370)` and the other literals listed in R3,
  not "69 modules".
- Target wall times ("1-4 minutes", "5-15 minutes") are not requirements:
  each card states its predicted saving and the integrator measures it.
