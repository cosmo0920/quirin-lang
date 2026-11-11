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
# Unevaluated `sizeof` dereference operands (card W44-COLLECTION-ALLOCATION)

The abstract rule implemented by `crates/qcore-c-gateway/src/sizeof_operand.rs`
and its two hooks in `legacy_c.rs` (the lexical access gate's view and the
protected-name mention of a borrowed argument).

C11 6.5.3.4p2: *"If the type of the operand is a variable length array type,
the operand is evaluated; otherwise, the operand is not evaluated and the result
is an integer constant."* The corpus allocates arrays as
`v = malloc(n * sizeof(*v))`, and the lexical access gate read the operand's
`*` as a dereference. This module models:

* **C types** with a variably-modified flag on the base (`CType`), the type of
  an operand `*…*NAME` / `NAME[LIT]` under a declaration, and the
  **evaluation fact**: an operand is evaluated exactly when its type is a
  variable length array type;
* the **reader's answer** for a declaration (a pointer declarator with no array
  dimension over a specifier the reader resolves as never variably modified)
  and the **admission** it licenses;
* that an admitted operand is **never evaluated**, so it reads, dereferences
  and writes nothing (`an_admitted_operand_has_no_effect`), whatever the name
  holds — released, null or unbound;
* that a **variably modified** operand is evaluated and is **refused**, as are
  an array declarator, an unreadable declaration, an unresolved specifier and a
  deeper dereference than declared;
* the **gate view**: an admitted operand is replaced by a marker that is not an
  access and not a literal; with no admitted operand the view is the old view,
  and the rule can only turn a gate refusal into a pass, never the reverse, so
  every statement the gate already passed keeps its lowering;
* the **mention filter**: the names a borrowed argument hands to its callee
  lose only the names inside admitted operands.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
C token/type extraction — that the declarator reader's facts and its typedef
resolution describe the translation unit C means, so that a specifier it
resolves as never variably modified is one (`Faithful`) — and the Rust–Lean
correspondence are premises (trust boundary 28), stated here as hypotheses, not
proved. Only the size's *custody* is modelled: the marker is never a computed
size (constraint 5a).
-/

namespace QCore.SizeofOperand

/-! ## C types and the evaluation fact -/

/-- A C object type, as far as `sizeof` evaluation needs it: a base type that is
or is not variably modified, pointers, and arrays whose bound is constant or
variable. -/
inductive CType where
  | base (vm : Bool)
  | ptr (target : CType)
  | arr (element : CType) (varBound : Bool)
  deriving DecidableEq, Repr

/-- Variably modified (C11 6.7.6p3): a variable length array anywhere in the
declarator chain, or a variably modified base. -/
def CType.vm : CType → Bool
  | .base b => b
  | .ptr t => t.vm
  | .arr t v => v || t.vm

/-- A variable length array type: an array whose bound is not constant. -/
def CType.isVLA : CType → Bool
  | .arr _ v => v
  | _ => false

/-- A VLA type is variably modified. -/
theorem vla_is_vm (t : CType) (h : t.isVLA = true) : t.vm = true := by
  cases t with
  | base b => simp [CType.isVLA] at h
  | ptr t => simp [CType.isVLA] at h
  | arr e v => simp [CType.isVLA] at h; simp [CType.vm, h]

/-- The C evaluation fact (C11 6.5.3.4p2), stated of the operand's type. -/
def evaluates (t : CType) : Bool := t.isVLA

/-- One dereference (or one literal subscript, which designates the same type):
a pointer's target, or an array's element after decay. -/
def CType.deref : CType → Option CType
  | .ptr t => some t
  | .arr t _ => some t
  | .base _ => none

/-- `k` dereferences. -/
def derefN : Nat → CType → Option CType
  | 0, t => some t
  | k + 1, t => t.deref.bind (derefN k)

/-- `s` pointer levels over a base. -/
def pointers : Nat → CType → CType
  | 0, t => t
  | s + 1, t => .ptr (pointers s t)

