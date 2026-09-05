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
#[path = "../tests/unit/research_extensions.rs"]
mod tests;
