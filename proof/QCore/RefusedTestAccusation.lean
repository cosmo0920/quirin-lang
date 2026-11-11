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
# A leak claim does not rest on a refused test (card SUP07-REFUSED-TEST-ACCUSATION)

The abstract rule implemented by `crates/qcore-c-gateway/src/legacy_refused_test.rs`
and its hook in the custody work list of `legacy_c.rs`, gateway rule 12e.

Rule 10b keeps a null test of an *unknown* binding from deciding anything but
the tested binding: the test reports `UnknownAlias`, narrows the name, and keeps
every site on both edges. On the **null** edge a site the name may denote then
stays live, and `report_leaks` accused it at every exit that edge reaches.
Monkey's `mk_http_error_page` (`mk_server/mk_http.c:392-399`) was a provisional
`Unsafe` for that reason only: `temp` joins a refused `mk_ptr_to_buf` with a
nullable `mk_string_dup` copy, `if (!temp) return -1;` is refused, and the copy
was accused at the `return -1`, where it is NULL or was never made.

## What is proved

* **The executions.** If joined provenance ties a site `S` to the tested name
  `X` (on every execution on which `S` exists, `X` denotes it), no execution
  that takes the null edge owes `S` anything (`tied_null_edge_owes_nothing`).
  Without the tie an execution on that edge may owe it
  (`without_the_tie_a_null_edge_may_owe`): the tie is the premise, and the
  refused test alone establishes nothing. `mk_http_error_page`'s executions are
  tied and owe nothing on the null edge.
* **The decision.** `edgeStep` settles a site (it does not exist on that edge)
  exactly on a refused test's null edge, for a site the tested binding may
  denote and provenance ties to the name (`settle_iff`); a decided test, the
  other edge, an untied site and an untargeted one are kept as they were. The
  Rust table `EDGE_VECTORS` is `edgeVectors` (`edge_vectors_hold`).
* **Claims follow the executions.** On a settled edge no leak is claimed, and
  that is exactly what the executions say (`the_rule_matches_the_executions`);
  the pre-card claim on `mk_http_error_page`'s null edge was made while no
  execution owed anything (`the_old_claim_was_false`).
* **Established accusations stay.** A claim established on another edge
  survives the join with a settled edge
  (`an_established_claim_survives_the_join`); an unresolved-effect marker in
  its place would have withdrawn it (`a_marker_would_withdraw_it`), which is
  why the rule settles the one site on the one edge rather than marking it.
* **Never `Safe`.** The refused test keeps its identity refusal, an `Unknown`
  leaf, so a function carrying it is never `Safe`
  (`a_refused_test_is_never_safe`); the control shape is `Unknown`, and an
  established leak beside it keeps it a provisional `Unsafe`.

## What this is not

A proof that the Rust computes provenance, targets or edges faithfully. That
joined provenance's binding sets mean what `Tied` says is rule 10b's own
premise (`QCore.JoinedProvenanceCycles`, `QCore.JoinedProvenanceCallFrames`),
here over the tie names, which add each local whose only `&` uses are member
addresses through it (`&NAME->F`, the member of `*NAME`, C11 6.5.3.2);
`a_marker_would_withdraw_it` is why this rule replaced the interim withhold
rule (`QCore.RefusedNullTest`, removed), which marked every nullable site the
binding may denote;
the lexical reading of the test and the correspondence with the compiled Rust
are review premises (trust boundary 28), pinned by the shared-vector test in
`crates/qcore-c-gateway/tests/unit/legacy_refused_test.rs` and the fixtures in
`crates/qcore-c-gateway/tests/sup07_refused_test_accusation.rs`. It is not a
statement about Monkey.
-/

namespace QCore.RefusedTestAccusation

open QCore.VerdictSemigroup (V fold)

/-! ## Executions at the test -/

/-- One execution reaching the test, as far as one site `S` and the tested
name `X` go. -/
structure Run where
  /-- `S`'s acquisition ran on this execution. -/
  acquired : Bool
  /-- It returned NULL (a nullable acquisition that failed). -/
  failed : Bool
  /-- `X` denotes `S`'s value at the test. -/
  denotes : Bool
  /-- `X` is NULL at the test. -/
  xNull : Bool
  deriving DecidableEq, Repr

/-- `X` denotes `S` only where `S` was acquired, and then `X` is NULL exactly
when the acquisition failed. -/
def Run.wf (r : Run) : Prop :=
  r.denotes = true → r.acquired = true ∧ r.xNull = r.failed

