//! Card **RULE34-PILOT**: the gateway as host of the transfer position (route
//! map §10, `qcore_gateway_rule_api::Position::Transfer`).
//!
//! The `ContractCall` arm of `legacy_transfer.rs` asks [`container_order`]
//! for the order of one admitted contract call's container operands: those
//! applied before the call's own reading, and those applied after it. The
//! transfer arms (today rule 34's `destroyer-order`,
//! `rule-34-element-destroyer`) are asked through
//! [`crate::arm_obligations::consult`] and their run-time switches
//! ([`crate::gateway_rules::enabled`]). When no arm admits, the host's order
//! stands, every operand after the call, except for a call that declares an
//! unlink of a parameter it releases: no transfer rule orders it, and the
//! host's order would release the object before unlinking it, which rule 34
//! forbids (`docs/agents/rules/34.md`) and nothing models. That call is
//! refused ([`unordered_destroyer`]). The guard is the host's end of the
//! table, so it is not switchable: switched off, rule 34 only removes its
//! admission, and the call reads `Unknown` as it did before rule 34, when no
//! such declaration was admitted.
//!
//! An admission is read fail-closed: one phase per declared effect, else a
//! refusal. Card ARM-OBLIGATIONS: its obligations are carried into the rows
//! of the unit whose analysis asked ([`crate::arm_obligations`]), and an
//! admission asked outside one, where no row could carry them, is refused.

use qcore_gateway_rule_api::{DeclaredEffect, Phase, TransferArm, TransferQuery};
use qcore_schemas::contract::{ContainerEffect, ResourceFunctionContract, ResourceParameterMode};

use crate::legacy_c::container_publication::Operands;

/// The transfer arms, in consultation order.
const TRANSFER_ARMS: [&dyn TransferArm; 1] = [&rule_34_element_destroyer::ElementDestroyer];

/// The call's declared container effects as the transfer query reads them, in
/// the declaration's position order (the order the operands are built in).
#[must_use]
pub(crate) fn declared_effects(contract: &ResourceFunctionContract) -> Vec<DeclaredEffect> {
    contract
        .container_effects
        .iter()
        .map(|(&index, effect)| DeclaredEffect {
            unlinks: matches!(effect, ContainerEffect::Unpublished),
            releases: matches!(
                contract.parameters.get(index),
                Some(ResourceParameterMode::Released { .. })
            ),
        })
        .collect()
}

/// The host's guard: an effect only a transfer rule may order (an unlink of
/// a parameter the same declaration releases).
#[must_use]
fn needs_an_order(effect: &DeclaredEffect) -> bool {
    effect.unlinks && effect.releases
}

/// The phases of one call's effects: an admitting transfer arm's, else the
/// host's order, or `None` when the guard refuses the call.
fn phases(called: &str, effects: &[DeclaredEffect]) -> Option<Vec<Phase>> {
    let query = TransferQuery {
        callee: called,
        effects,
    };
    // A phase list of the wrong length is refused before its obligations
    // are carried, so a refused admission carries nothing.
    let admitted = crate::arm_obligations::consult(
        &TRANSFER_ARMS,
        |arm| arm.id(),
        |arm| crate::gateway_rules::enabled(arm.id()),
        |_, arm| match arm.ask(&query) {
            qcore_gateway_rule_api::Answer::Admit(admission)
                if admission.lowering.len() != effects.len() =>
            {
                qcore_gateway_rule_api::Answer::Refuse
            }
            answer => answer,
        },
    );
    match admitted {
        Some(decided) => Some(decided.lowering),
        None if effects.iter().any(needs_an_order) => None,
        None => Some(vec![Phase::AfterCall; effects.len()]),
    }
}

/// One call's container operands, split into those applied before the call's
/// own reading and those applied after it (operands pair with the contract's
/// container effects by position, as [`crate::legacy_c::container_publication::operands`]
/// builds them); `None` when the host's guard refuses the call.
#[must_use]
pub(crate) fn container_order(
    called: &str,
    contract: &ResourceFunctionContract,
    operands: &Operands,
) -> Option<(Operands, Operands)> {
    let phases = phases(called, &declared_effects(contract))?;
    let mut before = Vec::new();
    let mut after = Vec::new();
    for (phase, operand) in phases.iter().zip(&operands.0) {
        match phase {
            Phase::BeforeCall => before.push(operand.clone()),
            Phase::AfterCall => after.push(operand.clone()),
        }
    }
    Some((Operands(before), Operands(after)))
}

/// The refusal text of a call the guard refuses.
#[must_use]
pub(crate) fn unordered_destroyer(called: &str) -> String {
    format!(
        "element destroyer `{called}`: no transfer rule orders its unlink before its release \
         (gateway rule 34 is off)"
    )
}

#[cfg(test)]
#[path = "../tests/unit/transfer_rules.rs"]
mod tests;
