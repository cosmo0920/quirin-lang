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
# Symbolic premise statements (card SP1-SYMBOLIC-OPENSSL-PREMISES)

Contract specification C04.18.9, grammar G05. A named premise may state
`requires resource_premise(NAME, FORMULA);`, a formula over a closed
vocabulary of sorted terms. This module is the formula language's model:

* **Syntax** (`Raw`): the tree `crates/qcore-frontend/src/premise_formula.rs`
  reads (`parse_tokens`), rendered by its `lean` function.
* **Sorts and well-formedness** (`check`, `wf`): the judgment the loader
  applies (`check` there): every term registered with its signature
  (`vocab`), every constant a class, every variable bound and
  never shadowing, the whole a proposition.
* **Denotation** (`eval`) over an abstract model (`Model`) of the call's
  objects, callback sets and the runs of each callback.

Theorems:

* well-formed formulas denote (`wf_denotes`, from `check_denotes`), and so
  does a well-formed quantifier's body at every element of its domain
  (`quant_body_denotes`);
* the loader's acceptance agrees with the judgment on the shared decision
  vectors (`decision_vectors_agree`; bound to the Rust table
  `DECISION_VECTORS` by `crates/qcore-frontend/tests/premise_formula.rs`,
  which also binds `vocab` and the four formulas below to the Rust reader);
* the six OpenSSL statements are well-formed (`openssl_formulas_wf`);
* `#inert` is its definition (`inert_definition`);
* `openssl_unstated_callback_slots` entails `openssl_ex_data_callbacks` in
  every coherent model (`unstated_entails_ex_data`), in particular for the
  SSL object's ex_data class (`unstated_entails_ex_data_for_ssl`).

## What this is not

A premise is an undischarged assumption (C04.18): nothing here, and no
consumer that decides a verdict, establishes that a model describes OpenSSL.
That the Rust reader computes `check` is pinned by the shared vectors, not
proved; syntax rejections (a malformed token sequence) are the Rust reader's
and are tested there.
-/

namespace QCore.PremiseFormula

/-! ## Syntax -/

inductive Srt where
  | obj | cls | fn | meth | res | sobj | sfn | smeth | sres | prop | eff
  deriving DecidableEq, Repr

inductive Op where
  | imp | conj | mapsTo | mem | eqv | union | diff
  deriving DecidableEq, Repr

inductive Raw where
  | quant (v : String) (dom body : Raw)
  | bin (op : Op) (l r : Raw)
  | app0 (f : String)
  | app1 (f : String) (a : Raw)
  | app2 (f : String) (a b : Raw)
  | const (c : String)
  | empty
  deriving DecidableEq, Repr

/-! ## The closed vocabulary (the Rust `VOCABULARY`, `CLASSES`) -/

def vocab : List (String × List Srt × Srt) := [
  ("args", [], .sobj),
  ("created", [], .sobj),
  ("copied", [], .sobj),
  ("freed", [], .sobj),
  ("class", [.obj], .cls),
  ("of", [.cls, .sobj], .sobj),
  ("ctx", [.sobj], .sobj),
  ("bios", [.sobj], .sobj),
  ("chain", [.sobj], .sobj),
  ("ssl_of", [.obj], .sobj),
  ("slots", [.sobj], .sfn),
  ("stated", [], .sfn),
  ("ex_data", [.cls], .sfn),
  ("ex_data_free", [.cls], .sfn),
  ("method", [.obj], .meth),
  ("builtin", [], .smeth),
  ("fns", [.obj], .sfn),
  ("store", [.sobj], .sobj),
  ("lookups", [.sobj], .sobj),
  ("teardown", [.obj], .sfn),
  ("settable", [], .sfn),
  ("library", [], .sfn),
  ("loaded", [], .sfn),
  ("in_turn", [.fn, .sobj], .sfn),
  ("caller_held", [], .sres),
  ("takes", [.sres], .sres),
  ("releases", [.sres], .sres),
  ("retains", [.sres], .sres),
  ("inert", [], .eff)
]

def classes : List String :=
  ["BIO", "DH", "DSA", "EC_KEY", "ENGINE", "EVP_PKEY", "RSA", "SSL", "SSL_CTX",
    "SSL_SESSION", "UI", "UI_METHOD", "X509", "X509_STORE", "X509_STORE_CTX"]

