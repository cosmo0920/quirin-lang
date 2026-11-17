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

import QCore.ResourceKinds

/-!
# Thread-specific data keys in place (cards CURRENT-CONTEXT-NEW-TRIAGE, D3, and CG25-THREAD-KEY-AUTOMATIC-REVISIT)

`crates/qcore-c-gateway/src/legacy_in_place_custody.rs` (`in_place_actuals`,
`automatic_key`, `every_reference_is`, `report_uncertified_key_uses`,
`STATIC_KEY_RELEASE_REFUSAL`, `THREAD_KEY_STORAGE_REFUSAL`),
`legacy_classify.rs` (the static-held positions read as borrowed), and the
libc declarations of `pthread_key_create` / `pthread_key_delete` /
`pthread_getspecific` (in-place kind `thread_key`; the getter's key is a
by-value use).

Gateway rule 11v (5) and (7) extend CG25's in-place reading
(`QCore.ResourceKinds`) to keys:

* A key created into `&G`, `G` of static storage duration, is held by the
  static object under the stated premise **P-THREAD-KEY-STATIC**: a key
  created into static storage with no destructor is created once and is not
  owed a delete. The creation binds **no frame obligation**, so it owes
  nothing at any exit and needs no status witness.
* A by-value finaliser of a static key is **refused**: the holder's lifetime
  is outside the frame.
* A key created into `&k`, `k` the one automatic key object of the function
  (one `VarDecl`, and every reference to the name designates it), is
  **CG25's initialiser** (rule 11v (7)). Its status witnesses it and
  `pthread_key_delete(k)` finalises it. A missing delete is a leak candidate,
  and a second delete is a double-release candidate.
* A creation into any other storage (a member, a formal, through a pointer,
  a name that also designates a file-scope object) is **refused**.
* A by-value **use** (`pthread_getspecific(k)`) is admitted only where the
  key is **certified**: bound to a live site whose status test decided
  success. A use of a released site is a `use_after_close` candidate. Every
  other use is refused: before the creation, before the status test, or on
  the failure edge. CG25's own `use` checks only the released bit, which
  would admit a use of a key value POSIX leaves undefined.

## What is proved

* A static creation leaves the in-place state unchanged, so it owes nothing
  at any exit (`a_static_creation_owes_nothing`), whatever runs after it.
* A static finaliser is refused (`a_static_finaliser_is_refused`) and so is
  never `Safe` (`a_static_finaliser_is_never_safe`).
* An automatic creation is CG25's initialiser (`an_automatic_creation_is_cg25`).
  Any other creation is refused and binds nothing
  (`another_creation_is_refused`, `another_creation_tracks_nothing`).
* A certified use changes nothing (`a_certified_use_changes_nothing`). A use
  after the delete is a candidate (`a_use_after_delete_is_a_candidate`).
  Every uncertified use is refused (`an_uncertified_use_is_refused`). No use
  changes custody (`a_use_changes_no_custody`).
* Every step only adds diagnostics (`diagnostics_only_grow`), so **a `Safe`
  verdict used only certified keys** (`a_safe_verdict_used_only_certified_keys`).
* Witness programs. Create, test, use, delete is `Safe`. None of these is
  `Safe`: a use after the delete, a missing delete, a double delete, a use
  before the test, a use on the failure edge, a use before the creation
  (`the_create_use_delete_program_is_safe` and the theorems after it).
* `mk_http_thread_initialize_tls_once`'s shape, two static creations and
  nothing else, owes nothing and carries no diagnostic
  (`the_two_static_creations_owe_nothing`).

## What this is not

* Not a proof that a static key is created once: that is
  P-THREAD-KEY-STATIC, stated, not proved.
* Not a proof that `pthread_key_create` returns zero exactly when it
  created the key: that is the libc premise CG25 states for every in-place
  initialiser.
* Not a proof of what the token stream and Clang say (trust boundary 28):
  which actuals are static, and which name is the one automatic object.
* Not a proof that every mention of a tracked key is custody: that is rule
  11v (6)'s guard (`QCore.InPlaceMentionGuard`).

The two read-off facts are pinned by
`crates/qcore-c-gateway/tests/thread_key_in_place.rs`.
-/

namespace QCore.ThreadKeyStatic

open QCore.KindedCustody
open QCore.ResourceKinds

