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
mod tests {
    use super::*;
    use crate::{ExecutionOutcome, SourceOp, TraceEvent};

    fn source(operations: Vec<SourceOp>) -> SourceProgram {
        SourceProgram::new("subject", vec![1, 2], ["Failure".to_owned()], operations).unwrap()
    }

    #[test]
    fn validates_all_three_observations() {
        let source = source(vec![
            SourceOp::Acquire {
                resource: 1,
                succeeds: true,
                error: "Failure".into(),
            },
            SourceOp::TransferToResult(1),
            SourceOp::Return(7),
        ]);
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let certificate = validate_translation(&source, &cfg, "OK 7\nA 1\nM 1\nP 1\n").unwrap();
        assert_eq!(certificate.source, certificate.cfg);
        assert_eq!(certificate.cfg, certificate.c11);
    }

    #[test]
    fn rejects_a_target_observation_that_drops_cleanup() {
        let source = source(vec![
            SourceOp::Acquire {
                resource: 1,
                succeeds: true,
                error: "Failure".into(),
            },
            SourceOp::Check {
                passes: false,
                error: "Failure".into(),
            },
            SourceOp::Drop(1),
            SourceOp::Return(0),
        ]);
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let error = validate_translation(&source, &cfg, "ERR 1\nA 1\n").unwrap_err();
        assert!(matches!(
            error,
            TranslationValidationError::CfgC11Mismatch { .. }
        ));
    }

    #[test]
    fn exhaustive_small_corpus_preserves_source_cfg_semantics() {
        for first_succeeds in [false, true] {
            for second_succeeds in [false, true] {
                for check_passes in [false, true] {
                    let source = source(vec![
                        SourceOp::Acquire {
                            resource: 1,
                            succeeds: first_succeeds,
                            error: "Failure".into(),
                        },
                        SourceOp::Acquire {
                            resource: 2,
                            succeeds: second_succeeds,
                            error: "Failure".into(),
                        },
                        SourceOp::Check {
                            passes: check_passes,
                            error: "Failure".into(),
                        },
                        SourceOp::Drop(2),
                        SourceOp::Drop(1),
                        SourceOp::Return(11),
                    ]);
                    let cfg = lower_cleanup_cfg(&source).unwrap();
                    validate_cfg_boundary(&source, &cfg).unwrap();
                    assert_eq!(
                        interpret_source(&source).unwrap(),
                        interpret_cfg(&cfg).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn certificate_retains_error_trace() {
        let source = source(vec![
            SourceOp::Acquire {
                resource: 1,
                succeeds: true,
                error: "Failure".into(),
            },
            SourceOp::Acquire {
                resource: 2,
                succeeds: false,
                error: "Failure".into(),
            },
            SourceOp::Drop(2),
            SourceOp::Drop(1),
            SourceOp::Return(0),
        ]);
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let certificate = validate_translation(&source, &cfg, "ERR 1\nA 1\nR 1\n").unwrap();
        assert_eq!(
            certificate.source.outcome,
            ExecutionOutcome::Error {
                name: "Failure".into(),
                tag: 1
            }
        );
        assert_eq!(
            certificate.source.trace,
            vec![TraceEvent::Acquire(1), TraceEvent::Release(1)]
        );
    }
}
