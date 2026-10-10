# NULL-DEREF-QTT-GROUNDING

Card **NULL-DEREF-QTT-GROUNDING** (wave 48, 2026-10-10; card-high). Base: local
`main` at `cc7a20c3e`. Prediction: [PREDICTION.md](PREDICTION.md), frozen at
`4352e48b7` before any proof or spec text. The scope grew twice in flight, by
the user's direction and decision relayed by the integrator (sections 3 and
4); the prediction covers the first scope only. Contract specification
[C09.3](../../QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)
("Grounding", "Decomposition", "Two levels"); language specification
[§L07](../../QCORE_SPEC_r0_3.md#l07-usage-grades), with cross-references from
§L06, §L08 and §L09; grammar
[G05d](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks);
gateway rule [14q](../../agents/gateway.md#constraint-14q); Lean
`QCore.NullDerefGrade` and `QCore.DerefDecomposition`.

## 1. The user's question and decisions (verbatim where the user's words were given)

* **2026-10-11, grounding.** The user asked which of Quirin's type systems
  `quant null-deref` belongs to, since `quant` always refers to the
  quantitative types (the question as the brief records it). Decision: "We
  need grounding." The integrator's analysis, which the user accepted, is
  stated normatively in C09.3 and §L07. `quant null-deref #B;` is the
  contract-level projection of a §L07 usage grade, read with §L08–§L09 access
  and identity. It is not a §L06 index or dimension, and not a `credit`,
  `live` or `cost` axis.
* **2026-10-11, decomposition.** `quant null-deref` is read through the
  original `quant` family: existence, extent and liveness. Direction: "This
  can be derived from the proven lean. And it can be natural."
* **2026-10-11, two levels.** Contract authors may write either the shorthand
  form or the decomposed form, and both must mean the same thing. The user
  chose "Two levels".

## 2. Grounding (spec and Lean; no behaviour change)

`QCore.NullDerefGrade` (36 audited theorems, `proof/AuditNullDerefGrade.lean`)
reuses `QCore.Usage` and `QCore.UsageDemand` (the model behind `usage.rs`; see
`docs/QTT_ALIGNMENT.md`) and `QCore.NullDeref`.

| Brief theorem | Lean |
| --- | --- |
| a `relevant` demand on every path ⇔ the clause's callee validation | `relevant_accepts_body_iff_inferred`, `necessary_iff_relevant`, `relevant_body_needs_the_identity` |
| lower bound 0 imposes no caller obligation (so no clause) | `zero_lower_imposes_nothing`, `many_imposes_nothing`, `a_path_without_a_first_dereference_infers_no_clause` |
| a met demand carries the identity forward; the moved question is sequential addition | `moved_question_is_sequential_addition` (with `nonNull_kept`, `requiresIdentity_add`) |
| the branch join of grades matches the walk's path join | `branch_join_matches`, `requiresIdentity_join`, `established_join`, `relevant_admits_join_iff` |
| `QCore.NullDeref`'s caller obligation equals grade admission against an existing identity | `caller_obligation_is_grade_admission`, `entry_is_an_admitted_identity`, `admission_establishes_the_entry` |

The path model's `firstOf` is proved equal to the Rust `First` fold
(`firstOf_is_firstAfter`, over the existing `firstVectors`). A new table,
`gradeVectors`, binds `qcore_core::null_deref`'s `First`, `inferred` and
`clause_standing` to `qcore_core::usage::Usage`
(`crates/qcore-core/tests/null_deref_grade.rs`, 5 tests, including exhaustive
checks on every path of up to 5 events and on every pair of arms of up to 3).
A clause position is §L07 demand substitution
(`a_clause_position_demands_the_identity`); an unannotated one demands nothing.

**Divergence found, decided conservatively, no behaviour change.** §L07 admits
a declared grade only when it contains the body's interval, and the usage
checker rejects `one` on an unused parameter. The clause's body check reports
`stronger-than-necessary` instead and never rejects (decision 2 of
2026-10-10). The grounding states the exception normatively: the clause is a
caller-side existence precondition, not a linear demand, and read shares are
borrowed, not consumed. `stronger_than_necessary_is_still_sound` proves that
such a body's grade is one `relevant` refuses, yet every caller meeting the
obligation still establishes the callee's entry reading. This is a statement
of the walk's existing behaviour, not a correction.

The prediction's second possible divergence holds as predicted. A body with no
terminating path has no grade, and the walk infers nothing from it.

**Residual.** The walk computes `First`, the grade's abstraction, and never
constructs a `Usage`. That its five `First` sites are the Lean path model is a
review premise (trust boundary 28): `NULL-GRADE-WALK-CORRESPONDENCE`.

## 3. Decomposition (spec and Lean)

`QCore.DerefDecomposition` (27 audited theorems,
`proof/AuditDerefDecomposition.lean`). A safe dereference of `B` needs three
parts: (a) existence, `quant null-deref #B`; (b) extent, `quant checks B,
Idx #i: #i == 0`; (c) liveness, custody's `live`. The only modelling choice is
the concrete pointer `Ptr`: NULL, or a position in an identity's object with
the elements that remain from it and that identity's
`QCore.BorrowLiveness.Status`. No new rule and no axiom.

| Entailment | Theorem (derived from) |
| --- | --- |
| 1. `checks(B, #i == 0) ⊢ null-deref(B)` | `checks_zero_entails_null_deref`, `checks_zero_entails_extent` (`IndexExtent.element_access_sound`, `IndexQuant`'s `requirement` and `bothInBounds`, `NullDerefGrade.admits`, `NullDeref.sound`) |
| 2. The converse fails | `null_deref_does_not_entail_checks_zero` (one past the end), `an_unknown_extent_is_never_in_bounds` |
| 3. Grade ≥ 1 obligates (a) and every stated part; grade 0 none | `relevant_obligates_existence`, `relevant_obligates_a_stated_part`, `an_unstated_part_is_never_obligated`, `zero_lower_obligates_no_part`, `body_grade_decides_the_obligations` (§L07 substitution: `callerDemand_is_substitution`) |
| 4. Neither (a) nor (b) discharges (c) | `existence_and_extent_do_not_discharge_liveness`, `a_released_identity_is_refused_although_it_exists_and_has_extent` (`BorrowLiveness.a_settled_released_identity_is_refused`), `each_part_fails_alone` |

No analysis uses entailment 1. Discharging a NULL obligation from a bounds
fact is the registered follow-up `NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`, with
`checks_zero_entails_null_deref` as its basis.

## 4. Two levels (loader elaboration)

| Piece | Where | Rule |
| --- | --- | --- |
| `quant deref #B;` and `quant live #B;` parse as `QuantDeref` and `QuantLive` (before: parse errors); `quant WORD #B;` with an unknown word is `quant-shorthand-unknown` | `qcore-frontend/src/frontend.rs` | 14q |
| Elaboration into canonical parts; overlaps merged when they agree, refused when not (`quant-deref-conflict`); shapes and subjects named | `qcore-contract/src/deref_parts.rs` (new), hooks in `bounds_contract.rs` | 14q |
| `BoundsClause::Live`; `for_bounds` also removes it; `canonical_sha256` beside the pinned `text_sha256` | `bounds_contract.rs` | 14q |
| A liveness part admitted on a route only where the function's resource v1 declaration in force states `resource_owned` there (`quant-live-unbacked`, `quant-live-contradicts-custody`); canonical digests and backed parts in the bounds selection record only when a liveness part exists | `deref_parts::check_live_backing`; `qcore-audit/src/bounds_selection.rs` (`select_for_route` takes the route's resource environment; `pinned_route.rs`, `current.rs`, `no_build.rs`) | 14q |
| Exhaustive matches over the clause kind | `contract_views.rs`, `contract_inclusion.rs`, `index_extent/body_evidence.rs`, `compiler/tests/qcore_contract_corpus.rs` | — |

**Vocabulary choice.** Both new forms are admitted in **bounds contract v1
only**. It is the vocabulary that already carries collocated check forms, pins
them and selects them on the routes, and its loader is the one place where
all three parts of one declaration meet. Each canonical part reaches its own
consumer:

* (b), the extent part, reaches the bounds consumer.
* (a), the existence part, reaches the null property.
* (c), the liveness part, reaches custody only as a checked restatement of the
  function's own resource v1 declaration. Custody reads resource v1 alone, and
  the resource v1 loader refuses both forms (they are `quant` clauses; its
  loader admits `requires` and `ensures normal`), so a part custody cannot
  read is refused, never ignored.

`resource_owned` backs a liveness part; `resource_borrowed` does not, because
custody's `Borrowed` rule refuses only an identity released on every reaching
path. That narrowing is registered as `QUANT-LIVE-CUSTODY-READING`.
`null_deref_beside` is the precedent: a checked agreement between a bounds
declaration and its resource view.

**Lean.** The loader's elaboration and merge preserve meaning in the
three-part model: `shorthand_means_its_parts`, `elaboration_preserves_meaning`,
`merge_preserves_meaning`, `merging_an_agreeing_overlap_is_idempotent`,
`merge_merge`, `shorthand_and_decomposition_are_one_canonical_set`,
`a_mix_is_the_shorthands_canonical_set`, `one_canonical_set_one_meaning`.
That the Rust loader is the model's `elaborate` and `merge` is a review
premise, pinned by the controls below.

**Controls.**

* `crates/qcore-contract/tests/deref_parts.rs` (8 tests):
  * the shorthand, the decomposition and seven mixes give one canonical set and
    one canonical digest, with different pinned digests;
  * the bounds view and the null clauses are equal;
  * `quant null-deref` keeps its meaning;
  * contradiction, duplicate, shape and subject controls;
  * the unknown-shorthand control;
  * resource v1 refuses both forms;
  * liveness backing: `resource_owned` backs a liveness part, while
    `resource_borrowed` and an absent declaration refuse it.
* `crates/qcore-c-gateway/tests/deref_parts_null.rs`: on a real Clang AST, the
  shorthand, the decomposition, two mixes and `quant null-deref` alone give
  byte-identical null rows (verdict, checks, standing, dependencies).

No corpus contract uses the new forms, and no contract-loader pin table
changed.

## 5. Measurement: byte identity

Every route ran with the null selection and the parity stream, BEFORE on
main's binary and AFTER on the card's, each under the shared lock
([scripts/measure.sh](scripts/measure.sh), [scripts/identity.py](scripts/identity.py)).

| Run | Binary | Built at |
| --- | --- | --- |
| BEFORE | `2a7b71cb…` | `cc7a20c3e` (main) |
| AFTER (grounding: spec, Lean, a Rust test) | `31b464e6…` | `c40756b2f` |
| AFTER2 (with the two-level loader) | `23766fab…` | `10a0bb00c` |

The binaries differ although the first AFTER changed no Rust outside a test:
`crates/qcore-audit/build.rs` embeds `HEAD`'s commit
(`QCORE_AUDIT_BUILD_COMMIT`). The same length differs in 1,593 bytes. The
prediction that the binary would be byte-identical did not hold for that
reason. The outputs are the evidence.

AFTER2_TABLE

Totals (BEFORE, reproducing the brief): custody corpus **915 / 9 / 3,910**
(`ded14632…`), system **91 / 4 / 353**, slice **50 / 1 / 349 / 4 Pending**;
null corpus **1,219 Safe unconditional / 1,442 Safe conditional / 2,260
Unknown / 0 Unsafe**, 2,173 functions with a candidate. Wall time, BEFORE /
AFTER / AFTER2: corpus 478.0 / 486.6 / AFTER2_CORPUS s, system 64.2 / 64.6 /
AFTER2_SYSTEM s, slice 29.3 / 29.3 / AFTER2_SLICE s.

## 6. Prediction against outcome

| Predicted | Measured |
| --- | --- |
| custody, null rows and parity byte-identical on corpus, system and slice | custody reports, null rows, summaries and normalised transcripts byte-identical; the parity streams identical as multisets (addresses erased), their order not reproducible between runs |
| the release binary byte-identical | **no**: the build embeds `HEAD`'s commit |
| grounding provable without changing the walk (1–5) | yes, all five |
| divergence: §L07 would reject a lower-bound-0 body; decision 2 reports it | yes, stated normatively; `stronger_than_necessary_is_still_sound` |
| no grade for a body with no terminating path | yes (`bodyGrade_none_iff`; the walk infers nothing) |
| one module, 309 audits | two scope additions: three modules, **311** audits |

## 7. Gate

GATE_SECTION

## 8. Follow-ups

Registered, ledger 748 → 751:

* `NULL-GRADE-WALK-CORRESPONDENCE`: the walk's five `First` sites are the grade
  model by review only.
* `NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`: entailment 1 is proved but unused.
* `QUANT-LIVE-CUSTODY-READING`: the liveness part is a checked restatement of
  `resource_owned`; custody never reads it.

None is retired: the card's brief named no ledger entry of its own.

## Files

`scripts/`:

* measurement and comparison: `build.sh`, `measure.sh`, `measure-binary.sh`,
  `heavy.sh`, `identity.py`, `parity.py`;
* gate: `gate.sh`, `write_gate_json.py`, `finish-gate.sh`;
* records: `update_ledger.py`.

`artifacts/`: `identity*.json`, `parity-*.json`, null summaries.
