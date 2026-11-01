import QCore.CallArgumentAccess

/-!
# Constant-index integer elements at a cell-bearing call
(card MK-EVENT-CELL-ARRAY-SLOTS, gateway rule 38a)

The abstract rule implemented by `element_read`, the `cell_bearing` arm of
`Parser::candidate_arguments` and `AdmittedArgument::matches` in
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`, with the member
class `TypedAccessModel::integer_array_length` and `canonical_decimal` of
`typed_field_access.rs`.

The lexical access gate refuses a call actual that reads memory. Card
W44-INDIRECT-CALL-ARGUMENT-ACCESS (`QCore.CallArgumentAccess`) admits typed
reads such as the scalar member read `B->F`. This card admits one more actual:
an element `B->F[K]`, `S.F[K]`, or its address `&B->F[K]`, `&S.F[K]`. It
must satisfy all of the following:

* `F` is an integer array member of declared length `N`, with one dimension
  and a literal length;
* `K` is a canonical decimal literal with `K < N`;
* the base is typed as W44 types the base of `B->F`;
* the position is `Borrowed` in a withheld declaration that carries a
  descriptor-cell state.

Rule 38 then decides the cell. The admitted actual is then handed to W44
**as the member read of its base**: the same shape, the same bases and the
same names. So every obligation, diagnostic and accusation of
`QCore.CallArgumentAccess` applies to it unchanged.

Rule 12t classifies a withheld call under a minted name. The probe's admitted
actuals of a cell-bearing call are also matched under that name, and only
under the name of their own callee.

## What this is not

This models the **decision**, not C. The following are review premises (trust
boundary 28):

* faithful extraction of tokens and declarations;
* that an in-bounds element of an integer array is an integer object inside
  the member (C11 6.5.2.1, 6.5.6p8), so its address holds no pointer;
* that the minted name stands for the same callee and positions;
* the correspondence with the compiled Rust.

Rule 38's own decision is `QCore.DescriptorCells`. Here it is an opaque answer
that the switch can only replace by a refusal.
-/

namespace QCore.CellArraySlots

open QCore.CallArgumentAccess (Position Shape Actual Statement Instr lower Facts
  readDiagnostics Diagnostic)

/-! ## The element -/

/-- The member `F`, as the total-or-reject declaration reader read it. -/
inductive Member where
  /-- `INTEGER-TYPE F [ N ]`: one dimension, a literal length, no `*`, no
  `volatile`/`_Atomic`. -/
  | integerArray (length : Nat)
  /-- An array of floating, `_Bool`, enumeration or plain `char` elements. -/
  | otherArray (length : Nat)
  /-- A pointer member: `F[K]` reads through a pointer, not an array. -/
  | pointer
  /-- A multi-dimensional, computed-length or unread member. -/
  | other
  deriving DecidableEq, Repr

/-- The index `K`, as spelled. -/
inductive Index where
  /-- A canonical decimal literal (`0`, or a nonzero digit then digits). -/
  | literal (k : Nat)
  /-- An octal or hexadecimal spelling, a suffix, or a constant expression. -/
  | nonCanonical
  /-- `- k`. -/
  | negative
  /-- Any expression reading a variable. -/
  | variable
  deriving DecidableEq, Repr

/-- The base, as W44 types the base of `B->F` / `S.F`. -/
inductive Base where
  /-- `B` declared `struct T *` (name `b`). -/
  | recordPointer (b : Nat)
  /-- `S` declared `struct T`, a record of this frame. -/
  | record
  /-- Anything else. -/
  | untyped
  deriving DecidableEq, Repr

structure Element where
  base : Base
  member : Member
  index : Index
  /-- The `&` form. -/
  address : Bool
  position : Position
  /-- The callee's contract in force is a withheld (retaining) declaration. -/
  withheld : Bool
  /-- That declaration carries a descriptor-cell state (§C04.6.2). -/
  cellBearing : Bool
  deriving DecidableEq, Repr

/-- The index names an element inside the member: `K < N`. -/
def inBounds : Member → Index → Bool
  | .integerArray n, .literal k => decide (k < n)
  | _, _ => false

def Base.typed : Base → Bool
  | .untyped => false
  | _ => true

/-- `element_read`, asked only at a cell-bearing withheld declaration's
`Borrowed` position. -/
def admits (e : Element) : Bool :=
  e.withheld && e.cellBearing && e.position == .borrowed && inBounds e.member e.index &&
    e.base.typed

/-! ## The read W44 already obligates -/

def Base.bases : Base → List Nat
  | .recordPointer b => [b]
  | _ => []

/-- The typed scalar member read `B->F` (`S.F`) at `p`, as
`QCore.CallArgumentAccess` reads it: a scalar value over `B`, reading `B`. -/
def memberRead (b : Base) (p : Position) : Actual :=
  ⟨p, true, .scalar b.bases, b.bases⟩

/-- The actual the recogniser hands W44: the member read when admitted, a
refused shape otherwise. -/
def Element.actual (e : Element) : Actual :=
  if admits e then memberRead e.base e.position else ⟨e.position, true, .refused, e.base.bases⟩

/-! ## What is admitted -/

theorem inBounds_eq_true (m : Member) (i : Index) :
    inBounds m i = true ↔ ∃ k n, i = .literal k ∧ m = .integerArray n ∧ k < n := by
  cases m <;> cases i <;> simp [inBounds]

theorem admits_eq_true (e : Element) :
    admits e = true ↔ e.withheld = true ∧ e.cellBearing = true ∧ e.position = .borrowed ∧
      inBounds e.member e.index = true ∧ e.base.typed = true := by
  simp only [admits, Bool.and_eq_true, beq_iff_eq, and_assoc]

theorem an_admitted_element_is_in_bounds (e : Element) (h : admits e = true) :
    ∃ k n, e.index = .literal k ∧ e.member = .integerArray n ∧ k < n :=
  (inBounds_eq_true e.member e.index).mp ((admits_eq_true e).mp h).2.2.2.1

theorem an_admitted_element_is_at_a_borrowed_position (e : Element) (h : admits e = true) :
    e.position = .borrowed :=
  ((admits_eq_true e).mp h).2.2.1

theorem an_admitted_element_is_at_a_cell_bearing_withheld_declaration (e : Element)
    (h : admits e = true) : e.withheld = true ∧ e.cellBearing = true :=
  ⟨((admits_eq_true e).mp h).1, ((admits_eq_true e).mp h).2.1⟩

/-- Refusal by any one conjunct. -/
theorem admits_eq_false_of_not_in_bounds (e : Element) (h : inBounds e.member e.index = false) :
    admits e = false := by
  cases hadm : admits e
  · rfl
  · rw [((admits_eq_true e).mp hadm).2.2.2.1] at h
    cases h

theorem admits_eq_false_of_position (e : Element) (h : e.position ≠ .borrowed) :
    admits e = false := by
  cases hadm : admits e
  · rfl
  · exact absurd ((admits_eq_true e).mp hadm).2.2.1 h

theorem a_custody_position_never_admits (e : Element) (h : e.position.takesCustody = true) :
    admits e = false := by
  apply admits_eq_false_of_position
  intro hp
  rw [hp] at h
  cases h

theorem a_data_tail_never_admits (e : Element) (h : e.position = .tail) : admits e = false := by
  apply admits_eq_false_of_position
  rw [h]
  intro hp
  cases hp

theorem a_variable_index_never_admits (e : Element) (h : e.index = .variable) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [h]
  cases e.member <;> rfl

theorem a_negative_index_never_admits (e : Element) (h : e.index = .negative) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [h]
  cases e.member <;> rfl

theorem a_non_canonical_index_never_admits (e : Element) (h : e.index = .nonCanonical) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [h]
  cases e.member <;> rfl

theorem an_out_of_bounds_index_never_admits (e : Element) (k n : Nat)
    (hi : e.index = .literal k) (hm : e.member = .integerArray n) (h : n ≤ k) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [hi, hm]
  exact decide_eq_false (Nat.not_lt.mpr h)

theorem a_pointer_member_never_admits (e : Element) (h : e.member = .pointer) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [h]
  cases e.index <;> rfl

theorem a_non_integer_element_never_admits (e : Element) (n : Nat) (h : e.member = .otherArray n) :
    admits e = false := by
  apply admits_eq_false_of_not_in_bounds
  rw [h]
  cases e.index <;> rfl

theorem a_declaration_without_cells_never_admits (e : Element) (h : e.cellBearing = false) :
    admits e = false := by
  cases hadm : admits e
  · rfl
  · rw [((admits_eq_true e).mp hadm).2.1] at h
    cases h

theorem a_function_map_callee_never_admits (e : Element) (h : e.withheld = false) :
    admits e = false := by
  cases hadm : admits e
  · rfl
  · rw [((admits_eq_true e).mp hadm).1] at h
    cases h

theorem an_untyped_base_never_admits (e : Element) (h : e.base = .untyped) :
    admits e = false := by
  cases hadm : admits e
  · rfl
  · have ht := ((admits_eq_true e).mp hadm).2.2.2.2
    rw [h] at ht
    cases ht

/-! ## Soundness relative to the typed read -/

/-- The law: an admitted element (read or address) is handed to W44 as exactly
the typed member read of its base at its position. -/
theorem an_admitted_element_reads_as_the_member_read (e : Element) (h : admits e = true) :
    e.actual = memberRead e.base e.position := by
  simp [Element.actual, h]

theorem an_admitted_element_is_a_candidate (e : Element) (h : admits e = true) :
    e.actual.candidate = true := by
  rw [an_admitted_element_reads_as_the_member_read e h, an_admitted_element_is_at_a_borrowed_position e h]
  rfl

theorem a_refused_element_is_never_a_candidate (e : Element) (h : admits e = false) :
    e.actual.candidate = false := by
  simp [Element.actual, h, Actual.candidate, Shape.recognised]

/-- Every base an admitted element reads through carries the typed read's
obligation in its statement's lowering. -/
theorem an_admitted_element_obligates_its_base (s : Statement) (e : Element) (b : Nat)
    (hmem : e.actual ∈ s.actuals) (h : admits e = true) (hb : e.base = .recordPointer b) :
    b ∈ s.bases := by
  have hused : e.actual ∈ s.used := by
    simp only [Statement.used, List.mem_filter]
    exact ⟨hmem, an_admitted_element_is_a_candidate e h⟩
  apply QCore.CallArgumentAccess.every_used_base_is_obligated s e.actual b hused
  rw [an_admitted_element_reads_as_the_member_read e h, hb]
  simp [memberRead, Base.bases, Shape.bases]

/-- A released base of an admitted element refuses and is accused, exactly
as for the member read (`a_released_base_refuses_and_is_accused`). -/
theorem a_released_base_of_an_admitted_element_refuses_and_is_accused (f : Facts)
    (s : Statement) (e : Element) (b site : Nat) (binding : QCore.IndirectScalarAccess.Binding)
    (flags : QCore.IndirectScalarAccess.Flags) (hmem : e.actual ∈ s.actuals)
    (h : admits e = true) (hbase : e.base = .recordPointer b)
    (hb : f.state.binding b = some binding) (ht : binding.targets = [site])
    (hk : binding.unknown = false) (hs : f.state.status site = some flags)
    (hr : flags.released = true) (hu : f.state.unresolved site = false) :
    Diagnostic.unknownAlias b ∈ readDiagnostics f s ∧
      QCore.IndirectScalarAccess.accuses f.state b = some site :=
  QCore.CallArgumentAccess.a_released_base_refuses_and_is_accused f s b site binding flags
    (an_admitted_element_obligates_its_base s e b hmem h hbase) hb ht hk hs hr hu

/-- An element alone never makes a statement admitted when it is refused: a
statement whose only memory-reading actual is a refused element keeps the
gate's refusal. -/
theorem a_statement_of_one_refused_element_keeps_the_refusal (s : Statement) (e : Element)
    (hs : s.actuals = [e.actual]) (h : admits e = false) : lower s = [.refusal] := by
  have hempty : s.used = [] := by
    simp [Statement.used, hs, a_refused_element_is_never_a_candidate e h]
  simp [lower, Statement.admitted, hempty]

/-! ## The minted name -/

/-- The callee a classification names: the plain callee or rule 12t's minted
`qcore_retained__` name for it. -/
inductive Name where
  | plain (callee : Nat)
  | minted (callee : Nat)
  deriving DecidableEq, Repr

/-- `AdmittedArgument::matches` on the callee: the minted name is recognised
only for an entry of a cell-bearing withheld declaration, and only as its own
callee. -/
def matchesName (cellBearing : Bool) (entry : Nat) : Name → Bool
  | .plain callee => callee == entry
  | .minted callee => cellBearing && callee == entry

theorem a_minted_name_matches_only_its_own_callee (cellBearing : Bool) (entry callee : Nat)
    (h : matchesName cellBearing entry (.minted callee) = true) :
    cellBearing = true ∧ callee = entry := by
  simpa only [matchesName, Bool.and_eq_true, beq_iff_eq] using h

theorem without_cells_the_minted_name_never_matches (entry callee : Nat) :
    matchesName false entry (.minted callee) = false := by
  simp [matchesName]

theorem the_plain_name_is_unchanged (cellBearing : Bool) (entry callee : Nat) :
    matchesName cellBearing entry (.plain callee) = (callee == entry) := rfl

/-! ## Rule 38 decides; its switch never widens -/

/-- Rule 38's answer for a cell call, opaque here (`QCore.DescriptorCells`). -/
inductive CellAnswer where
  | decided
  | refused
  deriving DecidableEq, Repr

/-- The outcome of a cell call carrying the element: the gate's refusal when
it is not admitted; otherwise rule 38's answer, which its switch, when off,
replaces by a refusal. -/
def outcome (e : Element) (rule38On : Bool) (answer : CellAnswer) : CellAnswer :=
  if admits e then (if rule38On then answer else .refused) else .refused

theorem the_switch_off_refuses_every_cell_call (e : Element) (answer : CellAnswer) :
    outcome e false answer = .refused := by
  simp [outcome]

theorem a_refused_element_never_reaches_rule_38 (e : Element) (on : Bool) (answer : CellAnswer)
    (h : admits e = false) : outcome e on answer = .refused := by
  simp [outcome, h]

theorem an_admitted_element_reaches_rule_38 (e : Element) (answer : CellAnswer)
    (h : admits e = true) : outcome e true answer = answer := by
  simp [outcome, h]

/-- The switch never widens: whatever rule 38 answers, off is never a decision
that on is not. -/
theorem the_switch_never_widens (e : Element) (answer : CellAnswer)
    (h : outcome e false answer = .decided) : outcome e true answer = .decided := by
  rw [the_switch_off_refuses_every_cell_call] at h
  cases h

/-! ## Non-vacuity -/

/-- `&sched->ch_events[0]` at `mk_event_channel_create`'s `r_fd` (Borrowed),
`sched` (name 0) declared `struct flb_sched *`, `ch_events` an `int[2]`. -/
def schedSlot : Element :=
  { base := .recordPointer 0, member := .integerArray 2, index := .literal 0, address := true,
    position := .borrowed, withheld := true, cellBearing := true }

theorem the_brief_slot_is_admitted : admits schedSlot = true := by decide

theorem the_second_slot_is_admitted : admits { schedSlot with index := .literal 1 } = true := by
  decide

theorem one_past_the_end_is_refused : admits { schedSlot with index := .literal 2 } = false := by
  decide

theorem at_mk_event_add_it_is_refused : admits { schedSlot with cellBearing := false } = false := by
  decide

/-- The creation statement lowers to the typed read of `sched`, the second
obligation, then the call. -/
def schedCreate : Statement :=
  { gateRefused := true
    readsDescriptor := false
    contracted := true
    actuals := [schedSlot.actual, { schedSlot with index := .literal 1 }.actual]
    probeIsCall := true }

theorem the_brief_creation_lowers_to_the_reads_then_the_call :
    lower schedCreate = [.typedRead [0, 0], .argumentReads [0, 0] [] [], .call] := by decide

theorem the_brief_creation_adds_no_diagnostic_over_a_live_base :
    readDiagnostics QCore.CallArgumentAccess.liveFacts schedCreate = [] := by decide

end QCore.CellArraySlots
