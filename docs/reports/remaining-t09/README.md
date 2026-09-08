# T09 — disjoint access footprints and alias identity

Date: 2026-09-08. Base checkpoint: `c60fd91` (the second delegated increment's
consolidation commit). Card: T09, P3, workstream W6, concurrency lane, kind
`bounded_design`, scope M.

Design document:
[`../../design/DISJOINT_FOOTPRINT_PROFILE.md`](../../design/DISJOINT_FOOTPRINT_PROFILE.md).

**Scope.** A design plus a bounded, model-level prototype and its Lean model.
`DisjointFootprintsV1` derives byte intervals and decides disjointness against
**one** allocation generation, under an explicit record layout contract for
fields, and separates descriptor protocol-cell identity from memory-slice
disjointness. It is **not** wired into source lowering, any production audit
route or any other checker; it does not discharge `DeclaredFootprintShape`;
no CT case is mapped; no raw Cycle 19 unit is promoted; no existing diagnostic
code, checker rule or pinned expectation changed.

---

## 1. What landed

| Artifact | Lines | Role |
|---|---|---|
| `crates/qcore-core/src/disjoint_footprint.rs` | 1,308 | new module, entry points `check_disjoint_footprint_program` / `recheck_disjoint_footprint_certificate` |
| `crates/qcore-core/tests/unit/disjoint_footprint.rs` | 1,670 | 35 tests |
| `proof/QCore/DisjointFootprint.lean` | 527 | new namespace `QCore.DisjointFootprint`; 28 public theorems, 14 decidable scenarios |
| `proof/AuditDisjointFootprint.lean` | 30 | new audit file; 28 `#print axioms` registrations |
| `docs/design/DISJOINT_FOOTPRINT_PROFILE.md` | 331 | the design: model, the three acceptance items, refusal vocabulary, premises, six follow-on units |

Edits to existing files are two, both additive:

| File | Change |
|---|---|
| `crates/qcore-core/src/lib.rs` | `pub mod disjoint_footprint;` and one re-export block (+8 lines) |
| `proof/QCore.lean` | `import QCore.DisjointFootprint` (+1 line) |

`git diff --stat` against the base checkpoint over tracked files is exactly
those two files, 9 insertions, 0 deletions. Nothing was deleted, no gate was
weakened, no pinned expectation was edited, no diagnostic was downgraded, and
`crates/qcore-schemas` is untouched (still 5.0.0).

### 1.1 A dependency that is not present

The card names T05 (counted read shares) as its landed dependency, and the
delegation brief describes the branch as taken from a tip containing
T04/T05/T06/T08. **Only T04 is present in this worktree**:
`docs/reports/` contains `remaining-t04` and no `remaining-t05`,
`remaining-t06` or `remaining-t08`, and `git log` at the base checkpoint ends
at the T04/P05/P06/P07/O04/F01/CT consolidation. This is recorded rather than
worked around: no T05 API is referenced, nothing here assumes a share algebra,
and §7 names the composition that a later card must do once T05 exists.

The direct dependency was therefore read as T04's recorded successor notes
(`../remaining-t04/README.md` §10) and T02's footprint algebra
(`../remaining-t02/README.md` §2.5). The share algebra and the footprint
algebra are orthogonal: T02's ledger conserves *shares* over *atoms* and says
nothing about where an atom's boundaries came from. This card supplies the
boundaries; it does not touch shares.

---

## 2. The model, and what it decides

The design document is normative; this section states only what the evidence
below is evidence *of*.

An `Allocate` fixes an extent for one generation. A `DeriveSlice` checks handle
freshness, allocation liveness, generation currency, a nonzero element count and
`elem_offset + elem_count <= count` **in `u64`**, then **computes** the byte
interval and compares any caller-supplied interval against it. A `DeriveField`
requires a validated whole-record layout contract and a base slice covering
exactly one element. A `DeclareDescriptorAlias` records a declared protocol
cell.

