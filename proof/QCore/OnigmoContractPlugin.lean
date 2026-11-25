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

import QCore.VerdictSemigroup

/-!
# The Onigmo contract plugin

Card **ONIGMO-PROVIDER-PLUGIN** (user decisions 2026-11-25). The user crate
`providers/quirin-provider-onigmo` states Onigmo's regex functions in clause
kinds that already exist (contract specification C04.2, C04.5, C04.14,
C04.19, C06.2); its answers reach the audit only through the checker-plugin
firewall of `QCore.VerdictSemigroup` and the join. This module models the
three readings the crate's declarations rest on and the checker's
composition:

* **`onig_new`'s slot** (`regcomp.c:5875-5895`). Every exit leaves the slot
  NULL or a fresh object, and fresh exactly on the `0` result. So the
  unconditional `resource_out(reg, nullable)` states every exit, and a
  non-null slot is always the fresh object.
* **`onig_region_free`'s `free_self` split** (`regexec.c:342-355`). The body
  releases the region object exactly when `free_self ≠ 0` (a NULL region:
  nothing); the declaration, bounded by the literal-argument premise
  `free_self == 1`, states the release only there, agrees with the body
  there, and leaves every other call undeclared (`Unknown`, never `Safe`).
  Reading the `0` arm as a release would be wrong.
