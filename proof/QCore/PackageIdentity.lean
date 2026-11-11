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
# The package-identity state machine, from load to dispatch

Card `P01-F3`. Model of `crates/qcore-verifier-host/src/executable_identity.rs`
and the construction discipline in `crates/qcore-verifier-host/src/lib.rs`:
a `ProcessProvider` exists only through `verify_executable_package`, and every
spawn re-checks the executable it is about to run.

## What is modelled

A *world* is what the filesystem holds right now: a content for each file
identity, and which identity (if any) the package's executable path names. A
*verified* handle is what verification captured: the digest of the bytes it
read, and the identity of the file it read them through — the descriptor the
host keeps open.

`revalidate` is the spawn-time decision. It is deliberately two independent
questions:

* has the *verified file* changed? — read back through the held identity, which
  a path substitution cannot redirect;
* does the *path* still name that file? — because the path is what `execve`
  resolves.

## Deliberate reductions

* Bytes are a single natural number per file, and the digest is an arbitrary
  parameter `digest`. Nothing here says SHA-256 is collision-resistant; where a
  claim needs injectivity it takes it as a hypothesis, and the one place the
  Rust code relies on it (the template re-check being unreachable behind the
  digest comparison) is recorded in the report as measurement, not proof.
* The embedded-template binding itself is `QCore.PackageTemplateBinding`; here
  it is one opaque predicate on content, so this module says how the *identity*
  gate composes with it, not what it decides.
* The residual window is modelled explicitly, as a theorem that says the gate
  does **not** close it (`a_post_check_substitution_is_not_refused`). That is
  the honest statement: this narrows the window, and only executing the held
  description itself would close it.
-/

namespace QCore.PackageIdentity

/-- A file identity: on Unix, the `(device, inode)` pair. -/
abbrev Ident := Nat

/-- File content, abstractly. -/
abbrev Content := Nat

/-- What the filesystem holds at one instant. -/
structure World where
  /-- The content of each file, by identity. -/
  file : Ident → Content
  /-- Which file the package's executable path names, if any. -/
  named : Option Ident
  deriving Inhabited

/-- What verification captured and kept: the digest it accepted, and the file it
read through. The held descriptor is *this identity*, not the path. -/
structure Verified where
  digest : Nat
  ident : Ident
  deriving DecidableEq, Repr

/-- The spawn-time decision. Every constructor other than `ok` is a refusal. -/
inductive Decision
  /-- The verified file is intact and the path still names it. -/
  | ok
  /-- The held descriptor reads back different content. -/
  | contentChanged
  /-- The path no longer names the verified file. -/
  | pathReplaced
  deriving DecidableEq, Repr

-- The digest function. An arbitrary parameter: no theorem below assumes it is
-- injective unless it says so.
variable (digest : Content → Nat)

/-- The spawn-time re-check.

Order matches the implementation: the content comparison through the held
descriptor runs first, so a rewritten file is named as changed content rather
than as a manifest divergence, and the path identity check runs last, as close
to the spawn as the code structure allows. -/
def revalidate (v : Verified) (w : World) : Decision :=
  if digest (w.file v.ident) ≠ v.digest then Decision.contentChanged
  else if w.named ≠ some v.ident then Decision.pathReplaced
  else Decision.ok

/-- The content an `execve` on the path would load, if the path names anything. -/
def loaded (w : World) : Option Content := w.named.map w.file

/-! ## What acceptance means -/

/-- Acceptance is exactly the conjunction of the two questions. -/
theorem revalidate_ok_iff (v : Verified) (w : World) :
    revalidate digest v w = Decision.ok ↔
      digest (w.file v.ident) = v.digest ∧ w.named = some v.ident := by
  unfold revalidate
  by_cases hc : digest (w.file v.ident) = v.digest
  · by_cases hn : w.named = some v.ident
    · simp [hc, hn]
    · simp [hc, hn]
  · simp [hc]

/-- An accepted spawn loads the very content the held descriptor just read. This
is the positive direction: acceptance is not merely "nothing looked wrong". -/
theorem ok_loads_the_rechecked_content (v : Verified) (w : World)
    (h : revalidate digest v w = Decision.ok) :
    loaded w = some (w.file v.ident) := by
  have := (revalidate_ok_iff digest v w).mp h
  simp [loaded, this.2]

