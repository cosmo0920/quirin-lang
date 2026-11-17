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

import QCore.VerdictSemigroup

/-!
# Library version ranges and the OpenSSL plugin's premises

Card **OPENSSL-PROVIDER-PLUGIN** (user decisions of 2026-11-16; contract
specification §C06.2, "Library version ranges"). This is the model of
`crates/qcore-schemas/src/library_version.rs` (versions, ranges, `admits`),
`crates/qcore-audit/src/library_version.rs` (`decide`) and the gateway's hold
(`legacy_pin_hold::hold_description_for`).

## What is proved

* **The in-force predicate.** A declaration stating a range is in force
  exactly when the audited headers' version was read (readable and
  unambiguous), lies in `[lower, removed)`, and equals the version the lock
  pins (`decide_inForce_iff`). Below the lower bound, at or past the removal,
  unreadable, or different from the pin, it is not in force.
* **A contract out of range discharges nothing.** A declaration not in force
  holds every call relying on it at `Pending` (a version that only differs
  from the pin: review and re-pin) or `Unknown` (out of range, unreadable,
  ambiguous), never `Safe`, so no chain containing such a call is `Safe`
  (`not_in_force_never_safe`, `held_call_keeps_chain_non_safe`); the split is
  exactly the user's (`changed_is_pending`, `range_failure_is_unknown`).
* **The OpenSSL premises.** A release of a reference-counted object (X509,
  EVP_PKEY, SSL, SSL_CTX, BIO, ENGINE) is a counted drop: it frees the object
  only when the drop takes the count to zero, so with another reference
  outstanding the object survives (`counted_release_is_not_final`), and the
  drop of the last reference does free it (`last_drop_frees`); a reading of
  the drop as a final free is refuted by a reachable state
  (`final_free_reading_is_unsound`). A NULL-accepting release of NULL changes
  nothing (`null_release_is_noop`), and a NULL actual is never a held
  reference, so it discharges no obligation (`null_drop_discharges_nothing`).

## What this is not