theorem pointers_vm (s : Nat) (b : CType) : (pointers s b).vm = b.vm := by
  induction s with
  | zero => rfl
  | succ s ih => simp [pointers, CType.vm, ih]

theorem derefN_pointers (k s : Nat) (b : CType) (h : k ≤ s) :
    derefN k (pointers s b) = some (pointers (s - k) b) := by
  induction k generalizing s with
  | zero => simp [derefN]
  | succ k ih =>
    cases s with
    | zero => omega
    | succ s =>
      simp [derefN, pointers, CType.deref, Option.bind]
      simpa using ih s (by omega)

/-! ## Operands, declarations and the reader -/

/-- The operand shapes the rule recognises: `k` stars before a name, or a name
with one literal subscript. -/
inductive Operand where
  | deref (k : Nat)
  | subscript (index : Nat)
  deriving DecidableEq, Repr

/-- How many dereferences an operand applies. -/
def Operand.depth : Operand → Nat
  | .deref k => k
  | .subscript _ => 1

/-- The type of an operand under the name's declared type. -/
def operandType (op : Operand) (declared : CType) : Option CType :=
  derefN op.depth declared

/-- What the total-or-reject declarator reader answers for one declaration of
the name: its star count, its array dimensions, and whether its specifier list
resolved as never variably modified. -/
structure DeclFact where
  stars : Nat
  dimensions : Nat
  specifierNeverVM : Bool
  deriving DecidableEq, Repr

/-- The C type a readable declaration fact describes, over a base whose
variably-modified flag the translation unit fixes. -/
def DeclFact.type (d : DeclFact) (baseVM : Bool) : CType :=
  (List.replicate d.dimensions ()).foldl (fun t _ => .arr t false) (pointers d.stars (.base baseVM))

/-- Faithful extraction (trust boundary 28): a specifier the reader resolves as
never variably modified is not variably modified. -/
def Faithful (d : DeclFact) (baseVM : Bool) : Prop :=
  d.specifierNeverVM = true → baseVM = false

/-- The reader's facts for a name: `none` when the function's declarations
were unreadable (a nested declarator such as `int (*p)[n]` refuses them all),
otherwise every declaration of the name. -/
abbrev Facts := Option (List DeclFact)

/-- One declaration admits `k` dereferences. -/
def DeclFact.admits (d : DeclFact) (k : Nat) : Bool :=
  d.dimensions == 0 && 1 ≤ k && k ≤ d.stars && d.specifierNeverVM

/-- The admission: the function was read, the name is declared, and **every**
declaration of it admits the operand's depth. -/
def admitted (facts : Facts) (op : Operand) : Bool :=
  match facts with
  | none => false
  | some ds => !ds.isEmpty && ds.all (fun d => d.admits op.depth)

/-! ## An admitted operand is never evaluated -/

