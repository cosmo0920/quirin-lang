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

//! Card **W45-FB-RETRY-LIFETIME**: intrusive-container publication
//! (contract specification §C04.8, gateway constraint 10d).
//!
//! `mk_list_add(&node->_head, &queue->pending)` links the object `node` points
//! to into a list the caller reaches. Before this card the declaration read
//! both positions as `resource_borrowed`, so a body that allocated `node` and
//! linked it was still accused of a leak at the next exit
//! (`MK-LIST-ADD-BORROWED-PUBLICATION`), and a body that linked a formal's
//! object told its callers nothing had happened to it.
//!
//! A declaration now states the publication (`resource_published(entry,
//! container)`) and its retirement (`resource_unpublished(entry)`). The call
//! keeps every rule a borrowed call has; this module applies the effect after
//! it, under these rules:
//!
//! 1. **Publish.** The entry actual must name its object: `&B->F` (the object
//!    `B` points to) or a plain name `B`. The container actual must be reached
//!    from a formal of this function that the body never rebinds: `&P->F…` or
//!    `P` itself, so the list lives in memory the caller supplied and outlives
//!    the call. When `B` is bound to exactly one known, non-null, live
//!    identity with no unresolved effect, that identity's custody passes to
//!    the container: its status becomes `ESCAPED` and it is recorded as
//!    published. An exit then owes nothing for it.
//! 2. **Refusals.** Everything else that could move custody refuses, with an
//!    unresolved effect on what `B` may denote: an entry the reader cannot
//!    name, a container not reached from an unrebound formal (a local, an
//!    object this body acquired; a static object is rule 5's), a binding that is
//!    unknown, ambiguous, derived or possibly null, and an identity that is
//!    not live — including a name-only formal site, which is how a borrowed
//!    formal is seeded: publishing an object the body does not hold would
//!    otherwise let a derived summary call it borrowed (constraint 10's
//!    launder). An entry `B` bound to nothing is untracked, and the call keeps
//!    its borrowed reading, as before.
//! 3. **Release while published is a violation candidate.** A released
//!    published identity reports `release_after_escape` (the existing
//!    diagnostic of a release after a transfer). A `return` of it refuses
//!    rather than escaping again: the caller would receive a pointer into a
//!    container, not custody.
//! 4. **Unlink.** `resource_unpublished(entry)` restores direct custody only
//!    for an identity this path definitely published (the published set joins
//!    by intersection) and whose status is still exactly `ESCAPED`. Every other
//!    unlink refuses, an untracked entry included: unlinking an object the
//!    body does not hold hands it custody nobody accounts for.
//!
//! The Lean model is `QCore.ContainerPublication`.
//!
//! # Card W45-CONTAINER-ROOT-REACH
//!
//! Two widenings, each under a stated premise (gateway constraint 10d, second
//! paragraph; `QCore.ContainerRootReach`):
//!
//! 5. **Static containers** (P-CRR-STATIC). A container spelled `&G` or
//!    `&G.F…`, where `G` is an object of static storage duration this body
//!    does not shadow (a file-scope object or a `static` local), lives for the
//!    program: publishing into it discharges exactly as a caller's container
//!    does. A list link is additive, so no later store can lose the object the
//!    way constraint 10's overwritten global slot can. A pointer-valued global
//!    (`G->F`) is not static storage for its pointee and stays refused.
//! 6. **Foreign unlink** (P-CRR-MEMBER, P-CRR-LINKED). `mk_list_del(&E->F)` of
//!    an element `E` this body did not publish gives the body custody of the
//!    element when every write of `E` is an admitted `container_of` binding
//!    (rule 5l) over a container rooted at an unrebound formal, a local
//!    initialised from one, or a static object ([`ForeignOrigins`]), and `E`
//!    holds no binding at the unlink. The call is lowered with a fresh site
//!    for the element ([`settle_foreign_unlinks`]). The element is then live
//!    custody of this body: an exit that still holds it leaks it, a second
//!    release is a double release, and a publication moves it on. The
//!    premises: an object linked into a container is a heap object that
//!    container owns (P-CRR-MEMBER), and it is still linked there when the
//!    body unlinks it — no call between its binding and the unlink removed or
//!    released it (P-CRR-LINKED, the unlink-side twin of P-ILI-CALLEE-FRAME).
//!    A body with such an unlink publishes no summary and establishes no
//!    specification ([`has_foreign_unlink`]): a caller would read `Borrowed`
//!    for the container's formal although the body released what the
//!    container held, and the caller's own published objects may be among
//!    them.
//!
//! Every other container and entry keeps its refusal: a local list, a
//! container a callee returned, one rooted at an element or at a rebound
//! formal, a body-acquired owner, a formal's own object whose container is
//! unnamed, and an embedded element.
//!
//! # Card W45-SR5-SOUNDNESS
//!
//! Two refusals STABILITY-REEVAL-5 measured missing (`QCore.Sr5Soundness`):
//!
//! 7. **A released root** (`PUBLICATION-ROOT-RELEASED`). Rule 1's discharge
//!    rests on the container outliving the call. A body that also hands the
//!    container's root, whole, to a position its contract in force does not
//!    declare `Borrowed` (`q_free(q)`, a summary's `Released`) may end that
//!    storage itself, and whether the container's release hands on its
//!    elements is a premise no rule states. Such a publication refuses as
//!    `Other` ([`CallerFormals::with_released_roots`]); a local initialised
//!    from the formal alone is the same object. Rule 6 is unchanged: a
//!    teardown unlinks elements and then releases the root.
//! 8. **An element bound over a formal**
//!    (`CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER`). Rule 5l(2) binds
//!    `ctx = container_of(arg, …)` over a never-bound formal and leaves `ctx`
//!    untracked, so a publication of `&ctx->F` took the borrowed reading and a
//!    derived summary called `arg` `Borrowed` while the body linked it: rule
//!    1's launder. Such an entry now takes a formal entry's rule, an
//!    unresolved identity ([`ForeignOrigins::note_over_formal`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ContainerEffect, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
};

