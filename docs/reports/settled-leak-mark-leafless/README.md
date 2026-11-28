# SETTLED-LEAK-MARK-LEAFLESS: a leafless mark is decided from settled states, with an `Unknown` reason when unwitnessed

Card SETTLED-LEAK-MARK-LEAFLESS, user decision of 2026-11-28: option (b).
Base `287dce8ed` (integrate/r86). Commit order:

1. probe `8ba33c206` (fixtures pinning the base's schedule-read verdicts;
   analysis unchanged);
2. [PREDICTION.md](PREDICTION.md) and the measurement scripts `442cfa003`;
3. implementation `2e949e11a` (the measured tip; later commits change no
   gateway source);
4. `468591c12`: a correction of how the suite's nine leafless marks were
   characterised (rule text, module comment, the no-reason pin; no semantic
   change); `fe807ee90`: the census summary lists the candidates.

Gateway rule [84](../../agents/gateway.md#constraint-84) is amended in place
(no new id; 93 is not taken). Rules 74c and 6 gain amendment lines. New Lean
module `QCore.SettledLeakLeafless` with `proof/AuditSettledLeakLeafless.lean`.

## 1. Probe at base

Rule 84 decides a marked leak report beside an `Unknown` or `Pending` leaf.
Beside neither, the mark is no refusal's: rule 43's `ElementsReleased` clause
(`forget_published`) withdraws every object the caller published. Rule 74c's
held report decided such an identity, so it read the schedule.

The probe needed a *claimable* leafless mark, which the suite did not have:
`forget_published` marks published objects, which are `ESCAPED`. A caller
that publishes `x` on one arm and leaves it open on the other has `x` both
`LIVE` and marked after the drain. Fixtures in
`crates/qcore-c-gateway/tests/cross_unit_elements_released/mark_leafless.rs`
(a module of an existing test target; the imported `drain` of that file):

| shape | order | base `287dce8ed` | tip |
|---|---|---|---|
| `ONE_ARM`: publish+drain on one arm, other arm open | open arm first | Unsafe, 1 leak, `EXIT_WITHHELD` | **Unsafe, 1 leak, `EXIT_WITHHELD`** |
| same | drained arm first | Unknown, 0, `EXIT_WITHHELD` | **Unsafe, 1 leak, `EXIT_WITHHELD`** |
| `RETAINED`: the open arm through a retained loop exit (`while (!k)`, `k = 0`) | open arm first | Unsafe, 1 leak, retained-edge refusal | **Unknown, 0, reason + retained-edge refusal** |
| same | drained arm first | Unknown, 0, retained-edge refusal | **Unknown, 0, reason + retained-edge refusal** |
| `JOINED`: one-arm publication, drain after the join | unpublished arm first | Unsafe, 1 leak (no mark) | Unsafe, 1 leak |
| same | published arm first | Unknown, 0, `EXIT_WITHHELD` | **Unsafe, 1 leak, `EXIT_WITHHELD`** |

The retained exit edge is `unmodeled` with no diagnostic until the
settled-state report after the settle point (rule 5a), so that function is
leafless at the settle point and no witness exists. In `JOINED` the
published-first program's mark is a stale iterate's: the published set joins
by intersection and the mark by union. The settled states of the two programs
differ (registered as FORGET-PUBLISHED-STALE-MARK, §5); the open arm is
witnessed, so the tip publishes it.

## 2. The change

`crates/qcore-c-gateway/src/legacy_mark_leafless.rs` (new, `mark_leafless_route`):

* **Candidates** (`candidates`): when the diagnostics at the settle point carry
  no blocking leaf (`blocking_leaf`), the marked current-generation
  `(exit, identity)` pairs (`marked_exits`) whose identity the exit's settled
  output would claim but for the mark (`claimable`: `LIVE`, not exempt, no
  reference family). `leafless(blocking, marked, claimable)`.
* **Decision** (`outcome(witnessed) = (witnessed, !witnessed)`): rule 74c's
  held reports leave the candidates alone; rule 84's `settle` publishes each
  witnessed one (its replay check included); every other candidate records the
  `Unknown` reason at its exit, unless a leak with the same key is published
  there:

  > the identity acquired at line N may be live at this exit under a modelled
  > effect's mark, and no refusal explains the mark (mark-without-refusal)

  The reason is a gateway `Unsupported` leaf: a category, not a `Q` code, so
  the `Q`-space icebox is not engaged and no code is allocated.
* **Not candidates**: an exempt or non-live marked identity (no leak in any
  schedule; still rule 74c's), and anything beside a blocking leaf (rule 84
  as amended by SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF).
* **Moves possible**: `Unsafe -> Unknown`, `Unknown -> Unsafe`, and
  `Safe -> Unknown` only where the reason fires on a row with no other leaf.
  Never `-> Safe`.

**Hooks in shared files** (each commented with the card name):

* `legacy_transfer.rs`, the settle block after
  `mark_without_unknown_route::record`: `let leafless =
  mark_leafless_route::candidates(..)`; `held_leaks.settle` receives
  `decided ∪ leafless`; `mark_leafless_route::settle(..)` after
  `joined_leak_route::settle`.
* `legacy_c.rs`: one `mod mark_leafless_route` line after
  `mark_without_unknown_route`.
* `legacy_mark_without_unknown.rs`: two doc-comment lines.
* `tests/cross_unit_elements_released.rs`: the trailing `#[path] mod
  mark_leafless;` (no new test target).

**Lean** (`QCore.SettledLeakLeafless`): `leafless`, `outcome`, `decideAt`
(rule 84 beside a blocking leaf, this card for a claimable leafless mark, rule
74c otherwise), `reasonLeaf`. Theorems: `the_reason_is_unknown`,
`the_reason_is_never_safe_or_unsafe`, `a_leafless_decision_is_never_safe`,
`a_withdrawal_in_a_leafless_row_is_unknown`, `outcome_is_order_independent`
(blocking leaf, claimable, or unmarked exit), `the_leafless_case_read_the_schedule`,
`agrees_with_rule_84_beside_a_blocking_leaf`, `a_witnessed_mark_is_published`,
`an_unclaimable_mark_is_left_to_rule_74c`, `both_orders_publish_the_open_arm`,
`both_orders_record_the_reason`, `outcome_vectors_hold`. Axioms: none,
`propext`, `Quot.sound` at most; no `sorry`, no project axiom.
`tests/unit/legacy_mark_leafless.rs` parses `outcomeVectors` from the Lean
file and checks `OUTCOME_VECTORS`, `leafless`, `outcome` and the reason's
`Unknown` verdict against it.

## 3. Census and measurement

**Gateway suite** (`QCORE_MARK_CENSUS`, feature `indirect-census`, each from
its own `git archive` tree with the libevent submodule extracted;
[scripts/suite-census.sh](scripts/suite-census.sh)).
[Base (probe commit)](evidence/suite-census-base.json): 174 marked exit lines
in analyses with no `Unknown` leaf. Beside no leaf at all: 9 non-claimable
lines in `cross_unit_elements_released::an_imported_clause_reaches_the_caller_and_both_bodies_replay`
and 20 claimable lines, all the probe fixtures'. [Tip](evidence/suite-census-tip.json):
the same 174 by class (the census runs before the decision). No other
claimable leafless mark exists in the suite.

**Correction.** The previous card recorded the 9 lines as rule 43 marks "on
an exempt entry-custody site". They are site 0, `x = q_alloc()` in the
published-then-drained body: `x` is published (`ESCAPED`, never `LIVE`) when
the drain marks it, so it is not claimable for that reason. Pinned by
`a_published_mark_records_no_reason` (`Safe`, no reason).

Both suite census runs exit 101 only for the two `qcore-c-gateway` lib tests
that read `git ls-files` (`arm_obligations::...no_host_reads_an_admission...`,
`census_sink_inventory::every_declared_sink_forces_one_job`), which cannot run
in a `git archive` tree: 4,463 / 4,466 passed, 2 failed each. Both pass in the
gate's worktree run.

**Fluent Bit 046accf6**, prepared inputs `private/fb-046accf6` (read-only),
each run from its own `git archive` tree and its own release `quirin-audit`
built with `indirect-census`, through `scripts/gate_slot.py audit`:

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | census lines |
|---|---|---|---|---:|---|---:|
| opt-in base | `287dce8ed` | `1ff2c8e696b2b548…` | 00:08:08 / 01:12:16 | 0 | 1,209 / 2 / 3,586 / 7 | 0 |
| opt-in tip | `2e949e11a` | `aa692957425de326…` | 01:20:23 / 02:13:46 | 0 | 1,209 / 2 / 3,586 / 7 | 0 |
| system base | `287dce8ed` | as opt-in base | 01:14:11 / 01:24:45 | 0 | 124 / 0 / 320 / 4 | 0 |
| system tip | `2e949e11a` | as opt-in tip | 02:31:37 / 02:39:03 | 0 | 124 / 0 / 320 / 4 | 0 |

The opt-in route is `current --monkey-linked`
([opt-in-current.sh](scripts/opt-in-current.sh), the provider script path
updated to its archive location; a first base attempt failed on the old path
and is kept as `private/slf/run-base-attempt1`, not evidence). The system
route is [system.sh](scripts/system.sh) (fluent-bit-system: `current
--selection system-selection.json`, the eight-home link map).

**Moved rows: none** on either route ([moved-rows.json](evidence/moved-rows.json),
[changed-rows.txt](evidence/changed-rows.txt), [totals.json](evidence/totals.json) with each report's SHA-256: no row whose status or leaf
multiset differs). No census line on either route at base or tip: no Fluent
Bit function has a marked exit beside no `Unknown` leaf. No Unknown -> Safe
move needs justifying; no Safe -> Unknown move occurred.

**Against the prediction.** Every figure is at its predicted expected value,
0. The suite matched too: the 9 non-fixture lines kept their `Safe` rows with
no reason, and only the new fixtures moved.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); not run, nothing reported.

