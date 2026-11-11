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

/-! The index/access-bounds property (card CG16-INDEX-AND-OFFSET-EXTENTS,
contract specification C09.1, gateway rule 11s).

The decisions of `crates/qcore-core/src/index_extent.rs`, transcribed:

* an element access `base[i]` against `N` elements is in bounds exactly when
  `0 ≤ i < N`; a formation `base + i` is valid exactly when `0 ≤ i ≤ N`, so
  the one-past pointer is formed but never accessed; an empty extent admits no
  access; a missing extent is never in bounds;
* a byte access of width `w` at offset `o` in `B` bytes needs `0 ≤ o`,
  `0 < w`, `o + w ≤ B`, and `element_size * index` is checked against the
  target's object-size limit;
* a declaration never overrides a source-established extent;
* a write withdraws every fact naming the written variable;
* the function verdict under the review-hold standing rule, and its
  composition with custody;
* admission of recorded evidence only under an equal identity.

What this is not: a model of C, of Clang's extraction, of the declared external
bodies, or of reachability (the source consumer validates reachability and the
model takes it as the `reachable` flag). Symbols stand for parameter values
unchanged since entry; `env` interprets them. The Rust correspondence is bound by
`elementVectors`, `byteVectors`, `resolveVectors` and `verdictVectors`
(`crates/qcore-core/tests/index_extent.rs`). -/
namespace QCore.IndexExtent

/-- A count: a constant, or a stable symbol (numbered). -/
inductive Count where
  | const (n : Nat)
  | sym (s : Nat)
  deriving DecidableEq, Repr

/-- What is known about an index or an offset. -/
inductive IndexFact where
  | const (i : Int)
  | range (lo : Int) (hi : Count)
  | unknown
  deriving DecidableEq, Repr

inductive Refusal where
  | missingExtent
  | unsupportedRange
  | rangeNotContained
  | symbolMismatch
  | overflow
  | emptyWidth
  | emptyRange
  deriving DecidableEq, Repr

inductive Decision where
  | inBounds
  | outOfBounds
  | undecided (r : Refusal)
  deriving DecidableEq, Repr

def Count.eval (env : Nat → Nat) : Count → Nat
  | .const n => n
  | .sym s => env s

/-- `v` is a value the fact allows. -/
def Describes (env : Nat → Nat) : IndexFact → Int → Prop
  | .const i, v => v = i
  | .range lo hi, v => lo ≤ v ∧ v < (hi.eval env : Int)
  | .unknown, _ => True

/-- `contained` in Rust. -/
def contained (idx : IndexFact) (n : Count) (inclusive : Bool) : Decision :=
  match idx with
  | .unknown => .undecided .unsupportedRange
  | .const i =>
      match n with
      | .const n =>
          if 0 ≤ i ∧ (if inclusive then i ≤ (n : Int) else i < (n : Int)) then .inBounds
          else .outOfBounds
      | .sym _ => .undecided .symbolMismatch
  | .range lo hi =>
      match hi with
      | .const h =>
          if (h : Int) ≤ lo then .undecided .emptyRange
          else if lo < 0 then .undecided .rangeNotContained
          else
            match n with
            | .const n => if h ≤ n then .inBounds else .undecided .rangeNotContained
            | .sym _ => .undecided .symbolMismatch
      | .sym h =>
          if lo < 0 then .undecided .rangeNotContained
          else
            match n with
            | .sym s => if h = s then .inBounds else .undecided .symbolMismatch
            | .const _ => .undecided .symbolMismatch

def elementAccess (count : Option Count) (idx : IndexFact) : Decision :=
  match count with
  | none => .undecided .missingExtent
  | some n => contained idx n false

def formation (count : Option Count) (idx : IndexFact) : Decision :=
  match count with
  | none => .undecided .missingExtent
  | some n => contained idx n true