That Clang reports the macros' values, that the official documentation's
HISTORY sections are accurate, and that OpenSSL implements its documented
reference counting, are premises. That the Rust functions compute these is
pinned by their own tests (`library_version.rs` in `qcore-schemas` and
`qcore-audit`, `quirin-provider-openssl`'s checker tests).
-/

namespace QCore.LibraryVersionRange

open QCore.VerdictSemigroup

/-- A library version: three numeric components. -/
structure Version where
  major : Nat
  minor : Nat
  patch : Nat
  deriving DecidableEq, Repr

namespace Version

/-- The lexicographic order. -/
def lt (a b : Version) : Bool :=
  a.major < b.major ||
    (a.major == b.major && (a.minor < b.minor || (a.minor == b.minor && a.patch < b.patch)))

def le (a b : Version) : Bool := a == b || lt a b

theorem le_refl (a : Version) : le a a = true := by
  simp [le]

theorem lt_irrefl (a : Version) : lt a a = false := by
  simp [lt]

end Version

/-- A declaration's range: an inclusive lower bound (`since` or `documented`)
and an optional exclusive removal. -/
structure Bound where
  lower : Version
  removed : Option Version

/-- `LibraryVersionBound::admits`. -/
def admits (b : Bound) (v : Version) : Bool :=
  Version.le b.lower v &&
    match b.removed with
    | none => true
    | some r => Version.lt v r

/-- What the audit decides for one declaration (`LibraryVersionFailure`, or
in force). -/
inductive Decision where
  | inForce
  | changed
  | outOfRange
  | unreadable
  deriving DecidableEq, Repr

/-- `qcore_audit::library_version::decide`. `probed` is `none` when the
version is unreadable or ambiguous (both are held as `Unknown`). -/
def decide (b : Bound) (probed : Option Version) (pinned : Option Version) : Decision :=
  match probed with
  | none => .unreadable
  | some v =>
    if admits b v then
      if pinned = some v then .inForce else .changed
    else .outOfRange

/-- The verdict a call relying on a withheld declaration contributes
(`hold_description_for`): `Pending` for a changed version, `Unknown`
otherwise. An in-force declaration holds nothing; the call then contributes
whatever the analysis of its contract decides, modelled as `analysed`. -/
def held (analysed : V) : Decision → V
  | .inForce => analysed
  | .changed => .pending
  | .outOfRange => .unknown
  | .unreadable => .unknown

theorem decide_inForce_iff (b : Bound) (probed pinned : Option Version) :
    decide b probed pinned = .inForce ↔
      ∃ v, probed = some v ∧ admits b v = true ∧ pinned = some v := by
  cases probed with
  | none => simp [decide]
  | some v =>
    by_cases h : admits b v = true
    · by_cases hp : pinned = some v
      · simp [decide, h, hp]
      · simp [decide, h, hp]
    · simp [decide, h]

theorem below_lower_not_in_force (b : Bound) (v : Version) (pinned : Option Version)
    (h : Version.le b.lower v = false) : decide b (some v) pinned = .outOfRange := by
  simp [decide, admits, h]

theorem past_removal_not_in_force (b : Bound) (v r : Version) (pinned : Option Version)
    (hr : b.removed = some r) (h : Version.lt v r = false) :
    decide b (some v) pinned = .outOfRange := by
  simp [decide, admits, hr, h]

theorem unreadable_not_in_force (b : Bound) (pinned : Option Version) :
    decide b none pinned = .unreadable := rfl

theorem pin_mismatch_not_in_force (b : Bound) (v w : Version)
    (hin : admits b v = true) (hne : w ≠ v) :
    decide b (some v) (some w) = .changed := by
  have : ¬ (some w = some v) := by simp [hne]
  simp [decide, hin, this]

theorem in_range_and_pinned_in_force (b : Bound) (v : Version) (hin : admits b v = true) :
    decide b (some v) (some v) = .inForce := by
  simp [decide, hin]

/-- A declaration not in force discharges nothing: whatever the analysis
would have said, the call is never `Safe`. -/
theorem not_in_force_never_safe (analysed : V) (d : Decision) (h : d ≠ .inForce) :
    held analysed d ≠ .safe := by
  cases d <;> simp_all [held]

theorem changed_is_pending (analysed : V) : held analysed .changed = .pending := rfl

theorem range_failure_is_unknown (analysed : V) :
    held analysed .outOfRange = .unknown ∧ held analysed .unreadable = .unknown :=
  ⟨rfl, rfl⟩

/-- No chain containing a call held for a range failure or a version change
is `Safe`. -/
theorem held_call_keeps_chain_non_safe (analysed : V) (d : Decision) (h : d ≠ .inForce)
    (rest : List V) : fold (held analysed d :: rest) ≠ .safe := by
  intro hfold
  have hle := every_link_is_below_the_fold
    (x := held analysed d) (xs := held analysed d :: rest) (List.mem_cons_self ..)
  rw [hfold] at hle
  have hz : (held analysed d).rank = 0 := Nat.le_zero.mp hle
  exact not_in_force_never_safe analysed d h (V.rank_injective (b := .safe) hz)

/-! ## The hold under a reduced environment

Card **LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING**. The gateway rewrites a
withheld callee's call sites first, then classifies the held call by the
withheld entry it finds in the environment it reads
(`hold_description_for`): a range failure is `Unknown`, anything else, or no
entry, the human-resolvable mismatch, `Pending`. A unit whose ABI gate
withdraws some declaration is classified under a rebuilt, reduced environment
(`AbiAgreement::withdraw_environment`). A reduction that drops the withheld
table publishes a range failure `Pending` (`dropped_table_lifts_a_range_failure`,
the defect); one that carries it (`carry_pin_mismatch`) classifies every held
call exactly as the unreduced environment does
(`carried_reduction_keeps_the_verdict`, `carried_reduction_agrees_with_held`).
A checker binding's claim is joined onto the row, so it never lowers a held
call, and a `Pending` claim leaves a range failure `Unknown`. -/

/-- What the classifier finds for a held callee: its withheld entry's
decision, or no entry. -/
abbrev Withheld := Option Decision

/-- `hold_description_for`, read as the verdict its description carries. -/
def heldBy : Withheld → V
  | some .outOfRange => .unknown
  | some .unreadable => .unknown
  | _ => .pending

/-- An environment's withheld table, by callee, after a reduction that
withdraws other declarations: kept when `carry`, dropped otherwise. -/
def reduce (carry : Bool) (table : String → Withheld) : String → Withheld :=
  if carry then table else fun _ => none

/-- On a held call (any decision but in force) the classifier's verdict is
the split's. -/
theorem heldBy_agrees_with_held (analysed : V) (d : Decision) (h : d ≠ .inForce) :
    heldBy (some d) = held analysed d := by
  cases d <;> simp_all [heldBy, held]

/-- The defect: a reduction that drops the table publishes an unreadable
version's hold `Pending`. -/
theorem dropped_table_lifts_a_range_failure :
    heldBy (reduce false (fun _ => some .unreadable) "f") = .pending := rfl

/-- The fix: a reduction that carries the table classifies as the unreduced
environment does. -/
theorem carried_reduction_keeps_the_verdict (table : String → Withheld) (f : String) :
    heldBy (reduce true table f) = heldBy (table f) := rfl

theorem carried_reduction_agrees_with_held (table : String → Withheld) (f : String)
    (analysed : V) (d : Decision) (hd : table f = some d) (h : d ≠ .inForce) :
    heldBy (reduce true table f) = held analysed d := by
  rw [carried_reduction_keeps_the_verdict, hd]
  exact heldBy_agrees_with_held analysed d h

/-- A checker binding's claim is joined onto the held call's verdict: it
never lowers it. -/
theorem binding_never_lowers_a_hold (analysed claim : V) (d : Decision) :
    (held analysed d).rank ≤ (V.join (held analysed d) claim).rank :=
  le_join_left _ _

/-- A binding's `Pending` claim leaves a range failure `Unknown`, and a
changed version `Pending`. -/
theorem pending_binding_keeps_the_split (analysed : V) :
    V.join (held analysed .outOfRange) .pending = .unknown ∧
      V.join (held analysed .unreadable) .pending = .unknown ∧
      V.join (held analysed .changed) .pending = .pending := by
  exact ⟨rfl, rfl, rfl⟩

/-! ## The OpenSSL plugin's premises -/

/-- A reference-counted object: its count, and whether it was freed. -/
structure Counted where
  count : Nat
  freed : Bool
  deriving DecidableEq, Repr

/-- `X509_free` / `EVP_PKEY_free` / `SSL_free` / `SSL_CTX_free` / `BIO_free` /
`ENGINE_free` on a non-NULL object, as documented: decrement, and free when
the count reaches zero. -/
def drop (o : Counted) : Counted :=
  { count := o.count - 1, freed := o.count - 1 == 0 }

theorem counted_release_is_not_final (o : Counted) (h : 2 ≤ o.count) :
    (drop o).freed = false ∧ 1 ≤ (drop o).count := by
  refine ⟨?_, ?_⟩
  · simp [drop]; omega
  · simp [drop]; omega

theorem last_drop_frees (o : Counted) (h : o.count = 1) : (drop o).freed = true := by
  simp [drop, h]

/-- Reading a counted release as a final free is unsound: there is a state
where the object survives the release. -/
theorem final_free_reading_is_unsound : ∃ o : Counted, (drop o).freed = false :=
  ⟨⟨2, false⟩, by decide⟩

/-- A NULL-accepting release on an optional object: NULL does nothing. -/
def release : Option Counted → Option Counted
  | none => none
  | some o => some (drop o)

theorem null_release_is_noop : release none = none := rfl

/-- The references a function holds after a drop through `p`: a NULL actual
is never a held reference, so it discharges nothing (the gateway refuses a
possibly-NULL actual at a reference drop, rule 13b). -/
def heldAfter (held : Nat) : Option Counted → Nat
  | none => held
  | some _ => held - 1

theorem null_drop_discharges_nothing (held : Nat) : heldAfter held none = held := rfl

end QCore.LibraryVersionRange
