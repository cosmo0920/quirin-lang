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

//! Card **SP10-FORMAL-BINDING-DESCRIPTIONS** (contract specification C04.18.2,
//! C04.18.3, C04.18.9's SP10 block; diagnostics namespace `QF***`): a binding
//! manifest's or flow model's environment and formal bindings state typed
//! references, never English descriptions.
//!
//! * Each `environment.variables` value is a **typed environment reference**:
//!   the canonical JSON list of G01p parameters (`{"name","sort"}`, the
//!   encoding the bindings use for quantifiers), nonempty and strictly
//!   increasing by name. Every term is a parameter of a bound atom's typed
//!   definition in the same sort, or a registered term in its rendered sort
//!   (`premise_formula::rendered_parameter_sort`: the C04.18.9 vocabulary and
//!   the SP6, SP7 and SP10 blocks). The reference says which typed terms an
//!   environment component consists of; it asserts no condition.
//! * A value that binds the identity of exact bytes held elsewhere is a
//!   content digest (64 lowercase hex).
//! * A `FormalBinding` names a qualified proposition (a Lean definition or a recorded proposition id) and its
//!   quantifiers and conclusion are exactly the bound atom's typed parameters
//!   and formula, each canonical JSON: they are derived from the typed atom,
//!   so nothing is left to describe.
//!
//! Anything else, English above all, is refused with [`PROSE_BINDING`] on
//! every load route: `hypothesis_bindings::resolve`, `resolve_bounds`,
//! `resolve_model` and `premise_flow::compile`, `compile_bounds`,
//! `compile_model`. Frozen manifests are read only through the test-side
//! legacy reader (`tests/support/`, confined to test code). The check is
//! reference integrity only: it establishes no source correspondence, no
//! proposition and no verdict.
use qcore_schemas::hypothesis::{Atom, EnvironmentDefinition, FormalBinding, Proposition};
use qcore_schemas::quant_proposition::{Definition, Parameter};

/// The code of the refusal of an environment description or formal binding
/// field that is not a typed reference (C04.18.2, C04.18.3).
pub const PROSE_BINDING: &str = "QF005";

fn refuse(message: impl std::fmt::Display) -> String {
    format!("{PROSE_BINDING}: {message}")
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn identifier(value: &str, extra: &[u8]) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || extra.contains(&b))
}

/// A qualified proposition name: a Lean definition (`QCore.Module.definition`)
/// or a recorded proposition id (`libc.strerror.….v1`), two or more nonempty
/// dot-separated segments of `[A-Za-z0-9_']`, so never a sentence.
fn qualified_name(value: &str) -> bool {
    let segments = value.split('.').collect::<Vec<_>>();
    segments.len() >= 2
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'\'')
        })
}

/// The typed definition an atom binds, if it has one.
#[must_use]
pub fn definition(atom: &Atom) -> Option<&Definition> {
    match &atom.proposition {
        Proposition::Formula { definition } | Proposition::EntryCondition { definition } => {
            Some(definition)
        }
        Proposition::Opaque { .. } | Proposition::Typed { .. } => None,
    }
}

/// The canonical encoding of a typed environment reference: the terms'
/// G01p parameters, sorted by name.
pub fn reference(terms: &[Parameter]) -> Result<String, String> {
    let mut terms = terms.to_vec();
    terms.sort_by(|a, b| a.name.cmp(&b.name));
    serde_json::to_string(&terms).map_err(|e| e.to_string())
}

