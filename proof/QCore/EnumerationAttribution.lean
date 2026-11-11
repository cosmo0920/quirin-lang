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

/-! Unit `E01-TAIL`, residual `E01-F1-STDIN`: *which* text of a line-marked
preprocessing belongs to the audited main source.

`docs/reports/remaining-e01-f1/README.md` recorded the residual. E01-F1 read
the primary input from the first line marker and attributed a token to the main
source by comparing the current marker's **file spelling** against it. The
pinned Cycle 19 compatibility transport spells its primary input `<stdin>`
(`CSourceInput::HistoricalStdin`), a fixed name any author can write, so the
header/main split of every such unit rested on the transport rather than on
where Clang actually read the text.

This models the repair in
`qcore_c_gateway::legacy_c::enumeration::tokenize_marked`: a token is
main-source exactly when its **include depth** is zero. Clang's markers carry
the provenance directly — flag `1` enters a file, flag `2` returns from one —
and a `#line` directive carries neither, so it moves no token between files.

What is proved here:

* **the two rules are not the same rule**, and the witnesses that separate them
  are the two measured directions: a header relabelled onto the primary
  spelling (which name attribution wrongly counts) and a main-source tail
  relabelled off it (which name attribution wrongly drops);
* **`#line` is inert for attribution** — a flagless marker never changes any
  token's depth, in either direction;
* **depth attribution is transport-independent** — renaming every file in a
  stream, the primary included, leaves the main-source token set unchanged, so
  a `<stdin>` unit and a declared-file unit are attributed by one rule;
* **enter/return are inverse** on a balanced stream, and the preamble Clang
  emits (`<built-in>`, `<command line>`, a forced `-include`) is out of scope;
* **fail-closed on an unreadable stack** — an underflowing or unclosed stream
  is refused, never attributed, and the refusal is an environment ground whose
  subject is never the analysed source (§L21.1 rule 1).

What is **not** proved here, and stays a tool boundary: that Clang emits flag
`1` on entering a file and flag `2` on returning, that it emits a flagless
marker for `#line`, and that it opens its output with a marker naming the
primary input. Those are observations of the tool, pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_stdin_attribution.rs` (which drives real
preprocessings in both transports) and by the reader-level matrix in
`crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs`. The streams below
are transcriptions of those measurements, not extractions from them. -/
namespace QCore.EnumerationAttribution

/-- A file spelling, as a line marker writes it. -/
abbrev FileName := String

/-- One line marker. `enters` is Clang's flag `1`, `returns` its flag `2`.
Flags `3` (system header) and `4` (`extern "C"`) say nothing about the include
stack and are not modelled; a `#line` directive sets neither flag. -/
structure Marker where
  file : FileName
  enters : Bool
  returns : Bool
  deriving DecidableEq, Repr

/-- A marker that only renames: exactly what Clang emits for `#line`. -/
def relabel (file : FileName) : Marker := ⟨file, false, false⟩

/-- A marker that enters `file`. -/
def enter (file : FileName) : Marker := ⟨file, true, false⟩

/-- A marker that returns to `file`. -/
def leave (file : FileName) : Marker := ⟨file, false, true⟩

/-- One item of a line-marked preprocessing: a marker, or a token of source. -/
inductive Item where
  | mark (marker : Marker)
  | tok (text : String)
  deriving DecidableEq, Repr

/-- The state the reader carries: the current include depth, the current file
spelling, and whether the stack has already been seen to break. -/
structure State where
  depth : Nat
  file : FileName
  broken : Bool
  deriving DecidableEq, Repr

/-- `tokenize_marked`'s marker branch. A `returns` at depth zero is an
underflow: there is nothing to return to, so the stream is unreadable. -/
def step (state : State) (marker : Marker) : State :=
  if marker.enters then
    { state with depth := state.depth + 1, file := marker.file }
  else if marker.returns then
    match state.depth with
    | 0 => { state with file := marker.file, broken := true }
    | next + 1 => { state with depth := next, file := marker.file }
  else
    { state with file := marker.file }

/-- Run a stream, collecting each token with the depth and file spelling it was
read at. -/
def run (primary : FileName) : List Item → State → State × List (String × Nat × FileName)
  | [], state => (state, [])
  | .mark marker :: rest, state => run primary rest (step state marker)
  | .tok text :: rest, state =>
      let (final, tail) := run primary rest state
      (final, (text, state.depth, state.file) :: tail)

/-- The initial state: Clang's opening marker names the primary input at
depth 0. -/
def start (primary : FileName) : State := ⟨0, primary, false⟩

/-- Every token of a stream, with its depth and file. -/
def readings (primary : FileName) (items : List Item) : List (String × Nat × FileName) :=
  (run primary items (start primary)).2

/-- Whether the stream's include stack balances: no underflow, and no file left
open at end of output. -/
def balanced (primary : FileName) (items : List Item) : Bool :=
  let final := (run primary items (start primary)).1
  !final.broken && final.depth == 0