use super::{
    AbstractState, Derivation, ESCAPED, LIVE, LegacyDiagnostic, LegacyDiagnosticKind,
    LegacyInstruction, mark_unresolved_effect,
};

/// The object an entry actual designates.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Entry {
    /// `&B->F` or `B`: the object `B` points to.
    Name(String),
    /// Any other spelling.
    Unnamed(String),
}

/// Where a published object's container lives.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Container {
    /// Reached from this formal, which the body never rebinds.
    CallerFormal(String),
    /// Card W45-CONTAINER-ROOT-REACH: an object of static storage duration
    /// (`&G`, `&G.F…`), which lives for the program (P-CRR-STATIC).
    Static(String),
    /// Anything else, spelled as written.
    Other(String),
}

/// One container effect at one call. `formal` records that the entry names a
/// formal of the function, whose object the caller holds.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Operand {
    Published {
        entry: Entry,
        container: Container,
        formal: bool,
    },
    Unpublished {
        entry: Entry,
        /// Card W45-CONTAINER-ROOT-REACH: set by [`settle_foreign_unlinks`]
        /// when the entry is an element of an admitted container.
        foreign: Option<Foreign>,
        /// Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway rule 46): the link
        /// member an element destroyer's declaration names
        /// (`resource_unpublished(p, MEMBER)`), checked by [`link_member`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
    },
    /// Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): a derived
    /// summary's `ElementsReleased` at this position: the call may release
    /// what the container the actual reaches holds. `root` is the formal of
    /// this function the container is rooted at (through a derived local),
    /// when it is one; this body's own summary must then carry the clause for
    /// that formal ([`element_release`]).
    ElementsReleased {
        container: Container,
        root: Option<String>,
    },
}

/// An admitted foreign unlink: the fresh site the element's custody takes, and
/// the containers its bindings reach it through, as spelled.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Foreign {
    pub site: usize,
    pub containers: Vec<String>,
    /// Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): the formals of this
    /// function those containers are rooted at, when every one is (a stable
    /// formal, or a derived local's root formal); `None` when one is a static
    /// object, which no summary clause can name.
    #[serde(default)]
    pub roots: Option<BTreeSet<String>>,
    /// Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway rule 46): the member
    /// every `container_of` binding of the element names
    /// (`mk_list_entry(it, T, MEMBER)`), the link of the list it is reached
    /// through.
    #[serde(default)]
    pub members: BTreeSet<String>,
}

/// Every container effect of one call, in parameter order.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Operands(pub Vec<Operand>);

/// The names of one body a container may be reached from.
///
/// * `stable`: the formals no statement rebinds — never the destination of an
///   assignment, an increment or a decrement, and never address-taken;
/// * `derived`: the locals whose one write is their declaration's initialiser
///   `L = P` or `L = P->F…` with `P` stable, and which are never address-taken:
///   they name memory the caller supplied exactly as `P` does;
/// * `all`: every formal, rebound or not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallerFormals {
    stable: BTreeSet<String>,
    derived: BTreeSet<String>,
    all: BTreeSet<String>,
    /// Card W45-CONTAINER-ROOT-REACH: the names whose object has static
    /// storage duration and which this body does not shadow
    /// (`PublicationFacts::destinations`).
    statics: BTreeSet<String>,
    /// Card W45-SR5-SOUNDNESS: each derived local whose initialiser is a
    /// stable formal itself (`L = P`), mapped to that formal: one object.
    aliases: BTreeMap<String, String>,
    /// Card W45-SR5-SOUNDNESS (`PUBLICATION-ROOT-RELEASED`): the stable and
    /// derived names this body hands whole to a position its contract in
    /// force does not declare `Borrowed` ([`CallerFormals::with_released_roots`]).
    released: BTreeSet<String>,
    /// Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): each derived local
    /// mapped to the stable formal its initialiser is rooted at (`L = P` or
    /// `L = P->F…`).
    derived_roots: BTreeMap<String, String>,
}

const WRITES: [&str; 12] = [
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "++",
];

/// The indices at which `name` is written or has its address taken. A token
/// after `->` or `.` is a member name, never the variable.
fn writes(body: &[&str], name: &str) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter(|(index, token)| {
            let previous = index.checked_sub(1).and_then(|at| body.get(at)).copied();
            let next = body.get(index + 1).copied();
            // `&P->F`, `&P.F` and `&P[I]` take a member's address, not `P`'s.
            let member = matches!(next, Some("->" | "." | "["));
            **token == name
                && !matches!(previous, Some("->" | "."))
                && (next.is_some_and(|next| WRITES.contains(&next) || next == "--")
                    || matches!(previous, Some("++" | "--"))
                    || (previous == Some("&") && !member))
        })
        .map(|(index, _)| index)
        .collect()
}

impl CallerFormals {
    /// Read from the body's own tokens.
    pub fn read(body: &[&str], formals: &BTreeSet<String>) -> Self {
        let fused = fuse(body);
        let body = fused
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let stable = formals
            .iter()
            .filter(|name| writes(&body, name).is_empty())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut derived = BTreeSet::new();
        let mut aliases = BTreeMap::new();
        let mut derived_roots = BTreeMap::new();
        for (index, token) in body.iter().enumerate() {
            if !is_identifier(token)
                || formals.contains(*token)
                || derived.contains(*token)
                || body.get(index + 1) != Some(&"=")
            {
                continue;
            }
            // A declarator: preceded by a type word or `*`, not an expression.
            let declarator = index
                .checked_sub(1)
                .and_then(|at| body.get(at))
                .is_some_and(|previous| {
                    *previous == "*"
                        || (is_identifier(previous)
                            && !matches!(*previous, "return" | "else" | "do" | "case" | "goto"))
                });
            // Exactly one write, at this token.
            if !declarator || writes(&body, token) != [index] {
                continue;
            }
            // `= P ;` or `= P -> F ( -> F )* ;` with `P` stable.
            let value = &body[index + 2..];
            let end = value.iter().position(|text| *text == ";");
            let initialiser = end.map(|end| &value[..end]);
            let from_stable = initialiser.is_some_and(|value| match value {
                [root, rest @ ..] => {
                    stable.contains(*root)
                        && rest.len() % 2 == 0
                        && rest
                            .chunks(2)
                            .all(|pair| pair[0] == "->" && is_identifier(pair[1]))
                }
                [] => false,
            });
            if declarator && from_stable {
                derived.insert((*token).to_owned());
                if let Some([root]) = initialiser {
                    aliases.insert((*token).to_owned(), (*root).to_owned());
                }
                if let Some([root, ..]) = initialiser {
                    derived_roots.insert((*token).to_owned(), (*root).to_owned());
                }
            }
        }
        Self {
            stable,
            derived,
            all: formals.clone(),
            statics: BTreeSet::new(),
            aliases,
            released: BTreeSet::new(),
            derived_roots,
        }
    }

