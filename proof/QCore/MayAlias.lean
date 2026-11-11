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
# The bounded may-alias refinement (card C02-P-ALIAS)

`QCore.BorrowedActual` models the closure that decides which bases a body never
binds to a resource identity, and `QCore.ExitResiduals` the whitelist that reads
through one. Both are *name-based*: neither relates two **distinct declared
names** that may denote the same object. Card C02-P recorded exactly that as its
open boundary, with the measured shape

```c
void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }
```

answering `Safe`, and pinned the field-read form to answer identically so that a
later increment would move both together.

This module models the decision that increment makes,
`crates/qcore-c-gateway/src/may_alias.rs` together with
`legacy_c::require_unaliased_reads`, `legacy_c::distinct_by_provenance` and the
`BranchOnDistinct` edge refinement:

* **the class relation** — two names are peers when they carry the same declared
  `struct TAG *` type and are distinct. Card ALIAS-LOCALS lifted v1's further
  requirement that at least one of them be a formal parameter, so two **locals**
  are related exactly like any other pair; that bound was a blast-radius choice,
  never a soundness argument, and it was launderable by two assignments;
* **the refusal** — a modeled read of `y` is refused when some peer `x` was
  released on the path and neither discharge applies;
* **the two discharges** — provenance (`y` is bound to one live identity this
  body acquired) and a pointer-inequality guard (a branch proved `x ≠ y` on this
  edge, dropped when either name is rebound);
* **the definite-alias relation kind** (ALIAS-LOCALS) — the projection has bound
  `x` and `y` to the *same* single non-null identity. Not a discharge: a
  strengthening. It defeats the guard, because a source-level comparison may not
  overrule the projection's own identity model, and refusing on that
  contradiction is strictly the more refusing of the two answers;
* **the join** — releases are unioned and guards intersected, which is the
  direction each fact must move to stay sound and is also why the work list
  terminates.

The `released`/`distinct` half of the `State` below is what `legacy_c`'s
`alias_facts` computes, in a **separate** forward fixed point over the same
CFG, before the custody analysis runs; the `bound` half is read from the custody
state at the use. That separation is measured, not stylistic: carrying the two
sets inside the custody state reordered its work list, and leak diagnostics are
reported from intermediate iterates of an exit node
(`C02-P-ALIAS-LEAK-ORDER`).

Everything here is a model of the *decision*. That the Rust reader extracts the
declared types and the release events faithfully from C source is a premise, not
a theorem, exactly as in `QCore.BorrowedActual`; nothing here is a claim about
arbitrary C memory safety, and nothing here decides aliasing — the rule only
*refuses to decide* where the projection previously answered by silence.
-/

namespace QCore.MayAlias

/-- A declared name of one function body. -/
abbrev Name := Nat

/-- A record tag. -/
abbrev Tag := Nat

/-- An abstract acquisition identity. -/
abbrev Site := Nat

/-- `==` on names and tags is symmetric. Stated once, used by the class
    relation's symmetry below. -/
private theorem beq_symm (a b : Nat) : (a == b) = (b == a) := by
  cases h : a == b with
  | true =>
    have : a = b := by simpa using h
    simp [this]
  | false =>
    cases h' : b == a with
    | true =>
      have : b = a := by simpa using h'
      simp [this] at h
    | false => rfl

/-- The declared record-pointer classes of one function body, as
    `MayAliasClasses` reads them.

    `tag n = none` covers every name this reader will not commit to: a
    non-pointer, a `void *`, a multi-star, an unresolved typedef, and *every*
    name at all when the declaration reader refused the function
    (`C02-P-ALIAS-UNTYPED`).

    Whether a name is a formal parameter is deliberately **not** recorded.
    v1 carried a `formal : Name → Bool` field and required at least one formal
    in every related pair; card ALIAS-LOCALS lifted that bound, and the
    caller-held half of the question is answered where it belongs — by the
    provenance discharge's `bodyMade`. -/
structure Classes where
  tag : Name → Option Tag

