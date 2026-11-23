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

//! Card **AMPERSAND-RUN-PARITY-OTHER-READERS** (gateway rule 74e): the one
//! reading of a run of `&` tokens every address decision of the unfused
//! token stream uses.
//!
//! The importer's lexer emits one token per punctuation character, so `p &&
//! q` reaches a reader as `p & & q` and `p && &q` as `p & & & q`. C lexes a
//! run of adjacent `&` characters by maximal munch (C11 6.4p4): `k` of them
//! are `k / 2` `&&` operators followed, when `k` is odd, by one `&`. A
//! whitespace-separated `a & &b` would be a binary `&` with a pointer operand,
//! which C11 6.5.10p2 forbids, so no conforming body spells it. The last `&`
//! of a run before an operand is therefore an `&` operator exactly when the
//! run is odd ([`ends_in_address`]); an even run ends in a logical AND
//! ([`ends_in_logical_and`]), or GNU's `&&label`, which names no object.
//!
//! Whether a single `&` is the unary address operator or a binary AND is a
//! separate question (the token before it), which each reader keeps deciding
//! as it did: the parity only replaces the reading of `&&`. Rule 74d (C)
//! introduced the parity for rule 10a's frame; `QCore.AmpersandRunParity`
//! states it over `QCore.NullableLazyConstructor`'s maximal-munch lemma and
//! lists every reader that uses it ([`READERS`]).

/// The number of consecutive `&` tokens ending just before `at`.
pub(crate) fn run_before<S: AsRef<str>>(texts: &[S], at: usize) -> usize {
    let end = at.min(texts.len());
    texts[..end]
        .iter()
        .rev()
        .take_while(|text| text.as_ref() == "&")
        .count()
}

/// Whether the run of `&` tokens ending just before `at` ends in a single
/// `&` operator (the run is odd): `&q` and `p && &q` take an address (when
/// that `&` is unary), `p && q` does not.
pub(crate) fn ends_in_address<S: AsRef<str>>(texts: &[S], at: usize) -> bool {
    run_before(texts, at) % 2 == 1
}

/// Whether the run of `&` tokens ending just before `at` is a non-empty even
/// run, so the operand at `at` is the right operand of a logical AND.
pub(crate) fn ends_in_logical_and<S: AsRef<str>>(texts: &[S], at: usize) -> bool {
    let run = run_before(texts, at);
    run >= 2 && run.is_multiple_of(2)
}

/// The index after each maximal run of `&` tokens that ends in a single `&`
/// operator (an odd run). Rule 74d's reading for rule 10a's frame, moved here
/// unchanged from `legacy_c.rs`.
pub(crate) fn unary_address_runs<S: AsRef<str>>(body: &[S]) -> Vec<usize> {
    let mut after = Vec::new();
    let mut index = 0;
    while index < body.len() {
        if body[index].as_ref() != "&" {
            index += 1;
            continue;
        }
        let start = index;
        while body.get(index).is_some_and(|text| text.as_ref() == "&") {
            index += 1;
        }
        if (index - start) % 2 == 1 {
            after.push(index);
        }
    }
    after
}

/// The keywords whose `(` opens a controlling expression or an operand, not
/// an argument list: a group after them is read as any other group.
const NON_CALLEE_HEADS: &[&str] = &[
    "return", "case", "if", "while", "switch", "for", "else", "do",
];

