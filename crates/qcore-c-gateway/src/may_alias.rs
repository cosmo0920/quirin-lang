//! C02-P-ALIAS — the bounded may-alias refinement over declared names.
//!
//! ## The boundary this closes
//!
//! Card C02-P recorded, and deliberately did not repair, a property of the
//! whole resource projection rather than of its own admission
//! (`docs/reports/remaining-c02-p/README.md` §6): **two distinct declared names
//! may denote the same object, and a name-based projection relates them not at
//! all.** The measured shape was
//!
//! ```c
//! void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }
//! ```
//!
//! which answered `Safe`. Nothing in the body establishes `a != b`, so the
//! borrow of `b` may read an object `b_release` has already destroyed. The
//! projection said nothing about `b` because it had bound nothing to `b`, and
//! *silence about a name is what made the answer `Safe`*.
//!
//! ## The rule, v1
//!
//! A **use** of a name `Y` at a position the projection models as reading its
//! actual is *unresolved* — never `Safe` — when this body released some other
//! name `X` on the same path and every one of the following holds:
//!
//! 1. `X` and `Y` are **distinct declared names** of the same function;
//! 2. both are declared **exactly `struct TAG *`** for the same `TAG`, read
//!    from the same preprocessed token stream every other typed rule reads
//!    (`FunctionTypes::record_pointer_names`);
//! 3. **at least one of them is a formal parameter** — the v1 blast-radius
//!    bound (`C02-P-ALIAS-LOCALS`, §"Open" of the card report). A pair of two
//!    locals keeps the pre-existing answer;
//! 4. the release of `X` **took effect on this path** and `X` has not been
//!    rebound since;
//! 5. neither **discharge** applies (below).
//!
//! The rule only ever *adds* an `UnknownAlias` diagnostic. It never removes
//! one, never suppresses a leak and never converts an `Unsafe` into anything
//! else, so the movement it can cause is exactly `Safe` → `Unknown`.
//!
//! ## The two discharges
//!
//! * **Provenance.** `Y` is bound, at the use, to exactly one live identity
//!   *this body acquired* — not a caller-held entry site. A body-made
//!   acquisition returns an object distinct from every object live when it ran,
//!   and the projection's own identity for `Y` is then what answers the
//!   question. Decided in [`crate::legacy_c`] against the abstract state, not
//!   here.
//! * **Pointer inequality.** A branch on `X != Y` (or `X == Y`) reaching the
//!   use on the distinct edge. Decided flow-sensitively in [`crate::legacy_c`]
//!   through `LegacyInstruction::BranchOnDistinct`; a rebinding of either name
//!   drops the fact.
//!
//! Which releases have run and which guards hold are computed by
//! `legacy_c::alias_facts`, a **separate** forward fixed point over the CFG that
//! never touches the custody state. That separation is measured rather than
//! stylistic — see `C02-P-ALIAS-LEAK-ORDER` in the card report.
//!
//! ## What is deliberately outside v1
//!
//! * a pair of two **locals** (`C02-P-ALIAS-LOCALS`);
//! * a function whose declarations the reader **refused**: no type sharing is
//!   established there, so the precondition of the rule never holds and the
//!   pre-existing answer stands (`C02-P-ALIAS-UNTYPED`);
//! * names whose declared type is not exactly `struct TAG *` — `void *`,
//!   multi-star, an unresolved typedef (`C02-P-ALIAS-UNTYPED`);
//! * **consuming** an argument, as opposed to releasing it
//!   (`C02-P-ALIAS-CONSUME`);
//! * uses that are not modeled reads of the pointer — a scalar-field no-op, a
//!   branch condition, an `Out` slot address (`C02-P-ALIAS-USES`).
//!
//! Each is a *retained* pre-existing answer, never a new admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::typed_field_access::FunctionTypes;

/// The declared record-pointer classes of one function body.
#[derive(Clone, Debug, Default)]
pub(crate) struct MayAliasClasses {
    /// Name -> declared record tag, for names declared exactly `struct TAG *`.
    tags: BTreeMap<String, String>,
    /// Which of those names are formal parameters of this function.
    formals: BTreeSet<String>,
}

impl MayAliasClasses {
    /// Read the classes from the same declaration facts every other typed rule
    /// reads. A function whose declarations were refused yields no class at
    /// all, so it relates no pair of names.
    pub(crate) fn read(types: &FunctionTypes, formal_parameters: &BTreeSet<String>) -> Self {
        let tags = types
            .record_pointer_names()
            .into_iter()
            .map(|(name, tag)| (name.to_owned(), tag.to_owned()))
            .collect::<BTreeMap<_, _>>();
        let formals = tags
            .keys()
            .filter(|name| formal_parameters.contains(*name))
            .cloned()
            .collect();
        Self { tags, formals }
    }

