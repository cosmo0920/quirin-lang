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

//! Card **AG1-INDIRECT-MEMBER-ACCESS**, gateway rule 61 (1): a local aggregate
//! pointer spelled through a struct alias is the struct pointer for the
//! aggregate-store (O00 blocker B08) premises.
//!
//! The lexical declaration reader ([`crate::typed_field_access`]) answers
//! `Opaque` for a typedef that **defines** its struct
//! (`typedef struct TAG { … } A;`) and for a name declared by two typedefs that
//! the reader cannot compare (`typedef struct TAG A;` then the defining one).
//! That answer is kept byte for byte for every shared consumer. C11 6.7.8p3
//! makes a typedef name a synonym for the type it names, so where **every**
//! typedef declaration of `A` in the unit names one struct `TAG`, a local
//! declared exactly `A * NAME` has type `struct TAG *`, and the aggregate-store
//! premises of [`crate::aggregate_store`] may read it so. This module decides
//! exactly that and nothing else:
//!
//! * the alias: every `typedef` declaration of `A` is `typedef struct TAG A;`
//!   or `typedef struct TAG { … } A;` with one `TAG`
//!   ([`crate::typed_field_access::TypedAccessModel::struct_alias_tag`]), and
//!   `struct TAG` has exactly one complete, non-union definition in the unit;
//! * the declaration: a statement of the body that is exactly `A * NAME ;` or
//!   `A * NAME = … ;` (one declarator, one `*`, no qualifier or storage class);
//! * the name: declared by exactly one declaration statement of the body, not a
//!   formal, and the shared reader read it and committed to no type (`Opaque`).
//!
//! Every other premise of the aggregate store (a single acquired binding, no
//! escape, no aliasing, the member classes of `struct TAG`, the
//! flow-sensitive owner check) is unchanged and still decides. A refused
//! answer leaves the aggregate store exactly as before.

use std::collections::{BTreeMap, BTreeSet};

use crate::typed_field_access::{DeclaredType, FunctionTypes, TypedAccessModel, is_name};

/// Record one typedef declaration's struct head for `alias`. The first
/// declaration sets it; any later one that disagrees (or names no struct)
/// makes the alias unusable.
pub(crate) fn note_struct_alias_head(
    heads: &mut BTreeMap<String, Option<String>>,
    alias: &str,
    head: Option<String>,
) {
    match heads.get(alias) {
        Some(existing) if *existing == head => {}
        Some(_) => {
            heads.insert(alias.to_owned(), None);
        }
        None => {
            heads.insert(alias.to_owned(), head);
        }
    }
}

/// The tag of a definition head exactly `struct TAG`.
pub(crate) fn struct_tag_of_head(head: &str) -> Option<&str> {
    let mut words = head.split_ascii_whitespace();
    match (words.next(), words.next(), words.next()) {
        (Some("struct"), Some(tag), None) if is_name(tag) => Some(tag),
        _ => None,
    }
}

/// The facts rule 61 (1) decides one local on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AliasLocalFacts {
    /// Every typedef declaration of the alias names one struct.
    pub(crate) alias_agrees: bool,
    /// That struct has exactly one complete, non-union definition.
    pub(crate) record_complete: bool,
    /// The declaration is exactly `A * NAME ;` or `A * NAME = … ;`.
    pub(crate) exact_spelling: bool,
    /// The name is declared by exactly one declaration statement of the body.
    pub(crate) unique: bool,
    /// The name is not a formal.
    pub(crate) local: bool,
    /// The shared reader committed to no type for the name.
    pub(crate) shared_opaque: bool,
}

/// The decision: admitted only when every fact holds.
pub(crate) const fn admits(facts: AliasLocalFacts) -> bool {
    facts.alias_agrees
        && facts.record_complete
        && facts.exact_spelling
        && facts.unique
        && facts.local
        && facts.shared_opaque
}

