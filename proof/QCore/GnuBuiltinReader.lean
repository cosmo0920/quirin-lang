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

import QCore.TypedefProjection

/-!
# `__builtin_va_list` in the declared-ABI reader

Card **W44-LIBC-RESIDUALS** (`LIBC-PRINTF-VA-LIST`). `QCore.DeclaredAbiAgreement`
models the comparison of a declared contract's ABI spellings against the
analysed unit's C declaration, `QCore.TypedefProjection` how a typedef name is
followed, and `QCore.PointerQualifierReader` how a parameter declarator's words
are read. This module models the one step this card changed in
`crates/qcore-c-gateway/src/legacy_declared_abi.rs`.

glibc's `vsnprintf`, `vprintf`, `vfprintf` and `vsprintf` end in
`__gnuc_va_list __arg`, with `typedef __builtin_va_list __gnuc_va_list;`. The
builtin itself is the compiler's and is never a typedef in the token stream,
so resolution stopped there and every such declaration was withdrawn as
unreadable. It now projects to the loader's `va_list` spelling, at a
**parameter** of pointer depth zero only. The spelling names the type rather
than its target-dependent representation, so no target is assumed.

(An earlier revision of this card also discarded GNU `__extension__` among a
declaration's specifiers, for a declaration of glibc's header-inline
`__bswap_64`. That declaration was dropped in favour of W4's validated
summary of the header body, nothing else needed the widening, and it was
reverted together with its model.)

## What is proved

* **The widening only adds**: every reading the earlier reader committed to is
  unchanged, and the widening is strict, with the measured witness.
* **`va_list` is its own projection, in both directions**: at a parameter the
  builtin admits `va_list` and nothing else — not the x86-64 object pointer it
  decays to, not an integer — and no earlier reading admits `va_list`.
