-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.VerdictSemigroup

/-!
# A declared contract its own body refutes serves no caller

Card **W45-REFUTED-CONTRACT-WITHDRAWAL** (contract spec §C06.4, gateway
constraint 10g). A declared contract of a function whose body an audit
analyses is checked against that body; when the body refutes it (a refuted
counted-user clause of its own row, or a contradicted custody duty) every call
that would apply it is **held**: rewritten to an undeclared name whose refusal
carries the reason key `contract-refuted-by-body`, of category
`stale-evidence`. The refutation is found only where the defining unit is
analysed, so an audit runs in two phases: a first pass computes a table (the
union of every unit's refutations) and a second applies it to every unit.

The model abstracts one caller as a chain of items: a call to a declared
callee, with the value the call contributes when its declaration is applied,
or any other link. `verdict R items` folds the chain under the refuted set `R`
(`QCore.VerdictSemigroup.fold`). Units are abstract; `refutes u` is what the
first pass finds in unit `u`, and `analyse T u` is the second pass under table
`T`.

## What is proved

* **The held value** (`a_held_call_is_unknown`): a held call is `Unknown`,
  never `Pending` — `stale-evidence` is not a human-resolvable category.
* **A caller's verdict never rests on a refuted specification**
  (`never_rests_on_a_refuted_specification`,
  `the_verdict_ignores_what_a_refuted_contract_claims`): with a call to a
  refuted callee the verdict is neither `Safe` nor `Pending`, and it does not
  depend on what the refuted declaration would have contributed.
* **Withdrawal is monotone: it never raises a verdict towards `Safe`**
  (`withdrawal_never_certifies`): withdrawing more can only take a caller out
  of `Safe` or `Pending`, never into it; a caller left `Safe` or `Pending`
  under the larger set has the same verdict under the smaller.
* **Established and external contracts still serve**
  (`an_unrefuted_contract_still_serves`, `an_external_contract_is_never_in_the_table`).
* **Pinned together with refuted** (`pinned_pending_refuted_unknown_join_unknown`):
  a pin mismatch is `Pending`, a refutation `Unknown`, and their join `Unknown`.
* **Two phases compose** (`two_phases_over_groups_equal_the_whole_run`,
  `the_table_is_independent_of_unit_order`): the table folded over groups of
  units is the table of the whole run, so a grouped second phase equals the
  whole run's; under the premise that the second pass reads its table only by
  membership, any order of the units (any `--jobs`) gives the same results.

## What this is not

That the gateway finds exactly the refutations the rows record, holds exactly
the call sites of the table's names outside their own bodies, and that the
second pass reads the table only by membership are Rust boundaries, pinned by
`crates/qcore-c-gateway/tests/refuted_contract_withdrawal.rs` and
`crates/qcore-audit/src/refuted_contracts.rs`'s tests (trust boundary 28).
-/

namespace QCore.RefutedContractWithdrawal

open QCore.VerdictSemigroup
open QCore.ReasonCategory

/-- One link of a caller's chain. -/
inductive Item where
  /-- A call to the declared callee `f`, contributing `applied` when its
  declaration is applied. -/
  | call (f : Nat) (applied : V)
  /-- Any other link. -/
  | other (v : V)
  deriving DecidableEq, Repr

/-- The value of a held call: a call held for a refuted declaration. -/
def held : V := (Link.held .staleEvidence).value

/-- The value one item contributes under the refuted set `R`. -/
def Item.value (R : List Nat) : Item → V
  | .call f v => if f ∈ R then held else v
  | .other v => v

/-- A caller's verdict under the refuted set `R`. -/
def verdict (R : List Nat) (items : List Item) : V :=
  fold (items.map (Item.value R))

/-- The held value is `Unknown`. -/
theorem a_held_call_is_unknown : held = .unknown := by
  decide

/-- The held value is never `Pending`: a refutation is the tool's decision,
not a named human act. -/
theorem a_held_call_is_never_pending : held ≠ .pending := by
  decide

/-- With a call to a refuted callee, a caller is neither `Safe` nor
`Pending`. -/
theorem never_rests_on_a_refuted_specification (R : List Nat) (items : List Item)
    (f : Nat) (v : V) (member : Item.call f v ∈ items) (refuted : f ∈ R) :
    2 ≤ (verdict R items).rank := by
  have link : Item.value R (.call f v) ∈ items.map (Item.value R) :=
    List.mem_map_of_mem member
  have below := every_link_is_below_the_fold link
  simp only [Item.value, refuted, ite_true, a_held_call_is_unknown] at below
  exact below

/-- What a refuted declaration would contribute never reaches the verdict. -/
theorem the_verdict_ignores_what_a_refuted_contract_claims (R : List Nat) (f : Nat)
    (refuted : f ∈ R) (v w : V) : Item.value R (.call f v) = Item.value R (.call f w) := by
  simp [Item.value, refuted]

/-- An unrefuted contract is applied exactly as declared. -/
theorem an_unrefuted_contract_still_serves (R : List Nat) (f : Nat) (v : V)
    (standing : f ∉ R) : Item.value R (.call f v) = v := by
  simp [Item.value, standing]

/-- Withdrawal never raises a verdict towards `Safe`: an item's value under a
larger refuted set is either its value under the smaller one or the held
value. -/
theorem item_value_under_more (R R' : List Nat) (sub : ∀ f ∈ R, f ∈ R') (i : Item) :
    Item.value R' i = Item.value R i ∨ Item.value R' i = held := by
  cases i with
  | other v => exact Or.inl rfl
  | call f v =>
    by_cases h' : f ∈ R'
    · exact Or.inr (by simp [Item.value, h'])
    · have h : f ∉ R := fun h => h' (sub f h)
      exact Or.inl (by simp [Item.value, h, h'])

/-- Withdrawal never certifies: a caller `Safe` or `Pending` under a larger
refuted set has exactly that verdict under the smaller one. -/
theorem withdrawal_never_certifies (R R' : List Nat) (sub : ∀ f ∈ R, f ∈ R')
    (items : List Item) (low : (verdict R' items).rank ≤ 1) :
    verdict R items = verdict R' items := by
  have values : ∀ i ∈ items, Item.value R i = Item.value R' i := by
    intro i member
    rcases item_value_under_more R R' sub i with same | isHeld
    · exact same.symm
    · exfalso
      have link : Item.value R' i ∈ items.map (Item.value R') := List.mem_map_of_mem member
      have below : (Item.value R' i).rank ≤ (verdict R' items).rank :=
        every_link_is_below_the_fold link
      rw [isHeld, a_held_call_is_unknown] at below
      have two : V.unknown.rank = 2 := rfl
      omega
  unfold verdict
  congr 1
  exact List.map_congr_left values

/-- A pin mismatch is `Pending`, a refutation `Unknown`, and their join is
`Unknown`. -/
theorem pinned_pending_refuted_unknown_join_unknown :
    (Link.held .contractPinMismatch).value = .pending ∧ held = .unknown ∧
      V.join (Link.held .contractPinMismatch).value held = .unknown := by
  decide

/-! ## The table and the two phases -/

variable {U R : Type}

/-- The first phase's table: every unit's refutations. -/
def table (refutes : U → List Nat) (units : List U) : List Nat :=
  (units.map refutes).flatten

/-- The whole run: every unit analysed under the whole run's table. -/
def whole (refutes : U → List Nat) (analyse : List Nat → U → R) (units : List U) : List R :=
  units.map (analyse (table refutes units))

/-- Two phases over groups of units: each group's table, their union, then
each group analysed under the union. -/
def grouped (refutes : U → List Nat) (analyse : List Nat → U → R) (groups : List (List U)) :
    List (List R) :=
  let applied := (groups.map (table refutes)).flatten
  groups.map (fun group => group.map (analyse applied))

/-- The table of the groups' tables is the table of the whole run. -/
theorem the_union_of_group_tables_is_the_whole_table (refutes : U → List Nat)
    (groups : List (List U)) :
    (groups.map (table refutes)).flatten = table refutes groups.flatten := by
  unfold table
  induction groups with
  | nil => simp
  | cons group groups ih => simp [List.flatten_cons, List.map_append, ih]

/-- A grouped two-phase run, concatenated, is the whole run. -/
theorem two_phases_over_groups_equal_the_whole_run (refutes : U → List Nat)
    (analyse : List Nat → U → R) (groups : List (List U)) :
    (grouped refutes analyse groups).flatten = whole refutes analyse groups.flatten := by
  unfold grouped whole
  rw [the_union_of_group_tables_is_the_whole_table]
  induction groups with
  | nil => simp
  | cons group groups ih => simp [List.flatten_cons, List.map_append]

/-- A refutation is in the table exactly when some unit found it, whatever the
order of the units. -/
theorem mem_table_iff (refutes : U → List Nat) (units : List U) (f : Nat) :
    f ∈ table refutes units ↔ ∃ u ∈ units, f ∈ refutes u := by
  unfold table
  simp only [List.mem_flatten, List.mem_map]
  constructor
  · rintro ⟨l, ⟨u, member, rfl⟩, found⟩; exact ⟨u, member, found⟩
  · rintro ⟨u, member, found⟩; exact ⟨refutes u, ⟨u, member, rfl⟩, found⟩

/-- The table is independent of the order units are analysed or folded in:
under the premise that the second pass reads its table only by membership,
two orders of the same units give the same result for every unit. -/
theorem the_table_is_independent_of_unit_order (refutes : U → List Nat)
    (analyse : List Nat → U → R)
    (byMembership : ∀ T T' u, (∀ f, f ∈ T ↔ f ∈ T') → analyse T u = analyse T' u)
    (units units' : List U) (same : ∀ u, u ∈ units ↔ u ∈ units') (u : U) :
    analyse (table refutes units) u = analyse (table refutes units') u := by
  apply byMembership
  intro f
  rw [mem_table_iff, mem_table_iff]
  constructor
  · rintro ⟨v, member, found⟩; exact ⟨v, (same v).mp member, found⟩
  · rintro ⟨v, member, found⟩; exact ⟨v, (same v).mpr member, found⟩

/-- Externals are unaffected: a unit refutes only what it defines, so a name
no unit defines is never in the table, and its calls keep their declared
value. -/
theorem an_external_contract_is_never_in_the_table (refutes : U → List Nat)
    (defines : U → Nat → Prop) (onlyDefined : ∀ u f, f ∈ refutes u → defines u f)
    (units : List U) (f : Nat) (external : ∀ u ∈ units, ¬ defines u f) :
    f ∉ table refutes units := by
  rw [mem_table_iff]
  rintro ⟨u, member, found⟩
  exact external u member (onlyDefined u f found)

/-! ## Non-vacuity -/

/-- A caller certified through a contract its body refutes is `Unknown` once
the contract is withdrawn, and the same caller is `Safe` when it is not. -/
theorem a_refuted_caller_moves_from_safe_to_unknown :
    verdict [] [.call 7 .safe, .other .safe] = .safe ∧
      verdict [7] [.call 7 .safe, .other .safe] = .unknown := by
  decide

/-- A caller with an independent violation stays provisional `Unsafe`. -/
theorem an_independent_violation_stays :
    verdict [7] [.call 7 .safe, .other .provisionalUnsafe] = .provisionalUnsafe := by
  decide

end QCore.RefutedContractWithdrawal
