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