/-- The obligation `S` carries on an execution: acquired and not failed. -/
def Run.owes (r : Run) : Bool := r.acquired && !r.failed

/-- Joined provenance's fact at the test: on every execution on which `S`
exists, `X` denotes it. -/
def Tied (runs : List Run) : Prop :=
  ∀ r ∈ runs, r.acquired = true → r.denotes = true

/-- The executions that take the test's null edge. -/
def nullEdge (runs : List Run) : List Run := runs.filter (·.xNull)

/-- Under the tie, no execution on the null edge owes the site anything: it
either never acquired it or the acquisition returned NULL. -/
theorem tied_null_edge_owes_nothing (runs : List Run) (wf : ∀ r ∈ runs, r.wf)
    (tied : Tied runs) : ∀ r ∈ nullEdge runs, r.owes = false := by
  intro r member
  have ⟨inRuns, isNull⟩ := List.mem_filter.mp member
  unfold Run.owes
  cases acquired : r.acquired with
  | false => rfl
  | true =>
    have ⟨_, same⟩ := wf r inRuns (tied r inRuns acquired)
    have failed : r.failed = true := by rw [← same]; exact isNull
    simp [failed]

/-- `S` acquired and kept, `X` overwritten on the same execution by a value
the analysis cannot see, which is NULL. -/
def overwritten : List Run := [⟨true, false, false, true⟩]

/-- Without the tie, an execution on the null edge may owe the site: the
refused test establishes nothing, and the tie is the rule's premise. -/
theorem without_the_tie_a_null_edge_may_owe :
    (∀ r ∈ overwritten, r.wf) ∧ ¬ Tied overwritten ∧
      ∃ r ∈ nullEdge overwritten, r.owes = true := by
  refine ⟨?_, ?_, ⟨true, false, false, true⟩, by decide, rfl⟩
  · intro r member
    simp [overwritten] at member
    subst member
    simp [Run.wf]
  · intro tied
    have := tied ⟨true, false, false, true⟩ (by simp [overwritten]) rfl
    exact absurd this (by decide)

/-- `mk_http_error_page`'s executions at `if (!temp)`: `mk_ptr_to_buf`
returned NULL or a buffer (no copy made), or the copy failed or was made. -/
def errorPage : List Run := [
  ⟨false, false, false, true⟩,
  ⟨false, false, false, false⟩,
  ⟨true, true, true, true⟩,
  ⟨true, false, true, false⟩
]

theorem the_error_page_is_tied : (∀ r ∈ errorPage, r.wf) ∧ Tied errorPage := by
  constructor
  · intro r member
    simp [errorPage] at member
    rcases member with h | h | h | h <;> subst h <;> simp [Run.wf]
  · intro r member
    simp [errorPage] at member
    rcases member with h | h | h | h <;> subst h <;> simp

theorem the_error_page_null_edge_owes_nothing :
    ∀ r ∈ nullEdge errorPage, r.owes = false :=
  tied_null_edge_owes_nothing errorPage the_error_page_is_tied.1 the_error_page_is_tied.2

/-! ## The decision -/

/-- The flow's facts about one site on one edge (`none`: the site does not
exist there). -/
structure Site where
  live : Bool
  unresolved : Bool
  deriving DecidableEq, Repr

/-- `report_leaks`: a leak is claimed at an exit for a live site with no
unresolved effect. -/
def claimed : Option Site → Bool
  | none => false
  | some s => s.live && !s.unresolved

/-- What a test's edge does to one site. -/
inductive EdgeStep where
  | keep
  | settle
  deriving DecidableEq, Repr

/-- `refused_test::edge_step`: settle exactly on a refused test's null
(invalid) edge, for a site the tested binding may denote and provenance ties
to the tested name. -/
def edgeStep (refused absent tied targeted : Bool) : EdgeStep :=
  if refused && absent && tied && targeted then .settle else .keep

/-- A settled site does not exist on the edge; a kept one is as it was. -/
def apply : EdgeStep → Option Site → Option Site
  | .settle, _ => none
  | .keep, s => s

theorem settle_iff (r a t g : Bool) :
    edgeStep r a t g = .settle ↔ r = true ∧ a = true ∧ t = true ∧ g = true := by
  cases r <;> cases a <;> cases t <;> cases g <;> simp [edgeStep]

/-- A decided test is not this rule's: it refines as it always did. -/
theorem a_decided_test_is_untouched (a t g : Bool) : edgeStep false a t g = .keep := by
  simp [edgeStep]

