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
# A bare member read through an untracked root keeps family S's base (card RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT, rule 74v)

`return_expression_instruction` asks card B01's opaque member read before
family S. It classifies the synthetic `PLACE = E ;` and admits an
`OpaqueValue` into the return place when `E` has exactly one member
selection. The classifier produces that `OpaqueValue` for exactly one
admitted access over a root `UntrackedBases::never_bound` reports (C02-P
premise 4). So the value carries no tracked identity, and the exit escapes
nothing. But the `OpaqueValue` carried **no obligation** for the root that
`->` dereferences. `return p->n;` through a null local `p` was `Safe`, while
family S gives `return p->n + 1;` the base `p` and refuses it
(`unknown_alias:p`) (`old_misses_the_base`).

Rule 74v reads what the admitted expression dereferences
(`memberReadBase`). `ROOT - > FIELD` lowers to family S's read node, with the
root as its only base. An expression with no `>`, `[`, `]`, `*` or `&` (a `.`
read of a record value) keeps the `OpaqueValue`. Every other spelling
declines, and the later routes decide it.

* Before the change, B01 admitted every expression it read. So whatever the
  tip admits, the base admitted (`the_fix_only_refuses_more`). Expressions
  B01 does not read are untouched (`outside_b01_unchanged`).
* A value read is unchanged (`a_value_read_is_unchanged`). A root read is
  admitted exactly when the base obligation discharges
  (`a_root_read_needs_its_base`), which is family S's verdict for the same
  spelling (`the_bare_read_gets_family_s_verdict`). An unbound or non-null
  untracked root discharges it and keeps its admission
  (`a_discharged_root_keeps_its_admission`).
* A root read names exactly the token before `- >`
  (`the_root_is_the_dereferenced_name`), and a value read contains no
  dereferencing token (`a_value_read_dereferences_nothing`).

## What this is not

This is not a model of the classifier or of the flow check. Each token's
class is the Rust reader's output (trust boundary 28), bound by
`readVectors`. The base obligation is abstracted as a predicate `ok` that the
flow check decides. It holds for an unbound root or a non-null untracked
binding. It fails with `unknown_alias` for a possibly-null or unknown one.
-/

namespace QCore.ReturnStrictMemberNull

/-- One token of the admitted expression, as the Rust reader classes it. -/
inductive Tok where
  /-- A name (`is_identifier`). -/
  | ident (name : String)
  /-- `-`. -/
  | minus
  /-- `>`. -/
  | gt
  /-- `[` or `]`. -/
  | bracket
  /-- `*`. -/
  | star
  /-- `&`. -/
  | amp
  /-- `.`. -/
  | dot
  /-- Anything else (a number, `(`, `)`, `+`, …). -/
  | other
  deriving DecidableEq, Repr

/-- What the admitted read dereferences. -/
inductive Read where
  /-- Nothing: the `OpaqueValue` is kept. -/
  | value
  /-- This root: family S's read node with it as the only base. -/
  | root (name : String)
  /-- Declined: the later routes decide the expression. -/
  | declined
  deriving DecidableEq, Repr

/-- A token after which, or at which, the expression may dereference. -/
def derefs : Tok → Bool
  | .gt | .bracket | .star | .amp => true
  | _ => false

/-- Rule 74v's `member_read_base`. -/
def memberReadBase : List Tok → Read
  | [.ident r, .minus, .gt, .ident _] => .root r
  | ts => if ts.any derefs then .declined else .value

/-- The base: B01 admitted whatever it read, with no obligation. -/
def baseAdmits (b01 later : Bool) : Bool :=
  if b01 then true else later

/-- The tip. `ok` is the base obligation's discharge, and `later` is the verdict
of the routes after B01. -/
def tipAdmits (b01 : Bool) (ts : List Tok) (ok : String → Bool) (later : Bool) : Bool :=
  if b01 then
    match memberReadBase ts with
    | .value => true
    | .root r => ok r
    | .declined => later
  else later

/-- Family S's verdict for a read whose only base is the untracked root `r`. -/
def familySAdmits (r : String) (ok : String → Bool) : Bool := ok r

