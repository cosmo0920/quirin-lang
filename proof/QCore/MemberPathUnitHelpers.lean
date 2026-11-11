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
# Rule 30's validation through the unit's helpers (card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS)

The model of the bounded path reading behind gateway rule 30's body check
(contract specification C04.19.6), amended by this card. Abstract rule
theorems over one stated member path, not an extraction of the Rust
analyzer (`crates/qcore-c-gateway/src/member_path_reading.rs`: `Reader`,
`refine`, `establishes`, `guard_may_hold`).

* **Concrete reading.** On one run the member either is untouched since
  entry or was written, and reads null or not (`Conc`). A program is the
  reading's statement forms (`Stmt`): a null store, any other store, a call
  the reading does not follow, a null / non-null test, sequence, choice
  (branches, loops and `goto` unroll into these), and a call of a unit
  function (`call f`, whose body is `body f`). `Exec` is the concrete
  big-step relation; a call runs its body.
* **Abstract reading.** `aexec` over a value set (`Vals`: Rust `ORIG`,
  `ORIG_NULL`, `NULL`, `OTHER`), following a call while `fuel` lasts (the
  frame depth; a recursive call is not followed either) and leaving it
  `other` (not followed) at fuel `0`.
* **Soundness** (`aexec_sound`): every concrete run's final reading is
  admitted by the abstract reading, for every fuel. Hence a validated must
  path reads null on every run (`a_validated_must_path_reads_null`), a
  validated set path is untouched or null
  (`a_validated_set_path_is_kept_or_null`), a validated stale or kept path is
  untouched (`a_validated_stale_path_is_untouched`).
* **A call summary is inlining** (`a_call_summary_is_inlining`,
  `a_call_runs_its_body`): reading `call f` with fuel left is reading
  `body f`, and a concrete call runs exactly its body.
* **A path not followed never validates**
  (`a_path_not_followed_never_validates`,
  `the_depth_bound_never_validates`): the value set after a call that is
  not followed admits `other`, which establishes no state.
* **Guards** (`an_exempt_exit_falsifies_its_guard`,
  `an_unnamed_return_is_never_exempt`, `an_eq_guard_is_never_exempt`): an
  exit is exempt from a state only when it returns the enumerator of the
  state's `!=` guard, where the guard is false; every other exit is read.
-/

namespace QCore.MemberPathUnitHelpers

/-- A member's concrete reading on one run: untouched since entry or
written, and whether it reads null. -/
inductive Conc where
  | untouched (isNull : Bool)
  | written (isNull : Bool)
  deriving DecidableEq, Repr

/-- The reading's statement forms. -/
inductive Stmt where
  | skip
  | storeNull
  | storeOther
  | notFollowed
  | testNull
  | testNonNull
  | seq (a b : Stmt)
  | choice (a b : Stmt)
  | call (f : Nat)
  deriving Repr

