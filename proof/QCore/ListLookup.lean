import Init.Data.List.Lemmas

/-!
The exact read-only child/next frame computes an indexed non-owning result.
The source reader and Rust/Lean correspondence are maintained boundary-28
premises. A coherent finite concrete/list representation is a separate named
premise at callers. This model does not establish that premise, create an
identity, transfer custody, or turn borrowed provenance into membership.
-/
namespace QCore.ListLookup

def lookup : List Nat → Nat → Option Nat
  | [], _ => none
  | x :: _, 0 => some x
  | _ :: xs, n + 1 => lookup xs n

theorem empty_is_null (n : Nat) : lookup [] n = none := rfl

theorem head_is_the_same_identity (x : Nat) (xs : List Nat) :
    lookup (x :: xs) 0 = some x := rfl

theorem nonnull_is_a_member (xs : List Nat) (n item : Nat)
    (h : lookup xs n = some item) : item ∈ xs := by
  induction xs generalizing n with
  | nil => simp [lookup] at h
  | cons x xs ih =>
    cases n with
    | zero => simp [lookup] at h; simp [h]
    | succ n => exact List.mem_cons_of_mem x (ih n h)

theorem out_of_range_is_null (xs : List Nat) (n : Nat)
    (h : xs.length ≤ n) : lookup xs n = none := by
  induction xs generalizing n with
  | nil => rfl
  | cons x xs ih =>
    cases n with
    | zero => simp at h
    | succ n => exact ih n (Nat.le_of_succ_le_succ h)

-- Executable correspondence controls: empty, head, middle, tail, missing.
def vectors : List (List Nat × Nat × Option Nat) := [
  ([], 0, none),
  ([10], 0, some 10),
  ([10], 1, none),
  ([10, 20, 30], 0, some 10),
  ([10, 20, 30], 1, some 20),
  ([10, 20, 30], 2, some 30),
  ([10, 20, 30], 3, none)
]
theorem vectors_hold : vectors.all (fun (xs, n, r) => lookup xs n == r) = true := by decide

/-- Unknown links and checked NULL are distinct; each visited item must have
    independently checked membership/lifetime and no repeated identity. -/
def walk (n : Nat) (head : Option (Option Nat))
    (next : Nat → Option (Option Nat)) (valid : Nat → Bool)
    (seen : List Nat := []) : Option (Option Nat) :=
  match head with
  | none => none
  | some none => some none
  | some (some item) =>
    if valid item && !seen.contains item then
      match n with
      | 0 => some (some item)
      | k + 1 => walk k (next item) next valid (item :: seen)
    else none

theorem refined_nonnull_needs_valid_member (n : Nat)
    (head : Option (Option Nat)) (next : Nat → Option (Option Nat))
    (valid : Nat → Bool) (seen : List Nat) (item : Nat)
    (h : walk n head next valid seen = some (some item)) : valid item = true := by
  induction n generalizing head seen with
  | zero =>
    cases head with
    | none => simp [walk] at h
    | some head =>
      cases head with
      | none => simp [walk] at h
      | some x =>
        simp [walk] at h
        exact h.2 ▸ h.1.1
  | succ n ih =>
    cases head with
    | none => simp [walk] at h
    | some head =>
      cases head with
      | none => simp [walk] at h
      | some x =>
        simp [walk] at h
        exact ih (next x) (x :: seen) h.2

theorem unknown_head_never_creates_identity (n : Nat) (next) (valid) (seen) :
    walk n none next valid seen = none := by cases n <;> rfl

theorem explicit_null_stays_null (n : Nat) (next) (valid) (seen) :
    walk n (some none) next valid seen = some none := by cases n <;> rfl

/-- The concrete/list relation states link values, live membership and no
    repeated object. It states no conclusion about any indexed lookup. -/
def Represents : List Nat → Option (Option Nat) →
    (Nat → Option (Option Nat)) → (Nat → Bool) → List Nat → Prop
  | [], head, _, _, _ => head = some none
  | x :: xs, head, next, valid, seen =>
    head = some (some x) ∧ valid x = true ∧ x ∉ seen ∧
      Represents xs (next x) next valid (x :: seen)

theorem coherent_prefix_selects_the_same_list_identity (xs : List Nat) (n : Nat)
    (head : Option (Option Nat)) (next : Nat → Option (Option Nat))
    (valid : Nat → Bool) (seen : List Nat)
    (rep : Represents xs head next valid seen) :
    walk n head next valid seen = some (lookup xs n) := by
  induction xs generalizing n head seen with
  | nil =>
    simp only [Represents] at rep
    subst head
    cases n <;> simp [walk, lookup]
  | cons x xs ih =>
    rcases rep with ⟨same, live, fresh, tail⟩
    subst head
    cases n with
    | zero => simp [walk, lookup, live, fresh]
    | succ n => simpa [walk, lookup, live, fresh] using ih n (next x) (x :: seen) tail


def nextOf : List Nat → Nat → Option (Option Nat)
  | [], _ => none
  | x :: xs, item => if x == item then some xs.head? else nextOf xs item

theorem physical_vectors_match_indexed_lookup :
    vectors.all (fun (xs, n, r) =>
      walk n (some xs.head?) (nextOf xs) xs.contains [] == some r) = true := by decide

end QCore.ListLookup
