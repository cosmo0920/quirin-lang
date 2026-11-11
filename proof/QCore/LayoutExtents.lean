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
# Layout-derived extents (card CG17-LAYOUT-DERIVED-EXTENTS, gateway rule 11u)

The abstract rules implemented by `crates/qcore-c-gateway/src/layout_extents.rs`
(`flexible_extent`, `alignment`, `container_of`, `vla_decision`,
`alignment_verdict`) and the route-G adapter `legacy_layout_extents.rs`.

Four source-derived layout facts, each from Clang's own layout values (never a
host computation):

* **Flexible array member** (M13). The remaining extent of `T data[]` in an
  allocation of `size` bytes is `(size - offsetof(data)) / sizeof(T)` elements,
  and only when the allocation size is established. Every counted element lies
  inside the allocation, and the count is the largest that does.
* **Variable-length array** (M12). The declaration evaluates its bound once,
  before the array's lifetime starts, carries the numeric obligation that the
  bound is positive (C11 6.7.6.2p5; CG20 owns numeric discharge), and the
  lifetime ends at the block's exit. An address that leaves the block is
  dangling, so a VLA whose address escapes is never admitted.
* **Alignment** (M19). A conversion to a pointer whose referenced type needs
  alignment `r` (C11 6.3.2.3p7) is *aligned* only when the derived address's
  base alignment and offset are multiples of `r`, *misaligned* only when the
  base alignment is a multiple of `r` and the constant offset is not, and
  otherwise undecided. The property is `Safe` only when every obligation is
  aligned; a misaligned obligation is a provisional candidate (no reachability
  validation here), so it joins to `Unknown`, never to a published `Unsafe`.
* **container_of** (E18). `(T *)((char *)p - offsetof(T, m))` is the enclosing
  object only when `p` is established as pointing at member `m` of a `T`
  object; the address round trip then returns that object's address. Any other
  pointer is refused: the subtraction then names no object start.

## What this is not

A model of the decisions and of the address arithmetic they rest on. Faithful
Clang extraction of types, offsets, sizes, alignments and declaration ids is
trust boundary 28; that a pointer of type `T *` formed without undefined
behaviour is aligned for `T` is C11 6.3.2.3p7's, stated as
`P-CG17-POINTER-ALIGNED`, not proved.
-/

namespace QCore.LayoutExtents

/-! ## Flexible array member extent -/

inductive FlexRefusal where
  | sizeNotEstablished
  | belowMember
  | zeroWidth
  deriving DecidableEq, Repr

inductive FlexExtent where
  | elements (count : Nat)
  | refused (reason : FlexRefusal)
  deriving DecidableEq, Repr

/-- `layout_extents::flexible_extent`: the allocation size (if established),
the member's byte offset and the element width, all Clang's. -/
def flexibleExtent (size : Option Nat) (offset width : Nat) : FlexExtent :=
  match size with
  | none => .refused .sizeNotEstablished
  | some s =>
      if width = 0 then .refused .zeroWidth
      else if s < offset then .refused .belowMember
      else .elements ((s - offset) / width)

/-- Every counted element's bytes lie inside the allocation. -/
theorem every_counted_element_lies_inside (s offset width count i : Nat)
    (h : flexibleExtent (some s) offset width = .elements count) (hi : i < count) :
    offset + (i + 1) * width ≤ s := by
  unfold flexibleExtent at h
  by_cases hw : width = 0
  · simp [hw] at h
  · by_cases hs : s < offset
    · simp [hw, hs] at h
    · simp only [hw, hs, ite_false] at h
      injection h with h
      subst h
      have h1 : (i + 1) * width ≤ (s - offset) / width * width :=
        Nat.mul_le_mul_right width hi
      have h2 : (s - offset) / width * width ≤ s - offset := Nat.div_mul_le_self _ _
      omega

/-- The count is the largest that fits: one more element would leave the
allocation. -/
theorem the_count_is_maximal (s offset width count : Nat)
    (h : flexibleExtent (some s) offset width = .elements count) :
    s < offset + (count + 1) * width := by
  unfold flexibleExtent at h
  by_cases hw : width = 0
  · simp [hw] at h
  · by_cases hs : s < offset
    · simp [hw, hs] at h
    · simp only [hw, hs, ite_false] at h
      injection h with h
      subst h
      have hpos : 0 < width := Nat.pos_of_ne_zero hw
      have := Nat.lt_div_mul_add (a := s - offset) hpos
      rw [Nat.add_mul, Nat.one_mul]
      omega

