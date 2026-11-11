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

//! Card RULE34-PILOT: the gateway as host of the transfer position.

use std::collections::BTreeMap;

use qcore_gateway_rule_api::DeclaredEffect;
use qcore_schemas::contract::{
    ContainerEffect, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

use super::{container_order, declared_effects, unordered_destroyer};
use crate::gateway_rules::test_override::with_rules_off;
use crate::legacy_c::container_publication::{Container, Entry, Operand, Operands};

/// `f(e, h)`: `e`'s mode and container effect as given, `h` a borrowed list
/// head that `link` (when set) publishes `e` into.
fn contract(
    entry: ResourceParameterMode,
    effect: Option<ContainerEffect>,
    link: bool,
) -> ResourceFunctionContract {
    let mut container_effects = BTreeMap::new();
    if let Some(effect) = effect {
        container_effects.insert(0, effect);
    }
    if link {
        container_effects.insert(
            1,
            ContainerEffect::Published {
                container_parameter: 0,
            },
        );
    }
    ResourceFunctionContract {
        parameters: vec![entry, ResourceParameterMode::Borrowed],
        result: ResourceReturnMode::None,
        container_effects,
        ..ResourceFunctionContract::default()
    }
}

fn unlink(name: &str) -> Operand {
    Operand::Unpublished {
        entry: Entry::Name(name.to_owned()),
        foreign: None,
    }
}

fn link(name: &str) -> Operand {
    Operand::Published {
        entry: Entry::Name(name.to_owned()),
        container: Container::Other("e".to_owned()),
        formal: false,
    }
}

const RELEASED: ResourceParameterMode = ResourceParameterMode::Released { nullable: false };

#[test]
fn the_query_reads_each_effect_beside_its_parameter_mode() {
    let destroyer = contract(RELEASED, Some(ContainerEffect::Unpublished), false);
    assert_eq!(
        declared_effects(&destroyer),
        [DeclaredEffect {
            unlinks: true,
            releases: true
        }]
    );
    let plain_unlink = contract(
        ResourceParameterMode::Borrowed,
        Some(ContainerEffect::Unpublished),
        true,
    );
    assert_eq!(
        declared_effects(&plain_unlink),
        [
            DeclaredEffect {
                unlinks: true,
                releases: false
            },
            DeclaredEffect {
                unlinks: false,
                releases: false
            }
        ]
    );
}

#[test]
fn rule_34_orders_the_destroyers_unlink_first_and_off_the_guard_refuses_the_call() {
    let destroyer = contract(RELEASED, Some(ContainerEffect::Unpublished), true);
    let operands = Operands(vec![unlink("e"), link("h")]);
    // On: exactly the old `destroyer_split`.
    assert_eq!(
        container_order("destroy", &destroyer, &operands),
        Some((Operands(vec![unlink("e")]), Operands(vec![link("h")])))
    );
    // Off: the arm is not asked and the host's guard refuses the call.
    with_rules_off("34", || {
        assert_eq!(container_order("destroy", &destroyer, &operands), None);
    });
    // Another rule off leaves rule 34 ordering.
    with_rules_off("35", || {
        assert!(container_order("destroy", &destroyer, &operands).is_some());
    });
    assert!(unordered_destroyer("destroy").contains("`destroy`"));
}

#[test]
fn a_call_without_a_destroyer_keeps_the_host_order_on_and_off() {
    let calls = [
        contract(
            ResourceParameterMode::Borrowed,
            Some(ContainerEffect::Unpublished),
            false,
        ),
        contract(ResourceParameterMode::Borrowed, None, true),
        contract(RELEASED, None, true),
    ];
    for call in &calls {
        let operands = Operands(
            call.container_effects
                .values()
                .map(|effect| match effect {
                    ContainerEffect::Unpublished => unlink("e"),
                    ContainerEffect::Published { .. } => link("e"),
                })
                .collect(),
        );
        let host = Some((Operands(Vec::new()), operands.clone()));
        assert_eq!(container_order("f", call, &operands), host);
        with_rules_off("34", || {
            assert_eq!(container_order("f", call, &operands), host);
        });
    }
}
