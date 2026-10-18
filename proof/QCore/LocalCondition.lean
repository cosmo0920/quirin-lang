/-! S3 scoped conditions: a written prerequisite is an obligation of its caller,
not an assumed proof. Unknown conditions preserve both feasible branches.
Facts retain exact binder/object identity; joining paths drops one-sided facts.
Executable counterpart: crates/qcore-core/src/local_condition.rs. -/
namespace QCore.LocalCondition
inductive Truth where
  | yes | no | unknown
  deriving DecidableEq, Repr
structure Fact where
  binder : Nat
  root : Nat
  value : Bool
  deriving DecidableEq, Repr
structure Guard where
  binder : Nat
  root : Nat
  negated : Bool
  deriving DecidableEq, Repr

def prerequisite : Truth → Bool → Bool
  | .no, _ => true
  | _, discharged => discharged

def exact (g : Guard) (f : Fact) : Bool :=
  g.binder == f.binder && g.root == f.root

def matching (g : Guard) (facts : List Fact) : List Fact :=
  facts.filter (exact g)

def reading (g : Guard) (facts : List Fact) : Truth :=
  match matching g facts with
  | [] => .unknown
  | f :: rest =>
    if rest.any (fun x => x.value != f.value) then .unknown
    else if f.value != g.negated then .yes else .no

def join (left right : List Fact) : List Fact :=
  left.filter (fun f => right.contains f)

def scoped (outer : List Fact) (localFact : Fact) : List Fact :=
  outer ++ [localFact]

theorem unknown_never_waives_a_prerequisite (discharged : Bool) :
    prerequisite .unknown discharged = discharged := rfl

theorem active_never_assumes_a_prerequisite (discharged : Bool) :
    prerequisite .yes discharged = discharged := rfl

theorem inactive_waives_only_its_own_prerequisite (discharged : Bool) :
    prerequisite .no discharged = true := rfl

theorem unknown_covers_every_concrete_branch (discharged : Bool) :
    prerequisite .unknown discharged =
      (prerequisite .yes discharged && prerequisite .no discharged) := by
  cases discharged <;> rfl

theorem an_unknown_undischarged_prerequisite_refuses :
    prerequisite .unknown false = false := rfl

theorem a_join_fact_was_established_on_both_paths
    (left right : List Fact) (f : Fact) (h : f ∈ join left right) :
    f ∈ left ∧ f ∈ right := by
  simpa [join, List.mem_filter, List.contains_iff_mem] using h

theorem no_common_fact_leaves_no_join_facts (left right : List Fact)
    (h : ∀ f, f ∈ left → f ∉ right) : join left right = [] := by
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro f hf
  obtain ⟨hl, hr⟩ := a_join_fact_was_established_on_both_paths left right f hf
  exact h f hl hr

theorem an_unmatched_instance_is_unresolved (g : Guard) (facts : List Fact)
    (h : ∀ f ∈ facts, f.root ≠ g.root) : reading g facts = .unknown := by
  have empty : matching g facts = [] := by
    apply List.eq_nil_iff_forall_not_mem.mpr
    intro f hf
    have hf' := List.mem_filter.mp hf
    have roots : g.root = f.root := by
      simpa [exact, Bool.and_eq_true, beq_iff_eq] using (Bool.and_eq_true.mp hf'.2).2
    exact h f hf'.1 roots.symm
  simp [reading, empty]

def vectors : List (Truth × Bool × Bool) := [
  (.yes, false, false), (.yes, true, true),
  (.no, false, true), (.no, true, true),
  (.unknown, false, false), (.unknown, true, true)]
theorem shared_vectors_hold :
    vectors.all (fun (t, d, expected) => prerequisite t d == expected) = true := by decide
end QCore.LocalCondition
