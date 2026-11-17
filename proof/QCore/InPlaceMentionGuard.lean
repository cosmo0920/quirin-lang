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

import QCore.ResourceKinds

/-!
# Every mention of a tracked in-place object, on every route (card CG25-VOID-CAST-GUARD-ESCAPE)

Gateway rule 11v (6): `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`
(`InPlaceNames::body_mentions`, `verify_mentions`) and its hook after
`verify_cfg` in `legacy_c.rs`.

Rule 11v (4)'s statement guard reads only a statement's first
classification. A statement whose first classification was already a refusal
skipped it, and a later route admitted a lowering that read tracked storage
as data. Examples: `(void) re.used;` through the cast route,
`if (re.used)` through a condition reader, `return (int) re.used;`. D3's
`(void) pthread_getspecific(key)` was the same gap.

The repair does not trust any route. The body's mentions of a tracked object
are read once off the token stream, and the function is admitted only when:

* every mention is **admitted**: the object's declarator, a `sizeof`
  operand, or a custody actual at a position of the object's own kind;
* every custody actual is **carried** by at least as many call nodes as the
  body spells it.

Otherwise a refusal is added on every path (a new entry node).

## What is proved

* Whatever route lowered the body, a `Safe` verdict implies the guard
  passed, so every mention was admitted and every custody actual carried
  (`every_admitted_lowering_passed_the_guard`, and its two projections).
* A stray mention is refused on every route
  (`a_stray_mention_is_refused_on_every_route`). So is a custody actual that
  no node carries (`an_uncarried_custody_actual_is_refused`).
* Where the guard passes, the lowering is CG25's in-place reading unchanged
  (`a_passing_body_is_cg25`).
* The census shapes (`shapeAdmissions`, bound to the Rust fixtures by
  `crates/qcore-c-gateway/tests/in_place_mention_guard.rs`) classify as
  stated. D3's getter and the member reads are refused.
  `rd_regex_match`'s five mentions pass (`the_regex_match_mentions_pass`).

## What this is not

Which token is which mention is read lexically by the Rust code, under trust
boundary 28's faithful-extraction premise; this module assumes that
classification. No route's own lowering is proved correct here. That is not
needed: the theorem quantifies over every route.
-/

namespace QCore.InPlaceMentionGuard

open QCore.KindedCustody
open QCore.ResourceKinds

/-- How one mention of a tracked object is spelled in the body. -/
inductive Mention where
  /-- The object's declarator (`regex_t re;`). -/
  | declarator
  /-- `sizeof re`, `sizeof (re)`: not evaluated (C11 6.5.3.4p2). -/
  | sizeofOperand
  /-- A whole `&re` actual (or `re` at a by-value finaliser of a by-value
  kind) of a declared callee `callee`, at position `pos`, of kind `k`. -/
  | custody (callee pos : Nat) (k : Kind)
  /-- Every other spelling: a member read or write, a copy, an operand of a
  cast, comma, conditional or statement expression, a condition, a return,
  an unkinded position. -/
  | other
  deriving DecidableEq, Repr

/-- Whether a mention of an object of kind `k` is admitted. -/
def admitted (k : Kind) : Mention → Bool
  | .declarator => true
  | .sizeofOperand => true
  | .custody _ _ k' => decide (k' = k)
  | .other => false

/-- The custody actuals of a body, by callee and position. -/
def custodyKeys (ms : List Mention) : List (Nat × Nat) :=
  ms.filterMap fun
    | .custody callee pos _ => some (callee, pos)
    | _ => none

/-- What any route produced: the call nodes that carry the object's place
(by callee and position) and the program the transfer runs. -/
structure Lowering where
  carried : List (Nat × Nat)
  program : List IInstr

/-- A route: anything at all that lowers a body. -/
abbrev Route := List Mention → Lowering

def strayFree (k : Kind) (ms : List Mention) : Bool :=
  ms.all (admitted k)

def allCarried (ms : List Mention) (l : Lowering) : Bool :=
  (custodyKeys ms).all fun key => (custodyKeys ms).count key ≤ l.carried.count key

/-- Rule 11v (6). -/
def passes (k : Kind) (ms : List Mention) (l : Lowering) : Bool :=
  strayFree k ms && allCarried ms l

/-- The gateway over one body: the route's lowering, with a refusal on every
path unless the guard passes. -/
def gate (k : Kind) (s : IState) (ms : List Mention) (l : Lowering) : IState :=
  let t := runI s l.program
  if passes k ms l then t else { t with diagnostics := .unknown :: t.diagnostics }

theorem a_failing_body_is_refused (k : Kind) (s : IState) (ms : List Mention) (l : Lowering)
    (h : passes k ms l = false) :
    .unknown ∈ (gate k s ms l).diagnostics := by
  simp [gate, h]

/-- **Every admitted lowering passed the guard**, whichever route produced it. -/
theorem every_admitted_lowering_passed_the_guard (route : Route) (k : Kind) (s : IState)
    (ms : List Mention) (h : classify (gate k s ms (route ms)).diagnostics = .safe) :
    passes k ms (route ms) = true := by
  cases hp : passes k ms (route ms)
  · exact absurd h (a_refusal_is_never_safe _ (a_failing_body_is_refused k s ms _ hp))
  · rfl

theorem a_safe_body_mentions_only_admitted_forms (route : Route) (k : Kind) (s : IState)
    (ms : List Mention) (h : classify (gate k s ms (route ms)).diagnostics = .safe) :
    ∀ m ∈ ms, admitted k m = true := by
  have hp := every_admitted_lowering_passed_the_guard route k s ms h
  simp only [passes, Bool.and_eq_true, strayFree, List.all_eq_true] at hp
  exact hp.1

