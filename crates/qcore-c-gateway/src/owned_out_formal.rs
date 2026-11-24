// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card **OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY** (gateway rule 74l): an
//! owned result stored through a `*NAME` slot.
//!
//! The delivery reading the card's discharge asks for already exists: W4's
//! `Out { nullable }` parameter mode (`legacy_summary.rs`, `out_mode` and
//! condition (6) of `observations_match_class`). A body whose every reachable
//! exit leaves a fresh, live, non-null heap site in `*out` (or the null
//! constant, which makes the mode nullable) publishes an `Out` summary, and a
//! caller's `fill(&p)` binds the delivered site to `p`, so a caller that
//! ignores it leaks. Nothing here changes that reading.
//!
//! What this module owns is the **refusal** when no such summary or contract
//! sanctions the slot: a partial-path store, an in-out slot (`if (!*out) *out
//! = make();`), a store followed by a release or a second store, a
//! possibly-null result, a slot of a local pointer. The row's own analysis
//! then runs with no out slot, and the store refused before this card as
//! `discarded owned result of resource API \`F\``, the text a real discard
//! (`make();`) carries. The result was stored, not discarded; it is now
//! refused by name, still `unsupported`, so the row stays `Unknown` exactly as
//! before ([`unsanctioned_owned_store`]). The kind, the state poisoning and the
//! summary gate are unchanged: a body with the refusal publishes no summary,
//! so its callers keep `call to uncontracted function`.
//!
//! The Lean model is `QCore.OwnedOutFormalDelivery`; [`OUT_MODE_VECTORS`] are
//! its `outModeVectors`, run against `legacy_summary::out_mode` itself.

/// The refusal of a call whose owned result is stored through `place`, an
/// indirect place (`*NAME`) that no out-parameter contract or validated
/// summary in force sanctions. Worded as its sibling for a variable's store
/// (`store through \`*out\` without an out-parameter contract`).
pub(crate) fn unsanctioned_owned_store(called: &str, place: &str) -> String {
    format!(
        "owned result of resource API `{called}` stored through `{place}` without an out-parameter contract"
    )
}

/// The out-mode vectors shared with `proof/QCore/OwnedOutFormalDelivery.lean`
/// (`outModeVectors`): one out slot's observation at each reachable exit, and
/// the mode `out_mode` reads (`"out"`, `"out-nullable"`, or `None` for no
/// summary). An exit is `unwritten`, `null`, `ambiguous`, or
/// `site-<disposition>[-nullable]`.
#[cfg(test)]
pub(crate) const OUT_MODE_VECTORS: &[(&[&str], Option<&str>)] = &[
    // Every exit delivers: the four rows' sound counterpart.
    (&["site-live"], Some("out")),
    (&["site-live", "site-live"], Some("out")),
    (&["site-live", "null"], Some("out-nullable")),
    // A partial-path store: an exit leaves the slot as the caller had it.
    (&["site-live", "unwritten"], None),
    (&["unwritten"], None),
    // Only null stores deliver nothing.
    (&["null"], None),
    // A store followed by a release, or a delivery moved on.
    (&["site-released"], None),
    (&["site-live", "site-released"], None),
    (&["site-escaped"], None),
    (&["site-contained"], None),
    // A possibly-null result (`*out = maybe_make();`).
    (&["site-live-nullable"], None),
    // A slot holding another object's value, or several sites.
    (&["ambiguous"], None),
    (&["site-live", "ambiguous"], None),
    // No reachable exit.
    (&[], None),
];

#[cfg(test)]
#[path = "../tests/unit/owned_out_formal.rs"]
mod tests;