`AssertDisjoint` is the checked obligation and `decide_disjoint` the only route
to a judgement. Its rules, in order: incomparable kinds → cross-allocation →
stale generation → shared bit-field unit → interval arithmetic; and for
descriptors, shared cell → distinct cells.

---

## 3. Acceptance

### 3.1 "Prove interval bounds/disjointness against one underlying generation identity"

| Obligation | Rust | Lean |
|---|---|---|
| bounds are checked, not assumed | `a_slice_past_the_declared_extent_is_refused` (`Q633`, with an in-bounds positive control) | `sliceBytes_within_extent` |
| the bounds arithmetic cannot wrap | `an_element_range_that_wraps_a_u32_is_refused_rather_than_wrapping` — `elem_offset = u32::MAX`, `elem_count = 2` reports `elem_end = 4294967297` instead of `1` | — |
| degenerate and unrepresentable extents | `a_degenerate_or_unrepresentable_extent_is_refused` (`Q630`, five cases including a `2 × 0x8000_0000` byte extent exactly one past the representation) | `sliceBytes_nonempty` |
| the interval is derived, not believed | `a_supplied_byte_interval_is_compared_never_believed` — an agreeing interval is admitted, a wider one is `Q645` with both numbers named | — |
| the element→byte encoding is faithful | — | `range_disjoint_iff_bytes_disjoint`, **both directions** over a nonzero element size |
| an admitted judgement means no shared byte | — | `disjoint_intervals_share_no_byte`, `admitted_memory_disjointness_shares_no_byte` |
| the refusal is not vacuous | `slices_touching_at_one_boundary_are_disjoint_and_overlapping_ones_are_refused` | `overlapping_intervals_share_a_byte` |
| **one generation identity** | `disjointness_across_two_allocation_identities_is_refused_not_assumed` — refused (`Q642`) *even when the byte numbers look disjoint*, so the refusal is about the identity, not the arithmetic | `admitted_disjointness_has_a_ground` (the memory ground requires a common `alloc` **and** a common `gen`), `cross_allocation_disjointness_is_refused`, `stale_generation_is_refused` |
| the decision cannot be bought by operand order | — | `admitted_disjointness_is_symmetric` |

### 3.2 "Overlaps and stale aliases reject; struct fields and bit-fields require distinct explicit layout contracts"