theorem element_access_sound (env : Nat → Nat) (n : Count) (idx : IndexFact) (v : Int)
    (h : elementAccess (some n) idx = .inBounds) (hv : Describes env idx v) :
    0 ≤ v ∧ v < (n.eval env : Int) := by
  unfold elementAccess contained at h
  cases idx with
  | unknown => simp at h
  | const i =>
      cases n with
      | const n =>
          simp [Describes] at hv
          subst hv
          by_cases hc : 0 ≤ v ∧ v < (n : Int)
          · exact ⟨hc.1, by simpa [Count.eval] using hc.2⟩
          · simp [hc] at h
      | sym _ => simp at h
  | range lo hi =>
      simp [Describes] at hv
      cases hi with
      | const hh =>
          by_cases h1 : (hh : Int) ≤ lo
          · simp [h1] at h
          · by_cases h2 : lo < 0
            · simp [h1, h2] at h
            · cases n with
              | const nn =>
                  by_cases h3 : hh ≤ nn
                  · simp [Count.eval] at hv ⊢
                    omega
                  · simp [h1, h2, h3] at h
              | sym _ => simp [h1, h2] at h
      | sym hs =>
          by_cases h2 : lo < 0
          · simp [h2] at h
          · cases n with
            | sym s =>
                by_cases h3 : hs = s
                · subst h3
                  simp [Count.eval] at hv ⊢
                  omega
                · simp [h2, h3] at h
            | const _ => simp [h2] at h

theorem formation_sound (env : Nat → Nat) (n : Count) (idx : IndexFact) (v : Int)
    (h : formation (some n) idx = .inBounds) (hv : Describes env idx v) :
    0 ≤ v ∧ v ≤ (n.eval env : Int) := by
  unfold formation contained at h
  cases idx with
  | unknown => simp at h
  | const i =>
      cases n with
      | const n =>
          simp [Describes] at hv
          subst hv
          by_cases hc : 0 ≤ v ∧ v ≤ (n : Int)
          · exact ⟨hc.1, by simpa [Count.eval] using hc.2⟩
          · simp [hc] at h
      | sym _ => simp at h
  | range lo hi =>
      simp [Describes] at hv
      cases hi with
      | const hh =>
          by_cases h1 : (hh : Int) ≤ lo
          · simp [h1] at h
          · by_cases h2 : lo < 0
            · simp [h1, h2] at h
            · cases n with
              | const nn =>
                  by_cases h3 : hh ≤ nn
                  · simp [Count.eval] at hv ⊢
                    omega
                  · simp [h1, h2, h3] at h
              | sym _ => simp [h1, h2] at h
      | sym hs =>
          by_cases h2 : lo < 0
          · simp [h2] at h
          · cases n with
            | sym s =>
                by_cases h3 : hs = s
                · subst h3
                  simp [Count.eval] at hv ⊢
                  omega
                · simp [h2, h3] at h
            | const _ => simp [h2] at h

/-- An out-of-bounds decision is a violation for every value the fact allows:
only an exact index is ever called out of bounds. -/
theorem out_of_bounds_sound (env : Nat → Nat) (n : Count) (idx : IndexFact) (v : Int)
    (h : elementAccess (some n) idx = .outOfBounds) (hv : Describes env idx v) :
    ¬ (0 ≤ v ∧ v < (n.eval env : Int)) := by
  unfold elementAccess contained at h
  cases idx with
  | unknown => simp at h
  | const i =>
      cases n with
      | const n =>
          simp [Describes] at hv
          subst hv
          by_cases hc : 0 ≤ v ∧ v < (n : Int)
          · simp [hc] at h
          · simpa [Count.eval] using hc
      | sym _ => simp at h
  | range lo hi =>
      cases hi with
      | const hh =>
          by_cases h1 : (hh : Int) ≤ lo
          · simp [h1] at h
          · by_cases h2 : lo < 0
            · simp [h1, h2] at h
            · cases n with
              | const nn =>
                  by_cases h3 : hh ≤ nn
                  · simp [h1, h2, h3] at h
                  · simp [h1, h2, h3] at h
              | sym _ => simp [h1, h2] at h
      | sym hs =>
          by_cases h2 : lo < 0
          · simp [h2] at h
          · cases n with
            | sym s =>
                by_cases h3 : hs = s
                · simp [h2, h3] at h
                · simp [h2, h3] at h
            | const _ => simp [h2] at h