/-- The non-null edge keeps every site: there `X` denotes it if anything. -/
theorem the_non_null_edge_is_untouched (r t g : Bool) : edgeStep r false t g = .keep := by
  simp [edgeStep]

/-- A site provenance does not tie to the tested name keeps its claim. -/
theorem an_untied_site_is_untouched (r a g : Bool) : edgeStep r a false g = .keep := by
  simp [edgeStep]

/-- A site the tested binding does not denote keeps its claim. -/
theorem an_untargeted_site_is_untouched (r a t : Bool) : edgeStep r a t false = .keep := by
  simp [edgeStep]

/-- `(refused, absent, tied, targeted)` and the step, shared with
`EDGE_VECTORS`. -/
abbrev EdgeVector := Bool × Bool × Bool × Bool × EdgeStep

def edgeVectors : List EdgeVector := [
  (false, false, false, false, .keep),
  (false, false, false, true, .keep),
  (false, false, true, false, .keep),
  (false, false, true, true, .keep),
  (false, true, false, false, .keep),
  (false, true, false, true, .keep),
  (false, true, true, false, .keep),
  (false, true, true, true, .keep),
  (true, false, false, false, .keep),
  (true, false, false, true, .keep),
  (true, false, true, false, .keep),
  (true, false, true, true, .keep),
  (true, true, false, false, .keep),
  (true, true, false, true, .keep),
  (true, true, true, false, .keep),
  (true, true, true, true, .settle)
]

theorem edge_vectors_hold :
    edgeVectors.all (fun v => edgeStep v.1 v.2.1 v.2.2.1 v.2.2.2.1 == v.2.2.2.2) = true := by
  decide

/-! ## Claims follow the executions -/

/-- On a settled edge no leak is claimed, and no execution owes the site:
the claim and the executions agree. -/
theorem the_rule_matches_the_executions (runs : List Run) (wf : ∀ r ∈ runs, r.wf)
    (tied : Tied runs) (s : Option Site) :
    claimed (apply (edgeStep true true true true) s) = (nullEdge runs).any (·.owes) := by
  have none : (nullEdge runs).any (·.owes) = false := by
    rw [List.any_eq_false]
    intro r member
    simp [tied_null_edge_owes_nothing runs wf tied r member]
  rw [none]
  rfl

/-- Before the card the refused test kept the copy (`keep`), so a leak was
claimed on `mk_http_error_page`'s null edge while no execution owed it. -/
theorem the_old_claim_was_false :
    claimed (apply .keep (some ⟨true, false⟩)) = true ∧
      (nullEdge errorPage).any (·.owes) = false := by
  decide

/-! ## Established accusations stay -/

/-- The flow's union join of one site's facts. -/
def join : Option Site → Option Site → Option Site
  | none, b => b
  | a, none => a
  | some a, some b => some ⟨a.live || b.live, a.unresolved || b.unresolved⟩

/-- A claim established on the other edge survives the join with a settled
edge. -/
theorem an_established_claim_survives_the_join (a b : Option Site)
    (established : claimed b = true) :
    claimed (join (apply .settle a) b) = true := by
  simpa [apply, join] using established

/-- An unresolved-effect marker on the null edge, in place of the settlement,
would have withdrawn that claim at the join. -/
def mark : Option Site → Option Site
  | none => none
  | some s => some ⟨s.live, true⟩

theorem a_marker_would_withdraw_it :
    claimed (join (mark (some ⟨true, false⟩)) (some ⟨true, false⟩)) = false := by
  decide

/-! ## Never `Safe` -/

/-- The refused test's identity refusal is an `Unknown` leaf, so a function
carrying it is never `Safe`, whatever else it holds. -/
theorem a_refused_test_is_never_safe (xs : List V) : fold (.unknown :: xs) ≠ .safe := by
  intro h
  have := (QCore.VerdictSemigroup.fold_eq_safe_iff (.unknown :: xs)).mp h .unknown (by simp)
  exact absurd this (by decide)

/-- The control shape: the test's refusal and no claim. -/
theorem the_shape_is_unknown : fold [.unknown] = .unknown := by decide

/-- Before the card: the same refusal and the false claim. -/
theorem the_old_shape_was_unsafe : fold [.unknown, .provisionalUnsafe] = .provisionalUnsafe := by
  decide

/-- An established leak beside the refused test keeps the provisional
`Unsafe`. -/
theorem an_established_leak_beside_it_stays_unsafe :
    fold [.unknown, .unknown, .provisionalUnsafe] = .provisionalUnsafe := by
  decide

end QCore.RefusedTestAccusation
