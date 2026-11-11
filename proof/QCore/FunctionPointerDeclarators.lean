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
# Function-pointer declarators in the lexical declaration reader

Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (ledger `RECORD-BODY-TOTAL-OR-REJECT`):
the model of the two readings `crates/qcore-c-gateway/src/typed_field_access.rs`
changes.

* **Record bodies** (`read_record_body`). A body is a list of member
  declarations. Before the card any parenthesised declaration refused the whole
  body. Now a declaration that is exactly a function-pointer declarator is
  skipped — it joins no class — and every other unreadable declaration still
  refuses the whole body.
* **Formals** (`read_parameters`). A function-pointer formal is read as a name
  of no committed type (`opaque`) instead of refusing every declaration of the
  function.

The member and formal types, and which declarations parse, are premises the
reader supplies (as in `QCore.TypedFieldAccess`); nothing here proves the
reader extracts them faithfully from C.
-/

namespace QCore.FunctionPointerDeclarators

/-- A member's class, as far as the reader commits. -/
inductive MemberClass where
  | scalar
  | pointer
  | aggregate
  deriving DecidableEq, Repr

/-- One member declaration of a record body. -/
inductive MemberDecl where
  /-- A declaration the reader parses, with the class it gives the name. -/
  | read (name : String) (cls : MemberClass)
  /-- Exactly `SPECIFIERS ( * NAME ) ( PARAMETERS )`. -/
  | fnPointer (name : String)
  /-- Any other declaration the reader does not parse (a bitfield, a nested
      aggregate, any other parenthesised declarator). -/
  | unreadable
  deriving DecidableEq, Repr

def MemberDecl.isFnPointer : MemberDecl → Bool
  | .fnPointer _ => true
  | _ => false

/-- The reading after the card: function-pointer members are skipped. -/
def readBody : List MemberDecl → Option (List (String × MemberClass))
  | [] => some []
  | .unreadable :: _ => none
  | .fnPointer _ :: rest => readBody rest
  | .read name cls :: rest => (readBody rest).map ((name, cls) :: ·)

/-- The reading before the card: any function-pointer member refuses too. -/
def readBodyBefore (body : List MemberDecl) : Option (List (String × MemberClass)) :=
  if body.any MemberDecl.isFnPointer then none else readBody body

/-- The class the reading gives one member name, if any. -/
def classOf (body : List MemberDecl) (field : String) : Option MemberClass :=
  (readBody body).bind fun members => (members.find? (·.1 == field)).map (·.2)

/-- Every other declaration still refuses the whole body. -/
theorem an_unreadable_member_refuses_the_record (body : List MemberDecl)
    (h : MemberDecl.unreadable ∈ body) : readBody body = none := by
  induction body with
  | nil => cases h
  | cons d rest ih =>
    cases d with
    | unreadable => rfl
    | fnPointer n =>
      simp only [readBody]
      exact ih (by cases h with
        | tail _ h => exact h)
    | read n c =>
      simp only [readBody]
      rw [ih (by cases h with
        | tail _ h => exact h)]
      rfl

/-- A body with no unreadable declaration is read. -/
theorem a_body_without_unreadable_members_is_read (body : List MemberDecl)
    (h : MemberDecl.unreadable ∉ body) : (readBody body).isSome := by
  induction body with
  | nil => rfl
  | cons d rest ih =>
    have hrest : MemberDecl.unreadable ∉ rest := fun m => h (List.mem_cons_of_mem _ m)
    cases d with
    | unreadable => exact absurd (List.mem_cons_self) h
    | fnPointer n => simpa [readBody] using ih hrest
    | read n c =>
      simp only [readBody]
      cases hr : readBody rest with
      | none => rw [hr] at ih; simp at ih; exact absurd ih hrest
      | some m => rfl

/-- The reading extends the old one: every body the old reader read is read
    to the same members. -/
theorem the_reading_extends_the_old_reading (body : List MemberDecl)
    (members : List (String × MemberClass))
    (h : readBodyBefore body = some members) : readBody body = some members := by
  unfold readBodyBefore at h
  split at h
  · contradiction
  · exact h

/-- Skipping a function-pointer member changes no other member's reading:
    the body reads exactly as the same body without its function pointers. -/
theorem skipping_preserves_the_other_members (body : List MemberDecl) :
    readBody body = readBody (body.filter fun d => !d.isFnPointer) := by
  induction body with
  | nil => rfl
  | cons d rest ih =>
    cases d with
    | unreadable => rfl
    | fnPointer n =>
      rw [List.filter_cons_of_neg (by simp [MemberDecl.isFnPointer])]
      simp only [readBody]
      exact ih
    | read n c =>
      rw [List.filter_cons_of_pos (by rfl)]
      simp only [readBody]
      rw [ih]

/-- Every member the reading returns is a parsed declaration of the body. -/
theorem a_read_member_is_a_parsed_declaration (body : List MemberDecl)
    (members : List (String × MemberClass)) (h : readBody body = some members)
    (x : String × MemberClass) (hx : x ∈ members) : MemberDecl.read x.1 x.2 ∈ body := by
  induction body generalizing members with
  | nil =>
    simp only [readBody, Option.some.injEq] at h
    subst h; cases hx
  | cons d rest ih =>
    cases d with
    | unreadable => simp [readBody] at h
    | fnPointer n =>
      exact List.mem_cons_of_mem _ (ih members (by simpa [readBody] using h) hx)
    | read n c =>
      simp only [readBody] at h
      cases hr : readBody rest with
      | none => rw [hr] at h; simp at h
      | some m =>
        rw [hr] at h
        simp only [Option.map_some, Option.some.injEq] at h
        subst h
        cases hx with
        | head => exact List.mem_cons_self
        | tail _ hm => exact List.mem_cons_of_mem _ (ih m hr hm)