/-- The concrete big-step relation; `body f` is the unit's function `f`. -/
inductive Exec (body : Nat → Stmt) : Stmt → Conc → Conc → Prop where
  | skip (c : Conc) : Exec body .skip c c
  | storeNull (c : Conc) : Exec body .storeNull c (.written true)
  | storeOther (c : Conc) (b : Bool) : Exec body .storeOther c (.written b)
  | notFollowedKeeps (c : Conc) : Exec body .notFollowed c c
  | notFollowedWrites (c : Conc) (b : Bool) : Exec body .notFollowed c (.written b)
  | testNullUntouched : Exec body .testNull (.untouched true) (.untouched true)
  | testNullWritten : Exec body .testNull (.written true) (.written true)
  | testNonNullUntouched : Exec body .testNonNull (.untouched false) (.untouched false)
  | testNonNullWritten : Exec body .testNonNull (.written false) (.written false)
  | seq {a b : Stmt} {c c' c'' : Conc} :
      Exec body a c c' → Exec body b c' c'' → Exec body (.seq a b) c c''
  | choiceLeft {a b : Stmt} {c c' : Conc} : Exec body a c c' → Exec body (.choice a b) c c'
  | choiceRight {a b : Stmt} {c c' : Conc} : Exec body b c c' → Exec body (.choice a b) c c'
  | call {f : Nat} {c c' : Conc} : Exec body (body f) c c' → Exec body (.call f) c c'

/-- The reading's value set (Rust `ORIG`, `ORIG_NULL`, `NULL`, `OTHER`). -/
structure Vals where
  orig : Bool
  origNull : Bool
  null : Bool
  other : Bool
  deriving DecidableEq, Repr

/-- What a value set admits (`orig` reads either way; `other` anything). -/
def admits (v : Vals) : Conc → Bool
  | .untouched false => v.orig
  | .untouched true => v.orig || v.origNull
  | .written true => v.null || v.other
  | .written false => v.other

/-- Rust `join_into`. -/
def Vals.join (v w : Vals) : Vals :=
  ⟨v.orig || w.orig, v.origNull || w.origNull, v.null || w.null, v.other || w.other⟩

/-- A call not followed, any other store: anything may follow. -/
def havoc (v : Vals) : Vals := { v with other := true }

/-- Rust `refine`, the branch where the path reads null. -/
def refineNull (v : Vals) : Vals :=
  ⟨false, v.orig || v.origNull, v.null || v.other, false⟩

/-- Rust `refine`, the branch where the path reads non-null. -/
def refineNonNull (v : Vals) : Vals :=
  ⟨v.orig, false, false, v.other⟩

/-- The abstract reading, following a call while `fuel` lasts. -/
def aexec (body : Nat → Stmt) : Nat → Stmt → Vals → Vals
  | _, .skip, v => v
  | _, .storeNull, _ => ⟨false, false, true, false⟩
  | _, .storeOther, v => havoc v
  | _, .notFollowed, v => havoc v
  | _, .testNull, v => refineNull v
  | _, .testNonNull, v => refineNonNull v
  | fuel, .seq a b, v => aexec body fuel b (aexec body fuel a v)
  | fuel, .choice a b, v => (aexec body fuel a v).join (aexec body fuel b v)
  | 0, .call _, v => havoc v
  | fuel + 1, .call f, v => aexec body fuel (body f) v
termination_by fuel s => (fuel, sizeOf s)

theorem admits_join_left (v w : Vals) (c : Conc) (h : admits v c = true) :
    admits (v.join w) c = true := by
  cases c with
  | untouched n =>
      cases n <;> simp [admits, Vals.join] at h ⊢ <;> (try rcases h with h | h) <;> simp [h]
  | written n =>
      cases n <;> simp [admits, Vals.join] at h ⊢ <;> (try rcases h with h | h) <;> simp [h]

theorem admits_join_right (v w : Vals) (c : Conc) (h : admits w c = true) :
    admits (v.join w) c = true := by
  cases c with
  | untouched n =>
      cases n <;> simp [admits, Vals.join] at h ⊢ <;> (try rcases h with h | h) <;> simp [h]
  | written n =>
      cases n <;> simp [admits, Vals.join] at h ⊢ <;> (try rcases h with h | h) <;> simp [h]

theorem admits_havoc_keeps (v : Vals) (c : Conc) (h : admits v c = true) :
    admits (havoc v) c = true := by
  cases c with
  | untouched n => cases n <;> simp_all [admits, havoc]
  | written n => cases n <;> simp_all [admits, havoc]

theorem admits_havoc_written (v : Vals) (b : Bool) : admits (havoc v) (.written b) = true := by
  cases b <;> simp [admits, havoc]

/-- A run that ends untouched started from the same reading: every form
keeps the reading or writes it. -/
theorem untouched_end_is_entry (body : Nat → Stmt) {s : Stmt} {c c' : Conc}
    (h : Exec body s c c') : ∀ n, c' = .untouched n → c = .untouched n := by
  induction h with
  | skip => intro n h; exact h
  | storeNull => intro n h; cases h
  | storeOther => intro n h; cases h
  | notFollowedKeeps => intro n h; exact h
  | notFollowedWrites => intro n h; cases h
  | testNullUntouched => intro n h; exact h
  | testNullWritten => intro n h; exact h
  | testNonNullUntouched => intro n h; exact h
  | testNonNullWritten => intro n h; exact h
  | seq _ _ iha ihb => intro n h; exact iha n (ihb n h)
  | choiceLeft _ ih => intro n h; exact ih n h
  | choiceRight _ ih => intro n h; exact ih n h
  | call _ ih => intro n h; exact ih n h

/-- **Soundness**: every concrete run's final reading is admitted by the
abstract reading of the same statement, for every fuel. -/
theorem aexec_sound (body : Nat → Stmt) {s : Stmt} {c c' : Conc}
    (h : Exec body s c c') : ∀ (fuel : Nat) (v : Vals), admits v c = true →
      admits (aexec body fuel s v) c' = true := by
  induction h with
  | skip c => intro fuel v hv; simpa [aexec] using hv
  | storeNull c => intro fuel v _; simp [aexec, admits]
  | storeOther c b => intro fuel v _; simpa [aexec] using admits_havoc_written v b
  | notFollowedKeeps c => intro fuel v hv; simpa [aexec] using admits_havoc_keeps v c hv
  | notFollowedWrites c b => intro fuel v _; simpa [aexec] using admits_havoc_written v b
  | testNullUntouched => intro fuel v hv; simp_all [aexec, admits, refineNull]
  | testNullWritten => intro fuel v hv; simp_all [aexec, admits, refineNull]
  | testNonNullUntouched => intro fuel v hv; simp_all [aexec, admits, refineNonNull]
  | testNonNullWritten => intro fuel v hv; simp_all [aexec, admits, refineNonNull]
  | seq _ _ iha ihb => intro fuel v hv; simpa [aexec] using ihb fuel _ (iha fuel v hv)
  | choiceLeft _ ih =>
      intro fuel v hv; simpa [aexec] using admits_join_left _ _ _ (ih fuel v hv)
  | choiceRight _ ih =>
      intro fuel v hv; simpa [aexec] using admits_join_right _ _ _ (ih fuel v hv)
  | @call f c c' hb ih =>
      intro fuel v hv
      cases fuel with
      | zero =>
          cases c' with
          | untouched n =>
              have hc := untouched_end_is_entry body hb n rfl
              subst hc
              simpa [aexec] using admits_havoc_keeps v _ hv
          | written n => simpa [aexec] using admits_havoc_written v n
      | succ n => simpa [aexec] using ih n v hv

/-- Rust `establishes` for `#released` / `#moved`: no untouched, no other. -/
def establishesMust (v : Vals) : Bool := !v.orig && !v.other

/-- Rust `establishes` for `{V, #kept}`: no other (beside the null store the
reading saw). -/
def establishesSet (v : Vals) : Bool := !v.other

/-- Rust `establishes` for `#stale` / `#kept`: untouched only. -/
def establishesStale (v : Vals) : Bool := !v.null && !v.other

def readsNull : Conc → Bool
  | .untouched n => n
  | .written n => n

def keptOrNull : Conc → Bool
  | .untouched _ => true
  | .written n => n

def isUntouched : Conc → Bool
  | .untouched _ => true
  | .written _ => false

/-- A validated must path reads null on every run. -/
theorem a_validated_must_path_reads_null (body : Nat → Stmt) (s : Stmt) (fuel : Nat)
    (v : Vals) (c c' : Conc) (hrun : Exec body s c c') (hv : admits v c = true)
    (hest : establishesMust (aexec body fuel s v) = true) : readsNull c' = true := by
  have ha := aexec_sound body hrun fuel v hv
  generalize aexec body fuel s v = w at ha hest
  cases c' with
  | untouched n => cases n <;> simp_all [admits, establishesMust, readsNull]
  | written n => cases n <;> simp_all [admits, establishesMust, readsNull]

/-- A validated set path is untouched or null on every run. -/
theorem a_validated_set_path_is_kept_or_null (body : Nat → Stmt) (s : Stmt) (fuel : Nat)
    (v : Vals) (c c' : Conc) (hrun : Exec body s c c') (hv : admits v c = true)
    (hest : establishesSet (aexec body fuel s v) = true) : keptOrNull c' = true := by
  have ha := aexec_sound body hrun fuel v hv
  generalize aexec body fuel s v = w at ha hest
  cases c' with
  | untouched n => cases n <;> simp_all [admits, establishesSet, keptOrNull]
  | written n => cases n <;> simp_all [admits, establishesSet, keptOrNull]

/-- A validated stale (or kept) path is untouched on every run. -/
theorem a_validated_stale_path_is_untouched (body : Nat → Stmt) (s : Stmt) (fuel : Nat)
    (v : Vals) (c c' : Conc) (hrun : Exec body s c c') (hv : admits v c = true)
    (hest : establishesStale (aexec body fuel s v) = true) : isUntouched c' = true := by
  have ha := aexec_sound body hrun fuel v hv
  generalize aexec body fuel s v = w at ha hest
  cases c' with
  | untouched n => rfl
  | written n => cases n <;> simp_all [admits, establishesStale]

/-- **A call summary is inlining**: with fuel left, reading the call is
reading the callee's body from the caller's value set. -/
theorem a_call_summary_is_inlining (body : Nat → Stmt) (fuel f : Nat) (v : Vals) :
    aexec body (fuel + 1) (.call f) v = aexec body fuel (body f) v := by
  simp [aexec]

/-- A concrete call runs exactly its body. -/
theorem a_call_runs_its_body (body : Nat → Stmt) (f : Nat) (c c' : Conc) :
    Exec body (.call f) c c' ↔ Exec body (body f) c c' := by
  constructor
  · intro h; cases h with
    | call hb => exact hb
  · intro h; exact .call h

/-- **A path not followed never validates**: after a call the reading does
not follow, no state is established. -/
theorem a_path_not_followed_never_validates (v : Vals) :
    establishesMust (havoc v) = false ∧ establishesSet (havoc v) = false ∧
      establishesStale (havoc v) = false := by
  simp [havoc, establishesMust, establishesSet, establishesStale]

/-- At the depth bound (or a recursive call) the call is not followed. -/
theorem the_depth_bound_never_validates (body : Nat → Stmt) (f : Nat) (v : Vals) :
    aexec body 0 (.call f) v = havoc v := by
  simp [aexec]

/-- A null store then the end validates a must path (the base reading). -/
theorem a_null_store_validates_must (body : Nat → Stmt) (fuel : Nat) (v : Vals) :
    establishesMust (aexec body fuel .storeNull v) = true := by
  simp [aexec, establishesMust]

inductive GuardOp where
  | eq
  | ne
  deriving DecidableEq, Repr

/-- A state's guard `#result OP @K`. -/
structure Guard where
  op : GuardOp
  constant : String
  deriving DecidableEq, Repr

/-- Whether the guard holds of `result`, the constant's value being `k`. -/
def holds (g : Guard) (result k : Int) : Bool :=
  match g.op with
  | .eq => result == k
  | .ne => result != k

/-- Rust `guard_may_hold`: an exit returning the named enumerator (`some`)
or anything (`none`). -/
def mayHold (g : Guard) : Option String → Bool
  | some name => if name = g.constant then g.op = .eq else true
  | none => true

/-- An exempt exit returns the guard's own enumerator, where a `!=` guard is
false. -/
theorem an_exempt_exit_falsifies_its_guard (g : Guard) (name : String) (k : Int)
    (h : mayHold g (some name) = false) : name = g.constant ∧ holds g k k = false := by
  obtain ⟨op, constant⟩ := g
  by_cases hn : name = constant
  · cases op <;> simp_all [mayHold, holds]
  · simp_all [mayHold]

/-- An exit whose returned value is not a named enumerator is always read. -/
theorem an_unnamed_return_is_never_exempt (g : Guard) : mayHold g none = true := rfl

/-- An `==` guard exempts no exit. -/
theorem an_eq_guard_is_never_exempt (constant : String) (r : Option String) :
    mayHold ⟨.eq, constant⟩ r = true := by
  cases r with
  | none => rfl
  | some name => by_cases h : name = constant <;> simp [mayHold, h]

end QCore.MemberPathUnitHelpers