fn is_name(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Card **CAST-BEFORE-AMPERSAND-READ-AS-BINARY** (rule 74f): whether the `)`
/// at `close` may end a cast `( type-name )`, so that a single `&` after it is
/// the unary address operator, not a binary AND.
///
/// The one cast detector of the unfused token stream, shared by every reader
/// that decides a single `&` by the token before it
/// (`legacy_refused_operands::handed_on`, `legacy_call_instantiation`'s
/// `residual_takes_an_address`). A token reading cannot always tell a cast
/// from a parenthesised expression (`(a) & b` is either, by whether `a` names
/// a type), so the answer leans to the cast, which reads more addresses:
///
/// * **not a cast** (binary): the group is an argument list (its `(` follows
///   a name that is no control keyword, a `)` or a `]`: `f(x) & mask`,
///   `(*fp)(x) & m`); or it holds a token no scalar cast's type name holds at
///   its top level (an operator other than `*`, a literal, `[`, `.`, `,`: `(a
///   + b) & m`, `(s.f) & m`, `(p[0]) & m`), or, nested, any token besides
///   names, `*`, brackets, numbers, `,` and `...`; or it is a single name in
///   `objects` (the function's declared objects, minus every name any
///   typedef spells: `(a) & b` with `a` a variable);
/// * **may be a cast** (unary): everything else, an unmatched `)` and an
///   undecided `(a)` included.
///
/// C11 6.5.4p2 confines a cast's type name to a scalar type or `void`, so
/// brackets, numbers and `,` appear only inside a nested declarator group
/// (`(char (*)[4])`, `(void (*)(int, ...))`).
pub(crate) fn may_close_a_cast<S: AsRef<str>>(
    texts: &[S],
    close: usize,
    objects: &std::collections::BTreeSet<String>,
) -> bool {
    if texts.get(close).map(AsRef::as_ref) != Some(")") {
        return false;
    }
    let mut depth = 0usize;
    let mut open = None;
    for at in (0..close).rev() {
        match texts[at].as_ref() {
            ")" => depth += 1,
            "(" if depth == 0 => {
                open = Some(at);
                break;
            }
            "(" => depth -= 1,
            _ => {}
        }
    }
    let Some(open) = open else {
        return true;
    };
    if let Some(head) = open.checked_sub(1).map(|at| texts[at].as_ref())
        && ((is_name(head) && !NON_CALLEE_HEADS.contains(&head)) || matches!(head, ")" | "]"))
    {
        return false;
    }
    let inner = &texts[open + 1..close];
    if let [name] = inner
        && objects.contains(name.as_ref())
    {
        return false;
    }
    let mut depth = 0usize;
    // A nested declarator group closed at the top level: `(char (*)[4])`
    // continues with its array declarator there.
    let mut declarator = false;
    for (at, text) in inner.iter().enumerate() {
        let text = text.as_ref();
        let dot = |other: Option<&S>| other.is_some_and(|other| other.as_ref() == ".");
        let ellipsis = text == "..."
            || (text == "." && (dot(inner.get(at + 1)) || at > 0 && dot(inner.get(at - 1))));
        let number = text.as_bytes().first().is_some_and(u8::is_ascii_digit);
        let allowed = match text {
            "(" => {
                depth += 1;
                true
            }
            ")" => {
                depth = depth.saturating_sub(1);
                declarator |= depth == 0;
                true
            }
            "*" => true,
            _ if is_name(text) => true,
            "[" | "]" if declarator => true,
            _ if number && declarator => true,
            _ if depth == 0 => false,
            "[" | "]" | "," => true,
            _ if number => true,
            _ => ellipsis,
        };
        if !allowed {
            return false;
        }
    }
    true
}

/// The files whose readers decide address-taking from the unfused token
/// stream through this module, with the number of call sites in each and the
/// class of the reading each replaced (`QCore.AmpersandRunParity.readers`
/// restates the list). A test counts the call sites, so a reader added or
/// removed updates this census.
#[cfg(test)]
pub(crate) const READERS: &[(&str, usize, &str)] = &[
    ("legacy_c.rs", 1, "frame"),
    ("loop_owner_instance.rs", 2, "unsound"),
    ("loop_owner_holder_opaque.rs", 2, "unsound"),
    ("thread_entry_census.rs", 3, "unsound"),
    ("legacy_retained_by.rs", 2, "admitting+fail-closed"),
    ("legacy_callback_route.rs", 2, "admitting+fail-closed"),
    ("legacy_list_iteration.rs", 2, "fail-closed"),
    ("legacy_reference_result.rs", 1, "fail-closed"),
    ("indexed_collection.rs", 1, "fail-closed"),
    ("aggregate_store.rs", 1, "fail-closed"),
    ("registration_by_id.rs", 2, "fail-closed"),
    ("typed_field_access.rs", 1, "fail-closed"),
    ("borrowed_actual.rs", 2, "fail-closed"),
    ("legacy_retention_guard.rs", 1, "fail-closed"),
    ("legacy_reference_count.rs", 1, "fail-closed"),
    ("legacy_slot_move_borrow.rs", 1, "fail-closed"),
    ("legacy_refused_operands.rs", 2, "fail-closed+cast"),
    ("legacy_in_place_custody.rs", 1, "fail-closed"),
    ("legacy_call_instantiation.rs", 2, "fail-closed+cast"),
];

/// `(tokens, index of the operand, run ends in an & operator?, run ends in
/// a logical AND?)`: the vectors `QCore.AmpersandRunParity.decisionVectors`
/// restates.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, usize, bool, bool)] = &[
    ("p & & & q", 4, true, false),
    ("p & & q", 3, false, true),
    ("p & q", 2, true, false),
    ("& & & q", 3, true, false),
    ("& ( q )", 1, true, false),
    ("a & & q", 3, false, true),
    ("= q", 1, false, false),
    ("& & label", 2, false, true),
    ("p & & & & q", 5, false, true),
];

