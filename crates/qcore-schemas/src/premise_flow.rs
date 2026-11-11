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

//! Separate versioned proposed flow records. Never a source/certificate claim.
use crate::hypothesis::{
    Context, Disposition, EnvironmentDefinition, FormalBinding, Instance, Warrant,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Expr {
    Atom { atom: String },
    Flow { flow: String },
    And { left: Box<Expr>, right: Box<Expr> },
    Or { left: Box<Expr>, right: Box<Expr> },
    Except { left: Box<Expr>, right: Box<Expr> },
    Terminal { left: Box<Expr>, right: Box<Expr> },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum View {
    Resource,
    Bounds,
    Model,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub environment: EnvironmentDefinition,
    pub atoms: BTreeMap<String, FormalBinding>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub view: View,
    pub source_module: String,
    pub declaration: String,
    pub source_sha256: String,
    pub graph_sha256: String,
    pub model_sha256: String,
    pub entries: BTreeMap<String, Expr>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowInstance {
    pub document: Document,
    pub document_sha256: String,
    pub hypotheses: Instance,
}
/// Evaluated logical assignments only. Source adapters must independently replay
/// their own sealed scoped receipts before wrapping a bounded source result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedEvaluation {
    pub entry: String,
    pub disposition: Disposition,
    pub terminated: bool,
    pub atom_dependencies: Vec<String>,
    pub warrants: BTreeMap<String, Warrant>,
    pub context: Context,
}
