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
# OpenTelemetry protocol obligations (card OTEL-PROTO-AUDIT)

The model of the protocol-derived obligations the audit states over the
OpenTelemetry decoders and encoders, with one definition per term of the OTP
block of the C04.18.9 typed rendering (`otpTerms`) and one theorem per
formula of `docs/reports/otel-proto-audit/formulas.json` (`formula_<id>`;
the formulas are stated as written in `formulaTable`, bound by
`crates/qcore-frontend/tests/otel_proto_formulas.rs`).

* **Representation.** A message's field is stored as protobuf-c's unpack
  stores it (`Slot`): a pointer (a sub-message or a string) that is NULL or
  designates a value, an array of element values (so no element below the
  count is NULL, and the array pointer is non-NULL exactly when the count is
  positive), bytes as their contents (`data` is non-NULL exactly when `len`
  is positive), or a scalar (an enum keeps any `int32`). The facts that hold
  of every such representation are theorems here
  (`formula_otp_repeated_elements_nonnull`, `formula_otp_repeated_array_nonnull`,
  `formula_otp_bytes_data_iff_length`).
* **The unpack postcondition.** `WellFormed` states what protobuf-c 1.5's
  `parse_required_member` / `parse_oneof_member` establish for the pointer
  members of an unpacked message: a singular sub-message is non-NULL exactly
  when the wire carried it, the selected oneof message member is non-NULL, a
  string is non-NULL. It is the premise the C runtime is assumed to meet
  (the audit cites the runtime's lines; nothing here proves the C), and the
  formulas resting on it take it as a hypothesis
  (`formula_otp_absent_message_is_null`, `formula_otp_selected_member_nonnull`,
  `formula_otp_string_nonnull`).
* **Sites.** A site records the fact its obligation needs (`Holds`), whether
  the syntactic reader found a guard (`tested`), whether its function is
  reachable from an unpack of wire bytes in its unit (`wire`) and whether the
  reader decided it. `classify` is the census's classification; a census is
  `Sound` when every guard the reader cites holds at its site. Then a guarded
  site's obligation holds (`formula_otp_deref_needs_nonnull` and its
  siblings), the classes partition the sites, a candidate is on the wire and
  unguarded, an unguarded site off the wire is undetermined, and no class is
  an Unsafe (`formula_otp_no_unsafe_claimed`).
* **Identity.** A use of the protocol is refused exactly when its identity
  differs from the pinned one.
-/

namespace QCore.OtelProtoObligations

/-- A field's type, as the generated descriptor states it. -/
inductive Kind where
  | scalar
  | enum
  | string
  | bytes
  | message
  deriving DecidableEq, Repr

/-- A field of a generated message descriptor. -/
structure Field where
  number : Int
  kind : Kind
  repeated : Bool
  oneof : Bool
  deriving DecidableEq, Repr

/-- How protobuf-c stores a field of an unpacked message. -/
inductive Slot where
  | ptr (target : Option Nat)
  | arr (elems : List Nat)
  | bytes (data : List Nat)
  | scalar (value : Int)

/-- A message is named by a natural number; a position by its index. -/
abbrev Msg := Nat
abbrev Pos := Int

/-- The decoded messages of one run. -/
structure State where
  slot : Msg → Field → Slot
  present : Msg → Field → Prop
  caseOf : Msg → Field → Int
  named : Field → Int → Prop
  unpacked : Msg → Prop

variable (st : State)

def nonnull (m : Msg) (f : Field) : Prop :=
  match st.slot m f with
  | .ptr t => ∃ v, t = some v
  | .arr l => l ≠ []
  | _ => False

def count (m : Msg) (f : Field) : Int :=
  match st.slot m f with
  | .arr l => (l.length : Int)
  | _ => 0

def at_ (p : Pos) : Int := p

def elemNonnull (m : Msg) (f : Field) (p : Pos) : Prop :=
  match st.slot m f with
  | .arr l => 0 ≤ p ∧ p < (l.length : Int)
  | _ => False

def caseOf (m : Msg) (f : Field) : Int := st.caseOf m f

def number (f : Field) : Int := f.number

def len (m : Msg) (f : Field) : Int :=
  match st.slot m f with
  | .bytes d => (d.length : Int)
  | _ => 0

