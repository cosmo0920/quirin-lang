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

//! Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES**: the type-switched release
//! (contract specification §C04.19.2, grammar G05, gateway rule 12o), read
//! total-or-reject like every other clause.
//!
//! ```text
//! requires resource_discriminant(P, FIELD, MASK);
//! ensures normal resource_arm_released(P, PATH) when FIELD == VALUE;
//! ensures normal resource_arm_empty(P) when FIELD == VALUE;
//! ensures normal resource_arm_released(P, PATH) otherwise;
//! ensures normal resource_arm_empty(P) otherwise;
//! ```
//!
//! USER DECISION 2026-10-14 (clause 2's form): per-arm `when` / `otherwise`
//! clauses with `resource_discriminant`, `resource_arm_released` and
//! `resource_arm_empty`; decimal constants citing the source. `P` names a
//! depth-one pointer parameter the declaration releases; `FIELD` and `PATH`
//! are member paths (`rko_type`, `rko_u.offset_commit.partitions`); `MASK`
//! (the bits the body clears before switching) and `VALUE` are canonical
//! decimals, never names.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::arm_release::{ArmClause, ArmEffect, ArmGuard, Discriminant};
use qcore_schemas::contract::{ResourceFunctionContract, ResourceKind, ResourceParameterMode};

pub(super) const DISCRIMINANT: &str = "resource_discriminant";
pub(super) const ARM_RELEASED: &str = "resource_arm_released";
pub(super) const ARM_EMPTY: &str = "resource_arm_empty";
const WHEN: &str = "when";
const OTHERWISE: &str = "otherwise";

/// The depth-one pointer types the discriminated object may have.
const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];

/// `resource_discriminant(P, FIELD, MASK)` as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenDiscriminant {
    pub(super) object: String,
    pub(super) field: String,
    pub(super) mask: u64,
}

/// An arm clause as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenArm {
    pub(super) object: String,
    /// `None` for `resource_arm_empty`.
    pub(super) path: Option<String>,
    /// `None` for `otherwise`; otherwise the guard's field and value.
    pub(super) guard: Option<(String, i64)>,
}

/// Whether `text` is a member path: identifiers joined by `.`.
pub(super) fn is_path(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with(|ch: char| ch.is_ascii_digit())
                && segment
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        })
}

fn is_identifier(text: &str) -> bool {
    is_path(text) && !text.contains('.')
}

/// A canonical decimal (C04.5's spelling): an optional leading `-`, no
/// leading zero, no `-0`, no suffix.
fn canonical(text: &str) -> Option<i128> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || text == "-0"
    {
        return None;
    }
    text.parse().ok()
}

/// Split `KEYWORD(A, B, …) REST` into its keyword's arguments and the text
/// after the closing parenthesis; `None` when `expression` does not start
/// with `keyword(`.
fn call<'a>(expression: &'a str, keyword: &str) -> Option<(Vec<&'a str>, &'a str)> {
    let rest = expression.trim().strip_prefix(keyword)?;
    let rest = rest.trim_start().strip_prefix('(')?;
    let close = rest.find(')')?;
    let arguments = rest[..close].split(',').map(str::trim).collect();
    Some((arguments, rest[close + 1..].trim()))
}

/// Read `requires resource_discriminant(P, FIELD, MASK)`, `None` for any
/// other requirement.
pub(super) fn discriminant_clause(expression: &str) -> Result<Option<WrittenDiscriminant>, String> {
    let Some((arguments, rest)) = call(expression, DISCRIMINANT) else {
        return Ok(None);
    };
    let malformed = || {
        format!(
            "malformed `{DISCRIMINANT}` requirement `{expression}`: write \
             `requires {DISCRIMINANT}(P, FIELD, MASK);` with a canonical decimal mask"
        )
    };
    let [object, field, mask] = arguments.as_slice() else {
        return Err(malformed());
    };
    if !rest.is_empty() || !is_identifier(object) || !is_path(field) {
        return Err(malformed());
    }
    let mask = canonical(mask)
        .and_then(|mask| u64::try_from(mask).ok())
        .ok_or_else(malformed)?;
    Ok(Some(WrittenDiscriminant {
        object: (*object).to_owned(),
        field: (*field).to_owned(),
        mask,
    }))
}

