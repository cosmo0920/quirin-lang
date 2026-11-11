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

import QCore.TypedAccessPathStatements

/-!
# The common typed place on a tracked root (card CG10-COMMON-TYPED-PLACE, rule 10t)

The abstract rule implemented by `crates/qcore-c-gateway/src/typed_place.rs`
(`read_place`, `decide`, `tracked_chain`) and its hook in
`typed_access_path.rs` (`Reader::path`).

## The place

A place is a root followed by steps. The root is one parameter or automatic
local (its Clang DeclID); a step is `->F`, `.F`, a subscript or an indirection,
and a subscript or indirection records whether it reads a pointer value from
memory (a *hop*) or indexes an array (no hop). Every other form is refused:
`readPlace` is `none` on any unlisted form.

## The decision

W46-ACCESS-PATH-STATEMENTS admitted a mixed path only on a never-bound root
(`TypedAccessPathStatements.admits`, its conjunct `!mixedPath || untrackedRoot`).
On any other pointer root this rule admits the same path when the pointers it
reads through are the root alone (`rootOnly`) or the root and the direct member
place `R->F` (`directMember`), and lowers it to the existing chain obligation
(`IndirectChainAccess.Obligation`) with `rootNeverBound := false`, so the flow
decides: the root must be one known, non-null, live identity and `R->F` must
carry the admitted store's evidence. Every other shape is refused.

## What this is not

A model of the decision and its lowering. Faithful Clang extraction (types,
field ids, cast kinds, DeclIDs) and the Rust-Lean correspondence are review
premises (trust boundary 28); `decisionVectors` binds the Rust `decide` table
mechanically (`tests/unit/typed_place.rs`). Bounds, alignment and the liveness
of untracked objects are outside the property, as in the chain model.
-/

namespace QCore.TypedPlace

open QCore.IndirectScalarAccess (Binding Diagnostic Access)
open QCore.TypedAccessPathStatements (Shape Statement)

/-! ## The place grammar -/

inductive RootKind where
  | pointer
  | record
  | array
  deriving DecidableEq, Repr

inductive Step where
  | arrow
  | dot
  | indexPointer
  | indexArray
  | derefPointer
  | derefArray
  deriving DecidableEq, Repr

/-- A step reads a pointer value from memory and dereferences it. -/
def Step.hop : Step → Bool
  | .arrow => true
  | .indexPointer => true
  | .derefPointer => true
  | _ => false

/-- What the Clang reader sees at one position: a listed step or anything else. -/
inductive Form where
  | step (s : Step)
  | other
  deriving DecidableEq, Repr

def readForm : Form → Option Step
  | .step s => some s
  | .other => none

/-- `read_place`: every form must be listed. -/
def readPlace (forms : List Form) : Option (List Step) :=
  forms.mapM readForm

/-- Every unlisted place form is refused. -/
theorem every_unlisted_form_refuses (forms : List Form) (h : Form.other ∈ forms) :
    readPlace forms = none := by
  induction forms with
  | nil => cases h
  | cons f rest ih =>
    cases f with
    | other => simp [readPlace, List.mapM_cons, readForm]
    | step s =>
      have hrest : Form.other ∈ rest := by
        rcases List.mem_cons.mp h with h | h
        · cases h
        · exact h
      have := ih hrest
      simp only [readPlace] at this
      simp [readPlace, List.mapM_cons, readForm, this]

/-- A place of listed forms reads as exactly those steps. -/
theorem a_listed_place_reads_its_steps (steps : List Step) :
    readPlace (steps.map Form.step) = some steps := by
  induction steps with
  | nil => rfl
  | cons s rest ih =>
    simp only [readPlace] at ih
    simp [readPlace, List.mapM_cons, readForm, ih]

/-! ## The decision -/

inductive Decision where
  | rootOnly
  | directMember
  | recordRoot
  | arrayRoot
  | elementPlace
  | deeperPlace
  | noRootHop
  deriving DecidableEq, Repr

def Decision.admits : Decision → Bool
  | .rootOnly => true
  | .directMember => true
  | _ => false

/-- The positions of the hops, from `index`. -/
def hopsFrom : Nat → List Step → List Nat
  | _, [] => []
  | index, s :: rest => if s.hop then index :: hopsFrom (index + 1) rest else hopsFrom (index + 1) rest

