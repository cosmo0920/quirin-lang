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

/-! Explicit bundled-module selection. This models exact name resolution and
capability admission; source token extraction and per-file driver wiring are
tested Rust boundaries, not consequences of these abstract theorems.

Card `STD-MODULES` generalises the registry from two modules to five. The
existing theorem *names* and *meanings* are preserved; their structure literals
grew three fields, which is a deliberate widening recorded in that card's
report. Nothing about the concurrency or extern capabilities changed. -/
namespace QCore.ModuleImports

structure Imports where
  concurrency : Bool := false
  /-- Card `source.extern-call.v1`: calling a declared extern function is a
  second, independent file-local opt-in. -/
  externCalls : Bool := false
  /-- Card `UNSAFE-LINKAGE`: declaring an extern function with **no** contract,
  so an external library can be linked without one. A third, independent
  file-local opt-in: `std.extern` never grants it and it never grants
  `std.extern`. -/
  uncheckedExternCalls : Bool := false
  /-- Card `STD-MODULES`: naming a rendering. Opens no stream on its own. -/
  fmt : Bool := false
  /-- Card `STD-MODULES`: writing a rendered line to standard output. -/
  io : Bool := false
  /-- Card `STD-MODULES`: writing a level-tagged line to standard error, and
  the `flush` sink-handoff barrier. -/
  logger : Bool := false
  /-- Card `STD-BUFFER`: acquiring, indexing, measuring or releasing an owned
  byte buffer. A seventh independent file-local opt-in, granted by no other
  module and granting none: `std.io` never grants it and it never grants
  `std.io`. -/
  buffer : Bool := false
  /-- Card `STRING-SLICE`: constructing an immutable UTF-8 string view, reading
  its byte length or slicing it. An eighth independent file-local opt-in,
  granted by no other module and granting none — in particular not `std.buffer`,
  whose value is owned and mutable where this one is an unowned immutable view,
  and not the output modules, which this amendment does not make accept text. -/
  string : Bool := false
  /-- Card `QUIRIN-SOCKET-V1`: creating, binding, listening, connecting,
  accepting, sending on, receiving from or closing a socket. A ninth independent
  file-local opt-in, granted by no other module and granting none — in
  particular it opens no stream (`std.io` and `std.logger` still own every line)
  and it is not `std.buffer`, whose value is memory this program owns where a
  socket is a kernel endpoint it merely holds a descriptor for. -/
  socket : Bool := false
  /-- Card `HTTP-QUIRIN-V1`: declaring an HTTP/1.1 request or response, adding a
  header field to one, or setting its content. A tenth independent file-local
  opt-in, granted by no other module and granting none.

  Two independences are the point rather than a detail. It grants **no
  transport**: a message is a value, and putting one on a wire is `std.socket`'s,
  so a program that actually speaks HTTP names both. And it grants **no text**: a
  quoted token at a method, a target, a field name, a field value or a content
  position is this module's own syntax and constructs no `string` value, which is
  the case `STRING_SLICE_V1.md` §2 excludes from the string module's own gate. -/
  http : Bool := false
  deriving DecidableEq, Repr

/-- The empty selection: every capability withheld. -/
def none : Imports := ⟨false, false, false, false, false, false, false, false, false, false⟩

/-- The field one module name sets. Total-or-reject: a name outside the
registry has no setter, which is what makes `resolve` refuse it. -/
def setterFor (name : String) : Option (Imports → Imports) :=
  if name = "std.concurrency" then some (fun i => { i with concurrency := true })
  else if name = "std.extern" then some (fun i => { i with externCalls := true })
  else if name = "std.extern.unsafe" then
    some (fun i => { i with uncheckedExternCalls := true })
  else if name = "std.fmt" then some (fun i => { i with fmt := true })
  else if name = "std.io" then some (fun i => { i with io := true })
  else if name = "std.logger" then some (fun i => { i with logger := true })
  else if name = "std.buffer" then some (fun i => { i with buffer := true })
  else if name = "std.string" then some (fun i => { i with string := true })
  else if name = "std.socket" then some (fun i => { i with socket := true })
  else if name = "std.http" then some (fun i => { i with http := true })
  else Option.none

