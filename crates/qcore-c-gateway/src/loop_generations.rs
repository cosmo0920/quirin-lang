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

//! Card **CG04-LOOP-SITE-GENERATIONS**: generation-indexed acquisition sites.
//!
//! The custody flow names a resource by the CFG node that acquires it (its
//! *site*). A node on a CFG cycle runs more than once, so one site used to
//! stand for every object the node ever produced: the re-acquisition wrote
//! the site `LIVE` over a live earlier generation, and a refuted nullable
//! acquisition on the failure edge then removed the site, and with it the
//! earlier, still live, generation. `for (;;) { p = malloc(n); if (p == NULL)
//! break; use(p); }` was reported `Safe` while every earlier generation leaks.
//!
//! This module decides what a re-acquisition does, and names the
//! generations. Each acquisition site has three identities:
//!
//! * the **current** generation, the site itself: the object the node
//!   produced last on the path, one object per execution;
//! * the **previous** generation ([`previous`]): the object the node produced
//!   the time before, also one object per execution, so every precise update
//!   the flow makes to a site it may make to it;
//! * the **older** generations ([`older`]): a summary of every generation
//!   before that. It may stand for many objects, so no precise update may
//!   discharge it: every binding to it is unknown, it is never removed by a
//!   null refinement, and it is widened towards "possibly live", never
//!   towards "discharged".
//!
//! At a re-acquisition (the site is already present in the state the node
//! runs in) the generations shift: previous is merged into older, current is
//! renamed previous, and the node then creates a fresh current. A refuted
//! nullable acquisition on the failure edge removes only the fresh current
//! generation.
//!
//! The merge into the summary is refused ([`Step::ShiftRefused`], `Unknown`)
//! when the merged summary may be live and a place that can later update it
//! without an unknown binding still holds it (an aggregate edge: a field
//! link or a containment; a pending reallocation; a publication into a
//! container; an out slot), or when one side may be live and the other
//! carries an unresolved effect, which would hide it once merged. Every input
//! of the decision only grows with the state, so it is monotone. An
//! unknown binding refuses every custody effect through it on its own, so a
//! binding that still names the summary needs no refusal here: either the
//! program discharges through it later (refused there) or it never does, and
//! a live summary at an exit is the leak the source has.
//!
//! The Lean model is `QCore.LoopSiteGenerations`
//! (`proof/QCore/LoopSiteGenerations.lean`); [`STEP_VECTORS`] and
//! [`IDENTITY_VECTORS`] are shared with it.

/// The flow keeps an acquisition site's generations apart. Rule 10x's
/// placement guard reads it: a custody split on a CFG cycle was refused only
/// because it did not.
pub(crate) const SITES_ARE_GENERATION_INDEXED: bool = true;

/// The tag of a previous-generation identity.
pub(crate) const PREVIOUS_TAG: usize = 1 << 29;
/// The tag of an older-generations summary identity.
pub(crate) const OLDER_TAG: usize = 1 << 30;

/// Which generation an identity names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Generation {
    Current,
    Previous,
    Older,
}

/// Whether `site` is an acquisition site the tags can index (every site a
/// CFG produces is far below the tags; a larger one is refused).
pub(crate) const fn indexable(site: usize) -> bool {
    site < PREVIOUS_TAG
}

/// The previous-generation identity of an acquisition site.
pub(crate) const fn previous(site: usize) -> usize {
    site | PREVIOUS_TAG
}

/// The older-generations summary identity of an acquisition site.
pub(crate) const fn older(site: usize) -> usize {
    site | OLDER_TAG
}

/// The acquisition site an identity is a generation of.
pub(crate) const fn base(identity: usize) -> usize {
    identity & !(PREVIOUS_TAG | OLDER_TAG)
}

/// Which generation `identity` names.
pub(crate) const fn generation(identity: usize) -> Generation {
    if identity & OLDER_TAG != 0 {
        Generation::Older
    } else if identity & PREVIOUS_TAG != 0 {
        Generation::Previous
    } else {
        Generation::Current
    }
}

/// Whether `identity` is a summary that may stand for many objects.
pub(crate) const fn is_summary(identity: usize) -> bool {
    matches!(generation(identity), Generation::Older)
}

