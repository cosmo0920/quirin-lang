# W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL

A verdict-contract soundness repair of gateway rule 10i
([amendment](../../agents/gateway.md#constraint-10i)). Base: local `main` at
`fefb9199`. Branch `worktree-agent-a699eb30a516b6732`.
[PREDICTION.md](PREDICTION.md) was frozen before any arbiter run (its order is
disclosed there).

## 1. The defect

Two rewrites hold a call for a named human act: a declaration awaiting review
(`qcore_review_pending__`, contract spec C06.1) and a pinned declaration whose
lock the source does not match (`qcore_pin_mismatch__`, C06.2). The held
statement refuses with a `Pending` reason. Two things made a row `Pending`
below its real standing:

1. **The held refusal replaces the statement's other refusal.** The statement
   classifier checks the hold before the lexical access gate
   (`legacy_c.rs`, the `W45-CONTRACT-DEFERRED-REVIEW` hook), and a statement
   reports one refusal. `i = k_input_new((*ctx)->config, …)` reports only the
   held call, although with `k_input_new` accepted the `(*ctx)->config` actual
   keeps `indirect memory access is outside the resource projection`
   (fixture, §4).
2. **The held call has no effect.** Nothing the accepted declaration would
   make of the path is seen: a second release, a leak, an argument the
   declaration does not admit (`k_close(0)` against `resource_released(h, …)`:
   `arguments do not match resource contract`), or a body the accepted
   declaration's own row refutes (the revision-mode fixture's
   `dropped-release`, where a re-pin as written leaves `box_drop` in
   `summary_conflict` and its caller's call withdrawn).

`Pending` claims that a named human act alone stands between the row and
`Safe` (rule 10i, §L21.2). In every case above the act would leave the row
`Unknown`, so publishing `Pending` put a verdict below the function's real one
(`Pending < Unknown`): hard invariant 2.

A refusal on a **later** statement is not masked: the refused statement's
writes are havocked and the path goes on, so the held analysis records it
(fixture `a_later_refusal_on_the_path_is_recorded_by_the_held_analysis`).

## 2. Where the fix belongs, and the mechanism

A per-statement repair (keep the access refusal beside the held one) would fix
case 1 only partly: which actuals a position admits depends on the
declaration in force (a `Borrowed` position reads its actual; the classifier
must know the contract to decide), and it cannot see case 2 at all. The hold
itself must stay: it is what keeps a withheld declaration from discharging or
accusing. So the reasons are **re-derived**
(`crates/qcore-c-gateway/src/legacy_review_standing.rs`):

- `ResourceContracts::with_human_holds_in_force` (qcore-schemas): the
  environment with every pending-review and pin-mismatch declaration moved into
  the function map as written, nothing held; the refutation table is kept.
- `analyze_legacy_c_translation_unit_selected_partial` (every unit route) and
  `analyze_legacy_c_tokens` (the single-function entry) run their held
  analysis as before. If some row is `Pending`, `review_standing::settle_with`
  runs the same analysis once more under that environment, with parity rows
  muted, and joins every `Pending` row with each reason above `Pending` its
  function carries there. Each is recorded as an `Unsupported` diagnostic
  `reason standing beside a call held for a human act: with every held
  declaration in force as written, this function still carries: …`; a
  violation there is recorded the same way (`a double release candidate, not
  validated: …`), never as a violation. The row is re-folded, so it is
  `Unknown`; the unit status is re-folded.
- A row whose second analysis is `Safe` keeps `Pending`. A failed second
  analysis, a missing row, or a `Pending` row with nothing held keeps nothing
  `Pending` (an `unavailable` standing reason).
- Rows that are not `Pending` are never read; a unit with none is analysed
  exactly once. So the change can only move a `Pending` row to `Unknown`.

The held-call diagnostics stay, so the `pending_human_review` tally, the unit
reason's held-call component and its reason code are unchanged (see §6 for
the code).

## 3. The other holds (task item 3)

| Prefix | Verdict of the hold | Preemption | Disposition |
|---|---|---|---|
| `qcore_review_pending__` | `Pending` | masked a standing reason | **fixed** (§2) |
| `qcore_pin_mismatch__` | `Pending` | the same | **fixed** (§2) |
| `qcore_refuted_by_body__` | `Unknown` (`stale-evidence`) | replaces the statement's refusal | attribution only |
| `qcore_abi_withdrawn__`, `qcore_mode_withdrawn__` | `Unknown` | the same | attribution only |
| `qcore_premise_unmet__` (with `loop_owner__`, `thread_sync__`) | `Unknown` | the same | attribution only |
| `qcore_source_spelled__` (escape) | `Unknown` | the same | attribution only |

An `Unknown` hold absorbs what it preempts: the preempted reason is a
statement refusal (itself `Unknown`), and a violation the withdrawn
declaration would allege is not validated while it is withdrawn. Proved as
`an_unknown_hold_absorbs_every_refusal`; measured size of a verdict effect: 0
by construction, so nothing is registered. The Quirin-language extern-call
route (`qcore_verification::driver`, `pending_review_refusal`) emits a `Q023`
diagnostic per declaration and continues, so it masks nothing.

## 4. Fixture before and after

`crates/qcore-c-gateway/tests/review_hold_standing.rs` (held-out `k_*` family,
9 tests). BEFORE and AFTER were measured with one probe,
[`fixture_before_after.rs`](scripts/fixture_before_after.rs), run in the base
tree and in this worktree by
[`fixture_before_after.sh`](scripts/fixture_before_after.sh) (exit 0 both);
its rows are [`artifacts/fixture/before.txt`](artifacts/fixture/before.txt)
and [`after.txt`](artifacts/fixture/after.txt). BEFORE reproduces the defect
on `main`.

| Shape | Declarations in force | BEFORE (held) | AFTER (held) |
|---|---|---|---|
| `i = k_input_new((*ctx)->config, …)` (the observed shape) | `Unknown` (access refusal) | `Pending` | **`Unknown`**, standing reason names the refusal |
| the same with a formal actual | `Safe` | `Pending` | `Pending` |
| `k_close(h, 0); k_close(h, 0);` | `Unsafe` (double release) | `Pending` | **`Unknown`**, standing `double release candidate, not validated` |
| open and forget | `Unsafe` (leak) | `Pending` | **`Unknown`**, standing leak candidate |
| open, test, close | `Safe` | `Pending` | `Pending` |
| a refused store after the held call | `Unknown` | `Unknown` | `Unknown` (no standing reason: the held analysis records it) |
| an unrelated held declaration | as without it | byte-identical | byte-identical |

Each row is checked under both holds (review and pin mismatch), on the unit
route, the single-function entry and the Clang production route (unit
verdict).

**Corrections to existing pins** (each was an instance of the defect; none
is a `Safe` move):

- `contract_deferred_review.rs::a_deferred_contract_accuses_nothing` and
  `contract_pin_hold.rs::a_withheld_pin_accuses_nothing`: `TWICE` and `LEAKED`
  `Pending → Unknown` (the accepted declaration alleges the violation);
  `BALANCED` stays `Pending`; no violation is alleged in any.
- `sr5_soundness.rs::a_genuine_hold_is_still_pending`: its held call
  `k_close(0)` does not match the declaration in force, so `Pending →
  Unknown`; a formal actual is now the genuine hold. The unread-definition
  test uses the formal actual too (its gates are unchanged).
- `qcore-audit` `contract_revision_audit.rs`: the pinned mutation table's
  `dropped-release` and `changed-transfer` callers `Pending → Unknown` (the
  flexible rows of the same table are exactly the second analysis: `Unknown`
  there), `premature-destroy` stays `Pending` (flexible: `Safe`); the
  end-to-end test's `user_drop` `Pending → Unknown` with reasons
  `contract-pin-mismatch, unsupported`, and `compare`'s coverage-lost reasons
  follow. `contract_revision_identities.rs`: revision C drops the release,
  `user_drop` `Pending → Unknown`. The W45-CONTRACT-REVISION-MODE report's
  harness table is frozen evidence of its checkpoint and is not edited.

## 5. Corpus, system and slice

[`scripts/measure.sh`](scripts/measure.sh) `PHASE TREE` runs the three
arbiters (the common brief's commands, `--jobs 8`) into fresh directories
under `private/rh/`. BEFORE is the base tree `fefb9199` (an archive of the
commit unpacked into the session scratchpad, outside `qcore-c/`; release binary
`2fc22203…`), AFTER this worktree at `6f33cfa7` (binary `2eb2ef03…`; the tip
differs from it only by `rustfmt` layout, comments and records). The first
BEFORE corpus run exited 1 (`could not verify Cycle 18 Libevent source:
GitUnavailable`: the archive leaves `third_party/libevent` empty); it is kept
as a failed attempt and discarded, and the corpus was rerun (`before-corpus`)
with that directory linked to this worktree's checkout of the pinned
`d82464a2`. Comparisons: [`artifacts/compare/`](artifacts/compare/)
([`compare.py`](scripts/compare.py): statuses, leaf multisets, unit keys and
every differing JSON path; [`before_after.py`](scripts/before_after.py): raw
files), written by [`compare_all.sh`](scripts/compare_all.sh).

| Route | Baseline (condvar integration) | BEFORE | AFTER |
|---|---|---|---|
| corpus (185 units, `66910c10`) | 526 / 10 / 4,298 functions; units 6 / 8 / 167 / 4 | `resource-diagnostics.json` **byte-identical** to the baseline | **byte-identical** to BEFORE and the baseline |
| system (`046accf6`) | 31 / 2 / 201; units 7 Unknown / 1 Unsafe | 0 status, 0 leaf moves vs baseline | 0 status, 0 leaf moves vs BEFORE |
| tail-pause slice (`90978ac6`) | 24 / 1 / 245; units 12 Unknown / 1 Unsafe | 0 status, 0 leaf moves vs baseline | 0 status, 0 leaf moves vs BEFORE |

Functions are Safe / Unsafe / Unknown (unique by library, file and function);
**0 Pending** on every route and side. The system and slice
`resource-diagnostics.json` differ from BEFORE (and BEFORE from the baseline)
only at `compile_configuration/…/resource_contract_sources` and
`inventory_sha256`, digests over the measured tree's paths. The other corpus
files: `audit.log` equal once the run directory is named alike, both parity
streams equal once sorted, the contract sidecar equal once the tree root is
named alike, `resource-diagnostics.source-effects.json` differing only in
identity digests (`build_digest`, `configuration_digest`, `task_digest`, the
`resource_contracts:` obligation id), `run.json` in paths, binary hash and
times; system and slice `console.log` differ only in the `preparation:`
wall-clock line (validation policy: a measurement).

**No function moves**, so there is no attribution to give, no Safe →
non-Safe move and no correction on the routes, and no leak or violation is
added or removed. This is the proof the brief asks for that `main` has nothing
this card changes: the second analysis runs only for a unit with a `Pending`
row, and no route has one. The baseline stays the condvar integration
archive; nothing new is archived.

## 6. The two in-flight branches

**W47-LOOP-OWNER-INSTANCE-FLOW** (`worktree-agent-aa0821f6bf47bd9c0`). Its
report registers this card on `src/flb_lib.c::flb_input` (slice, both
variants) moving `Unknown → Pending`, and reads the held statement as masking
`ctx->config`'s argument refusal. Measured here without merging code: an
archive of this card's tip in the scratchpad, overwritten with that branch's
four contract-data files (`input_instance.qv`, the fluent-bit provider's
`contracts.lock`, `providers/contract-load-path.json`,
`cycle19-resource-contracts.json`; [`overlay_branch_contracts.sh`](scripts/overlay_branch_contracts.sh)
lists them), whose code changes are census-only and never run on the
per-unit routes; the slice run by
[`branch_contracts_probe.sh`](scripts/branch_contracts_probe.sh) with each
binary. Rows: [`artifacts/instance-flow-probe.json`](artifacts/instance-flow-probe.json).

| Run | `flb_input` (×2) | Slice rows (every variant) |
|---|---|---|
| base binary (`if-before`) | **Pending**, one held call | 37 Safe / 1 Unsafe / 364 Unknown / 2 Pending: the branch's figure reproduced |
| this card (`if-after`) | **Pending**, unchanged | identical `resource-diagnostics.json` |
| declarations accepted as written, flexible ([`accept_as_written.py`](scripts/accept_as_written.py), `if-accepted`) | **Safe**, no diagnostic | 39 Safe / 1 Unsafe / 364 Unknown |

With `flb_input_new` in force its `config` position is `Borrowed`, and the
typed reading admits `ctx->config` there; the refusal the branch saw is the
argument gate of an **uncontracted** callee, which the accepted declaration
removes. So `flb_input` is not an instance of this defect: under the rule as
amended, accepting the held declarations as written would make it `Safe`,
and it stays `Pending` (my prediction, `Unknown` at 90%, was wrong; §7). What
is questionable about that `Pending` is different: the marker on
`resource_borrowed(config)` itself says the clause is `retained, not borrowed`
and `must not be unwrapped as written`, so acceptance as written is not a
resolution the marker allows. That is registered, not decided here
(`W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN`, a product decision). The branch's
system move (`flb_storage_create`, a reworded `Unknown` leaf) is not
`Pending`, so this card does not read it.

**W47-CONDVAR-JOIN-STOP-PATHS** (`worktree-agent-a1c83288552269c3a`).
`flb_output_exit` (system), `flb_input_instance_exit` and `flb_create`
(slice) reword a leaf to the held call and stay `Unknown` on other leaves.
None is `Pending`, so this card never reads them and their bytes do not
change; whether a held statement also replaced a refusal there moves
attribution only (the row is `Unknown` either way). Not measured on a
combined tree (predicted by construction: `settle` reads `Pending` rows only).

Per function, merged with this card: `flb_input` stays `Pending` (measured);
`flb_storage_create`, `flb_output_exit`, `flb_input_instance_exit`,
`flb_create` stay `Unknown` with unchanged leaves.

## 7. Prediction against outcome

| Prediction | Outcome |
|---|---|
| corpus, system, slice at `main`: no movement (99%) | **held**: corpus byte-identical; system and slice 0 status and 0 leaf moves (only the tree-path digests differ) |
| BEFORE reproduces the baseline (95%) | **held** for the corpus (byte-identical) on the second attempt (the first failed on the archive's empty libevent submodule), system and slice (0 moves) |
| instance-flow `flb_input` `Pending → Unknown` (90%) | **wrong**: `Pending` before and after; `Safe` with the declaration accepted as written (the `ctx->config` refusal belonged to the uncontracted call) |
| instance-flow `flb_storage_create`, corpus rows unchanged (99%) | not re-measured on a combined system run; unchanged by construction (not `Pending`) |
| condvar stop-paths rows unchanged (99%) | unchanged by construction (not `Pending`); not re-measured |


## 8. Lean

`QCore.ReviewHoldStanding` (`proof/QCore/ReviewHoldStanding.lean`), audited
in `proof/AuditReviewHoldStanding.lean`: 22 theorems, no placeholders, no
project axioms (`propext` at most). The model: a row's held reasons and the
reasons of its analysis with every held declaration in force; `cap` (a
violation of the second analysis counts `Unknown`); `publish h r = if h =
Pending then join Pending (cap r) else h`; `standing` (what `settle` appends).

- `Row.pending_only_if_every_reason_is_a_hold`: a published `Pending` has
  every reason of both analyses `Safe` or `Pending`, and a held link for a
  human-resolvable reason (task: Pending only if every non-Safe reason is a
  hold).
- `Row.a_held_call_never_removes_another_reason`: on a row the hold touched,
  every reason of the second analysis, capped, is below the published verdict.
- `Row.never_below_the_verdict_without_the_rewrite`,
  `never_below_the_resolution`: the published verdict is at least the capped
  verdict without the rewrite; `publish_never_lowers`,
  `only_a_pending_row_moves`, `a_held_row_is_never_published_safe`,
  `the_second_analysis_never_publishes_unsafe`.
- `Row.the_settled_fold_is_published` (with `pending_join_standing`): the
  Rust fold over the held reasons plus the standing reasons is exactly
  `publish`.
- `an_unknown_hold_absorbs_every_refusal`: the other holds (§3).
- `Row.a_held_call_beside_a_standing_refusal_is_unknown`, `Row.an_alleged_violation_is_unknown`.

Premises stated, not proved: which reasons each Rust analysis records, and
that the second analysis is the held one with the holds lifted (pinned by the
fixture tests); a declaration another unit's body would refute once accepted
is not seen (`W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION`).

## 9. Records and follow-ups

Gateway rule 10i amended and its router row; language spec §L21.2 and
contract spec C06.1/C06.2 sentences; STATUS verdict and follow-up rows;
[wave-47.md](../../development/increments/wave-47.md) paragraph.
[`scripts/update_ledger.py`](scripts/update_ledger.py) (idempotent) applies
[`followups.json`](followups.json): 551 → 554.

- Retires `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`, which
  W47-LOOP-OWNER-INSTANCE-FLOW registers on its branch; it is not registered
  on `main`, so the retirement is a no-op here and says so. **Integrator:**
  re-run the script after merging that branch.
- Registers `W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION` (size 0 on the three
  routes at `main`), `W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN` (1 slice
  function, 2 variants, on the instance-flow branch only; §6) and
  `W45-HELD-UNIT-REASON-CODE-NAMES-HOLD-FIRST` (an `Unknown` unit with a held
  call keeps the human-resolvable unit `reason_code`, since
  `ReasonCategory::classify` names the held premise first; the verdict is
  unaffected; size 0 at `main`).

A census build (`indirect-census`, off by default) records the second
analysis's streams too for a unit with a `Pending` row; no such unit exists on
the routes today.

## 10. Gate

GATE