theorem missing_extent_never_in_bounds (idx : IndexFact) :
    elementAccess none idx ≠ .inBounds ∧ formation none idx ≠ .inBounds := by
  simp [elementAccess, formation]

/-- The one-past pointer is formed, and accessing it is a violation. -/
theorem one_past_formed_never_accessed (n : Nat) :
    formation (some (.const n)) (.const n) = .inBounds ∧
      elementAccess (some (.const n)) (.const n) = .outOfBounds := by
  simp [formation, elementAccess, contained]

/-- An empty extent admits no element access, whatever the index fact. -/
theorem empty_extent_admits_no_access (idx : IndexFact) :
    elementAccess (some (.const 0)) idx ≠ .inBounds := by
  intro h
  have := element_access_sound (fun _ => 0) (.const 0) idx
  cases idx with
  | const i =>
      have hv := this i h rfl
      simp [Count.eval] at hv
      omega
  | range lo hi =>
      unfold elementAccess contained at h
      cases hi with
      | const hh =>
          by_cases h1 : (hh : Int) ≤ lo
          · simp [h1] at h
          · by_cases h2 : lo < 0
            · simp [h1, h2] at h
            · by_cases h3 : hh ≤ 0
              · omega
              · simp [h1, h2, h3] at h
      | sym _ =>
          by_cases h2 : lo < 0
          · simp [h2] at h
          · simp [h2] at h
  | unknown => simp [elementAccess, contained] at h

/-! Bytes and checked arithmetic. -/

def byteAccess (extent : Nat) (offset : Int) (width limit : Nat) : Decision :=
  if width = 0 then .undecided .emptyWidth
  else if offset < 0 then .outOfBounds
  else if offset.toNat + width > limit then .undecided .overflow
  else if offset.toNat + width ≤ extent then .inBounds
  else .outOfBounds

theorem byte_access_sound (extent : Nat) (offset : Int) (width limit : Nat)
    (h : byteAccess extent offset width limit = .inBounds) :
    0 ≤ offset ∧ 0 < width ∧ offset + width ≤ (extent : Int) := by
  unfold byteAccess at h
  by_cases h1 : width = 0
  · simp [h1] at h
  · by_cases h2 : offset < 0
    · simp [h1, h2] at h
    · by_cases h3 : offset.toNat + width > limit
      · simp [h1, h2, h3] at h
      · by_cases h4 : offset.toNat + width ≤ extent
        · refine ⟨by omega, by omega, ?_⟩
          omega
        · simp [h1, h2, h3, h4] at h

def scale (index : Int) (size limit : Nat) : Except Refusal Int :=
  if size = 0 then .error .emptyWidth
  else if (index * size).natAbs > limit then .error .overflow
  else .ok (index * size)

theorem scale_sound (index : Int) (size limit : Nat) (p : Int)
    (h : scale index size limit = .ok p) : p = index * size ∧ p.natAbs ≤ limit ∧ 0 < size := by
  unfold scale at h
  by_cases h1 : size = 0
  · simp [h1] at h
  · by_cases h2 : (index * size).natAbs > limit
    · simp [h1, h2] at h
    · simp [h1, h2] at h
      subst h
      exact ⟨rfl, by omega, by omega⟩

/-! Source facts and declarations. -/

inductive Standing where
  | source
  | resolved
  | held
  deriving DecidableEq, Repr

inductive Resolution where
  | source (n : Nat)
  | declared (c : Count) (s : Standing)
  | conflict (source : Nat) (declared : Count)
  | missing
  deriving DecidableEq, Repr

def resolveExtent : Option Nat → Option (Count × Standing) → Resolution
  | some s, none => .source s
  | some s, some (.const d, _) => if d = s then .source s else .conflict s (.const d)
  | some s, some (d, _) => .conflict s d
  | none, some (d, st) => .declared d st
  | none, none => .missing

