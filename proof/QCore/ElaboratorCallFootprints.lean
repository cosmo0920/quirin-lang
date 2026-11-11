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

import QCore.ElaborationCore
import QCore.NestedCallEffects

/-!
# The elaborator's call events with their contract footprints
(card CG05-ELABORATOR-CALL-FOOTPRINTS, gateway rule 11n)

The abstract rule implemented by `crates/qcore-c-gateway/src/elaboration.rs`
(`event_footprint`, `Context::call_footprint`, `EvalGraph::check_with_footprints`)
and the route-G adapter `legacy_elaborator_calls.rs`.

Stated here:

* **Event footprints** (`eventFootprint`): every elaborated event other than a
  call carries the footprint the Rust `event_footprint` gives it, in
  `QCore.NestedCallEffects`'s cells; a call carries the footprint the route
  read from its contract, or none.
* **The decision on a call pair** (`callPair`): the pair commutes exactly when
  both events carry a footprint and the footprints commute. A call with no
  footprint is refused, exactly as `QCore.ElaborationCore.commutes` refuses
  every call (`a_footprintless_call_is_refused`,
  `empty_map_is_check_with`).
* **The admission** (`a_scalar_destination_read_commutes_with_a_call`): the
  read of a scalar local whose address is never taken commutes with every call
  footprint that does not write that local's own storage; the read then
  observes one value in both orders (`a_disjoint_read_observes_alike`), so a
  compound assignment `x op= f(…)` reaches one state in both orders
  (`the_compound_assignment_runs_alike`).
* **The refusals**: `*q op= f(q)` with `f` writing `q`'s identity
  (`a_read_through_the_written_identity_is_refused`), a destination whose
  address is taken against any identity write
  (`an_address_taken_destination_is_refused`), two identity writes
  (`two_identity_writes_are_refused`).

The rest is `QCore.NestedCallEffects`: `footprint_commute_sound` (two effects
whose footprints commute commute on every state) and
`every_interleaving_runs_alike`. What this is not: a model of Clang, of the
contract clauses, or a proof that two names are distinct identities. The Rust
correspondence is bound by `footprintVectors`
(`tests/unit/elaborator_call_footprints.rs`).
-/

namespace QCore.ElaboratorCallFootprints

open QCore.NestedCallEffects

/-- An elaborated event as the order check sees it: a named object's read
(`scalar`: its value is a scalar) or write (`pointer`: a pointer value is
stored), a path rooted at an object (`indirect`: it leaves the object through
a pointer), a call with the footprint the route read (none where it declined),
or a control event. Declarations and identity names are numbered alike. -/
inductive EEv where
  | readObject (d : Nat) (scalar : Bool)
  | writeObject (d : Nat) (pointer : Bool)
  | readPath (root : Nat) (indirect scalar : Bool)
  | writePath (root : Nat) (indirect pointer : Bool)
  | call (fp : Option Footprint)
  | control

def isCall : EEv → Bool
  | .call _ => true
  | _ => false

/-- `event_footprint` in Rust, and the call's own footprint. -/
def eventFootprint : EEv → Option Footprint
  | .readObject d scalar => some ⟨.object d :: (if scalar then [] else [.identity d]), []⟩
  | .writeObject d pointer => some ⟨[], .object d :: (if pointer then [.identity d] else [])⟩
  | .readPath r indirect scalar =>
      some ⟨.object r :: (if indirect || !scalar then [.identity r] else []), []⟩
  | .writePath r indirect pointer =>
      some ⟨[], .object r :: (if indirect || pointer then [.identity r] else [])⟩
  | .call fp => fp
  | .control => some ⟨[], []⟩

/-- The decision `check_with_footprints` takes on an unordered pair that
`commutes` leaves as a call pair. -/
def callPair (u : Nat → Bool) (a b : EEv) : Bool :=
  match eventFootprint a, eventFootprint b with
  | some f, some g => fpCommute u f g
  | _, _ => false

/-- A call with no footprint refuses every pair it is in. -/
theorem a_footprintless_call_is_refused (u : Nat → Bool) (e : EEv) :
    callPair u (.call none) e = false ∧ callPair u e (.call none) = false := by
  constructor
  · simp [callPair, eventFootprint]
  · unfold callPair
    cases eventFootprint e <;> simp [eventFootprint]

