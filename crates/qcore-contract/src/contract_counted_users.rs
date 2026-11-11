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

//! Card **W45-FB-RETRY-LIFETIME**: the counted-user and intrusive-container
//! clauses of resource vocabulary version 1 (contract specification §C04.7 and
//! §C04.8), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_user_added(p, COUNTER);
//! ensures normal resource_user_dropped(p, COUNTER);
//! ensures normal resource_user_dropped_checked(p, COUNTER);
//! ensures normal resource_user_release_check(p, COUNTER);
//! ensures normal resource_user_neutral();
//! ensures normal resource_user_exit(p, RETURNED, DELTA);
//! ensures normal resource_published(entry, container);
//! ensures normal resource_unpublished(entry);
//! ```
//!
//! `COUNTER` names the member of the designated object that holds the count
//! (`users` for `task->users`): a body defined in the analysed unit is checked
//! against the operation by its writes to that member, and a derivation reads
//! a direct write to it as a counter change.
//!
//! `RETURNED` is a canonical decimal integer (the literal-argument premise's
//! spelling rules: no leading zero, no `-0`), and `DELTA` one of the words
//! `added` (+1), `restored` (0) and `dropped` (-1), so a net user count is
//! never spelled as arithmetic.
//!
//! Every position these clauses name keeps its `resource_borrowed` custody
//! mode, which [`validate`] requires: a counted user is not custody of the
//! object, and a publication's custody move is the container effect itself,
//! which a consumer that does not apply it must refuse. One exception (card
//! W48-SCHED-CLEANUP-PRODUCTION, gateway rule 34): `resource_unpublished(p)`
//! may sit beside `resource_released(p, …)`, the element destroyer that
//! unlinks the object and then ends it; a consumer applies the unlink first.

use std::collections::BTreeMap;

use qcore_frontend::lexer::{Symbol, TokenKind};
use qcore_schemas::contract::{
    ContainerEffect, CountedUserClauses, ResourceParameterMode, UserEffect, UserExitClause,
    UserOperation,
};

use super::token_kinds;

/// One clause of this family, with its parameters still spelled by name.
pub(super) enum Clause {
    Effect(String, UserOperation),
    Neutral,
    Exit {
        parameter: String,
        returned: i64,
        delta: i8,
    },
    Published {
        entry: String,
        container: String,
    },
    Unpublished(String),
}

/// The clauses of one declaration, by parameter index.
#[derive(Default)]
pub(super) struct Collected {
    pub container_effects: BTreeMap<usize, ContainerEffect>,
    pub users: CountedUserClauses,
}

/// Read `expression` as a clause of this family. `Ok(None)` means it is not
/// one, and the caller reads it as written; a malformed clause of this family
/// is an error, so the whole module rejects.
pub(super) fn parse(words: Option<&[&str]>, expression: &str) -> Result<Option<Clause>, String> {
    if let Some(words) = words {
        return Ok(match words {
            [
                name @ ("resource_user_added"
                | "resource_user_dropped"
                | "resource_user_dropped_checked"
                | "resource_user_release_check"),
                p,
                counter,
            ] => {
                let effect = match *name {
                    "resource_user_added" => UserEffect::Added,
                    "resource_user_dropped" => UserEffect::Dropped,
                    "resource_user_dropped_checked" => UserEffect::DroppedChecked,
                    _ => UserEffect::ReleaseCheck,
                };
                Some(Clause::Effect(
                    (*p).to_owned(),
                    UserOperation {
                        effect,
                        counter: (*counter).to_owned(),
                    },
                ))
            }
            ["resource_user_neutral"] => Some(Clause::Neutral),
            ["resource_published", entry, container] => Some(Clause::Published {
                entry: (*entry).to_owned(),
                container: (*container).to_owned(),
            }),
            ["resource_unpublished", entry] => Some(Clause::Unpublished((*entry).to_owned())),
            [
                "resource_user_added"
                | "resource_user_dropped"
                | "resource_user_dropped_checked"
                | "resource_user_release_check"
                | "resource_user_neutral"
                | "resource_published"
                | "resource_unpublished",
                ..,
            ] => {
                return Err(format!(
                    "malformed counted-user or container clause `{expression}`"
                ));
            }
            _ => None,
        });
    }
    // Not an identifier-only predicate: the exit clause is the one clause of
    // this family carrying a value.
    let tokens = token_kinds(expression)?;
    if !matches!(tokens.first(), Some(TokenKind::Word(word)) if word == "resource_user_exit") {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed exit clause `{expression}`: write \
             `resource_user_exit(PARAMETER, RETURNED, added|restored|dropped)`"
        )
    };
    let [
        TokenKind::Word(_),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        rest @ ..,
    ] = tokens.as_slice()
    else {
        return Err(malformed());
    };
    let (negative, digits, rest) = match rest {
        [
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(digits),
            rest @ ..,
        ] => (true, digits, rest),
        [TokenKind::Integer(digits), rest @ ..] => (false, digits, rest),
        _ => return Err(malformed()),
    };
    let [
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(delta),
        TokenKind::Symbol(Symbol::RightParen),
    ] = rest
    else {
        return Err(malformed());
    };
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || (negative && digits == "0")
    {
        return Err(malformed());
    }
    let magnitude: i64 = digits.parse().map_err(|_| malformed())?;
    let delta = match delta.as_str() {
        "added" => 1,
        "restored" => 0,
        "dropped" => -1,
        _ => return Err(malformed()),
    };
    Ok(Some(Clause::Exit {
        parameter: parameter.clone(),
        returned: if negative { -magnitude } else { magnitude },
        delta,
    }))
}

