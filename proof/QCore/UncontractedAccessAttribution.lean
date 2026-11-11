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

/-!
# The gate's refusal of an uncontracted call names its callee (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, gateway rule 12k)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_uncontracted_access.rs`
(`Parser::uncontracted_access_attribution`,
`Parser::uncontracted_return_attribution`) and the shared reading
`Parser::reclassified_with_borrowed_actuals` in
`legacy_call_argument_access.rs`.

The lexical gate refuses a statement whose call's actuals read memory before
the call's contract is looked up. When the callee has no contract in force,
rule W44 never reads those actuals, so the leaf names the access although the
statement is refused for the call. Rule 12k is asked last, only for exactly the
gate's refusal of a statement whose first call names a callee with no
contract and no held contract; it reads the statement again with every
memory-reading actual read as at a `Borrowed` position. When that reading is
refused for exactly the same uncontracted callee, the refusal keeps its
writes and takes that description; otherwise nothing changes.

This module proves that the rule is a re-attribution: it changes at most the
description of a refusal, never its writes, never a non-refusal, and never the
verdict of any leaf, hence never the status of a function (the join of its
leaves' verdicts, `Safe < Pending < Unknown < Unsafe`).

## What this is not

A model of the classifier. Whether the reading's answer is the uncontracted
refusal is the Rust's; that the reading leaves no state behind (its counters
are restored and its instruction discarded) is reviewed and bound by tests;
faithful token extraction is trust boundary 28.
-/

namespace QCore.UncontractedAccessAttribution

/-- The four-valued verdict and its order `Safe < Pending < Unknown < Unsafe`. -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

namespace Verdict

def rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- The join (the composition of verdicts). -/
def join (a b : Verdict) : Verdict := if a.rank ≤ b.rank then b else a

end Verdict

/-- A refusal's description, as far as the rule distinguishes it. -/
inductive Description where
  /-- `indirect memory access is outside the resource projection`. -/
  | gate
  /-- `call to uncontracted function `F``. -/
  | uncontracted (callee : String)
  /-- A held call's refusal (review or pin hold): its verdict is `Pending`. -/
  | held (callee : String)
  /-- Every other refusal. -/
  | other (text : String)
  deriving DecidableEq, Repr

/-- One lowered node: a refusal with its writes, or anything else. -/
inductive Node where
  | refusal (description : Description) (writes : List String)
  | admitted (name : String)
  deriving DecidableEq, Repr

/-- The verdict one node contributes (`legacy_verdict::diagnostic_verdict`):
a held call is `Pending`, every other refusal `Unknown`; an admitted node
contributes nothing (`Safe`, the unit of the join). -/
def verdict : Node → Verdict
  | .refusal (.held _) _ => .pending
  | .refusal _ _ => .unknown
  | .admitted _ => .safe

/-- A function's status: the join of its nodes' verdicts. -/
def status (nodes : List Node) : Verdict := nodes.foldl (fun v n => v.join (verdict n)) .safe

/-- What the rule knows about the statement's first call. -/
structure Call where
  callee : String
  /-- A contract (declared or a validated summary) is in force. -/
  contracted : Bool
  /-- A review or pin hold withholds the callee's contract. -/
  heldCallee : Bool
  /-- The statement reads a tracked descriptor. -/
  readsDescriptor : Bool
  deriving DecidableEq, Repr

/-- The rule: `reading` is the classification with every memory-reading actual
read as at a `Borrowed` position (`none` when an actual is recognised by
neither reader). -/
def reattribute (call : Call) (reading : Option Node) : Node → Node
  | .refusal .gate writes =>
      if call.contracted || call.heldCallee || call.readsDescriptor then
        .refusal .gate writes
      else
        match reading with
        | some (.refusal (.uncontracted f) _) =>
            if f = call.callee then .refusal (.uncontracted call.callee) writes
            else .refusal .gate writes
        | _ => .refusal .gate writes
  | n => n

theorem only_the_gate_refusal_is_read (call : Call) (reading : Option Node) (n : Node)
    (h : ∀ w, n ≠ .refusal .gate w) : reattribute call reading n = n := by
  cases n with
  | refusal d w =>
      cases d with
      | gate => exact absurd rfl (h w)
      | uncontracted _ => rfl
      | held _ => rfl
      | other _ => rfl
  | admitted _ => rfl

theorem the_writes_are_kept (call : Call) (reading : Option Node) (w : List String) :
    ∃ d, reattribute call reading (.refusal .gate w) = .refusal d w := by
  simp only [reattribute]
  split
  · exact ⟨_, rfl⟩
  · split
    · split
      · exact ⟨_, rfl⟩
      · exact ⟨_, rfl⟩
    · exact ⟨_, rfl⟩

theorem the_result_is_the_gate_refusal_or_the_callees (call : Call) (reading : Option Node)
    (w : List String) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w ∨
      reattribute call reading (.refusal .gate w) = .refusal (.uncontracted call.callee) w := by
  simp only [reattribute]
  split
  · exact Or.inl rfl
  · split
    · split
      · exact Or.inr rfl
      · exact Or.inl rfl
    · exact Or.inl rfl

