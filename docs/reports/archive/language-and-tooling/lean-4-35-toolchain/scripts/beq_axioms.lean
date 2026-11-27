/-! LEAN-4-35-TOOLCHAIN: why 88 audited declarations gain `Classical.choice` under 4.35.
Under 4.35 `BEq` on `Int`, `UIntN`, `IntN` and `Fin` (and pairs, options and lists of
them) is synthesized through `Std.PreorderPackage.toBEq` from the type's linear order
package, whose closure includes `Classical.choice`; `==` on `Nat`, `String`, `Char` and
`Bool`, and `decide`, `<`, `max` and `compare` on `Int`, are unchanged. -/

set_option pp.explicit true in
#synth BEq Int

def bInt (a b : Int) : Bool := a == b
def bNat (a b : Nat) : Bool := a == b
def bString (a b : String) : Bool := a == b
def bChar (a b : Char) : Bool := a == b
def bUInt8 (a b : UInt8) : Bool := a == b
def bUInt64 (a b : UInt64) : Bool := a == b
def bInt8 (a b : Int8) : Bool := a == b
def bInt64 (a b : Int64) : Bool := a == b
def bBool (a b : Bool) : Bool := a == b
def bOptInt (a b : Option Int) : Bool := a == b
def bListInt (a b : List Int) : Bool := a == b
def bListNat (a b : List Nat) : Bool := a == b
def bFin (a b : Fin 5) : Bool := a == b
def bBitVec (a b : BitVec 8) : Bool := a == b
def bPairStrInt (a b : String × Int) : Bool := a == b
def ltInt (a b : Int) : Bool := a < b
def decInt (a b : Int) : Bool := decide (a = b)
def maxInt (a b : Int) : Int := max a b
def cmpInt (a b : Int) : Ordering := compare a b
def neInt (a b : Int) : Bool := a != b
#print axioms bInt
#print axioms bNat
#print axioms bString
#print axioms bChar
#print axioms bUInt8
#print axioms bUInt64
#print axioms bInt8
#print axioms bInt64
#print axioms bBool
#print axioms bOptInt
#print axioms bListInt
#print axioms bListNat
#print axioms bFin
#print axioms bBitVec
#print axioms bPairStrInt
#print axioms ltInt
#print axioms decInt
#print axioms maxInt
#print axioms cmpInt
#print axioms neInt
