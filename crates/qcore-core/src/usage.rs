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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The upper endpoint of a usage interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageUpper {
    Finite(u64),
    Infinity,
}

impl UsageUpper {
    fn add(self, other: Self) -> Result<Self, UsageArithmeticError> {
        match (self, other) {
            (Self::Finite(left), Self::Finite(right)) => left
                .checked_add(right)
                .map(Self::Finite)
                .ok_or(UsageArithmeticError::Overflow),
            _ => Ok(Self::Infinity),
        }
    }

    fn multiply(self, other: Self) -> Result<Self, UsageArithmeticError> {
        match (self, other) {
            (Self::Finite(left), Self::Finite(right)) => left
                .checked_mul(right)
                .map(Self::Finite)
                .ok_or(UsageArithmeticError::Overflow),
            (Self::Finite(0), Self::Infinity) | (Self::Infinity, Self::Finite(0)) => {
                Ok(Self::Finite(0))
            }
            _ => Ok(Self::Infinity),
        }
    }

    fn max(self, other: Self) -> Self {
        match (self, other) {
            (Self::Finite(left), Self::Finite(right)) => Self::Finite(left.max(right)),
            _ => Self::Infinity,
        }
    }

    fn contains(self, other: Self) -> bool {
        match (self, other) {
            (Self::Infinity, _) => true,
            (Self::Finite(_), Self::Infinity) => false,
            (Self::Finite(outer), Self::Finite(inner)) => inner <= outer,
        }
    }
}

/// A closed interval `[lower, upper]` of possible runtime demands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Usage {
    lower: u64,
    upper: UsageUpper,
}

impl Usage {
    pub const ZERO: Self = Self::finite(0, 0);
    pub const ONE: Self = Self::finite(1, 1);
    pub const AFFINE: Self = Self::finite(0, 1);
    pub const RELEVANT: Self = Self {
        lower: 1,
        upper: UsageUpper::Infinity,
    };
    pub const MANY: Self = Self {
        lower: 0,
        upper: UsageUpper::Infinity,
    };

    pub const fn finite(lower: u64, upper: u64) -> Self {
        assert!(lower <= upper, "invalid usage interval");
        Self {
            lower,
            upper: UsageUpper::Finite(upper),
        }
    }

    pub const fn unbounded(lower: u64) -> Self {
        Self {
            lower,
            upper: UsageUpper::Infinity,
        }
    }

    pub const fn lower(self) -> u64 {
        self.lower
    }

    pub const fn upper(self) -> UsageUpper {
        self.upper
    }

    pub fn checked_add(self, other: Self) -> Result<Self, UsageArithmeticError> {
        Ok(Self {
            lower: self
                .lower
                .checked_add(other.lower)
                .ok_or(UsageArithmeticError::Overflow)?,
            upper: self.upper.add(other.upper)?,
        })
    }

    pub fn checked_multiply(self, other: Self) -> Result<Self, UsageArithmeticError> {
        Ok(Self {
            lower: self
                .lower
                .checked_mul(other.lower)
                .ok_or(UsageArithmeticError::Overflow)?,
            upper: self.upper.multiply(other.upper)?,
        })
    }

    /// Least enclosing interval, used at mutually exclusive control-flow joins.
    pub fn join(self, other: Self) -> Self {
        Self {
            lower: self.lower.min(other.lower),
            upper: self.upper.max(other.upper),
        }
    }

    /// Whether every demand admitted by `actual` is admitted by `self`.
    pub fn contains(self, actual: Self) -> bool {
        self.lower <= actual.lower && self.upper.contains(actual.upper)
    }

    fn is_definitely_zero(self) -> bool {
        self == Self::ZERO
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.upper {
            UsageUpper::Finite(upper) => write!(f, "[{},{}]", self.lower, upper),
            UsageUpper::Infinity => write!(f, "[{},inf]", self.lower),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageArithmeticError {
    Overflow,
}

impl fmt::Display for UsageArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("usage arithmetic overflow")
    }
}

impl std::error::Error for UsageArithmeticError {}

/// Runtime demand for each free variable in an expression.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DemandContext(BTreeMap<String, Usage>);

impl DemandContext {
    pub fn singleton(name: impl Into<String>) -> Self {
        Self(BTreeMap::from([(name.into(), Usage::ONE)]))
    }