def resolve : List String → Option Imports
  | [] => some none
  | name :: rest =>
    match setterFor name with
    | some set => (resolve rest).map set
    | Option.none => Option.none

def admits (imports : Imports) (needsConcurrency : Bool) : Bool :=
  !needsConcurrency || imports.concurrency

/-- The extern-call capability is gated by its own module, never by another. -/
def admitsExternCall (imports : Imports) (callsExtern : Bool) : Bool :=
  !callsExtern || imports.externCalls

/-- Card `UNSAFE-LINKAGE`: declaring an extern function with no contract is
gated by its own module, never by the call module. The gate is on the
*declaration*, because declaring an uncontracted symbol is itself the trust
act. -/
def admitsUncheckedExtern (imports : Imports) (declaresUnchecked : Bool) : Bool :=
  !declaresUnchecked || imports.uncheckedExternCalls

/-- Writing to standard output is gated by `std.io`, never by another module. -/
def admitsPrint (imports : Imports) (prints : Bool) : Bool :=
  !prints || imports.io

/-- Logging and the barrier are gated by `std.logger`, never by another. -/
def admitsLog (imports : Imports) (logs : Bool) : Bool :=
  !logs || imports.logger

/-- Naming a rendering is gated by `std.fmt`, never by another. -/
def admitsRendering (imports : Imports) (renders : Bool) : Bool :=
  !renders || imports.fmt

/-- Card `STD-BUFFER`: holding an owned byte buffer is gated by `std.buffer`,
never by another. -/
def admitsBuffer (imports : Imports) (buffers : Bool) : Bool :=
  !buffers || imports.buffer

theorem empty_imports_grant_no_concurrency : resolve [] = some none := rfl

theorem empty_imports_grant_no_extern_calls :
    (resolve []).map Imports.externCalls = some false := rfl

theorem empty_imports_grant_no_output :
    (resolve []).map Imports.io = some false := rfl

theorem empty_imports_grant_no_logger :
    (resolve []).map Imports.logger = some false := rfl

theorem empty_imports_grant_no_rendering :
    (resolve []).map Imports.fmt = some false := rfl

theorem unknown_module_rejects (name : String) (rest : List String)
    (unknownConcurrency : name ≠ "std.concurrency")
    (unknownExtern : name ≠ "std.extern")
    (unknownUncheckedExtern : name ≠ "std.extern.unsafe")
    (unknownFmt : name ≠ "std.fmt")
    (unknownIo : name ≠ "std.io")
    (unknownLogger : name ≠ "std.logger")
    (unknownBuffer : name ≠ "std.buffer")
    (unknownString : name ≠ "std.string")
    (unknownSocket : name ≠ "std.socket")
    (unknownHttp : name ≠ "std.http") : resolve (name :: rest) = Option.none := by
  simp [resolve, setterFor, unknownConcurrency, unknownExtern,
    unknownUncheckedExtern, unknownFmt, unknownIo, unknownLogger, unknownBuffer,
    unknownString, unknownSocket, unknownHttp]

/-- **The capability-gating theorem.** If resolution enabled a capability, the
module that owns it was named. Stated once over an arbitrary field and
instantiated five times below, so every module is gated by exactly the same
argument and none can drift. -/
theorem admitted_capability_requires_its_module
    (field : Imports → Bool) (module : String)
    (base : field none = false)
    (only : ∀ name set, setterFor name = some set → name ≠ module →
      ∀ i, field (set i) = field i)
    {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : field imports = true) :
    module ∈ names := by
  induction names generalizing imports with
  | nil =>
    simp only [resolve, Option.some.injEq] at resolved
    subst resolved
    simp [base] at enabled
  | cons name rest ih =>
    by_cases named : name = module
    · simp [named]
    · refine List.mem_cons_of_mem _ ?_
      cases setter : setterFor name with
      | none => simp [resolve, setter] at resolved
      | some set =>
        cases inner : resolve rest with
        | none => simp [resolve, setter, inner] at resolved
        | some i =>
          simp only [resolve, setter, inner, Option.map_some, Option.some.injEq] at resolved
          subst resolved
          exact ih inner (by rw [← only name set setter named i]; exact enabled)

