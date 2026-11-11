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

/-! Unit `E01-RAW-COND`: conditional-group brace balance in the raw token scan.

`docs/reports/remaining-e01-f1-raw/README.md` §3.2 recorded the residual this
unit closes. The raw scan behind `legacy_c_source_functions` is deliberately
**not** a preprocessor: `docs/reports/remaining-e01-f1/README.md` §1.4 makes
keeping *both* arms of a `#if … #else … #endif` group load-bearing, so a
definition guarded out by `#if 0` never leaves the audited denominator. But the
scan also *summed* the arms' braces, and `librdkafka/src/rdkafka.c`'s
`rd_kafka_dir_is_empty` opens one brace in each of two arms with a single `}`
after `#endif`, so the file-scope depth rose by one and stayed there — hiding
fifteen definitions.

The rule modelled here, implemented by `qcore_c_gateway::legacy_c`'s
`apply_conditional`/`function_names`:

> A group's resulting file-scope brace depth is **the greatest depth its arms
> reach**, each arm scanned from the depth the group began at, with a group that
> has no `#else` carrying an implicit empty arm that reaches the entry depth.

No controlling expression is read and no arm is ever selected. Two consequences
are the whole content of the unit, and both are proved below:

* **agreement is exactness** — when every arm reaches the same depth, that depth
  *is* the greatest, so the group is decided exactly whichever arm survives.
  This is the `rd_kafka_dir_is_empty` case;
* **disagreement is refused, never guessed** — the greatest depth dominates
  every arm selection, so the scan can only hide definitions, never invent one,
  and the disagreement is recorded so the enumeration's completeness claim is
  refused rather than silently over-claimed.

What is proved here:

* the per-group closing rule dominates every arm and is exact exactly when the
  arms agree, and a disagreement always sets the refusal flag;
* an ordinary token is the identity on this state, which is why the exhaustive
  domain below omits it;
* refusal is monotone: no later item ever clears it;
* every arm is scanned from the entry depth — the E01-F1 §1.4 monotonicity the
  raw scan rests on;
* the reduced `rd_kafka_dir_is_empty` shape is decided under both rules: the
  summing rule this unit replaces ends one level deep, the rule ends at zero;
* a shape where the summing rule produced a **phantom** — an arm that closes
  more than it opens lowered the depth below every arm selection, so a
  block-macro statement inside a body was read as a file-scope definition —
  is decided both ways too;
* **exhaustively over every item string of length at most four**: the scan's
  depth dominates every arm selection's depth, an unrefused scan is exact for
  every selection and has at least one, and a stream no selection semantics can
  read is always refused;
* **exhaustive agreement with the Rust implementation** over the same domain,
  tied by a checksum both sides compute independently: the Rust side renders
  each item string as C text, runs the real `legacy_c_source_scan`, and reads
  the depth back through probe definitions.

What is **not** proved here, and stays a Rust boundary: that the byte lexer
recognises exactly the directive spellings this model's `ifStart`, `alt`,
`otherwise` and `endIf` stand for (`crates/qcore-c-gateway/tests/e01_raw_cond.rs`
pins that behaviourally, and `QCore.RawDirectiveLexing` proves the logical-line
rule that decides whether a `#` is a directive at all), and that
`function_names` reads a depth-zero name as a definition. The exhaustive
theorems are bounded at length four; the longer shapes that matter are decided
individually. -/
namespace QCore.ConditionalGroupDepth

/-- One item of the raw scan's stream, abstracted to what moves file-scope brace
depth or group structure. -/
inductive Item where
  /-- `{`. -/
  | lbrace
  /-- `}`. -/
  | rbrace
  /-- Any other token. -/
  | other
  /-- `#if`, `#ifdef`, `#ifndef`. -/
  | ifStart
  /-- `#elif`, `#elifdef`, `#elifndef`. -/
  | alt
  /-- `#else`. -/
  | otherwise
  /-- `#endif`. -/
  | endIf
  deriving DecidableEq, Repr