/-- A context or environment lookup: the most recent binding first. -/
def lk {α : Type} : List (String × α) → String → Option α
  | [], _ => none
  | (k, v) :: rest, x => if k = x then some v else lk rest x

def term (f : String) : Option (List Srt × Srt) := lk vocab f

def Srt.setOf : Srt → Option Srt
  | .obj => some .sobj
  | .fn => some .sfn
  | .meth => some .smeth
  | _ => none

def Srt.elem : Srt → Option Srt
  | .sobj => some .obj
  | .sfn => some .fn
  | .smeth => some .meth
  | _ => none

/-- The sort of a binary operation (the Rust `sort_of`'s `Raw::Bin` arm). -/
def binSort : Op → Srt → Srt → Option Srt
  | .imp, .prop, .prop => some .prop
  | .conj, .prop, .prop => some .prop
  | .conj, .eff, .eff => some .eff
  | .mapsTo, .fn, .eff => some .prop
  | .mapsTo, .sfn, .eff => some .prop
  | .mem, a, b => if a.setOf = some b then some .prop else none
  | .eqv, .sres, .sres => some .eff
  | .union, .sobj, .sobj => some .sobj
  | .union, .sfn, .sfn => some .sfn
  | .union, .smeth, .smeth => some .smeth
  | .diff, .sobj, .sobj => some .sobj
  | .diff, .sfn, .sfn => some .sfn
  | .diff, .smeth, .smeth => some .smeth
  | _, _, _ => none

def bound (Γ : List (String × Srt)) (v : String) : Bool :=
  (lk Γ v).isSome

/-- The sort judgment (the Rust `sort_of`). -/
def check (Γ : List (String × Srt)) : Raw → Option Srt
  | .quant v d b =>
      if (term v).isSome || bound Γ v then none
      else
        match (check Γ d).bind Srt.elem with
        | some e => if check ((v, e) :: Γ) b = some .prop then some .prop else none
        | none => none
  | .bin op l r =>
      match check Γ l, check Γ r with
      | some a, some c => binSort op a c
      | _, _ => none
  | .app0 f =>
      match lk Γ f with
      | some s => some s
      | none =>
          match term f with
          | some ([], s) => some s
          | _ => none
  | .app1 f a =>
      match term f, check Γ a with
      | some ([x], s), some y => if x = y then some s else none
      | _, _ => none
  | .app2 f a b =>
      match term f, check Γ a, check Γ b with
      | some ([x, y], s), some x', some y' => if x = x' ∧ y = y' then some s else none
      | _, _, _ => none
  | .const c => if classes.contains c then some .cls else none
  | .empty => some .sres

/-- The well-formedness judgment: a closed proposition (the Rust `check`). -/
def wf (t : Raw) : Bool := check [] t == some .prop

/-! ## Denotation -/

/-- An abstract model of one call: its objects, classes, methods, callbacks
are elements of `U`; sets are predicates; `runs f` are the runs
of callback `f` during the call; the resource terms are read per run. -/
structure Model where
  U : Type
  Run : Type
  args : U → Prop
  created : U → Prop
  copied : U → Prop
  freed : U → Prop
  cls : U → U
  ctx : (U → Prop) → U → Prop
  bios : (U → Prop) → U → Prop
  chain : (U → Prop) → U → Prop
  sslOf : U → U → Prop
  slots : (U → Prop) → U → Prop
  stated : U → Prop
  exData : U → U → Prop
  exDataFree : U → U → Prop
  method : U → U
  builtin : U → Prop
  fns : U → U → Prop
  store : (U → Prop) → U → Prop
  lookups : (U → Prop) → U → Prop
  teardown : U → U → Prop
  settable : U → Prop
  library : U → Prop
  loaded : U → Prop
  inTurn : U → (U → Prop) → U → Prop
  callerHeld : Run → U → Prop
  takes : Run → (U → Prop) → U → Prop
  releases : Run → (U → Prop) → U → Prop
  retains : Run → (U → Prop) → U → Prop
  runs : U → Run → Prop
  const : String → U

/-- A value: an element, a set, a per-run resource set, a proposition, a
property of a run, or nothing (an ill-sorted application). -/
inductive Val (M : Model) where
  | el (u : M.U)
  | st (s : M.U → Prop)
  | rs (s : M.Run → M.U → Prop)
  | pr (p : Prop)
  | ef (e : M.Run → Prop)
  | bad

def Val.holds {M : Model} : Val M → Prop
  | .pr p => p
  | _ => False

def noRes {M : Model} : M.U → Prop := fun _ => False

/-- `#inert`, spelled out exactly as `eval` reads its definition. -/
def inertEff (M : Model) : M.Run → Prop := fun r =>
  ((M.takes r (M.callerHeld r) = noRes ∧ M.releases r (M.callerHeld r) = noRes) ∧
    M.retains r (M.callerHeld r) = noRes)

def prim0 (M : Model) (f : String) : Val M :=
  if f = "args" then .st M.args
  else if f = "created" then .st M.created
  else if f = "copied" then .st M.copied
  else if f = "freed" then .st M.freed
  else if f = "stated" then .st M.stated
  else if f = "builtin" then .st M.builtin
  else if f = "settable" then .st M.settable
  else if f = "library" then .st M.library
  else if f = "loaded" then .st M.loaded
  else if f = "caller_held" then .rs M.callerHeld
  else if f = "inert" then .ef (inertEff M)
  else .bad

def prim1 (M : Model) (f : String) : Val M → Val M
  | .el o =>
      if f = "class" then .el (M.cls o)
      else if f = "ssl_of" then .st (M.sslOf o)
      else if f = "ex_data" then .st (M.exData o)
      else if f = "ex_data_free" then .st (M.exDataFree o)
      else if f = "method" then .el (M.method o)
      else if f = "fns" then .st (M.fns o)
      else if f = "teardown" then .st (M.teardown o)
      else .bad
  | .st s =>
      if f = "ctx" then .st (M.ctx s)
      else if f = "bios" then .st (M.bios s)
      else if f = "chain" then .st (M.chain s)
      else if f = "slots" then .st (M.slots s)
      else if f = "store" then .st (M.store s)
      else if f = "lookups" then .st (M.lookups s)
      else .bad
  | .rs s =>
      if f = "takes" then .rs (fun r => M.takes r (s r))
      else if f = "releases" then .rs (fun r => M.releases r (s r))
      else if f = "retains" then .rs (fun r => M.retains r (s r))
      else .bad
  | _ => .bad

def prim2 (M : Model) (f : String) : Val M → Val M → Val M
  | .el k, .st s =>
      if f = "of" then .st (fun o => s o ∧ M.cls o = k)
      else if f = "in_turn" then .st (M.inTurn k s)
      else .bad
  | _, _ => .bad

/-- `#of(k, S)` is the members of `S` of class `k`; `F |-> E` is every run of
callback `F` (or of every member of set `F`) during the call having `E`. -/
def binVal (M : Model) : Op → Val M → Val M → Val M
  | .imp, .pr p, .pr q => .pr (p → q)
  | .conj, .pr p, .pr q => .pr (p ∧ q)
  | .conj, .ef e, .ef g => .ef (fun r => e r ∧ g r)
  | .mapsTo, .el f, .ef e => .pr (∀ r, M.runs f r → e r)
  | .mapsTo, .st F, .ef e => .pr (∀ f, F f → ∀ r, M.runs f r → e r)
  | .mem, .el u, .st s => .pr (s u)
  | .eqv, .rs a, .rs b => .ef (fun r => a r = b r)
  | .union, .st a, .st b => .st (fun u => a u ∨ b u)
  | .diff, .st a, .st b => .st (fun u => a u ∧ ¬ b u)
  | _, _, _ => .bad

def eval (M : Model) : List (String × Val M) → Raw → Val M
  | ρ, .quant v d b =>
      match eval M ρ d with
      | .st s => .pr (∀ u, s u → (eval M ((v, .el u) :: ρ) b).holds)
      | _ => .bad
  | ρ, .bin op l r => binVal M op (eval M ρ l) (eval M ρ r)
  | ρ, .app0 f =>
      match lk ρ f with
      | some x => x
      | none => prim0 M f
  | ρ, .app1 f a => prim1 M f (eval M ρ a)
  | ρ, .app2 f a b => prim2 M f (eval M ρ a) (eval M ρ b)
  | _, .const c => .el (M.const c)
  | _, .empty => .rs (fun _ => noRes)

/-! ## Well-formed formulas denote -/

inductive Kind where
  | el | st | rs | pr | ef
  deriving DecidableEq, Repr

def Srt.kind : Srt → Kind
  | .obj | .cls | .fn | .meth | .res => .el
  | .sobj | .sfn | .smeth => .st
  | .sres => .rs
  | .prop => .pr
  | .eff => .ef

def Val.kind {M : Model} : Val M → Option Kind
  | .el _ => some .el
  | .st _ => some .st
  | .rs _ => some .rs
  | .pr _ => some .pr
  | .ef _ => some .ef
  | .bad => none

/-- An environment agrees with a context: the same names, in order, each
value of its sort's kind. -/
inductive Agree {M : Model} : List (String × Srt) → List (String × Val M) → Prop
  | nil : Agree [] []
  | cons {Γ ρ} (x : String) (s : Srt) (v : Val M) :
      v.kind = some s.kind → Agree Γ ρ → Agree ((x, s) :: Γ) ((x, v) :: ρ)

theorem agree_lookup {M : Model} {Γ : List (String × Srt)} {ρ : List (String × Val M)}
    (h : Agree Γ ρ) (x : String) :
    (lk Γ x = none ∧ lk ρ x = none) ∨
      ∃ s v, lk Γ x = some s ∧ lk ρ x = some v ∧ v.kind = some s.kind := by
  induction h with
  | nil => exact Or.inl ⟨rfl, rfl⟩
  | cons y s v hv _ ih =>
      by_cases hy : y = x
      · exact Or.inr ⟨s, v, by simp [lk, hy], by simp [lk, hy], hv⟩
      · simpa [lk, hy] using ih

theorem term_mem {f : String} {sig : List Srt × Srt} (h : term f = some sig) :
    (f, sig) ∈ vocab := by
  unfold term at h
  generalize vocab = l at h
  induction l with
  | nil => simp [lk] at h
  | cons p rest ih =>
      obtain ⟨k, w⟩ := p
      by_cases hk : k = f
      · subst hk; simp [lk] at h; subst h; exact List.mem_cons_self ..
      · simp only [lk, hk, ↓reduceIte] at h; exact List.mem_cons_of_mem _ (ih h)

theorem prim0_kind (M : Model) {f : String} {s : Srt} (h : term f = some ([], s)) :
    (prim0 M f).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  rcases hm with ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ <;>
    simp_all [prim0, Val.kind, Srt.kind]

theorem prim1_kind (M : Model) {f : String} {x s : Srt} {v : Val M}
    (h : term f = some ([x], s)) (hv : v.kind = some x.kind) :
    (prim1 M f v).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  rcases hm with ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ <;>
    (simp at hs) <;>
    (try obtain ⟨rfl, rfl⟩ := hs) <;>
    (cases v <;> simp_all [prim1, Val.kind, Srt.kind])

theorem prim2_kind (M : Model) {f : String} {x y s : Srt} {v w : Val M}
    (h : term f = some ([x, y], s)) (hv : v.kind = some x.kind)
    (hw : w.kind = some y.kind) :
    (prim2 M f v w).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  rcases hm with ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ | ⟨rfl, hs⟩ |
      ⟨rfl, hs⟩ | ⟨rfl, hs⟩ <;>
    (simp at hs) <;>
    (try obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hs) <;>
    (cases v <;> cases w <;> simp_all [prim2, Val.kind, Srt.kind])

theorem binVal_kind (M : Model) {op : Op} {a b s : Srt} {v w : Val M}
    (h : binSort op a b = some s) (hv : v.kind = some a.kind)
    (hw : w.kind = some b.kind) : (binVal M op v w).kind = some s.kind := by
  cases op <;> cases a <;> cases b <;> simp [binSort, Srt.setOf] at h <;> subst h <;>
    (cases v <;> cases w <;> simp_all [binVal, Val.kind, Srt.kind])

theorem elem_kind {d e : Srt} (h : d.elem = some e) : d.kind = .st ∧ e.kind = .el := by
  cases d <;> simp [Srt.elem] at h <;> subst h <;> exact ⟨rfl, rfl⟩

/-- Every well-sorted term denotes a value of its sort's kind. -/
theorem check_denotes (M : Model) :
    ∀ (t : Raw) (Γ : List (String × Srt)) (ρ : List (String × Val M)) (s : Srt),
      Agree Γ ρ → check Γ t = some s → (eval M ρ t).kind = some s.kind := by
  intro t
  induction t with
  | quant v d b ihd _ =>
      intro Γ ρ s hA h
      simp only [check] at h
      split at h
      · exact absurd h (by simp)
      · cases hd : check Γ d with
        | none => simp [hd] at h
        | some ds =>
            cases he : ds.elem with
            | none => simp [hd, he] at h
            | some e =>
                simp only [hd, he, Option.bind_some] at h
                split at h
                · have hs : s = .prop := by simpa using h.symm
                  subst hs
                  have hk := ihd Γ ρ ds hA hd
                  rw [(elem_kind he).1] at hk
                  simp only [eval]
                  cases hev : eval M ρ d <;> simp_all [Val.kind, Srt.kind]
                · simp at h
  | bin op l r ihl ihr =>
      intro Γ ρ s hA h
      simp only [check] at h
      cases hl : check Γ l <;> cases hr : check Γ r <;> simp [hl, hr] at h
      exact binVal_kind M h (ihl Γ ρ _ hA hl) (ihr Γ ρ _ hA hr)
  | app0 f =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      rcases agree_lookup hA f with ⟨hΓ, hρ⟩ | ⟨s', v, hΓ, hρ, hk⟩
      · rw [hΓ] at h
        rw [hρ]
        cases ht : term f with
        | none => simp [ht] at h
        | some sig =>
            obtain ⟨args, out⟩ := sig
            cases args with
            | nil =>
                simp [ht] at h
                subst h
                exact prim0_kind M ht
            | cons _ _ => simp [ht] at h
      · rw [hΓ] at h
        simp at h
        subst h
        rw [hρ]
        exact hk
  | app1 f a iha =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      cases ht : term f with
      | none => simp [ht] at h
      | some sig =>
          obtain ⟨args, out⟩ := sig
          cases ha : check Γ a with
          | none => simp [ht, ha] at h
          | some y =>
              rcases args with _ | ⟨x, _ | ⟨_, _⟩⟩
              · simp [ht, ha] at h
              · by_cases hxy : x = y
                · subst hxy
                  simp [ht, ha] at h
                  subst h
                  exact prim1_kind M ht (iha Γ ρ _ hA ha)
                · simp [ht, ha, hxy] at h
              · simp [ht, ha] at h
  | app2 f a b iha ihb =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      cases ht : term f with
      | none => simp [ht] at h
      | some sig =>
          obtain ⟨args, out⟩ := sig
          cases ha : check Γ a with
          | none => simp [ht, ha] at h
          | some x' =>
              cases hb : check Γ b with
              | none => simp [ht, ha, hb] at h
              | some y' =>
                  rcases args with _ | ⟨x, _ | ⟨y, _ | ⟨_, _⟩⟩⟩
                  · simp [ht, ha, hb] at h
                  · simp [ht, ha, hb] at h
                  · by_cases hxy : x = x' ∧ y = y'
                    · obtain ⟨rfl, rfl⟩ := hxy
                      simp [ht, ha, hb] at h
                      subst h
                      exact prim2_kind M ht (iha Γ ρ _ hA ha) (ihb Γ ρ _ hA hb)
                    · simp [ht, ha, hb, hxy] at h
                  · simp [ht, ha, hb] at h
  | const c =>
      intro Γ ρ s _ h
      simp only [check] at h
      split at h
      · simp at h; subst h; rfl
      · simp at h
  | empty =>
      intro Γ ρ s _ h
      simp only [check] at h
      simp at h
      subst h
      rfl

/-- A well-formed formula denotes a proposition. -/
theorem wf_denotes (M : Model) {t : Raw} (h : wf t = true) :
    ∃ p, eval M [] t = .pr p := by
  have hc : check [] t = some .prop := by simpa [wf] using h
  have hk := check_denotes M t [] [] .prop Agree.nil hc
  cases he : eval M [] t with
  | pr p => exact ⟨p, rfl⟩
  | _ => simp_all [Val.kind, Srt.kind]

/-- A well-formed quantifier's body denotes a proposition at every element
of its domain. -/
theorem quant_body_denotes (M : Model) {Γ : List (String × Srt)}
    {ρ : List (String × Val M)} {v : String} {d b : Raw} (hA : Agree Γ ρ)
    (h : check Γ (.quant v d b) = some .prop) (u : M.U) :
    ∃ p, eval M ((v, .el u) :: ρ) b = .pr p := by
  simp only [check] at h
  split at h
  · simp at h
  · cases hd : check Γ d with
    | none => simp [hd] at h
    | some ds =>
        cases he : ds.elem with
        | none => simp [hd, he] at h
        | some e =>
            simp only [hd, he, Option.bind_some] at h
            split at h
            · rename_i hb
              have hk := check_denotes M b ((v, e) :: Γ) ((v, .el u) :: ρ) .prop
                (Agree.cons v e (.el u) (by simp [Val.kind, (elem_kind he).2]) hA) hb
              cases hev : eval M ((v, .el u) :: ρ) b with
              | pr p => exact ⟨p, rfl⟩
              | _ => simp_all [Val.kind, Srt.kind]
            · simp at h

/-! ## The defined term -/

/-- `#takes(#caller_held) == {} && #releases(#caller_held) == {} &&
#retains(#caller_held) == {}` (the Rust `INERT_DEFINITION`). -/
def inertDefinition : Raw := (.bin .conj (.bin .conj (.bin .eqv (.app1 "takes" (.app0 "caller_held")) .empty) (.bin .eqv (.app1 "releases" (.app0 "caller_held")) .empty)) (.bin .eqv (.app1 "retains" (.app0 "caller_held")) .empty))

theorem inert_definition (M : Model) :
    eval M [] inertDefinition = eval M [] (.app0 "inert") := by
  simp [eval, inertDefinition, lk, prim0, prim1, binVal]
  rfl

theorem inert_definition_sort : check [] inertDefinition = some .eff := by decide

/-! ## Decision vectors (the Rust `DECISION_VECTORS`) -/

def decisionVectors : List (Raw × Bool) := [
  ((.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "inert"))), true),
  ((.bin .mapsTo (.app0 "stated") (.bin .conj (.bin .conj (.bin .eqv (.app1 "takes" (.app0 "caller_held")) .empty) (.bin .eqv (.app1 "releases" (.app0 "caller_held")) .empty)) (.bin .eqv (.app1 "retains" (.app0 "caller_held")) .empty))), true),
  ((.quant "f" (.app0 "stated") (.bin .mapsTo (.app0 "f") (.app0 "inert"))), true),
  ((.quant "b" (.app2 "of" (.const "BIO") (.app0 "args")) (.bin .imp (.bin .mem (.app0 "b") (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data_free" (.app1 "class" (.app0 "b"))) (.app0 "inert")))), true),
  ((.quant "b" (.app0 "args") (.bin .imp (.bin .mem (.app1 "method" (.app0 "b")) (.app0 "builtin")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert")))), true),
  ((.bin .conj (.bin .mapsTo (.bin .diff (.app1 "slots" (.app0 "args")) (.app0 "stated")) (.app0 "inert")) (.bin .mapsTo (.app1 "slots" (.app1 "ctx" (.app0 "args"))) (.app0 "inert"))), true),
  ((.bin .mapsTo (.app0 "unknown") (.app0 "inert")), false),
  ((.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "args"))) (.app0 "inert")), false),
  ((.quant "o" (.app0 "created") (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "x"))) (.app0 "inert"))), false),
  ((.quant "args" (.app0 "created") (.bin .mapsTo (.app0 "stated") (.app0 "inert"))), false),
  ((.quant "o" (.app0 "created") (.quant "o" (.app0 "freed") (.bin .mapsTo (.app0 "stated") (.app0 "inert")))), false),
  ((.app0 "stated"), false),
  ((.app0 "inert"), false),
  ((.bin .mapsTo (.bin .union (.app0 "created") (.app0 "stated")) (.app0 "inert")), false),
  ((.bin .mem (.const "SSL") (.app0 "args")), false),
  ((.bin .mapsTo (.app0 "freed") (.app0 "inert")), false),
  ((.quant "b" (.app2 "of" (.const "NOT_A_CLASS") (.app0 "args")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert"))), false),
  ((.quant "b" (.app2 "of" (.app0 "args") (.const "BIO")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert"))), false),
  ((.bin .mapsTo (.app0 "slots") (.app0 "inert")), false),
  ((.quant "o" (.app0 "created") (.app0 "o")), false),
  ((.bin .mapsTo .empty (.app0 "inert")), false),
  ((.bin .conj (.bin .mapsTo (.app0 "stated") (.app0 "inert")) (.app0 "inert")), false)
]

theorem decision_vectors_agree :
    decisionVectors.all (fun v => wf v.1 == v.2) = true := by decide

/-! ## The OpenSSL premises (`quirin-provider-openssl`'s `documented.rs`): seven names,
nine statements (the unstated-slots premise has a handshake and a free statement) -/

def exDataFormula : Raw := (.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "inert")))

def unstatedFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .mapsTo (.bin .diff (.app1 "slots" (.bin .union (.bin .union (.app2 "of" (.const "SSL") (.app0 "args")) (.app1 "ctx" (.app2 "of" (.const "SSL") (.app0 "args")))) (.app2 "of" (.const "SSL_CTX") (.app0 "args")))) (.app0 "stated")) (.app0 "inert")) (.quant "l" (.app1 "lookups" (.app1 "store" (.bin .union (.app1 "ctx" (.app2 "of" (.const "SSL") (.app0 "args"))) (.app2 "of" (.const "SSL_CTX") (.app0 "args"))))) (.bin .mapsTo (.bin .diff (.app1 "fns" (.app0 "l")) (.app0 "stated")) (.app0 "inert")))) (.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.bin .diff (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "stated")) (.app0 "inert")))) (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.bin .union (.app0 "stated") (.app0 "settable"))) (.app0 "inert"))))

