import QCore.RkLogCallerFrame

/-! Card RULE32A-PILOT: rule 32a as a rule package at the token hold, and
what "off" means for a hold.

The gateway's three token pipelines (`legacy_caller_frame_hold`) ask the token
hold `caller-frame-hold` (`rule-32a-caller-frame-hold`) for each call inside a
registered frame member's own body: the call is held when the callee is a
member of the same frame and its declaration is in force (`holds`, whose
table `holdVectors` is the package's `DECISION_VECTORS`). A held call reads
`frameHeld` (`QCore.RkLogCallerFrame`), so the package's decision is exactly
the model's `withinFrame` reading on a declared callee.

What off means. A hold's admission is the refusal of a reading. Two offs were
considered (pluggable-cards plan, section 6 item 10):

* a refusing off, as every other package's: the hold is not asked. That is the
  model's empty frame, under which the logger's own row reads its sibling's
  declaration and is `Safe` again (`a_refusing_off_would_widen`), the halted
  regression. It widens, so it is not offered.
* an off that holds more, never less: every call to a member held, from any
  caller. It certifies nothing the hold does not
  (`holding_more_never_certifies_a_link`), but it changes no member's row
  (`holding_more_changes_no_member_row`); every row it moves is a caller
  outside the frame (`holding_more_moves_an_outside_row`), which rule 32a
  leaves unchanged by definition
  (`QCore.RkLogCallerFrame.an_outside_caller_reads_the_declaration`). Its
  measured contribution would be rule 32's (the outside callers' reading of
  the declarations), recorded as rule 32a's.

So the hold is registered not switchable: a switch list naming it is refused
(`a_list_naming_the_hold_is_refused`), the host asks it under every list,
refused or not (`a_hold_is_asked_under_every_list`), and no accepted list
changes any row (`the_switch_never_removes_the_hold`,
`the_switch_never_widens`). The Rust/model correspondence (which tokens are
calls, which body holds one, the registry's rows) remains a premise (trust
boundary 28), pinned by the package's and the gateway's tests. -/
namespace QCore.CallerFrameHoldSwitch

open QCore.VerdictSemigroup
open QCore.RkLogCallerFrame

/-! ## The package's decision -/

/-- The decision for one frame: `(caller is a member, callee is a member,
callee's declaration in force)`. -/
def held (callerMember calleeMember inForce : Bool) : Bool :=
  callerMember && calleeMember && inForce

/-- The Rust table `DECISION_VECTORS`:
`(caller member, callee member, in force, held)`. -/
def holdVectors : List (Bool × Bool × Bool × Bool) :=
  [ (false, false, false, false)
  , (false, false, true, false)
  , (false, true, false, false)
  , (false, true, true, false)
  , (true, false, false, false)
  , (true, false, true, false)
  , (true, true, false, false)
  , (true, true, true, true) ]

theorem the_hold_vectors_are_the_package (callerMember calleeMember inForce : Bool) :
    (callerMember, calleeMember, inForce, held callerMember calleeMember inForce)
      ∈ holdVectors := by
  cases callerMember <;> cases calleeMember <;> cases inForce <;> decide

theorem every_hold_vector_is_the_package_decision :
    holdVectors.all (fun row => held row.1 row.2.1 row.2.2.1 == row.2.2.2) = true := by
  decide

/-- The package's decision on one call under the frame `members`. -/
def holds (members : List String) (caller callee : String) (inForce : Bool) : Bool :=
  held (decide (caller ∈ members)) (decide (callee ∈ members)) inForce

/-- A call is held exactly when it stays inside the frame (the model's
`withinFrame`) and the callee's declaration is in force. -/
theorem holds_iff (members : List String) (caller callee : String) (inForce : Bool) :
    holds members caller callee inForce = true ↔
      withinFrame members caller callee ∧ inForce = true := by
  simp [holds, held, withinFrame, and_assoc]

/-- A held call reads `Unknown` and never the callee's declaration. -/
theorem a_held_call_reads_unknown (members : List String) (caller callee : String)
    (h : holds members caller callee true = true) (v : V) :
    Item.value members caller (.call callee v) = .unknown := by
  have w := ((holds_iff members caller callee true).mp h).1
  simp [Item.value, w, a_frame_hold_is_unknown]

/-- An undeclared callee is never held: there is no declaration to read. -/
theorem an_undeclared_callee_is_not_held (members : List String) (caller callee : String) :
    holds members caller callee false = false := by
  simp [holds, held]

/-! ## The switch -/

/-- What a switch list does to a registry entry. -/
inductive Off where
  | refuses
  | notSwitchable
  deriving DecidableEq, Repr

/-- A switch list is accepted only when every id names a switchable entry
(`RulesOff::parse`: an unknown id or a hold is refused). -/
def accepts (registry : List (String × Off)) (off : List String) : Bool :=
  off.all fun id => registry.any fun entry => entry.1 == id && entry.2 == .refuses