private theorem setter_preserves_other_fields
    (field : Imports → Bool) (module : String)
    (h : ∀ i, (field { i with concurrency := true } = field i ∨ module = "std.concurrency")
      ∧ (field { i with externCalls := true } = field i ∨ module = "std.extern")
      ∧ (field { i with uncheckedExternCalls := true } = field i ∨ module = "std.extern.unsafe")
      ∧ (field { i with fmt := true } = field i ∨ module = "std.fmt")
      ∧ (field { i with io := true } = field i ∨ module = "std.io")
      ∧ (field { i with logger := true } = field i ∨ module = "std.logger")
      ∧ (field { i with buffer := true } = field i ∨ module = "std.buffer")
      ∧ (field { i with string := true } = field i ∨ module = "std.string")
      ∧ (field { i with socket := true } = field i ∨ module = "std.socket")
      ∧ (field { i with http := true } = field i ∨ module = "std.http")) :
    ∀ name set, setterFor name = some set → name ≠ module →
      ∀ i, field (set i) = field i := by
  intro name set setter named i
  simp only [setterFor] at setter
  split at setter
  · next eq => subst eq
               injection setter with setter; subst setter
               rcases (h i).1 with p | p
               · exact p
               · exact absurd p.symm named
  · split at setter
    · next eq => subst eq
                 injection setter with setter; subst setter
                 rcases (h i).2.1 with p | p
                 · exact p
                 · exact absurd p.symm named
    · split at setter
      · next eq => subst eq
                   injection setter with setter; subst setter
                   rcases (h i).2.2.1 with p | p
                   · exact p
                   · exact absurd p.symm named
      · split at setter
        · next eq => subst eq
                     injection setter with setter; subst setter
                     rcases (h i).2.2.2.1 with p | p
                     · exact p
                     · exact absurd p.symm named
        · split at setter
          · next eq => subst eq
                       injection setter with setter; subst setter
                       rcases (h i).2.2.2.2.1 with p | p
                       · exact p
                       · exact absurd p.symm named
          · split at setter
            · next eq => subst eq
                         injection setter with setter; subst setter
                         rcases (h i).2.2.2.2.2.1 with p | p
                         · exact p
                         · exact absurd p.symm named
            · split at setter
              · next eq => subst eq
                           injection setter with setter; subst setter
                           rcases (h i).2.2.2.2.2.2.1 with p | p
                           · exact p
                           · exact absurd p.symm named
              · split at setter
                · next eq => subst eq
                             injection setter with setter; subst setter
                             rcases (h i).2.2.2.2.2.2.2.1 with p | p
                             · exact p
                             · exact absurd p.symm named
                · split at setter
                  · next eq => subst eq
                               injection setter with setter; subst setter
                               rcases (h i).2.2.2.2.2.2.2.2.1 with p | p
                               · exact p
                               · exact absurd p.symm named
                  · split at setter
                    · next eq => subst eq
                                 injection setter with setter; subst setter
                                 rcases (h i).2.2.2.2.2.2.2.2.2 with p | p
                                 · exact p
                                 · exact absurd p.symm named
                    · simp at setter