def builtinFormula : Raw := (.quant "b" (.app1 "chain" (.bin .union (.app2 "of" (.const "BIO") (.app0 "args")) (.app1 "bios" (.app2 "of" (.const "SSL") (.app0 "args"))))) (.bin .imp (.bin .mem (.app1 "method" (.app0 "b")) (.app0 "builtin")) (.bin .conj (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert")) (.quant "f" (.app1 "fns" (.app0 "b")) (.bin .mapsTo (.app2 "in_turn" (.app0 "f") (.bin .diff (.app1 "ssl_of" (.app0 "b")) (.app0 "args"))) (.app0 "inert"))))))

def bioFreeFormula : Raw := (.quant "b" (.bin .union (.app2 "of" (.const "BIO") (.app0 "args")) (.app1 "chain" (.app1 "bios" (.app2 "of" (.const "SSL") (.app0 "args"))))) (.bin .imp (.bin .mem (.app0 "b") (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data_free" (.const "BIO")) (.app0 "inert"))))

def unstatedFreeFormula : Raw := (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.bin .union (.app0 "stated") (.app0 "settable"))) (.app0 "inert")))

def builtinLookupFormula : Raw := (.quant "l" (.app1 "lookups" (.app2 "of" (.const "X509_STORE") (.app0 "freed"))) (.bin .imp (.bin .mem (.app1 "method" (.app0 "l")) (.app0 "builtin")) (.bin .mapsTo (.app1 "fns" (.app0 "l")) (.app0 "inert"))))

/-- Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the library-value premise
of the frees, the one premise for the modules the library loads, and the
unstated-slots statement of a free through a released argument. -/
def builtinTeardownFormula : Raw := (.quant "o" (.app0 "freed") (.quant "f" (.app1 "teardown" (.app0 "o")) (.bin .imp (.bin .mem (.app0 "f") (.app0 "library")) (.bin .mapsTo (.app0 "f") (.app0 "inert")))))

def unstatedReleasedFreeFormula : Raw := (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.app0 "stated")) (.app0 "inert")))

def loadedTeardownFormula : Raw := (.quant "o" (.app0 "freed") (.quant "f" (.app1 "teardown" (.app0 "o")) (.bin .imp (.bin .mem (.app0 "f") (.app0 "loaded")) (.bin .conj (.bin .mapsTo (.app0 "f") (.app0 "inert")) (.quant "g" (.app2 "in_turn" (.app0 "f") (.bin .union (.bin .union (.app0 "freed") (.app0 "args")) (.app0 "created"))) (.bin .mem (.app0 "g") (.bin .union (.app0 "library") (.app0 "loaded"))))))))

theorem teardown_formulas_wf :
    wf builtinTeardownFormula = true ∧ wf loadedTeardownFormula = true ∧
      wf unstatedReleasedFreeFormula = true := by decide

theorem openssl_formulas_wf :
    wf exDataFormula = true ∧ wf unstatedFormula = true ∧ wf unstatedFreeFormula = true ∧
      wf builtinFormula = true ∧ wf builtinLookupFormula = true ∧
      wf bioFreeFormula = true := by decide

/-! ## The unstated-slots premise entails the ex_data premise -/

/-- No clause states an ex_data callback (C04.18: the registry is not a slot,
and no clause of the vocabulary states one), so `#stated` holds none. -/
def Model.Coherent (M : Model) : Prop := ∀ k f, M.exData k f → ¬ M.stated f

/-- What `openssl_ex_data_callbacks` states. -/
theorem eval_exData (M : Model) :
    (eval M [] exDataFormula).holds ↔
      ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) →
        ∀ f, M.exData (M.cls o) f → ∀ r, M.runs f r → inertEff M r := by
  simp [eval, exDataFormula, lk, prim0, prim1, binVal, Val.holds]

/-- What `openssl_unstated_callback_slots` states of the ex_data callbacks:
its third conjunct (card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS added a fourth, the
statement of a free). -/
theorem unstated_ex_data_part (M : Model) (h : (eval M [] unstatedFormula).holds) :
    ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) →
      ∀ f, (M.exData (M.cls o) f ∧ ¬ M.stated f) → ∀ r, M.runs f r → inertEff M r := by
  simp only [eval, unstatedFormula, lk, prim0, prim1, prim2, binVal, Val.holds] at h
  simpa using h.1.2