theorem a_safe_body_carries_every_custody_actual (route : Route) (k : Kind) (s : IState)
    (ms : List Mention) (h : classify (gate k s ms (route ms)).diagnostics = .safe) :
    ∀ key ∈ custodyKeys ms, (custodyKeys ms).count key ≤ (route ms).carried.count key := by
  have hp := every_admitted_lowering_passed_the_guard route k s ms h
  simp only [passes, Bool.and_eq_true, allCarried, List.all_eq_true, decide_eq_true_eq] at hp
  exact hp.2

/-- A stray mention is refused, on every route. -/
theorem a_stray_mention_is_refused_on_every_route (route : Route) (k : Kind) (s : IState)
    (ms : List Mention) (m : Mention) (hm : m ∈ ms) (hs : admitted k m = false) :
    classify (gate k s ms (route ms)).diagnostics ≠ .safe := by
  intro h
  have := a_safe_body_mentions_only_admitted_forms route k s ms h m hm
  rw [hs] at this
  exact Bool.false_ne_true this

/-- A custody actual no node carries is refused. -/
theorem an_uncarried_custody_actual_is_refused (k : Kind) (s : IState) (ms : List Mention)
    (l : Lowering) (key : Nat × Nat) (hk : key ∈ custodyKeys ms) (hc : l.carried.count key = 0) :
    classify (gate k s ms l).diagnostics ≠ .safe := by
  intro h
  have := a_safe_body_carries_every_custody_actual (fun _ => l) k s ms h key hk
  rw [hc] at this
  have hpos : 0 < (custodyKeys ms).count key := List.count_pos_iff.mpr hk
  omega

/-- Where the guard passes, the gateway is CG25's in-place reading unchanged. -/
theorem a_passing_body_is_cg25 (k : Kind) (s : IState) (ms : List Mention) (l : Lowering)
    (h : passes k ms l = true) : gate k s ms l = runI s l.program := by
  simp [gate, h]

/-- A body that names no tracked object passes with any lowering. -/
theorem a_body_with_no_mention_passes (k : Kind) (l : Lowering) : passes k [] l = true := by
  simp [passes, strayFree, allCarried, custodyKeys]

/-! ### The census shapes -/

/-- `SHAPE_ADMISSIONS`: each census shape's mention of `re` (kind `regex`)
and whether it is admitted. The Rust fixtures assert the stray refusal
exactly where `false`. Callees: 1 `k_regcomp`, 2 `k_regexec`, 3 `k_regfree`,
4 `k_peek` (unkinded). -/
def shapeAdmissions : List (String × Bool) := [
  ("(void) re.used;", false),
  ("(void) (re.used = 0);", false),
  ("(void) k_take(re.used);", false),
  ("(void) (re.used ? 1 : 0);", false),
  ("if (re.used) t = 1;", false),
  ("while (re.used) { n = 0; break; }", false),
  ("return (int) re.used;", false),
  ("t = k_regexec(&re, s) + (int) re.used;", false),
  ("(void) k_peek(&re);", false),
  ("n = sizeof re;", true),
  ("k_regexec(&re, s);", true),
  ("t = k_regexec(&re, s) != 1;", true),
  ("if (k_regexec(&re, s) == 0) t = 1;", true)
]

/-- The mentions those shapes spell, in the same order (the mixed shape
spells two). -/
def shapeMentions : List (List Mention) := [
  [.other],
  [.other],
  [.other],
  [.other],
  [.other],
  [.other],
  [.other],
  [.custody 2 0 .regex, .other],
  [.custody 4 0 .heap],
  [.sizeofOperand],
  [.custody 2 0 .regex],
  [.custody 2 0 .regex],
  [.custody 2 0 .regex]
]

theorem shape_admissions_are_the_model :
    shapeAdmissions.map Prod.snd = shapeMentions.map (strayFree .regex) := by
  decide

/-- D3's escape: `(void) pthread_getspecific(key)` names the key at a
position of no kind, so it is a stray mention of a `thread_key` object. -/
theorem the_void_cast_getter_is_refused (route : Route) (s : IState) :
    classify (gate .threadKey s [.declarator, .custody 5 0 .threadKey, .other]
      (route [.declarator, .custody 5 0 .threadKey, .other])).diagnostics ≠ .safe :=
  a_stray_mention_is_refused_on_every_route route .threadKey s _ .other (by simp) rfl

/-- `rd_regex_match`: the declarator and four `regex`-position actuals
(`regcomp`, `regerror` at position 1, `regexec`, `regfree`), each carried. -/
def regexMatchMentions : List Mention :=
  [.declarator, .custody 1 0 .regex, .custody 6 1 .regex, .custody 2 0 .regex,
   .custody 3 0 .regex]

theorem the_regex_match_mentions_pass :
    passes .regex regexMatchMentions ⟨[(1, 0), (6, 1), (2, 0), (3, 0)], []⟩ = true := by
  decide

/-- The same body with `(void) re.re_nsub;` after the free (the census mutant)
is refused, whatever the route. -/
theorem the_regex_match_mutant_is_refused (route : Route) (s : IState) :
    classify (gate .regex s (regexMatchMentions ++ [.other])
      (route (regexMatchMentions ++ [.other]))).diagnostics ≠ .safe :=
  a_stray_mention_is_refused_on_every_route route .regex s _ .other (by simp) rfl

end QCore.InPlaceMentionGuard