/-- And that content answers to the verified digest. -/
theorem ok_loads_content_of_the_verified_digest (v : Verified) (w : World)
    (h : revalidate digest v w = Decision.ok) :
    ∃ c, loaded w = some c ∧ digest c = v.digest := by
  have hpair := (revalidate_ok_iff digest v w).mp h
  exact ⟨w.file v.ident, by simp [loaded, hpair.2], hpair.1⟩

/-- With an injective digest — which SHA-256 is *assumed* to be, never proved
here — acceptance pins the loaded bytes to the verified bytes exactly. -/
theorem ok_loads_the_verified_bytes (v : Verified) (w₀ w : World)
    (hinj : ∀ a b, digest a = digest b → a = b)
    (hverified : digest (w₀.file v.ident) = v.digest)
    (h : revalidate digest v w = Decision.ok) :
    loaded w = some (w₀.file v.ident) := by
  have hpair := (revalidate_ok_iff digest v w).mp h
  have : w.file v.ident = w₀.file v.ident :=
    hinj _ _ (by rw [hpair.1, hverified])
  simp [loaded, hpair.2, this]

/-! ## The refusals -/

/-- Every refusal is a refusal: no constructor other than `ok` is acceptance. -/
theorem refusals_are_not_acceptance (d : Decision) (h : d ≠ Decision.ok) :
    d = Decision.contentChanged ∨ d = Decision.pathReplaced := by
  cases d <;> simp_all

/-- Rewriting the verified file in place — the same identity, new content — is
refused as changed content. The path is untouched, so only the content
comparison through the held descriptor can see it. -/
theorem an_in_place_rewrite_is_refused (v : Verified) (w : World)
    (_hnamed : w.named = some v.ident)
    (hchanged : digest (w.file v.ident) ≠ v.digest) :
    revalidate digest v w = Decision.contentChanged := by
  simp [revalidate, hchanged]

/-- Replacing the path with a *different* file is refused, even when the new
file is byte-identical: what an `execve` would load is not the file that was
verified, and the digest cannot distinguish the two. -/
theorem a_replacement_is_refused (v : Verified) (w : World) (other : Ident)
    (hother : other ≠ v.ident)
    (hnamed : w.named = some other)
    (hintact : digest (w.file v.ident) = v.digest) :
    revalidate digest v w = Decision.pathReplaced := by
  have : w.named ≠ some v.ident := by
    rw [hnamed]
    simpa using hother
  simp [revalidate, hintact, this]

/-- Byte-identical content at a fresh identity is still refused. This is the
case a content-only check would admit, and the reason the path identity is
checked at all. -/
theorem a_byte_identical_replacement_is_refused (v : Verified) (w : World)
    (other : Ident) (hother : other ≠ v.ident)
    (hnamed : w.named = some other)
    (hidentical : w.file other = w.file v.ident)
    (hintact : digest (w.file v.ident) = v.digest) :
    revalidate digest v w = Decision.pathReplaced ∧ loaded w = some (w.file v.ident) :=
  ⟨a_replacement_is_refused digest v w other hother hnamed hintact, by
    simp [loaded, hnamed, hidentical]⟩

/-- Removing the executable is refused. -/
theorem a_removed_executable_is_refused (v : Verified) (w : World)
    (hnamed : w.named = none)
    (hintact : digest (w.file v.ident) = v.digest) :
    revalidate digest v w = Decision.pathReplaced := by
  simp [revalidate, hintact, hnamed]

/-- Fail-closed, contrapositive: anything the gate accepts satisfies both
conjuncts, so no substitution of either kind is admitted. -/
theorem no_substitution_is_admitted (v : Verified) (w : World)
    (h : revalidate digest v w = Decision.ok) :
    ¬ (digest (w.file v.ident) ≠ v.digest) ∧ ¬ (w.named ≠ some v.ident) := by
  have := (revalidate_ok_iff digest v w).mp h
  exact ⟨by simpa using this.1, by simpa using this.2⟩

/-! ## The load-to-dispatch state machine

The construction discipline in Rust is that `ProcessProvider` — the only thing a
registry accepts — is built exclusively by `verify_executable_package`, and that
`UnverifiedProvider` is a distinct type with no owning accessor. Here that is
the state machine: `dispatched` is reachable only through `verified`. -/

