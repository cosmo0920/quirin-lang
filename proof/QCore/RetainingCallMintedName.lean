import QCore.CellArraySlots

/-!
# The minted name at every withheld declaration
(card RETAINING-CALL-MINTED-NAME-ACTUALS, gateway rule 38b)

The abstract rule implemented by `minted_admission`, `AdmittedArgument::matched`
and `minted_positions_borrowed` in
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`.

Rule 12t withholds a retaining declaration from the function map and
classifies a call of it a second time, under its minted name
(`qcore_retained__<callee>`). The W44 probe (`QCore.CallArgumentAccess`)
records each actual it admitted, keyed by the plain callee, the argument
index and the tokens. Rule 38a (`QCore.CellArraySlots`) let the minted name
match an entry only at a cell-bearing declaration. This card makes the
entry's flag `mintedAdmission`: every admitted actual of a cell-bearing call,
as before, and at any withheld declaration an actual admitted at a declared
`Borrowed` position that is not a rule-1 release.

`QCore.CellArraySlots.matchesName flag entry` is the match on the callee; its
first argument is now the entry's `mintedAdmission` flag, which equals the
cell-bearing flag of rule 38a at every cell-bearing declaration
(`rule_38a_is_the_cell_bearing_case`).

Proved here:

* under the minted name of its own callee a minted entry matches exactly the
  actuals it matches under the plain name, and an entry never matches the
  minted name of another callee;
* over any set of entries, the minted name never admits more than the plain
  name, and exactly as much when every entry of the callee is minted;
* a function-map callee, an uncontracted callee, a custody position, a data
  tail and a rule-1 release are never minted without cells, and an actual
  the probe refused is admitted under no name;
* an actual admitted under the minted name at a position the call in force
  reads as other than `Borrowed` (a callback's `resource_owned`) keeps the
  refusal;
* the decision table, entry for entry with the Rust vectors.

## What this is not

This models the decision, not C. Review premises (trust boundary 28):

* faithful extraction of tokens;
* that rule 12t's reserved prefix is escaped in source, so a plain name is
  never a minted one (`Name` has two constructors);
* that the minted name stands for the same call, actuals and declared
  positions (`custody_contract` changes only the result);
* the correspondence with the compiled Rust.
-/

namespace QCore.RetainingCallMintedName

open QCore.CallArgumentAccess (Position)
open QCore.CellArraySlots (Name matchesName)

/-! ## The flag -/

/-- What the probe knows of one admitted actual's call. -/
structure Site where
  /-- Rule 12t withholds the callee (a retaining declaration). -/
  withheld : Bool
  /-- The declaration carries a descriptor-cell state (§C04.6.2). -/
  cellBearing : Bool
  /-- The declared position of the actual. -/
  position : Position
  /-- Admitted as a rule-1 release (`member_release::admits_release`). -/
  release : Bool
  deriving DecidableEq, Repr

/-- `minted_admission`. -/
def mintedAdmission (s : Site) : Bool :=
  s.cellBearing || (s.withheld && s.position == .borrowed && !s.release)

theorem rule_38a_is_the_cell_bearing_case (s : Site) (h : s.cellBearing = true) :
    mintedAdmission s = true := by
  simp [mintedAdmission, h]

theorem every_withheld_borrowed_read_is_minted (s : Site) (hw : s.withheld = true)
    (hp : s.position = .borrowed) (hr : s.release = false) : mintedAdmission s = true := by
  simp [mintedAdmission, hw, hp, hr]

theorem a_function_map_callee_is_never_minted (s : Site) (hw : s.withheld = false)
    (hc : s.cellBearing = false) : mintedAdmission s = false := by
  simp [mintedAdmission, hw, hc]

theorem an_uncontracted_callee_is_never_minted (s : Site) (hp : s.position = .uncontracted)
    (hc : s.cellBearing = false) : mintedAdmission s = false := by
  simp [mintedAdmission, hp, hc]

theorem a_custody_position_is_never_minted_without_cells (s : Site)
    (hp : s.position ≠ .borrowed) (hc : s.cellBearing = false) : mintedAdmission s = false := by
  simp [mintedAdmission, hp, hc]

theorem a_data_tail_is_never_minted_without_cells (s : Site) (hp : s.position = .tail)
    (hc : s.cellBearing = false) : mintedAdmission s = false := by
  simp [mintedAdmission, hp, hc]

theorem a_release_is_never_minted_without_cells (s : Site) (hr : s.release = true)
    (hc : s.cellBearing = false) : mintedAdmission s = false := by
  simp [mintedAdmission, hr, hc]

/-- Without withholding, the flag is rule 38a's. -/
theorem off_a_withheld_declaration_the_flag_is_rule_38a_s (s : Site) (hw : s.withheld = false) :
    mintedAdmission s = s.cellBearing := by
  simp [mintedAdmission, hw]

/-! ## The match -/

/-- One admitted actual: its plain callee, argument index, tokens and site. -/
structure Entry where
  callee : Nat
  index : Nat
  texts : List Nat
  site : Site
  deriving DecidableEq, Repr

/-- `AdmittedArgument::matches`. -/
def Entry.matches (e : Entry) (n : Name) (i : Nat) (a : List Nat) : Bool :=
  matchesName (mintedAdmission e.site) e.callee n && e.index == i && e.texts == a

/-- `AdmittedArgument::matched`'s report: the match was under the minted name. -/
def Entry.mintedMatch (e : Entry) (n : Name) (i : Nat) (a : List Nat) : Bool :=
  e.matches n i a && match n with
    | .minted _ => true
    | .plain _ => false

theorem a_minted_entry_matches_its_minted_name_as_its_plain_name (e : Entry)
    (h : mintedAdmission e.site = true) (c i : Nat) (a : List Nat) :
    e.matches (.minted c) i a = e.matches (.plain c) i a := by
  simp [Entry.matches, matchesName, h]

theorem the_minted_name_matches_only_its_own_callee (e : Entry) (c i : Nat) (a : List Nat)
    (h : e.matches (.minted c) i a = true) : c = e.callee := by
  simp only [Entry.matches, matchesName, Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.2

theorem an_unminted_entry_never_matches_a_minted_name (e : Entry)
    (h : mintedAdmission e.site = false) (c i : Nat) (a : List Nat) :
    e.matches (.minted c) i a = false := by
  simp [Entry.matches, matchesName, h]

theorem the_plain_name_is_read_as_before (e : Entry) (c i : Nat) (a : List Nat) :
    e.matches (.plain c) i a = (c == e.callee && e.index == i && e.texts == a) := rfl

theorem a_minted_match_is_reported_as_minted (e : Entry) (c i : Nat) (a : List Nat) :
    e.mintedMatch (.minted c) i a = e.matches (.minted c) i a := by
  simp [Entry.mintedMatch]

theorem a_plain_match_is_never_reported_as_minted (e : Entry) (c i : Nat) (a : List Nat) :
    e.mintedMatch (.plain c) i a = false := by
  simp [Entry.mintedMatch]

/-! ## The admission set -/

/-- The actuals the gate lets out of a statement whose call is named `n`. -/
def admitted (es : List Entry) (n : Name) (i : Nat) (a : List Nat) : Bool :=
  es.any (fun e => e.matches n i a)

theorem a_minted_match_is_a_plain_match (e : Entry) (c i : Nat) (a : List Nat)
    (h : e.matches (.minted c) i a = true) : e.matches (.plain c) i a = true := by
  have hm : mintedAdmission e.site = true := by
    cases hf : mintedAdmission e.site
    · rw [an_unminted_entry_never_matches_a_minted_name e hf] at h
      exact absurd h (by decide)
    · rfl
  rwa [← a_minted_entry_matches_its_minted_name_as_its_plain_name e hm]

/-- The minted name never admits more than the plain name. -/
theorem the_minted_name_never_admits_more (es : List Entry) (c i : Nat) (a : List Nat)
    (h : admitted es (.minted c) i a = true) : admitted es (.plain c) i a = true := by
  simp only [admitted, List.any_eq_true] at h ⊢
  obtain ⟨e, he, hm⟩ := h
  exact ⟨e, he, a_minted_match_is_a_plain_match e c i a hm⟩

/-- Where every entry is minted, the minted name admits exactly the plain set. -/
theorem the_minted_name_admits_exactly_the_plain_set (es : List Entry) (c i : Nat)
    (a : List Nat) (h : ∀ e ∈ es, mintedAdmission e.site = true) :
    admitted es (.minted c) i a = admitted es (.plain c) i a := by
  unfold admitted
  induction es with
  | nil => rfl
  | cons e rest ih =>
    simp only [List.any_cons]
    rw [a_minted_entry_matches_its_minted_name_as_its_plain_name e
      (h e (List.mem_cons_self ..)) c i a]
    rw [ih (fun e' he' => h e' (List.mem_cons_of_mem _ he'))]

/-- An actual the probe refused has no entry, so no name admits it. -/
theorem a_refused_actual_is_admitted_under_no_name (es : List Entry) (n : Name) (i : Nat)
    (a : List Nat) (h : ∀ e ∈ es, ¬ (e.index = i ∧ e.texts = a)) :
    admitted es n i a = false := by
  simp only [admitted, List.any_eq_false, Entry.matches, Bool.and_eq_true, beq_iff_eq]
  intro e he hm
  exact h e he ⟨hm.1.2, hm.2⟩

/-! ## The position in force -/

/-- An entry used by the classification, with whether it left the gate under
the minted name (`minted_used`). -/
structure Used where
  entry : Entry
  mintedUsed : Bool
  deriving DecidableEq, Repr

/-- `minted_positions_borrowed`: every actual that left the gate under the
minted name sits at a `Borrowed` position of the call in force. -/
def positionsBorrowed (inForce : Nat → Position) (used : List Used) : Bool :=
  used.all (fun u => !u.mintedUsed || inForce u.entry.index == .borrowed)

/-- The probe's answer (`probe_call_arguments`): admitted only when the
classification is a contracted call, its rule-1 positions are rewritable and
every minted actual is still read. -/
def probeAdmits (contractedCall rewritable : Bool) (inForce : Nat → Position)
    (used : List Used) : Bool :=
  contractedCall && rewritable && positionsBorrowed inForce used

theorem a_minted_actual_at_an_owned_position_keeps_the_refusal (cc rw : Bool)
    (inForce : Nat → Position) (used : List Used) (u : Used) (hu : u ∈ used)
    (hm : u.mintedUsed = true) (ho : inForce u.entry.index = .owned) :
    probeAdmits cc rw inForce used = false := by
  have : positionsBorrowed inForce used = false := by
    simp only [positionsBorrowed, List.all_eq_false]
    exact ⟨u, hu, by simp [hm, ho]⟩
  simp [probeAdmits, this]

theorem plain_actuals_need_no_position_check (inForce : Nat → Position) (used : List Used)
    (h : ∀ u ∈ used, u.mintedUsed = false) : positionsBorrowed inForce used = true := by
  simp only [positionsBorrowed, List.all_eq_true]
  intro u hu
  simp [h u hu]

theorem an_admitted_probe_reads_every_minted_actual_borrowed (cc rw : Bool)
    (inForce : Nat → Position) (used : List Used) (h : probeAdmits cc rw inForce used = true)
    (u : Used) (hu : u ∈ used) (hm : u.mintedUsed = true) :
    inForce u.entry.index = .borrowed := by
  simp only [probeAdmits, positionsBorrowed, Bool.and_eq_true, List.all_eq_true] at h
  have := h.2 u hu
  simpa [hm] using this

/-! ## The decision table, shared with Rust -/

/-- `MINTED_ADMISSION_VECTORS`: `(withheld, cellBearing, position, release,
minted)`. Rust's `Position::Custody("released")`, `("consumed")` and
`("owned")` are `released`, `consumed` and `owned`. -/
def mintedAdmissionVectors : List (Bool × Bool × Position × Bool × Bool) :=
  [ (true, false, .borrowed, false, true),
    (true, false, .borrowed, true, false),
    (true, false, .released, true, false),
    (true, false, .consumed, false, false),
    (true, false, .owned, false, false),
    (true, false, .tail, false, false),
    (true, false, .beyond, false, false),
    (true, true, .borrowed, false, true),
    (true, true, .borrowed, true, true),
    (true, true, .released, true, true),
    (false, false, .borrowed, false, false),
    (false, false, .tail, false, false),
    (false, false, .released, true, false),
    (false, false, .uncontracted, false, false) ]

theorem minted_admission_vectors_hold :
    mintedAdmissionVectors.all (fun (w, c, p, r, m) =>
      mintedAdmission { withheld := w, cellBearing := c, position := p, release := r } == m)
      = true := by
  decide

/-! ## Non-vacuity -/

/-- `mk_event_add(loop, c->ch[0], …)`: the by-value element at `fd` (index 1)
of a withheld declaration without cells. -/
def mkEventAddFd : Entry :=
  { callee := 1, index := 1, texts := [10, 11, 12, 13, 14, 15],
    site := { withheld := true, cellBearing := false, position := .borrowed, release := false } }

/-- `flb_task_destroy(ic->task, …)`: a rule-1 release at a withheld declaration. -/
def taskDestroy : Entry :=
  { callee := 2, index := 0, texts := [20, 11, 21],
    site := { withheld := true, cellBearing := false, position := .released, release := true } }

theorem the_brief_mk_event_add_actual_is_admitted_under_the_minted_name :
    admitted [mkEventAddFd] (.minted 1) 1 [10, 11, 12, 13, 14, 15] = true := by
  decide

theorem a_withheld_release_is_not_admitted_under_the_minted_name :
    admitted [taskDestroy] (.minted 2) 0 [20, 11, 21] = false := by
  decide

theorem the_release_is_still_admitted_under_its_plain_name :
    admitted [taskDestroy] (.plain 2) 0 [20, 11, 21] = true := by
  decide

theorem another_callee_s_minted_name_admits_nothing :
    admitted [mkEventAddFd, taskDestroy] (.minted 3) 1 [10, 11, 12, 13, 14, 15] = false := by
  decide

end QCore.RetainingCallMintedName
