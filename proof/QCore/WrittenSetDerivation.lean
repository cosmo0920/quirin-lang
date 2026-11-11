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

import QCore.ResultSet

/-!
# The slot written-set derivation (card CELL-RESULT-SET-WRITTEN-SET-DERIVATION)

USER DECISIONS 2026-11-04 (verbatim): "Basically, Q1(a) and Q2(i) But we must
not handle descriptor 0 as a valid handle. So, we need to exclude the 0
descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in
[1, MAX]"; "Write [1, MAX]". The user launched the card on 2026-11-04
(verbatim): "CELL-RESULT-SET-WRITTEN-SET-DERIVATION."

Contract specification C09.4, grammar G05j as amended, gateway rule 38e as
amended. A pinned `written_set(SLOT[K], SET) when #result == C` is derived at
`quirin-audit pin --bounds --derive-results` time from the configured unit's
body (Rust `qcore_c_gateway::null_deref` `written.rs` walks the AST; the rule's
C-free part is `crates/qcore-core/src/written_set.rs`: `combine`, `narrow`,
`resolve`, `after_store`, `COMBINE_VECTORS`, `RESOLVE_VECTORS`). Abstract
theorems over meanings and shared vectors over the interval implementation,
not an extraction of the Rust.

* **Paths combine** (`combine_sound`): each path ending at a `return` carries
  the values it may return and the values the slot may hold there (or none:
  unwritten, unknown). If every path that may return `C` carries a slot set,
  the union of those sets holds the slot on every run returning `C`.
* **Narrowing** (`narrow_sound`, `a_narrowing_without_the_class_constrains_nothing`):
  a test `v OP k` keeps every value its edge admits, so a `return ret` whose
  narrowed set excludes `C` never returns `C`.
* **Resolution** (`resolve_sound`, `an_empty_narrowing_is_infeasible`): once a
  call's result is narrowed to exactly one class `c`, the out array holds the
  callee's written set of class `c`.
* **Aliasing stores** (`after_store_sound`): a later store through a pointer
  that may alias the slot joins its value.
* **Forwarding** (`forwarding_sound`): a function returning exactly its
  callee's value, the callee's write last, has the callee's written set.
* **The bound decides, not POSIX** (`the_declared_bound_admits_the_creation`,
  `posix_s_set_derives_descriptor_zero`): with `pipe`'s declared `[1, MAX]`
  the epoll chain's slot set lies in `live_set([1, MAX])`; with POSIX's
  `[0, MAX]` it holds descriptor 0 and does not.
* **Vectors** (`combine_vectors_hold`, `resolve_vectors_hold`): Rust
  `COMBINE_VECTORS` and `RESOLVE_VECTORS`, entry for entry.
-/

namespace QCore.WrittenSetDerivation

open QCore.ResultSet

/-! ## The rule over meanings -/

/-- One path's end over meanings: what it may return, and what the slot may
hold there (`none`: unwritten or unknown). -/
structure EndP where
  returns : Int → Bool
  slot : Option (Int → Bool)

/-- The slot's values on a path end (none when it carries no set). -/
def slotHas (e : EndP) (v : Int) : Bool :=
  match e.slot with
  | some s => s v
  | none => false

/-- The written set of class `c`: the union of the slot sets of the ends that
may return `c`, when each carries one and some end returns `c`. -/
def combineP (c : Int) (ends : List EndP) : Option (Int → Bool) :=
  if ends.all (fun e => !e.returns c || e.slot.isSome) && ends.any (fun e => e.returns c) then
    some (fun v => ends.any (fun e => e.returns c && slotHas e v))
  else none

/-- A run that took the path ending at `e`, returned `r` and left `v` in the
slot, where `e` is sound for it (it may return `r`, and its slot set, if any,
holds `v`), returns `c` only with `v` in the derived set. -/
theorem combine_sound (c : Int) (ends : List EndP) (W : Int → Bool)
    (hW : combineP c ends = some W) (e : EndP) (he : e ∈ ends) (r v : Int)
    (hr : e.returns r = true) (hv : ∀ s, e.slot = some s → s v = true) (hc : r = c) :
    W v = true := by
  unfold combineP at hW
  split at hW
  · rename_i hcond
    cases hW
    simp only [Bool.and_eq_true, List.all_eq_true, List.any_eq_true] at hcond
    have hall := hcond.1 e he
    subst hc
    simp only [hr, Bool.not_true, Bool.false_or, Option.isSome_iff_exists] at hall
    obtain ⟨s, hs⟩ := hall
    simp only [List.any_eq_true, Bool.and_eq_true]
    exact ⟨e, he, hr, by simp [slotHas, hs, hv s hs]⟩
  · cases hW

