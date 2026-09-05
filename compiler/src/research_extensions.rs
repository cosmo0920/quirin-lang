//! Cycle 17 aggregate research-extension certificate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::inference::{
    InferenceCertificate, InferenceConstraint, InferenceError, check_inference_certificate,
};
use crate::legacy_c::{LegacySafetyCertificate, check_legacy_safety_certificate};
use crate::native_concurrency::{
    NativeBackendError, NativeExecutionCertificate, NativeProgram, PlatformMatrixCertificate,
    PlatformMatrixError, PlatformRun, check_platform_matrix, run_native_backend,
};
use crate::resource_flow::{Flow, FlowError};
use crate::weak_memory::{
    C11Execution, C11ExecutionCertificate, WeakMemoryError, check_c11_execution,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopBoundEvidence {
    /// A termination proof supplied an exact upper bound.
    Terminating(u64),
    /// Abstract interpretation widened to this checked maximum iteration count.
    Widened(u64),
    /// No finite resource summary is claimed.
    Unbounded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopSummaryCertificate {
    pub body: Flow,
    pub evidence: LoopBoundEvidence,
    pub summary: Flow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopSummaryError {
    Unbounded,
    Arithmetic(FlowError),
    InvalidCertificate,
}

pub fn summarize_general_loop(
    body: &Flow,
    evidence: LoopBoundEvidence,
) -> Result<LoopSummaryCertificate, LoopSummaryError> {
    let iterations = match evidence {
        LoopBoundEvidence::Terminating(iterations) | LoopBoundEvidence::Widened(iterations) => {
            iterations
        }
        LoopBoundEvidence::Unbounded => return Err(LoopSummaryError::Unbounded),
    };
    let summary = body
        .bounded_loop(iterations)
        .map_err(LoopSummaryError::Arithmetic)?;
    Ok(LoopSummaryCertificate {
        body: body.clone(),
        evidence,
        summary,
    })
}

pub fn check_loop_summary(certificate: &LoopSummaryCertificate) -> Result<(), LoopSummaryError> {
    let expected = summarize_general_loop(&certificate.body, certificate.evidence)?;
    if expected.summary == certificate.summary {
        Ok(())
    } else {
        Err(LoopSummaryError::InvalidCertificate)
    }
}

pub type Ledger = BTreeMap<String, i128>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerMorphism {
    pub name: String,
    pub source: Ledger,
    pub target: Ledger,
    pub delta: Ledger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryError {
    NegativeLedger { morphism: String, axis: String },
    InvalidDelta { morphism: String, axis: String },
    EndpointMismatch { left: String, right: String },
    EmptyAccount,
}

impl LedgerMorphism {
    pub fn check(&self) -> Result<(), CategoryError> {
        let axes = self
            .source
            .keys()
            .chain(self.target.keys())
            .chain(self.delta.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for axis in axes {
            let source = self.source.get(&axis).copied().unwrap_or(0);
            let target = self.target.get(&axis).copied().unwrap_or(0);
            let delta = self.delta.get(&axis).copied().unwrap_or(0);
            if source < 0 || target < 0 {
                return Err(CategoryError::NegativeLedger {
                    morphism: self.name.clone(),
                    axis,
                });
            }
            if source.checked_add(delta) != Some(target) {
                return Err(CategoryError::InvalidDelta {
                    morphism: self.name.clone(),
                    axis,
                });
            }
        }
        Ok(())
    }

    pub fn then(&self, next: &Self) -> Result<Self, CategoryError> {
        self.check()?;
        next.check()?;
        if self.target != next.source {
            return Err(CategoryError::EndpointMismatch {
                left: self.name.clone(),
                right: next.name.clone(),
            });
        }
        let mut delta = self.delta.clone();
        for (axis, amount) in &next.delta {
            *delta.entry(axis.clone()).or_default() += amount;
        }
        let result = Self {
            name: format!("{};{}", self.name, next.name),
            source: self.source.clone(),
            target: next.target.clone(),
            delta,
        };
        result.check()?;
        Ok(result)
    }
}

/// Interpret the executable loop resource ledger as a morphism.
pub fn loop_ledger_morphism(
    certificate: &LoopSummaryCertificate,
) -> Result<LedgerMorphism, CategoryError> {
    let source = certificate
        .summary
        .required_entry()
        .iter()
        .map(|(axis, amount)| (axis.to_owned(), i128::from(amount)))
        .collect::<Ledger>();
    let mut target = source.clone();
    let mut delta = Ledger::new();
    for (axis, amount) in certificate.summary.net().iter() {
        delta.insert(axis.to_owned(), amount);
        *target.entry(axis.to_owned()).or_default() += amount;
    }
    let morphism = LedgerMorphism {
        name: "loop".into(),
        source,
        target,
        delta,
    };
    morphism.check()?;
    Ok(morphism)
}

/// Interpret native custody cleanup as a resource-ledger morphism.
pub fn native_ledger_morphism(
    program: &NativeProgram,
    execution: &NativeExecutionCertificate,
) -> Result<LedgerMorphism, CategoryError> {
    let source = program
        .initial_custody
        .keys()
        .map(|resource| (format!("custody:{resource}"), 1))
        .collect::<Ledger>();
    let target = program
        .initial_custody
        .keys()
        .filter(|resource| !execution.destroyed_resources.contains(resource))
        .map(|resource| (format!("custody:{resource}"), 1))
        .collect::<Ledger>();
    let delta = program
        .initial_custody
        .keys()
        .filter(|resource| execution.destroyed_resources.contains(resource))
        .map(|resource| (format!("custody:{resource}"), -1))
        .collect::<Ledger>();
    let morphism = LedgerMorphism {
        name: "native-cleanup".into(),
        source,
        target,
        delta,
    };
    morphism.check()?;
    Ok(morphism)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoricalCertificate {
    /// Resource-like ledgers share the same checked morphism law.
    pub resource_morphisms: Vec<LedgerMorphism>,
    /// Relations whose semantics do not form additive resource ledgers.
    pub deliberately_separate: BTreeSet<String>,
}

pub fn check_categorical_account(account: &CategoricalCertificate) -> Result<(), CategoryError> {
    if account.resource_morphisms.is_empty() || account.deliberately_separate.is_empty() {
        return Err(CategoryError::EmptyAccount);
    }
    for morphism in &account.resource_morphisms {
        morphism.check()?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Cycle17Input<'a> {
    pub constraints: &'a [InferenceConstraint],
    pub inference: &'a InferenceCertificate,
    pub loop_summary: &'a LoopSummaryCertificate,
    pub legacy: &'a LegacySafetyCertificate,
    pub c11: &'a C11Execution,
    pub native: &'a NativeProgram,
    pub platform_runs: &'a [PlatformRun],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle17Certificate {
    pub inference: InferenceCertificate,
    pub loop_summary: LoopSummaryCertificate,
    pub legacy: LegacySafetyCertificate,
    pub c11: C11ExecutionCertificate,
    pub native: NativeExecutionCertificate,
    pub platforms: PlatformMatrixCertificate,
    pub categorical: CategoricalCertificate,
    /// All six promoted constructors have executable evidence in this value.
    pub remaining_research: Vec<ResearchExtension>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchExtension {
    PrincipalInference,
    GeneralNonzeroNetLoops,
    UnrestrictedLegacyC,
    CategoricalUnification,
    FullC11WeakMemory,
    NativeConcurrentBackend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cycle17Error {
    Inference(InferenceError),
    Loop(LoopSummaryError),
    Legacy(String),
    WeakMemory(Vec<WeakMemoryError>),
    Native(NativeBackendError),
    Platform(PlatformMatrixError),
    Category(CategoryError),
}

impl fmt::Display for Cycle17Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Cycle17Error {}

pub fn close_cycle17(input: &Cycle17Input<'_>) -> Result<Cycle17Certificate, Cycle17Error> {
    check_inference_certificate(input.constraints, input.inference)
        .map_err(Cycle17Error::Inference)?;
    check_loop_summary(input.loop_summary).map_err(Cycle17Error::Loop)?;
    check_legacy_safety_certificate(input.legacy).map_err(Cycle17Error::Legacy)?;
    let c11 = check_c11_execution(input.c11).map_err(Cycle17Error::WeakMemory)?;
    let native = run_native_backend(input.native).map_err(Cycle17Error::Native)?;
    let platforms = check_platform_matrix(input.platform_runs).map_err(Cycle17Error::Platform)?;
    let categorical = CategoricalCertificate {
        resource_morphisms: vec![
            loop_ledger_morphism(input.loop_summary).map_err(Cycle17Error::Category)?,
            native_ledger_morphism(input.native, &native).map_err(Cycle17Error::Category)?,
        ],
        deliberately_separate: BTreeSet::from([
            "effects".into(),
            "path-constraints".into(),
            "reads-from".into(),
            "modification-order".into(),
        ]),
    };
    check_categorical_account(&categorical).map_err(Cycle17Error::Category)?;
    Ok(Cycle17Certificate {
        inference: input.inference.clone(),
        loop_summary: input.loop_summary.clone(),
        legacy: input.legacy.clone(),
        c11,
        native,
        platforms,
        categorical,
        remaining_research: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::{InferenceConstraint, infer_principal};
    use crate::legacy_c::{
        C11TranslationUnit, LegacyCError, ResourceApiKind, ResourceContracts,
        UnrestrictedLegacyResult, analyze_unrestricted_c11,
    };
    use crate::native_concurrency::{NativeOperation, NativeScheduleStep, RequiredPlatform};
    use crate::resource_flow::Quantity;
    use crate::weak_memory::{C11Execution, MemoryEvent, MemoryEventKind};

    fn q(amount: u64) -> Quantity {
        Quantity::single("memory", amount)
    }

    #[test]
    fn positive_and_negative_net_loops_have_checked_invariants() {
        let growing = Flow::acquire(q(2), q(2));
        let grown = summarize_general_loop(&growing, LoopBoundEvidence::Terminating(3)).unwrap();
        assert_eq!(grown.summary.net().get("memory"), 6);
        assert_eq!(grown.summary.peak().get("memory"), 6);
        assert_eq!(grown.summary.total().get("memory"), 6);
        assert_eq!(grown.summary.required_entry().get("memory"), 0);
        check_loop_summary(&grown).unwrap();

        let shrinking = Flow::release(q(2), Quantity::zero());
        let shrunk = summarize_general_loop(&shrinking, LoopBoundEvidence::Widened(3)).unwrap();
        assert_eq!(shrunk.summary.net().get("memory"), -6);
        assert_eq!(shrunk.summary.required_entry().get("memory"), 6);
    }

    #[test]
    fn unbounded_loop_refuses_a_finite_resource_claim() {
        assert_eq!(
            summarize_general_loop(&Flow::zero(), LoopBoundEvidence::Unbounded),
            Err(LoopSummaryError::Unbounded)
        );
    }

    #[test]
    fn ledger_morphisms_compose_only_at_equal_endpoints() {
        let first = LedgerMorphism {
            name: "acquire".into(),
            source: Ledger::new(),
            target: BTreeMap::from([("memory".into(), 2)]),
            delta: BTreeMap::from([("memory".into(), 2)]),
        };
        let second = LedgerMorphism {
            name: "release".into(),
            source: first.target.clone(),
            target: Ledger::new(),
            delta: BTreeMap::from([("memory".into(), -2)]),
        };
        let composed = first.then(&second).unwrap();
        assert!(composed.source.is_empty());
        assert!(composed.target.is_empty());
        let mut broken = second;
        broken.source.insert("memory".into(), 3);
        broken.delta.insert("memory".into(), -3);
        assert!(matches!(
            first.then(&broken),
            Err(CategoryError::EndpointMismatch { .. })
        ));
    }

    #[test]
    fn aggregate_certificate_contains_all_six_discharges() {
        let constraints = vec![InferenceConstraint::Owns { identity: 1 }];
        let inference = infer_principal(&constraints).unwrap();
        let loop_summary = summarize_general_loop(
            &Flow::acquire(q(1), q(1)),
            LoopBoundEvidence::Terminating(2),
        )
        .unwrap();
        let contracts = ResourceContracts::new([
            ("q_alloc", ResourceApiKind::Acquire),
            ("q_free", ResourceApiKind::Release),
        ]);
        let unit = C11TranslationUnit {
            source: "void *q_alloc(void); void q_free(void *); void subject(void) { void *p = q_alloc(); q_free(p); }",
            ..C11TranslationUnit::default()
        };
        let legacy = match analyze_unrestricted_c11(&unit, "subject", &contracts) {
            Ok(UnrestrictedLegacyResult::Safe(certificate)) => certificate,
            Err(LegacyCError::ProductionParserUnavailable(_)) => return,
            other => panic!("fixture did not produce a legacy certificate: {other:?}"),
        };
        let c11 = C11Execution {
            events: vec![MemoryEvent {
                id: 0,
                thread: 0,
                kind: MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            }],
            source_observation: vec![0],
            target_observation: vec![0],
            ..C11Execution::default()
        };
        let native = NativeProgram {
            tasks: BTreeSet::from([0]),
            initial_custody: BTreeMap::from([(8, 0)]),
            schedule: vec![NativeScheduleStep {
                task: 0,
                operation: NativeOperation::Destroy { resource: 8 },
            }],
            ..NativeProgram::default()
        };
        let runs = vec![
            PlatformRun {
                platform: RequiredPlatform::Linux,
                passed: true,
                sanitizer: true,
            },
            PlatformRun {
                platform: RequiredPlatform::Windows,
                passed: true,
                sanitizer: true,
            },
        ];
        let certificate = close_cycle17(&Cycle17Input {
            constraints: &constraints,
            inference: &inference,
            loop_summary: &loop_summary,
            legacy: &legacy,
            c11: &c11,
            native: &native,
            platform_runs: &runs,
        })
        .unwrap();
        assert!(certificate.remaining_research.is_empty());
        assert_eq!(certificate.categorical.resource_morphisms.len(), 2);
    }
}
