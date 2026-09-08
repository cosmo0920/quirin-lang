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
    Ok(DeclaredContract {
        parameters,
        result,
        result_mode: return_mode(contract.result),
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

const fn return_mode(mode: ResourceReturnMode) -> ContractReturnMode {
    match mode {
        ResourceReturnMode::None => ContractReturnMode::None,
        ResourceReturnMode::Owned { nullable } => ContractReturnMode::Owned { nullable },
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => ContractReturnMode::Reallocated {
            parameter,
            nullable,
        },
    }
}