def Resolution.inForce : Resolution → Option (Count × Standing)
  | .source n => some (.const n, .source)
  | .conflict n _ => some (.const n, .source)
  | .declared c s => some (c, s)
  | .missing => none

/-- A declaration never enlarges, shrinks or erases a source extent: the count
in force is the source's, with source standing. -/
theorem declared_never_overrides_source (s : Nat) (d : Option (Count × Standing)) :
    (resolveExtent (some s) d).inForce = some (.const s, .source) := by
  cases d with
  | none => rfl
  | some p =>
      obtain ⟨c, st⟩ := p
      cases c with
      | const n =>
          by_cases h : n = s
          · simp [resolveExtent, h, Resolution.inForce]
          · simp [resolveExtent, h, Resolution.inForce]
      | sym _ => rfl

/-- A declaration that differs from the source is reported, never silent. -/
theorem a_larger_claim_is_a_conflict (s d : Nat) (st : Standing) (h : s < d) :
    resolveExtent (some s) (some (.const d, st)) = .conflict s (.const d) := by
  have : d ≠ s := by omega
  simp [resolveExtent, this]

/-! Fact invalidation. -/

/-- A scalar fact about variable `v`. -/
structure Fact where
  var : Nat
  value : IndexFact
  deriving DecidableEq, Repr

def IndexFact.mentions (x : Nat) : IndexFact → Bool
  | .range _ (.sym s) => s == x
  | _ => false

/-- `Facts::write` in Rust (scalar part). -/
def write (x : Nat) (facts : List Fact) : List Fact :=
  facts.filter (fun f => f.var != x && !f.value.mentions x)

theorem write_withdraws_every_mention (x : Nat) (facts : List Fact) (f : Fact)
    (h : f ∈ write x facts) : f.var ≠ x ∧ f.value.mentions x = false := by
  unfold write at h
  rw [List.mem_filter] at h
  have h2 := h.2
  simp at h2
  exact ⟨h2.1, by simpa using h2.2⟩

theorem write_keeps_unrelated (x : Nat) (facts : List Fact) (f : Fact) (hf : f ∈ facts)
    (h1 : f.var ≠ x) (h2 : f.value.mentions x = false) : f ∈ write x facts := by
  unfold write
  rw [List.mem_filter]
  exact ⟨hf, by simp [h1, h2]⟩

/-! The function verdict and the standing rule. -/

inductive Outcome where
  | inBounds
  | violation (reachable : Bool)
  | held (clause : Nat)
  | unresolved (reason : Nat)
  | excluded
  deriving DecidableEq, Repr

inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

def Verdict.rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

def Verdict.join (a b : Verdict) : Verdict := if a.rank ≤ b.rank then b else a

def Outcome.clear : Outcome → Bool
  | .inBounds => true
  | .excluded => true
  | _ => false

def Outcome.isHeld : Outcome → Bool
  | .held _ => true
  | _ => false

/-- The decision table of `bounds_verdict`: a reachable violation, any held
judgement, any other obstacle, and whether the hypothetical judgements are all
clear. -/
def decide4 : Bool → Bool → Bool → Bool → Verdict
  | true, _, _, _ => .«unsafe»
  | false, false, false, _ => .safe
  | false, true, false, true => .pending
  | false, _, _, _ => .unknown

/-- `bounds_verdict` in Rust: each judgement is `(actual, hypothetical)`;
`fnReasons` counts the function-level reasons. -/
def boundsVerdict (js : List (Outcome × Outcome)) (fnReasons : Nat) : Verdict :=
  decide4 (js.any (fun j => j.1 == .violation true)) (js.any (fun j => j.1.isHeld))
    (fnReasons != 0 || js.any (fun j => !j.1.clear && !j.1.isHeld)) (js.all (fun j => j.2.clear))