/-- One open `#if … #endif` group while its arms are scanned. -/
structure Frame where
  /-- The file-scope depth the group began at; every arm is scanned from here. -/
  entry : Nat
  /-- The depth each already-scanned arm ended at. -/
  arms : List Nat
  /-- Whether an `#else` arm has been opened. -/
  sawElse : Bool
  deriving DecidableEq, Repr

/-- The scan state. The two refusal flags are the two named grounds
`ConditionalScanRefusal::ArmDepthDisagreement` and `::UnstructuredGroup`. -/
structure State where
  depth : Nat
  frames : List Frame
  disagreed : Bool
  unstructured : Bool
  deriving DecidableEq, Repr

/-- The state a scan starts in. -/
def initial : State := { depth := 0, frames := [], disagreed := false, unstructured := false }

/-- `#elif` and `#else`: end the current arm and start the next one from the
entry depth. An `#elif` or a second `#else` after an `#else` is a group this
scan cannot read. -/
def separator (s : State) (isElse : Bool) : State :=
  match s.frames with
  | [] => { s with unstructured := true }
  | f :: rest =>
      { depth := f.entry,
        frames := { f with arms := f.arms ++ [s.depth], sawElse := f.sawElse || isElse } :: rest,
        disagreed := s.disagreed,
        unstructured := s.unstructured || f.sawElse }

/-- Every depth an arm of this group may leave, including the implicit empty arm
of a group with no `#else`. -/
def armDepths (f : Frame) (final : Nat) : List Nat :=
  f.arms ++ [final] ++ (if f.sawElse then [] else [f.entry])

/-- The group's contribution: the greatest depth its arms reach. -/
def groupDepth : List Nat → Nat
  | [] => 0
  | d :: rest => max d (groupDepth rest)

/-- Whether every arm reached the same depth. -/
def armsAgree (depths : List Nat) : Bool := depths.all (fun d => d == depths.headD 0)

/-- `#endif`: close the group, take the greatest arm depth, and record a
disagreement if the arms did not all reach it. -/
def close (s : State) : State :=
  match s.frames with
  | [] => { s with unstructured := true }
  | f :: rest =>
      let depths := armDepths f s.depth
      { depth := groupDepth depths,
        frames := rest,
        disagreed := s.disagreed || !armsAgree depths,
        unstructured := s.unstructured }

/-- One item. -/
def step (s : State) : Item → State
  | .other => s
  | .lbrace => { s with depth := s.depth + 1 }
  | .rbrace => { s with depth := s.depth - 1 }
  | .ifStart =>
      { s with frames := { entry := s.depth, arms := [], sawElse := false } :: s.frames }
  | .alt => separator s false
  | .otherwise => separator s true
  | .endIf => close s

/-- End of input: a group that was never closed is a group this scan cannot
read. -/
def finish (s : State) : State :=
  if s.frames.isEmpty then s else { s with unstructured := true }

/-- The whole scan. -/
def scan (items : List Item) : State := finish (items.foldl step initial)

/-- Whether the scan established every conditional group exactly. -/
def refused (s : State) : Bool := s.disagreed || s.unstructured

/-! ## 1. The rule this unit replaces

The pre-unit scan had no group structure at all: a directive contributed no
token, so the braces of *every* arm moved the same depth. -/

/-- The pre-unit rule: conditional directives are invisible. -/
def stepSummed (s : Nat) : Item → Nat
  | .lbrace => s + 1
  | .rbrace => s - 1
  | _ => s

/-- The pre-unit scan's final depth. -/
def scanSummed (items : List Item) : Nat := items.foldl stepSummed 0

/-! ## 2. The per-group rule -/

/-- The group's depth dominates every arm's depth. This is what makes the scan a
*subset* of every arm selection's definitions: its depth is never lower than the
true one, so it can hide a definition but never invent one. -/
theorem groupDepth_dominates {depths : List Nat} {d : Nat} (h : d ∈ depths) :
    d ≤ groupDepth depths := by
  induction depths with
  | nil => cases h
  | cons head tail ih =>
      rcases List.mem_cons.mp h with rfl | rest
      · simp [groupDepth]
        omega
      · have := ih rest
        simp [groupDepth]
        omega

