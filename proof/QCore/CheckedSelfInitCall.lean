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

import QCore.PointerStore

/-! Rule 36a. The leaf uses CG11's existing self-address stores. A new call
is available only to the checked local row, never as a custody summary.
Source extraction, the full CFG check and Rust correspondence are premise 28. -/
namespace QCore.CheckedSelfInitCall

structure SourceFacts where
  voidSingleFormal : Bool
  completePureBody : Bool
  typedSelfStores : Bool
  originalDirectActual : Bool
  nonEscaping : Bool
  leafCfgChecked : Bool
  exactStoreMapping : Bool
  interfaceAbsent : Bool
  deriving DecidableEq, Repr

def sourceAdmits (f : SourceFacts) : Bool :=
  f.voidSingleFormal && f.completePureBody && f.typedSelfStores &&
  f.originalDirectActual && f.nonEscaping && f.leafCfgChecked &&
  f.exactStoreMapping && f.interfaceAbsent

theorem a_missing_source_obligation_refuses (f : SourceFacts)
    (h : f.voidSingleFormal = false ∨ f.completePureBody = false ∨
      f.typedSelfStores = false ∨ f.originalDirectActual = false ∨
      f.nonEscaping = false ∨ f.leafCfgChecked = false ∨
      f.exactStoreMapping = false ∨ f.interfaceAbsent = false) :
    sourceAdmits f = false := by
  rcases h with h | h | h | h | h | h | h | h <;> simp [sourceAdmits, h]

structure Pointer where
  targets : List Nat
  unknown : Bool := false
  identity : Bool := true
  deriving DecidableEq, Repr

structure State (α : Type) where
  ledger : List (Nat × Nat) := []
  otherCustodyEmpty : Bool := true
  memberAlias : Bool := false
  unmodeled : Bool := false
  widened : Bool := false
  pointer : Option Pointer := none
  diagnostics : List Nat := []
  frame : α
  deriving DecidableEq, Repr

def pointerAdmits : Option Pointer → Bool
  | none => true
  | some p => !p.unknown && p.identity && p.targets.length == 1

def admits (s : State α) (enabled : Bool) : Bool :=
  enabled && s.ledger.isEmpty && s.otherCustodyEmpty && !s.memberAlias &&
    !s.unmodeled && !s.widened && pointerAdmits s.pointer

/-- `fallbackFrame` and `fallbackPointer` are the existing Unknown transfer's
binding/frame result. That transfer changes no resource status. -/
def transfer (s : State α) (enabled : Bool) (fallbackFrame : α)
    (fallbackPointer : Option Pointer) : State α :=
  if admits s enabled then s else
    { s with
      frame := fallbackFrame
      pointer := fallbackPointer
      unmodeled := true
      diagnostics := 0 :: s.diagnostics }

theorem admitted_call_preserves_every_field (s : State α) (enabled : Bool)
    (f : α) (p : Option Pointer) (h : admits s enabled = true) :
    transfer s enabled f p = s := by simp [transfer, h]

theorem no_transfer_discharges_an_obligation (s : State α) (enabled : Bool)
    (f : α) (p : Option Pointer) :
    (transfer s enabled f p).ledger = s.ledger := by
  unfold transfer
  split <;> rfl

theorem default_entry_refuses (s : State α) : admits s false = false := by
  simp [admits]

theorem a_tracked_frame_refuses (s : State α) (site status : Nat)
    (rest : List (Nat × Nat)) (enabled : Bool)
    (h : s.ledger = (site, status) :: rest) : admits s enabled = false := by
  simp [admits, h]

theorem refusal_remains_unknown (s : State α) (enabled : Bool)
    (f : α) (p : Option Pointer) (h : admits s enabled = false) :
    (transfer s enabled f p).unmodeled = true ∧
    (transfer s enabled f p).diagnostics = 0 :: s.diagnostics := by
  simp [transfer, h]

def repeatCalls (s : State α) (f : α) (p : Option Pointer) : Nat → State α
  | 0 => s
  | n + 1 => transfer (repeatCalls s f p n) true f p

theorem finite_checked_calls_preserve_the_frame (s : State α) (f : α)
    (p : Option Pointer) (h : admits s true = true) (n : Nat) :
    repeatCalls s f p n = s := by
  induction n with
  | zero => rfl
  | succ n ih => simp [repeatCalls, ih, transfer, h]

inductive Consumer where
  | localRow | ordinarySummary | callback | exported | recursive
  deriving DecidableEq, Repr

def available (consumer : Consumer) (sourceChecked rowChecked : Bool) : Bool :=
  consumer == .localRow && sourceChecked && rowChecked

theorem no_ordinary_summary (s r : Bool) : available .ordinarySummary s r = false := by rfl
theorem no_callback_transport (s r : Bool) : available .callback s r = false := by rfl
theorem no_cross_unit_transport (s r : Bool) : available .exported s r = false := by rfl
theorem no_recursive_assumption (s r : Bool) : available .recursive s r = false := by rfl

/-- A concrete self edge is non-owning. Its reference cannot outlive its
referent; all old lifetime obligations are retained. -/
def lifetimeValid (ends : Nat → Nat) (edges : List (Nat × Nat)) : Prop :=
  ∀ edge ∈ edges, ends edge.1 ≤ ends edge.2

theorem self_stores_preserve_reference_lifetimes (ends : Nat → Nat)
    (edges : List (Nat × Nat)) (base : Nat) (h : lifetimeValid ends edges) :
    lifetimeValid ends ((base, base) :: edges) := by
  intro edge member
  rcases List.mem_cons.mp member with same | old
  · subst edge; exact Nat.le_refl _
  · exact h edge old

def initial : State Unit := { frame := () }
def named : State Unit := { initial with pointer := some { targets := [7] } }
def flowVectors : List Bool :=
  [ admits initial true, admits initial false,
    admits { initial with ledger := [(7,1)] } true,
    admits { initial with ledger := [(7,2)] } true,
    admits { initial with ledger := [(7,4)] } true,
    admits { initial with otherCustodyEmpty := false } true,
    admits { initial with unmodeled := true } true,
    admits { initial with widened := true } true,
    admits { initial with memberAlias := true } true,
    admits named true,
    admits { named with pointer := some { targets := [7], unknown := true } } true,
    admits { initial with pointer := some { targets := [] } } true,
    admits { initial with pointer := some { targets := [7,8] } } true,
    admits { initial with pointer := some { targets := [7], identity := false } } true ]

theorem shared_flow_vectors : flowVectors =
    [true,false,false,false,false,false,false,false,false,true,false,false,false,false] := by decide

def completeSource : SourceFacts := ⟨true,true,true,true,true,true,true,true⟩
def sourceVectors : List Bool :=
  [ sourceAdmits completeSource,
    sourceAdmits { completeSource with voidSingleFormal := false },
    sourceAdmits { completeSource with completePureBody := false },
    sourceAdmits { completeSource with typedSelfStores := false },
    sourceAdmits { completeSource with originalDirectActual := false },
    sourceAdmits { completeSource with nonEscaping := false },
    sourceAdmits { completeSource with leafCfgChecked := false },
    sourceAdmits { completeSource with exactStoreMapping := false },
    sourceAdmits { completeSource with interfaceAbsent := false } ]

theorem shared_source_vectors : sourceVectors =
    [true,false,false,false,false,false,false,false,false] := by decide

end QCore.CheckedSelfInitCall
