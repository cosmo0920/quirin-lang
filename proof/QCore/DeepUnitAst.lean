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

import QCore.ContractDependencies

/-!
# A unit index nesting beyond the default JSON limit (card RDHTTP-HEADER-SUMMARY-GAP, rules 13o–13p)

The abstract rules implemented by
`crates/qcore-c-gateway/src/deep_unit_ast.rs` and its hook in
`typed_access_path::AstIndex::from_json`.

* **Depth.** A JSON text's nesting depth is the greatest number of `[` and `{`
  open at once outside strings (a `\` escapes the next character inside a
  string), as `serde_json` counts it (`depth`).
* **Default.** `serde_json`'s default parse refuses a document nesting
  `defaultLimit` (128) or more; the unit then keeps no index.
* **Rule 13o.** A document the default parse refused is read again without the
  limit when it nests at most `admittedDepth` (256). The text and the parser are
  the same, so the index is the document (`index`).
* **Rule 13p.** Beyond the bound the unit keeps no index, as before, and a body
  it examined with no evidence stays `unvalidated` in rule 13g's fold
  (`QCore.ContractDependencies.standing`): the fold never skips a unit.

Theorems:
* `an_index_read_before_is_read_after`, `the_index_is_the_document`,
  `beyond_the_bound_there_is_no_index`, `only_the_deep_band_changes`;
* `a_unit_without_an_index_keeps_the_declaration_unvalidated`,
  `the_fold_never_skips_a_unit`;
* the shared vectors `depth_vectors_hold`, bound to the Rust by the unit test
  `deep_unit_ast::tests::depth_vectors_hold`.

## What this is not

A proof that `serde_json` parses a document identically with and without its
recursion limit, or that Clang's JSON is faithful (trust boundary 28). The
first is the library's documented contract for `disable_recursion_limit` (the
limit is a stack guard and changes no accepted value) and is checked on
nested documents by `a_deep_document_within_the_bound_is_the_same_value`.
-/

namespace QCore.DeepUnitAst

open QCore.ContractDependencies (Outcome Standing standing)

/-- `serde_json`'s default recursion limit. -/
def defaultLimit : Nat := 128

/-- Rule 13o's bound: twice the default. -/
def admittedDepth : Nat := 2 * defaultLimit

/-- The scan state: open containers, the deepest seen, inside a string, after
a backslash inside a string. -/
structure Scan where
  open_ : Nat
  deepest : Nat
  inString : Bool
  escaped : Bool
  deriving DecidableEq, Repr

def step (s : Scan) (c : Char) : Scan :=
  if s.inString then
    if s.escaped then { s with escaped := false }
    else if c = '\\' then { s with escaped := true }
    else if c = '"' then { s with inString := false }
    else s
  else if c = '"' then { s with inString := true }
  else if c = '[' ∨ c = '{' then
    { s with open_ := s.open_ + 1, deepest := max s.deepest (s.open_ + 1) }
  else if c = ']' ∨ c = '}' then { s with open_ := s.open_ - 1 }
  else s

/-- The nesting depth of a JSON text. -/
def depth (text : String) : Nat :=
  (text.toList.foldl step ⟨0, 0, false, false⟩).deepest

/-- Whether the default parse reads a well-formed document of depth `d`. -/
def defaultReads (d : Nat) : Bool := decide (d < defaultLimit)

/-- Whether the unit index is read after rule 13o. -/
def reads (d : Nat) : Bool := defaultReads d || decide (d ≤ admittedDepth)

/-- The unit index of a well-formed document `doc` of depth `d`: before the
card (`after = false`) and after it. -/
def index {α : Type} (after : Bool) (doc : α) (d : Nat) : Option α :=
  if (if after then reads d else defaultReads d) then some doc else none

/-- Rule 13o never removes an index the default parse produced. -/
theorem an_index_read_before_is_read_after {α : Type} {doc x : α} {d : Nat}
    (h : index false doc d = some x) : index true doc d = some x := by
  unfold index at *
  unfold reads
  by_cases r : defaultReads d = true
  · simp_all
  · simp_all

/-- The index rule 13o reads is the document itself. -/
theorem the_index_is_the_document {α : Type} {after : Bool} {doc x : α} {d : Nat}
    (h : index after doc d = some x) : x = doc := by
  unfold index at h
  split at h <;> simp_all

/-- Rule 13p: beyond the bound the unit keeps no index. -/
theorem beyond_the_bound_there_is_no_index {α : Type} {after : Bool} {doc : α} {d : Nat}
    (beyond : admittedDepth < d) : index after doc d = none := by
  have nd : ¬ d < defaultLimit := by
    unfold admittedDepth at beyond; unfold defaultLimit at *; omega
  have na : ¬ d ≤ admittedDepth := by omega
  cases after <;> simp [index, reads, defaultReads, nd, na]

/-- The card changes the index exactly for the depths from the default limit
to the bound. -/
theorem only_the_deep_band_changes {α : Type} (doc : α) (d : Nat) :
    index false doc d ≠ index true doc d ↔ defaultLimit ≤ d ∧ d ≤ admittedDepth := by
  unfold index reads defaultReads
  by_cases lt : d < defaultLimit
  · have : d ≤ admittedDepth := by unfold admittedDepth; omega
    simp [lt]; omega
  · by_cases le : d ≤ admittedDepth
    · simp [lt, le]; omega
    · simp [lt, le]

/-- A unit with no index records no typed reading of a header body; examined
with no evidence and no failure it is `unvalidated`, and the declaration is
never `validated` (rule 13g). -/
theorem a_unit_without_an_index_keeps_the_declaration_unvalidated
    {held : Bool} {others : List Outcome} :
    standing held (QCore.ContractDependencies.unitOutcome false false :: others) ≠
      .validated := by
  apply QCore.ContractDependencies.an_unvalidated_body_is_never_validated
  simp [QCore.ContractDependencies.unitOutcome]

/-- The fold is never told to skip a unit: dropping an unvalidated unit from
the outcomes could only raise the standing, and the card does not. Stated as:
every outcome list containing an `unvalidated` unit stays short of
`validated`, whatever the other units say. -/
theorem the_fold_never_skips_a_unit {held : Bool} (before after : List Outcome) :
    standing held (before ++ .unvalidated :: after) ≠ .validated := by
  apply QCore.ContractDependencies.an_unvalidated_body_is_never_validated
  simp

/-- Shared vectors with `deep_unit_ast::DEPTH_VECTORS`: a text, its depth,
whether the default parse reads it, whether rule 13o reads it. -/
def depthVectors : List (String × Nat × Bool × Bool) := [
  ("1", 0, true, true),
  ("[]", 1, true, true),
  ("{\"a\":[1,{\"b\":[]}]}", 4, true, true),
  ("\"[[[{\"", 0, true, true),
  ("[\"\\\"[\",[]]", 2, true, true),
  ("{\"s\":\"\\\\\",\"t\":[[]]}", 3, true, true)
]

theorem depth_vectors_hold :
    depthVectors.all (fun (text, d, dflt, after) =>
      depth text == d && defaultReads d == dflt && reads d == after) = true := by
  decide

/-- The band's edges: 127 is read by default, 128 and 256 only by rule 13o,
257 by neither. -/
theorem band_edges :
    defaultReads 127 = true ∧ defaultReads 128 = false ∧ reads 128 = true ∧
      reads 256 = true ∧ reads 257 = false := by
  decide

end QCore.DeepUnitAst