    /// Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): the formal a
    /// container reached from `name` is rooted at: `name` itself when it is a
    /// stable formal, a derived local's root formal, `None` otherwise.
    pub fn formal_root(&self, name: &str) -> Option<String> {
        if self.stable.contains(name) {
            return Some(name.to_owned());
        }
        self.derived_roots
            .get(name)
            .filter(|root| self.stable.contains(*root))
            .cloned()
    }

    /// Card W45-SR5-SOUNDNESS (`PUBLICATION-ROOT-RELEASED`): the same reading,
    /// with the container roots this body releases. A stable formal or a
    /// derived local handed whole, as a value (not `P->F`, `P.F` or `P[I]`),
    /// to a call position whose contract in force is not `Borrowed` (a
    /// release, a consumption, a derived summary's `Released`, a variadic
    /// tail) may end the container's storage in this very body, so a
    /// publication into a container reached from it no longer shows that the
    /// container outlives the call, and refuses (rule 1's `Other`). A formal
    /// and a local initialised from it alone are one object. An uncontracted
    /// call refuses on its own and is not read here. Stated, not proved: a
    /// declared `Borrowed` position does not end the storage of what it
    /// receives (P-B08-SHALLOW).
    pub fn with_released_roots(mut self, body: &[&str], contracts: &ResourceContracts) -> Self {
        let fused = fuse(body);
        let body = fused
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let roots = self
            .stable
            .iter()
            .chain(&self.derived)
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut released = BTreeSet::new();
        for (index, token) in body.iter().enumerate() {
            if !is_identifier(token) || body.get(index + 1) != Some(&"(") {
                continue;
            }
            let Some(contract) = contracts.contract(token) else {
                continue;
            };
            let Some(close) = matching_close(&body, index + 1) else {
                continue;
            };
            for (position, argument) in split_top_level(&body[index + 2..close])
                .into_iter()
                .enumerate()
            {
                if matches!(
                    contract.parameters.get(position),
                    Some(ResourceParameterMode::Borrowed)
                ) {
                    continue;
                }
                for (at, word) in argument.iter().enumerate() {
                    let previous = at.checked_sub(1).map(|p| argument[p]);
                    let next = argument.get(at + 1).copied();
                    if roots.contains(*word)
                        && !matches!(previous, Some("->" | "."))
                        && !matches!(next, Some("->" | "." | "["))
                    {
                        released.insert((*word).to_owned());
                    }
                }
            }
        }
        for (local, root) in &self.aliases {
            if released.contains(local) || released.contains(root) {
                released.insert(local.clone());
                released.insert(root.clone());
            }
        }
        self.released = released;
        self
    }

    /// Card W45-CONTAINER-ROOT-REACH: the same reading, with the body's
    /// static-storage names (P-CRR-STATIC). A formal or a derived local of
    /// the same name shadows it.
    pub fn with_statics(mut self, statics: &BTreeSet<String>) -> Self {
        self.statics = statics
            .iter()
            .filter(|name| !self.all.contains(*name) && !self.derived.contains(*name))
            .cloned()
            .collect();
        self
    }

    /// Whether `name` is a formal of this function, rebound or not.
    pub fn is_formal(&self, name: &str) -> bool {
        self.all.contains(name)
    }

    /// Whether `name` is an unrebound formal.
    pub fn contains(&self, name: &str) -> bool {
        self.stable.contains(name)
    }

    /// Whether a container reached from `name` lives in memory the caller
    /// supplied.
    fn reaches_caller(&self, name: &str) -> bool {
        self.stable.contains(name) || self.derived.contains(name)
    }

    /// Card W45-SR5-SOUNDNESS: a publication's container, withdrawn to
    /// `Other` when its root is one this body releases. Only a publication
    /// reads it: rule 6's foreign unlink takes an element out of the
    /// container, which a teardown that then releases the root may do.
    fn unreleased(&self, container: Container, argument: &[&str]) -> Container {
        match container {
            Container::CallerFormal(root) if self.released.contains(&root) => Container::Other(
                fuse(argument)
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            other => other,
        }
    }
}

/// C's multi-character punctuators, fused from the gateway's one-character
/// punctuation tokens, each with the index of its first token. Longest match
/// first, so `<<=` is one token and `- >` is `->`.
pub(crate) fn fuse(texts: &[&str]) -> Vec<(String, usize)> {
    const THREE: [&str; 3] = ["<<=", ">>=", "..."];
    const TWO: [&str; 19] = [
        "->", "++", "--", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=",
        "|=", "^=", "<<", ">>",
    ];
    let single = |text: &str| {
        text.len() == 1 && !text.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    };
    let mut fused = Vec::with_capacity(texts.len());
    let mut index = 0;
    while index < texts.len() {
        let joined = |count: usize| -> Option<String> {
            let run = texts.get(index..index + count)?;
            run.iter().all(|text| single(text)).then(|| run.concat())
        };
        if let Some(three) = joined(3).filter(|text| THREE.contains(&text.as_str())) {
            fused.push((three, index));
            index += 3;
        } else if let Some(two) = joined(2).filter(|text| TWO.contains(&text.as_str())) {
            fused.push((two, index));
            index += 2;
        } else {
            fused.push((texts[index].to_owned(), index));
            index += 1;
        }
    }
    fused
}

/// The index of the `)` closing the `(` at `open` in fused tokens.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" => depth += 1,
            ")" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The top-level comma-separated operands of an argument list.
fn split_top_level<'a>(texts: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut operands = vec![Vec::new()];
    let mut depth = 0usize;
    for text in texts {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                operands.push(Vec::new());
                continue;
            }
            _ => {}
        }
        operands.last_mut().expect("one operand").push(*text);
    }
    if operands.len() == 1 && operands[0].is_empty() {
        operands.clear();
    }
    operands
}