theorem admits_spec {d : DeclFact} {k : Nat} (h : d.admits k = true) :
    d.dimensions = 0 ∧ 1 ≤ k ∧ k ≤ d.stars ∧ d.specifierNeverVM = true := by
  simp only [DeclFact.admits, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at h
  obtain ⟨⟨⟨h1, h2⟩, h3⟩, h4⟩ := h
  exact ⟨h1, h2, h3, h4⟩

/-- Under an admitted declaration the operand's type is pointers over a base
that is not variably modified. -/
theorem admitted_operand_type (d : DeclFact) (baseVM : Bool) (op : Operand)
    (hf : Faithful d baseVM) (h : d.admits op.depth = true) :
    operandType op (d.type baseVM) = some (pointers (d.stars - op.depth) (.base false)) := by
  obtain ⟨hd, _, hk, hs⟩ := admits_spec h
  have hb : baseVM = false := hf hs
  simp [operandType, DeclFact.type, hd, hb, derefN_pointers _ _ _ hk]

/-- **The rule's soundness fact.** Whichever declaration the operand's name
refers to, an admitted operand's type is not variably modified, so C does not
evaluate it. -/
theorem an_admitted_operand_is_not_evaluated (facts : List DeclFact) (op : Operand)
    (baseVM : DeclFact → Bool) (hf : ∀ d ∈ facts, Faithful d (baseVM d))
    (h : admitted (some facts) op = true) (d : DeclFact) (hd : d ∈ facts) :
    ∃ t, operandType op (d.type (baseVM d)) = some t ∧ t.vm = false ∧ evaluates t = false := by
  simp [admitted, List.all_eq_true] at h
  have hadm := h.2 d hd
  refine ⟨_, admitted_operand_type d (baseVM d) op (hf d hd) hadm, ?_, ?_⟩
  · simp [pointers_vm, CType.vm]
  · cases hstars : d.stars - op.depth with
    | zero => simp [pointers, evaluates, CType.isVLA]
    | succ n => simp [pointers, evaluates, CType.isVLA]

/-- The effects an operand's evaluation would have on the name it mentions. -/
inductive Effect where
  | readName
  | dereference
  deriving DecidableEq, Repr

/-- `sizeof`'s effects: the operand's, exactly when C evaluates it. -/
def sizeofEffects (t : CType) (operandEffects : List Effect) : List Effect :=
  if evaluates t then operandEffects else []

/-- An admitted operand reads, dereferences and writes nothing — so a released,
null or unbound name in it is no use. -/
theorem an_admitted_operand_has_no_effect (facts : List DeclFact) (op : Operand)
    (baseVM : DeclFact → Bool) (hf : ∀ d ∈ facts, Faithful d (baseVM d))
    (h : admitted (some facts) op = true) (d : DeclFact) (hd : d ∈ facts)
    (operandEffects : List Effect) :
    ∃ t, operandType op (d.type (baseVM d)) = some t ∧ sizeofEffects t operandEffects = [] := by
  obtain ⟨t, ht, _, he⟩ := an_admitted_operand_is_not_evaluated facts op baseVM hf h d hd
  exact ⟨t, ht, by simp [sizeofEffects, he]⟩

/-! ## Refusals -/

/-- A variably modified operand is evaluated: `int (*p)[n]; sizeof(*p)` reads
`n`'s array through `p`. Non-vacuity of the evaluation fact. -/
theorem a_variably_modified_operand_is_evaluated :
    operandType (.deref 1) (.ptr (.arr (.base false) true)) = some (.arr (.base false) true) ∧
      evaluates (.arr (.base false) true) = true := by
  decide

/-- An unreadable function's declarations admit nothing (a nested declarator
such as `int (*p)[n]` makes the reader refuse them all). -/
theorem an_unreadable_declaration_is_refused (op : Operand) : admitted none op = false := rfl

/-- An undeclared name (a file-scope object, say) is refused. -/
theorem an_undeclared_name_is_refused (op : Operand) : admitted (some []) op = false := rfl

/-- One array declaration of the name refuses the operand. -/
theorem an_array_declarator_is_refused (ds : List DeclFact) (d : DeclFact) (op : Operand)
    (hd : d ∈ ds) (harr : d.dimensions ≠ 0) : admitted (some ds) op = false := by
  simp only [admitted, Bool.and_eq_false_iff]
  right
  simp only [List.all_eq_false]
  exact ⟨d, hd, by simp [DeclFact.admits, harr]⟩

/-- A specifier the reader does not resolve as never variably modified — a
block-scope VLA typedef, an unresolved or disagreeing typedef — refuses. -/
theorem a_possibly_variably_modified_specifier_is_refused (ds : List DeclFact) (d : DeclFact)
    (op : Operand) (hd : d ∈ ds) (hs : d.specifierNeverVM = false) :
    admitted (some ds) op = false := by
  simp only [admitted, Bool.and_eq_false_iff]
  right
  simp only [List.all_eq_false]
  exact ⟨d, hd, by simp [DeclFact.admits, hs]⟩