/-- `decide` in `typed_place.rs`. -/
def decidePlace (root : RootKind) (steps : List Step) : Decision :=
  match root with
  | .record => .recordRoot
  | .array => .arrayRoot
  | .pointer =>
    match hopsFrom 0 steps with
    | [0] => .rootOnly
    | 0 :: 1 :: _ => if steps.head? = some .arrow then .directMember else .elementPlace
    | 0 :: _ :: _ => .deeperPlace
    | _ => .noRootHop

/-- The Rust `DECISION_VECTORS`, spelling by spelling. -/
def decisionVectors : List (String × RootKind × List Step × Decision) := [
  ("s->a", .pointer, [.arrow], .rootOnly),
  ("s->in.x", .pointer, [.arrow, .dot], .rootOnly),
  ("s->arr[i]", .pointer, [.arrow, .indexArray], .rootOnly),
  ("s->in.arr[i]", .pointer, [.arrow, .dot, .indexArray], .rootOnly),
  ("s[i].a", .pointer, [.indexPointer, .dot], .rootOnly),
  ("(*s).in.x", .pointer, [.derefPointer, .dot, .dot], .rootOnly),
  ("s->next->a", .pointer, [.arrow, .arrow], .directMember),
  ("s->ints[i]", .pointer, [.arrow, .indexPointer], .directMember),
  ("*s->ip", .pointer, [.arrow, .derefPointer], .directMember),
  ("s->next->in.x", .pointer, [.arrow, .arrow, .dot], .directMember),
  ("s->next->next->a", .pointer, [.arrow, .arrow, .arrow], .directMember),
  ("s->in.next->a", .pointer, [.arrow, .dot, .arrow], .deeperPlace),
  ("s->links[i]->a", .pointer, [.arrow, .indexArray, .arrow], .deeperPlace),
  ("v[i]->a", .pointer, [.indexPointer, .arrow], .elementPlace),
  ("(*v)->a", .pointer, [.derefPointer, .arrow], .elementPlace),
  ("o.in.x", .record, [.dot, .dot], .recordRoot),
  ("t[i].a", .array, [.indexArray, .dot], .arrayRoot)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (_, root, steps, decision) => decidePlace root steps == decision) =
      true := by
  decide

/-- A record-object or array root keeps the never-bound premise: this rule
never admits it. -/
theorem a_record_or_array_root_is_refused (root : RootKind) (steps : List Step)
    (h : root ≠ .pointer) : (decidePlace root steps).admits = false := by
  cases root <;> simp_all [decidePlace, Decision.admits]

/-- A pointer loaded from below the root's first selection, other than `R->F`
itself, is refused. -/
theorem a_deeper_place_is_refused :
    (decidePlace .pointer [.arrow, .dot, .arrow]).admits = false ∧
      (decidePlace .pointer [.arrow, .indexArray, .arrow]).admits = false ∧
      (decidePlace .pointer [.indexPointer, .arrow]).admits = false ∧
      (decidePlace .pointer [.derefPointer, .arrow]).admits = false := by
  decide

/-! ## The lowering to the chain obligation -/

/-- The obligation this rule lowers an admitted place to: the root (never read
as never bound), the place `R->F` and one link per hop after the root's. -/
def obligation (root firstPlace : Nat) (links : List Nat) : IndirectChainAccess.Obligation :=
  { root := root, dereferencedRoot := true, rootNeverBound := false,
    firstPlace := firstPlace, links := links }

/-- A root-only place has no link. -/
def lowerPlace (d : Decision) (root firstPlace : Nat) (links : List Nat) :
    Option IndirectChainAccess.Obligation :=
  match d with
  | .rootOnly => some (obligation root firstPlace [])
  | .directMember => if links = [] then none else some (obligation root firstPlace links)
  | _ => none

theorem a_refused_decision_has_no_obligation (d : Decision) (root firstPlace : Nat)
    (links : List Nat) (h : d.admits = false) : lowerPlace d root firstPlace links = none := by
  cases d <;> simp_all [Decision.admits, lowerPlace]