fn is_identifier(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && token.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn entry(argument: &[&str]) -> Entry {
    let fused = fuse(argument);
    let argument = fused
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<Vec<_>>();
    match argument.as_slice() {
        ["&", base, "->", field] if is_identifier(base) && is_identifier(field) => {
            Entry::Name((*base).to_owned())
        }
        [base] if is_identifier(base) => Entry::Name((*base).to_owned()),
        _ => Entry::Unnamed(argument.join(" ")),
    }
}

fn container(argument: &[&str], formals: &CallerFormals) -> Container {
    let fused = fuse(argument);
    let argument = fused
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<Vec<_>>();
    let root = match argument.as_slice() {
        [root] => Some(*root),
        ["&", root, "->", rest @ ..]
            if !rest.is_empty()
                && rest.chunks(2).enumerate().all(|(index, pair)| match pair {
                    [field] => index * 2 + 1 == rest.len() && is_identifier(field),
                    [field, "->"] => is_identifier(field),
                    _ => false,
                }) =>
        {
            Some(*root)
        }
        _ => None,
    };
    match root {
        Some(root) if formals.reaches_caller(root) => Container::CallerFormal(root.to_owned()),
        _ => match static_root(&argument) {
            Some(root) if formals.statics.contains(root) => Container::Static(root.to_owned()),
            _ => Container::Other(argument.join(" ")),
        },
    }
}

/// `& G` or `& G . F ( . F )*`: the root of an address inside a static
/// object's own storage, never one reached through a pointer.
fn static_root<'a>(argument: &[&'a str]) -> Option<&'a str> {
    match argument {
        ["&", root] if is_identifier(root) => Some(*root),
        ["&", root, ".", rest @ ..]
            if is_identifier(root)
                && !rest.is_empty()
                && rest.chunks(2).enumerate().all(|(index, pair)| match pair {
                    [field] => index * 2 + 1 == rest.len() && is_identifier(field),
                    [field, "."] => is_identifier(field),
                    _ => false,
                }) =>
        {
            Some(*root)
        }
        _ => None,
    }
}

/// The operands of one call whose contract in force has a container effect;
/// `None` for every other call, which is every call in an environment that
/// declares no container effect.
pub fn operands(
    contract: &ResourceFunctionContract,
    arguments: Option<&[&[&str]]>,
    formals: &CallerFormals,
) -> Option<Box<Operands>> {
    if !contract.has_container_effect() {
        return None;
    }
    let argument = |index: usize| -> &[&str] {
        arguments
            .and_then(|arguments| arguments.get(index).copied())
            .unwrap_or(&[])
    };
    Some(Box::new(Operands(
        contract
            .container_effects
            .iter()
            .map(|(&index, effect)| match effect {
                ContainerEffect::Published {
                    container_parameter,
                } => {
                    let entry = entry(argument(index));
                    let formal = matches!(&entry, Entry::Name(name) if formals.is_formal(name));
                    Operand::Published {
                        entry,
                        container: formals.unreleased(
                            container(argument(*container_parameter), formals),
                            argument(*container_parameter),
                        ),
                        formal,
                    }
                }
                ContainerEffect::Unpublished => Operand::Unpublished {
                    entry: entry(argument(index)),
                    foreign: None,
                    link: contract.link_members.get(&index).cloned(),
                },
                ContainerEffect::ElementsReleased => {
                    let container = container(argument(index), formals);
                    let root = match &container {
                        Container::CallerFormal(name) => formals.formal_root(name),
                        Container::Static(_) | Container::Other(_) => None,
                    };
                    Operand::ElementsReleased { container, root }
                }
            })
            .collect(),
    )))
}

fn refuse(
    function: &str,
    line: usize,
    description: String,
    entry: Option<&str>,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported { description },
    });
    let pointer = entry.and_then(|name| state.pointers.get(name)).cloned();
    match pointer {
        Some(pointer) => mark_unresolved_effect(state, &pointer),
        // An unnamed entry may be any tracked object.
        None if entry.is_none() => {
            let every = super::AbstractPointer {
                unknown: true,
                ..Default::default()
            };
            mark_unresolved_effect(state, &every);
        }
        None => {}
    }
    state.unmodeled = true;
}

/// The one live, settled identity `name` denotes, if it denotes exactly one.
fn settled_site(state: &AbstractState, name: &str) -> Result<Option<usize>, ()> {
    let Some(pointer) = state.pointers.get(name) else {
        return Ok(None);
    };
    if pointer.unknown
        || pointer.may_be_null
        || pointer.targets.len() != 1
        || !matches!(pointer.derivation, Derivation::Identity)
    {
        return Err(());
    }
    let site = *pointer.targets.first().expect("one target");
    if state.unresolved_effect_sites.contains(&site) || state.nullable_sites.contains(&site) {
        return Err(());
    }
    Ok(Some(site))
}

/// Apply one call's container effects after its borrowed reading.
pub(super) fn apply(
    function: &str,
    line: usize,
    called: &str,
    operands: &Operands,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    apply_with(function, line, called, operands, false, state, diagnostics);
}

/// Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): rule 34's element
/// destroyers' unlinks, applied before the call's own reading. As
/// [`apply`], and one more admitted entry: an object this path holds as one
/// live, unpublished identity it never unlinked
/// ([`element_release::destroyer_takes_held`]).
pub(super) fn apply_destroyers(
    function: &str,
    line: usize,
    called: &str,
    operands: &Operands,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    apply_with(function, line, called, operands, true, state, diagnostics);
}

