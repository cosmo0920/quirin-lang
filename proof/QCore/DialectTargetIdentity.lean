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

import Std

namespace QCore.DialectTargetIdentity

/-! Dialect, target and type identities of the C frontend (card
CG30-DIALECTS-FLAGS-TYPES, gateway rule 11y;
`crates/qcore-c-gateway/src/c_frontend_identity.rs`).

The card's claims, each proved here over a model of the Rust decisions:

* **Identity separation.** Each dialect has its own identity bytes, and the
  four baseline dialects keep exactly their one historical byte
  (`dialectByte`, `distinct_dialect_distinct_bytes`,
  `baseline_bytes_unchanged`). An evidence key carries dialect, target and ABI;
  a cache answers a key only with a verdict minted under that exact key, so a
  verdict reached under one dialect or target is never reused under another
  (`lookup_minted_under_key`, `no_reuse_across_dialects`,
  `no_reuse_across_targets`). The Rust digest realises the key by hashing it;
  SHA-256 collision freedom is the stated premise, not proved.
* **Target admission.** Only the readers' LP64 premise (or no explicit
  selection) is admitted; an ABI fixing ILP32 is refused whatever the triple
  (`admitDataModel`, `admit_iff`, `ilp32_abi_refused`).
* **Withdrawal.** A definition spelling a registered, unadopted difference is
  analysed as one uncontracted call: never `Safe`, never `Unsafe`
  (`withdrawn_is_unknown`), whatever the body was.
* **Extension refusal.** Adding the named refusal joins `Unknown` into the
  row: it is never `Safe`, keeps a validated `Unsafe`, and only moves a row up
  the order (`refusal_never_safe`, `refusal_keeps_unsafe`,
  `refusal_is_monotone`).
* **One type, one identity.** The canonical type key of a specifier multiset
  is invariant under reordering and reads C23's `bool` as `_Bool`
  (`typeId`, `perm_same_typeId`, `bool_is_Bool`, `distinct_counts_distinct`).

Correspondence with the Rust implementation is maintained by review and by the
shared vectors named in `tests/unit/c_frontend_identity.rs`; it is not an
extraction theorem (constraint 28). -/

-- ---------------------------------------------------------------------------
-- Dialects
-- ---------------------------------------------------------------------------

inductive Dialect where
  | c11 | gnu99 | gnu11 | gnu17 | c17 | c23 | c2y
  deriving DecidableEq, Repr

/-- `dialect_identity_bytes`' first byte. -/
def dialectByte : Dialect → Nat
  | .c11 => 0 | .gnu99 => 1 | .gnu11 => 2 | .gnu17 => 3
  | .c17 => 4 | .c23 => 5 | .c2y => 6

theorem dialectByte_injective {a b : Dialect} (h : dialectByte a = dialectByte b) :
    a = b := by
  cases a <;> cases b <;> simp [dialectByte] at h ⊢

/-- Registered differences, keys interned as naturals (C2y extends C23). -/
def c23Keys : List Nat := List.range 17
def c2yKeys : List Nat := (List.range 6).map (· + 100)

def registry : Dialect → List Nat
  | .c23 => c23Keys
  | .c2y => c23Keys ++ c2yKeys
  | _ => []

/-- The registry marker (`qcore.c-dialect-differences.v1`), interned. -/
def registryMarker : Nat := 1000

def identityBytes (d : Dialect) : List Nat :=
  dialectByte d :: (if registry d = [] then [] else registryMarker :: registry d)

theorem distinct_dialect_distinct_bytes {a b : Dialect} (h : a ≠ b) :
    identityBytes a ≠ identityBytes b := by
  intro equal
  apply h
  apply dialectByte_injective
  have := congrArg List.head? equal
  simpa [identityBytes] using this

theorem baseline_bytes_unchanged (d : Dialect)
    (h : d = .c11 ∨ d = .gnu99 ∨ d = .gnu11 ∨ d = .gnu17 ∨ d = .c17) :
    identityBytes d = [dialectByte d] := by
  rcases h with h | h | h | h | h <;> subst h <;> rfl

theorem c2y_extends_c23 : ∀ k ∈ registry .c23, k ∈ registry .c2y := by
  intro k hk
  simp [registry] at hk ⊢
  exact Or.inl hk

-- ---------------------------------------------------------------------------
-- Evidence keys and reuse
-- ---------------------------------------------------------------------------

inductive Verdict where
  | safe | pending | unknown | violated  -- `violated`: a validated `Unsafe`
  deriving DecidableEq, Repr

def rank : Verdict → Nat
  | .safe => 0 | .pending => 1 | .unknown => 2 | .violated => 3

/-- The join of `Safe < Pending < Unknown < Unsafe` (§L21.2). -/
def join (a b : Verdict) : Verdict := if rank a ≤ rank b then b else a

/-- The evidence identity: dialect, explicit target and ABI (interned), and
the rest of the request (interned). -/
structure Key where
  dialect : Dialect
  target : Option Nat
  abi : Option Nat
  request : Nat
  deriving DecidableEq, Repr

/-- A cache of minted verdicts, answered by exact key. -/
def lookup (cache : List (Key × Verdict)) (k : Key) : Option Verdict :=
  (cache.find? (fun entry => entry.1 = k)).map Prod.snd