def dataNonnull (m : Msg) (f : Field) : Prop :=
  match st.slot m f with
  | .bytes d => d ≠ []
  | _ => False

def valueNamed (m : Msg) (f : Field) : Prop :=
  match st.slot m f with
  | .scalar v => st.named f v
  | _ => False

def messageTyped (f : Field) : Prop := f.kind = .message
def isRepeated (f : Field) : Prop := f.repeated = true
def isOneof (f : Field) : Prop := f.oneof = true
def isString (f : Field) : Prop := f.kind = .string
def isBytes (f : Field) : Prop := f.kind = .bytes
def isEnum (f : Field) : Prop := f.kind = .enum

/-- protobuf-c's unpack postcondition for the pointer members of an
unpacked message (the premise on the C runtime). -/
structure WellFormed : Prop where
  singular_message : ∀ m f, st.unpacked m → f.kind = .message → f.repeated = false →
    f.oneof = false → ∃ t, st.slot m f = .ptr t ∧ ((∃ v, t = some v) ↔ st.present m f)
  selected_message : ∀ m f, st.unpacked m → f.oneof = true → f.kind = .message →
    st.caseOf m f = f.number → ∃ v, st.slot m f = .ptr (some v)
  string_member : ∀ m f, st.unpacked m → f.kind = .string → f.repeated = false →
    (f.oneof = false ∨ st.caseOf m f = f.number) → ∃ v, st.slot m f = .ptr (some v)

/-! ## The representation's facts and the unpack postcondition -/

theorem formula_otp_absent_message_is_null (h : WellFormed st) :
    ∀ m f, (st.unpacked m ∧ ((messageTyped f ∧ ¬ isRepeated f) ∧ ¬ isOneof f)) →
      ((st.present m f → nonnull st m f) ∧ (nonnull st m f → st.present m f)) := by
  intro m f ⟨hu, ⟨hk, hr⟩, ho⟩
  have hr' : f.repeated = false := by simp [isRepeated] at hr; exact hr
  have ho' : f.oneof = false := by simp [isOneof] at ho; exact ho
  obtain ⟨t, hs, hiff⟩ := h.singular_message m f hu hk hr' ho'
  constructor
  · intro hp; simp only [nonnull, hs]; exact hiff.mpr hp
  · intro hn; simp only [nonnull, hs] at hn; exact hiff.mp hn

theorem formula_otp_repeated_elements_nonnull :
    ∀ m f (p : Pos), ((st.unpacked m ∧ (isRepeated f ∧ messageTyped f)) ∧
      (0 ≤ at_ p ∧ at_ p < count st m f)) → elemNonnull st m f p := by
  intro m f p ⟨_, h0, hlt⟩
  simp only [at_] at h0 hlt
  unfold count at hlt
  unfold elemNonnull
  cases hs : st.slot m f with
  | arr l =>
    rw [hs] at hlt
    exact ⟨h0, hlt⟩
  | ptr t => rw [hs] at hlt; dsimp only at hlt; omega
  | bytes d => rw [hs] at hlt; dsimp only at hlt; omega
  | scalar v => rw [hs] at hlt; dsimp only at hlt; omega

theorem formula_otp_repeated_array_nonnull :
    ∀ m f, (st.unpacked m ∧ isRepeated f) → (0 < count st m f → nonnull st m f) := by
  intro m f _ hc
  unfold count at hc
  unfold nonnull
  cases hs : st.slot m f with
  | arr l =>
    rw [hs] at hc
    intro he
    rw [he] at hc
    simp at hc
  | ptr t => rw [hs] at hc; simp at hc
  | bytes d => rw [hs] at hc; simp at hc
  | scalar v => rw [hs] at hc; simp at hc

theorem formula_otp_selected_member_nonnull (h : WellFormed st) :
    ∀ m f, ((st.unpacked m ∧ isOneof f) ∧ messageTyped f) →
      (caseOf st m f = number f → nonnull st m f) := by
  intro m f ⟨⟨hu, ho⟩, hk⟩ hc
  obtain ⟨v, hs⟩ := h.selected_message m f hu ho hk hc
  simp only [nonnull, hs]
  exact ⟨v, rfl⟩

