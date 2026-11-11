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

//! Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rule 12u): the integer
//! projection of an enumeration type, read from Clang.
//!
//! # The decision
//!
//! USER DECISION 2026-10-05 (recorded verbatim in contract specification
//! §C04.1): a parameter or result whose C type is an enumeration — tagged or
//! untagged, reached through any typedef chain — is spelled in a declaration
//! as its **underlying integer type** (`i32`, `u32`, …). The reader takes
//! that type from Clang's AST for the target, never assumes `int`, and
//! requires an exact width and signedness match; when Clang does not state
//! the type, it refuses. No type grammar is added and the declaration does
//! not record the enumeration's name.
//!
//! # Why Clang, and how it is asked
//!
//! An enumeration's compatible integer type is implementation-defined (C11
//! 6.7.2.2p4): Clang gives an enumeration without a negative value
//! `unsigned int`, one with a negative value `int`, a wider type when a value
//! does not fit, the fixed underlying type when one is written (`enum : T`,
//! C23), and the smallest type under `-fshort-enums`. The lexical reader
//! cannot see any of that, so it asks Clang about the analysed unit itself
//! (its source and frontend request, `typed_access_path::appended_declarations`).
//! Clang's JSON states a fixed underlying type (`fixedUnderlyingType`) but not
//! a computed one, so each enumeration type `T` the reader meets at a
//! contracted declaration is probed by one appended declaration whose
//! enumerators Clang evaluates in that unit:
//!
//! ```c
//! enum __qcore_abi_enum_probe_N {
//!   __qcore_abi_enum_probe_N_w = sizeof(T) * __CHAR_BIT__,   /* width   */
//!   __qcore_abi_enum_probe_N_s = ((T)-1 < 0),                /* signed  */
//!   __qcore_abi_enum_probe_N_k0 = __builtin_types_compatible_p(T, signed char),
//!   …                                                        /* one per standard integer type */
//! };
//! ```
//!
//! `__builtin_types_compatible_p(T, X)` is 1 exactly when `X` is the
//! enumeration's integer type (an enumerated type is compatible with its
//! integer type and no other, C11 6.7.2.2p4; Clang's `mergeTypes`), fixed or
//! computed. The projection is admitted only when **exactly one** standard
//! integer type is compatible, its signedness is the probed signedness, and
//! the probed width is 8, 16, 32 or 64; it is then the one spelling of that
//! width and signedness (`i32`, `u32`, …), never the pointer-width `isize` /
//! `usize` readings a `long` also admits. Everything else refuses: a launch
//! that fails (an incomplete enumeration, a block-scope typedef the appended
//! file-scope probe cannot see), a source that already spells the probe
//! prefix, no or two compatible types, a signedness disagreement, another
//! width, and any reading outside an analysed source.
//!
//! # Scope of a reading
//!
//! The reader consults [`project`] only while [`AbiAgreement::read`] holds an
//! oracle ([`collecting`] then [`answering`]); every other caller of the
//! lexical reader (`declares_object_pointer_result`, `declared_type_id`) sees
//! no oracle and keeps refusing an enumeration, exactly as before this card.
//! One probe launch per unit answers every enumeration type its contracted
//! declarations name, and Clang's launch cache answers a repeated scope.
//!
//! The analysed unit is read under the gateway's frontend request, which
//! transports no `--target` (CG30): a per-unit `-arch` or `-fshort-enums`
//! that the request does not carry is not seen (`ABI-ENUM-UNIT-FRONTEND-FLAGS`,
//! the boundary CG20 records for plain `long`).
//!
//! `QCore.AbiEnumConstPointer` is the Lean model.
//!
//! [`AbiAgreement::read`]: super::declared_abi

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The prefix of every probe declaration's name.
pub(crate) const PROBE_PREFIX: &str = "__qcore_abi_enum_probe_";

/// The standard integer types an enumeration's integer type is compared
/// with, each with its signedness. Plain `char` is not among them: its
/// signedness is the target's, and Clang never chooses it for an
/// enumeration, so an enumeration compatible with it refuses.
pub(crate) const CANDIDATES: [(&str, bool); 10] = [
    ("signed char", true),
    ("unsigned char", false),
    ("short", true),
    ("unsigned short", false),
    ("int", true),
    ("unsigned int", false),
    ("long", true),
    ("unsigned long", false),
    ("long long", true),
    ("unsigned long long", false),
];

