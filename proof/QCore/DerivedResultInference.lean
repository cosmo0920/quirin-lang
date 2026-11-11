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

import QCore.DerivedResult

/-!
# Inferred derived-result summaries

Card **W43-RESULT-ALIAS-RETURNED**, discharging `W43-RESULT-ALIAS-RETURNED`.
Card W43-LIBC-RESULT-ALIAS (`QCore.DerivedResult`) gave the vocabulary a
derived result and refused every escape of a derived pointer. This module models
the rule that lets a body whose every exit returns NULL or a pointer derived
from one borrowed formal publish the summary a declaration would have stated
(`crates/qcore-c-gateway/src/legacy_derived_summary.rs`), and the gate's
derived arm that validates it (`result_matches` in `legacy_summary.rs`).

## The model

An exit's returned value is what `observe_exit` reports: nothing tracked, a
null pointer constant, an identity site, a derived binding over one site (with
its extent and its *relative* nullability and dropped `const`), or an ambiguous
binding. Each exit also reports whether a custody effect reached the base's
**name-only** site (`derived_base_touched`). A candidate summary is a `Spec`:
extent, nullability and `const_dropped`.

Both sides are given a concrete meaning. A pointer value is null or a pair
(object, offset). A `Spec` *admits* a result `r` over a base value `b` when `r`
is null and either the base is null or the spec is nullable, or the spec is
exact and `r = b`, or the spec is interior and `r` points into `b`'s object.
That is the meaning a caller applies (`derive_pointer`: the result may be null
when its base may be, or when the step may be). An exit observation *denotes*
the results the analysis claims for it: a null constant is null; an exact
derived binding is `b` itself, or null if a step may be; an interior one points
into `b`'s object, or is null if a step may be or the base is.

## What is proved

* **The gate is sound against the contract meaning**
  (`an_accepted_exit_is_admitted`, `validation_is_sound`): every concrete
  result an accepted exit denotes is admitted by the spec, for every base value.
* **The proposal validates** (`the_proposal_validates`), and it is **tight**:
  exact exactly when no derived exit is interior
  (`the_proposal_is_exact_iff_every_exit_is`).
* **Every refused shape the card names is refused by the gate**, for every
  spec: a mixed exit (`a_mixed_exit_refuses`), a pointer derived from another
  site (`another_base_refuses`), a touched base (`a_touched_base_refuses`); and
  the proposal declines each of them and an all-null body
  (`the_proposal_declines_a_mixed_exit`, `an_all_null_body_proposes_nothing`).
* **The name-only base stays name-only under borrowing**: deriving, using and
  null-testing leave a status-free site status-free, and a release reaching it
  either adds a diagnostic or gives it a status, so it is never silent
  (`borrowing_keeps_the_base_name_only`, `a_release_reaching_the_base_is_seen`).
* **An offset base is interior, not an exact alias**
  (`an_offset_result_is_not_an_exact_alias`, `an_offset_result_is_interior`).
* **Callers apply the summary exactly as a declaration**: the application is
  the `derive` instruction of `QCore.DerivedResult`, so a wrapper's result is
  invalidated by its base's release and its release is never the base's
  (`a_wrapper_result_is_invalidated`, `releasing_a_wrapper_result_never_releases_its_base`,
  and two witnesses).

## What this is not

A proof about the Rust transfer; the correspondence is trust boundary 28's
premise, maintained by review. A candidate is never a validated upstream
defect.
-/

namespace QCore.DerivedResultInference

open QCore.DerivedResult

/-! ### Observations and specs -/

/-- What one exit returns, as `observe_exit` reports it. -/
inductive Returned where
  /-- No tracked pointer: `return;`, an untracked or opaque value. -/
  | none
  /-- A null pointer constant (`ReturnedValue::Null`). -/
  | null
  /-- An identity site: a fresh resource or a formal's own binding. -/
  | site (s : Nat)
  /-- A derived binding over one site (`ReturnedValue::Derived`). -/
  | derived (s : Nat) (extent : Extent) (nullable constDropped : Bool)
  /-- Several sites, an unknown one, or a mixed binding. -/
  | ambiguous
  deriving DecidableEq, Repr

