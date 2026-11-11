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

//! Structural theorem instantiation. This is not a Lean verifier or evidence issuer.
use qcore_schemas::hypothesis::ArtifactReference;
use qcore_schemas::quant_proposition::{Formula as F, Term};
use qcore_schemas::quant_theorem::{Application, TheoremSchema};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenBinding {
    Open,
}
/// Sealed, serialize-only obligations: construction requires checked substitution.
/// Neither artifact identities nor this record establish a model proof or C binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InstantiationObligations {
    reference: String,
    prerequisites: Vec<F>,
    conclusion: F,
    model: ArtifactReference,
    audit: ArtifactReference,
    model_proof_binding: OpenBinding,
    source_binding: OpenBinding,
}
impl InstantiationObligations {
    pub fn prerequisites(&self) -> &[F] {
        &self.prerequisites
    }
    pub fn conclusion(&self) -> &F {
        &self.conclusion
    }
    pub fn model_proof_binding(&self) -> OpenBinding {
        self.model_proof_binding
    }
    pub fn source_binding(&self) -> OpenBinding {
        self.source_binding
    }
}
fn error() -> String {
    "quant-theorem: schema, signature, capture or instantiated formula mismatch".into()
}
fn identity(a: &ArtifactReference) -> bool {
    !a.path.is_empty()
        && a.path.len() <= 4096
        && !a.path.chars().any(char::is_control)
        && a.sha256.len() == 64
        && a.sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn symbol(name: &str, substitutions: &BTreeMap<String, String>) -> String {
    substitutions
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.into())
}
fn term(t: &Term, s: &BTreeMap<String, String>) -> Term {
    match t {
        Term::Variable { name } => Term::Variable {
            name: symbol(name, s),
        },
        Term::Integer { decimal } => Term::Integer {
            decimal: decimal.clone(),
        },
        Term::Apply {
            function,
            arguments,
        } => Term::Apply {
            function: symbol(function, s),
            arguments: arguments.iter().map(|t| term(t, s)).collect(),
        },
        Term::Add { left, right } => Term::Add {
            left: Box::new(term(left, s)),
            right: Box::new(term(right, s)),
        },
        Term::Sub { left, right } => Term::Sub {
            left: Box::new(term(left, s)),
            right: Box::new(term(right, s)),
        },
        Term::Mul { left, right } => Term::Mul {
            left: Box::new(term(left, s)),
            right: Box::new(term(right, s)),
        },
        Term::Neg { term: t } => Term::Neg {
            term: Box::new(term(t, s)),
        },
    }
}
fn formula(f: &F, s: &BTreeMap<String, String>) -> Result<F, String> {
    Ok(match f {
        F::Predicate { name, arguments } => F::Predicate {
            name: symbol(name, s),
            arguments: arguments.iter().map(|t| term(t, s)).collect(),
        },
        F::Equal { left, right } => F::Equal {
            left: term(left, s),
            right: term(right, s),
        },
        F::Compare {
            operator,
            left,
            right,
        } => F::Compare {
            operator: operator.clone(),
            left: term(left, s),
            right: term(right, s),
        },
        F::Not { body } => F::Not {
            body: Box::new(formula(body, s)?),
        },
        F::And { left, right } => F::And {
            left: Box::new(formula(left, s)?),
            right: Box::new(formula(right, s)?),
        },
        F::Or { left, right } => F::Or {
            left: Box::new(formula(left, s)?),
            right: Box::new(formula(right, s)?),
        },
        F::Implies { left, right } => F::Implies {
            left: Box::new(formula(left, s)?),
            right: Box::new(formula(right, s)?),
        },
        F::Forall {
            variable,
            sort,
            body,
        }
        | F::Exists {
            variable,
            sort,
            body,
        } => {
            // Reject captures rather than inventing bound names. Original schema validation
            // already rejects any binder shadowing a formal value/relation/function.
            if s.values().any(|v| v == variable) {
                return Err(error());
            }
            let body = Box::new(formula(body, s)?);
            if matches!(f, F::Forall { .. }) {
                F::Forall {
                    variable: variable.clone(),
                    sort: sort.clone(),
                    body,
                }
            } else {
                F::Exists {
                    variable: variable.clone(),
                    sort: sort.clone(),
                    body,
                }
            }
        }
    })
}
pub fn instantiate(
    expected: &TheoremSchema,
    app: &Application,
) -> Result<InstantiationObligations, String> {
    qcore_frontend::quant_theorem::validate(app)?;
    qcore_frontend::quant_theorem::validate_formulas(
        "theorem_schema",
        &expected.parameters,
        &expected.prerequisites,
        &expected.conclusion,
    )?;
    if !qcore_frontend::quant_theorem::valid_reference(&expected.reference) {
        return Err(error());
    }
    if app.reference != expected.reference
        || app.arguments.len() != expected.parameters.len()
        || !identity(&expected.model)
        || !identity(&expected.audit)
    {
        return Err(error());
    }
    let mut substitutions = BTreeMap::new();
    for (formal, actual) in expected.parameters.iter().zip(&app.arguments) {
        let actual_parameter = app
            .parameters
            .iter()
            .find(|p| p.name == *actual)
            .ok_or_else(error)?;
        if formal.sort != actual_parameter.sort {
            return Err(error());
        }
        substitutions.insert(formal.name.clone(), actual.clone());
    }
    let prerequisites = expected
        .prerequisites
        .iter()
        .map(|f| formula(f, &substitutions))
        .collect::<Result<Vec<_>, _>>()?;
    let conclusion = formula(&expected.conclusion, &substitutions)?;
    if prerequisites != app.prerequisites || conclusion != app.conclusion {
        return Err(error());
    }
    Ok(InstantiationObligations {
        reference: expected.reference.clone(),
        prerequisites,
        conclusion,
        model: expected.model.clone(),
        audit: expected.audit.clone(),
        model_proof_binding: OpenBinding::Open,
        source_binding: OpenBinding::Open,
    })
}
