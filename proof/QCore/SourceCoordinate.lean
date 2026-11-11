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

import QCore.EnumerationAttribution

/-! Card `GATEWAY-C-LOCATION`: the *presented* original coordinate of an
existing-C gateway diagnostic, and why a `#line` directive in analysed source
cannot forge one.

`QCore.EnumerationAttribution` settles which text of a line-marked
preprocessing is **in the audited scope**: a token is main-source exactly when
its include depth is zero, and a flagless marker — which is how Clang re-emits
a `#line` — never moves a depth. That rule is unchanged here and is not
re-proved.

This module models the *second* reader over the same marker stream,
`qcore_c_gateway::legacy_c::location::walk_marked`, which gives each content
line the `file:line` a report presents it at. Both readers run over one stream,
so the question this card must answer is whether the presentation opens a door
the scope rule closed.

What is proved here:

* **a presented file spelling is always one an entering marker wrote** — the
  general statement, over every stream: the spelling a *flagless* marker
  carries can never appear in a presented coordinate, and Clang re-emits a
  `#line` as a flagless marker;
* **depth 0 is not named by a marker at all** — the primary frame presents
  `primary`, a name the caller supplies, so the pinned Cycle 19 transport's
  fixed `<stdin>` spelling reaches no report;
* **a disagreeing flagless marker withdraws** — the frame it renames presents
  *no* coordinate rather than a renamed one, in both measured directions (a
  header writing the primary spelling, and a main source renaming its own
  tail), while the audited scope of that same stream is unmoved;
* **an agreeing flagless marker only resynchronises** — non-vacuity: Clang's
  own resync, and a bare `#line 500`, still set the counter, which is the one
  thing a `#line` may do inside the file it is written in;
* **lines advance by one per content line**, which is what makes a macro
  expansion present its expansion site: Clang writes an expansion out at the
  invocation's line and emits no marker for it;
* **fail-closed** — content with no frame presents nothing, an unreadable
  include stack breaks the reader, and every refusal ground's §L21.1 subject is
  the environment, never the analysed source.

What is **not** proved here, and stays a tool boundary exactly as it does for
`EnumerationAttribution`: that Clang emits flag `1` on entering a file, flag
`2` on returning, a flagless marker for `#line`, and an opening marker naming
the primary input. Those are observations, pinned behaviourally by
`crates/qcore-c-gateway/tests/gateway_c_location.rs` (which drives real
preprocessings, `#line` directives included) and by the reader-level matrix in
`crates/qcore-c-gateway/tests/unit/legacy_location.rs`. -/

namespace QCore.SourceCoordinate

open QCore.EnumerationAttribution

/-- What a presented coordinate names. `primary` is deliberately not a string:
include depth 0 is named by the caller's own configured file, never by a
marker. -/
inductive OriginFile where
  | primary
  | included (file : FileName)
  deriving DecidableEq, Repr

/-- One presented coordinate. -/
structure Coordinate where
  depth : Nat
  file : OriginFile
  line : Nat
  deriving DecidableEq, Repr

/-- One open file of the presentation reader's include stack.

Two fields carry the file, because the reader does two different things with
it. `spelling` is the name the frame was **entered** as, and is what a flagless
marker is compared against. `presented` is what a coordinate of this frame
*names*: an entered file names itself, and the primary input names
[`OriginFile.primary`] — the caller's own configured file, never a marker
spelling. The primary frame is the one the opening marker establishes on an
empty stack, which is include depth 0. -/
structure Frame where
  spelling : FileName
  presented : OriginFile
  line : Nat
  relabelled : Bool
  deriving DecidableEq, Repr

/-- The reader's state: the include stack, innermost **first**, and whether the
stack has been seen to break. -/
structure Reader where
  stack : List Frame
  broken : Bool
  deriving DecidableEq, Repr

/-- `walk_marked`'s marker branch.

The arms are the whole anti-forgery argument:

* `enters` pushes a frame **named by this marker** — the only place a file
  spelling becomes presentable, and Clang sets this flag only when it enters a
  file;
