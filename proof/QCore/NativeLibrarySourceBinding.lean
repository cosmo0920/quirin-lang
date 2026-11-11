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

import QCore.NativeLibrary

/-! Card `NATIVE-LIBRARY-SOURCE-BINDING`: the **source-file-scoped** selection of
a manifest-bound library, `extern library NAME;`.

Cards `UNSAFE-LINKAGE-ARTIFACT` and `NATIVE-LIBRARY-CONTRACT-SLICE` built the
manifest and made a symbol resolve through it. What neither built is a statement
*in the source file* about which build its extern surface is resolved against:
resolution was by symbol name across the whole selection, so whichever selected
manifest happened to name a symbol supplied its premise, and no file said so.

This module models what saying so means. The vocabulary is **imported** from
`QCore.NativeLibrary` — the identity, the ABI premises and the header identity —
so there is one vocabulary and not two that could drift.

What is established here:

* **the binding is a function** — an unknown name, an ambiguous name and a
  repeated name each refuse, so a resolved binding names exactly one build;
* **an unbound file is unchanged** — the empty binding list yields the
  selection-wide view, which is the pre-card resolution exactly. This is the
  opt-in property every existing program rests on;
* **a binding only ever narrows** — every symbol a bound view resolves, the
  unbound view resolved too. A binding can remove a resolution and can never add
  one, which is hard invariant 1's direction;
* **a narrowed symbol refuses, it is not silently downgraded** — the converse of
  the above, and the load-bearing theorem: exactly where a binding removes a
  resolution the file is owed a refusal. Falling through to the weaker premise
  would let a file stop its certificate naming a build by binding a different
  library;
* **absence of evidence is not evidence** — a symbol *no* manifest binds is not
  foreign to anyone, and stays exactly as unresident as it was;
* **bindings do not leak across compilation inputs** — each file's view is a
  function of that file's own list and the selection, and of nothing else;
* **the premise names the build** — a resolved binding carries the identity of
  the manifest that declared the name, so the published evidence says
  `library@hash`.

What is **not** established, here or anywhere: that the named artifact is the one
the linker used, or that the library conforms to anything. Both stay premises,
and this card resolves a *name to a declaration* — it opens no file and hashes
nothing. -/

namespace QCore.NativeLibrarySourceBinding

open QCore.NativeLibrary (Identity Premises HeaderIdentity)

/-! ## The selection, as this card reads it -/

/-- One selected manifest, projected to what a source binding reads: the
selection key it was selected under, the library name it declares, the identity
and ABI premises it states, the header it binds, and the symbols it binds.

The entry kinds are deliberately *not* modelled: whether a symbol is waived or
contracted is `QCore.NativeLibrary` and `QCore.NativeLibraryContract`'s business
and is unchanged by this card. What this card decides is **which manifests are
visible to a file at all**, which is prior to, and independent of, how a visible
one reads a symbol. -/
structure Manifest where
  key : String
  library : String
  identity : Identity
  premises : Premises
  header : Option HeaderIdentity
  symbols : List String
  deriving DecidableEq, Repr

/-- An explicit, immutable selection of manifests. -/
abbrev Selection := List Manifest

/-- The selection key of the manifest that binds `symbol`, or `none`.

The first match, which is the whole answer because the selection loader already
refuses two manifests binding one symbol (`Q040`): in a loaded selection there is
at most one. -/
def symbolModule (sel : Selection) (symbol : String) : Option String :=
  (sel.find? fun m => m.symbols.contains symbol).map Manifest.key

/-- The manifests declaring a given library name. Two is an ambiguity, not a
choice. -/
def declaring (sel : Selection) (name : String) : List Manifest :=
  sel.filter fun m => m.library == name

/-! ## Resolution -/

/-- One resolved `extern library NAME;`: what the file spelled, and which build
that turned out to be. Publishing both is the point — a later selection
resolving the same name to a different build is then a visible difference. -/
structure Resolved where
  named : String
  manifest : String
  identity : Identity
  premises : Premises
  header : Option HeaderIdentity
  deriving DecidableEq, Repr

/-- Why a file's binding list does not resolve. Every arm refuses; there is no
arm that resolves partially and none that falls back. -/
inductive Refusal where
  /-- The file binds one library name twice. -/
  | repeated (named : String)
  /-- No selected manifest declares that name. -/
  | unknown (named : String)
  /-- Two or more selected manifests declare it, so the binding would not be a
  function — the ground `Q040` refuses its own ambiguity on. -/
  | ambiguous (named : String)
  /-- A symbol this file declares is bound by a manifest this file did **not**
  bind. Reported at the declaration, not resolved to the weaker premise. -/
  | foreign (symbol : String) (manifest : String)
  deriving DecidableEq, Repr

/-- One file's view of the selection.

