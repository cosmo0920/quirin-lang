# QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN: prediction (frozen before implementation)

Base: local `main` at `e0b92ece6`. Frozen after reading the quota path and
before any source, test or Lean edit. Authority: the ledger entry
`QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN` (recorded by
EXECUTION-QUOTA-LOAD-INDEPENDENCE, found by reading) and hard invariant 1.

## The defect, as read at the base

`crates/qcore-verifier-host/src/lib.rs`:

* `quota_descriptor` maps `QuotaSession::procs_descriptor()` (an `fd` dup,
  `OwnedFd::try_clone`) through `.ok()`. A failed dup (`EMFILE`, `ENFILE`,
  `ENOMEM`) gives `quota_procs` None.
* `run_process` installs the join `pre_exec` only `if let Some(procs)`, so the
  child is then spawned **without** the join and runs outside its
  per-execution cgroup.
* `verify_process` calls `finish_quota(quota)` unconditionally, so the record
  carries an observation of the empty cgroup, under `EnforcedLimits` whose
  `process_quota` names the quota. A bound is claimed that was never enforced.

## Audit of the quota path (session creation to the `pre_exec` join)

Every `.ok()`, `unwrap_or*`, `let _ =` and unchecked count between
`QuotaSession::create` and the join, read at the base:

| # | Site | Can a child run outside its declared quota while the record names it? | Predicted action |
|---|---|---|---|
| 1 | `quota_descriptor`: `procs_descriptor().ok()` | **yes** (the defect) | refuse before spawning: `RefusedByProfile`, `enforced` None, `quota_observed` None, precise reason naming the cgroup and the errno |
| 2 | `verify_process`: `finish_quota(quota)` for every execution that created a session, including ones that never spawned (identity refusal, a pre-spawn `Protocol` error in `run_process`) and ones whose spawn failed (a failed join fails the spawn) | no child runs outside the quota, but the record carries an **observation of a quota no child was proved to have joined** | record an observation only when `spawn` returned a child (every `pre_exec`, the join first, succeeded); otherwise release the cgroup and record none |
| 3 | `process_quota::linux::join`: only `written < 0` is checked; a short count is accepted | not on Linux kernfs (a `cgroup.procs` write returns the whole count or an error), so not reachable; it is still an unchecked result | treat any count other than the two bytes as a failed join (allocation-free errno), pinned by a unit test |
| 4 | `QuotaSession::create`: clock `unwrap_or_default()` | no: names only; the per-process sequence keeps names distinct, and `EEXIST` refuses | none |
| 5 | `QuotaSession::create`: `let _ = remove_dir` on its own failure path | no: cleanup after an error that is returned as a refusal | none |
| 6 | `QuotaSession::finish` / `release`: `.ok()` on counters, `let _ =` on `cgroup.kill` | no: after the run, measurement fields that are `Option` in the record | none |

No other site between the session and the join was found. `prepare_quota` and
`prepare_confinement` propagate with `?` and already refuse before spawning.

## Test seam

`RLIMIT_NOFILE` cannot isolate the dup: `create` opens `pids.max` and
`cgroup.procs` while `pids.max` is still open, then closes `pids.max`, so the
dup takes the freed lower slot. Any limit that lets `create` succeed lets the
dup succeed. The test therefore injects the failure through the existing
one-directional override: `MechanismAvailability` gains a fourth field
(`quota_join_descriptor`), whose `ForcedUnavailable` makes the dup report
`EMFILE` at the real call site, after the cgroup exists, with the override
named in the detail. Like the other three, it can only add a refusal.

## Tests predicted

* `execution_quota` +3: the forced dup failure refuses before spawning (no
  marker, `RefusedByProfile`, `enforced` None, `quota_observed` None, no
  identity window, registry `Unknown`/`QP006` with no certificate, and the same
  fixture without the override runs and is observed); an identity refusal after
  the cgroup exists records no observation; a failed spawn records no
  observation. 13 → 16.
* `qcore-verifier-host` unit tests +1 (the join's count check). 31 → 32.
* Every other row unchanged. The suite total moves by +4 over main's.

## Lean

`QCore.ProcessQuotaPlan` gains a model of the execution steps (session,
descriptor, join, exec) and its record, with theorems: a failed descriptor
refuses before spawning with no observation; an observation implies the join;
a spawned record naming a quota implies the join; a failed join never spawns;
and the legacy step function recorded an unjoined observation (the defect is
real in the model). Registered in `AuditProcessQuotaPlan.lean`; the audit
count stays **316** (theorems are added to an existing audit).

## Measurement

The changed code runs only when a process provider is spawned under a quota.
No arbiter route spawns one, so:

* corpus 935 / 9 / 3,890 (`bdf5f1c8…`), system 93 / 4 / 351, slice
  52 / 1 / 347 / 4 Pending: **byte-identical** BEFORE (base binary) against
  AFTER (tip binary), each measured in this worktree;
* the null property (`QCORE_NULL_PROPERTY`) on each route: byte-identical rows.

No verdict moves, so no correction is predicted. Any movement would falsify
this prediction and be attributed function by function.

## Records

Constraint 26 amended in place (no new rule number). Increment paragraph in
`wave-48-part7.md` (opened here if absent). Ledger 763 → 762: the entry retires
if the forced test and the refusal hold.