/-- Whether two names carry the same declared `struct TAG *` type. -/
def sameTag (c : Classes) (x y : Name) : Bool :=
  match c.tag x, c.tag y with
  | some t, some u => t == u
  | _, _ => false

/-- Whether releasing `other` can compromise a read of `used`. -/
def peer (c : Classes) (used other : Name) : Bool :=
  !(other == used) && sameTag c used other

/-- What the projection knows about the identity a name denotes at one program
    point. -/
structure Binding where
  /-- The single **non-null** identity the projection committed to, or `none`.
      `none` covers every way it can decline to commit: an unbound name, an
      unknown pointer, an ambiguous target set and a possibly-null binding —
      exactly `may_alias::Denotation::settled` in the Rust. -/
  site : Option Site
  /-- The identity was acquired by **this** body, not handed in by the caller.
      A caller-held entry site is deliberately not provenance: two formals
      carrying declared entry custody are the pair this card exists for. -/
  bodyMade : Bool
  /-- The identity is live here and carries no unresolved effect. -/
  live : Bool

/-- Order a pair of names, so a guard fact cannot depend on spelling order. -/
def normalize (a b : Name) : Name × Name := if a ≤ b then (a, b) else (b, a)

/-- One path's may-alias facts. -/
structure State where
  /-- Names through which a release took effect on this path. A *may* fact. -/
  released : List Name
  /-- Name pairs a pointer-inequality guard proved distinct. A *must* fact. -/
  distinct : List (Name × Name)
  /-- What the projection has bound to each name here. -/
  bound : Name → Option Binding

/-- The empty state: nothing released, nothing proved, nothing bound. -/
def State.initial : State :=
  { released := [], distinct := [], bound := fun _ => none }

/-! ## The two discharges -/

/-- Provenance: `y` denotes exactly one live identity this body acquired. -/
def dischargedByProvenance (s : State) (y : Name) : Bool :=
  match s.bound y with
  | some b => b.site.isSome && b.bodyMade && b.live
  | none => false

/-- A pointer-inequality guard proved this pair distinct on this path.

    This is the raw `BranchOnDistinct` fact `alias_facts` carries. Whether it
    *discharges* is [`guardDischarges`] below, which subtracts the definite
    aliases. -/
def dischargedByGuard (s : State) (y x : Name) : Bool :=
  decide (normalize y x ∈ s.distinct)

/-! ## The third relation kind (ALIAS-LOCALS) -/

/-- The single non-null identity the projection has bound to `n` here. -/
def denotes (s : State) (n : Name) : Option Site :=
  (s.bound n).bind (fun b => b.site)

/-- ALIAS-LOCALS: the projection's own bindings show `x` and `y` denote **one**
    object here — they do not merely *may* alias, they *do*.

    This is not a discharge, and it never relates a pair the declared types do
    not already relate: it refines the peer relation, it never widens it. -/
def definitelyAliased (s : State) (y x : Name) : Bool :=
  match denotes s y, denotes s x with
  | some a, some b => a == b
  | _, _ => false

/-- ALIAS-LOCALS: the guard discharge, with the definite-alias exception.

    A branch asserting `y ≠ x` cannot discharge a pair the projection has bound
    to one identity. Refusing on that contradiction is strictly the more
    refusing answer, and it is the only one that does not let a source-level
    comparison overrule the projection's own bindings. -/
def guardDischarges (s : State) (y x : Name) : Bool :=
  dischargedByGuard s y x && !definitelyAliased s y x

/-! ## The refusal -/

/-- Whether a modeled read of `y` is refused here. -/
def refuses (c : Classes) (s : State) (y : Name) : Bool :=
  !dischargedByProvenance s y
    && s.released.any (fun x => peer c y x && !guardDischarges s y x)

/-! ## The state operations -/

/-- Record a release through `x`. -/
def recordRelease (s : State) (x : Name) : State :=
  { s with released := x :: s.released }

/-- Rebind `n`.

    Only the *guard* facts mentioning `n` are dropped: an inequality proved of
    the old value says nothing about the new one. `released` is deliberately
    untouched — rebinding the name that released an object does not resurrect
    the object. -/
