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
# Thread-specific data keys in place (card CURRENT-CONTEXT-NEW-TRIAGE, D3)

`crates/qcore-c-gateway/src/legacy_in_place_custody.rs` (`in_place_actuals`,
`STATIC_KEY_RELEASE_REFUSAL`), `legacy_classify.rs` (the static-held
positions read as borrowed), and the libc declarations of
`pthread_key_create` / `pthread_key_delete` (in-place kind `thread_key`).

Gateway rule 11v (5) extends CG25's in-place reading (`QCore.ResourceKinds`)
to keys:

* A key created into `&k`, `k` one automatic object, is **refused**. A
  tracked by-value key could reach a statement route that reads it as data:
  the card's gate found `(void) pthread_getspecific(k)` admitted after
  `pthread_key_delete(k)`. So no key is tracked.
* A key created into `&G`, `G` of static storage duration, is held by the
  static object under the stated premise **P-THREAD-KEY-STATIC**: a key
  created into static storage with no destructor is created once and is not
  owed a delete. The creation binds **no frame obligation**, so it owes
  nothing at any exit and needs no status witness.
* A by-value finaliser of a static key is **refused**: the holder's lifetime
  is outside the frame, and a delete another frame may still depend on is
  not this frame's to decide.

## What is proved

* A static creation leaves the in-place state unchanged, so it owes nothing
  at any exit (`a_static_creation_owes_nothing`), whatever runs after it.
* A static finaliser is refused (`a_static_finaliser_is_refused`) and so is
  never `Safe` (`a_static_finaliser_is_never_safe`).
* An automatic creation is refused and never `Safe`
  (`an_automatic_creation_is_refused`), and it binds no storage, so no later
  step can read a tracked key (`an_automatic_creation_tracks_nothing`).
* Every other in-place step is `QCore.ResourceKinds`' step
  (`in_place_is_cg25`).
* `mk_http_thread_initialize_tls_once`'s shape, two static creations and
  nothing else, owes nothing and carries no diagnostic
  (`the_two_static_creations_owe_nothing`).

## What this is not

Not a proof that a static key is created once: that is P-THREAD-KEY-STATIC,
stated, not proved. Monkey guards each creation by `pthread_once` or a flag.
Which actuals are static is read off the token stream and Clang
(`publication::destinations` minus `thread_local_objects`, trust boundary 28),
pinned by `crates/qcore-c-gateway/tests/thread_key_in_place.rs`.
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
  /-- `pthread_key_create(&k, NULL)`, `k` not of static storage duration. -/
  | automaticCreate (k : Nat)
  /-- Every other step: CG25's in-place reading. -/
  | inPlace (i : IInstr)
  deriving DecidableEq, Repr

def transferK (s : IState) : KInstr → IState
  | .staticCreate _ => s
  | .staticFinalise _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .automaticCreate _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .inPlace i => transferI s i

def runK (s : IState) (program : List KInstr) : IState :=
  program.foldl transferK s

theorem in_place_is_cg25 (s : IState) (i : IInstr) : transferK s (.inPlace i) = transferI s i :=
  rfl

/-- A static creation changes nothing a later step or an exit reads. -/
theorem a_static_creation_owes_nothing (s : IState) (g : Nat) (sites : List Nat) :
    exitI (transferK s (.staticCreate g)) sites = exitI s sites := rfl

theorem a_static_creation_binds_no_storage (s : IState) (g : Nat) :
    (transferK s (.staticCreate g)).storage = s.storage := rfl

theorem a_static_finaliser_is_refused (s : IState) (g : Nat) :
    .unknown ∈ (transferK s (.staticFinalise g)).diagnostics := by
  simp [transferK]

theorem a_static_finaliser_is_never_safe (s : IState) (g : Nat) :
    classify (transferK s (.staticFinalise g)).diagnostics ≠ .safe :=
  a_refusal_is_never_safe _ (a_static_finaliser_is_refused s g)

theorem an_automatic_creation_is_refused (s : IState) (k : Nat) :
    .unknown ∈ (transferK s (.automaticCreate k)).diagnostics
      ∧ classify (transferK s (.automaticCreate k)).diagnostics ≠ .safe := by
  have h : .unknown ∈ (transferK s (.automaticCreate k)).diagnostics := by simp [transferK]
  exact ⟨h, a_refusal_is_never_safe _ h⟩

theorem an_automatic_creation_tracks_nothing (s : IState) (k : Nat) :
    (transferK s (.automaticCreate k)).storage = s.storage
      ∧ (transferK s (.automaticCreate k)).status = s.status := ⟨rfl, rfl⟩

/-- A start state with nothing tracked. -/
def startK : IState :=
  { status := fun _ => none,
    kind := fun _ => .heap,
    storage := fun _ => none,
    witness := fun _ => none,
    mayFail := fun _ => false,
    diagnostics := [] }

/-- `mk_http_thread_initialize_tls_once`: two static creations (statics 7, 8). -/
theorem the_two_static_creations_owe_nothing :
    exitI (runK startK [.staticCreate 7, .staticCreate 8]) [0] = []
      ∧ classify (runK startK [.staticCreate 7, .staticCreate 8]).diagnostics = .safe := by
  decide

end QCore.ThreadKeyStatic