| Obligation | Rust | Lean |
|---|---|---|
| overlap rejects | `slices_touching_at_one_boundary_…` (`Q640`), `a_slice_is_never_disjoint_from_itself_or_from_a_copy_of_itself` (both the same handle twice and two handles over one range) | `nonempty_interval_is_not_disjoint_from_itself` |
| reallocation retires every handle | `reallocation_retires_every_handle_of_the_previous_generation` — two stale operands each report `Q632`, a re-derivation under the new generation is admitted, and deriving under the retired one is refused | `reallocation_retires_every_handle_of_the_old_generation` |
| destruction retires them and is terminal | `a_destroyed_allocation_retires_its_handles_and_admits_nothing_further` (`Q632` then three `Q646` cases: derive, re-destroy, reallocate) | `destruction_retires_every_handle` |
| a generation is never reused or lowered | `a_reallocation_may_not_reuse_or_lower_a_generation` | — |
| retirement is not a blunt instrument | — | `retirement_preserves_other_allocations` |
| a stale handle decides nothing at all | (the `Q632` cases above precede every interval comparison) | `a_stale_handle_decides_nothing`, plus the pair `live_handles_of_one_generation_are_decided` / `the_same_pair_after_a_reallocation_is_refused` |
| handle identities are never reused | `handle_and_allocation_identities_are_never_reused` (`Q634`, including after retirement) | — |
| fields need an explicit layout contract | `a_record_allocation_without_a_layout_contract_is_refused` (`Q636`, and a contract for a *different* record does not serve), `distinct_addressable_fields_are_disjoint_only_under_a_layout_contract` as the positive control | `addressable_fields_with_disjoint_ranges_are_disjoint`; there is no other constructor for a field footprint |
| a contract is total or refused | `an_inconsistent_layout_contract_is_refused_whole` — 11 defects across `Q637`, plus `DuplicateRecord`; `a_refused_layout_contract_refuses_every_derivation_that_names_its_record` | — |
| a field stays inside its element | — | `field_stays_inside_its_element` |
| unknown field, wrong base | `an_unknown_field_is_refused` (`Q638`), `a_field_derivation_needs_a_record_base_of_exactly_one_element` (`Q639`: scalar allocation, two-element base, descriptor base) | — |
| **bit-fields sharing a unit are one memory location** | `adjacent_bitfields_of_one_storage_unit_are_never_disjoint` — bit ranges `0..3` and `3..8` are disjoint and the pair is still `Q641` | `shared_bitfield_unit_is_never_disjoint`, quantified over **every** pair of byte ranges and bit ranges |
| the unit rule is not a disguised overlap | (the refusal names the unit) | `the_bitfield_rule_precedes_the_interval_rule` |
| the positive controls | `bitfields_in_distinct_storage_units_are_disjoint`, `a_bitfield_and_an_addressable_field_of_one_record_are_decided_by_bytes` | `distinct_bitfield_units_are_decided_by_bytes`, `an_addressable_field_shares_no_storage_unit` |
| a bit-field never leaks into the old checkers | `adjacent_bitfields_…` asserts `export() == SharedMemoryLocation { unit: 0 }` | — |

The last row is the fail-closed edge that is easy to miss.
`DerivedFootprint::export` is a three-way `FootprintExport`, not an `Option`:
projecting a bit-field unit onto a plain `AccessFootprint` would let
`check_concurrent` compare two bit-fields of one unit by bytes and call them
unrelated, which is exactly what `Q641` exists to prevent.

### 3.3 "State separately how descriptor aliases sharing a protocol cell differ from memory-slice disjointness"

Stated in the design document §2.6 and enforced:

| Obligation | Rust | Lean |
|---|---|---|
| two descriptors over one cell are never disjoint | `two_descriptors_sharing_a_protocol_cell_are_never_disjoint` (`Q644`) | `descriptors_sharing_a_cell_are_never_disjoint` |
| distinct cells are decided | `two_descriptors_over_distinct_cells_are_disjoint` | `descriptors_over_distinct_cells_are_disjoint` |
| the kinds never mix | `memory_disjointness_does_not_transfer_to_a_protocol_cell` — one program admits two disjoint slices of a buffer, then refuses `(slice, descriptor)` and `(descriptor, slice)` with `Q643` and the descriptor pair with `Q644` | `memory_and_descriptor_are_incomparable`, both argument orders |

The separation in one sentence: a memory slice is a **set of bytes**, and
disjointness of two slices is a statement about that set; a descriptor is a
**handle**, and two descriptors can touch disjoint bytes — or no bytes — while
both advancing one file offset, reference count or connection phase. A proof
about bytes therefore licenses nothing about two descriptors, and the checker
refuses the mixed question rather than answering it.

The cell itself is a **declaration** (`DeclaredProtocolCellIdentity`). This
profile does not discover which descriptors alias; deriving that from source is
follow-on unit F4.

---

## 4. Refusal vocabulary

Eighteen named refusals occupying the free `Q630`–`Q647` range, inside the
`Q6xx` concurrency range of `docs/QCORE_SPEC_r0_3.md` §L21.
`named_refusals_occupy_the_q630_to_q647_range` pins the whole table, asserts
each code is used once, asserts `Display` starts with the code, and asserts the
absence of `Q600`, `Q610` and `Q623`. A repository-wide grep before choosing the
range confirmed `Q600` and `Q610`–`Q623` are the only `Q6xx` codes in use.