theorem lookup_minted_under_key {cache : List (Key × Verdict)} {k : Key} {v : Verdict}
    (h : lookup cache k = some v) : (k, v) ∈ cache := by
  unfold lookup at h
  cases hf : cache.find? (fun entry => entry.1 = k) with
  | none => simp [hf] at h
  | some entry =>
    simp [hf] at h
    have hmem := List.mem_of_find?_eq_some hf
    have hkey : entry.1 = k := by simpa using List.find?_some hf
    have : entry = (k, v) := by
      cases entry
      simp_all
    exact this ▸ hmem

theorem no_reuse_across_dialects {cache : List (Key × Verdict)} {k : Key}
    (h : ∀ entry ∈ cache, entry.1.dialect ≠ k.dialect) : lookup cache k = none := by
  cases hl : lookup cache k with
  | none => rfl
  | some v =>
    exact absurd rfl (h (k, v) (lookup_minted_under_key hl))

theorem no_reuse_across_targets {cache : List (Key × Verdict)} {k : Key}
    (h : ∀ entry ∈ cache, entry.1.target ≠ k.target ∨ entry.1.abi ≠ k.abi) :
    lookup cache k = none := by
  cases hl : lookup cache k with
  | none => rfl
  | some v =>
    rcases h (k, v) (lookup_minted_under_key hl) with hne | hne <;> exact absurd rfl hne

-- ---------------------------------------------------------------------------
-- Target admission
-- ---------------------------------------------------------------------------

inductive DataModel where
  | lp64 | llp64 | ilp32
  deriving DecidableEq, Repr

/-- `CTargetIdentity::data_model`: an ABI fixing a model overrides the triple's. -/
def effectiveModel (triple abi : Option DataModel) : Option DataModel :=
  match abi with
  | some model => some model
  | none => triple

/-- `CTargetIdentity::admit`: no explicit model (the default target, the
readers' standing premise) or LP64. -/
def admitDataModel : Option DataModel → Bool
  | none => true
  | some .lp64 => true
  | some _ => false

theorem admit_iff (m : Option DataModel) :
    admitDataModel m = true ↔ m = none ∨ m = some .lp64 := by
  cases m with
  | none => simp [admitDataModel]
  | some model => cases model <;> simp [admitDataModel]

theorem ilp32_abi_refused (triple : Option DataModel) :
    admitDataModel (effectiveModel triple (some .ilp32)) = false := rfl

theorem llp64_target_refused :
    admitDataModel (effectiveModel (some .llp64) none) = false := rfl

-- ---------------------------------------------------------------------------
-- Withdrawal of a definition spelling a registered difference
-- ---------------------------------------------------------------------------

/-- What route G reads a statement as: its verdict contribution. -/
inductive Stmt where
  | read (v : Verdict)
  | uncontractedCall
  deriving DecidableEq, Repr

def stmtVerdict : Stmt → Verdict
  | .read v => v
  | .uncontractedCall => .unknown

def analyze (body : List Stmt) : Verdict :=
  body.foldl (fun acc s => join acc (stmtVerdict s)) .safe

/-- `refuse_dialect_differences`: the body becomes one marked call. -/
def withdraw (_body : List Stmt) : List Stmt := [.uncontractedCall]

theorem withdrawn_is_unknown (body : List Stmt) : analyze (withdraw body) = .unknown := rfl

-- ---------------------------------------------------------------------------
-- Extension refusal
-- ---------------------------------------------------------------------------

/-- `name_refusals`: an `Unsupported` diagnostic joins `Unknown` into the row. -/
def addRefusal (v : Verdict) : Verdict := join v .unknown

theorem refusal_never_safe (v : Verdict) : addRefusal v ≠ .safe := by
  cases v <;> simp [addRefusal, join, rank]

theorem refusal_keeps_unsafe : addRefusal .violated = .violated := rfl

theorem refusal_is_monotone (v : Verdict) : rank v ≤ rank (addRefusal v) := by
  cases v <;> simp [addRefusal, join, rank]

-- ---------------------------------------------------------------------------
-- Canonical type identity
-- ---------------------------------------------------------------------------

inductive Word where
  | signed | unsigned | char | short | int | long | float | double
  | boolKw | bool | void | int128
  deriving DecidableEq, Repr

/-- C23's `bool` is `_Bool` (`CTypeId` reads it so). -/
def norm : Word → Word
  | .bool => .boolKw
  | w => w

def countedWords : List Word :=
  [.signed, .unsigned, .char, .short, .int, .long, .float, .double, .boolKw, .void, .int128]

/-- The identity of a specifier multiset: its normalised counts. -/
def typeId (l : List Word) : List Nat :=
  countedWords.map (fun w => (l.map norm).count w)

theorem perm_same_typeId {l₁ l₂ : List Word} (h : l₁.Perm l₂) : typeId l₁ = typeId l₂ := by
  unfold typeId
  have hp : (l₁.map norm).Perm (l₂.map norm) := h.map norm
  congr 1
  funext w
  exact hp.count_eq w

theorem bool_is_Bool : typeId [.bool] = typeId [.boolKw] := rfl

/-- `long unsigned int` and `unsigned long` are one identity; `long` and
`long long` are two. -/
theorem long_unsigned_int_vector :
    typeId [.long, .unsigned, .int] = typeId [.unsigned, .long, .int] := by decide

theorem distinct_counts_distinct : typeId [.long] ≠ typeId [.long, .long] := by decide

end QCore.DialectTargetIdentity
