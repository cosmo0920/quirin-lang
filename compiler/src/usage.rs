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

/// Reduced expression core for the cycle-2 usage pass.
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DemandError {
    Arithmetic(UsageArithmeticError),
    ApplicationArity { expected: usize, actual: usize },
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
    pub fn synthesize_demands(&self) -> Result<DemandContext, DemandError> {
        match self {
            Self::Unit => Ok(DemandContext::default()),
            Self::Variable(name) => Ok(DemandContext::singleton(name)),
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
                write!(f, "{} usage violation(s)", violations.len())
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
mod tests {
    use super::*;

    fn variable(name: &str) -> UsageExpr {
        UsageExpr::Variable(name.into())
    }

    fn declarations(entries: &[(&str, Usage)]) -> BTreeMap<String, Usage> {
        entries
            .iter()
            .map(|(name, usage)| ((*name).into(), *usage))
            .collect()
    }

    fn violations(error: UsageCheckError) -> Vec<UsageViolation> {
        match error {
            UsageCheckError::Violations(violations) => violations,
            UsageCheckError::Demand(error) => panic!("unexpected demand error: {error}"),
        }
    }

    #[test]
    fn interval_operations_follow_the_usage_semiring() {
        assert_eq!(Usage::ONE.checked_add(Usage::ONE), Ok(Usage::finite(2, 2)));
        assert_eq!(Usage::ZERO.checked_multiply(Usage::MANY), Ok(Usage::ZERO));
        assert_eq!(
            Usage::AFFINE.checked_multiply(Usage::finite(2, 2)),
            Ok(Usage::finite(0, 2))
        );
        assert_eq!(
            Usage::finite(u64::MAX, u64::MAX).checked_add(Usage::ONE),
            Err(UsageArithmeticError::Overflow)
        );
    }

    #[test]
    fn exact_one_rejects_loss_and_duplication() {
        let declared = declarations(&[("x", Usage::ONE)]);
        assert!(matches!(
            &violations(check_usage(&declared, &UsageExpr::Unit).unwrap_err())[..],
            [UsageViolation::ExactOneNotConsumed { .. }]
        ));
        let twice = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
        assert!(matches!(
            &violations(check_usage(&declared, &twice).unwrap_err())[..],
            [UsageViolation::ExactOneConsumedMoreThanOnce { .. }]
        ));
        assert!(check_usage(&declared, &variable("x")).is_ok());
    }

    #[test]
    fn affine_allows_loss_but_rejects_duplication() {
        let declared = declarations(&[("x", Usage::AFFINE)]);
        assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
        assert!(check_usage(&declared, &variable("x")).is_ok());
        let twice = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
        assert!(matches!(
            &violations(check_usage(&declared, &twice).unwrap_err())[..],
            [UsageViolation::DeclaredUsageExceeded { .. }]
        ));
    }

    #[test]
    fn many_allows_weakening_and_contraction() {
        let declared = declarations(&[("x", Usage::MANY)]);
        assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
        let repeated = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
        assert!(check_usage(&declared, &repeated).is_ok());
    }

    #[test]
    fn relevant_forbids_weakening_but_allows_contraction() {
        let declared = declarations(&[("x", Usage::RELEVANT)]);
        assert!(matches!(
            &violations(check_usage(&declared, &UsageExpr::Unit).unwrap_err())[..],
            [UsageViolation::DeclaredUsageExceeded { .. }]
        ));
        let repeated = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
        assert!(check_usage(&declared, &repeated).is_ok());
    }

    #[test]
    fn erased_variable_cannot_occur_at_runtime() {
        let declared = declarations(&[("proof", Usage::ZERO)]);
        assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
        assert!(matches!(
            &violations(check_usage(&declared, &variable("proof")).unwrap_err())[..],
            [UsageViolation::ErasedVariableAtRuntime { .. }]
        ));
    }

    #[test]
    fn branch_join_is_the_least_enclosing_interval() {
        let expression = UsageExpr::Branch {
            condition: Box::new(variable("condition")),
            then_expression: Box::new(variable("x")),
            else_expression: Box::new(UsageExpr::Unit),
        };
        let demands = expression.synthesize_demands().unwrap();
        assert_eq!(demands.get("condition"), Usage::ONE);
        assert_eq!(demands.get("x"), Usage::AFFINE);

        let both = UsageExpr::Branch {
            condition: Box::new(UsageExpr::Unit),
            then_expression: Box::new(variable("x")),
            else_expression: Box::new(variable("x")),
        };
        assert_eq!(both.synthesize_demands().unwrap().get("x"), Usage::ONE);
    }

    #[test]
    fn application_substitutes_parameter_usage_pointwise() {
        let argument = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
        let expression = UsageExpr::Apply {
            function: Box::new(variable("f")),
            parameter_usages: vec![Usage::AFFINE],
            arguments: vec![argument],
        };
        let demands = expression.synthesize_demands().unwrap();
        assert_eq!(demands.get("f"), Usage::ONE);
        assert_eq!(demands.get("x"), Usage::finite(0, 2));
    }

    #[test]
    fn application_checks_arity() {
        let expression = UsageExpr::Apply {
            function: Box::new(UsageExpr::Unit),
            parameter_usages: vec![Usage::ONE],
            arguments: Vec::new(),
        };
        assert_eq!(
            expression.synthesize_demands(),
            Err(DemandError::ApplicationArity {
                expected: 1,
                actual: 0
            })
        );
    }
}