/-- A group whose arms all reach the same depth is decided **exactly**: the
greatest depth is that depth. No arm is selected and no conditional is
evaluated. -/
theorem agreeing_arms_are_exact {depths : List Nat} {d : Nat}
    (nonempty : depths ≠ []) (agree : armsAgree depths = true) (mem : d ∈ depths) :
    groupDepth depths = d := by
  have head : ∀ e ∈ depths, e = depths.headD 0 := by
    intro e he
    have := List.all_eq_true.mp agree e he
    exact of_decide_eq_true this
  have all_head : ∀ e ∈ depths, e = d := by
    intro e he
    rw [head e he, ← head d mem]
  have general : ∀ l : List Nat, (∀ e ∈ l, e = d) → l ≠ [] → groupDepth l = d := by
    intro l
    induction l with
    | nil => intro _ h; exact absurd rfl h
    | cons x xs ih =>
        intro hall _
        have hx : x = d := hall x (by simp)
        cases xs with
        | nil => simp [groupDepth, hx]
        | cons y ys =>
            have hrest : ∀ e ∈ y :: ys, e = d := by
              intro e he; exact hall e (by simp [he])
            have inner := ih hrest (by simp)
            simp [groupDepth] at inner ⊢
            omega
  exact general depths all_head nonempty

/-- Arms that do **not** agree always set the refusal flag: the scan never
guesses silently. -/
theorem disagreeing_arms_refuse (s : State) (f : Frame) (rest : List Frame)
    (frames : s.frames = f :: rest)
    (disagree : armsAgree (armDepths f s.depth) = false) :
    (close s).disagreed = true := by
  simp [close, frames, disagree]

/-- Arms that agree never set the refusal flag: agreement really is the
decided case, not merely an undetected one. -/
theorem agreeing_arms_do_not_refuse (s : State) (f : Frame) (rest : List Frame)
    (frames : s.frames = f :: rest) (undecided : s.disagreed = false)
    (agree : armsAgree (armDepths f s.depth) = true) :
    (close s).disagreed = false := by
  simp [close, frames, agree, undecided]

/-- The depth a group leaves is a depth one of its arms actually reaches: the
scan never invents a depth, it only declines to choose among the ones written. -/
theorem groupDepth_is_reached {depths : List Nat} (nonempty : depths ≠ []) :
    groupDepth depths ∈ depths := by
  induction depths with
  | nil => exact absurd rfl nonempty
  | cons head tail ih =>
      cases tail with
      | nil => simp [groupDepth]
      | cons second rest =>
          have inner := ih (by simp)
          have unfold : groupDepth (head :: second :: rest)
              = max head (groupDepth (second :: rest)) := rfl
          rw [unfold]
          rcases Nat.le_total head (groupDepth (second :: rest)) with h | h
          · have e : max head (groupDepth (second :: rest)) = groupDepth (second :: rest) := by
              omega
            rw [e]
            exact List.mem_cons_of_mem head inner
          · have e : max head (groupDepth (second :: rest)) = head := by omega
            rw [e]
            simp

/-- A group with no `#else` carries the implicit empty arm: every controlling
expression may be false, and that outcome leaves the entry depth. -/
theorem a_group_without_else_carries_the_entry_depth (f : Frame) (final : Nat)
    (noElse : f.sawElse = false) : f.entry ∈ armDepths f final := by
  simp [armDepths, noElse]

/-- The arm that ends at `#endif` is one of the arms measured. -/
theorem the_last_arm_is_measured (f : Frame) (final : Nat) : final ∈ armDepths f final := by
  simp [armDepths]

/-- An ordinary token is the identity on the whole state, which is why the
exhaustive domain below omits it. -/
theorem other_is_the_identity (s : State) : step s .other = s := rfl

