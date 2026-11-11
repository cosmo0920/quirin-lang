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

//! The impact declaration (`--declared PREDICTION.md`) and its check against
//! the measured moves (plan seam A, "Impact declaration").
//!
//! A prediction stays prose for people; the tool reads one fenced block of it,
//! tagged `measure-prediction`:
//!
//! ````text
//! ```measure-prediction
//! route corpus
//! move cJSON_DetachItemFromArray Unknown -> Safe
//! move librdkafka::cJSON.c::cJSON_DeleteItemFromArray Unknown -> Safe
//! delta Safe +2
//! delta Unknown -2
//! route system
//! none
//! ```
//! ````
//!
//! * `route NAME` opens a route's declarations; a measured route the block
//!   does not name predicts no move.
//! * `move SELECTOR BEFORE -> AFTER`: a function row selected by `function`,
//!   `file::function` or `library::file::function` changes status. `*` is any
//!   status; `absent` is a row that does not exist on that side.
//! * `delta STATUS ±N`: the route's total of rows with that status moves by
//!   exactly N (re-derived from both runs, never from the declared moves).
//! * `none`: the route predicts no move.
//! * blank lines and lines starting with `#` are ignored.
//!
//! Every measured move no declaration covers is **unpredicted**; every
//! declared move no measured move matches is **missing**; both are named in
//! the record, row by row. A missing block, an unknown directive or a
//! malformed line refuses the check (fail-closed), never an empty prediction.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// The fence tag.
pub const TAG: &str = "measure-prediction";

/// One declared move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredMove {
    pub line: usize,
    pub selector: String,
    pub before: String,
    pub after: String,
}

/// A route's declarations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeclaredRoute {
    pub moves: Vec<DeclaredMove>,
    pub deltas: BTreeMap<String, i64>,
    pub none: bool,
}

/// The parsed block.
pub type Declaration = BTreeMap<String, DeclaredRoute>;

/// Parse the `measure-prediction` block of a prediction document.
///
/// # Errors
///
/// No block, more than one, or a malformed line (named with its number).
pub fn parse(text: &str) -> Result<Declaration, String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<(usize, &str)>> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        match &mut current {
            None if trimmed.starts_with("```") && trimmed.trim_start_matches('`').trim() == TAG => {
                current = Some(Vec::new());
            }
            Some(_) if trimmed.starts_with("```") => {
                blocks.push(current.take().unwrap_or_default());
            }
            Some(lines) => lines.push((index + 1, trimmed)),
            None => {}
        }
    }
    if current.is_some() {
        return Err(format!("the `{TAG}` block is not closed"));
    }
    let lines = match blocks.as_slice() {
        [only] => only,
        [] => return Err(format!("no fenced `{TAG}` block")),
        _ => {
            return Err(format!(
                "{} `{TAG}` blocks; exactly one is read",
                blocks.len()
            ));
        }
    };
    let mut declaration = Declaration::new();
    let mut route: Option<String> = None;
    for &(number, line) in lines {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let fail = |why: &str| format!("line {number}: {why}: `{line}`");
        match words.as_slice() {
            ["route", name] => {
                if declaration.contains_key(*name) {
                    return Err(fail("route declared twice"));
                }
                declaration.insert((*name).to_owned(), DeclaredRoute::default());
                route = Some((*name).to_owned());
            }
            [directive, ..] => {
                let name = route
                    .as_ref()
                    .ok_or_else(|| fail("no `route` line before"))?;
                let entry = declaration.get_mut(name).expect("opened above");
                match (*directive, &words[1..]) {
                    ("move", [selector, before, "->", after]) => {
                        entry.moves.push(DeclaredMove {
                            line: number,
                            selector: (*selector).to_owned(),
                            before: (*before).to_owned(),
                            after: (*after).to_owned(),
                        });
                    }
                    ("delta", [status, amount]) => {
                        let value: i64 = amount
                            .strip_prefix('+')
                            .unwrap_or(amount)
                            .parse()
                            .map_err(|_| fail("a delta needs a signed integer"))?;
                        if entry.deltas.insert((*status).to_owned(), value).is_some() {
                            return Err(fail("delta declared twice"));
                        }
                    }
                    ("none", []) => entry.none = true,
                    _ => return Err(fail("unknown or malformed directive")),
                }
                if entry.none && !entry.moves.is_empty() {
                    return Err(fail("`none` beside a `move`"));
                }
            }
            [] => {}
        }
    }
    Ok(declaration)
}

/// Whether a selector names a row.
#[must_use]
pub fn selects(selector: &str, library: &str, file: &str, function: &str) -> bool {
    let parts: Vec<&str> = selector.split("::").collect();
    match parts.as_slice() {
        [name] => *name == function,
        [unit, name] => *name == function && (*unit == file || file.ends_with(&format!("/{unit}"))),
        [lib, unit, name] => *lib == library && *unit == file && *name == function,
        _ => false,
    }
}

fn status_matches(declared: &str, measured: &str) -> bool {
    declared == "*" || declared == measured
}

/// A measured move, as the check reads it.
#[derive(Clone, Debug)]
pub struct MeasuredMove {
    pub key: String,
    pub library: String,
    pub file: String,
    pub function: String,
    pub before: String,
    pub after: String,
}