/-- What it states of the SSL object's and its SSL_CTX's slots: its first
conjunct. -/
theorem unstated_slots_part (M : Model) (h : (eval M [] unstatedFormula).holds) :
    ∀ f, (M.slots (fun u =>
            ((M.args u ∧ M.cls u = M.const "SSL") ∨
              M.ctx (fun o => M.args o ∧ M.cls o = M.const "SSL") u) ∨
            (M.args u ∧ M.cls u = M.const "SSL_CTX")) f ∧ ¬ M.stated f) →
        ∀ r, M.runs f r → inertEff M r := by
  simp only [eval, unstatedFormula, lk, prim0, prim1, prim2, binVal, Val.holds] at h
  simpa using h.1.1.1

theorem unstated_entails_ex_data (M : Model) (hC : M.Coherent)
    (h : (eval M [] unstatedFormula).holds) : (eval M [] exDataFormula).holds := by
  have hx := unstated_ex_data_part M h
  rw [eval_exData]
  intro o ho f hf r hr
  exact hx o ho f ⟨hf, hC _ _ hf⟩ r hr

/-- In particular for the SSL object's ex_data class. -/
theorem unstated_entails_ex_data_for_ssl (M : Model) (hC : M.Coherent)
    (h : (eval M [] unstatedFormula).holds) :
    ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) → M.cls o = M.const "SSL" →
      ∀ f, M.exData (M.const "SSL") f → ∀ r, M.runs f r → inertEff M r := by
  intro o ho hcls f hf r hr
  rw [← hcls] at hf
  exact (eval_exData M).1 (unstated_entails_ex_data M hC h) o ho f hf r hr