theorem unsafe_only_for_a_reachable_violation (js : List (Outcome × Outcome)) (r : Nat)
    (h : boundsVerdict js r = .«unsafe») : ∃ j ∈ js, j.1 = .violation true := by
  unfold boundsVerdict at h
  generalize ha : js.any (fun j => j.1 == .violation true) = a at h
  generalize hb : js.any (fun j => j.1.isHeld) = b at h
  generalize hc : (r != 0 || js.any (fun j => !j.1.clear && !j.1.isHeld)) = c at h
  generalize hd : js.all (fun j => j.2.clear) = d at h
  cases a <;> cases b <;> cases c <;> cases d <;> simp [decide4] at h
  all_goals
    obtain ⟨j, hj, he⟩ := List.any_eq_true.mp ha
    exact ⟨j, hj, by simpa using he⟩

theorem safe_only_when_every_obligation_discharges (js : List (Outcome × Outcome)) (r : Nat)
    (h : boundsVerdict js r = .safe) : r = 0 ∧ ∀ j ∈ js, j.1.clear = true := by
  unfold boundsVerdict at h
  generalize ha : js.any (fun j => j.1 == .violation true) = a at h
  generalize hb : js.any (fun j => j.1.isHeld) = b at h
  generalize hc : (r != 0 || js.any (fun j => !j.1.clear && !j.1.isHeld)) = c at h
  generalize hd : js.all (fun j => j.2.clear) = d at h
  cases a <;> cases b <;> cases c <;> cases d <;> simp [decide4] at h
  all_goals
    simp at hb hc
    obtain ⟨hr, hother⟩ := hc
    refine ⟨hr, fun ⟨x, y⟩ hj => ?_⟩
    have a := hb x y hj
    have b := hother x y hj
    cases hcl : x.clear
    · simp [hcl, a] at b
    · rfl

/-- A held declaration discharges nothing: a function relying on one is never
`Safe`. -/
theorem held_never_safe (js : List (Outcome × Outcome)) (r : Nat) (j : Outcome × Outcome)
    (hj : j ∈ js) (hh : j.1.isHeld = true) : boundsVerdict js r ≠ .safe := by
  intro h
  have := (safe_only_when_every_obligation_discharges js r h).2 j hj
  cases hj1 : j.1 <;> simp [hj1, Outcome.clear, Outcome.isHeld] at this hh

/-- A held declaration accuses nothing: with no reachable violation under the
in-force premises, no hypothetical violation makes the verdict `Unsafe`. -/
theorem held_never_accuses (js : List (Outcome × Outcome)) (r : Nat)
    (h : ∀ j ∈ js, j.1 ≠ .violation true) : boundsVerdict js r ≠ .«unsafe» := by
  intro hu
  obtain ⟨j, hj, he⟩ := unsafe_only_for_a_reachable_violation js r hu
  exact h j hj he

/-- `Pending` only when the held declarations are the sole obstacle and, in
force as written, leave no other obstacle. -/
theorem pending_only_as_the_sole_obstacle (js : List (Outcome × Outcome)) (r : Nat)
    (h : boundsVerdict js r = .pending) :
    r = 0 ∧ (∀ j ∈ js, j.1.clear = true ∨ j.1.isHeld = true) ∧ (∀ j ∈ js, j.2.clear = true) := by
  unfold boundsVerdict at h
  generalize ha : js.any (fun j => j.1 == .violation true) = a at h
  generalize hb : js.any (fun j => j.1.isHeld) = b at h
  generalize hc : (r != 0 || js.any (fun j => !j.1.clear && !j.1.isHeld)) = c at h
  generalize hd : js.all (fun j => j.2.clear) = d at h
  cases a <;> cases b <;> cases c <;> cases d <;> simp [decide4] at h
  simp at hc hd
  obtain ⟨hr, hother⟩ := hc
  refine ⟨hr, fun ⟨x, y⟩ hj => ?_, fun ⟨x, y⟩ hj => hd x y hj⟩
  have b := hother x y hj
  cases hcl : x.clear
  · simp [hcl] at b
    exact Or.inr b
  · exact Or.inl rfl

