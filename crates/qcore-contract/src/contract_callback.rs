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

//! Card **CG22-CALLBACK-REGISTRY-CONTRACT**: the callback clauses of resource
//! vocabulary version 1 (contract specification §C04.19, grammar G05), read
//! total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_callback_invoked(F, A…);          // rule 13u
//! ensures normal resource_callback_table(T, A…);            // rule 13v
//! ensures normal resource_callback_registered(F, H);        // rule 13w; H may be `result`
//! ensures normal resource_callback_registry_invoked(H, A…); // rule 13w
//! ensures normal resource_callback_unregistered(H);         // rule 13w
//! ensures normal resource_element_added(E, H);              // rule 13x
//! ensures normal resource_elements_dropped(H);              // rule 13x
//! ensures normal resource_released_if_constructed(H);       // rule 13x
//! ```
//!
//! `F` names a `fnptr` parameter (whose only mode is `resource_borrowed`);
//! `T`, `H` and `E` name depth-one pointer parameters, `resource_borrowed`,
//! with no kind. An actual `A` is a parameter name other than `F` (the
//! callback receives that actual), `_` (a value the callee supplies) or, in a
//! registry invocation only, `element` (each element `H` holds). A
//! registration's holder may be the word `result`, the object a
//! `resource_return_owned(nonnull)` declaration returns. A declaration
//! carrying any callback clause is withheld from the function map with the
//! retaining declarations ([`super::load_resource_contract_sources`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::callback::{
    CallbackActual, CallbackClause, CallbackHolder, CallbackMember, MemberObject,
};
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};

pub(super) const INVOKED: &str = "resource_callback_invoked";
pub(super) const TABLE: &str = "resource_callback_table";
pub(super) const REGISTERED: &str = "resource_callback_registered";
pub(super) const REGISTRY_INVOKED: &str = "resource_callback_registry_invoked";
pub(super) const UNREGISTERED: &str = "resource_callback_unregistered";
pub(super) const ELEMENT_ADDED: &str = "resource_element_added";
pub(super) const ELEMENTS_DROPPED: &str = "resource_elements_dropped";
pub(super) const RELEASED_IF_CONSTRUCTED: &str = "resource_released_if_constructed";
/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.3, rule 12n).
pub(super) const MEMBER_INVOKED: &str = "resource_callback_member_invoked";
/// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (§C04.19.3, rule 12n; user decision
/// Q2): a registration into a parameter's member, a setter. The `result`
/// holder of the same word is a constructor's result state (§C04.19.4).
pub(super) const MEMBER_REGISTERED: &str = "resource_callback_member_registered";

/// The actual word for a value the callee supplies.
pub(super) const SUPPLIED: &str = "_";
/// The actual word for each element a registry holds.
pub(super) const ELEMENT: &str = "element";
/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.1, rule 12m; USER
/// DECISION 2026-10-14, clause 1's word): a live object the callee supplies
/// and hands the callback, which must take it.
pub(super) const SUPPLIED_OWNED: &str = "supplied_owned";
/// The holder word for the object the call returns.
pub(super) const RESULT: &str = "result";

/// The depth-one pointer types a table, holder or element may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// A callback clause as written: its keyword and argument words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenCallback {
    pub(super) keyword: &'static str,
    pub(super) arguments: Vec<String>,
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the `where #ID =
    /// #result in P.LIST` binding of a registration forwarded through an id,
    /// `(ID, P, LIST)`; the first argument is then `#ID`.
    pub(super) by_id: Option<(String, String, Vec<String>)>,
}

/// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (§C04.19.3, rule 12n;
/// USER DECISION 2026-10-16, "Result + list"): the binding keyword.
pub(super) const WHERE: &str = "where";
/// The binding's set word.
pub(super) const IN: &str = "in";
/// The bound value: the call's integer result.
pub(super) const RESULT_VALUE: &str = "#result";