theorem a_refusal_stays_a_refusal (call : Call) (reading : Option Node) (d : Description)
    (w : List String) : ∃ d', reattribute call reading (.refusal d w) = .refusal d' w := by
  cases d with
  | gate => exact the_writes_are_kept call reading w
  | uncontracted f => exact ⟨_, rfl⟩
  | held f => exact ⟨_, rfl⟩
  | other t => exact ⟨_, rfl⟩

theorem the_verdict_of_every_node_is_unchanged (call : Call) (reading : Option Node) (n : Node) :
    verdict (reattribute call reading n) = verdict n := by
  cases n with
  | admitted _ => rfl
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_the_callees call reading w with h | h
          · rw [h]
          · rw [h]; rfl
      | uncontracted _ => rfl
      | held _ => rfl
      | other _ => rfl

theorem the_status_of_a_function_is_unchanged (call : Node → Call) (reading : Node → Option Node)
    (nodes : List Node) :
    status (nodes.map fun n => reattribute (call n) (reading n) n) = status nodes := by
  unfold status
  suffices ∀ v : Verdict, List.foldl (fun (v : Verdict) n => v.join (verdict n)) v
      (nodes.map fun n => reattribute (call n) (reading n) n) =
      List.foldl (fun (v : Verdict) n => v.join (verdict n)) v nodes from this _
  induction nodes with
  | nil => intro v; rfl
  | cons n rest ih =>
      intro v
      simp only [List.map_cons, List.foldl_cons, the_verdict_of_every_node_is_unchanged]
      exact ih _

theorem a_contracted_callee_keeps_the_gate_refusal (call : Call) (reading : Option Node)
    (w : List String) (h : call.contracted = true) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, h]

theorem a_held_callee_keeps_the_gate_refusal (call : Call) (reading : Option Node)
    (w : List String) (h : call.heldCallee = true) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, h]

theorem a_descriptor_read_keeps_the_gate_refusal (call : Call) (reading : Option Node)
    (w : List String) (h : call.readsDescriptor = true) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, h]

theorem an_unrecognised_actual_keeps_the_gate_refusal (call : Call) (w : List String) :
    reattribute call none (.refusal .gate w) = .refusal .gate w := by
  simp only [reattribute]
  split <;> rfl

theorem an_admitting_reading_keeps_the_gate_refusal (call : Call) (name : String)
    (w : List String) :
    reattribute call (some (.admitted name)) (.refusal .gate w) = .refusal .gate w := by
  simp only [reattribute]
  split <;> rfl

theorem another_refusal_keeps_the_gate_refusal (call : Call) (d : Description)
    (w w' : List String) (h : ∀ f, d ≠ .uncontracted f) :
    reattribute call (some (.refusal d w')) (.refusal .gate w) = .refusal .gate w := by
  cases d with
  | uncontracted f => exact absurd rfl (h f)
  | gate => simp only [reattribute]; split <;> rfl
  | held _ => simp only [reattribute]; split <;> rfl
  | other _ => simp only [reattribute]; split <;> rfl

theorem the_attributed_callee_is_the_statements (call : Call) (reading : Option Node)
    (w : List String) (f : String)
    (h : reattribute call reading (.refusal .gate w) = .refusal (.uncontracted f) w) :
    f = call.callee ∧ call.contracted = false ∧ call.heldCallee = false := by
  rcases the_result_is_the_gate_refusal_or_the_callees call reading w with h' | h'
  · rw [h'] at h; cases h
  · rw [h'] at h
    cases h
    refine ⟨rfl, ?_, ?_⟩
    · cases hc : call.contracted
      · rfl
      · rw [a_contracted_callee_keeps_the_gate_refusal call reading w hc] at h'; cases h'
    · cases hh : call.heldCallee
      · rfl
      · rw [a_held_callee_keeps_the_gate_refusal call reading w hh] at h'; cases h'

theorem an_attributed_leaf_is_never_pending (call : Call) (reading : Option Node) (n : Node) :
    verdict (reattribute call reading n) = .pending → verdict n = .pending := by
  rw [the_verdict_of_every_node_is_unchanged]; exact id

theorem the_attribution_is_idempotent (call : Call) (reading : Option Node) (n : Node) :
    reattribute call reading (reattribute call reading n) = reattribute call reading n := by
  cases n with
  | admitted _ => rfl
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_the_callees call reading w with h | h
          · rw [h, h]
          · rw [h]; rfl
      | uncontracted _ => rfl
      | held _ => rfl
      | other _ => rfl

/-- `cmt_atomic_load(&m->hist_count)`, uncontracted, its actual read: the leaf
names the callee and stays `Unknown`. -/
example :
    reattribute ⟨"cmt_atomic_load", false, false, false⟩
        (some (.refusal (.uncontracted "cmt_atomic_load") ["v"])) (.refusal .gate ["v"]) =
      .refusal (.uncontracted "cmt_atomic_load") ["v"] := by decide

/-- The same call under a review hold keeps the gate's refusal. -/
example :
    reattribute ⟨"cmt_atomic_load", false, true, false⟩
        (some (.refusal (.uncontracted "cmt_atomic_load") ["v"])) (.refusal .gate ["v"]) =
      .refusal .gate ["v"] := by decide

end QCore.UncontractedAccessAttribution
