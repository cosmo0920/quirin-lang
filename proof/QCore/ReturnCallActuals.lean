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

import QCore.CallArgumentAccess
import QCore.ReturnCallCustody

/-!
# A refused `return`'s call read with rule W44 (card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS, gateway rule 12i)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_return_call_access.rs`
(`Parser::return_call_argument_access`) and its hook in `return_statement`.

The return position decides `return f(args);` as the synthetic statement
`PLACE = f(args)` (C-GATEWAY-RETURN-CALL-W4) and B01's `return E(f(args))` as
the call alone. The lexical gate refuses both when an actual reads memory, and
the return position never asked rule W44, which reads those actuals at the
statement position. This rule asks it, after every older route declined:

* the statement W44 sees is the one the return position classified, so W44's
  whole decision (`QCore.CallArgumentAccess.Statement.admitted`) is reused
  unchanged: an actual at a custody-taking position is never used, an
  uncontracted callee, a descriptor read or a declined probe keeps the refusal;
* the return position's acceptance of the call's result is reused unchanged
  (`QCore.ReturnCallCustody.escapesOnlyFreshCustody`; B01's form accepts only a
  `none` result, whose exit escapes nothing);
* the lowering is W44's block, reads first and the call last, followed by the
  return position's exit: the existing pair `PLACE = f(args); return PLACE;`
  with the reads before the call.

## What this is not

A model of the decision and its composition. That C evaluates every actual
before the call (C11 6.5.2.2p10) is the sequencing the lowering states; the
correspondence with the Rust is reviewed and bound by tests; faithful Clang and
token extraction is trust boundary 28.
-/

namespace QCore.ReturnCallActuals

open QCore.CallArgumentAccess (Statement Instr lower)
open QCore.ReturnCallCustody (ResultMode escapesOnlyFreshCustody)

/-- The two forms the rule reads. -/
inductive Form where
  /-- `return f(args);` -/
  | wholeCall
  /-- `return E(f(args));`, `E` scalar, the callee externally declared (B01). -/
  | scalarCall
  deriving DecidableEq, Repr

/-- The return position's acceptance of the call's result, per form. -/
def accepts : Form → ResultMode → Bool
  | .wholeCall, m => escapesOnlyFreshCustody m
  | .scalarCall, m => m == .none

/-- One instruction of the return's lowering: W44's, then the exit. -/
inductive Node where
  | instr (i : Instr)
  /-- `EscapeIfOwned PLACE` for the whole call, `NoOp` for B01's form. -/
  | exit (escapes : Bool)
  | refusal
  deriving DecidableEq, Repr

def admitted (f : Form) (s : Statement) (m : ResultMode) : Bool :=
  s.admitted && accepts f m

/-- The lowering of a refused `return`: the refusal unless admitted; otherwise
W44's block of the same statement, then the exit. -/
def lowerReturn (f : Form) (s : Statement) (m : ResultMode) : List Node :=
  if admitted f s m then
    (lower s).map Node.instr ++ [.exit (f == .wholeCall)]
  else [.refusal]

/-- **The composition.** An admitted return is W44's statement lowering,
unchanged, followed by the return position's exit. -/
theorem the_lowering_is_the_statement_lowering_then_the_exit
    (f : Form) (s : Statement) (m : ResultMode) (h : admitted f s m = true) :
    lowerReturn f s m = (lower s).map Node.instr ++ [.exit (f == .wholeCall)] := by
  simp [lowerReturn, h]

/-- The reads come first, then the call, then the exit. -/
theorem the_reads_precede_the_call_which_precedes_the_exit
    (f : Form) (s : Statement) (m : ResultMode) (h : admitted f s m = true) :
    lowerReturn f s m =
      [.instr (.typedRead s.bases), .instr (.argumentReads s.names s.pointerBases s.pointerPlaces),
       .instr .call, .exit (f == .wholeCall)] := by
  have hs : s.admitted = true := by
    simp only [admitted, Bool.and_eq_true] at h; exact h.1
  simp [lowerReturn, h, lower, hs]

/-- Every refusal of W44 is kept. -/
theorem a_statement_w44_refuses_keeps_the_refusal
    (f : Form) (s : Statement) (m : ResultMode) (h : s.admitted = false) :
    lowerReturn f s m = [.refusal] := by
  simp [lowerReturn, admitted, h]

