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
# Binding an installed manifest to the template embedded in an executable

The model of card P01-F2's enable-time gate. The deciding implementation is
`crates/qcore-verifier-host/src/embedded_template.rs`, reached from
`verify_executable_package`; the recorded threat model and refusal matrix are
`docs/reports/remaining-p01-f2/README.md`.

Two decisions are modeled, in the order the implementation takes them.

* **Extraction.** An executable is a sequence of tokens. The template is the
  token run between one opening and one closing marker. The procedure accepts
  only when there is *exactly one* marker of each kind, in that order: absence,
  duplication and truncation are separate refusals, never a payload.
* **Binding.** A manifest is a digest plus a total assignment of values to the
  fifteen declared fields. The installed manifest binds to the template when its
  digest is the recomputed digest of the executable's own bytes and every field
  agrees. The template's *own* digest slot is never consulted, because an
  executable cannot carry its own hash.

Three reductions are deliberate, and are why this is a model rather than a
transcription. Bytes are abstract tokens, so nothing here says the marker
spelling is unambiguous in a real ELF file (the implementation decides that by
counting occurrences, as modeled). Field values are natural numbers, so field
*equality* is modeled but not the shape of any field. And parsing the payload
into a manifest is an opaque parameter: JSON, `serde` strictness and manifest
validation are outside this model.

Nothing here claims the executable *behaves* as its template declares, nor that
the executable is authentic. An actor who can rebuild or forge the executable
controls its embedded template too; that boundary is stated in the report and is
not narrowed by any theorem below.
-/

namespace QCore.PackageTemplateBinding

/-! ## Executable bytes and the envelope -/

/-- A token of the executable: an opening marker, a closing marker, or an
ordinary byte run carrying no marker. -/
inductive Tok where
  /-- The opening marker `<<<qcore.provider-template.v1:begin>>>`. -/
  | opening
  /-- The closing marker `<<<qcore.provider-template.v1:end>>>`. -/
  | closing
  /-- Any content that is not a marker. -/
  | plain (n : Nat)
  deriving DecidableEq, Repr

/-- Whether a token is one of the two markers. -/
def Tok.isMarker : Tok → Bool
  | .plain _ => false
  | _ => true

/-- Occurrences of one token, defined here so its append law is available
without depending on any library lemma. -/
def count (t : Tok) : List Tok → Nat
  | [] => 0
  | x :: rest => (if x = t then 1 else 0) + count t rest

theorem count_append (t : Tok) (xs ys : List Tok) :
    count t (xs ++ ys) = count t xs + count t ys := by
  induction xs with
  | nil => simp [count]
  | cons x rest ih => simp [count, ih, Nat.add_assoc]

theorem count_eq_zero_of_markerless (t : Tok) (h : t.isMarker = true) :
    ∀ xs : List Tok, (∀ x ∈ xs, x.isMarker = false) → count t xs = 0
  | [], _ => rfl
  | x :: rest, hx => by
    have head : x ≠ t := by
      intro eq
      have := hx x (by simp)
      rw [eq] at this
      simp [h] at this
    have tail := count_eq_zero_of_markerless t h rest (fun y hy => hx y (by simp [hy]))
    simp [count, head, tail]

/-- The result of looking for the embedded template. -/
inductive Extraction where
  /-- No marker at all: the executable carries no template. -/
  | absent
  /-- More than one marker of either kind: which envelope is authoritative is
  not decidable, so none is accepted. -/
  | ambiguous
  /-- An opening marker with no closing marker after it, or the two in the
  wrong order. -/
  | unterminated
  /-- The single well-formed payload. -/
  | payload (bytes : List Tok)
  deriving DecidableEq, Repr

/-- The tail after the first opening marker. -/
def afterOpening : List Tok → Option (List Tok)
  | [] => none
  | .opening :: rest => some rest
  | _ :: rest => afterOpening rest