def rebind (s : State) (n : Name) : State :=
  { s with distinct := s.distinct.filter (fun p => !(p.1 == n) && !(p.2 == n)) }

/-- The edge of a `BranchOnDistinct` on which the two names are distinct. -/
def assumeDistinct (s : State) (a b : Name) : State :=
  { s with distinct := normalize a b :: s.distinct }

/-- The join: releases are unioned, guards intersected. -/
def join (s t : State) : State :=
  { released := s.released ++ t.released,
    distinct := s.distinct.filter (fun p => decide (p ∈ t.distinct)),
    bound := s.bound }

/-! ## The verdict this rule can move -/

/-- What one path answers about a read. -/
inductive Answer where
  | safe
  | unresolved
  deriving DecidableEq, Repr

/-- The rule composes with whatever the projection already answered: a refusal
    forces `unresolved`, and nothing else changes. -/
def answer (refused : Bool) (before : Answer) : Answer :=
  if refused then Answer.unresolved else before

/-! ## The class relation -/

theorem sameTag_is_symmetric (c : Classes) (x y : Name) : sameTag c x y = sameTag c y x := by
  unfold sameTag
  cases hx : c.tag x <;> cases hy : c.tag y <;> simp [beq_symm]

theorem peer_is_symmetric (c : Classes) (x y : Name) : peer c x y = peer c y x := by
  unfold peer
  rw [sameTag_is_symmetric, beq_symm]

/-- A name is never its own peer, so a release through it is judged by the
    existing double-release model rather than by this rule. -/
theorem a_name_is_never_its_own_peer (c : Classes) (x : Name) : peer c x x = false := by
  simp [peer]

/-- Different declared record tags are never peers. -/
theorem a_different_tag_is_never_a_peer {c : Classes} {x y : Name} {t u : Tag}
    (hx : c.tag x = some t) (hy : c.tag y = some u) (h : t ≠ u) :
    peer c x y = false := by
  simp [peer, sameTag, hx, hy, h]

/-- `C02-P-ALIAS-UNTYPED`: a name this reader will not type relates to nothing.
    A function whose declarations were refused has `tag = fun _ => none`, so no
    pair in it is related and its pre-existing answer stands. -/
theorem an_untyped_name_is_never_a_peer {c : Classes} {x y : Name}
    (hx : c.tag x = none) : peer c x y = false := by
  simp [peer, sameTag, hx]

/-- The relation is non-vacuous, and after ALIAS-LOCALS this one statement is
    the whole positive direction: two distinct same-tagged names are peers,
    whatever their storage.

    It replaces v1's `two_same_tagged_formals_are_peers` and
    `a_formal_and_a_local_are_peers`, which are its instances, and it is the
    exact inverse of v1's `two_locals_are_never_peers`. All three are retired by
    name in `docs/reports/remaining-alias-locals/README.md` §5. -/
theorem two_same_tagged_names_are_peers {c : Classes} {x y : Name} {t : Tag}
    (hne : x ≠ y) (hx : c.tag x = some t) (hy : c.tag y = some t) : peer c x y = true := by
  have h : ¬ (y = x) := fun h => hne h.symm
  simp [peer, sameTag, hx, hy, h]

/-- `C02-P-ALIAS-LOCALS` lifted: a pair of locals is not special. The model
    carries no notion of "formal" at all any more, so this is the statement that
    the *declared class* is the whole premise — two distinct names of one tag are
    peers, and the previous bound cannot be recovered from anything here. -/
theorem two_locals_of_one_class_are_peers {c : Classes} {one two : Name} {t : Tag}
    (hne : one ≠ two) (h1 : c.tag one = some t) (h2 : c.tag two = some t) :
    peer c one two = true ∧ peer c two one = true :=
  ⟨two_same_tagged_names_are_peers hne h1 h2,
   two_same_tagged_names_are_peers (fun h => hne h.symm) h2 h1⟩

/-! ## The refusal fires exactly on a released, undischarged peer -/

/-- The headline: the measured shape. `x` was released, `y` is its peer, `y` is
    not discharged, so the read of `y` is refused. -/