theorem an_uncontracted_callee_keeps_the_refusal
    (f : Form) (s : Statement) (m : ResultMode) (h : s.contracted = false) :
    lowerReturn f s m = [.refusal] := by
  apply a_statement_w44_refuses_keeps_the_refusal
  simp [Statement.admitted, h]

theorem a_descriptor_read_keeps_the_refusal
    (f : Form) (s : Statement) (m : ResultMode) (h : s.readsDescriptor = true) :
    lowerReturn f s m = [.refusal] := by
  apply a_statement_w44_refuses_keeps_the_refusal
  simp [Statement.admitted, h]

theorem a_declined_probe_keeps_the_refusal
    (f : Form) (s : Statement) (m : ResultMode) (h : s.probeIsCall = false) :
    lowerReturn f s m = [.refusal] := by
  apply a_statement_w44_refuses_keeps_the_refusal
  simp [Statement.admitted, h]

/-- No actual at a custody-taking position is ever used by an admitted return. -/
theorem a_custody_position_is_never_used
    (s : Statement) (a : QCore.CallArgumentAccess.Actual) (h : a ∈ s.used) :
    a.position.takesCustody = false :=
  QCore.CallArgumentAccess.a_custody_position_is_never_used s a h

/-- A reallocated result keeps the refusal, whatever W44 decides. -/
theorem a_reallocated_result_keeps_the_refusal (s : Statement) (p : Nat) (f : Form) :
    lowerReturn f s (.reallocated p) = [.refusal] := by
  cases f <;> simp [lowerReturn, admitted, accepts, escapesOnlyFreshCustody]

/-- B01's form accepts only a `none` result, and its exit escapes nothing. -/
theorem the_scalar_form_returns_no_custody (s : Statement) (m : ResultMode)
    (h : admitted .scalarCall s m = true) :
    m = .none ∧ (lowerReturn .scalarCall s m).getLast? = some (.exit false) := by
  have hm : m = .none := by
    simp only [admitted, accepts, Bool.and_eq_true, beq_iff_eq] at h; exact h.2
  refine ⟨hm, ?_⟩
  simp [lowerReturn, h]

/-- The exit of an admitted whole-call return discharges no obligation the body
already owed (`QCore.ReturnCallCustody`'s theorem, at the accepted modes). -/
theorem the_exit_discharges_nothing_prior (s : Statement) (m : ResultMode)
    (h : admitted .wholeCall s m = true)
    (st : QCore.ReturnCallCustody.State) (site : Nat) (hs : site < st.nextSite) :
    (QCore.ReturnCallCustody.returnCall m st).custody site = st.custody site := by
  have hm : escapesOnlyFreshCustody m = true := by
    simp only [admitted, accepts, Bool.and_eq_true] at h; exact h.2
  exact QCore.ReturnCallCustody.admitted_modes_discharge_nothing_prior m st site hm hs

/-! ## Examples -/

/-- `return strcmp(a->name, b->name);`: two pointer reads at `Borrowed`
positions of a contracted callee with a `none` result. -/
def comparator : Statement :=
  { gateRefused := true, readsDescriptor := false, contracted := true,
    actuals :=
      [ { position := .borrowed, access := true, shape := .pointer (some 1) (some 11), names := [1] }
      , { position := .borrowed, access := true, shape := .pointer (some 2) (some 12), names := [2] } ],
    probeIsCall := true }

/-- `return f(o->child);` at a `Released` position. -/
def releasing : Statement :=
  { comparator with
    actuals := [ { position := .released, access := true, shape := .pointer (some 1) (some 11),
                   names := [1] } ],
    probeIsCall := false }

theorem the_comparator_is_admitted : admitted .wholeCall comparator .none = true := by decide

theorem the_comparator_is_read_before_it_is_returned :
    lowerReturn .wholeCall comparator .none =
      [.instr (.typedRead [1, 2]), .instr (.argumentReads [1, 2] [1, 2] [11, 12]),
       .instr .call, .exit true] := by decide

theorem a_releasing_return_keeps_the_refusal :
    lowerReturn .wholeCall releasing .none = [.refusal] := by decide

end QCore.ReturnCallActuals
