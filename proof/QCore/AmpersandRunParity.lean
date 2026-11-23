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

import QCore.NullableLazyConstructor

/-!
# Every address decision reads a run of `&` by its parity (card AMPERSAND-RUN-PARITY-OTHER-READERS, rule 74e)

The importer emits one token per `&`. `QCore.NullableLazyConstructor` (rule
74d, part C) proved that a run of `k` adjacent `&` lexes by maximal munch to
`k / 2` `&&` operators and, when `k` is odd, one `&`
(`the_run_ends_in_an_address_iff_odd`). This module states the reading every
reader of the unfused token stream now uses (`ampersand_runs.rs`, bound to
`DECISION_VECTORS`), the three readings it replaced, and what replacing each
did:

* the parity is maximal munch (`parity_is_munch`);
* the old unsound reading (a run of exactly one `&`) misses `p && &x`
  (`exactly_one_misses_an_odd_run`), and the parity keeps every address it
  saw (`the_parity_keeps_every_single_address`), so fixing it only adds
  addresses;
* the old fail-closed and admitting reading (any run) differs from the parity
  exactly on the even runs (`the_parity_drops_only_even_runs`), which are the
  logical ANDs (`logical_and_is_the_even_complement`);
* each listed reader reads the parity at the tip, and against its base
  reading the change only adds addresses for an unsound reader and only
  removes even runs for the others (`every_reader_reads_the_parity`,
  `an_unsound_fix_only_adds`, `a_fail_closed_change_only_drops_even_runs`).

## What this is not

A model of tokenisation, of C's grammar, or of each reader's surrounding
logic. That the importer splits punctuators into characters, that a
conforming body never spells `a & &b`, that a reader's single `&` after an
operand end keeps its own reading, and that the reader list is the Rust's
(`ampersand_runs::READERS`, whose call sites a Rust test counts) are premises
(trust boundary 28).
-/

namespace QCore.AmpersandRunParity

open QCore.NullableLazyConstructor (munch addressTaken address_taken_agrees_with_munch)

/-- A token before the operand: an `&`, or anything else. -/
inductive Tok where
  | amp
  | other
  deriving DecidableEq, Repr

/-- The run of `&` tokens ending just before the operand; the prefix is
listed most recent token first. -/
def runBefore : List Tok → Nat
  | .amp :: rest => runBefore rest + 1
  | _ => 0

/-- Rule 74e: the run ends in an `&` operator (`ends_in_address`). -/
def endsInAddress (pre : List Tok) : Bool := runBefore pre % 2 == 1

/-- The operand follows a logical AND (`ends_in_logical_and`). -/
def endsInLogicalAnd (pre : List Tok) : Bool :=
  2 ≤ runBefore pre && runBefore pre % 2 == 0

/-- The old unsound reading: `texts[k-1] == "&" && texts[k-2] != "&"`. -/
def exactlyOne (pre : List Tok) : Bool := runBefore pre == 1

/-- The old fail-closed and admitting reading: `previous == Some("&")`. -/
def anyRun (pre : List Tok) : Bool := 1 ≤ runBefore pre

theorem parity_is_munch (pre : List Tok) :
    endsInAddress pre = true ↔ (munch (runBefore pre)).getLast? = some .amp := by
  rw [← address_taken_agrees_with_munch]
  simp [endsInAddress, addressTaken]

/-- `p && &x`: the old reading saw no address, the parity sees it. -/
theorem exactly_one_misses_an_odd_run :
    exactlyOne [.amp, .amp, .amp, .other] = false ∧
      endsInAddress [.amp, .amp, .amp, .other] = true := by
  decide

theorem the_parity_keeps_every_single_address (pre : List Tok)
    (h : exactlyOne pre = true) : endsInAddress pre = true := by
  simp [exactlyOne] at h
  simp [endsInAddress, h]

theorem the_parity_drops_only_even_runs (pre : List Tok) :
    (anyRun pre = true ∧ endsInAddress pre = false) ↔ endsInLogicalAnd pre = true := by
  simp only [anyRun, endsInAddress, endsInLogicalAnd]
  generalize runBefore pre = n
  constructor
  · intro h
    simp at h
    simp
    omega
  · intro h
    simp at h
    simp
    omega

theorem logical_and_is_the_even_complement (pre : List Tok) (h : 1 ≤ runBefore pre) :
    endsInAddress pre = true ↔ endsInLogicalAnd pre = false := by
  simp only [endsInAddress, endsInLogicalAnd]
  generalize runBefore pre = n at h
  constructor
  · intro a
    simp at a
    simp
    omega
  · intro a
    simp at a
    simp
    omega

theorem the_parity_is_within_any_run (pre : List Tok) (h : endsInAddress pre = true) :
    anyRun pre = true := by
  simp [endsInAddress] at h
  simp [anyRun]
  omega

