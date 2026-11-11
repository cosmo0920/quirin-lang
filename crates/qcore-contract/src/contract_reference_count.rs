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

//! Card **REFCOUNT-RESOURCE-KIND**: the reference clauses of resource
//! vocabulary version 1 (contract specification §C04.17, grammar G05), read
//! total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_reference_kept(p);      // +1 (rule 13b)
//! ensures normal resource_reference_dropped(p);   // -1, possibly the last
//! ```
//!
//! `p` names a depth-one pointer parameter (`ptr<mut, u8>` or
//! `ptr<const, u8>`) whose mode is `resource_borrowed` and which carries no
//! kind. A declaration carries at most one reference clause. A keep's result
//! is `resource_return_alias(p, nonnull)` (the keep hands back its argument)
//! or `resource_return_none()`; a drop's is `resource_return_none()`. `p` is
//! neither retained nor a slot. A declaration carrying a reference clause is
//! withheld from the function map with the retaining declarations (the
//! loader, [`super::load_resource_contract_sources`]), so the borrowed
//! reading of `p` — the custody-neutral reading the user rejected — is never
//! what a consumer that knows nothing of references sees.

use std::collections::BTreeMap;

use qcore_schemas::contract::{
    DerivedExtent, ResourceFunctionContract, ResourceKind, ResourceParameterMode,
    ResourceReturnMode,
};
use qcore_schemas::reference_count::{ReferenceClause, ReferenceOperation};

/// `resource_reference_kept(p)`.
pub(super) const KEPT: &str = "resource_reference_kept";

/// `resource_reference_dropped(p)`.
pub(super) const DROPPED: &str = "resource_reference_dropped";

/// The pointer types a referenced parameter may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// A reference clause as written, before the declaration's parameters are
/// known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenReference {
    pub(super) parameter: String,
    pub(super) operation: ReferenceOperation,
}

/// Read one reference clause from its predicate words, `None` for any other
/// predicate.
pub(super) fn clause(words: &[&str]) -> Option<WrittenReference> {
    let (operation, parameter) = match words {
        [KEPT, parameter] => (ReferenceOperation::Kept, parameter),
        [DROPPED, parameter] => (ReferenceOperation::Dropped, parameter),
        _ => return None,
    };
    Some(WrittenReference {
        parameter: (*parameter).to_owned(),
        operation,
    })
}

/// Validate the reference clauses of one declaration whose custody contract
/// `contract` is already loaded.
pub(super) fn declaration(
    written: &[WrittenReference],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<Vec<ReferenceClause>, String> {
    let [clause] = written else {
        return if written.is_empty() {
            Ok(Vec::new())
        } else {
            Err("a declaration carries at most one reference clause".into())
        };
    };
    let name = clause.parameter.as_str();
    let parameter = names
        .get(name)
        .copied()
        .ok_or_else(|| format!("unknown parameter {name}"))?;
    if !DEPTH_ONE.contains(&parameter_types[parameter].as_str()) {
        return Err(format!(
            "a referenced object is named by a depth-one pointer parameter, not `{name}`"
        ));
    }
    if contract.parameters[parameter] != ResourceParameterMode::Borrowed {
        return Err(format!(
            "a referenced object is named by a `resource_borrowed` parameter, not `{name}`"
        ));
    }
    if parameter_kinds[parameter].is_some() {
        return Err(format!("a referenced parameter carries no kind (`{name}`)"));
    }
    let result_admitted = match (clause.operation, contract.result) {
        (_, ResourceReturnMode::None) => true,
        (
            ReferenceOperation::Kept,
            ResourceReturnMode::Derived {
                parameter: base,
                extent: DerivedExtent::Exact,
                nullable: false,
                ..
            },
        ) => base == parameter,
        _ => false,
    };
    if !result_admitted {
        return Err(match clause.operation {
            ReferenceOperation::Kept => format!(
                "a keep returns `resource_return_alias({name}, nonnull)` or `resource_return_none()`"
            ),
            ReferenceOperation::Dropped => "a drop returns `resource_return_none()`".to_owned(),
        });
    }
    Ok(vec![ReferenceClause {
        parameter,
        operation: clause.operation,
    }])
}
