/-!
# Holder paths and the removal of one registration
(card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, rule 12t)

The abstract rule implemented by `crates/qcore-c-gateway/src/retained_by.rs`
(`decide`, `removal_key`, `replay`, `PATH_DECISION_VECTORS`,
`PATH_TRACE_VECTORS`) and its route-G adapter `legacy_retained_by.rs`,
contract specification §C04.14.

USER DECISIONS 2026-10-16: "Tracked holder path" — a holder reached through a
member is spelled as a path and tracked by the analysis; "R3: quant state" —
`quant live #loop: #holds(event) |-> #absent;` removes exactly the
registration of `event` from `loop`, every other registration unchanged.

## The model

`QCore.RetainedBy`'s table, extended by a path holder (`Holder.path`: the
object at a member path below an unrebound formal, resolved at the call), an
interior of a tracked identity (`Retained.interior`) and a removal event
(`Event.remove`, carrying the record it names or `none` when it names
none). A path holder outlives the frame like a tracked holder: a record held
there refuses at the exit unless a removal of exactly that record dropped it.

## The theorems

* `an_unresolved_path_never_decides`: a holder the body cannot name (an
  unresolved path reads as `foreign`) refuses, wherever it stands.
* `a_path_record_ends_only_by_its_own_removal`: a record held by a path that
  no later event removes exactly is still there at the exit, so the path is
  never `Safe`.