theorem admitted_concurrency_requires_import {names : List String} {imports : Imports}
    (resolved : resolve names = some imports) (enabled : imports.concurrency = true) :
    "std.concurrency" ∈ names :=
  admitted_capability_requires_its_module Imports.concurrency "std.concurrency" rfl
    (setter_preserves_other_fields _ _ (by intro i; refine ⟨Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_extern_calls_require_the_extern_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.externCalls = true) : "std.extern" ∈ names :=
  admitted_capability_requires_its_module Imports.externCalls "std.extern" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_rendering_requires_the_fmt_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.fmt = true) : "std.fmt" ∈ names :=
  admitted_capability_requires_its_module Imports.fmt "std.fmt" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_output_requires_the_io_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.io = true) : "std.io" ∈ names :=
  admitted_capability_requires_its_module Imports.io "std.io" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem admitted_logging_requires_the_logger_import {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.logger = true) : "std.logger" ∈ names :=
  admitted_capability_requires_its_module Imports.logger "std.logger" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl,
            Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl⟩))
    resolved enabled

theorem the_concurrency_import_alone_grants_no_extern_call :
    admitsExternCall ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false := rfl

/-- Card `STD-MODULES`: the three new capabilities are mutually independent.
Importing standard output does not grant the logger, and vice versa. -/
theorem the_io_import_alone_grants_no_logging :
    admitsLog ⟨false, false, false, true, true, false, false, false, false, false⟩ true = false := rfl

theorem the_logger_import_alone_grants_no_output :
    admitsPrint ⟨false, false, false, true, false, true, false, false, false, false⟩ true = false := rfl

/-- `std.fmt` opens no stream: holding it alone admits neither writer. -/
theorem the_fmt_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, true, false, false, false, false, false, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, true, false, false, false, false, false, false⟩ true = false := ⟨rfl, rfl⟩

/-- Neither writer's module grants the rendering vocabulary. -/
theorem neither_writer_grants_the_rendering :
    admitsRendering ⟨false, false, false, false, true, true, false, false, false, false⟩ true = false := rfl

/-- Concurrency grants none of the three, so a threaded module gets no free
output channel. -/
theorem concurrency_grants_no_output_capability :
    admitsPrint ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false
      ∧ admitsLog ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false
      ∧ admitsRendering ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

theorem sequential_code_needs_no_extern_import (imports : Imports) :
    admitsExternCall imports false = true := by simp [admitsExternCall]

theorem code_that_prints_nothing_needs_no_io_import (imports : Imports) :
    admitsPrint imports false = true := by simp [admitsPrint]

theorem code_that_logs_nothing_needs_no_logger_import (imports : Imports) :
    admitsLog imports false = true := by simp [admitsLog]

theorem code_that_renders_nothing_needs_no_fmt_import (imports : Imports) :
    admitsRendering imports false = true := by simp [admitsRendering]

