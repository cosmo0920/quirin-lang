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
# Route selection reads no declaration name (card QR-LEXICAL-SELECTORS-DECLARATION-NAMES)

Generalises `QCore.PrintRouteSelection` (card QR-PRINT-ROUTE-MATCHES-DECLARATION),
which models the print-call selector alone, to every **token-stream** route
selector of `compile_elaborated_source`
(`crates/qcore-verification/src/driver.rs`) and to the `Q015` import gate:

* the gate's token loop (`module_imports::concurrency_span`): the words
  `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`,
  `qcore_error_payload`, `spawn`, `join`, and the pairs `task <`, `mutex <`,
  `mutex guards`, `mutex rank`;
* `source_tasks::has_source_tasks`: the three `qcore_buffer_*` words;
* `source_epoch::has_source_epoch`: `epoch {`;
* `source_mutex::has_source_mutex`: `mutex guards`, `mutex rank`;
* `source_scalar_calls::has_scalar_calls`: `qcore_acquire` / `qcore_check`
  (deselect) and `throws < ArithmeticError` (select).

The other selectors read function-body **statements** (`std_io_uses`,
`buffer_uses`, `socket_uses`, …) and so never see a declaration; they are not
modelled here.

The module is its retained token stream, each token tagged with its
**position** (`route_selection::SelectorScope`). Never read: a declaration's
own name (other than a record's), a parameter's name, a contract clause, a
library binding. Read: bodies, signatures (types, effect and `throws` rows),
other declaration text such as an alias's type, and a **record's name and
fields** — constraint 14's recorded `AGGREGATE-SELECTOR-WORDS` decision keeps
a record spelling a selector word refused at its declaration. Each selector
tests the token at a read position against the tokens that follow it, as the
Rust tests do (`scope.reads(head)` and the window).

Proved: a selector fires only at a read position, so a module whose selector
words all stand at name positions fires no selector, trips no gate and
reaches the sequential route; a use that names a declared word refuses (a
record's own name is not a use of it); and a `module contract;` module is
never compiled on a source-only route. Card ROUTE-SELECTION-ORDER-RESTATED
adds the driver's one route table (the last section): the guard covers
exactly the source-only entries of the table selection reads.

Premise (trust boundary 28): the Rust functions compute the positions from
the frontend's declaration, body, parameter and contract spans and apply
these tests; agreement is checked by the census
(`compiler/tests/qr_lexical_selector_census.rs`, every word × mode × shape
against its control) and the selection tests, not proved here.
-/

namespace QCore.RouteSelection

/-- Where a token stands (`SelectorScope`). -/
inductive Position where
  /-- Inside a source function's body. -/
  | body
  /-- A parameter or result type, an effect or throws row, an alias's type:
  declaration text that is not a name. -/
  | signature
  /-- A record's own name (read: `AGGREGATE-SELECTOR-WORDS`). -/
  | recordName
  /-- A record's field name (read: `AGGREGATE-SELECTOR-WORDS`). -/
  | recordField
  /-- The own name of any other declaration (`extern fn`, `fn`, `error`, …). -/
  | declarationName
  | parameterName
  | contractClause
  | libraryBinding
  deriving DecidableEq, Repr

/-- `SelectorScope::reads`. -/
def Position.read : Position → Bool
  | .body => true
  | .signature => true
  | .recordName => true
  | .recordField => true
  | _ => false

/-- A read position that is a **use** of a word, not a declaration's own name
(`SelectorScope::is_declaration_name`). -/
def Position.use : Position → Bool
  | .recordName => false
  | p => p.read

/-- The tokens the token-stream selectors distinguish; every other is `other`. -/
inductive Tok where
  | bufferNew | bufferRead | bufferWrite | errorPayload | spawn | join
  | acquire | check | task | mutex | guards | rank | epoch | throws
  | arithmeticError | less | lbrace | other
  deriving DecidableEq, Repr

structure Site where
  tok : Tok
  pos : Position
  deriving DecidableEq, Repr

/-- One selector's lexical test: the token and the tokens that follow it. -/
abbrev Test := Tok → List Tok → Bool

/-- Each site with the tokens that follow it. -/
def windows : List Site → List (Site × List Tok)
  | [] => []
  | s :: rest => (s, rest.map Site.tok) :: windows rest

/-- A token-stream selector fires when its test holds at a site it reads. -/
def fires (test : Test) (sites : List Site) : Bool :=
  (windows sites).any fun w => w.1.pos.read && test w.1.tok w.2

/-! ## The selectors -/

def gateWord : Tok → Bool
  | .bufferNew | .bufferRead | .bufferWrite | .errorPayload | .spawn | .join => true
  | _ => false

/-- The `Q015` gate's token loop. -/
def concurrencyGate : Test
  | .task, .less :: _ => true
  | .mutex, .less :: _ => true
  | .mutex, .guards :: _ => true
  | .mutex, .rank :: _ => true
  | t, _ => gateWord t

def tasks : Test
  | .bufferNew, _ | .bufferRead, _ | .bufferWrite, _ => true
  | _, _ => false

def epoch : Test
  | .epoch, .lbrace :: _ => true
  | _, _ => false

def mutex : Test
  | .mutex, .guards :: _ => true
  | .mutex, .rank :: _ => true
  | _, _ => false

def scalarDeselect : Test
  | .acquire, _ | .check, _ => true
  | _, _ => false

def scalarThrows : Test
  | .throws, .less :: .arithmeticError :: _ => true
  | _, _ => false

/-- The head words: every token at which some selector's test can hold. -/
def head : Tok → Bool
  | .bufferNew | .bufferRead | .bufferWrite | .errorPayload | .spawn | .join
  | .acquire | .check | .task | .mutex | .epoch | .throws => true
  | _ => false

def selectors : List Test :=
  [concurrencyGate, tasks, epoch, mutex, scalarDeselect, scalarThrows]

/-- Every selector's test holds only at a head word. -/
theorem test_holds_only_at_a_head :
    ∀ test ∈ selectors, ∀ t next, test t next = true → head t = true := by
  intro test htest t next h
  simp only [selectors, List.mem_cons, List.mem_nil_iff, or_false] at htest
  cases t <;> first
    | rfl
    | (rcases htest with rfl | rfl | rfl | rfl | rfl | rfl <;>
        rcases next with _ | ⟨n, _ | ⟨n2, rest⟩⟩ <;> (try cases n) <;> (try cases n2) <;>
        simp_all [concurrencyGate, tasks, epoch, mutex, scalarDeselect, scalarThrows,
          gateWord])

theorem mem_windows (sites : List Site) :
    ∀ w ∈ windows sites, w.1 ∈ sites := by
  induction sites with
  | nil => intro w hw; simp [windows] at hw
  | cons s rest ih =>
    intro w hw
    simp only [windows, List.mem_cons] at hw
    rcases hw with rfl | hw
    · simp
    · exact List.mem_cons_of_mem _ (ih w hw)

/-- **A selector fires only at a site it reads.** -/
theorem fires_only_at_a_read_site (test : Test) (sites : List Site)
    (h : fires test sites = true) :
    ∃ s ∈ sites, s.pos.read = true ∧ ∃ next, test s.tok next = true := by
  unfold fires at h
  rcases List.any_eq_true.mp h with ⟨w, hw, hread⟩
  simp only [Bool.and_eq_true] at hread
  exact ⟨w.1, mem_windows sites w hw, hread.1, w.2, hread.2⟩

/-- **A declaration name never selects.** If every head word of the module
stands where no selector reads (a declaration's name, a parameter's name, a
contract clause, a library binding, another declaration), no selector fires. -/
theorem declaration_name_never_selects (sites : List Site)
    (h : ∀ s ∈ sites, head s.tok = true → s.pos.read = false) :
    ∀ test ∈ selectors, fires test sites = false := by
  intro test htest
  cases hf : fires test sites with
  | false => rfl
  | true =>
    rcases fires_only_at_a_read_site test sites hf with ⟨s, hs, hread, next, htrue⟩
    have := h s hs (test_holds_only_at_a_head test htest s.tok next htrue)
    rw [hread] at this
    cases this

/-! ## Route selection and its two fail-closed arms -/

/-- The routes the token-stream selectors decide among. -/
inductive Route where
  | epochBound
  | mutexGuard
  | scopedTasks
  | scalarCalls
  deriving DecidableEq, Repr

inductive Outcome where
  /-- `Q015`: the file spells concurrency without `#import std.concurrency;`. -/
  | importRequired
  /-- `Q020` at selection: a use names a declared word, or a contract module
  would reach a source-only route. -/
  | refused
  | compiled (route : Route)
  | sequential
  deriving DecidableEq, Repr

structure Module where
  sites : List Site
  /-- The names the module declares, as selector tokens. -/
  declared : List Tok
  contract : Bool
  concurrencyImport : Bool
  deriving Repr

/-- The word a use of a head names (`route_selection::selector_word_at`). -/
def named : Tok → Tok
  | .throws => .arithmeticError
  | t => t

/-- A read head word that some selector's test holds at, naming a declared word. -/
def ambiguous (m : Module) : Bool :=
  (windows m.sites).any fun w =>
    w.1.pos.use && selectors.any (fun test => test w.1.tok w.2) &&
      m.declared.contains (named w.1.tok)

/-- The selection order of `compile_elaborated_source`, restricted to the
token-stream selectors (each selected route's other inputs are abstracted).
`has_scalar_calls` holds by its throws row and fails on a deselecting word. -/
def selected (m : Module) : Option Route :=
  if fires epoch m.sites then some .epochBound
  else if fires mutex m.sites then some .mutexGuard
  else if fires tasks m.sites then some .scopedTasks
  else if fires scalarThrows m.sites && !fires scalarDeselect m.sites then some .scalarCalls
  else none

def route (m : Module) : Outcome :=
  if fires concurrencyGate m.sites && !m.concurrencyImport then .importRequired
  else if ambiguous m then .refused
  else match selected m with
    | some r => if m.contract then .refused else .compiled r
    | none => .sequential

theorem Position.read_of_use (p : Position) (h : p.use = true) : p.read = true := by
  cases p <;> simp_all [Position.use, Position.read]

theorem ambiguous_false_of_no_read_head (m : Module)
    (h : ∀ s ∈ m.sites, head s.tok = true → s.pos.read = false) :
    ambiguous m = false := by
  unfold ambiguous
  cases hc : (windows m.sites).any _ with
  | false => rfl
  | true =>
    rcases List.any_eq_true.mp hc with ⟨w, hw, hall⟩
    simp only [Bool.and_eq_true] at hall
    rcases List.any_eq_true.mp hall.1.2 with ⟨test, htest, htrue⟩
    have := h w.1 (mem_windows _ w hw) (test_holds_only_at_a_head test htest _ _ htrue)
    rw [Position.read_of_use _ hall.1.1] at this
    cases this

/-- **Declaration names leave a module on the sequential route**: no gate,
no refusal, no source-only route, in every mode. -/
theorem declaration_names_reach_sequential (m : Module)
    (h : ∀ s ∈ m.sites, head s.tok = true → s.pos.read = false) :
    route m = .sequential := by
  have hsel := declaration_name_never_selects m.sites h
  have hg : fires concurrencyGate m.sites = false := hsel _ (by simp [selectors])
  have he : fires epoch m.sites = false := hsel _ (by simp [selectors])
  have hm : fires mutex m.sites = false := hsel _ (by simp [selectors])
  have ht : fires tasks m.sites = false := hsel _ (by simp [selectors])
  have hs : fires scalarThrows m.sites = false := hsel _ (by simp [selectors])
  unfold route selected
  simp [hg, he, hm, ht, hs, ambiguous_false_of_no_read_head m h]

/-- **A use that names a declared word refuses** rather than being routed
(once the import gate is satisfied). -/
theorem ambiguous_use_refuses (m : Module)
    (hgate : (fires concurrencyGate m.sites && !m.concurrencyImport) = false)
    (h : ambiguous m = true) : route m = .refused := by
  unfold route
  rw [hgate, h]
  rfl

/-- **A contract module is never compiled on a source-only route.** -/
theorem contract_module_never_compiled (m : Module) (h : m.contract = true) :
    ∀ r, route m ≠ .compiled r := by
  intro r
  unfold route
  rw [h]
  split
  · simp
  · split
    · simp
    · cases selected m <;> simp

/-! ## Vectors -/

private def at_ (t : Tok) (p : Position) : Site := ⟨t, p⟩

/-- `module contract; extern fn qcore_buffer_new(…) -> … contract { … };`
(the print card's probe, refused `Q015` at the name before this card). -/
def bufferNewContract : Module :=
  ⟨[at_ .other .declarationName, at_ .other .declarationName,
    at_ .bufferNew .declarationName, at_ .other .signature,
    at_ .other .contractClause], [.bufferNew], true, false⟩

/-- `error epoch { Failed; }` beside `fn main() -> i32 { return 0; }`. -/
def epochError : Module :=
  ⟨[at_ .other .signature, at_ .epoch .declarationName, at_ .lbrace .signature,
    at_ .other .signature, at_ .lbrace .body, at_ .other .body], [.epoch], false, false⟩

/-- `fn spawn() -> i32 { return 0; }`: a source function named after a word. -/
def spawnFunction : Module :=
  ⟨[at_ .other .signature, at_ .spawn .declarationName, at_ .other .signature,
    at_ .lbrace .body, at_ .other .body], [.spawn], false, false⟩

/-- `type spawn { x: i32 }` without the import: a record's name stays read
(`AGGREGATE-SELECTOR-WORDS`), so the gate still refuses it. -/
def spawnRecord : Module :=
  ⟨[at_ .other .signature, at_ .spawn .recordName, at_ .lbrace .signature,
    at_ .other .recordField], [.spawn], false, false⟩

/-- The same record with the import: its own name is not a use, so no
ambiguity, and no route is selected by it. -/
def spawnRecordImported : Module := { spawnRecord with concurrencyImport := true }

/-- `type epoch { lo: i32 }`: still the epoch route (which refuses it). -/
def epochRecord : Module :=
  ⟨[at_ .other .signature, at_ .epoch .recordName, at_ .lbrace .signature,
    at_ .other .recordField], [.epoch], false, false⟩

/-- `fn main() -> i32 effects { spawn, join } { … }` without the import: the
effect row is read, so the gate still requires the import. -/
def effectRowWithoutImport : Module :=
  ⟨[at_ .other .signature, at_ .spawn .signature, at_ .join .signature,
    at_ .lbrace .body, at_ .other .body], [.other], false, false⟩

/-- `let b = qcore_buffer_new(42);` in a body, with the import: the task route. -/
def bodyBufferNew : Module :=
  ⟨[at_ .lbrace .body, at_ .bufferNew .body, at_ .other .body], [.other], false, true⟩

/-- The same body in a module that also declares `qcore_buffer_new`. -/
def bodyBufferNewDeclared : Module := { bodyBufferNew with declared := [.bufferNew] }

/-- The same body in a contract module. -/
def bodyBufferNewContract : Module := { bodyBufferNew with contract := true }

/-- `fn f() -> i32 throws<ArithmeticError> { … }`: the scalar route's row. -/
def throwsRow : Module :=
  ⟨[at_ .throws .signature, at_ .less .signature, at_ .arithmeticError .signature,
    at_ .lbrace .body], [.other], false, false⟩

theorem buffer_new_contract_sequential : route bufferNewContract = .sequential := by decide
theorem epoch_error_sequential : route epochError = .sequential := by decide
theorem spawn_function_sequential : route spawnFunction = .sequential := by decide
theorem spawn_record_requires_import : route spawnRecord = .importRequired := by decide
theorem spawn_record_imported_sequential : route spawnRecordImported = .sequential := by decide
theorem epoch_record_epoch_route : route epochRecord = .compiled .epochBound := by decide
theorem effect_row_requires_import :
    route effectRowWithoutImport = .importRequired := by decide
theorem body_buffer_new_tasks : route bodyBufferNew = .compiled .scopedTasks := by decide
theorem body_buffer_new_declared_refused : route bodyBufferNewDeclared = .refused := by decide
theorem body_buffer_new_contract_refused : route bodyBufferNewContract = .refused := by decide
theorem throws_row_scalar : route throwsRow = .compiled .scalarCalls := by decide

/-! ## The selector table (card ROUTE-SELECTION-ORDER-RESTATED)

`driver::source_routes::SOURCE_ROUTES` (`crates/qcore-verification/src/driver/source_routes.rs`)
is the driver's route order, written once. `compile_elaborated_source` selects
one entry (`source_routes::select`: the first whose selector holds) and reads
it for the closure refusal, the contract guard and the lowering. Each entry is
modelled by its name, its selector over an abstract module, whether it is
source-only and whether it hosts closures; the ambiguous-use refusal and an
entry's selection refusal precede the lowering and are abstracted.

Proved, for every table: first match is the if-chain in table order
(`choose_eq_chain`); the guard refuses a contract module exactly when the
selected entry is source-only (`guard_refuses_iff_source_only`), so an entry
added anywhere is guarded (`added_entry_is_guarded`); what is lowered is the
selected entry (`lowered_is_chosen`); a contract module is never lowered on a
source-only entry (`contract_never_lowered_on_source_only`) and, when every
entry but the last is source-only, only on the last
(`contract_lowered_only_on_last`) — the argument that deletes the completeness
check after routing and the guard in `compile_scalar_calls`; and a closure is
lowered only on an entry that hosts closures. For the driver's table
(`driverTable`, pinned identically by `source_routes::tests`): a contract
module is lowered only on the sequential route, a closure only on the two
scoped-task routes, and a module that dispatches and sockets is refused naming
the socket route, where the base's restated list named the event route.

Premise (trust boundary 28): the Rust table's rows are `driverTable`'s, in
order (`the_table_is_the_pinned_order`, `closure_rules_follow_the_table`), and
the selectors are pure; agreement is checked by the unit tests and the route
censuses, not proved here.
-/

/-- One row of the driver's route table, over an abstract module type. -/
structure Entry (α : Type) where
  name : String
  selects : α → Bool
  sourceOnly : Bool
  hostsClosures : Bool

/-- `source_routes::select`: the first entry whose selector holds. -/
def choose {α : Type} (table : List (Entry α)) (m : α) : Option (Entry α) :=
  table.find? fun e => e.selects m

/-- The if-chain the table replaced: one `if` per entry, in table order. -/
def chain {α : Type} : List (Entry α) → α → Option (Entry α)
  | [], _ => none
  | e :: rest, m => if e.selects m then some e else chain rest m

/-- **Same order, same first match**: reading the table first-match is the
if-chain written in the table's order. -/
theorem choose_eq_chain {α : Type} (table : List (Entry α)) (m : α) :
    choose table m = chain table m := by
  induction table with
  | nil => rfl
  | cons e rest ih =>
    unfold choose at ih ⊢
    cases h : e.selects m <;> simp [chain, h, ih]

/-- What `compile_elaborated_source` does with the selected entry. -/
inductive Dispatch (α : Type) where
  /-- `Q600`: a closure on an entry that does not host closures. -/
  | closureRefused (name : String)
  /-- `Q020`: a contract module on a source-only entry, naming it. -/
  | guardRefused (name : String)
  /-- The entry's lowering runs. -/
  | lowered (e : Entry α)
  /-- No entry selected (excluded when the last entry always selects). -/
  | unselected

/-- The driver's reading of the one selected entry, in its order: the closure
refusal, then the contract guard, then the lowering. -/
def dispatch {α : Type} (table : List (Entry α)) (contract closure : Bool) (m : α) :
    Dispatch α :=
  match choose table m with
  | none => .unselected
  | some e =>
    if closure && !e.hostsClosures then .closureRefused e.name
    else if contract && e.sourceOnly then .guardRefused e.name
    else .lowered e

theorem choose_mem {α : Type} {table : List (Entry α)} {m : α} {e : Entry α}
    (h : choose table m = some e) : e ∈ table :=
  List.mem_of_find?_eq_some h

/-- **The guard covers exactly the source-only entries of the table that
selection reads**: a contract module (spelling no closure) is refused by the
guard exactly when its selected entry is source-only, naming that entry. -/
theorem guard_refuses_iff_source_only {α : Type} (table : List (Entry α)) (m : α)
    (name : String) :
    dispatch table true false m = .guardRefused name ↔
      ∃ e, choose table m = some e ∧ e.sourceOnly = true ∧ e.name = name := by
  unfold dispatch
  cases hc : choose table m with
  | none => simp
  | some e =>
    simp only [Option.some.injEq, exists_eq_left']
    cases e.sourceOnly <;> simp

/-- **An entry added anywhere is guarded**: no second list to update. -/
theorem added_entry_is_guarded {α : Type} (pre post : List (Entry α)) (e : Entry α)
    (m : α) (hchosen : choose (pre ++ e :: post) m = some e) (hsource : e.sourceOnly = true) :
    dispatch (pre ++ e :: post) true false m = .guardRefused e.name :=
  (guard_refuses_iff_source_only _ m e.name).mpr ⟨e, hchosen, hsource, rfl⟩

/-- What is lowered is the selected entry: the guard and the lowering read one
row. -/
theorem lowered_is_chosen {α : Type} (table : List (Entry α)) (contract closure : Bool)
    (m : α) (e : Entry α) (h : dispatch table contract closure m = .lowered e) :
    choose table m = some e := by
  unfold dispatch at h
  cases hc : choose table m with
  | none => rw [hc] at h; cases h
  | some e' =>
    rw [hc] at h
    simp only at h
    split at h
    · cases h
    · split at h
      · cases h
      · cases h; rfl

/-- **A contract module is never lowered on a source-only entry.** -/
theorem contract_never_lowered_on_source_only {α : Type} (table : List (Entry α))
    (closure : Bool) (m : α) (e : Entry α)
    (h : dispatch table true closure m = .lowered e) : e.sourceOnly = false := by
  unfold dispatch at h
  cases hc : choose table m with
  | none => rw [hc] at h; cases h
  | some e' =>
    rw [hc] at h
    simp only at h
    split at h
    · cases h
    · split at h
      · cases h
      · rename_i hguard
        cases h
        cases hs : e.sourceOnly
        · rfl
        · simp [hs] at hguard

/-- **A closure is lowered only on an entry that hosts closures.** -/
theorem closure_lowered_only_on_hosting {α : Type} (table : List (Entry α))
    (contract : Bool) (m : α) (e : Entry α)
    (h : dispatch table contract true m = .lowered e) : e.hostsClosures = true := by
  unfold dispatch at h
  cases hc : choose table m with
  | none => rw [hc] at h; cases h
  | some e' =>
    rw [hc] at h
    simp only at h
    split at h
    · cases h
    · rename_i hclosure
      split at h
      · cases h
      · cases h
        cases hh : e.hostsClosures
        · simp [hh] at hclosure
        · rfl

/-- **Contract modules reach only the last entry** when every other entry is
source-only: the completeness check after routing, and the guard in
`compile_scalar_calls`, have nothing left to catch. -/
theorem contract_lowered_only_on_last {α : Type} (pre : List (Entry α)) (last : Entry α)
    (hpre : ∀ e ∈ pre, e.sourceOnly = true) (closure : Bool) (m : α) (e : Entry α)
    (h : dispatch (pre ++ [last]) true closure m = .lowered e) : e = last := by
  have hfalse := contract_never_lowered_on_source_only _ closure m e h
  have hmem := choose_mem (lowered_is_chosen _ true closure m e h)
  rcases List.mem_append.mp hmem with hin | hin
  · rw [hpre e hin] at hfalse
    cases hfalse
  · simpa using hin

/-- With a last entry that always selects, selection is total. -/
theorem choose_total_of_last {α : Type} (pre : List (Entry α)) (last : Entry α)
    (hlast : ∀ m, last.selects m = true) (m : α) :
    choose (pre ++ [last]) m ≠ none := by
  unfold choose
  intro hnone
  have := List.find?_eq_none.mp hnone last (by simp)
  rw [hlast m] at this
  exact this rfl

/-! ### The driver's table -/

/-- `SOURCE_ROUTES`, row by row: (name, source-only, hosts closures). The
same rows are pinned by `source_routes::tests`. -/
def driverTable : List (String × Bool × Bool) :=
  [("source.ffi.v1", true, false),
   ("source.bitfield-concurrency.v1", true, false),
   ("source.epoch.tasks.v1", true, false),
   ("source.epoch.bound.v1", true, false),
   ("source.mutex.guard.v1", true, false),
   ("source.std-io-tasks.v1", true, true),
   ("the scoped-task route", true, true),
   ("source.std-io-extern.v1", true, false),
   ("source.extern-call.v1", true, false),
   ("source.socket.v1", true, false),
   ("source.event.v1", true, false),
   ("source.string.v1", true, false),
   ("source.resource-calls.v1", true, false),
   ("source.scalar-calls.v1", true, false),
   ("source.buffer.v1", true, false),
   ("source.http.v1", true, false),
   ("source.string.v1", true, false),
   ("source.socket.v1", true, false),
   ("source.scalar-calls.v1", true, false),
   ("source.std-io.v1", true, false),
   ("source.scalar-calls.v1", true, false),
   ("the sequential cleanup route", false, false)]

/-- The rows as entries; the `i`th row's selector is `sel i`. -/
def entriesFrom {α : Type} (sel : Nat → α → Bool) :
    Nat → List (String × Bool × Bool) → List (Entry α)
  | _, [] => []
  | i, (name, sourceOnly, hosts) :: rest =>
    ⟨name, sel i, sourceOnly, hosts⟩ :: entriesFrom sel (i + 1) rest

def driverEntries {α : Type} (sel : Nat → α → Bool) : List (Entry α) :=
  entriesFrom sel 0 driverTable

theorem mem_entriesFrom {α : Type} (sel : Nat → α → Bool) :
    ∀ (i : Nat) (rows : List (String × Bool × Bool)) (e : Entry α),
      e ∈ entriesFrom sel i rows →
        ∃ row ∈ rows, e.name = row.1 ∧ e.sourceOnly = row.2.1 ∧ e.hostsClosures = row.2.2
  | _, [], e, h => by simp [entriesFrom] at h
  | i, (name, sourceOnly, hosts) :: rest, e, h => by
    simp only [entriesFrom, List.mem_cons] at h
    rcases h with rfl | h
    · exact ⟨(name, sourceOnly, hosts), by simp, rfl, rfl, rfl⟩
    · rcases mem_entriesFrom sel (i + 1) rest e h with ⟨row, hrow, h1, h2, h3⟩
      exact ⟨row, List.mem_cons_of_mem _ hrow, h1, h2, h3⟩

theorem driver_table_length : driverTable.length = 22 := by decide

/-- Every row but the sequential route's is source-only. -/
theorem driver_table_reads_contracts_only_on_sequential :
    ∀ row ∈ driverTable, row.2.1 = false → row.1 = "the sequential cleanup route" := by
  decide

/-- Only the two scoped-task routes host closures. -/
theorem driver_table_hosts_closures_only_on_task_routes :
    ∀ row ∈ driverTable, row.2.2 = true →
      row.1 = "source.std-io-tasks.v1" ∨ row.1 = "the scoped-task route" := by
  decide

/-- **The driver lowers a contract module only on the sequential route**,
whatever its selectors read. -/
theorem driver_contract_lowered_only_on_sequential {α : Type} (sel : Nat → α → Bool)
    (closure : Bool) (m : α) (e : Entry α)
    (h : dispatch (driverEntries sel) true closure m = .lowered e) :
    e.name = "the sequential cleanup route" := by
  have hfalse := contract_never_lowered_on_source_only _ closure m e h
  have hmem := choose_mem (lowered_is_chosen _ true closure m e h)
  rcases mem_entriesFrom sel 0 driverTable e hmem with ⟨row, hrow, hname, hsource, _⟩
  rw [hname]
  exact driver_table_reads_contracts_only_on_sequential row hrow (hsource ▸ hfalse)

/-- **The driver lowers a closure only on the two scoped-task routes.** -/
theorem driver_closure_lowered_only_on_task_routes {α : Type} (sel : Nat → α → Bool)
    (contract : Bool) (m : α) (e : Entry α)
    (h : dispatch (driverEntries sel) contract true m = .lowered e) :
    e.name = "source.std-io-tasks.v1" ∨ e.name = "the scoped-task route" := by
  have hhosts := closure_lowered_only_on_hosting _ contract m e h
  have hmem := choose_mem (lowered_is_chosen _ contract true m e h)
  rcases mem_entriesFrom sel 0 driverTable e hmem with ⟨row, hrow, hname, _, hclosure⟩
  rw [hname]
  exact driver_table_hosts_closures_only_on_task_routes row hrow (hclosure ▸ hhosts)

/-! ### Vector: a module that dispatches and sockets -/

/-- Its selectors: the socket-and-event composition (row 9), the event route
(row 10) and the sequential route (row 21) hold. -/
def dispatchAndSocket (i : Nat) (_ : Unit) : Bool := i == 9 || i == 10 || i == 21

/-- The table names the route that would lower it. -/
theorem dispatch_and_socket_contract_names_socket :
    dispatch (driverEntries dispatchAndSocket) true false () =
      .guardRefused "source.socket.v1" := rfl

/-- The base's restated list (`route_selection::source_only_route`): the
driver's rows without the composition (row 9) and without the sequential
route, so its first match for the same module was the event route. -/
def restatedList : List (Entry Unit) :=
  (driverEntries dispatchAndSocket).eraseIdx 9 |>.dropLast

theorem restated_list_named_event :
    (choose restatedList ()).map Entry.name = some "source.event.v1" := rfl

/-- Executable, the same module is lowered on the composition's row. -/
theorem dispatch_and_socket_executable_lowered :
    (match dispatch (driverEntries dispatchAndSocket) false false () with
      | .lowered e => some e.name
      | _ => none) = some "source.socket.v1" := rfl

end QCore.RouteSelection
