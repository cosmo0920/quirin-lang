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

/-! EXT-01 filename guards. Parsing supplies the mode; a filename never selects
another mode. The Rust-to-Lean binding is review-maintained, not extraction. -/
namespace QCore.SourceExtensions
inductive Mode where
  | executable | library | contract
  deriving DecidableEq, Repr
inductive Suffix where
  | legacy | runtime | formal | abi | checkpoint | other
  deriving DecidableEq, Repr

def admit : Suffix → Mode → Option Mode
  | .formal, .contract => some .contract
  | .formal, _ => none
  | .runtime, .contract => none
  | .runtime, mode => some mode
  | .abi, _ | .checkpoint, _ => none
  | .legacy, mode | .other, mode => some mode

theorem accepted_mode_is_parsed_mode (suffix : Suffix) (parsed accepted : Mode)
    (h : admit suffix parsed = some accepted) : accepted = parsed := by
  cases suffix <;> cases parsed <;> simp_all [admit]

theorem formal_acceptance_requires_contract (mode accepted : Mode)
    (h : admit .formal mode = some accepted) : mode = .contract := by
  cases mode <;> simp_all [admit]

theorem runtime_never_admits_contract : admit .runtime .contract = none := by
  rfl

theorem legacy_preserves_every_mode (mode : Mode) : admit .legacy mode = some mode := by
  cases mode <;> rfl

/-- Warnings accompany results and cannot replace the checker's outcome. -/
def publish {Outcome : Type} (outcome : Outcome) (legacy : Bool) : Outcome × Bool :=
  (outcome, legacy)

theorem warning_does_not_replace_outcome {Outcome : Type} (outcome : Outcome) (legacy : Bool) :
    (publish outcome legacy).1 = outcome := by rfl
end QCore.SourceExtensions
