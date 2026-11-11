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

//! The reason-key reservation: every reason key a package emits is reserved
//! in its registry row, under one closed §L21.1 category, and no two packages
//! reserve the same key.

use std::collections::BTreeMap;

use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

/// One reserved reason key and its category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReasonKey {
    pub key: &'static str,
    pub category: ReasonCategory,
}

fn kebab(key: &str) -> bool {
    !key.is_empty()
        && !key.starts_with('-')
        && !key.ends_with('-')
        && !key.contains("--")
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Check every `(rule id, reserved key)` pair: each key is kebab-case, speaks
/// about the analysed source or the analyser (a package's refusal is never an
/// environment fact, §L21.1 rule 1), and is reserved by one rule only.
///
/// # Errors
///
/// The first malformed, environment-subject or doubly reserved key, by name.
pub fn check_reservations<'a>(
    reservations: impl IntoIterator<Item = (&'a str, &'a ReasonKey)>,
) -> Result<(), String> {
    let mut owners = BTreeMap::<&str, &str>::new();
    for (rule, reserved) in reservations {
        if !kebab(reserved.key) {
            return Err(format!(
                "rule {rule}: reason key `{}` is not kebab-case",
                reserved.key
            ));
        }
        if reserved.category.subject() == ReasonSubject::Environment {
            return Err(format!(
                "rule {rule}: reason key `{}` has an environment category; a package's \
                 refusal speaks about the source or the analyser",
                reserved.key
            ));
        }
        if let Some(owner) = owners.insert(reserved.key, rule) {
            return Err(format!(
                "reason key `{}` is reserved twice (rule {owner}, rule {rule})",
                reserved.key
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ReasonKey, check_reservations};
    use qcore_schemas::reason::ReasonCategory;

    const SELF_RELIANCE: ReasonKey = ReasonKey {
        key: "caller-frame-self-reliance",
        category: ReasonCategory::IncompleteObligationCoverage,
    };

    #[test]
    fn a_key_is_reserved_once_kebab_case_and_never_an_environment_fact() {
        assert_eq!(check_reservations([("32a", &SELF_RELIANCE)]), Ok(()));
        assert!(check_reservations([("32a", &SELF_RELIANCE), ("33", &SELF_RELIANCE)]).is_err());
        let malformed = ReasonKey {
            key: "Caller_Frame",
            category: ReasonCategory::IncompleteObligationCoverage,
        };
        assert!(check_reservations([("32a", &malformed)]).is_err());
        let environment = ReasonKey {
            key: "no-clang",
            category: ReasonCategory::EnvironmentUnavailable,
        };
        assert!(check_reservations([("x", &environment)]).is_err());
    }
}
