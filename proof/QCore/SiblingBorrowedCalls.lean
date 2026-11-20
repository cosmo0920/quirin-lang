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

import QCore.ResidualAccessReadings

/-!
# Several sibling `Borrowed`-only calls in one scalar statement (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3, gateway rule 67)

Rule 14a read a scalar statement whose value holds **exactly one** call
whose contract has a `None` result and every position `Borrowed`, and said
"do not admit two calls" (`QCore.ResidualAccessReadings.two_calls_are_never_read`,
superseded by `scalarCallAdmitted`'s amendment there). Rule 67 lifts that limit for
sibling calls (no call inside another's callee or actuals), each of which
takes no custody: C leaves the order of the calls and of the other operands
unspecified (C11 6.5p2, 6.5.2.2p10), and this module proves that the
custody state after any sequence of such calls, in any order and with the
statement's reads anywhere between them, is the state before them, so every
read is decided alike whatever order the program takes.

The near-miss is the reason the premise is needed: a releasing call beside a
borrowed one decides a read differently on either side of it.

## What this is not

A model of the classifier or of Clang. The Rust decision
(`scalar_call_admitted`) is bound to `QCore.ResidualAccessReadings.scalarCallVectors`;
that each sibling call is lowered alone with W44's reads is the gateway's,
and faithful extraction is trust boundary 28.
-/

namespace QCore.SiblingBorrowedCalls

open QCore.ResidualAccessReadings

/-- The custody state after a sequence of calls, each given by its positions'
modes, taken in list order. -/
def effects : List (List Mode) → State → State
  | [], s => s
  | ms :: rest, s => effects rest (effect ms s)

/-- Every call of the sequence takes no custody. -/
def everyCallBorrowed (cs : List (List Mode)) : Bool := cs.all allBorrowed

theorem a_borrowed_sequence_changes_no_custody (cs : List (List Mode)) (s : State)
    (h : everyCallBorrowed cs = true) : effects cs s = s := by
  induction cs generalizing s with
  | nil => rfl
  | cons ms rest ih =>
      simp only [everyCallBorrowed, List.all_cons, Bool.and_eq_true] at h
      simp only [effects]
      rw [a_borrowed_call_changes_no_custody ms s h.1]
      exact ih s h.2

/-- Any permutation of the calls (C's unspecified order) leaves the same state. -/
theorem every_order_leaves_the_same_state (cs ds : List (List Mode)) (s : State)
    (hp : cs.Perm ds) (h : everyCallBorrowed cs = true) :
    effects cs s = effects ds s := by
  have hd : everyCallBorrowed ds = true := by
    simp only [everyCallBorrowed, List.all_eq_true] at h ⊢
    intro x hx
    exact h x (hp.mem_iff.mpr hx)
  rw [a_borrowed_sequence_changes_no_custody cs s h,
    a_borrowed_sequence_changes_no_custody ds s hd]

/-- A read placed anywhere in the sequence (after any prefix of the calls) is
decided as it is before every call. -/
theorem a_read_between_the_calls_is_decided_alike (cs : List (List Mode)) (k i : Nat)
    (s : State) (h : everyCallBorrowed cs = true) :
    readResolves i (effects (cs.take k) s) = readResolves i s := by
  have ht : everyCallBorrowed (cs.take k) = true := by
    simp only [everyCallBorrowed, List.all_eq_true] at h ⊢
    intro x hx
    exact h x (List.mem_of_mem_take hx)
  rw [a_borrowed_sequence_changes_no_custody _ s ht]

/-- The near-miss: with a releasing call beside a borrowed one, the read of
the released identity is decided differently before and after the pair, so
the order C leaves open would matter. -/
theorem a_releasing_sibling_does_not_commute :
    readResolves 0 (effects [[.borrowed], [.released]] [.live]) ≠
      readResolves 0 [.live] := by decide

/-- And the near-miss is excluded by the premise. -/
theorem a_releasing_sibling_is_never_admitted :
    everyCallBorrowed [[.borrowed], [.released]] = false := by decide

/-- Rule 67's decision on the census's terms is rule 14a's amended decision
(`scalarCallAdmitted`): any positive number of calls, each `None`-result
and all-`Borrowed`. -/
theorem several_calls_are_read (n : Nat) (h : 1 ≤ n) :
    scalarCallAdmitted n true true = true := by
  simp only [scalarCallAdmitted, Bool.and_true, decide_eq_true_eq]
  exact h

theorem no_call_is_never_read (n b : Bool) : scalarCallAdmitted 0 n b = false := by
  simp [scalarCallAdmitted]

end QCore.SiblingBorrowedCalls
