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

use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder};
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

/// The actual word for a value the callee supplies.
pub(super) const SUPPLIED: &str = "_";
/// The actual word for each element a registry holds.
pub(super) const ELEMENT: &str = "element";
/// The holder word for the object the call returns.
pub(super) const RESULT: &str = "result";

/// The depth-one pointer types a table, holder or element may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// A callback clause as written: its keyword and argument words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenCallback {
    pub(super) keyword: &'static str,
    pub(super) arguments: Vec<String>,
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
    })
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
    for reserved in [SUPPLIED, ELEMENT] {
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
    let actuals = |words: &[String], callback: Option<usize>, element: bool| {
        words
            .iter()
            .map(|word| match word.as_str() {
                SUPPLIED => Ok(CallbackActual::Supplied),
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
                    actuals: actuals(rest, Some(callback), false)?,
                }
            }
            TABLE => {
                let (table, rest) = arguments.split_first().ok_or_else(shape)?;
                let table = object(table, "callback table")?;
                CallbackClause::Table {
                    table,
                    actuals: actuals(rest, None, false)?,
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
                    actuals: actuals(rest, None, true)?,
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
            other => return Err(format!("unsupported callback clause `{other}`")),
        };
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
    Ok(clauses)
}
