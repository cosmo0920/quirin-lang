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

namespace QCore.ListUnlink

/-! Link-cell semantics under live reciprocal-neighbor and exclusive-access
premises. Neighbor nodes may alias each other; entry is distinct from them.
No allocation is freed by these cell writes. C import is a trusted boundary. -/
inductive Link where
  | prev
  | next
  deriving DecidableEq
abbrev Memory := Nat → Link → Option Nat

def write (memory : Memory) (node : Nat) (field : Link) (value : Option Nat) : Memory :=
  fun other link => if other = node ∧ link = field then value else memory other link

def unlink (memory : Memory) (entry previous next : Nat) : Memory :=
  let memory := write memory previous .next (some next)
  let memory := write memory next .prev (some previous)
  let memory := write memory entry .prev none
  write memory entry .next none

theorem unlink_clears_entry_prev (memory : Memory) (entry previous next : Nat) :
    unlink memory entry previous next entry .prev = none := by
  simp [unlink, write]

theorem unlink_clears_entry_next (memory : Memory) (entry previous next : Nat) :
    unlink memory entry previous next entry .next = none := by
  simp [unlink, write]

theorem unlink_reconnects_previous (memory : Memory) (entry previous next : Nat)
    (distinct : previous ≠ entry) :
    unlink memory entry previous next previous .next = some next := by
  simp [unlink, write, distinct]

theorem unlink_reconnects_next (memory : Memory) (entry previous next : Nat)
    (distinct : next ≠ entry) :
    unlink memory entry previous next next .prev = some previous := by
  simp [unlink, write, distinct]

theorem singleton_unlink_restores_head (memory : Memory) (entry head : Nat)
    (distinct : head ≠ entry) :
    unlink memory entry head head head .prev = some head ∧
    unlink memory entry head head head .next = some head := by
  simp [unlink, write, distinct]

theorem unlink_preserves_other_cells (memory : Memory) (entry previous next other : Nat)
    (field : Link) (outsideEntry : other ≠ entry) (outsidePrevious : other ≠ previous)
    (outsideNext : other ≠ next) :
    unlink memory entry previous next other field = memory other field := by
  simp [unlink, write, outsideEntry, outsidePrevious, outsideNext]

end QCore.ListUnlink
