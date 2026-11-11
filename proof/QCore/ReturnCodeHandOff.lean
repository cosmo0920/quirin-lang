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

/-!
# A hand-off by return code (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF)

The model of gateway rule 12m's guarded hand-off and its caller branch
check, and of rule 13l's surface (contract specification C04.19.5, grammar
G05d). Abstract rule theorems, not an extraction of the Rust analyzer
(`crates/qcore-schemas/src/handoff.rs`: `owner_after`;
`crates/qcore-c-gateway/src/handoff_branch.rs`: `forward`, `check_path`;
`crates/qcore-contract/src/contract_handoff.rs`,
`contract_constructed_state.rs`).

USER DECISIONS 2026-10-15: `quant live #rko: #owner |-> #callback when
#result != @RD_KAFKA_OP_RES_PASS;` (or `==`, or a module alias `#PASS`); a
conditional hand-off is admitted only when the caller's body branches on the
same result with the same constant and does not touch the object where the
callback took it; aliases are a second spelling of one `@` name; `|-> absent`
becomes `|-> #absent`, the bare word rejecting.

* **Ownership is a function of the result and the guard.** After the call
  the object is the callback's exactly when the guard holds of the result
  (`owner_after_is_the_guard`, `owner_after_eq`, `owner_after_ne`).
* **An admitted branch agrees.** The caller's reading splits the result into
  the constants it compares it with and the rest; a branch is admitted only
  when no class the guard admits names the object again; then no result the
  guard admits is touched (`an_admitted_branch_never_touches_a_taken_object`),
  and a value the guard does not admit is never constrained
  (`the_caller_keeps_what_the_guard_leaves`). Following the result through a
  forwarder composes (`admission_composes_through_a_forwarder`).
* **A guard the caller does not branch on never admits**
  (`an_unbranched_guard_never_admits`): the same constant must be compared.
* **Alias canonicalisation is identity on meaning**
  (`an_alias_means_its_name`, `an_undeclared_alias_rejects`).
* **The `#absent` migration preserves meaning**
  (`the_migration_preserves_absent`, `a_bare_absent_rejects`,
  `a_function_designator_is_unchanged`).
* **Vectors.** `owner_vectors_hold` (Rust `OWNER_VECTORS`),
  `branch_vectors_hold` (Rust `BRANCH_VECTORS`).
-/

namespace QCore.ReturnCodeHandOff

/-- How a guard compares the result with its constant. -/
inductive GuardOp where
  | eq
  | ne
  deriving DecidableEq, Repr

/-- `when #result OP @K`, the constant resolved to its value. -/
structure Guard where
  op : GuardOp
  value : Int
  deriving DecidableEq, Repr

/-- Whether the guard holds of a result. -/
def holds (g : Guard) (r : Int) : Bool :=
  match g.op with
  | .eq => r == g.value
  | .ne => r != g.value

/-- Who owns the handed object after the call. -/
inductive Owner where
  | caller
  | callback
  deriving DecidableEq, Repr

/-- Rust `owner_after`. -/
def ownerAfter (g : Guard) (r : Int) : Owner :=
  if holds g r then .callback else .caller

theorem owner_after_is_the_guard (g : Guard) (r : Int) :
    ownerAfter g r = .callback ↔ holds g r = true := by
  unfold ownerAfter
  cases h : holds g r <;> simp

theorem owner_after_eq (k r : Int) :
    ownerAfter ⟨.eq, k⟩ r = if r = k then .callback else .caller := by
  unfold ownerAfter holds
  by_cases h : r = k <;> simp [h]

theorem owner_after_ne (k r : Int) :
    ownerAfter ⟨.ne, k⟩ r = if r = k then .caller else .callback := by
  unfold ownerAfter holds
  by_cases h : r = k <;> simp [h]

/-! ### The caller's reading and its admission -/

/-- One caller's reading of the handed object after the call: whether each
compared constant's class names it again, whether the rest does, and whether
storing the call's result into the variable the body tests converted it
(`result_converted`). -/
structure Reading where
  compared : List (Int × Bool)
  rest : Bool
  resultConverted : Bool := false
  deriving Repr