/-- Approval alone cannot hide another obstacle. -/
theorem another_obstacle_keeps_unknown (js : List (Outcome × Outcome)) (r : Nat)
    (j k : Outcome × Outcome) (hj : j ∈ js) (hk : k ∈ js) (hh : j.1.isHeld = true)
    (ho : k.1 = .unresolved 0) (hv : ∀ x ∈ js, x.1 ≠ .violation true) :
    boundsVerdict js r = .unknown := by
  have hno : js.any (fun j => j.1 == .violation true) = false := by
    rw [List.any_eq_false]
    intro x hx
    simpa using hv x hx
  have hheld : js.any (fun j => j.1.isHeld) = true :=
    List.any_eq_true.mpr ⟨j, hj, hh⟩
  have hother : js.any (fun j => !j.1.clear && !j.1.isHeld) = true :=
    List.any_eq_true.mpr ⟨k, hk, by simp [ho, Outcome.clear, Outcome.isHeld]⟩
  unfold boundsVerdict
  rw [hno, hheld, hother]
  cases (js.all (fun j => j.2.clear)) <;> simp [decide4]

/-! Composition with custody and admission. -/

def compose (custodyRequested boundsRequested : Bool) (custody bounds : Option Verdict) : Verdict :=
  if !custodyRequested && !boundsRequested then .unknown
  else
    let row := fun (requested : Bool) (v : Option Verdict) =>
      if requested then v.getD .unknown else .safe
    (row custodyRequested custody).join (row boundsRequested bounds)

theorem custody_safe_bounds_unknown_is_unknown :
    compose true true (some .safe) (some .unknown) = .unknown := by decide

theorem custody_safe_bounds_pending_is_pending :
    compose true true (some .safe) (some .pending) = .pending := by decide

theorem custody_safe_bounds_unsafe_is_unsafe :
    compose true true (some .safe) (some .«unsafe») = .«unsafe» := by decide

/-- A requested bounds row that is absent is `Unknown`, never an omission. -/
theorem a_missing_requested_row_is_unknown (c : Option Verdict) :
    compose true true c none ≠ .safe ∧ compose false true c none = .unknown := by
  cases c with
  | none => decide
  | some v => cases v <;> decide

theorem combined_safe_needs_both (c b : Verdict) (h : compose true true (some c) (some b) = .safe) :
    c = .safe ∧ b = .safe := by
  cases c <;> cases b <;> simp_all [compose, Verdict.join, Verdict.rank]

structure Identity where
  property : Nat
  modelVersion : Nat
  source : Nat
  target : Nat
  contracts : Nat
  deriving DecidableEq, Repr

def admit (recorded fresh : Identity × Nat × Verdict) (property version : Nat) : Bool :=
  recorded.1.property == property && recorded.1.modelVersion == version &&
    recorded.1 == fresh.1 && recorded.2.1 == fresh.2.1 && recorded.2.2 == fresh.2.2

theorem admitted_evidence_matches (recorded fresh : Identity × Nat × Verdict) (p v : Nat)
    (h : admit recorded fresh p v = true) :
    recorded.1 = fresh.1 ∧ recorded.2.1 = fresh.2.1 ∧ recorded.2.2 = fresh.2.2 := by
  simp only [admit, Bool.and_eq_true, beq_iff_eq] at h
  exact ⟨h.1.1.2, h.1.2, h.2⟩

/-! Shared vectors, decided here and by the Rust module. -/