/-- Every arm is scanned from the depth the group began at: a separator restores
the entry depth. This is E01-F1 §1.4's monotonicity — a definition written in
*any* arm is seen at file scope if that arm would put it there. -/
theorem a_separator_restores_the_entry_depth (s : State) (f : Frame) (rest : List Frame)
    (frames : s.frames = f :: rest) (isElse : Bool) :
    (separator s isElse).depth = f.entry := by
  simp [separator, frames]

/-- A group opens its first arm at the depth the group began at. -/
theorem a_group_opens_at_the_entry_depth (s : State) :
    (step s .ifStart).depth = s.depth := rfl

/-- Refusal is monotone: no later item clears a recorded ground. -/
theorem refusal_is_monotone (s : State) (i : Item) :
    (s.disagreed = true → (step s i).disagreed = true)
      ∧ (s.unstructured = true → (step s i).unstructured = true) := by
  cases i <;>
    first
      | exact ⟨fun h => h, fun h => h⟩
      | (constructor <;> intro h <;>
          simp [step, separator, close] <;>
          cases s.frames <;> simp [h])

/-! ## 3. Selection semantics

What a preprocessor would produce: one arm per group, or nothing when a group
has no `#else` and every controlling expression is false. The scan is compared
against *all* of these at once. -/

/-- A token stream with every conditional resolved. -/
inductive Plain where
  | lbrace
  | rbrace
  | other
  deriving DecidableEq, Repr

/-- File-scope brace depth of a resolved stream, saturating at zero exactly as
the Rust scan's `saturating_sub` does. -/
def plainDepth : List Plain → Nat → Nat
  | [], d => d
  | .lbrace :: rest, d => plainDepth rest (d + 1)
  | .rbrace :: rest, d => plainDepth rest (d - 1)
  | .other :: rest, d => plainDepth rest d

/-- Every concatenation of one head choice with one tail choice. -/
def crossAppend (heads tails : List (List Plain)) : List (List Plain) :=
  heads.flatMap fun h => tails.map fun t => h ++ t

mutual

/-- Expand a run of items up to — but not consuming — the separator or `#endif`
that ends the enclosing arm. -/
def expandSeq : Nat → List Item → List (List Plain) × List Item
  | 0, items => ([], items)
  | _ + 1, [] => ([[]], [])
  | f + 1, .lbrace :: rest =>
      let (outs, rem) := expandSeq f rest
      (outs.map (fun o => Plain.lbrace :: o), rem)
  | f + 1, .rbrace :: rest =>
      let (outs, rem) := expandSeq f rest
      (outs.map (fun o => Plain.rbrace :: o), rem)
  | f + 1, .other :: rest =>
      let (outs, rem) := expandSeq f rest
      (outs.map (fun o => Plain.other :: o), rem)
  | f + 1, .ifStart :: rest =>
      let (arms, hasElse, rem) := expandArms f rest
      let (tails, rem2) := expandSeq f rem
      -- No arm at all means no selection semantics can read this group.
      let choices := if arms.isEmpty then [] else if hasElse then arms else arms ++ [[]]
      (crossAppend choices tails, rem2)
  | _ + 1, .alt :: rest => ([[]], .alt :: rest)
  | _ + 1, .otherwise :: rest => ([[]], .otherwise :: rest)
  | _ + 1, .endIf :: rest => ([[]], .endIf :: rest)

/-- Expand every arm of the group whose first arm begins here, returning the
arms' resolved streams, whether an `#else` closed it, and the rest. An empty
result means no selection semantics can read this stream. -/
def expandArms : Nat → List Item → List (List Plain) × Bool × List Item
  | 0, items => ([], false, items)
  | f + 1, items =>
      let (armOuts, rem) := expandSeq f items
      match rem with
      | .alt :: rest =>
          let (more, hasElse, rem2) := expandArms f rest
          if more.isEmpty then ([], hasElse, []) else (armOuts ++ more, hasElse, rem2)
      | .otherwise :: rest =>
          let (elseOuts, rem2) := expandSeq f rest
          match rem2 with
          | .endIf :: r => (armOuts ++ elseOuts, true, r)
          | _ => ([], true, [])
      | .endIf :: rest => (armOuts, false, rest)
      | _ => ([], false, [])

