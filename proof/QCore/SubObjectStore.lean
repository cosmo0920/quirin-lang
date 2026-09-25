import QCore.IndirectScalarAccess

/-!
# Stores into a sub-object through a resolved base (card W45-SUB-OBJECT-STORES)

The abstract rule implemented by `legacy_c::sub_object_store`
(`read_destination`, `read_value`, `transfer`, `decide`, `call_runs`) and
`Parser::sub_object_store_statement` in `crates/qcore-c-gateway`.

A statement the lexical gate refused as a store into a sub-object (`R->F`,
`R[I].F`, `(R)->F`, `S.F`, `(&S)->F`, `A[I].F`) is lowered, when its root and
member resolve through the declaration reader, to a store whose effect on
custody is decided on the state at the node:

* the dereferenced base must satisfy W43's obligation (`resolves`), and for a
  pointer member it must carry no tracked target (a tracked aggregate's
  member may hold a field link only O04-B08's store rewrites);
* a scalar, null or literal value is custody-neutral; the member's place
  binding is forgotten;
* a named pointer value (a call result is bound to a temporary first) is
  neutral when no reaching path bound it or its binding carries no target and
  is not unknown; it **escapes** when it is exactly one known, exactly-live
  heap identity this body acquired (not a caller's site, no collection) and
  the base is a formal the caller reaches; everything else is the fallback;
* the fallback is the gate's refusal: it reports, and the refused hand-on
  rule (card W44-LEAK-PIN-AUDIT) marks the value's identities unresolved.

The resource and binding halves reuse `QCore.IndirectScalarAccess`'s state.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
token and declaration extraction and the correspondence with the compiled Rust
are review premises (trust boundary 28). That a member store writes that one
member and nothing else is the C object model under the no-undefined-behaviour
reading constraint 10a rests on, stated here, not proved. That an escape into
the caller's record discharges this body's obligation is the same reading a
return gets: the caller holds the object.
-/

namespace QCore.SubObjectStore

open QCore.IndirectScalarAccess

/-- Facts about sites the store reads but never writes. -/
structure Env where
  /-- A site the caller holds at entry (a formal's entry custody). -/
  callerSite : Nat → Bool
  /-- A heap site, as opposed to a descriptor, stream or directory. -/
  heap : Nat → Bool
  /-- A site carrying an indexed collection. -/
  collection : Nat → Bool

/-- The destination: the dereferenced base (none for this function's own
record), the member's place, whether the member holds a pointer, and whether
the base is a formal the caller reaches. -/
structure Destination where
  base : Option Nat
  place : Nat
  pointerMember : Bool
  callerVisible : Bool

/-- The value after classification (a call result is its temporary's name). -/
inductive Value where
  | scalar
  | untracked
  | name (n : Nat)

inductive Decision where
  | neutral
  | escape (site : Nat)
  | refuse
  deriving DecidableEq, Repr

def tracked (s : State) (n : Nat) : Bool :=
  match s.binding n with
  | some b => !b.targets.isEmpty
  | none => false

/-- Step 1: the base premises (`call_runs`). -/
def baseOk (s : State) (d : Destination) : Bool :=
  match d.base with
  | none => true
  | some b => resolves s b && !(d.pointerMember && tracked s b)

/-- Step 3: what storing a named pointer does to custody (`decide`). -/
def liveAt (s : State) (site : Nat) : Bool :=
  match s.status site with
  | some flags => flags.exactlyLive
  | none => false

/-- The escape premises on one known target. -/
def escapable (e : Env) (s : State) (visible : Bool) (b : Binding) (site : Nat) : Bool :=
  visible && !b.unknown && b.derivation == .identity && !e.callerSite site && liveAt s site &&
    !s.unresolved site && e.heap site && !e.collection site

def decide (e : Env) (s : State) (visible : Bool) (n : Nat) : Decision :=
  match s.binding n with
  | none => .neutral
  | some b =>
      match b.targets with
      | [] => if b.unknown then .refuse else .neutral
      | [site] => if escapable e s visible b site then .escape site else .refuse
      | _ => .refuse

/-- Custody leaves with the caller's record: the site is escaped, no longer
live. -/
def escapeSite (s : State) (site : Nat) : State :=
  { s with status := fun x => if x = site then some ⟨false, false, true⟩ else s.status x }

def forget (s : State) (place : Nat) : State :=
  { s with binding := fun x => if x = place then none else s.binding x }

inductive Outcome where
  | admitted (s : State)
  | refused

/-- The store's transfer, fallback excluded. -/
def store (e : Env) (s : State) (d : Destination) (v : Value) : Outcome :=
  if !baseOk s d then .refused
  else
    match v with
    | .scalar | .untracked => .admitted (forget s d.place)
    | .name n =>
        match decide e s d.callerVisible n with
        | .neutral => .admitted (forget s d.place)
        | .escape site => .admitted (forget (escapeSite s site) d.place)
        | .refuse => .refused

/-- The fallback, as far as custody goes: the refused hand-on rule marks every
target of the stored value unresolved and changes no status. -/
def handOn (s : State) (v : Value) : State :=
  match v with
  | .name n =>
      match s.binding n with
      | some b => { s with unresolved := fun x => s.unresolved x || b.targets.contains x }
      | none => s
  | _ => s

/-! ## The decision's shape -/

/-- Every decision is neutral, a refusal, or the escape of the one known target
whose escape premises hold. -/
theorem decide_cases (e : Env) (s : State) (visible : Bool) (n : Nat) :
    decide e s visible n = .neutral ∨ decide e s visible n = .refuse ∨
      ∃ b site, s.binding n = some b ∧ b.targets = [site] ∧
        escapable e s visible b site = true ∧ decide e s visible n = .escape site := by
  unfold decide
  cases hb : s.binding n with
  | none => simp
  | some b =>
    simp only
    rcases hbt : b.targets with _ | ⟨x, _ | ⟨y, rest⟩⟩
    · cases b.unknown <;> simp
    · by_cases hc : escapable e s visible b x = true
      · exact Or.inr (Or.inr ⟨b, x, rfl, hbt, hc, by simp [hc]⟩)
      · exact Or.inr (Or.inl (by simp [hc]))
    · simp

theorem an_escape_needs_a_caller_reachable_base (e : Env) (s : State) (visible : Bool)
    (n site : Nat) (h : decide e s visible n = .escape site) : visible = true := by
  rcases decide_cases e s visible n with hd | hd | ⟨b, site', _, _, hc, _⟩
  · rw [hd] at h; cases h
  · rw [hd] at h; cases h
  · cases visible <;> simp_all [escapable]

/-- What an admitted store leaves: the member forgotten, and at most the one
escape of a named value's target. -/
theorem store_admitted (e : Env) (s s' : State) (d : Destination) (v : Value)
    (h : store e s d v = .admitted s') :
    baseOk s d = true ∧
      (s' = forget s d.place ∨
        ∃ n site, v = .name n ∧ decide e s d.callerVisible n = .escape site ∧
          s' = forget (escapeSite s site) d.place) := by
  unfold store at h
  by_cases hbase : baseOk s d = true
  · refine ⟨hbase, ?_⟩
    simp only [hbase, Bool.not_true, Bool.false_eq_true, ite_false] at h
    cases v with
    | scalar => simp only [Outcome.admitted.injEq] at h; exact Or.inl h.symm
    | untracked => simp only [Outcome.admitted.injEq] at h; exact Or.inl h.symm
    | name n =>
      cases hd : decide e s d.callerVisible n with
      | neutral => simp only [hd, Outcome.admitted.injEq] at h; exact Or.inl h.symm
      | escape site =>
        simp only [hd, Outcome.admitted.injEq] at h
        exact Or.inr ⟨n, site, rfl, hd, h.symm⟩
      | refuse => simp [hd] at h
  · simp [hbase] at h

/-! ## Scalar and untracked values are custody-neutral -/

theorem a_neutral_store_changes_no_status (e : Env) (s s' : State) (d : Destination)
    (v : Value) (hv : v = .scalar ∨ v = .untracked) (h : store e s d v = .admitted s') :
    s'.status = s.status ∧ s'.unresolved = s.unresolved ∧ s'.nullable = s.nullable ∧
      s'.rest = s.rest := by
  rcases store_admitted e s s' d v h with ⟨_, rfl | ⟨n, _, rfl, _, _⟩⟩
  · exact ⟨rfl, rfl, rfl, rfl⟩
  · rcases hv with hv | hv <;> cases hv

/-- The member's old fact is forgotten by every admitted store. -/
theorem an_admitted_store_forgets_the_member (e : Env) (s s' : State) (d : Destination)
    (v : Value) (h : store e s d v = .admitted s') : s'.binding d.place = none := by
  rcases store_admitted e s s' d v h with ⟨_, rfl | ⟨_, _, _, _, rfl⟩⟩ <;> simp [forget]

/-- Constraint 5i: the store writes the member's place and no other name. -/
theorem an_admitted_store_rebinds_no_other_name (e : Env) (s s' : State) (d : Destination)
    (v : Value) (x : Nat) (hx : x ≠ d.place) (h : store e s d v = .admitted s') :
    s'.binding x = s.binding x := by
  rcases store_admitted e s s' d v h with ⟨_, rfl | ⟨_, _, _, _, rfl⟩⟩ <;>
    simp [forget, escapeSite, hx]

/-! ## The base premises -/

theorem an_unresolved_base_refuses (e : Env) (s : State) (d : Destination) (v : Value)
    (b : Nat) (hb : d.base = some b) (hr : resolves s b = false) :
    store e s d v = .refused := by
  simp [store, baseOk, hb, hr]

theorem a_pointer_store_through_a_tracked_base_refuses (e : Env) (s : State)
    (d : Destination) (v : Value) (b : Nat) (hb : d.base = some b)
    (hp : d.pointerMember = true) (ht : tracked s b = true) :
    store e s d v = .refused := by
  simp [store, baseOk, hb, hp, ht]

/-! ## Owned values: escape only into the caller's record -/

/-- Only an escape changes a status, and an escape needs a caller-reachable
base. -/
theorem a_status_changes_only_by_an_escape_into_the_callers_record (e : Env) (s s' : State)
    (d : Destination) (v : Value) (h : store e s d v = .admitted s')
    (hne : s'.status ≠ s.status) : d.callerVisible = true := by
  rcases store_admitted e s s' d v h with ⟨_, rfl | ⟨n, site, _, hd, _⟩⟩
  · exact absurd rfl hne
  · exact an_escape_needs_a_caller_reachable_base e s _ n site hd

/-- No `Safe` rests on a store that moves custody unseen: an admitted store of
a name bound to one known target escapes exactly that target. -/
theorem a_stored_identity_is_never_dropped_unseen (e : Env) (s s' : State) (d : Destination)
    (n site : Nat) (b : Binding) (hb : s.binding n = some b) (ht : b.targets = [site])
    (h : store e s d (.name n) = .admitted s') :
    s'.status site = some ⟨false, false, true⟩ := by
  have hd : decide e s d.callerVisible n = .escape site ∨
      decide e s d.callerVisible n = .refuse := by
    by_cases hc : escapable e s d.callerVisible b site = true <;> simp [decide, hb, ht, hc]
  by_cases hbase : baseOk s d = true
  · rcases hd with hd | hd <;> simp [store, hbase, hd] at h
    subst h
    simp [forget, escapeSite]
  · simp [store, hbase] at h

/-- An owned identity stored into a record the caller does not reach (a local
record, a rebound formal) keeps the refusal. -/
theorem an_owned_identity_into_an_invisible_record_refuses (e : Env) (s : State)
    (d : Destination) (n site : Nat) (b : Binding) (hv : d.callerVisible = false)
    (hbase : baseOk s d = true) (hb : s.binding n = some b) (ht : b.targets = [site]) :
    store e s d (.name n) = .refused := by
  simp [store, hbase, decide, hb, ht, hv, escapable]

/-- A caller's own site is never escaped by a store: a formal's entry custody
stays the caller's, so no W4 summary changes mode through this rule. -/
theorem a_callers_site_is_never_escaped (e : Env) (s : State) (d : Destination)
    (n site : Nat) (b : Binding) (hbase : baseOk s d = true) (hb : s.binding n = some b)
    (ht : b.targets = [site]) (hc : e.callerSite site = true) :
    store e s d (.name n) = .refused := by
  simp [store, hbase, decide, hb, ht, hc, escapable]

/-- A descriptor (a non-heap site) is custody, never data: never escaped by
this rule. -/
theorem a_descriptor_is_never_stored (e : Env) (s : State) (d : Destination)
    (n site : Nat) (b : Binding) (hbase : baseOk s d = true) (hb : s.binding n = some b)
    (ht : b.targets = [site]) (hk : e.heap site = false) :
    store e s d (.name n) = .refused := by
  simp [store, hbase, decide, hb, ht, hk, escapable]

/-- A derived pointer is never escaped. -/
theorem a_derived_pointer_is_never_escaped (e : Env) (s : State) (d : Destination)
    (n site : Nat) (b : Binding) (hbase : baseOk s d = true) (hb : s.binding n = some b)
    (ht : b.targets = [site]) (hd : b.derivation = .derived) :
    store e s d (.name n) = .refused := by
  simp [store, hbase, decide, hb, ht, hd, escapable]

/-- An unknown (poisoned) binding may hold any identity: refused. -/
theorem an_unknown_value_refuses (e : Env) (s : State) (d : Destination) (n : Nat)
    (b : Binding) (hbase : baseOk s d = true) (hb : s.binding n = some b)
    (ht : b.targets = []) (hu : b.unknown = true) :
    store e s d (.name n) = .refused := by
  simp [store, hbase, decide, hb, ht, hu]

/-! ## The refused store still hands on -/

/-- The fallback discharges nothing: no status changes. -/
theorem the_fallback_changes_no_status (s : State) (v : Value) :
    (handOn s v).status = s.status := by
  unfold handOn
  split
  · split <;> rfl
  · rfl

/-- The fallback marks every target of the stored value unresolved, so no leak
is claimed for an identity a refused store may have stored. -/
theorem the_fallback_marks_the_stored_identity (s : State) (n site : Nat) (b : Binding)
    (hb : s.binding n = some b) (hmem : site ∈ b.targets) :
    (handOn s (.name n)).unresolved site = true := by
  simp [handOn, hb, hmem]

/-! ## Non-vacuity -/

def env : Env := ⟨fun site => site = 9, fun site => site ≠ 8, fun _ => false⟩

/-- Name 0 holds the live heap site 7, name 1 the live descriptor site 8,
name 2 the caller's site 9; name 3 is the formal base, never bound. -/
def owner : State :=
  { binding := fun n =>
      if n = 0 then some ⟨[7], false, false, .identity⟩
      else if n = 1 then some ⟨[8], false, false, .identity⟩
      else if n = 2 then some ⟨[9], false, false, .identity⟩
      else none
    status := fun site => if site = 7 ∨ site = 8 ∨ site = 9 then some ⟨true, false, false⟩ else none
    nullable := fun _ => false
    unresolved := fun _ => false
    rest := ⟨0, 0, 0⟩ }

def visible : Destination := ⟨some 3, 30, true, true⟩
def localRecord : Destination := ⟨none, 31, true, false⟩

theorem the_owned_pointer_escapes_into_the_callers_record :
    decide env owner true 0 = .escape 7 := by decide

theorem the_owned_pointer_into_a_local_record_refuses :
    decide env owner false 0 = .refuse := by decide

theorem the_descriptor_refuses : decide env owner true 1 = .refuse := by decide

theorem the_callers_own_pointer_refuses : decide env owner true 2 = .refuse := by decide

theorem an_unbound_pointer_is_neutral : decide env owner true 5 = .neutral := by decide

theorem the_formal_base_resolves : baseOk owner visible = true := by decide

theorem a_scalar_store_is_admitted :
    ∃ s', store env owner visible .scalar = .admitted s' := ⟨_, rfl⟩

end QCore.SubObjectStore
