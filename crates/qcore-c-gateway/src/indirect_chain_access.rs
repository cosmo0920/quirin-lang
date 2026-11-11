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

//! Card **W44-INDIRECT-CHAIN-ACCESS**: typed record-pointer member chains in
//! statements and controlling expressions.
//!
//! Card W43-INDIRECT-ACCESS-GENERAL's typed scalar access admits one selection
//! per access (`*B`, `B[I]`, `B->F`, `S.F`); W43-INDEX-TRAVERSAL's typed
//! condition read uses the same reader. This module supplies the reader for
//! a **chain** of selections through record pointers,
//!
//! ```text
//! R -> L1 -> … -> Ln -> F        ( * R ) . L1 -> … -> F        S . L1 -> … -> F
//! ```
//!
//! with between two and [`CHAIN_SELECTION_LIMIT`] selections, and the flow
//! obligation each dereferenced link carries. It is consulted only through
//! [`crate::typed_field_access::typed_scalar_statement_with`] and
//! [`crate::typed_field_access::typed_scalar_condition_with`], and only where
//! the existing classifier and the single-selection typed route have both
//! refused, so a statement or condition any older route admits keeps its
//! exact lowering.
//!
//! ## Premises of the reader (syntactic, from the unit's own declarations)
//!
//! 1. **Typed root.** `R` is declared exactly `struct T *` (for `->` and for
//!    `( * R ) .`), or `S` is declared `struct T` (a record object, for a
//!    leading `.`), by this function's declaration reader.
//! 2. **Every link is exactly `struct POINTEE *`.** Each member selected and
//!    then dereferenced is [`TypedAccessModel::record_pointer_field`] of a
//!    `struct` defined exactly once in the unit with every member read, and
//!    the walk re-enters the model at `POINTEE`, where an incomplete,
//!    ambiguous or `union` record refuses. A `.` after the first selection
//!    (a nested aggregate sub-object), a subscript, a call through a member,
//!    `void *` and every typedef'd or multi-star pointer member refuse.
//! 3. **Scalar final member.** The last member is a
//!    [`TypedAccessModel::scalar_field`] of the last record and the access
//!    does not continue. A pointer-valued final member is a member-link or
//!    escape question this card does not decide: its store is refused, and
//!    its read keeps whatever older route (card O04-ARMS's opaque field read,
//!    card REACH-GATE's F4 chain) already admits it.
//! 4. **Bounded.** At most [`CHAIN_SELECTION_LIMIT`] selections, the bound
//!    card O04-ARMS set; a longer chain is refused, never truncated.
//!
//! The statement or condition as a whole must be one the typed scalar walker
//! admits with chains as additional operands: no call, no unary `&`, no
//! comma, no nested assignment, no increment except the statement's own, and
//! a scalar destination and value.
//!
//! ## The obligation of each link (flow-sensitive, [`chain_resolves`])
//!
//! A chain reads through `k` pointers: the root `R` (unless the root is a
//! record object `S`) and the value of each selected link. Each is decided
//! on an **owner** reading, *untracked* or *tracked* (one known live
//! identity):
//!
//! * **the root** must be either a name this body can never bind to a tracked
//!   identity ([`UntrackedBases::never_bound`], card C02-P premise 4) whose
//!   binding here is absent or non-null with no tracked target — the owner is
//!   then untracked — or exactly one known, non-null, live identity with an
//!   identity derivation and no nullable or unresolved-effect marker — the
//!   owner is then that identity. Anything else refuses. The root is also a
//!   base of the enclosing typed access, so it keeps W43's dereference
//!   obligation, its C02-P-ALIAS obligation and its `use_after_release`
//!   accusation unchanged;
//! * **a link of an untracked owner** is untracked. This is card C02-P's
//!   premise 3 (O04-ARMS): a member of an object no reaching path bound is a
//!   place this function never names, and the projection binds `->` places
//!   only under card O04-B08's aggregate store, whose base is body-acquired
//!   and therefore never a never-bound name. An intermediate object can only
//!   have been released here through a local bound by an opaque field read,
//!   and releasing an opaque-bound name reports `UnknownAlias`, so no such
//!   body reaches `Safe`. Nullness of such a link is outside the property,
//!   exactly as an unbound base's is (spec L18);
//! * **a link of a tracked owner** is a member the projection *can* bind, so
//!   it needs the projection's own evidence: the place `R->L1` must be bound,
//!   non-null, identity-derived and not unknown, and either carry exactly one
//!   live identity equal to the field link the admitted store recorded under
//!   the owner (card O04-READ-BOUND's conjuncts; the owner becomes that
//!   identity) or carry no target and no field link at all (the owner becomes
//!   untracked). A missing place binding, a possibly-null or unknown binding,
//!   a drifted field link and a released, escaped or unresolved child refuse;
//! * **a tracked owner beyond the first link** refuses: no place deeper than
//!   `BASE->FIELD` is ever bound, so the evidence cannot exist.
//!
//! A refused link records `UnknownAlias` naming the spelled prefix it failed
//! at. Admitted or refused, the chain changes no binding, status, marker,
//! custody, containment or field link: it only adds diagnostics. The
//! statement's scalar destination and its `*B`/`B[I]` frame are the enclosing
//! typed access's, unchanged.
//!
//! What is **not** decided: bounds, alignment, constness and the liveness or
//! nullness of any object the projection does not track (the scope string
//! and spec L18). The Lean model is `QCore.IndirectChainAccess`.