/-- A deeper dereference than some declaration's stars refuses. -/
theorem a_deeper_dereference_is_refused (ds : List DeclFact) (d : DeclFact) (k : Nat)
    (hd : d ∈ ds) (hk : d.stars < k) : admitted (some ds) (.deref k) = false := by
  simp only [admitted, Bool.and_eq_false_iff]
  right
  simp only [List.all_eq_false]
  refine ⟨d, hd, ?_⟩
  have hnot : ¬ (k ≤ d.stars) := by omega
  simp [DeclFact.admits, Operand.depth, hnot]

/-- Zero stars is not a dereference operand. -/
theorem no_stars_is_refused (ds : List DeclFact) : admitted (some ds) (.deref 0) = false := by
  cases ds with
  | nil => rfl
  | cons d ds => simp [admitted, DeclFact.admits, Operand.depth]

/-! ## The gate view -/

/-- Statement tokens, as far as the lexical access gate reads them. -/
inductive Tok where
  | name (n : Nat)
  | literal (v : Nat)
  | star
  | arrow
  | bracket
  | other (code : Nat)
  /-- The scalar marker an admitted operand is replaced by. -/
  | marker
  deriving DecidableEq, Repr

/-- The gate's access predicate on one token (a coarse model: the Rust gate
also reads a star's left neighbour; the marker is a plain operand either way). -/
def Tok.access : Tok → Bool
  | .star => true
  | .arrow => true
  | .bracket => true
  | _ => false

/-- A statement split into pieces: an ordinary token, or an admitted operand
span (its original tokens). -/
inductive Piece where
  | tok (t : Tok)
  | operand (span : List Tok)
  deriving DecidableEq, Repr

/-- The original tokens. -/
def original : List Piece → List Tok
  | [] => []
  | .tok t :: rest => t :: original rest
  | .operand s :: rest => s ++ original rest

/-- The gate's view: every admitted operand becomes the marker. -/
def view : List Piece → List Tok
  | [] => []
  | .tok t :: rest => t :: view rest
  | .operand _ :: rest => .marker :: view rest

/-- The gate refuses a token list with an access token. -/
def gateRefuses (ts : List Tok) : Bool := ts.any Tok.access

/-- The marker is neither an access nor a literal: no literal-argument premise
can be satisfied by it and it is never a computed size (constraint 5a). -/
theorem the_marker_is_not_an_access_or_a_literal :
    Tok.marker.access = false ∧ ∀ v, Tok.marker ≠ .literal v := by
  refine ⟨rfl, ?_⟩
  intro v h
  cases h

/-- With no admitted operand the view is the old view, byte for byte. -/
theorem no_operand_leaves_the_gate_unchanged (ps : List Piece)
    (h : ∀ p ∈ ps, ∃ t, p = .tok t) : view ps = original ps := by
  induction ps with
  | nil => rfl
  | cons p rest ih =>
    obtain ⟨t, rfl⟩ := h p (by simp)
    simp [view, original, ih (fun q hq => h q (by simp [hq]))]

/-- The view never refuses where the original passed: the rule can only
withdraw a refusal, never create one. -/
theorem the_view_refuses_no_more (ps : List Piece) :
    gateRefuses (view ps) = true → gateRefuses (original ps) = true := by
  induction ps with
  | nil => simp [view, gateRefuses]
  | cons p rest ih =>
    cases p with
    | tok t =>
      simp only [view, original, gateRefuses, List.any_cons, Bool.or_eq_true]
      intro h
      rcases h with h | h
      · exact Or.inl h
      · exact Or.inr (ih h)
    | operand s =>
      simp only [view, original, gateRefuses, List.any_cons, List.any_append,
        Bool.or_eq_true, Tok.access]
      intro h
      rcases h with h | h
      · simp at h
      · exact Or.inr (ih h)

