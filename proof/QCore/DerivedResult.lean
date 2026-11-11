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
# Derived results: a non-owning pointer into a resource

Card **W43-LIBC-RESULT-ALIAS**, discharging `LIBC-MEM-RESULT-ALIAS` and
`W43-LIBC-RESULT-ALIAS-NAMES`. `memcpy` returns its first argument and `strchr`
returns a pointer into its first argument or NULL. The contract predicates
`resource_return_alias(p, n)` and `resource_return_interior(p, n)` state that
relation (contract spec §C04.3), and the gateway
(`crates/qcore-c-gateway/src/legacy_c.rs`) gives the result a **derived**
binding. This module models that binding and the rules the Rust applies to it.

## The model

A state has a status per resource site and a binding per name. A binding
targets at most one site and says how it relates to it: `identity` (the name is
the resource), `derived` (a pointer produced by a declared derived result, with
its extent and whether `const` was dropped), or `mixed` (different paths
disagree). The instructions are the ones the rules speak about.

## What is proved

* **Non-owning.** Deriving a pointer changes no resource status.
* **Releasing a derived pointer is not a release of its base.** No status
  changes; an interior pointer is **accused** (`invalidRelease`), an exact alias
  and a mixed binding are **refused**, never accused and never credited.
* **Releasing the base invalidates every pointer derived from it.** After the
  base's release, a use or a release of the derived pointer is a
  `useAfterRelease` violation candidate.
* **A derived null test narrows only itself**: no status and no other binding
  moves, so a null derived pointer cannot make its base's obligation disappear.
* **Never delivered**: a derived pointer stored through an out slot delivers
  nothing and is refused.
* **No `Safe` from any of it**: each new diagnostic is a safety issue, and a
  refusal keeps the result imprecise.
* **Conservativity**: on identity bindings every instruction behaves exactly as
  the pre-card transfer did, so a program that never derives is unaffected.
* **The join** is idempotent, a definitely-null path takes the
  other path's relation, and `mixed` absorbs every disagreement.
* **Non-vacuity**: each rule is reached by a concrete program.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. That the
Rust transfer implements these rules is the Rust–Lean correspondence premise of
trust boundary 28; that `memcpy` returns `dst` and `strchr` a pointer into `s`
are the trusted contract premises of the libc crate. A `useAfterRelease` or
`invalidRelease` is a *candidate*: the model does not know an interior
pointer's offset, and a candidate is never a validated upstream defect.
-/

namespace QCore.DerivedResult

/-- How much of the base object a derived result may designate. -/
inductive Extent where
  | exact
  | interior
  deriving DecidableEq, Repr

/-- How a binding relates to the site it targets. -/
inductive Derivation where
  | identity
  | derived (extent : Extent) (constDropped : Bool)
  | mixed
  deriving DecidableEq, Repr

inductive Status where
  | live
  | released
  deriving DecidableEq, Repr

/-- A binding: at most one target site, and its relation to it. `target = none`
is a null or untracked pointer. -/
structure Binding where
  target : Option Nat
  derivation : Derivation
  deriving DecidableEq, Repr

inductive Diagnostic where
  | doubleRelease (site : Nat)
  | invalidRelease (site : Nat)
  | useAfterRelease (site : Nat)
  | leak (site : Nat)
  | unknown
  deriving DecidableEq, Repr

structure State where
  status : Nat → Option Status
  binding : Nat → Option Binding
  diagnostics : List Diagnostic
  /-- Sites delivered to the caller through an out slot. -/
  delivered : List Nat

inductive Instruction where
  /-- `x = acquire()` at a fresh site. -/
  | acquire (x site : Nat)
  /-- `x = f(base, …)` for a declared derived result of `f`. -/
  | derive (x base : Nat) (extent : Extent) (constDropped : Bool)
  /-- `release(x)`. -/
  | release (x : Nat)
  /-- `x` passed at a borrowed position. -/
  | use (x : Nat)
  /-- The edge of `if (x == NULL)` on which `x` is null (`true`) or not. -/
  | nullTest (x : Nat) (isNull : Bool)
  /-- `*out = x` through a sanctioned out slot. -/
  | deliver (x : Nat)
  deriving DecidableEq, Repr

def update {β : Type} (f : Nat → β) (a : Nat) (b : β) : Nat → β :=
  fun x => if x = a then b else f x

def initial : State :=
  { status := fun _ => none, binding := fun _ => none, diagnostics := [], delivered := [] }