end

/-- Every arm selection of a stream, or none when no selection semantics can
read it. -/
def expand (items : List Item) : List (List Plain) :=
  let (outs, rem) := expandSeq (items.length + 2) items
  if rem.isEmpty then outs else []

/-! ## 4. The measured shapes, decided under both rules -/

/-- The reduced `rd_kafka_dir_is_empty` shape: a function body, then a group
whose two arms each open a brace, then the single `}` that closes whichever
brace survived, then the body's own `}`. -/
def rdkafkaShape : List Item :=
  [.lbrace, .ifStart, .lbrace, .otherwise, .lbrace, .endIf, .rbrace, .rbrace]

/-- The rule returns to file scope, so every later definition is visible. -/
theorem the_rule_balances_the_measured_shape : (scan rdkafkaShape).depth = 0 := by decide

/-- Nothing is refused: both arms reach the same depth, so no arm was selected
and no conditional was evaluated. -/
theorem the_measured_shape_is_not_refused : refused (scan rdkafkaShape) = false := by decide

/-- The rule this unit replaces summed the arms and stayed one level deep, which
is exactly the fifteen-definition residual measured on the pinned corpus. -/
theorem the_summing_rule_desynchronises_the_measured_shape :
    scanSummed rdkafkaShape = 1 := by decide

/-- The two rules differ on it. -/
theorem the_unit_changes_the_measured_shape :
    (scan rdkafkaShape).depth ≠ scanSummed rdkafkaShape := by decide

/-- Every selection of the measured shape agrees with the rule's depth. -/
theorem every_selection_of_the_measured_shape_is_matched :
    (expand rdkafkaShape).all (fun sel => plainDepth sel 0 == (scan rdkafkaShape).depth) = true := by
  decide

/-- A shape the summing rule got wrong in the *dangerous* direction: an arm that
closes more than it opens lowered the depth below every selection, so a
block-macro statement inside the body was read as a file-scope definition. -/
def phantomShape : List Item :=
  [.lbrace, .ifStart, .rbrace, .endIf, .other]

/-- The summing rule reaches file scope inside the body — where the phantom came
from. -/
theorem the_summing_rule_reaches_file_scope_inside_a_body :
    scanSummed phantomShape = 0 := by decide

/-- The rule stays inside the body, and records the disagreement. -/
theorem the_rule_dominates_the_phantom_shape :
    (scan phantomShape).depth = 1 ∧ (scan phantomShape).disagreed = true := by decide

/-- Every selection of that shape is dominated. -/
theorem every_selection_of_the_phantom_shape_is_dominated :
    (expand phantomShape).all (fun sel => plainDepth sel 0 ≤ (scan phantomShape).depth) = true := by
  decide

/-- An `#elif` chain is one group with one contribution, decided exactly when its
arms agree. -/
def elifChain : List Item :=
  [.lbrace, .ifStart, .lbrace, .alt, .lbrace, .alt, .lbrace, .otherwise, .lbrace, .endIf,
    .rbrace, .rbrace]

theorem an_elif_chain_is_decided_exactly :
    (scan elifChain).depth = 0 ∧ refused (scan elifChain) = false := by decide

/-- Nested groups compose: the inner group's contribution is taken before the
arm that contains it is measured. -/
def nestedShape : List Item :=
  [.lbrace, .ifStart, .lbrace, .ifStart, .other, .otherwise, .other, .endIf, .otherwise,
    .lbrace, .endIf, .rbrace, .rbrace]

theorem nested_groups_compose :
    (scan nestedShape).depth = 0 ∧ refused (scan nestedShape) = false := by decide