structure Exit where
  returned : Returned
  /-- A custody effect reached the base's name-only site on the way here. -/
  baseTouched : Bool
  deriving DecidableEq, Repr

/-- A derived-result summary `Derived { parameter, extent, nullable,
const_dropped }` for a fixed parameter, whose name-only site is `base`. -/
structure Spec where
  extent : Extent
  nullable : Bool
  constDropped : Bool
  deriving DecidableEq, Repr

/-- The contract's extent covers the observed one: interior covers both. -/
def covers : Extent → Extent → Bool
  | .interior, _ => true
  | .exact, .exact => true
  | .exact, .interior => false

/-- The returned value a derived summary accepts at one exit. -/
def returnedMatches (spec : Spec) (base : Nat) : Returned → Bool
  | .null => spec.nullable
  | .derived s e n c =>
      s == base && covers spec.extent e && (!n || spec.nullable) && (!c || spec.constDropped)
  | .none | .site _ | .ambiguous => false

/-- `result_matches`' derived arm, for one exit. -/
def exitMatches (spec : Spec) (base : Nat) (x : Exit) : Bool :=
  !x.baseTouched && returnedMatches spec base x.returned

/-- The gate: a clean run (`Safe`, no diagnostics), at least one reachable exit,
and every exit accepted. -/
def validates (spec : Spec) (base : Nat) (clean : Bool) (exits : List Exit) : Bool :=
  clean && !exits.isEmpty && exits.all (exitMatches spec base)

/-! ### The proposal -/

/-- The accumulator `propose` threads. -/
structure Acc where
  extent : Extent
  nullable : Bool
  constDropped : Bool
  derived : Bool
  deriving DecidableEq, Repr

def joinExtent : Extent → Extent → Extent
  | .exact, .exact => .exact
  | _, _ => .interior

def step (base : Nat) : Option Acc → Returned → Option Acc
  | none, _ => none
  | some a, .null => some { a with nullable := true }
  | some a, .derived s e n c =>
      if s = base then
        some { extent := joinExtent a.extent e, nullable := a.nullable || n,
               constDropped := a.constDropped || c, derived := true }
      else none
  | some _, _ => none

def start : Acc :=
  { extent := .exact, nullable := false, constDropped := false, derived := false }

def finish : Option Acc → Option Spec
  | some a => if a.derived then some ⟨a.extent, a.nullable, a.constDropped⟩ else none
  | none => none

/-- `legacy_derived_summary::propose`: read the spec off the exits' returned
values. The touched-base check runs before it (`infer_for_base`). -/
def propose (base : Nat) (exits : List Exit) : Option Spec :=
  finish ((exits.map (·.returned)).foldl (step base) (some start))

/-! ### Concrete meaning -/

/-- A concrete pointer value. -/
inductive Val where
  | null
  | ptr (object offset : Nat)
  deriving DecidableEq, Repr

/-- `r` points into the object `b` points to. -/
def sameObject : Val → Val → Prop
  | .ptr o _, .ptr o' _ => o = o'
  | _, _ => False

/-- What a spec admits as the result over the base value `b`. -/
def Spec.admits (spec : Spec) (b r : Val) : Prop :=
  (r = .null ∧ (b = .null ∨ spec.nullable = true))
    ∨ (spec.extent = .exact ∧ r = b)
    ∨ (spec.extent = .interior ∧ sameObject b r)

/-- The concrete results an exit observation claims over the base value `b`. -/
def denotes (b : Val) : Returned → Val → Prop
  | .null, r => r = .null
  | .derived _ .exact n _, r => r = b ∨ (n = true ∧ r = .null)
  | .derived _ .interior n _, r => sameObject b r ∨ (r = .null ∧ (b = .null ∨ n = true))
  | _, _ => True

/-! ### The gate is sound -/