/-- A classifier with the gate in front: the lowering after the gate reads the
original tokens. -/
def classify (lower : List Tok → Option Nat) (gate : List Tok) (ts : List Tok) : Option Nat :=
  if gateRefuses gate then none else lower ts

/-- **An admitted statement keeps its lowering.** Every statement the old gate
passed is passed by the new view and lowered from the same original tokens. -/
theorem an_admitted_statement_keeps_its_lowering (lower : List Tok → Option Nat)
    (ps : List Piece) (h : gateRefuses (original ps) = false) :
    classify lower (view ps) (original ps) = classify lower (original ps) (original ps) := by
  have hv : gateRefuses (view ps) = false := by
    cases hvr : gateRefuses (view ps) with
    | false => rfl
    | true => rw [the_view_refuses_no_more ps hvr] at h; cases h
  simp [classify, hv, h]

/-- **A refusal stays a refusal** unless the view passes; nothing else changes
what the classifier returns. -/
theorem a_refusal_is_withdrawn_only_by_the_view (lower : List Tok → Option Nat)
    (ps : List Piece) (h : gateRefuses (view ps) = true) :
    classify lower (view ps) (original ps) = none := by
  simp [classify, h]

/-! ## The protected-name mention of a borrowed argument -/

/-- The names an argument mentions outside admitted operands. -/
def evaluatedNames : List Piece → List Nat
  | [] => []
  | .tok (.name n) :: rest => n :: evaluatedNames rest
  | .tok _ :: rest => evaluatedNames rest
  | .operand _ :: rest => evaluatedNames rest

/-- Every name an argument mentions. -/
def mentionedNames : List Piece → List Nat
  | [] => []
  | .tok (.name n) :: rest => n :: mentionedNames rest
  | .tok _ :: rest => mentionedNames rest
  | .operand s :: rest => (s.filterMap fun | .name n => some n | _ => none) ++ mentionedNames rest

/-- The filter only forgets names: every name it still sees was mentioned. -/
theorem the_mention_filter_only_forgets (ps : List Piece) (n : Nat) :
    n ∈ evaluatedNames ps → n ∈ mentionedNames ps := by
  induction ps with
  | nil => simp [evaluatedNames]
  | cons p rest ih =>
    cases p with
    | tok t =>
      intro h
      cases t <;> simp only [evaluatedNames, mentionedNames, List.mem_cons] at h ⊢ <;>
        first
        | exact ih h
        | exact h.imp id ih
    | operand s =>
      intro h
      simp only [mentionedNames, List.mem_append]
      exact Or.inr (ih h)

/-- With no admitted operand the filter forgets nothing. -/
theorem no_operand_forgets_no_name (ps : List Piece) (h : ∀ p ∈ ps, ∃ t, p = .tok t) :
    evaluatedNames ps = mentionedNames ps := by
  induction ps with
  | nil => rfl
  | cons p rest ih =>
    obtain ⟨t, rfl⟩ := h p (by simp)
    have := ih (fun q hq => h q (by simp [hq]))
    cases t <;> simp [evaluatedNames, mentionedNames, this]

/-! ## Non-vacuity -/

/-- `struct item **v; v = malloc(n * sizeof(*v));` — a declaration of two stars
over a tagged base admits `*v`, whose type `struct item *` is not evaluated. -/
theorem the_collection_allocation_is_admitted :
    admitted (some [{ stars := 2, dimensions := 0, specifierNeverVM := true }]) (.deref 1) = true ∧
      operandType (.deref 1) (DeclFact.type { stars := 2, dimensions := 0, specifierNeverVM := true } false)
        = some (.ptr (.base false)) := by
  decide

/-- The gate refused the allocation for the operand's star and passes the view. -/
theorem the_allocation_gate_passes_the_view :
    let ps := [Piece.tok (.name 0), .tok (.other 1), .operand [.star, .name 0], .tok (.other 2), .tok (.name 3)]
    gateRefuses (original ps) = true ∧ gateRefuses (view ps) = false := by
  decide

end QCore.SizeofOperand
