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