/-! ## The two attribution rules -/

/-- The repair: a token is main-source exactly when its include depth is 0. -/
def byDepth (primary : FileName) (items : List Item) : List String :=
  (readings primary items).filterMap fun r => if r.2.1 = 0 then some r.1 else none

/-- E01-F1's rule: a token is main-source when the marker in force names the
primary input. Kept here only to be refuted. -/
def byName (primary : FileName) (items : List Item) : List String :=
  (readings primary items).filterMap fun r => if r.2.2 = primary then some r.1 else none

/-! ## The measured witnesses

Both are transcriptions of real Clang 21 output recorded in
`docs/reports/remaining-e01-tail/README.md` §2. -/

/-- A header that writes the primary input's own spelling. Clang enters the
header (flag 1), the header's `#line` renames without a flag, then Clang
returns (flag 2). -/
def relabelledHeader (primary : FileName) : List Item :=
  [.mark (enter "h.h"), .mark (relabel primary), .tok "headerFn",
   .mark ⟨primary, false, true⟩, .tok "mainFn"]

/-- A main source that renames its own tail to a foreign spelling. No file is
entered or left; only the name changes. -/
def relabelledTail : List Item :=
  [.tok "before", .mark (relabel "far.h"), .tok "after"]

/-- Clang's measured preamble: `<built-in>` opened, `<command line>` opened
inside it, both closed, then the return to the primary input. A forced
`-include` file sits inside `<built-in>`, at depth 2. -/
def preamble (primary : FileName) : List Item :=
  [.mark (enter "<built-in>"), .mark (enter "<command line>"),
   .mark (leave "<built-in>"), .mark (enter "forced.h"), .tok "forcedFn",
   .mark (leave "<built-in>"), .mark (leave primary)]

/-! ## `#line` is inert -/

/-- A flagless marker changes only the recorded file spelling. -/
theorem relabel_preserves_depth (state : State) (file : FileName) :
    (step state (relabel file)).depth = state.depth := by
  simp [step, relabel]

/-- …and cannot break the stack. -/
theorem relabel_preserves_breakage (state : State) (file : FileName) :
    (step state (relabel file)).broken = state.broken := by
  simp [step, relabel]

/-- Entering then returning restores the depth. -/
theorem enter_then_leave_restores_depth (state : State) (inner outer : FileName) :
    (step (step state (enter inner)) (leave outer)).depth = state.depth := by
  simp [step, enter, leave]

/-! ## The two directions that separate the rules -/

/-- **Direction one.** A header that writes the primary spelling keeps its
definitions out of the audited scope under depth attribution: boundary E01-B2
holds under a relabel. Stated for both transports — the pinned compatibility
route's `<stdin>` and a declared file — because the residual was that the two
answered differently. -/
theorem a_relabelled_header_is_not_main_source :
    byDepth "<stdin>" (relabelledHeader "<stdin>") = ["mainFn"]
      ∧ byDepth "subject.c" (relabelledHeader "subject.c") = ["mainFn"] := by
  decide

/-- …and name attribution wrongly counted it. This is the scope misstatement
E01-F1-STDIN named, reachable for `<stdin>` without knowing any path. -/
theorem name_attribution_counted_the_relabelled_header :
    byName "<stdin>" (relabelledHeader "<stdin>") = ["headerFn", "mainFn"] := by
  decide

/-- **Direction two.** A main source that renames its own tail keeps that tail
in the denominator under depth attribution. -/
theorem a_relabelled_tail_is_main_source :
    byDepth "<stdin>" relabelledTail = ["before", "after"] := by
  decide

/-- …and name attribution wrongly dropped it. This is the fail-open direction:
a macro-produced definition after such a directive has no raw-scan row to fall
back on, so it left the denominator with nothing said. -/
theorem name_attribution_dropped_the_relabelled_tail :
    byName "<stdin>" relabelledTail = ["before"] := by
  decide

/-- Non-vacuity, stated as one sentence: the two rules genuinely differ, and
they differ in both directions. -/
theorem the_two_rules_are_different :
    byDepth "<stdin>" (relabelledHeader "<stdin>") ≠ byName "<stdin>" (relabelledHeader "<stdin>")
      ∧ byDepth "<stdin>" relabelledTail ≠ byName "<stdin>" relabelledTail := by
  decide

/-! ## Transport independence

The `<stdin>` spelling is not special, because no spelling is: depth
attribution never reads a file name. -/

/-- Rename every file in a stream, including the primary input. -/
def rename (f : FileName → FileName) : Item → Item
  | .mark m => .mark ⟨f m.file, m.enters, m.returns⟩
  | .tok t => .tok t