/-- A test's edge keeps the values it admits. -/
def narrowP (S : Int → Bool) (op : Cmp) (k : Int) (edge : Bool) : Int → Bool :=
  fun x => S x && (cmpHolds op x k == edge)

theorem narrow_sound (S : Int → Bool) (op : Cmp) (k : Int) (edge : Bool) (x : Int)
    (hx : S x = true) (ht : cmpHolds op x k = edge) : narrowP S op k edge x = true := by
  simp [narrowP, hx, ht]

/-- A `return ret` whose narrowed set excludes `c` never returns `c`. -/
theorem a_narrowing_without_the_class_constrains_nothing (S : Int → Bool) (op : Cmp)
    (k : Int) (edge : Bool) (c x : Int) (hc : narrowP S op k edge c = false)
    (hx : narrowP S op k edge x = true) : x ≠ c := by
  intro h
  subst h
  rw [hc] at hx
  cases hx

/-- Resolution over meanings: the call's result `r` lies in the narrowed set
`within`, which holds exactly one class `a`; the callee leaves its out
element `v` in `W` whenever it returns `a`. Then `v` is in `W`. -/
theorem resolve_sound (within : Int → Bool) (a r v : Int) (W : Int → Bool)
    (hsingle : ∀ x, within x = true → x = a) (hr : within r = true)
    (hcallee : r = a → W v = true) : W v = true :=
  hcallee (hsingle r hr)

/-- An empty narrowing admits no result: the path cannot occur. -/
theorem an_empty_narrowing_is_infeasible (within : Int → Bool) (r : Int)
    (hempty : ∀ x, within x = false) : within r ≠ true := by
  rw [hempty r]
  decide

/-- A store through a pointer that may alias the slot. -/
def afterStoreP (slot : Option (Int → Bool)) (stored : Int → Bool) : Option (Int → Bool) :=
  slot.map (fun s v => s v || stored v)

/-- Whether the store reached the slot or not, its final value is in the
joined set. -/
theorem after_store_sound (s stored : Int → Bool) (old new : Int) (aliased : Bool)
    (hs : s old = true) (hw : stored new = true) (t : Int → Bool)
    (ht : afterStoreP (some s) stored = some t) :
    t (if aliased then new else old) = true := by
  simp only [afterStoreP, Option.map_some, Option.some.injEq] at ht
  subst ht
  cases aliased <;> simp [hs, hw]

/-- A written set of class `c` over a function's runs `(result, slot value)`. -/
def WrittenOn (runs : Int × Int → Prop) (c : Int) (W : Int → Bool) : Prop :=
  ∀ r v, runs (r, v) → r = c → W v = true

