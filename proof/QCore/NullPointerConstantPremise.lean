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

import QCore.LiteralArgumentPremise

/-!
# The null pointer constant at a pointer premise (card CURRENT-CONTEXT-NEW-TRIAGE, D3)

User decision, 2026-11-16, quoted exactly: "Admit (void *) 0
(Recommended)". Contract specification §C04.5.1 now admits the C11 6.3.2.3p3
null pointer constant `(void *) 0` as a spelling of the declared literal `0`,
and only at a **pointer-typed** premise parameter.
`crates/qcore-c-gateway/src/legacy_literal_premise.rs` (`satisfies_premise`,
`void_pointer_null_constant`) recognises the spelling structurally: matched
parentheses, the cast exactly `void *`, and the operand the canonical `0`.

This module extends `QCore.LiteralArgumentPremise`'s decision with that one
alternative. The recogniser is a parameter (`isVoidNull`), because spelling
recognition is the Rust's and is pinned by its unit tests (trust boundary 28).

## What is proved

* At a scalar premise, or against a declared literal other than `0`, the
  decision is the old one (`a_scalar_premise_is_unchanged`,
  `another_literal_is_unchanged`).
* The widening is strict: whatever satisfied the old decision still does
  (`the_old_spellings_still_satisfy`).
* The premise still never alters a declaration. An applied contract is the
  declared one (`an_applied_contract_is_still_the_declared_one`), and a
  poisoned environment still applies nothing.
* Non-vacuity: the cast spelling is admitted at a pointer premise of `0` and
  refused at a scalar one (`the_cast_meets_only_a_pointer_premise`).
-/

namespace QCore.NullPointerConstantPremise

open QCore.LiteralArgumentPremise

/-- A premise with the parameter's pointer-ness. -/
structure PPremise where
  premise : Premise
  pointer : Bool
  deriving DecidableEq, Repr

/-- The decision: the old one, or, at a pointer premise of literal `[0]`, an
actual the structural recogniser reads as `(void *) 0`. -/
def satisfiesP (isVoidNull : Tokens → Bool) (arguments : List Tokens) (p : PPremise) : Bool :=
  satisfies arguments p.premise
    || (p.pointer && p.premise.literal == [0]
        && (match arguments[p.premise.index]? with
            | some actual => isVoidNull actual
            | none => false))

theorem a_scalar_premise_is_unchanged (v : Tokens → Bool) (arguments : List Tokens)
    (p : PPremise) (h : p.pointer = false) :
    satisfiesP v arguments p = satisfies arguments p.premise := by
  simp [satisfiesP, h]

theorem another_literal_is_unchanged (v : Tokens → Bool) (arguments : List Tokens)
    (p : PPremise) (h : p.premise.literal ≠ [0]) :
    satisfiesP v arguments p = satisfies arguments p.premise := by
  simp [satisfiesP, h]

theorem the_old_spellings_still_satisfy (v : Tokens → Bool) (arguments : List Tokens)
    (p : PPremise) (h : satisfies arguments p.premise = true) :
    satisfiesP v arguments p = true := by
  simp [satisfiesP, h]

/-- An environment whose premises carry pointer-ness. -/
structure PEnvironment where
  contracts : Name → Option Contract
  premises : Name → List PPremise
  poisoned : Bool

def unmetP (v : Tokens → Bool) (e : PEnvironment) (c : Call) : Bool :=
  e.poisoned || (e.premises c.callee).any (fun p => !satisfiesP v c.arguments p)

def appliedP (v : Tokens → Bool) (e : PEnvironment) (c : Call) : Option Contract :=
  if unmetP v e c then none else e.contracts c.callee

theorem an_applied_contract_is_still_the_declared_one (v : Tokens → Bool) {e : PEnvironment}
    {c : Call} {k : Contract} (h : appliedP v e c = some k) : e.contracts c.callee = some k := by
  unfold appliedP at h
  split at h
  · exact absurd h (by simp)
  · exact h

theorem a_poisoned_environment_still_applies_nothing (v : Tokens → Bool) {e : PEnvironment}
    {c : Call} (h : e.poisoned = true) : appliedP v e c = none := by
  unfold appliedP unmetP; simp [h]

/-- Token code 7 stands for the spelling `(void *) 0`. -/
def recogniser (t : Tokens) : Bool := t == [7]

theorem the_cast_meets_only_a_pointer_premise :
    satisfiesP recogniser [[1], [7]] ⟨⟨1, [0]⟩, true⟩ = true
      ∧ satisfiesP recogniser [[1], [7]] ⟨⟨1, [0]⟩, false⟩ = false
      ∧ satisfiesP recogniser [[1], [7]] ⟨⟨1, [1]⟩, true⟩ = false := by
  decide

end QCore.NullPointerConstantPremise
