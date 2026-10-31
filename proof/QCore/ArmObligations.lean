/-! Card ARM-OBLIGATIONS (`GATEWAY-RULE-API-ARM-OBLIGATIONS`): the gateway
carries a rule package admission's obligations into the published row.

A package admits a position with a lowering and a list of obligations
(`qcore_gateway_rule_api::Admission`): named premises, which the row must list
among its assumptions, and open obligations, which keep it in the denominator.
The host (`crates/qcore-c-gateway/src/arm_obligations.rs`) carries them into
the frame of the unit whose analysis asked ([`carry`]); with no frame open no
row could carry them, and the admission is refused. Every row of the unit then
carries the frame's obligations ([`apply`]): its carried list grows by them,
and an open obligation joins its verdict with `Unknown`.

The law: an admission's lowering is never used without its obligations (they
are carried or the admission is refused), carriage never erases an obligation
(frames and rows only grow), and a row carrying an undischarged (open)
obligation is never `Safe`. A carriage of premises alone, or of nothing,
leaves the verdict unchanged, so the card moves no verdict where no admission
has an open obligation.

Premises kept separate (trust boundary 28): the Rust/model correspondence,
pinned by `applyVectors` (read by the gateway's unit test), and the claim that
every consultation in the gateway goes through the carriage, pinned by the
gateway's source scan. -/
namespace QCore.ArmObligations

/-- The four-value verdict (§L21.2). -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

/-- `Safe < Pending < Unknown < Unsafe`. -/
def Verdict.rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- The join of the verdict order. -/
def Verdict.join : Verdict → Verdict → Verdict
  | .«unsafe», _ => .«unsafe»
  | _, .«unsafe» => .«unsafe»
  | .unknown, _ => .unknown
  | _, .unknown => .unknown
  | .pending, _ => .pending
  | _, .pending => .pending
  | .safe, .safe => .safe

/-- An admission's obligation. -/
inductive Obligation where
  | premise (name : String)
  | open (key : String)
  deriving DecidableEq, Repr

/-- An admission: the position's lowering and every obligation it rests on. -/
structure Admission (L : Type) where
  lowering : L
  obligations : List Obligation

/-- A frame's (or a row's) carried obligations. -/
abbrev Carried := List Obligation

/-- The host's carriage of one admission: the lowering and the frame after
it, or `none` (a refusal). `frame` is `none` when no unit frame is open. -/
def carry {L : Type} (frame : Option Carried) (a : Admission L) :
    Option (L × Option Carried) :=
  if a.obligations = [] then some (a.lowering, frame)
  else
    match frame with
    | none => none
    | some carried => some (a.lowering, some (carried ++ a.obligations))

/-- An undischarged obligation. -/
def Obligation.isOpen : Obligation → Bool
  | .open _ => true
  | .premise _ => false

def hasOpen (carried : Carried) : Bool := carried.any Obligation.isOpen

/-- A published row: its verdict and what it carries. -/
structure Row where
  verdict : Verdict
  carried : Carried

/-- A row carrying an open obligation is not `Safe`. -/
def Row.wellFormed (r : Row) : Prop := hasOpen r.carried = true → r.verdict ≠ .safe

/-- Carry a frame's obligations into one of its rows. -/
def apply (carried : Carried) (r : Row) : Row :=
  { verdict := if hasOpen carried then r.verdict.join .unknown else r.verdict
    carried := r.carried ++ carried }

/-- A nested analysis's frame closes into its enclosing frame. -/
def close (outer inner : Carried) : Carried := outer ++ inner

theorem the_join_with_unknown_is_never_safe (v : Verdict) : v.join .unknown ≠ .safe := by
  cases v <;> decide

theorem the_join_never_lowers (v w : Verdict) : v.rank ≤ (v.join w).rank := by
  cases v <;> cases w <;> decide

theorem an_admission_without_obligations_is_its_lowering {L : Type}
    (frame : Option Carried) (a : Admission L) (h : a.obligations = []) :
    carry frame a = some (a.lowering, frame) := by
  simp [carry, h]

