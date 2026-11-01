//! Card MK-EVENT-FIRST-MEMBER-CELL (rule 38 (1)): which object a member
//! cell's actual names.
//!
//! A member cell `#P: TAG.FIELD` is read through a pointer to `struct TAG`.
//! The actual is read directly when Clang's type of it, before the conversion
//! to `void *`, is `struct TAG *` in every call of the callee in the calling
//! function. C11 6.7.2.1p15 adds one case: "A pointer to a structure object,
//! suitably converted, points to its initial member" (and there is no
//! padding at its beginning). So when that type is `struct R *` for one
//! record `R`, and the unit's one complete definition of `R` declares its
//! first member `m` with type `struct TAG`, the actual `X` names the same
//! object as `&X->m`, the direct reading's key. That is the only extra case:
//!
//! * a first member of any other type (`struct TAG *`, `struct TAG[2]`, a
//!   qualified `const struct TAG`, an integer, another record) refuses, so a
//!   nested first-member chain refuses at its first level;
//! * a later member is never read, and a record with no member refuses;
//! * a union, a pointer to pointer, `void *`, an incomplete record or one
//!   with more than one complete definition refuses;
//! * no one type in every call (a mix, or an unknown type) refuses.
//!
//! The host supplies Clang's facts; this module decides. The Lean model is
//! `QCore.DescriptorCells` (`the_initial_member_is_the_event_cell`), bound by
//! [`INITIAL_MEMBER_VECTORS`].

/// What the unit holds for a record tag, as the host read it from Clang.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// Exactly one complete `struct` definition, and its first member: its
    /// name and its declared type as Clang spells it (desugared), or `None`
    /// when the record has no member.
    Struct(Option<(String, String)>),
    /// No complete definition, more than one, or not a `struct`.
    Undecided,
}

/// How the actual names the cell's object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    /// The actual is a `struct TAG *`: read as spelled.
    Direct,
    /// The actual is a `struct R *` whose first member (named here) is the
    /// `struct TAG`: read as `&X->member`.
    InitialMember(String),
    /// Anything else, with why: the call refuses.
    Refused(&'static str),
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The classification arm `descriptor-cell-initial-member` (registry rule
/// `38`): the view of a member cell's actual. `actual` is Clang's type of
/// the actual in every call of the callee in the calling function (`None`
/// when the calls disagree or a type is unknown); `tag` is the cell's
/// `TAG`; `record(R)` is what the unit holds for `struct R`.
#[must_use]
pub fn member_view(actual: Option<&str>, tag: &str, record: &dyn Fn(&str) -> Record) -> View {
    let Some(actual) = actual else {
        return View::Refused("not one Clang type in every call here");
    };
    if actual == format!("struct {tag} *") {
        return View::Direct;
    }
    let Some(name) = actual
        .strip_prefix("struct ")
        .and_then(|rest| rest.strip_suffix(" *"))
        .filter(|name| is_identifier(name))
    else {
        return View::Refused("not a pointer to a struct");
    };
    match record(name) {
        Record::Struct(Some((member, declared)))
            if !member.is_empty()
                && is_identifier(&member)
                && declared == format!("struct {tag}") =>
        {
            View::InitialMember(member)
        }
        Record::Struct(Some(_)) => {
            View::Refused("its first member is not a `struct` of the cell's tag")
        }
        Record::Struct(None) => View::Refused("the record has no member"),
        Record::Undecided => View::Refused("the record has no one complete definition here"),
    }
}

/// The shared vectors (`QCore.DescriptorCells.initial_member_vectors_hold`):
/// Clang's type of the actual (`?` when the calls disagree), the record's
/// first member (`name: type`, `none` for no member, `incomplete`, or `-`
/// where no record is consulted), and the view (`direct`, `initial NAME`,
/// `refused`), for the cell tag `mk_event`.
pub const INITIAL_MEMBER_VECTORS: &[(&str, &str, &str)] = &[
    ("struct mk_event *", "-", "direct"),
    ("struct ins *", "event: struct mk_event", "initial event"),
    ("struct ins *", "fd: int", "refused"),
    ("struct ins *", "event: struct mk_event *", "refused"),
    ("struct ins *", "events: struct mk_event[2]", "refused"),
    ("struct ins *", "event: const struct mk_event", "refused"),
    ("struct outer *", "ins: struct ins", "refused"),
    ("struct ins *", "none", "refused"),
    ("struct ins *", "incomplete", "refused"),
    ("union ins *", "-", "refused"),
    ("struct ins **", "-", "refused"),
    ("void *", "-", "refused"),
    ("?", "-", "refused"),
];

/// Run one vector; the outcome word.
#[must_use]
pub fn run_vector(actual: &str, first: &str) -> String {
    let record = |_: &str| -> Record {
        match first {
            "incomplete" | "-" => Record::Undecided,
            "none" => Record::Struct(None),
            spelled => match spelled.split_once(": ") {
                Some((name, declared)) => {
                    Record::Struct(Some((name.to_owned(), declared.to_owned())))
                }
                None => Record::Undecided,
            },
        }
    };
    let actual = (actual != "?").then_some(actual);
    match member_view(actual, "mk_event", &record) {
        View::Direct => "direct".to_owned(),
        View::InitialMember(name) => format!("initial {name}"),
        View::Refused(_) => "refused".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_vectors_hold() {
        for (actual, first, expected) in INITIAL_MEMBER_VECTORS {
            assert_eq!(run_vector(actual, first), *expected, "{actual} / {first}");
        }
    }

    #[test]
    fn the_direct_reading_never_consults_a_record() {
        let record = |_: &str| -> Record { panic!("not consulted") };
        assert_eq!(
            member_view(Some("struct mk_event *"), "mk_event", &record),
            View::Direct
        );
    }

    #[test]
    fn an_unnamed_first_member_refuses() {
        let record = |_: &str| Record::Struct(Some((String::new(), "struct mk_event".to_owned())));
        assert!(matches!(
            member_view(Some("struct ins *"), "mk_event", &record),
            View::Refused(_)
        ));
    }
}