/// What Clang stated about one enumeration type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClangEnumFacts {
    /// `sizeof(T) * __CHAR_BIT__`.
    pub(crate) width: i128,
    /// `((T)-1 < 0)`.
    pub(crate) signed: bool,
    /// Which [`CANDIDATES`] are compatible with `T`, in their order.
    pub(crate) compatible: [bool; 10],
}

/// The integer projection Clang's facts establish, or `None` (refuse).
#[must_use]
pub(crate) fn projection_of(facts: &ClangEnumFacts) -> Option<&'static str> {
    let mut chosen = facts
        .compatible
        .iter()
        .zip(CANDIDATES)
        .filter(|(compatible, _)| **compatible)
        .map(|(_, candidate)| candidate);
    let (_, candidate_signed) = chosen.next()?;
    if chosen.next().is_some() || candidate_signed != facts.signed {
        return None;
    }
    Some(match (facts.width, facts.signed) {
        (8, true) => "i8",
        (8, false) => "u8",
        (16, true) => "i16",
        (16, false) => "u16",
        (32, true) => "i32",
        (32, false) => "u32",
        (64, true) => "i64",
        (64, false) => "u64",
        _ => return None,
    })
}

enum Oracle {
    /// The first reading: record every enumeration spelling met, answer none.
    Collect(BTreeSet<String>),
    /// The second reading: Clang's answer per spelling.
    Answer(BTreeMap<String, Option<&'static str>>),
}

thread_local! {
    static ORACLE: RefCell<Option<Oracle>> = const { RefCell::new(None) };
}

/// Restores the enclosing oracle (normally none) even on unwinding.
struct Restore(Option<Oracle>);

impl Drop for Restore {
    fn drop(&mut self) {
        ORACLE.with(|cell| {
            cell.replace(self.0.take());
        });
    }
}

/// The lexical reader's question: the integer projection of the enumeration
/// type spelled `spelling` (`enum TAG`, or a typedef name), `None` to refuse.
pub(crate) fn project(spelling: &str) -> Option<&'static str> {
    ORACLE.with(|cell| match cell.borrow_mut().as_mut()? {
        Oracle::Collect(seen) => {
            seen.insert(spelling.to_owned());
            None
        }
        Oracle::Answer(answers) => answers.get(spelling).copied().flatten(),
    })
}

/// Run `read` with an oracle that records every enumeration spelling the
/// reader meets and answers none of them.
pub(crate) fn collecting<T>(read: impl FnOnce() -> T) -> (T, BTreeSet<String>) {
    let previous = ORACLE.with(|cell| cell.replace(Some(Oracle::Collect(BTreeSet::new()))));
    let restore = Restore(previous);
    let value = read();
    let seen = ORACLE.with(|cell| match cell.borrow_mut().take() {
        Some(Oracle::Collect(seen)) => seen,
        _ => BTreeSet::new(),
    });
    drop(restore);
    (value, seen)
}

/// Run `read` with Clang's answers for the spellings [`collecting`] found.
pub(crate) fn answering<T>(
    answers: BTreeMap<String, Option<&'static str>>,
    read: impl FnOnce() -> T,
) -> T {
    let previous = ORACLE.with(|cell| cell.replace(Some(Oracle::Answer(answers))));
    let _restore = Restore(previous);
    read()
}

/// The appended probe for `spellings`, in their order (`N` is the index).
#[must_use]
pub(crate) fn probe_text(spellings: &[&str]) -> String {
    let mut text = String::new();
    for (index, spelling) in spellings.iter().enumerate() {
        let name = format!("{PROBE_PREFIX}{index}");
        text.push_str(&format!(
            "enum {name} {{ {name}_w = sizeof({spelling}) * __CHAR_BIT__, \
             {name}_s = (({spelling})-1 < 0)"
        ));
        for (position, (candidate, _)) in CANDIDATES.iter().enumerate() {
            text.push_str(&format!(
                ", {name}_k{position} = __builtin_types_compatible_p({spelling}, {candidate})"
            ));
        }
        text.push_str(" };\n");
    }
    text
}