* `an_unmatched_removal_keeps_every_record`, `a_removal_drops_exactly_its_record`.
* `a_safe_path_never_releases_a_held_identity`: a release of an identity
  while any record (the identity's own or an interior of it) is live refuses;
  so on a `Safe` path every release comes after the records that held it ended
  — the holder outlived the held object only where it was removed or released
  first.
* `a_safe_path_leaves_no_outliving_holder_holding`.
* `path_decision_vectors_hold`, `path_trace_vectors_hold`: the tables shared
  with Rust.

## What this is not

A model over abstract identities. That the gateway resolves a path only below
an unrebound formal named through member accesses off the path
(`path_stable`, premise `P-HOLDER-PATH-STABLE`), and reads `&X->f` as an
interior of `X`, are review premises (trust boundary 28), pinned by the shared
vectors and `crates/qcore-c-gateway/tests/event_loop_retention.rs`.
-/

namespace QCore.RetentionPath

inductive Retained where
  | static
  | callerObject (name : Nat)
  | tracked (site : Nat)
  | interior (site : Nat) (field : Nat)
  | other
  deriving DecidableEq, Repr

inductive Holder where
  | frame (name : Nat)
  | tracked (site : Nat)
  | path (root : Nat) (field : Nat)
  | global
  | foreign
  deriving DecidableEq, Repr

inductive HeldObject where
  | site (site : Nat)
  | siteInterior (site : Nat) (field : Nat)
  | name (name : Nat)
  deriving DecidableEq, Repr

/-- The identity a record keeps alive. -/
def HeldObject.identity : HeldObject → Option Nat
  | .site s => some s
  | .siteInterior s _ => some s
  | .name _ => none

inductive HeldBy where
  | frame (name : Nat)
  | site (site : Nat)
  | path (root : Nat) (field : Nat)
  deriving DecidableEq, Repr

inductive Decision where
  | outlives
  | record (object : HeldObject) (by_ : HeldBy)
  | undecided
  deriving DecidableEq, Repr

def heldBy : Holder → Option HeldBy
  | .frame l => some (.frame l)
  | .tracked h => some (.site h)
  | .path r f => some (.path r f)
  | .global => none
  | .foreign => none

/-- Rule 12t's table with path holders and interiors (`retained_by::decide`). -/
def decide : Retained → Holder → Decision
  | .static, _ => .outlives
  | .other, _ => .undecided
  | _, .foreign => .undecided
  | _, .global => .undecided
  | .callerObject _, .frame _ => .outlives
  | .callerObject n, .tracked h => .record (.name n) (.site h)
  | .callerObject n, .path r f => .record (.name n) (.path r f)
  | .tracked s, .frame l => .record (.site s) (.frame l)
  | .tracked s, .tracked h => .record (.site s) (.site h)
  | .tracked s, .path r f => .record (.site s) (.path r f)
  | .interior s f, .frame l => .record (.siteInterior s f) (.frame l)
  | .interior s f, .tracked h => .record (.siteInterior s f) (.site h)
  | .interior s f, .path r g => .record (.siteInterior s f) (.path r g)

/-- The record a removal names (`retained_by::removal_key`). -/
def removalKey : Retained → Holder → Option (HeldObject × HeldBy)
  | .callerObject n, h => (heldBy h).map fun b => (.name n, b)
  | .tracked s, h => (heldBy h).map fun b => (.site s, b)
  | .interior s f, h => (heldBy h).map fun b => (.siteInterior s f, b)
  | .static, _ => none
  | .other, _ => none

inductive Event where
  | retain (d : Decision)
  | release (site : Nat)
  | releaseHolder (site : Nat)
  | remove (record : Option (HeldObject × HeldBy))
  deriving DecidableEq, Repr

abbrev Records := List (HeldObject × HeldBy)

def holds (rs : Records) (s : Nat) : Bool := rs.any fun r => r.1.identity == some s

def step (rs : Records) : Event → Option Records
  | .retain .outlives => some rs
  | .retain .undecided => none
  | .retain (.record o b) => some ((o, b) :: rs)
  | .release s => if holds rs s then none else some (rs.filter fun r => r.2 != .site s)
  | .releaseHolder h => some (rs.filter fun r => r.2 != .site h)
  | .remove none => some rs
  | .remove (some r) => some (rs.filter fun r' => r' != r)

def run (rs : Records) : List Event → Option Records
  | [] => some rs
  | e :: es => (step rs e).bind fun rs' => run rs' es

/-- A record whose holder outlives the frame. -/
def outlivesFrame (r : HeldObject × HeldBy) : Bool :=
  match r.2 with
  | .frame _ => false
  | .site _ => true
  | .path _ _ => true

def replay (es : List Event) : Bool :=
  match run [] es with
  | none => false
  | some rs => !(rs.any outlivesFrame)

theorem run_append (rs : Records) (pre post : List Event) :
    run rs (pre ++ post) = (run rs pre).bind fun r => run r post := by
  induction pre generalizing rs with
  | nil => simp [run]
  | cons e es ih =>
      simp only [List.cons_append, run]
      cases step rs e with
      | none => rfl
      | some r => simp only [Option.bind_some]; exact ih r

/-- **An unresolved path never decides**: the body reads it as a holder it
cannot name, and the retention refuses wherever it stands. -/
theorem an_unresolved_path_never_decides (r : Retained) (hr : r ≠ .static)
    (pre post : List Event) :
    replay (pre ++ .retain (decide r .foreign) :: post) = false := by
  have hd : decide r .foreign = .undecided := by cases r <;> simp_all [decide]
  rw [hd]
  unfold replay
  rw [run_append]
  cases run [] pre with
  | none => rfl
  | some rs => simp [run, step]

/-- **An unmatched removal keeps every record.** -/
theorem an_unmatched_removal_keeps_every_record (rs : Records) :
    step rs (.remove none) = some rs := rfl

/-- **A removal drops exactly its record**, and keeps every other. -/
theorem a_removal_drops_exactly_its_record (rs : Records) (r : HeldObject × HeldBy) :
    step rs (.remove (some r)) = some (rs.filter fun r' => r' != r) := rfl

/-- One step keeps a path-held record unless it is that record's own removal. -/
theorem step_keeps_a_path_record (rs rs' : Records) (e : Event) (o : HeldObject)
    (p f : Nat) (hmem : (o, HeldBy.path p f) ∈ rs) (hstep : step rs e = some rs')
    (hnot : e ≠ .remove (some (o, .path p f))) : (o, HeldBy.path p f) ∈ rs' := by
  cases e with
  | retain d =>
      cases d <;> simp [step] at hstep
      · subst hstep; exact hmem
      · subst hstep; exact List.mem_cons_of_mem _ hmem
  | release s =>
      simp only [step] at hstep
      split at hstep
      · simp at hstep
      · simp at hstep; subst hstep; simp [hmem]
  | releaseHolder h =>
      simp [step] at hstep; subst hstep; simp [hmem]
  | remove r =>
      cases r with
      | none => simp [step] at hstep; subst hstep; exact hmem
      | some r =>
          simp [step] at hstep; subst hstep
          have : r ≠ (o, .path p f) := fun h => hnot (by rw [h])
          simp [hmem]
          exact fun h => this h.symm

/-- **A path record ends only by its own removal**: when no event of the
rest of the path removes exactly that record, it is still there at the exit
(if the path does not refuse first), so the path is never `Safe`. -/
theorem a_path_record_ends_only_by_its_own_removal (es : List Event) (o : HeldObject)
    (p f : Nat) (rs : Records) (hmem : (o, HeldBy.path p f) ∈ rs)
    (hnone : ∀ e ∈ es, e ≠ .remove (some (o, .path p f))) :
    ∀ rs', run rs es = some rs' → (o, HeldBy.path p f) ∈ rs' := by
  induction es generalizing rs with
  | nil => intro rs' h; simp [run] at h; subst h; exact hmem
  | cons e es ih =>
      intro rs' h
      simp only [run] at h
      cases hs : step rs e with
      | none => simp [hs] at h
      | some rs₁ =>
          rw [hs] at h
          simp only [Option.bind_some] at h
          have hk := step_keeps_a_path_record rs rs₁ e o p f hmem hs
            (hnone e (List.mem_cons_self ..))
          exact ih rs₁ hk (fun e' he' => hnone e' (List.mem_cons_of_mem _ he')) rs' h

theorem a_path_record_without_its_removal_is_never_safe (n p f : Nat) (es : List Event)
    (hnone : ∀ e ∈ es, e ≠ .remove (some (.name n, .path p f))) :
    replay (.retain (decide (.callerObject n) (.path p f)) :: es) = false := by
  unfold replay
  simp only [run, decide, step, Option.bind_some]
  cases h : run [(.name n, .path p f)] es with
  | none => rfl
  | some rs' =>
      have := a_path_record_ends_only_by_its_own_removal es (.name n) p f
        [(.name n, .path p f)] (List.mem_singleton_self _) hnone rs' h
      simp only [Bool.not_eq_false', List.any_eq_true]
      exact ⟨_, this, rfl⟩

theorem release_refused_iff (rs : Records) (s : Nat) :
    step rs (.release s) = none ↔ holds rs s = true := by
  simp only [step]
  split <;> simp_all

/-- **On a `Safe` path no release reaches a held identity** (its own record
or an interior's): the holder outlived the object only where the record ended
first. -/
theorem a_safe_path_never_releases_a_held_identity (pre post : List Event) (s : Nat)
    (hsafe : replay (pre ++ .release s :: post) = true) (rs : Records)
    (hpre : run [] pre = some rs) : holds rs s = false := by
  unfold replay at hsafe
  rw [run_append, hpre] at hsafe
  simp only [Option.bind_some, run] at hsafe
  cases hh : holds rs s with
  | false => rfl
  | true =>
      have : step rs (.release s) = none := (release_refused_iff rs s).mpr hh
      rw [this] at hsafe
      simp at hsafe

/-- **At the exit of a `Safe` path no tracked or path holder still holds.** -/
theorem a_safe_path_leaves_no_outliving_holder_holding (es : List Event)
    (hsafe : replay es = true) :
    ∃ rs, run [] es = some rs ∧ ∀ r ∈ rs, outlivesFrame r = false := by
  unfold replay at hsafe
  cases h : run [] es with
  | none => simp [h] at hsafe
  | some rs =>
      refine ⟨rs, rfl, fun r hr => ?_⟩
      rw [h] at hsafe
      simp only [Bool.not_eq_true', List.any_eq_false] at hsafe
      cases ht : outlivesFrame r with
      | false => rfl
      | true => exact absurd ht (hsafe r hr)

/-! ## Shared vectors -/

inductive RClass where
  | static | caller | tracked | interior | other
  deriving DecidableEq, Repr

inductive HClass where
  | path | frame | tracked | global | foreign
  deriving DecidableEq, Repr

inductive Label where
  | outlives | record | undecided
  deriving DecidableEq, Repr

def RClass.sample : RClass → Retained
  | .static => .static
  | .caller => .callerObject 0
  | .tracked => .tracked 1
  | .interior => .interior 1 0
  | .other => .other

def HClass.sample : HClass → Holder
  | .path => .path 0 0
  | .frame => .frame 0
  | .tracked => .tracked 2
  | .global => .global
  | .foreign => .foreign

def label : Decision → Label
  | .outlives => .outlives
  | .record .. => .record
  | .undecided => .undecided

/-- `PATH_DECISION_VECTORS` of `retained_by.rs`, row by row. -/
def pathDecisionVectors : List (RClass × HClass × Label) := [
  (.static, .path, .outlives),
  (.caller, .path, .record),
  (.tracked, .path, .record),
  (.interior, .path, .record),
  (.other, .path, .undecided),
  (.interior, .frame, .record),
  (.interior, .tracked, .record),
  (.interior, .global, .undecided),
  (.interior, .foreign, .undecided)
]

theorem path_decision_vectors_hold :
    pathDecisionVectors.all (fun (r, h, l) => label (decide r.sample h.sample) == l) = true := by
  decide

def coll : Holder := .path 0 0
def config : Holder := .path 1 0
def caller : Retained := .callerObject 7
def interiorP : Retained := .interior 1 0

/-- `PATH_TRACE_VECTORS` of `retained_by.rs`, each path as
`retained_by::path_trace` builds it. -/
def pathTraceVectors : List (List Event × Bool) := [
  ([.retain (decide caller coll)], false),
  ([.retain (decide caller coll), .remove (removalKey caller coll)], true),
  ([.retain (decide caller coll), .remove (removalKey caller config)], false),
  ([.retain (decide caller coll), .remove (removalKey .other coll)], false),
  ([.remove (removalKey caller coll)], true),
  ([.retain (decide interiorP (.tracked 2)), .release 1, .releaseHolder 2], false),
  ([.retain (decide interiorP (.tracked 2)), .releaseHolder 2, .release 1], true),
  ([.retain (decide caller coll), .retain (decide (.callerObject 8) coll),
    .remove (removalKey caller coll)], false)
]

theorem path_trace_vectors_hold :
    pathTraceVectors.all (fun (es, safe) => replay es == safe) = true := by
  decide

end QCore.RetentionPath