/-- One step of the key reading. -/
inductive KInstr where
  /-- `pthread_key_create(&G, NULL)`, `G` of static storage duration. -/
  | staticCreate (g : Nat)
  /-- `pthread_key_delete(G)`, `G` a static key. -/
  | staticFinalise (g : Nat)
  /-- `st = pthread_key_create(&k, NULL)`, `k` the one automatic key object
  of the function, its status held by the plain name `st`, creating `site`. -/
  | automaticCreate (k st site : Nat)
  /-- `pthread_key_create(&k, NULL)` into any other storage. -/
  | otherCreate (k : Nat)
  /-- `pthread_getspecific(k)`, `k` a tracked key: the by-value use. -/
  | keyUse (k : Nat)
  /-- Every other step: CG25's in-place reading (`pthread_key_delete(k)` is
  `.finalise k .threadKey`). -/
  | inPlace (i : IInstr)
  deriving DecidableEq, Repr

/-- Whether key `k` is certified: bound to a live site whose status test
decided success. -/
def certified (s : IState) (k : Nat) : Bool :=
  match s.storage k with
  | some site => decide (s.status site = some .live) && !(s.mayFail site)
  | none => false

def refuse (s : IState) : IState := { s with diagnostics := .unknown :: s.diagnostics }

/-- Rule 11v (7): the by-value use. -/
def useK (s : IState) (k : Nat) : IState :=
  match s.storage k with
  | some site =>
      if s.status site = some .released then
        { s with diagnostics := .useAfterClose site :: .unknown :: s.diagnostics }
      else if certified s k then s
      else refuse s
  | none => refuse s

def transferK (s : IState) : KInstr → IState
  | .staticCreate _ => s
  | .staticFinalise _ => refuse s
  | .automaticCreate k st site => transferI s (.init k st site)
  | .otherCreate _ => refuse s
  | .keyUse k => useK s k
  | .inPlace i => transferI s i

def runK (s : IState) (program : List KInstr) : IState :=
  program.foldl transferK s

theorem in_place_is_cg25 (s : IState) (i : IInstr) : transferK s (.inPlace i) = transferI s i :=
  rfl

/-! ### Static keys (rule 11v (5)) -/

/-- A static creation changes nothing a later step or an exit reads. -/
theorem a_static_creation_owes_nothing (s : IState) (g : Nat) (sites : List Nat) :
    exitI (transferK s (.staticCreate g)) sites = exitI s sites := rfl

theorem a_static_creation_binds_no_storage (s : IState) (g : Nat) :
    (transferK s (.staticCreate g)).storage = s.storage := rfl

theorem a_static_finaliser_is_refused (s : IState) (g : Nat) :
    .unknown ∈ (transferK s (.staticFinalise g)).diagnostics := by
  simp [transferK, refuse]

theorem a_static_finaliser_is_never_safe (s : IState) (g : Nat) :
    classify (transferK s (.staticFinalise g)).diagnostics ≠ .safe :=
  a_refusal_is_never_safe _ (a_static_finaliser_is_refused s g)

/-! ### Automatic keys (rule 11v (7)) -/

theorem an_automatic_creation_is_cg25 (s : IState) (k st site : Nat) :
    transferK s (.automaticCreate k st site) = transferI s (.init k st site) := rfl

theorem another_creation_is_refused (s : IState) (k : Nat) :
    .unknown ∈ (transferK s (.otherCreate k)).diagnostics
      ∧ classify (transferK s (.otherCreate k)).diagnostics ≠ .safe := by
  have h : .unknown ∈ (transferK s (.otherCreate k)).diagnostics := by
    simp [transferK, refuse]
  exact ⟨h, a_refusal_is_never_safe _ h⟩

theorem another_creation_tracks_nothing (s : IState) (k : Nat) :
    (transferK s (.otherCreate k)).storage = s.storage
      ∧ (transferK s (.otherCreate k)).status = s.status := ⟨rfl, rfl⟩

