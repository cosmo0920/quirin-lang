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

use qcore_core::ast::{AxisDecl, Declaration, DimensionRef, Program};

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
        Self::check_located(program).map_err(|(error, _)| error)
    }

    /// Identical validation to [`StaticEnvironment::check`], additionally
    /// reporting the index in `program.declarations` of the declaration that
    /// produced the failure. Callers that reconstructed `program` from source
    /// slices use that index to attribute the diagnostic to the offending
    /// declaration's original span instead of a degenerate `0..0` span. The
    /// accepted/rejected set and the returned environment are unchanged.
    pub fn check_located(program: &Program) -> Result<Self, (CheckError, usize)> {
        let mut environment = Self::default();

        // Name collection is deliberately separate from validation so named
        // dimensions can be referenced before their declaration.
        for (index, declaration) in program.declarations.iter().enumerate() {
            if let Declaration::Dimension(decl) = declaration
                && !environment.dimensions.insert(decl.name.clone())
            {
                return Err((CheckError::DuplicateDimension(decl.name.clone()), index));
            }
        }
        for (index, declaration) in program.declarations.iter().enumerate() {
            if let Declaration::Axis(decl) = declaration {
                if let DimensionRef::Named(name) = &decl.dimension
                    && !environment.dimensions.contains(name)
                {
                    return Err((CheckError::UnknownDimension(name.clone()), index));
                }
                if environment
                    .axes
                    .insert(decl.name.clone(), decl.clone())
                    .is_some()
                {
                    return Err((CheckError::DuplicateAxis(decl.name.clone()), index));
                }
            }
        }
        Ok(environment)
    }
}

#[cfg(test)]
#[path = "../tests/unit/check.rs"]
mod tests;