theorem an_uncarriable_obligation_is_refused {L : Type} (a : Admission L)
    (h : a.obligations ≠ []) : carry none a = none := by
  simp [carry, h]

/-- The lowering is never used without its obligations: an admission with an
obligation that the host answers leaves a frame holding every one of them. -/
theorem the_lowering_never_travels_without_its_obligations {L : Type}
    (frame : Option Carried) (a : Admission L) (l : L) (after : Option Carried)
    (hc : carry frame a = some (l, after)) (h : a.obligations ≠ []) :
    ∃ carried, after = some carried ∧ ∀ o ∈ a.obligations, o ∈ carried := by
  cases frame with
  | none => simp [carry, h] at hc
  | some before =>
    simp [carry, h] at hc
    obtain ⟨_, rfl⟩ := hc
    exact ⟨before ++ a.obligations, rfl, fun o ho => List.mem_append.mpr (Or.inr ho)⟩

/-- Carriage never erases what a frame already carried. -/
theorem the_carriage_never_erases {L : Type} (before : Carried) (a : Admission L)
    (l : L) (after : Option Carried) (hc : carry (some before) a = some (l, after)) :
    ∃ carried, after = some carried ∧ ∀ o ∈ before, o ∈ carried := by
  by_cases h : a.obligations = []
  · simp [carry, h] at hc
    obtain ⟨_, rfl⟩ := hc
    exact ⟨before, rfl, fun _ ho => ho⟩
  · simp [carry, h] at hc
    obtain ⟨_, rfl⟩ := hc
    exact ⟨before ++ a.obligations, rfl, fun o ho => List.mem_append.mpr (Or.inl ho)⟩

/-- Carriage preserves every obligation, old and new. -/
theorem the_carriage_preserves_every_obligation {L : Type} (before : Carried)
    (a : Admission L) (l : L) (after : Option Carried)
    (hc : carry (some before) a = some (l, after)) :
    ∃ carried, after = some carried ∧
      (∀ o ∈ before, o ∈ carried) ∧ (∀ o ∈ a.obligations, o ∈ carried) := by
  by_cases h : a.obligations = []
  · simp [carry, h] at hc
    obtain ⟨_, rfl⟩ := hc
    exact ⟨before, rfl, fun _ ho => ho, fun o ho => by simp [h] at ho⟩
  · simp [carry, h] at hc
    obtain ⟨_, rfl⟩ := hc
    exact ⟨before ++ a.obligations, rfl, fun o ho => List.mem_append.mpr (Or.inl ho),
      fun o ho => List.mem_append.mpr (Or.inr ho)⟩

theorem a_nested_frame_hands_on_its_obligations (outer inner : Carried) :
    (∀ o ∈ outer, o ∈ close outer inner) ∧ (∀ o ∈ inner, o ∈ close outer inner) :=
  ⟨fun _ ho => List.mem_append.mpr (Or.inl ho), fun _ ho => List.mem_append.mpr (Or.inr ho)⟩

/-- The row carries every obligation of its frame and keeps its own. -/
theorem the_row_carries_every_obligation (carried : Carried) (r : Row) :
    (∀ o ∈ carried, o ∈ (apply carried r).carried) ∧
      (∀ o ∈ r.carried, o ∈ (apply carried r).carried) :=
  ⟨fun _ ho => List.mem_append.mpr (Or.inr ho), fun _ ho => List.mem_append.mpr (Or.inl ho)⟩

/-- A row with an undischarged obligation is never `Safe`. -/
theorem an_open_obligation_is_never_safe (carried : Carried) (r : Row)
    (h : hasOpen carried = true) : (apply carried r).verdict ≠ .safe := by
  simp only [apply, h, ite_true]
  exact the_join_with_unknown_is_never_safe r.verdict

theorem hasOpen_append (a b : Carried) : hasOpen (a ++ b) = (hasOpen a || hasOpen b) := by
  simp [hasOpen, List.any_append]

