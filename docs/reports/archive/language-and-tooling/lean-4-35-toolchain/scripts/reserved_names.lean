/-! LEAN-4-35-TOOLCHAIN: names 4.35 no longer accepts. Under 4.34 this file elaborates;
under 4.35 `given` in a `def` binder is a forbidden token (the def contract clauses,
lean4#14825), and `recall` and `monotonicity_by` are keywords (lean4#14899,
lean4#14861). The `theorem` binder below is accepted by both: `declSig` forbids nothing. -/

def ownInForce (env : Nat) (f : Nat) (given : Option Nat) : Nat := env + f + given.getD 0

def usesRecall (recall : Nat) : Nat := recall

def usesMono (monotonicity_by : Nat) : Nat := monotonicity_by

theorem theoremBinder (given : 1 = 1) : 1 = 1 := given