theorem formula_otp_string_nonnull (h : WellFormed st) :
    ∀ m f, (st.unpacked m ∧ ((isString f ∧ ¬ isRepeated f) ∧
      (¬ isOneof f ∨ caseOf st m f = number f))) → nonnull st m f := by
  intro m f ⟨hu, ⟨hk, hr⟩, ho⟩
  have hr' : f.repeated = false := by simp [isRepeated] at hr; exact hr
  have ho' : f.oneof = false ∨ st.caseOf m f = f.number := by
    rcases ho with ho | ho
    · left; simp [isOneof] at ho; exact ho
    · right; exact ho
  obtain ⟨v, hs⟩ := h.string_member m f hu hk hr' ho'
  simp only [nonnull, hs]
  exact ⟨v, rfl⟩

theorem formula_otp_bytes_data_iff_length :
    ∀ m f, (st.unpacked m ∧ ((isBytes f ∧ ¬ isRepeated f) ∧
      (¬ isOneof f ∨ caseOf st m f = number f))) →
      ((dataNonnull st m f → 0 < len st m f) ∧ (0 < len st m f → dataNonnull st m f)) := by
  intro m f _
  unfold dataNonnull len
  cases hs : st.slot m f with
  | bytes d =>
    constructor
    · intro hne
      cases d with
      | nil => exact absurd rfl hne
      | cons _ t => simp
    · intro hl he
      rw [he] at hl
      simp at hl
  | ptr t =>
    exact ⟨fun hf => by simp at hf, fun hl => by simp at hl⟩
  | arr l =>
    exact ⟨fun hf => by simp at hf, fun hl => by simp at hl⟩
  | scalar v =>
    exact ⟨fun hf => by simp at hf, fun hl => by simp at hl⟩

/-! ## Sites and their classification -/

/-- A site's obligation (O0 to O5). -/
inductive Ob where
  | unpack
  | deref
  | subscript
  | member
  | data
  | enumIdx
  deriving DecidableEq, Repr

structure Site where
  ob : Ob
  msg : Msg
  field : Field
  pos : Pos
  wire : Bool
  determinable : Bool
  tested : Bool
  reasonCode : String

/-- The fact a site's obligation needs; an unpack result is named by a
positive number (zero is NULL). -/
def Holds (s : Site) : Prop :=
  match s.ob with
  | .unpack => 0 < s.msg
  | .deref => nonnull st s.msg s.field
  | .subscript => 0 ≤ s.pos ∧ s.pos < count st s.msg s.field
  | .member => caseOf st s.msg s.field = number s.field
  | .data => 0 < len st s.msg s.field ∨ dataNonnull st s.msg s.field
  | .enumIdx => valueNamed st s.msg s.field

inductive Reason where
  | offWire
  | named (code : String)

inductive Class where
  | guarded
  | candidate
  | undetermined (r : Reason)

/-- The census's classification of a site. -/
def classify (s : Site) : Class :=
  if s.tested then .guarded
  else if s.wire then (if s.determinable then .candidate else .undetermined (.named s.reasonCode))
  else .undetermined .offWire

def guarded (s : Site) : Prop := classify s = .guarded
def candidate (s : Site) : Prop := classify s = .candidate
def undetermined (s : Site) : Prop := ∃ r, classify s = .undetermined r
def reads (s : Site) (m : Msg) (f : Field) : Prop := s.msg = m ∧ s.field = f
def posOf (s : Site) : Pos := s.pos
def resultNonnull (s : Site) : Prop := 0 < s.msg
def cited (s : Site) : Prop := s.tested = true
def reasonNamed (s : Site) : Prop :=
  match classify s with
  | .undetermined _ => True
  | _ => False
def unsafeClaimed (_ : Site) : Prop := False
def isWire (s : Site) : Prop := s.wire = true

/-- A census is sound when every guard it cites holds at its site. -/
def Sound (s : Site) : Prop := s.tested = true → Holds st s

theorem guarded_iff_tested (s : Site) : guarded s ↔ s.tested = true := by
  unfold guarded classify
  cases ht : s.tested <;> cases hw : s.wire <;> cases hd : s.determinable <;> simp

theorem guarded_holds (s : Site) (hs : Sound st s) (hg : guarded s) : Holds st s :=
  hs ((guarded_iff_tested s).mp hg)