fn apply_with(
    function: &str,
    line: usize,
    called: &str,
    operands: &Operands,
    destroyer: bool,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for operand in &operands.0 {
        match operand {
            // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): the call may
            // have released anything this path had published.
            Operand::ElementsReleased { .. } => element_release::forget_published(state),
            Operand::Published {
                entry,
                container,
                formal,
            } => {
                let name = match entry {
                    Entry::Name(name) => name,
                    Entry::Unnamed(spelling) => {
                        refuse(
                            function,
                            line,
                            format!(
                                "publication by `{called}` of an object the projection cannot name (`{spelling}`)"
                            ),
                            None,
                            state,
                            diagnostics,
                        );
                        continue;
                    }
                };
                let site = match settled_site(state, name) {
                    // A formal this analysis has not bound: its object is the
                    // caller's, so publishing it moves the caller's custody.
                    // Report the unresolved identity, which is also W4's
                    // signal to bind the formal and derive what the body does
                    // to it (constraint 10's borrowed launder stays closed).
                    Ok(None) if *formal => {
                        diagnostics.insert(LegacyDiagnostic {
                            function: function.to_owned(),
                            line,
                            kind: LegacyDiagnosticKind::UnknownAlias {
                                variable: name.clone(),
                            },
                        });
                        state.unmodeled = true;
                        continue;
                    }
                    // Untracked: the borrowed reading stands.
                    Ok(None) => continue,
                    Ok(Some(site)) if state.resources.get(&site) == Some(&LIVE) => site,
                    Ok(Some(_)) | Err(()) => {
                        refuse(
                            function,
                            line,
                            format!(
                                "publication by `{called}` of `{name}`, which this function does not hold as one live object"
                            ),
                            Some(name),
                            state,
                            diagnostics,
                        );
                        continue;
                    }
                };
                match container {
                    Container::CallerFormal(_) | Container::Static(_) => {
                        // Card MONKEY-VHOST-PROPERTY-CUSTODY (rule 12d): field-
                        // linked children that are exactly live and
                        // unencumbered go into the container with their owner,
                        // and an unlink of the owner hands them back.
                        let carried = carried_children(state, site);
                        // What the object owns otherwise, through a member or
                        // a stored child, goes into the container with it, but
                        // an unlink could not hand it back: the owned objects
                        // are withdrawn in both directions instead, and the
                        // call is refused, so no exit is certified over them.
                        let children = state
                            .field_links
                            .get(&site)
                            .into_iter()
                            .flat_map(|fields| fields.values().flatten().copied())
                            .chain(state.contained.get(&site).into_iter().flatten().copied())
                            .collect::<BTreeSet<_>>();
                        if let Some(carried) = carried {
                            for child in carried {
                                state.resources.insert(child, ESCAPED);
                            }
                        } else if !children.is_empty() {
                            diagnostics.insert(LegacyDiagnostic {
                                function: function.to_owned(),
                                line,
                                kind: LegacyDiagnosticKind::Unsupported {
                                    description: format!(
                                        "publication by `{called}` of `{name}`, which owns linked objects"
                                    ),
                                },
                            });
                            let owned = super::AbstractPointer {
                                targets: children,
                                ..Default::default()
                            };
                            mark_unresolved_effect(state, &owned);
                            state.unmodeled = true;
                        }
                        state.resources.insert(site, ESCAPED);
                        state.published.insert(site);
                    }
                    Container::Other(spelling) => refuse(
                        function,
                        line,
                        format!(
                            "publication by `{called}` into a container this function does not show outlives it (`{spelling}`)"
                        ),
                        Some(name),
                        state,
                        diagnostics,
                    ),
                }
            }
            Operand::Unpublished {
                entry,
                foreign,
                link,
            } => {
                let link = link.as_deref();
                let restored = match entry {
                    Entry::Name(name) => match settled_site(state, name) {
                        Ok(Some(site))
                            if state.published.contains(&site)
                                && state.resources.get(&site) == Some(&ESCAPED) =>
                        {
                            // Card MONKEY-VHOST-PROPERTY-CUSTODY (rule 12d): the
                            // children the publication carried come back with
                            // their owner; a child that may have been carried
                            // on some paths only refuses the unlink.
                            match handed_back(state, site) {
                                Some(restored) => {
                                    state.resources.insert(site, LIVE);
                                    state.published.remove(&site);
                                    state.unlinked.insert(site);
                                    for child in restored {
                                        state.resources.insert(child, LIVE);
                                    }
                                }
                                None => refuse(
                                    function,
                                    line,
                                    format!(
                                        "unlink by `{called}` of `{name}`, whose linked objects may have been carried into the container on some paths only"
                                    ),
                                    Some(name),
                                    state,
                                    diagnostics,
                                ),
                            }
                            true
                        }
                        // Card W45-CONTAINER-ROOT-REACH, rule 6: an element of
                        // an admitted container, holding no binding here,
                        // becomes this body's custody at a fresh site.
                        // Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway rule
                        // 46): an element destroyer takes the element only when
                        // it unlinks the member the element's list links
                        // (P-LERI-LINK-MEMBER, checked).
                        Ok(None) if destroyer && !link_member::admits(foreign.as_ref(), link) => {
                            let description =
                                link_member::foreign_refusal(called, name, foreign.as_ref(), link);
                            refuse(function, line, description, Some(name), state, diagnostics);
                            true
                        }
                        Ok(None) => foreign.as_ref().is_some_and(|foreign| {
                            take_foreign_custody(name, foreign.site, state);
                            state.unlinked.insert(foreign.site);
                            true
                        }),
                        // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43):
                        // an element destroyer handed an object this path
                        // already holds takes nothing at its unlink; its
                        // release then ends that custody.
                        Ok(Some(site))
                            if destroyer && element_release::destroyer_takes_held(state, site) =>
                        {
                            // Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway
                            // rule 46): only links this body initialised are
                            // links an unlink may remove (P-LERI-HELD-LINKS,
                            // decided from the body's own initialisation).
                            if link_member::held_admits(state, site, link) {
                                state.unlinked.insert(site);
                            } else {
                                let description = link_member::held_refusal(called, name, link);
                                refuse(function, line, description, Some(name), state, diagnostics);
                            }
                            true
                        }
                        _ => false,
                    },
                    Entry::Unnamed(_) => false,
                };
                if !restored {
                    let (name, spelling) = match entry {
                        Entry::Name(name) => (Some(name.as_str()), name.clone()),
                        Entry::Unnamed(spelling) => (None, spelling.clone()),
                    };
                    refuse(
                        function,
                        line,
                        format!(
                            "unlink by `{called}` of `{spelling}`, which this function has not published"
                        ),
                        name,
                        state,
                        diagnostics,
                    );
                }
            }
        }
    }
}