theorem duplicate_import_is_idempotent (names : List String) :
    resolve ("std.concurrency" :: "std.concurrency" :: names) =
      resolve ("std.concurrency" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem duplicate_io_import_is_idempotent (names : List String) :
    resolve ("std.io" :: "std.io" :: names) = resolve ("std.io" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

theorem both_imports_are_independent :
    resolve ["std.concurrency", "std.extern"] =
      some ⟨true, true, false, false, false, false, false, false, false, false⟩ := by
  simp [resolve, setterFor, none]

theorem the_three_std_modules_are_independent :
    resolve ["std.fmt", "std.io", "std.logger"] =
      some ⟨false, false, false, true, true, true, false, false, false, false⟩ := by
  simp [resolve, setterFor, none]

/-- Card `UNSAFE-LINKAGE`. The waiver is granted by its own module and by no
other, and it grants no other capability in return: importing it does not let a
file call an extern function, and importing `std.extern` does not let a file
waive one. -/
theorem admitted_unchecked_linkage_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.uncheckedExternCalls = true) : "std.extern.unsafe" ∈ names :=
  admitted_capability_requires_its_module Imports.uncheckedExternCalls
    "std.extern.unsafe" rfl
    (setter_preserves_other_fields _ _
      (by intro i; refine ⟨Or.inl rfl, Or.inl rfl, Or.inr rfl, ?_⟩; simp))
    resolved enabled

theorem empty_imports_grant_no_unchecked_linkage :
    (resolve []).map Imports.uncheckedExternCalls = some false := rfl

theorem the_extern_import_alone_grants_no_waiver :
    admitsUncheckedExtern ⟨false, true, false, false, false, false, false, false, false, false⟩ true = false := rfl

theorem the_waiver_alone_grants_no_extern_call :
    admitsExternCall ⟨false, false, true, false, false, false, false, false, false, false⟩ true = false := rfl

/-- An unchecked-linkage file names both modules, and naming both grants
exactly both. -/
theorem the_two_extern_modules_are_independent :
    resolve ["std.extern", "std.extern.unsafe"] =
      some ⟨false, true, true, false, false, false, false, false, false, false⟩ := by
  simp [resolve, setterFor, none]

theorem code_that_waives_nothing_needs_no_waiver_import (imports : Imports) :
    admitsUncheckedExtern imports false = true := by simp [admitsUncheckedExtern]

/-- The waiver is not reachable through a near-miss spelling: `std.extern.unsafe`
is one registry entry, not a prefix rule over `std.extern`. -/
theorem a_near_miss_waiver_spelling_rejects :
    resolve ["std.extern.unsafe.please"] = Option.none := by
  simp [resolve, setterFor]

theorem missing_required_capability_rejects :
    admits ⟨false, false, false, false, false, false, false, false, false, false⟩ true = false := rfl

theorem sequential_code_needs_no_concurrency (imports : Imports) :
    admits imports false = true := by simp [admits]

/-! ## Card `STD-BUFFER`: the seventh module

The byte-buffer capability is gated by **the same generic argument** every other
module is gated by — `admitted_capability_requires_its_module`, instantiated
once more — so it cannot drift from the six that precede it. -/

theorem empty_imports_grant_no_buffer :
    (resolve []).map Imports.buffer = some false := rfl

theorem admitted_buffer_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.buffer = true) : "std.buffer" ∈ names :=
  admitted_capability_requires_its_module Imports.buffer "std.buffer" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl,
            Or.inr rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl⟩))
    resolved enabled

/-- `std.io` never grants the buffer, and `std.buffer` never grants a stream.
The two capabilities are independent in **both** directions, which is what makes
"a module that both buffers and prints names both modules" a fact rather than a
convention. -/
theorem the_io_import_alone_grants_no_buffer :
    admitsBuffer ⟨false, false, false, true, true, true, false, false, false, false⟩ true = false := rfl

theorem the_buffer_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, false, false, false, true, false, false, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, false, false, false, true, false, false, false⟩ true = false
      ∧ admitsRendering ⟨false, false, false, false, false, false, true, false, false, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

/-- Concurrency grants no buffer either, so a threaded module gets no free
byte storage any more than it gets a free output channel. -/
theorem concurrency_grants_no_buffer_capability :
    admitsBuffer ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false := rfl

/-- The extern modules grant no buffer, which is the registry-level half of the
route-level statement that a buffer does not cross the extern boundary. -/
theorem neither_extern_module_grants_the_buffer :
    admitsBuffer ⟨false, true, true, false, false, false, false, false, false, false⟩ true = false := rfl

theorem code_that_buffers_nothing_needs_no_buffer_import (imports : Imports) :
    admitsBuffer imports false = true := by simp [admitsBuffer]