`modules = none` is the **unbound** file: it made no statement, and resolution is
selection-wide, which is exactly the pre-card behaviour. `modules = some ms` is a
file that bound `ms`. The two are different states rather than an empty list
standing for both, because "said nothing" and "said something that resolved to
nothing" are different statements — and the second cannot arise, since a file
that declared a binding at all resolved at least one. -/
structure View where
  modules : Option (List String)
  bindings : List Resolved
  deriving DecidableEq, Repr

/-- The unbound view. -/
def unbound : View := ⟨none, []⟩

/-- Resolve one file's binding list against the selection, total or reject.

A function of its two inputs and of nothing else, which is what makes it
rederivable at a certificate recheck — the same property the manifest load and
the slice totality check already have. -/
def resolve (sel : Selection) : List String → Except Refusal View
  | [] => .ok unbound
  | names =>
    let rec go (rest : List String) (seen : List String) (acc : List Resolved) :
        Except Refusal (List Resolved) :=
      match rest with
      | [] => .ok acc
      | name :: tail =>
        if seen.contains name then .error (.repeated name)
        else
          match declaring sel name with
          | [] => .error (.unknown name)
          | [m] =>
            go tail (name :: seen)
              (acc ++ [⟨name, m.key, m.identity, m.premises, m.header⟩])
          | _ => .error (.ambiguous name)
    match go names [] [] with
    | .error r => .error r
    | .ok bindings => .ok ⟨some (bindings.map Resolved.manifest), bindings⟩

/-- Whether a manifest key is one this view resolves against. -/
def View.visible (v : View) (key : String) : Bool :=
  match v.modules with
  | none => true
  | some ms => ms.contains key

/-- Whether this view resolves `symbol` to a manifest at all. -/
def View.resolves (v : View) (sel : Selection) (symbol : String) : Bool :=
  match symbolModule sel symbol with
  | none => false
  | some key => v.visible key

/-- Whether this view owes a **foreign-symbol refusal** for `symbol`: some
selected manifest binds it and this file did not bind that manifest.

An unbound view is never owed one — it made no statement to contradict — which
is what keeps every pre-card program unmoved. -/
def View.foreign (v : View) (sel : Selection) (symbol : String) : Bool :=
  match v.modules with
  | none => false
  | some ms =>
    match symbolModule sel symbol with
    | none => false
    | some key => !ms.contains key

/-! ## The binding is a function -/

/-- A name no selected manifest declares refuses. Nothing resolves implicitly. -/
theorem an_unknown_name_refuses (sel : Selection) (name : String)
    (absent : declaring sel name = []) :
    resolve sel [name] = .error (.unknown name) := by
  simp [resolve, resolve.go, absent]

/-- Two manifests declaring one library name refuse. Choosing one by selection
order would make the published premise depend on the order the builder listed its
manifests, which is the ground `Q040` refuses a symbol bound twice on. -/
theorem an_ambiguous_name_refuses (sel : Selection) (name : String) (a b : Manifest)
    (rest : List Manifest) (two : declaring sel name = a :: b :: rest) :
    resolve sel [name] = .error (.ambiguous name) := by
  simp [resolve, resolve.go, two]

/-- A repeated binding refuses. A second binding of one name adds nothing and
means nothing; taking the first would make a repeated line silently ignorable. -/
theorem a_repeated_name_refuses (sel : Selection) (name : String) (m : Manifest)
    (one : declaring sel name = [m]) :
    resolve sel [name, name] = .error (.repeated name) := by
  simp [resolve, resolve.go, one]

/-! ## An unbound file is unchanged -/

/-- **The opt-in property.** A file that declares no binding gets the unbound
view, which resolves selection-wide: this is the pre-card resolution exactly, and
it is what every program written before this card rests on. -/
theorem the_empty_binding_is_unbound (sel : Selection) :
    resolve sel [] = .ok unbound := rfl

theorem an_unbound_view_resolves_selection_wide (sel : Selection) (symbol : String) :
    unbound.resolves sel symbol = (symbolModule sel symbol).isSome := by
  cases h : symbolModule sel symbol <;> simp [View.resolves, View.visible, unbound, h]

/-- An unbound file is never owed a foreign-symbol refusal. It made no statement
to contradict. -/
theorem an_unbound_view_is_never_foreign (sel : Selection) (symbol : String) :
    unbound.foreign sel symbol = false := by
  simp [View.foreign, unbound]

/-! ## A binding only ever narrows, and a narrowing refuses -/

/-- **Hard invariant 1's direction.** Every symbol a bound view resolves, the
unbound view resolved too: a binding can remove a resolution and can never add
one. -/
theorem a_binding_only_narrows (sel : Selection) (v : View) (symbol : String)
    (resolved : v.resolves sel symbol = true) :
    unbound.resolves sel symbol = true := by
  unfold View.resolves at resolved ⊢
  cases h : symbolModule sel symbol with
  | none => simp [h] at resolved
  | some key => simp [View.visible, unbound]

