use std::fmt;

use crate::ast::DimensionRef;

/// An index expression annotated with its nominal dimension.
///
/// Constructors are private so every compound term passes through a dimension
/// check. Scaling accepts only a known natural constant, which keeps this core
/// inside QF-LIA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexTerm {
    dimension: DimensionRef,
    expression: IndexExpression,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexExpression {
    Literal(u128),
    Variable(String),
    Projection { quant: String, axis: String },
    Add(Box<IndexTerm>, Box<IndexTerm>),
    Scale(u128, Box<IndexTerm>),
    Min(Box<IndexTerm>, Box<IndexTerm>),
    Max(Box<IndexTerm>, Box<IndexTerm>),
    CheckedMonus(Box<IndexTerm>, Box<IndexTerm>),
    SizeOf(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexError {
    pub operation: &'static str,
    pub left: DimensionRef,
    pub right: DimensionRef,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dimension mismatch in {}: {} versus {}",
            self.operation, self.left, self.right
        )
    }
}

impl std::error::Error for IndexError {}

impl IndexTerm {
    pub fn literal(value: u128, dimension: DimensionRef) -> Self {
        Self {
            dimension,
            expression: IndexExpression::Literal(value),
        }
    }

    pub fn variable(name: impl Into<String>, dimension: DimensionRef) -> Self {
        Self {
            dimension,
            expression: IndexExpression::Variable(name.into()),
        }
    }

    pub fn projection(
        quant: impl Into<String>,
        axis: impl Into<String>,
        dimension: DimensionRef,
    ) -> Self {
        Self {
            dimension,
            expression: IndexExpression::Projection {
                quant: quant.into(),
                axis: axis.into(),
            },
        }
    }

    pub fn size_of(type_name: impl Into<String>) -> Self {
        Self {
            dimension: DimensionRef::Named("Bytes".into()),
            expression: IndexExpression::SizeOf(type_name.into()),
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self, IndexError> {
        self.binary(other, "addition", IndexExpression::Add)
    }

    pub fn min(self, other: Self) -> Result<Self, IndexError> {
        self.binary(other, "min", IndexExpression::Min)
    }

    pub fn max(self, other: Self) -> Result<Self, IndexError> {
        self.binary(other, "max", IndexExpression::Max)
    }

    /// Builds checked monus. A later constraint pass must prove `other <= self`.
    pub fn checked_monus(self, other: Self) -> Result<Self, IndexError> {
        self.binary(other, "monus", IndexExpression::CheckedMonus)
    }

    pub fn scale(self, factor: u128) -> Self {
        Self {
            dimension: self.dimension.clone(),
            expression: IndexExpression::Scale(factor, Box::new(self)),
        }
    }

    pub fn dimension(&self) -> &DimensionRef {
        &self.dimension
    }

    pub fn expression(&self) -> &IndexExpression {
        &self.expression
    }

    fn binary(
        self,
        other: Self,
        operation: &'static str,
        constructor: fn(Box<IndexTerm>, Box<IndexTerm>) -> IndexExpression,
    ) -> Result<Self, IndexError> {
        if self.dimension != other.dimension {
            return Err(IndexError {
                operation,
                left: self.dimension,
                right: other.dimension,
            });
        }
        Ok(Self {
            dimension: self.dimension.clone(),
            expression: constructor(Box::new(self), Box::new(other)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes() -> DimensionRef {
        DimensionRef::Named("Bytes".into())
    }

    #[test]
    fn same_dimension_addition_is_constructible() {
        let n = IndexTerm::variable("n", bytes());
        let term = n.checked_add(IndexTerm::literal(8, bytes())).unwrap();
        assert_eq!(term.dimension(), &bytes());
        assert!(matches!(term.expression(), IndexExpression::Add(_, _)));
    }

    #[test]
    fn cross_dimension_addition_is_rejected() {
        let memory = IndexTerm::variable("memory", bytes());
        let files = IndexTerm::variable("files", DimensionRef::Count("Fd".into()));
        let error = memory.checked_add(files).unwrap_err();
        assert_eq!(error.operation, "addition");
        assert_eq!(error.left, bytes());
        assert_eq!(error.right, DimensionRef::Count("Fd".into()));
    }

    #[test]
    fn scaling_is_constant_by_construction() {
        let term = IndexTerm::variable("n", bytes()).scale(16);
        assert!(matches!(term.expression(), IndexExpression::Scale(16, _)));
    }
}