/-- The run before the first closing marker. -/
def beforeClosing : List Tok → Option (List Tok)
  | [] => none
  | .closing :: _ => some []
  | t :: rest => (beforeClosing rest).map (t :: ·)

/-- The run bracketed by the first opening marker and the first closing marker
after it, if both are present in that order. -/
def locate (xs : List Tok) : Option (List Tok) :=
  match afterOpening xs with
  | none => none
  | some rest => beforeClosing rest

/-- Locate the embedded template, refusing on absence, duplication and
truncation. This never executes anything: it reads the token sequence. -/
def extract (xs : List Tok) : Extraction :=
  if count .opening xs = 0 ∧ count .closing xs = 0 then .absent
  else if count .opening xs > 1 ∨ count .closing xs > 1 then .ambiguous
  else
    match locate xs with
    | none => .unterminated
    | some payload => .payload payload

/-- Wrap a template the way a package executable embeds it. -/
def envelope (prefixBytes payload suffixBytes : List Tok) : List Tok :=
  prefixBytes ++ .opening :: (payload ++ .closing :: suffixBytes)

theorem afterOpening_of_markerless (payload : List Tok)
    (h : ∀ x ∈ payload, x.isMarker = false) (rest : List Tok) :
    afterOpening (payload ++ .opening :: rest) = some rest := by
  induction payload with
  | nil => rfl
  | cons x tail ih =>
    have hx := h x (by simp)
    have tail' := ih (fun y hy => h y (by simp [hy]))
    cases x with
    | opening => simp [Tok.isMarker] at hx
    | closing => simp [Tok.isMarker] at hx
    | plain n => simpa [afterOpening] using tail'

