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

//! Card **RULE33-PILOT**: two host capabilities for a statement-position
//! package with caller expansion, added for rule 33's migration (plan
//! `docs/plans/pluggable-cards/README.md` §3 B: "a migration card that needs a
//! further reading adds it here as a host capability").
//!
//! * [`StatementExpansion`] (§2.1, statement rewrite; read on each CFG build,
//!   §1): a package that owns a checked *frame* (a function whose exact body
//!   the host has checked against its contract environment and type model)
//!   rewrites the statements that use it into the frame's constituent calls.
//!   It is asked once per analysed body, with the body's own frame, if any,
//!   and the checked frames of the callees its statements name; it answers
//!   the whole rewritten body. Refusal (and a disabled package) keeps the body
//!   verbatim, so every use of the frame keeps the host's existing reading
//!   (for an uncontracted callee, its refusal).
//! * [`EntryRow`]: a frame's own row, which the host analysed under the
//!   package's own entry, stands only while the package admits the run.
//!   Refusal (and a disabled package) leaves the row to the host's ordinary,
//!   contract-free analysis, and installs nothing a caller could read.
//!
//! Neither query needs the host's vocabulary: bodies are word lists and the
//! frame and run types belong to the package, which the host builds after its
//! own checks. Both answer [`Answer::Admit`] or [`Answer::Refuse`] and compose
//! through [`crate::consult`] like every other position.

use std::collections::{BTreeMap, BTreeSet};

use crate::answer::Answer;
use crate::traits::RulePackage;

/// One body asked of a [`StatementExpansion`].
#[derive(Debug)]
pub struct BodyQuery<'q, F> {
    /// The analysed function.
    pub function: &'q str,
    /// The body's token spellings, from its `{` through the matching `}`.
    pub body: &'q [&'q str],
    /// The function's own checked frame, when the host read one.
    pub own: Option<&'q F>,
    /// The checked frames of the callees the body's statements name, by
    /// callee. A callee whose frame the host did not check is absent.
    pub callees: &'q BTreeMap<String, F>,
}

/// An expanded body: every word, with the index of the [`BodyQuery::body`]
/// word whose source line it takes. A verbatim word is anchored to itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expanded {
    pub words: Vec<(String, usize)>,
}

impl Expanded {
    /// Every anchor indexes the queried body.
    #[must_use]
    pub fn anchored_within(&self, body_len: usize) -> bool {
        self.words.iter().all(|(_, anchor)| *anchor < body_len)
    }
}

/// §2.1 (statement rewrite), read on each CFG build (§1): the caller
/// expansion of a package's checked frames.
pub trait StatementExpansion: RulePackage {
    /// A frame the host checked, in the package's own terms.
    type Frame;
    /// The reserved prefix of every place an expansion introduces: the host
    /// asks nothing when the unit's stream spells it (C11 7.1.3).
    fn reserved_prefix(&self) -> &'static str;
    /// The callees whose statements in `body` the package may expand (a
    /// cheap lexical read before the host checks any frame).
    fn candidates<'b>(&self, function: &str, body: &[&'b str]) -> BTreeSet<&'b str>;
    /// The rewritten body, or a refusal (the body stays verbatim).
    fn ask(&self, query: &BodyQuery<'_, Self::Frame>) -> Answer<Expanded>;
}

/// A frame's own row, analysed by the host under the package's entry.
pub trait EntryRow: RulePackage {
    /// What the host observed of the run, in the package's own terms.
    type Run;
    /// Admit: the run validates the frame, and its row is the entry's row.
    fn ask(&self, run: &Self::Run) -> Answer<()>;
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{BodyQuery, EntryRow, Expanded, StatementExpansion};
    use crate::answer::{Answer, consult};
    use crate::traits::RulePackage;

    /// Expands `f(K);` into `g(K); h(K);` for a checked frame `f`.
    struct Twice;
    impl RulePackage for Twice {
        fn id(&self) -> &'static str {
            "twice"
        }
    }
    impl StatementExpansion for Twice {
        type Frame = (&'static str, &'static str);
        fn reserved_prefix(&self) -> &'static str {
            "__twice_"
        }
        fn candidates<'b>(&self, _: &str, body: &[&'b str]) -> BTreeSet<&'b str> {
            body.windows(2)
                .filter(|pair| pair[1] == "(")
                .map(|pair| pair[0])
                .collect()
        }
        fn ask(&self, query: &BodyQuery<'_, Self::Frame>) -> Answer<Expanded> {
            let mut words = Vec::new();
            let mut changed = false;
            let mut at = 0;
            while at < query.body.len() {
                if let Some((first, second)) = query.callees.get(query.body[at])
                    && query.body.get(at + 1) == Some(&"(")
                    && query.body.get(at + 3) == Some(&")")
                {
                    for callee in [first, second] {
                        for word in [callee, "(", query.body[at + 2], ")", ";"] {
                            words.push((word.to_string(), at));
                        }
                    }
                    at += 5;
                    changed = true;
                } else {
                    words.push((query.body[at].to_owned(), at));
                    at += 1;
                }
            }
            if changed {
                Answer::admit(Expanded { words })
            } else {
                Answer::Refuse
            }
        }
    }
    impl EntryRow for Twice {
        type Run = bool;
        fn ask(&self, run: &bool) -> Answer<()> {
            Answer::from_option(run.then_some(()))
        }
    }

    #[test]
    fn a_disabled_expansion_keeps_the_body_and_its_frames_are_not_rows() {
        let body = ["{", "f", "(", "3", ")", ";", "}"];
        let callees = BTreeMap::from([("f".to_owned(), ("g", "h"))]);
        let query = BodyQuery {
            function: "caller",
            body: &body,
            own: None,
            callees: &callees,
        };
        let packages = [Twice];
        assert_eq!(
            Twice.candidates("caller", &body),
            BTreeSet::from(["f"]),
            "the cheap read names the callee"
        );
        let on = consult(
            &packages,
            |_| true,
            |_, p| StatementExpansion::ask(p, &query),
        )
        .expect("expands")
        .admission
        .lowering;
        let spelled = on.words.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>();
        assert_eq!(
            spelled,
            ["{", "g", "(", "3", ")", ";", "h", "(", "3", ")", ";", "}"]
        );
        assert!(on.anchored_within(body.len()));
        assert!(
            consult(
                &packages,
                |_| false,
                |_, p| StatementExpansion::ask(p, &query)
            )
            .is_none(),
            "off: the body stays verbatim"
        );
        let unchecked = BTreeMap::new();
        let query = BodyQuery {
            callees: &unchecked,
            ..query
        };
        assert!(
            consult(
                &packages,
                |_| true,
                |_, p| StatementExpansion::ask(p, &query)
            )
            .is_none(),
            "an unchecked callee is never expanded"
        );
        assert!(consult(&packages, |_| true, |_, p| EntryRow::ask(p, &true)).is_some());
        assert!(consult(&packages, |_| false, |_, p| EntryRow::ask(p, &true)).is_none());
        assert!(consult(&packages, |_| true, |_, p| EntryRow::ask(p, &false)).is_none());
        let stray = Expanded {
            words: vec![("x".into(), 7)],
        };
        assert!(!stray.anchored_within(body.len()));
    }
}
