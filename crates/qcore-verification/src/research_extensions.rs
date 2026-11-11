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

//! Cycle 17 aggregate research-extension certificate.

use std::collections::BTreeSet;
use std::fmt;

use qcore_c_gateway::legacy_c::{LegacySafetyCertificate, check_legacy_safety_certificate};
use qcore_core::inference::{
    InferenceCertificate, InferenceConstraint, InferenceError, check_inference_certificate,
};
use qcore_core::native_concurrency::{
    NativeBackendError, NativeExecutionCertificate, NativeProgram, PlatformMatrixCertificate,
    PlatformMatrixError, PlatformRun, check_platform_matrix, run_native_backend,
};
use qcore_core::weak_memory::{
    C11Execution, C11ExecutionCertificate, WeakMemoryError, check_c11_execution,
};

pub use qcore_core::resource_accounting::{
    CategoricalCertificate, CategoryError, Ledger, LedgerMorphism, LoopBoundEvidence,
    LoopSummaryCertificate, LoopSummaryError, check_categorical_account, check_loop_summary,
    loop_ledger_morphism, native_ledger_morphism, summarize_general_loop,
};

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
