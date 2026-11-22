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

//! Card **SP5-GLOBAL-PROSE-PREMISE-BAN**: the one legacy reader of a retired
//! string premise, confined to test code.
//!
//! The contract format refuses `resource_premise(NAME, "STATEMENT")` on every
//! route (contract specification C04.18, `QF001`), so a frozen text that
//! states one (a historical `.qv` under `docs/reports/`, a frozen fixture, the
//! byte restoration of `sp2_premise_checkpoint`) is never handed to a loader
//! as it stands. A test that replays one reads it through [`lift`], which
//! writes each recorded string statement as the formula card
//! SP2-SYMBOLIC-LIBRDKAFKA-PREMISES recorded for it
//! (`docs/reports/sp2-symbolic-librdkafka-premises/migration.json`, the
//! `(name, prose, formula)` statements), and nothing else. A string premise
//! the record does not hold stays a string, so the loader still refuses it:
//! the lift is total over the record and never invents a formula. A frozen
//! hypothesis graph is read through [`lift_graph`], which lifts its legacy
//! atoms and erasures the same way.
//!
//! **Unreachable from production.** This file is not part of any crate's
//! source: test targets include it by `#[path]`, and
//! `qcore-contract`'s `the_legacy_prose_reader_is_confined_to_test_code`
//! checks that every file naming it is test code. No production entry point
//! admits a string statement, and nothing here changes what one admits.

#![allow(dead_code)]

const RECORD: &str =
    include_str!("../../docs/reports/sp2-symbolic-librdkafka-premises/migration.json");

/// The recorded `(name, prose, formula)` statements.
pub fn statements() -> Vec<(String, String, String)> {
    let record: serde_json::Value =
        serde_json::from_str(RECORD).expect("the SP2 migration record is JSON");
    let mut out = Vec::new();
    for statement in record["statements"].as_array().unwrap() {
        let field = |key: &str| statement[key].as_str().unwrap().to_owned();
        out.push((field("name"), field("prose"), field("formula")));
    }
    out
}

/// `text` with every recorded `resource_premise(NAME, "PROSE")` written as
/// `resource_premise(NAME, FORMULA)`. Every other byte is unchanged.
pub fn lift(text: &str) -> String {
    let mut out = text.to_owned();
    for (name, prose, formula) in statements() {
        out = out.replace(
            &format!("resource_premise({name}, \"{prose}\")"),
            &format!("resource_premise({name}, {formula})"),
        );
    }
    out
}

/// The formula recorded for the string statement `prose` of the premise
/// `name`; `None` when the record holds no such statement.
pub fn formula_of(name: &str, prose: &str) -> Option<String> {
    statements()
        .into_iter()
        .find(|(n, p, _)| n == name && p == prose)
        .map(|(_, _, formula)| formula)
}

/// A frozen hypothesis graph (its JSON value) with each legacy premise's
/// recorded string statement lifted to its formula: the erasure of every
/// `legacy_premise_*` node and the statement of its `legacy_atom_*` atom.
/// Returns how many legacy premises were lifted.
pub fn lift_graph(graph: &mut serde_json::Value) -> usize {
    let mut lifted = Vec::new();
    if let Some(premises) = graph["premises"].as_object_mut() {
        for (id, premise) in premises.iter_mut() {
            if !id.starts_with("legacy_premise_") {
                continue;
            }
            let name = premise["erasure"]["name"].as_str().unwrap().to_owned();
            let prose = premise["erasure"]["statement"].as_str().unwrap().to_owned();
            if let Some(formula) = formula_of(&name, &prose) {
                premise["erasure"]["statement"] = serde_json::Value::String(formula.clone());
                lifted.push((
                    id.replacen("legacy_premise_", "legacy_atom_", 1),
                    prose,
                    formula,
                ));
            }
        }
    }
    for (atom, prose, formula) in &lifted {
        let statement = &mut graph["atoms"][atom]["proposition"]["statement"];
        assert_eq!(statement.as_str(), Some(prose.as_str()), "{atom}");
        *statement = serde_json::Value::String(formula.clone());
    }
    lifted.len()
}

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN: the frozen guarded adoption plan
/// (`docs/reports/guarded-contract-adoption/adoption-plan.json`) replayed on
/// the module's current bytes. The plan froze the whole source as it stood
/// before card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: `restored` (its exact
/// recovery, `sp2_premise_checkpoint::restored_file`) must have the plan's
/// two digests, before and after the rendered `addition`. The returned plan
/// is the frozen one with its graph lifted ([`lift_graph`]) and its two
/// digests those of `current`, whose only differences from `restored` are
/// SP2's recorded conversions; nothing else in the plan changes.
pub fn lifted_adoption_plan(
    frozen: &[u8],
    restored: &str,
    current: &str,
    addition: &str,
    sha256: fn(&[u8]) -> String,
) -> Vec<u8> {
    let mut plan: serde_json::Value = serde_json::from_slice(frozen).unwrap();
    let digests = |text: &str| {
        assert_eq!(text.matches(addition).count(), 1);
        (
            sha256(text.replacen(addition, "", 1).as_bytes()),
            sha256(text.as_bytes()),
        )
    };
    let (before, after) = digests(restored);
    assert_eq!(plan["before_sha256"].as_str(), Some(before.as_str()));
    assert_eq!(plan["after_sha256"].as_str(), Some(after.as_str()));
    assert!(
        lift_graph(&mut plan["graph"]) > 0,
        "the plan's graph states no recorded prose"
    );
    let (before, after) = digests(current);
    plan["before_sha256"] = serde_json::Value::String(before);
    plan["after_sha256"] = serde_json::Value::String(after);
    serde_json::to_vec_pretty(&plan).unwrap()
}
