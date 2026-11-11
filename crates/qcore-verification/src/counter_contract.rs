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

//! Checked unit counter updates from complete typed C paths. The native contract
//! supplies field selection and update direction; caller storage, exclusive
//! access and the unsigned-C/usize ABI binding remain explicit premises.
use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use qcore_frontend::frontend::{DeclarationKind, SourceContractClauseKind, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CounterCertificate {
    source: SourceSummary,
    contract_source: String,
    field: String,
    delta: i8,
    root_c_type: String,
    root_qcore_type: String,
}
impl CounterCertificate {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn field(&self) -> &str {
        &self.field
    }
    pub fn delta(&self) -> i8 {
        self.delta
    }
    pub fn root_c_type(&self) -> &str {
        &self.root_c_type
    }
    pub fn root_qcore_type(&self) -> &str {
        &self.root_qcore_type
    }
    /// The checked unsigned storage type. `check_counter_contract` accepted
    /// this certificate only after requiring the imported signature to be
    /// exactly `"{counter type} ({root C type})"`, so the split is total.
    pub fn counter_c_type(&self) -> &str {
        self.source
            .signature()
            .strip_suffix(&format!(" ({})", self.root_c_type))
            .expect("checked counter signature")
    }
    /// `maximum` must be the unsigned C type's actual maximum in the selected
    /// ABI. Failure leaves the supplied value untouched. No shared state changes.
    pub fn instantiate(&self, value: u128, maximum: u128) -> Result<u128, String> {
        if maximum == 0 || value > maximum {
            return Err("counter is outside the supplied unsigned range".into());
        }
        let next = if self.delta == 1 {
            value.checked_add(1)
        } else {
            value.checked_sub(1)
        };
        next.filter(|next| *next <= maximum)
            .ok_or("counter call would wrap; contract precondition unmet".into())
    }
}
fn tokens(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != TokenKind::Eof)
        .collect())
}
fn is_symbol(value: &SourceValue, operation: usize, ty: &str) -> bool {
    value.c_type == ty && value.expression == Expr::Symbol { operation }
}

pub fn check_counter_contract(
    summary: &SourceSummary,
    source: &str,
    root_c_type: &str,
    root_qcore_type: &str,
) -> Result<CounterCertificate, String> {
    let [path] = summary.paths() else {
        return Err("counter requires one complete source path".into());
    };
    let [
        Op::Read { place },
        Op::Write {
            place: written,
            value,
        },
        Op::Read { place: returned },
        Op::ReturnValue { value: result },
    ] = path.operations.as_slice()
    else {
        return Err("counter requires one read/update/read/return with no other effects".into());
    };
    let Expr::Field { base, name } = &place.expression else {
        return Err("counter must be a direct field".into());
    };
    let ty = &place.c_type;
    if !matches!(
        ty.as_str(),
        "unsigned int" | "unsigned long" | "unsigned long long"
    ) || base.c_type != root_c_type
        || base.expression != (Expr::Parameter { index: 0 })
        || written != place
        || returned != place
        || summary.signature() != format!("{ty} ({root_c_type})")
    {
        return Err("counter field/root/result binding mismatch".into());
    }
    let Expr::Binary {
        operator,
        left,
        right,
    } = &value.expression
    else {
        return Err("counter write is not a unit update".into());
    };
    if value.c_type != *ty
        || !is_symbol(left, 0, ty)
        || right.c_type != *ty
        || right.expression != (Expr::Integer { value: "1".into() })
        || !is_symbol(result, 2, ty)
    {
        return Err("counter update/result producer mismatch".into());
    }
    let delta = match operator.as_str() {
        "+" => 1,
        "-" => -1,
        _ => return Err("counter update is not addition/subtraction".into()),
    };
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == summary.function())
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one counter contract".into());
    };
    let decl = &decl.value;
    if decl.kind != DeclarationKind::ExternFunction || decl.parameters.len() != 1 {
        return Err("counter requires one borrowed external parameter".into());
    }
    let parameter = &decl.parameters[0];
    let actual = tokens(&parameter.type_source)?;
    let expected = tokens(&format!("borrow<'call, mut, {root_qcore_type}>"))?;
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(&expected)
            .enumerate()
            .any(|(i, (a, b))| i != 3 && a != b)
        || !matches!(actual.get(3), Some(TokenKind::Word(_)))
        || parameter.usage.as_ref().map(|u| u.value) != Some(qcore_core::usage::Usage::ONE)
    {
        return Err("counter requires bound mutable borrow with explicit use one".into());
    }
    let arrow = decl
        .tokens
        .iter()
        .position(|t| *t == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing counter result")?;
    if decl.tokens.get(arrow + 1) != Some(&TokenKind::Word("usize".into()))
        || decl.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err("counter requires usize result without extra effects".into());
    }
    let mut premises = BTreeSet::new();
    for clause in &decl
        .function_contract
        .as_ref()
        .ok_or("missing counter clauses")?
        .clauses
    {
        if clause.value.kind == SourceContractClauseKind::Requires {
            let ts = tokens(&clause.value.expression_source)?;
            let [TokenKind::Word(name)] = ts.as_slice() else {
                return Err("unsupported counter premise".into());
            };
            if !premises.insert(name.clone()) {
                return Err("duplicate counter premise".into());
            }
        }
    }
    let required = BTreeSet::from([
        "counter_is_live".into(),
        "exclusive_counter_access".into(),
        "c_unsigned_matches_usize".into(),
        "counter_in_range".into(),
        format!("counter_field__{name}"),
        if delta == 1 {
            "counter_below_maximum".into()
        } else {
            "counter_positive".into()
        },
    ]);
    if premises != required {
        return Err("counter contract lacks exact field/domain/nonwrapping premises".into());
    }
    let mut facts: Vec<_> = premises.iter().map(|p| (p.as_str(), 1)).collect();
    facts.extend([
        ("delta", i128::from(delta)),
        ("result_is_updated_counter", 1),
        ("only_selected_field_written", 1),
        ("no_unsigned_wrap", 1),
    ]);
    qcore_contract::contract_observation::check_contract_facts(source, summary.function(), &facts)?;
    Ok(CounterCertificate {
        source: summary.clone(),
        contract_source: source.into(),
        field: name.clone(),
        delta,
        root_c_type: root_c_type.into(),
        root_qcore_type: root_qcore_type.into(),
    })
}