/// The typed environment reference naming `names`, each typed as a
/// parameter of one of `definitions` or, failing that, as a registered term
/// in its rendered sort. A name that is neither is refused.
pub fn reference_to(names: &[&str], definitions: &[&Definition]) -> Result<String, String> {
    let terms = names
        .iter()
        .map(|name| {
            definitions
                .iter()
                .flat_map(|d| d.parameters.iter())
                .find(|p| p.name == *name)
                .cloned()
                .or_else(|| {
                    qcore_frontend::premise_formula::rendered_parameter_sort(name).map(|sort| {
                        Parameter {
                            name: (*name).to_owned(),
                            sort,
                        }
                    })
                })
                .ok_or_else(|| refuse(format!("`#{name}` is not a term to reference")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    reference(&terms)
}

/// A binding's derived quantifiers and conclusion: the atom's typed
/// parameters and formula, each canonical JSON.
pub fn derived(atom: &Atom) -> Result<(Vec<String>, String), String> {
    let definition =
        definition(atom).ok_or_else(|| refuse("the atom has no typed definition to reference"))?;
    let quantifiers = definition
        .parameters
        .iter()
        .map(|p| serde_json::to_string(p).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let conclusion = serde_json::to_string(&definition.formula).map_err(|e| e.to_string())?;
    Ok((quantifiers, conclusion))
}

/// What one environment value is (the Lean `EnvValue` of
/// `QCore.FormalBindingEnvironments`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// A content digest: an identity, not a description.
    Digest,
    /// A JSON parameter list; `canonical` when it is nonempty, strictly
    /// increasing by name and byte-canonical, `resolved` when every term is
    /// a bound atom's parameter or a registered term in its sort.
    Terms { canonical: bool, resolved: bool },
    /// Anything else: English, a quoted string, any other JSON.
    Text,
}

/// Classify one environment value against the bound atoms' definitions.
#[must_use]
pub fn classify(value: &str, definitions: &[&Definition]) -> ValueKind {
    if digest(value) {
        return ValueKind::Digest;
    }
    let Ok(terms) = serde_json::from_str::<Vec<Parameter>>(value) else {
        return ValueKind::Text;
    };
    let canonical = !terms.is_empty()
        && terms.windows(2).all(|w| w[0].name < w[1].name)
        && serde_json::to_string(&terms).ok().as_deref() == Some(value);
    let resolved = terms.iter().all(|t| {
        definitions
            .iter()
            .any(|d| d.parameters.iter().any(|p| p == t))
            || qcore_frontend::premise_formula::rendered_parameter_sort(&t.name).as_ref()
                == Some(&t.sort)
    });
    ValueKind::Terms {
        canonical,
        resolved,
    }
}

/// Whether a value of this kind is admitted (the Lean `admitEnv`).
#[must_use]
pub fn admitted(kind: ValueKind) -> bool {
    match kind {
        ValueKind::Digest => true,
        ValueKind::Terms {
            canonical,
            resolved,
        } => canonical && resolved,
        ValueKind::Text => false,
    }
}

/// Check an environment: an identifier, identifier variable names, and every
/// value a typed environment reference or a content digest.
pub fn check_environment(
    environment: &EnvironmentDefinition,
    definitions: &[&Definition],
) -> Result<(), String> {
    if !identifier(&environment.id, b".-") {
        return Err(refuse(format!(
            "environment id `{}` is not an identifier",
            environment.id
        )));
    }
    if environment.variables.is_empty() {
        return Err(refuse("an environment names no component"));
    }
    for (name, value) in &environment.variables {
        if !identifier(name, b"") {
            return Err(refuse(format!(
                "environment variable `{name}` is not an identifier"
            )));
        }
        match classify(value, definitions) {
            ValueKind::Text => {
                return Err(refuse(format!(
                    "environment variable `{name}` is a description, not a typed reference: {value:?}"
                )));
            }
            ValueKind::Terms {
                canonical: false, ..
            } => {
                return Err(refuse(format!(
                    "environment variable `{name}` is not a canonical typed reference (nonempty, strictly increasing by name, canonical JSON)"
                )));
            }
            ValueKind::Terms {
                resolved: false, ..
            } => {
                return Err(refuse(format!(
                    "environment variable `{name}` names a term that is neither a bound atom's parameter nor a registered term in that sort"
                )));
            }
            ValueKind::Digest | ValueKind::Terms { .. } => {}
        }
    }
    Ok(())
}

/// Check a formal binding: a qualified proposition name, and the bound atom's
/// typed parameters and formula as its quantifiers and conclusion.
pub fn check_binding(id: &str, binding: &FormalBinding, atom: &Atom) -> Result<(), String> {
    if !qualified_name(&binding.proposition) {
        return Err(refuse(format!(
            "binding `{id}`: proposition {:?} is not a qualified proposition name",
            binding.proposition
        )));
    }
    if definition(atom).is_none() {
        return Err(refuse(format!(
            "binding `{id}`: the atom has no typed definition to reference"
        )));
    }
    let (quantifiers, conclusion) = derived(atom)?;
    if binding.quantifiers != quantifiers {
        return Err(refuse(format!(
            "binding `{id}`: quantifiers are not the atom's typed parameters"
        )));
    }
    if binding.conclusion != conclusion {
        return Err(refuse(format!(
            "binding `{id}`: conclusion is not the atom's formula"
        )));
    }
    Ok(())
}

/// Check an environment and its bindings against the graph's atoms. Every
/// bound atom must exist (the callers check the domain first).
pub fn check_model<'a>(
    environment: &EnvironmentDefinition,
    bindings: impl IntoIterator<Item = (&'a String, &'a FormalBinding)> + Clone,
    atoms: &std::collections::BTreeMap<String, Atom>,
) -> Result<(), String> {
    let mut definitions = Vec::new();
    for (id, binding) in bindings.clone() {
        let atom = atoms
            .get(id)
            .ok_or_else(|| refuse(format!("binding `{id}` names no atom")))?;
        check_binding(id, binding, atom)?;
        definitions.extend(definition(atom));
    }
    check_environment(environment, &definitions)
}

/// The decision vectors shared with `proof/QCore/FormalBindingEnvironments.lean`
/// (`environmentVectors`): an environment value and whether it is admitted
/// against [`VECTOR_DEFINITION`]'s parameters.
pub const ENVIRONMENT_DECISION_VECTORS: &[(&str, bool)] = &[
    // The atom's own parameters.
    (
        r#"[{"name":"live","sort":{"kind":"predicate","arguments":["obj"]}},{"name":"parent","sort":{"kind":"value","name":"obj"}}]"#,
        true,
    ),
    // A registered SP10 term.
    (
        r#"[{"name":"bnd_source_corresponds","sort":{"kind":"predicate","arguments":[]}}]"#,
        true,
    ),
    // A content digest.
    (
        "27c0736584d81d659e44dd29eae891ccdc37cfeaf019a57a620fadaa73d910a1",
        true,
    ),
    // The retired English descriptions, whatever they say.
    (
        "Entry liveness predicate, source interpretation Open",
        false,
    ),
    ("live", false),
    (r#""live""#, false),
    (r#"{"name":"live"}"#, false),
    // A digest is lowercase hex.
    (
        "27C0736584D81D659E44DD29EAE891CCDC37CFEAF019A57A620FADAA73D910A1",
        false,
    ),
    // Not canonical: empty, unsorted, duplicated, spaced.
    ("[]", false),
    (
        r#"[{"name":"parent","sort":{"kind":"value","name":"obj"}},{"name":"live","sort":{"kind":"predicate","arguments":["obj"]}}]"#,
        false,
    ),
    (
        r#"[{"name":"parent","sort":{"kind":"value","name":"obj"}},{"name":"parent","sort":{"kind":"value","name":"obj"}}]"#,
        false,
    ),
    (
        r#"[ {"name":"parent","sort":{"kind":"value","name":"obj"}}]"#,
        false,
    ),
    // A term that is neither a parameter nor registered.
    (
        r#"[{"name":"environment_holds","sort":{"kind":"predicate","arguments":[]}}]"#,
        false,
    ),
    // A parameter or registered term in another sort.
    (
        r#"[{"name":"parent","sort":{"kind":"value","name":"fn"}}]"#,
        false,
    ),
];

/// The definition [`ENVIRONMENT_DECISION_VECTORS`] are classified against.
pub const VECTOR_DEFINITION: &str = r#"{"id":"a","parameters":[{"name":"live","sort":{"kind":"predicate","arguments":["obj"]}},{"name":"parent","sort":{"kind":"value","name":"obj"}}],"formula":{"kind":"predicate","name":"live","arguments":[{"kind":"variable","name":"parent"}]}}"#;