/-- Every concrete result an accepted exit denotes is admitted by the spec. -/
theorem an_accepted_exit_is_admitted (spec : Spec) (base : Nat) (x : Exit) (b r : Val)
    (hm : exitMatches spec base x = true) (hd : denotes b x.returned r) :
    spec.admits b r := by
  obtain ⟨ret, touched⟩ := x
  simp only [exitMatches, Bool.and_eq_true] at hm
  obtain ⟨_, hm⟩ := hm
  unfold Spec.admits
  cases ret with
  | none => simp [returnedMatches] at hm
  | site s => simp [returnedMatches] at hm
  | ambiguous => simp [returnedMatches] at hm
  | null =>
      simp only [returnedMatches] at hm
      simp only [denotes] at hd
      exact Or.inl ⟨hd, Or.inr hm⟩
  | derived s e n c =>
      simp only [returnedMatches, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true']
        at hm
      obtain ⟨⟨⟨_, hcov⟩, hn⟩, _⟩ := hm
      cases e with
      | exact =>
          simp only [denotes] at hd
          rcases hd with hr | ⟨hn', hr⟩
          · cases hext : spec.extent with
            | exact => exact Or.inr (Or.inl ⟨rfl, hr⟩)
            | interior =>
                cases b with
                | null => exact Or.inl ⟨hr, Or.inl rfl⟩
                | ptr o off => exact Or.inr (Or.inr ⟨rfl, by rw [hr]; simp [sameObject]⟩)
          · rcases hn with hn | hn
            · simp [hn] at hn'
            · exact Or.inl ⟨hr, Or.inr hn⟩
      | interior =>
          cases hext : spec.extent with
          | exact => simp [covers, hext] at hcov
          | interior =>
              simp only [denotes] at hd
              rcases hd with hr | ⟨hr, hb | hn'⟩
              · exact Or.inr (Or.inr ⟨rfl, hr⟩)
              · exact Or.inl ⟨hr, Or.inl hb⟩
              · rcases hn with hn | hn
                · simp [hn] at hn'
                · exact Or.inl ⟨hr, Or.inr hn⟩

/-- A validated summary admits every concrete result every exit denotes. -/
theorem validation_is_sound (spec : Spec) (base : Nat) (clean : Bool) (exits : List Exit)
    (hv : validates spec base clean exits = true) (x : Exit) (hx : x ∈ exits) (b r : Val)
    (hd : denotes b x.returned r) :
    spec.admits b r ∧ x.baseTouched = false := by
  simp only [validates, Bool.and_eq_true, List.all_eq_true] at hv
  have hm := hv.2 x hx
  refine ⟨an_accepted_exit_is_admitted spec base x b r hm hd, ?_⟩
  simp only [exitMatches, Bool.and_eq_true, Bool.not_eq_true'] at hm
  exact hm.1

/-! ### Every refused shape is refused -/

/-- A mixed exit — no pointer, an identity site (a fresh resource, the formal
itself), or an ambiguous binding — refutes every spec. -/
theorem a_mixed_exit_refuses (spec : Spec) (base : Nat) (clean : Bool) (exits : List Exit)
    (x : Exit) (hx : x ∈ exits)
    (hmixed : x.returned = .none ∨ (∃ s, x.returned = .site s) ∨ x.returned = .ambiguous) :
    validates spec base clean exits = false := by
  have hno : exitMatches spec base x = false := by
    obtain ⟨ret, touched⟩ := x
    simp only at hmixed
    rcases hmixed with h | ⟨s, h⟩ | h <;> subst h <;> simp [exitMatches, returnedMatches]
  simp only [validates, Bool.and_eq_false_iff, List.all_eq_false]
  exact Or.inr ⟨x, hx, by simp [hno]⟩

/-- A pointer derived from any other site refutes every spec. -/
theorem another_base_refuses (spec : Spec) (base : Nat) (clean : Bool) (exits : List Exit)
    (x : Exit) (hx : x ∈ exits) (s : Nat) (e : Extent) (n c : Bool)
    (hr : x.returned = .derived s e n c) (hs : s ≠ base) :
    validates spec base clean exits = false := by
  have hno : exitMatches spec base x = false := by
    obtain ⟨ret, touched⟩ := x
    simp only at hr
    subst hr
    simp [exitMatches, returnedMatches, hs]
  simp only [validates, Bool.and_eq_false_iff, List.all_eq_false]
  exact Or.inr ⟨x, hx, by simp [hno]⟩

/-- A custody effect that reached the base at any exit refutes every spec. -/
theorem a_touched_base_refuses (spec : Spec) (base : Nat) (clean : Bool) (exits : List Exit)
    (x : Exit) (hx : x ∈ exits) (ht : x.baseTouched = true) :
    validates spec base clean exits = false := by
  have hno : exitMatches spec base x = false := by simp [exitMatches, ht]
  simp only [validates, Bool.and_eq_false_iff, List.all_eq_false]
  exact Or.inr ⟨x, hx, by simp [hno]⟩

/-- An unclean run, or one with no reachable exit, refutes every spec. -/
theorem an_unclean_or_exitless_run_refuses (spec : Spec) (base : Nat) (clean : Bool)
    (exits : List Exit) (h : clean = false ∨ exits = []) :
    validates spec base clean exits = false := by
  rcases h with h | h <;> simp [validates, h]

/-! ### The proposal -/

theorem step_none (base : Nat) (rs : List Returned) :
    rs.foldl (step base) none = none := by
  induction rs with
  | nil => rfl
  | cons r rs ih => simpa [List.foldl, step] using ih

/-- The fold succeeds only if every returned value is NULL or derived from the
base, and then its accumulator bounds each of them. -/
theorem fold_some (base : Nat) (rs : List Returned) (a a' : Acc)
    (h : rs.foldl (step base) (some a) = some a') :
    (a.derived = true → a'.derived = true)
      ∧ (a.nullable = true → a'.nullable = true)
      ∧ (a.constDropped = true → a'.constDropped = true)
      ∧ (a.extent = .interior → a'.extent = .interior)
      ∧ ∀ r ∈ rs, (r = .null ∧ a'.nullable = true)
          ∨ ∃ e n c, r = .derived base e n c ∧ a'.derived = true
              ∧ (e = .interior → a'.extent = .interior)
              ∧ (n = true → a'.nullable = true) ∧ (c = true → a'.constDropped = true) := by
  induction rs generalizing a with
  | nil =>
      simp only [List.foldl, Option.some.injEq] at h
      subst h
      simp
  | cons r rs ih =>
      cases r with
      | none => simp [List.foldl, step, step_none] at h
      | site s => simp [List.foldl, step, step_none] at h
      | ambiguous => simp [List.foldl, step, step_none] at h
      | null =>
          simp only [List.foldl, step] at h
          obtain ⟨hd, hn, hc, he, hall⟩ := ih _ h
          refine ⟨hd, fun _ => hn rfl, hc, he, ?_⟩
          intro r' hr'
          rcases List.mem_cons.mp hr' with rfl | hr'
          · exact Or.inl ⟨rfl, hn rfl⟩
          · exact hall r' hr'
      | derived s e n c =>
          by_cases hs : s = base
          · subst hs
            simp only [List.foldl, step] at h
            obtain ⟨hd, hn, hc, he, hall⟩ := ih _ h
            refine ⟨fun _ => hd rfl, fun ha => hn (by simp [ha]),
              fun ha => hc (by simp [ha]), fun ha => he (by simp [joinExtent, ha]), ?_⟩
            intro r' hr'
            rcases List.mem_cons.mp hr' with rfl | hr'
            · refine Or.inr ⟨e, n, c, rfl, hd rfl, fun hi => he (by
                  cases ha : a.extent <;> simp [joinExtent, hi]), fun hn' => hn (by simp [hn']),
                fun hc' => hc (by simp [hc'])⟩
            · exact hall r' hr'
          · simp [List.foldl, step, hs, step_none] at h

/-- Each returned value the proposal read is accepted by the spec it proposed. -/
theorem the_proposal_accepts_every_exit (base : Nat) (exits : List Exit) (spec : Spec)
    (hp : propose base exits = some spec) (x : Exit) (hx : x ∈ exits) :
    returnedMatches spec base x.returned = true := by
  unfold propose at hp
  cases hf : (exits.map (·.returned)).foldl (step base) (some start) with
  | none => simp [hf, finish] at hp
  | some a =>
      rw [hf] at hp
      simp only [finish] at hp
      split at hp
      · simp only [Option.some.injEq] at hp
        subst hp
        obtain ⟨_, _, _, _, hall⟩ := fold_some base _ start a hf
        rcases hall x.returned (List.mem_map.mpr ⟨x, hx, rfl⟩) with ⟨hr, hn⟩ | ⟨e, n, c, hr, _, he, hn, hc⟩
        · simp [hr, returnedMatches, hn]
        · rw [hr]
          simp only [returnedMatches, beq_self_eq_true, Bool.true_and, Bool.and_eq_true,
            Bool.or_eq_true, Bool.not_eq_true']
          refine ⟨⟨?_, ?_⟩, ?_⟩
          · cases e with
            | exact => cases a.extent <;> rfl
            | interior => simp [he rfl, covers]
          · cases n <;> simp_all
          · cases c <;> simp_all
      · simp at hp

/-- The proposal validates on a clean run whose exits leave the base
untouched: proposal and gate agree. -/
theorem the_proposal_validates (base : Nat) (exits : List Exit) (spec : Spec)
    (hp : propose base exits = some spec) (hclean : clean = true)
    (huntouched : ∀ x ∈ exits, x.baseTouched = false) :
    validates spec base clean exits = true := by
  have hne : exits ≠ [] := by
    intro h
    subst h
    simp [propose, finish, start] at hp
  simp only [validates, hclean, Bool.true_and, Bool.and_eq_true, List.all_eq_true]
  refine ⟨by simpa using hne, fun x hx => ?_⟩
  simp [exitMatches, huntouched x hx, the_proposal_accepts_every_exit base exits spec hp x hx]

/-- The proposal is exact exactly when no returned value is an interior one. -/
theorem the_proposal_is_exact_iff_every_exit_is (base : Nat) (exits : List Exit)
    (spec : Spec) (hp : propose base exits = some spec) :
    spec.extent = .exact ↔ ∀ x ∈ exits, ∀ s n c, x.returned ≠ .derived s .interior n c := by
  constructor
  · intro hexact x hx s n c hr
    have := the_proposal_accepts_every_exit base exits spec hp x hx
    rw [hr] at this
    simp [returnedMatches, covers, hexact] at this
  · intro hall
    -- Every step joins the extent with an exact one, so it stays exact.
    unfold propose at hp
    have key : ∀ (rs : List Returned) (a a' : Acc), a.extent = .exact →
        (∀ r ∈ rs, ∀ s n c, r ≠ .derived s .interior n c) →
        rs.foldl (step base) (some a) = some a' → a'.extent = .exact := by
      intro rs
      induction rs with
      | nil => intro a a' ha _ h; simp at h; subst h; exact ha
      | cons r rs ih =>
          intro a a' ha hrs h
          cases r with
          | none => simp [List.foldl, step, step_none] at h
          | site s => simp [List.foldl, step, step_none] at h
          | ambiguous => simp [List.foldl, step, step_none] at h
          | null =>
              simp only [List.foldl, step] at h
              exact ih { a with nullable := true } a' ha
                (fun r hr => hrs r (List.mem_cons_of_mem _ hr)) h
          | derived s e n c =>
              by_cases hs : s = base
              · subst hs
                simp only [List.foldl, step] at h
                cases e with
                | interior => exact absurd rfl (hrs _ (List.mem_cons_self ..) s n c)
                | exact =>
                    exact ih _ _ (by simp [joinExtent, ha])
                      (fun r hr => hrs r (List.mem_cons_of_mem _ hr)) h
              · simp [List.foldl, step, hs, step_none] at h
    cases hf : (exits.map (·.returned)).foldl (step base) (some start) with
    | none => simp [hf, finish] at hp
    | some a =>
        rw [hf] at hp
        simp only [finish] at hp
        split at hp
        · simp only [Option.some.injEq] at hp
          subst hp
          exact key _ start a rfl (by
            intro r hr s n c
            obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hr
            exact hall x hx s n c) hf
        · simp at hp

/-- The proposal declines a body with a mixed exit. -/
theorem the_proposal_declines_a_mixed_exit (base : Nat) (exits : List Exit) (x : Exit)
    (hx : x ∈ exits)
    (hmixed : x.returned = .none ∨ (∃ s, x.returned = .site s) ∨ x.returned = .ambiguous
      ∨ ∃ s e n c, x.returned = .derived s e n c ∧ s ≠ base) :
    propose base exits = none := by
  cases hp : propose base exits with
  | none => rfl
  | some spec =>
      have := the_proposal_accepts_every_exit base exits spec hp x hx
      obtain ⟨ret, touched⟩ := x
      simp only at hmixed this
      rcases hmixed with h | ⟨s, h⟩ | h | ⟨s, e, n, c, h, hs⟩
      · subst h; simp [returnedMatches] at this
      · subst h; simp [returnedMatches] at this
      · subst h; simp [returnedMatches] at this
      · subst h; simp [returnedMatches, hs] at this

/-- A body whose every exit returns NULL has no derived summary: it is the
ordinary route's `None` result, not this one. -/
theorem an_all_null_body_proposes_nothing (base : Nat) (exits : List Exit)
    (hnull : ∀ x ∈ exits, x.returned = .null) :
    propose base exits = none := by
  unfold propose
  have key : ∀ (rs : List Returned) (a : Acc), a.derived = false →
      (∀ r ∈ rs, r = .null) →
      ∃ a', rs.foldl (step base) (some a) = some a' ∧ a'.derived = false := by
    intro rs
    induction rs with
    | nil => intro a ha _; exact ⟨a, rfl, ha⟩
    | cons r rs ih =>
        intro a ha hrs
        rw [hrs r (List.mem_cons_self ..)]
        simp only [List.foldl, step]
        exact ih _ ha (fun r hr => hrs r (List.mem_cons_of_mem _ hr))
  obtain ⟨a', h, hd⟩ := key (exits.map (·.returned)) start rfl (by
    intro r hr
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hr
    exact hnull x hx)
  simp [h, finish, hd]

/-! ### The name-only base -/

/-- Deriving from, using and null-testing a binding leave a status-free site
status-free: borrowing the base never makes it a resource. -/
theorem borrowing_keeps_the_base_name_only (s : State) (site : Nat) (i : Instruction)
    (hi : (∃ x b e c, i = .derive x b e c) ∨ (∃ x, i = .use x) ∨ (∃ x n, i = .nullTest x n))
    (hs : s.status site = none) :
    (transfer s i).status site = none := by
  rcases hi with ⟨x, b, e, c, rfl⟩ | ⟨x, rfl⟩ | ⟨x, n, rfl⟩
  · simp [transfer, hs]
  · simp only [transfer, transferBase]
    split <;> (try split) <;> (try split) <;> simp_all
  · simp only [transfer, transferBase]
    split <;> (try split) <;> (try split) <;> simp_all [update] <;> split <;> simp_all

/-- A release reaching the name-only base — through the formal or anything
derived from it — is never silent: it adds a diagnostic, or it gives the site
a status, which is exactly what `derived_base_touched` reads. -/
theorem a_release_reaching_the_base_is_seen (s : State) (x site : Nat) (d : Derivation)
    (hb : s.binding x = some ⟨some site, d⟩) (hs : s.status site = none) :
    (transfer s (.release x)).diagnostics ≠ s.diagnostics
      ∨ (transfer s (.release x)).status site ≠ none := by
  left
  cases d with
  | identity => simp [transfer, transferBase, hb, hs]
  | derived e c => cases e <;> simp [transfer, hb, hs]
  | mixed => simp [transfer, hb]

/-! ### An offset base -/

/-- `memcpy(p + k, …)` with `k > 0` returns `p + k`, which is not `p`: no exact
spec admits it over a non-null base. -/
theorem an_offset_result_is_not_an_exact_alias (spec : Spec) (o off k : Nat) (hk : 0 < k)
    (hexact : spec.extent = .exact) :
    ¬ spec.admits (.ptr o off) (.ptr o (off + k)) := by
  intro h
  rcases h with ⟨h, _⟩ | ⟨_, h⟩ | ⟨h, _⟩
  · cases h
  · simp at h; omega
  · rw [hexact] at h; cases h

/-- It is inside the base's object, so every interior spec admits it. -/
theorem an_offset_result_is_interior (spec : Spec) (o off k : Nat)
    (hinterior : spec.extent = .interior) :
    spec.admits (.ptr o off) (.ptr o (off + k)) :=
  Or.inr (Or.inr ⟨hinterior, rfl⟩)

/-! ### Callers apply the summary as a declaration -/

/-- A call `x = f(actual, …)` to a function with this summary: the `derive`
instruction a declared derived result is, with the summary's extent and dropped
`const`. -/
def apply (spec : Spec) (x actual : Nat) : Instruction :=
  .derive x actual spec.extent spec.constDropped

/-- The summary acquires, releases and discharges nothing at its call. -/
theorem applying_the_summary_changes_no_status (s : State) (spec : Spec) (x actual : Nat) :
    (transfer s (apply spec x actual)).status = s.status :=
  deriving_changes_no_status s x actual spec.extent spec.constDropped

/-- A wrapper's result is invalidated by its base's release: using it afterwards
is the same `useAfterRelease` candidate as a declared derived result's. -/
theorem a_wrapper_result_is_invalidated (s : State) (spec : Spec) (b x site : Nat)
    (hb : s.binding b = some ⟨some site, .identity⟩) (hx : x ≠ b)
    (hl : s.status site = some .live) :
    (transfer (transfer (transfer s (apply spec x b)) (.release b)) (.use x)).diagnostics
      = .useAfterRelease site :: s.diagnostics := by
  have hbind : (transfer s (apply spec x b)).binding x
      = some ⟨some site, .derived spec.extent spec.constDropped⟩ := by
    simp [apply, transfer, update, hb, deriveFrom]
  have hbase : (transfer s (apply spec x b)).binding b = some ⟨some site, .identity⟩ := by
    simp [apply, transfer, update, hb, Ne.symm hx]
  have hlive : (transfer s (apply spec x b)).status site = some .live := by
    simp [applying_the_summary_changes_no_status, hl]
  have hdiag : (transfer s (apply spec x b)).diagnostics = s.diagnostics :=
    (deriving_binds_only_its_destination s x b spec.extent spec.constDropped b hx.symm).2
  rw [releasing_the_base_invalidates_every_derived_pointer _ b x site spec.extent
    spec.constDropped hbase hbind hlive, hdiag]

/-- Releasing a wrapper's result never releases its base. -/
theorem releasing_a_wrapper_result_never_releases_its_base (s : State) (spec : Spec)
    (b x : Nat) (bind : Binding) (hb : s.binding b = some bind) :
    (transfer (transfer s (apply spec x b)) (.release x)).status = s.status := by
  have hx : (transfer s (apply spec x b)).binding x
      = some ⟨bind.target, deriveFrom bind.derivation spec.extent spec.constDropped⟩ := by
    obtain ⟨t, d⟩ := bind
    simp [apply, transfer, update, hb]
  rw [releasing_a_derived_pointer_never_releases_its_base _ x _ hx (by
      obtain ⟨t, d⟩ := bind
      cases d <;> simp [deriveFrom])]
  exact applying_the_summary_changes_no_status s spec x b

/-! ### Witnesses -/

/-- `XXH_memcpy`: `return memcpy(dest, src, size);`, one exact exit over the
first formal's name-only site. -/
def xxhMemcpy : List Exit := [⟨.derived 0 .exact false false, false⟩]

theorem the_exact_wrapper_witness :
    propose 0 xxhMemcpy = some ⟨.exact, false, false⟩
      ∧ validates ⟨.exact, false, false⟩ 0 true xxhMemcpy = true := by
  decide

/-- `mk_string_casestr`: `return NULL;` beside `return strcasestr(heystack, …);`,
whose interior step may be null and drops `const`. -/
def mkStringCasestr : List Exit :=
  [⟨.null, false⟩, ⟨.derived 0 .interior true true, false⟩]

theorem the_nullable_interior_wrapper_witness :
    propose 0 mkStringCasestr = some ⟨.interior, true, true⟩
      ∧ validates ⟨.interior, true, true⟩ 0 true mkStringCasestr = true
      ∧ validates ⟨.exact, true, true⟩ 0 true mkStringCasestr = false
      ∧ validates ⟨.interior, false, true⟩ 0 true mkStringCasestr = false := by
  decide

/-- A fresh allocation on one path beside the derived exit: declined, and no
spec validates it. -/
def freshOnOnePath : List Exit := [⟨.site 7, false⟩, ⟨.derived 0 .exact false false, false⟩]

theorem the_mixed_exit_witness :
    propose 0 freshOnOnePath = none
      ∧ validates ⟨.interior, true, true⟩ 0 true freshOnOnePath = false := by
  decide

end QCore.DerivedResultInference