/-- Whether a result value touches the object under the reading. -/
def touches (rd : Reading) (r : Int) : Bool :=
  match rd.compared.find? (fun p => p.1 == r) with
  | some p => p.2
  | none => rd.rest

/-- Whether some value the guard admits lies outside the compared ones. -/
def meetsRest (g : Guard) (rd : Reading) : Bool :=
  match g.op with
  | .eq => !(rd.compared.any fun p => p.1 == g.value)
  | .ne => true

/-- Rust `forward`'s admission: no class the guard admits touches. -/
def admit (g : Guard) (rd : Reading) : Bool :=
  (rd.compared.all fun p => !(holds g p.1 && p.2)) && !(meetsRest g rd && rd.rest)

/-- The user's same-constant requirement: the guard's constant is compared. -/
def branchesOn (g : Guard) (rd : Reading) : Bool :=
  rd.compared.any fun p => p.1 == g.value

/-- The admission rule 12m reads (`check_path` and `direct_admission`): the
result is not stored through a conversion, no admitted class touches, and the
same constant is compared. -/
def admitted (g : Guard) (rd : Reading) : Bool :=
  !rd.resultConverted && admit g rd && branchesOn g rd

theorem find_some_mem {l : List (Int × Bool)} {r : Int} {p : Int × Bool}
    (h : l.find? (fun q => q.1 == r) = some p) : p ∈ l ∧ p.1 = r := by
  constructor
  · exact List.mem_of_find?_eq_some h
  · have := List.find?_some h
    simpa using this

theorem find_none_not_mem {l : List (Int × Bool)} {r : Int}
    (h : l.find? (fun q => q.1 == r) = none) : ∀ p ∈ l, p.1 ≠ r := by
  intro p hp heq
  have := List.find?_eq_none.mp h p hp
  simp [heq] at this

