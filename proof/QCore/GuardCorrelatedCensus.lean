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

import QCore.ConditionalGroupDepth

/-! Card `TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE` (rule 49): the raw scan's
guard-correlation reading.

Unit `E01-RAW-COND` (`QCore.ConditionalGroupDepth`) refuses the definition
census when one conditional group's arms leave different file-scope brace
depths. Fluent Bit's `plugins/in_tail/tail_file.c` (`tag_compose`) opens a
brace inside one `#ifdef FLB_HAVE_REGEX … #endif` group and closes it inside a
second group with the same guard; each group alone disagrees, but in every
preprocessing both are kept or both are skipped.

The reading modelled here, implemented by
`qcore_c_gateway::legacy_c::guard_correlation::correlated_source_scan`, runs
one raw scan per **world** (a scan state and assumptions `NAME is (not)
defined`). A *guard group* — opened by exactly `#ifdef NAME`/`#ifndef NAME`,
with no `#elif` — is read under the world's assumption about `NAME`, splitting
the world when it has none; every other group is read as E01-RAW-COND reads
it, each arm from the entry depth, and a disagreement refuses. Another
directive (`reset` here: `#define`, `#undef`, `#include`, `#pragma`) clears a
world's assumptions. Worlds with equal scan states merge, keeping only shared
assumptions. No controlling expression is evaluated.

What is proved:

* the assumption bookkeeping: a split covers every value, a merge and a clear
  only weaken assumptions, so the worlds keep covering every valuation;
* the measured `tag_compose` shape is decided (its later definition is
  collected) where E01-RAW-COND refused it, and the decision is sound against
  every configuration of that shape;
* the negatives: the same shape behind `#if defined(A)` (a group that is not a
  guard group), or with an `#elif`, stays refused; a `#define` between the two
  groups only loses precision, never soundness;
* **exhaustively over every item string of length at most four** over ten
  items: whenever the scan decides, every definition any configuration keeps
  at file scope is collected — configurations range over every valuation of
  the guard per `reset` epoch and every outcome of every other controlling
  expression, so a guarded-out definition is covered too;
* **exhaustive agreement with the Rust implementation** over the same domain,
  tied by a checksum both sides compute;
* E01-RAW-COND's rule (`QCore.ConditionalGroupDepth`) refuses the measured
  shape and ends it one level deep, which is the defect this card closes.

What is **not** proved, and stays a Rust boundary: that `guard_of` recognises
exactly the `#ifdef NAME`/`#ifndef NAME` spellings, that every directive other
than a conditional one is reported as a `reset`, the `_Pragma` operator's
clearing, the refusal of a directive spelling `_Pragma`/`__pragma`/
`push_macro`/`pop_macro`, and the `extern "C"` linkage braces (shared unchanged with
`function_names`); and that a valuation read beyond the enumerated epochs and
positions does not matter (the bounded theorems enumerate exactly the ones a
stream can read). -/
namespace QCore.GuardCorrelatedCensus

/-- One item of the raw scan's stream. A `defn` stands for a definition head
(`int f(void) { }`: balanced, read at file scope). -/
inductive Item where
  | lbrace
  | rbrace
  | defn
  /-- Any directive other than a conditional one. -/
  | reset
  /-- `#ifdef NAME`. -/
  | ifdef (name : Nat)
  /-- `#ifndef NAME`. -/
  | ifndef (name : Nat)
  /-- Any other opening (`#if EXPR`, a commented guard, …). -/
  | ifPlain
  | alt
  | otherwise
  | endIf
  deriving DecidableEq, Repr

/-! ## 1. Group structure -/

/-- One group's shape. -/
structure Group where
  guard : Option (Nat × Bool)
  hasElse : Bool
  hasAlt : Bool
  deriving DecidableEq, Repr

/-- The structure pass's accumulator. -/
structure SAcc where
  groups : List Group
  groupOf : List (Nat × Nat)
  nextOf : List (Nat × Nat)
  stack : List (Nat × Nat)
  ok : Bool
  deriving Repr

def lookup (pos : Nat) (l : List (Nat × Nat)) : Nat :=
  ((l.find? (fun e => e.1 == pos)).map (·.2)).getD 0

def opening : Item → Option (Option (Nat × Bool))
  | .ifdef n => some (some (n, true))
  | .ifndef n => some (some (n, false))
  | .ifPlain => some none
  | _ => none

def noGroup : Group := ⟨none, false, false⟩