* `returns` pops and resynchronises the frame beneath, keeping that frame's own
  spelling and its own presented identity;
* a flagless marker sets the counter of the frame in force and, when it names
  another file, marks that frame relabelled. It writes neither `spelling` nor
  `presented`. On an **empty** stack it is Clang's opening marker, which
  establishes the primary frame — and that frame presents
  [`OriginFile.primary`], so even there the marker's spelling is not adopted
  for presentation. -/
def step (reader : Reader) (marker : Marker) (number : Nat) : Reader :=
  if marker.enters then
    { reader with
      stack := ⟨marker.file, .included marker.file, number, false⟩ :: reader.stack }
  else if marker.returns then
    match reader.stack with
    | [] => { reader with broken := true }
    | [_] => { reader with broken := true }
    | _ :: outer :: rest =>
        { reader with
          stack := ⟨outer.spelling, outer.presented, number,
            marker.file ≠ outer.spelling⟩ :: rest }
  else
    match reader.stack with
    | [] => { reader with stack := [⟨marker.file, .primary, number, false⟩] }
    | frame :: rest =>
        { reader with
          stack := ⟨frame.spelling, frame.presented, number,
            marker.file ≠ frame.spelling⟩ :: rest }

/-- The coordinate the frame in force gives a content line, and the reader
after that line. A relabelled frame gives **none**. -/
def content (reader : Reader) : Option Coordinate × Reader :=
  match reader.stack with
  | [] => (none, reader)
  | frame :: rest =>
      let coordinate :=
        if frame.relabelled then none
        else some ⟨rest.length, frame.presented, frame.line⟩
      (coordinate,
        { reader with
          stack := ⟨frame.spelling, frame.presented, frame.line + 1, frame.relabelled⟩ :: rest })

/-- One line of a marked stream, carrying the number a marker declares. -/
inductive Line where
  | mark (marker : Marker) (number : Nat)
  | content (text : String)
  deriving DecidableEq, Repr

/-- Run a stream, collecting each content line's presented coordinate. -/
def readings : List Line → Reader → List (String × Option Coordinate)
  | [], _ => []
  | .mark marker number :: rest, reader => readings rest (step reader marker number)
  | .content text :: rest, reader =>
      let (coordinate, next) := content reader
      (text, coordinate) :: readings rest next

/-- The reader before the opening marker: no frame at all. -/
def start : Reader := ⟨[], false⟩

/-- Every content line's coordinate. -/
def coordinates (items : List Line) : List (String × Option Coordinate) :=
  readings items start

/-! ## A flagless marker's spelling is never presented

The general statement, over every stream: every *file spelling* a presented
coordinate carries was written by a marker that **entered** a file. Clang
re-emits a `#line` directive as a flagless marker, so no `#line` can put a file
name into a report. -/

/-- The file spellings the entering markers of a stream write. -/
def entered : List Line → List FileName
  | [] => []
  | .mark m _ :: rest => if m.enters then m.file :: entered rest else entered rest
  | .content _ :: rest => entered rest

/-- The spelling an `OriginFile` names, if it names one at all. -/
def OriginFile.name : OriginFile → Option FileName
  | .primary => none
  | .included file => some file

/-- The spellings a stack could present. -/
def stackNames (stack : List Frame) : List FileName :=
  stack.filterMap fun frame => frame.presented.name

/-- The spellings a run of readings actually presents. -/
def presentedFiles (rows : List (String × Option Coordinate)) : List FileName :=
  rows.filterMap fun row =>
    match row.2 with
    | some coordinate => coordinate.file.name
    | none => none

/-- Membership in a stack's presentable spellings, one frame at a time. -/
theorem mem_stackNames_cons (frame : Frame) (rest : List Frame) (file : FileName) :
    file ∈ stackNames (frame :: rest) ↔
      frame.presented.name = some file ∨ file ∈ stackNames rest := by
  cases h : frame.presented with
  | primary => simp [stackNames, h, OriginFile.name]
  | included g => simp [stackNames, h, OriginFile.name, eq_comm]