theorem formula_otp_unpack_result_tested (sound : ∀ s, Sound st s) :
    ∀ s, (s.ob = .unpack ∧ guarded s) → resultNonnull s := by
  intro s ⟨ho, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  exact h

theorem formula_otp_deref_needs_nonnull (sound : ∀ s, Sound st s) :
    ∀ s m f, ((s.ob = .deref ∧ reads s m f) ∧ guarded s) → nonnull st m f := by
  intro s m f ⟨⟨ho, hm, hf⟩, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  subst hm hf
  exact h

theorem formula_otp_subscript_needs_bound (sound : ∀ s, Sound st s) :
    ∀ s m f, ((s.ob = .subscript ∧ reads s m f) ∧ guarded s) →
      (0 ≤ at_ (posOf s) ∧ at_ (posOf s) < count st m f) := by
  intro s m f ⟨⟨ho, hm, hf⟩, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  subst hm hf
  exact h

theorem formula_otp_member_needs_case (sound : ∀ s, Sound st s) :
    ∀ s m f, ((s.ob = .member ∧ reads s m f) ∧ guarded s) → caseOf st m f = number f := by
  intro s m f ⟨⟨ho, hm, hf⟩, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  subst hm hf
  exact h

theorem formula_otp_data_needs_length (sound : ∀ s, Sound st s) :
    ∀ s m f, ((s.ob = .data ∧ reads s m f) ∧ guarded s) →
      (0 < len st m f ∨ dataNonnull st m f) := by
  intro s m f ⟨⟨ho, hm, hf⟩, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  subst hm hf
  exact h

theorem formula_otp_enum_index_needs_name (sound : ∀ s, Sound st s) :
    ∀ s m f, (((s.ob = .enumIdx ∧ reads s m f) ∧ isEnum f) ∧ guarded s) → valueNamed st m f := by
  intro s m f ⟨⟨⟨ho, hm, hf⟩, _⟩, hg⟩
  have h := guarded_holds st s (sound s) hg
  simp only [Holds, ho] at h
  subst hm hf
  exact h

theorem formula_otp_bounded_element_nonnull (sound : ∀ s, Sound st s) :
    ∀ s m f, ((((s.ob = .subscript ∧ reads s m f) ∧ guarded s) ∧ st.unpacked m) ∧
      (isRepeated f ∧ messageTyped f)) → elemNonnull st m f (posOf s) := by
  intro s m f ⟨⟨⟨⟨ho, hr⟩, hg⟩, hu⟩, hk⟩
  have hb := formula_otp_subscript_needs_bound st sound s m f ⟨⟨ho, hr⟩, hg⟩
  exact formula_otp_repeated_elements_nonnull st m f (posOf s) ⟨⟨hu, hk⟩, hb⟩

theorem formula_otp_selected_member_guarded_nonnull (h : WellFormed st)
    (sound : ∀ s, Sound st s) :
    ∀ s m f, ((((s.ob = .member ∧ reads s m f) ∧ guarded s) ∧ st.unpacked m) ∧
      (isOneof f ∧ messageTyped f)) → nonnull st m f := by
  intro s m f ⟨⟨⟨⟨ho, hr⟩, hg⟩, hu⟩, ho', hk⟩
  have hc := formula_otp_member_needs_case st sound s m f ⟨⟨ho, hr⟩, hg⟩
  exact formula_otp_selected_member_nonnull st h m f ⟨⟨hu, ho'⟩, hk⟩ hc

theorem formula_otp_classes_partition :
    ∀ s, ((guarded s ∨ candidate s) ∨ undetermined s) ∧
      ((¬ (guarded s ∧ candidate s) ∧ ¬ (guarded s ∧ undetermined s)) ∧
        ¬ (candidate s ∧ undetermined s)) := by
  intro s
  unfold guarded candidate undetermined
  cases classify s with
  | guarded => simp
  | candidate => simp
  | undetermined r => simp

theorem formula_otp_candidate_is_wire_and_unguarded :
    ∀ s, candidate s → (isWire s ∧ ¬ guarded s) := by
  intro s h
  unfold candidate classify at h
  rw [guarded_iff_tested]
  unfold isWire
  cases ht : s.tested <;> cases hw : s.wire <;> cases hd : s.determinable <;> simp_all

