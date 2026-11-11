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

/-! `qcore_io` — the single emission route's decision procedure
(card `TRACE-STDIO`).

# What is proved here, and what is not

Card `TRACE-STDIO` moved four synthesized C backends' trace instrumentation and
`source.std-io.v1`'s `print`/`log`/`flush` onto one C-level emission route, so
an emitted program contains exactly one standard-output spelling: the one in
the route's own prelude. The Rust side states that property as a **decision
procedure** — `qcore_core::io_emit::direct_stdio_calls` — which scans an emitted
program and returns every line, outside the prelude, that carries a direct
output call.

This module models that procedure and proves the three things a reader must be
able to rely on when a test reports "no violations":

* **Soundness** — an empty result really means no line carries a spelling
  (`empty_means_no_direct_call`), so a green test is not a scan that gave up.
* **Completeness** — any line that does carry one is reported
  (`a_direct_call_is_always_reported`), so a second printing route cannot
  reappear unseen.
* **Maximality of the reported spelling** — the spelling named in a violation
  is one of greatest length among those the line contains
  (`the_reported_spelling_is_maximal`). This is not decoration: `fprintf`
  contains `printf` and `fputs` contains `puts`, and the first implementation
  of this procedure reported the short one. The property is what makes the
  ordering of the spelling table load-bearing rather than incidental.

Two further properties bear directly on how backends use the route:

* **Exemption is whole-line identity, not similarity**
  (`a_look_alike_line_is_not_exempt`) — a macro named like the route's does not
  buy an exemption.
* **Composition** (`concatenation_is_violation_free_iff_both_are`) — a program
  assembled from a prelude and several backend fragments is violation-free
  exactly when every fragment is, which is why installing the prelude once per
  program and routing every write is sufficient.