/-- **The one place a spelling becomes presentable.** After a step, every
presentable spelling on the stack is either this marker's *and* the marker
entered a file, or was already presentable before. A flagless marker adds
none — including the opening one, whose frame presents `primary`. -/
theorem step_adds_only_an_entering_spelling (reader : Reader) (marker : Marker) (number : Nat) :
    ∀ file ∈ stackNames (step reader marker number).stack,
      (marker.enters = true ∧ file = marker.file) ∨ file ∈ stackNames reader.stack := by
  obtain ⟨stack, broken⟩ := reader
  intro file hfile
  by_cases he : marker.enters
  · simp only [step, he, ite_eq_left] at hfile
    rw [mem_stackNames_cons] at hfile
    rcases hfile with h | h
    · simp only [OriginFile.name, Option.some.injEq] at h
      exact Or.inl ⟨he, h.symm⟩
    · exact Or.inr h
  · refine Or.inr ?_
    by_cases hr : marker.returns
    · cases stack with
      | nil => simp [step, he, hr, stackNames] at hfile
      | cons a tail =>
          cases tail with
          | nil => simpa [step, he, hr] using hfile
          | cons outer rest =>
              simp only [step, he, hr, Bool.false_eq_true, ite_eq_right, ite_eq_left,
                not_false_eq_true] at hfile
              rw [mem_stackNames_cons] at hfile
              rw [mem_stackNames_cons, mem_stackNames_cons]
              exact Or.inr hfile
    · cases stack with
      | nil => simp [step, he, hr, stackNames, OriginFile.name] at hfile
      | cons frame rest =>
          simp only [step, he, hr, Bool.false_eq_true, ite_eq_right, not_false_eq_true] at hfile
          rw [mem_stackNames_cons] at hfile
          rw [mem_stackNames_cons]
          exact hfile

/-- A content line changes no presentable spelling. -/
theorem content_keeps_the_stack_names (reader : Reader) :
    stackNames (content reader).2.stack = stackNames reader.stack := by
  obtain ⟨stack, broken⟩ := reader
  cases stack with
  | nil => simp [content]
  | cons frame rest => simp [content, stackNames, List.filterMap_cons]

/-- A content line presents only a spelling already on the stack. -/
theorem content_presents_a_stack_name (reader : Reader) (coordinate : Coordinate)
    (file : FileName) (hc : (content reader).1 = some coordinate)
    (hf : coordinate.file.name = some file) : file ∈ stackNames reader.stack := by
  obtain ⟨stack, broken⟩ := reader
  cases stack with
  | nil => simp [content] at hc
  | cons frame rest =>
      by_cases hr : frame.relabelled
      · simp [content, hr] at hc
      · simp only [content, hr, Bool.false_eq_true, ite_eq_right, not_false_eq_true,
          Option.some.injEq] at hc
        subst hc
        simp only [stackNames, List.filterMap_cons]
        simp only at hf
        rw [hf]
        simp

