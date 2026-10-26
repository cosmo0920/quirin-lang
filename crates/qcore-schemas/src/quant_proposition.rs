//! First-order hypothesis syntax. These records carry no truth or proof authority.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Sort {
    Value {
        name: String,
    },
    Predicate {
        arguments: Vec<String>,
    },
    Function {
        arguments: Vec<String>,
        result: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub sort: Sort,
}
/// Mathematical terms only: integer operations have no machine overflow semantics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Term {
    Variable {
        name: String,
    },
    Integer {
        decimal: String,
    },
    Apply {
        function: String,
        arguments: Vec<Term>,
    },
    Add {
        left: Box<Term>,
        right: Box<Term>,
    },
    Sub {
        left: Box<Term>,
        right: Box<Term>,
    },
    Mul {
        left: Box<Term>,
        right: Box<Term>,
    },
    Neg {
        term: Box<Term>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Comparison {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Formula {
    Predicate {
        name: String,
        arguments: Vec<Term>,
    },
    Equal {
        left: Term,
        right: Term,
    },
    Compare {
        operator: Comparison,
        left: Term,
        right: Term,
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
