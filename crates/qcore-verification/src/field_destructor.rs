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

//! Exhaustive nullable-field custody replay over privately imported C paths.
//! Live/exclusive root storage, pairwise-distinct owned-or-null fields, matching
//! C/QCore root types and libc `free` binding are explicit contract premises.
//! This bounded checker proves field release, not enclosing-object teardown.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use qcore_contract::contract_observation::{ObservationVerdict, check_contract_observation};
use qcore_frontend::frontend::{DeclarationKind, SourceContractClauseKind, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldReleaseCase {
    pub initially_live: Vec<String>,
    pub released_in_order: Vec<String>,
    pub source_path: usize,
}

/// Construction requires checking every realizable nullable-field assignment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldDestructorCertificate {
    source: SourceSummary,
    contract_source: String,
    root_c_type: String,
    root_qcore_type: String,
    cases: Vec<FieldReleaseCase>,
    postconditions_per_case: usize,
}

impl FieldDestructorCertificate {
    pub fn cases(&self) -> &[FieldReleaseCase] {
        &self.cases
    }
    pub fn source(&self) -> &SourceSummary {
        &self.source
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

/// The owned-field inventory a pinned destructor contract declares, read with
/// the same `owned_or_null__` rule [`check_field_destructor`] applies. This is
/// a read-only extraction: it grants nothing, and a caller that disagrees with
/// it is refused rather than reconciled.
pub fn declared_owned_fields(source: &str, function: &str) -> Result<BTreeSet<String>, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == function)
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one destructor contract".into());
    };
    let mut fields = BTreeSet::new();
    for clause in &decl
        .value
        .function_contract
        .as_ref()
        .ok_or("missing destructor contract")?
        .clauses
    {
        if clause.value.kind != SourceContractClauseKind::Requires {
            continue;
        }
        let ts = tokens(&clause.value.expression_source)?;
        let [TokenKind::Word(name)] = ts.as_slice() else {
            return Err("unsupported destructor premise".into());
        };
        if let Some(path) = name.strip_prefix("owned_or_null__") {
            if path.split("__").any(|part| part.is_empty()) {
                return Err("empty field path component".into());
            }
            if !fields.insert(path.to_owned()) {
                return Err("duplicate declared owned field".into());
            }
        }
    }
    if fields.is_empty() {
        return Err("destructor contract declares no owned field".into());
    }
    Ok(fields)
}