theorem beforeClosing_of_markerless (payload : List Tok)
    (h : ∀ x ∈ payload, x.isMarker = false) (rest : List Tok) :
    beforeClosing (payload ++ .closing :: rest) = some payload := by
  induction payload with
  | nil => rfl
  | cons x tail ih =>
    have hx := h x (by simp)
    have tail' := ih (fun y hy => h y (by simp [hy]))
    cases x with
    | opening => simp [Tok.isMarker] at hx
    | closing => simp [Tok.isMarker] at hx
    | plain n => simp [beforeClosing, tail']

/-- **Round trip.** A package whose executable carries exactly one envelope
yields exactly the embedded payload, whatever surrounds it. -/
theorem extract_envelope (prefixBytes payload suffixBytes : List Tok)
    (hp : ∀ x ∈ prefixBytes, x.isMarker = false)
    (hb : ∀ x ∈ payload, x.isMarker = false)
    (hs : ∀ x ∈ suffixBytes, x.isMarker = false) :
    extract (envelope prefixBytes payload suffixBytes) = .payload payload := by
  have zeroP := count_eq_zero_of_markerless .opening rfl prefixBytes hp
  have zeroB := count_eq_zero_of_markerless .opening rfl payload hb
  have zeroS := count_eq_zero_of_markerless .opening rfl suffixBytes hs
  have zeroP' := count_eq_zero_of_markerless .closing rfl prefixBytes hp
  have zeroB' := count_eq_zero_of_markerless .closing rfl payload hb
  have zeroS' := count_eq_zero_of_markerless .closing rfl suffixBytes hs
  have opens :
      count .opening (prefixBytes ++ .opening :: (payload ++ .closing :: suffixBytes)) = 1 := by
    simp [count_append, count, zeroP, zeroB, zeroS]
  have closes :
      count .closing (prefixBytes ++ .opening :: (payload ++ .closing :: suffixBytes)) = 1 := by
    simp [count_append, count, zeroP', zeroB', zeroS']
  simp [extract, envelope, locate, opens, closes,
    beforeClosing_of_markerless payload hb,
    afterOpening_of_markerless prefixBytes hp]

/-- An executable with no marker is refused as absent, never as a payload. -/
theorem extract_absent (xs : List Tok) (h : ∀ x ∈ xs, x.isMarker = false) :
    extract xs = .absent := by
  simp [extract, count_eq_zero_of_markerless .opening rfl xs h,
    count_eq_zero_of_markerless .closing rfl xs h]

/-- Two envelopes in one executable are ambiguous: neither is accepted. -/
theorem extract_duplicate_is_ambiguous
    (a b p q : List Tok)
    (ha : ∀ x ∈ a, x.isMarker = false) (hb : ∀ x ∈ b, x.isMarker = false)
    (hp : ∀ x ∈ p, x.isMarker = false) (hq : ∀ x ∈ q, x.isMarker = false) :
    extract (envelope a p [] ++ envelope b q []) = .ambiguous := by
  have opens :
      count .opening (envelope a p [] ++ envelope b q []) = 2 := by
    simp [envelope, count_append, count,
      count_eq_zero_of_markerless .opening rfl a ha,
      count_eq_zero_of_markerless .opening rfl b hb,
      count_eq_zero_of_markerless .opening rfl p hp,
      count_eq_zero_of_markerless .opening rfl q hq]
  simp [extract, opens]

/-- An opening marker with nothing after it is truncation, not a payload. -/
theorem extract_unterminated_is_refused :
    extract [.plain 0, .opening, .plain 1] = .unterminated := by rfl

/-- A closing marker before the opening one is truncation too. -/
theorem extract_reversed_is_refused :
    extract [.closing, .plain 1, .opening] = .unterminated := by rfl

theorem beforeClosing_bracketed :
    ∀ (xs p : List Tok), beforeClosing xs = some p → ∃ post, xs = p ++ .closing :: post := by
  intro xs
  induction xs with
  | nil => intro p h; simp [beforeClosing] at h
  | cons x rest ih =>
    intro p h
    cases x with
    | closing =>
      refine ⟨rest, ?_⟩
      simp [beforeClosing] at h
      simp [← h]
    | opening =>
      simp only [beforeClosing, Option.map_eq_some_iff] at h
      obtain ⟨q, hq, hp⟩ := h
      obtain ⟨post, hpost⟩ := ih q hq
      exact ⟨post, by simp [← hp, hpost]⟩
    | plain n =>
      simp only [beforeClosing, Option.map_eq_some_iff] at h
      obtain ⟨q, hq, hp⟩ := h
      obtain ⟨post, hpost⟩ := ih q hq
      exact ⟨post, by simp [← hp, hpost]⟩

theorem locate_bracketed :
    ∀ (xs p : List Tok), locate xs = some p →
      ∃ pre post, xs = pre ++ .opening :: (p ++ .closing :: post) := by
  intro xs
  induction xs with
  | nil => intro p h; simp [locate, afterOpening] at h
  | cons x rest ih =>
    intro p h
    cases x with
    | opening =>
      have hrest : beforeClosing rest = some p := by
        simpa [locate, afterOpening] using h
      obtain ⟨post, hpost⟩ := beforeClosing_bracketed rest p hrest
      exact ⟨[], post, by simp [hpost]⟩
    | closing =>
      have : locate rest = some p := by simpa [locate, afterOpening] using h
      obtain ⟨pre, post, hrest⟩ := ih p this
      exact ⟨.closing :: pre, post, by simp [hrest]⟩
    | plain n =>
      have : locate rest = some p := by simpa [locate, afterOpening] using h
      obtain ⟨pre, post, hrest⟩ := ih p this
      exact ⟨.plain n :: pre, post, by simp [hrest]⟩

theorem extract_payload_locate (xs p : List Tok) (h : extract xs = .payload p) :
    locate xs = some p := by
  unfold extract at h
  split at h
  · simp at h
  · split at h
    · simp at h
    · cases hl : locate xs with
      | none => rw [hl] at h; simp at h
      | some q =>
        rw [hl] at h
        simp only [Extraction.payload.injEq] at h
        exact congrArg some h

/-- Extraction never invents a payload: every accepted payload came from the
executable's own single envelope. -/
theorem extract_payload_is_bracketed (xs p : List Tok) (h : extract xs = .payload p) :
    ∃ pre post, xs = pre ++ .opening :: (p ++ .closing :: post) :=
  locate_bracketed xs p (extract_payload_locate xs p h)

/-! ## The declared fields and the binding decision -/

/-- The manifest fields the gate compares, in the order the implementation
compares them. `package_digest` is *not* here: it is the one field a template
cannot carry, and is bound separately to the executable's recomputed hash. -/
inductive Field where
  | pluginId | packageVersion | protocolVersions | evidenceVersions | entrypoint
  | targets | capabilities | requiredForeignSummaries | requiredObservationHooks
  | filesystemInputs | networkAccess | limits | trustClass | certificateChecker
  | leanTheoremFamily
  deriving DecidableEq, Repr

/-- The complete field inventory. The Rust procedure obtains the same
exhaustiveness by destructuring the manifest, so a new schema field fails to
compile rather than going unbound. -/
def allFields : List Field :=
  [.pluginId, .packageVersion, .protocolVersions, .evidenceVersions, .entrypoint,
   .targets, .capabilities, .requiredForeignSummaries, .requiredObservationHooks,
   .filesystemInputs, .networkAccess, .limits, .trustClass, .certificateChecker,
   .leanTheoremFamily]

theorem mem_allFields (f : Field) : f ∈ allFields := by
  cases f <;> simp [allFields]

/-- A manifest: the digest slot plus a total assignment to the declared
fields. Field *values* are abstract; only their equality is modeled. -/
structure Manifest where
  digest : Nat
  value : Field → Nat

/-- The first field on which the installed manifest departs from the template,
in inventory order. -/
def firstDivergence : List Field → (Field → Nat) → (Field → Nat) → Option Field
  | [], _, _ => none
  | f :: rest, i, t => if i f = t f then firstDivergence rest i t else some f

theorem firstDivergence_none_iff (fs : List Field) (i t : Field → Nat) :
    firstDivergence fs i t = none ↔ ∀ f ∈ fs, i f = t f := by
  induction fs with
  | nil => simp [firstDivergence]
  | cons f rest ih =>
    by_cases h : i f = t f <;> simp [firstDivergence, h, ih]

theorem firstDivergence_some (fs : List Field) (i t : Field → Nat) (f : Field) :
    firstDivergence fs i t = some f → i f ≠ t f ∧ f ∈ fs := by
  induction fs with
  | nil => simp [firstDivergence]
  | cons g rest ih =>
    by_cases h : i g = t g
    · simp only [firstDivergence, h, ite_eq_left]
      intro hs
      exact ⟨(ih hs).1, by simp [(ih hs).2]⟩
    · simp only [firstDivergence, h, ite_eq_right, not_false_iff]
      intro hs
      cases hs
      exact ⟨h, by simp⟩

/-- The verdict of the whole gate. Every non-`bound` constructor is a refusal;
there is no downgrade to "verified". -/
inductive Verdict where
  | bound
  | envelopeAbsent
  | envelopeAmbiguous
  | envelopeUnterminated
  | templateMalformed
  | digestMismatch
  | divergent (f : Field)
  deriving DecidableEq, Repr

/-- The comparison, given a template already extracted and parsed. -/
def bind (installed template : Manifest) (executableDigest : Nat) : Verdict :=
  if installed.digest ≠ executableDigest then .digestMismatch
  else
    match firstDivergence allFields installed.value template.value with
    | some f => .divergent f
    | none => .bound

/-- The whole gate: locate, parse, compare. `parse` is opaque — JSON syntax and
manifest validation are outside this model. -/
def verify (executable : List Tok) (parse : List Tok → Option Manifest)
    (installed : Manifest) (executableDigest : Nat) : Verdict :=
  match extract executable with
  | .absent => .envelopeAbsent
  | .ambiguous => .envelopeAmbiguous
  | .unterminated => .envelopeUnterminated
  | .payload bytes =>
    match parse bytes with
    | none => .templateMalformed
    | some template => bind installed template executableDigest

/-! ## What the gate decides -/

/-- **Soundness and completeness of the comparison.** The gate binds exactly
when the installed digest is the executable's recomputed digest and *every*
declared field agrees. -/
theorem bind_eq_bound_iff (installed template : Manifest) (d : Nat) :
    bind installed template d = .bound ↔
      installed.digest = d ∧ ∀ f, installed.value f = template.value f := by
  constructor
  · intro h
    by_cases hd : installed.digest = d
    · refine ⟨hd, ?_⟩
      have : firstDivergence allFields installed.value template.value = none := by
        revert h
        simp only [bind, hd, ne_eq, not_true_eq_false, ite_eq_right, not_false_iff]
        cases hf : firstDivergence allFields installed.value template.value with
        | none => intro _; rfl
        | some f => simp
      intro f
      exact (firstDivergence_none_iff _ _ _).mp this f (mem_allFields f)
    · simp [bind, hd] at h
  · rintro ⟨hd, hf⟩
    have : firstDivergence allFields installed.value template.value = none :=
      (firstDivergence_none_iff _ _ _).mpr (fun f _ => hf f)
    simp [bind, hd, this]

/-- A divergence is reported by a field that really diverges: the diagnostic
names a fact, not a guess. -/
theorem divergent_names_a_real_divergence
    (installed template : Manifest) (d : Nat) (f : Field) :
    bind installed template d = .divergent f → installed.value f ≠ template.value f := by
  intro h
  by_cases hd : installed.digest = d
  · have : firstDivergence allFields installed.value template.value = some f := by
      revert h
      simp only [bind, hd, ne_eq, not_true_eq_false, ite_eq_right, not_false_iff]
      cases hf : firstDivergence allFields installed.value template.value with
      | none => simp
      | some g => intro hg; cases hg; rfl
    exact (firstDivergence_some _ _ _ _ this).1
  · simp [bind, hd] at h

/-- **Every declared field is compared.** For each field there is a schema-valid
pair differing only there, and the gate refuses it *by that field's name*. The
comparison is therefore not vacuous on any field. -/
theorem every_field_is_bound (f : Field) :
    bind ⟨0, fun g => if g = f then 1 else 0⟩ ⟨0, fun _ => 0⟩ 0 = .divergent f := by
  cases f <;> rfl

/-- **The template's own digest slot is never consulted.** An executable cannot
carry its own hash, so the build placeholder in the template cannot change the
verdict; the installed digest is bound to the recomputed hash instead. -/
theorem the_template_digest_is_not_consulted
    (installed template : Manifest) (d x : Nat) :
    bind installed template d = bind installed { template with digest := x } d := by
  rfl

/-- A digest edit is refused as the substituted field, not silently accepted. -/
theorem a_digest_edit_is_refused (installed template : Manifest) (d : Nat)
    (h : installed.digest ≠ d) : bind installed template d = .digestMismatch := by
  simp [bind, h]

/-- **Fail-closed on the envelope.** An executable whose template cannot be
located never binds, whatever manifest is installed. -/
theorem no_envelope_never_binds (executable : List Tok)
    (parse : List Tok → Option Manifest) (installed : Manifest) (d : Nat)
    (h : ∀ p, extract executable ≠ .payload p) :
    verify executable parse installed d ≠ .bound := by
  simp only [verify]
  cases hx : extract executable with
  | absent => simp
  | ambiguous => simp
  | unterminated => simp
  | payload p => exact absurd hx (h p)

/-- **Fail-closed on the payload.** A template that does not parse never binds. -/
theorem a_malformed_template_never_binds (executable : List Tok)
    (parse : List Tok → Option Manifest) (installed : Manifest) (d : Nat)
    (h : ∀ p, extract executable = .payload p → parse p = none) :
    verify executable parse installed d ≠ .bound := by
  simp only [verify]
  cases hx : extract executable with
  | absent => simp
  | ambiguous => simp
  | unterminated => simp
  | payload p => simp [h p hx]

/-- **The recorded gap, closed.** A schema-valid, digest-intact edit of any
declared field — the P01 audit's cases D and E — no longer verifies. -/
theorem a_digest_intact_declaration_edit_never_binds
    (executable : List Tok) (parse : List Tok → Option Manifest)
    (installed template : Manifest) (d : Nat) (p : List Tok)
    (hx : extract executable = .payload p) (hp : parse p = some template)
    (hf : ∃ f, installed.value f ≠ template.value f) :
    verify executable parse installed d ≠ .bound := by
  obtain ⟨f, hf⟩ := hf
  simp only [verify, hx, hp]
  intro h
  exact hf (((bind_eq_bound_iff _ _ _).mp h).2 f)

/-- **The honest package verifies.** An unmodified package — one envelope, a
template that parses, a matching digest and untouched declarations — binds. -/
theorem an_unmodified_package_binds
    (prefixBytes payload suffixBytes : List Tok)
    (hp : ∀ x ∈ prefixBytes, x.isMarker = false)
    (hb : ∀ x ∈ payload, x.isMarker = false)
    (hs : ∀ x ∈ suffixBytes, x.isMarker = false)
    (parse : List Tok → Option Manifest) (template : Manifest) (d : Nat)
    (hparse : parse payload = some template) :
    verify (envelope prefixBytes payload suffixBytes) parse
        ⟨d, template.value⟩ d = .bound := by
  simp [verify, extract_envelope prefixBytes payload suffixBytes hp hb hs, hparse,
    (bind_eq_bound_iff ⟨d, template.value⟩ template d).mpr ⟨rfl, fun _ => rfl⟩]

/-- A stripped envelope is refused distinctly, and not as a divergence. -/
theorem a_stripped_envelope_is_named_absent (executable : List Tok)
    (parse : List Tok → Option Manifest) (installed : Manifest) (d : Nat)
    (h : ∀ x ∈ executable, x.isMarker = false) :
    verify executable parse installed d = .envelopeAbsent := by
  simp [verify, extract_absent executable h]

/-- A second envelope pasted into the executable is refused as ambiguous: an
attacker cannot append a template that outvotes the real one. -/
theorem a_second_envelope_is_named_ambiguous
    (a b p q : List Tok)
    (ha : ∀ x ∈ a, x.isMarker = false) (hb : ∀ x ∈ b, x.isMarker = false)
    (hp : ∀ x ∈ p, x.isMarker = false) (hq : ∀ x ∈ q, x.isMarker = false)
    (parse : List Tok → Option Manifest) (installed : Manifest) (d : Nat) :
    verify (envelope a p [] ++ envelope b q []) parse installed d = .envelopeAmbiguous := by
  simp [verify, extract_duplicate_is_ambiguous a b p q ha hb hp hq]

/-- The refusal vocabulary is disjoint from acceptance: no refusal is a verdict
a caller may read as `bound`. -/
theorem refusals_are_not_acceptance :
    ∀ v : Verdict, v ≠ .bound →
      v = .envelopeAbsent ∨ v = .envelopeAmbiguous ∨ v = .envelopeUnterminated ∨
      v = .templateMalformed ∨ v = .digestMismatch ∨ ∃ f, v = .divergent f := by
  intro v h
  cases v with
  | bound => exact absurd rfl h
  | envelopeAbsent => simp
  | envelopeAmbiguous => simp
  | envelopeUnterminated => simp
  | templateMalformed => simp
  | digestMismatch => simp
  | divergent f => exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr ⟨f, rfl⟩))))

end QCore.PackageTemplateBinding