theorem duplicate_buffer_import_is_idempotent (names : List String) :
    resolve ("std.buffer" :: "std.buffer" :: names) = resolve ("std.buffer" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

/-- Naming the buffer module beside the three output modules grants exactly
those four, and nothing else. -/
theorem the_buffer_module_is_independent_of_the_output_modules :
    resolve ["std.buffer", "std.fmt", "std.io", "std.logger"] =
      some ⟨false, false, false, true, true, true, true, false, false, false⟩ := by
  simp [resolve, setterFor, none]

/-- A near-miss spelling refuses: `std.buffer` is one registry entry, and the
constructor's own spelling (`bytes`) is not a module name at all. -/
theorem a_near_miss_buffer_spelling_rejects :
    resolve ["std.buffers"] = Option.none ∧ resolve ["std.bytes"] = Option.none := by
  constructor <;> simp [resolve, setterFor]

/-! ## Card `STRING-SLICE`: the eighth module

The string-view capability is gated by **the same generic argument** every other
module is gated by — `admitted_capability_requires_its_module`, instantiated once
more — so it cannot drift from the seven that precede it. -/

/-- Card `STRING-SLICE`: constructing, measuring or slicing an immutable UTF-8
string view is gated by `std.string`, never by another. -/
def admitsString (imports : Imports) (strings : Bool) : Bool :=
  !strings || imports.string

theorem empty_imports_grant_no_string :
    (resolve []).map Imports.string = some false := rfl

theorem admitted_string_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.string = true) : "std.string" ∈ names :=
  admitted_capability_requires_its_module Imports.string "std.string" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl,
            Or.inl rfl, Or.inr rfl, Or.inl rfl, Or.inl rfl⟩))
    resolved enabled

/-- **The §5 separation, as a registry fact.** `std.buffer` never grants the
string and `std.string` never grants the buffer. A buffer is owned and mutable
and a string is an unowned immutable view, so neither is implicitly the other —
and the conversion between them (`STRING-SLICE-BUFFER-BORROW`) is a capability
neither import supplies. -/
theorem the_buffer_import_alone_grants_no_string :
    admitsString ⟨false, false, false, false, false, false, true, false, false, false⟩ true = false := rfl

theorem the_string_import_alone_grants_no_buffer :
    admitsBuffer ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false := rfl

/-- **Amendment §7, as a registry fact.** Importing `std.string` opens no stream
and names no rendering: this amendment does not make `std.io` or `std.logger`
accept text, so a module that slices and prints a length names every module it
uses. -/
theorem the_string_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false
      ∧ admitsRendering ⟨false, false, false, false, false, false, false, true, false, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

theorem the_output_modules_grant_no_string :
    admitsString ⟨false, false, false, true, true, true, false, false, false, false⟩ true = false := rfl

theorem concurrency_grants_no_string_capability :
    admitsString ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false := rfl

/-- Neither extern module grants the string, which is the registry-level half of
the route-level statement that a string view does not cross the C04 boundary
(`STRING-SLICE-ABI`). -/
theorem neither_extern_module_grants_the_string :
    admitsString ⟨false, true, true, false, false, false, false, false, false, false⟩ true = false := rfl

theorem code_that_strings_nothing_needs_no_string_import (imports : Imports) :
    admitsString imports false = true := by simp [admitsString]

theorem duplicate_string_import_is_idempotent (names : List String) :
    resolve ("std.string" :: "std.string" :: names) = resolve ("std.string" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

/-- Naming the string module beside the buffer module and the three output
modules grants exactly those five, and nothing else. -/
theorem the_string_module_is_independent_of_its_neighbours :
    resolve ["std.string", "std.buffer", "std.fmt", "std.io", "std.logger"] =
      some ⟨false, false, false, true, true, true, true, true, false, false⟩ := by
  simp [resolve, setterFor, none]

/-- A near-miss spelling refuses: `std.string` is one registry entry, and the
type name the module supplies is not a module name at all. -/
theorem a_near_miss_string_spelling_rejects :
    resolve ["std.strings"] = Option.none ∧ resolve ["std.text"] = Option.none := by
  constructor <;> simp [resolve, setterFor]


/-! ## Card `QUIRIN-SOCKET-V1`: the ninth module

The socket capability is gated by **the same generic argument** every other
module is gated by — `admitted_capability_requires_its_module`, instantiated once
more — so it cannot drift from the eight that precede it. -/

/-- Card `QUIRIN-SOCKET-V1`: creating, binding, listening, connecting,
accepting, sending on, receiving from or closing a socket is gated by
`std.socket`, never by another. -/
def admitsSocket (imports : Imports) (sockets : Bool) : Bool :=
  !sockets || imports.socket

theorem empty_imports_grant_no_socket :
    (resolve []).map Imports.socket = some false := rfl

theorem admitted_socket_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.socket = true) : "std.socket" ∈ names :=
  admitted_capability_requires_its_module Imports.socket "std.socket" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl,
            Or.inl rfl, Or.inl rfl, Or.inr rfl, Or.inl rfl⟩))
    resolved enabled

