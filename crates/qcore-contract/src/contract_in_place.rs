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

//! Card **CG25-RESOURCE-KINDS**: the placement rules of an **in-place** kind
//! (contract spec §C04.6.1).
//!
//! An in-place kind (`regex`) is initialised into storage the caller owns: the
//! obligation lives in the object a pointer parameter designates, not in a
//! returned pointer. Its protocol has exactly three positions, all on a
//! `ptr<mut, u8>` / `ptr<const, u8>` that names the object:
//!
//! * **initialise**: `resource_out(p, nullable)` + `resource_kind(p, KIND)`.
//!   The call may fail, and it says so only by its **status**: the result is
//!   an `i32` with result mode `none`, zero on success and nonzero on failure
//!   (the kind's sentinel). A `nonnull` initialiser (an infallible one) has no
//!   declared use and is refused; so is a second in-place initialiser in one
//!   declaration, which one status could not witness.
//! * **finalise**: `resource_released(p, nonnull)` + `resource_kind(p, KIND)`.
//! * **use**: `resource_borrowed(p)` + `resource_kind(p, KIND)`. The call reads
//!   (or updates) the object and leaves its obligation as it was. This is the
//!   one kinded borrowed position: an unkinded borrowed position (`memset`)
//!   may overwrite the object's bytes, so a consumer refuses a tracked
//!   in-place object there.
//!
//! An owned, consumed, stored or removed in-place position, and an in-place
//! result, are refused: storage is neither transferred nor returned.

use super::{DeclaredKinds, ValidityWord};
use qcore_schemas::contract::{ResourceKind, ResourceParameterMode, ResourceReturnMode};

/// One in-place-kinded parameter position.
pub(super) fn check_parameter(
    kind: ResourceKind,
    mode: ResourceParameterMode,
    word: Option<ValidityWord>,
    ty: &str,
) -> Result<(), String> {
    let spelling = kind.spelling();
    if !matches!(ty, "ptr<mut, u8>" | "ptr<const, u8>") {
        return Err(format!(
            "a {spelling} position names the object: `ptr<mut, u8>` or `ptr<const, u8>`"
        ));
    }
    if word == Some(ValidityWord::Integer) {
        return Err(format!(
            "a {spelling} position spells its nullability `nullable` or `nonnull`"
        ));
    }
    match mode {
        ResourceParameterMode::Out { nullable: true } if ty == "ptr<mut, u8>" => Ok(()),
        ResourceParameterMode::Out { nullable: true } => Err(format!(
            "a {spelling} initialiser writes the object: `ptr<mut, u8>`"
        )),
        ResourceParameterMode::Out { nullable: false } => Err(format!(
            "a {spelling} initialiser fails by its status: spell it `nullable`"
        )),
        ResourceParameterMode::Released { nullable: false } => Ok(()),
        ResourceParameterMode::Released { nullable: true } => Err(format!(
            "a {spelling} finaliser names an object: spell it `nonnull`"
        )),
        ResourceParameterMode::Borrowed => Ok(()),
        ResourceParameterMode::Owned { .. }
        | ResourceParameterMode::Consumed { .. }
        | ResourceParameterMode::Stored { .. }
        | ResourceParameterMode::Removed { .. } => Err(format!(
            "a {spelling} object stays in caller storage: only initialise, use and finalise positions"
        )),
    }
}

/// The whole-declaration rules: an initialiser returns its status.
pub(super) fn check_declaration(
    declared: &DeclaredKinds,
    parameters: &[ResourceParameterMode],
    result: ResourceReturnMode,
    return_type: &str,
) -> Result<(), String> {
    let initialisers = parameters
        .iter()
        .enumerate()
        .filter(|(index, mode)| {
            matches!(mode, ResourceParameterMode::Out { .. })
                && declared.parameter_kind(*index).is_in_place()
        })
        .count();
    if initialisers == 0 {
        return Ok(());
    }
    if initialisers > 1 {
        return Err("one status witnesses at most one in-place initialisation".into());
    }
    if result != ResourceReturnMode::None || return_type != "i32" {
        return Err(
            "an in-place initialiser returns its `i32` status with `resource_return_none()`".into(),
        );
    }
    if parameters.iter().enumerate().any(|(index, mode)| {
        !matches!(mode, ResourceParameterMode::Borrowed)
            && !declared.parameter_kind(index).is_in_place()
    }) {
        return Err("an in-place initialiser moves no other custody".into());
    }
    Ok(())
}
