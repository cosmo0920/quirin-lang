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

//! Card **W45-FB-LIST-FINITENESS-PREMISE** (contract specification C04.22,
//! grammar G05l, gateway rule 48): the declared list-finiteness premise.
//!
//! A premise file holds one declaration per intrusive list head:
//!
//! ```text
//! // a comment
//! quant list flb_config.outputs |-> #finite;
//! ```
//!
//! `quant list TAG.MEMBER |-> #finite;` states that the list whose head is the
//! `MEMBER` field of a `struct TAG` object is finite and well formed whenever
//! a traversal of it begins, and that no other thread writes its links while
//! the traversal runs. It is a **premise, stated and never proved**: no
//! consumer derives it, and it adds nothing to any resource contract. The
//! loader is total-or-reject: any line that is not a comment, blank or one
//! such declaration, and any duplicate, rejects the whole file, and a consumer
//! then treats every head as undeclared.

use std::collections::BTreeSet;

/// The declared heads, as `(TAG, MEMBER)` pairs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListFinitenessPremises {
    heads: BTreeSet<(String, String)>,
}

impl ListFinitenessPremises {
    /// Whether `struct TAG`'s `MEMBER` head is declared finite.
    pub fn declares(&self, tag: &str, member: &str) -> bool {
        self.heads.contains(&(tag.to_owned(), member.to_owned()))
    }

    /// The declared heads, sorted.
    pub fn heads(&self) -> impl Iterator<Item = (&str, &str)> {
        self.heads.iter().map(|(t, m)| (t.as_str(), m.as_str()))
    }

    pub fn len(&self) -> usize {
        self.heads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heads.is_empty()
    }

    /// The premise text of one head, as a declaration spells it.
    pub fn spelling(tag: &str, member: &str) -> String {
        format!("quant list {tag}.{member} |-> #finite")
    }
}

fn identifier(word: &str) -> bool {
    let mut bytes = word.bytes();
    bytes
        .next()
        .is_some_and(|b| b == b'_' || b.is_ascii_alphabetic())
        && bytes.all(|b| b == b'_' || b.is_ascii_alphanumeric())
}

/// Read one declaration: `quant list TAG.MEMBER |-> #finite;`, with any
/// whitespace between the tokens. `None` when the line is not one.
fn declaration(line: &str) -> Option<(String, String)> {
    let line = line.strip_suffix(';')?.trim_end();
    let rest = line.strip_prefix("quant")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start().strip_prefix("list")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (head, finite) = rest.trim_start().split_once("|->")?;
    let finite = finite.trim();
    let finite = finite.strip_prefix('#')?;
    if finite != "finite" {
        return None;
    }
    let (tag, member) = head.trim().split_once('.')?;
    (identifier(tag) && identifier(member)).then(|| (tag.to_owned(), member.to_owned()))
}

/// Load a premise file, total-or-reject.
pub fn parse_list_finiteness_premises(text: &str) -> Result<ListFinitenessPremises, String> {
    let mut premises = ListFinitenessPremises::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let Some(head) = declaration(line) else {
            return Err(format!(
                "line {}: not a list-finiteness declaration `quant list TAG.MEMBER |-> #finite;`: `{line}`",
                index + 1
            ));
        };
        if !premises.heads.insert(head.clone()) {
            return Err(format!(
                "line {}: duplicate declaration of `{}.{}`",
                index + 1,
                head.0,
                head.1
            ));
        }
    }
    Ok(premises)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declaration_loads() {
        let premises = parse_list_finiteness_premises(
            "// the configuration's lists\nquant list flb_config.outputs |-> #finite;\n\n  quant   list cio_ctx.streams|->#finite ;\n",
        )
        .unwrap();
        assert!(premises.declares("flb_config", "outputs"));
        assert!(premises.declares("cio_ctx", "streams"));
        assert!(!premises.declares("flb_config", "inputs"));
        assert_eq!(premises.len(), 2);
    }

    #[test]
    fn every_other_line_rejects_the_whole_file() {
        for text in [
            "quant list flb_config.outputs |-> #finite",
            "quant list flb_config |-> #finite;",
            "quant list flb_config.outputs.x |-> #finite;",
            "quant list flb_config.outputs |-> #bounded;",
            "quant cell flb_config.outputs |-> #finite;",
            "quantlist flb_config.outputs |-> #finite;",
            "quant list 1tag.outputs |-> #finite;",
            "quant list flb_config.outputs |-> #finite; extra",
            "/* block */",
        ] {
            assert!(
                parse_list_finiteness_premises(text).is_err(),
                "accepted `{text}`"
            );
        }
        assert!(
            parse_list_finiteness_premises(
                "quant list a.b |-> #finite;\nquant list a.b |-> #finite;"
            )
            .is_err()
        );
    }
}