theorem a_released_peer_refuses_the_read {c : Classes} {s : State} {x y : Name}
    (hx : x ∈ s.released)
    (hpeer : peer c y x = true)
    (hguard : guardDischarges s y x = false)
    (hprov : dischargedByProvenance s y = false) :
    refuses c s y = true := by
  simp only [refuses, hprov, Bool.not_false, Bool.true_and, List.any_eq_true]
  exact ⟨x, hx, by simp [hpeer, hguard]⟩

/-- The converse, and the statement that the rule invents nothing: a refusal
    always names a released peer that no discharge covered. -/
theorem a_refusal_names_a_released_undischarged_peer {c : Classes} {s : State} {y : Name}
    (h : refuses c s y = true) :
    dischargedByProvenance s y = false
      ∧ ∃ x, x ∈ s.released ∧ peer c y x = true ∧ guardDischarges s y x = false := by
  have hprov : dischargedByProvenance s y = false := by
    cases hp : dischargedByProvenance s y with
    | false => rfl
    | true => simp [refuses, hp] at h
  refine ⟨hprov, ?_⟩
  simp only [refuses, hprov, Bool.not_false, Bool.true_and, List.any_eq_true] at h
  obtain ⟨x, hx, hxs⟩ := h
  simp only [Bool.and_eq_true, Bool.not_eq_eq_eq_not, Bool.not_true] at hxs
  exact ⟨x, hx, hxs.1, hxs.2⟩

/-- Nothing released, nothing refused. The rule is triggered by a release, never
    by a declaration. -/
theorem no_release_refuses_nothing (c : Classes) (s : State) (y : Name)
    (h : s.released = []) : refuses c s y = false := by
  simp [refuses, h]

/-- Releasing a name never refuses a read of *that* name through this rule. -/
theorem releasing_a_name_does_not_refuse_itself (c : Classes) (y : Name) :
    refuses c (recordRelease State.initial y) y = false := by
  simp [refuses, recordRelease, State.initial, a_name_is_never_its_own_peer]

/-- Releasing a non-peer refuses nothing. -/
theorem releasing_a_non_peer_refuses_nothing {c : Classes} {s : State} {x y : Name}
    (hs : s.released = []) (hpeer : peer c y x = false) :
    refuses c (recordRelease s x) y = false := by
  simp [refuses, recordRelease, hs, hpeer]

/-! ## The provenance discharge -/

/-- A live identity this body acquired discharges every peer at once. -/
theorem a_live_body_made_binding_discharges (c : Classes) (s : State) (y : Name) {b : Binding}
    (hb : s.bound y = some b) (hs : b.site.isSome = true) (hm : b.bodyMade = true)
    (hl : b.live = true) : refuses c s y = false := by
  simp [refuses, dischargedByProvenance, hb, hs, hm, hl]

/-- An unbound name is never discharged by provenance: silence about a name is
    exactly what this card refuses to read as safety. -/
theorem an_unbound_name_is_never_discharged (s : State) (y : Name)
    (h : s.bound y = none) : dischargedByProvenance s y = false := by
  simp [dischargedByProvenance, h]

/-- Provenance requires the identity to be **live**: a place this body acquired
    and then released does not discharge. -/
theorem a_dead_binding_never_discharges (s : State) (y : Name) {b : Binding}
    (hb : s.bound y = some b) (hl : b.live = false) :
    dischargedByProvenance s y = false := by
  simp [dischargedByProvenance, hb, hl]

/-- Provenance requires the identity to be **this body's**: a caller-held entry
    site never discharges. -/
theorem a_caller_held_binding_never_discharges (s : State) (y : Name) {b : Binding}
    (hb : s.bound y = some b) (hm : b.bodyMade = false) :
    dischargedByProvenance s y = false := by
  simp [dischargedByProvenance, hb, hm]

/-- And provenance requires a *single* resolved identity. -/
theorem an_ambiguous_binding_never_discharges (s : State) (y : Name) {b : Binding}
    (hb : s.bound y = some b) (hs : b.site = none) :
    dischargedByProvenance s y = false := by
  simp [dischargedByProvenance, hb, hs]