/-- With no call footprint in the map, every call pair is refused, which is
the pre-card decision (`QCore.ElaborationCore.call_never_commutes`): the
check is CG18's `check_with`. -/
theorem empty_map_is_check_with (u : Nat → Bool) (e : EEv)
    (x : QCore.ElaborationCore.Ev) :
    callPair u (.call none) e = QCore.ElaborationCore.commutes .call x := by
  rw [(a_footprintless_call_is_refused u e).1,
    (QCore.ElaborationCore.call_never_commutes x).1]

theorem callPair_symm (u : Nat → Bool) (a b : EEv) : callPair u a b = callPair u b a := by
  unfold callPair
  cases eventFootprint a <;> cases eventFootprint b <;> simp [fpCommute, Bool.and_comm]

/-- No cell a footprint writes is the local's own storage. -/
def sparesObject (d : Nat) (f : Footprint) : Prop := ∀ w ∈ f.writes, w ≠ .object d

/-- **The admission.** The read of a scalar local whose address is never taken
commutes with every call footprint that does not write the local's own
storage: identities never reach it, fresh identities are no other cell, and
other objects are distinct. -/
theorem a_scalar_destination_read_commutes_with_a_call (u : Nat → Bool) (d : Nat)
    (hu : u d = true) (f : Footprint) (hf : sparesObject d f) :
    callPair u (.readObject d true) (.call (some f)) = true := by
  have hc : clash u f ⟨[.object d], []⟩ = false := by
    unfold clash
    rw [List.any_eq_false]
    intro w hw
    have hne := hf w hw
    cases w with
    | object y =>
      have : y ≠ d := fun h => hne (by rw [h])
      simp [touches, overlap, this]
    | identity n => simp [touches, overlap, hu]
    | fresh k => simp [touches, overlap]
  show fpCommute u ⟨[.object d], []⟩ f = true
  unfold fpCommute
  rw [hc]
  simp [clash]

/-- Under a sound overlap, a call effect respecting a footprint that commutes
with a read's footprint leaves every location the read names unchanged: the
read observes one value before and after the call. -/
theorem a_disjoint_read_observes_alike {L V : Type} (u : Nat → Bool) (ρ : Den L)
    (hρ : Sound u ρ) (R G : Footprint) (g : (L → V) → (L → V))
    (hg : Respects ρ G g) (h : fpCommute u R G = true) (s : L → V) :
    ∀ l, inCells ρ R.reads l → g s l = s l := by
  intro l hl
  have hGR : clash u G R = false := by
    unfold fpCommute at h
    cases hc : clash u G R <;> simp_all
  apply hg.frame s l
  intro hw
  apply disjoint_of_no_clash hρ hGR l hw
  obtain ⟨c, hc, hcl⟩ := hl
  exact ⟨c, List.mem_append_left _ hc, hcl⟩

/-- **The compound assignment runs alike.** `x op= f(…)`: the read of `x` and
the call are unordered, the write of `x` follows both. When the call leaves
`x`'s location unchanged, both orders write the same value over the same
state. -/
theorem the_compound_assignment_runs_alike {L V : Type} [DecidableEq L] (x : L)
    (op : V → V → V) (result : V) (g : (L → V) → (L → V)) (s : L → V)
    (hx : g s x = s x) :
    -- read first, then the call
    (fun l => if l = x then op (s x) result else g s l) =
    -- the call first, then the read
    (fun l => if l = x then op (g s x) result else g s l) := by
  rw [hx]

/-- `*q op= f(q)` with `f` writing `q`'s identity: the destination read
through `q` touches that identity, and is refused. -/
theorem a_read_through_the_written_identity_is_refused (u : Nat → Bool) (q : Nat) :
    callPair u (.readPath q true true) (.call (some ⟨[], [.identity q]⟩)) = false := by
  simp [callPair, eventFootprint, fpCommute, clash, touches, overlap]

/-- A destination whose address is taken may be what an identity denotes. -/
theorem an_address_taken_destination_is_refused (u : Nat → Bool) (d n : Nat)
    (hu : u d = false) :
    callPair u (.readObject d true) (.call (some ⟨[], [.identity n]⟩)) = false := by
  simp [callPair, eventFootprint, fpCommute, clash, touches, overlap, hu]

/-- Two calls writing identities are refused: names may alias. -/
theorem two_identity_writes_are_refused (u : Nat → Bool) (p q : Nat) :
    callPair u (.call (some ⟨[], [.identity p]⟩)) (.call (some ⟨[], [.identity q]⟩)) =
      false := by
  simp [callPair, eventFootprint, fpCommute, clash, touches, overlap]

/-! ## The address boundary (CG05-FOOTPRINT-ADDRESS-CONSERVATISM)

