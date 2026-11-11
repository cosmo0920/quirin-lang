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

//! Bridge from a loaded resource-contract v1 environment to the callable
//! extern-call model (card `source.extern-call.v1`).
//!
//! This module adds **no contract vocabulary**. The only admitted contracts are
//! the ones `contract_loader` already loads total-or-reject, and the only
//! admitted call shapes are the ones `qcore_core::extern_call::bridge_contract`
//! already admits. What lives here is the join: reading each declaration's ABI
//! spelling back out of the environment's retained source assumptions, so the
//! model's typed view is built from the same immutable text the loader
//! validated rather than from a caller's claim about it.
//!
//! An environment with no retained source map is refused. Contract spec §C05 is
//! explicit that a programmatically constructed environment's lack of declared
//! source text must not be reported as source-backed verification, and this
//! route has no other way to recover the declared ABI types.
use std::collections::BTreeMap;

use qcore_core::extern_call::{
    AbiType, ContractParameterMode, ContractReturnMode, DeclaredContract,
};
use qcore_frontend::frontend::{DeclarationKind, SourceModuleMode, parse_source_program};
use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

/// One declaration exactly as the selected contract module writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredExternSignature {
    pub parameter_names: Vec<String>,
    /// Normalized type spellings, whitespace removed.
    pub parameter_types: Vec<String>,
    pub result_type: String,
    /// The module path the declaration came from, for attribution.
    pub module: String,
}

fn normalize(source: &str) -> String {
    source.split_whitespace().collect()
}

/// Read every declaration's signature out of the environment's retained
/// sources. Total-or-reject: one unreadable module refuses the whole map.
pub fn declared_signatures(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredExternSignature>, String> {
    if contracts.source_assumptions().is_empty() {
        return Err(
            "the selected extern contract environment retains no declared source text; a programmatically constructed environment is not source-backed"
                .into(),
        );
    }
    let mut signatures = BTreeMap::new();
    for (module, source) in contracts.source_assumptions() {
        let parsed = parse_source_program(source).map_err(|error| format!("{module}: {error}"))?;
        if parsed.mode != SourceModuleMode::Contract {
            return Err(format!("{module}: expected a contract module"));
        }
        for declaration in &parsed.declarations {
            if declaration.value.kind != DeclarationKind::ExternFunction {
                return Err(format!(
                    "{module}: `{}` is not an extern function declaration",
                    declaration.value.name
                ));
            }
            let signature = DeclaredExternSignature {
                parameter_names: declaration
                    .value
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.value.clone())
                    .collect(),
                parameter_types: declaration
                    .value
                    .parameters
                    .iter()
                    .map(|parameter| normalize(&parameter.type_source))
                    .collect(),
                result_type: declaration
                    .value
                    .result_type
                    .as_ref()
                    .map_or_else(String::new, |result| normalize(&result.source)),
                module: module.clone(),
            };
            if signatures
                .insert(declaration.value.name.clone(), signature)
                .is_some()
            {
                return Err(format!(
                    "{module}: duplicate declaration of `{}`",
                    declaration.value.name
                ));
            }
        }
    }
    Ok(signatures)
}

