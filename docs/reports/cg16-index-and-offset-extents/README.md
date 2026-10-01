# CG16-INDEX-AND-OFFSET-EXTENTS

Wave 48, round 9. Card CG16 of the Clang gap queue (entries M14, M15, M16,
M17), route G with an explicitly selected bounds consumer; gateway rule
[11s](../../agents/gateway.md#constraint-11s); contract specification
[C09.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned),
whose status paragraph now names what landed; grammar
[G05b](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05b-bounds-contract-v1). Lean:
`QCore.IndexExtent` ([`proof/QCore/IndexExtent.lean`](../../../proof/QCore/IndexExtent.lean),
audited in `proof/AuditIndexExtent.lean`). Base: local main `b6a3dc42`.
Detailed brief: [cg16-reviewed-index-contracts.md](../../plans/clang-gap-cards/cg16-reviewed-index-contracts.md).
Absorbs `W43-INDEX-TRAVERSAL-STEP-COMPUTED`,
`W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS` and `W46-ROUTING-QUERY-EXTENTS`.

**Published property and scope.** `c.bounds.index-extent.v1`: every
subscript, dereference, `->` access, pointer formation, pointer subtraction or
ordering, and every pointer passed to a call, of a selected function, is
within the extent of its object — a fixed array's extent from the source, or
an extent a reviewed bounds declaration states — under the recorded target's
type sizes. Reported separately from custody; joined with it only when a
request names both. It is a bounded sub-increment of the card: §6 lists what
did not land, and the CG16 ledger entry is amended, not retired.

## 1. What changed

| Layer | Item | Owns |
|---|---|---|
| core | [`qcore_core::index_extent`](../../../crates/qcore-core/src/index_extent.rs) | element access `0 <= i < N`, formation `0 <= i <= N`, checked byte access and `element_size * index` against `PTRDIFF_MAX`, same-object, source/declared extent resolution (no override), fact invalidation (`Facts::write`, `call`, `join`), the standing verdict (`bounds_verdict`), composition (`compose`), admission (`admit`) |
| contract | [`qcore_contract::bounds_contract`](../../../crates/qcore-contract/src/bounds_contract.rs) | bounds contract v1 (G05b): selector `bounds_contract_v1`; `bounds_extent`, `bounds_index`, `bounds_global_extent`, `bounds_return_extent`, `bounds_preserves_extents`; own marker `bounds_review_pending` with C06.1's reason/owner/disposition; total-or-reject; per-declaration text digest; environment digest; a refuted-name table |
| gateway | [`qcore_c_gateway::index_extent`](../../../crates/qcore-c-gateway/src/index_extent.rs) | the route-G source consumer (rule 11s): inventory, fixed extents from Clang types, values, guards, invalidation, reachability, exclusion, review holds, body refutation of a return extent, evidence re-derivation (`check_bounds_report`) |
| audit | `QCORE_BOUNDS_PROPERTY` ([`projection.rs`](../../../crates/qcore-audit/src/projection.rs), [`report.rs`](../../../crates/qcore-audit/src/report.rs)) | when set, each projected function's bounds row, its custody status and the combined verdict, as JSON lines plus a tally; the custody document never reads them |
| Lean | `QCore.IndexExtent` | the model of the core decisions, 32 audited theorems, shared vectors |

No custody code changed: there is no hook in `legacy_c.rs`, `typed_source.rs`
or any custody route; the audit change runs only under the variable. No shared
foundation (`elaboration.rs`, `instantiate`, summaries, the typed place) was
edited. Shared items added: `qcore_core::index_extent` (new module),
`qcore_contract::bounds_contract` (new module), `AuditReport::{record_bounds,
write_bounds_selection}` and its `bounds` field (excluded from the document),
`projection::BOUNDS_SELECTION`.

## 2. Cited claims (CG00's BEFORE controls)

`crates/qcore-c-gateway/tests/clang_gap_phase3.rs` (M14–M17) reproduced on the
base, unchanged by the card (15 passed), and each control gains a bounds row in
`tests/cg16_index_extents.rs`:

| Entry | Claim | Custody (unchanged) | Bounds row (new) |
|---|---|---|---|
| M14 | computed indices carry no bounds obligation; `s->arr[7]` custody-Safe | reproduced: `s->arr[i]`, `s->arr[7]` Safe; the resource loop refused (IND) | `s->arr[7]` **Unsafe** (subobject of 4); `s->arr[i]` Unknown (`bounds-unsupported-range`) |
| M15 | pointer arithmetic carries no offset | reproduced: `*(p + i)`, `q = p + 3` Safe | Unknown (no extent for `p`) |
| M16 | pointer difference and ordering carry no same-object obligation | reproduced: `a - b`, `a < b` Safe | Unknown with a `same-object` record |
| M17 | one-past formation is not separated from dereference | reproduced: `e = a + n; *e` Safe; `s->arr + 4` refused (IND) | `arr + 4` formed, `*e` **Unsafe**; `s->arr + 4` Unknown (no extent for `s`) |

Q25's claim (the intervals of `indexed_collection.rs` are release
obligations, not bounds) holds: the consumer reads none of them. No cited
claim failed to reproduce.

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen at `353dc337`, after the core,
Lean model and loader and before the gateway consumer and the audit selection.