/-- Without an established allocation size nothing is counted. -/
theorem an_unestablished_size_counts_nothing (offset width : Nat) :
    flexibleExtent none offset width = .refused .sizeNotEstablished := rfl

/-- The vectors, spelled as `layout_extents::FLEX_VECTORS`:
`(size, offset, width, answer)`. -/
def flexVectors : List (Option Nat × Nat × Nat × FlexExtent) :=
  [ (some 20, 4, 4, .elements 4)
  , (some 22, 4, 4, .elements 4)
  , (some 4, 4, 4, .elements 0)
  , (some 3, 4, 4, .refused .belowMember)
  , (some 40, 8, 0, .refused .zeroWidth)
  , (none, 4, 4, .refused .sizeNotEstablished)
  , (some 24, 8, 8, .elements 2) ]

theorem flex_vectors_hold :
    flexVectors.all (fun (s, o, w, a) => flexibleExtent s o w == a) = true := by
  decide

/-! ## Variable-length arrays -/

/-- The bound expression's evaluation class (Clang's node, read once). -/
inductive BoundClass where
  | readsOnly
  | effect
  | unread
  deriving DecidableEq, Repr

/-- One use of the array object in its function. -/
inductive VlaUse where
  /-- A subscript base whose element is read or stored as a scalar. -/
  | element
  /-- A `sizeof` operand. -/
  | sizeQuery
  /-- Any other use: a decay into an actual, a return, a store, `&a`, `&a[i]`,
  `a + i`. The address may leave the block. -/
  | address
  deriving DecidableEq, Repr

inductive VlaDecision where
  | admitted
  | elementNotScalar
  | boundEffect
  | boundUnread
  | addressEscapes
  deriving DecidableEq, Repr

/-- `layout_extents::vla_decision`. -/
def vlaDecision (scalarElement : Bool) (bound : BoundClass) (uses : List VlaUse) :
    VlaDecision :=
  if !scalarElement then .elementNotScalar
  else if bound = .effect then .boundEffect
  else if bound = .unread then .boundUnread
  else if uses.contains .address then .addressEscapes
  else .admitted

/-- The numeric obligations an admitted declaration carries, never
discharged here (CG20 owns numeric discharge). -/
inductive NumericObligation where
  /-- The bound's own arithmetic has no C fault case (signed overflow). -/
  | boundDefined
  /-- C11 6.7.6.2p5: the bound is greater than zero. -/
  | boundPositive
  /-- The array's byte size is representable (`bound * sizeof(T)`). -/
  | sizeRepresentable
  deriving DecidableEq, Repr

def obligationsOf : VlaDecision → List NumericObligation
  | .admitted => [.boundDefined, .boundPositive, .sizeRepresentable]
  | _ => []

theorem an_admitted_declaration_carries_the_bound_obligation (e : Bool) (b : BoundClass)
    (u : List VlaUse) (h : vlaDecision e b u = .admitted) :
    NumericObligation.boundPositive ∈ obligationsOf (vlaDecision e b u) := by
  rw [h]; simp [obligationsOf]

theorem an_escaping_address_is_never_admitted (e : Bool) (b : BoundClass) (u : List VlaUse)
    (h : VlaUse.address ∈ u) : vlaDecision e b u ≠ .admitted := by
  unfold vlaDecision
  cases e <;> cases b <;> simp [h]

theorem a_bound_with_an_effect_is_never_admitted (e : Bool) (u : List VlaUse) :
    vlaDecision e .effect u ≠ .admitted := by
  cases e <;> simp [vlaDecision]

theorem admitted_iff (e : Bool) (b : BoundClass) (u : List VlaUse) :
    vlaDecision e b u = .admitted ↔ e = true ∧ b = .readsOnly ∧ VlaUse.address ∉ u := by
  cases e <;> cases b <;> simp [vlaDecision]

/-! ### The block's events

A block holding one VLA declaration runs the bound's reads (each once, in
Clang's order), then the array's lifetime start, then the body, then the
lifetime end at every exit of the block. -/

inductive Event where
  | boundRead (name : Nat)
  | lifetimeStart
  | access
  | lifetimeEnd
  deriving DecidableEq, Repr

