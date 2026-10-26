//! Instantiation syntax carries no proof or source authority.
use crate::hypothesis::ArtifactReference;
use crate::quant_proposition::{Formula, Parameter};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Application {
    pub id: String,
    pub parameters: Vec<Parameter>,
    pub reference: String,
    pub arguments: Vec<String>,
    pub prerequisites: Vec<Formula>,
    pub conclusion: Formula,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoremSchema {
    pub reference: String,
    pub parameters: Vec<Parameter>,
    pub prerequisites: Vec<Formula>,
    pub conclusion: Formula,
    pub model: ArtifactReference,
    pub audit: ArtifactReference,
}
