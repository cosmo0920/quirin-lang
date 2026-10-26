//! First-order hypothesis syntax. These records carry no truth or proof authority.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Sort {
    Value { name: String },
    Predicate { arguments: Vec<String> },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub sort: Sort,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Formula {
    Predicate {
        name: String,
        arguments: Vec<String>,
    },
    Equal {
        left: String,
        right: String,
    },
    Not {
        body: Box<Formula>,
    },
    And {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Or {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Implies {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Forall {
        variable: String,
        sort: String,
        body: Box<Formula>,
    },
    Exists {
        variable: String,
        sort: String,
        body: Box<Formula>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub parameters: Vec<Parameter>,
    pub formula: Formula,
}
