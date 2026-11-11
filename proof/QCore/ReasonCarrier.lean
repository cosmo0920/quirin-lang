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

import QCore.ReasonCategory

/-! Card `GEN-REASON` (follow-ons `REASON-CODES-B1`/`B2`/`B3`): how a reason row
*carries* its closed §L21.1 category.

`QCore.ReasonCategory` models the classification relation — which category a
reason text has. This module models the orthogonal question the B-follow-ons
raise: a row's category may be recovered from its text, or **named at the site
that minted it** because the text is opaque OS/tool output that no closed
vocabulary can recognise. Two carriers therefore exist in the Rust:

* `UnitRefusal::classified` / `ChunkSourceResult::unknown` — recover the
  category from the text (`crates/qcore-audit/src/report.rs`,
  `crates/qcore-c-gateway/src/boolean_source.rs`);
* `UnitRefusal::environment` — name it at the site
  (`fs::read` in `crates/qcore-audit/src/current.rs`, and the `canonicalize`
  and `git show` failures in `crates/qcore-audit/src/lib.rs`).

The property that makes the second carrier safe to add is not that it is
*more* informative but that it can only be *less* claiming: its category is
`environmentUnavailable`, the sole inhabitant of `Subject.environment`, so a
site-named row is never readable as a statement about the analysed source and
never supports a provisional `Unsafe` — exactly as the `none` it replaces
asserted nothing. That is what is proved below, together with the
non-widening comparison against the text route and the preservation of
fail-closed behaviour on the text route itself.

What is **not** proved here, and stays a Rust boundary: that the three named
opaque sites are the only ones on this route (a code census, pinned by
`crates/qcore-audit/src/report.rs`'s unit tests), and that no site names a
source-subject category for opaque text — which the Rust makes unconstructible
by offering no such constructor, rather than by proof. -/
namespace QCore.ReasonCarrier

open QCore.ReasonCategory QCore.ReasonCategory.Category

/-- How a row's category reached it.

`textual` recovers the category from the reason text through the unchanged
closed vocabulary; `environmentSite` is a site that names its category because
its text is opaque. There is deliberately **no** constructor that names a
source-subject category at a site: the Rust mirrors this by publishing only
`UnitRefusal::environment`, so the missing case is missing by construction on
both sides. -/
inductive Carrier where
  | textual : Input → Carrier
  | environmentSite
  deriving DecidableEq, Repr

/-- The category a carrier publishes, mirroring
`reason_code_value(refusal.category())`. -/
def published : Carrier → Option Category
  | .textual i => classify i
  | .environmentSite => some .environmentUnavailable

/-- The text route is the unchanged classification relation: adding the second
carrier introduced no second vocabulary. -/
theorem the_text_route_is_the_unchanged_classification (i : Input) :
    published (.textual i) = classify i := rfl

/-- The site route publishes exactly one category. -/
theorem the_site_route_publishes_environment_unavailable :
    published .environmentSite = some .environmentUnavailable := rfl

/-- Fail-closed is preserved on the text route: an unrecognised reason still
publishes no category rather than a defaulted one. -/
theorem an_unrecognised_text_still_publishes_no_category :
    published (.textual .unrecognised) = none := rfl

/-- …and in particular publishes no source-subject category. -/
theorem an_unrecognised_text_publishes_no_source_statement (c : Category) :
    published (.textual .unrecognised) = some c → describesAnalysedSource c = false := by
  simp [published, classify]

/-- The site route never publishes a statement about the analysed source. This
is the whole safety argument for naming a category at an opaque site. -/
theorem the_site_route_never_states_anything_about_the_source (c : Category)
    (h : published .environmentSite = some c) : describesAnalysedSource c = false := by
  simp only [published, Option.some.injEq] at h
  subst h
  rfl

/-- The site route never supports a provisional `Unsafe`. -/
theorem the_site_route_never_supports_unsafe (c : Category)
    (h : published .environmentSite = some c) : maySupportUnsafe c = false := by
  simp only [published, Option.some.injEq] at h
  subst h
  rfl

/-- The site route's subject is the environment, which is the sole reason it
may be named without inspecting the text. -/
theorem the_site_route_speaks_about_the_environment (c : Category)
    (h : published .environmentSite = some c) : subject c = Subject.environment := by
  simp only [published, Option.some.injEq] at h
  subst h
  rfl

/-- **Non-widening.** Replacing an opaque row's `none` with the site-named
category does not turn a row that claimed nothing about the source into one
that does: neither shape describes the analysed source. -/
theorem naming_at_the_site_never_widens_a_source_claim :
    (∀ c, published (.textual .unrecognised) = some c → describesAnalysedSource c = false) ∧
      ∀ c, published .environmentSite = some c → describesAnalysedSource c = false := by
  refine ⟨?_, ?_⟩
  · simp [published, classify]
  · exact the_site_route_never_states_anything_about_the_source

/-- **Non-widening, for `Unsafe` support.** The same statement for the other
half of §L21.1 rule 1: neither shape can contribute to a provisional `Unsafe`. -/
theorem naming_at_the_site_never_widens_unsafe_support :
    (∀ c, published (.textual .unrecognised) = some c → maySupportUnsafe c = false) ∧
      ∀ c, published .environmentSite = some c → maySupportUnsafe c = false := by
  refine ⟨?_, ?_⟩
  · simp [published, classify]
  · exact the_site_route_never_supports_unsafe

/-- Publication is single-valued: a carrier is a function, so one row never
carries two categories. This is what lets a producer-supplied category be
republished instead of reclassified (`reason_code_value`). -/
theorem publication_is_single_valued {r : Carrier} {a b : Category}
    (ha : published r = some a) (hb : published r = some b) : a = b := by
  rw [ha] at hb; exact Option.some.inj hb

/-- The two carriers agree wherever both are defined on the same fact: if a
text classifies to `environmentUnavailable`, naming it at the site publishes
the same category. So a site that could have been left to the text route loses
nothing by naming, and gains nothing it was not entitled to. -/
theorem the_carriers_agree_on_environment_text (i : Input)
    (h : classify i = some .environmentUnavailable) :
    published (.textual i) = published .environmentSite := by
  simp [published, h]

/-- **Non-vacuity of the refusal theorems.** The text route really does reach a
source-subject category for some input, so the theorems above are not proved of
an empty set. -/
theorem the_text_route_can_publish_a_source_statement :
    ∃ i c, published (.textual i) = some c ∧ describesAnalysedSource c = true :=
  ⟨.known .unmodeledConstruct, .unsupportedSyntax, rfl, rfl⟩

/-- **Non-vacuity of the site route.** It publishes a category, not `none`; the
site theorems are therefore about a reachable shape. -/
theorem the_site_route_publishes_something :
    ∃ c, published .environmentSite = some c := ⟨.environmentUnavailable, rfl⟩

/-- Every carrier's publication is decided, so the model has no gap where a row
could carry an unstated category. -/
theorem publication_is_total (r : Carrier) :
    published r = none ∨ ∃ c, published r = some c := by
  cases h : published r with
  | none => exact Or.inl rfl
  | some c => exact Or.inr ⟨c, rfl⟩

/-- The only carrier that can publish `none` is the text route, and only on an
input the vocabulary refuses. A site-named row is never uncoded. -/
theorem only_a_refused_text_publishes_nothing {r : Carrier}
    (h : published r = none) : r = .textual .unrecognised := by
  cases r with
  | environmentSite => simp [published] at h
  | textual i =>
    cases i with
    | unrecognised => rfl
    | known f => simp [published, classify] at h

end QCore.ReasonCarrier
