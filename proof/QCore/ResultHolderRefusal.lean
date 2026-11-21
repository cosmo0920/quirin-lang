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

import QCore.MemberStoreRetention

/-!
# Why a result-holder retention is not derived (card CG10-SMALL-BUNDLE, gateway rule 74a)

CG10-RESULT-HOLDER-RETAINED-BY asks whether a formal `q` stored into a member
of an object the body allocated and returns (`pmi->member = member; return
pmi;`) can be derived as §C04.14.2's `resource_retained_by(q, result)`. At a
call, rule 13e ends such a record **when the holder is released**: for a
declared contract the release (a heap handle's destructor) not releasing `q`
is a reviewed, pinned assumption (constraint 4). A derived summary carries no
review, and the result's destructor is any body: it may release the member
that keeps `q` (the holder's own release of what it keeps).

Over `QCore.MemberStoreRetention`'s concrete memory (a holder keeps a
pointer; a double release faults), this module proves:

* **the refusal's reason**: a checker that reads the result's destructor as
  the end of the record (a reset of the holder) accepts a caller trace whose
  concrete run, with a destructor that releases the kept member, faults by a
  double release of the caller's object;
* the same trace is refused where the destructor is read as what it does
  (`releaseHeld` while the caller still owns `q`: rule 70's checker refuses
  the caller's own release of `q` after it), so the end of the record needs a
  premise on the destructor's member effect that no derived summary states;
* with a destructor that releases only the holder (no member effect), the
  end-of-record reading is the concrete run: the trace does not fault. This is
  the premise a future reading would have to establish per destructor.

## What this is not

A model of the gateway: the gateway keeps rule 10v's refusal of a store into
a tracked owner (`tests/small_bundle.rs` pins it); this module states why
that refusal stands.
-/

namespace QCore.ResultHolderRefusal

open QCore.MemberStoreRetention
open QCore.RetentionPath (HeldBy)

/-- The result's member that keeps the caller's object. -/
def resultMember : HeldBy := .site 0

/-- The caller's object, retained by the result. -/
def callerObject : Nat := 1

/-- The caller's trace as the end-of-record reading abstracts it: the call
stores `q` into the result, the destructor ends the record (a reset), then
the caller releases its own object. -/
def abstracted : List Act :=
  [.store (some callerObject) resultMember, .reset resultMember, .release callerObject]

/-- The same trace with a destructor that releases the member it keeps. -/
def withMemberRelease : List Act :=
  [.store (some callerObject) resultMember, .releaseHeld resultMember, .release callerObject]

/-- The same trace with a destructor that releases only the holder. -/
def withoutMemberRelease : List Act := abstracted

/-- The end-of-record reading accepts the caller. -/
theorem the_end_of_record_reading_accepts_the_caller : checkAll Chk.init abstracted ≠ none := by
  simp [abstracted, checkAll, check, Chk.init, Chk.drop, resultMember, callerObject]

/-- **The refusal's reason**: with a destructor that releases the kept
member, the accepted caller faults (a double release of its object). -/
theorem a_member_releasing_destructor_faults : execAll Mem.init withMemberRelease = none := by
  simp [withMemberRelease, execAll, exec, Mem.init, Mem.put, resultMember, callerObject]

/-- Read as what it does, the destructor's release of the kept member makes
the caller's own release a refused double release. -/
theorem the_destructor_read_as_it_is_refuses_the_caller :
    checkAll Chk.init withMemberRelease = none := by
  simp [withMemberRelease, checkAll, check, Chk.init, Chk.drop, resultMember, callerObject,
    List.find?]

/-- With a destructor that has no member effect, the end-of-record reading
is the concrete run: no fault. -/
theorem a_destructor_without_member_effect_does_not_fault :
    execAll Mem.init withoutMemberRelease ≠ none := by
  simp [withoutMemberRelease, abstracted, execAll, exec, Mem.init, Mem.put, resultMember,
    callerObject]

/-- So the end-of-record reading is sound for a caller only under a premise
on the destructor's member effect: the two destructors are indistinguishable
to the abstraction and differ in the concrete run. -/
theorem the_end_of_record_needs_a_destructor_premise :
    checkAll Chk.init abstracted ≠ none ∧ execAll Mem.init withMemberRelease = none ∧
      execAll Mem.init withoutMemberRelease ≠ none :=
  ⟨the_end_of_record_reading_accepts_the_caller, a_member_releasing_destructor_faults,
    a_destructor_without_member_effect_does_not_fault⟩

end QCore.ResultHolderRefusal