Ten shapes are **represented refusals** rather than omissions — union members,
flexible array members, VLAs, computed indices, pointer arithmetic outside a
declared extent, nested anonymous aggregates, type-punned aliases, overlapping
copies, cross-allocation alias proofs and atomic reclamation — each a real
event, so `unsupported_footprint_constructs_are_refused_not_approximated`
measures all ten instead of relying on the model simply lacking a way to say
them.

---

## 5. Certificate and recheck

`FootprintCertificate` carries nine fields;
`recheck_disjoint_footprint_certificate` re-runs the checker and refuses any
field re-derivation does not reproduce
(`recheck_refuses_every_tampered_certificate_field`, 8 mutations, each asserting
the exact field name). `recheck_refuses_a_certificate_whose_program_no_longer_checks`
closes the program direction.

The pinned negative is `a_checked_certificate_never_declares_disjointness`:
`FootprintAssumption::DeclaredDisjointness` is never emitted, does not appear in
`assumption_names()`, and a forged certificate carrying it is refused **before
any field is compared**, so it cannot be bought by otherwise matching. This is
T02's `a_checked_certificate_never_carries_trusted_guard_assumptions` shape.

`a_declared_premise_appears_only_when_its_declaration_is_used` pins that a
validated layout contract no derivation uses adds no premise, and
`an_empty_program_is_accepted_with_no_premise` that the empty trace carries
none.

---

## 6. Lean

New namespace `QCore.DisjointFootprint`: 18 definitions, 28 public theorems and
14 decidable scenarios mirroring the Rust tests. The decision function
`decideDisjoint` / `decideMemory` is written in the same rule order as the Rust
`decide_disjoint`, so each refusal theorem is a statement about one guard.

`AuditDisjointFootprint.lean` registers all 28.
`PATH="$HOME/.elan/bin:$PATH" python3 -B scripts/check_lean_audits.py` passes
**24** audits (was 23). Axiom dependencies across the 28: 21 on `[propext]`, 5
on `[propext, Quot.sound]`, 2 on `[propext, Classical.choice, Quot.sound]`.
`AuditAll.lean`'s whole-project closure still admits only those three.
**No new axiom, no `sorry`, no placeholder**, and `lake build` reports no
warning.

### 6.1 Named Lean boundaries

1. **Source and layout extraction are premises.** No theorem says the Rust
   checker produces this model for any source, or that a `LayoutContract`'s
   numbers are a real target ABI's layout. The Rust/Lean correspondence here is
   *by inspection of the same rule order*, not a mechanical extraction and not
   an F01-style digest-checked transcription.
2. **The model has no allocator, no threads and no memory model.** "Two tasks
   may mutate disjoint slices independently" is not stated: the model has no
   tasks. What is stated is that an admitted judgement means no byte is in both
   footprints.
3. **The store level is one allocation deep.** `resolve`/`assertDisjoint` model
   generation currency and retirement; they do not model derivation, layout
   contracts or descriptor declaration, which live at the Rust level only.
4. **`sharedUnit` is decided from declared unit identity.** That two C members
   really share a storage unit is `DeclaredRecordLayout`, not a theorem.

---

## 7. What this card does not establish

* **`DeclaredFootprintShape` is not discharged.** `check_concurrent` and
  `check_mutex_invariant_program` are byte-identical and still take declared
  intervals. Composition is follow-on F3, and it needs an agreed
  `AllocationId ↔ SharedResourceId` identity rule that this card deliberately
  does not invent.
* **No source surface, no lowering, no CT mapping.** `docs/reevaluation/ct_manifest.json`
  is untouched at **15/50 mapped**. CT07–CT10 and CT42 are *candidates* only
  (§8); mapping any of them requires executed source-stage evidence, which
  requires F1/F4.
