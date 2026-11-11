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

//! Typed entry/exit assertions. No predicate is evaluated or source-bound here.
use qcore_schemas::quant_condition::{Definition, Selector};
fn selector_valid(selector: &Selector) -> bool {
    match selector {
        Selector::Error(name) => {
            !name.trim().is_empty() && name.len() <= 1024 && !name.chars().any(char::is_control)
        }
        _ => true,
    }
}
/// Parse the expression after the explicit selector, excluding the semicolon.
pub fn parse(selector: Selector, expression: &str) -> Result<Definition, String> {
    if !selector_valid(&selector) {
        return Err("quant-condition: invalid error selector".into());
    }
    let proposition = crate::quant_proposition::parse(expression)?;
    Ok(Definition {
        selector,
        proposition,
    })
}
pub fn validate(definition: &Definition) -> Result<(), String> {
    if !selector_valid(&definition.selector) {
        return Err("quant-condition: invalid error selector".into());
    }
    crate::quant_proposition::validate(&definition.proposition)
}
