# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `d227adc5`. It is
merged onto main `1ab56306`, after W44-INDIRECT-CHAIN-ACCESS,
W44-CORPUS-CALLEE-SUMMARIES and W44-BRANCH-CONDITION-CALLS. The run used the
integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are
in [gate.json](gate.json).

## Conflicts

* **`legacy_c.rs`:** both module groups are kept: `condition_desugar`, and
  `refused_operands`, `cast_return` and `leak_witness`.
* **`tests/branch_condition_calls.rs`:** main's pin is kept. The card had
  re-pinned the short-circuit release `if (flag && (q_free(p), 1))` to "refused,
  no leak". On the combined tree W44-BRANCH-CONDITION-CALLS desugars that
  condition, so no statement is refused. The hand-on rule acts only where a
  refusal was reported, so it does not act here, and the skipped-call path's
  leak stays claimed: Unsafe, no branch refusal.
* **Records:**
  * the ledger is merged per entry: 367 + 11 registered − 1 retired = **377**;
  * the increment paragraph is prepended;
  * the report index is regenerated;
  * `STATUS.md` corpus rows are re-derived from this arbiter.

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,489 passed / 0 failed / 11 ignored**, 293 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 182 audits (`CastReturnEscape`, `RefusedHandOnCustody` added) |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-branch-condition-calls/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt), [explained-leaves.json](explained-leaves.json), checker v3) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 377 follow-ups; 330 tests |

## Composition

**Card delta.** Against `w43-indexed-resource-collections/integration/artifacts`,
the card removes 260 leaves and adds 19. It moves 79 functions and 35 units.

**Overlap.** One removed leaf was already gone on main: the `snappy.c:1712` leak,
which W44-BRANCH-CONDITION-CALLS decided through its null refinement.

**The one interaction: `rd_kafka_snappy_java_uncompress` (3 leaves, status kept).**
* **On the card's base.** The card withdrew this function's four leaks through
  the refused hand-on rule. Its only handing statement was the refused condition
  `if (unlikely(!outbuf))` at `snappy.c:1706`, as recorded in
  `handoff-classification.json`.
* **On main.** W44-BRANCH-CONDITION-CALLS desugars that condition, so it is no
  longer a refused statement and the rule correctly does not act.
* **What remains.** The three remaining leaks, at exits 1648, 1660 and 1701,
  are main's own, byte for byte. The audit classifies them (c):
  path-insensitive correlation on the `pass` counter. The function keeps main's
  provisional Unsafe. `LEAK-PATH-SCALAR-CORRELATION` is amended to record this.

**Checker change.** To state this, the composition checker gains a v3
`status_kept` explanation. It is accepted only when the function keeps PREV's
status exactly and the restored leaves are listed.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `1ab56306` | combined |
|---|---|---|
| functions | 296 / 85 / 4,453 | **299 / 10 / 4,525** |
| units | 3 / 43 / 135 / 4 | **3 / 9 / 169 / 4** |
| leak | 270 | **21** |
| unknown_alias | 4,457 | 4,473 |
| unsupported | 45,416 | 45,412 |
| double / invalid release, use after release | 0 / 0 / 0 | 0 / 0 / 0 |

* **Unsafe → Unknown.** 75 functions move from provisional Unsafe to Unknown,
  and so do 34 units. Their leaks are withdrawn and their refusals kept. This is
  the card's 76 functions and 35 units, less `rd_kafka_snappy_java_uncompress`
  and `snappy.c`.
* **Unknown → Safe.** Three `rdkafka_admin.c` accessors move from Unknown to
  Safe through the cast-return decision, not the hand-on rule.
* **Refusal censuses.** The indirect-access and return-expression rows grow
  because the functions that moved to Unknown bring their existing refusals into
  those censuses. No refusal was added.

**Cost, stated plainly.** The rule withdraws **128 leaks the audit judged real
by reading**, together with the artifacts and premise gaps; they are registered
as `REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`. This is the fail-closed direction:
every withdrawn leak becomes Unknown, never Safe. The 21 remaining leaks are
provisional candidates, not upstream defect claims.

## Carried to later cards

* **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE.** Merge the
  `DESTINATION-SUFFIX-PLACE-SPELLING` evidence into that card's discharge. The
  two `unpack_basic_type` leaks are that card's.
* **W44-INDIRECT-CALL-ARGUMENT-ACCESS and W44-COLLECTION-ALLOCATION.** Re-check
  their leak claims on this combined tree before merging. Retire
  `W44-REFUSED-HANDOFF-LEAK-CLAIMS` and `W44-CALL-ARGUMENT-EXPOSED-LEAKS` there;
  neither is on main yet.
* **Gateway standing constraint 7.** It still opens with "272". Its preserved
  text is left for the wave-close DOCS-COMPACT card. The current pin is 21, and
  this report is the attribution.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