/-- Carriage keeps every row well formed: whatever open obligation a row
carries afterwards, it is not `Safe`. -/
theorem the_carriage_keeps_rows_well_formed (carried : Carried) (r : Row)
    (h : r.wellFormed) : (apply carried r).wellFormed := by
  intro hopen
  by_cases hc : hasOpen carried = true
  · exact an_open_obligation_is_never_safe carried r hc
  · have hr : hasOpen r.carried = true := by
      simp only [apply, hasOpen_append] at hopen
      cases hr : hasOpen r.carried <;> simp_all
    simp only [apply, hc]
    exact h hr

/-- `Safe` only when every carried obligation is discharged (none is open). -/
theorem a_safe_row_carries_no_open_obligation (carried : Carried) (r : Row)
    (h : r.wellFormed) (hs : (apply carried r).verdict = .safe) :
    ∀ o ∈ (apply carried r).carried, o.isOpen = false := by
  intro o ho
  have wf := the_carriage_keeps_rows_well_formed carried r h
  cases hopen : o.isOpen
  · rfl
  · have : hasOpen (apply carried r).carried = true := by
      simp only [hasOpen, List.any_eq_true]
      exact ⟨o, ho, hopen⟩
    exact absurd hs (wf this)

/-- Carriage never improves a verdict. -/
theorem the_carriage_never_improves (carried : Carried) (r : Row) :
    r.verdict.rank ≤ (apply carried r).verdict.rank := by
  simp only [apply]
  split
  · exact the_join_never_lowers r.verdict .unknown
  · exact Nat.le_refl _

/-- A carriage of premises alone leaves the verdict unchanged. -/
theorem premises_alone_keep_the_verdict (carried : Carried) (r : Row)
    (h : hasOpen carried = false) : (apply carried r).verdict = r.verdict := by
  simp [apply, h]

/-- The identity: a unit whose admissions carry nothing leaves its rows as
they were. -/
theorem the_empty_carriage_is_the_identity (r : Row) : apply [] r = r := by
  cases r
  simp [apply, hasOpen]

/-- The Rust table `APPLY_VECTORS` (`tests/unit/arm_obligations.rs`): the
verdict before, whether an open obligation is carried, the verdict after. -/
def applyVectors : List (Verdict × Bool × Verdict) := [
  (.safe, false, .safe),
  (.safe, true, .unknown),
  (.pending, false, .pending),
  (.pending, true, .unknown),
  (.unknown, false, .unknown),
  (.unknown, true, .unknown),
  (.«unsafe», false, .«unsafe»),
  (.«unsafe», true, .«unsafe»)
]

/-- The carried list a vector stands for. -/
def vectorCarried (hasOpenObligation : Bool) : Carried :=
  if hasOpenObligation then [.open "k"] else [.premise "p"]

theorem the_apply_vectors_are_apply :
    applyVectors.all (fun row =>
      (apply (vectorCarried row.2.1) ⟨row.1, []⟩).verdict == row.2.2) = true := by
  decide

/-! ## Row grain (card ARM-OBLIGATIONS-UNIT-GRAIN)

