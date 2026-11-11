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

import QCore.Quant

namespace QCore

abbrev SignedVector := Axis -> Int

namespace SignedVector

def zero : SignedVector := fun _ => 0
def add (a b : SignedVector) : SignedVector := fun axis => a axis + b axis
def ofQuant (q : QuantVector) : SignedVector := fun axis => Int.ofNat (q axis)
def negateQuant (q : QuantVector) : SignedVector := fun axis => -Int.ofNat (q axis)

/-- Convert an internal signed offset to its nonnegative contribution to peak. -/
def nonnegative (q : SignedVector) : QuantVector := fun axis => Int.toNat (q axis)

end SignedVector

/-- Static resource behavior relative to entry of a path. -/
structure Flow where
  net : SignedVector
  peak : QuantVector
  total : QuantVector

namespace Flow

def zero : Flow where
  net := SignedVector.zero
  peak := QuantVector.zero
  total := QuantVector.zero

def acquire (footprint cost : QuantVector) : Flow where
  net := SignedVector.ofQuant footprint
  peak := footprint
  total := cost

def release (footprint cost : QuantVector) : Flow where
  net := SignedVector.negateQuant footprint
  peak := QuantVector.zero
  total := cost

/-- Sequential composition `(d₁+d₂, max(p₁, d₁+p₂), t₁+t₂)`. -/
def seq (first second : Flow) : Flow where
  net := SignedVector.add first.net second.net
  peak := QuantVector.join first.peak
    (SignedVector.nonnegative (SignedVector.add first.net
      (SignedVector.ofQuant second.peak)))
  total := QuantVector.add first.total second.total

/-- Join of mutually exclusive branches after establishing equal post-state
    net changes. -/
def branchJoin (left right : Flow) (_agree : left.net = right.net) : Flow where
  net := left.net
  peak := QuantVector.join left.peak right.peak
  total := QuantVector.join left.total right.total

abbrev FunctionSummary := Flow

def call (summary : FunctionSummary) : Flow := summary

/-- The accepted summary of at most `bound` iterations of a zero-net body. -/
def boundedZeroNetLoop (body : Flow) (bound : Nat)
    (_zeroNet : body.net = SignedVector.zero) : Flow where
  net := SignedVector.zero
  peak := body.peak
  total := QuantVector.scale bound body.total

theorem acquire_net (footprint cost : QuantVector) :
    (acquire footprint cost).net = SignedVector.ofQuant footprint := by
  rfl

theorem acquire_peak (footprint cost : QuantVector) :
    (acquire footprint cost).peak = footprint := by
  rfl

theorem release_net (footprint cost : QuantVector) :
    (release footprint cost).net = SignedVector.negateQuant footprint := by
  rfl

theorem release_has_zero_peak (footprint cost : QuantVector) :
    (release footprint cost).peak = QuantVector.zero := by
  rfl

theorem seq_net (first second : Flow) :
    (seq first second).net = SignedVector.add first.net second.net := by
  rfl

theorem seq_total (first second : Flow) :
    (seq first second).total = QuantVector.add first.total second.total := by
  rfl

theorem seq_peak_covers_first (first second : Flow) :
    QuantVector.Le first.peak (seq first second).peak := by
  exact QuantVector.left_le_join first.peak
    (SignedVector.nonnegative (SignedVector.add first.net
      (SignedVector.ofQuant second.peak)))

theorem seq_peak_covers_shifted_second (first second : Flow) :
    QuantVector.Le
      (SignedVector.nonnegative (SignedVector.add first.net
        (SignedVector.ofQuant second.peak)))
      (seq first second).peak := by
  exact QuantVector.right_le_join first.peak
    (SignedVector.nonnegative (SignedVector.add first.net
      (SignedVector.ofQuant second.peak)))

theorem branch_peak_covers_left (left right : Flow) (agree : left.net = right.net) :
    QuantVector.Le left.peak (branchJoin left right agree).peak := by
  exact QuantVector.left_le_join left.peak right.peak

theorem branch_peak_covers_right (left right : Flow) (agree : left.net = right.net) :
    QuantVector.Le right.peak (branchJoin left right agree).peak := by
  exact QuantVector.right_le_join left.peak right.peak

theorem branch_total_covers_left (left right : Flow) (agree : left.net = right.net) :
    QuantVector.Le left.total (branchJoin left right agree).total := by
  exact QuantVector.left_le_join left.total right.total

theorem branch_total_covers_right (left right : Flow) (agree : left.net = right.net) :
    QuantVector.Le right.total (branchJoin left right agree).total := by
  exact QuantVector.right_le_join left.total right.total

theorem branch_preserves_agreed_net (left right : Flow)
    (agree : left.net = right.net) :
    (branchJoin left right agree).net = left.net ∧
      (branchJoin left right agree).net = right.net := by
  exact ⟨rfl, agree⟩

theorem call_uses_summary (summary : FunctionSummary) : call summary = summary := by
  rfl

theorem bounded_loop_has_zero_net (body : Flow) (bound : Nat)
    (zeroNet : body.net = SignedVector.zero) :
    (boundedZeroNetLoop body bound zeroNet).net = SignedVector.zero := by
  rfl

theorem bounded_loop_preserves_peak (body : Flow) (bound : Nat)
    (zeroNet : body.net = SignedVector.zero) :
    (boundedZeroNetLoop body bound zeroNet).peak = body.peak := by
  rfl

theorem bounded_loop_scales_total (body : Flow) (bound : Nat)
    (zeroNet : body.net = SignedVector.zero) :
    (boundedZeroNetLoop body bound zeroNet).total =
      QuantVector.scale bound body.total := by
  rfl

structure Observation where
  peak : QuantVector
  total : QuantVector

def Covers (flow : Flow) (observation : Observation) : Prop :=
  QuantVector.Le observation.peak flow.peak ∧
    QuantVector.Le observation.total flow.total

structure Bounds where
  peak : QuantVector
  total : QuantVector

def Bounds.Accepts (bounds : Bounds) (flow : Flow) : Prop :=
  QuantVector.Le flow.peak bounds.peak ∧ QuantVector.Le flow.total bounds.total

/-- If interpretation is covered by its static summary and the summary is
    accepted, the observed peak and total cannot exceed the declared bounds. -/
theorem accepted_flow_bounds_observation (flow : Flow) (observation : Observation)
    (bounds : Bounds) (covered : flow.Covers observation)
    (accepted : bounds.Accepts flow) :
    QuantVector.Le observation.peak bounds.peak ∧
      QuantVector.Le observation.total bounds.total := by
  exact ⟨QuantVector.le_trans covered.1 accepted.1,
    QuantVector.le_trans covered.2 accepted.2⟩

end Flow
end QCore
