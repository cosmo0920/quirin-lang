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

//!
//! Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS** extends it to the retired string
//! hypothesis atom `resource_hypothesis(ID, "STATEMENT")` (contract
//! specification C04.18.7, `QF002`): [`lift`] also writes each recorded
//! string atom as the typed clause `quant proposition #ID(...) = FORMULA`
//! that replaced it, [`lift_graph`] each recorded opaque atom as its typed
//! graph atom, and [`lift_bindings`] each guarded prerequisite symbol that
//! repeated a recorded atom's sentence as its formula
//! (`docs/reports/sp6-symbolic-hypothesis-atoms/migration.json`,
//! `hypotheses`). The same rule holds: an unrecorded string atom stays a
//! string, and the parser refuses it.
//!
//! Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS** extends it to the retired
//! string statement of an outer contract definition, `quant #outer
//! [[contract]] NAME, "STATEMENT";` (contract specification C04.20, `QF004`):
//! [`lift`] writes each recorded statement as the formula that replaced it
//! (`docs/reports/sp8-outer-contract-symbolic-statements/migration.json`,
//! `outer_statements`). An unrecorded string statement stays a string, and
//! the parser refuses it.

//!
//! Card **SP6-GUARDED-SYMBOL-PROSE** extends it to the retired string forms
//! of the guarded source heads (contract specification C04.18.6, `QF003`):
//! [`lift`] also writes each recorded string text of a
//! `resource_guarded_symbol_v1` clause as the formula that replaced it
//! (`docs/reports/sp6-guarded-symbol-prose/migration.json`, `symbols`), and
//! each string `resource_guarded_decomposition_v1` document as its unquoted
//! clauses (a respelling of the same symbolic clauses, `;` to `,` and `∧` to
//! `&&`, never a new formula); [`lift_bindings`] lifts the recorded texts of
//! guarded metadata bindings; [`render_string_form`] reproduces the retired
//! source form of frozen metadata, so a frozen digest can still be checked.
//! An unrecorded string text stays a string, and the parser refuses it.

#![allow(dead_code)]

const SP7_RECORD: &str = include_str!("../../docs/reports/sp6-guarded-symbol-prose/migration.json");

/// One recorded guarded symbol text: ID, role, the retired string text and
/// the formula that replaced it (card SP6-GUARDED-SYMBOL-PROSE).
pub struct GuardedSymbol {
    pub id: String,
    pub role: String,
    pub prose: String,
    pub formula: String,
}

/// The recorded guarded symbol texts (card SP6-GUARDED-SYMBOL-PROSE).
pub fn guarded_symbols() -> Vec<GuardedSymbol> {
    let record: serde_json::Value =
        serde_json::from_str(SP7_RECORD).expect("the SP7 migration record is JSON");
    record["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let field = |key: &str| s[key].as_str().unwrap().to_owned();
            GuardedSymbol {
                id: field("id"),
                role: field("role"),
                prose: field("prose"),
                formula: field("formula"),
            }
        })
        .collect()
}

/// A string guarded document's clauses in their unquoted source spelling:
/// the version header dropped, one clause per `;`-terminated line, joined by
/// `, `, `∧` respelled `&&`. `None` when `readable` is not such a document.
pub fn document_source(readable: &str) -> Option<String> {
    let mut lines = readable.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next()? != "guarded_decomposition_v1;" {
        return None;
    }
    let clauses = lines
        .map(|l| l.strip_suffix(';').map(|c| c.replace(" ∧ ", " && ")))
        .collect::<Option<Vec<_>>>()?;
    Some(clauses.join(", "))
}

