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
# A memo served only on key equality (card W45-GATEWAY-MEMO-KEY)

The abstract rule implemented by the per-thread memo of
`member_releasing_functions` in
`crates/qcore-c-gateway/src/legacy_member_release.rs`.

That memo answered a token stream from its one cached entry whenever a 64-bit
hash of the stream's token texts matched the entry's. Two different streams
with one hash would have been handed each other's answer: the set of functions
the unit-callee guard must precede, so a collision could drop a guard and turn
its refusal into an admission. The card keeps the hash only as a cheap reject
and serves an entry only when the whole key, every token text, is equal.

The model is generic in the key `K`, the answer `V`, the uncached function
`f : K → V` and the digest `digest : K → D`. Nothing is assumed of the digest:
every theorem below holds for a constant one, the worst collision there is.

* `lookup` is the verified hit: the digest and the key both agree.
* `call` is the memoised function: a hit returns the stored answer and keeps
  the state, a miss computes `f` and replaces the single entry.
* `Filled` is the invariant the memo itself maintains: its entry's answer is
  `f` of its entry's key. A state that breaks it (the test hook's injected
  entry) is covered by `a_mismatched_entry_gives_the_uncached_answer`, which
  needs no invariant at all.

The main result, `run_eq_map`, is observational equivalence: any sequence of
calls through the memo returns exactly `f` of each key, whatever the digest.
`the_digest_alone_serves_a_collision` shows the replaced rule is not.

## The memo's removal (card W45-MEMO-MEMORY)

The memo's key, a copy of every token text, stayed resident in every worker
thread, and the memo saved no measurable time, so card W45-MEMO-MEMORY removed
it: `member_releasing_functions` now computes `f` on every call. That removal
changes no answer by `removing_the_memo_changes_no_answer`, a corollary of
`run_eq_map` from any state the memo filled, the empty one included. The
definitions above now model the replaced code, not current Rust.

## What this is not

A proof about the Rust (trust boundary 28): that the Rust key is the whole
input the uncached function reads (every token text; `line` is not read), and
that `compute_member_releasing_functions` is a function of it, were review
premises pinned by the module's unit tests. Since the removal there is no key
and no memo state: the Rust calls the function itself, which is what `f`
stands for here.
-/

namespace QCore.MemoKey

/-- The memo's single entry: the digest it was filed under, its whole key and
its answer. -/
structure Entry (K V D : Type) where
  digest : D
  key : K
  value : V

variable {K V D : Type} [DecidableEq K] [DecidableEq D]

/-- A verified lookup: a hit only when the digest **and** the whole key agree. -/
def lookup (digest : K → D) (k : K) : Option (Entry K V D) → Option V
  | some e => if e.digest = digest k ∧ e.key = k then some e.value else none
  | none => none

/-- The memoised call: its answer and the memo state after it. -/
def call (f : K → V) (digest : K → D) (k : K) (m : Option (Entry K V D)) :
    V × Option (Entry K V D) :=
  match lookup digest k m with
  | some v => (v, m)
  | none => (f k, some ⟨digest k, k, f k⟩)

/-- The invariant the memo maintains: its entry answers its own key with `f`. -/
def Filled (f : K → V) : Option (Entry K V D) → Prop
  | some e => e.value = f e.key
  | none => True