* **No cross-allocation reasoning.** Two allocations are `Q642`. This is a real
  precision cost: a partition across two `malloc`ed buffers, which a C
  programmer would call obviously disjoint, is Unknown here. It is the honest
  position for a model with no allocator.
* **No sub-slicing.** Slices derive from an allocation only; the partition laws
  a `slice_of_slice` would need are F1.
* **No real C record is read.** All layout numbers are declared.
* **No T05 composition.** Shares are untouched; a footprint here has no share.
* **No schema record, no wire format, no process provider.** `qcore-schemas`
  is unchanged; F5 specifies the record.
* **No native evidence.** There is nothing to execute: this is a decision
  procedure over declared structure, not a lowering. Unlike T03 and T04, no
  sanitizer run is claimed and none was performed.

---

## 8. Proposed CT candidates — for the integrator, not applied

`docs/reevaluation/ct_manifest.json` is **not edited**. None of these is
proposed as `mapped`: the harness requires executed source-stage evidence and
this card has no source surface. They are recorded so a later card knows what
the prototype already decides at the model level.

| Case | Scenario | What the model already decides | Still needed to map |
|---|---|---|---|
| CT07 | two disjoint array halves mutated independently | `two_disjoint_halves_of_one_allocation_are_admitted`; `two_halves_of_one_allocation_are_admitted` (Lean) | F1 source slices; a Buffer write statement (T04 §7); native execution |
| CT08 | overlapping array ranges | `slices_touching_at_one_boundary_are_disjoint_and_overlapping_ones_are_refused` (`Q640`) | F1 |
| CT09 | distinct ordinary struct fields — accept only after layout/access proof | `distinct_addressable_fields_are_disjoint_only_under_a_layout_contract` with `a_record_allocation_without_a_layout_contract_is_refused` as its refusal half | F1 and F2 (a *real* layout, not a declared one — CT09 says "layout proof") |
| CT10 | adjacent bit-fields in one memory location — reject independent concurrent writes | `adjacent_bitfields_of_one_storage_unit_are_never_disjoint` (`Q641`), with disjoint bit ranges | F1, F2, and a write statement |
| CT42 | two descriptors share an underlying logical state | `two_descriptors_sharing_a_protocol_cell_are_never_disjoint` (`Q644`) and `memory_disjointness_does_not_transfer_to_a_protocol_cell` (`Q643`) | F4 (cell identity derived from source, not declared) |

Proposed ledger text for the integrator, if this card is consolidated:

> Disjoint footprints and alias identity (T09): `DisjointFootprintsV1` derives
> byte intervals from element ranges and decides disjointness only against one
> allocation generation; two allocation identities, a stale generation, a shared
> bit-field storage unit, a shared protocol cell and a memory/descriptor pair
> are each a named refusal in the free `Q630`–`Q647` range. Record fields exist
> only under a whole-record layout contract validated by eleven structural rules
> with no partial admission. Certificates name three declared premises and never
> `DeclaredDisjointness`, which an independent recheck refuses before comparing
> anything. Model-level only: `DeclaredFootprintShape` is **not** discharged,
> no source surface exists, CT coverage stays 15/50 and no raw Cycle 19 unit is
> promoted. Evidence: `docs/reports/remaining-t09/README.md`; design:
> `docs/design/DISJOINT_FOOTPRINT_PROFILE.md`.

---

## 9. Integrator decisions taken here

1. **Cross-allocation disjointness is refused, not granted.** The alternative —
   treating two `AllocationId`s as provably non-overlapping under the existing
   `InitialAuthorityAndUniqueLifetimeIds` premise — would have made CT07-style
   cases easier and is what a reader might expect. It was rejected: unique
   *lifetime identity* is not a non-aliasing proof, and granting it would have
   made the profile's central claim ("disjointness against one underlying
   generation identity") quietly false. `CrossAllocationAliasProof` is also an
   explicit `Unsupported` construct so the gap is testable.