/// Card SP6-GUARDED-SYMBOL-PROSE: `text` with every string
/// `resource_guarded_decomposition_v1(ENTRY, "DOCUMENT")` written with its
/// clauses unquoted ([`document_source`]).
fn lift_documents(text: &str) -> String {
    const HEAD: &str = "resource_guarded_decomposition_v1(";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(HEAD) {
        let (before, tail) = rest.split_at(at + HEAD.len());
        out.push_str(before);
        rest = tail;
        let Some((entry, after_entry)) = rest.split_once(", \"") else {
            continue;
        };
        let Some((readable, after)) = after_entry.split_once("\")") else {
            continue;
        };
        if entry.contains(['(', ')', '"', '\n']) {
            continue;
        }
        if let Some(source) = document_source(readable) {
            out.push_str(&format!("{entry}, {source})"));
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// The retired source form of guarded metadata (`{entry, bindings,
/// readable}` as JSON): the rendering `guarded_metadata_bridge::render` gave
/// before card SP6-GUARDED-SYMBOL-PROSE, each proposition and the document a
/// string literal. Frozen digests were taken over this form.
pub fn render_string_form(metadata: &serde_json::Value) -> String {
    let mut out = String::new();
    for (id, binding) in metadata["bindings"].as_object().unwrap() {
        let deps = binding["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| format!(", {}", d.as_str().unwrap()))
            .collect::<String>();
        out.push_str(&format!(
            "    requires resource_guarded_symbol_v1({id}, {}, \"{}\"{deps});\n",
            binding["role"].as_str().unwrap(),
            binding["proposition"].as_str().unwrap()
        ));
    }
    out.push_str(&format!(
        "    requires resource_guarded_decomposition_v1({}, \"{}\");\n",
        metadata["entry"].as_str().unwrap(),
        metadata["readable"].as_str().unwrap()
    ));
    out
}

const RECORD: &str =
    include_str!("../../docs/reports/sp2-symbolic-librdkafka-premises/migration.json");

const SP6_RECORD: &str =
    include_str!("../../docs/reports/sp6-symbolic-hypothesis-atoms/migration.json");

const SP8_RECORD: &str =
    include_str!("../../docs/reports/sp8-outer-contract-symbolic-statements/migration.json");

/// The recorded `(name, prose, formula)` outer contract statements (card
/// SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS).
pub fn outer_statements() -> Vec<(String, String, String)> {
    let record: serde_json::Value =
        serde_json::from_str(SP8_RECORD).expect("the SP8 migration record is JSON");
    record["outer_statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let field = |key: &str| s[key].as_str().unwrap().to_owned();
            (field("name"), field("prose"), field("formula"))
        })
        .collect()
}

/// One recorded string hypothesis atom: its ID, its retired sentence, the
/// typed clause that replaced it (`quant proposition #ID(...) = FORMULA`),
/// that clause's formula, and its graph atom (JSON).
pub struct Hypothesis {
    pub id: String,
    pub prose: String,
    pub typed: String,
    pub formula: String,
    pub atom: serde_json::Value,
}

/// The recorded hypothesis atoms (card SP6-SYMBOLIC-HYPOTHESIS-ATOMS).
pub fn hypotheses() -> Vec<Hypothesis> {
    let record: serde_json::Value =
        serde_json::from_str(SP6_RECORD).expect("the SP6 migration record is JSON");
    record["hypotheses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            let field = |key: &str| h[key].as_str().unwrap().to_owned();
            Hypothesis {
                id: field("id"),
                prose: field("prose"),
                typed: field("typed"),
                formula: field("formula"),
                atom: h["atom"].clone(),
            }
        })
        .collect()
}

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
/// `resource_premise(NAME, FORMULA)`, every recorded `requires
/// resource_hypothesis(ID, "PROSE")` as its typed clause, and every recorded
/// `quant #outer [[contract]] NAME, "PROSE"` as `quant #outer [[contract]]
/// NAME, FORMULA`. Every other byte is unchanged.
pub fn lift(text: &str) -> String {
    let mut out = text.to_owned();
    for (name, prose, formula) in statements() {
        out = out.replace(
            &format!("resource_premise({name}, \"{prose}\")"),
            &format!("resource_premise({name}, {formula})"),
        );
    }
    for (name, prose, formula) in outer_statements() {
        out = out.replace(
            &format!("quant #outer [[contract]] {name}, \"{prose}\""),
            &format!("quant #outer [[contract]] {name}, {formula}"),
        );
    }
    for h in hypotheses() {
        out = out.replace(
            &format!("requires resource_hypothesis({}, \"{}\")", h.id, h.prose),
            &h.typed,
        );
        // A guarded prerequisite symbol that repeated the atom's sentence
        // repeats its formula.
        out = out.replace(
            &format!(
                "resource_guarded_symbol_v1({}, prerequisite, \"{}\", {})",
                h.id, h.prose, h.id
            ),
            &format!(
                "resource_guarded_symbol_v1({}, prerequisite, \"{}\", {})",
                h.id, h.formula, h.id
            ),
        );
    }
    // Card SP6-GUARDED-SYMBOL-PROSE: each recorded string symbol text as its
    // formula, and each string document as its unquoted clauses.
    for s in guarded_symbols() {
        out = out.replace(
            &format!(
                "resource_guarded_symbol_v1({}, {}, \"{}\"",
                s.id, s.role, s.prose
            ),
            &format!(
                "resource_guarded_symbol_v1({}, {}, {}",
                s.id, s.role, s.formula
            ),
        );
    }
    lift_documents(&out)
}

/// A frozen hypothesis graph's recorded opaque atoms written as their typed
/// atoms (the graph version raised to the typed one, 3, when it was lower).
/// Returns how many atoms were lifted.
pub fn lift_graph_hypotheses(graph: &mut serde_json::Value) -> usize {
    let mut lifted = 0;
    let recorded = hypotheses();
    if let Some(atoms) = graph["atoms"].as_object_mut() {
        for (id, atom) in atoms.iter_mut() {
            if id.starts_with("legacy_atom_") || atom["proposition"]["kind"] != "opaque" {
                continue;
            }
            let statement = atom["proposition"]["statement"].as_str().unwrap();
            if let Some(h) = recorded
                .iter()
                .find(|h| h.id == *id && h.prose == statement)
            {
                *atom = h.atom.clone();
                lifted += 1;
            }
        }
    }
    if lifted > 0 && graph["version"].as_u64().is_some_and(|v| v < 3) {
        graph["version"] = serde_json::Value::from(3);
    }
    lifted
}

/// Guarded metadata bindings (`{ID: {role, proposition, dependencies}}`)
/// with each symbol whose proposition repeated a recorded atom's sentence
/// (the atom of the same ID) written as that atom's formula, and (card
/// SP6-GUARDED-SYMBOL-PROSE) each symbol whose proposition is a recorded
/// English text written as its recorded formula. Returns how many were
/// lifted.
pub fn lift_bindings(bindings: &mut serde_json::Value) -> usize {
    let mut lifted = 0;
    let recorded = hypotheses();
    let guarded = guarded_symbols();
    if let Some(bindings) = bindings.as_object_mut() {
        for (id, binding) in bindings.iter_mut() {
            let proposition = binding["proposition"].as_str().unwrap_or_default();
            if let Some(h) = recorded
                .iter()
                .find(|h| h.id == *id && h.prose == proposition)
            {
                binding["proposition"] = serde_json::Value::String(h.formula.clone());
                lifted += 1;
            } else if let Some(s) = guarded.iter().find(|s| {
                s.id == *id
                    && s.prose == proposition
                    && s.prose != s.formula
                    && binding["role"].as_str() == Some(s.role.as_str())
            }) {
                binding["proposition"] = serde_json::Value::String(s.formula.clone());
                lifted += 1;
            }
        }
    }
    lifted
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
    // Card SP6-SYMBOLIC-HYPOTHESIS-ATOMS: and its recorded string atoms.
    lifted.len() + lift_graph_hypotheses(graph)
}

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN: the frozen guarded adoption plan
/// (`docs/reports/guarded-contract-adoption/adoption-plan.json`) replayed on
/// the module's current bytes. The plan froze the whole source as it stood
/// before card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: `restored` (its exact
/// recovery, `sp2_premise_checkpoint::restored_file`) must have the plan's
/// two digests, before and after the rendered `addition`. The returned plan
/// is the frozen one with its graph lifted ([`lift_graph`]) and its two
/// digests those of `current`, whose only differences from `restored` are
/// SP2's recorded conversions; nothing else in the plan changes. Card
/// SP6-SYMBOLIC-HYPOTHESIS-ATOMS: the graph's nine string atoms are lifted
/// too, and the nine prerequisite symbols of the metadata repeat their
/// formulas (`lift_bindings`); `current`'s digests are taken around the
/// lifted `addition`.
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
    // Card SP6-SYMBOLIC-HYPOTHESIS-ATOMS: the nine guarded prerequisite
    // symbols repeat their atoms' formulas. Card SP6-GUARDED-SYMBOL-PROSE:
    // the other eight state their recorded formulas, and `addition` (the
    // frozen metadata's string form, `render_string_form`) is lifted to the
    // unquoted form.
    assert_eq!(lift_bindings(&mut plan["metadata"]["bindings"]), 17);
    let addition = &lift(addition);
    let digests = |text: &str| {
        assert_eq!(text.matches(addition.as_str()).count(), 1);
        (
            sha256(text.replacen(addition.as_str(), "", 1).as_bytes()),
            sha256(text.as_bytes()),
        )
    };
    let (before, after) = digests(current);
    plan["before_sha256"] = serde_json::Value::String(before);
    plan["after_sha256"] = serde_json::Value::String(after);
    serde_json::to_vec_pretty(&plan).unwrap()
}
