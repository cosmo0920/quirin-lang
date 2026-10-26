//! Conditional own-operation relations, not installed summaries or evidence.
use crate::quant_proposition::{Formula, Parameter, Term};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub parameters: Vec<Parameter>,
    pub entry: String,
    pub exit: String,
    pub guard: Formula,
    pub reads: Vec<Term>,
    pub writes: Vec<Term>,
    pub frame: Formula,
    pub transition: Formula,
}