theorem the_fix_only_refuses_more (b01 later : Bool) (ts : List Tok) (ok : String → Bool)
    (h : tipAdmits b01 ts ok later = true) : baseAdmits b01 later = true := by
  cases b01 <;> simp_all [tipAdmits, baseAdmits]

theorem outside_b01_unchanged (ts : List Tok) (ok : String → Bool) (later : Bool) :
    tipAdmits false ts ok later = baseAdmits false later := by
  simp [tipAdmits, baseAdmits]

theorem a_value_read_is_unchanged (ts : List Tok) (ok : String → Bool) (later : Bool)
    (h : memberReadBase ts = .value) : tipAdmits true ts ok later = baseAdmits true later := by
  simp [tipAdmits, baseAdmits, h]

theorem a_root_read_needs_its_base (ts : List Tok) (r : String) (ok : String → Bool)
    (later : Bool) (h : memberReadBase ts = .root r) :
    tipAdmits true ts ok later = ok r := by
  simp [tipAdmits, h]

theorem a_declined_read_defers (ts : List Tok) (ok : String → Bool) (later : Bool)
    (h : memberReadBase ts = .declined) : tipAdmits true ts ok later = later := by
  simp [tipAdmits, h]

theorem the_bare_read_gets_family_s_verdict (r f : String) (ok : String → Bool)
    (later : Bool) :
    tipAdmits true [.ident r, .minus, .gt, .ident f] ok later = familySAdmits r ok := by
  simp [tipAdmits, memberReadBase, familySAdmits]

theorem a_discharged_root_keeps_its_admission (r f : String) (ok : String → Bool)
    (later : Bool) (h : ok r = true) :
    tipAdmits true [.ident r, .minus, .gt, .ident f] ok later = true := by
  simp [tipAdmits, memberReadBase, h]

theorem the_root_is_the_dereferenced_name (ts : List Tok) (r : String)
    (h : memberReadBase ts = .root r) : ∃ f, ts = [.ident r, .minus, .gt, .ident f] := by
  unfold memberReadBase at h
  split at h
  · cases h
    exact ⟨_, rfl⟩
  · split at h <;> cases h

theorem a_value_read_dereferences_nothing (ts : List Tok)
    (h : memberReadBase ts = .value) : ts.any derefs = false := by
  unfold memberReadBase at h
  split at h
  · cases h
  · split at h
    · cases h
    · simp_all

/-- `return p->n;` through a null `p`: admitted at the base, refused at the tip. -/
theorem old_misses_the_base :
    baseAdmits true false = true ∧
      tipAdmits true [.ident "p", .minus, .gt, .ident "n"] (fun _ => false) false = false := by
  decide

/-- The rows the Rust reader is pinned to: a spelling, its tokens and its
reading. The spelling is what the unit test hands `member_read_base`. -/
def readVectors : List (String × List Tok × Read) := [
  ("p - > n", [.ident "p", .minus, .gt, .ident "n"], .root "p"),
  ("ctx - > next", [.ident "ctx", .minus, .gt, .ident "next"], .root "ctx"),
  ("s . n", [.ident "s", .dot, .ident "n"], .value),
  ("p - > q - > n", [.ident "p", .minus, .gt, .ident "q", .minus, .gt, .ident "n"], .declined),
  ("( p ) - > n", [.other, .ident "p", .other, .minus, .gt, .ident "n"], .declined),
  ("p [ 0 ]", [.ident "p", .bracket, .other, .bracket], .declined),
  ("* p", [.star, .ident "p"], .declined),
  ("& p - > n", [.amp, .ident "p", .minus, .gt, .ident "n"], .declined),
  ("p - > n + 1", [.ident "p", .minus, .gt, .ident "n", .other, .other], .declined),
  ("p - > a . b", [.ident "p", .minus, .gt, .ident "a", .dot, .ident "b"], .declined),
  ("1 - > n", [.other, .minus, .gt, .ident "n"], .declined),
  ("p - >", [.ident "p", .minus, .gt], .declined)
]

def readVectorsAgree : Bool :=
  readVectors.all (fun (_, ts, expected) => memberReadBase ts == expected)

theorem read_vectors_hold : readVectorsAgree = true := by decide

end QCore.ReturnStrictMemberNull