/-- The converse fails: the ex_data premise says nothing of the SSL object's
and its SSL_CTX's unstated slots. -/
theorem ex_data_does_not_entail_unstated :
    ¬ ∀ M : Model, M.Coherent → (eval M [] exDataFormula).holds →
      (eval M [] unstatedFormula).holds := by
  intro hall
  let M : Model := {
    U := Bool, Run := Unit,
    args := fun _ => True, created := fun _ => False, copied := fun _ => False,
    freed := fun _ => False, cls := fun _ => true, ctx := fun _ _ => False,
    bios := fun _ _ => False, chain := fun _ _ => False, sslOf := fun _ _ => False,
    slots := fun _ _ => True, stated := fun _ => False, exData := fun _ _ => False,
    exDataFree := fun _ _ => False, method := id, builtin := fun _ => False,
    fns := fun _ _ => False, store := fun _ _ => False, lookups := fun _ _ => False,
    teardown := fun _ _ => False, settable := fun _ => False,
    library := fun _ => False, loaded := fun _ => False,
    inTurn := fun _ _ _ => False, callerHeld := fun _ _ => True,
    takes := fun _ s => s, releases := fun _ _ _ => False,
    retains := fun _ _ _ => False, runs := fun _ _ => True, const := fun _ => true }
  have hC : M.Coherent := fun _ _ h => h.elim
  have hE : (eval M [] exDataFormula).holds := by
    rw [eval_exData]; intro o ho; simp [M] at ho
  have hU := unstated_slots_part M (hall M hC hE)
  have := hU true ⟨by simp [M], by simp [M]⟩ () trivial
  have h1 := congrFun this.1.1 true
  simp [M, noRes] at h1

end QCore.PremiseFormula
