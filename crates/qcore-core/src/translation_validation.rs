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

//! Cycle 13 translation validation at the checked source/CFG/C boundary.
//!
//! The validator is deliberately independent of the emitter.  It re-runs the
//! source semantics, checks that the retained CFG is exactly the cleanup
//! lowering of that source, interprets the CFG, and parses an instrumented C
//! observation.  A successful certificate therefore records three equal
//! observations rather than trusting a pass-local success flag.

use std::fmt;

use crate::c_emit::{CEmitError, parse_c_observation};
use crate::cfg::{CfgError, ExecutionObservation, TypedCfg, interpret_cfg};
use crate::cleanup_lowering::{LoweringError, SourceProgram, interpret_source, lower_cleanup_cfg};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationCertificate {
    pub source: ExecutionObservation,
    pub cfg: ExecutionObservation,
    pub c11: ExecutionObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TranslationValidationError {
    Source(LoweringError),
    Cfg(CfgError),
    COutput(CEmitError),
    PassBoundaryMismatch,
    SourceCfgMismatch {
        source: ExecutionObservation,
        cfg: ExecutionObservation,
    },
    CfgC11Mismatch {
        cfg: ExecutionObservation,
        c11: ExecutionObservation,
    },
}

impl fmt::Display for TranslationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TranslationValidationError {}

/// Validate the pass boundary structurally, including cleanup block contents,
/// identities, error tags, and every CFG edge.
pub fn validate_cfg_boundary(
    source: &SourceProgram,
    cfg: &TypedCfg,
) -> Result<(), TranslationValidationError> {
    let expected = lower_cleanup_cfg(source).map_err(TranslationValidationError::Source)?;
    if &expected == cfg {
        Ok(())
    } else {
        Err(TranslationValidationError::PassBoundaryMismatch)
    }
}

/// Validate observational agreement from checked source through the emitted
/// C11 program's instrumentation output.
pub fn validate_translation(
    source: &SourceProgram,
    cfg: &TypedCfg,
    c_stdout: &str,
) -> Result<TranslationCertificate, TranslationValidationError> {
    validate_cfg_boundary(source, cfg)?;
    let source_observation =
        interpret_source(source).map_err(TranslationValidationError::Source)?;
    let cfg_observation = interpret_cfg(cfg).map_err(TranslationValidationError::Cfg)?;
    if source_observation != cfg_observation {
        return Err(TranslationValidationError::SourceCfgMismatch {
            source: source_observation,
            cfg: cfg_observation,
        });
    }
    let c11_observation = parse_c_observation(c_stdout, cfg.error_abi())
        .map_err(TranslationValidationError::COutput)?;
    if cfg_observation != c11_observation {
        return Err(TranslationValidationError::CfgC11Mismatch {
            cfg: cfg_observation,
            c11: c11_observation,
        });
    }
    Ok(TranslationCertificate {
        source: source_observation,
        cfg: cfg_observation,
        c11: c11_observation,
    })
}

#[cfg(test)]
#[path = "../tests/unit/translation_validation.rs"]
mod tests;