/-- Where a package handle is in its life. -/
inductive Phase
  /-- Loaded, nothing checked: `UnverifiedProvider`. -/
  | unverified
  /-- Verified: digest and embedded template bound, descriptor held. -/
  | verified (v : Verified)
  /-- A spawn happened. -/
  | dispatched (v : Verified)
  /-- A refusal: nothing spawned. -/
  | refused (d : Decision)
  deriving Repr

/-- The events a caller can apply. -/
inductive Event
  /-- Run the verification gate. -/
  | verify (v : Verified) (accepted : Bool)
  /-- Ask for a spawn in world `w`. -/
  | spawn (w : World)

/-- The transition relation.

There is deliberately no `verify`-free route into `dispatched`, and a spawn from
`unverified` stays `unverified` rather than dispatching: in Rust that state is
unreachable from a registry, because an unverified package is a different type.
A refused spawn lands in `refused`, from which nothing dispatches. -/
def step (p : Phase) (e : Event) : Phase :=
  match p, e with
  | Phase.unverified, Event.verify v accepted =>
      if accepted then Phase.verified v else Phase.unverified
  | Phase.unverified, Event.spawn _ => Phase.unverified
  | Phase.verified v, Event.spawn w =>
      match revalidate digest v w with
      | Decision.ok => Phase.dispatched v
      | d => Phase.refused d
  | p, _ => p

/-- Runs a sequence of events. -/
def run (p : Phase) : List Event → Phase
  | [] => p
  | e :: rest => run (step digest p e) rest

/-- Is this phase a dispatch? -/
def dispatched : Phase → Bool
  | Phase.dispatched _ => true
  | _ => false

/-- Is this phase verified-or-later? -/
def everVerified : Phase → Bool
  | Phase.unverified => false
  | _ => true

/-- One step never dispatches from an unverified handle: the only exit from
`unverified` is a successful verification, and that lands in `verified`. -/
theorem no_dispatch_from_unverified (e : Event) :
    dispatched (step digest Phase.unverified e) = false := by
  cases e with
  | verify v accepted => cases accepted <;> simp [step, dispatched]
  | spawn w => simp [step, dispatched]

/-- A dispatch's predecessor was verified. -/
theorem dispatch_predecessor_is_verified (p : Phase) (e : Event)
    (h : dispatched (step digest p e) = true) : everVerified p = true := by
  cases p with
  | unverified =>
      rw [no_dispatch_from_unverified] at h
      simp at h
  | verified v => simp [everVerified]
  | dispatched v => simp [everVerified]
  | refused d => simp [everVerified]

/-- Does this event carry a successful verification? -/
def acceptedVerify : Event → Bool
  | Event.verify _ accepted => accepted
  | Event.spawn _ => false

/-- An unverified handle stays unverified under any event that is not a
successful verification. -/
theorem unverified_is_stable (e : Event) (h : acceptedVerify e = false) :
    step digest Phase.unverified e = Phase.unverified := by
  cases e with
  | verify v accepted =>
      cases accepted with
      | false => simp [step]
      | true => simp [acceptedVerify] at h
  | spawn w => simp [step]

/-- **No path from unverified to dispatched.** Without a successful
verification in the trace, a handle that started unverified never dispatches —
however many spawns, refused verifications and further spawns are applied. -/
theorem never_dispatched_without_an_accepted_verification (events : List Event)
    (h : ∀ e ∈ events, acceptedVerify e = false) :
    dispatched (run digest Phase.unverified events) = false := by
  induction events with
  | nil => simp [run, dispatched]
  | cons e rest ih =>
      have hrest : ∀ x ∈ rest, acceptedVerify x = false :=
        fun x hx => h x (List.mem_cons_of_mem _ hx)
      have he : acceptedVerify e = false := h e (by simp)
      simp only [run, unverified_is_stable digest e he]
      exact ih hrest

/-- Contrapositive, as a requirement on the trace: a dispatch out of an
unverified handle needs an accepted verification somewhere before it. -/
theorem a_dispatch_requires_an_accepted_verification (events : List Event)
    (h : dispatched (run digest Phase.unverified events) = true) :
    ∃ e ∈ events, acceptedVerify e = true := by
  refine Classical.byContradiction fun hcon => ?_
  have hall : ∀ e ∈ events, acceptedVerify e = false := by
    intro e he
    cases hb : acceptedVerify e with
    | false => rfl
    | true => exact absurd ⟨e, he, hb⟩ hcon
  rw [never_dispatched_without_an_accepted_verification digest events hall] at h
  exact Bool.noConfusion h

