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

//! OpenSSL's own contracts, carried by the plugin
//! (`contracts/openssl-3.5.5.json`, schema [`SCHEMA`]).
//!
//! One **binding** per OpenSSL function the Fluent Bit routes call
//! uncontracted (the librdkafka census and the round-2 ranking): its public
//! header and prototype, a contract **class** with the contract's text, and,
//! for a class the checker has a rule for, the defining file, the object the
//! pinned build compiles it to, and the flag set it is compiled with. A
//! `member-store` binding lists its stores and the parameters that escape; a
//! `static-object-return` binding names the object it returns.
//!
//! The set is total-or-refuse: an unknown field, an unknown class, a repeated
//! function, a checked binding without a definition or a release other than
//! the pin's refuses the whole set, and the run answers nothing.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// The contract set's schema.
pub const SCHEMA: &str = "quirin-openssl-contracts/1";

/// The contract-set version this checker reads. A set of another version is
/// refused: its classes may mean something this checker does not check.
pub const CONTRACT_SET_VERSION: u32 = 1;

/// The contract set the plugin carries.
pub const OPENSSL_3_5_5: &str = include_str!("../contracts/openssl-3.5.5.json");

/// A class the checker has a rule for. Every other class is answered
/// `Unknown`.
pub const CHECKED_CLASSES: [&str; 4] = [
    "no-pointer-interface",
    "static-object-return",
    "pure-reader",
    "member-store",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractSet {
    pub schema: String,
    pub contract_set_version: u32,
    pub library: String,
    pub release: String,
    /// The compile flags of each library, as the pinned build's `make -n`
    /// prints them without the compiler, the dependency-file flags, `-c`,
    /// `-o OBJECT` and the source.
    pub flag_sets: BTreeMap<String, Vec<String>>,
    pub classes: BTreeMap<String, Class>,
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    pub checked: bool,
    pub meaning: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// The defining file, relative to the release root.
    pub file: String,
    /// The object the pinned build compiles it to.
    pub object: String,
    /// The key of [`ContractSet::flag_sets`].
    pub flags: String,
}

/// One store of a `member-store` body: `param[base]->member operator e`,
/// where `e` reads exactly `parameters`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub base: usize,
    pub member: String,
    pub operator: String,
    pub parameters: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub function: String,
    pub header: String,
    /// The declared type as Clang spells it (`void (void)`).
    pub prototype: String,
    pub class: String,
    /// The contract's statement.
    pub contract: String,
    pub definition: Option<Definition>,
    /// `static-object-return`: the file-scope object returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns: Option<String>,
    /// `member-store`: the stores, in body order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stores: Option<Vec<Store>>,
    /// `member-store`: the stored pointer parameters (they escape into the
    /// base object).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escapes: Option<Vec<usize>>,
    /// Caller-callee pairs of the librdkafka census naming the function.
    pub census_pairs: u32,
}

impl ContractSet {
    /// Parse and validate a contract set for `release`.
    ///
    /// # Errors
    ///
    /// Malformed JSON or any refusal of [`Self::validate`].
    pub fn parse(text: &str, release: &str) -> Result<Self, String> {
        let set: Self =
            serde_json::from_str(text).map_err(|error| format!("contract set: {error}"))?;
        set.validate(release)?;
        Ok(set)
    }

