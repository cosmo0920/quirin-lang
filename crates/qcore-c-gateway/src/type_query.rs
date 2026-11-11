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

//! Fixed scalar C type queries for the custody projection.
//!
//! C11 6.5.3.4 makes `sizeof` of a non-VLA operand and `_Alignof`
//! unevaluated. Admit only a readable scalar type name, or (for `sizeof`)
//! one declared scalar local. Arrays, pointer declarators, calls, compound
//! operands and GNU type operators remain outside this rule. In particular,
//! a typedef must resolve through the existing declaration reader: spelling
//! something inside `sizeof` does not by itself make it unevaluated.
//!
//! The replacement `1` is an operand-class marker, NEVER a computed size or
//! alignment. Consumers may only ask custody-neutrality predicates about the
//! resulting tokens, never use them to build a CFG or refine a branch.

use crate::typed_field_access::{FunctionTypes, TypedAccessModel};

pub(crate) fn scalar_operands<'a>(
    texts: &[&'a str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<Vec<&'a str>> {
    if !types.is_readable()
        || !texts
            .iter()
            .any(|text| matches!(*text, "sizeof" | "_Alignof"))
    {
        return None;
    }
    let mut result = Vec::new();
    let mut cursor = 0;
    let mut found = false;
    while cursor < texts.len() {
        let operator = texts[cursor];
        if !matches!(operator, "sizeof" | "_Alignof") {
            result.push(operator);
            cursor += 1;
            continue;
        }
        let start = cursor + 1;
        let (operand, end) = if texts.get(start) == Some(&"(") {
            // No admitted operand contains parentheses itself. Stopping at
            // the first ')' is consequently total-or-reject, not truncation.
            let close = start + texts[start..].iter().position(|text| *text == ")")?;
            (&texts[start + 1..close], close + 1)
        } else if operator == "sizeof" {
            (texts.get(start..start + 1)?, start + 1)
        } else {
            return None;
        };
        let type_name = texts.get(start) == Some(&"(") && model.scalar_type_name(operand);
        let scalar_name =
            operator == "sizeof" && matches!(operand, [name] if types.declared_is_scalar(name));
        if !type_name && !scalar_name {
            return None;
        }
        result.push("1");
        cursor = end;
        found = true;
    }
    found.then_some(result)
}
