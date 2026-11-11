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

//! Card **CG10-BOUND-UNTRACKED-ROOTS** (gateway rule 11d): a typed place on a
//! root the body may bind but, on every path reaching the access, has not
//! bound and has not handed to a custody position.
//!
//! ## The gap
//!
//! Rule 10t (card CG10-COMMON-TYPED-PLACE) lowers a mixed place on a pointer
//! root to the chain obligation with `root_never_bound` false, and
//! [`crate::indirect_chain_access::chain_owner`] reads such a root only as
//! exactly one known, non-null, live identity. A formal or local the custody
//! closure cannot prove never bound (it is an actual at a position no
//! contract declares `Borrowed`, or assigned from a call) but which carries no
//! tracked identity was therefore refused at every access, even on a path
//! before anything could have taken it.
//!
//! ## The premise ([`decide`])
//!
//! A **separate**, weaker-scoped premise beside the never-bound one. The root
//! is read as an untracked, borrowed object at an access only when
//!
//! 1. it is **eligible** ([`eligible_root`], static, from the Clang AST): a
//!    pointer declared exactly once in the function as a parameter or an
//!    automatic local, every reference naming that declaration, and its
//!    address never taken (`&R`: class (d) stays refused);
//! 2. the state entering the access gives it **no tracked identity** on any
//!    reaching path: no binding at all (a join with a bound path yields a
//!    binding, so absence means every path left it unbound), or a known
//!    non-null binding with no target, not widened (a borrowed alias, a
//!    derived result of an untracked base once null-tested);
//! 3. **no place rooted at it is bound** (`R->F`, `R.F`, `R[…]`, `*R`): no
//!    admitted store recorded a fact about a member the untracked reading
//!    would forget;
//! 4. **no path reaching the access was refused or left an unresolved effect**
//!    (the state's `unmodeled` flag, joined by disjunction). Every
//!    custody-taking position that receives an object with no tracked
//!    identity reports a refusal there (`Consumed`, `Released`: the unknown
//!    identity; `Owned`, `Stored`, `Removed`: `require_live_resource`), and
//!    so does every call to an uncontracted function, so on every reaching
//!    path no custody position has received the root (or any alias of it)
//!    before the access. A `Borrowed` actual reports nothing and changes no
//!    binding: the premise survives it.
//!
//! Under the premise the access carries exactly the per-hop obligations the
//! chain model already states for an untracked owner: every link of an
//! untracked owner is untracked (C02-P premise 3, here backed by condition 3
//! rather than the never-bound closure).
//!
//! ## The interlock
//!
//! The never-bound answer is unchanged for every root: the premise is asked
//! only where [`crate::indirect_chain_access::chain_owner`] had refused an
//! unbound or target-free root the closure does not prove never bound, and a
//! root with a tracked identity keeps the tracked reading. Rules 10l, 10o and
//! 10t are untouched.
//!
//! The Lean model is `QCore.BoundUntracked`; its vectors are
//! [`DECISION_VECTORS`] and [`EVENT_VECTORS`].

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// How the state entering an access binds the root (census and decision).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingClass {
    /// No binding: no reaching path bound the root.
    Absent,
    /// A known-untracked value: no target, the `unknown` flag of a borrowed or
    /// derived pointer with no tracked base, not possibly null, not widened.
    TargetFree,
    /// Possibly null.
    Nullable,
    /// A target-free binding this rule cannot read as untracked: the state
    /// widened (targets forgotten), a mixed derivation, or a known value with
    /// no target: it may denote a tracked identity or is no pointer read.
    Unresolved,
    /// One or more tracked targets: the tracked reading decides.
    Tracked,
}

impl BindingClass {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::TargetFree => "target-free",
            Self::Nullable => "nullable",
            Self::Unresolved => "unresolved",
            Self::Tracked => "tracked",
        }
    }
}

/// The premise's answer at one access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Premise {
    /// The root is an untracked, borrowed object on every reaching path.
    Holds,
    /// Not a root this rule reads (address taken, shadowed, not one
    /// parameter or automatic local, not a pointer).
    Ineligible,
    /// The binding is not an untracked one (possibly null, widened, tracked).
    Bound,
    /// A place rooted at the root carries a binding.
    RootedPlace,
    /// Some reaching path passed a refusal or an unresolved effect: a custody
    /// position may have received the root.
    PathRefused,
}

impl Premise {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Holds => "holds",
            Self::Ineligible => "ineligible",
            Self::Bound => "bound",
            Self::RootedPlace => "rooted-place",
            Self::PathRefused => "path-refused",
        }
    }
}