/-- A reader's reading of the run before an operand. -/
inductive Reading where
  | parity
  | exactlyOne
  | anyRun
  deriving DecidableEq, Repr

def Reading.decides : Reading → List Tok → Bool
  | .parity => endsInAddress
  | .exactlyOne => QCore.AmpersandRunParity.exactlyOne
  | .anyRun => QCore.AmpersandRunParity.anyRun

/-- The files whose readers decide address-taking through `ampersand_runs`
(`READERS`). `loopOwnerCoroPlugin`'s call index is a call reading, not an
address decision, and is stated separately (`a_callee_after_an_ampersand_is_called`). -/
inductive Reader where
  | legacyC
  | loopOwnerInstance
  | loopOwnerHolderOpaque
  | threadEntryCensus
  | legacyRetainedBy
  | legacyCallbackRoute
  | legacyListIteration
  | legacyReferenceResult
  | indexedCollection
  | aggregateStore
  | registrationById
  | typedFieldAccess
  | borrowedActual
  | legacyRetentionGuard
  | legacyReferenceCount
  | legacySlotMoveBorrow
  | legacyRefusedOperands
  | legacyInPlaceCustody
  | legacyCallInstantiation
  deriving DecidableEq, Repr

/-- Each reader's reading at the base. -/
def baseReading : Reader → Reading
  | .legacyC => .parity
  | .loopOwnerInstance => .exactlyOne
  | .loopOwnerHolderOpaque => .exactlyOne
  | .threadEntryCensus => .exactlyOne
  | _ => .anyRun

/-- Each reader's reading at the tip. -/
def tipReading : Reader → Reading := fun _ => .parity

theorem every_reader_reads_the_parity (r : Reader) : tipReading r = .parity := rfl

/-- Fixing an unsound reader only adds addresses: whatever it saw before it
still sees. -/
theorem an_unsound_fix_only_adds (r : Reader) (h : baseReading r = .exactlyOne)
    (pre : List Tok) (seen : (baseReading r).decides pre = true) :
    (tipReading r).decides pre = true := by
  rw [h] at seen
  simp only [Reading.decides, tipReading] at seen ⊢
  exact the_parity_keeps_every_single_address pre seen

/-- Changing a fail-closed or admitting reader drops exactly the even runs:
what it no longer sees as an address is a logical AND. -/
theorem a_fail_closed_change_only_drops_even_runs (r : Reader) (h : baseReading r = .anyRun)
    (pre : List Tok) :
    ((baseReading r).decides pre = true ∧ (tipReading r).decides pre = false) ↔
      endsInLogicalAnd pre = true := by
  rw [h]
  simp only [Reading.decides, tipReading]
  exact the_parity_drops_only_even_runs pre

theorem a_fail_closed_change_adds_nothing (r : Reader) (h : baseReading r = .anyRun)
    (pre : List Tok) (seen : (tipReading r).decides pre = true) :
    (baseReading r).decides pre = true := by
  rw [h]
  simp only [Reading.decides, tipReading] at seen ⊢
  exact the_parity_is_within_any_run pre seen

/-- The call index: a callee after any run of `&` is called, because `& f (`
is never an address (a call's result is not an lvalue). The old index
dropped it after any run, including `x && f (`. -/
def oldCalled (pre : List Tok) : Bool := !anyRun pre

def called (_pre : List Tok) : Bool := true

theorem a_callee_after_an_ampersand_is_called :
    oldCalled [.amp, .amp, .other] = false ∧ called [.amp, .amp, .other] = true := by
  decide

theorem the_call_index_only_adds (pre : List Tok) :
    oldCalled pre = true → called pre = true := fun _ => rfl

/-- The vectors `ampersand_runs::DECISION_VECTORS` restates: the prefix before
the operand (most recent first), the address reading, the logical-AND
reading. -/
def decisionVectors : List (List Tok × Bool × Bool) :=
  [ ([.amp, .amp, .amp, .other], true, false),        -- `p & & & q`
    ([.amp, .amp, .other], false, true),              -- `p & & q`
    ([.amp, .other], true, false),                    -- `p & q`
    ([.amp, .amp, .amp], true, false),                -- `& & & q`
    ([.amp], true, false),                            -- `& ( q )`
    ([.amp, .amp, .other], false, true),              -- `a & & q`
    ([.other], false, false),                         -- `= q`
    ([.amp, .amp], false, true),                      -- `& & label`
    ([.amp, .amp, .amp, .amp, .other], false, true) ] -- `p & & & & q`

theorem decision_vectors_hold :
    decisionVectors.all
        (fun (p, a, l) => endsInAddress p == a && endsInLogicalAnd p == l) = true := by
  decide

end QCore.AmpersandRunParity