fn plain_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Read `resource_callback_member_registered(#ID, TAG.FIELD, F) where #ID =
/// #result in P.LIST[.LIST]`, `None` when the expression carries no
/// `where`. Every other shape with `where` rejects the module.
fn by_id_clause(expression: &str) -> Result<Option<WrittenCallback>, String> {
    let Some(rest) = expression.trim().strip_prefix(MEMBER_REGISTERED) else {
        return Ok(None);
    };
    if !expression
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .any(|word| word == WHERE)
    {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed `{MEMBER_REGISTERED}` clause `{expression}`: a registration forwarded \
             through an id is `{MEMBER_REGISTERED}(#id, TAG.FIELD, F) where #id = #result in \
             P.LIST;`"
        )
    };
    let rest = rest.trim_start().strip_prefix('(').ok_or_else(malformed)?;
    let close = rest.find(')').ok_or_else(malformed)?;
    let (inner, binding) = rest.split_at(close);
    let binding = binding[1..].trim();
    let arguments = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let [id, member, callback] = arguments.as_slice() else {
        return Err(malformed());
    };
    let id = id.strip_prefix('#').map(str::trim).ok_or_else(malformed)?;
    fn dotted(text: &str) -> Option<Vec<&str>> {
        let segments = text.split('.').collect::<Vec<_>>();
        (segments.len() >= 2 && segments.iter().all(|segment| plain_identifier(segment)))
            .then_some(segments)
    }
    if !plain_identifier(id) || dotted(member).is_none() || !plain_identifier(callback) {
        return Err(malformed());
    }
    let binding = binding.replace('#', " #").replace('=', " = ");
    let words = binding.split_whitespace().collect::<Vec<_>>();
    let [where_word, binder, equals, value, in_word, list] = words.as_slice() else {
        return Err(malformed());
    };
    if *where_word != WHERE || *equals != "=" || *in_word != IN {
        return Err(malformed());
    }
    if binder.strip_prefix('#') != Some(id) {
        return Err(format!(
            "`#{id}` is bound by its `where` clause: the binding names `{binder}`"
        ));
    }
    if *value != RESULT_VALUE {
        return Err(format!(
            "an id binding is `#{id} = {RESULT_VALUE} in P.LIST`, not `= {value}`"
        ));
    }
    let list = dotted(list).ok_or_else(|| {
        format!("`{IN}` takes a holder path `FORMAL.FIELD[.FIELD]`, not `{list}`")
    })?;
    Ok(Some(WrittenCallback {
        keyword: MEMBER_REGISTERED,
        arguments: vec![
            format!("#{id}"),
            (*member).to_owned(),
            (*callback).to_owned(),
        ],
        by_id: Some((
            id.to_owned(),
            list[0].to_owned(),
            list[1..]
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect(),
        )),
    }))
}

/// Read one callback clause from its predicate words, `None` for any other
/// predicate.
pub(super) fn clause(words: &[&str]) -> Option<WrittenCallback> {
    let (keyword, arguments) = words.split_first()?;
    let keyword = [
        INVOKED,
        TABLE,
        REGISTERED,
        REGISTRY_INVOKED,
        UNREGISTERED,
        ELEMENT_ADDED,
        ELEMENTS_DROPPED,
        RELEASED_IF_CONSTRUCTED,
    ]
    .into_iter()
    .find(|candidate| candidate == keyword)?;
    Some(WrittenCallback {
        keyword,
        arguments: arguments.iter().map(|word| (*word).to_owned()).collect(),
        by_id: None,
    })
}

/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: read
/// `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)`, whose dotted
/// member the identifier-only predicate reader does not read; `None` for any
/// other predicate.
pub(super) fn member_clause(expression: &str) -> Result<Option<WrittenCallback>, String> {
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the forwarded form.
    if let Some(written) = by_id_clause(expression)? {
        return Ok(Some(written));
    }
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q2): a setter's registration into
    // a parameter's member, `resource_callback_member_registered(P,
    // TAG.FIELD, F)` (the `result` holder is read by the result-state reader
    // first, so only a parameter holder reaches here).
    let (keyword, rest) = match expression.trim().strip_prefix(MEMBER_INVOKED) {
        Some(rest) => (MEMBER_INVOKED, rest),
        None => match expression.trim().strip_prefix(MEMBER_REGISTERED) {
            Some(rest) => (MEMBER_REGISTERED, rest),
            None => return Ok(None),
        },
    };
    let malformed = || format!("malformed `{keyword}` clause `{expression}`");
    let inner = rest
        .trim_start()
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .ok_or_else(malformed)?;
    let arguments = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let word = |text: &str, dotted: bool| {
        !text.is_empty()
            && text.split('.').all(|segment| {
                !segment.is_empty()
                    && !segment.starts_with(|ch: char| ch.is_ascii_digit())
                    && segment
                        .chars()
                        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
            })
            && (dotted || !text.contains('.'))
    };
    if arguments.len() < 2
        || (keyword == MEMBER_REGISTERED && arguments.len() != 3)
        || arguments
            .iter()
            .enumerate()
            .any(|(at, argument)| !word(argument, at == 1))
    {
        return Err(malformed());
    }
    Ok(Some(WrittenCallback {
        keyword,
        arguments: arguments.into_iter().map(str::to_owned).collect(),
        by_id: None,
    }))
}