use std::collections::BTreeSet;

use crate::borrowed_actual::UntrackedBases;
use crate::typed_field_access::{
    DeclaredType, FunctionTypes, TypedAccessModel, TypedScalarPlan, is_name,
};

/// Maximum number of member selections in one admitted chain: card O04-ARMS's
/// `borrowed_actual::ACCESS_CHAIN_LIMIT`, repeated by card REACH-GATE. A longer
/// chain is refused.
pub(crate) const CHAIN_SELECTION_LIMIT: usize = 4;

/// One admitted chain's obligation, carried by
/// `LegacyInstruction::IndirectScalarAccess::chains`.
///
/// `pub` only because `LegacyInstruction` is; the module is crate-private.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AccessChain {
    /// The root name: a record pointer `R` or a record object `S`.
    pub root: String,
    /// Whether the root itself is dereferenced (`R ->`, `( * R ) .`), as
    /// opposed to a record object whose member is selected (`S .`).
    pub dereferenced_root: bool,
    /// `UntrackedBases::never_bound` for the root, read by the classifier.
    pub root_never_bound: bool,
    /// Card **CG10-BOUND-UNTRACKED-ROOTS** (rule 11d): the root is eligible
    /// for the per-path bound-untracked premise
    /// ([`crate::bound_untracked::eligible_root`]); the flow decides it
    /// ([`ChainView::bound_untracked`]). False on every route but the typed
    /// place reader's, and never read where `root_never_bound` holds.
    pub root_bound_untracked: bool,
    /// Card **CG10-BOUND-UNTRACKED-ROOTS** (rule 11d): the root must read as
    /// untracked (never bound, or the per-path premise); a tracked root
    /// refuses. Set only for a pointer read handed to a `Borrowed` actual on
    /// a root rule 10k(3)'s never-bound premise does not cover, whose
    /// tracked reading that rule does not relax.
    pub root_untracked_only: bool,
    /// The members selected and then dereferenced, in order: every selection
    /// but the last.
    pub links: Vec<String>,
    /// The scalar member read or written.
    pub member: String,
}

impl AccessChain {
    /// The spelling of the root followed by its first `selections` members,
    /// `R->L1->L2` or `S.L1->L2`, used to name a refused link.
    pub(crate) fn prefix(&self, selections: usize) -> String {
        let mut spelled = self.root.clone();
        for (index, field) in self.links.iter().take(selections).enumerate() {
            spelled.push_str(if index == 0 && !self.dereferenced_root {
                "."
            } else {
                "->"
            });
            spelled.push_str(field);
        }
        spelled
    }
}