/// Card MONKEY-VHOST-PROPERTY-CUSTODY (rule 12d): the children a publication
/// of `owner` carries into the container, or `None` when rule 10d's
/// withdrawal stands (a contained child, a child that is the owner, or a
/// field-linked child [`crate::carried_publication::publish_step`] refuses).
fn carried_children(state: &AbstractState, owner: usize) -> Option<BTreeSet<usize>> {
    use crate::carried_publication::{ChildFacts, PublishStep, publish_step};
    if state
        .contained
        .get(&owner)
        .is_some_and(|children| !children.is_empty())
    {
        return None;
    }
    let mut carried = BTreeSet::new();
    for (field, children) in state.field_links.get(&owner).into_iter().flatten() {
        for &child in children {
            if child == owner {
                return None;
            }
            let facts = ChildFacts {
                partial: one_sided(state, owner, field, child),
                status: state.resources.get(&child).copied(),
                unresolved: state.unresolved_effect_sites.contains(&child),
                published: state.published.contains(&child),
                owns: state
                    .field_links
                    .get(&child)
                    .is_some_and(|fields| fields.values().any(|linked| !linked.is_empty()))
                    || state
                        .contained
                        .get(&child)
                        .is_some_and(|contained| !contained.is_empty()),
            };
            match publish_step(facts) {
                PublishStep::Carry => {
                    carried.insert(child);
                }
                PublishStep::Refuse => return None,
            }
        }
    }
    Some(carried)
}

/// Card MONKEY-VHOST-PROPERTY-CUSTODY (rule 12d): the children an unlink of
/// the published `owner` hands back, or `None` when one may have been carried
/// on some paths only ([`crate::carried_publication::unlink_step`]).
fn handed_back(state: &AbstractState, owner: usize) -> Option<BTreeSet<usize>> {
    use crate::carried_publication::{UnlinkStep, unlink_step};
    let mut restored = BTreeSet::new();
    for (field, children) in state.field_links.get(&owner).into_iter().flatten() {
        for &child in children {
            match unlink_step(
                one_sided(state, owner, field, child),
                state.resources.get(&child).copied(),
            ) {
                UnlinkStep::Keep => {}
                UnlinkStep::Restore => {
                    restored.insert(child);
                }
                UnlinkStep::Refuse => return None,
            }
        }
    }
    Some(restored)
}

/// Whether `owner`'s link of `child` under `field` may not hold on every path
/// (rule 11c's partial triple).
fn one_sided(state: &AbstractState, owner: usize, field: &str, child: usize) -> bool {
    state
        .partial_links
        .get(&owner)
        .and_then(|fields| fields.get(field))
        .is_some_and(|partial| partial.contains(&child))
}

/// Rule 6's effect: the element `name` denotes becomes this body's live
/// custody at `site`, exactly as a non-null acquisition binds its destination
/// (a strong update in a loop; prior unresolved effects are kept).
fn take_foreign_custody(name: &str, site: usize, state: &mut AbstractState) {
    state.resources.insert(site, LIVE);
    state.nullable_sites.remove(&site);
    state.published.remove(&site);
    state.pointers.insert(
        name.to_owned(),
        super::AbstractPointer {
            targets: BTreeSet::from([site]),
            unknown: false,
            may_be_null: false,
            derivation: Derivation::Identity,
        },
    );
}

/// Card W45-CONTAINER-ROOT-REACH: the `container_of` bindings rule 5l
/// admitted in one body, each keyed by the element name and the line of that
/// name's token, with the container its pointer reaches as spelled (`None`
/// when the pointer is no container's element this rule admits: a saved
/// successor, or a never-bound formal).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ForeignOrigins {
    admitted: BTreeMap<(String, usize), BTreeSet<Origin>>,
    pending: Option<(String, Origin)>,
    /// Card W45-SR5-SOUNDNESS (`CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER`):
    /// every name rule 5l(2) bound from a never-bound formal
    /// (`ctx = container_of(arg, …)`), wherever the binding was proposed.
    over_formal: BTreeSet<String>,
}

impl ForeignOrigins {
    /// Card W45-SR5-SOUNDNESS: `element` was bound over a formal: the object
    /// it names is the caller's, exactly as the formal's own is.
    pub(crate) fn note_over_formal(&mut self, element: &str) {
        self.over_formal.insert(element.to_owned());
    }

    /// The binding rule 5l is about to return for `element`, through
    /// `member` (card LERI-HELD-AND-LINK-MEMBER-PREMISES).
    pub(crate) fn propose(&mut self, element: &str, container: Option<Vec<String>>, member: &str) {
        self.pending = Some((element.to_owned(), (container, member.to_owned())));
    }

    /// Card GAPO-RETURN-CORRECTNESS (rule 15x, shared item): the part a
    /// classification probe may move and a declined one hands back, the
    /// admitted bindings and the pending proposal. `over_formal` is a
    /// monotone fact about the source statement and is not part of it.
    pub(crate) fn lowering(&self) -> OriginsLowering {
        let Self {
            admitted,
            pending,
            over_formal: _,
        } = self;
        OriginsLowering {
            admitted: admitted.clone(),
            pending: pending.clone(),
        }
    }

