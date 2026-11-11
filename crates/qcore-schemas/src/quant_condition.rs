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

//! Explicit assertion position; declarations establish neither truth nor execution.
use crate::quant_proposition;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "error",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Selector {
    Entry,
    Normal,
    Error(String),
    Any,
}
impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entry => write!(f, "entry"),
            Self::Normal => write!(f, "normal"),
            Self::Any => write!(f, "any"),
            Self::Error(error) => write!(f, "error({error})"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub selector: Selector,
    pub proposition: quant_proposition::Definition,
}
