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
//!    name, a container not reached from an unrebound formal (a local, a
//!    file-scope object, an object this body acquired), a binding that is
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

use std::collections::BTreeSet;

use qcore_schemas::contract::{ContainerEffect, ResourceFunctionContract};

use super::{
    AbstractState, Derivation, ESCAPED, LIVE, LegacyDiagnostic, LegacyDiagnosticKind,
    mark_unresolved_effect,
};

/// The object an entry actual designates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `&B->F` or `B`: the object `B` points to.
    Name(String),
    /// Any other spelling.
    Unnamed(String),
}

/// Where a published object's container lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Container {
    /// Reached from this formal, which the body never rebinds.
    CallerFormal(String),
    /// Anything else, spelled as written.
    Other(String),
}

/// One container effect at one call. `formal` records that the entry names a
/// formal of the function, whose object the caller holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Published {
        entry: Entry,
        container: Container,
        formal: bool,
    },
    Unpublished {
        entry: Entry,
    },
}

/// Every container effect of one call, in parameter order.
#[derive(Clone, Debug, PartialEq, Eq)]
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
            }
        }
        Self {
            stable,
            derived,
            all: formals.clone(),
        }
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
        _ => Container::Other(argument.join(" ")),
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
                        container: container(argument(*container_parameter), formals),
                        formal,
                    }
                }
                ContainerEffect::Unpublished => Operand::Unpublished {
                    entry: entry(argument(index)),
                },
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
    for operand in &operands.0 {
        match operand {
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
                    Container::CallerFormal(_) => {
                        // What the object owns through a member or a stored
                        // child goes into the container with it, but an unlink
                        // could not hand it back: the owned objects are
                        // withdrawn in both directions instead, and the call
                        // is refused, so no exit is certified over them.
                        let children = state
                            .field_links
                            .get(&site)
                            .into_iter()
                            .flat_map(|fields| fields.values().flatten().copied())
                            .chain(state.contained.get(&site).into_iter().flatten().copied())
                            .collect::<BTreeSet<_>>();
                        if !children.is_empty() {
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
            Operand::Unpublished { entry } => {
                let restored = match entry {
                    Entry::Name(name) => match settled_site(state, name) {
                        Ok(Some(site))
                            if state.published.contains(&site)
                                && state.resources.get(&site) == Some(&ESCAPED) =>
                        {
                            state.resources.insert(site, LIVE);
                            state.published.remove(&site);
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
            matches!(operand, Operand::Unpublished { entry: Entry::Name(name) } if name == owner)
        })
    }) && matches!(settled_site(state, owner), Ok(Some(site))
        if state.published.contains(&site) && state.resources.get(&site) == Some(&ESCAPED))
}

/// The join of the published sets: a site is published at a node only when
/// every arriving path published it.
pub(super) fn join(current: &mut BTreeSet<usize>, incoming: &BTreeSet<usize>) {
    current.retain(|site| incoming.contains(site));
}

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