/-! ## The guard discharge -/

/-- The guard discharges exactly the pair it proved. -/
theorem the_guard_discharges_its_pair (s : State) (a b : Name) :
    dischargedByGuard (assumeDistinct s a b) a b = true := by
  simp [dischargedByGuard, assumeDistinct]

/-- The guard is symmetric in its two names, as the normalized pair makes it. -/
theorem the_guard_is_symmetric (s : State) (a b : Name) :
    dischargedByGuard s a b = dischargedByGuard s b a := by
  unfold dischargedByGuard normalize
  by_cases h : a ≤ b
  · by_cases h' : b ≤ a
    · have hab : a = b := Nat.le_antisymm h h'
      subst hab
      rfl
    · simp [h, h']
  · have h' : b ≤ a := Nat.le_of_lt (Nat.lt_of_not_le h)
    simp [h, h']

/-- A guard on one pair discharges no *other* peer. This is why a three-name
    class needs a guard per pair. -/
theorem a_guard_discharges_no_other_peer {s : State} {a b z : Name}
    (h : dischargedByGuard s a z = false) (hne : normalize a z ≠ normalize a b) :
    dischargedByGuard (assumeDistinct s a z) a b = false →
      dischargedByGuard (assumeDistinct s a b) a z = false := by
  intro _
  simp only [dischargedByGuard, assumeDistinct, List.mem_cons, decide_eq_false_iff_not,
    not_or] at h ⊢
  exact ⟨hne, by simpa using h⟩

/-- Rebinding either name drops the guard: the inequality was about the old
    value. -/
theorem rebinding_drops_the_guard (s : State) (a b : Name) :
    dischargedByGuard (rebind (assumeDistinct s a b) a) a b = false := by
  simp only [dischargedByGuard, rebind, assumeDistinct, decide_eq_false_iff_not, List.mem_filter]
  intro hm
  have := hm.2
  unfold normalize at this
  split at this <;> simp at this

theorem rebinding_the_other_name_drops_the_guard (s : State) (a b : Name) :
    dischargedByGuard (rebind (assumeDistinct s a b) b) a b = false := by
  simp only [dischargedByGuard, rebind, assumeDistinct, decide_eq_false_iff_not, List.mem_filter]
  intro hm
  have := hm.2
  unfold normalize at this
  split at this <;> simp at this

/-- Rebinding a *third* name leaves the guard intact, so the invalidation is
    exactly as wide as it must be. -/
theorem rebinding_an_unrelated_name_keeps_the_guard {s : State} {a b n : Name}
    (ha : ¬ (a = n)) (hb : ¬ (b = n)) :
    dischargedByGuard (rebind (assumeDistinct s a b) n) a b = true := by
  simp only [dischargedByGuard, rebind, assumeDistinct, decide_eq_true_eq, List.filter_cons]
  unfold normalize
  split <;> simp [ha, hb]

/-! ## Rebinding never discharges a release -/

/-- Rebinding only ever *drops* guard facts, never adds one. -/
theorem rebinding_only_drops_guards {s : State} {n y z : Name}
    (h : dischargedByGuard s y z = false) :
    dischargedByGuard (rebind s n) y z = false := by
  simp only [dischargedByGuard, rebind, decide_eq_false_iff_not, List.mem_filter] at h ⊢
  exact fun hm => h hm.1

/-- And so the *discharge* only ever shrinks under a rebinding: `rebind` leaves
    the bindings alone, so a definite alias stays definite while the raw guard
    fact can only be dropped. -/
theorem rebinding_only_drops_the_guard_discharge {s : State} {n y z : Name}
    (h : guardDischarges s y z = false) : guardDischarges (rebind s n) y z = false := by
  simp only [guardDischarges, Bool.and_eq_false_iff, Bool.not_eq_false'] at h ⊢
  cases h with
  | inl h => exact Or.inl (rebinding_only_drops_guards h)
  | inr h => exact Or.inr (by simpa [definitelyAliased, denotes, rebind] using h)

/-- Releases only accumulate: `rebind` leaves the release list exactly as it
    was. -/
theorem rebinding_preserves_every_release (s : State) (n : Name) :
    (rebind s n).released = s.released := rfl

/-- The bindings a rebinding of one name leaves are the caller's business; the
    fail-closed direction that matters is this one. Rebinding the *releasing*
    name does not resurrect the object it released, so a refusal that no guard
    covered survives. -/
theorem rebinding_the_releasing_name_keeps_the_refusal {c : Classes} {s : State} {n y : Name}
    (h : refuses c s y = true)
    (hb : dischargedByProvenance (rebind s n) y = false) :
    refuses c (rebind s n) y = true := by
  obtain ⟨_, x, hx, hpeer, hguard⟩ := a_refusal_names_a_released_undischarged_peer h
  exact a_released_peer_refuses_the_read (x := x)
    (by simpa [rebind] using hx) hpeer (rebinding_only_drops_the_guard_discharge hguard) hb

/-! ## Monotonicity, and what the join must do -/

/-- Recording one more release can only refuse more. -/
theorem adding_a_release_only_refuses_more {c : Classes} {s : State} {x y : Name}
    (h : refuses c s y = true) : refuses c (recordRelease s x) y = true := by
  obtain ⟨hprov, x', hx', hpeer, hguard⟩ := a_refusal_names_a_released_undischarged_peer h
  exact a_released_peer_refuses_the_read (x := x')
    (by simpa [recordRelease] using List.mem_cons_of_mem x hx') hpeer hguard hprov

/-- The join really is a union on releases. -/
theorem the_join_unions_the_releases (s t : State) (x : Name)
    (h : x ∈ s.released ∨ x ∈ t.released) : x ∈ (join s t).released := by
  simp only [join]
  cases h with
  | inl h => exact List.mem_append_left _ h
  | inr h => exact List.mem_append_right _ h

/-- A guard the join dropped was not proved on both paths. -/
theorem the_join_only_keeps_shared_guards {s t : State} {p : Name × Name}
    (h : p ∈ (join s t).distinct) : p ∈ s.distinct ∧ p ∈ t.distinct := by
  simp only [join, List.mem_filter, decide_eq_true_eq] at h
  exact h

/-- A refusal on either incoming path survives the join, because releases are
    joined by **union** and guards only shrink. -/
theorem the_join_preserves_a_refusal {c : Classes} {s t : State} {y : Name}
    (h : refuses c s y = true)
    (hprov : dischargedByProvenance (join s t) y = false) :
    refuses c (join s t) y = true := by
  obtain ⟨_, x, hx, hpeer, hguard⟩ := a_refusal_names_a_released_undischarged_peer h
  refine a_released_peer_refuses_the_read (x := x)
    (the_join_unions_the_releases s t x (Or.inl hx)) hpeer ?_ hprov
  -- The join keeps `s`'s bindings, so a definite alias survives it; and the
  -- guard list can only shrink.
  simp only [guardDischarges, Bool.and_eq_false_iff, Bool.not_eq_false'] at hguard ⊢
  cases hguard with
  | inl hguard =>
    refine Or.inl ?_
    simp only [dischargedByGuard, decide_eq_false_iff_not] at hguard ⊢
    exact fun hm => hguard (the_join_only_keeps_shared_guards hm).1
  | inr hguard => exact Or.inr (by simpa [definitelyAliased, denotes, join] using hguard)

/-- And a guard only one path proved does not survive: that is the direction a
    *must* fact has to move. -/
theorem the_join_intersects_the_guards {s t : State} {p : Name × Name}
    (h : p ∉ t.distinct) : p ∉ (join s t).distinct :=
  fun hm => h (the_join_only_keeps_shared_guards hm).2

/-- A guard both paths proved does survive, so the intersection is not the empty
    set by construction. -/
theorem the_join_keeps_a_guard_both_paths_proved {s t : State} {p : Name × Name}
    (hs : p ∈ s.distinct) (ht : p ∈ t.distinct) : p ∈ (join s t).distinct := by
  simp only [join, List.mem_filter, decide_eq_true_eq]
  exact ⟨hs, ht⟩

/-- The join's release list only grows, which — with the guard list only
    shrinking — is the finite-descent argument the work list needs. -/
theorem the_join_never_shortens_the_releases (s t : State) :
    s.released.length ≤ (join s t).released.length := by
  simp [join]

theorem the_join_never_lengthens_the_guards (s t : State) :
    (join s t).distinct.length ≤ s.distinct.length := by
  simpa [join] using List.length_filter_le (fun p => decide (p ∈ t.distinct)) s.distinct

/-! ## The rule adds refusals only -/

/-- A refusal forces `unresolved`. -/
theorem a_refusal_is_unresolved (before : Answer) : answer true before = Answer.unresolved := by
  simp [answer]

/-- And where it does not fire, the projection's own answer is untouched. -/
theorem no_refusal_changes_nothing (before : Answer) : answer false before = before := by
  simp [answer]

/-- The fail-closed direction that matters: this rule can never turn an
    `unresolved` answer into a `safe` one. Every movement it causes is
    `safe → unresolved`. -/
theorem the_rule_never_promotes {r : Bool} {before : Answer}
    (h : answer r before = Answer.safe) : before = Answer.safe := by
  cases r with
  | false => simpa [answer] using h
  | true => simp [answer] at h

/-- Non-vacuity in both directions: the rule does refuse somewhere, and does
    stay silent somewhere. Without this every refusal theorem above would hold
    of the empty rule. -/
theorem the_rule_is_not_vacuous :
    ∃ (c : Classes) (s : State) (y : Name),
      refuses c s y = true ∧ ∃ z, refuses c s z = false := by
  refine ⟨{ tag := fun _ => some 0 },
          { released := [0], distinct := [], bound := fun _ => none }, 1, ?_, 0, ?_⟩ <;>
    simp [refuses, dischargedByProvenance, peer, sameTag, guardDischarges, dischargedByGuard]

/-- And each discharge is load-bearing: on its own it turns the measured
    refusal back into silence. -/
theorem the_guard_discharge_is_load_bearing :
    ∃ (c : Classes) (s : State) (y x : Name),
      refuses c s y = true ∧ refuses c (assumeDistinct s y x) y = false := by
  refine ⟨{ tag := fun _ => some 0 },
          { released := [0], distinct := [], bound := fun _ => none }, 1, 0, ?_, ?_⟩ <;>
    simp [refuses, dischargedByProvenance, peer, sameTag, guardDischarges, dischargedByGuard,
      definitelyAliased, denotes, assumeDistinct, normalize]

theorem the_provenance_discharge_is_load_bearing :
    ∃ (c : Classes) (s : State) (y : Name),
      refuses c s y = true
        ∧ refuses c { s with
              bound := fun _ => some { site := some 0, bodyMade := true, live := true } } y
            = false := by
  refine ⟨{ tag := fun _ => some 0 },
          { released := [0], distinct := [], bound := fun _ => none }, 1, ?_, ?_⟩ <;>
    simp [refuses, dischargedByProvenance, peer, sameTag, guardDischarges, dischargedByGuard]

/-! ## The definite-alias relation kind (ALIAS-LOCALS) -/

/-- The positive direction: two names the projection bound to one identity are
    a definite alias. -/
theorem two_names_bound_to_one_identity_are_definitely_aliased {s : State} {y x : Name}
    {b b' : Binding} {i : Site}
    (hy : s.bound y = some b) (hx : s.bound x = some b')
    (hby : b.site = some i) (hbx : b'.site = some i) :
    definitelyAliased s y x = true := by
  simp [definitelyAliased, denotes, hy, hx, hby, hbx]

/-- Distinct identities are not, which is what keeps the kind from holding of
    every pair. -/
theorem two_names_bound_to_different_identities_are_not {s : State} {y x : Name}
    {b b' : Binding} {i j : Site}
    (hy : s.bound y = some b) (hx : s.bound x = some b')
    (hby : b.site = some i) (hbx : b'.site = some j) (hne : i ≠ j) :
    definitelyAliased s y x = false := by
  simp [definitelyAliased, denotes, hy, hx, hby, hbx, hne]

/-- The relation is symmetric, as the guard question it answers requires. -/
theorem the_definite_relation_is_symmetric (s : State) (y x : Name) :
    definitelyAliased s y x = definitelyAliased s x y := by
  unfold definitelyAliased
  cases hy : denotes s y <;> cases hx : denotes s x <;> simp [beq_symm]

/-- Every way the projection declines to commit fails **closed**: an
    uncommitted binding is never a definite alias, so the guard keeps
    discharging exactly the pairs it discharged before this card. `site = none`
    is the model of `Denotation::settled` returning `None`, which covers an
    unbound name, an unknown pointer, an ambiguous target set and a
    possibly-null binding alike. -/
theorem an_uncommitted_binding_is_never_a_definite_alias {s : State} {y x : Name}
    (h : denotes s y = none) : definitelyAliased s y x = false := by
  simp [definitelyAliased, h]

/-- An unbound name denotes nothing, so it is never a definite alias either. -/
theorem an_unbound_name_is_never_a_definite_alias {s : State} {y x : Name}
    (h : s.bound y = none) : definitelyAliased s y x = false := by
  simp [definitelyAliased, denotes, h]

/-- **The exception, stated.** A definite alias defeats the guard: the branch
    asserts the two names are distinct while the projection has bound them to
    one identity, and the rule refuses on that contradiction. -/
theorem a_definite_alias_defeats_the_guard {s : State} {y x : Name}
    (h : definitelyAliased s y x = true) : guardDischarges s y x = false := by
  simp [guardDischarges, h]

/-- And where there is no definite alias the guard discharges exactly as it
    did, so this card weakened no discharge it did not mean to. -/
theorem the_guard_still_discharges_a_non_definite_pair {s : State} {y x : Name}
    (hguard : dischargedByGuard s y x = true) (h : definitelyAliased s y x = false) :
    guardDischarges s y x = true := by
  simp [guardDischarges, hguard, h]

/-- The composite direction: a released definite alias with no provenance is
    refused **even under a guard**. This is the shape card ALIAS-LOCALS
    measured, `one = b_open(...); two = one; if (one != two) { b_release(one);
    b_probe(two); }`. -/
theorem a_released_definite_alias_refuses_under_a_guard {c : Classes} {s : State} {x y : Name}
    (hx : x ∈ s.released)
    (hpeer : peer c y x = true)
    (halias : definitelyAliased s y x = true)
    (hprov : dischargedByProvenance s y = false) :
    refuses c s y = true :=
  a_released_peer_refuses_the_read hx hpeer (a_definite_alias_defeats_the_guard halias) hprov

/-- The exception is load-bearing and is not vacuous: one state, one pair, and
    the guard discharges it only when the bindings do not make it definite.
    Without this, `a_definite_alias_defeats_the_guard` would hold of a rule that
    never found a definite alias at all. -/
theorem the_definite_alias_exception_is_load_bearing :
    ∃ (c : Classes) (s : State) (y x : Name),
      dischargedByGuard s y x = true
        ∧ refuses c s y = true
        ∧ refuses c { s with bound := fun _ => none } y = false := by
  refine ⟨{ tag := fun _ => some 0 },
          { released := [0], distinct := [(0, 1)],
            bound := fun _ => some { site := some 5, bodyMade := false, live := false } },
          1, 0, ?_, ?_, ?_⟩ <;>
    simp [refuses, dischargedByProvenance, peer, sameTag, guardDischarges, dischargedByGuard,
      definitelyAliased, denotes, normalize]

/-- The definite-alias kind **refines** the peer relation and never widens it:
    a pair the declared types do not relate stays unrelated however definite the
    projection's bindings are (`C02-P-ALIAS-UNTYPED`, retained). -/
theorem a_definite_alias_of_untyped_names_still_refuses_nothing {c : Classes} {s : State}
    {y : Name} (hy : c.tag y = none) : refuses c s y = false := by
  simp only [refuses, Bool.and_eq_false_iff, List.any_eq_false]
  exact Or.inr (fun z _ => by simp [an_untyped_name_is_never_a_peer hy])

end QCore.MayAlias
