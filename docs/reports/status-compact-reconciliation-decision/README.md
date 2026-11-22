# STATUS-COMPACT reconciliation claims: user decision

**Decision (user, 2026-11-22):** "ok, accept the reconciliation failures."

[STATUS-COMPACT](../status-compact/README.md) compacted `docs/STATUS.md` and moved
its history verbatim to the status checkpoint
(`docs/development/increments/wave-48-compaction-status-part*.md`). Of 27 frozen
reconciliation verify claims on `docs/STATUS.md`, 16 failed before the rewrite;
25 fail after it. The nine that changed:

- six non-vacuous claims whose text now lives only in the checkpoint;
- the three vacuous `281` claims already tracked by `W45-RECONCILIATION-SUBSTRING-CLAIMS`.

The user accepts these failures as they stand.

- **The frozen records are not rewritten.** The integration policy lets each
  close check its own record against its own tree.
- **No gate or CI step runs these claims.** They run only when a close checks
  its own record by hand.

This decision retires `STATUS-COMPACT-RECONCILIATION-CLAIMS`. It leaves no
residual, because every claimed text is preserved verbatim in the checkpoint.

`W45-RECONCILIATION-SUBSTRING-CLAIMS` stays registered. It tracks a separate
defect: the sweep's substring matching. That defect predates this compaction,
and this decision does not discharge it.

No code, verdict, pin or baseline changes.