/// Read an arm clause, `None` for any other predicate. A clause that names
/// an arm keyword and is malformed rejects.
pub(super) fn arm_clause(expression: &str) -> Result<Option<WrittenArm>, String> {
    let (keyword, arguments, rest) = if let Some((arguments, rest)) = call(expression, ARM_RELEASED)
    {
        (ARM_RELEASED, arguments, rest)
    } else if let Some((arguments, rest)) = call(expression, ARM_EMPTY) {
        (ARM_EMPTY, arguments, rest)
    } else {
        return Ok(None);
    };
    let malformed = |why: &str| format!("malformed `{keyword}` clause `{expression}`: {why}");
    let (object, path) = match (keyword, arguments.as_slice()) {
        (ARM_RELEASED, [object, path]) if is_identifier(object) && is_path(path) => {
            ((*object).to_owned(), Some((*path).to_owned()))
        }
        (ARM_EMPTY, [object]) if is_identifier(object) => ((*object).to_owned(), None),
        _ => return Err(malformed("unsupported arguments")),
    };
    let guard = if rest == OTHERWISE {
        None
    } else {
        let guard = rest
            .strip_prefix(WHEN)
            .filter(|guard| guard.starts_with(char::is_whitespace))
            .ok_or_else(|| malformed("an arm needs `when FIELD == VALUE` or `otherwise`"))?;
        let (field, value) = guard
            .split_once("==")
            .ok_or_else(|| malformed("an arm's guard is `FIELD == VALUE`"))?;
        let (field, value) = (field.trim(), value.trim());
        if !is_path(field) {
            return Err(malformed("an arm's guard names the discriminant's member"));
        }
        let value = canonical(value)
            .and_then(|value| i64::try_from(value).ok())
            .ok_or_else(|| malformed("an arm's value is a canonical decimal"))?;
        Some((field.to_owned(), value))
    };
    Ok(Some(WrittenArm {
        object,
        path,
        guard,
    }))
}

/// Validate one declaration's discriminant and arms, `(None, [])` when it
/// carries neither.
pub(super) fn declaration(
    discriminants: &[WrittenDiscriminant],
    arms: &[WrittenArm],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    parameter_kinds: &[Option<ResourceKind>],
    names: &BTreeMap<&str, usize>,
) -> Result<(Option<Discriminant>, Vec<ArmClause>), String> {
    if discriminants.is_empty() && arms.is_empty() {
        return Ok((None, Vec::new()));
    }
    let [written] = discriminants else {
        return Err(if discriminants.is_empty() {
            format!("an arm clause needs `requires {DISCRIMINANT}(P, FIELD, MASK);`")
        } else {
            format!("a declaration has one `{DISCRIMINANT}`")
        });
    };
    if arms.is_empty() {
        return Err(format!("`{DISCRIMINANT}` without an arm clause"));
    }
    let parameter = *names
        .get(written.object.as_str())
        .ok_or_else(|| format!("unknown parameter {}", written.object))?;
    if !DEPTH_ONE.contains(&parameter_types[parameter].as_str()) {
        return Err(format!(
            "a discriminated object is a depth-one pointer parameter, not `{}`",
            written.object
        ));
    }
    if !matches!(
        contract.parameters[parameter],
        ResourceParameterMode::Released { .. }
    ) {
        return Err(format!(
            "an arm releases members of an object the declaration releases: \
             `{}` is not `resource_released`",
            written.object
        ));
    }
    if parameter_kinds[parameter].is_some() {
        return Err(format!(
            "a discriminated object carries no kind (`{}`)",
            written.object
        ));
    }
    let mut clauses = Vec::with_capacity(arms.len());
    let mut by_guard: BTreeMap<ArmGuard, (bool, BTreeSet<String>)> = BTreeMap::new();
    for arm in arms {
        if arm.object != written.object {
            return Err(format!(
                "an arm names the discriminated object `{}`, not `{}`",
                written.object, arm.object
            ));
        }
        let guard = match &arm.guard {
            None => ArmGuard::Otherwise,
            Some((field, value)) => {
                if *field != written.field {
                    return Err(format!(
                        "an arm's guard names the discriminant `{}`, not `{field}`",
                        written.field
                    ));
                }
                if written.mask != 0 && (*value < 0 || (*value as u64) & written.mask != 0) {
                    return Err(format!(
                        "an arm value {value} carries a bit the mask {} clears: it never matches",
                        written.mask
                    ));
                }
                ArmGuard::Equals { value: *value }
            }
        };
        let entry = by_guard.entry(guard).or_default();
        let effect = match &arm.path {
            None => {
                if entry.0 || !entry.1.is_empty() {
                    return Err("an empty arm states no other clause for its guard".into());
                }
                entry.0 = true;
                ArmEffect::Empty
            }
            Some(path) => {
                if entry.0 {
                    return Err("an empty arm states no other clause for its guard".into());
                }
                if *path == written.field {
                    return Err("an arm does not release the discriminant itself".into());
                }
                if !entry.1.insert(path.clone()) {
                    return Err(format!("duplicate arm release `{path}`"));
                }
                ArmEffect::Released { path: path.clone() }
            }
        };
        clauses.push(ArmClause { guard, effect });
    }
    Ok((
        Some(Discriminant {
            parameter,
            field: written.field.clone(),
            mask: written.mask,
        }),
        clauses,
    ))
}