/-- The relation a derived result gives its destination, over its base's
(`derive_pointer`): interior unless both steps are exact, `const` dropped if
either step dropped it, `mixed` over a mixed base. -/
def deriveFrom (base : Derivation) (extent : Extent) (constDropped : Bool) : Derivation :=
  match base with
  | .identity => .derived extent constDropped
  | .derived e c =>
      .derived (if e = .exact ∧ extent = .exact then .exact else .interior)
        (c || constDropped)
  | .mixed => .mixed

/-- The pre-card transfer, on identity bindings. -/
def transferBase (s : State) : Instruction → State
  | .acquire x site =>
      { s with status := update s.status site (some .live),
               binding := update s.binding x (some ⟨some site, .identity⟩) }
  | .derive _ _ _ _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .release x =>
      match s.binding x with
      | some ⟨some site, _⟩ =>
          match s.status site with
          | some .live => { s with status := update s.status site (some .released) }
          | some .released => { s with diagnostics := .doubleRelease site :: s.diagnostics }
          | none => { s with diagnostics := .unknown :: s.diagnostics }
      | _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .use x =>
      match s.binding x with
      | some ⟨some site, _⟩ =>
          -- The pre-existing borrowed-liveness rule: a refusal, not a claim.
          if s.status site = some .released then
            { s with diagnostics := .unknown :: s.diagnostics }
          else s
      | _ => s
  | .nullTest x isNull =>
      match s.binding x with
      | some ⟨some site, d⟩ =>
          if isNull then
            -- The null alternative has no resource at this site.
            { s with status := update s.status site none,
                     binding := update s.binding x (some ⟨none, d⟩) }
          else s
      | _ => s
  | .deliver x =>
      match s.binding x with
      | some ⟨some site, _⟩ => { s with delivered := site :: s.delivered }
      | _ => s

/-- The transfer after this card. -/
def transfer (s : State) : Instruction → State
  | .derive x base extent constDropped =>
      let b := s.binding base
      let derived : Binding :=
        match b with
        | some ⟨target, d⟩ => ⟨target, deriveFrom d extent constDropped⟩
        | none => ⟨none, .derived extent constDropped⟩
      { s with binding := update s.binding x (some derived) }
  | .release x =>
      match s.binding x with
      | some ⟨some site, .derived .interior _⟩ =>
          if s.status site = some .released then
            { s with diagnostics := .useAfterRelease site :: s.diagnostics }
          else
            { s with diagnostics := .invalidRelease site :: s.diagnostics }
      | some ⟨some site, .derived .exact _⟩ =>
          if s.status site = some .released then
            { s with diagnostics := .useAfterRelease site :: .unknown :: s.diagnostics }
          else
            { s with diagnostics := .unknown :: s.diagnostics }
      | some ⟨_, .mixed⟩ => { s with diagnostics := .unknown :: s.diagnostics }
      | some ⟨none, .derived _ _⟩ => { s with diagnostics := .unknown :: s.diagnostics }
      | _ => transferBase s (.release x)
  | .use x =>
      match s.binding x with
      | some ⟨some site, .derived _ _⟩ =>
          if s.status site = some .released then
            { s with diagnostics := .useAfterRelease site :: s.diagnostics }
          else s
      | _ => transferBase s (.use x)
  | .nullTest x isNull =>
      match s.binding x with
      | some ⟨target, .derived e c⟩ =>
          if isNull then { s with binding := update s.binding x (some ⟨none, .derived e c⟩) }
          else { s with binding := update s.binding x (some ⟨target, .derived e c⟩) }
      | some ⟨_, .mixed⟩ => { s with diagnostics := .unknown :: s.diagnostics }
      | _ => transferBase s (.nullTest x isNull)
  | .deliver x =>
      match s.binding x with
      | some ⟨_, .derived _ _⟩ | some ⟨_, .mixed⟩ =>
          { s with diagnostics := .unknown :: s.diagnostics }
      | _ => transferBase s (.deliver x)
  | i => transferBase s i

def run (program : List Instruction) : State :=
  program.foldl transfer initial

/-- The exit: every live site is a leak. -/
def close (s : State) (sites : List Nat) : List Diagnostic :=
  sites.filterMap (fun site => if s.status site = some .live then some (.leak site) else none)
    ++ s.diagnostics

def isSafetyIssue : Diagnostic → Bool
  | .unknown => false
  | _ => true

inductive Verdict where
  | safe
  | defect
  | indeterminate
  deriving DecidableEq, Repr

def classify (diagnostics : List Diagnostic) : Verdict :=
  if diagnostics.any isSafetyIssue then .defect
  else if diagnostics.contains .unknown then .indeterminate
  else .safe

/-! ### Non-owning -/

