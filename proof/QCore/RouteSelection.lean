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
never compiled on a source-only route.

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

end QCore.RouteSelection