**Not proved here.** That the Rust emitters actually call this procedure's
subject (they are tested, not modeled); that the printed *bytes* are unchanged
by the migration (that is measured, before and after, by the Rust tests and by
F02's re-verified relation — a model cannot establish it); anything about C's
`fprintf` semantics, buffering, stream interleaving or durability. The spelling
table is a declared list, not a proof that it enumerates every way C can write
to a stream: a program could reach a stream through a function pointer or a
platform call this table does not name, and that is a stated boundary of the
procedure, on both sides.

The domain digest at the end ties this model to the Rust one: both sides walk
the same finite input domain and compute the same number independently. -/

namespace QCore.SingleEmissionRoute

/-- A line is a character list; a spelling is one too. Modeling the subject as
characters rather than an opaque `String` is what makes "contains" a relation
with a proof rather than an appeal to a library primitive. -/
abbrev Line := List Char

/-- `IsPrefix` as a decision, with its correctness proved below. -/
def prefixOf : Line → Line → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => if a = b then prefixOf as bs else false

/-- `containsInfix s l` — the spelling `s` occurs somewhere in the line `l`.
This is the whole content of "the line carries a direct output call". -/
def containsInfix (s : Line) : Line → Bool
  | [] => prefixOf s []
  | b :: bs => prefixOf s (b :: bs) || containsInfix s bs

theorem prefixOf_iff (s l : Line) : prefixOf s l = true ↔ s <+: l := by
  induction s generalizing l with
  | nil => simp [prefixOf, List.nil_prefix]
  | cons a as ih =>
    cases l with
    | nil => simp [prefixOf]
    | cons b bs =>
      by_cases h : a = b
      · subst h
        simp [prefixOf, ih, List.cons_prefix_cons]
      · simp [prefixOf, h, List.cons_prefix_cons]

theorem containsInfix_iff (s l : Line) : containsInfix s l = true ↔ s <:+: l := by
  induction l with
  | nil =>
    simp only [containsInfix, prefixOf_iff]
    constructor
    · intro h; exact h.isInfix
    · intro h
      have := h.sublist
      have : s = [] := List.eq_nil_of_sublist_nil this
      subst this; simp
  | cons b bs ih =>
    simp only [containsInfix, Bool.or_eq_true, prefixOf_iff, ih]
    constructor
    · intro h
      cases h with
      | inl hp => exact hp.isInfix
      | inr hi => exact hi.trans (List.suffix_cons b bs).isInfix
    · intro h
      obtain ⟨front, back, hEq⟩ := h
      cases front with
      | nil =>
        left
        exact ⟨back, by simpa using hEq⟩
      | cons c cs =>
        right
        refine ⟨cs, back, ?_⟩
        simpa using congrArg List.tail hEq

/-- The spelling table, longest first. The order is the procedure's, not a
convenience: `the_reported_spelling_is_maximal` is proved *from* it. -/
def spellings : List Line :=
  [ "vfprintf".toList
  , "fprintf".toList
  , "vprintf".toList
  , "printf".toList
  , "fwrite".toList
  , "fputs".toList
  , "puts".toList ]

/-- The table really is ordered by non-increasing length. Decided, not assumed;
reordering the table breaks this and therefore breaks maximality. -/
theorem spellings_sorted_by_decreasing_length :
    spellings.Pairwise (fun a b => b.length ≤ a.length) := by
  decide

theorem every_spelling_is_nonempty : ∀ s ∈ spellings, s ≠ [] := by
  decide

/-- The per-line half of the procedure: the first spelling of the table the line
contains, if any. -/
def scanLine (l : Line) : Option Line :=
  spellings.find? (fun s => containsInfix s l)

/-- The whole procedure: every non-exempt line that carries a spelling, paired
with its one-based position. `exempt` is the prelude's own text. -/
def violations (exempt : List Line) : Nat → List Line → List (Nat × Line)
  | _, [] => []
  | index, l :: rest =>
    let tail := violations exempt (index + 1) rest
    if exempt.contains l then tail
    else
      match scanLine l with
      | some s => (index, s) :: tail
      | none => tail

/-- The published entry point, matching the Rust signature's one-based lines. -/
def directStdioCalls (exempt : List Line) (program : List Line) : List (Nat × Line) :=
  violations exempt 1 program

/-! ## Soundness, completeness and maximality of the line scan -/

theorem scanLine_none_iff (l : Line) :
    scanLine l = none ↔ ∀ s ∈ spellings, ¬ s <:+: l := by
  simp [scanLine, List.find?_eq_none, containsInfix_iff]

theorem scanLine_some_contains {l s : Line} (h : scanLine l = some s) : s <:+: l := by
  have := List.find?_some h
  simpa [containsInfix_iff] using this

theorem scanLine_some_mem {l s : Line} (h : scanLine l = some s) : s ∈ spellings :=
  List.mem_of_find?_eq_some h

/-- **Completeness.** A line containing any table spelling is reported. -/
theorem a_direct_call_is_always_reported {l s : Line}
    (hs : s ∈ spellings) (hi : s <:+: l) : ∃ t, scanLine l = some t := by
  cases hScan : scanLine l with
  | none =>
    exact absurd hi ((scanLine_none_iff l).mp hScan s hs)
  | some t => exact ⟨t, rfl⟩

/-- The maximality lemma, over an arbitrary table sorted by non-increasing
length. Instantiated at `spellings` below. -/
theorem find?_sorted_is_maximal {ss : List Line} {p : Line → Bool} {s : Line}
    (hSorted : ss.Pairwise (fun a b => b.length ≤ a.length))
    (hFind : ss.find? p = some s) :
    ∀ t ∈ ss, p t = true → t.length ≤ s.length := by
  induction ss with
  | nil => simp at hFind
  | cons a as ih =>
    obtain ⟨hHead, hTail⟩ := List.pairwise_cons.mp hSorted
    cases ha : p a with
    | true =>
      simp only [List.find?_cons, ha] at hFind
      cases hFind
      intro t ht _
      cases List.mem_cons.mp ht with
      | inl hEq => exact hEq ▸ Nat.le_refl _
      | inr hMem => exact hHead t hMem
    | false =>
      simp only [List.find?_cons, ha] at hFind
      intro t ht hp
      cases List.mem_cons.mp ht with
      | inl hEq => rw [hEq] at hp; rw [ha] at hp; exact absurd hp (by simp)
      | inr hMem => exact ih hTail hFind t hMem hp

/-- **Maximality.** The spelling a violation names is one of greatest length
among those the line actually contains — so a line writing through `fprintf` is
never reported as `printf`. -/
theorem the_reported_spelling_is_maximal {l s : Line} (h : scanLine l = some s) :
    ∀ t ∈ spellings, t <:+: l → t.length ≤ s.length := by
  intro t ht hi
  exact find?_sorted_is_maximal spellings_sorted_by_decreasing_length h t ht
    (by simpa [containsInfix_iff] using hi)

/-- The two spelling pairs that made maximality necessary. Decided outright. -/
theorem the_containing_pairs_are_really_present :
    "printf".toList <:+: "fprintf".toList ∧ "puts".toList <:+: "fputs".toList ∧
      "printf".toList <:+: "vfprintf".toList := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [← containsInfix_iff] <;> decide

theorem an_fprintf_line_is_reported_as_fprintf :
    scanLine "    fprintf(stderr, \"x\");".toList = some "fprintf".toList := by
  decide

theorem an_fputs_line_is_reported_as_fputs :
    scanLine "    fputs(\"x\", stdout);".toList = some "fputs".toList := by
  decide

/-! ## The whole-program procedure -/

/-- **Soundness.** No violation means no non-exempt line carries a spelling. -/
theorem empty_means_no_direct_call {exempt : List Line} {index : Nat}
    {program : List Line} (h : violations exempt index program = []) :
    ∀ l ∈ program, exempt.contains l = true ∨ scanLine l = none := by
  induction program generalizing index with
  | nil => intro l hl; simp at hl
  | cons a rest ih =>
    intro l hl
    simp only [violations] at h
    by_cases hExempt : exempt.contains a = true
    · rw [ite_eq_left hExempt] at h
      cases List.mem_cons.mp hl with
      | inl hEq => exact hEq ▸ Or.inl hExempt
      | inr hMem => exact ih h l hMem
    · rw [ite_eq_right hExempt] at h
      cases hScan : scanLine a with
      | some s => rw [hScan] at h; simp at h
      | none =>
        rw [hScan] at h
        cases List.mem_cons.mp hl with
        | inl hEq => exact hEq ▸ Or.inr hScan
        | inr hMem => exact ih h l hMem

/-- The converse: a program whose every line is exempt or spelling-free has no
violation. Together with the above this is an exact characterization. -/
theorem no_direct_call_means_empty {exempt : List Line} {index : Nat}
    {program : List Line}
    (h : ∀ l ∈ program, exempt.contains l = true ∨ scanLine l = none) :
    violations exempt index program = [] := by
  induction program generalizing index with
  | nil => rfl
  | cons a rest ih =>
    have hHead := h a (by simp)
    have hRest : ∀ l ∈ rest, exempt.contains l = true ∨ scanLine l = none :=
      fun l hl => h l (List.mem_cons_of_mem a hl)
    simp only [violations]
    by_cases hExempt : exempt.contains a = true
    · rw [ite_eq_left hExempt]; exact ih hRest
    · rw [ite_eq_right hExempt]
      cases hHead with
      | inl hc => exact absurd hc hExempt
      | inr hScan => rw [hScan]; exact ih hRest

theorem violation_free_iff {exempt : List Line} {program : List Line} :
    directStdioCalls exempt program = [] ↔
      ∀ l ∈ program, exempt.contains l = true ∨ scanLine l = none :=
  ⟨empty_means_no_direct_call, no_direct_call_means_empty⟩

/-- **Composition.** A program assembled from fragments is violation-free
exactly when every fragment is. This is why a backend installs the prelude once
and routes each write, rather than needing a whole-program argument. -/
theorem concatenation_is_violation_free_iff_both_are
    {exempt : List Line} {a b : List Line} :
    directStdioCalls exempt (a ++ b) = [] ↔
      directStdioCalls exempt a = [] ∧ directStdioCalls exempt b = [] := by
  simp only [violation_free_iff, List.mem_append]
  constructor
  · intro h
    exact ⟨fun l hl => h l (Or.inl hl), fun l hl => h l (Or.inr hl)⟩
  · rintro ⟨ha, hb⟩ l (hl | hl)
    · exact ha l hl
    · exact hb l hl

/-! ## Exemption is identity, not similarity -/

/-- The route's two definitions, as the Rust prelude spells them. -/
def preludeDefines : List Line :=
  [ "#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))".toList
  , "#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))".toList ]

theorem the_prelude_definitions_are_exempt :
    directStdioCalls preludeDefines preludeDefines = [] := by
  decide

/-- **A look-alike is not exempt.** A macro named `QCORE_IO_WRITER` that writes
directly is reported, because exemption compares whole lines. -/
theorem a_look_alike_line_is_not_exempt :
    directStdioCalls preludeDefines
        ["#define QCORE_IO_WRITER(s, ...) fprintf(s, __VA_ARGS__)".toList] =
      [(1, "fprintf".toList)] := by
  decide

/-- Non-vacuity in the other direction: a routed write is not a violation, and
the check is therefore not trivially empty on real emitted text. -/
theorem a_routed_write_is_not_a_violation :
    directStdioCalls preludeDefines ["    QCORE_IO_WRITE(stdout, \"x\\n\");".toList] = [] := by
  decide

/-- Positions are one-based and count exempt lines, so a reported violation
names the line a reader will find in the emitted file. -/
theorem positions_are_one_based_and_count_every_line :
    directStdioCalls preludeDefines
        [ "#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))".toList
        , "    QCORE_IO_WRITE(stdout, \"x\\n\");".toList
        , "    printf(\"x\\n\");".toList ] =
      [(3, "printf".toList)] := by
  decide

/-! ## The two-sided domain tie

Both this model and `qcore_core::io_emit` walk the same finite domain of inputs
and report the same outcome vector. The two are computed independently and
compared; agreement is evidence that the model above and the procedure the
tests run decide the same thing, at least on this domain.

Kernel reduction over eleven character lists needs more recursion depth than
the default. -/
set_option maxRecDepth 8000

/-- The domain: every spelling, each embedded in a fixed line shape, plus the
routed write and the two prelude definitions. -/
def domain : List Line :=
  (spellings.map (fun s => "    ".toList ++ s ++ "(stdout, \"x\");".toList)) ++
    [ "    QCORE_IO_WRITE(stdout, \"x\\n\");".toList
    , "    QCORE_IO_FLUSH(stderr);".toList ] ++ preludeDefines

/-- The outcome this procedure reports for one line, reduced to the one number
that carries the maximality claim: the reported spelling's length, or `0` when
the line is exempt or carries no direct call. -/
def domainOutcome (l : Line) : Nat :=
  match directStdioCalls preludeDefines [l] with
  | [] => 0
  | (_, s) :: _ => s.length

/-- The pinned two-sided tie. `qcore_core::io_emit`'s test computes this same
vector from its own procedure over the same eleven lines and compares it; the
two are computed independently, so agreement is evidence that the model above
and the procedure the tests run decide the same thing on this domain.

Reading it: the seven spelling lines report 8, 7, 7, 6, 6, 5 and 4 — the
lengths of `vfprintf`, `fprintf`, `vprintf`, `printf`, `fwrite`, `fputs` and
`puts`. The `fputs` line reports **5**, not the 4 a naive scan would report for
the `puts` it also contains, and that single entry is the maximality property
made observable. The last four report `0`: two routed writes carry no direct
call, and the two prelude definitions are exempt. -/
theorem the_domain_outcomes_are_pinned :
    domain.map domainOutcome = [8, 7, 7, 6, 6, 5, 4, 0, 0, 0, 0] := by decide

theorem domain_has_eleven_lines : domain.length = 11 := by decide

end QCore.SingleEmissionRoute