/-- Deriving a pointer changes no resource status. -/
theorem deriving_changes_no_status (s : State) (x base : Nat) (e : Extent) (c : Bool) :
    (transfer s (.derive x base e c)).status = s.status := by
  simp only [transfer]

/-- Deriving a pointer adds no diagnostic and binds only its destination. -/
theorem deriving_binds_only_its_destination (s : State) (x base : Nat) (e : Extent)
    (c : Bool) (y : Nat) (hy : y ≠ x) :
    (transfer s (.derive x base e c)).binding y = s.binding y
      ∧ (transfer s (.derive x base e c)).diagnostics = s.diagnostics := by
  simp [transfer, update, hy]

/-- The derived binding targets exactly the base's target. -/
theorem a_derived_pointer_targets_its_base (s : State) (x base : Nat) (e : Extent)
    (c : Bool) (b : Binding) (hb : s.binding base = some b) :
    ((transfer s (.derive x base e c)).binding x).map (·.target) = some b.target := by
  cases b
  simp [transfer, update, hb]

/-! ### Releasing a derived pointer is not a release of its base -/

/-- No status changes when a derived or mixed pointer is released. -/
theorem releasing_a_derived_pointer_never_releases_its_base (s : State) (x : Nat)
    (b : Binding) (hb : s.binding x = some b) (hd : b.derivation ≠ .identity) :
    (transfer s (.release x)).status = s.status := by
  obtain ⟨target, d⟩ := b
  cases d with
  | identity => exact absurd rfl hd
  | derived e c =>
      cases target with
      | none => simp [transfer, hb]
      | some site =>
          cases e <;> simp only [transfer, hb] <;> split <;> rfl
  | mixed => cases target <;> simp [transfer, hb]

/-- An interior pointer handed to a release, its base still live, is accused. -/
theorem releasing_an_interior_pointer_is_a_candidate (s : State) (x site : Nat) (c : Bool)
    (hb : s.binding x = some ⟨some site, .derived .interior c⟩)
    (hl : s.status site = some .live) :
    (transfer s (.release x)).diagnostics = .invalidRelease site :: s.diagnostics := by
  simp [transfer, hb, hl]

/-- An exact alias is the base's own value, so its release is refused: neither
credited as the base's release (the status is unchanged, above) nor accused. -/
theorem releasing_a_live_exact_alias_is_refused (s : State) (x site : Nat) (c : Bool)
    (hb : s.binding x = some ⟨some site, .derived .exact c⟩)
    (hl : s.status site = some .live) :
    (transfer s (.release x)).diagnostics = .unknown :: s.diagnostics := by
  simp [transfer, hb, hl]

/-- A binding derived on only some paths is never accused. -/
theorem releasing_a_mixed_pointer_is_refused (s : State) (x : Nat) (t : Option Nat)
    (hb : s.binding x = some ⟨t, .mixed⟩) :
    (transfer s (.release x)).diagnostics = .unknown :: s.diagnostics := by
  cases t <;> simp [transfer, hb]

/-! ### Releasing the base invalidates every derived pointer -/

/-- Once the base's site is released, a use of a derived pointer is a candidate. -/
theorem a_use_after_the_base_is_released_is_a_candidate (s : State) (x site : Nat)
    (e : Extent) (c : Bool) (hb : s.binding x = some ⟨some site, .derived e c⟩)
    (hr : s.status site = some .released) :
    (transfer s (.use x)).diagnostics = .useAfterRelease site :: s.diagnostics := by
  simp [transfer, hb, hr]

/-- And so is a release of it, of either extent. -/
theorem a_release_after_the_base_is_released_is_a_candidate (s : State) (x site : Nat)
    (e : Extent) (c : Bool) (hb : s.binding x = some ⟨some site, .derived e c⟩)
    (hr : s.status site = some .released) :
    .useAfterRelease site ∈ (transfer s (.release x)).diagnostics := by
  cases e <;> simp [transfer, hb, hr]

