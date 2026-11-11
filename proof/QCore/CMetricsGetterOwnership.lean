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

import QCore.CmetricsApiContracts

/-! A source-adapter-independent model of the CMetrics getter ownership graph.
The Rust adapter must validate the concrete fields, calls and XXH3 bodies before
using this model. Parsing a `.qv` model alone grants no executable permission. -/
namespace QCore.CMetricsGetterOwnership

inductive Selection where
  | embedded
  | primary (id : Nat)
  deriving DecidableEq, Repr

structure State where
  live : Bool := true
  epoch : Nat := 0
  staticSet : Bool := false
  embeddedStorage : Bool := false
  primary : List Nat := []
  secondary : List Nat := []
  cached : Option Nat := none
  deriving DecidableEq, Repr

structure Destruction where
  state : State
  leakedEmbeddedStorage : Bool
  deriving DecidableEq, Repr

def lookup (s : State) (hash : BitVec 64) : Option Selection :=
  if hash = 0 then some .embedded
  else s.cached.map Selection.primary

def prepare (s : State) (selected : Selection) (write : Bool) : State :=
  match selected with
  | .embedded => if write then { s with embeddedStorage := true } else s
  | .primary _ => s

def borrowValid (s : State) (issuedEpoch : Nat) (selected : Selection) : Bool :=
  s.live && issuedEpoch == s.epoch && match selected with
    | .embedded => true
    | .primary id => s.primary.contains id

/-- The audited destructor releases embedded storage only under `staticSet`. -/
def destroy (s : State) : Destruction :=
  { leakedEmbeddedStorage := s.embeddedStorage && !s.staticSet
    state := { s with live := false, epoch := s.epoch + 1, embeddedStorage := false,
                       primary := [], secondary := [], cached := none } }

def rrmxmx (h : BitVec 64) (len : Nat) : BitVec 64 :=
  let h := h ^^^ h.rotateLeft 49 ^^^ h.rotateLeft 24
  let h := h * BitVec.ofNat 64 0x9fb21c651e98df25
  let h := h ^^^ ((h >>> 35) + BitVec.ofNat 64 len)
  let h := h * BitVec.ofNat 64 0x9fb21c651e98df25
  h ^^^ (h >>> 28)

def witnessBytes : List Nat := [0x42, 0xd5, 0x68, 0xe1, 0x38, 0xd7, 0x27, 0xff]
def witnessInput : BitVec 64 := BitVec.ofNat 64 0xe168d542ff27d738
def secret8 : BitVec 64 := BitVec.ofNat 64 0x1cad21f72c81017c
def secret16 : BitVec 64 := BitVec.ofNat 64 0xdb979083e96dd4de
def witnessHash : BitVec 64 := rrmxmx (witnessInput ^^^ (secret8 ^^^ secret16)) 8

theorem witness_bytes_are_nul_free : witnessBytes.all (· != 0) = true := by decide

theorem pinned_secret_bitflip : secret8 ^^^ secret16 =
    BitVec.ofNat 64 0xc73ab174c5ecd5a2 := by decide

theorem exact_short_hash_is_zero : witnessHash = 0 := by decide

theorem zero_hash_selects_embedded (s : State) : lookup s witnessHash = some .embedded := by
  simp [lookup, exact_short_hash_is_zero]

theorem primary_secondary_state_is_preserved_by_embedded_prepare (s : State) :
    let out := prepare s .embedded true
    out.primary = s.primary ∧ out.secondary = s.secondary ∧ out.cached = s.cached := by
  simp [prepare]

theorem destroy_invalidates_borrow (s : State) (e : Nat) (selected : Selection) :
    borrowValid (destroy s).state e selected = false := by
  simp [borrowValid, destroy]

theorem labeled_zero_path_reaches_destruction_leak :
    let initial : State := { staticSet := false }
    let selected := (lookup initial witnessHash).getD (.primary 0)
    (destroy (prepare initial selected true)).leakedEmbeddedStorage = true := by
  simp [lookup, exact_short_hash_is_zero, prepare, destroy]

/-- Source inspection is a premise supplied by the Rust adapter, never by a model file. -/
def modelPermission (_sourceChecked _modelParsed : Bool) : Bool := false

theorem parsed_contract_has_no_call_permission (sourceChecked parsed : Bool) :
    modelPermission sourceChecked parsed = false := by rfl

end QCore.CMetricsGetterOwnership
