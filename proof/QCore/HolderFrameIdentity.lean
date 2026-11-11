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

import QCore.OrderedLifetimeReplay
/-! Exact caller-root identity and source-checked holder preservation.
The extraction of the write/escape census and C layout remain premises.
Borrowed custody supplies neither a frame nor an equality. -/
namespace QCore.HolderFrameIdentity
structure Key where
  source : Nat
  formal : Nat
  record : Nat
  member : Nat
  deriving DecidableEq, Repr

abbrev Heap := Key → Nat

def store (heap : Heap) (write : Key × Nat) : Heap :=
  fun key => if key = write.1 then write.2 else heap key

def run (heap : Heap) (writes : List (Key × Nat)) : Heap := writes.foldl store heap

def checkedFrame (key : Key) (writes : List (Key × Nat)) (escaped unread : Bool) : Prop :=
  escaped = false ∧ unread = false ∧ ∀ write ∈ writes, write.1 ≠ key

theorem a_closed_frame_preserves_the_exact_holder
    (heap : Heap) (key : Key) (writes : List (Key × Nat))
    (closed : ∀ write ∈ writes, write.1 ≠ key) :
    run heap writes key = heap key := by
  induction writes generalizing heap with
  | nil => rfl
  | cons first rest ih =>
    have firstOther := closed first (by simp)
    have restOther : ∀ write ∈ rest, write.1 ≠ key := by
      intro write member
      exact closed write (by simp [member])
    simpa [run, List.foldl, store, Ne.symm firstOther] using ih (store heap first) restOther

theorem an_admitted_frame_preserves_the_holder
    (heap : Heap) (key : Key) (writes : List (Key × Nat)) (escaped unread : Bool)
    (frame : checkedFrame key writes escaped unread) :
    run heap writes key = heap key :=
  a_closed_frame_preserves_the_exact_holder heap key writes frame.2.2

theorem an_escaped_record_has_no_frame (key : Key) (writes : List (Key × Nat)) (unread : Bool) :
    ¬ checkedFrame key writes true unread := by simp [checkedFrame]

theorem an_unread_call_has_no_frame (key : Key) (writes : List (Key × Nat)) (escaped : Bool) :
    ¬ checkedFrame key writes escaped true := by simp [checkedFrame]

theorem a_holder_write_refuses_the_frame (key : Key) (value : Nat) (escaped unread : Bool) :
    ¬ checkedFrame key [(key, value)] escaped unread := by simp [checkedFrame]

theorem a_write_to_another_field_preserves_the_holder
    (heap : Heap) (key other : Key) (value : Nat) (distinct : other ≠ key) :
    run heap [(other, value)] key = heap key := by
  simp [run, store, Ne.symm distinct]

def frameMatch (formal expectedFormal record expectedRecord member expectedMember : Nat) : Bool :=
  formal == expectedFormal && record == expectedRecord && member == expectedMember

theorem an_admitted_frame_keeps_the_formal_and_record_and_member
    (a b c d e f : Nat) (admitted : frameMatch a b c d e f = true) :
    a = b ∧ c = d ∧ e = f := by simp_all [frameMatch]

theorem another_collector_never_matches (a b c e : Nat) (distinct : a ≠ b) :
    frameMatch a b c c e e = false := by simp [frameMatch, distinct]

theorem another_record_never_matches (a c d e : Nat) (distinct : c ≠ d) :
    frameMatch a a c d e e = false := by simp [frameMatch, distinct]

theorem another_loop_member_never_matches (a c e f : Nat) (distinct : e ≠ f) :
    frameMatch a a c c e f = false := by simp [frameMatch, distinct]

def aliasAdmission (initialized singleWrite unaddressed typed straightLine pointerRoot valueInitializer : Bool) : Bool :=
  initialized && singleWrite && unaddressed && typed && straightLine && pointerRoot && valueInitializer

theorem a_rebound_local_supplies_no_identity (a c d e p v : Bool) :
    aliasAdmission a false c d e p v = false := by simp [aliasAdmission]

theorem an_addressed_local_supplies_no_identity (a b d e p v : Bool) :
    aliasAdmission a b false d e p v = false := by simp [aliasAdmission]

theorem a_retyped_local_supplies_no_identity (a b c e p v : Bool) :
    aliasAdmission a b c false e p v = false := by simp [aliasAdmission]

theorem a_conditional_initialization_supplies_no_identity (a b c d p v : Bool) :
    aliasAdmission a b c d false p v = false := by simp [aliasAdmission]

theorem a_by_value_formal_supplies_no_pointer_identity (a b c d e v : Bool) :
    aliasAdmission a b c d e false v = false := by simp [aliasAdmission]

theorem an_address_initializer_supplies_no_value_identity (a b c d e p : Bool) :
    aliasAdmission a b c d e p false = false := by simp [aliasAdmission]

end QCore.HolderFrameIdentity
