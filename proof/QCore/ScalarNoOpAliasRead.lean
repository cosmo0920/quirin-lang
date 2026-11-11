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

import QCore.IndirectScalarAccess
import QCore.MayAlias

/-!
The older scalar-field lexical route has a read obligation even when the
member's value carries no resource custody. A syntactically admitted `B->f`
is lowered to the existing indirect scalar transfer with `B` in its base list.
The typed C reader and alias-peer extraction remain Rust trust premises; this
module models the flow decision after those premises have supplied the bases.
-/

namespace QCore.ScalarNoOpAliasRead

open QCore.IndirectScalarAccess

def lower (bases : List Nat) (destination : Option Nat) : Access :=
  { bases, destination, frame := none }

/-- The Rust read checks both the class-peer obligation and the existing
indirect-base obligation; neither can be erased by lexical admission. -/
def checkedRead (classes : MayAlias.Classes) (aliases : MayAlias.State)
    (custody : State) (base : Nat) : Bool :=
  !MayAlias.refuses classes aliases base && resolves custody base

theorem a_released_undischarged_peer_refuses_the_scalar_read
    (classes : MayAlias.Classes) (aliases : MayAlias.State) (custody : State)
    (base peer : Nat) (hrelease : peer ∈ aliases.released)
    (hpeer : MayAlias.peer classes base peer = true)
    (hguard : MayAlias.guardDischarges aliases base peer = false)
    (hprov : MayAlias.dischargedByProvenance aliases base = false) :
    checkedRead classes aliases custody base = false := by
  have hrefuse := MayAlias.a_released_peer_refuses_the_read
    hrelease hpeer hguard hprov
  simp [checkedRead, hrefuse]

theorem resource_projection_unchanged (s : State) (bases : List Nat)
    (destination : Option Nat) :
    (transfer s (lower bases destination)).1.status = s.status ∧
    (transfer s (lower bases destination)).1.rest = s.rest := by
  exact ⟨rfl, rfl⟩

theorem unrelated_binding_unchanged (s : State) (bases : List Nat)
    (destination : Option Nat) (name : Nat) (h : destination ≠ some name) :
    (transfer s (lower bases destination)).1.binding name = s.binding name := by
  exact without_a_frame_only_the_destination_changes s (lower bases destination) name rfl h

theorem a_refused_single_read_is_not_safe (s : State) (base : Nat)
    (destination : Option Nat) (h : resolves s base = false) :
    safe (transfer s (lower [base] destination)).2 = false := by
  simp [safe, transfer, refusals, lower, h]

theorem a_resolved_single_read_adds_no_diagnostic (s : State) (base : Nat)
    (destination : Option Nat) (h : resolves s base = true) :
    (transfer s (lower [base] destination)).2 = [] := by
  simp [transfer, refusals, lower, h]

end QCore.ScalarNoOpAliasRead