/// Why the reader refused a chain-shaped access. Read by the census only; the
/// rule treats every refusal alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChainRefusal {
    /// Not a chain of two or more selections at this position.
    NotAChain,
    /// The root is not a name this function declares.
    RootUndeclared,
    /// The root is declared, but not as the record pointer or record object
    /// its selector requires.
    RootUntyped,
    /// A record on the chain is incomplete, ambiguous or a `union`.
    IncompleteRecord,
    /// A link member is `void *`.
    VoidLink,
    /// A link member is another single-level pointer (a typedef'd or scalar
    /// pointee) rather than exactly `struct POINTEE *`.
    UntypedPointerLink,
    /// A `->` follows a member that is not a pointer at all.
    NonPointerLink,
    /// A `.` after the first selection: a nested aggregate sub-object.
    DotLink,
    /// A subscript on the chain.
    Subscript,
    /// A call through a member.
    Call,
    /// More than [`CHAIN_SELECTION_LIMIT`] selections.
    DepthLimit,
    /// A malformed selection (no member name).
    Malformed,
}

impl ChainRefusal {
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NotAChain => "not-a-chain",
            Self::RootUndeclared => "root-undeclared",
            Self::RootUntyped => "root-untyped",
            Self::IncompleteRecord => "incomplete-record",
            Self::VoidLink => "void-link",
            Self::UntypedPointerLink => "untyped-pointer-link",
            Self::NonPointerLink => "non-pointer-link",
            Self::DotLink => "dot-link",
            Self::Subscript => "subscript",
            Self::Call => "call",
            Self::DepthLimit => "depth-limit",
            Self::Malformed => "malformed",
        }
    }
}

/// What the final member of a parsed chain is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemberKind {
    Scalar,
    /// A single-level pointer member (never admitted here).
    Pointer,
    /// An aggregate, array, qualified or unread member.
    Other,
}

impl MemberKind {
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Pointer => "pointer",
            Self::Other => "other",
        }
    }
}

/// One chain the reader parsed at a position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedChain {
    /// The index just past the chain.
    pub(crate) end: usize,
    pub(crate) root: String,
    pub(crate) dereferenced_root: bool,
    pub(crate) links: Vec<String>,
    pub(crate) member: String,
    pub(crate) member_kind: MemberKind,
}

impl ParsedChain {
    /// The number of member selections.
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(crate) fn selections(&self) -> usize {
        self.links.len() + 1
    }
}

/// The root of a chain-shaped access at `index`: its name, whether it is
/// dereferenced, its record tag, and the index of the first member.
fn parse_root(
    texts: &[&str],
    index: usize,
    types: &FunctionTypes,
) -> Result<(String, bool, String, usize), ChainRefusal> {
    let at = |offset: usize| texts.get(index + offset).copied();
    let (root, dereferenced, first_field) = if at(0) == Some("(")
        && at(1) == Some("*")
        && at(2).is_some_and(is_name)
        && at(3) == Some(")")
        && at(4) == Some(".")
    {
        (at(2).unwrap_or_default(), true, index + 5)
    } else if let Some(name) = at(0).filter(|text| is_name(text)) {
        match (at(1), at(2)) {
            (Some("-"), Some(">")) => (name, true, index + 3),
            (Some("."), _) => (name, false, index + 2),
            _ => return Err(ChainRefusal::NotAChain),
        }
    } else {
        return Err(ChainRefusal::NotAChain);
    };
    let tag = match (types.declared_type(root), dereferenced) {
        (Some(DeclaredType::RecordPointer(tag)), true) => tag.clone(),
        (Some(DeclaredType::Record(tag)), false) => tag.clone(),
        (Some(_), _) => return Err(ChainRefusal::RootUntyped),
        (None, _) => return Err(ChainRefusal::RootUndeclared),
    };
    Ok((root.to_owned(), dereferenced, tag, first_field))
}