/-- Whether the host asks an entry: a hold under every list (a refused list
included, `gateway_rules::enabled`), any other entry when not listed. -/
def asked (off : List String) (entry : String × Off) : Bool :=
  entry.2 == .notSwitchable || !(off.contains entry.1)

/-- Rule 32a's registry row. -/
def rule32a : String × Off := ("32a", .notSwitchable)

theorem a_list_naming_the_hold_is_refused (registry : List (String × Off))
    (unique : ∀ entry ∈ registry, entry.1 = "32a" → entry = rule32a)
    (off : List String) (named : "32a" ∈ off) :
    accepts registry off = false := by
  unfold accepts
  apply Bool.eq_false_iff.mpr
  intro all
  have := (List.all_eq_true.mp all) "32a" named
  obtain ⟨entry, member, hit⟩ := List.any_eq_true.mp this
  simp only [Bool.and_eq_true, beq_iff_eq] at hit
  have := unique entry member hit.1
  rw [this] at hit
  cases hit.2

theorem a_hold_is_asked_under_every_list (off : List String) (id : String) :
    asked off (id, .notSwitchable) = true := by
  simp [asked]

/-- Rule 32a's reading of `caller`'s row under a switch list: the frame's
hold when the host asks the hold, no frame otherwise. -/
def rowUnder (off : List String) (members : List String) (caller : String)
    (items : List Item) : V :=
  if asked off rule32a then row members caller items else row [] caller items

/-- No switch list removes the hold. -/
theorem the_switch_never_removes_the_hold (off members : List String) (caller : String)
    (items : List Item) :
    rowUnder off members caller items = row members caller items := by
  simp [rowUnder, rule32a, asked]

/-- Off never widens: whatever a row reads under any list, it reads with
nothing off. -/
theorem the_switch_never_widens (off members : List String) (caller : String)
    (items : List Item) :
    (rowUnder off members caller items).rank = (rowUnder [] members caller items).rank := by
  rw [the_switch_never_removes_the_hold, the_switch_never_removes_the_hold]

/-! ## Why neither other off -/

/-- A refusing off would not ask the hold. -/
def rowRefusingOff (off members : List String) (caller : String) (items : List Item) : V :=
  if asked off ("32a", .refuses) then row members caller items else row [] caller items

/-- And it would widen: with `32a` listed, the logger's own row reads its
sibling's declaration and is `Safe`, where the hold reads `Unknown`. -/
theorem a_refusing_off_would_widen :
    rowRefusingOff ["32a"] logger "rd_kafka_log0"
        [.other .safe, .call "rd_kafka_log_buf" .safe] = .safe ∧
      rowRefusingOff [] logger "rd_kafka_log0"
        [.other .safe, .call "rd_kafka_log_buf" .safe] = .unknown := by
  decide

/-- An off that holds more: every call to a member is held, from any caller. -/
def Item.valueMore (members : List String) : Item → V
  | .call callee v => if callee ∈ members then frameHeld else v
  | .other v => v

def rowMore (members : List String) (items : List Item) : V :=
  fold (items.map (Item.valueMore members))

/-- It never certifies a link the hold does not: wherever it reads a link
`Safe` or `Pending`, the hold reads the same. -/
theorem holding_more_never_certifies_a_link (members : List String) (caller : String)
    (item : Item) (low : (Item.valueMore members item).rank ≤ 1) :
    Item.value members caller item = Item.valueMore members item := by
  cases item with
  | other v => rfl
  | call callee v =>
    by_cases hd : callee ∈ members
    · exfalso
      simp only [Item.valueMore, hd, ite_true, a_frame_hold_is_unknown] at low
      have two : V.unknown.rank = 2 := rfl
      omega
    · simp [Item.value, Item.valueMore, withinFrame, hd]

/-- But it changes no member's row: inside the frame it is the hold. -/
theorem holding_more_changes_no_member_row (members : List String) (caller : String)
    (member : caller ∈ members) (items : List Item) :
    rowMore members items = row members caller items := by
  unfold rowMore row
  congr 1
  apply List.map_congr_left
  intro item _
  cases item with
  | other v => rfl
  | call callee v =>
    by_cases hd : callee ∈ members
    · simp [Item.value, Item.valueMore, withinFrame, member, hd]
    · simp [Item.value, Item.valueMore, withinFrame, hd]

/-- Every row it moves is an outside caller's, which rule 32a leaves to the
declaration: the logger's outside caller reads `Safe` under the hold and
`Unknown` when the hold holds more. -/
theorem holding_more_moves_an_outside_row :
    row logger "rd_kafka_mock_cgrp_classic_member_active"
        [.other .safe, .call "rd_kafka_log0" .safe] = .safe ∧
      rowMore logger [.other .safe, .call "rd_kafka_log0" .safe] = .unknown := by
  decide

end QCore.CallerFrameHoldSwitch