/-- Forwarding `return G(.., P, ..)`: every run of `F` is a run of `G` (the
same result, `G`'s write of the slot last), so `G`'s written set is `F`'s. -/
theorem forwarding_sound (runsF runsG : Int × Int → Prop) (c : Int) (W : Int → Bool)
    (hsub : ∀ p, runsF p → runsG p) (hG : WrittenOn runsG c W) : WrittenOn runsF c W :=
  fun r v hrun hc => hG r v (hsub (r, v) hrun) hc

/-! ## The interval implementation (the Rust `combine`, `resolve`) -/

/-- A path end over intervals. -/
abbrev PathEnd := List Interval × Option (List Interval)

inductive Outcome where
  | ok (s : List Interval)
  | unwritten
  | noReturn
  deriving DecidableEq, Repr

def combineFrom (c : Int) : List PathEnd → Option (List Interval) → Outcome
  | [], none => .noReturn
  | [], some acc => .ok acc
  | (returns, slot) :: rest, acc =>
    if memI returns c then
      match slot with
      | none => .unwritten
      | some s =>
        let s := union INT [] s
        combineFrom c rest (some (match acc with
          | none => s
          | some a => union INT a s))
    else combineFrom c rest acc

/-- Rust `combine`. -/
def combine (c : Int) (ends : List PathEnd) : Outcome := combineFrom c ends none

inductive Resolution where
  | infeasible
  | known (s : List Interval)
  | unknown
  deriving DecidableEq, Repr

/-- Rust `resolve`: the narrowed result within the callee's result set. -/
def resolve (narrowed results : List Interval) (written : Int → Option (List Interval)) :
    Resolution :=
  match inter INT narrowed results with
  | [] => .infeasible
  | [(a, b)] =>
    if a == b then
      match written a with
      | some w => .known w
      | none => .unknown
    else .unknown
  | _ => .unknown

/-! ## The declared bound decides, not POSIX -/

/-- `[1, MAX]`, the declared bound, and POSIX's `[0, MAX]`. -/
def fromOne : List Interval := [(1, 2147483647)]
def fromZero : List Interval := [(0, 2147483647)]

/-- The epoll chain's three exits under a pipe written set `w`. -/
def epollEnds (w : List Interval) : List PathEnd :=
  [([(-1, -1)], none), ([(-1, -1)], none), ([(0, 0)], some w)]

/-- Inside: every interval of `a` lies in one of `b` (enough for these
normalised sets). -/
def within (a b : List Interval) : Bool :=
  a.all (fun i => b.any (fun j => decide (j.1 ≤ i.1 ∧ i.2 ≤ j.2)))

theorem the_declared_bound_admits_the_creation :
    (match combine 0 (epollEnds fromOne) with
      | .ok s => within s fromOne
      | _ => false) = true := by
  decide +kernel

theorem posix_s_set_derives_descriptor_zero :
    (match combine 0 (epollEnds fromZero) with
      | .ok s => memI s 0 && !within s fromOne
      | _ => false) = true := by
  decide +kernel

/-! ## Vectors (shared with the Rust, entry for entry) -/

/-- `(path ends, class, outcome)`: Rust `COMBINE_VECTORS`. -/
def combineVectors : List (List PathEnd × Int × Outcome) := [
  ([([(-1, -1)], none), ([(-1, -1)], none), ([(0, 0)], some [(1, 2147483647)])], 0, .ok [(1, 2147483647)]),
  ([([(-1, 0)], some [(1, 2147483647)])], 0, .ok [(1, 2147483647)]),
  ([([(-1, 0)], none), ([(0, 0)], some [(1, 2147483647)])], 0, .unwritten),
  ([([(0, 0)], some [(-2147483648, 2147483647)])], 0, .ok [(-2147483648, 2147483647)]),
  ([([(0, 0)], some [(1, 2147483647)]), ([(0, 0)], some [(-1, -1)])], 0, .ok [(-1, -1), (1, 2147483647)]),
  ([([(-1, -1)], none)], 0, .noReturn),
  ([([(0, 0)], some [(0, 2147483647)])], 0, .ok [(0, 2147483647)])
]

theorem combine_vectors_hold :
    combineVectors.all (fun (ends, c, o) => combine c ends == o) = true := by
  decide +kernel

/-- `(narrowed, callee result set, stated class, its written set, resolution)`:
Rust `RESOLVE_VECTORS`. -/
def resolveVectors : List (List Interval × List Interval × Int × List Interval × Resolution) := [
  ([(0, 2147483647)], [(-1, 0)], 0, [(1, 2147483647)], .known [(1, 2147483647)]),
  ([(-2147483648, -1)], [(-1, 0)], 0, [(1, 2147483647)], .unknown),
  ([(-2147483648, 2147483647)], [(-1, 0)], 0, [(1, 2147483647)], .unknown),
  ([(1, 2147483647)], [(-1, 0)], 0, [(1, 2147483647)], .infeasible),
  ([(0, 0)], [(-1, 0)], -1, [(1, 2147483647)], .unknown)
]

theorem resolve_vectors_hold :
    resolveVectors.all (fun (n, rs, c, w, o) =>
      resolve n rs (fun x => if x == c then some w else none) == o) = true := by
  decide +kernel

end QCore.WrittenSetDerivation
