import QCore.RetentionPath

/-! An exact declared removal wrapper forwards the caller's object and holder.
Source review and source pins establish the wrapper reading; these theorems
are about the existing retention model, not extraction from C. -/
namespace QCore.EventRemovalWrapper
open QCore.RetentionPath

def disable (object : HeldObject) (holder : HeldBy) : Event :=
  .remove (some object, some holder)

theorem disable_ends_a_fresh_exact_registration (object : HeldObject) (holder : HeldBy) :
    step [⟨object, holder, false⟩] (disable object holder) = some [] := by
  simp [disable, step, exact]

theorem disable_keeps_an_already_poisoned_registration (object : HeldObject) (holder : HeldBy) :
    step [⟨object, holder, true⟩] (disable object holder) = some [⟨object, holder, true⟩] := by
  cases object <;> simp [disable, step, exact, poisonIf, mayName, HeldObject.identity]

theorem disable_does_not_destroy_its_holder (object : HeldObject) (holder : HeldBy) :
    ends (disable object holder) holder = false := by
  simp [disable, ends]

end QCore.EventRemovalWrapper
