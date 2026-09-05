use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ast::{AxisDecl, Declaration, DimensionRef, Program};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticEnvironment {
    pub dimensions: BTreeSet<String>,
    pub axes: BTreeMap<String, AxisDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    DuplicateDimension(String),
    DuplicateAxis(String),
    UnknownDimension(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDimension(name) => write!(f, "duplicate dimension `{name}`"),
            Self::DuplicateAxis(name) => write!(f, "duplicate quant axis `{name}`"),
            Self::UnknownDimension(name) => write!(f, "unknown dimension `{name}`"),
        }
    }
}

impl std::error::Error for CheckError {}

impl StaticEnvironment {
    pub fn check(program: &Program) -> Result<Self, CheckError> {
        let mut environment = Self::default();

        // Name collection is deliberately separate from validation so named
        // dimensions can be referenced before their declaration.
        for declaration in &program.declarations {
            if let Declaration::Dimension(decl) = declaration
                && !environment.dimensions.insert(decl.name.clone())
            {
                return Err(CheckError::DuplicateDimension(decl.name.clone()));
            }
        }
        for declaration in &program.declarations {
            if let Declaration::Axis(decl) = declaration {
                if let DimensionRef::Named(name) = &decl.dimension
                    && !environment.dimensions.contains(name)
                {
                    return Err(CheckError::UnknownDimension(name.clone()));
                }
                if environment
                    .axes
                    .insert(decl.name.clone(), decl.clone())
                    .is_some()
                {
                    return Err(CheckError::DuplicateAxis(decl.name.clone()));
                }
            }
        }
        Ok(environment)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_program;

    use super::*;

    #[test]
    fn validates_named_and_count_dimensions() {
        let program = parse_program(
            "\
dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
quant axis files : live<Count<Fd>>;
",
        )
        .unwrap();
        let environment = StaticEnvironment::check(&program).unwrap();
        assert!(environment.dimensions.contains("Bytes"));
        assert_eq!(environment.axes.len(), 2);
    }

    #[test]
    fn rejects_unknown_named_dimension() {
        let program = parse_program("quant axis memory : live<Bytes>;").unwrap();
        assert_eq!(
            StaticEnvironment::check(&program),
            Err(CheckError::UnknownDimension("Bytes".into()))
        );
    }

    #[test]
    fn allows_forward_dimension_references() {
        let program = parse_program(
            "\
quant axis memory : live<Bytes>;
dimension Bytes : Idx + Ordered;
",
        )
        .unwrap();
        assert!(StaticEnvironment::check(&program).is_ok());
    }
}