/// Read Clang's facts for probe `index` out of the filtered declarations.
/// Exactly one `EnumDecl` of the probe's name, and exactly one evaluated
/// value per expected enumerator; anything else is `None`.
#[must_use]
pub(crate) fn facts_of(declarations: &[Value], index: usize) -> Option<ClangEnumFacts> {
    let name = format!("{PROBE_PREFIX}{index}");
    let mut matching = declarations
        .iter()
        .filter(|node| kind(node) == "EnumDecl" && node["name"].as_str() == Some(name.as_str()));
    let declaration = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let mut values = BTreeMap::new();
    for constant in children(declaration) {
        if kind(constant) != "EnumConstantDecl" {
            return None;
        }
        let constant_name = constant["name"].as_str()?;
        let value = evaluated(constant)?;
        if values.insert(constant_name.to_owned(), value).is_some() {
            return None;
        }
    }
    let read = |suffix: &str| values.get(&format!("{name}_{suffix}")).copied();
    let flag = |suffix: &str| match read(suffix)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    let mut compatible = [false; 10];
    for (position, slot) in compatible.iter_mut().enumerate() {
        *slot = flag(&format!("k{position}"))?;
    }
    if values.len() != 2 + CANDIDATES.len() {
        return None;
    }
    Some(ClangEnumFacts {
        width: read("w")?,
        signed: flag("s")?,
        compatible,
    })
}

/// The one `ConstantExpr` value under an enumerator's initializer.
fn evaluated(constant: &Value) -> Option<i128> {
    let mut found = Vec::new();
    collect_constant_values(constant, &mut found);
    match found.as_slice() {
        [value] => value.parse().ok(),
        _ => None,
    }
}

fn collect_constant_values<'a>(node: &'a Value, found: &mut Vec<&'a str>) {
    if kind(node) == "ConstantExpr" {
        if let Some(value) = node["value"].as_str() {
            found.push(value);
        }
        return;
    }
    for child in children(node) {
        collect_constant_values(child, found);
    }
}

/// Ask Clang, over the analysed unit, for every spelling's projection. A
/// failed launch, or no analysed source, answers `None` for every spelling.
#[must_use]
pub(crate) fn query(spellings: &BTreeSet<String>) -> BTreeMap<String, Option<&'static str>> {
    let ordered = spellings.iter().map(String::as_str).collect::<Vec<_>>();
    let declarations =
        crate::typed_access_path::appended_declarations(&probe_text(&ordered), PROBE_PREFIX);
    ordered
        .iter()
        .enumerate()
        .map(|(index, spelling)| {
            let projection = declarations
                .as_deref()
                .and_then(|declarations| facts_of(declarations, index))
                .as_ref()
                .and_then(projection_of);
            ((*spelling).to_owned(), projection)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(width: i128, signed: bool, compatible_with: &str) -> ClangEnumFacts {
        let mut compatible = [false; 10];
        for (slot, (candidate, _)) in compatible.iter_mut().zip(CANDIDATES) {
            *slot = candidate == compatible_with;
        }
        ClangEnumFacts {
            width,
            signed,
            compatible,
        }
    }

    #[test]
    fn exactly_one_compatible_type_of_the_probed_signedness_projects() {
        assert_eq!(
            projection_of(&facts(32, false, "unsigned int")),
            Some("u32")
        );
        assert_eq!(projection_of(&facts(32, true, "int")), Some("i32"));
        assert_eq!(
            projection_of(&facts(64, false, "unsigned long")),
            Some("u64")
        );
        assert_eq!(projection_of(&facts(8, false, "unsigned char")), Some("u8"));
        assert_eq!(projection_of(&facts(16, true, "short")), Some("i16"));
    }

    #[test]
    fn anything_clang_does_not_state_exactly_refuses() {
        // No compatible standard type (plain `char`, or nothing).
        assert_eq!(projection_of(&facts(8, true, "char")), None);
        // A signedness disagreement.
        assert_eq!(projection_of(&facts(32, true, "unsigned int")), None);
        // A width outside the vocabulary.
        assert_eq!(projection_of(&facts(128, true, "int")), None);
        // Two compatible types.
        let mut two = facts(32, true, "int");
        two.compatible[5] = true;
        assert_eq!(projection_of(&two), None);
    }

    #[test]
    fn no_oracle_refuses_and_the_collection_answers_none() {
        assert_eq!(project("enum e"), None);
        let ((), seen) = collecting(|| {
            assert_eq!(project("enum e"), None);
            assert_eq!(project("e_t"), None);
        });
        assert_eq!(
            seen,
            BTreeSet::from(["e_t".to_owned(), "enum e".to_owned()])
        );
        assert_eq!(project("enum e"), None);
        let answers = BTreeMap::from([("e_t".to_owned(), Some("u32"))]);
        answering(answers, || {
            assert_eq!(project("e_t"), Some("u32"));
            assert_eq!(project("enum e"), None);
        });
        assert_eq!(project("e_t"), None);
    }
}