/-- The soundness interlock (rules 10l, 10o): this route never serves the
never-bound premise, so a root with no binding on some path is refused, never
read as untracked. -/
theorem an_unbound_root_is_refused (s : IndirectChainAccess.State) (root firstPlace : Nat)
    (links : List Nat) (hb : s.base.binding root = none) :
    IndirectChainAccess.resolves s (obligation root firstPlace links) = false :=
  IndirectChainAccess.an_unbound_root_the_body_may_bind_refuses s _ rfl rfl hb

/-- A target-free root (an opaque read, a borrowed alias) is refused too. -/
theorem a_target_free_root_is_refused (s : IndirectChainAccess.State) (root firstPlace : Nat)
    (links : List Nat) (b : Binding) (hb : s.base.binding root = some b)
    (ht : b.targets = []) :
    IndirectChainAccess.resolves s (obligation root firstPlace links) = false :=
  IndirectChainAccess.a_target_free_root_the_body_may_bind_refuses s _ b rfl hb ht rfl

/-- Liveness comes from the custody state: a released (or reallocated, or
otherwise not exactly live) root is refused. -/
theorem a_root_that_is_not_live_is_refused (s : IndirectChainAccess.State)
    (root firstPlace site : Nat) (links : List Nat)
    (hb : s.base.binding root = some ⟨[site], false, false, .identity⟩)
    (hdead : IndirectChainAccess.exactlyLive s.base site = false) :
    IndirectChainAccess.resolves s (obligation root firstPlace links) = false := by
  simp [IndirectChainAccess.resolves, IndirectChainAccess.rootOwner, obligation, hb,
    IndirectChainAccess.singleIdentity, hdead]

/-- A possibly-null root is refused. -/
theorem a_nullable_root_is_refused (s : IndirectChainAccess.State) (root firstPlace : Nat)
    (links : List Nat) (b : Binding) (hb : s.base.binding root = some b)
    (hnull : b.mayBeNull = true) :
    IndirectChainAccess.resolves s (obligation root firstPlace links) = false :=
  IndirectChainAccess.a_nullable_root_refuses s _ b rfl hb hnull

/-- Per-hop sufficiency, root only: a root that is one known, non-null, live
identity discharges the place's whole obligation. -/
theorem a_live_root_discharges_a_root_only_place (s : IndirectChainAccess.State)
    (root firstPlace site : Nat)
    (hb : s.base.binding root = some ⟨[site], false, false, .identity⟩)
    (hlive : IndirectChainAccess.exactlyLive s.base site = true) :
    IndirectChainAccess.resolves s (obligation root firstPlace []) = true := by
  simp [IndirectChainAccess.resolves, IndirectChainAccess.rootOwner, obligation, hb,
    IndirectChainAccess.singleIdentity, hlive, IndirectChainAccess.walk]

/-- Per-hop sufficiency, direct member: a live root whose place `R->F` holds the
live child its admitted store linked discharges the obligation. -/
theorem a_live_linked_member_discharges_a_direct_member_place (s : IndirectChainAccess.State)
    (root firstPlace field site child : Nat)
    (hb : s.base.binding root = some ⟨[site], false, false, .identity⟩)
    (hlive : IndirectChainAccess.exactlyLive s.base site = true)
    (hp : s.base.binding firstPlace = some ⟨[child], false, false, .identity⟩)
    (hlink : s.fieldLink site field = some [child])
    (hchild : IndirectChainAccess.exactlyLive s.base child = true) :
    IndirectChainAccess.resolves s (obligation root firstPlace [field]) = true := by
  simp [IndirectChainAccess.resolves, IndirectChainAccess.rootOwner, obligation, hb,
    IndirectChainAccess.singleIdentity, hlive, IndirectChainAccess.walk,
    IndirectChainAccess.linkOwner, hp, hlink, hchild]

/-- A tracked owner one hop past `R->F` is refused: no deeper place is bound. -/
theorem a_tracked_second_link_is_refused (s : IndirectChainAccess.State)
    (root firstPlace field next site child : Nat)
    (hb : s.base.binding root = some ⟨[site], false, false, .identity⟩)
    (hlive : IndirectChainAccess.exactlyLive s.base site = true)
    (hp : s.base.binding firstPlace = some ⟨[child], false, false, .identity⟩)
    (hlink : s.fieldLink site field = some [child])
    (hchild : IndirectChainAccess.exactlyLive s.base child = true) :
    IndirectChainAccess.resolves s (obligation root firstPlace [field, next]) = false := by
  simp [IndirectChainAccess.resolves, IndirectChainAccess.rootOwner, obligation, hb,
    IndirectChainAccess.singleIdentity, hlive, IndirectChainAccess.walk,
    IndirectChainAccess.linkOwner, hp, hlink, hchild]