/-- **The load-bearing theorem: a narrowed symbol refuses, it is never silently
downgraded.** Exactly where a binding removes a resolution the file is owed a
foreign-symbol refusal. Without this the card would be a *weakening*: a file
could stop its certificate naming a build simply by binding a different library,
which is `Q041`'s shape — a contradiction rewarded with a premise that says
less. -/
theorem a_narrowed_symbol_is_refused (sel : Selection) (v : View) (symbol : String)
    (was : unbound.resolves sel symbol = true) (now : v.resolves sel symbol = false) :
    v.foreign sel symbol = true := by
  cases h : symbolModule sel symbol with
  | none => simp [View.resolves, h] at was
  | some key =>
    cases hm : v.modules with
    | none => simp [View.resolves, View.visible, h, hm] at now
    | some ms =>
      simp [View.resolves, View.visible, h, hm] at now
      simp [View.foreign, h, hm, now]

/-- **Absence of evidence is not evidence.** A symbol no selected manifest binds
is foreign to nobody: it stays exactly as unresident as it was, and only a symbol
an *unbound* manifest claims is a contradiction. -/
theorem a_symbol_no_manifest_binds_is_not_foreign (sel : Selection) (v : View)
    (symbol : String) (unclaimed : symbolModule sel symbol = none) :
    v.foreign sel symbol = false := by
  cases hm : v.modules <;> simp [View.foreign, hm, unclaimed]

/-- The refusal and the resolution are exclusive: a view never both resolves a
symbol and owes a refusal for it. -/
theorem resolving_and_refusing_are_exclusive (sel : Selection) (v : View) (symbol : String)
    (resolved : v.resolves sel symbol = true) :
    v.foreign sel symbol = false := by
  unfold View.resolves at resolved
  unfold View.foreign
  cases hm : v.modules with
  | none => simp
  | some ms =>
    cases h : symbolModule sel symbol with
    | none => simp
    | some key =>
      rw [h] at resolved
      simp [View.visible, hm] at resolved
      simp [resolved]

/-! ## Bindings do not leak across compilation inputs -/

/-- Every input file's view, in file order. -/
def resolveFiles (sel : Selection) (files : List (List String)) :
    List (Except Refusal View) :=
  files.map (resolve sel)

/-- **Constraint 13's discipline, applied to a binding.** Each file's view is
`resolve` of that file's own list, so one file's binding is not a fact about
another's declarations — a binding in another compilation input grants a file
nothing and takes nothing away. -/
theorem a_file_s_view_is_its_own (sel : Selection) (files : List (List String)) (i : Nat) :
    (resolveFiles sel files)[i]? = files[i]?.map (resolve sel) := by
  simp [resolveFiles]

/-- A bound file and an unbound sibling coexist: the sibling keeps the
selection-wide resolution it had. -/
theorem an_unbound_sibling_is_unmoved (sel : Selection) (names : List String) :
    (resolveFiles sel [names, []])[1]? = some (.ok unbound) := by
  simp [resolveFiles, the_empty_binding_is_unbound]

/-! ## The premise names the build -/

/-- A resolved binding carries the identity of the manifest that declared the
name it was spelled with, so the published premise reads *this name, from this
build*. Stated on the one-binding case, which is where the resolution's content
is decided. -/
theorem the_premise_names_the_build (sel : Selection) (name : String) (m : Manifest)
    (one : declaring sel name = [m]) :
    resolve sel [name] =
      .ok ⟨some [m.key], [⟨name, m.key, m.identity, m.premises, m.header⟩]⟩ := by
  simp [resolve, resolve.go, one]

/-- The identity a binding publishes is the *declared* one, not a probed one:
this card resolves a name to a declaration and reads no file. Stated as the fact
that the resolved identity is definitionally the manifest's own. -/
theorem the_identity_is_the_declared_one (sel : Selection) (name : String) (m : Manifest)
    (one : declaring sel name = [m]) :
    ∃ v, resolve sel [name] = .ok v ∧
      v.bindings.map Resolved.identity = [m.identity] := by
  exact ⟨_, the_premise_names_the_build sel name m one, by simp⟩

/-! ## Non-vacuity -/

/-- The route admits something reachable: a file that binds the library its
manifest declares resolves that manifest's symbols and no other's, and the other
manifest's symbol is refused rather than silently unresident. -/
theorem the_route_binds_narrows_and_refuses (identity other : Identity) (ps : Premises) :
    let a : Manifest := ⟨"a.qcorelib", "libA", identity, ps, none, ["k_a"]⟩
    let b : Manifest := ⟨"b.qcorelib", "libB", other, ps, none, ["k_b"]⟩
    let sel : Selection := [a, b]
    ∃ v, resolve sel ["libA"] = .ok v ∧
      v.resolves sel "k_a" = true ∧
      v.resolves sel "k_b" = false ∧
      v.foreign sel "k_b" = true ∧
      v.foreign sel "k_unclaimed" = false := by
  refine ⟨_, rfl, ?_, ?_, ?_, ?_⟩ <;>
    simp [View.resolves, View.foreign, View.visible, symbolModule]

end QCore.NativeLibrarySourceBinding
