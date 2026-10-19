//! Versioned evidence-only contract hypothesis graphs. No wire claim grants authority.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Proposition {
    Opaque { statement: String },
    Typed { subject: String, predicate: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Atom {
    pub id: String,
    pub proposition: Proposition,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErasedPremise {
    pub name: String,
    pub statement: String,
    pub origin: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conjunction {
    pub id: String,
    pub children: Vec<String>,
    pub erasure: ErasedPremise,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub atoms: BTreeMap<String, Atom>,
    pub premises: BTreeMap<String, Conjunction>,
}
/// Exact caller-held verdict context, never inferred from the presented evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: String,
    pub assumptions: String,
    pub source_identity: String,
    pub build_identity: String,
    pub evidence_version: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    Open,
    Supported,
    Refuted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Warrant {
    Derived,
    Observed,
    Declared,
    HumanReviewed,
    ProviderProposed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub atom: String,
    pub atom_sha256: String,
    pub context: Context,
    pub disposition: Disposition,
    pub warrant: Warrant,
    pub reference: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub graph: Graph,
    pub graph_sha256: String,
    pub context: Context,
    pub evidence: Vec<Evidence>,
}