## 4. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 287dce8ed
--out <worktree>/private/gate`, head `fe807ee90`, mode full (the diff reaches
`qcore-c-gateway`); record [gate/gate.json](gate/gate.json), logs in the
worktree's `private/gate/`. A first attempt at `2e949e11a` was stopped before
its Rust suite ran, when the correction commit changed a test; it is kept as
`private/gate-attempt1`, not evidence.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 725 units, 770 summaries; 10,415 passed, 0 failed, 24 ignored, 150 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 537 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 814 tests |

Also: `cargo clippy -p qcore-c-gateway --features indirect-census
--all-targets -- -D warnings` exit 0; the LLP64 fixture scan over the changed
test files exit 0 ("0 counted, 0 found"); `apply_amendments.py --check
--profiles-base 287dce8ed` exit 0; `check_report_hygiene.py` exit 0. No
provider file changed, so the provider gate was not selected. No base failure
was seen.

## 5. Ledger and follow-ups

**SETTLED-LEAK-MARK-LEAFLESS** is retired in
[integration-amendments.json](integration-amendments.json).

Registered: **FORGET-PUBLISHED-STALE-MARK**. Rule 43's `forget_published`
reads the intersection-joined published set and leaves a union-joined mark,
and a node's first incoming state is its first edge alone. So when a
publishing arm reaches a join first, an iterate marks `x`, and the mark stays
after the join drops `x` from the published set (the `JOINED` fixture). The
verdict no longer depends on it, but the settled states and `EXIT_WITHHELD`
still do.

**Boundary kept.** A held report of an identity that is marked at the settled
exit but not claimable there (it became exempt through a later out-slot
target, or carries a reference family) is still decided by rule 74c. Neither
census found one.

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`: the rule 84 settle block
  (`leafless` candidates, the `held_leaks.settle` argument, the
  `mark_leafless_route::settle` call).
* `crates/qcore-c-gateway/src/legacy_c.rs`: the `mod mark_leafless_route`
  line after `mark_without_unknown_route`.
* `crates/qcore-c-gateway/src/legacy_mark_without_unknown.rs`: module
  comment.
* `crates/qcore-c-gateway/tests/cross_unit_elements_released.rs`: the
  trailing module line.
* `docs/agents/rules/84.md`, `74c.md`, `6.md`; generated `docs/agents/gateway.md`,
  `docs/agents/README.md`, `proof/QCore.lean` (rerun the generators on
  conflict).

## Increment paragraph

The paragraph to prepend is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json).