/// `owned_or_null__a__b` declares the pointer field root->a->b. Every prefix
/// must also be declared. `old_live__a__b`, `live__a__b`, `released__a__b`
/// and `release_rank__a__b` are derived facts; rank is zero if no release occurs.
/// Unsupported predicates or operations reject even on an infeasible path.
pub fn check_field_destructor(
    summary: &SourceSummary,
    source: &str,
    root_c_type: &str,
    root_qcore_type: &str,
) -> Result<FieldDestructorCertificate, String> {
    if summary.signature() != format!("void ({root_c_type})") {
        return Err("destructor C signature does not match the supplied root binding".into());
    }
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == summary.function())
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one destructor contract".into());
    };
    let decl = &decl.value;
    if decl.kind != DeclarationKind::ExternFunction || decl.parameters.len() != 1 {
        return Err("destructor contract requires one external borrowed root".into());
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
        return Err("destructor requires the bound mutable borrow and explicit use one".into());
    }
    let arrow = decl
        .tokens
        .iter()
        .position(|t| *t == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing result type")?;
    if decl.tokens.get(arrow + 1) != Some(&TokenKind::Word("unit".into()))
        || decl.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err("destructor requires unit result without extra effects".into());
    }
    let mut premises = BTreeSet::new();
    let mut fields = BTreeSet::new();
    let contract = decl
        .function_contract
        .as_ref()
        .ok_or("missing destructor contract")?;
    for clause in &contract.clauses {
        if clause.value.kind != SourceContractClauseKind::Requires {
            continue;
        }
        let ts = tokens(&clause.value.expression_source)?;
        let [TokenKind::Word(name)] = ts.as_slice() else {
            return Err("unsupported destructor premise".into());
        };
        if !premises.insert(name.clone()) {
            return Err("duplicate destructor premise".into());
        }
        if let Some(path) = name.strip_prefix("owned_or_null__") {
            if path.split("__").any(|part| part.is_empty()) {
                return Err("empty field path component".into());
            }
            fields.insert(path.to_string());
        }
    }
    let required = [
        "root_is_live",
        "exclusive_access",
        "disjoint_owned_fields",
        "owned_fields_are_disjoint_from_root",
        "free_resolves_to_libc",
    ];
    if required.iter().any(|p| !premises.contains(*p))
        || premises.len() != required.len() + fields.len()
        || fields.is_empty()
        || fields.len() > 8
    {
        return Err("missing/unsupported custody premises or field bound exceeded".into());
    }
    for field in &fields {
        if let Some((parent, _)) = field.rsplit_once("__")
            && !fields.contains(parent)
        {
            return Err("nested owned field requires a declared parent".into());
        }
    }
    for path in summary.paths() {
        validate_operations(&path.operations)?;
    }
    let fields: Vec<_> = fields.into_iter().collect();
    let mut cases = Vec::new();
    let mut postconditions_per_case = 0;
    for mask in 0..(1usize << fields.len()) {
        let live: BTreeSet<_> = fields
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, f)| f.clone())
            .collect();
        // A null parent has no independently live child storage.
        if live.iter().any(|f| {
            f.rsplit_once("__")
                .is_some_and(|(parent, _)| !live.contains(parent))
        }) {
            continue;
        }
        let mut selected = Vec::new();
        for (index, path) in summary.paths().iter().enumerate() {
            let mut state = Replay {
                fields: &fields,
                initial: &live,
                released: Vec::new(),
                symbols: BTreeMap::new(),
            };
            if state.run(&path.operations)? {
                selected.push((index, state.released));
            }
        }
        let [(source_path, released)] = selected.as_slice() else {
            return Err("nullable state requires exactly one complete source path".into());
        };
        let mut facts: BTreeMap<_, _> = premises.iter().map(|p| (p.clone(), 1)).collect();
        facts.insert("root_preserved".into(), 1);
        facts.insert("fields_unchanged".into(), 1);
        for field in &fields {
            let was_live = live.contains(field);
            let rank = released
                .iter()
                .position(|f| f == field)
                .map_or(0, |i| i + 1);
            facts.insert(format!("old_live__{field}"), i128::from(was_live));
            facts.insert(format!("live__{field}"), i128::from(was_live && rank == 0));
            facts.insert(format!("released__{field}"), i128::from(rank != 0));
            facts.insert(format!("release_rank__{field}"), rank as i128);
        }
        match check_contract_observation(source, summary.function(), &facts)? {
            ObservationVerdict::Checked { postconditions } => {
                postconditions_per_case = postconditions
            }
            ObservationVerdict::OutsidePreconditions { predicate } => {
                return Err(format!("unmet destructor premise: {predicate}"));
            }
        }
        // An accepted field destructor must discharge its entire declared set,
        // even if a supplied postcondition is weakened to `true`.
        if live.iter().any(|f| !released.contains(f)) {
            return Err("destructor retains a declared owned field".into());
        }
        cases.push(FieldReleaseCase {
            initially_live: live.iter().cloned().collect(),
            released_in_order: released.clone(),
            source_path: *source_path,
        });
    }
    Ok(FieldDestructorCertificate {
        source: summary.clone(),
        contract_source: source.into(),
        root_c_type: root_c_type.into(),
        root_qcore_type: root_qcore_type.into(),
        cases,
        postconditions_per_case,
    })
}

fn validate_value(value: &SourceValue) -> Result<(), String> {
    match &value.expression {
        Expr::Parameter { index: 0 } if value.c_type.ends_with('*') => Ok(()),
        Expr::Symbol { .. } => Ok(()),
        Expr::Integer { value } if value == "0" => Ok(()),
        Expr::Cast { cast, value: inner }
            if (matches!(cast.as_str(), "BitCast" | "NoOp")
                && value.c_type.ends_with('*')
                && inner.c_type.ends_with('*'))
                || (cast == "NullToPointer"
                    && value.c_type.ends_with('*')
                    && matches!(&inner.expression,Expr::Integer {value} if value == "0")) =>
        {
            validate_value(inner)
        }
        Expr::Field { base, .. } if value.c_type.ends_with('*') => validate_value(base),
        Expr::Unary { operator, value } if operator == "!" => validate_value(value),
        Expr::Binary {
            operator,
            left,
            right,
        } if matches!(operator.as_str(), "==" | "!=") => {
            validate_value(left)?;
            validate_value(right)
        }
        _ => Err("expression outside nullable field destructor profile".into()),
    }
}