def structStep (acc : SAcc) (it : Item) (p : Nat) : SAcc :=
  match opening it with
  | some g =>
      { acc with groups := acc.groups ++ [⟨g, false, false⟩],
                 groupOf := (p, acc.groups.length) :: acc.groupOf,
                 stack := (acc.groups.length, p) :: acc.stack }
  | none =>
      if it = .alt ∨ it = .otherwise ∨ it = .endIf then
        match acc.stack with
        | [] => { acc with ok := false }
        | (gid, last) :: rest =>
            let g := acc.groups.getD gid noGroup
            if g.hasElse && it != .endIf then { acc with ok := false }
            else
              let acc := { acc with groupOf := (p, gid) :: acc.groupOf,
                                    nextOf := (last, p) :: acc.nextOf }
              if it = .alt then
                { acc with groups := acc.groups.set gid { g with hasAlt := true },
                           stack := (gid, p) :: rest }
              else if it = .otherwise then
                { acc with groups := acc.groups.set gid { g with hasElse := true },
                           stack := (gid, p) :: rest }
              else { acc with stack := rest }
      else acc

def structGo : List Item → Nat → SAcc → SAcc
  | [], _, acc => acc
  | it :: rest, p, acc => structGo rest (p + 1) (structStep acc it p)

/-- The group structure, or `none` for a stream no preprocessor reads. -/
def structureOf (items : List Item) : Option SAcc :=
  let acc := structGo items 0 ⟨[], [], [], [], true⟩
  if acc.ok && acc.stack.isEmpty then some acc else none

/-- A group's guard, when it is a guard group (no `#elif`). -/
def guardOf (g : Group) : Option (Nat × Bool) := if g.hasAlt then none else g.guard

/-! ## 2. Worlds -/

inductive Frame where
  | guard (tookFirst : Bool)
  | plain (entry : Nat) (arms : List Nat)
  deriving DecidableEq, Repr

structure Shape where
  depth : Nat
  frames : List Frame
  skip : Option Nat
  deriving DecidableEq, Repr

/-- Assumptions: `(name, defined)` pairs, at most one per name. -/
abbrev Assumed := List (Nat × Bool)

def get (a : Assumed) (n : Nat) : Option Bool := (a.find? (fun e => e.1 == n)).map (·.2)

def assume (a : Assumed) (n : Nat) (v : Bool) : Assumed := (n, v) :: a.filter (fun e => e.1 != n)

/-- The assumptions two merged worlds share. -/
def common (a b : Assumed) : Assumed := a.filter (fun e => get b e.1 == some e.2)

structure World where
  shape : Shape
  assumed : Assumed
  deriving DecidableEq, Repr

def budget : Nat := 64

inductive Outcome where
  | decided (defs : List Nat)
  | unstructured
  | disagree
  | overBudget
  deriving DecidableEq, Repr

def mergeInto (acc : List World) (w : World) : List World :=
  if acc.any (fun x => x.shape == w.shape) then
    acc.map (fun x => if x.shape == w.shape then { x with assumed := common x.assumed w.assumed } else x)
  else acc ++ [w]

def merge (ws : List World) : List World := ws.foldl mergeInto []

/-- `Except` carries a refusal. -/
abbrev Step := Except Outcome (List World)

def active (w : World) : Bool := w.shape.skip.isNone

/-- A conditional directive at position `p`, for one world. -/
def directive (s : SAcc) (it : Item) (p : Nat) (w : World) : Except Outcome (List World) :=
  match w.shape.skip with
  | some target =>
      if target == p then
        if it = .endIf then
          .ok [{ w with shape := { w.shape with skip := none, frames := w.shape.frames.drop 1 } }]
        else .ok [{ w with shape := { w.shape with skip := none } }]
      else .ok [w]
  | none =>
      let g := s.groups.getD (lookup p s.groupOf) noGroup
      match opening it with
      | some _ =>
          match guardOf g with
          | some (n, isIfdef) =>
              let values := match get w.assumed n with
                | some v => [v]
                | none => [true, false]
              .ok (values.map fun v =>
                let took := v == isIfdef
                let fr : List Frame := Frame.guard took :: w.shape.frames
                let sk : Option Nat := if took then none else some (lookup p s.nextOf)
                ({ shape := ⟨w.shape.depth, fr, sk⟩, assumed := assume w.assumed n v } : World))
          | none =>
              let fr : List Frame := Frame.plain w.shape.depth [] :: w.shape.frames
              .ok [{ w with shape := ⟨w.shape.depth, fr, w.shape.skip⟩ }]
      | none =>
          if it = .endIf then
            match w.shape.frames with
            | .guard _ :: rest => .ok [{ w with shape := { w.shape with frames := rest } }]
            | .plain entry arms :: rest =>
                let all := arms ++ [w.shape.depth] ++ (if g.hasElse then [] else [entry])
                if all.all (· == all.headD 0) then
                  .ok [{ w with shape := { w.shape with depth := all.headD 0, frames := rest } }]
                else .error .disagree
            | [] => .error .unstructured
          else
            match w.shape.frames with
            | .guard true :: _ =>
                .ok [{ w with shape := { w.shape with skip := some (lookup p s.nextOf) } }]
            | .plain entry arms :: rest =>
                let fr : List Frame := Frame.plain entry (arms ++ [w.shape.depth]) :: rest
                let sh : Shape := ⟨entry, fr, w.shape.skip⟩
                .ok [{ w with shape := sh }]
            | _ => .error .unstructured