/// `(tokens, index of the `)`, declared objects, may close a cast?)`: the
/// vectors `QCore.CastBeforeAmpersand.castVectors` restates by class.
#[cfg(test)]
pub(crate) const CAST_VECTORS: &[(&str, usize, &[&str], bool)] = &[
    ("( T * ) & x", 3, &[], true),
    ("( struct s * ) & v", 4, &[], true),
    ("( void ) & x", 2, &[], true),
    ("( my_t ) & x", 2, &["x"], true),
    ("( a ) & b", 2, &[], true),
    ("( a ) & b", 2, &["a"], false),
    ("f ( x ) & mask", 3, &[], false),
    ("( * fp ) ( x ) & m", 6, &[], false),
    ("( a + 1 ) & m", 4, &[], false),
    ("( p [ 0 ] ) & m", 5, &[], false),
    ("( s . n ) & m", 4, &[], false),
    ("( void ( * ) ( int , . . . ) ) & f", 12, &[], true),
    ("( char ( * ) [ 4 ] ) & b", 8, &[], true),
    ("return ( T * ) & x", 4, &[], true),
    ("sizeof ( T ) & x", 3, &[], false),
    ("T * ) & x", 2, &[], true),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_74f_cast_vectors_hold() {
        for (source, close, objects, cast) in CAST_VECTORS {
            let body = source.split(' ').collect::<Vec<_>>();
            assert_eq!(body[*close], ")", "{source}");
            let objects = objects.iter().map(|name| (*name).to_owned()).collect();
            assert_eq!(may_close_a_cast(&body, *close, &objects), *cast, "{source}");
        }
    }

    #[test]
    fn rule_74e_decision_vectors_hold() {
        for (source, at, address, logical) in DECISION_VECTORS {
            let body = source.split(' ').collect::<Vec<_>>();
            assert_eq!(ends_in_address(&body, *at), *address, "{source}");
            assert_eq!(ends_in_logical_and(&body, *at), *logical, "{source}");
        }
    }

    #[test]
    fn the_runs_agree_with_the_per_index_reading() {
        let body = "p & & & q ; r & & s ; & t ; a & ( b )"
            .split(' ')
            .collect::<Vec<_>>();
        let runs = unary_address_runs(&body);
        let per_index = (1..body.len())
            .filter(|&at| body[at] != "&" && body[at - 1] == "&" && ends_in_address(&body, at))
            .collect::<Vec<_>>();
        assert_eq!(runs, per_index);
        assert_eq!(runs, vec![4, 12, 16]);
    }

    #[test]
    fn every_reader_of_the_census_reads_the_parity() {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let needle = ["crate", "ampersand_runs", ""].join("::");
        let mut counted = std::collections::BTreeMap::new();
        for entry in walk(&source) {
            let text = std::fs::read_to_string(&entry).expect("source");
            let count = text.matches(&needle).count();
            if count > 0 {
                let name = entry.strip_prefix(&source).expect("prefix");
                counted.insert(name.to_string_lossy().into_owned(), count);
            }
        }
        let listed = READERS
            .iter()
            .map(|(file, count, _)| ((*file).to_owned(), *count))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(counted, listed);
    }

    fn walk(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
        out
    }

    #[test]
    fn a_run_at_the_start_and_beyond_the_end_is_read() {
        let body = ["&", "&", "&"];
        assert_eq!(run_before(&body, 3), 3);
        assert_eq!(run_before(&body, 9), 3);
        assert_eq!(run_before(&body, 0), 0);
        assert!(!ends_in_address(&body, 0));
    }
}