* **The `global` holder** (C04.14, rule 12t's table). `onig_new` keeps `enc`
  and `syntax` in the object its slot receives, a holder the vocabulary
  cannot name; the declaration names `global`. Whatever `global` admits
  (static storage only), every holder admits: the stronger holder is sound.
* **The checker and the firewall.** A binding is established exactly when
  all five checks pass; an established binding is published `Pending`, any
  other answer and a failed run `Unknown`; nothing is `Safe` or a provisional
  `Unsafe`; a caller applying a bound contract is never `Safe`, and a `Safe`
  host over established bindings only (`flb_regex_init` over `onig_init`) is
  `Pending`.

## The premises (not proved here)

* That the Rust computes these functions (the crate's and the gateway
  fixtures' tests), and that the C bodies are the ones the lock pins (C06.2).
* The custody vocabulary's own soundness: `QCore.RetainedBy`,
  `QCore.LiteralArgumentPremise`, `QCore.CallbackContracts`,
  `QCore.LibraryVersionRange`, `QCore.ContractRevisionMode`.
-/

namespace QCore.OnigmoContractPlugin

open QCore.VerdictSemigroup

/-! ## `onig_new`'s slot -/

/-- What the slot `*reg` holds when the call returns. -/
inductive Slot where
  | null
  | fresh
  deriving DecidableEq, Repr

/-- The body of `onig_new`: the allocation (`xmalloc`, `:5882-5883`), the
initialisation's code (`onig_reg_init`, `:5885-5886`) and the compilation's
(`onig_compile`, `:5888`); every failure leaves `*reg = NULL`
(`:5883`, `:5892`) and returns a nonzero code. `memory` is
`ONIGERR_MEMORY`. -/
def onigNew (allocated : Bool) (initCode compileCode memory : Int) : Int × Slot :=
  if !allocated then (memory, .null)
  else if initCode ≠ 0 then (initCode, .null)
  else if compileCode ≠ 0 then (compileCode, .null)
  else (0, .fresh)

/-- The slot is fresh exactly on the `0` result (given `ONIGERR_MEMORY ≠ 0`). -/
theorem fresh_iff_normal (allocated : Bool) (initCode compileCode memory : Int)
    (memoryNonzero : memory ≠ 0) :
    (onigNew allocated initCode compileCode memory).2 = .fresh ↔
      (onigNew allocated initCode compileCode memory).1 = 0 := by
  cases allocated
  · simp [onigNew, memoryNonzero]
  · by_cases i : initCode = 0 <;> by_cases c : compileCode = 0 <;> simp [onigNew, i, c]

/-- A nonzero result leaves the slot NULL: nothing is owed there. -/
theorem a_failure_leaves_null (allocated : Bool) (initCode compileCode memory : Int)
    (failed : (onigNew allocated initCode compileCode memory).1 ≠ 0) :
    (onigNew allocated initCode compileCode memory).2 = .null := by
  cases allocated
  · simp [onigNew]
  · by_cases i : initCode = 0 <;> by_cases c : compileCode = 0 <;> simp_all [onigNew]

/-- The unconditional nullable out slot: on every exit the slot is NULL or
the fresh object, and a non-null slot is the fresh object the caller owns. -/
theorem every_exit_is_a_nullable_out (allocated : Bool) (initCode compileCode memory : Int) :
    (onigNew allocated initCode compileCode memory).2 ≠ .null →
      (onigNew allocated initCode compileCode memory).2 = .fresh := by
  intro h
  generalize (onigNew allocated initCode compileCode memory).2 = s at h ⊢
  cases s <;> simp_all

/-! ## `onig_region_free`'s `free_self` split -/

/-- What one call does to the region object. -/
inductive RegionEffect where
  | nothing
  | arraysOnly
  | releasesObject
  deriving DecidableEq, Repr

/-- The body (`regexec.c:342-355`): nothing for NULL; the arrays always; the
object itself exactly when `free_self ≠ 0`. -/
def regionFree (isNull : Bool) (freeSelf : Int) : RegionEffect :=
  if isNull then .nothing else if freeSelf ≠ 0 then .releasesObject else .arraysOnly

/-- How the declaration reads a call: applied only where the actual at
`free_self` is the literal `1` (C04.5), there `resource_released(region,
nullable)`; elsewhere undeclared. -/
def declared (literalOne : Bool) : Option (Bool → RegionEffect) :=
  if literalOne then some (fun isNull => if isNull then .nothing else .releasesObject) else none

/-- Where the declaration applies, it states exactly what the body does with
the actual `1`. -/
theorem the_declared_arm_agrees_with_the_body (isNull : Bool) :
    ∃ f, declared true = some f ∧ f isNull = regionFree isNull 1 := by
  refine ⟨_, rfl, ?_⟩
  cases isNull <;> simp [regionFree]

/-- Every other actual (`0`, another value, a variable) is undeclared. -/
theorem other_actuals_are_undeclared : declared false = none := rfl

/-- The `0` arm keeps the object: a release reading there would be wrong,
which is why the premise bounds the declaration. -/
theorem the_zero_arm_is_not_a_release : regionFree false 0 ≠ .releasesObject := by
  simp [regionFree]

/-- The verdict of one call: an undeclared callee is `Unknown`; a declared
one decides by its custody (here `Safe` is the host's reading). -/
def callValue (literalOne : Bool) (custodyOk : Bool) : V :=
  match declared literalOne with
  | none => .unknown
  | some _ => if custodyOk then .safe else .unknown

/-- An undeclared `onig_region_free` call is never `Safe`. -/
theorem an_undeclared_region_free_is_never_safe (custodyOk : Bool) :
    callValue false custodyOk ≠ .safe := by
  simp [callValue, declared]

/-! ## The `global` holder -/

/-- What a retained position receives (rule 12t's classes). -/
inductive Retained where
  | static
  | nothing
  | callerObject
  | tracked
  | other
  deriving DecidableEq, Repr

/-- Who keeps the pointer. -/
inductive Holder where
  | global
  | frame
  | tracked
  | foreign
  deriving DecidableEq, Repr

/-- Rule 12t's decision: `true` is "outlives". -/
def decide : Retained → Holder → Bool
  | .static, _ => true
  | .nothing, _ => true
  | .callerObject, .frame => true
  | _, _ => false

/-- The `global` holder admits only static storage or no object. -/
theorem global_admits_only_static (p : Retained) (h : decide p .global = true) :
    p = .static ∨ p = .nothing := by
  cases p <;> simp_all [decide]

/-- Whatever the `global` holder admits, every holder admits: naming
`global` for the slot's object is the refusing direction. -/
theorem global_admission_holds_for_every_holder (p : Retained) (h : decide p .global = true)
    (holder : Holder) : decide p holder = true := by
  cases p <;> cases holder <;> simp_all [decide]

/-- Fluent Bit's `&OnigEncodingUTF_8`, `&OnigSyntaxRuby` are neither a
string literal nor a null constant: `other`, refused under `global`. -/
theorem a_file_scope_address_is_refused_under_global : decide .other .global = false := rfl

/-! ## The checker and the firewall -/

/-- What the checker sees for one binding: the five checks of
`providers/quirin-provider-onigmo/src/check.rs`. -/
structure Binding where
  moduleBuilt : Bool
  prototypeMatches : Bool
  rangeDocumentedAndAdmitting : Bool
  pinnedWithStatedPremises : Bool
  custodyExactAndDocumented : Bool
  deriving DecidableEq, Repr

/-- The checker's answer: a `Safe` claim with no evidence exactly when every
check passes. -/
def established (b : Binding) : Bool :=
  b.moduleBuilt && b.prototypeMatches && b.rangeDocumentedAndAdmitting &&
    b.pinnedWithStatedPremises && b.custodyExactAndDocumented

/-- One failed check establishes nothing. -/
theorem one_failed_check_establishes_nothing (b : Binding)
    (h : b.moduleBuilt = false ∨ b.prototypeMatches = false ∨
      b.rangeDocumentedAndAdmitting = false ∨ b.pinnedWithStatedPremises = false ∨
      b.custodyExactAndDocumented = false) :
    established b = false := by
  rcases h with h | h | h | h | h <;> simp [established, h]

/-- A run: completed with a binding's facts, or failed. -/
inductive Run where
  | completed (b : Binding)
  | failed
  deriving DecidableEq, Repr

/-- The checker package's outcome (`check::answer` through the firewall). -/
def outcome : Run → Outcome
  | .failed => .failure
  | .completed b => if established b then .answered (.safe .none) else .answered .unknown

/-- What the audit publishes for one binding. -/
def published (r : Run) : V := firewall (outcome r)

theorem never_published_safe (r : Run) : published r ≠ .safe := by
  cases r with
  | failed => simp [published, outcome, firewall]
  | completed b => cases h : established b <;> simp [published, outcome, h, firewall]

theorem never_published_unsafe (r : Run) : published r ≠ .provisionalUnsafe := by
  cases r with
  | failed => simp [published, outcome, firewall]
  | completed b => cases h : established b <;> simp [published, outcome, h, firewall]

theorem published_is_pending_or_unknown (r : Run) :
    published r = .pending ∨ published r = .unknown := by
  cases r with
  | failed => exact Or.inr rfl
  | completed b => cases h : established b <;> simp [published, outcome, h, firewall]

theorem an_established_binding_is_pending (b : Binding) (h : established b = true) :
    published (.completed b) = .pending := by
  simp [published, outcome, h, firewall]

/-- A caller's published value: its own analysis joined with the bound
contracts it applies. -/
def caller (host : V) (links : List Run) : V := fold (host :: links.map published)

theorem caller_never_below_host (host : V) (links : List Run) :
    host.rank ≤ (caller host links).rank :=
  every_link_is_below_the_fold (List.mem_cons_self ..)

theorem a_bound_caller_is_never_safe (host : V) (r : Run) (rs : List Run) (member : r ∈ rs) :
    caller host rs ≠ .safe :=
  a_non_safe_link_keeps_the_chain_non_safe
    (List.mem_cons_of_mem _ (List.mem_map_of_mem member)) (never_published_safe r)

/-- `flb_regex_init`: a `Safe` host over one established binding
(`onig_init`) is `Pending`, the one move the prediction allows. -/
theorem a_safe_host_over_one_established_binding_is_pending (b : Binding)
    (h : established b = true) : caller .safe [.completed b] = .pending := by
  simp [caller, fold, an_established_binding_is_pending b h, safe_join, join_safe]

/-- The reasoned join keeps every binding's reasons beside the host's. -/
theorem every_reason_is_kept {R : Type} (host link : Reasoned R) (r : R)
    (member : r ∈ link.reasons) : r ∈ (Reasoned.join host link).reasons :=
  Reasoned.join_keeps_reasons host link r (Or.inr member)

end QCore.OnigmoContractPlugin