/-- A refused verification is not a verification: it leaves the handle
unverified, so the next spawn still cannot dispatch. -/
theorem a_refused_verification_does_not_verify (v : Verified) :
    step digest Phase.unverified (Event.verify v false) = Phase.unverified := by
  simp [step]

/-- Nothing dispatches out of a refusal. -/
theorem a_refusal_never_dispatches (d : Decision) (e : Event) :
    dispatched (step digest (Phase.refused d) e) = false := by
  cases e <;> simp [step, dispatched]

/-- A dispatch carries the *same* verified handle it was verified with: the gate
cannot be passed with one package and spent on another. -/
theorem dispatch_carries_its_verification (v : Verified) (w : World)
    (h : step digest (Phase.verified v) (Event.spawn w) = Phase.dispatched v) :
    revalidate digest v w = Decision.ok := by
  simp only [step] at h
  split at h
  · assumption
  · simp at h

/-- A dispatch happens exactly when the re-check accepted. -/
theorem dispatch_iff_ok (v : Verified) (w : World) :
    dispatched (step digest (Phase.verified v) (Event.spawn w)) = true ↔
      revalidate digest v w = Decision.ok := by
  simp only [step]
  constructor
  · intro h
    split at h
    · assumption
    · simp [dispatched] at h
  · intro h; simp [h, dispatched]

/-! ## Non-vacuity, and the residual the gate does not close -/

/-- The honest package dispatches: the gate is not vacuously closed. -/
theorem an_unchanged_package_dispatches (v : Verified) (c : Content)
    (hdigest : digest c = v.digest) :
    dispatched (step digest (Phase.verified v)
      (Event.spawn { file := fun _ => c, named := some v.ident })) = true := by
  rw [dispatch_iff_ok]
  rw [revalidate_ok_iff]
  exact ⟨hdigest, rfl⟩

/-- **The window is narrowed, not closed.** For any verified handle there is a
world that passes the re-check and a *later* world, differing only in the file's
content, whose load is not the verified content. Nothing in this gate rules that
out: only executing the held file description itself would, and the report
records why that is not done. -/
theorem a_post_check_substitution_is_not_refused (v : Verified) (c c' : Content)
    (hdigest : digest c = v.digest) (hdiff : c ≠ c') :
    ∃ w after : World,
      revalidate digest v w = Decision.ok ∧
      loaded after ≠ loaded w ∧
      w.named = after.named := by
  refine ⟨{ file := fun _ => c, named := some v.ident },
          { file := fun _ => c', named := some v.ident }, ?_, ?_, rfl⟩
  · rw [revalidate_ok_iff]; exact ⟨hdigest, rfl⟩
  · intro heq
    have hcc : c' = c := by simpa [loaded] using heq
    exact hdiff hcc.symm

/-- The decision reads the world only through the two observations it makes: the
content behind the held identity, and what the path names. Two worlds agreeing
on those decide the same, so nothing else about the filesystem can move the
answer — and, in the other direction, no other observation is available to
narrow the window further. -/
theorem revalidate_depends_only_on_the_two_observations (v : Verified)
    (w w' : World)
    (hcontent : w.file v.ident = w'.file v.ident)
    (hnamed : w.named = w'.named) :
    revalidate digest v w = revalidate digest v w' := by
  simp [revalidate, hcontent, hnamed]

/-- Both refusals are reachable: neither branch is dead, so the two-question
structure is not one question with an unreachable second arm. -/
theorem both_refusals_are_reachable (v : Verified) (c : Content)
    (hdigest : digest c = v.digest) (hother : digest (c + 1) ≠ v.digest) :
    revalidate digest v { file := fun _ => c + 1, named := some v.ident } =
        Decision.contentChanged ∧
      revalidate digest v { file := fun _ => c, named := none } =
        Decision.pathReplaced := by
  constructor
  · simp [revalidate, hother]
  · simp [revalidate, hdigest]

/-- A refusal is never silently a dispatch, in either direction. -/
theorem refusal_and_dispatch_are_exclusive (v : Verified) (w : World) :
    ¬ (revalidate digest v w ≠ Decision.ok ∧
       dispatched (step digest (Phase.verified v) (Event.spawn w)) = true) := by
  intro ⟨hne, hd⟩
  exact hne ((dispatch_iff_ok digest v w).mp hd)

end QCore.PackageIdentity