def declare (reads : List Nat) : List Event :=
  reads.map Event.boundRead ++ [Event.lifetimeStart]

def block (reads : List Nat) (accesses : Nat) : List Event :=
  declare reads ++ List.replicate accesses Event.access ++ [Event.lifetimeEnd]

theorem the_bound_is_read_once (reads : List Nat) (x : Nat) :
    (declare reads).count (.boundRead x) = reads.count x := by
  induction reads with
  | nil => simp [declare]
  | cons y ys ih =>
      by_cases h : y = x
      · subst h; simp_all [declare]
      · simp_all [declare]

theorem the_lifetime_starts_once (reads : List Nat) (accesses : Nat) :
    (block reads accesses).count .lifetimeStart = 1 := by
  simp [block, declare, List.count_append, List.count_replicate]
  induction reads with
  | nil => simp
  | cons y ys ih => simp [ih]

/-- Every access of the block lies inside the lifetime: after the start and
before the end. -/
theorem accesses_lie_inside_the_lifetime (reads : List Nat) (accesses : Nat) :
    ∃ pre post, block reads accesses =
      pre ++ [Event.lifetimeStart] ++ List.replicate accesses Event.access ++ post ∧
      post = [Event.lifetimeEnd] := by
  exact ⟨reads.map Event.boundRead, [Event.lifetimeEnd], by simp [block, declare], rfl⟩

/-- A store of the array's address that outlives the block observes the
lifetime's end: the address is dangling at every later use. A model with one
frame: an address recorded at the declaration is live only while the block
runs. -/
def liveAfter (events : List Event) : Bool :=
  events.foldl (fun live e => match e with
    | .lifetimeStart => true
    | .lifetimeEnd => false
    | _ => live) false

theorem the_address_is_dead_after_the_block (reads : List Nat) (accesses : Nat) :
    liveAfter (block reads accesses) = false := by
  simp [liveAfter, block, List.foldl_append]

/-- The vectors, spelled as `layout_extents::VLA_VECTORS`. -/
def vlaVectors : List (Bool × BoundClass × List VlaUse × VlaDecision) :=
  [ (true, .readsOnly, [], .admitted)
  , (true, .readsOnly, [.element, .sizeQuery], .admitted)
  , (true, .readsOnly, [.element, .address], .addressEscapes)
  , (true, .effect, [.element], .boundEffect)
  , (true, .unread, [], .boundUnread)
  , (false, .readsOnly, [.element], .elementNotScalar) ]

theorem vla_vectors_hold :
    vlaVectors.all (fun (e, b, u, d) => vlaDecision e b u == d) = true := by
  decide

/-! ## Alignment obligations -/

/-- A derived address's offset from its base: a constant, an unknown multiple
of a stride (pointer arithmetic by a non-constant index), or unknown. -/
inductive Offset where
  | const (k : Int)
  | scaled (stride : Nat)
  | unknown
  deriving DecidableEq, Repr

inductive Alignment where
  | aligned
  | misaligned
  | undecided
  deriving DecidableEq, Repr

/-- `layout_extents::alignment`: the referenced type's alignment `required`,
the base address's guaranteed alignment `base`, and the offset. -/
def alignment (required base : Nat) (offset : Offset) : Alignment :=
  if required = 0 then .undecided
  else if base % required ≠ 0 then .undecided
  else match offset with
    | .const k => if k % (required : Int) = 0 then .aligned else .misaligned
    | .scaled s => if s % required = 0 then .aligned else .undecided
    | .unknown => .undecided

/-- The offsets an `Offset` stands for. -/
def Offset.denotes : Offset → Int → Prop
  | .const k, d => d = k
  | .scaled s, d => ∃ j : Int, d = j * s
  | .unknown, _ => True

