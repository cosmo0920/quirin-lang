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

import QCore.IndexExtent

/-! Pins for bounds declarations (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS,
contract specification C09.1 and C06.2, gateway rule 12r).

A bounds contract v1 declaration selected on an audit route is compared with
its crate's bounds lock (`bounds-contracts.lock`) over the audited source
before any unit is analysed, by the same decision C06.2 gives resource
declarations (`crates/qcore-audit/src/contract_revision.rs`, `pin_standing`):
in force exactly when a lock covers it, the lock has its entry, the entry's
contract digest is the declaration's (which covers every clause, the quant
check's text included) and the audited sites equal one reviewed identity. A
declaration out of force is **held** (`crates/qcore-contract/src/bounds_contract.rs`,
`BoundsContracts::hold_for_pin_mismatch`, read by `held()`), exactly as a
review marker holds it, so through `QCore.IndexExtent`'s verdict table it
discharges nothing and accuses nothing, and re-pinning after review restores
the verdict the declaration in force as written gives.

What is proved:

* the pin decision is in force exactly under the four conditions, so a
  changed clause or a changed body is never in force;
* a declaration's standing is resolved exactly when it carries no review
  marker and its pin is in force; otherwise held; never a source fact;
* a judgement relying on a held declaration is never a violation, and a
  function with one is never `Safe` (`held_never_safe`);
* re-pinning restores: a `Pending` verdict whose held judgements are put in
  force as written becomes `Safe`; recording another reviewed identity never
  takes an in-force pin out of force.

