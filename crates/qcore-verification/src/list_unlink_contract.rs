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

//! Complete source replay for a bounded intrusive-list unlink. The two neighbor
//! alias cases form a pointer-identity quotient; entry is distinct from its live
//! neighbors. No arbitrary addresses, integer arithmetic or opaque calls are
//! admitted. Ring membership, exclusive access and C/QCore ABI binding remain
//! caller premises. Unlinking does not release the containing allocation.
use qcore_c_gateway::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use qcore_frontend::frontend::{DeclarationKind, SourceContractClauseKind, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnlinkCase {
    pub neighbors_alias: bool,
    pub helper_calls: usize,
    pub writes: Vec<(u32, String, Option<u32>)>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ListUnlinkCertificate {
    source: SourceSummary,
    helper_source: SourceSummary,
    contract_source: String,
    c_type: String,
    resource_type: String,
    cases: Vec<UnlinkCase>,
}
impl ListUnlinkCertificate {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn helper_source(&self) -> &SourceSummary {
        &self.helper_source
    }
    pub fn cases(&self) -> &[UnlinkCase] {
        &self.cases
    }
}
fn null_literal(value: &SourceValue) -> bool {
    match &value.expression {
        Expr::Integer { value } => value == "0",
        Expr::Cast { cast, value: inner }
            if cast == "NullToPointer" && value.c_type.ends_with('*') =>
        {
            null_literal(inner)
        }
        _ => false,
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
fn check_contract(
    source: &str,
    function: &str,
    resource: &str,
    helper: bool,
    facts: &[(&str, i128)],
) -> Result<(), String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == function)
        .collect();
    let [decl] = declarations.as_slice() else {
        return Err("expected one list contract declaration".into());
    };
    let decl = &decl.value;
    if decl.kind != DeclarationKind::ExternFunction
        || decl.parameters.len() != if helper { 2 } else { 1 }
    {
        return Err("list contract parameter count mismatch".into());
    }
    for parameter in &decl.parameters {
        let actual = tokens(&parameter.type_source)?;
        if helper {
            if actual != tokens(&format!("ptr<mut, {resource}>"))? || parameter.usage.is_some() {
                return Err("neighbor helper requires raw mutable pointers; aliasing is permitted by its cell premises".into());
            }
        } else {
            let expected = tokens(&format!("borrow<'call, mut, {resource}>"))?;
            if actual.len() != expected.len()
                || actual
                    .iter()
                    .zip(&expected)
                    .enumerate()
                    .any(|(i, (a, b))| i != 3 && a != b)
                || !matches!(actual.get(3), Some(TokenKind::Word(_)))
                || parameter.usage.as_ref().map(|u| u.value) != Some(qcore_core::usage::Usage::ONE)
            {
                return Err("unlink requires the bound mutable root borrow with use one".into());
            }
        }
    }
    let arrow = decl
        .tokens
        .iter()
        .position(|t| *t == TokenKind::Symbol(Symbol::Arrow))
        .ok_or("missing list result")?;
    if decl.tokens.get(arrow + 1) != Some(&TokenKind::Word("unit".into()))
        || decl.tokens.get(arrow + 2) != Some(&TokenKind::Word("contract".into()))
    {
        return Err("list contract requires unit without extra effects".into());
    }
    let mut premises = BTreeSet::new();
    for clause in &decl
        .function_contract
        .as_ref()
        .ok_or("missing list clauses")?
        .clauses
    {
        if clause.value.kind == SourceContractClauseKind::Requires {
            let ts = tokens(&clause.value.expression_source)?;
            let [TokenKind::Word(name)] = ts.as_slice() else {
                return Err("unsupported list premise".into());
            };
            if !premises.insert(name.clone()) {
                return Err("duplicate list premise".into());
            }
        }
    }
    let required: &[&str] = if helper {
        &["neighbor_link_cells_are_live", "exclusive_link_access"]
    } else {
        &[
            "entry_is_live",
            "exclusive_link_access",
            "entry_has_live_reciprocal_neighbors",
            "entry_is_distinct_from_neighbors",
        ]
    };
    if premises != required.iter().map(|p| p.to_string()).collect() {
        return Err("list contract lacks exact live/alias/access premises".into());
    }
    let mut values: Vec<_> = premises.iter().map(|p| (p.as_str(), 1)).collect();
    values.extend_from_slice(facts);
    qcore_contract::contract_observation::check_contract_facts(source, function, &values)
}

pub fn check_list_unlink(
    source: &SourceSummary,
    helper: &SourceSummary,
    contract: &str,
    c_type: &str,
    resource: &str,
) -> Result<ListUnlinkCertificate, String> {
    if source.signature() != format!("void ({c_type})")
        || helper.signature() != format!("void ({c_type}, {c_type})")
        || source.source_revision() != helper.source_revision()
        || source.function() == helper.function()
    {
        return Err("list source/helper signature, identity or revision mismatch".into());
    }
    let [outer] = source.paths() else {
        return Err("unlink requires one complete path".into());
    };
    let [inner] = helper.paths() else {
        return Err("list helper requires one complete path".into());
    };
    let mut cases = Vec::new();
    for neighbors_alias in [false, true] {
        let previous = 2;
        let next = if neighbors_alias { 2 } else { 3 };
        let initial = BTreeMap::from([
            ((1, "prev".into()), Some(previous)),
            ((1, "next".into()), Some(next)),
            ((previous, "next".into()), Some(1)),
            ((next, "prev".into()), Some(1)),
        ]);
        let mut state = Replay {
            memory: initial.clone(),
            writes: Vec::new(),
            helper_calls: 0,
            c_type,
        };
        // Independently check the complete helper body at the two alias classes.
        state.run(&inner.operations, &[Some(previous), Some(next)], None)?;
        let helper_frame = state.writes.iter().all(|(id, field, _)| {
            (*id == previous && field == "next") || (*id == next && field == "prev")
        });
        check_contract(
            contract,
            helper.function(),
            resource,
            true,
            &[
                (
                    "previous_next_is_next",
                    i128::from(state.memory.get(&(previous, "next".into())) == Some(&Some(next))),
                ),
                (
                    "next_prev_is_previous",
                    i128::from(state.memory.get(&(next, "prev".into())) == Some(&Some(previous))),
                ),
                ("only_neighbor_link_cells_written", i128::from(helper_frame)),
                ("write_count", state.writes.len() as i128),
            ],
        )?;
        // Require the shape needed for compositional frame reasoning even if
        // supplied postconditions are weakened. No missing effects are assumed.
        if !helper_frame || state.writes.len() != 2 {
            return Err("helper footprint is not the two neighbor link writes".into());
        }
        state.memory = initial;
        state.writes.clear();
        state.run(&outer.operations, &[Some(1)], Some(helper))?;
        let frame = state.writes.iter().all(|(id, field, _)| {
            (*id == 1 && matches!(field.as_str(), "prev" | "next"))
                || (*id == previous && field == "next")
                || (*id == next && field == "prev")
        });
        check_contract(
            contract,
            source.function(),
            resource,
            false,
            &[
                (
                    "entry_prev_is_null",
                    i128::from(state.memory.get(&(1, "prev".into())) == Some(&None)),
                ),
                (
                    "entry_next_is_null",
                    i128::from(state.memory.get(&(1, "next".into())) == Some(&None)),
                ),
                (
                    "previous_next_is_next",
                    i128::from(state.memory.get(&(previous, "next".into())) == Some(&Some(next))),
                ),
                (
                    "next_prev_is_previous",
                    i128::from(state.memory.get(&(next, "prev".into())) == Some(&Some(previous))),
                ),
                ("only_unlink_cells_written", i128::from(frame)),
                ("helper_calls", state.helper_calls as i128),
                ("write_count", state.writes.len() as i128),
                ("no_object_release", 1),
            ],
        )?;
        if !frame || state.helper_calls != 1 || state.writes.len() != 4 {
            return Err("unlink footprint/call count differs from the bounded profile".into());
        }
        cases.push(UnlinkCase {
            neighbors_alias,
            helper_calls: state.helper_calls,
            writes: state.writes,
        });
    }
    Ok(ListUnlinkCertificate {
        source: source.clone(),
        helper_source: helper.clone(),
        contract_source: contract.into(),
        c_type: c_type.into(),
        resource_type: resource.into(),
        cases,
    })
}

struct Replay<'a> {
    memory: BTreeMap<(u32, String), Option<u32>>,
    writes: Vec<(u32, String, Option<u32>)>,
    helper_calls: usize,
    c_type: &'a str,
}
impl Replay<'_> {
    fn value(
        &self,
        value: &SourceValue,
        parameters: &[Option<u32>],
        symbols: &BTreeMap<usize, Option<u32>>,
    ) -> Result<Option<u32>, String> {
        match &value.expression {
            Expr::Parameter { index } if value.c_type == self.c_type => parameters
                .get(*index)
                .copied()
                .ok_or("missing list parameter".into()),
            Expr::Symbol { operation } if value.c_type == self.c_type => symbols
                .get(operation)
                .copied()
                .ok_or("missing list producer".into()),
            Expr::Cast { cast, value: inner }
                if matches!(cast.as_str(), "BitCast" | "NoOp")
                    && value.c_type.ends_with('*')
                    && inner.c_type.ends_with('*') =>
            {
                self.value(inner, parameters, symbols)
            }
            Expr::Cast { cast, value: inner }
                if cast == "NullToPointer"
                    && value.c_type.ends_with('*')
                    && null_literal(inner) =>
            {
                Ok(None)
            }
            _ => Err(format!("unmodeled list pointer expression: {value:?}")),
        }
    }
    fn place(
        &self,
        place: &SourceValue,
        parameters: &[Option<u32>],
        symbols: &BTreeMap<usize, Option<u32>>,
    ) -> Result<(u32, String), String> {
        let Expr::Field { base, name } = &place.expression else {
            return Err("list access is not a direct field".into());
        };
        if place.c_type != self.c_type || !matches!(name.as_str(), "prev" | "next") {
            return Err("list field/type outside link profile".into());
        }
        let id = self
            .value(base, parameters, symbols)?
            .ok_or("list access through null")?;
        let cell = (id, name.clone());
        if !self.memory.contains_key(&cell) {
            return Err("list cell has no supplied live binding".into());
        }
        Ok(cell)
    }
    fn run(
        &mut self,
        ops: &[Op],
        parameters: &[Option<u32>],
        helper: Option<&SourceSummary>,
    ) -> Result<(), String> {
        let mut symbols = BTreeMap::new();
        let Some((Op::Return, body)) = ops.split_last() else {
            return Err("list path must end with unit return".into());
        };
        for (index, op) in body.iter().enumerate() {
            match op {
                Op::Read { place } => {
                    let cell = self.place(place, parameters, &symbols)?;
                    symbols.insert(index, self.memory[&cell]);
                }
                Op::Write { place, value } => {
                    let cell = self.place(place, parameters, &symbols)?;
                    let value = self.value(value, parameters, &symbols)?;
                    self.memory.insert(cell.clone(), value);
                    self.writes.push((cell.0, cell.1, value));
                }
                Op::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    let helper = helper.ok_or("opaque or recursive list helper call")?;
                    if function != helper.function()
                        || signature != helper.signature()
                        || result_type != "void"
                        || arguments.len() != 2
                    {
                        return Err("list dependency call does not match the checked helper".into());
                    }
                    let args = arguments
                        .iter()
                        .map(|arg| self.value(arg, parameters, &symbols))
                        .collect::<Result<Vec<_>, _>>()?;
                    // The source call must use the entry's original neighbors,
                    // not another available node or a swapped argument binding.
                    if parameters != [Some(1)]
                        || args
                            != [
                                self.memory[&(1, "prev".into())],
                                self.memory[&(1, "next".into())],
                            ]
                    {
                        return Err("list helper arguments lack entry-neighbor binding".into());
                    }
                    self.helper_calls += 1;
                    self.run(&helper.paths()[0].operations, &args, None)?;
                }
                Op::LocalWrite { value, .. } => {
                    self.value(value, parameters, &symbols)?;
                }
                Op::LocalEnd { .. } => (),
                _ => return Err("unsupported effect in list source path".into()),
            }
        }
        Ok(())
    }
}