/-- A certified use changes nothing. -/
theorem a_certified_use_changes_nothing (s : IState) (k : Nat) (h : certified s k = true) :
    transferK s (.keyUse k) = s := by
  have h' := h
  unfold certified at h'
  cases hb : s.storage k with
  | none => simp [hb] at h'
  | some site =>
      simp only [hb, Bool.and_eq_true, decide_eq_true_eq] at h'
      have hr : s.status site ≠ some .released := by
        rw [h'.1]; decide
      simp [transferK, useK, hb, hr, h]

/-- A use of a released key is a `use_after_close` candidate. -/
theorem a_use_after_delete_is_a_candidate (s : IState) (k site : Nat)
    (hb : s.storage k = some site) (hr : s.status site = some .released) :
    .useAfterClose site ∈ (transferK s (.keyUse k)).diagnostics := by
  simp [transferK, useK, hb, hr]

/-- Every use that is not certified is refused (a released key is refused
too, beside its candidate). -/
theorem an_uncertified_use_is_refused (s : IState) (k : Nat) (h : certified s k = false) :
    .unknown ∈ (transferK s (.keyUse k)).diagnostics := by
  cases hb : s.storage k with
  | none => simp [transferK, useK, hb, refuse]
  | some site =>
      by_cases hr : s.status site = some .released
      · simp [transferK, useK, hb, hr]
      · simp [transferK, useK, hb, hr, h, refuse]

theorem an_uncertified_use_is_never_safe (s : IState) (k : Nat) (h : certified s k = false) :
    classify (transferK s (.keyUse k)).diagnostics ≠ .safe :=
  a_refusal_is_never_safe _ (an_uncertified_use_is_refused s k h)

/-- A use moves no custody: storage, status and failure marks are
untouched. -/
theorem a_use_changes_no_custody (s : IState) (k : Nat) :
    (transferK s (.keyUse k)).storage = s.storage
      ∧ (transferK s (.keyUse k)).status = s.status
      ∧ (transferK s (.keyUse k)).mayFail = s.mayFail := by
  cases hb : s.storage k with
  | none => simp [transferK, useK, hb, refuse]
  | some site =>
      by_cases hr : s.status site = some .released
      · simp [transferK, useK, hb, hr]
      · by_cases hc : certified s k = true
        · simp [transferK, useK, hb, hr, hc]
        · simp [transferK, useK, hb, hr, hc, refuse]

/-! ### Diagnostics only grow, so a `Safe` run used only certified keys -/

theorem transferI_diagnostics_grow (s : IState) (i : IInstr) :
    ∃ l, (transferI s i).diagnostics = l ++ s.diagnostics := by
  cases i with
  | init => exact ⟨[], rfl⟩
  | initUnheld => exact ⟨[.unknown], rfl⟩
  | statusTest st zero =>
      cases hw : s.witness st with
      | none => exact ⟨[], by simp [transferI, hw]⟩
      | some site =>
          cases zero
          · exact ⟨[], by simp [transferI, hw]⟩
          · exact ⟨[], by simp [transferI, hw]⟩
  | writeStatus => exact ⟨[], rfl⟩
  | finalise obj k =>
      cases hb : s.storage obj with
      | none => exact ⟨[.unknown], by simp [transferI, hb]⟩
      | some site =>
          by_cases hk : s.kind site = k
          · cases hs : s.status site with
            | none => exact ⟨[.unknown], by simp [transferI, hb, hk, hs]⟩
            | some st =>
                cases st with
                | live =>
                    cases hf : s.mayFail site
                    · exact ⟨[], by simp [transferI, hb, hk, hs, hf]⟩
                    · exact ⟨[.unknown], by simp [transferI, hb, hk, hs, hf]⟩
                | released =>
                    exact ⟨[.doubleRelease site], by simp [transferI, hb, hk, hs]⟩
          · exact ⟨[.kindMismatchedRelease site], by simp [transferI, hb, hk]⟩
  | use obj k =>
      cases hb : s.storage obj with
      | none => exact ⟨[], by simp [transferI, hb]⟩
      | some site =>
          by_cases hr : s.status site = some .released
          · exact ⟨[.useAfterClose site, .unknown], by simp [transferI, hb, hr]⟩
          · exact ⟨[], by simp [transferI, hb, hr]⟩
  | dataRoute => exact ⟨[.unknown], rfl⟩

theorem diagnostics_only_grow (s : IState) (i : KInstr) :
    ∃ l, (transferK s i).diagnostics = l ++ s.diagnostics := by
  cases i with
  | staticCreate => exact ⟨[], rfl⟩
  | staticFinalise => exact ⟨[.unknown], rfl⟩
  | automaticCreate k st site => exact transferI_diagnostics_grow s (.init k st site)
  | otherCreate => exact ⟨[.unknown], rfl⟩
  | keyUse k =>
      cases hb : s.storage k with
      | none => exact ⟨[.unknown], by simp [transferK, useK, hb, refuse]⟩
      | some site =>
          by_cases hr : s.status site = some .released
          · exact ⟨[.useAfterClose site, .unknown], by simp [transferK, useK, hb, hr]⟩
          · by_cases hc : certified s k = true
            · exact ⟨[], by simp [transferK, useK, hb, hr, hc]⟩
            · exact ⟨[.unknown], by simp [transferK, useK, hb, hr, hc, refuse]⟩
  | inPlace i => exact transferI_diagnostics_grow s i

theorem run_keeps_diagnostics (s : IState) (program : List KInstr) (d : Diagnostic)
    (h : d ∈ s.diagnostics) : d ∈ (runK s program).diagnostics := by
  induction program generalizing s with
  | nil => exact h
  | cons i rest ih =>
      show d ∈ (runK (transferK s i) rest).diagnostics
      apply ih
      obtain ⟨l, hl⟩ := diagnostics_only_grow s i
      rw [hl]
      exact List.mem_append_right _ h

/-- Every key use of `program`, run from `s`, met a certified key. -/
def usesCertified : IState → List KInstr → Prop
  | _, [] => True
  | s, .keyUse k :: rest => certified s k = true ∧ usesCertified (transferK s (.keyUse k)) rest
  | s, i :: rest => usesCertified (transferK s i) rest

/-- An uncertified use leaves a refusal that no later step removes. -/
theorem a_run_without_refusal_used_only_certified_keys (s : IState) (program : List KInstr)
    (h : .unknown ∉ (runK s program).diagnostics) : usesCertified s program := by
  induction program generalizing s with
  | nil => trivial
  | cons i rest ih =>
      have hrest : .unknown ∉ (runK (transferK s i) rest).diagnostics := h
      cases i with
      | keyUse k =>
          refine ⟨?_, ih _ hrest⟩
          cases hc : certified s k
          · exact absurd (run_keeps_diagnostics _ rest _
              (an_uncertified_use_is_refused s k hc)) hrest
          · rfl
      | staticCreate => exact ih _ hrest
      | staticFinalise => exact ih _ hrest
      | automaticCreate => exact ih _ hrest
      | otherCreate => exact ih _ hrest
      | inPlace => exact ih _ hrest

/-- **A `Safe` verdict used only certified keys.** -/
theorem a_safe_verdict_used_only_certified_keys (s : IState) (program : List KInstr)
    (sites : List Nat) (h : classify (exitI (runK s program) sites) = .safe) :
    usesCertified s program := by
  apply a_run_without_refusal_used_only_certified_keys
  intro hu
  exact a_refusal_is_never_safe _ (List.mem_append_right _ hu) h

/-! ### Witness programs -/

/-- A start state with nothing tracked; site 0 is a `thread_key` site. Key
name 0 (`k`), status name 1 (`r`). -/
def startK : IState :=
  { status := fun _ => none,
    kind := fun site => if site = 0 then .threadKey else .heap,
    storage := fun _ => none,
    witness := fun _ => none,
    mayFail := fun _ => false,
    diagnostics := [] }

/-- `r = pthread_key_create(&k, 0); if (r) return;`: the success edge. -/
def created : List KInstr := [.automaticCreate 0 1 0, .inPlace (.statusTest 1 true)]

/-- `…; v = pthread_getspecific(k); pthread_key_delete(k);` -/
theorem the_create_use_delete_program_is_safe :
    classify (exitI (runK startK (created ++ [.keyUse 0, .inPlace (.finalise 0 .threadKey)])) [0])
      = .safe := by
  decide

/-- The failure edge owes nothing. -/
theorem the_failure_edge_owes_nothing :
    exitI (runK startK [.automaticCreate 0 1 0, .inPlace (.statusTest 1 false)]) [0] = [] := by
  decide

/-- `…; pthread_key_delete(k); pthread_getspecific(k);` -/
theorem a_use_after_the_delete_is_a_candidate :
    .useAfterClose 0 ∈
      (runK startK (created ++ [.inPlace (.finalise 0 .threadKey), .keyUse 0])).diagnostics := by
  decide

/-- A missing delete is a leak candidate. -/
theorem a_missing_delete_leaks :
    exitI (runK startK (created ++ [.keyUse 0])) [0] = [.leak 0] := by
  decide

/-- A double delete is a candidate. -/
theorem a_double_delete_is_a_candidate :
    .doubleRelease 0 ∈ (runK startK (created ++ [.inPlace (.finalise 0 .threadKey),
      .inPlace (.finalise 0 .threadKey)])).diagnostics := by
  decide

/-- A use before the status test is refused. -/
theorem a_use_before_the_test_is_refused :
    classify (exitI (runK startK [.automaticCreate 0 1 0, .keyUse 0,
      .inPlace (.statusTest 1 true), .inPlace (.finalise 0 .threadKey)]) [0]) ≠ .safe := by
  decide

/-- A use on the failure edge is refused. -/
theorem a_use_on_the_failure_edge_is_refused :
    classify (exitI (runK startK [.automaticCreate 0 1 0, .inPlace (.statusTest 1 false),
      .keyUse 0]) [0]) ≠ .safe := by
  decide

/-- A use before any creation is refused. -/
theorem a_use_before_the_creation_is_refused :
    classify (exitI (runK startK (.keyUse 0 :: created ++ [.inPlace (.finalise 0 .threadKey)]))
      [0]) ≠ .safe := by
  decide

/-- `mk_http_thread_initialize_tls_once`: two static creations (statics 7, 8). -/
theorem the_two_static_creations_owe_nothing :
    exitI (runK startK [.staticCreate 7, .staticCreate 8]) [0] = []
      ∧ classify (runK startK [.staticCreate 7, .staticCreate 8]).diagnostics = .safe := by
  decide

end QCore.ThreadKeyStatic