What this is not: a model of the lexical site scanner (identities are abstract
values compared for equality), of the C bodies, or of which obligations a
clause decides (the consumer's judgements are the premise). The Rust
correspondence is bound by `decisionVectors`
(`crates/qcore-audit/tests/bounds_pin_hold.rs`, `decision_vectors_match_lean`)
and `standingVectors` (`crates/qcore-contract/tests/bounds_contract.rs`,
`a_pin_hold_holds_the_declaration_and_changes_the_digest`). -/
namespace QCore.BoundsPinHold

open QCore.IndexExtent

/-- Why a pin is not in force (C06.2's reasons). -/
inductive Reason where
  | noLock
  | notInLock
  | contractChanged
  | sourceChanged
  deriving DecidableEq, Repr

/-- How a pin stands against its lock. -/
inductive Pin where
  | inForce
  | mismatch (r : Reason)
  deriving DecidableEq, Repr

/-- A lock entry: the contract digest it was reviewed with and the reviewed
identities (abstract source identities). -/
structure Entry where
  digest : Nat
  identities : List Nat
  deriving DecidableEq, Repr

/-- `pin_standing` for a pinned (non-tip) entry: the lock covering the module
(`none` when no lock), the entry for the function in it, the declaration's
digest and the audited identity. -/
def pinDecision (lock : Option (List (Nat × Entry))) (function digest sites : Nat) : Pin :=
  match lock with
  | none => .mismatch .noLock
  | some entries =>
    match entries.lookup function with
    | none => .mismatch .notInLock
    | some e =>
      if e.digest ≠ digest then .mismatch .contractChanged
      else if e.identities = [] then .mismatch .notInLock
      else if sites ∈ e.identities then .inForce
      else .mismatch .sourceChanged

/-- In force exactly when a lock covers it, the lock has its entry, the digest
is the reviewed one and the audited identity is a reviewed identity. -/
theorem in_force_iff (lock : Option (List (Nat × Entry))) (f d s : Nat) :
    pinDecision lock f d s = .inForce ↔
      ∃ entries e, lock = some entries ∧ entries.lookup f = some e ∧ e.digest = d ∧
        s ∈ e.identities := by
  constructor
  · intro h
    cases hl : lock with
    | none => simp [pinDecision, hl] at h
    | some entries =>
      cases he : entries.lookup f with
      | none => simp [pinDecision, hl, he] at h
      | some e =>
        simp only [pinDecision, hl, he] at h
        by_cases hd : e.digest ≠ d
        · simp [hd] at h
        · by_cases hi : e.identities = []
          · simp [hd, hi] at h
          · by_cases hs : s ∈ e.identities
            · exact ⟨entries, e, rfl, he, by simpa using hd, hs⟩
            · simp [hd, hi, hs] at h
  · rintro ⟨entries, e, rfl, he, hd, hs⟩
    have hi : e.identities ≠ [] := by
      intro hn
      rw [hn] at hs
      simp at hs
    simp [pinDecision, he, hd, hi, hs]

/-- A changed clause (a digest other than the reviewed one) is never in force. -/
theorem changed_clause_never_in_force (entries : List (Nat × Entry)) (f d s : Nat) (e : Entry)
    (he : entries.lookup f = some e) (hd : e.digest ≠ d) :
    pinDecision (some entries) f d s = .mismatch .contractChanged := by
  simp [pinDecision, he, hd]

/-- A changed body (an audited identity no review recorded) is never in force. -/
theorem changed_body_never_in_force (lock : Option (List (Nat × Entry))) (f d s : Nat)
    (h : ∀ entries e, lock = some entries → entries.lookup f = some e → s ∉ e.identities) :
    pinDecision lock f d s ≠ .inForce := by
  intro hf
  obtain ⟨entries, e, hl, he, _, hs⟩ := (in_force_iff lock f d s).1 hf
  exact h entries e hl he hs

/-- No lock is never in force. -/
theorem no_lock_never_in_force (f d s : Nat) : pinDecision none f d s = .mismatch .noLock := rfl

/-- Recording another reviewed identity never takes an in-force pin out of
force. -/
theorem another_identity_keeps (f d s t : Nat) (e : Entry) (rest : List (Nat × Entry))
    (h : pinDecision (some ((f, e) :: rest)) f d s = .inForce) :
    pinDecision (some ((f, { e with identities := t :: e.identities }) :: rest)) f d s = .inForce := by
  obtain ⟨entries, e', hl, he, hd, hs⟩ := (in_force_iff _ f d s).1 h
  simp only [Option.some.injEq] at hl
  subst hl
  simp [List.lookup] at he
  subst he
  exact (in_force_iff _ f d s).2
    ⟨_, { e with identities := t :: e.identities }, rfl, by simp [List.lookup], hd,
      List.mem_cons_of_mem _ hs⟩

/-- Re-pinning after review: an entry recording the audited identity with the
current digest is in force. -/
theorem repin_in_force (f d s : Nat) (ids : List Nat) (rest : List (Nat × Entry)) :
    pinDecision (some ((f, ⟨d, s :: ids⟩) :: rest)) f d s = .inForce :=
  (in_force_iff _ f d s).2 ⟨_, ⟨d, s :: ids⟩, rfl, by simp [List.lookup], rfl, List.mem_cons_self⟩

/-! The declaration's standing. -/

/-- `BoundsFunctionContract::held` with the pin hold: a review marker or a pin
out of force holds the whole declaration. -/
def standing (reviewHeld : Bool) (pin : Pin) : Standing :=
  if reviewHeld then .held
  else match pin with
    | .inForce => .resolved
    | .mismatch _ => .held

theorem standing_resolved_iff (r : Bool) (p : Pin) :
    standing r p = .resolved ↔ r = false ∧ p = .inForce := by
  cases r <;> cases p <;> simp [standing]

theorem a_mismatch_is_held (r : Bool) (x : Reason) : standing r (.mismatch x) = .held := by
  cases r <;> simp [standing]

theorem a_declaration_is_never_a_source_fact (r : Bool) (p : Pin) : standing r p ≠ .source := by
  cases r <;> cases p <;> simp [standing]

/-- One obligation relying on the declaration: decided inside under it, or
not decided. The pair is (actual, the declaration in force as written). -/
def judge (s : Standing) (decided : Bool) : Outcome × Outcome :=
  if !decided then (.unresolved 0, .unresolved 0)
  else match s with
    | .held => (.held 0, .inBounds)
    | _ => (.inBounds, .inBounds)

/-- A held declaration accuses nothing: its judgements are never violations. -/
theorem judge_never_accuses (s : Standing) (d : Bool) : (judge s d).1 ≠ .violation true := by
  cases d <;> cases s <;> simp [judge]

/-- A function relying on a declaration whose pin is out of force is never
`Safe`. -/
theorem pin_mismatch_never_safe (js : List (Outcome × Outcome)) (r : Nat) (rv : Bool)
    (x : Reason) (h : judge (standing rv (.mismatch x)) true ∈ js) :
    boundsVerdict js r ≠ .safe := by
  apply held_never_safe js r _ h
  simp [a_mismatch_is_held, judge, Outcome.isHeld]

theorem clear_not_held (o : Outcome) (h : o.clear = true) : o.isHeld = false := by
  cases o <;> simp_all [Outcome.clear, Outcome.isHeld]

theorem clear_not_violation (o : Outcome) (h : o.clear = true) : (o == .violation true) = false := by
  cases o <;> simp_all [Outcome.clear]

/-- Put every held judgement in force as written. -/
def release (j : Outcome × Outcome) : Outcome × Outcome :=
  if j.1.isHeld then (j.2, j.2) else j

/-- Re-pinning restores: a `Pending` verdict (held declarations the sole
obstacle) becomes `Safe` once they are in force as written. -/
theorem repinned_pending_is_safe (js : List (Outcome × Outcome)) (r : Nat)
    (h : boundsVerdict js r = .pending) : boundsVerdict (js.map release) r = .safe := by
  obtain ⟨hr, h1, h2⟩ := pending_only_as_the_sole_obstacle js r h
  have hclear : ∀ j ∈ js.map release, j.1.clear = true := by
    intro j hj
    rw [List.mem_map] at hj
    obtain ⟨k, hk, rfl⟩ := hj
    by_cases hh : k.1.isHeld = true
    · simp only [release, hh, ite_true]
      exact h2 k hk
    · simp only [release, hh]
      rcases h1 k hk with c | c
      · simpa using c
      · exact absurd c hh
  have ha : (js.map release).any (fun j => j.1 == .violation true) = false := by
    rw [List.any_eq_false]
    intro j hj
    rw [clear_not_violation _ (hclear j hj)]
    simp
  have hb : (js.map release).any (fun j => j.1.isHeld) = false := by
    rw [List.any_eq_false]
    intro j hj
    rw [clear_not_held _ (hclear j hj)]
    simp
  have hc : (r != 0 || (js.map release).any (fun j => !j.1.clear && !j.1.isHeld)) = false := by
    have : (js.map release).any (fun j => !j.1.clear && !j.1.isHeld) = false := by
      rw [List.any_eq_false]
      intro j hj
      simp [hclear j hj]
    simp [hr, this]
  unfold boundsVerdict
  rw [ha, hb, hc]
  cases ((js.map release).all (fun j => j.2.clear)) <;> rfl

/-! Vectors shared with Rust (see the module header). -/

/-- (lock, function, digest, audited identity, decision). -/
def decisionVectors : List (Option (List (Nat × Entry)) × Nat × Nat × Nat × Pin) := [
  (none, 1, 7, 3, .mismatch .noLock),
  (some [], 1, 7, 3, .mismatch .notInLock),
  (some [(2, ⟨7, [3]⟩)], 1, 7, 3, .mismatch .notInLock),
  (some [(1, ⟨8, [3]⟩)], 1, 7, 3, .mismatch .contractChanged),
  (some [(1, ⟨7, []⟩)], 1, 7, 3, .mismatch .notInLock),
  (some [(1, ⟨7, [4]⟩)], 1, 7, 3, .mismatch .sourceChanged),
  (some [(1, ⟨7, [3]⟩)], 1, 7, 3, .inForce),
  (some [(1, ⟨7, [4, 3]⟩)], 1, 7, 3, .inForce),
  (some [(1, ⟨8, [4, 3]⟩)], 1, 7, 3, .mismatch .contractChanged)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (l, f, d, s, p) => pinDecision l f d s == p) = true := by decide

theorem decision_vectors_count : decisionVectors.length = 9 := by decide

/-- (review marker, pin, standing). -/
def standingVectors : List (Bool × Pin × Standing) := [
  (false, .inForce, .resolved),
  (true, .inForce, .held),
  (false, .mismatch .noLock, .held),
  (false, .mismatch .notInLock, .held),
  (false, .mismatch .contractChanged, .held),
  (false, .mismatch .sourceChanged, .held),
  (true, .mismatch .sourceChanged, .held)
]

theorem standing_vectors_hold :
    standingVectors.all (fun (r, p, s) => standing r p == s) = true := by decide

theorem standing_vectors_count : standingVectors.length = 7 := by decide

end QCore.BoundsPinHold