/-- Releasing the base marks exactly its site released, which every pointer
derived from it targets: the composition of the two rules above. -/
theorem releasing_the_base_invalidates_every_derived_pointer (s : State)
    (b x site : Nat) (e : Extent) (c : Bool)
    (hb : s.binding b = some ⟨some site, .identity⟩)
    (hx : s.binding x = some ⟨some site, .derived e c⟩)
    (hl : s.status site = some .live) :
    (transfer (transfer s (.release b)) (.use x)).diagnostics
      = .useAfterRelease site :: s.diagnostics := by
  have hx' : s.binding x = some ⟨some site, .derived e c⟩ := hx
  simp [transfer, transferBase, hb, hl, hx', update]

/-! ### A derived null test narrows only itself -/

/-- No status moves on either edge of a derived pointer's null test. -/
theorem a_derived_null_test_keeps_every_status (s : State) (x : Nat) (t : Option Nat)
    (e : Extent) (c : Bool) (hb : s.binding x = some ⟨t, .derived e c⟩) (isNull : Bool) :
    (transfer s (.nullTest x isNull)).status = s.status := by
  cases isNull <;> simp [transfer, hb]

/-- Nor does any other binding. -/
theorem a_derived_null_test_keeps_every_other_binding (s : State) (x y : Nat)
    (t : Option Nat) (e : Extent) (c : Bool) (hb : s.binding x = some ⟨t, .derived e c⟩)
    (isNull : Bool) (hy : y ≠ x) :
    (transfer s (.nullTest x isNull)).binding y = s.binding y := by
  cases isNull <;> simp [transfer, hb, update, hy]

/-- Whereas an identity's null edge removes its site: the rule the derived
binding must not inherit. -/
theorem an_identity_null_test_removes_its_site (s : State) (x site : Nat)
    (hb : s.binding x = some ⟨some site, .identity⟩) :
    (transfer s (.nullTest x true)).status site = none := by
  simp [transfer, transferBase, hb, update]

/-! ### A derived pointer is never delivered as custody -/

/-- A derived or mixed pointer stored through an out slot delivers nothing and
is refused: a summary could otherwise publish a pointer into an object as fresh
custody the caller must release. -/
theorem a_derived_pointer_is_never_delivered (s : State) (x : Nat) (b : Binding)
    (hb : s.binding x = some b) (hd : b.derivation ≠ .identity) :
    (transfer s (.deliver x)).delivered = s.delivered
      ∧ (transfer s (.deliver x)).diagnostics = .unknown :: s.diagnostics := by
  obtain ⟨t, d⟩ := b
  cases d with
  | identity => exact absurd rfl hd
  | derived e c => simp [transfer, hb]
  | mixed => simp [transfer, hb]

/-- Whereas an identity is delivered: the rule is not vacuous. -/
theorem an_identity_is_delivered (s : State) (x site : Nat)
    (hb : s.binding x = some ⟨some site, .identity⟩) :
    (transfer s (.deliver x)).delivered = site :: s.delivered := by
  simp [transfer, transferBase, hb]

/-! ### No `Safe` from any of it -/

theorem the_derived_candidates_are_safety_issues (site : Nat) :
    isSafetyIssue (.invalidRelease site) = true
      ∧ isSafetyIssue (.useAfterRelease site) = true := by
  simp [isSafetyIssue]

/-- A diagnostic list carrying either candidate is never `Safe`. -/
theorem a_candidate_is_never_safe (ds : List Diagnostic) (site : Nat)
    (h : .invalidRelease site ∈ ds ∨ .useAfterRelease site ∈ ds) :
    classify ds ≠ .safe := by
  unfold classify
  have hany : ds.any isSafetyIssue = true := by
    rw [List.any_eq_true]
    rcases h with h | h
    · exact ⟨_, h, rfl⟩
    · exact ⟨_, h, rfl⟩
  simp [hany]

/-- A refusal alone is never `Safe` either. -/
theorem a_refusal_is_never_safe (ds : List Diagnostic) (h : .unknown ∈ ds) :
    classify ds ≠ .safe := by
  unfold classify
  split
  · simp
  · simp [h]

/-! ### Conservativity -/

/-- On an identity binding every instruction but `derive` behaves exactly as the
pre-card transfer did. -/
theorem identity_bindings_are_unaffected (s : State) (i : Instruction)
    (hi : ∀ x base e c, i ≠ .derive x base e c)
    (hid : ∀ x b, s.binding x = some b → b.derivation = .identity) :
    transfer s i = transferBase s i := by
  cases i with
  | acquire x site => rfl
  | derive x base e c => exact absurd rfl (hi x base e c)
  | release x =>
      cases hb : s.binding x with
      | none => simp [transfer, hb]
      | some b =>
          have := hid x b hb
          obtain ⟨t, d⟩ := b
          simp only at this
          subst this
          cases t <;> simp [transfer, hb]
  | use x =>
      cases hb : s.binding x with
      | none => simp [transfer, hb]
      | some b =>
          have := hid x b hb
          obtain ⟨t, d⟩ := b
          simp only at this
          subst this
          cases t <;> simp [transfer, hb]
  | nullTest x isNull =>
      cases hb : s.binding x with
      | none => simp [transfer, hb]
      | some b =>
          have := hid x b hb
          obtain ⟨t, d⟩ := b
          simp only at this
          subst this
          simp [transfer, hb]
  | deliver x =>
      cases hb : s.binding x with
      | none => simp [transfer, hb]
      | some b =>
          have := hid x b hb
          obtain ⟨t, d⟩ := b
          simp only at this
          subst this
          simp [transfer, hb]

/-! ### The join -/

/-- `Derivation::join`, over two paths' bindings. -/
def join (one other : Binding) : Derivation :=
  if other.target = none ∧ other.derivation = .identity then one.derivation
  else if one.target = none ∧ one.derivation = .identity then other.derivation
  else match one.derivation, other.derivation with
    | .derived e c, .derived e' c' =>
        if e = e' then .derived e (c || c') else .mixed
    | d, d' => if d = d' then d else .mixed