/-! ## The statement shape with rule 10t -/

/-- `TypedAccessPathStatements.admits` with its mixed-path conjunct widened by
this rule's decision: a mixed path needs a never-bound root **or** an admitted
tracked-root decision. Every other conjunct is unchanged. -/
def admits (p : Shape) (d : Decision) : Bool :=
  p.exactType && p.knownFieldOwner && p.knownValueType && !p.unionField && !p.bitfield &&
  !p.incompleteRecord && !p.castInPath && !p.pointerArithmetic &&
  !p.volatileAtomic && !p.pointerStore && !p.call && !p.otherForm &&
  (!p.mixedPath || p.untrackedRoot || d.admits)

/-- Rule 10t only adds: every W46 admission stands. -/
theorem every_w46_admission_stands (p : Shape) (d : Decision)
    (h : TypedAccessPathStatements.admits p = true) : admits p d = true := by
  simp only [TypedAccessPathStatements.admits, admits] at h ⊢
  cases hm : p.mixedPath <;> cases hu : p.untrackedRoot <;> simp_all

/-- On a never-bound root the decision is not read: the untracked case is
exactly W46's. -/
theorem the_untracked_case_is_w46s (p : Shape) (d : Decision) (hu : p.untrackedRoot = true) :
    admits p d = TypedAccessPathStatements.admits p := by
  simp [admits, TypedAccessPathStatements.admits, hu]

/-- Every refusal W46 has other than the tracked mixed root stands. -/
theorem a_pointer_store_stays_refused (p : Shape) (d : Decision) (h : p.pointerStore = true) :
    admits p d = false := by
  simp [admits, h]

theorem a_refused_tracked_decision_keeps_the_refusal (p : Shape) (d : Decision)
    (hm : p.mixedPath = true) (hr : p.untrackedRoot = false) (hd : d.admits = false) :
    admits p d = false := by
  simp [admits, hm, hr, hd]

theorem any_unlisted_form_stays_refused (p : Shape) (d : Decision) (h : p.otherForm = true) :
    admits p d = false := by
  simp [admits, h]

/-- The lowering under rule 10t: W46's transfer, unchanged. -/
def lower (s : IndirectChainAccess.State) (p : Statement) (d : Decision) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  if admits p.shape d then some (IndirectChainAccess.transfer s p.scalar p.hops) else none

/-- Per-hop obligations suffice for the place access to be defined: every base
and every hop resolving leaves no diagnostic. -/
theorem every_hop_discharged_defines_the_access (s : IndirectChainAccess.State)
    (p : Statement) (d : Decision) (ha : admits p.shape d = true)
    (hb : ∀ base ∈ p.scalar.bases, IndirectScalarAccess.resolves s.base base = true)
    (hh : ∀ hop ∈ p.hops, IndirectChainAccess.resolves s hop = true) :
    ∃ after, lower s p d = some (after, []) := by
  have hbase := IndirectScalarAccess.an_admitted_statement_adds_no_diagnostic s.base p.scalar hb
  have hchain := IndirectChainAccess.admitted_chains_add_no_diagnostic s p.scalar p.hops hh
  refine ⟨(IndirectChainAccess.transfer s p.scalar p.hops).1, ?_⟩
  have hempty : (IndirectChainAccess.transfer s p.scalar p.hops).2 = [] := by
    rw [hchain, hbase]
  cases htrans : IndirectChainAccess.transfer s p.scalar p.hops with
  | mk after diagnostics =>
    simp only [htrans] at hempty
    subst diagnostics
    simp [lower, ha, htrans]