    /// Card GAPO-RETURN-CORRECTNESS (rule 15x, shared item): hand back
    /// [`Self::lowering`]'s answer; `over_formal` keeps what it grew to.
    pub(crate) fn restore_lowering(&mut self, lowering: &OriginsLowering) {
        self.admitted.clone_from(&lowering.admitted);
        self.pending.clone_from(&lowering.pending);
    }

    /// Forget any proposal: a statement's classification starts.
    pub(crate) fn clear(&mut self) {
        self.pending = None;
    }

    /// Keep the proposal only when the statement's final instruction is the
    /// binding itself, keyed by the element token's line in `tokens`.
    pub(crate) fn settle(&mut self, tokens: &[super::Token], instruction: &LegacyInstruction) {
        let Some((element, container)) = self.pending.take() else {
            return;
        };
        let LegacyInstruction::IndirectScalarAccess {
            destination: Some(destination),
            ..
        } = instruction
        else {
            return;
        };
        if *destination != element {
            return;
        }
        let Some(line) = tokens
            .windows(2)
            .find(|pair| pair[0].text == element && pair[1].text == "=")
            .map(|pair| pair[0].line)
        else {
            return;
        };
        self.admitted
            .entry((element, line))
            .or_default()
            .insert(container);
    }
}

/// Card GAPO-RETURN-CORRECTNESS (rule 15x, shared item): the probe state of
/// [`ForeignOrigins`] ([`ForeignOrigins::lowering`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OriginsLowering {
    admitted: BTreeMap<(String, usize), BTreeSet<Origin>>,
    pending: Option<(String, Origin)>,
}

/// One admitted binding's container, as spelled (`None`: no container this
/// rule admits), and the `container_of` member it names (card
/// LERI-HELD-AND-LINK-MEMBER-PREMISES).
type Origin = (Option<Vec<String>>, String);

/// Every lexical `container_of` binding `E = ( {` of `element` in `body`,
/// by line, or `None` when the name is written any other way (an assignment,
/// a step, its whole address taken) and so is not an element-only name.
fn element_binding_lines(body: &[&super::Token], element: &str) -> Option<Vec<usize>> {
    let texts = body
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let fused = fuse(&texts);
    let fused_texts = fused
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    for index in writes(&fused_texts, element) {
        let binding = fused_texts.get(index + 1) == Some(&"=")
            && fused_texts.get(index + 2) == Some(&"(")
            && fused_texts.get(index + 3) == Some(&"{");
        if !binding {
            return None;
        }
        lines.push(body[fused[index].1].line);
    }
    Some(lines)
}

/// Rule 6's lowering, after the whole body is parsed: every unlink of a name
/// whose every write is a binding rule 5l admitted over a container this body
/// may modify — reached from an unrebound formal, a local initialised from
/// one, or a static object — gets a fresh site for the element's custody.
/// Every other unlink keeps `foreign: None`, which refuses exactly as before.
pub(crate) fn settle_foreign_unlinks(
    cfg: &mut super::LegacyCfg,
    origins: &ForeignOrigins,
    body: &[super::Token],
    formals: &CallerFormals,
) {
    let body = body.iter().collect::<Vec<_>>();
    type Admitted = (Vec<String>, Option<BTreeSet<String>>, BTreeSet<String>);
    let admissible = |element: &str| -> Option<Admitted> {
        if formals.is_formal(element) || formals.statics.contains(element) {
            return None;
        }
        let lines = element_binding_lines(&body, element)?;
        if lines.is_empty() {
            return None;
        }
        let mut containers = BTreeSet::new();
        // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43): the formals the
        // containers are rooted at, while every one is.
        let mut roots = Some(BTreeSet::new());
        // Card LERI-HELD-AND-LINK-MEMBER-PREMISES: the members they name.
        let mut members = BTreeSet::new();
        for line in lines {
            let recorded = origins.admitted.get(&(element.to_owned(), line))?;
            for (container, member) in recorded {
                members.insert(member.clone());
                let spelling = container.as_ref()?;
                let texts = spelling.iter().map(String::as_str).collect::<Vec<_>>();
                match self::container(&texts, formals) {
                    Container::CallerFormal(name) => {
                        containers.insert(texts.concat());
                        match (formals.formal_root(&name), roots.as_mut()) {
                            (Some(root), Some(roots)) => {
                                roots.insert(root);
                            }
                            _ => roots = None,
                        }
                    }
                    Container::Static(_) => {
                        containers.insert(texts.concat());
                        roots = None;
                    }
                    Container::Other(_) => return None,
                }
            }
        }
        Some((containers.into_iter().collect(), roots, members))
    };
    let mut next = super::first_free_site(cfg);
    for node in &mut cfg.nodes {
        let LegacyInstruction::ContractCall {
            container: Some(operands),
            ..
        } = &mut node.instruction
        else {
            continue;
        };
        for operand in &mut operands.0 {
            // Card W45-SR5-SOUNDNESS: a publication of an element rule 5l(2)
            // bound over a formal publishes the caller's object, so it takes
            // the formal entry's rule (rule 1's first premise): an unresolved
            // identity, never the borrowed reading. Without it a derived
            // summary called the formal `Borrowed` while the body linked it.
            if let Operand::Published {
                entry: Entry::Name(name),
                formal,
                ..
            } = operand
                && origins.over_formal.contains(name.as_str())
            {
                *formal = true;
            }
            if let Operand::Unpublished {
                entry: Entry::Name(name),
                foreign,
                ..
            } = operand
                && let Some((containers, roots, members)) = admissible(name)
            {
                *foreign = Some(Foreign {
                    site: next,
                    containers,
                    roots,
                    members,
                });
                next += 1;
            }
        }
    }
}

/// The sites rule 6 lowered into one call's container effects.
pub(crate) fn foreign_sites(operands: &Operands) -> impl Iterator<Item = usize> + '_ {
    operands.0.iter().filter_map(|operand| match operand {
        Operand::Unpublished {
            foreign: Some(foreign),
            ..
        } => Some(foreign.site),
        _ => None,
    })
}

/// Shift those sites (the recursion combiner's disjointness).
pub(crate) fn offset_foreign_sites(operands: &mut Operands, offset: usize) {
    for operand in &mut operands.0 {
        if let Operand::Unpublished {
            foreign: Some(foreign),
            ..
        } = operand
        {
            foreign.site += offset;
        }
    }
}