/// Record one parsed clause. Duplicates reject the module.
pub(super) fn record(
    collected: &mut Collected,
    clause: Clause,
    index: &dyn Fn(&str) -> Result<usize, String>,
) -> Result<(), String> {
    match clause {
        Clause::Effect(parameter, operation) => {
            let position = index(&parameter)?;
            if collected
                .users
                .effects
                .insert(position, operation)
                .is_some()
            {
                return Err(format!("duplicate counted-user effect for {parameter}"));
            }
        }
        Clause::Neutral => {
            if std::mem::replace(&mut collected.users.neutral, true) {
                return Err("duplicate resource_user_neutral".into());
            }
        }
        Clause::Exit {
            parameter,
            returned,
            delta,
        } => {
            let parameter = index(&parameter)?;
            if collected
                .users
                .exits
                .iter()
                .any(|exit| exit.parameter == parameter && exit.returned == returned)
            {
                return Err("duplicate exit clause for one parameter and returned value".into());
            }
            collected.users.exits.push(UserExitClause {
                parameter,
                returned,
                delta,
            });
        }
        Clause::Published { entry, container } => {
            let position = index(&entry)?;
            let container_parameter = index(&container)?;
            if collected
                .container_effects
                .insert(
                    position,
                    ContainerEffect::Published {
                        container_parameter,
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate container effect for {entry}"));
            }
        }
        Clause::Unpublished(entry) => {
            let position = index(&entry)?;
            if collected
                .container_effects
                .insert(position, ContainerEffect::Unpublished)
                .is_some()
            {
                return Err(format!("duplicate container effect for {entry}"));
            }
        }
    }
    Ok(())
}

/// The placement rules, once the whole declaration is read.
///
/// * every named position is a `resource_borrowed` pointer parameter;
/// * a published entry and its container are distinct, and the container is
///   itself neither published nor unpublished;
/// * `resource_user_neutral()` stands alone among the counted-user clauses;
/// * an exit clause needs an integer result, and a parameter carries either an
///   effect or exit clauses, never both;
/// * the literal delta is one of the three words, already checked by [`parse`].
pub(super) fn validate(
    collected: &Collected,
    parameters: &[ResourceParameterMode],
    parameter_types: &[String],
    return_type: &str,
) -> Result<(), String> {
    let pointer = |position: usize| parameter_types[position].starts_with("ptr<");
    let borrowed_pointer = |position: usize, what: &str| {
        if parameters[position] != ResourceParameterMode::Borrowed || !pointer(position) {
            Err(format!(
                "{what} requires a resource_borrowed pointer parameter"
            ))
        } else {
            Ok(())
        }
    };
    for (&position, effect) in &collected.container_effects {
        // Card W48-SCHED-CLEANUP-PRODUCTION (gateway rule 34): an unlink may
        // also sit beside a release of the same object, the element
        // destroyer: the call unlinks the object and then ends it. No other
        // custody mode carries a container effect.
        let destroyer = matches!(effect, ContainerEffect::Unpublished)
            && matches!(parameters[position], ResourceParameterMode::Released { .. })
            && pointer(position);
        if !destroyer {
            borrowed_pointer(position, "a container effect")?;
        }
        if let ContainerEffect::Published {
            container_parameter,
        } = effect
        {
            borrowed_pointer(*container_parameter, "a publication's container")?;
            if *container_parameter == position
                || collected
                    .container_effects
                    .contains_key(container_parameter)
            {
                return Err("a publication's container must be a distinct parameter with no container effect".into());
            }
        }
    }
    for &position in collected.users.effects.keys() {
        borrowed_pointer(position, "a counted-user effect")?;
    }
    if collected.users.neutral
        && (!collected.users.effects.is_empty() || !collected.users.exits.is_empty())
    {
        return Err("resource_user_neutral() states that no user changes; it stands alone".into());
    }
    if !collected.users.exits.is_empty()
        && !super::SCALAR_ABI_TYPES
            .iter()
            .any(|ty| *ty == return_type && ty.starts_with(['i', 'u']))
    {
        return Err("an exit clause requires an integer result".into());
    }
    for exit in &collected.users.exits {
        borrowed_pointer(exit.parameter, "an exit clause")?;
        if collected.users.effects.contains_key(&exit.parameter) {
            return Err("a parameter carries either a counted-user effect or exit clauses".into());
        }
    }
    Ok(())
}
