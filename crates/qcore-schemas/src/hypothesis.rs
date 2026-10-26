//! Versioned evidence-only contract hypothesis graphs. No wire claim grants authority.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Proposition {
    Opaque {
        statement: String,
    },
    Typed {
        subject: String,
        predicate: String,
    },
    Formula {
        definition: crate::quant_proposition::Definition,
    },
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
/// Proposed decomposition of one existing legacy proposition. This is not an
/// equivalence or implication edge and never changes that proposition's truth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refinement {
    pub premise: String,
    pub children: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub atoms: BTreeMap<String, Atom>,
    pub premises: BTreeMap<String, Conjunction>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refinements: BTreeMap<String, Refinement>,
    /// Standalone formal entry obligations of a declared model. Interpreted
    /// resource/bounds summaries refuse these until an actual valuation is checked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,
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

/// Required content-addressed input, declared by retained contract bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingRequirement {
    pub source_module: String,
    pub declaration: String,
    pub premise: String,
    pub manifest_sha256: String,
    pub graph_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentDefinition {
    pub id: String,
    pub variables: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBinding {
    pub atom_sha256: String,
    pub proposition: String,
    pub model: ArtifactReference,
    pub audit: ArtifactReference,
    pub environment: String,
    pub quantifiers: Vec<String>,
    pub prerequisites: Vec<String>,
    pub conclusion: String,
}
/// Declared formal definitions and environment obligations, not a source proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingManifest {
    pub version: u32,
    pub declaration: String,
    pub premise: String,
    pub graph_sha256: String,
    pub source_binding: String,
    pub environment: EnvironmentDefinition,
    pub atoms: BTreeMap<String, FormalBinding>,
}