/// Parse one member chain of two or more selections starting at `index`.
///
/// `Err(NotAChain)` for a position that does not start a chain-shaped access
/// at all, or starts a single selection (the single-selection reader's). A
/// root that is not typed as its selector requires refuses before the depth
/// is known, so a single selection over an untyped root also reads as a
/// refusal here; the rule treats every refusal alike.
pub(crate) fn parse_chain(
    texts: &[&str],
    index: usize,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Result<ParsedChain, ChainRefusal> {
    let (root, dereferenced_root, mut tag, mut field_index) = parse_root(texts, index, types)?;
    let mut links: Vec<String> = Vec::new();
    loop {
        let field = texts
            .get(field_index)
            .copied()
            .filter(|field| is_name(field))
            .ok_or(ChainRefusal::Malformed)?;
        let after = field_index + 1;
        let arrow = texts.get(after) == Some(&"-") && texts.get(after + 1) == Some(&">");
        if arrow {
            if !model.record_definition_is_complete(&tag) {
                return Err(ChainRefusal::IncompleteRecord);
            }
            let Some(pointee) = model.record_pointer_field(&tag, field) else {
                return Err(if model.void_pointer_field(&tag, field) {
                    ChainRefusal::VoidLink
                } else if model.single_pointer_field(&tag, field) {
                    ChainRefusal::UntypedPointerLink
                } else {
                    ChainRefusal::NonPointerLink
                });
            };
            links.push(field.to_owned());
            if links.len() + 1 > CHAIN_SELECTION_LIMIT {
                return Err(ChainRefusal::DepthLimit);
            }
            tag = pointee.to_owned();
            field_index = after + 2;
            continue;
        }
        match texts.get(after).copied() {
            Some(".") => return Err(ChainRefusal::DotLink),
            Some("[") => return Err(ChainRefusal::Subscript),
            Some("(") => return Err(ChainRefusal::Call),
            _ => {}
        }
        // A leading `S .` whose member is followed by `.`/`[`/`(` refused
        // above; a single selection is the single-selection reader's.
        if links.is_empty() {
            return Err(ChainRefusal::NotAChain);
        }
        if !model.record_definition_is_complete(&tag) {
            return Err(ChainRefusal::IncompleteRecord);
        }
        let member_kind = if model.scalar_field(&tag, field) {
            MemberKind::Scalar
        } else if model.borrowable_field(&tag, field) {
            MemberKind::Pointer
        } else {
            MemberKind::Other
        };
        return Ok(ParsedChain {
            end: after,
            root,
            dereferenced_root,
            links,
            member: field.to_owned(),
            member_kind,
        });
    }
}

/// The access extension handed to the typed walker: a chain ending in a
/// scalar member.
fn scalar_chain_end(
    texts: &[&str],
    index: usize,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<usize> {
    parse_chain(texts, index, model, types)
        .ok()
        .filter(|chain| chain.member_kind == MemberKind::Scalar)
        .map(|chain| chain.end)
}

/// Every scalar chain in an admitted run, left to right. The walker admitted
/// the run with chains as operands, so every chain in it is one of these;
/// rereading the whole run can only find more obligations, never fewer.
fn scalar_chains(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Vec<AccessChain> {
    let mut chains = Vec::new();
    let mut index = 0usize;
    while index < texts.len() {
        match parse_chain(texts, index, model, types) {
            Ok(chain) if chain.member_kind == MemberKind::Scalar => {
                index = chain.end;
                chains.push(AccessChain {
                    root_never_bound: bases.never_bound(&chain.root),
                    root_bound_untracked: false,
                    root_untracked_only: false,
                    root: chain.root,
                    dereferenced_root: chain.dereferenced_root,
                    links: chain.links,
                    member: chain.member,
                });
            }
            _ => index += 1,
        }
    }
    chains
}

/// What an admitted chain statement or condition asks of the flow analysis:
/// the enclosing typed access's plan (with each dereferenced chain root among
/// its bases) and every chain's link obligation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChainPlan {
    pub(crate) plan: TypedScalarPlan,
    pub(crate) chains: Vec<AccessChain>,
}

fn with_chains(
    mut plan: TypedScalarPlan,
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Option<ChainPlan> {
    let chains = scalar_chains(texts, model, types, bases);
    if chains.is_empty() {
        return None;
    }
    for chain in chains.iter().filter(|chain| chain.dereferenced_root) {
        if !plan.bases.contains(&chain.root) {
            plan.bases.push(chain.root.clone());
        }
        if !plan.record_bases.contains(&chain.root) {
            plan.record_bases.push(chain.root.clone());
        }
    }
    Some(ChainPlan { plan, chains })
}

/// A complete statement the typed scalar route admits once scalar member
/// chains are operands too, carrying at least one chain.
pub(crate) fn chain_statement(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Option<ChainPlan> {
    let extension = |texts: &[&str], index: usize| scalar_chain_end(texts, index, model, types);
    let plan =
        crate::typed_field_access::typed_scalar_statement_with(texts, model, types, &extension)?;
    with_chains(plan, texts, model, types, bases)
}

/// A complete controlling expression the typed condition read admits once
/// scalar member chains are operands too, carrying at least one chain.
pub(crate) fn chain_condition(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    bases: &UntrackedBases,
) -> Option<ChainPlan> {
    let extension = |texts: &[&str], index: usize| scalar_chain_end(texts, index, model, types);
    let plan =
        crate::typed_field_access::typed_scalar_condition_with(texts, model, types, &extension)?;
    with_chains(plan, texts, model, types, bases)
}

/// One binding as the flow analysis holds it, read through [`ChainView`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct LinkBinding<'a> {
    pub(crate) targets: &'a BTreeSet<usize>,
    pub(crate) unknown: bool,
    pub(crate) may_be_null: bool,
    /// The derivation is exactly the identity (never derived, stepped or
    /// mixed).
    pub(crate) identity: bool,
}

/// The read-only view of one abstract state [`chain_resolves`] needs,
/// implemented by `legacy_c`'s state.
pub(crate) trait ChainView {
    /// The binding of a name or of a `BASE->FIELD` place, if any.
    fn binding(&self, place: &str) -> Option<LinkBinding<'_>>;
    /// Whether `site`'s status is exactly live, with neither a nullable nor
    /// an unresolved-effect marker.
    fn exactly_live(&self, site: usize) -> bool;
    /// The field link an admitted store recorded under `owner` for `field`.
    fn field_link(&self, owner: usize, field: &str) -> Option<&BTreeSet<usize>>;
    /// Card **CG10-BOUND-UNTRACKED-ROOTS** (rule 11d): whether the eligible
    /// root `root`, whose binding is absent or known target-free and non-null,
    /// is an untracked borrowed object on every path reaching this state
    /// ([`crate::bound_untracked::decide`]). The default reads nothing and
    /// answers no.
    fn bound_untracked(&self, _root: &str) -> bool {
        false
    }
}

/// The reading of the object a pointer on the chain designates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    /// An object no reaching path bound: its members are untracked.
    Untracked,
    /// Exactly one known live identity.
    Tracked(usize),
}

/// The single site of a non-null, known, identity binding with one target.
fn single_identity(binding: &LinkBinding<'_>) -> Option<usize> {
    (!binding.unknown && !binding.may_be_null && binding.identity && binding.targets.len() == 1)
        .then(|| binding.targets.first().copied())
        .flatten()
}

/// Card **W44-INDIRECT-CHAIN-ACCESS**: whether every pointer `chain` reads
/// through resolves in `view`; see the module header. `Err` names the spelled
/// prefix whose dereference refuses.
pub(crate) fn chain_resolves(view: &impl ChainView, chain: &AccessChain) -> Result<(), String> {
    chain_owner(view, chain).map(|_| ())
}

/// Card **CG11-POINTER-VALUED-STORES** (rule 10v): [`chain_resolves`]'s walk,
/// returning the owner of the chain's final member (the object it lives in).
/// `QCore.PointerStore.finalOwner`.
pub(crate) fn chain_owner(view: &impl ChainView, chain: &AccessChain) -> Result<Owner, String> {
    let root = || chain.root.clone();
    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): asked only where the
    // never-bound premise fails, for an eligible root with no tracked
    // identity; every other answer below is unchanged.
    let bound_untracked = || {
        !chain.root_never_bound && chain.root_bound_untracked && view.bound_untracked(&chain.root)
    };
    let mut owner = if chain.dereferenced_root {
        match view.binding(&chain.root) {
            None if chain.root_never_bound => Owner::Untracked,
            None if bound_untracked() => Owner::Untracked,
            None => return Err(root()),
            Some(binding) if binding.may_be_null => return Err(root()),
            Some(binding) if binding.targets.is_empty() => {
                if binding.unknown && (chain.root_never_bound || bound_untracked()) {
                    Owner::Untracked
                } else {
                    return Err(root());
                }
            }
            // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): an actual rule
            // 10k(3) left to the flow reads only an untracked root.
            Some(_) if chain.root_untracked_only => return Err(root()),
            Some(binding) => match single_identity(&binding) {
                Some(site) if view.exactly_live(site) => Owner::Tracked(site),
                _ => return Err(root()),
            },
        }
    } else if chain.root_never_bound {
        // A record object of this frame that no statement binds, addresses
        // or hands to a custody position: its members are untracked.
        Owner::Untracked
    } else {
        return Err(root());
    };
    for (depth, field) in chain.links.iter().enumerate() {
        owner = match owner {
            Owner::Untracked => Owner::Untracked,
            Owner::Tracked(_) if depth > 0 || !chain.dereferenced_root => {
                return Err(chain.prefix(depth + 1));
            }
            Owner::Tracked(site) => {
                let place = chain.prefix(1);
                let Some(binding) = view.binding(&place) else {
                    return Err(place);
                };
                let link = view.field_link(site, field);
                if binding.may_be_null || !binding.identity {
                    return Err(place);
                }
                if binding.targets.is_empty() {
                    if binding.unknown && link.is_none() {
                        Owner::Untracked
                    } else {
                        return Err(place);
                    }
                } else {
                    match single_identity(&binding) {
                        Some(child)
                            if link == Some(binding.targets) && view.exactly_live(child) =>
                        {
                            Owner::Tracked(child)
                        }
                        _ => return Err(place),
                    }
                }
            }
        };
    }
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use qcore_schemas::contract::ResourceContracts;

    use super::*;

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphanumeric() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    const RECORDS: &str = "struct c { int n; char *s; void *v; struct c *self; }; \
         struct b { struct c *c; int k; struct c inner; void *v; char *text; }; \
         struct a { struct b *b; int m; }; union u { int x; }; \
         struct d { union u *u; }; struct loop { struct loop *next; int v; };";

    fn facts<'a>(
        parameters: &'a str,
        body: &'a str,
    ) -> (TypedAccessModel, FunctionTypes, UntrackedBases) {
        let model = TypedAccessModel::read(&texts(RECORDS));
        let parameters = texts(parameters);
        let body_texts = texts(body);
        let types = FunctionTypes::read(&model, &parameters, &body_texts, &BTreeSet::new());
        let bases = crate::borrowed_actual::untracked_bases(
            &body_texts,
            &BTreeSet::new(),
            &ResourceContracts::default(),
            &BTreeSet::new(),
            &model,
            &types,
        );
        (model, types, bases)
    }

    fn statement(parameters: &str, body: &str, statement: &str) -> Option<ChainPlan> {
        let (model, types, bases) = facts(parameters, body);
        chain_statement(&texts(statement), &model, &types, &bases)
    }

    const PARAMETERS: &str = "struct a * p , struct loop * l , int x";
    const BODY: &str = "int y ; struct b local ; y = 0 ;";

    #[test]
    fn chain_stores_reads_increments_and_compound_stores_are_admitted() {
        for source in [
            "p -> b -> k = x",
            "p -> b -> c -> n = 1",
            "p -> b -> c -> n += x",
            "p -> b -> c -> n ++",
            "-- p -> b -> k",
            "y = p -> b -> c -> n + p -> m",
            "( * p ) . b -> k = 2",
            "local . c -> n = y",
            "l -> next -> next -> next -> v = 0",
        ] {
            let plan = statement(PARAMETERS, BODY, source)
                .unwrap_or_else(|| panic!("`{source}` is admitted"));
            assert!(!plan.chains.is_empty(), "{source}");
        }
    }

    #[test]
    fn the_plan_names_the_root_as_a_base_and_the_links_in_order() {
        let plan = statement(PARAMETERS, BODY, "y = p -> b -> c -> n + p -> m").unwrap();
        assert_eq!(plan.plan.bases, vec!["p".to_owned()]);
        assert_eq!(plan.plan.record_bases, vec!["p".to_owned()]);
        assert_eq!(plan.plan.destination.as_deref(), Some("y"));
        assert!(!plan.plan.writes_through_pointer);
        assert_eq!(
            plan.chains,
            vec![AccessChain {
                root: "p".to_owned(),
                dereferenced_root: true,
                root_never_bound: true,
                root_bound_untracked: false,
                root_untracked_only: false,
                links: vec!["b".to_owned(), "c".to_owned()],
                member: "n".to_owned(),
            }]
        );
        let object = statement(PARAMETERS, BODY, "local . c -> n = y").unwrap();
        assert!(object.plan.bases.is_empty());
        assert_eq!(object.chains[0].prefix(1), "local.c");
    }

    #[test]
    fn refused_shapes_are_not_admitted() {
        for source in [
            // A pointer-valued final member: a member-link question.
            "p -> b -> c -> s = 0",
            "p -> b -> text = 0",
            // A nested aggregate sub-object, a subscript, a call.
            "p -> b -> inner . n = 1",
            "p -> b [ 0 ] . k = 1",
            // `void *` and a pointer to a scalar are not record-pointer links.
            "p -> b -> v -> n = 1",
            "p -> b -> text -> n = 1",
            // More than four selections.
            "l -> next -> next -> next -> next -> v = 0",
            // A call, an address, a nested write, a comma.
            "y = f ( p -> b -> k )",
            "y = & p -> b -> k",
            "p -> b -> k = y = 1",
            // Increments of the root pointer.
            "p ++ -> b -> k = 1",
        ] {
            assert!(
                statement(PARAMETERS, BODY, source).is_none(),
                "`{source}` is refused"
            );
        }
        // A union pointee refuses at the link.
        let (model, types, bases) = facts("struct d * q", "");
        assert!(chain_statement(&texts("q -> u -> x = 1"), &model, &types, &bases).is_none());
    }

    #[test]
    fn a_single_selection_is_not_this_rule() {
        assert!(statement(PARAMETERS, BODY, "p -> m = 1").is_none());
        assert!(statement(PARAMETERS, BODY, "local . k = 1").is_none());
        assert_eq!(
            parse_chain(
                &texts("p -> m"),
                0,
                &facts(PARAMETERS, BODY).0,
                &facts(PARAMETERS, BODY).1
            ),
            Err(ChainRefusal::NotAChain)
        );
    }

    #[test]
    fn a_root_the_body_may_bind_is_marked() {
        let plan = statement(
            PARAMETERS,
            "int y ; struct a * q ; q = make ( ) ; y = 0 ;",
            "y = q -> b -> k",
        )
        .unwrap();
        assert!(!plan.chains[0].root_never_bound);
    }

    #[test]
    fn conditions_admit_chains_beside_pointer_operands() {
        let (model, types, bases) = facts(PARAMETERS, BODY);
        for source in [
            "p -> b -> k",
            "p -> b -> c -> n != 0 && p -> m",
            "p != ( void * ) 0 && p -> b -> k > y",
        ] {
            assert!(
                chain_condition(&texts(source), &model, &types, &bases).is_some(),
                "{source}"
            );
        }
        for source in ["p -> b -> c -> s", "p -> b -> k ++", "p -> m"] {
            assert!(
                chain_condition(&texts(source), &model, &types, &bases).is_none(),
                "{source}"
            );
        }
    }

    /// A state for the obligation: bindings by place, live sites, field links.
    #[derive(Default)]
    struct View {
        bindings: BTreeMap<String, (BTreeSet<usize>, bool, bool, bool)>,
        live: BTreeSet<usize>,
        links: BTreeMap<(usize, String), BTreeSet<usize>>,
    }

    impl ChainView for View {
        fn binding(&self, place: &str) -> Option<LinkBinding<'_>> {
            self.bindings
                .get(place)
                .map(|(targets, unknown, may_be_null, identity)| LinkBinding {
                    targets,
                    unknown: *unknown,
                    may_be_null: *may_be_null,
                    identity: *identity,
                })
        }
        fn exactly_live(&self, site: usize) -> bool {
            self.live.contains(&site)
        }
        fn field_link(&self, owner: usize, field: &str) -> Option<&BTreeSet<usize>> {
            self.links.get(&(owner, field.to_owned()))
        }
    }

    fn chain(never_bound: bool, links: &[&str]) -> AccessChain {
        AccessChain {
            root: "r".to_owned(),
            dereferenced_root: true,
            root_never_bound: never_bound,
            root_bound_untracked: false,
            root_untracked_only: false,
            links: links.iter().map(|link| (*link).to_owned()).collect(),
            member: "n".to_owned(),
        }
    }

    fn one(site: usize) -> BTreeSet<usize> {
        BTreeSet::from([site])
    }

    #[test]
    fn an_unbound_never_bound_root_resolves_at_every_depth() {
        let view = View::default();
        assert_eq!(chain_resolves(&view, &chain(true, &["b"])), Ok(()));
        assert_eq!(
            chain_resolves(&view, &chain(true, &["b", "c", "d"])),
            Ok(())
        );
    }

    #[test]
    fn a_root_the_body_may_bind_needs_one_live_identity() {
        let mut view = View::default();
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r".to_owned())
        );
        view.bindings
            .insert("r".to_owned(), (BTreeSet::new(), true, false, true));
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r".to_owned())
        );
        view.bindings
            .insert("r".to_owned(), (one(1), false, false, true));
        // Live owner, but the member place is not bound: no evidence.
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r".to_owned())
        );
        view.live.insert(1);
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r->b".to_owned())
        );
    }

    #[test]
    fn a_nullable_or_released_root_refuses() {
        let mut view = View::default();
        view.bindings
            .insert("r".to_owned(), (BTreeSet::new(), true, true, true));
        assert_eq!(
            chain_resolves(&view, &chain(true, &["b"])),
            Err("r".to_owned())
        );
        view.bindings
            .insert("r".to_owned(), (one(1), false, false, true));
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r".to_owned())
        );
    }

    #[test]
    fn a_tracked_owner_needs_the_stored_link_and_a_live_child() {
        let mut view = View::default();
        view.bindings
            .insert("r".to_owned(), (one(1), false, false, true));
        view.live.insert(1);
        view.bindings
            .insert("r->b".to_owned(), (one(2), false, false, true));
        // No field link recorded: the binding drifted.
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r->b".to_owned())
        );
        view.links.insert((1, "b".to_owned()), one(2));
        // The child is not live.
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r->b".to_owned())
        );
        view.live.insert(2);
        assert_eq!(chain_resolves(&view, &chain(false, &["b"])), Ok(()));
        // A tracked owner beyond the first link has no place to bind.
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b", "c"])),
            Err("r->b->c".to_owned())
        );
        // A possibly-null member place refuses.
        view.bindings
            .insert("r->b".to_owned(), (one(2), false, true, true));
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b"])),
            Err("r->b".to_owned())
        );
    }

    #[test]
    fn an_untracked_non_null_member_of_a_tracked_owner_is_untracked() {
        let mut view = View::default();
        view.bindings
            .insert("r".to_owned(), (one(1), false, false, true));
        view.live.insert(1);
        view.bindings
            .insert("r->b".to_owned(), (BTreeSet::new(), true, false, true));
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b", "c", "d"])),
            Ok(())
        );
        view.links.insert((1, "b".to_owned()), one(3));
        assert_eq!(
            chain_resolves(&view, &chain(false, &["b", "c"])),
            Err("r->b".to_owned())
        );
    }

    #[test]
    fn a_record_object_root_must_never_be_bound() {
        let view = View::default();
        let mut object = chain(true, &["c"]);
        object.dereferenced_root = false;
        assert_eq!(chain_resolves(&view, &object), Ok(()));
        object.root_never_bound = false;
        assert_eq!(chain_resolves(&view, &object), Err("r".to_owned()));
    }
}
