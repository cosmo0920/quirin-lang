/-!
# The W4 derivation scope for corpus-defined callees

Card **W44-CORPUS-CALLEE-SUMMARIES**
(`crates/qcore-c-gateway/src/legacy_callee_scope.rs`).

Under the main-source scope a unit *reports* the definitions its main source
writes. Until this card the same selection also bounded the definitions W4
*derived summaries for*, so a `static inline` helper a header defines in the
translation unit — the body every call to it in the unit runs — was never
offered to the derivation, and every call to it stayed uncontracted. The card
widens the derivation scope, and only the derivation scope, to the selection
together with every unselected definition of **internal linkage** the selection
reaches through direct calls.

## What is proved

* **Every scope member is selected, or unreserved and of internal linkage**
  (`every_scope_member_is_selected_or_internal`), so **an unselected definition
  of external linkage is never in scope**
  (`an_unselected_external_definition_is_never_in_scope`), nor is an
  unselected implementation-reserved one such as `__bswap_32`
  (`an_unselected_reserved_definition_is_never_in_scope`), and the selection
  is always in scope (`the_selection_is_in_scope`).
* **A newly admitted callee's calls run its analysed body**
  (`an_admitted_callee_runs_only_its_analysed_body`): for an unselected scope
  member, every body a call in this unit may run is the unit's own definition.
  The linkage premise is necessary, not decorative: an external definition's
  call may run a body from elsewhere (`an_external_definition_may_run_another_body`).
* **Declared contracts still win** (`a_declaration_is_never_overridden`), and an
  inferred contract is installed **exactly** as derived, so callers apply it
  with the semantics a declaration of the same contract has
  (`an_installed_inference_is_exactly_the_derived_contract`); anything
  installed that no declaration supplied was **validated** and lies **in
  scope** (`an_installed_inference_was_validated_in_scope`).
* **An existing summary survives the widening**
  (`an_existing_summary_survives_the_widening`): if validation needs every
  callee contracted and reads only its callees' contracts, a summary derived
  under a smaller environment is derived unchanged under any environment that
  extends it. Widening therefore only *adds* summaries.
* Witnesses: a reached static helper is admitted; an unreached one, an
  external `inline` one and a reserved one are not (`the_witness_scope`).

## What is not proved