    pub fn get(&self, name: &str) -> Usage {
        self.0.get(name).copied().unwrap_or(Usage::ZERO)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Usage)> {
        self.0.iter().map(|(name, usage)| (name.as_str(), *usage))
    }

    pub fn add(&self, other: &Self) -> Result<Self, UsageArithmeticError> {
        self.combine(other, Usage::checked_add)
    }

    pub fn scale(&self, factor: Usage) -> Result<Self, UsageArithmeticError> {
        let mut result = BTreeMap::new();
        for (name, usage) in &self.0 {
            let scaled = factor.checked_multiply(*usage)?;
            if !scaled.is_definitely_zero() {
                result.insert(name.clone(), scaled);
            }
        }
        Ok(Self(result))
    }

    pub fn join(&self, other: &Self) -> Self {
        let names: BTreeSet<_> = self.0.keys().chain(other.0.keys()).collect();
        let demands = names
            .into_iter()
            .filter_map(|name| {
                let joined = self.get(name).join(other.get(name));
                (!joined.is_definitely_zero()).then(|| (name.clone(), joined))
            })
            .collect();
        Self(demands)
    }

    fn combine(
        &self,
        other: &Self,
        operation: fn(Usage, Usage) -> Result<Usage, UsageArithmeticError>,
    ) -> Result<Self, UsageArithmeticError> {
        let names: BTreeSet<_> = self.0.keys().chain(other.0.keys()).collect();
        let mut demands = BTreeMap::new();
        for name in names {
            let combined = operation(self.get(name), other.get(name))?;
            if !combined.is_definitely_zero() {
                demands.insert(name.clone(), combined);
            }
        }
        Ok(Self(demands))
    }
}

/// Reduced demand-analysis core, including per-invocation lambda checking.
/// Application grades must come from a separately checked function contract.
/// This core does not establish ordinary types or operational evaluation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageExpr {
    Unit,
    Variable(String),
    Sequence(Vec<UsageExpr>),
    Branch {
        condition: Box<UsageExpr>,
        then_expression: Box<UsageExpr>,
        else_expression: Box<UsageExpr>,
    },
    Apply {
        function: Box<UsageExpr>,
        parameter_usages: Vec<Usage>,
        arguments: Vec<UsageExpr>,
    },
    /// Check one invocation before any surrounding demand scales the captures.
    /// This is a demand-analysis node, not an ordinary/dependent type checker.
    Lambda {
        parameter: String,
        parameter_usage: Usage,
        body: Box<UsageExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DemandError {
    Arithmetic(UsageArithmeticError),
    ApplicationArity {
        expected: usize,
        actual: usize,
    },
    ParameterUsage {
        name: String,
        declared: Usage,
        actual: Usage,
    },
}

impl fmt::Display for DemandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arithmetic(error) => error.fmt(f),
            Self::ApplicationArity { expected, actual } => {
                write!(
                    f,
                    "application expects {expected} arguments but received {actual}"
                )
            }
            Self::ParameterUsage {
                name,
                declared,
                actual,
            } => write!(
                f,
                "parameter `{name}` declares usage {declared}, but one invocation demands {actual}"
            ),
        }
    }
}

impl std::error::Error for DemandError {}

impl From<UsageArithmeticError> for DemandError {
    fn from(error: UsageArithmeticError) -> Self {
        Self::Arithmetic(error)
    }
}

impl UsageExpr {
    /// Scale free-variable demand only after checking every local binder.
    /// Even demand zero must not hide an invalid lambda body.
    pub fn synthesize_demands_at(&self, quantity: Usage) -> Result<DemandContext, DemandError> {
        Ok(self.synthesize_demands()?.scale(quantity)?)
    }

