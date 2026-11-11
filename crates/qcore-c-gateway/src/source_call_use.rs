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

//! Def/use facts for nonvoid calls in complete flat source paths. These describe
//! source use of a result, not the call's success, failure, or resource effects.
use crate::typed_source::{
    SourceExpression as Expr, SourceOperation as Op, SourceSummary, SourceValue,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallUse {
    pub operation: usize,
    pub function: String,
    pub result_type: String,
    pub uses: Vec<usize>,
    pub guard_uses: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PathCallUse {
    pub path: usize,
    pub calls: Vec<CallUse>,
}
fn symbols(value: &SourceValue, out: &mut BTreeSet<usize>) {
    match &value.expression {
        Expr::Symbol { operation } => {
            out.insert(*operation);
        }
        Expr::Cast { value, .. } | Expr::Unary { value, .. } => symbols(value, out),
        Expr::Field { base, .. } => symbols(base, out),
        Expr::Index { base, index } => {
            symbols(base, out);
            symbols(index, out);
        }
        Expr::Binary { left, right, .. } => {
            symbols(left, out);
            symbols(right, out);
        }
        Expr::Aggregate { elements } => {
            for value in elements {
                symbols(value, out);
            }
        }
        _ => (),
    }
}
/// Alias substitution is already performed by the private source importer.
/// A use includes storing, passing, returning, computing with or guarding on the
/// produced value. Guard uses are direct symbolic dependencies; heap reloads are
/// not inferred to retain a stored result's identity. Structured loops fail closed.
pub fn analyze_source_call_uses(summary: &SourceSummary) -> Result<Vec<PathCallUse>, String> {
    summary.paths().iter().enumerate().map(|(path,source)|{
        let mut calls:Vec<CallUse>=Vec::new();
        for (index,op) in source.operations.iter().enumerate(){
            let mut used=BTreeSet::new();let guard=matches!(op,Op::Guard{..});
            match op{
                Op::Read{place}=>symbols(place,&mut used),
                Op::Write{place,value}=>{symbols(place,&mut used);symbols(value,&mut used);},
                Op::LocalWrite{value,..}|Op::ReturnValue{value}=>symbols(value,&mut used),
                Op::Guard{condition,..}=>symbols(condition,&mut used),
                Op::Call{arguments,..}=>for value in arguments{symbols(value,&mut used);},
                Op::LocalStart{..}|Op::LocalEnd{..}|Op::Return|Op::Terminate=>(),
                Op::ScanLoop{..}=>return Err("call-use analysis requires flat source paths; loop dependencies remain Unknown".into()),
            }
            for call in &mut calls{if used.contains(&call.operation){call.uses.push(index);if guard{call.guard_uses.push(index);}}}
            if let Op::Call{function,result_type,..}=op{calls.push(CallUse{operation:index,function:function.clone(),result_type:result_type.clone(),uses:Vec::new(),guard_uses:Vec::new()});}
        }
        Ok(PathCallUse{path,calls})
    }).collect()
}

#[cfg(test)]
#[path = "../tests/unit/source_call_use.rs"]
mod tests;