`fprintf(stderr, …, f(&q))`: the read of the file-scope pointer `stderr`
reads its storage and the identity it denotes; `f(&q)` may write the
automatic `q`, whose address is taken. The refusal is kept as a measured
boundary: no flow fact proves the identity distinct from `q` (a `Borrowed`
callee may retain an address), and no premise is taken. The theorems below
say the refusal is necessary without one: a sound denotation exists in which
the identity is `q`'s storage, and there the two orders observe different
values. -/

/-- The file-scope pointer's identity `n` is the address-taken local `d`'s
storage: one location both cells name. -/
def aliasedDen (d n : Nat) : Den Unit := fun c _ => c = .object d ∨ c = .identity n

theorem aliased_den_sound (u : Nat → Bool) (d n : Nat) (hu : u d = false) :
    Sound u (aliasedDen d n) := by
  intro a b _ ha hb
  rcases ha with rfl | rfl <;> rcases hb with rfl | rfl <;> simp [overlap, hu]

/-- The denotation meets the premises `overlap_sound` rests on: the local's
address is taken, so the unaliased premise says nothing about it. -/
theorem aliased_den_meets_the_premises (u : Nat → Bool) (d n : Nat) (hu : u d = false) :
    Premises u (aliasedDen d n) :=
  { objects := by
      intro a b _ ha hb
      rcases ha with h | h <;> rcases hb with h' | h' <;> simp_all
    fresh := by
      intro k c _ hk _
      rcases hk with h | h <;> cases h
    unaliasedIdentity := by
      intro d' _ _ hu' _ _
      have : d' ≠ d := fun h => by simp [h, hu] at hu'
      simp_all [aliasedDen] }

/-- The call writing the local respects its footprint in that denotation. -/
theorem a_local_write_respects_its_footprint (d n : Nat) (v : Int) :
    Respects (aliasedDen d n) ⟨[], [.object d]⟩ (store v) :=
  { frame := by
      intro _ _ h
      exact absurd ⟨.object d, by simp, Or.inl rfl⟩ h
    local_ := by
      intro _ _ _ _ _
      rfl }

/-- **The address boundary is necessary.** The pair is refused, and in a sound
denotation meeting the premises the read of the identity observes the old value
in one order and the call's value in the other. -/
theorem the_address_boundary_is_necessary (u : Nat → Bool) (d n : Nat) (hu : u d = false)
    (v : Int) (s : Unit → Int) (hv : s () ≠ v) :
    fpCommute u ⟨[.object (d + n + 1), .identity n], []⟩ ⟨[], [.object d]⟩ = false ∧
      s () ≠ store v s () := by
  refine ⟨?_, hv⟩
  simp [fpCommute, clash, touches, overlap, hu]

/-! ## Shared vectors -/

/-- The fixture contracts' footprints: `r_release_code(p)` writes `p`'s
identity; `r_borrow_code(p)` reads it; `q_alloc()` writes a fresh identity. -/
def releaseP : Footprint := ⟨[], [.identity 0]⟩
def borrowP : Footprint := ⟨[.identity 0], []⟩
def allocFresh : Footprint := ⟨[], [.fresh 9]⟩

/-- `FOOTPRINT_VECTORS` of `elaboration.rs`: two unordered events and the
decision `check_with_footprints` takes, over declaration `0` (unaliased) and
`1` (address taken), `u0`. -/
def footprintVectors : List (EEv × EEv × Bool) := [
  (.readObject 0 true, .call (some releaseP), true),
  (.writeObject 0 false, .call (some releaseP), true),
  (.readObject 1 true, .call (some releaseP), false),
  (.readObject 1 true, .call (some borrowP), true),
  (.readObject 0 false, .call (some releaseP), false),
  (.readPath 0 true true, .call (some releaseP), false),
  (.readPath 0 true true, .call (some borrowP), true),
  (.readPath 0 false true, .call (some releaseP), true),
  (.readPath 1 false true, .call (some releaseP), false),
  (.writePath 0 true false, .call (some borrowP), false),
  (.readObject 0 true, .call none, false),
  (.call (some borrowP), .call (some borrowP), true),
  (.call (some releaseP), .call (some borrowP), false),
  (.call (some allocFresh), .call (some borrowP), true),
  (.call (some allocFresh), .call none, false),
  (.control, .call (some releaseP), true)
]

theorem footprint_vectors_hold :
    footprintVectors.all (fun (a, b, r) => callPair u0 a b == r && callPair u0 b a == r) =
      true := by
  decide

theorem footprint_vectors_count : footprintVectors.length = 16 := by decide

end QCore.ElaboratorCallFootprints