/// Card W45-CONTRACT-DEFERRED-REVIEW: the refusal for a callee whose declared
/// contract awaits human review, or `None` if it does not.
///
/// The loader withholds such a declaration from the function map, so
/// `contracts.contract(name)` is already `None` and the route already refuses
/// the callee as uncontracted; this only names the reason. A pending contract
/// is never joined into a [`DeclaredContract`], so it can neither admit a call
/// nor contribute a custody effect.
#[must_use]
pub fn pending_review_refusal(name: &str, contracts: &ResourceContracts) -> Option<String> {
    let pending = contracts.pending_review().get(name)?;
    let reasons = pending
        .deferrals
        .iter()
        .map(|deferral| deferral.reason.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    Some(format!(
        "`{name}`'s declared contract in {} awaits human review (awaiting-human-review), so it \
         admits no call: {reasons}",
        pending.module
    ))
}

/// Join one loaded contract with its declared signature into the model's typed
/// view. Every failure names the exact spelling that is not in the C04 ABI
/// projection; nothing is approximated.
pub fn declared_contract(
    name: &str,
    signature: &DeclaredExternSignature,
    contract: &ResourceFunctionContract,
) -> Result<DeclaredContract, String> {
    if signature.parameter_types.len() != contract.parameters.len() {
        return Err(format!(
            "`{name}`: the declaration has {} parameters but its loaded contract has {}",
            signature.parameter_types.len(),
            contract.parameters.len()
        ));
    }
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the checked extern-call model has
    // one custody kind, so a declaration naming another is refused rather than
    // projected onto the heap kind, which would let `free` discharge a stream.
    if contract.has_non_heap_kind() {
        return Err(format!(
            "`{name}`: a non-heap resource kind (card W43-DESCRIPTOR-RESOURCE-TRACKING) \
             has no extern-call model; the checked extern-call route has one custody kind"
        ));
    }
    // Card W45-FB-RETRY-LIFETIME: neither a container publication nor a
    // counted user has an extern-call model; projecting one onto its borrowed
    // modes would drop exactly the effect it declares.
    if contract.has_container_effect() || !contract.counted_users.is_empty() {
        return Err(format!(
            "`{name}`: a container or counted-user clause (card W45-FB-RETRY-LIFETIME) \
             has no extern-call model"
        ));
    }
    let mut parameters = Vec::with_capacity(contract.parameters.len());
    for (index, (spelling, mode)) in signature
        .parameter_types
        .iter()
        .zip(&contract.parameters)
        .enumerate()
    {
        let abi = AbiType::parse(spelling).ok_or_else(|| {
            format!("`{name}`: parameter {index} type `{spelling}` is not an admitted ABI type")
        })?;
        parameters.push((abi, parameter_mode(*mode)));
    }
    let result = AbiType::parse(&signature.result_type).ok_or_else(|| {
        format!(
            "`{name}`: result type `{}` is not an admitted ABI type",
            signature.result_type
        )
    })?;
    let result_mode = return_mode(contract.result).ok_or_else(|| {
        format!(
            "`{name}`: a derived result (card W43-LIBC-RESULT-ALIAS) has no extern-call model; \
             the checked extern-call route admits only fresh, reallocated or no custody"
        )
    })?;
    Ok(DeclaredContract {
        parameters,
        result,
        result_mode,
    })
}

const fn parameter_mode(mode: ResourceParameterMode) -> ContractParameterMode {
    match mode {
        ResourceParameterMode::Borrowed => ContractParameterMode::Borrowed,
        ResourceParameterMode::Owned { nullable } => ContractParameterMode::Owned { nullable },
        ResourceParameterMode::Consumed { nullable } => {
            ContractParameterMode::Consumed { nullable }
        }
        ResourceParameterMode::Released { nullable } => {
            ContractParameterMode::Released { nullable }
        }
        ResourceParameterMode::Out { nullable } => ContractParameterMode::Out { nullable },
        ResourceParameterMode::Stored {
            owner_parameter,
            nullable,
        } => ContractParameterMode::Stored {
            owner: owner_parameter,
            nullable,
        },
        ResourceParameterMode::Removed {
            owner_parameter,
            nullable,
        } => ContractParameterMode::Removed {
            owner: owner_parameter,
            nullable,
        },
    }
}

/// `None` for a derived result: the extern-call model has no constructor for a
/// non-owning result that aliases a parameter, so the declaration is refused
/// rather than projected onto `None` (which would drop the alias).
const fn return_mode(mode: ResourceReturnMode) -> Option<ContractReturnMode> {
    match mode {
        ResourceReturnMode::None => Some(ContractReturnMode::None),
        ResourceReturnMode::Owned { nullable } => Some(ContractReturnMode::Owned { nullable }),
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => Some(ContractReturnMode::Reallocated {
            parameter,
            nullable,
        }),
        ResourceReturnMode::Derived { .. } | ResourceReturnMode::Removed { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{declared_contract, declared_signatures};
    use crate::contract_loader::load_resource_contract_sources;

    /// Card W43-LIBC-RESULT-ALIAS: the checked extern-call model has no
    /// derived-result constructor, so a derived declaration is refused by
    /// name rather than projected onto `None`, which would drop the alias.
    #[test]
    fn a_derived_result_is_refused_not_projected_onto_none() {
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "k.qcore".to_owned(),
            "module contract;\n\
             extern fn k_chr(s: ptr<const, u8>, c: i32) -> ptr<mut, u8>\n\
             contract {\n    requires resource_contract_v1;\n    \
             ensures normal resource_borrowed(s);\n    \
             ensures normal resource_borrowed(c);\n    \
             ensures normal resource_return_interior(s, nullable);\n};\n\
             extern fn k_len(s: ptr<const, u8>) -> usize\n\
             contract {\n    requires resource_contract_v1;\n    \
             ensures normal resource_borrowed(s);\n    \
             ensures normal resource_return_none();\n};\n"
                .to_owned(),
        )]))
        .expect("the module loads");
        let signatures = declared_signatures(&contracts).expect("signatures read");
        let refusal = declared_contract(
            "k_chr",
            &signatures["k_chr"],
            contracts.contract("k_chr").expect("declared"),
        )
        .expect_err("a derived result has no extern-call model");
        assert!(refusal.contains("derived result"), "{refusal}");
        // The neutral neighbour in the same module is unaffected.
        assert!(
            declared_contract(
                "k_len",
                &signatures["k_len"],
                contracts.contract("k_len").expect("declared"),
            )
            .is_ok()
        );
    }

    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the extern-call model has one
    /// custody kind, so a kinded declaration — pointer- or integer-carried —
    /// is refused by name rather than projected onto the heap kind.
    #[test]
    fn a_non_heap_kind_is_refused_not_projected_onto_the_heap() {
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "k.qcore".to_owned(),
            "module contract;\n\
             extern fn k_open(p: ptr<const, u8>) -> ptr<mut, u8>\n\
             contract {\n    requires resource_contract_v1;\n    \
             ensures normal resource_borrowed(p);\n    \
             ensures normal resource_return_owned(nullable);\n    \
             ensures normal resource_return_kind(stream);\n};\n\
             extern fn k_close(fd: i32) -> i32\n\
             contract {\n    requires resource_contract_v1;\n    \
             ensures normal resource_released(fd, valid);\n    \
             ensures normal resource_kind(fd, descriptor);\n    \
             ensures normal resource_return_none();\n};\n"
                .to_owned(),
        )]))
        .expect("the module loads");
        let signatures = declared_signatures(&contracts).expect("signatures read");
        for name in ["k_open", "k_close"] {
            let refusal = declared_contract(
                name,
                &signatures[name],
                contracts.contract(name).expect("declared"),
            )
            .expect_err("a kinded declaration has no extern-call model");
            assert!(refusal.contains("non-heap resource kind"), "{refusal}");
        }
    }
}