/-- A refused hop leaves a reason: the function cannot be `Safe`. -/
theorem a_refused_hop_is_never_safe (s : IndirectChainAccess.State) (p : Statement)
    (d : Decision) (hop : IndirectChainAccess.Obligation) (hmem : hop ∈ p.hops)
    (hr : IndirectChainAccess.resolves s hop = false) :
    ∀ after diagnostics, lower s p d = some (after, diagnostics) →
      IndirectScalarAccess.safe diagnostics = false := by
  intro after diagnostics hl
  simp only [lower] at hl
  split at hl
  · cases hl
    exact IndirectChainAccess.a_refused_chain_is_never_safe s p.scalar p.hops hop hmem hr
  · cases hl

/-- A read or store of a non-pointer value (no frame) leaves the custody frame
unchanged: statuses, markers, field links and every binding but the scalar
destination. -/
theorem a_nonpointer_access_preserves_the_frame (s : IndirectChainAccess.State)
    (p : Statement) (d : Decision) (ha : admits p.shape d = true)
    (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lower s p d = some (after, diagnostics) ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink ∧
      after.base.status = s.base.status ∧
      (∀ name, p.scalar.destination ≠ some name → after.base.binding name = s.base.binding name) := by
  refine ⟨(IndirectChainAccess.transfer s p.scalar p.hops).1,
    (IndirectChainAccess.transfer s p.scalar p.hops).2, ?_, rfl, ?_, ?_, ?_⟩
  · simp [lower, ha]
  · exact IndirectChainAccess.field_links_are_unchanged s p.scalar p.hops
  · exact (IndirectChainAccess.the_statuses_and_markers_are_unchanged s p.scalar p.hops).1
  · intro name hd
    rw [IndirectChainAccess.a_chain_changes_no_binding]
    exact IndirectScalarAccess.without_a_frame_only_the_destination_changes s.base p.scalar name hf hd

/-- A whole pointer-valued read binds its destination as P2's borrowed alias. -/
def lowerBorrowed (s : IndirectChainAccess.State) (p : Statement) (d : Decision)
    (destination : Nat) : Option (IndirectChainAccess.State × List Diagnostic) :=
  if p.shape.borrowedPointerValue && p.scalar.frame.isNone then
    (lower s p d).map fun (after, diagnostics) =>
      (TypedAccessPathStatements.bindBorrowed after destination, diagnostics)
  else none

/-- A pointer value read through a tracked place is a borrowed alias: no
target, so no custody. -/
theorem a_pointer_read_is_a_borrow_without_custody (s : IndirectChainAccess.State)
    (p : Statement) (d : Decision) (destination : Nat) (ha : admits p.shape d = true)
    (hp : p.shape.borrowedPointerValue = true) (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lowerBorrowed s p d destination = some (after, diagnostics) ∧
      (after.base.binding destination).map Binding.targets = some [] ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink := by
  refine ⟨TypedAccessPathStatements.bindBorrowed
      (IndirectChainAccess.transfer s p.scalar p.hops).1 destination,
    (IndirectChainAccess.transfer s p.scalar p.hops).2, ?_, ?_, rfl, rfl⟩
  · simp [lowerBorrowed, lower, ha, hp, hf]
  · simp [TypedAccessPathStatements.bindBorrowed, TypedAccessPathStatements.borrowedAlias]

/-- A pointer-valued store is refused, so no borrowed lowering carries a write
frame. -/
theorem a_borrowed_read_with_a_frame_refuses (s : IndirectChainAccess.State) (p : Statement)
    (d : Decision) (destination : Nat) (frame : Nat → Bool) (hf : p.scalar.frame = some frame) :
    lowerBorrowed s p d destination = none := by
  simp [lowerBorrowed, hf]

/-! ## Non-vacuity -/

/-- The M01 control: root 1 is bound to live site 7 (`IndirectChainAccess`'s
example); its root-only places `s->in.x` and `s->arr[1]` are admitted. -/
theorem the_m01_control_is_admitted :
    decidePlace .pointer [.arrow, .dot] = .rootOnly ∧
      IndirectChainAccess.resolves IndirectChainAccess.example_state (obligation 1 2 []) = true := by
  decide

/-- The same place on the never-bound formal 0, now read by this route, is
refused: the route does not serve the never-bound premise. -/
theorem the_route_refuses_an_unbound_formal :
    IndirectChainAccess.resolves IndirectChainAccess.example_state (obligation 0 99 []) = false := by
  decide

end QCore.TypedPlace