/-- **The route's own separation, as a registry fact.** `std.socket` opens no
stream and names no rendering: a module that receives a byte and prints it names
every module it uses. -/
theorem the_socket_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false
      ∧ admitsLog ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false
      ∧ admitsRendering ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

theorem the_output_modules_grant_no_socket :
    admitsSocket ⟨false, false, false, true, true, true, false, false, false, false⟩ true = false := rfl

/-- Neither the buffer nor the string module grants the socket, and the socket
module grants neither of them. That is the registry-level half of the route-level
statement that a payload is a list of check-time byte constants
(`SOCKET-BUFFER-PAYLOAD`, `SOCKET-STRING-PAYLOAD`). -/
theorem the_socket_module_is_independent_of_the_value_modules :
    admitsSocket ⟨false, false, false, false, false, false, true, true, false, false⟩ true = false
      ∧ admitsBuffer ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false
      ∧ admitsString ⟨false, false, false, false, false, false, false, false, true, false⟩ true = false :=
  ⟨rfl, rfl, rfl⟩

theorem concurrency_grants_no_socket_capability :
    admitsSocket ⟨true, false, false, false, false, false, false, false, false, false⟩ true = false := rfl

/-- Neither extern module grants the socket: a socket is not reached through a
declared C04 boundary on this route, and nothing about unchecked linkage changes
that. -/
theorem neither_extern_module_grants_the_socket :
    admitsSocket ⟨false, true, true, false, false, false, false, false, false, false⟩ true = false := rfl

theorem code_that_sockets_nothing_needs_no_socket_import (imports : Imports) :
    admitsSocket imports false = true := by simp [admitsSocket]