/-- Renaming does not change any token's depth. -/
theorem rename_preserves_depths (f : FileName → FileName) (items : List Item)
    (primary : FileName) (state : State) :
    (run (f primary) (items.map (rename f)) ⟨state.depth, f state.file, state.broken⟩).2.map
        (fun r => (r.1, r.2.1))
      = (run primary items state).2.map (fun r => (r.1, r.2.1)) := by
  induction items generalizing state with
  | nil => simp [run]
  | cons item rest ih =>
      cases item with
      | mark m =>
          simp only [List.map_cons, rename, run]
          by_cases h : m.enters
          · simpa [step, h] using ih (state := step state m)
          · by_cases h2 : m.returns
            · cases hd : state.depth with
              | zero => simpa [step, h, h2, hd] using ih (state := step state m)
              | succ n => simpa [step, h, h2, hd] using ih (state := step state m)
            · simpa [step, h, h2] using ih (state := step state m)
      | tok t =>
          simp only [List.map_cons, rename, run]
          simpa using ih (state := state)

/-- **The E01-F1-STDIN sentence.** The audited main-source token set does not
depend on how the transport spells the primary input: a `<stdin>` unit and a
declared-file unit carrying the same include structure are attributed
identically. -/
theorem depth_attribution_is_transport_independent (f : FileName → FileName)
    (items : List Item) (primary : FileName) :
    byDepth (f primary) (items.map (rename f)) = byDepth primary items := by
  have h := rename_preserves_depths f items primary (start primary)
  simp only [start] at h
  simp only [byDepth, readings, start]
  have : ∀ l : List (String × Nat × FileName),
      l.filterMap (fun r => if r.2.1 = 0 then some r.1 else none)
        = (l.map (fun r => (r.1, r.2.1))).filterMap
            (fun r => if r.2 = 0 then some r.1 else none) := by
    intro l; induction l with
    | nil => simp
    | cons a t ih => by_cases ha : a.2.1 = 0 <;> simp [ha, ih]
  rw [this, this, h]

/-- Name attribution has no such property: the same stream under two spellings
of the primary input gives two different answers. -/
theorem name_attribution_is_not_transport_independent :
    byName "<stdin>" (relabelledHeader "<stdin>")
      ≠ byName "main.c" (relabelledHeader "<stdin>") := by
  decide

/-! ## The preamble stays out of scope -/

/-- `<built-in>`, `<command line>` and a forced `-include` are all entered, so
none of their tokens is main-source; the stream balances. -/
theorem the_preamble_contributes_no_main_source_token :
    byDepth "<stdin>" (preamble "<stdin>" ++ [.tok "mainFn"]) = ["mainFn"]
      ∧ balanced "<stdin>" (preamble "<stdin>" ++ [.tok "mainFn"]) = true := by
  decide

/-! ## Fail-closed on an unreadable stack -/

/-- A return with nothing to return to is recorded as broken. -/
theorem an_underflowing_stream_is_unbalanced :
    balanced "m.c" [.mark (leave "m.c"), .tok "t"] = false := by
  decide

/-- A file still open at end of output is unbalanced. -/
theorem an_unclosed_stream_is_unbalanced :
    balanced "m.c" [.mark (enter "h.h"), .tok "t"] = false := by
  decide

/-- The well-formed control is balanced, so the two refusals above are not
vacuous. -/
theorem a_well_formed_stream_is_balanced :
    balanced "m.c" [.mark (enter "h.h"), .tok "t", .mark (leave "m.c"), .tok "u"] = true := by
  decide

/-- Once broken, always broken: no later marker can repair a stack the reader
already lost, so an imbalance cannot be hidden by the rest of the stream. -/
theorem breakage_is_absorbing (state : State) (marker : Marker) (h : state.broken = true) :
    (step state marker).broken = true := by
  unfold step
  split
  · simpa using h
  · split
    · split <;> simp [h]
    · simpa using h

/-! ## §L21.1: the refusal ground's subject -/

/-- The §L21.1 subject partition, as `ReasonSubject`. -/
inductive Subject where
  | analysedSource
  | analyser
  | environment
  deriving DecidableEq, Repr

/-- The category `EnumerationRefusal::UnbalancedIncludeMarkers` carries. -/
inductive Category where
  | unsupportedSyntax
  | environmentUnavailable
  deriving DecidableEq, Repr

def Category.subject : Category → Subject
  | .unsupportedSyntax => .analysedSource
  | .environmentUnavailable => .environment

/-- The new ground: the *tool's* output could not be read. -/
def unbalancedIncludeMarkers : Category := .environmentUnavailable

/-- §L21.1 rule 1 over this ground: a failure to read the analysis machine's
own output is never a statement about the analysed source, so it can never
contribute to a provisional `Unsafe`. -/
theorem the_unbalanced_ground_says_nothing_about_the_source :
    unbalancedIncludeMarkers.subject = Subject.environment := by
  decide

/-- …and it is genuinely a different subject from the token-stream grounds, so
the partition is not vacuous here. -/
theorem the_two_subjects_are_distinct :
    Category.unsupportedSyntax.subject ≠ Category.environmentUnavailable.subject := by
  decide

end QCore.EnumerationAttribution
