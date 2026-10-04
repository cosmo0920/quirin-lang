//! Card **CALLEE-RETENTION-CLASS1**, stage (b): the retention clauses of
//! resource vocabulary version 1 (contract specification §C04.14, grammar
//! G05), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_retained_by(p, h);   // h: a parameter, or `global`
//! ensures normal resource_holder_reset(h);
//! ```
//!
//! `p` names a depth-one pointer parameter whose mode is `resource_borrowed`
//! (the caller keeps custody); `h` names a distinct depth-one pointer
//! parameter whose mode is `resource_borrowed`, or is the word `global`
//! (storage of static duration, or a holder no caller object denotes). A
//! parameter is retained at most once; a reset names a depth-one pointer
//! parameter, `resource_borrowed`, at most once. No kind stands on either
//! position. A declaration carrying either clause is withheld from the
//! function map (the loader, [`super::load_resource_contract_sources`]).

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceFunctionContract, ResourceKind, ResourceParameterMode};
use qcore_schemas::retained_by::{RetainingContract, Retention, RetentionHolder};

/// `resource_retained_by(p, h)`.
pub(super) const RETAINED_BY: &str = "resource_retained_by";

/// `resource_holder_reset(h)`.
pub(super) const HOLDER_RESET: &str = "resource_holder_reset";

/// The holder word for static storage.
pub(super) const GLOBAL: &str = "global";

/// The depth-one pointer types a retained pointer or a holder may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// Validate the retention clauses of one declaration whose custody contract
/// `contract` is already loaded.
pub(super) fn declaration(
    retentions: &[(String, String)],
    resets: &[String],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<RetainingContract, String> {
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    if names.contains_key(GLOBAL) {
        return Err(format!(
            "a retention clause reads `{GLOBAL}` as static storage: rename the parameter \
             `{GLOBAL}`"
        ));
    }
    let position = |name: &str, role: &str| -> Result<usize, String> {
        let at = index(name)?;
        if !DEPTH_ONE.contains(&parameter_types[at].as_str()) {
            return Err(format!(
                "a {role} names a depth-one pointer parameter, not `{name}`"
            ));
        }
        if contract.parameters[at] != ResourceParameterMode::Borrowed {
            return Err(format!(
                "a {role} names a `resource_borrowed` parameter: the caller keeps custody of \
                 `{name}`"
            ));
        }
        if parameter_kinds[at].is_some() {
            return Err(format!("a {role} carries no kind (`{name}`)"));
        }
        Ok(at)
    };
    let mut declared = Vec::with_capacity(retentions.len());
    for (retained, holder) in retentions {
        let retained = position(retained, "retained pointer")?;
        if declared
            .iter()
            .any(|existing: &Retention| existing.retained == retained)
        {
            return Err(format!(
                "duplicate retention of {}",
                names
                    .iter()
                    .find(|(_, at)| **at == retained)
                    .map_or("?", |(name, _)| *name)
            ));
        }
        let holder = if holder == GLOBAL {
            RetentionHolder::Global
        } else {
            let at = position(holder, "holder")?;
            if at == retained {
                return Err(format!("a pointer does not retain itself (`{holder}`)"));
            }
            RetentionHolder::Parameter { index: at }
        };
        declared.push(Retention { retained, holder });
    }
    let mut reset = Vec::with_capacity(resets.len());
    for holder in resets {
        let at = position(holder, "reset holder")?;
        if reset.contains(&at) {
            return Err(format!("duplicate holder reset of {holder}"));
        }
        reset.push(at);
    }
    Ok(RetainingContract {
        contract: contract.clone(),
        retentions: declared,
        resets: reset,
    })
}