theorem formula_otp_unguarded_off_wire_undetermined :
    ∀ s, (¬ guarded s ∧ ¬ isWire s) → undetermined s := by
  intro s ⟨hg, hw⟩
  rw [guarded_iff_tested] at hg
  unfold isWire at hw
  refine ⟨.offWire, ?_⟩
  unfold classify
  cases ht : s.tested <;> cases hw' : s.wire <;> simp_all

theorem formula_otp_guard_is_cited : ∀ s, guarded s → cited s := by
  intro s hg
  exact (guarded_iff_tested s).mp hg

theorem formula_otp_undetermined_names_reason : ∀ s, undetermined s → reasonNamed s := by
  intro s ⟨r, h⟩
  simp [reasonNamed, h]

theorem formula_otp_no_unsafe_claimed : ∀ s, ¬ unsafeClaimed s := by
  intro s h
  exact h

/-! ## The pinned protocol identity -/

/-- A protocol identity: the commit and every `.proto` file's digest. -/
abbrev Identity := String × List (String × String)

def pinned (p d : Identity) : Prop := d = p
def refused (p d : Identity) : Prop := d ≠ p

theorem formula_otp_identity_refused_unless_pinned (p : Identity) :
    ∀ d, ¬ pinned p d → refused p d := by
  intro d h
  exact h

theorem formula_otp_pinned_identity_admitted (p : Identity) :
    ∀ d, pinned p d → ¬ refused p d := by
  intro d h hr
  exact hr h

/-! ## The term and formula tables -/

def otpSorts : List String := ["otp_msg", "otp_field", "otp_pos", "otp_site", "otp_identity"]

/-- The Lean definition each term is modelled by. -/
def otpTerms : List (String × String) := [
  ("otp_unpacked", "State.unpacked"),
  ("otp_message_typed", "messageTyped"),
  ("otp_repeated", "isRepeated"),
  ("otp_oneof", "isOneof"),
  ("otp_string", "isString"),
  ("otp_bytes", "isBytes"),
  ("otp_enum", "isEnum"),
  ("otp_present", "State.present"),
  ("otp_nonnull", "nonnull"),
  ("otp_count", "count"),
  ("otp_at", "at_"),
  ("otp_elem_nonnull", "elemNonnull"),
  ("otp_case", "caseOf"),
  ("otp_number", "number"),
  ("otp_len", "len"),
  ("otp_data_nonnull", "dataNonnull"),
  ("otp_value_named", "valueNamed"),
  ("otp_reads", "reads"),
  ("otp_unpack_site", "Ob.unpack"),
  ("otp_result_nonnull", "resultNonnull"),
  ("otp_deref_site", "Ob.deref"),
  ("otp_subscript_site", "Ob.subscript"),
  ("otp_pos_of", "posOf"),
  ("otp_member_site", "Ob.member"),
  ("otp_data_site", "Ob.data"),
  ("otp_enum_site", "Ob.enumIdx"),
  ("otp_guarded", "guarded"),
  ("otp_candidate", "candidate"),
  ("otp_undetermined", "undetermined"),
  ("otp_wire", "isWire"),
  ("otp_cited", "cited"),
  ("otp_reason_named", "reasonNamed"),
  ("otp_unsafe_claimed", "unsafeClaimed"),
  ("otp_pinned", "pinned"),
  ("otp_refused", "refused")]

