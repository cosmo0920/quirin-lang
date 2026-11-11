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
# Print-call route selection (card QR-PRINT-ROUTE-MATCHES-DECLARATION)

`source_scalar_calls::print_call_selection`
(`crates/qcore-frontend/src/source_scalar_calls.rs`) decides whether a module
reaches `source.scalar-calls.v1` because it calls the G02s builtin
`print(EXPRESSION);`. Before this card the test was lexical: any `print (`
token pair anywhere in the module selected the route, so a contract module
declaring `extern fn print(...)` (cJSON's static `print`) was lowered on a
route that refuses every `extern fn`.

The model reads the module as the selector does: its retained token stream in
order, each token tagged with whether it lies inside a source function body,
plus whether any declaration is named `print`. Each `print (` pair is

* **ignored** outside every function body (a declaration name, an `extern fn`,
  a contract clause, a record field);
* a **call** inside a body at a statement head (the previous token is `{`,
  `;` or `}`);
* **ambiguous** inside a body anywhere else.

A module with an ambiguous occurrence, or with a call while it also declares
something named `print`, is refused; a module with a call and neither is
selected; every other module is left to the later routes.

The secondary guard is modelled too: a `module contract;` module is never
compiled on the scalar-calls route, whatever selected it.

Premise (trust boundary 28): the Rust function computes this classification
over the frontend's lexical tokens and body spans; the agreement is checked by
the frontend's unit tests and the route census, not proved here.
-/

namespace QCore.PrintRouteSelection

/-- The tokens the selector distinguishes; every other token is `other`. -/
inductive Tok where
  | print
  | lparen
  | lbrace
  | rbrace
  | semi
  | other
  deriving DecidableEq, Repr

/-- One retained token and whether it lies inside a source function body. -/
structure Site where
  tok : Tok
  inBody : Bool
  deriving DecidableEq, Repr

/-- A module as the selector reads it. -/
structure Module where
  sites : List Site
  declaresPrint : Bool
  contract : Bool
  deriving Repr

inductive Occurrence where
  | ignored
  | call
  | ambiguous
  deriving DecidableEq, Repr

inductive Selection where
  | absent
  | selected
  | ambiguous
  deriving DecidableEq, Repr

/-- A statement head: the previous token closes or separates a statement. -/
def statementHead : Option Tok → Bool
  | some .lbrace => true
  | some .rbrace => true
  | some .semi => true
  | _ => false

/-- The classification of one `print` site, given the previous token. -/
def classify (previous : Option Tok) (site : Site) : Occurrence :=
  if !site.inBody then .ignored
  else if statementHead previous then .call
  else .ambiguous

/-- Every `print (` pair of the stream, classified, in order. -/
def occurrences : Option Tok → List Site → List Occurrence
  | _, [] => []
  | previous, site :: rest =>
    let tail := occurrences (some site.tok) rest
    match rest with
    | next :: _ =>
      if site.tok = .print ∧ next.tok = .lparen then classify previous site :: tail
      else tail
    | [] => tail

def select (m : Module) : Selection :=
  match (occurrences none m.sites).contains .ambiguous,
      (occurrences none m.sites).contains .call, m.declaresPrint with
  | true, _, _ => .ambiguous
  | false, true, true => .ambiguous
  | false, true, false => .selected
  | false, false, _ => .absent

/-- The outcome at the scalar-calls route's entry (the secondary guard). -/
inductive Outcome where
  | compiled
  | refused
  | laterRoutes
  deriving DecidableEq, Repr

def route (m : Module) : Outcome :=
  match select m with
  | .absent => .laterRoutes
  | .ambiguous => .refused
  | .selected => if m.contract then .refused else .compiled

/-- An occurrence outside every body is ignored, whatever precedes it. -/
theorem outside_body_ignored (previous : Option Tok) (site : Site)
    (h : site.inBody = false) : classify previous site = .ignored := by
  simp [classify, h]

/-- A stream whose `print` tokens all lie outside function bodies classifies
every pair as ignored. -/
theorem occurrences_outside_bodies (previous : Option Tok) :
    ∀ sites : List Site,
      (∀ s ∈ sites, s.tok = .print → s.inBody = false) →
      ∀ o ∈ occurrences previous sites, o = .ignored := by
  intro sites
  induction sites generalizing previous with
  | nil => intro _ o ho; simp [occurrences] at ho
  | cons site rest ih =>
    intro hall o ho
    have hrest : ∀ s ∈ rest, s.tok = .print → s.inBody = false :=
      fun s hs => hall s (List.mem_cons_of_mem _ hs)
    cases rest with
    | nil => simp [occurrences] at ho
    | cons next more =>
      simp only [occurrences] at ho
      split at ho
      · rename_i hpair
        rcases List.mem_cons.mp ho with h | h
        · rw [h]
          exact outside_body_ignored previous site (hall site (by simp) hpair.1)
        · exact ih (some site.tok) hrest o h
      · exact ih (some site.tok) hrest o ho

/-- **A declaration name never selects the route.** A module whose only
`print` tokens are outside function bodies (an `extern fn print(...)`, a
contract clause, a record field) is left to the later routes. -/
theorem declaration_name_never_selects (m : Module)
    (h : ∀ s ∈ m.sites, s.tok = .print → s.inBody = false) :
    select m = .absent := by
  have hall := occurrences_outside_bodies none m.sites h
  have hamb : (occurrences none m.sites).contains .ambiguous = false := by
    cases hc : (occurrences none m.sites).contains .ambiguous with
    | false => rfl
    | true =>
      have := hall .ambiguous (by simpa using hc)
      cases this
  have hcall : (occurrences none m.sites).contains .call = false := by
    cases hc : (occurrences none m.sites).contains .call with
    | false => rfl
    | true =>
      have := hall .call (by simpa using hc)
      cases this
  unfold select
  rw [hamb, hcall]

/-- Selection is only ever by a call: a selected module holds a classified
call occurrence. -/
theorem selected_has_a_call (m : Module) (h : select m = .selected) :
    Occurrence.call ∈ occurrences none m.sites := by
  unfold select at h
  cases ha : (occurrences none m.sites).contains .ambiguous <;>
    cases hc : (occurrences none m.sites).contains .call <;>
    cases hd : m.declaresPrint <;> rw [ha, hc, hd] at h <;>
    first | (simpa using hc) | (cases h)

/-- A module that declares something named `print` is never selected: the
builtin call and the declaration are ambiguous. -/
theorem declared_print_never_selected (m : Module) (h : m.declaresPrint = true) :
    select m ≠ .selected := by
  unfold select
  cases ha : (occurrences none m.sites).contains .ambiguous <;>
    cases hc : (occurrences none m.sites).contains .call <;> rw [h] <;> simp

/-- An ambiguous occurrence refuses rather than being routed anywhere. -/
theorem ambiguous_occurrence_refuses (m : Module)
    (h : Occurrence.ambiguous ∈ occurrences none m.sites) : route m = .refused := by
  have hc : (occurrences none m.sites).contains .ambiguous = true := by simpa using h
  unfold route select
  rw [hc]

/-- **The secondary guard.** A contract module is never compiled on the
scalar-calls route. -/
theorem contract_module_never_compiled (m : Module) (h : m.contract = true) :
    route m ≠ .compiled := by
  unfold route
  cases select m <;> simp [h]

/-! ## Vectors: the three shapes the card names -/

private def b (t : Tok) : Site := ⟨t, true⟩
private def d (t : Tok) : Site := ⟨t, false⟩

/-- `extern fn print(item: …) -> … contract { … };` in a contract module
(cJSON's static `print`, round 3's withdrawn declaration). -/
def cjsonDeclaration : Module :=
  ⟨[d .other, d .other, d .print, d .lparen, d .other, d .other, d .semi], true, true⟩

/-- `fn main() -> i32 { let f = 7; print(f); return 0; }`
(`examples/scalar/printing.qr`'s shape). -/
def printingExample : Module :=
  ⟨[d .other, d .other, d .lparen, d .other, d .other,
    b .lbrace, b .other, b .other, b .other, b .other, b .semi,
    b .print, b .lparen, b .other, b .other, b .semi,
    b .other, b .other, b .semi, b .rbrace], false, false⟩

/-- `fn main() -> i32 { let x = print(1); return x; }`. -/
def printAsValue : Module :=
  ⟨[d .other, d .other, d .lparen, d .other, d .other,
    b .lbrace, b .other, b .other, b .other, b .print, b .lparen, b .other, b .other,
    b .semi, b .other, b .other, b .semi, b .rbrace], false, false⟩

theorem cjson_declaration_reaches_later_routes : route cjsonDeclaration = .laterRoutes := by
  decide

theorem printing_example_selected : route printingExample = .compiled := by
  decide

theorem print_as_value_refused : route printAsValue = .refused := by
  decide

/-- The same printing body in a module that also declares `print` refuses. -/
theorem printing_beside_a_declared_print_refused :
    route { printingExample with declaresPrint := true } = .refused := by
  decide

/-- The same printing body in a contract module refuses (secondary guard). -/
theorem printing_in_a_contract_module_refused :
    route { printingExample with contract := true } = .refused := by
  decide

end QCore.PrintRouteSelection
