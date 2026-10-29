# Pilot round 1 landing gate

The merged source is `686d547c7` on `integrate/pilots-r1`, integrating onto
local main `3ee2c5ac1` (the event-holder-identity merge):

- PIN-REGENERATE-STAMP-MONOTONICITY at `68a6bbf3a` ([report](../../pin-regenerate-stamp-monotonicity/README.md));
- RULE35-PILOT at `07f97c251` ([report](../../rule35-pilot/README.md));
- RULE37-PILOT at `0be85e712` ([report](../../rule37-pilot/README.md)).

Integration changes beyond the three merges:

- Both rule packages kept in the shared manifests, layering pins, the hosted
  package list and the architecture row; `qcore-schemas` stays at main's 24.0.0.
- RULE35-PILOT's generator requires an arm route to be a `pub fn` of its
  package. Rule 37 gains `sizeof_operand::decide` and
  `asserted_allocation::decide`, thin wrappers over its existing arms, and its
  registry routes name them. The gateway's call sites and every decision are
  unchanged.
- RULE37-PILOT's retirement of `MEASURE-TOOL-RULES-OFF-LIVE` is dropped: RULE35-PILOT,
  applied first, retires it with its own discharge record.

The amendments were applied in the order PIN, RULE35, RULE37; `profiles: []`
for all three, so no live baseline moves (both pilots are identity runs with
switch-measured contribution ledgers).

`gate_scope.py --base 3ee2c5ac1 --full` on `686d547c7` (full mode: manifests,
gateway and a proof root changed): submodules, fmt, Clippy exit 0; Rust
578 units, 8,873 passed / 0 failed / 15 ignored (34 filtered); both
reevaluation probes exit 0; Lean 392 audits exit 0; Python 555 tests, 1
failure (`test_the_derived_facts_are_recomputed_rather_than_carried`: the
committed matrix's schema version and Lean audit count were stale). The matrix,
report index and contributions page were regenerated (`land.py check` fresh)
and committed with these records before the Python rerun. The provider gate was
not selected (no `providers/` change). [Steps, exits and log paths](gate-steps.json).
Private logs are under `private/landing-pilots-r1/`.

The first landing gate exhausted disk space in its Rust suite (ENOSPC) and is
not validation; its logs remain in `private/landing-pilots-r1/gate-1-enospc`.
The build directories of the three merged cards were reclaimed before this
gate. Final publication is `land.py publish` on the committed landing records;
no outcome of it is inferred here.

## Halted cards and tip validation (2026-10-29)

On the user's request the running cards RULE36-PILOT, RULE33-PILOT and
RULE34-PILOT were halted and every card's commits merged onto main. Each
halted card's uncommitted draft report was committed as found (WIP). They
landed **without** their own identity measurement, contribution ledger,
card gate or integration amendments; their reports are drafts and
`GATEWAY-RULE-API-PILOT-MIGRATIONS` is not amended for them. Rule 33 gains
`selection_route::{expand,row}`, thin wrappers over its arms, so its
registry routes satisfy RULE35's generator check. Main had meanwhile gained
`41edebfc4` (this branch) and `0d15e7b19` (codex/event-collector-selection),
merged here as `442dfa6f0`.

`gate_scope.py --base 80c8aea98 --full` on `442dfa6f0`: submodules, fmt,
Clippy exit 0; Rust 584 units, 8,927 passed / 0 failed / 15 ignored (69
filtered); Lean 396 audits exit 0; Python 556 tests, 1 failure (the stale
matrix derived counts, regenerated below); provider fmt exit 0; provider
build, verify, test and Clippy failed because `providers/Cargo.lock` lacked
the five rule packages ([steps](tip-gate-steps.json)). `dc5d07c95` adds
exactly those five lock entries; the provider steps rerun on it all exit 0:
five packages built and verified, provider Rust 133 passed / 0 failed / 1
ignored (43 binaries), fmt, Clippy ([steps](tip-provider-steps.json)). Logs:
`private/landing-pilots-r1/gate-tip-442dfa6f0/` and
`private/landing-pilots-r1/provider-dc5d07c95/`. The matrix, report index and
contributions page were regenerated (`land.py check` fresh) and committed
with this record before the Python rerun.