def formulaTable : List (String × String) := [
  ("otp_absent_message_is_null", "forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_message_typed(#f) && !#otp_repeated(#f)) && !#otp_oneof(#f))) => ((#otp_present(#m,#f) => #otp_nonnull(#m,#f)) && (#otp_nonnull(#m,#f) => #otp_present(#m,#f)))) } }"),
  ("otp_repeated_elements_nonnull", "forall (#m: otp_msg) { forall (#f: otp_field) { forall (#p: otp_pos) { (((#otp_unpacked(#m) && (#otp_repeated(#f) && #otp_message_typed(#f))) && (0 <= #otp_at(#p) && #otp_at(#p) < #otp_count(#m,#f))) => #otp_elem_nonnull(#m,#f,#p)) } } }"),
  ("otp_repeated_array_nonnull", "forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && #otp_repeated(#f)) => (0 < #otp_count(#m,#f) => #otp_nonnull(#m,#f))) } }"),
  ("otp_selected_member_nonnull", "forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_unpacked(#m) && #otp_oneof(#f)) && #otp_message_typed(#f)) => (#otp_case(#m,#f) == #otp_number(#f) => #otp_nonnull(#m,#f))) } }"),
  ("otp_string_nonnull", "forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_string(#f) && !#otp_repeated(#f)) && (!#otp_oneof(#f) || #otp_case(#m,#f) == #otp_number(#f)))) => #otp_nonnull(#m,#f)) } }"),
  ("otp_bytes_data_iff_length", "forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_bytes(#f) && !#otp_repeated(#f)) && (!#otp_oneof(#f) || #otp_case(#m,#f) == #otp_number(#f)))) => ((#otp_data_nonnull(#m,#f) => 0 < #otp_len(#m,#f)) && (0 < #otp_len(#m,#f) => #otp_data_nonnull(#m,#f)))) } }"),
  ("otp_unpack_result_tested", "forall (#s: otp_site) { ((#otp_unpack_site(#s) && #otp_guarded(#s)) => #otp_result_nonnull(#s)) }"),
  ("otp_deref_needs_nonnull", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_deref_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => #otp_nonnull(#m,#f)) } } }"),
  ("otp_subscript_needs_bound", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_subscript_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => (0 <= #otp_at(#otp_pos_of(#s)) && #otp_at(#otp_pos_of(#s)) < #otp_count(#m,#f))) } } }"),
  ("otp_member_needs_case", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_member_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => #otp_case(#m,#f) == #otp_number(#f)) } } }"),
  ("otp_data_needs_length", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_data_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => (0 < #otp_len(#m,#f) || #otp_data_nonnull(#m,#f))) } } }"),
  ("otp_enum_index_needs_name", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { ((((#otp_enum_site(#s) && #otp_reads(#s,#m,#f)) && #otp_enum(#f)) && #otp_guarded(#s)) => #otp_value_named(#m,#f)) } } }"),
  ("otp_bounded_element_nonnull", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((((#otp_subscript_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) && #otp_unpacked(#m)) && (#otp_repeated(#f) && #otp_message_typed(#f))) => #otp_elem_nonnull(#m,#f,#otp_pos_of(#s))) } } }"),
  ("otp_selected_member_guarded_nonnull", "forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((((#otp_member_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) && #otp_unpacked(#m)) && (#otp_oneof(#f) && #otp_message_typed(#f))) => #otp_nonnull(#m,#f)) } } }"),
  ("otp_classes_partition", "forall (#s: otp_site) { (((#otp_guarded(#s) || #otp_candidate(#s)) || #otp_undetermined(#s)) && ((!(#otp_guarded(#s) && #otp_candidate(#s)) && !(#otp_guarded(#s) && #otp_undetermined(#s))) && !(#otp_candidate(#s) && #otp_undetermined(#s)))) }"),
  ("otp_candidate_is_wire_and_unguarded", "forall (#s: otp_site) { (#otp_candidate(#s) => (#otp_wire(#s) && !#otp_guarded(#s))) }"),
  ("otp_unguarded_off_wire_undetermined", "forall (#s: otp_site) { ((!#otp_guarded(#s) && !#otp_wire(#s)) => #otp_undetermined(#s)) }"),
  ("otp_guard_is_cited", "forall (#s: otp_site) { (#otp_guarded(#s) => #otp_cited(#s)) }"),
  ("otp_undetermined_names_reason", "forall (#s: otp_site) { (#otp_undetermined(#s) => #otp_reason_named(#s)) }"),
  ("otp_no_unsafe_claimed", "forall (#s: otp_site) { !#otp_unsafe_claimed(#s) }"),
  ("otp_identity_refused_unless_pinned", "forall (#d: otp_identity) { (!#otp_pinned(#d) => #otp_refused(#d)) }"),
  ("otp_pinned_identity_admitted", "forall (#d: otp_identity) { (#otp_pinned(#d) => !#otp_refused(#d)) }")]

theorem the_tables_are_counted :
    formulaTable.length = 22 ∧ otpTerms.length = 35 ∧ otpSorts.length = 5 := by
  decide

end QCore.OtelProtoObligations
