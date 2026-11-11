-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-!
# The effective-type premise of an out slot (card OUT-PARAMETER-SCALAR-FRAME-PREMISE)

The decision implemented by `crates/qcore-c-gateway/src/legacy_effective_type.rs`
(`Frame::spares`, `wide_integer_or_enum`, `strict_aliasing_in_force`) and its
use in the typed scalar store's refused-write frame (`legacy_transfer.rs`),
gateway rule 11r(7).

A store `*Y = V` may write any object `Y` points to, including the object a
pointer out slot `*X` designates, unless the two are known apart. C11 6.5p7
lets an lvalue of type `t` access an object of effective type `e` only when
the types agree or `t` is a character type (simplified to the types this
route reads; qualifiers, signedness variants and aggregates containing `e`
are not read and so never admit). The listed premise `effective-type:` is
that the program's executed stores are all defined (`hypothesis`):

* under it, a non-character integer or enumeration store leaves an object of
  pointer effective type as it was (`wide_store_keeps_pointer_object`);
* a character store may write any object (`character_store_may_write`), and
  without the premise every store may (`no_premise_may_write`);
* the gateway's decision spares a slot only for a wide store, a pointer slot
  and a build that keeps the rule (`spares_sound`); a character, floating or
  unread lvalue type and a `-fno-strict-aliasing` build refuse
  (`character_refuses`, `floating_refuses`, `opaque_refuses`,
  `no_strict_aliasing_refuses`).

## What this is not

A model of the decision and of 6.5p7's consequence for one store, not of C's
object model. That Clang's type of the lvalue and of the formal are what the
build compiles, that the analysed store is its line's only write, and that
the deployed build's flags are the compile database's are review premises
(trust boundary 28). The premise itself is stated, not proved, and every row
resting on it lists it.
-/

namespace QCore.OutParameterScalarFramePremise

/-- The lvalue and effective types the route reads. -/
inductive LType where
  | char
  | signedChar
  | unsignedChar
  | integer
  | enumeration
  | floating
  | pointer
  | opaque
  deriving DecidableEq, Repr

def character : LType → Bool
  | .char | .signedChar | .unsignedChar => true
  | _ => false

/-- A non-character integer or an enumeration. -/
def wide : LType → Bool
  | .integer | .enumeration => true
  | _ => false

/-- C11 6.5p7 over these types: the types agree, or a character lvalue. -/
def mayAccess (t e : LType) : Bool :=
  decide (t = e) || character t

structure Obj where
  eff : LType
  val : Nat
  deriving DecidableEq, Repr

/-- What a store through `t` of `v` may leave in an object it might reach.
With the premise (`hypothesis`: every executed store is defined) a store
reaches only objects its lvalue may access; without it, any. -/
def storeEffect (hypothesis : Bool) (t : LType) (o : Obj) (v : Nat) : Obj :=
  if hypothesis && !mayAccess t o.eff then o else { o with val := v }

theorem wide_store_keeps_pointer_object (t : LType) (ht : wide t = true) (o : Obj)
    (ho : o.eff = .pointer) (v : Nat) : storeEffect true t o v = o := by
  cases t <;> simp_all [wide, storeEffect, mayAccess, character]

theorem character_store_may_write (t : LType) (hc : character t = true) (o : Obj) (v : Nat) :
    storeEffect true t o v = { o with val := v } := by
  simp [storeEffect, mayAccess, hc]

theorem no_premise_may_write (t : LType) (o : Obj) (v : Nat) :
    storeEffect false t o v = { o with val := v } := by
  simp [storeEffect]

/-! ## The gateway's decision -/

/-- `Frame::spares` with `strict_aliasing_in_force`: the build keeps the rule,
the store's lvalue is wide, and the slot's object is a pointer. -/
def spares (strict : Bool) (t slot : LType) : Bool :=
  strict && wide t && decide (slot = .pointer)

theorem spares_sound (strict : Bool) (t : LType) (o : Obj) (v : Nat)
    (h : spares strict t o.eff = true) : storeEffect strict t o v = o := by
  simp only [spares, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨hs, hw⟩, hp⟩ := h
  subst hs
  exact wide_store_keeps_pointer_object t hw o hp v

theorem character_refuses (strict : Bool) (t slot : LType) (hc : character t = true) :
    spares strict t slot = false := by
  cases t <;> simp_all [character, spares, wide]

theorem floating_refuses (strict : Bool) (slot : LType) :
    spares strict .floating slot = false := by
  simp [spares, wide]

theorem opaque_refuses (strict : Bool) (slot : LType) :
    spares strict .opaque slot = false := by
  simp [spares, wide]

theorem no_strict_aliasing_refuses (t slot : LType) : spares false t slot = false := by
  simp [spares]

end QCore.OutParameterScalarFramePremise
