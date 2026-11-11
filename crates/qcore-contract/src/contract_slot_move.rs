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

//! Card **SLOT-MOVE-AND-INTERIOR-BORROW**: the slot clauses of resource
//! vocabulary version 1 (contract specification §C04.15, §C04.16, grammar
//! G05), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_slot_moved(s, n);       // class 2 (rule 12x)
//! ensures normal resource_out_derived(s, p, n);   // a derived out slot (rule 12z)
//! ```
//!
//! `s` names a pointer-to-pointer parameter whose mode is `resource_borrowed`
//! and which carries no kind: `ptr<mut, ptr<mut, u8>>` for a slot move, that
//! or `ptr<mut, ptr<const, u8>>` for a derived out slot. `p` names a distinct
//! depth-one pointer parameter, `resource_borrowed`, with no kind. A slot is
//! named by at most one slot clause. A declaration carrying either clause is
//! withheld from the function map with the retaining declarations (the
//! loader, [`super::load_resource_contract_sources`]).
//!
//! The class-8 result, `resource_return_borrowed_from(p, n)` (§C04.3.4, rule
//! 12y), is a result mode and is read by the loader's result reader.

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceFunctionContract, ResourceKind, ResourceParameterMode};
use qcore_schemas::slot_move::{DerivedOut, SlotMove};

/// `resource_slot_moved(s, n)`.
pub(super) const SLOT_MOVED: &str = "resource_slot_moved";

/// `resource_out_derived(s, p, n)`.
pub(super) const OUT_DERIVED: &str = "resource_out_derived";

/// `resource_return_borrowed_from(p, n)`.
pub(super) const RETURN_BORROWED_FROM: &str = "resource_return_borrowed_from";

/// The slot type a slot move names.
const MOVED_SLOT: &str = "ptr<mut, ptr<mut, u8>>";

/// The slot types a derived out slot names.
const OUT_SLOTS: [&str; 2] = ["ptr<mut, ptr<mut, u8>>", "ptr<mut, ptr<const, u8>>"];

/// The depth-one pointer types a derived out slot's base may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// The result types a borrowed result may have: a depth-one pointer, or a
/// pointer to an array of child pointers the parent owns
/// (`rd_kafka_admin_result_ret_groups` returns `const T **`).
pub const BORROWED_RESULTS: [&str; 4] = [
    "ptr<mut, u8>",
    "ptr<const, u8>",
    "ptr<mut, ptr<mut, u8>>",
    "ptr<mut, ptr<const, u8>>",
];

/// A slot clause as written, before the declaration's parameters are known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WrittenSlot {
    Moved {
        slot: String,
        null: String,
    },
    OutDerived {
        slot: String,
        base: String,
        null: String,
    },
}

/// Read one slot clause from its predicate words, `None` for any other
/// predicate.
pub(super) fn clause(words: &[&str]) -> Option<WrittenSlot> {
    match words {
        [SLOT_MOVED, slot, null] => Some(WrittenSlot::Moved {
            slot: (*slot).to_owned(),
            null: (*null).to_owned(),
        }),
        [OUT_DERIVED, slot, base, null] => Some(WrittenSlot::OutDerived {
            slot: (*slot).to_owned(),
            base: (*base).to_owned(),
            null: (*null).to_owned(),
        }),
        _ => None,
    }
}

fn nullability(word: &str) -> Result<bool, String> {
    match word {
        "nullable" => Ok(true),
        "nonnull" => Ok(false),
        other => Err(format!(
            "a slot clause takes `nullable` or `nonnull`, not `{other}`"
        )),
    }
}

/// Validate the slot clauses of one declaration whose custody contract
/// `contract` is already loaded.
pub(super) fn declaration(
    written: &[WrittenSlot],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<(Vec<SlotMove>, Vec<DerivedOut>), String> {
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    let borrowed = |at: usize, name: &str, role: &str| -> Result<(), String> {
        if contract.parameters[at] != ResourceParameterMode::Borrowed {
            return Err(format!(
                "a {role} names a `resource_borrowed` parameter, not `{name}`"
            ));
        }
        if parameter_kinds[at].is_some() {
            return Err(format!("a {role} carries no kind (`{name}`)"));
        }
        Ok(())
    };
    let mut slots: Vec<usize> = Vec::new();
    let mut moves = Vec::new();
    let mut outs = Vec::new();
    for clause in written {
        let (slot_name, null) = match clause {
            WrittenSlot::Moved { slot, null } | WrittenSlot::OutDerived { slot, null, .. } => {
                (slot.as_str(), null.as_str())
            }
        };
        let slot = index(slot_name)?;
        if slots.contains(&slot) {
            return Err(format!(
                "a slot is named by one slot clause (`{slot_name}`)"
            ));
        }
        slots.push(slot);
        let nullable = nullability(null)?;
        match clause {
            WrittenSlot::Moved { .. } => {
                if parameter_types[slot] != MOVED_SLOT {
                    return Err(format!(
                        "a moved slot names a `{MOVED_SLOT}` parameter, not `{slot_name}`"
                    ));
                }
                borrowed(slot, slot_name, "moved slot")?;
                moves.push(SlotMove { slot, nullable });
            }
            WrittenSlot::OutDerived { base, .. } => {
                if !OUT_SLOTS.contains(&parameter_types[slot].as_str()) {
                    return Err(format!(
                        "a derived out slot names a pointer-to-pointer parameter, not `{slot_name}`"
                    ));
                }
                borrowed(slot, slot_name, "derived out slot")?;
                let base_at = index(base)?;
                if base_at == slot {
                    return Err(format!("a derived out slot is not its own base (`{base}`)"));
                }
                if !DEPTH_ONE.contains(&parameter_types[base_at].as_str()) {
                    return Err(format!(
                        "a derived out slot's base names a depth-one pointer parameter, not `{base}`"
                    ));
                }
                borrowed(base_at, base, "derived out slot's base")?;
                outs.push(DerivedOut {
                    slot,
                    base: base_at,
                    nullable,
                    const_dropped: parameter_types[base_at] == "ptr<const, u8>"
                        && parameter_types[slot] == "ptr<mut, ptr<mut, u8>>",
                });
            }
        }
    }
    Ok((moves, outs))
}