2. **A new module rather than an extension of `mutex_invariant`.** T02's checker
   is a state machine over a trace; this is a decision procedure over declared
   structure with no state machine at all. Extending it would have meant
   editing a 1,923-line pinned module with 48 pinned tests to add an orthogonal
   concern. The cost of the choice is that composition is deferred to F3.
3. **`Q630`–`Q647` rather than extending `Q6xx` upward from `Q624`.** A
   decade-round gap after T02's `Q623` leaves room for a mutex-invariant
   follow-on to extend its own family without colliding.
4. **`FootprintExport` is a three-way enum, not `Option<AccessFootprint>`.**
   An `Option` would have made "a bit-field has no interval" indistinguishable
   from "a descriptor has no interval", and a future caller could plausibly have
   unwrapped the bit-field case by computing the interval itself.
5. **The layout contract is validated whole, before any event.** A per-field
   check would admit the good fields of a bad record. `DuplicateRecord` refuses
   the second contract rather than merging or last-wins.
6. **`docs/QCORE_SPEC_r0_3.md` §L16 and §L21 are not edited.** The profile has
   no consumer yet, and §L21's table lists ranges by pass; `Q6xx` already names
   the `Concurrency` pass, which is where this module sits. Adding a normative
   paragraph for a profile nothing calls would overstate its status. Recorded as
   a follow-on for whichever card lands F3.

---

## 10. Gate results

Run at this card's tip on Linux, after `git submodule update --init --recursive`
(the submodule was absent in the fresh worktree and the pin gate correctly
refused until it was initialized).

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1176 passed, 0 failed** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **56 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests -p "test_*.py" -t scripts/tests` | **43 tests, OK** |
| `PATH="$HOME/.elan/bin:$PATH" bash -c 'cd proof && lake build'` | success, no warnings |
| `PATH="$HOME/.elan/bin:$PATH" python3 -B scripts/check_lean_audits.py` | **Passed 24 Lean audits** |

Before → after: workspace tests **1141 → 1176** (+35, exactly the new tests,
and 1141 matches the second delegated increment's recorded baseline); Lean
audits **23 → 24**; provider tests **56 → 56**; Python tests **43 → 43**.

One clippy finding was raised and fixed rather than allowed: the certificate
mutation table tripped `clippy::type_complexity` and is now a named
`CertificateMutation` type alias. No `#[allow]` was added anywhere.

### 10.1 Pinned artifacts

* **The four pinned 185-unit Cycle 19 artifacts were not re-measured, and this
  is a claim by construction rather than a measurement.** The pinned corpus
  lives outside the repository (`../fluent-bit`) and is absent from this
  isolated worktree, exactly as recorded for T02 and T04. No artifact file under
  `docs/reports/` was modified.
* The claim by construction: the only mentions of `check_disjoint_footprint_program`
  outside the new module and its tests are the two `crates/qcore-core/src/lib.rs`
  re-export lines. `qcore-core` has no serialization surface, so no serialized
  byte can move, and no existing function body was edited.
* The contract corpus stays at **59** `.qcore` modules, the
  `third_party/libevent` submodule pin is unchanged, and
  `docs/reevaluation/ct_manifest.json` is untouched at **15/50 mapped**
  (35 unmapped).
* No raw Cycle 19 unit is promoted: **2 Safe / 42 provisional Unsafe / 137
  Unknown** plus four data-only units is unchanged. `qcore-schemas` stays
  **5.0.0**; prose revision 0.3, plugin protocol v1, evidence v2 and program
  schema/profile v2 are unchanged. Full language conformance stays false.
* `AGENTS.md`, `CLAUDE.md`, `docs/reevaluation/ct_manifest.json`,
  `docs/reviews/r0_3/acceptance.json` and
  `docs/reviews/r0_3/acceptance_matrix.json` were **not edited**, per the
  delegation brief. The ledger text this card proposes is §8 above.