    pub fn synthesize_demands(&self) -> Result<DemandContext, DemandError> {
        match self {
            Self::Unit => Ok(DemandContext::default()),
            Self::Variable(name) => Ok(DemandContext::singleton(name)),
            Self::Lambda {
                parameter,
                parameter_usage,
                body,
            } => {
                let mut demands = body.synthesize_demands()?;
                let actual = demands.get(parameter);
                if !parameter_usage.contains(actual) {
                    return Err(DemandError::ParameterUsage {
                        name: parameter.clone(),
                        declared: *parameter_usage,
                        actual,
                    });
                }
                demands.0.remove(parameter);
                Ok(demands)
            }
            Self::Sequence(expressions) => expressions
                .iter()
                .try_fold(DemandContext::default(), |demands, expression| {
                    Ok(demands.add(&expression.synthesize_demands()?)?)
                }),
            Self::Branch {
                condition,
                then_expression,
                else_expression,
            } => {
                let condition = condition.synthesize_demands()?;
                let branches = then_expression
                    .synthesize_demands()?
                    .join(&else_expression.synthesize_demands()?);
                Ok(condition.add(&branches)?)
            }
            Self::Apply {
                function,
                parameter_usages,
                arguments,
            } => {
                if parameter_usages.len() != arguments.len() {
                    return Err(DemandError::ApplicationArity {
                        expected: parameter_usages.len(),
                        actual: arguments.len(),
                    });
                }
                let mut demands = function.synthesize_demands()?;
                for (parameter_usage, argument) in parameter_usages.iter().zip(arguments.iter()) {
                    demands =
                        demands.add(&argument.synthesize_demands()?.scale(*parameter_usage)?)?;
                }
                Ok(demands)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageViolation {
    UnknownVariable {
        name: String,
    },
    ErasedVariableAtRuntime {
        name: String,
        actual: Usage,
    },
    ExactOneNotConsumed {
        name: String,
        actual: Usage,
    },
    ExactOneConsumedMoreThanOnce {
        name: String,
        actual: Usage,
    },
    DeclaredUsageExceeded {
        name: String,
        declared: Usage,
        actual: Usage,
    },
}

impl fmt::Display for UsageViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVariable { name } => write!(f, "unknown usage variable `{name}`"),
            Self::ErasedVariableAtRuntime { name, actual } => {
                write!(f, "erased variable `{name}` has runtime demand {actual}")
            }
            Self::ExactOneNotConsumed { name, actual }
            | Self::ExactOneConsumedMoreThanOnce { name, actual } => write!(
                f,
                "variable `{name}` requires usage {}, but has demand {actual}",
                Usage::ONE
            ),
            Self::DeclaredUsageExceeded {
                name,
                declared,
                actual,
            } => write!(
                f,
                "variable `{name}` declares usage {declared}, but has demand {actual}"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageCheckError {
    Demand(DemandError),
    Violations(Vec<UsageViolation>),
}

impl fmt::Display for UsageCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Demand(error) => error.fmt(f),
            Self::Violations(violations) => {
                for (index, violation) in violations.iter().enumerate() {
                    if index > 0 {
                        f.write_str("; ")?;
                    }
                    violation.fmt(f)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for UsageCheckError {}

/// Checks synthesized runtime demand against the declared grades.
pub fn check_usage(
    declarations: &BTreeMap<String, Usage>,
    expression: &UsageExpr,
) -> Result<DemandContext, UsageCheckError> {
    let demands = expression
        .synthesize_demands()
        .map_err(UsageCheckError::Demand)?;
    let mut violations = Vec::new();

    for (name, actual) in demands.iter() {
        if !declarations.contains_key(name) {
            violations.push(UsageViolation::UnknownVariable {
                name: name.to_owned(),
            });
        }
        if declarations.get(name) == Some(&Usage::ZERO) {
            violations.push(UsageViolation::ErasedVariableAtRuntime {
                name: name.to_owned(),
                actual,
            });
        }
    }

    for (name, declared) in declarations {
        let actual = demands.get(name);
        if *declared == Usage::ONE {
            if actual.lower() < 1 {
                violations.push(UsageViolation::ExactOneNotConsumed {
                    name: name.clone(),
                    actual,
                });
            }
            if !matches!(actual.upper(), UsageUpper::Finite(upper) if upper <= 1) {
                violations.push(UsageViolation::ExactOneConsumedMoreThanOnce {
                    name: name.clone(),
                    actual,
                });
            }
        } else if *declared != Usage::ZERO && !declared.contains(actual) {
            violations.push(UsageViolation::DeclaredUsageExceeded {
                name: name.clone(),
                declared: *declared,
                actual,
            });
        }
    }

    if violations.is_empty() {
        Ok(demands)
    } else {
        Err(UsageCheckError::Violations(violations))
    }
}

#[cfg(test)]
#[path = "../tests/unit/usage.rs"]
mod tests;