theorem an_admitted_branch_never_touches_a_taken_object (g : Guard) (rd : Reading)
    (r : Int) (ha : admit g rd = true) (hg : holds g r = true) : touches rd r = false := by
  unfold admit at ha
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at ha
  obtain ⟨hall, hrest⟩ := ha
  unfold touches
  cases hf : rd.compared.find? (fun p => p.1 == r) with
  | some p =>
    obtain ⟨hmem, hfst⟩ := find_some_mem hf
    have := List.all_eq_true.mp hall p hmem
    simp only [Bool.not_eq_true', Bool.and_eq_false_iff] at this
    rcases this with h | h
    · rw [hfst] at h; rw [hg] at h; exact absurd h (by decide)
    · simpa using h
  | none =>
    have hnot := find_none_not_mem hf
    simp only [Bool.and_eq_false_iff] at hrest
    rcases hrest with h | h
    · -- the guard meets no rest class: it is `==` with a compared constant
      unfold meetsRest at h
      cases hop : g.op with
      | ne => rw [hop] at h; simp at h
      | eq =>
        rw [hop] at h
        simp only [Bool.not_eq_false', List.any_eq_true, beq_iff_eq] at h
        obtain ⟨p, hp, hpv⟩ := h
        unfold holds at hg
        rw [hop] at hg
        simp only [beq_iff_eq] at hg
        exact absurd (hpv.trans hg.symm) (hnot p hp)
    · exact h

/-- A value the guard does not admit is never constrained by the admission:
the caller keeps or disposes of the object there as its reading says. -/
theorem the_caller_keeps_what_the_guard_leaves (g : Guard) (r : Int)
    (hg : holds g r = false) : ownerAfter g r = .caller := by
  unfold ownerAfter; simp [hg]

theorem an_unbranched_guard_never_admits (g : Guard) (rd : Reading)
    (h : branchesOn g rd = false) : admitted g rd = false := by
  unfold admitted; simp [h]

/-- A site that stores the result through a conversion (`result_converted`)
never admits (`check_path` and `direct_admission`). -/
theorem a_converted_result_never_admits (g : Guard) (rd : Reading)
    (h : rd.resultConverted = true) : admitted g rd = false := by
  unfold admitted; simp [h]

/-- Any admitted site stores the result without conversion. -/
theorem an_admitted_branch_never_converts_the_result (g : Guard) (rd : Reading)
    (h : admitted g rd = true) : rd.resultConverted = false := by
  unfold admitted at h
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at h
  exact h.1.1

/-- A forwarder returns the result unchanged where it does not touch: the
caller's reading of the forwarded value composes with the forwarder's. -/
theorem admission_composes_through_a_forwarder (g : Guard) (inner outer : Reading)
    (r : Int) (hi : admit g inner = true) (ho : admit g outer = true)
    (hg : holds g r = true) : touches inner r = false ∧ touches outer r = false :=
  ⟨an_admitted_branch_never_touches_a_taken_object g inner r hi hg,
   an_admitted_branch_never_touches_a_taken_object g outer r ho hg⟩

/-! ### The surface: aliases and the `#absent` migration -/

/-- A guard's constant as written: an `@` name or a module alias. -/
inductive Spelling where
  | at (name : String)
  | alias (name : String)
  deriving DecidableEq, Repr

/-- The canonical C name a spelling loads to (`none`: the module rejects). -/
def canonical (aliases : List (String × String)) : Spelling → Option String
  | .at name => some name
  | .alias name => (aliases.find? fun p => p.1 == name).map (·.2)

theorem an_alias_means_its_name (aliases : List (String × String)) (a n : String)
    (h : aliases.find? (fun p => p.1 == a) = some (a, n)) :
    canonical aliases (.alias a) = canonical aliases (.at n) := by
  simp [canonical, h]

theorem an_undeclared_alias_rejects (aliases : List (String × String)) (a : String)
    (h : aliases.find? (fun p => p.1 == a) = none) : canonical aliases (.alias a) = none := by
  simp [canonical, h]

/-- A member's value. -/
inductive MemberValue where
  | absent
  | function (name : String)
  deriving DecidableEq, Repr

/-- The right of `|->` as written. -/
inductive Right where
  | keyword (word : String)
  | bare (word : String)
  deriving DecidableEq, Repr

/-- Before the card: `absent` was a bare reserved word. -/
def readBefore : String → Option MemberValue
  | "absent" => some .absent
  | name => some (.function name)

/-- After the card: `#absent` is the keyword; a bare `absent` rejects. -/
def readAfter : Right → Option MemberValue
  | .keyword "absent" => some .absent
  | .keyword _ => none
  | .bare "absent" => none
  | .bare name => some (.function name)

theorem the_migration_preserves_absent :
    readAfter (.keyword "absent") = readBefore "absent" := rfl

theorem a_bare_absent_rejects : readAfter (.bare "absent") = none := rfl

theorem a_function_designator_is_unchanged (name : String) (h : name ≠ "absent") :
    readAfter (.bare name) = readBefore name := by
  unfold readAfter readBefore
  split <;> simp_all

/-! ### Vectors shared with the Rust tests -/

/-- Rust `OWNER_VECTORS`: (op is `!=`, result, constant value, the callback
owns). -/
def ownerVectors : List (Bool × Int × Int × Bool) := [
  (true, 0, 0, false),
  (true, 1, 0, true),
  (true, 2, 0, true),
  (true, 3, 0, true),
  (false, 2, 2, true),
  (false, 1, 2, false),
  (false, 0, 2, false),
  (false, 3, 2, false)
]

theorem owner_vectors_hold :
    ownerVectors.all (fun (ne, r, k, cb) =>
      (ownerAfter ⟨if ne then .ne else .eq, k⟩ r == .callback) == cb) = true := by
  decide

/-- Rust `BRANCH_VECTORS`: (guard is `!=`, guard value, compared values with
whether each touches, whether the rest touches, admitted). -/
def branchVectors : List (Bool × Int × List (Int × Bool) × Bool × Bool) := [
  (true, 0, [(0, false)], false, true),
  (true, 0, [(1, false), (2, false), (3, false)], true, false),
  (false, 2, [(2, false), (3, false)], true, true),
  (false, 2, [(2, true)], false, false),
  (false, 2, [], true, false),
  (false, 2, [], false, true)
]

theorem branch_vectors_hold :
    branchVectors.all (fun (ne, k, compared, rest, ok) =>
      admit ⟨if ne then .ne else .eq, k⟩ { compared, rest } == ok) = true := by
  decide

end QCore.ReturnCodeHandOff