def isDirective : Item → Bool
  | .ifdef _ | .ifndef _ | .ifPlain | .alt | .otherwise | .endIf => true
  | _ => false

def concatMapE (f : World → Except Outcome (List World)) : List World → Except Outcome (List World)
  | [] => .ok []
  | w :: rest => do
      let a ← f w
      let b ← concatMapE f rest
      pure (a ++ b)

/-- One item for every world; `defs` collects positions of definitions some
world reaches at file scope. -/
def stepAll (s : SAcc) (it : Item) (p : Nat) (ws : List World) (defs : List Nat) :
    Except Outcome (List World × List Nat) :=
  if isDirective it then do
    let out ← concatMapE (directive s it p) ws
    let m := merge out
    if m.length > budget then .error .overBudget else pure (m, defs)
  else if it = .reset then
    let out := ws.map (fun w => if active w then { w with assumed := [] } else w)
    let m := merge out
    if m.length > budget then .error .overBudget else .ok (m, defs)
  else
    let ws' := ws.map fun w =>
      if !active w then w
      else match it with
        | .lbrace => { w with shape := { w.shape with depth := w.shape.depth + 1 } }
        | .rbrace => { w with shape := { w.shape with depth := w.shape.depth - 1 } }
        | _ => w
    let found := it = .defn ∧ ws.any (fun w => active w && w.shape.depth == 0)
    .ok (ws', if found then defs ++ [p] else defs)

def scanGo (s : SAcc) : List Item → Nat → List World → List Nat → Except Outcome (List World × List Nat)
  | [], _, ws, defs => .ok (ws, defs)
  | it :: rest, p, ws, defs => do
      let (ws', defs') ← stepAll s it p ws defs
      scanGo s rest (p + 1) ws' defs'

def start : World := ⟨⟨0, [], none⟩, []⟩

/-- The guard-correlation scan. -/
def cscan (items : List Item) : Outcome :=
  match structureOf items with
  | none => .unstructured
  | some s =>
      match scanGo s items 0 [start] [] with
      | .ok (_, defs) => .decided defs
      | .error e => e

/-! ## 3. Assumption bookkeeping

A valuation `v` is consistent with assumptions `a` when it agrees on every
assumed name. -/

def consistent (v : Nat → Bool) (a : Assumed) : Prop := ∀ e ∈ a, v e.1 = e.2

/-- A split covers every valuation: one of its two worlds is consistent. -/
theorem a_split_covers_every_value (v : Nat → Bool) (a : Assumed) (n : Nat)
    (h : consistent v a) : consistent v (assume a n (v n)) := by
  intro e he
  simp [assume] at he
  rcases he with rfl | ⟨mem, _⟩
  · rfl
  · exact h e mem

/-- A merge keeps only shared assumptions, so a valuation consistent with
either world is consistent with the merged one. -/
theorem a_merge_only_weakens (v : Nat → Bool) (a b : Assumed) (h : consistent v a) :
    consistent v (common a b) := by
  intro e he
  exact h e (List.mem_filter.mp he).1

/-- A cleared world assumes nothing, so every valuation is consistent with it:
a `#define` or an `#include` costs precision, never soundness. -/
theorem a_clear_assumes_nothing (v : Nat → Bool) : consistent v [] := by
  intro e he; cases he

/-! ## 4. Selection semantics

What a preprocessor keeps under one configuration: `val e n` is whether guard
`n` is defined in the `e`-th epoch (`reset` starts a new epoch), `cond p` the
outcome of the controlling expression at position `p` for every other opening
and `#elif`. -/

structure PFrame where
  parent : Bool
  taken : Bool
  active : Bool
  sawElse : Bool
  deriving DecidableEq, Repr

structure PState where
  depth : Nat
  stack : List PFrame
  epoch : Nat
  defs : List Nat
  ok : Bool
  deriving Repr

def pActive (s : PState) : Bool :=
  match s.stack with
  | [] => true
  | f :: _ => f.active

def openWith (s : PState) (t : Bool) : PState :=
  let a := pActive s
  { s with stack := ⟨a, if a then t else true, a && t, false⟩ :: s.stack }

def pStep (val : Nat → Nat → Bool) (cond : Nat → Bool) (s : PState) (it : Item) (p : Nat) :
    PState :=
  let a := pActive s
  match it with
  | .lbrace => if a then { s with depth := s.depth + 1 } else s
  | .rbrace => if a then { s with depth := s.depth - 1 } else s
  | .defn => if a && s.depth == 0 then { s with defs := s.defs ++ [p] } else s
  | .reset => if a then { s with epoch := s.epoch + 1 } else s
  | .ifdef n => openWith s (val s.epoch n)
  | .ifndef n => openWith s (!val s.epoch n)
  | .ifPlain => openWith s (cond p)
  | .alt =>
      match s.stack with
      | [] => { s with ok := false }
      | f :: rest =>
          if f.sawElse then { s with ok := false }
          else if f.taken then { s with stack := { f with active := false } :: rest }
          else
            let t := cond p
            { s with stack := { f with taken := t, active := f.parent && t } :: rest }
  | .otherwise =>
      match s.stack with
      | [] => { s with ok := false }
      | f :: rest =>
          if f.sawElse then { s with ok := false }
          else { s with stack := ⟨f.parent, true, f.parent && !f.taken, true⟩ :: rest }
  | .endIf =>
      match s.stack with
      | [] => { s with ok := false }
      | _ :: rest => { s with stack := rest }

def pGo (val : Nat → Nat → Bool) (cond : Nat → Bool) : List Item → Nat → PState → PState
  | [], _, s => s
  | it :: rest, p, s => pGo val cond rest (p + 1) (pStep val cond s it p)

/-- The file-scope definitions one configuration keeps, or `none` for a stream
no preprocessor reads. -/
def run (val : Nat → Nat → Bool) (cond : Nat → Bool) (items : List Item) : Option (List Nat) :=
  let s := pGo val cond items 0 ⟨0, [], 0, [], true⟩
  if s.ok && s.stack.isEmpty then some s.defs else none

/-! ## 5. Configurations, enumerated

A stream reads the guard valuation only at epochs up to its number of
`reset`s, and `cond` only at its own positions; every such reading is
enumerated. -/

def bools : Nat → List (List Bool)
  | 0 => [[]]
  | k + 1 => (bools k).flatMap fun l => [true :: l, false :: l]

def resets (items : List Item) : Nat := items.countP (· == .reset)

/-- Every configuration of a stream over `names` guard names. -/
def configs (names : Nat) (items : List Item) : List ((Nat → Nat → Bool) × (Nat → Bool)) :=
  let vbits := (resets items + 1) * names
  (bools vbits).flatMap fun vs =>
    (bools items.length).map fun cs =>
      (fun e n => vs.getD (e * names + n) false, fun p => cs.getD p false)

/-- Soundness of one stream: when the scan decides, every definition any
configuration keeps is collected. -/
def sound (names : Nat) (items : List Item) : Bool :=
  match cscan items with
  | .decided defs =>
      (configs names items).all fun (val, cond) =>
        match run val cond items with
        | some kept => kept.all (· ∈ defs)
        | none => true
  | _ => true

/-! ## 6. The measured shape and its negatives

`tag_compose` reduced: a body; `#ifdef A` opening a brace; `#ifdef A` closing
it; the body's `}`; a later definition. -/

def A : Nat := 0
def B : Nat := 1

def tagCompose : List Item :=
  [.lbrace, .ifdef A, .lbrace, .endIf, .ifdef A, .rbrace, .endIf, .rbrace, .defn]

/-- Decided, and the later definition (position 8) is collected. -/
theorem the_measured_shape_is_decided : cscan tagCompose = .decided [8] := by decide +kernel

/-- Sound against every configuration of it. -/
theorem the_measured_shape_is_sound : sound 1 tagCompose = true := by decide +kernel

/-- The same shape behind `#if defined(A)` is not a guard group: refused. -/
def tagComposePlain : List Item :=
  [.lbrace, .ifPlain, .lbrace, .endIf, .ifPlain, .rbrace, .endIf, .rbrace, .defn]

theorem an_uncorrelated_shape_is_refused : cscan tagComposePlain = .disagree := by decide +kernel

/-- An `#elif` makes a guard group an ordinary one: refused. -/
def tagComposeElif : List Item :=
  [.lbrace, .ifdef A, .lbrace, .alt, .endIf, .ifdef A, .rbrace, .endIf, .rbrace, .defn]

theorem an_elif_group_is_refused : cscan tagComposeElif = .disagree := by decide +kernel

/-- A `#define` between the two groups clears the assumption: the second group
splits again, and the scan stays sound (precision lost, not soundness). -/
def tagComposeReset : List Item :=
  [.lbrace, .ifdef A, .lbrace, .endIf, .reset, .ifdef A, .rbrace, .endIf, .rbrace, .defn]

theorem a_reset_between_is_still_sound : sound 1 tagComposeReset = true := by decide +kernel

/-- …and still decided: every world reaches the later definition at file scope
in some configuration. -/
theorem a_reset_between_is_decided : cscan tagComposeReset = .decided [9] := by decide +kernel

/-- Two different guards whose arms leave different definition sets: in the
configurations where `A` is defined and `B` is not, the later definition is
inside a brace; where `A` is undefined it is at file scope. The scan follows
every configuration and collects it. -/
def twoGuards : List Item :=
  [.ifdef A, .lbrace, .endIf, .defn, .ifdef B, .rbrace, .endIf, .defn]

theorem different_guards_collect_every_configuration :
    cscan twoGuards = .decided [3, 7] ∧ sound 2 twoGuards = true := by decide +kernel

/-! ## 7. The exhaustive domain -/

def alphabet : List Item :=
  [.lbrace, .rbrace, .defn, .reset, .ifdef A, .ifndef A, .ifPlain, .alt, .otherwise, .endIf]

def strings : Nat → List (List Item)
  | 0 => [[]]
  | n + 1 => (strings n).flatMap fun s => alphabet.map fun c => c :: s

def domain (n : Nat) : List (List Item) := (List.range (n + 1)).flatMap strings

set_option maxRecDepth 100000 in
theorem the_domain_is_exhaustive : (domain 4).length = 11111 := by decide +kernel

set_option maxRecDepth 4000000 in
/-- **The load-bearing exhaustive theorem.** Over every stream of length at
most four, whenever the scan decides, every configuration's file-scope
definitions are collected. -/
theorem the_domain_is_sound : (domain 4).all (sound 1) = true := by decide +kernel

/-! ## 8. Exhaustive agreement with the Rust implementation -/

def mask (defs : List Nat) : Nat :=
  (List.range 4).foldl (fun acc k => if k ∈ defs then acc + 2 ^ k else acc) 0

def observation (items : List Item) : Nat :=
  match cscan items with
  | .unstructured => 1
  | .disagree => 2
  | .overBudget => 3
  | .decided defs => 4 + mask defs

def domainChecksum (n : Nat) : Nat :=
  (domain n).foldl (fun acc items => (acc * 1000033 + observation items) % 1000003) 13

set_option maxRecDepth 4000000 in
/-- The checksum, decided in the kernel. `crates/qcore-c-gateway`'s
`the_lean_guard_domain_checksum_reproduces` computes the same fold by running
the real `correlated_source_scan` on the corresponding C text. -/
theorem the_domain_checksum_is_pinned : domainChecksum 4 = 673782 := by decide +kernel

/-! ## 9. The rule this card leaves in force

E01-RAW-COND's reading of the measured shape (a `defn` is an ordinary token
there): it disagrees, so without rule 49 the census stays refused. Rule 49
runs only on a scan whose single ground is that disagreement. -/

def tagComposeRaw : List QCore.ConditionalGroupDepth.Item :=
  [.lbrace, .ifStart, .lbrace, .endIf, .ifStart, .rbrace, .endIf, .rbrace, .other]

theorem e01_raw_cond_refuses_the_measured_shape :
    (QCore.ConditionalGroupDepth.scan tagComposeRaw).disagreed = true
      ∧ (QCore.ConditionalGroupDepth.scan tagComposeRaw).unstructured = false
      ∧ (QCore.ConditionalGroupDepth.scan tagComposeRaw).depth = 1 := by decide

end QCore.GuardCorrelatedCensus
