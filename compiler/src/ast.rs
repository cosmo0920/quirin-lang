use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Declaration {
    Dimension(DimensionDecl),
    Axis(AxisDecl),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DimensionDecl {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisDecl {
    pub name: String,
    pub mode: AxisMode,
    pub dimension: DimensionRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisMode {
    Credit,
    Live,
    CostTotal,
    CostPeak,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DimensionRef {
    Named(String),
    Count(String),
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for declaration in &self.declarations {
            writeln!(f, "{declaration}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimension(decl) => write!(f, "dimension {} : Idx + Ordered;", decl.name),
            Self::Axis(decl) => write!(
                f,
                "quant axis {} : {}<{}>;",
                decl.name, decl.mode, decl.dimension
            ),
        }
    }
}

impl fmt::Display for AxisMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Credit => "credit",
            Self::Live => "live",
            Self::CostTotal => "cost.total",
            Self::CostPeak => "cost.peak",
        })
    }
}

impl fmt::Display for DimensionRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => f.write_str(name),
            Self::Count(resource) => write!(f, "Count<{resource}>"),
        }
    }
}
