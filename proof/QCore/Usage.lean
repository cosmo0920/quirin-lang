namespace QCore

inductive UsageUpper where
  | finite : Nat -> UsageUpper
  | infinity : UsageUpper
  deriving DecidableEq, Repr

namespace UsageUpper

def covers : UsageUpper -> Nat -> Prop
  | finite upper, lower => lower <= upper
  | infinity, _ => True

def add : UsageUpper -> UsageUpper -> UsageUpper
  | finite a, finite b => finite (a + b)
  | _, _ => infinity

def mul : UsageUpper -> UsageUpper -> UsageUpper
  | finite a, finite b => finite (a * b)
  | finite 0, infinity => finite 0
  | infinity, finite 0 => finite 0
  | _, _ => infinity

end UsageUpper

structure Usage where
  lower : Nat
  upper : UsageUpper
  deriving DecidableEq, Repr

namespace Usage

def Valid (usage : Usage) : Prop := usage.upper.covers usage.lower

def add (a b : Usage) : Usage :=
  { lower := a.lower + b.lower, upper := a.upper.add b.upper }

def zero : Usage := { lower := 0, upper := .finite 0 }
def one : Usage := { lower := 1, upper := .finite 1 }
def affine : Usage := { lower := 0, upper := .finite 1 }
def relevant : Usage := { lower := 1, upper := .infinity }
def many : Usage := { lower := 0, upper := .infinity }

theorem zero_valid : zero.Valid := by
  exact Nat.le_refl 0

theorem one_valid : one.Valid := by
  exact Nat.le_refl 1

theorem add_valid {a b : Usage} (ha : a.Valid) (hb : b.Valid) : (add a b).Valid := by
  cases a with
  | mk al au =>
    cases b with
    | mk bl bu =>
      cases au <;> cases bu
      · exact Nat.add_le_add ha hb
      · trivial
      · trivial
      · trivial

end Usage
end QCore