fn validate_operations(ops: &[Op]) -> Result<(), String> {
    if !matches!(ops.last(), Some(Op::Return)) {
        return Err("destructor needs a normal unit return on every path".into());
    }
    for op in ops {
        match op {
            Op::Read { place } if matches!(&place.expression, Expr::Field { .. }) => {
                validate_value(place)?
            }
            Op::Guard { condition, .. } => validate_value(condition)?,
            Op::LocalWrite { value, .. } => validate_value(value)?,
            Op::Call {
                function,
                signature,
                arguments,
                result_type,
            } if function == "free"
                && signature == "void (void *)"
                && result_type == "void"
                && arguments.len() == 1 =>
            {
                validate_value(&arguments[0])?
            }
            Op::LocalEnd { .. } | Op::Return => (),
            _ => {
                return Err(format!(
                    "operation outside nullable field destructor profile: {op:?}"
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Root,
    Pointer(String),
    Null,
    Boolean(bool),
}
struct Replay<'a> {
    fields: &'a [String],
    initial: &'a BTreeSet<String>,
    released: Vec<String>,
    symbols: BTreeMap<usize, Value>,
}
impl Replay<'_> {
    fn truth(value: Value) -> bool {
        !matches!(value, Value::Null | Value::Boolean(false))
    }
    fn value(&self, value: &SourceValue) -> Result<Value, String> {
        match &value.expression {
            Expr::Parameter { index: 0 } => Ok(Value::Root),
            Expr::Symbol { operation } => self
                .symbols
                .get(operation)
                .cloned()
                .ok_or("missing destructor producer".into()),
            Expr::Integer { value } if value == "0" => Ok(Value::Null),
            Expr::Cast { value, .. } => self.value(value),
            Expr::Unary { operator, value } if operator == "!" => {
                Ok(Value::Boolean(!Self::truth(self.value(value)?)))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.value(left)?;
                let right = self.value(right)?;
                let same = left == right
                    || matches!(
                        (&left, &right),
                        (Value::Null, Value::Boolean(false)) | (Value::Boolean(false), Value::Null)
                    );
                Ok(Value::Boolean(if operator == "==" { same } else { !same }))
            }
            _ => Err("value outside destructor replay".into()),
        }
    }
    fn run(&mut self, operations: &[Op]) -> Result<bool, String> {
        for (index, op) in operations.iter().enumerate() {
            match op {
                Op::Read { place } => {
                    let Expr::Field { base, name } = &place.expression else {
                        return Err("nonfield destructor read".into());
                    };
                    let field = match self.value(base)? {
                        Value::Root => name.clone(),
                        Value::Pointer(parent) if !self.released.contains(&parent) => {
                            format!("{parent}__{name}")
                        }
                        _ => return Err("destructor reads null or released storage".into()),
                    };
                    if !self.fields.contains(&field) {
                        return Err(format!("unmodeled destructor field {field}"));
                    }
                    self.symbols.insert(
                        index,
                        if self.initial.contains(&field) {
                            Value::Pointer(field)
                        } else {
                            Value::Null
                        },
                    );
                }
                Op::Guard { condition, taken } => {
                    if Self::truth(self.value(condition)?) != *taken {
                        return Ok(false);
                    }
                }
                Op::Call { arguments, .. } => match self.value(&arguments[0])? {
                    Value::Null => (),
                    Value::Pointer(field) => {
                        if self.released.contains(&field) {
                            return Err("duplicate owned field release".into());
                        }
                        if self.initial.iter().any(|f| {
                            f.starts_with(&format!("{field}__")) && !self.released.contains(f)
                        }) {
                            return Err("parent release precedes a live owned child".into());
                        }
                        self.released.push(field);
                    }
                    _ => return Err("free lacks owned field provenance".into()),
                },
                Op::LocalWrite { value, .. } => {
                    self.value(value)?;
                }
                Op::LocalEnd { .. } => (),
                Op::Return => return Ok(true),
                _ => return Err("unsupported destructor operation".into()),
            }
        }
        Err("missing destructor return".into())
    }
}