/// The premise: a pure function of the four facts. `QCore.BoundUntracked.decide`
/// is the same function.
pub(crate) fn decide(
    eligible: bool,
    binding: BindingClass,
    rooted: bool,
    path_refused: bool,
) -> Premise {
    if !eligible {
        Premise::Ineligible
    } else if !matches!(binding, BindingClass::Absent | BindingClass::TargetFree) {
        Premise::Bound
    } else if rooted {
        Premise::RootedPlace
    } else if path_refused {
        Premise::PathRefused
    } else {
        Premise::Holds
    }
}

/// The decision vectors shared with `proof/QCore/BoundUntracked.lean`
/// (`decisionVectors`): eligible, binding, rooted, path refused, premise.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, &str, bool, bool, &str)] = &[
    (true, "absent", false, false, "holds"),
    (true, "target-free", false, false, "holds"),
    (false, "absent", false, false, "ineligible"),
    (true, "nullable", false, false, "bound"),
    (true, "unresolved", false, false, "bound"),
    (true, "tracked", false, false, "bound"),
    (true, "absent", true, false, "rooted-place"),
    (true, "absent", false, true, "path-refused"),
    (true, "target-free", false, true, "path-refused"),
    (false, "tracked", true, true, "ineligible"),
];

/// One event on a path, as the flow meets it (`QCore.BoundUntracked.Event`),
/// and the premise after a path of them, starting from an absent root. The
/// Rust flow's own arms are what the vectors bind: `tests/unit` runs each
/// path through the gateway and compares the verdict.
#[cfg(test)]
pub(crate) const EVENT_VECTORS: &[(&[&str], &str)] = &[
    (&[], "holds"),
    (&["borrowed"], "holds"),
    (&["borrowed", "borrowed"], "holds"),
    (&["released"], "path-refused"),
    (&["consumed"], "path-refused"),
    (&["stored"], "path-refused"),
    (&["owned"], "path-refused"),
    (&["uncontracted"], "path-refused"),
    (&["released", "borrowed"], "path-refused"),
    (&["uncontracted", "rebind-none"], "path-refused"),
    (&["rebind-none"], "holds"),
    (&["rebind-derived"], "bound"),
    (&["rebind-derived", "null-test"], "holds"),
    (&["rebind-owned"], "bound"),
    (&["member-store"], "rooted-place"),
];

/// Whether `name` (declared by `decl`) is a root this rule reads in
/// `function`: a pointer declared exactly once as a parameter or an automatic
/// local, every `DeclRefExpr` of its spelling naming that declaration, and no
/// `&` of it anywhere in the function.
pub(crate) fn eligible_root(function: &Value, name: &str, decl: &str) -> bool {
    let mut declarations = Vec::new();
    let mut foreign = false;
    let mut addressed = false;
    visit(function, &mut |node| match kind(node) {
        "ParmVarDecl" | "VarDecl" if node["name"].as_str() == Some(name) => {
            declarations.push((
                node["id"].as_str().map(str::to_owned),
                node["storageClass"].as_str().map(str::to_owned),
                node["type"]["qualType"].as_str().map(str::to_owned),
            ));
        }
        "DeclRefExpr"
            if node["referencedDecl"]["name"].as_str() == Some(name)
                && node["referencedDecl"]["id"].as_str() != Some(decl) =>
        {
            foreign = true;
        }
        "UnaryOperator"
            if node["opcode"].as_str() == Some("&")
                && children(node)
                    .first()
                    .and_then(names_declaration)
                    .is_some_and(|id| id == decl) =>
        {
            addressed = true;
        }
        _ => {}
    });
    !foreign
        && !addressed
        && matches!(declarations.as_slice(),
            [(Some(id), None, Some(ty))] if id == decl && ty.trim_end().ends_with('*'))
}

/// The declaration a `&` operand names through parentheses and no-op
/// conversions, if it is a plain name.
fn names_declaration(mut node: &Value) -> Option<&str> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("NoOp") => {}
            "DeclRefExpr" => return node["referencedDecl"]["id"].as_str(),
            _ => return None,
        }
        node = children(node).first()?;
    }
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// Whether `place` (a binding key of the flow state) is a place rooted at
/// `root` other than the name itself: `root->…`, `root.…`, `root[…]`,
/// `*root`, `(*root)…`.
pub(crate) fn rooted_at(place: &str, root: &str) -> bool {
    if place == root {
        return false;
    }
    let trimmed = place.trim_start_matches(['*', '(', '&', ' ']);
    let prefixed = trimmed.len() != place.len();
    trimmed.strip_prefix(root).is_some_and(|rest| {
        rest.starts_with("->")
            || rest.starts_with('.')
            || rest.starts_with('[')
            || (prefixed && (rest.is_empty() || rest.starts_with(')')))
    }) || crate::may_alias::place_root(place) == Some(root)
}

#[cfg(test)]
#[path = "../tests/unit/bound_untracked.rs"]
mod tests;