    /// The names whose release leaves a use of `used` unresolved, in a stable
    /// order. Empty whenever the rule's precondition does not hold, which is
    /// the pre-existing answer.
    pub(crate) fn peers(&self, used: &str) -> Vec<String> {
        let Some(tag) = self.tags.get(used) else {
            return Vec::new();
        };
        let used_is_formal = self.formals.contains(used);
        self.tags
            .iter()
            .filter(|(name, other)| {
                name.as_str() != used
                    && *other == tag
                    && (used_is_formal || self.formals.contains(*name))
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// The root name a canonical place dereferences: the identifier before the
/// first `->`, `.` or `[`.
///
/// `"storage->cio"` yields `"storage"`, `"b"` yields `"b"`. An indirect place
/// (`"*out"`) yields `None`: it names a slot, not a pointer this rule relates.
pub(crate) fn place_root(place: &str) -> Option<&str> {
    if place.starts_with('*') {
        return None;
    }
    let end = place
        .find(['.', '['])
        .into_iter()
        .chain(place.find("->"))
        .min()
        .unwrap_or(place.len());
    let root = &place[..end];
    (!root.is_empty()).then_some(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_field_access::TypedAccessModel;

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphanumeric() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    const DECLARATIONS: &str = "struct rec { int state ; } ; struct other { int state ; } ;";

    /// Build the classes over one parameter list and one body.
    fn classes(parameters: &str, body: &str) -> MayAliasClasses {
        let unit: &'static str =
            Box::leak(format!("{DECLARATIONS} {parameters} {body}").into_boxed_str());
        let model = TypedAccessModel::read(&texts(unit));
        let parameter_texts: &'static [&'static str] =
            Box::leak(texts(Box::leak(parameters.to_owned().into_boxed_str())).into_boxed_slice());
        let body_texts: &'static [&'static str] =
            Box::leak(texts(Box::leak(body.to_owned().into_boxed_str())).into_boxed_slice());
        let protected = BTreeSet::new();
        let types = FunctionTypes::read(&model, parameter_texts, body_texts, &protected);
        let formals = parameter_texts
            .iter()
            .filter(|text| {
                crate::typed_field_access::is_name(text)
                    && !matches!(**text, "struct" | "int" | "char" | "rec" | "other" | "void")
            })
            .map(|text| (*text).to_owned())
            .collect::<BTreeSet<_>>();
        MayAliasClasses::read(&types, &formals)
    }

    #[test]
    fn two_formals_of_the_same_record_pointer_type_are_peers() {
        let classes = classes("struct rec * a , struct rec * b", "");
        assert_eq!(classes.peers("a"), vec!["b".to_owned()]);
        assert_eq!(classes.peers("b"), vec!["a".to_owned()]);
    }

    #[test]
    fn different_record_tags_are_never_peers() {
        let classes = classes("struct rec * a , struct other * b", "");
        assert!(classes.peers("a").is_empty());
        assert!(classes.peers("b").is_empty());
    }

    #[test]
    fn a_formal_and_a_local_of_the_same_type_are_peers_in_both_directions() {
        let classes = classes("struct rec * a", "struct rec * held ;");
        assert_eq!(classes.peers("a"), vec!["held".to_owned()]);
        assert_eq!(classes.peers("held"), vec!["a".to_owned()]);
    }

    /// The v1 blast-radius bound: two locals are related by nothing here.
    #[test]
    fn two_locals_are_not_peers_in_v1() {
        let classes = classes("int n", "struct rec * one ; struct rec * two ;");
        assert!(classes.peers("one").is_empty());
        assert!(classes.peers("two").is_empty());
    }

    /// A three-name class relates every ordered pair through the formal.
    #[test]
    fn a_three_name_class_relates_every_pair_through_its_formal() {
        let classes = classes("struct rec * a , struct rec * b", "struct rec * held ;");
        assert_eq!(classes.peers("a"), vec!["b".to_owned(), "held".to_owned()]);
        assert_eq!(classes.peers("held"), vec!["a".to_owned(), "b".to_owned()]);
    }

    /// A name is never its own peer, so a release through it is judged by the
    /// existing double-release model rather than by this rule.
    #[test]
    fn a_name_is_never_its_own_peer() {
        let classes = classes("struct rec * a , struct rec * b", "");
        assert!(!classes.peers("a").contains(&"a".to_owned()));
    }

    /// A refused declaration establishes no sharing, so the pre-existing answer
    /// stands (`C02-P-ALIAS-UNTYPED`).
    #[test]
    fn an_unreadable_function_relates_nothing() {
        let classes = classes(
            "struct rec * a",
            "void ( * hook ) ( int ) ; struct rec * held ;",
        );
        assert!(classes.peers("a").is_empty());
        assert!(classes.peers("held").is_empty());
    }

    /// `void *`, a multi-star and an unresolved typedef are not `struct TAG *`.
    #[test]
    fn a_type_the_reader_will_not_commit_to_relates_nothing() {
        let classes = classes("struct rec * a , struct rec * * b", "");
        assert!(classes.peers("b").is_empty());
        assert!(classes.peers("a").is_empty());
    }

    #[test]
    fn a_place_root_is_the_identifier_before_the_first_selection() {
        assert_eq!(place_root("storage->cio"), Some("storage"));
        assert_eq!(place_root("value.one"), Some("value"));
        assert_eq!(place_root("table[0]"), Some("table"));
        assert_eq!(place_root("b"), Some("b"));
        assert_eq!(place_root("*out"), None);
    }

    #[test]
    fn a_root_stops_at_whichever_selector_comes_first() {
        assert_eq!(place_root("a.b->c"), Some("a"));
        assert_eq!(place_root("a->b.c"), Some("a"));
    }
}