Measured with [`scripts/measure.sh`](scripts/measure.sh) (corpus `66910c10`
`--jobs 4`, system, tail-pause slice) and, for the AFTER corpus,
[`scripts/corpus.sh`](scripts/corpus.sh); BEFORE binary `e7e36b7c…` (main
`b6a3dc42`), AFTER binary `e0b60203…` (tip `a859fbf6`; later commits are
formatting, clippy and records only). Clang 21.1.8. Comparison:
[`artifacts/identity.json`](artifacts/identity.json) by
[`scripts/identity.py`](scripts/identity.py).

The card was rebased five times after the measurement (onto main
`8e8ed41f` CG17, `06ad81cb` CG20, `b934c560` CG25, `f0bbc0eb` CG26, `f6336c9d` CG28). None of them reads the
bounds selection and this card changes no custody code, so the comparison is
not repeated; the bounds and combined figures below are those of the base
`b6a3dc42` (CG25's +1 custody Safe after it is not in the combined column).

**Custody (unchanged, as predicted).**

| Route | BEFORE | AFTER (bounds selected on the corpus) | Movement |
|---|---|---|---|
| Corpus | 629 / 10 / 4,195, `3d53a5d0…` (= main's stated baseline) | byte-identical | 0 |
| System | 44 / 2 / 188, `667b40d1…` | byte-identical | 0 |
| Tail-pause slice | 29 / 1 / 239 / 1 Pending, `e1decbfe…` | byte-identical | 0 |

The system and slice digests differ from the brief's stated `79404fed…` and
`93cc660c…` with equal totals: those reports embed the measuring checkout's
paths (8 occurrences of this worktree's path in the system report), so their
bytes are per-worktree; the comparison above is within one worktree.

**Bounds (the new property).** The selection rows
([`artifacts/after-bounds.summary.json`](artifacts/after-bounds.summary.json),
[`artifacts/after-bounds.digest.json`](artifacts/after-bounds.digest.json) by
[`scripts/bounds_summary.py`](scripts/bounds_summary.py); the 82.7 MB rows are
`private/cg16ix/after-bounds.jsonl`, `06875c5d…` in
[`artifacts/after-bounds.jsonl.sha256`](artifacts/after-bounds.jsonl.sha256))
cover all **4,921** enumerated definitions: the 4,834 projected functions and
the 87 pinned-build exclusions, which the custody report does not project and
the bounds row reports as `bounds-import-failure` (no definition in the
analysed text).

| Measure | Predicted | Outcome |
|---|---|---|
| Custody, every route | byte-identical | **byte-identical** |
| Bounds Safe | ~10% of projected (5–20%) | **162** (3.4% of 4,834): **missed low**. 160 have no selected access and pass no pointer to a call; 2 (`LZ4_isLittleEndian`, `XXH_isLittleEndian`) read `one.c[0]` of a 4-element union member |
| Bounds Pending | 0 | **0** |
| Bounds Unsafe | 0 (0–2) | **0** |
| Bounds Unknown | the rest | **4,759** (4,672 projected + 87 not projected) |
| Combined (custody + bounds) Safe | 50–250 | **111** (combined: 111 Safe / 4,800 Unknown / 10 Unsafe) |

Custody × bounds over the projected functions: Safe×Safe 111, Safe×Unknown
518, Unknown×Safe 51, Unknown×Unknown 4,144, Unsafe×Unknown 10. Every bounds
row's custody status equals the published one (4,834 matched, 0 mismatched).
The miss: the prediction did not count how many otherwise scalar functions
pass a pointer to an uncontracted call; 4,133 Unknown functions carry such a
call obligation (660 of them have nothing else).

Unknown reasons (functions; a function counts once per code):
`bounds-missing-extent` 4,650 (sole 3,715), `bounds-unsupported-form` 817
(sole 10; statement expressions 774, `_Generic` 36, `va_arg` 15, C11 atomics
3, inline `asm` 2, one `ExprWithCleanups`), `bounds-unsupported-range` 214
(sole 11), `bounds-range-not-contained` 8, `bounds-import-failure` 87.
Obligations: 192,639 records (call preconditions 91,780, member access
90,479, dereference 3,458, element access 4,386, formation 2,164, same-object
372); in bounds 5,636, excluded by the reachability check 148. **No Unsafe
to classify and no Safe → non-Safe move.** No correction.

## 4. Acceptance controls (brief §4)

All through the source consumer (`tests/cg16_index_extents.rs`, 20 tests),
the loader (`crates/qcore-contract/tests/bounds_contract.rs`, 5) and the core
(`crates/qcore-core/tests/index_extent.rs`, 10):

| Control | Outcome |
|---|---|
| `int arr[4]`, access at 0 or 3 | Safe |
| same object at 4, 7, -1 | Unsafe, no contract |
| source 4, reviewed claim 8 (`bounds_global_extent`) | `table[7]` stays Unsafe with `bounds-declaration-contradicted`; `table[2]` Unknown (a contradicted premise discharges nothing); `extern int table[]` takes the claim |
| custody profile's no-UB premise | custody Safe, bounds Unsafe/Unknown on the same function: the custody row discharges nothing |
| external base/count/index relation, held | Pending; the reason carries the clause scope, reason, owner, category and the declaration's SHA-256 |
| the same relation resolved | Safe, scope `conditional on subject's bounds preconditions (declared): …` |
| held plus an unrelated unsupported access | Unknown with both reasons |
| violation only under the held declaration | Unknown, `bounds-held-premise-violation`; resolved, Unsafe |
| missing extent, unsupported range, stale identity | Unknown; stale identity refused at admission; no pin hold exists in this vocabulary (no Pending from a pin) |
| callee requires an in-range index | unestablished: Unknown; established by a guard: Safe; violated: Unsafe; callee held: Pending |
| guard then change | index reassigned, address taken, base reassigned, release call: Unknown; re-derived constants and tracked steps decide |
| zero extent, bytes vs elements, overflow, signedness | empty declared extent Unsafe; 16 bytes vs `int[4]` / `int[3]` Safe / Unsafe; scaled index past `PTRDIFF_MAX` `bounds-arithmetic-overflow`; `(unsigned)-1` an exact violation, `(signed char)200` refused |
| one-past vs dereference; unrelated subtraction | `arr + 4` formed, `*e` Unsafe, `arr + 5` Unsafe; `&a[1] - &b[0]` Unsafe, `&a[3] - &a[0]` Safe |
| unreachable vs unresolved reachability | `if (0)` and after `return`: excluded and Safe; `if (x)`, after a call, beside a call, under `&&`: Unknown |
| custody Safe with bounds Unknown/Pending/Unsafe | combined Unknown/Pending/Unsafe; a missing requested row Unknown |
| missing access, omitted row, changed contract/target/source, malformed verdict | `check_bounds_report` refuses each |
| resource-v1 consumer given the new clause | total rejection (both directions) |

Also: positions (condition, actual, loop body, return, declaration), pointer
reads and writes, a statement expression and a VLA as named Unknowns, and the
in-unit body refutation of a return extent (the caller sees an uncontracted
callee). Not covered by a control because the vocabulary has none:
inheritance (G05a) — a bounds module with `#include` is rejected — and
cross-unit refutation on the audit routes (§6).

## 5. Lean

`QCore.IndexExtent` models the core decisions over `Int`/`Nat` with symbols
interpreted by an environment, and `AuditIndexExtent.lean` audits 32
theorems (only `propext`, `Classical.choice`, `Quot.sound`; no placeholder):

* soundness of the decisions: `element_access_sound` and `formation_sound`
  (an in-bounds decision holds for every value the index fact allows),
  `out_of_bounds_sound` (a violation holds for every such value: only exact
  indices are called out of bounds), `missing_extent_never_in_bounds`,
  `one_past_formed_never_accessed`, `empty_extent_admits_no_access`,
  `byte_access_sound`, `scale_sound`;
* source facts: `declared_never_overrides_source`,
  `a_larger_claim_is_a_conflict`;
* invalidation: `write_withdraws_every_mention`, `write_keeps_unrelated`;
* the standing verdict: `unsafe_only_for_a_reachable_violation`,
  `safe_only_when_every_obligation_discharges`, `held_never_safe`,
  `held_never_accuses`, `pending_only_as_the_sole_obstacle`,
  `another_obstacle_keeps_unknown`;
* composition and identity: `custody_safe_bounds_{unknown,pending,unsafe}_is_*`,
  `a_missing_requested_row_is_unknown`, `combined_safe_needs_both`,
  `admitted_evidence_matches`;
* shared vectors decided by `decide`: `elementVectors` (16),
  `byteVectors` (7), `resolveVectors` (7), `verdictVectors` (11), each parsed
  from the Lean text and decided by the Rust module in
  `crates/qcore-core/tests/index_extent.rs`.

**Proof, binding, observation.** The theorems are about the model. The Rust
decisions are bound to it by the shared vectors (a correspondence on those
inputs, not an extraction). The source consumer's reading of Clang (inventory,
extents, values, reachability) is bound by the executed controls of §4 and
observed on the corpus (§3); trust boundary 28 (faithful Clang extraction) is
a stated premise. No theorem claims end-to-end C correctness.

## 6. Residuals and follow-ups

Ledger edits by [`scripts/update_ledger.py`](scripts/update_ledger.py)
(`named_followups` 672 → 676 on main `f6336c9d`; 666 → 670 on the base):

* **CG16-INDEX-AND-OFFSET-EXTENTS: amended, not retired.** Its amended
  DISCHARGE (review state persisted through inheritance, packaging,
  checkpoints and caching; pins) is not met. The amendment carries the
  **reconciliation note for CG20**: the value subset of `index_extent.rs`
  (`eval`, `in_type`, `convert`, the guards, the counted `for`) is this
  card's own, measured before CG20 landed (main `06ad81cb`); it duplicates
  `c_numeric.rs` (`TargetIntegerModel`, `binary`/`convert`, `Interval`, rule 11q)
  and is to be re-pointed at it with parity (`CG16-BOUNDS-SYMBOLIC-RANGES`).
* **Absorbed, amended with their bounds-side measurement, not closed:**
  `W43-INDEX-TRAVERSAL-STEP-COMPUTED` (2,164 formation records, 1,806
  unresolved) and `W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS` (3,458
  dereference records, 2,903 with no extent): their DISCHARGE clauses are
  custody admissions, and their custody sizes are unchanged because the
  custody report is byte-identical. `W46-ROUTING-QUERY-EXTENTS`: the
  vocabulary can now state the readable extents; no Fluent Bit declaration
  was written or reviewed.
* **Registered:** `CG16-BOUNDS-CONTRACT-TRANSPORT` (pins, inclusion,
  packaging, checkpoints, the audit's bounds-module selection, the cross-unit
  refutation phase); `CG16-BOUNDS-POINTER-EXTENTS` (4,650 functions, 3,715
  sole; 4,133 with an uncontracted pointer-passing call, 660 sole);
  `CG16-BOUNDS-UNSUPPORTED-FORMS` (817 functions, 10 sole);
  `CG16-BOUNDS-SYMBOLIC-RANGES` (214 functions, 11 sole; under CG20).
* For CG24: the vocabulary is single-base (one object per pointer fact, one
  count per extent); multi-base, derived and frame relations extend
  `BoundsClause` and `PointerFact`.
* For CG17: a variable-length array and a flexible member are named Unknowns
  here (`… is CG17's`).

**A discarded attempt.** The first AFTER corpus run used the BEFORE binary:
`scripts/run_pinned_audit.py` always runs `target/release/quirin-audit`, and
the AFTER binary had been built into `target/after`. It was stopped (exit 241)
and kept as `private/cg16ix/discarded-after-wrong-binary`; the AFTER binary was
copied to `target/release` and the corpus run again with
[`scripts/corpus.sh`](scripts/corpus.sh). The system and slice runs of the same
script took the AFTER binary through `BINARY` and are valid.

## 7. Gate

The full standing gate ran at the card's tip `02ccd5be` on main `b934c560`
([`scripts/gate-locked.sh`](scripts/gate-locked.sh), which wraps
[`scripts/gate.sh`](scripts/gate.sh) in the integrator's shared heavy-step
lock and cleans the dev profile after it; exits in [`gate/`](gate/), logs in
`private/cg16ix/gate`):

| Step | Exit | Count |
|---|---|---|
| `cargo check --workspace --all-targets` (after the rebase) | 0 | — |
| `git submodule update --init --recursive` | 0 | — |
| `scripts/run_test_binaries.py` | 0 | 419 units, 432 summaries; **7,257 passed**, 0 failed, 11 ignored (`private/test-binaries/20261001T191135Z-4005844/`) |
| `cargo fmt --all -- --check` | 0 | — |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | — |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | — |
| `scripts/check_lean_audits.py` | 0 | 267 audits (266 on that main + `AuditIndexExtent.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 1 | 365 run, 1 expected failure before the matrix regeneration (the derived `lean_audits` 266 → 267, `test_build_matrix_sections`) |
| `build_docs_index.py --check` | 0 | — |

The matrix was then regenerated with [`gate.json`](gate.json) and the Python
suite and the index check run again (`09a`, `09b`, `10b` in [`gate/`](gate/)).
Main then moved twice more (`f0bbc0eb`, CG26: contract loader and
variadics; `f6336c9d`, CG28: weak-memory events; both disjoint from this
card's code); the card was rebased onto each and checked by
[`scripts/rebased-check.sh`](scripts/rebased-check.sh): the workspace check,
the card's test files and the Lean audits (`r1`–`r4` in [`gate/`](gate/) on
`f0bbc0eb`, 268 audits; `gate/recheck/` on `f6336c9d`, 269 audits), then the
matrix regenerated again and the Python suite and index check run on the final
tree (`09c`, `09d`, `10c`). `providers/` is unchanged, so the provider gate is
not due and was not run. Clang 21.1.8.

**Discarded attempts.** Two earlier gate runs on the base hit a full disk
(other cards' builds; `No space left on device`, linker bus errors) and are
kept, not used: `private/cg16ix/gate-discarded-disk-full`,
`gate-discarded-disk-full-2`; a third on the base
(`gate-base-b6a3dc42-partial`) passed fmt, clippy, both probes, Lean (264)
and the index, its tests again hit the full disk, and their locked re-run was
cancelled when main moved. The heavy steps then ran under the shared lock.
