# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`MUTEX-SHARE-CROSS` (2026-09-14): CT46's composition — *"Known shared wrapper
with non-Sync but Send payload"*, expected **"Accept only through verified
mutex protocol"** — landed as the **checked half**, with the native emission
half named as a residual rather than attempted. The two pieces CT46 needs
existed separately and had never met: `qcore_core::transfer` carried the
Send/Sync-style judgment and recorded its own gap as data
(`MUTEX_SHARING_GAP { requestable: false }`, with a test pinning that *no*
`(kind, mode)` pair anywhere yielded `CheckedMutex`), while
`CheckedMutexInvariantV1` was already a **multi-task** model whose child tasks
could reach a protected resource under their own acquisition. What did not
exist was a crossing that was *declared, judged and gated on the protocol*.
The card adds one, as an **opt-in clause on the mutex route's existing
child-task form**: `thread scope NAME share PLACE under MUTEX { … }`, told
apart from the plain child and from `T03-GUARD-JOIN`'s `under GUARD` by one
token at a fixed position. Constraint-14 cost is **+0 reserved words, +0
tokens and +0 name-resolution exclusions** — the cleanest outcome in that
ledger, and a consequence of the design rather than luck: both clause words
were already reserved, and both clause *names* stay genuine references, which
is measured (an unbound name refuses at `Q013` before the card's gates run)
rather than asserted. The judgment grows by its **fifth admissible row** on
the `WRITE-LEND-TASKS` precedent — kinds `ProtectedResource` (the second
Send-but-not-Sync kind: custody moves over a derivable synchronization edge,
and sharing is admitted under `CheckedMutex` in **both** directions, because a
mutex orders mutation where a counted read share removes it and a byte
partition partitions it) and `MutexShare`, mode `ShareUnderMutex`, delivered
transfer `ShareUnderMutex` — and, generalising in the refusing direction, each
sharing mode now requests **one** named protocol and is admitted exactly when
the kind's own discipline is that protocol. That generalisation is what makes
"refuses when it crosses any other way" quantified: an `own<Buffer>` refuses
`share … under …` with `CountedReadShares` named, and a `protected N` payload
refuses both lend families with `CheckedMutex` named. The card's discipline
was to **compose the unedited mutex checker rather than re-implement its rules
beside the judgment**, and the three refusals it names divide accordingly:
**absent** and **mismatched** are the crossing's own gates at `Q600` (the
`under` name is not a generation; it guards some other resource — neither a
classification question, since the judgment cannot see a declared footprint),
while **unverified** is the core's own verdict at `Q611`
`UnguardedProtectedAccess`, because a protected footprint has no unguarded
access at all. Beside them the declaration is made **load-bearing** — a
declaring child reaches the payload it named and no other declared resource,
checked at every resource-resolution site — an opt-in strengthening on the
lock-rank precedent that leaves every module without the clause
byte-identical. Zero new diagnostic codes (the `Q-NAMESPACE-DECISION` icebox
untouched; the widenings are stated in spec §L21's `Q6xx` row). Lean audits
**125 → 126** (`AuditMutexShareCross.lean`): the soundness statement is
`QCore.MutexShareCross.aSharedPayloadIsReachableOnlyThroughTheProtocol` — any
admitted access to a footprint a live generation protects, by **any** task at
**any** state, names a guard that the accessing task holds over that
footprint's own protector — lifted along an arbitrary admitted schedule by
`everyReachAlongAnAdmittedTraceHoldsTheProtocol`, so *"at every point of every
interleaving"* is the statement's own quantifier rather than a reading of it.
It is proved against `QCore.MutexInvariant`'s **existing** `step`/`run` model,
unedited by this card, which is the evidence that the composition happened.
Both transferability matrices are re-decided whole at 16 x 6 and 16 x 6 x 16,
with CT47's `theBoundaryIsDecidedExactlyOnce` and
`aWriteCaptureIsRefusedByName` **re-decided rather than weakened**. Two pins
moved deliberately and are disclosed in the report: the matrices, and
`the_mutex_protocol_is_named_and_unreachable` → its successor — the update
that test's own doc comment had asked for. Scope, stated honestly: one
wrapper, one payload, one mutex, spawn..join, no nesting; the **task** route
still has no mutex, so `MUTEX_SHARING_GAP` stays open and still reads `false`
(`MUTEX-SHARE-CROSS-TASK-BUFFER`); and a multi-task mutex trace still receives
no C at all, a **pre-existing** boundary (`mutex_c_emit::is_single_task_trace`)
this card does not move, so the crossing is checked and **not executed**
(`MUTEX-SHARE-CROSS-EMISSION`) — no native run, no byte comparison, no
data-race claim. Evidence and the CT46 readiness statement:
`docs/reports/mutex-share-cross/README.md`.

<!-- migrated-section:start -->
## Increment records

<!-- migrated-section:end -->