The linkage reader (`has_internal_linkage`: `static` present and `extern`
absent on the definition's own specifiers), the reserved-name reader
(`is_reserved_identifier`: a leading `__`, or `_` and an uppercase letter) and the reachability reader
(`direct_calls`) are Rust transcriptions of `candidate` and `step`; the
correspondence is maintained by review (trust boundary 28). The *selected*
definitions' own call-resolution premise — that a call in the unit to a
selected external definition runs that definition — is the pre-existing W4
premise, unchanged by this card and neither strengthened nor weakened here.
Validation itself is `QCore.CustodySummary`'s relation, taken here as an
abstract predicate. Nothing here is a statement about whole C.
-/

namespace QCore.CalleeSummaryScope

abbrev Name := String

inductive Linkage where
  | internal
  | external
  deriving DecidableEq, Repr

/-- One file-scope function definition of a translation unit. -/
structure Definition where
  name : Name
  linkage : Linkage
  /-- Chosen by the main-source selection (reported by the unit). -/
  selected : Bool
  /-- The name begins `__` or `_` and an uppercase letter: reserved for the
  implementation in every context (C11 7.1.3), so the body is a library or
  compiler header body. -/
  reserved : Bool
  /-- The names its body calls directly. -/
  calls : List Name
  deriving DecidableEq, Repr

/-- An unselected, unreserved definition of internal linkage: the only kind the
widening admits. -/
def candidate (d : Definition) : Bool :=
  !d.selected && d.linkage == .internal && !d.reserved

def callsOf (unit : List Definition) (n : Name) : List Name :=
  match unit.find? (fun d => d.name == n) with
  | some d => d.calls
  | none => []

def selectedNames (unit : List Definition) : List Name :=
  (unit.filter (fun d => d.selected)).map (fun d => d.name)

/-- One round of reachability: add every candidate a scope member calls. -/
def step (unit : List Definition) (s : List Name) : List Name :=
  s ++ ((unit.filter fun d =>
      candidate d && s.any fun m => (callsOf unit m).contains d.name).map
    (fun d => d.name))

def iterate (unit : List Definition) : Nat → List Name → List Name
  | 0, s => s
  | k + 1, s => iterate unit k (step unit s)

/-- The derivation scope. `unit.length` rounds suffice for reachability over
`unit.length` definitions; the Rust work list computes the same closure. -/
def scope (unit : List Definition) : List Name :=
  iterate unit unit.length (selectedNames unit)

/-- A name the widened derivation may consider. -/
def Admissible (unit : List Definition) (n : Name) : Prop :=
  ∃ d ∈ unit, d.name = n ∧
    (d.selected = true ∨ (d.linkage = .internal ∧ d.reserved = false))

/-- Definitions of one unit carry distinct names (C11 6.9p3/p5). -/
def UniqueNames (unit : List Definition) : Prop :=
  ∀ d ∈ unit, ∀ d' ∈ unit, d.name = d'.name → d = d'

theorem candidate_is_internal {d : Definition} (h : candidate d = true) :
    d.linkage = .internal := by
  unfold candidate at h
  cases hl : d.linkage <;> simp_all

theorem candidate_is_unreserved {d : Definition} (h : candidate d = true) :
    d.reserved = false := by
  unfold candidate at h
  cases hr : d.reserved <;> simp_all

theorem step_preserves_admissible {unit : List Definition} {s : List Name}
    (hs : ∀ m ∈ s, Admissible unit m) : ∀ m ∈ step unit s, Admissible unit m := by
  intro m hm
  unfold step at hm
  rcases List.mem_append.mp hm with hm | hm
  · exact hs m hm
  · rcases List.mem_map.mp hm with ⟨d, hd, rfl⟩
    have hd' := List.mem_filter.mp hd
    have hc : candidate d = true := by
      have := hd'.2
      simp only [Bool.and_eq_true] at this
      exact this.1
    exact ⟨d, hd'.1, rfl, Or.inr ⟨candidate_is_internal hc, candidate_is_unreserved hc⟩⟩

theorem iterate_preserves_admissible (unit : List Definition) :
    ∀ (k : Nat) (s : List Name), (∀ m ∈ s, Admissible unit m) →
      ∀ m ∈ iterate unit k s, Admissible unit m := by
  intro k
  induction k with
  | zero => intro s hs; exact hs
  | succ k ih =>
    intro s hs
    exact ih (step unit s) (step_preserves_admissible hs)

theorem selected_admissible (unit : List Definition) :
    ∀ m ∈ selectedNames unit, Admissible unit m := by
  intro m hm
  unfold selectedNames at hm
  rcases List.mem_map.mp hm with ⟨d, hd, rfl⟩
  have hd' := List.mem_filter.mp hd
  exact ⟨d, hd'.1, rfl, Or.inl hd'.2⟩

/-- Every scope member is selected or of internal linkage. -/
theorem every_scope_member_is_selected_or_internal (unit : List Definition) :
    ∀ m ∈ scope unit, Admissible unit m :=
  iterate_preserves_admissible unit unit.length _ (selected_admissible unit)

/-- An unselected definition of external linkage is never in scope. -/
theorem an_unselected_external_definition_is_never_in_scope
    {unit : List Definition} (unique : UniqueNames unit) {d : Definition}
    (hd : d ∈ unit) (unselected : d.selected = false) (external : d.linkage = .external) :
    d.name ∉ scope unit := by
  intro hm
  rcases every_scope_member_is_selected_or_internal unit d.name hm with
    ⟨d', hd', hname, hadm⟩
  have := unique d' hd' d hd hname
  subst this
  rcases hadm with h | ⟨h, _⟩
  · rw [unselected] at h; exact Bool.false_ne_true h
  · rw [external] at h; exact Linkage.noConfusion h

/-- An unselected implementation-reserved definition (`__bswap_32`) is never in
scope: whether a library header body is checked or declared stays the libc
provider's decision. -/
theorem an_unselected_reserved_definition_is_never_in_scope
    {unit : List Definition} (unique : UniqueNames unit) {d : Definition}
    (hd : d ∈ unit) (unselected : d.selected = false) (reserved : d.reserved = true) :
    d.name ∉ scope unit := by
  intro hm
  rcases every_scope_member_is_selected_or_internal unit d.name hm with
    ⟨d', hd', hname, hadm⟩
  have := unique d' hd' d hd hname
  subst this
  rcases hadm with h | ⟨_, h⟩
  · rw [unselected] at h; exact Bool.false_ne_true h
  · rw [reserved] at h; exact Bool.noConfusion h

theorem step_superset (unit : List Definition) (s : List Name) :
    ∀ m ∈ s, m ∈ step unit s := fun _ hm => List.mem_append_left _ hm

theorem iterate_superset (unit : List Definition) :
    ∀ (k : Nat) (s : List Name), ∀ m ∈ s, m ∈ iterate unit k s := by
  intro k
  induction k with
  | zero => intro s m hm; exact hm
  | succ k ih => intro s m hm; exact ih (step unit s) m (step_superset unit s m hm)

/-- The selection is always in scope: no selected definition loses its
derivation. -/
theorem the_selection_is_in_scope (unit : List Definition) :
    ∀ m ∈ selectedNames unit, m ∈ scope unit :=
  iterate_superset unit unit.length _

/-- The bodies a call to `n` in this unit may run. The unit's own definition
always may. A definition of external linkage may in addition be bypassed or
interposed (C11 6.7.4p7 for an `inline` definition), running any body
`elsewhere` supplies; an internal one cannot (C11 6.2.2p3). -/
def MayRun (unit : List Definition) (elsewhere : Name → List Definition) (n : Name)
    (b : Definition) : Prop :=
  (b ∈ unit ∧ b.name = n) ∨
    ((∃ d ∈ unit, d.name = n ∧ d.linkage = .external) ∧ b ∈ elsewhere n)

/-- A newly admitted callee — an unselected scope member — runs only the body
this unit analysed, whatever `elsewhere` supplies. This is what makes a summary
validated against that body a statement about every call to it here. -/
theorem an_admitted_callee_runs_only_its_analysed_body
    {unit : List Definition} (unique : UniqueNames unit)
    (elsewhere : Name → List Definition) {d : Definition} (hd : d ∈ unit)
    (unselected : d.selected = false) (inScope : d.name ∈ scope unit)
    {b : Definition} (run : MayRun unit elsewhere d.name b) : b = d := by
  rcases every_scope_member_is_selected_or_internal unit d.name inScope with
    ⟨d', hd', hname, hadm⟩
  have hsame := unique d' hd' d hd hname
  subst hsame
  have internal : d'.linkage = .internal := by
    rcases hadm with h | ⟨h, _⟩
    · rw [unselected] at h; exact absurd h Bool.false_ne_true
    · exact h
  rcases run with ⟨hb, hbn⟩ | ⟨⟨e, he, hen, hext⟩, _⟩
  · exact unique b hb d' hd' hbn
  · have := unique e he d' hd' hen
    subst this
    rw [internal] at hext
    exact Linkage.noConfusion hext

/-- The linkage premise is necessary: an external definition's call may run a
body other than the one analysed. -/
theorem an_external_definition_may_run_another_body :
    let d : Definition := ⟨"f", .external, false, false, []⟩
    let other : Definition := ⟨"f", .external, true, false, ["g"]⟩
    MayRun [d] (fun _ => [other]) "f" other ∧ other ≠ d := by
  refine ⟨Or.inr ⟨⟨_, List.mem_singleton_self _, rfl, rfl⟩, List.mem_singleton_self _⟩, ?_⟩
  decide

/-! ## Installing summaries -/

abbrev Env (σ : Type) := Name → Option σ

/-- Declared contracts first; an inferred summary only where none is declared. -/
def install {σ : Type} (declared inferred : Env σ) : Env σ :=
  fun n => match declared n with
    | some k => some k
    | none => inferred n

/-- The inferred environment: a derivation run over the scope only. -/
def inferredOver {σ : Type} (inScope : Name → Bool) (derive : Name → Option σ) : Env σ :=
  fun n => if inScope n then derive n else none

theorem a_declaration_is_never_overridden {σ : Type} (declared inferred : Env σ)
    {n : Name} {k : σ} (h : declared n = some k) : install declared inferred n = some k := by
  simp [install, h]

theorem an_installed_inference_is_exactly_the_derived_contract {σ : Type}
    (declared inferred : Env σ) {n : Name} (h : declared n = none) :
    install declared inferred n = inferred n := by
  simp [install, h]

/-- Anything installed that no declaration supplied lies in scope and was
validated, provided the derivation publishes only validated summaries. -/
theorem an_installed_inference_was_validated_in_scope {σ : Type}
    (declared : Env σ) (inScope : Name → Bool) (derive : Name → Option σ)
    (validates : Name → σ → Prop)
    (sound : ∀ n s, derive n = some s → validates n s)
    {n : Name} {s : σ} (hnone : declared n = none)
    (h : install declared (inferredOver inScope derive) n = some s) :
    inScope n = true ∧ validates n s := by
  rw [an_installed_inference_is_exactly_the_derived_contract _ _ hnone] at h
  unfold inferredOver at h
  cases hs : inScope n
  · rw [hs] at h; simp at h
  · rw [hs] at h
    exact ⟨rfl, sound n s (by simpa using h)⟩

/-- An existing summary survives the widening. Validation refuses a body with
an uncontracted callee (`strict`, condition (1) of the gate: an uncontracted
call is `Unknown`) and reads only its callees' contracts (`local_`). Then a
summary derived under `e₁` is derived unchanged under any `e₂` extending
`e₁`. -/
theorem an_existing_summary_survives_the_widening {σ : Type}
    (calls : Name → List Name) (body : Name → Env σ → Option σ)
    (strict : ∀ f e s, body f e = some s → ∀ c ∈ calls f, e c ≠ none)
    (local_ : ∀ f (e₁ e₂ : Env σ), (∀ c ∈ calls f, e₁ c = e₂ c) → body f e₁ = body f e₂)
    (e₁ e₂ : Env σ) (grow : ∀ c s, e₁ c = some s → e₂ c = some s)
    {f : Name} {s : σ} (h : body f e₁ = some s) : body f e₂ = some s := by
  rw [← local_ f e₁ e₂ ?_]
  · exact h
  · intro c hc
    have hne := strict f e₁ s h c hc
    cases he : e₁ c with
    | none => exact absurd he hne
    | some t => rw [grow c t he]

/-! ## Witnesses -/

def witnessUnit : List Definition :=
  [ ⟨"selected", .external, true, false, ["helper", "external_inline", "__bswap_32"]⟩
  , ⟨"helper", .internal, false, false, ["inner"]⟩
  , ⟨"inner", .internal, false, false, []⟩
  , ⟨"unreached", .internal, false, false, []⟩
  , ⟨"external_inline", .external, false, false, []⟩
  , ⟨"__bswap_32", .internal, false, true, []⟩ ]

/-- A reached static helper and its static callee are admitted; an unreached
static definition, a reached external `inline` definition and a reached
reserved (library header) definition are not. -/
theorem the_witness_scope :
    "helper" ∈ scope witnessUnit ∧ "inner" ∈ scope witnessUnit ∧
      "unreached" ∉ scope witnessUnit ∧ "external_inline" ∉ scope witnessUnit ∧
      "__bswap_32" ∉ scope witnessUnit := by
  decide

end QCore.CalleeSummaryScope