/-- **The anti-forgery theorem.** Every file spelling a run presents was written
by a marker that *entered* a file, or was already presentable on the reader's
stack when the run began. A flagless marker's spelling is in neither set, so a
`#line` directive in analysed source can name no file in a presented
coordinate — which is the presentation half of standing constraint 12. -/
theorem presented_files_are_entered_files :
    ∀ (items : List Line) (reader : Reader) (file : FileName),
      file ∈ presentedFiles (readings items reader) →
        file ∈ entered items ∨ file ∈ stackNames reader.stack := by
  intro items
  induction items with
  | nil => intro reader file h; simp [readings, presentedFiles] at h
  | cons item rest ih =>
      intro reader file h
      cases item with
      | mark marker number =>
          simp only [readings] at h
          rcases ih (step reader marker number) file h with hrest | hstack
          · left
            by_cases he : marker.enters <;> simp [entered, he, hrest]
          · rcases step_adds_only_an_entering_spelling reader marker number file hstack with
              ⟨he, rfl⟩ | hprev
            · left; simp [entered, he]
            · right; exact hprev
      | content text =>
          simp only [readings, presentedFiles, List.filterMap_cons] at h
          have htail : file ∈ presentedFiles (readings rest (content reader).2) →
              file ∈ entered rest ∨ file ∈ stackNames reader.stack := by
            intro hf
            rcases ih (content reader).2 file hf with hr | hs
            · exact Or.inl hr
            · exact Or.inr (by rwa [content_keeps_the_stack_names] at hs)
          cases hc : (content reader).1 with
          | none =>
              rw [hc] at h
              rcases htail (by simpa [presentedFiles] using h) with hr | hs
              · left; simpa [entered] using hr
              · right; exact hs
          | some coordinate =>
              rw [hc] at h
              cases hf : coordinate.file.name with
              | none =>
                  simp only [hf] at h
                  rcases htail (by simpa [presentedFiles] using h) with hr | hs
                  · left; simpa [entered] using hr
                  · right; exact hs
              | some named =>
                  simp only [hf, List.mem_cons] at h
                  rcases h with rfl | h
                  · right; exact content_presents_a_stack_name reader coordinate file hc hf
                  · rcases htail (by simpa [presentedFiles] using h) with hr | hs
                    · left; simpa [entered] using hr
                    · right; exact hs

/-- The corollary at the reader a real run starts from: presented spellings are
entered spellings, full stop. -/
theorem a_flagless_marker_names_no_presented_file (items : List Line) (file : FileName)
    (h : file ∈ presentedFiles (coordinates items)) : file ∈ entered items := by
  rcases presented_files_are_entered_files items start file h with hr | hs
  · exact hr
  · simp [start, stackNames] at hs

/-! ## The measured streams, transcribed

These are the streams `EnumerationAttribution` already records, with the line
numbers Clang writes beside them. -/

/-- An ordinary include: `# 1 "<stdin>"`, `# 1 "inc/h.h" 1`, the header's text,
`# 4 "<stdin>" 2`, the main source's text. -/
def ordinaryInclude : List Line :=
  [.mark (relabel "<stdin>") 1, .mark (enter "inc/h.h") 1, .content "headerFn",
   .mark (leave "<stdin>") 4, .content "mainFn"]

/-- A header that writes the primary input's own spelling: the `#line 100
"<stdin>"` inside it is re-emitted flagless. -/
def relabelledHeader : List Line :=
  [.mark (relabel "<stdin>") 1, .mark (enter "h.h") 1, .mark (relabel "<stdin>") 100,
   .content "headerFn", .mark (leave "<stdin>") 3, .content "mainFn"]

/-- A main source that renames its own tail to a foreign spelling. -/
def relabelledTail : List Line :=
  [.mark (relabel "<stdin>") 1, .content "before", .mark (relabel "far.h") 500,
   .content "after"]

/-- Clang's own resynchronisation inside one file, which is also the shape a
bare `#line 500` (no file spelling) is re-emitted as. -/
def agreeingResync : List Line :=
  [.mark (relabel "<stdin>") 1, .content "before", .mark (relabel "<stdin>") 500,
   .content "after"]

/-! ## Depth 0 is never named by a marker -/

/-- The primary input presents `primary`: the caller's own name, not the
opening marker's `<stdin>`. The header beside it presents its entering marker's
spelling. -/
theorem the_primary_frame_presents_no_marker_spelling :
    coordinates ordinaryInclude
      = [("headerFn", some ⟨1, .included "inc/h.h", 1⟩), ("mainFn", some ⟨0, .primary, 4⟩)] := by
  decide

/-! ## A disagreeing flagless marker withdraws -/

/-- **Direction one.** A `#line "<stdin>"` inside a header does not hand the
header's line the main file — it withdraws the coordinate entirely. The main
source's own line is unaffected. -/
theorem a_relabelled_header_presents_nothing :
    coordinates relabelledHeader
      = [("headerFn", none), ("mainFn", some ⟨0, .primary, 3⟩)] := by
  decide