/-- A function-pointer member joins no class: its name gets none, given that
    no parsed member of the body carries the same name (C forbids duplicate
    member names). -/
theorem a_function_pointer_member_is_never_classified (body : List MemberDecl)
    (name : String) (distinct : ∀ c, MemberDecl.read name c ∉ body) :
    classOf body name = none := by
  unfold classOf
  cases hb : readBody body with
  | none => rfl
  | some members =>
    simp only [Option.bind_some, Option.map_eq_none_iff, List.find?_eq_none]
    intro x hx hname
    have hmem := a_read_member_is_a_parsed_declaration body members hb x hx
    have : x.1 = name := by simpa using hname
    rw [this] at hmem
    exact distinct x.2 hmem

/-! ## Rule 5g's store route keeps its domain

Rule 5g lowers `BASE->FIELD = scalar` to a store whose flow check refuses an
unsettled base, before the statement's refusal reaches the later fallbacks.
It reads the member through the old reading, so a record read past a
function-pointer member stays outside it and the statement keeps its route
order: the reader repair only admits. -/

/-- The class rule 5g's store route reads: the old, whole-record reading. -/
def storeRouteClass (body : List MemberDecl) (field : String) : Option MemberClass :=
  (readBodyBefore body).bind fun members => (members.find? (·.1 == field)).map (·.2)

/-- A record read past a function-pointer member is outside the store route. -/
theorem a_record_read_past_a_function_pointer_is_outside_the_store_route
    (body : List MemberDecl) (field : String)
    (h : body.any MemberDecl.isFnPointer = true) : storeRouteClass body field = none := by
  simp [storeRouteClass, readBodyBefore, h]

/-- On a record read with every member, the store route and the reading agree. -/
theorem on_a_fully_read_record_the_store_route_reads_as_before
    (body : List MemberDecl) (field : String)
    (h : body.any MemberDecl.isFnPointer = false) :
    storeRouteClass body field = classOf body field := by
  simp [storeRouteClass, classOf, readBodyBefore, h]

/-! ## Formals -/

/-- A formal's declared type, as far as the reader commits. `opaque` is never
    admitted by any typed rule (`QCore.TypedFieldAccess`'s whitelist admits
    only scalar members through committed record pointers). -/
inductive DeclaredType where
  | scalar
  | recordPointer (tag : String)
  | opaque
  deriving DecidableEq, Repr

inductive Formal where
  | read (name : String) (ty : DeclaredType)
  | fnPointer (name : String)
  | unreadable
  deriving DecidableEq, Repr

/-- The formals' reading after the card: `none` refuses every declaration of
    the function. -/
def readFormals : List Formal → Option (List (String × DeclaredType))
  | [] => some []
  | .unreadable :: _ => none
  | .fnPointer name :: rest => (readFormals rest).map ((name, .opaque) :: ·)
  | .read name ty :: rest => (readFormals rest).map ((name, ty) :: ·)

/-- A typed rule commits only to a scalar or a record pointer. -/
def committed : DeclaredType → Bool
  | .scalar => true
  | .recordPointer _ => true
  | .opaque => false

/-- A function-pointer formal is read, and read as `opaque`. -/
theorem a_function_pointer_formal_is_opaque (rest : List Formal) (name : String)
    (members : List (String × DeclaredType)) (h : readFormals rest = some members) :
    readFormals (.fnPointer name :: rest) = some ((name, .opaque) :: members) := by
  simp [readFormals, h]

/-- So no typed rule commits to it. -/
theorem an_opaque_formal_is_never_committed : committed .opaque = false := rfl

/-- Every other unparsed formal still refuses the whole function. -/
theorem an_unreadable_formal_refuses_the_function (formals : List Formal)
    (h : Formal.unreadable ∈ formals) : readFormals formals = none := by
  induction formals with
  | nil => cases h
  | cons d rest ih =>
    cases d with
    | unreadable => rfl
    | fnPointer n =>
      have hrest : Formal.unreadable ∈ rest := by
        cases h with
        | tail _ h => exact h
      simp [readFormals, ih hrest]
    | read n t =>
      have hrest : Formal.unreadable ∈ rest := by
        cases h with
        | tail _ h => exact h
      simp [readFormals, ih hrest]

def Formal.isFnPointer : Formal → Bool
  | .fnPointer _ => true
  | _ => false

/-- The formals as rule 5g's store route reads them: the old reading, which
    refused every declaration of a function with a function-pointer formal. -/
def storeRouteFormals (formals : List Formal) : Option (List (String × DeclaredType)) :=
  if formals.any Formal.isFnPointer then none else readFormals formals

/-- A function read past a function-pointer formal is outside the store route. -/
theorem a_function_read_past_a_function_pointer_formal_is_outside_the_store_route
    (formals : List Formal) (h : formals.any Formal.isFnPointer = true) :
    storeRouteFormals formals = none := by
  simp [storeRouteFormals, h]

/-! ## Vectors (shared with the Rust unit tests' shapes) -/

/-- `struct r { int a ; void ( * f ) ( int ) ; struct q * p ; }` reads `a` and
    `p`, and gives `f` no class. -/
def vectorBody : List MemberDecl :=
  [.read "a" .scalar, .fnPointer "f", .read "p" .pointer]

theorem vectors_hold :
    classOf vectorBody "a" = some .scalar ∧
    classOf vectorBody "p" = some .pointer ∧
    classOf vectorBody "f" = none ∧
    readBodyBefore vectorBody = none ∧
    readBody [.read "a" .scalar, .fnPointer "f", .unreadable] = none := by
  decide

end QCore.FunctionPointerDeclarators