theorem join_of_equal_is_itself (b : Binding) : join b b = b.derivation := by
  obtain ⟨t, d⟩ := b
  unfold join
  cases d <;> simp

/-- An identity joined with a derived pointer, both targeting a site, is mixed. -/
theorem identity_and_derived_join_to_mixed (site : Nat) (e : Extent) (c : Bool) :
    join ⟨some site, .identity⟩ ⟨some site, .derived e c⟩ = .mixed := by
  simp [join]

/-- A definitely-null path carries no relation, so the other path's is kept. -/
theorem a_null_path_keeps_the_other_relation (b : Binding) :
    join b ⟨none, .identity⟩ = b.derivation := by
  simp [join]

/-- `mixed` absorbs a derived binding on the other side. -/
theorem mixed_absorbs (t t' : Option Nat) (e : Extent) (c : Bool)
    (ht : t' ≠ none) :
    join ⟨t, .mixed⟩ ⟨t', .derived e c⟩ = .mixed := by
  simp [join, ht]

/-! ### `const` dropped travels with the pointer -/

/-- A pointer derived from a `const`-dropped pointer is `const`-dropped. -/
theorem const_dropped_survives_derivation (e e' : Extent) (c : Bool) :
    ∃ e'', deriveFrom (.derived e true) e' c = .derived e'' true := by
  cases e <;> cases e' <;> simp [deriveFrom]

/-- Exactness survives only an exact step over an exact pointer. -/
theorem an_interior_step_is_interior (d : Extent) (c c' : Bool) :
    deriveFrom (.derived d c) .interior c' = .derived .interior (c || c') := by
  cases d <;> simp [deriveFrom]

/-! ### Witnesses -/

/-- `b = alloc(); p = strchr(b, c); free(b); strlen(p);` -/
def useAfterBaseRelease : List Instruction :=
  [.acquire 0 0, .derive 1 0 .interior true, .release 0, .use 1]

theorem the_use_after_release_witness :
    (run useAfterBaseRelease).diagnostics = [.useAfterRelease 0] := by
  decide

/-- `b = alloc(); p = strchr(b, c); free(p); free(b);` — one candidate, and the
base is released exactly once: no double release. -/
def freeInteriorThenBase : List Instruction :=
  [.acquire 0 0, .derive 1 0 .interior true, .release 1, .release 0]

theorem the_invalid_release_witness :
    (run freeInteriorThenBase).diagnostics = [.invalidRelease 0]
      ∧ (run freeInteriorThenBase).status 0 = some .released := by
  decide

/-- `b = alloc(); p = strchr(b, c); if (!p) return;` — the base still leaks on
the null edge. -/
def nullDerivedLeaks : List Instruction :=
  [.acquire 0 0, .derive 1 0 .interior true, .nullTest 1 true]

theorem the_null_derived_pointer_leaves_its_base_live :
    close (run nullDerivedLeaks) [0] = [.leak 0] := by
  decide

/-- Were the null test read as the base's, the leak would vanish: the witness
that the narrowing is load-bearing. -/
theorem an_identity_null_test_would_have_hidden_it :
    close (run [.acquire 0 0, .nullTest 0 true]) [0] = [] := by
  decide

/-- `q = memcpy(b, …); free(q);` — refused, and the base neither released nor
accused. -/
def freeExactAlias : List Instruction :=
  [.acquire 0 0, .derive 1 0 .exact false, .release 1]

theorem the_exact_alias_witness :
    (run freeExactAlias).diagnostics = [.unknown]
      ∧ (run freeExactAlias).status 0 = some .live
      ∧ classify (run freeExactAlias).diagnostics = .indeterminate := by
  decide

/-- Scanning a string the body does not own certifies. -/
theorem scanning_an_untracked_string_is_safe :
    classify (close (run [.derive 1 0 .interior true, .nullTest 1 false, .use 1]) []) = .safe := by
  decide

end QCore.DerivedResult
