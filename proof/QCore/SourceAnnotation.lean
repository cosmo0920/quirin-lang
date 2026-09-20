namespace QCore.SourceAnnotation

/-! A reduced model of executable local annotation checking.  An omitted
annotation leaves inference unchanged.  Scalar annotations are checked for
equality.  An owned annotation is admitted only when the lowering independently
publishes the same nominal payload; custody with no nominal evidence is not
enough.  Unsupported syntax always refuses. -/

inductive Annotation where
  | omitted
  | i32
  | unit
  | owned (nominal : String)
  | unsupported
  deriving DecidableEq, Repr

inductive Actual where
  | i32
  | unit
  | owned (nominal : Option String)
  deriving DecidableEq, Repr

def check : Annotation → Actual → Bool
  | .omitted, _ => true
  | .i32, .i32 => true
  | .unit, .unit => true
  | .owned annotation, .owned (some actual) => annotation == actual
  | _, _ => false

inductive Compatible : Annotation → Actual → Prop where
  | omitted : Compatible .omitted actual
  | i32 : Compatible .i32 .i32
  | unit : Compatible .unit .unit
  | owned : Compatible (.owned nominal) (.owned (some nominal))

theorem accepted_is_compatible {annotation : Annotation} {actual : Actual}
    (accepted : check annotation actual = true) : Compatible annotation actual := by
  cases annotation with
  | omitted => exact .omitted
  | i32 =>
      cases actual <;> simp [check] at accepted
      exact .i32
  | unit =>
      cases actual <;> simp [check] at accepted
      exact .unit
  | owned annotation =>
      cases actual with
      | i32 | unit => simp [check] at accepted
      | owned actual =>
          cases actual with
          | none => simp [check] at accepted
          | some actual =>
              simp [check] at accepted
              subst actual
              exact .owned
  | unsupported => cases actual <;> simp [check] at accepted

theorem compatible_is_accepted {annotation : Annotation} {actual : Actual}
    (compatible : Compatible annotation actual) : check annotation actual = true := by
  cases compatible <;> simp [check]

theorem unsupported_refuses (actual : Actual) : check .unsupported actual = false := by
  cases actual <;> rfl

theorem opaque_owned_refuses (nominal : String) :
    check (.owned nominal) (.owned none) = false := rfl

theorem wrong_scalar_annotation_refuses : check .unit .i32 = false := rfl

example : check .i32 .i32 = true := rfl
example : check (.owned "Buffer") (.owned (some "Buffer")) = true := by decide

end QCore.SourceAnnotation