/-- `(count, index, element access, formation)`. -/
def elementVectors : List (Option Count × IndexFact × Decision × Decision) := [
  (some (.const 4), .const 0, .inBounds, .inBounds),
  (some (.const 4), .const 3, .inBounds, .inBounds),
  (some (.const 4), .const 4, .outOfBounds, .inBounds),
  (some (.const 4), .const 7, .outOfBounds, .outOfBounds),
  (some (.const 4), .const (-1), .outOfBounds, .outOfBounds),
  (some (.const 0), .const 0, .outOfBounds, .inBounds),
  (none, .const 0, .undecided .missingExtent, .undecided .missingExtent),
  (some (.const 4), .unknown, .undecided .unsupportedRange, .undecided .unsupportedRange),
  (some (.const 4), .range 0 (.const 4), .inBounds, .inBounds),
  (some (.const 4), .range 0 (.const 5), .undecided .rangeNotContained, .undecided .rangeNotContained),
  (some (.const 4), .range (-1) (.const 4), .undecided .rangeNotContained, .undecided .rangeNotContained),
  (some (.const 4), .range 2 (.const 2), .undecided .emptyRange, .undecided .emptyRange),
  (some (.sym 1), .range 0 (.sym 1), .inBounds, .inBounds),
  (some (.sym 1), .range 0 (.sym 2), .undecided .symbolMismatch, .undecided .symbolMismatch),
  (some (.sym 1), .const 0, .undecided .symbolMismatch, .undecided .symbolMismatch),
  (some (.const 8), .range 0 (.sym 1), .undecided .symbolMismatch, .undecided .symbolMismatch)
]

theorem element_vectors_hold :
    elementVectors.all (fun (n, i, a, f) => elementAccess n i == a && formation n i == f) = true := by
  decide

theorem element_vectors_count : elementVectors.length = 16 := by decide

/-- `(extent, offset, width, limit, decision)`. -/
def byteVectors : List (Nat × Int × Nat × Nat × Decision) := [
  (16, 0, 4, 1000, .inBounds),
  (16, 12, 4, 1000, .inBounds),
  (16, 13, 4, 1000, .outOfBounds),
  (16, -1, 1, 1000, .outOfBounds),
  (16, 0, 0, 1000, .undecided .emptyWidth),
  (16, 999, 4, 1000, .undecided .overflow),
  (0, 0, 1, 1000, .outOfBounds)
]

theorem byte_vectors_hold :
    byteVectors.all (fun (b, o, w, l, d) => byteAccess b o w l == d) = true := by decide

theorem byte_vectors_count : byteVectors.length = 7 := by decide

/-- `(source, declared, count in force)`. -/
def resolveVectors : List (Option Nat × Option (Count × Standing) × Option (Count × Standing)) := [
  (some 4, none, some (.const 4, .source)),
  (some 4, some (.const 8, .resolved), some (.const 4, .source)),
  (some 4, some (.const 4, .held), some (.const 4, .source)),
  (some 4, some (.const 2, .resolved), some (.const 4, .source)),
  (none, some (.const 8, .held), some (.const 8, .held)),
  (none, some (.sym 1, .resolved), some (.sym 1, .resolved)),
  (none, none, none)
]

theorem resolve_vectors_hold :
    resolveVectors.all (fun (s, d, r) => (resolveExtent s d).inForce == r) = true := by decide

theorem resolve_vectors_count : resolveVectors.length = 7 := by decide

/-- `(judgements, function reasons, verdict)`. -/
def verdictVectors : List (List (Outcome × Outcome) × Nat × Verdict) := [
  ([], 0, .safe),
  ([(.inBounds, .inBounds), (.excluded, .excluded)], 0, .safe),
  ([(.inBounds, .inBounds)], 1, .unknown),
  ([(.violation true, .violation true)], 0, .«unsafe»),
  ([(.violation false, .violation false)], 0, .unknown),
  ([(.held 0, .inBounds)], 0, .pending),
  ([(.held 0, .violation true)], 0, .unknown),
  ([(.held 0, .inBounds), (.unresolved 0, .unresolved 0)], 0, .unknown),
  ([(.held 0, .inBounds)], 1, .unknown),
  ([(.held 0, .inBounds), (.violation true, .violation true)], 0, .«unsafe»),
  ([(.held 0, .unresolved 1)], 0, .unknown)
]

theorem verdict_vectors_hold :
    verdictVectors.all (fun (js, r, v) => boundsVerdict js r == v) = true := by decide

theorem verdict_vectors_count : verdictVectors.length = 11 := by decide

end QCore.IndexExtent