theorem duplicate_socket_import_is_idempotent (names : List String) :
    resolve ("std.socket" :: "std.socket" :: names) = resolve ("std.socket" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

/-- Naming the socket module beside the two output modules and the rendering
vocabulary grants exactly those four, and nothing else. -/
theorem the_socket_module_is_independent_of_its_neighbours :
    resolve ["std.socket", "std.fmt", "std.io", "std.logger"] =
      some ⟨false, false, false, true, true, true, false, false, true, false⟩ := by
  simp [resolve, setterFor, none]

/-- A near-miss spelling refuses. `std.net` is deliberately **not** a registry
entry: the module claims exactly what it grants, and addressing, resolution and
transport security are not it. -/
theorem a_near_miss_socket_spelling_rejects :
    resolve ["std.sockets"] = Option.none ∧ resolve ["std.net"] = Option.none := by
  constructor <;> simp [resolve, setterFor]

/-! ## Card `HTTP-QUIRIN-V1`: the tenth module

The HTTP capability is gated by **the same generic argument** every other module
is gated by — `admitted_capability_requires_its_module`, instantiated once more —
so it cannot drift from the nine that precede it. -/

/-- Card `HTTP-QUIRIN-V1`: declaring an HTTP/1.1 message, adding a header field
to one or setting its content is gated by `std.http`, never by another. -/
def admitsHttp (imports : Imports) (declaresMessage : Bool) : Bool :=
  !declaresMessage || imports.http

theorem empty_imports_grant_no_http :
    (resolve []).map Imports.http = some false := rfl

theorem admitted_http_requires_its_module {names : List String}
    {imports : Imports} (resolved : resolve names = some imports)
    (enabled : imports.http = true) : "std.http" ∈ names :=
  admitted_capability_requires_its_module Imports.http "std.http" rfl
    (setter_preserves_other_fields _ _
      (by intro i
          exact ⟨Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inl rfl,
            Or.inl rfl, Or.inl rfl, Or.inl rfl, Or.inr rfl⟩))
    resolved enabled

/-- **The composition rule as a registry fact, half one.** `std.http` grants no
transport: a message is a value, and putting one on a wire is `std.socket`'s. A
program that actually speaks HTTP names both, which is what the route's own
import gate says and this is the registry-level statement of it. -/
theorem the_http_import_alone_grants_no_socket :
    admitsSocket ⟨false, false, false, false, false, false, false, false, false, true⟩ true
      = false := rfl

/-- **Half two, and the one this card had to argue.** `std.http` grants no text:
a quoted token at a method, a target, a field name, a field value or a content
position is this module's own syntax and constructs no `string` value. The
converse holds too — the string module opens no HTTP vocabulary — so the two are
independent in both directions. -/
theorem the_http_module_is_independent_of_the_text_module :
    admitsString ⟨false, false, false, false, false, false, false, false, false, true⟩ true
        = false
      ∧ admitsHttp ⟨false, false, false, false, false, false, false, true, false, false⟩ true
        = false :=
  ⟨rfl, rfl⟩

/-- And it opens no stream: a transcript line is `std.io`'s or `std.logger`'s. -/
theorem the_http_import_alone_grants_no_stream :
    admitsPrint ⟨false, false, false, false, false, false, false, false, false, true⟩ true = false
      ∧ admitsLog ⟨false, false, false, false, false, false, false, false, false, true⟩ true
        = false :=
  ⟨rfl, rfl⟩

/-- Neither the socket module nor the output modules grant the HTTP vocabulary:
a program that opens a socket and prints has not thereby acquired a message. -/
theorem the_neighbouring_modules_grant_no_http :
    admitsHttp ⟨false, false, false, true, true, true, false, false, true, false⟩ true = false
      := rfl

theorem concurrency_grants_no_http_capability :
    admitsHttp ⟨true, false, false, false, false, false, false, false, false, false⟩ true
      = false := rfl

/-- Neither extern module grants it. The design record
`docs/design/STD_HTTP_BINDING_V1.md` recommended reaching HTTP *through* a
contracted native-library binding; the user directed the pure-source route for
this wave, and this theorem is the registry-level statement that the two are
separate capabilities rather than one reached through the other. -/
theorem neither_extern_module_grants_http :
    admitsHttp ⟨false, true, true, false, false, false, false, false, false, false⟩ true
      = false := rfl

theorem code_that_declares_no_message_needs_no_http_import (imports : Imports) :
    admitsHttp imports false = true := by simp [admitsHttp]

theorem duplicate_http_import_is_idempotent (names : List String) :
    resolve ("std.http" :: "std.http" :: names) = resolve ("std.http" :: names) := by
  simp only [resolve]
  cases resolve names <;> rfl

/-- Naming the HTTP module beside the transport and the two output modules
grants exactly those four, and nothing else — in particular not `std.string`,
which is the composition this card measured rather than assumed. -/
theorem the_http_module_is_independent_of_its_neighbours :
    resolve ["std.http", "std.socket", "std.fmt", "std.io"] =
      some ⟨false, false, false, true, true, false, false, false, true, true⟩ := by
  simp [resolve, setterFor, none]

/-- A near-miss spelling refuses. `std.web` and `std.https` are deliberately
**not** registry entries: the module claims exactly what it grants, and this one
grants a bounded HTTP/1.1 message subset. -/
theorem a_near_miss_http_spelling_rejects :
    resolve ["std.https"] = Option.none ∧ resolve ["std.web"] = Option.none := by
  constructor <;> simp [resolve, setterFor]

end QCore.ModuleImports