/-- Running a sequence of keys through the memo, threading its state. -/
def run (f : K → V) (digest : K → D) : List K → Option (Entry K V D) → List V
  | [], _ => []
  | k :: ks, m =>
    let (v, m') := call f digest k m
    v :: run f digest ks m'

/-- A hit returns an answer only for an entry whose whole key is the query. -/
theorem a_hit_has_an_equal_key (digest : K → D) (k : K) (e : Entry K V D) (v : V)
    (h : lookup digest k (some e) = some v) : e.key = k ∧ e.value = v := by
  simp only [lookup] at h
  split at h
  · rename_i hc
    exact ⟨hc.2, Option.some.inj h⟩
  · cases h

/-- **A forced collision is a miss**: an entry whose digest equals the query's
but whose key differs is never served. -/
theorem a_colliding_entry_is_a_miss (digest : K → D) (k : K) (e : Entry K V D)
    (_hd : e.digest = digest k) (hk : e.key ≠ k) : lookup digest k (some e) = none := by
  simp [lookup, hk]

/-- An entry with another key gives the uncached answer, whatever its digest
and whatever answer it holds: no invariant is needed. -/
theorem a_mismatched_entry_gives_the_uncached_answer (f : K → V) (digest : K → D)
    (k : K) (e : Entry K V D) (hk : e.key ≠ k) : (call f digest k (some e)).1 = f k := by
  have hmiss : lookup digest k (some e) = none := by simp [lookup, hk]
  simp [call, hmiss]

/-- Under the invariant, every call returns the uncached answer. -/
theorem call_eq_uncached (f : K → V) (digest : K → D) (k : K) (m : Option (Entry K V D))
    (h : Filled f m) : (call f digest k m).1 = f k := by
  unfold call
  cases hl : lookup digest k m with
  | none => rfl
  | some v =>
    cases m with
    | none => cases hl
    | some e =>
      obtain ⟨hkey, hval⟩ := a_hit_has_an_equal_key digest k e v hl
      show v = f k
      rw [← hval, ← hkey]
      exact h

/-- Every call keeps the invariant. -/
theorem call_keeps_filled (f : K → V) (digest : K → D) (k : K) (m : Option (Entry K V D))
    (h : Filled f m) : Filled f (call f digest k m).2 := by
  unfold call
  cases lookup digest k m with
  | none => show f k = f k; rfl
  | some _ => exact h

/-- **Observational equivalence**: from an empty memo, or any state the memo
filled, a sequence of calls answers exactly `f` of each key, for every digest. -/
theorem run_eq_map (f : K → V) (digest : K → D) :
    ∀ (ks : List K) (m : Option (Entry K V D)), Filled f m → run f digest ks m = ks.map f
  | [], _, _ => rfl
  | k :: ks, m, h => by
    show (call f digest k m).1 :: run f digest ks (call f digest k m).2 = f k :: ks.map f
    rw [call_eq_uncached f digest k m h,
      run_eq_map f digest ks _ (call_keeps_filled f digest k m h)]

omit [DecidableEq K] [DecidableEq D] in
/-- The empty memo satisfies the invariant. -/
theorem empty_is_filled (f : K → V) : Filled (D := D) f none := True.intro

/-- **Removing the memo changes no answer** (card W45-MEMO-MEMORY): calling the
uncached function on every key answers exactly what the memo answered, from
the empty memo or from any state the memo filled on earlier keys (a worker
thread's earlier units), for every digest. -/
theorem removing_the_memo_changes_no_answer (f : K → V) (digest : K → D)
    (ks : List K) (m : Option (Entry K V D)) (h : Filled f m) :
    ks.map f = run f digest ks m :=
  (run_eq_map f digest ks m h).symm

/-- The rule the card replaces: a hit on the digest alone. -/
def lookupDigestOnly (digest : K → D) (k : K) : Option (Entry K V D) → Option V
  | some e => if e.digest = digest k then some e.value else none
  | none => none

/-- The digest-only memoised call. -/
def callDigestOnly (f : K → V) (digest : K → D) (k : K) (m : Option (Entry K V D)) :
    V × Option (Entry K V D) :=
  match lookupDigestOnly digest k m with
  | some v => (v, m)
  | none => (f k, some ⟨digest k, k, f k⟩)

/-- **The replaced rule is unsound**: with a colliding digest, the second key
is answered with the first key's answer, from a filled memo. -/
theorem the_digest_alone_serves_a_collision :
    (callDigestOnly (fun n : Nat => n) (fun _ => (0 : Nat)) 1
        (callDigestOnly (fun n : Nat => n) (fun _ => (0 : Nat)) 0 none).2).1 = 0 ∧
      (call (fun n : Nat => n) (fun _ => (0 : Nat)) 1
        (call (fun n : Nat => n) (fun _ => (0 : Nat)) 0 none).2).1 = 1 := by
  decide

end QCore.MemoKey