/// Check one route's measured moves and total deltas against its
/// declarations (`None`: the route is not declared, so it predicts no move).
#[must_use]
pub fn check_route(
    declared: Option<&DeclaredRoute>,
    measured: &[MeasuredMove],
    deltas: &BTreeMap<String, i64>,
) -> Value {
    let empty = DeclaredRoute::default();
    let declared_route = declared.unwrap_or(&empty);
    let mut used = vec![false; declared_route.moves.len()];
    let mut unpredicted = Vec::new();
    for movement in measured {
        let mut covered = false;
        for (index, declaration) in declared_route.moves.iter().enumerate() {
            if selects(
                &declaration.selector,
                &movement.library,
                &movement.file,
                &movement.function,
            ) && status_matches(&declaration.before, &movement.before)
                && status_matches(&declaration.after, &movement.after)
            {
                used[index] = true;
                covered = true;
            }
        }
        if !covered {
            unpredicted.push(json!({"row": movement.key, "before": movement.before,
                                    "after": movement.after}));
        }
    }
    let missing: Vec<Value> = declared_route
        .moves
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(declaration, _)| {
            json!({"line": declaration.line, "selector": declaration.selector,
                   "before": declaration.before, "after": declaration.after})
        })
        .collect();
    let delta_mismatches: Vec<Value> = declared_route
        .deltas
        .iter()
        .filter(|(status, value)| deltas.get(*status).copied().unwrap_or(0) != **value)
        .map(|(status, value)| {
            json!({"status": status, "declared": value,
                   "measured": deltas.get(status).copied().unwrap_or(0)})
        })
        .collect();
    let matched = unpredicted.is_empty() && missing.is_empty() && delta_mismatches.is_empty();
    json!({
        "declared": declared.is_some(),
        "result": if matched { "match" } else { "mismatch" },
        "unpredicted": unpredicted,
        "missing": missing,
        "delta_mismatches": delta_mismatches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Prediction\n\nprose\n\n```measure-prediction\nroute corpus\n\
        move cJSON_DetachItemFromArray Unknown -> Safe\nmove l::cJSON.c::cJSON_Delete * -> Safe\n\
        delta Safe +2\ndelta Unknown -2\n# comment\nroute system\nnone\n```\n";

    fn movement(function: &str, before: &str, after: &str) -> MeasuredMove {
        MeasuredMove {
            key: format!("l::cJSON.c::unit#0::{function}#0"),
            library: "l".into(),
            file: "cJSON.c".into(),
            function: function.into(),
            before: before.into(),
            after: after.into(),
        }
    }

    #[test]
    fn the_block_parses() {
        let declaration = parse(DOC).unwrap();
        assert_eq!(declaration["corpus"].moves.len(), 2);
        assert_eq!(declaration["corpus"].deltas["Unknown"], -2);
        assert!(declaration["system"].none);
    }

    #[test]
    fn malformed_or_missing_blocks_refuse() {
        assert!(parse("no block").unwrap_err().contains("no fenced"));
        assert!(
            parse("```measure-prediction\nmove a b -> c\n```")
                .unwrap_err()
                .contains("no `route`")
        );
        assert!(
            parse("```measure-prediction\nroute a\nmove a b c\n```")
                .unwrap_err()
                .contains("malformed")
        );
        assert!(
            parse("```measure-prediction\nroute a\n")
                .unwrap_err()
                .contains("not closed")
        );
        assert!(parse("```measure-prediction\nroute a\nnone\nmove x A -> B\n```").is_err());
    }

    #[test]
    fn unpredicted_missing_and_delta_mismatches_are_named() {
        let declaration = parse(DOC).unwrap();
        let deltas = BTreeMap::from([("Safe".to_owned(), 2), ("Unknown".to_owned(), -2)]);
        let exact = [
            movement("cJSON_DetachItemFromArray", "Unknown", "Safe"),
            movement("cJSON_Delete", "Unknown", "Safe"),
        ];
        let result = check_route(declaration.get("corpus"), &exact, &deltas);
        assert_eq!(result["result"], "match");
        let extra = [exact[0].clone(), movement("other", "Safe", "Unknown")];
        let result = check_route(declaration.get("corpus"), &extra, &BTreeMap::new());
        assert_eq!(result["result"], "mismatch");
        assert_eq!(
            result["unpredicted"][0]["row"],
            "l::cJSON.c::unit#0::other#0"
        );
        assert_eq!(result["missing"][0]["selector"], "l::cJSON.c::cJSON_Delete");
        assert_eq!(result["delta_mismatches"].as_array().unwrap().len(), 2);
        let undeclared = check_route(None, &exact[..1], &BTreeMap::new());
        assert_eq!(undeclared["declared"], false);
        assert_eq!(undeclared["result"], "mismatch");
    }

    #[test]
    fn selectors_qualify_by_file_and_library() {
        assert!(selects("f", "l", "src/a.c", "f"));
        assert!(selects("a.c::f", "l", "src/a.c", "f"));
        assert!(!selects("b.c::f", "l", "src/a.c", "f"));
        assert!(selects("l::src/a.c::f", "l", "src/a.c", "f"));
        assert!(!selects("m::src/a.c::f", "l", "src/a.c", "f"));
    }
}