/// Validate the callback clauses of one declaration whose custody contract
/// `contract` is already loaded.
pub(super) fn declaration(
    written: &[WrittenCallback],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<Vec<CallbackClause>, String> {
    if written.is_empty() {
        return Ok(Vec::new());
    }
    for reserved in [SUPPLIED, ELEMENT, SUPPLIED_OWNED] {
        if names.contains_key(reserved) {
            return Err(format!(
                "a callback clause reads `{reserved}` as an actual word: rename the parameter \
                 `{reserved}`"
            ));
        }
    }
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    let function_pointer = |name: &str| -> Result<usize, String> {
        let at = index(name)?;
        if parameter_types[at] != "fnptr" {
            return Err(format!(
                "a callback clause names a `fnptr` parameter, not `{name}`"
            ));
        }
        Ok(at)
    };
    let object = |name: &str, role: &str| -> Result<usize, String> {
        let at = index(name)?;
        if !DEPTH_ONE.contains(&parameter_types[at].as_str()) {
            return Err(format!(
                "a {role} names a depth-one pointer parameter, not `{name}`"
            ));
        }
        if contract.parameters[at] != ResourceParameterMode::Borrowed {
            return Err(format!(
                "a {role} names a `resource_borrowed` parameter (`{name}`)"
            ));
        }
        if parameter_kinds[at].is_some() {
            return Err(format!("a {role} carries no kind (`{name}`)"));
        }
        Ok(at)
    };
    let actuals = |words: &[String], callback: Option<usize>, element: bool, owned: bool| {
        if words.iter().filter(|word| *word == SUPPLIED_OWNED).count() > 1 {
            return Err(format!("at most one `{SUPPLIED_OWNED}` per actual list"));
        }
        words
            .iter()
            .map(|word| match word.as_str() {
                SUPPLIED => Ok(CallbackActual::Supplied),
                SUPPLIED_OWNED if owned => Ok(CallbackActual::SuppliedOwned),
                SUPPLIED_OWNED => Err(format!(
                    "`{SUPPLIED_OWNED}` is an actual of a synchronous or member invocation only"
                )),
                ELEMENT if element => Ok(CallbackActual::Element),
                ELEMENT => Err(format!(
                    "`{ELEMENT}` is an actual of a registry invocation only"
                )),
                name => {
                    let at = index(name)?;
                    if Some(at) == callback {
                        return Err(format!("a callback is not its own actual (`{name}`)"));
                    }
                    Ok(CallbackActual::Parameter { index: at })
                }
            })
            .collect::<Result<Vec<_>, String>>()
    };
    let mut clauses = Vec::with_capacity(written.len());
    let mut seen: BTreeSet<(&str, usize)> = BTreeSet::new();
    let mut callbacks_named: BTreeSet<usize> = BTreeSet::new();
    let mut members_named: BTreeSet<(MemberObject, CallbackMember)> = BTreeSet::new();
    for clause in written {
        let arguments = clause.arguments.as_slice();
        let shape = || format!("malformed `{}` clause", clause.keyword);
        let declared = match clause.keyword {
            INVOKED => {
                let (callback, rest) = arguments.split_first().ok_or_else(shape)?;
                let callback = function_pointer(callback)?;
                if !callbacks_named.insert(callback) {
                    return Err("a callback is invoked or registered by one clause".into());
                }
                CallbackClause::Invoked {
                    callback,
                    actuals: actuals(rest, Some(callback), false, true)?,
                }
            }
            TABLE => {
                let (table, rest) = arguments.split_first().ok_or_else(shape)?;
                let table = object(table, "callback table")?;
                CallbackClause::Table {
                    table,
                    actuals: actuals(rest, None, false, false)?,
                }
            }
            REGISTERED => {
                let [callback, holder] = arguments else {
                    return Err(shape());
                };
                let callback = function_pointer(callback)?;
                if !callbacks_named.insert(callback) {
                    return Err("a callback is invoked or registered by one clause".into());
                }
                let holder = if holder == RESULT {
                    if names.contains_key(RESULT) {
                        return Err(format!(
                            "a registration reads `{RESULT}` as the returned object: rename the \
                             parameter `{RESULT}`"
                        ));
                    }
                    if !matches!(
                        contract.result,
                        ResourceReturnMode::Owned { nullable: false }
                    ) {
                        return Err(format!(
                            "a `{RESULT}` holder needs `resource_return_owned(nonnull)`: the \
                             caller owns the object that keeps the callback"
                        ));
                    }
                    CallbackHolder::Result
                } else {
                    CallbackHolder::Parameter {
                        index: object(holder, "callback holder")?,
                    }
                };
                CallbackClause::Registered { callback, holder }
            }
            REGISTRY_INVOKED => {
                let (holder, rest) = arguments.split_first().ok_or_else(shape)?;
                let holder = object(holder, "callback holder")?;
                CallbackClause::RegistryInvoked {
                    holder,
                    actuals: actuals(rest, None, true, false)?,
                }
            }
            UNREGISTERED | ELEMENTS_DROPPED | RELEASED_IF_CONSTRUCTED => {
                let [holder] = arguments else {
                    return Err(shape());
                };
                let holder = object(holder, "callback holder")?;
                match clause.keyword {
                    UNREGISTERED => CallbackClause::Unregistered { holder },
                    ELEMENTS_DROPPED => CallbackClause::ElementsDropped { holder },
                    _ => CallbackClause::ReleasedIfConstructed { holder },
                }
            }
            ELEMENT_ADDED => {
                let [element, holder] = arguments else {
                    return Err(shape());
                };
                let element = object(element, "registry element")?;
                let holder = object(holder, "callback holder")?;
                if element == holder {
                    return Err("a holder is not its own element".into());
                }
                CallbackClause::ElementAdded { element, holder }
            }
            MEMBER_INVOKED => {
                let [object, member, rest @ ..] = arguments else {
                    return Err(shape());
                };
                let object = if object == SUPPLIED_OWNED {
                    MemberObject::SuppliedOwned
                } else {
                    // The member is read before any release the declaration
                    // states of the object (`rd_kafka_op_destroy` calls the
                    // op's callback, then frees the op).
                    let at = index(object)?;
                    if !DEPTH_ONE.contains(&parameter_types[at].as_str()) {
                        return Err(format!(
                            "a member invocation reads a depth-one pointer parameter, not \
                             `{object}`"
                        ));
                    }
                    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: or
                    // `resource_owned` for the formal a hand-off hands, which
                    // the loader checks once the hand-off is read.
                    if !matches!(
                        contract.parameters[at],
                        ResourceParameterMode::Borrowed
                            | ResourceParameterMode::Released { .. }
                            | ResourceParameterMode::Owned { nullable: false }
                    ) {
                        return Err(format!(
                            "a member invocation's object is `resource_borrowed` or \
                             `resource_released` (`{object}`)"
                        ));
                    }
                    if parameter_kinds[at].is_some() {
                        return Err(format!(
                            "a member invocation's object carries no kind (`{object}`)"
                        ));
                    }
                    MemberObject::Parameter { index: at }
                };
                let (tag, path) = member
                    .split_once('.')
                    .ok_or_else(|| format!("a member is spelled `TAG.FIELD`, not `{member}`"))?;
                let actuals = actuals(rest, None, false, true)?;
                let member = CallbackMember {
                    tag: tag.to_owned(),
                    path: path.to_owned(),
                };
                if !members_named.insert((object, member.clone())) {
                    return Err(format!("duplicate `{MEMBER_INVOKED}` clause"));
                }
                CallbackClause::MemberInvoked {
                    object,
                    member,
                    actuals,
                }
            }
            // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (§C04.19.3, Q2): a setter
            // stores its callback parameter into its object parameter's
            // member. One registration per member and per callback.
            MEMBER_REGISTERED if clause.by_id.is_some() => {
                // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (§C04.19.3):
                // the record holding the member is the one the call links
                // into `P.LIST` and names by its integer result (the loader
                // checks the result beside the result mode).
                let [_, member, callback] = arguments else {
                    return Err(shape());
                };
                let Some((binder, root, list)) = &clause.by_id else {
                    return Err(shape());
                };
                if names.contains_key(binder.as_str()) || binder == RESULT {
                    return Err(format!(
                        "`#{binder}` binds the returned id: rename the parameter `{binder}`"
                    ));
                }
                let holder = object(root, "registration list's formal")?;
                let callback = function_pointer(callback)?;
                if !callbacks_named.insert(callback) {
                    return Err("a callback is invoked or registered by one clause".into());
                }
                let (tag, path) = member
                    .split_once('.')
                    .ok_or_else(|| format!("a member is spelled `TAG.FIELD`, not `{member}`"))?;
                let member = CallbackMember {
                    tag: tag.to_owned(),
                    path: path.to_owned(),
                };
                if !members_named
                    .insert((MemberObject::Parameter { index: holder }, member.clone()))
                {
                    return Err(format!("duplicate `{MEMBER_REGISTERED}` clause"));
                }
                CallbackClause::MemberRegistered {
                    object: holder,
                    member,
                    callback,
                    by_id: Some(qcore_schemas::callback::IdLink {
                        binder: binder.clone(),
                        list: list.clone(),
                    }),
                }
            }
            MEMBER_REGISTERED => {
                let [holder, member, callback] = arguments else {
                    return Err(shape());
                };
                if holder == RESULT {
                    return Err(format!(
                        "`{MEMBER_REGISTERED}(result, …)` is a result state (§C04.19.4)"
                    ));
                }
                let holder = object(holder, "member registration's holder")?;
                let callback = function_pointer(callback)?;
                if !callbacks_named.insert(callback) {
                    return Err("a callback is invoked or registered by one clause".into());
                }
                let (tag, path) = member
                    .split_once('.')
                    .ok_or_else(|| format!("a member is spelled `TAG.FIELD`, not `{member}`"))?;
                let member = CallbackMember {
                    tag: tag.to_owned(),
                    path: path.to_owned(),
                };
                if !members_named
                    .insert((MemberObject::Parameter { index: holder }, member.clone()))
                {
                    return Err(format!("duplicate `{MEMBER_REGISTERED}` clause"));
                }
                CallbackClause::MemberRegistered {
                    object: holder,
                    member,
                    callback,
                    by_id: None,
                }
            }
            other => return Err(format!("unsupported callback clause `{other}`")),
        };
        if matches!(
            declared,
            CallbackClause::MemberInvoked { .. } | CallbackClause::MemberRegistered { .. }
        ) {
            clauses.push(declared);
            continue;
        }
        // At most one clause of each keyword per position.
        let position = match &declared {
            CallbackClause::Invoked { callback, .. } => *callback,
            CallbackClause::Registered { callback, .. } => *callback,
            CallbackClause::Table { table, .. } => *table,
            CallbackClause::RegistryInvoked { holder, .. }
            | CallbackClause::Unregistered { holder }
            | CallbackClause::ElementsDropped { holder }
            | CallbackClause::ReleasedIfConstructed { holder }
            | CallbackClause::ElementAdded { holder, .. } => *holder,
            CallbackClause::MemberInvoked { .. } | CallbackClause::MemberRegistered { .. } => {
                unreachable!("pushed above")
            }
        };
        if !seen.insert((clause.keyword, position)) {
            return Err(format!("duplicate `{}` clause", clause.keyword));
        }
        clauses.push(declared);
    }
    // A registry clause of a holder stands only on a holder no registration of
    // the same declaration names: a declaration registers or uses a registry,
    // never both on one object.
    for clause in &clauses {
        if let CallbackClause::Registered {
            holder: CallbackHolder::Parameter { index },
            ..
        } = clause
            && clauses.iter().any(|other| {
                other.needs_registration()
                    && matches!(other,
                        CallbackClause::RegistryInvoked { holder, .. }
                        | CallbackClause::Unregistered { holder }
                        | CallbackClause::ElementsDropped { holder }
                        | CallbackClause::ReleasedIfConstructed { holder }
                        | CallbackClause::ElementAdded { holder, .. } if holder == index)
            })
        {
            return Err("a declaration registers a callback or uses a registry, not both".into());
        }
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: a member of the supplied
    // object is read only where the declaration hands a callback that object.
    let supplied = clauses.iter().any(|clause| match clause {
        CallbackClause::Invoked { actuals, .. } | CallbackClause::MemberInvoked { actuals, .. } => {
            actuals.contains(&CallbackActual::SuppliedOwned)
        }
        _ => false,
    });
    if !supplied
        && clauses.iter().any(|clause| {
            matches!(
                clause,
                CallbackClause::MemberInvoked {
                    object: MemberObject::SuppliedOwned,
                    ..
                }
            )
        })
    {
        return Err(format!(
            "a member of the `{SUPPLIED_OWNED}` object is read only where a callback is handed \
             that object"
        ));
    }
    Ok(clauses)
}