/// What the flow knows, at a node that acquires a site, about that site's
/// earlier generations in the state the node runs in. Every field only grows
/// as the state grows (a status bit, a holder, an unresolved effect), so the
/// decision is monotone and the settled state decides as every iterate did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Earlier {
    /// The site is present: an earlier generation reaches this node, so the
    /// node lies on a CFG cycle.
    pub(crate) present: bool,
    /// The previous generation may be live.
    pub(crate) previous_live: bool,
    /// A place other than a local binding holds the previous generation.
    pub(crate) previous_held: bool,
    /// The previous generation carries an unresolved effect.
    pub(crate) previous_unresolved: bool,
    /// The older summary may be live.
    pub(crate) older_live: bool,
    /// A place other than a local binding holds the older summary.
    pub(crate) older_held: bool,
    /// The older summary carries an unresolved effect.
    pub(crate) older_unresolved: bool,
}

/// What a node does to its site's generations before it acquires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// No earlier generation: the acquisition is the site's first here.
    Fresh,
    /// Shift the generations; nothing is refused.
    Shift,
    /// Shift the generations, and refuse: the merged summary may owe an
    /// obligation, and a place that can update it precisely holds it, or an
    /// unresolved effect on the other side would hide it.
    ShiftRefused,
}

impl Step {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Shift => "shift",
            Self::ShiftRefused => "shift-refused",
        }
    }
}

/// The decision for one re-acquisition (`QCore.LoopSiteGenerations.decide`):
/// refused when the merged summary may be live and a holder could update it
/// precisely, or when one side may be live and the other's unresolved effect
/// would hide it once merged.
pub(crate) const fn decide(earlier: Earlier) -> Step {
    let Earlier {
        present,
        previous_live,
        previous_held,
        previous_unresolved,
        older_live,
        older_held,
        older_unresolved,
    } = earlier;
    if !present {
        Step::Fresh
    } else if ((previous_live || older_live) && (previous_held || older_held))
        || (previous_live && older_unresolved)
        || (older_live && previous_unresolved)
    {
        Step::ShiftRefused
    } else {
        Step::Shift
    }
}

/// `(present, previous_live, previous_held, previous_unresolved, older_live,
/// older_held, older_unresolved)` and the step.
#[cfg(test)]
pub(crate) type StepVector = ((bool, bool, bool, bool, bool, bool, bool), &'static str);

/// Shared with `QCore.LoopSiteGenerations.stepVectors`, row by row.
#[cfg(test)]
pub(crate) const STEP_VECTORS: &[StepVector] = &[
    ((false, false, false, false, false, false, false), "fresh"),
    ((false, true, true, false, true, true, false), "fresh"),
    ((true, false, false, false, false, false, false), "shift"),
    ((true, true, false, false, false, false, false), "shift"),
    ((true, false, false, false, true, false, false), "shift"),
    ((true, true, false, false, true, false, false), "shift"),
    ((true, false, true, false, false, true, false), "shift"),
    ((true, false, false, true, false, false, true), "shift"),
    ((true, true, false, true, false, false, false), "shift"),
    (
        (true, true, false, true, true, false, true),
        "shift-refused",
    ),
    (
        (true, true, true, false, false, false, false),
        "shift-refused",
    ),
    (
        (true, true, false, false, false, true, false),
        "shift-refused",
    ),
    (
        (true, false, true, false, true, false, false),
        "shift-refused",
    ),
    (
        (true, false, false, false, true, true, false),
        "shift-refused",
    ),
    (
        (true, true, false, false, false, false, true),
        "shift-refused",
    ),
    (
        (true, false, false, true, true, false, false),
        "shift-refused",
    ),
    (
        (true, true, true, false, true, true, false),
        "shift-refused",
    ),
];

/// `(identity, base, generation code)`, shared with
/// `QCore.LoopSiteGenerations.identityVectors` (0 current, 1 previous,
/// 2 older).
#[cfg(test)]
pub(crate) const IDENTITY_VECTORS: &[(usize, usize, u8)] = &[
    (0, 0, 0),
    (7, 7, 0),
    (previous(7), 7, 1),
    (older(7), 7, 2),
    (previous(0), 0, 1),
    (older(0), 0, 2),
    (older(12_345), 12_345, 2),
];

/// The generation code the identity vectors use.
#[cfg(test)]
pub(crate) const fn generation_code(generation: Generation) -> u8 {
    match generation {
        Generation::Current => 0,
        Generation::Previous => 1,
        Generation::Older => 2,
    }
}

#[cfg(test)]
#[path = "../tests/unit/loop_generations.rs"]
mod tests;