/-- **Direction two.** A main source renaming its own tail keeps that tail in
the audited scope (`EnumerationAttribution.byDepth`) and presents nothing for
it. Both halves in one sentence: the scope is unmoved, the coordinate is
withdrawn. -/
theorem a_relabelled_tail_stays_in_scope_and_presents_nothing :
    coordinates relabelledTail = [("before", some ⟨0, .primary, 1⟩), ("after", none)]
      ∧ byDepth "<stdin>"
          [.tok "before", .mark (relabel "far.h"), .tok "after"] = ["before", "after"] := by
  decide

/-! ## An agreeing flagless marker only resynchronises

Non-vacuity: the withdrawal above is a real discrimination, not a reader that
withdraws on every flagless marker. -/

/-- A flagless marker carrying the frame's own name sets the counter and keeps
the frame. This is Clang's resync, and a `#line 500` with no file spelling —
the directive renumbering the file it is written in, which is its C11 §6.10.4
meaning and moves no file identity. -/
theorem an_agreeing_resync_sets_the_line :
    coordinates agreeingResync
      = [("before", some ⟨0, .primary, 1⟩), ("after", some ⟨0, .primary, 500⟩)] := by
  decide

/-- The two streams differ only in the name their second marker carries, and
they present different answers — so the name comparison is load-bearing. -/
theorem the_name_comparison_is_load_bearing :
    coordinates agreeingResync ≠ coordinates relabelledTail := by
  decide

/-! ## Lines advance by one, which is why an expansion presents its site -/

/-- Content lines of one frame advance the counter by one each, with no marker
between them. A macro invocation's expansion is written out at the invocation's
own line and carries no marker, so it reads as that line. -/
theorem content_lines_advance_by_one :
    coordinates [.mark (relabel "<stdin>") 7, .content "a", .content "b", .content "c"]
      = [("a", some ⟨0, .primary, 7⟩), ("b", some ⟨0, .primary, 8⟩),
         ("c", some ⟨0, .primary, 9⟩)] := by
  decide

/-! ## Fail-closed -/

/-- Content before any marker presents nothing: there is no frame to belong to,
which is the `primary-input-unidentified` ground. -/
theorem content_before_the_opening_marker_presents_nothing :
    coordinates [.content "a"] = [("a", none)] := by
  decide

/-- A return with nothing to return to breaks the reader, which is the
`unbalanced-include-markers` ground; the whole map is then refused. -/
theorem an_underflowing_return_breaks_the_reader :
    (step (step start (relabel "<stdin>") 1) (leave "<stdin>") 1).broken = true := by
  decide

/-- The well-formed control does not break, so the refusal above is not
vacuous. -/
theorem a_well_formed_return_does_not_break_the_reader :
    (step (step (step start (relabel "<stdin>") 1) (enter "h.h") 1) (leave "<stdin>") 4).broken
      = false := by
  decide

/-! ## §L21.1: the refusal grounds' subject

All four grounds of `LocationRefusal` are failures of this analysis machinery.
None is a statement about the analysed source, so none can contribute to a
provisional `Unsafe`. -/

/-- The four grounds, as the Rust enumeration spells them. -/
inductive Refusal where
  | expansionUnavailable
  | primaryInputUnidentified
  | unbalancedIncludeMarkers
  | contentDisagreement
  deriving DecidableEq, Repr

/-- Every ground's §L21.1 category. -/
def Refusal.category : Refusal → Category
  | _ => .environmentUnavailable

/-- Every refusal this reader can publish speaks about the environment. -/
theorem every_location_refusal_says_nothing_about_the_source (refusal : Refusal) :
    refusal.category.subject = Subject.environment := by
  cases refusal <;> decide

/-- …and that is genuinely a different subject from a source-subject category,
so the statement is not vacuous. -/
theorem the_environment_subject_is_not_the_source_subject :
    Subject.environment ≠ Subject.analysedSource := by
  decide

end QCore.SourceCoordinate