    /// The structural rules of the set.
    ///
    /// # Errors
    ///
    /// The first violation.
    pub fn validate(&self, release: &str) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!(
                "contract set: schema `{}` is not `{SCHEMA}`",
                self.schema
            ));
        }
        if self.contract_set_version != CONTRACT_SET_VERSION {
            return Err(format!(
                "contract set: version {} is not the checker's {CONTRACT_SET_VERSION} \
                 (contract-set version mismatch)",
                self.contract_set_version
            ));
        }
        if self.library != "openssl" || self.release != release {
            return Err(format!(
                "contract set: `{} {}` is not the pinned `openssl {release}` (release pin \
                 mismatch)",
                self.library, self.release
            ));
        }
        for class in CHECKED_CLASSES {
            if !self.classes.get(class).is_some_and(|class| class.checked) {
                return Err(format!(
                    "contract set: checked class `{class}` is missing or not marked checked"
                ));
            }
        }
        for (name, class) in &self.classes {
            if class.checked && !CHECKED_CLASSES.contains(&name.as_str()) {
                return Err(format!(
                    "contract set: class `{name}` is marked checked, and the checker has no \
                     rule for it"
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for binding in &self.bindings {
            let name = &binding.function;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return Err(format!("contract set: `{name}` is not a C identifier"));
            }
            if !seen.insert(name.as_str()) {
                return Err(format!("contract set: `{name}` is bound twice"));
            }
            let Some(class) = self.classes.get(&binding.class) else {
                return Err(format!(
                    "contract set: `{name}` has the unknown class `{}`",
                    binding.class
                ));
            };
            if !binding.header.starts_with("openssl/") || binding.header.contains("..") {
                return Err(format!(
                    "contract set: `{name}`'s header `{}` is not a public header",
                    binding.header
                ));
            }
            let member_store = binding.class == "member-store";
            let static_return = binding.class == "static-object-return";
            if member_store != binding.stores.is_some()
                || member_store != binding.escapes.is_some()
                || static_return != binding.returns.is_some()
            {
                return Err(format!(
                    "contract set: `{name}`'s class-specific fields do not match its class \
                     `{}`",
                    binding.class
                ));
            }
            match (&binding.definition, class.checked) {
                (None, true) => {
                    return Err(format!(
                        "contract set: `{name}` is in a checked class without a definition"
                    ));
                }
                (Some(definition), _) => {
                    if !self.flag_sets.contains_key(&definition.flags) {
                        return Err(format!(
                            "contract set: `{name}` names the unknown flag set `{}`",
                            definition.flags
                        ));
                    }
                    for path in [&definition.file, &definition.object] {
                        if path.is_empty() || path.starts_with('/') || path.contains("..") {
                            return Err(format!(
                                "contract set: `{name}`'s path `{path}` is not relative to the \
                                 release"
                            ));
                        }
                    }
                }
                (None, false) => {}
            }
        }
        Ok(())
    }

    /// The headers the bindings name, sorted and distinct.
    #[must_use]
    pub fn headers(&self) -> Vec<&str> {
        self.bindings
            .iter()
            .map(|binding| binding.header.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Whether the checker has a rule for `binding`'s class.
    #[must_use]
    pub fn checked(&self, binding: &Binding) -> bool {
        self.classes
            .get(&binding.class)
            .is_some_and(|class| class.checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_carried_set_validates_and_binds_the_census() {
        let set = ContractSet::parse(OPENSSL_3_5_5, "3.5.5").unwrap();
        assert_eq!(set.bindings.len(), 68);
        assert_eq!(
            set.bindings
                .iter()
                .map(|binding| binding.census_pairs)
                .sum::<u32>(),
            99
        );
        // ERR_clear_error first: the registered follow-up's first callee.
        assert_eq!(set.bindings[0].function, "ERR_clear_error");
        assert_eq!(set.bindings[0].class, "no-pointer-interface");
        let checked = set
            .bindings
            .iter()
            .filter(|binding| set.checked(binding))
            .count();
        assert_eq!(checked, 18);
    }

    #[test]
    fn another_release_or_version_is_refused() {
        assert!(
            ContractSet::parse(OPENSSL_3_5_5, "3.5.6")
                .unwrap_err()
                .contains("release pin mismatch")
        );
        let text =
            OPENSSL_3_5_5.replace("\"contract_set_version\": 1", "\"contract_set_version\": 2");
        assert!(
            ContractSet::parse(&text, "3.5.5")
                .unwrap_err()
                .contains("contract-set version mismatch")
        );
    }

    #[test]
    fn an_unchecked_class_marked_checked_is_refused() {
        let text = OPENSSL_3_5_5.replacen("\"checked\": false", "\"checked\": true", 1);
        assert!(
            ContractSet::parse(&text, "3.5.5")
                .unwrap_err()
                .contains("has no rule")
        );
    }

    #[test]
    fn a_repeated_binding_or_unknown_field_is_refused() {
        let mut set: ContractSet = serde_json::from_str(OPENSSL_3_5_5).unwrap();
        let first = set.bindings[0].clone();
        set.bindings.push(first);
        assert!(set.validate("3.5.5").unwrap_err().contains("bound twice"));
        let text = OPENSSL_3_5_5.replacen("\"census_pairs\"", "\"extra\": 1, \"census_pairs\"", 1);
        assert!(ContractSet::parse(&text, "3.5.5").is_err());
    }
}
