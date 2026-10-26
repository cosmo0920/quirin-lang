import QCore.QuantEffect

/-! Conditional GCC/Clang CFL own-event effects. Backend selection, ABI,
linearization/event completeness, unsigned-value interpretation and C builtin
correspondence are independent premises. No mutex/MSVC or synchronization proof. -/
namespace QCore.CflAtomicEffects
open QCore.QuantEffect

abbrev Memory := Nat → Int

def write (before : Memory) (target : Nat) (value : Int) : Memory :=
  fun object => if object = target then value else before object

def cas (before : Memory) (target : Nat) (expected replacement : Int) : Int × Memory :=
  if before target = expected then (1, write before target replacement) else (0, before)

theorem store_target (before : Memory) (target : Nat) (value : Int) :
    write before target value target = value := by simp [write]

theorem store_frame (before : Memory) (target : Nat) (value : Int) :
    PreservesOutside (fun object => object = target) before (write before target value) := by
  intro object outside
  simp [write, outside]

theorem cas_success (before : Memory) (target : Nat) (expected replacement : Int)
    (equalValue : before target = expected) :
    (cas before target expected replacement).1 = 1 ∧
    (cas before target expected replacement).2 target = replacement ∧
    PreservesOutside (fun object => object = target) before
      (cas before target expected replacement).2 := by
  simp only [cas, ite_eq_left equalValue]
  exact ⟨True.intro, store_target before target replacement, store_frame before target replacement⟩

theorem cas_failure (before : Memory) (target : Nat) (expected replacement : Int)
    (differs : before target ≠ expected) :
    cas before target expected replacement = (0, before) := by simp [cas, differs]

theorem strong_cas_no_spurious_failure (before : Memory) (target : Nat)
    (expected replacement : Int) :
    (cas before target expected replacement).1 = 0 ↔ before target ≠ expected := by
  by_cases equalValue : before target = expected <;> simp [cas, equalValue]

/-- An expected argument is a value. A separate caller slot is framed only if
it is outside the storage footprint; alias separation must be established. -/
theorem expected_slot_preserved (before : Memory) (target expectedSlot : Nat)
    (expected replacement : Int) (separate : expectedSlot ≠ target) :
    (cas before target expected replacement).2 expectedSlot = before expectedSlot := by
  by_cases equalValue : before target = expected <;> simp [cas, write, equalValue, separate]

theorem own_read_frame (before : Memory) :
    PreservesOutside (fun _ => False) before before := by intro _ _; rfl

end QCore.CflAtomicEffects
