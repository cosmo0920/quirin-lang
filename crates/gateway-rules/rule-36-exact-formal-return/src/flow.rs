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

//! Rule 36's exit flow classifier (`QCore.ExactFormalReturn.classify`).
//!
//! The host reads the returned name's binding out of its abstract state at
//! the `ExactFormalReturn` exit and asks [`classify`]. The exit admits only a
//! known identity: definitely NULL, or one status-free original formal site
//! with the matching index and no unresolved effect. A known non-null
//! untracked, unknown, mixed, derived, owned, released or unrelated binding
//! refuses; the host then adds `UnknownAlias` and its unresolved-effect
//! exposure without changing any resource status, binding or containment.

use std::collections::BTreeSet;

/// The classifier's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// The returned value is definitely NULL.
    Null,
    /// The returned value is exactly the original formal's entry site.
    Exact(usize),
    /// Anything else.
    Refuse,
}

impl Flow {
    /// The Lean constructor's spelling (`QCore.ExactFormalReturn.Flow`).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Exact(_) => "exact",
            Self::Refuse => "refuse",
        }
    }
}

/// The returned name's binding at the exit, as the host's abstract state
/// holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding<'b> {
    /// The binding is unknown.
    pub unknown: bool,
    /// The binding's derivation is the identity (no step, no mixing).
    pub identity: bool,
    /// The binding may be NULL.
    pub may_be_null: bool,
    /// The sites the binding may point to.
    pub targets: &'b BTreeSet<usize>,
}

/// `QCore.ExactFormalReturn.classify`. `binding` is `None` when the state
/// holds no binding for the returned name; `unresolved_effects` whether the
/// state has any unresolved effect site; `base_of(site)` the formal position
/// whose entry site `site` is (the entry custody's derived bases);
/// `tracked(site)` whether the state's ledger tracks a resource at `site`.
pub fn classify(
    binding: Option<Binding<'_>>,
    unresolved_effects: bool,
    parameter: usize,
    base_of: impl FnOnce(usize) -> Option<usize>,
    tracked: impl FnOnce(usize) -> bool,
) -> Flow {
    let Some(binding) = binding else {
        return Flow::Refuse;
    };
    if binding.unknown || !binding.identity || unresolved_effects {
        return Flow::Refuse;
    }
    if binding.targets.is_empty() {
        return if binding.may_be_null {
            Flow::Null
        } else {
            Flow::Refuse
        };
    }
    if binding.targets.len() == 1 {
        let site = *binding.targets.first().expect("one target");
        if base_of(site) == Some(parameter) && !tracked(site) {
            return Flow::Exact(site);
        }
    }
    Flow::Refuse
}

/// One flow vector: the binding (`None`: absent; else `unknown`, `identity`,
/// `may_be_null`, `targets`), whether an unresolved effect is pending, the
/// tracked sites, the parameter asked, and the expected answer. The entry
/// custody's derived bases are `{7 -> 1}` throughout.
pub type FlowVector = (
    Option<(bool, bool, bool, &'static [usize])>,
    bool,
    &'static [usize],
    usize,
    Flow,
);

/// The fourteen classifier cases shared with
/// `proof/QCore/ExactFormalReturn.lean` (`flowVectors` and
/// `shared_flow_vectors`), in its order. The ledger always tracks site 20;
/// rows 10-12 add site 7 (live, released, escaped in Lean: any tracked status
/// refuses).
pub const FLOW_VECTORS: [FlowVector; 14] = [
    (
        Some((false, true, true, &[7])),
        false,
        &[20],
        1,
        Flow::Exact(7),
    ),
    (Some((false, true, true, &[])), false, &[20], 1, Flow::Null),
    (
        Some((false, true, false, &[])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (None, false, &[20], 1, Flow::Refuse),
    (
        Some((true, true, true, &[7])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7, 8])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[8])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, false, true, &[7])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, false, true, &[7])),
        false,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7])),
        false,
        &[7, 20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7])),
        false,
        &[7, 20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7])),
        false,
        &[7, 20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7])),
        true,
        &[20],
        1,
        Flow::Refuse,
    ),
    (
        Some((false, true, true, &[7])),
        false,
        &[20],
        0,
        Flow::Refuse,
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Binding, FLOW_VECTORS, Flow, classify};

    #[test]
    fn the_shared_flow_vectors_classify_as_lean_proves() {
        let lean = [
            "exact", "null", "refuse", "refuse", "refuse", "refuse", "refuse", "refuse", "refuse",
            "refuse", "refuse", "refuse", "refuse", "refuse",
        ];
        for (index, (binding, unresolved, tracked, parameter, expected)) in
            FLOW_VECTORS.into_iter().enumerate()
        {
            let targets;
            let binding = match binding {
                Some((unknown, identity, may_be_null, sites)) => {
                    targets = sites.iter().copied().collect::<BTreeSet<_>>();
                    Some(Binding {
                        unknown,
                        identity,
                        may_be_null,
                        targets: &targets,
                    })
                }
                None => None,
            };
            let answer = classify(
                binding,
                unresolved,
                parameter,
                |site| (site == 7).then_some(1),
                |site| tracked.contains(&site),
            );
            assert_eq!(answer, expected, "vector {index}");
            assert_eq!(answer.code(), lean[index], "vector {index}");
        }
    }

    #[test]
    fn only_a_single_untracked_base_of_the_asked_formal_is_exact() {
        let one = BTreeSet::from([3]);
        let binding = Binding {
            unknown: false,
            identity: true,
            may_be_null: false,
            targets: &one,
        };
        let base = |site| (site == 3).then_some(2);
        assert_eq!(
            classify(Some(binding), false, 2, base, |_| false),
            Flow::Exact(3)
        );
        assert_eq!(
            classify(Some(binding), false, 1, base, |_| false),
            Flow::Refuse
        );
        assert_eq!(
            classify(Some(binding), false, 2, |_| None, |_| false),
            Flow::Refuse
        );
        assert_eq!(
            classify(Some(binding), false, 2, base, |_| true),
            Flow::Refuse
        );
    }
}
