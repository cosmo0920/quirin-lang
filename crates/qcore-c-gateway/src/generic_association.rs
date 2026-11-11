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

//! Card **CG07-GENERIC-AND-STATEMENT-EXPRESSIONS** (gateway rule 11g), gap
//! entry E17: Clang's selected association of a C11 generic selection.
//!
//! `_Generic (control, T1: e1, …, default: ed)` evaluates exactly one
//! association, the one the controlling expression's type selects at
//! translation time (C11 §6.5.1.1p3); the controlling expression is not
//! evaluated and neither is any other association. Clang records the
//! selection in its JSON AST: a `GenericSelectionExpr`'s children are the
//! controlling expression (and its type), then one entry per association in
//! source order, each with an `associationKind` (`case` or `default`); the
//! selected one carries `"selected": true`, and its last child is the
//! association's expression.
//!
//! [`selected`] reads that expression, and nothing else: the route that asks
//! evaluates it as the whole selection and never reads the controlling
//! expression or an unselected association, so neither contributes an event
//! or an obligation. [`associations`] reads the kinds and the selected index
//! so a token route can check that its own reading of the selection has the
//! same associations in the same order before it takes Clang's choice.
//! `QCore.StatementExpressions` models the selection (`select_events`,
//! `unselected_no_event`, `control_no_event`).

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// One association of a generic selection, as Clang lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Association {
    /// `type-name : expression`.
    Case,
    /// `default : expression`.
    Default,
}

/// The associations of a `GenericSelectionExpr` in source order, and the
/// index of the one Clang selected. `None` when the node is not a generic
/// selection, an association kind is unknown, or no association or more than
/// one is selected.
pub(crate) fn associations(node: &Value) -> Option<(Vec<Association>, usize)> {
    if kind(node) != "GenericSelectionExpr" {
        return None;
    }
    let mut out = Vec::new();
    let mut chosen = None;
    for child in children(node) {
        let Some(association) = child["associationKind"].as_str() else {
            continue;
        };
        let association = match association {
            "case" => Association::Case,
            "default" => Association::Default,
            _ => return None,
        };
        if child["selected"].as_bool() == Some(true) {
            if chosen.is_some() {
                return None;
            }
            chosen = Some(out.len());
        }
        out.push(association);
    }
    Some((out, chosen?))
}

/// The expression of the association Clang selected, the only part of the
/// selection C evaluates. `None` when [`associations`] reads no selection.
pub(crate) fn selected(node: &Value) -> Option<&Value> {
    let (_, chosen) = associations(node)?;
    let entry = children(node)
        .iter()
        .filter(|child| child["associationKind"].is_string())
        .nth(chosen)?;
    let expression = children(entry).last()?;
    // A `case` entry lists its type then its expression; a `default` entry
    // its expression alone. Either way the expression is an `Expr` node.
    (!kind(expression).is_empty() && !kind(expression).ends_with("Type")).then_some(expression)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(name: &str) -> Value {
        json!({"kind": "CallExpr", "inner": [{"kind": "DeclRefExpr", "referencedDecl": {"name": name}}]})
    }

    fn selection(chosen: usize) -> Value {
        let mut entries = vec![
            json!({"associationKind": "case", "inner": [{"kind": "BuiltinType"}, call("f")]}),
            json!({"associationKind": "case", "inner": [{"kind": "BuiltinType"}, call("g")]}),
            json!({"associationKind": "default", "inner": [{"kind": "IntegerLiteral"}]}),
        ];
        entries[chosen]["selected"] = json!(true);
        let mut inner = vec![
            json!({"kind": "DeclRefExpr"}),
            json!({"kind": "BuiltinType"}),
        ];
        inner.extend(entries);
        json!({"kind": "GenericSelectionExpr", "inner": inner})
    }

    #[test]
    fn the_selected_association_is_read_and_nothing_else() {
        let node = selection(1);
        assert_eq!(
            associations(&node),
            Some((
                vec![Association::Case, Association::Case, Association::Default],
                1
            ))
        );
        assert_eq!(selected(&node), Some(&call("g")));
        let node = selection(2);
        assert_eq!(kind(selected(&node).unwrap()), "IntegerLiteral");
    }

    #[test]
    fn no_selection_or_two_selections_refuse() {
        let mut node = selection(0);
        node["inner"][2]["selected"] = json!(false);
        assert_eq!(selected(&node), None);
        let mut node = selection(0);
        node["inner"][3]["selected"] = json!(true);
        assert_eq!(selected(&node), None);
        assert_eq!(selected(&call("f")), None);
    }
}
