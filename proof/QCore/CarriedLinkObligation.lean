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

import QCore.ElementLinkPremises

/-!
# A formal's link obligation, carried to the caller (card FB-RETRY-OR6-ELEMENT-IDENTITY)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_carried_link.rs` (gateway rule 91), with
its hooks in `legacy_container_publication.rs` (rule 43's held arm),
`legacy_transfer.rs` (the entry seed, the join, the kill policy and the
exit observation) and `legacy_summary.rs` (the W4 probe and candidate).

Rule 46 (3) admits a held object at an element destroyer only when the body
initialised the destroyer's link member itself (`QCore.ElementLinkPremises`).
A formal's links are its caller's. Rule 91 makes them the caller's
obligation: a contract stating `resource_unpublished(p, M)` at a bound
position `p` gives its body a `ready M` record at `p` on entry, which holds
because every caller discharges it (`the_seeded_record_holds`). The record
is forgotten at every statement that may write the object, exactly as rule
46's facts are, and the join keeps only records both sides carry
(`retaining_keeps_every_ready_record`, `the_join_keeps_every_ready_record`).
The held arm admits on a `ready` record of the declared member
(`the_carried_admission_is_sound`), and only there (`a_spent_record_admits_nothing`,
`a_memberless_destroyer_takes_no_carried_object`). A carried formal is
observed released at an exit only when its record there is `spent`
(`a_carried_release_is_the_destroyers`); a formal no clause carries is
observed as before (`an_uncarried_disposition_is_unchanged`).

**The caller's discharge.** The published summary is an element destroyer's,
so each call is read by rules 34, 43 and 46. Each way the caller may pass is
a way the precondition holds at the call (`the_discharge_establishes_the_precondition`):
its own initialisation (rule 46 (3)), an element its list links through `M`
(rule 46 (2), under P-CRR-LINKED), or its own formal carried again (this
rule). A caller that can do none refuses there, so a body's admission never
rests on an undischarged clause (`no_discharge_no_admission`).

## What this is not

A model of the decision. That the summary's candidates are read off the
destroyer calls, that the Rust kill policy over-approximates a statement's
writes, that every consumer of a summary with a container effect applies or
refuses it, and the correspondence with the compiled Rust are review premises
(trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/held_formal_link_obligation.rs`. P-CRR-LINKED
(an element of an iterated list is linked, so its links are valid) is rule
10f's, still stated.
-/

namespace QCore.CarriedLinkObligation

open QCore.ElementLinkPremises

/-- A carried record at a formal: `ready m` until a statement may write the
object, `spent m` once a destroyer unlinking `m` took it. -/
inductive Carried where
  | ready (m : String)
  | spent (m : String)
  deriving DecidableEq, Repr

/-- The records of one path, by object. -/
abbrev Records := List (Nat × Carried)

/-- Every `ready` record's member is unlinkable in `h`. -/
def readyHolds (rs : Records) (h : Heap) : Prop :=
  ∀ x m, (x, Carried.ready m) ∈ rs → unlinkDefined (h x m) = true

/-- The caller's obligation for position `x` and member `m`. -/
def precondition (h : Heap) (x : Nat) (m : String) : Prop :=
  unlinkDefined (h x m) = true

/-- The entry seed: one `ready` record per carried position. -/
def seed (carried : List (Nat × String)) : Records :=
  carried.map (fun c => (c.1, Carried.ready c.2))

theorem the_seeded_record_holds (carried : List (Nat × String)) (h : Heap)
    (pre : ∀ c ∈ carried, precondition h c.1 c.2) : readyHolds (seed carried) h := by
  intro x m hm
  simp only [seed, List.mem_map] at hm
  obtain ⟨c, hc, he⟩ := hm
  simp only [Prod.mk.injEq, Carried.ready.injEq] at he
  obtain ⟨rfl, rfl⟩ := he
  exact pre c hc

/-- Whether a record is `spent`. -/
def isSpent : Carried → Bool
  | .spent _ => true
  | .ready _ => false

/-- `retain`: rule 46's kill policy for the `ready` records; `spent` ones
survive every statement. -/
def retain : Stmt → Records → Records
  | .inert, rs => rs
  | .call reached, rs => rs.filter (fun r => isSpent r.2 || !reached.contains r.1)
  | .other, rs => rs.filter (fun r => isSpent r.2)

theorem retaining_keeps_every_ready_record (s : Stmt) (rs : Records) (h h' : Heap)
    (hold : readyHolds rs h) (w : writesOnly s h h') : readyHolds (retain s rs) h' := by
  intro x m hm
  cases s with
  | inert => simp only [writesOnly] at w; subst w; exact hold x m hm
  | call reached =>
    simp only [retain, List.mem_filter, isSpent, Bool.false_or] at hm
    obtain ⟨hmem, hnot⟩ := hm
    have hx : x ∉ reached := by simpa using hnot
    rw [w x hx m]
    exact hold x m hmem
  | other => simp [retain, isSpent] at hm

/-- The join keeps a record only when both paths carry the same one. -/
def join (a b : Records) : Records := a.filter (fun r => b.contains r)

theorem the_join_keeps_every_ready_record (a b : Records) (h : Heap)
    (ha : readyHolds a h) : readyHolds (join a b) h := by
  intro x m hm
  simp only [join, List.mem_filter] at hm
  exact ha x m hm.1

theorem a_record_on_one_side_only_is_dropped (a : Records) (r : Nat × Carried) :
    r ∉ join a [] := by
  simp [join]

-- The admission.

/-- `decides`: the destroyer names the member a `ready` record carries. -/
def admits (link : Option String) : Option Carried → Bool
  | some (.ready m) => link == some m
  | _ => false

theorem the_carried_admission_is_sound (rs : Records) (h : Heap) (x : Nat)
    (link : Option String) (r : Carried) (hold : readyHolds rs h) (hr : (x, r) ∈ rs)
    (ha : admits link (some r) = true) :
    ∃ m, link = some m ∧ unlinkDefined (h x m) = true := by
  cases r with
  | spent m => simp [admits] at ha
  | ready m =>
    refine ⟨m, ?_, hold x m hr⟩
    simpa [admits] using ha

theorem a_spent_record_admits_nothing (link : Option String) (m : String) :
    admits link (some (.spent m)) = false := rfl

theorem a_memberless_destroyer_takes_no_carried_object (r : Option Carried) :
    admits none r = false := by
  cases r with
  | none => rfl
  | some r => cases r <;> rfl

theorem no_record_no_admission (link : Option String) : admits link none = false := rfl

/-- The admission spends the record. -/
def spend : Carried → Carried
  | .ready m => .spent m
  | r => r

theorem spending_admits_nothing_more (link : Option String) (r : Carried) :
    admits link (some (spend r)) = false := by
  cases r <;> rfl

-- The exit.

/-- `observed`: whether a disposition reads `Released`, for a formal the
contract carries (or not), with its record at the exit. -/
def observedReleased (carried : Bool) (r : Option Carried) (released : Bool) : Bool :=
  released && (!carried || match r with | some (.spent _) => true | _ => false)

theorem a_carried_release_is_the_destroyers (r : Option Carried) (released : Bool)
    (h : observedReleased true r released = true) : ∃ m, r = some (.spent m) := by
  cases r with
  | none => simp [observedReleased] at h
  | some r =>
    cases r with
    | ready m => simp [observedReleased] at h
    | spent m => exact ⟨m, rfl⟩

theorem an_uncarried_disposition_is_unchanged (r : Option Carried) (released : Bool) :
    observedReleased false r released = released := by
  simp [observedReleased]

theorem the_observation_only_refutes (carried : Bool) (r : Option Carried) (released : Bool)
    (h : observedReleased carried r released = true) : released = true := by
  simp only [observedReleased, Bool.and_eq_true] at h
  exact h.1

-- The caller's discharge.

/-- How a caller may pass the actual at a call of the published destroyer. -/
inductive Discharge where
  /-- Rule 46 (3): this caller initialised `m` itself (a recorded fact). -/
  | initialised (facts : List (Nat × String))
  /-- Rule 46 (2): an element its list links through `m`. -/
  | element (bindings : List String)
  /-- Rule 91: the caller's own formal, carried again. -/
  | carried (r : Option Carried)
  /-- Anything else. -/
  | other

/-- The caller's decision at the call (rules 46 and 91). -/
def discharges (link : Option String) (x : Nat) : Discharge → Bool
  | .initialised facts =>
    match link with
    | some m => facts.contains (x, m)
    | none => false
  | .element bindings => foreignAdmits link bindings
  | .carried r => admits link r
  | .other => false

/-- P-CRR-LINKED, as this model states it: an element bound through `m` of
an iterated list has valid `m` links. -/
def linkedValid (h : Heap) (x : Nat) (m : String) : Prop := unlinkDefined (h x m) = true

theorem the_discharge_establishes_the_precondition (h : Heap) (x : Nat) (m : String)
    (d : Discharge)
    (facts_hold : ∀ facts, d = .initialised facts → holds facts h)
    (linked : ∀ bindings, d = .element bindings → m ∈ bindings → linkedValid h x m)
    (carried_hold : ∀ r, d = .carried r → ∀ n, r = some (.ready n) → unlinkDefined (h x n) = true)
    (hd : discharges (some m) x d = true) : precondition h x m := by
  cases d with
  | initialised facts =>
    simp only [discharges, List.contains_iff_mem] at hd
    exact facts_hold facts rfl (x, m) hd
  | element bindings =>
    simp only [discharges, foreignAdmits, Bool.and_eq_true, Bool.not_eq_true',
      List.all_eq_true, beq_iff_eq] at hd
    obtain ⟨hne, hall⟩ := hd
    have hne' : bindings ≠ [] := by
      intro hnil; subst hnil; simp at hne
    obtain ⟨b, hb⟩ := List.exists_mem_of_ne_nil bindings hne'
    have : b = m := hall b hb
    subst this
    exact linked bindings rfl hb
  | carried r =>
    cases r with
    | none => simp [discharges, admits] at hd
    | some r =>
      cases r with
      | spent n => simp [discharges, admits] at hd
      | ready n =>
        have hn : n = m := by
          have : m = n := by simpa [discharges, admits] using hd
          exact this.symm
        subst hn
        exact carried_hold _ rfl n rfl
  | other => simp [discharges] at hd

theorem no_discharge_no_admission (link : Option String) (x : Nat) :
    discharges link x .other = false := rfl

-- The Rust vectors (`ADMIT_VECTORS`, `OBSERVE_VECTORS`, checked by the unit
-- tests of `crates/qcore-c-gateway/src/legacy_carried_link.rs`). A record
-- `(true, m)` is `ready m`, `(false, m)` is `spent m`.

def record : Option (Bool × String) → Option Carried
  | some (true, m) => some (.ready m)
  | some (false, m) => some (.spent m)
  | none => none

def admitVectors : List (Option String × Option (Bool × String) × Bool) :=
  [ (some "_head", some (true, "_head"), true),
    (some "_head", some (false, "_head"), false),
    (some "_head", some (true, "_rotate_head"), false),
    (some "_head", none, false),
    (none, some (true, "_head"), false) ]

def observeVectors : List (Bool × Option (Bool × String) × Bool × Bool) :=
  [ (true, some (false, "_head"), true, true),
    (true, some (true, "_head"), true, false),
    (true, none, true, false),
    (false, none, true, true),
    (true, none, false, false) ]

theorem vectors_hold :
    admitVectors.all (fun (l, r, out) => admits l (record r) == out) = true ∧
    observeVectors.all (fun (c, r, rel, out) => observedReleased c (record r) rel == out)
      = true := by
  decide

end QCore.CarriedLinkObligation