/// Rule 61 (1): the locals of `body` declared `A * NAME` through a struct
/// alias, mapped to the struct tag, for the aggregate-store premises only.
pub(crate) fn alias_record_locals(
    body: &[&str],
    formal_parameters: &BTreeSet<String>,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> BTreeMap<String, String> {
    if !types.is_readable() {
        return BTreeMap::new();
    }
    let unique = crate::typed_field_access::unique_local_names(model, body);
    let mut out = BTreeMap::new();
    let mut start = 0usize;
    for (index, text) in body.iter().enumerate() {
        if !matches!(*text, ";" | "{" | "}") {
            continue;
        }
        let statement = &body[start..index];
        start = index + 1;
        let (alias, name) = match statement {
            [alias, "*", name] => (*alias, *name),
            [alias, "*", name, "=", ..] => (*alias, *name),
            _ => continue,
        };
        if !is_name(alias) || !is_name(name) {
            continue;
        }
        let tag = model.struct_alias_tag(alias);
        let facts = AliasLocalFacts {
            alias_agrees: tag.is_some(),
            record_complete: tag.is_some_and(|tag| model.record_definition_is_complete(tag)),
            exact_spelling: true,
            unique: unique.contains(name),
            local: !formal_parameters.contains(name),
            shared_opaque: types.declared_type(name) == Some(&DeclaredType::Opaque),
        };
        if let Some(tag) = tag
            && admits(facts)
        {
            out.insert(name.to_owned(), tag.to_owned());
        }
    }
    out
}

/// The decision vectors shared with `QCore.TypedefAggregateBase`
/// (`decision_vectors_hold`): every combination of the six facts, admitted
/// exactly when all hold.
#[cfg(test)]
pub(crate) fn decision_vectors() -> Vec<(AliasLocalFacts, bool)> {
    let mut vectors = Vec::new();
    for bits in 0u8..64 {
        let facts = AliasLocalFacts {
            alias_agrees: bits & 1 != 0,
            record_complete: bits & 2 != 0,
            exact_spelling: bits & 4 != 0,
            unique: bits & 8 != 0,
            local: bits & 16 != 0,
            shared_opaque: bits & 32 != 0,
        };
        vectors.push((facts, bits == 63));
    }
    vectors
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixtures are written with every token separated by a space.
    fn read(unit: &str, body: &str) -> BTreeMap<String, String> {
        let unit_texts: Vec<&str> = unit.split_ascii_whitespace().collect();
        let model = TypedAccessModel::read(&unit_texts);
        let body_texts: Vec<&str> = body.split_ascii_whitespace().collect();
        let types = FunctionTypes::read(&model, &[], &body_texts, &BTreeSet::new());
        alias_record_locals(&body_texts, &BTreeSet::new(), &model, &types)
    }

    #[test]
    fn the_decision_admits_only_when_every_fact_holds() {
        for (facts, expected) in decision_vectors() {
            assert_eq!(admits(facts), expected, "{facts:?}");
        }
    }

    #[test]
    fn a_defining_alias_names_its_struct() {
        let unit = "typedef struct n_s { int id ; char * host ; } n_t ;";
        assert_eq!(
            read(unit, "n_t * r = f ( ) ; r - > id = 1 ;")
                .get("r")
                .map(String::as_str),
            Some("n_s")
        );
    }

    #[test]
    fn a_forward_and_a_defining_alias_agree() {
        let unit = "typedef struct n_s n_t ; typedef struct n_s { int id ; char * host ; } n_t ;";
        assert_eq!(
            read(unit, "n_t * r ; r = f ( ) ;")
                .get("r")
                .map(String::as_str),
            Some("n_s")
        );
    }

    #[test]
    fn disagreeing_union_and_incomplete_aliases_refuse() {
        // Two structs under one alias.
        let unit = "typedef struct a_s a_t ; typedef struct b_s { int v ; } a_t ; struct a_s { int w ; } ;";
        assert!(read(unit, "a_t * r ;").is_empty());
        // A union.
        let unit = "typedef union u_s { int v ; char * p ; } u_t ;";
        assert!(read(unit, "u_t * r ;").is_empty());
        // An incomplete struct.
        let unit = "typedef struct i_s i_t ;";
        assert!(read(unit, "i_t * r ;").is_empty());
        // A pointer typedef.
        let unit = "struct p_s { int v ; } ; typedef struct p_s * p_t ;";
        assert!(read(unit, "p_t r ;").is_empty());
    }

    #[test]
    fn only_the_exact_single_declarator_is_read() {
        let unit = "typedef struct n_s { int id ; } n_t ;";
        // Two declarators, a qualifier, two stars, an array.
        assert!(read(unit, "n_t * a , * b ;").is_empty());
        assert!(read(unit, "const n_t * a ;").is_empty());
        assert!(read(unit, "n_t * * a ;").is_empty());
        assert!(read(unit, "n_t * a [ 2 ] ;").is_empty());
        // A name declared twice.
        assert!(read(unit, "n_t * a ; { n_t * a ; }").is_empty());
    }
}