/-- An aligned answer holds at every base address of the guaranteed alignment
and every offset the derivation stands for. -/
theorem aligned_is_sound (r b : Nat) (o : Offset) (h : alignment r b o = .aligned)
    (a d : Int) (ha : (b : Int) ∣ a) (hd : o.denotes d) : (r : Int) ∣ a + d := by
  unfold alignment at h
  by_cases hr : r = 0
  · simp [hr] at h
  · by_cases hb : b % r ≠ 0
    · simp [hr, hb] at h
    · have hb' : b % r = 0 := by omega
      have hrb : (r : Int) ∣ (b : Int) := by
        exact Int.natCast_dvd_natCast.mpr (Nat.dvd_of_mod_eq_zero hb')
      have hra : (r : Int) ∣ a := Int.dvd_trans hrb ha
      cases o with
      | const k =>
          simp only [hr, hb', ite_false, ne_eq, not_true_eq_false] at h
          by_cases hk : k % (r : Int) = 0
          · simp only [Offset.denotes] at hd
            subst hd
            exact Int.dvd_add hra (Int.dvd_of_emod_eq_zero hk)
          · simp [hk] at h
      | scaled s =>
          simp only [hr, hb', ite_false, ne_eq, not_true_eq_false] at h
          by_cases hs : s % r = 0
          · obtain ⟨j, hj⟩ := hd
            subst hj
            have hrs : (r : Int) ∣ (s : Int) :=
              Int.natCast_dvd_natCast.mpr (Nat.dvd_of_mod_eq_zero hs)
            exact Int.dvd_add hra (Int.dvd_mul_of_dvd_right hrs)
          · simp [hs] at h
      | unknown => simp [hr, hb'] at h

/-- A misaligned answer is misaligned at every base address of the guaranteed
alignment: no base can repair a constant offset. -/
theorem misaligned_is_sound (r b : Nat) (k : Int) (h : alignment r b (.const k) = .misaligned)
    (a : Int) (ha : (b : Int) ∣ a) : ¬ (r : Int) ∣ a + k := by
  unfold alignment at h
  by_cases hr : r = 0
  · simp [hr] at h
  · by_cases hb : b % r ≠ 0
    · simp [hr, hb] at h
    · have hb' : b % r = 0 := by omega
      have hrb : (r : Int) ∣ (b : Int) :=
        Int.natCast_dvd_natCast.mpr (Nat.dvd_of_mod_eq_zero hb')
      have hra : (r : Int) ∣ a := Int.dvd_trans hrb ha
      simp only [hr, hb', ite_false, ne_eq, not_true_eq_false] at h
      by_cases hk : k % (r : Int) = 0
      · simp [hk] at h
      · intro hsum
        have : (r : Int) ∣ k := (Int.dvd_add_right hra).mp hsum
        exact hk (Int.emod_eq_zero_of_dvd this)

/-- An unknown offset or an under-aligned base is never decided. -/
theorem an_unknown_offset_is_undecided (r b : Nat) : alignment r b .unknown ≠ .aligned := by
  unfold alignment
  by_cases hr : r = 0
  · simp [hr]
  · by_cases hb : b % r ≠ 0
    · simp [hr, hb]
    · simp [hr, hb]

/-- The four-valued property verdict over a function's obligations. -/
inductive Verdict where
  | safe
  | unknown
  deriving DecidableEq, Repr

/-- `layout_extents::alignment_verdict`: `Safe` only when every obligation
is aligned. A misaligned one is a provisional candidate with no reachability
validation, so it joins to `Unknown`, as an undecided one does. -/
def alignmentVerdict (obligations : List Alignment) : Verdict :=
  if obligations.all (· == .aligned) then .safe else .unknown

theorem safe_iff_every_obligation_is_aligned (os : List Alignment) :
    alignmentVerdict os = .safe ↔ ∀ o ∈ os, o = .aligned := by
  unfold alignmentVerdict
  constructor
  · intro h o ho
    by_cases hall : os.all (· == .aligned) = true
    · have := List.all_eq_true.mp hall o ho
      simpa using this
    · simp [hall] at h
  · intro h
    have : os.all (· == .aligned) = true := List.all_eq_true.mpr (by
      intro o ho; simp [h o ho])
    simp [this]

theorem a_misaligned_obligation_is_never_safe (os : List Alignment)
    (h : Alignment.misaligned ∈ os) : alignmentVerdict os = .unknown := by
  unfold alignmentVerdict
  have : os.all (· == .aligned) = false := by
    rw [List.all_eq_false]; exact ⟨_, h, by decide⟩
  simp [this]

/-- The vectors, spelled as `layout_extents::ALIGNMENT_VECTORS`:
`(required, base, offset, answer)`. -/
def alignmentVectors : List (Nat × Nat × Offset × Alignment) :=
  [ (4, 1, .const 1, .undecided)
  , (4, 4, .const 1, .misaligned)
  , (4, 4, .const 8, .aligned)
  , (4, 8, .const (-4), .aligned)
  , (4, 4, .scaled 4, .aligned)
  , (8, 4, .scaled 8, .undecided)
  , (4, 4, .scaled 2, .undecided)
  , (4, 4, .unknown, .undecided)
  , (1, 1, .const 3, .aligned)
  , (0, 4, .const 0, .undecided) ]

theorem alignment_vectors_hold :
    alignmentVectors.all (fun (r, b, o, a) => alignment r b o == a) = true := by
  decide

/-! ## container_of -/

/-- What the gateway establishes about the pointer operand `p`. -/
inductive Provenance where
  /-- `p` points at the member path `path` of the object `base` of record
  `record` (Clang's declaration ids). -/
  | memberOf (base record : Nat) (path : List Nat)
  | unestablished
  deriving DecidableEq, Repr

inductive ContainerDecision where
  | enclosing (base : Nat)
  | unestablished
  | recordMismatch
  | memberMismatch
  deriving DecidableEq, Repr

/-- `layout_extents::container_of`: the record and member path the offset
operand names. -/
def containerOf (p : Provenance) (record : Nat) (path : List Nat) : ContainerDecision :=
  match p with
  | .unestablished => .unestablished
  | .memberOf base r q =>
      if r ≠ record then .recordMismatch
      else if q ≠ path then .memberMismatch
      else .enclosing base

theorem an_enclosing_answer_has_its_premise (p : Provenance) (record : Nat) (path : List Nat)
    (base : Nat) (h : containerOf p record path = .enclosing base) :
    p = .memberOf base record path := by
  cases p with
  | unestablished => simp [containerOf] at h
  | memberOf b r q =>
      unfold containerOf at h
      by_cases hr : r = record
      · by_cases hq : q = path
        · subst hr; subst hq; simp at h; subst h; rfl
        · simp [hr, hq] at h
      · simp [hr] at h

theorem an_unrelated_pointer_is_refused (record : Nat) (path : List Nat) :
    containerOf .unestablished record path = .unestablished := rfl

/-- The address arithmetic: `addr` gives each object's address, `offsetOf`
each record's member-path byte offset (Clang's). A member pointer is
`addr base + offsetOf record path`. -/
def memberAddress (addr : Nat → Nat) (offsetOf : Nat → List Nat → Nat)
    (base record : Nat) (path : List Nat) : Nat :=
  addr base + offsetOf record path

/-- The round trip: subtracting the member's offset from a member pointer
yields the enclosing object's address. -/
theorem the_round_trip_returns_the_enclosing_object (addr : Nat → Nat)
    (offsetOf : Nat → List Nat → Nat) (p : Provenance) (record : Nat) (path : List Nat)
    (base : Nat) (h : containerOf p record path = .enclosing base) :
    ∃ b r q, p = .memberOf b r q ∧
      memberAddress addr offsetOf b r q - offsetOf record path = addr base := by
  have hp := an_enclosing_answer_has_its_premise p record path base h
  exact ⟨base, record, path, hp, by simp [memberAddress]⟩

/-- Without the premise the subtraction names no object start: a pointer one
byte into an object, less a member offset of one byte more, lands before it. -/
theorem the_round_trip_needs_the_premise :
    ∃ (addr : Nat → Nat) (p off : Nat), p - off ≠ addr 0 :=
  ⟨fun _ => 8, 9, 2, by decide⟩

/-- The vectors, spelled as `layout_extents::CONTAINER_VECTORS`:
`(provenance, record, path, answer)`. -/
def containerVectors : List (Provenance × Nat × List Nat × ContainerDecision) :=
  [ (.memberOf 7 1 [2], 1, [2], .enclosing 7)
  , (.memberOf 7 1 [2, 3], 1, [2, 3], .enclosing 7)
  , (.memberOf 7 1 [2], 1, [3], .memberMismatch)
  , (.memberOf 7 1 [2], 4, [2], .recordMismatch)
  , (.unestablished, 1, [2], .unestablished) ]

theorem container_vectors_hold :
    containerVectors.all (fun (p, r, q, d) => containerOf p r q == d) = true := by
  decide

end QCore.LayoutExtents