/-- A stream no selection semantics can read is refused, in each of its shapes.
None of them can occur in a unit a preprocessor accepted. -/
theorem unstructured_streams_are_refused :
    ([[Item.endIf], [.alt], [.otherwise], [.ifStart],
      [.ifStart, .otherwise, .otherwise, .endIf]] : List (List Item)).all
      (fun items => (scan items).unstructured && (expand items).isEmpty) = true := by decide

/-! ## 5. The exhaustive domain

Every item string of length at most four over the six items that move depth or
group structure. `other` is omitted because `other_is_the_identity` proves it is
the identity on the state. -/

/-- The exhaustive alphabet. -/
def alphabet : List Item := [.lbrace, .rbrace, .ifStart, .alt, .otherwise, .endIf]

/-- Every item string of length exactly `n`. -/
def strings : Nat → List (List Item)
  | 0 => [[]]
  | n + 1 => (strings n).flatMap fun s => alphabet.map fun c => c :: s

/-- Every such string of length at most `n`, shortest first. -/
def domain (n : Nat) : List (List Item) :=
  (List.range (n + 1)).flatMap strings

set_option maxRecDepth 100000 in
/-- The domain is the size it claims: 1 + 6 + 36 + 216 + 1296. -/
theorem the_domain_is_exhaustive : (domain 4).length = 1555 := by decide

/-- The three properties, over one stream:

1. **domination** — every selection's depth is at most the scan's, so a
   depth-zero reading of the scan is depth zero under every selection and the
   scan invents no file-scope definition;
2. **exactness** — an unrefused scan has at least one selection and agrees with
   every one of them, so agreement really is exactness;
3. **completeness of the refusal** — a stream no selection semantics can read is
   always refused. -/
def decided (items : List Item) : Bool :=
  let s := scan items
  let sels := expand items
  sels.all (fun sel => plainDepth sel 0 ≤ s.depth)
    && (refused s || (!sels.isEmpty && sels.all (fun sel => plainDepth sel 0 == s.depth)))
    && (!sels.isEmpty || refused s)

set_option maxRecDepth 4000000 in
/-- **The load-bearing exhaustive theorem.** Every stream of length at most four
is decided: dominated, exact when unrefused, and refused when unreadable. -/
theorem the_domain_is_decided : (domain 4).all decided = true := by decide

/-! ## 6. Exhaustive agreement with the Rust implementation

The Rust side renders each item string as C text — one item per line, so a
directive is line-initial — and runs the real `legacy_c_source_scan`. It reads
the depth back by appending `k` closing braces and a probe definition for
`k = 0 … 4`: the probe is a file-scope definition exactly when the depth is at
most `k`, so the least such `k` is the depth (and `5` stands for "deeper than
four"). Both sides fold the same checksum over the same domain. -/

/-- The observable: the least `k ≤ 4` at which appending `k` closing braces
brings the scan to file scope, or `5`. -/
def probeDepth (items : List Item) : Nat :=
  let reaches := fun k => (scan (items ++ List.replicate k Item.rbrace)).depth == 0
  if reaches 0 then 0
  else if reaches 1 then 1
  else if reaches 2 then 2
  else if reaches 3 then 3
  else if reaches 4 then 4
  else 5

/-- One stream's observable, folded: the probe depth and the two refusal
grounds. -/
def observation (items : List Item) : Nat :=
  let s := scan items
  probeDepth items * 4 + (if s.disagreed then 2 else 0) + (if s.unstructured then 1 else 0)

/-- The whole-domain checksum. `crates/qcore-c-gateway`'s
`the_lean_conditional_domain_checksum_reproduces` computes the same fold by
running the real scan on the corresponding C text; the two numbers agreeing is
the tie between this model and the implementation. -/
def domainChecksum (n : Nat) : Nat :=
  (domain n).foldl (fun acc items => (acc * 1000033 + observation items) % 1000003) 13

set_option maxRecDepth 4000000 in
/-- The checksum, decided in the kernel. -/
theorem the_domain_checksum_is_pinned : domainChecksum 4 = 360626 := by decide

end QCore.ConditionalGroupDepth
