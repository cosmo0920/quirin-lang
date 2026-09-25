/-!
# Refused escapes (card W44-LEAK-PIN-AUDIT)

The card audited every one of the 271 baseline `leak` leaves and repaired two
class (b) mechanisms in the C gateway: the cast return (`QCore.CastReturnEscape`)
and the refused escape (`crates/qcore-c-gateway/src/legacy_refused_operands.rs`),
which this module models.

## The refused escape

A statement the projection refuses reports `Unsupported` and never touched a
resource status. When its **value side** — a `return` operand or the right-hand
side of an assignment — is a tracked identity (`map->parent = (void *) counter;`,
`*bufferp = buf;`, `return (const char **) arr;`), the identity has left the
projection by the statement's own syntax, yet it stayed live and was accused at
the next exit. The rule marks the identities a refused statement's value side
hands on as carrying an **unresolved effect**, the projection's existing reading
of "an unmodelled action may have discharged it".

## What is proved

* The rule only **adds** unresolved effects and changes no status and no
  exemption; it is the identity where the node was not refused.
* It only **withdraws**: a leak reported after it was reported before, and a
  site the statement did not hand on keeps its report exactly (non-vacuity of
  the precision claim), while a live, handed-on site is withdrawn.
* It composes with the union join.
* **No `Safe`**: the rule fires only where the node reported `Unsupported`, so
  a function whose every accusation it withdraws is `Unknown`.
* **Union collateral, stated rather than hidden**: the unresolved effect joins
  by union, so a live obligation reaching one exit both along a refused escape
  and along a clean path is withdrawn too (`the_union_join_withdraws_a_clean_path`);
  the card measures that collateral per leaf.

## What this is not

A proof about the Rust (trust boundary 28), or about the reader that computes
`handed` from a statement's tokens (the value side of a store or `return`,
outside call arguments, excluding a store into a record object of the function
itself); that the reader extracts what C hands on is an extraction premise. Nor
is it about what an unmodelled callee does: an identity handed to an
uncontracted **call** is deliberately not withdrawn, and the accusation there
rests on a premise about that callee
(`REFUSED-CALL-ARGUMENT-PREMISE`). Every remaining leak is a provisional
candidate, never a validated upstream defect.
-/

namespace QCore.RefusedEscapeCustody

abbrev Site := Nat

/-- The part of an abstract state a leak report reads. -/
structure State where
  live : Site → Bool
  unresolved : Site → Bool
  exempt : Site → Bool

/-- `report_leaks` at one exit. -/
def reported (s : State) (x : Site) : Bool :=
  s.live x && !s.exempt x && !s.unresolved x

/-- The refused-escape rule after one node's transfer. `refused` is whether the
    node's own transfer reported `Unsupported`; `handed x` whether its value
    side handed on a place bound to `x` before it ran. -/
def apply (refused : Bool) (handed : Site → Bool) (s : State) : State :=
  if refused then
    { s with unresolved := fun x => s.unresolved x || (handed x && s.live x) }
  else s

/-- The union join of two paths. -/
def join (a b : State) : State where
  live x := a.live x || b.live x
  unresolved x := a.unresolved x || b.unresolved x
  exempt x := a.exempt x || b.exempt x

/-! ## The rule only adds unresolved effects -/

theorem the_rule_only_grows_unresolved (r : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hx : s.unresolved x = true) : (apply r h s).unresolved x = true := by
  unfold apply; cases r <;> simp [hx]

theorem the_rule_changes_no_status (r : Bool) (h : Site → Bool) (s : State) :
    (apply r h s).live = s.live ∧ (apply r h s).exempt = s.exempt := by
  unfold apply; cases r <;> simp

theorem an_admitted_node_is_untouched (h : Site → Bool) (s : State) :
    apply false h s = s := by
  simp [apply]

/-! ## The rule only withdraws -/

theorem the_rule_only_withdraws (r : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hr : reported (apply r h s) x = true) : reported s x = true := by
  unfold apply at hr; cases r <;> simp_all [reported]

/-- Precision: a site the refused statement did not hand on keeps its report. -/
theorem an_unhanded_site_keeps_its_report (r : Bool) (h : Site → Bool) (s : State) (x : Site)
    (hx : h x = false) : reported (apply r h s) x = reported s x := by
  unfold apply; cases r <;> simp [reported, hx]

/-- The repair: a live identity a refused statement's value side handed on is
    no longer reported. -/
theorem a_refused_escape_is_withdrawn (h : Site → Bool) (s : State) (x : Site)
    (hx : h x = true) : reported (apply true h s) x = false := by
  simp [apply, reported, hx]
  cases s.live x <;> simp

/-- The rule commutes with the union join when both paths hand on the same
    places, so marking before or after a join agrees. -/
theorem the_rule_distributes_over_the_join (r : Bool) (h : Site → Bool) (a b : State) :
    apply r h (join a b) = join (apply r h a) (apply r h b) := by
  unfold apply join
  cases r
  · rfl
  · simp only [ite_true]
    congr 1
    funext x
    cases a.unresolved x <;> cases b.unresolved x <;> cases h x <;>
      cases a.live x <;> cases b.live x <;> rfl

/-! ## No `Safe` -/

/-- The three verdicts `classify_diagnostics` publishes. -/
inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

def classify (violation refusal : Bool) : Verdict :=
  if violation then .provisionalUnsafe else if refusal then .unknown else .safe

/-- The rule fires only at a node that reported `Unsupported`, so the function
    has a refusal: whatever it withdraws, the verdict is never `Safe`. -/
theorem a_withdrawal_by_the_rule_is_never_safe (violation : Bool) :
    classify violation true ≠ .safe := by
  unfold classify; cases violation <;> simp

/-- Withdrawing every accusation of a function with a refusal leaves it
    `Unknown`, the fail-closed answer. -/
theorem a_fully_withdrawn_function_is_unknown : classify false true = .unknown := by
  rfl

/-- The union join's collateral: a path where the site stays live and clean,
    joined with a path where a refused escape marked it, reports nothing. The
    card measures how many corpus leaves this costs; it never costs a `Safe`. -/
theorem the_union_join_withdraws_a_clean_path :
    ∃ clean escaped : State, ∃ x : Site,
      reported clean x = true ∧ reported (join clean escaped) x = false := by
  refine ⟨⟨fun _ => true, fun _ => false, fun _ => false⟩,
    ⟨fun _ => false, fun _ => true, fun _ => false⟩, 0, ?_, ?_⟩ <;> rfl

end QCore.RefusedEscapeCustody