/// Whether `cfg` lowers a foreign unlink. Such a body publishes no summary
/// and establishes no specification: its callers would read `Borrowed` for
/// the container's formal, although it released what the container held.
pub(crate) fn has_foreign_unlink(cfg: &super::LegacyCfg) -> bool {
    cfg.nodes.iter().any(|node| {
        matches!(&node.instruction, LegacyInstruction::ContractCall {
            container: Some(operands),
            ..
        } if foreign_sites(operands).next().is_some())
    })
}

/// Rule 3's `return` half: a `return` of a published identity refuses, and
/// the caller learns nothing it could discharge. `true` when `site` is
/// published on this path, in which case the diagnostic is recorded.
pub(super) fn refuse_escape_of_published(
    function: &str,
    line: usize,
    source: &str,
    site: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    if !state.published.contains(&site) {
        return false;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "transfer of `{source}`, which a container holds after its publication"
            ),
        },
    });
    true
}

/// Whether this call unlinks `owner` and `owner` denotes one identity this
/// path published: the one case in which a published object's member address
/// is a use the O04-B09B10 liveness premise must admit.
pub(super) fn unlinks_published(
    operands: Option<&Operands>,
    owner: &str,
    state: &AbstractState,
) -> bool {
    operands.is_some_and(|operands| {
        operands.0.iter().any(|operand| {
            matches!(operand, Operand::Unpublished { entry: Entry::Name(name), .. } if name == owner)
        })
    }) && matches!(settled_site(state, owner), Ok(Some(site))
        if state.published.contains(&site) && state.resources.get(&site) == Some(&ESCAPED))
}

/// The join of the published sets: a site is published at a node only when
/// every arriving path published it.
pub(super) fn join(current: &mut BTreeSet<usize>, incoming: &BTreeSet<usize>) {
    current.retain(|site| incoming.contains(site));
}

#[path = "legacy_element_release.rs"]
pub(crate) mod element_release;

#[path = "legacy_link_member.rs"]
pub(crate) mod link_member;

#[cfg(test)]
mod tests {
    use super::*;

    fn formals(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_rebound_formal_is_not_a_caller_container() {
        let body = [
            "queue", "=", "other", ";", "x", "=", "&", "head", ";", "n", "++",
        ];
        let read = CallerFormals::read(&body, &formals(&["queue", "head", "n", "keep"]));
        assert_eq!(read.stable, formals(&["keep"]));
    }

    #[test]
    fn entries_and_containers_are_read_from_their_spelling() {
        let caller = CallerFormals::read(&["return"], &formals(&["queue"]));
        assert_eq!(
            entry(&["&", "node", "->", "_head"]),
            Entry::Name("node".into())
        );
        assert_eq!(entry(&["node"]), Entry::Name("node".into()));
        assert!(matches!(
            entry(&["&", "a", "->", "b", ".", "c"]),
            Entry::Unnamed(_)
        ));
        assert_eq!(
            container(&["&", "queue", "->", "pending"], &caller),
            Container::CallerFormal("queue".into())
        );
        assert_eq!(
            container(&["&", "queue", "->", "q", "->", "pending"], &caller),
            Container::CallerFormal("queue".into())
        );
        assert!(matches!(
            container(&["&", "local"], &caller),
            Container::Other(_)
        ));
        assert!(matches!(
            container(&["&", "queue", ".", "pending"], &caller),
            Container::Other(_)
        ));
        assert!(matches!(
            container(&["&", "other", "->", "pending"], &caller),
            Container::Other(_)
        ));
    }

    #[test]
    fn a_static_container_is_its_own_storage_never_a_pointee() {
        let caller = CallerFormals::read(&["return"], &formals(&["queue"]))
            .with_statics(&formals(&["registry", "global", "queue"]));
        assert_eq!(
            container(&["&", "registry"], &caller),
            Container::Static("registry".into())
        );
        assert_eq!(
            container(&["&", "global", ".", "a", ".", "b"], &caller),
            Container::Static("global".into())
        );
        // A formal of the same name shadows the static object.
        assert_eq!(
            container(&["&", "queue", "->", "items"], &caller),
            Container::CallerFormal("queue".into())
        );
        for argument in [
            &["&", "global", "->", "items"][..],
            &["global"][..],
            &["&", "other"][..],
            &["&", "global", ".", "a", "->", "b"][..],
        ] {
            assert!(
                matches!(container(argument, &caller), Container::Other(_)),
                "{argument:?}"
            );
        }
    }

    fn tokens(text: &str) -> Vec<super::super::Token> {
        super::super::tokenize(text)
    }

    #[test]
    fn an_element_name_is_bound_only_by_statement_expressions() {
        let body = tokens("e = ({ x; });\n q_use(e);\n e = ({ y; });");
        let body = body.iter().collect::<Vec<_>>();
        assert_eq!(element_binding_lines(&body, "e"), Some(vec![1, 3]));
        for text in [
            "e = ({ x; }); e = other;",
            "e = ({ x; }); q_use(&e);",
            "e = ({ x; }); e++;",
            "struct item *e = NULL; e = ({ x; });",
        ] {
            let body = tokens(text);
            let body = body.iter().collect::<Vec<_>>();
            assert_eq!(element_binding_lines(&body, "e"), None, "{text}");
        }
        let body = tokens("q_use(e->payload);");
        let body = body.iter().collect::<Vec<_>>();
        assert_eq!(element_binding_lines(&body, "e"), Some(Vec::new()));
    }

    #[test]
    fn the_published_join_is_an_intersection() {
        let mut current = BTreeSet::from([1, 2]);
        join(&mut current, &BTreeSet::from([2, 3]));
        assert_eq!(current, BTreeSet::from([2]));
    }

    #[test]
    fn no_container_effect_means_no_operands() {
        let contract = ResourceFunctionContract::default();
        assert!(operands(&contract, None, &CallerFormals::default()).is_none());
    }
}
