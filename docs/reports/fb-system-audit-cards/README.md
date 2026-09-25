# Fluent Bit system audit investigation cards

Dispatch date: 2026-09-26. Quirin base: `9a38004529180498c3d28532c2feb718ca198e7a`.
This branch is isolated from ongoing analyzer implementation. The user requested
three cards and resumable records; Sol 6 medium is the investigation default,
with Astra medium authorized for difficult issues.

## Candidate queue and this dispatch

| ID | Question | State | Dependency / acceptance |
|---|---|---|---|
| FB-AUDIT-01-COVERAGE | Does the corpus include engine/backlog/retry code under the source/build being discussed? | investigation executed | Bind source revisions, compile records and archived unit/function denominators; name absences separately from Unknown. |
| FB-AUDIT-02-RETRY-LIFETIME | What protects tasks/chunks across retry failure, completion and shutdown? | investigation executed | Source-bound transitions, guards, hypotheses and concrete reproduction steps. |
| FB-AUDIT-03-BACKPRESSURE-ACCOUNTING | Do fan-out accounting and error paths preserve capacity and progress? | investigation executed | Separate physical/per-output ledgers; inspect a failing backlog loop; record guards and next tests. |
| FB-AUDIT-04-FORCED-INTERLEAVINGS | Can a selected lifetime hypothesis be reproduced with controlled schedules? | queued | Requires card 02's exact source slice and native build binding. |
| FB-AUDIT-05-RECOVERY-ENVELOPE | What load/outage envelope permits backlog drainage? | queued | Requires card 03; explicitly declare arrival, service and drop policies. |
| FB-AUDIT-06-DURABILITY | What are crash/replay loss and duplication guarantees? | queued | Separate persist/ack/delete specification and fault model. |

These are investigation cards, not renamed completions of the semantic
FB-BACKLOG-CUSTODY, FB-BACKLOG-ACCOUNTING or FB-RETRY-LIFETIME cards in
[the prospective system queue](../../plans/fluent-bit-system-coverage/README.md).
Source review and reproducible measurement do not discharge a Rust/Lean semantic
implementation card. No analyzer change, corpus promotion or upstream defect
claim is authorized by a hypothesis alone. Expected verdict movement: zero.

## Measured outcomes

- [Coverage](census/README.md): all eight selected files have historical build
  records (15 preprocessing runs, all exit 0), but only one is in the 185-unit
  generic audit. Current source and historical build revisions remain separate.
- [Retry lifetime](retry/README.md): transition inventory and four candidate
  findings. An exact-function, stubbed native experiment observes timer retention
  on request-allocation failure; full-production reproduction remains open.
- [Backpressure/accounting](accounting/README.md): separate resource ledgers,
  admission inequality, conditional no-progress and partial-fan-out candidates.
  These are source-level findings, not reproduced production defects.

The user-authorized Astra medium escalation attempted a callback experiment,
but the tool rejected that turn with a cybersecurity safety flag before returning
execution evidence. This is recorded in accounting/experiment-attempt.json;
no success, timeout or native result is inferred from that attempt.

## Validation

The combined standing gate is running; `validation.json` and `VALIDATION.md`
will record every exit, count, failure and private log path. The three
investigations change no analyzer semantics, contracts or published verdicts.
There is no Rust/Lean implementation-card closure or new safety proof.

## Resume

Read `dispatch.json`, each card's README and RESUME, and `validation.json` when
present. Recheck branch/base and source hashes before continuing. Logs live in
this worktree's `private/fb-system-audit-cards/`; published evidence is in this
report. Never borrow a build identity from another source revision. Main is
being advanced independently; integrate this branch deliberately, without
resetting another agent's work.