A frame's obligations are attributed (`FrameCarriage` in
`arm_obligations.rs`): an admission of a named declaration (rule 37's
checked specification) to that declaration, every other admission (a host's,
a nested frame's hand-up) to the unit. A row carries the unit's obligations
and those of every declaration its derivation reaches ([`rowGrain`]).

The derivation's consultations are a relation `consulted m n` (function `m`'s
work consulted the name `n`); `Used` is its reflexive-transitive closure from
the row, so a use through an in-unit callee's summary is a use. The gateway
records a graph `direct` (`contract_dependencies::attribution`) and computes
the set of names a row reaches over it (`reaching`). Two premises are kept
separate (trust boundary 28): the recorded graph holds every consultation
(`complete`, rule 13f's never-omits claim, with the closure following every
function with work), and the computed set holds every name reachable in the
recorded graph (`closed`, pinned by the gateway's unit tests of `reaching`).
Under them a row's set is a subset of the unit's and holds every obligation
its derivation used. -/

/-- Where a frame attributes an obligation. -/
inductive Anchor where
  /-- Not established: every row of the unit carries it. -/
  | unit
  /-- The rows whose derivation reaches the named declaration carry it. -/
  | declaration (name : String)
  deriving DecidableEq, Repr

/-- A frame's carriage, by attribution. -/
abbrev FrameCarriage := List (Anchor × Carried)

/-- The unit grain: everything the frame carried. -/
def unitGrain (f : FrameCarriage) : Carried := f.flatMap (·.2)

/-- Whether a row that reaches the names `reach` says is reached by the
anchor's attribution. -/
def Anchor.reachedBy (reach : String → Bool) : Anchor → Bool
  | .unit => true
  | .declaration n => reach n

/-- The row grain: the obligations of every entry whose anchor the row
reaches. -/
def rowGrain (f : FrameCarriage) (reach : String → Bool) : Carried :=
  (f.filter (fun e => e.1.reachedBy reach)).flatMap (·.2)

/-- A nested frame closes into its enclosing one at unit grain. -/
def closeInto (outer : FrameCarriage) (inner : FrameCarriage) : FrameCarriage :=
  outer ++ [(.unit, unitGrain inner)]

/-- Reachability over a consultation relation, from a row. -/
inductive Reaches (step : String → String → Prop) : String → String → Prop where
  | refl (r : String) : Reaches step r r
  | tail {r m n : String} : Reaches step r m → step m n → Reaches step r n

/-- The names a row's derivation used: its own, and every name the work of a
function it used consulted. -/
abbrev Used (consulted : String → String → Prop) (r n : String) : Prop :=
  Reaches consulted r n

theorem mem_rowGrain {f : FrameCarriage} {reach : String → Bool} {o : Obligation} :
    o ∈ rowGrain f reach ↔ ∃ e ∈ f, e.1.reachedBy reach = true ∧ o ∈ e.2 := by
  simp [rowGrain, List.mem_flatMap, List.mem_filter, and_assoc]

theorem mem_unitGrain {f : FrameCarriage} {o : Obligation} :
    o ∈ unitGrain f ↔ ∃ e ∈ f, o ∈ e.2 := by
  simp [unitGrain, List.mem_flatMap]

/-- Row grain refines unit grain: a row's set is a subset of the unit's. -/
theorem the_row_grain_refines_the_unit_grain (f : FrameCarriage) (reach : String → Bool)
    (o : Obligation) (h : o ∈ rowGrain f reach) : o ∈ unitGrain f := by
  obtain ⟨e, he, _, ho⟩ := mem_rowGrain.mp h
  exact mem_unitGrain.mpr ⟨e, he, ho⟩

/-- An obligation whose attribution is not established reaches every row. -/
theorem an_unattributed_obligation_reaches_every_row (f : FrameCarriage)
    (reach : String → Bool) (c : Carried) (hc : (.unit, c) ∈ f) (o : Obligation)
    (ho : o ∈ c) : o ∈ rowGrain f reach :=
  mem_rowGrain.mpr ⟨(.unit, c), hc, rfl, ho⟩

/-- Attribution unknown for every declaration is the unit grain. -/
theorem an_unknown_attribution_is_the_unit_grain (f : FrameCarriage) :
    rowGrain f (fun _ => true) = unitGrain f := by
  have : ∀ e : Anchor × Carried, e.1.reachedBy (fun _ => true) = true := by
    intro e
    cases e.1 <;> rfl
  simp [rowGrain, unitGrain, List.filter_eq_self.mpr (fun e _ => this e)]

/-- Reachability in a complete record: every use is recorded. -/
theorem a_used_name_is_reached {consulted : String → String → Prop}
    {direct : String → String → Prop} (complete : ∀ m n, consulted m n → direct m n)
    {r n : String} (h : Used consulted r n) : Reaches direct r n := by
  induction h with
  | refl => exact .refl _
  | tail _ hstep ih => exact .tail ih (complete _ _ hstep)

/-- The row grain holds every obligation the row's derivation used: given a
record complete for the derivation's consultations and a reach set closed
over the record, an admission attributed to a declaration the row used is
carried by the row. -/
theorem the_row_grain_holds_every_obligation_its_derivation_used
    {consulted direct : String → String → Prop} (complete : ∀ m n, consulted m n → direct m n)
    (r : String) (reach : String → Bool) (closed : ∀ n, Reaches direct r n → reach n = true)
    (f : FrameCarriage) (s : String) (c : Carried) (hc : (.declaration s, c) ∈ f)
    (used : Used consulted r s) (o : Obligation) (ho : o ∈ c) : o ∈ rowGrain f reach :=
  mem_rowGrain.mpr ⟨(.declaration s, c), hc, closed s (a_used_name_is_reached complete used), ho⟩

/-- The transitive-callee case: a row that uses a function whose work used
the declaration carries the declaration's obligations. -/
theorem a_transitive_callee_use_is_carried
    {consulted direct : String → String → Prop} (complete : ∀ m n, consulted m n → direct m n)
    (r g s : String) (reach : String → Bool) (closed : ∀ n, Reaches direct r n → reach n = true)
    (hrg : consulted r g) (hgs : consulted g s)
    (f : FrameCarriage) (c : Carried) (hc : (.declaration s, c) ∈ f) (o : Obligation)
    (ho : o ∈ c) : o ∈ rowGrain f reach :=
  the_row_grain_holds_every_obligation_its_derivation_used complete r reach closed f s c hc
    (.tail (.tail (.refl r) hrg) hgs) o ho

/-- The nested case: everything a nested frame carried reaches every row of
the enclosing frame (unit grain), whatever the row reaches. -/
theorem a_nested_frame_reaches_every_enclosing_row (outer inner : FrameCarriage)
    (reach : String → Bool) (o : Obligation) (ho : o ∈ unitGrain inner) :
    o ∈ rowGrain (closeInto outer inner) reach :=
  an_unattributed_obligation_reaches_every_row _ reach _
    (List.mem_append.mpr (Or.inr (List.mem_singleton.mpr rfl))) o ho

/-- Precision: an attributed obligation does not reach a row that does not
reach its declaration, unless another entry carries it. -/
theorem an_unreached_declaration_is_not_carried (f : FrameCarriage) (reach : String → Bool)
    (o : Obligation)
    (only : ∀ e ∈ f, o ∈ e.2 → ∃ s, e.1 = .declaration s ∧ reach s = false) :
    o ∉ rowGrain f reach := by
  intro h
  obtain ⟨e, he, hr, ho⟩ := mem_rowGrain.mp h
  obtain ⟨s, hs, hfalse⟩ := only e he ho
  rw [hs] at hr
  simp [Anchor.reachedBy, hfalse] at hr

/-- A row carrying an open obligation it rests on is never `Safe`; so a
`Safe` row rests on no open obligation: neither an unattributed one nor one
of a declaration its derivation used. -/
theorem a_safe_row_rests_on_no_open_obligation
    {consulted direct : String → String → Prop} (complete : ∀ m n, consulted m n → direct m n)
    (r : String) (reach : String → Bool) (closed : ∀ n, Reaches direct r n → reach n = true)
    (f : FrameCarriage) (row : Row) (wf : row.wellFormed)
    (hs : (apply (rowGrain f reach) row).verdict = .safe) :
    (∀ c, (.unit, c) ∈ f → hasOpen c = false) ∧
      (∀ s c, (.declaration s, c) ∈ f → Used consulted r s → hasOpen c = false) := by
  have none_open : ∀ o ∈ rowGrain f reach, o.isOpen = false := fun o ho =>
    a_safe_row_carries_no_open_obligation (rowGrain f reach) row wf hs o
      (List.mem_append.mpr (Or.inr ho))
  have closedC : ∀ c : Carried, (∀ o ∈ c, o ∈ rowGrain f reach) → hasOpen c = false := by
    intro c hc
    cases h : hasOpen c
    · rfl
    · simp only [hasOpen, List.any_eq_true] at h
      obtain ⟨o, ho, hopen⟩ := h
      exact absurd (none_open o (hc o ho)) (by simp [hopen])
  refine ⟨fun c hc => closedC c (fun o ho =>
      an_unattributed_obligation_reaches_every_row f reach c hc o ho), ?_⟩
  intro s c hc used
  exact closedC c (fun o ho =>
    the_row_grain_holds_every_obligation_its_derivation_used complete r reach closed f s c hc
      used o ho)

end QCore.ArmObligations
