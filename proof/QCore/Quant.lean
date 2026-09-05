namespace QCore

abbrev Axis := Nat
abbrev QuantVector := Axis -> Nat

namespace QuantVector

def zero : QuantVector := fun _ => 0
def add (a b : QuantVector) : QuantVector := fun axis => a axis + b axis
def join (a b : QuantVector) : QuantVector := fun axis => max (a axis) (b axis)
def scale (factor : Nat) (q : QuantVector) : QuantVector := fun axis => factor * q axis
def Le (a b : QuantVector) : Prop := forall axis, a axis <= b axis

theorem zero_le (q : QuantVector) : Le zero q := by
  intro axis
  exact Nat.zero_le (q axis)

theorem left_le_join (a b : QuantVector) : Le a (join a b) := by
  intro axis
  exact Nat.le_max_left (a axis) (b axis)

theorem right_le_join (a b : QuantVector) : Le b (join a b) := by
  intro axis
  exact Nat.le_max_right (a axis) (b axis)

end QuantVector
end QCore

