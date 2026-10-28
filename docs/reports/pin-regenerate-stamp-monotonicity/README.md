# PIN-REGENERATE-STAMP-MONOTONICITY

Card of pilot round 1 (2026-10-28), base `80c8aea98`. It discharges the named
follow-up `PIN-REGENERATE-STAMP-MONOTONICITY`, which card LANDING-REPAIR
registered from code reading ([its report](../landing-repair/README.md#finding-left-open)).

## Defect

Contract spec [C06.6](../../QCORE_CONTRACT_SPEC_r0_3.md#c066-contract-supersession-across-bundles)
requires that a re-pin carry a later stamp than the entry it replaces, and that
an explicit stamp that is not later be refused. `quirin-audit pin` applied this
through `Stamper::replacing(function, old)`. Two paths passed it no `old`:

- `--regenerate` started from no lock (`existing = None`; only tip entries
  were carried), so every regenerated entry was stamped as new.
- On a revision-mode change (flexible to pinned, pinned to flexible, or tip to
  pinned under `--untrack-tip`), only the lock entry of the new kind was looked
  up. The replaced entry, which is of the other kind, was not.

On these paths an explicit `--pinned-at` earlier than (or equal to) the
replaced stamp was accepted, and a default clock stamp behind the lock was
written unchanged. Either can drop a declaration's stamp below another
bundle's and so change which declaration is in force.

The defect was reproduced before the fix by three failing tests in
`crates/qcore-audit/tests/pin_regenerate_stamps.rs` (commit `7eefaf63c`;
`0 passed; 3 failed`):

- `regeneration_refuses_an_explicit_stamp_not_later_than_the_one_it_replaces`:
  `--regenerate --pinned-at` with an earlier stamp exited 0 and rewrote the
  lock.
- `regeneration_moves_a_default_stamp_past_a_lock_ahead_of_the_clock`: a lock
  stamped `2999-01-01T00:00:00Z` was regenerated with that day's clock stamp.
- `a_mode_change_replaces_the_entry_of_the_other_kind`: making `item_new`
  pinned accepted an earlier explicit stamp, under an update and under a
  regeneration.

## Fix

The fix is in `crates/qcore-audit/src/contract_pin.rs`.

- `replaced_stamp(replaced, function)` returns the latest valid stamp of any
  entry of the function (pinned, flexible or tip) in the lock that was on disk
  before the run. Each entry that `update` writes anew now passes that stamp to
  `Stamper::replacing`. An explicit stamp that is not later refuses with ``a
  re-pin must carry a later stamp than the entry it replaces``. A default stamp
  moves to one second after the replaced stamp.
- `update` takes the replaced lock as a separate argument. For an update it is
  the existing lock. For `--regenerate` it is the lock being replaced, which is
  read once. Its tip entries are still carried as before.
- The update and regenerate loop now computes every lock before it writes any,
  so a refusal leaves all locks of the run unchanged.

The following behaviour is unchanged: identities, the replace-from-one-source
rule, the replacement notices, tip entries kept as written, unchanged entries
keeping their stamps on an update, the lock-set tie, and every other pin mode.
Every regenerated entry is still re-stamped. Its stamp is now later than the
stamp it replaces.

There is one boundary. A lock that `--regenerate` replaces because it cannot be
read supplies no stamps, so its entries are stamped as new. Audits refuse an
unreadable lock, so it was deciding no supersession.

After the fix, the three new tests pass. The pin tests that already existed
also pass: `contract_tip_mode` (including its no-stamp-moves-backwards
regeneration helper), `contract_revision_identities`, `contract_revision_audit`
and `contract_supersession`.

## Lean and specification

The Lean model `QCore.ContractSupersession` (card CONTRACT-SUPERSESSION) is a
decision over a set of stamped candidates: determinism, newest wins, ties
conflict, retention, never optimistic. It models no pin run, no regeneration
and no re-pin, so it has no counterpart theorem to add. Its consequence for
the decision, that a declaration stamped later than every other is in force,
is the existing `strictly_newest_wins`.

C06.6 now says which entry a re-pin replaces: the function's entry of any kind
in the lock on disk before the run, under a mode change and `--regenerate` too.
It also says that a refusal writes no lock. Its Boundaries paragraph says why
no Lean change is needed. The module documentation of `contract_pin.rs` says
the same.

## Shipped locks

No lock changes. `python3 providers/contracts.py check` exits 0. It covers all
17 pin locks via `pin --verify-contracts`, the lock-set window (1 of 3 sets,
tied to `046accf6`) and the inventory locks, and `git status` shows no lock in
the diff.

The 613 shipped entries carry five stamps, all on or before
`2026-10-27T12:11:52Z`:

| Stamp | Entries |
|---|---|
| `2026-10-25T18:10:00Z` | 445 |
| `2026-10-26T17:29:40Z` | 94 |
| `2026-10-26T17:29:50Z` | 71 |
| `2026-10-26T17:29:54Z` | 2 |
| `2026-10-27T12:11:52Z` | 1 |

No function has entries of two kinds in one lock. A default-clock pin or
regeneration run today therefore writes the same stamps as before the fix.
Only an explicit `--pinned-at` that is not later than an entry's stamp now
refuses. No shipped lock's supersession order changes, so this card moves no
verdict and needs no prediction or measurement.

`pin --check` against a reviewed Fluent Bit checkout was not run: `--check`
neither writes nor stamps, and this card does not change it. No checkout at
`046accf6` is prepared on this host.

## Follow-ups and interactions

- `PIN-TRACK-TIP-WRITE-ATOMICITY` (registered): `pin --track-tip` writes each
  target's lock inside its loop and checks for a named function that no
  selected crate declares only afterwards. A refused run can therefore leave
  earlier locks written. The update and regenerate loop is now
  compute-then-write. `--stamp-only` and `--drop-revision` already were.
- `SUPERSESSION-BOUNDS-OUTER-VIEWS`: the bounds and outer locks go through the
  same update and regenerate loop, so their stamps are now monotone under
  `--regenerate` as well. They are still not consulted (C06.6 Boundaries), and
  this card does not change that.
- `SUPERSESSION-OTHER-CONSUMERS` and `SUPERSESSION-INCLUSION-MODULES`: no
  interaction. Neither reads how stamps are written.

## Verification

Logs are in
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/logs/`.
The write guard of this card's worktree refused writes under
`/media/Data/GitHub/quirin-lang/private/`.

| Step | Log | Result |
|---|---|---|
| Failing tests before the fix | `red.log` | 0 passed, 3 failed |
| Pin tests after the fix | `green.log` | `pin_regenerate_stamps` 3, `contract_tip_mode` 2, `contract_revision_identities` 3, `contract_revision_audit` 4, `contract_supersession` 4; all pass |
| `providers/contracts.py check` | `contracts-check.log` | exit 0 |
| Scoped card gate (`python3 scripts/gate_scope.py --base 80c8aea98 --out private/gate`, head `bd091a916`) | `gate.json` in this directory; console `gate.log`; step logs in the worktree's `private/gate/` | exit 0, 7 steps: submodules 0, fmt 0, Clippy 0 (5 crates), Rust suite 0 (qcore-audit, qcore-compiler, qcore-verification, qcorec, quirin-source-analysis: 250 units, 3273 passed, 0 failed, 5 ignored), both reevaluation probes 0, Python suite 0 (554 tests); Lean none (no `proof/` change); provider gate not selected |