* **Where it still refuses**: a result, and any pointer to the builtin.
* **Resolution is conservative**: glibc's chain `va_list → __gnuc_va_list →
  __builtin_va_list` reaches the builtin; a unit that `typedef`s the builtin
  name is read through its own typedef; and whenever the chain does not reach
  the builtin, the reading is exactly the earlier one.
* **No custody is writable on it**: it is not a `ptr` spelling, so `Borrowed`
  is the only mode the loader's gate admits, and it is not in the result
  vocabulary at all.
* **The comparison**: the measured `vsnprintf` signature agrees once the
  builtin is read, and was withdrawn as unreadable before.

## What this is not

A model of a **decision**, as its neighbours are. That the words are the ones
the token stream carries is trust boundary 28's extraction premise; that
`__builtin_va_list` is the type `va_list` names is a GNU C fact supplied from
outside. Nothing here says a declaration is faithful to a C body.
-/

namespace QCore.GnuBuiltinReader

open QCore.DeclaredAbiAgreement
open QCore.TypedefProjection

/-! ## `__builtin_va_list` -/

/-- What a type name resolves to now: a reading the earlier reader commits to,
or the compiler's variable-argument list type. -/
inductive Target where
  | earlier (t : TypedefProjection.Target)
  | builtinVaList
  deriving DecidableEq, Repr

/-- The projection of one resolved type under `outer` further pointer levels,
at a result (`result = true`) or a parameter. -/
def project (t : Target) (outer : Nat) (result : Bool) (constPointee : Bool) : Projection :=
  match t with
  | .earlier e => TypedefProjection.project e outer constPointee
  | .builtinVaList => if outer = 0 ∧ result = false then some [.vaList] else none

/-- The reader before this card: the builtin was never resolved. -/
def projectBase (t : Target) (outer : Nat) (_result : Bool) (constPointee : Bool) : Projection :=
  match t with
  | .earlier e => TypedefProjection.project e outer constPointee
  | .builtinVaList => none

theorem the_va_list_widening_only_adds (t : Target) (outer : Nat) (result constPointee : Bool)
    {x : List AbiType} (h : projectBase t outer result constPointee = some x) :
    project t outer result constPointee = some x := by
  cases t with
  | earlier e => exact h
  | builtinVaList => simp [projectBase] at h

theorem the_va_list_widening_is_strict :
    projectBase .builtinVaList 0 false false = none
      ∧ project .builtinVaList 0 false false = some [.vaList] := by decide

/-- At a parameter the builtin admits `va_list` and nothing else. -/
theorem only_va_list_is_admitted_for_the_builtin (t : AbiType) (constPointee : Bool) :
    admits (project .builtinVaList 0 false constPointee) t = true ↔ t = .vaList := by
  cases constPointee <;> cases t <;> decide

/-- In particular not the object pointer it decays to on x86-64, and not an
integer of the same width: no target representation is assumed. -/
theorem the_builtin_is_not_its_x86_64_representation :
    admits (project .builtinVaList 0 false false) .ptrMut = false
      ∧ admits (project .builtinVaList 0 false false) .u64 = false
      ∧ admits (project .builtinVaList 0 false false) .usize = false := by decide

theorem a_va_list_result_refuses (outer : Nat) (constPointee : Bool) :
    project .builtinVaList outer true constPointee = none := by
  simp [project]

theorem a_pointer_to_a_va_list_refuses (outer : Nat) (result constPointee : Bool) :
    project .builtinVaList (outer + 1) result constPointee = none := by
  simp [project]

/-- No pointer projection is `va_list`. -/
theorem a_pointer_projection_never_admits_va_list (k : Nat) (constPointee : Bool) :
    admits (pointerProjection k constPointee) .vaList = false := by
  unfold pointerProjection
  split <;> decide

/-- And no earlier reading admits it, so the spelling is refused for every C
type that is not the builtin. -/
theorem no_earlier_reading_admits_va_list (e : TypedefProjection.Target) (outer : Nat)
    (constPointee : Bool)
    (hs : ∀ s, e = .scalar s → (s.any (· == AbiType.vaList)) = false) :
    admits (TypedefProjection.project e outer constPointee) .vaList = false := by
  cases e with
  | object k => exact a_pointer_projection_never_admits_va_list (k + outer) constPointee
  | aggregate k => exact a_pointer_projection_never_admits_va_list (k + outer) constPointee
  | functionPointer =>
    show admits (if outer = 0 then some [AbiType.fnptr] else none) AbiType.vaList = false
    by_cases ho : outer = 0
    · rw [ite_eq_left ho]; decide
    · rw [ite_eq_right ho]; rfl
  | unresolved => rfl
  | scalar s =>
    have h := hs s rfl
    show admits (if outer = 0 then some s else pointerProjection outer constPointee)
      AbiType.vaList = false
    by_cases ho : outer = 0
    · rw [ite_eq_left ho]; simpa [admits] using h
    · rw [ite_eq_right ho]
      exact a_pointer_projection_never_admits_va_list outer constPointee

/-! ### Resolution -/

/-- Follow one name as the Rust does: the builtin's name resolves to the builtin
only when the unit's own table has no entry for it; every other shape is the
earlier `TypedefProjection.resolve`. `builtin` is the index of the name
`__builtin_va_list`. -/
def resolveVa (table : Table) (builtin : Nat) : Nat → Nat → Target
  | _, 0 => .earlier .unresolved
  | n, fuel + 1 =>
      match table n with
      | .absent => if n = builtin then .builtinVaList else .earlier .unresolved
      | .direct t => .earlier t
      | .alias m k =>
          match resolveVa table builtin m fuel with
          | .builtinVaList => if k = 0 then .builtinVaList else .earlier .unresolved
          | .earlier _ => .earlier (resolve table n (fuel + 1))

/-- A chain that reaches the builtin was unresolved for the earlier reader: the
builtin's name has no entry in the unit's table. -/
theorem a_chain_to_the_builtin_was_unresolved (table : Table) (builtin : Nat) :
    ∀ (fuel n : Nat), resolveVa table builtin n fuel = .builtinVaList →
      resolve table n fuel = .unresolved := by
  intro fuel
  induction fuel with
  | zero => intro n h; simp [resolveVa] at h
  | succ fuel ih =>
    intro n h
    cases hn : table n with
    | absent => simp [resolve, hn]
    | direct t => simp [resolveVa, hn] at h
    | alias m k =>
      cases hm : resolveVa table builtin m fuel with
      | builtinVaList =>
        have hr := ih m hm
        by_cases hk : k = 0 <;> simp [resolve, hn, hr]
      | earlier y => simp [resolveVa, hn, hm] at h

/-- Whenever the chain does not reach the builtin, the reading is exactly the
earlier one. -/
theorem resolution_is_conservative (table : Table) (builtin : Nat) :
    ∀ (fuel n : Nat) (x : TypedefProjection.Target),
      resolveVa table builtin n fuel = .earlier x → x = resolve table n fuel := by
  intro fuel
  cases fuel with
  | zero => intro n x h; simp [resolveVa] at h; subst h; rfl
  | succ fuel =>
    intro n x h
    cases hn : table n with
    | absent =>
      by_cases hb : n = builtin
      · subst hb; simp [resolveVa, hn] at h
      · simp [resolveVa, hn, hb] at h; subst h; simp [resolve, hn]
    | direct t => simp [resolveVa, hn] at h; subst h; simp [resolve, hn]
    | alias m k =>
      cases hm : resolveVa table builtin m fuel with
      | builtinVaList =>
        have hr := a_chain_to_the_builtin_was_unresolved table builtin fuel m hm
        by_cases hk : k = 0
        · simp [resolveVa, hn, hm, hk] at h
        · simp [resolveVa, hn, hm, hk] at h; subst h; simp [resolve, hn, hr]
      | earlier y => simp [resolveVa, hn, hm] at h; subst h; rfl


/-- A unit that `typedef`s the builtin name itself is read through its own
typedef: the builtin reading cannot shadow a definition the stream carries. -/
theorem a_unit_typedef_of_the_builtin_name_is_followed (table : Table) (builtin fuel : Nat)
    (t : TypedefProjection.Target) (h : table builtin = .direct t) :
    resolveVa table builtin builtin (fuel + 1) = .earlier t := by
  simp [resolveVa, h]

/-- glibc's chain, measured: `va_list` (0) aliases `__gnuc_va_list` (1), which
aliases `__builtin_va_list` (2), which the unit does not define. -/
def glibcTable : Table
  | 0 => .alias 1 0
  | 1 => .alias 2 0
  | _ => .absent

theorem the_glibc_chain_reaches_the_builtin :
    resolveVa glibcTable 2 0 8 = .builtinVaList
      ∧ resolveVa glibcTable 2 1 8 = .builtinVaList
      ∧ resolve glibcTable 1 8 = .unresolved := by
  decide

/-- A pointer typedef of the builtin (`typedef __builtin_va_list *p;`) stays
unresolved, as it was. -/
def pointerTable : Table
  | 0 => .alias 2 1
  | _ => .absent

theorem a_typedef_pointer_to_the_builtin_stays_unresolved :
    resolveVa pointerTable 2 0 8 = .earlier .unresolved := by
  decide

/-! ### No custody is writable on it -/

theorem a_va_list_is_not_a_pointer_spelling : isPointerSpelling .vaList = false := by decide

theorem borrowed_is_the_only_mode_on_a_va_list (m : Mode) :
    modeAdmissible m .vaList = true ↔ m = .borrowed := by
  cases m <;> decide

theorem an_owned_result_of_a_va_list_is_rejected : ownedResultAdmissible .vaList = false := by
  decide

/-- The loader's result vocabulary: every spelling but `va_list`, which is a
parameter type only. -/
def resultSpelling : AbiType → Bool
  | .vaList => false
  | _ => true

/-- The parameter vocabulary: every spelling but `unit`. -/
def parameterSpelling : AbiType → Bool
  | .unit => false
  | _ => true

theorem va_list_is_a_parameter_spelling_only :
    parameterSpelling .vaList = true ∧ resultSpelling .vaList = false := by decide

theorem the_result_vocabulary_lost_only_va_list (t : AbiType) :
    resultSpelling t = false ↔ t = .vaList := by
  cases t <;> decide

/-! ### The comparison, with the measured `vsnprintf` -/

/-- `int vsnprintf (char *__restrict __s, size_t __maxlen,
const char *__restrict __format, __gnuc_va_list __arg)`, as declared. -/
def vsnprintfSignature : Signature :=
  { parameters := [.ptrMut, .usize, .ptrConst, .vaList], result := .i32, tail := false }

/-- The C side with the builtin read, and as the earlier reader saw it. -/
def vsnprintfNow : CDeclaration :=
  { result := some [.i32],
    parameters := [some [.ptrMut], some [.u64, .usize], some [.ptrConst],
      project .builtinVaList 0 false false],
    variadic := false, unspecified := false }

def vsnprintfBefore : CDeclaration :=
  { vsnprintfNow with
    parameters := [some [.ptrMut], some [.u64, .usize], some [.ptrConst],
      projectBase .builtinVaList 0 false false] }

theorem vsnprintf_agrees_now_and_was_withdrawn_as_unreadable :
    compare vsnprintfSignature vsnprintfNow = none
      ∧ compare vsnprintfSignature vsnprintfBefore = some .unreadableDeclaration := by
  decide

/-- The object-pointer spelling a target-bound reading would have used is a
disagreement, not an agreement. -/
theorem a_pointer_spelling_of_the_va_list_disagrees :
    compare { vsnprintfSignature with parameters := [.ptrMut, .usize, .ptrConst, .ptrMut] }
      vsnprintfNow = some .parameterType := by
  decide

end QCore.GnuBuiltinReader
