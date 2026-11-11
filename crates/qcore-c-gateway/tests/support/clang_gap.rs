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

//! Card CG00-CLANG-GAP-BASELINE: the shared BEFORE-control harness for the
//! Clang gap analysis's carded entries (`docs/plans/clang-gap-cards/`).
//!
//! Two routes, named as the analysis names them:
//!
//! * **G**, the custody gateway: [`g`] runs `legacy_c::analyze_unrestricted_c11`
//!   over one function `subject` with the synthetic `q_*`, `r_*`, `k_*` and
//!   `cg_*` fixture contracts (trusted test assumptions, not library facts).
//! * **T**, `typed_source.rs`: [`t`] runs `import_source_functions` over one
//!   function with an empty record profile and the scalar `sizeof` table.
//!
//! Every helper returns the route's own outcome unedited, so each test asserts
//! the current refusal or limit exactly. A `Safe` here is `Safe` over the
//! synthetic assumptions and promotes nothing.

#![allow(dead_code)]

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_c_gateway::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// C declarations of every fixture contract [`contracts`] loads.
pub const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *q_alloc(void); void q_free(void *p); void q_transfer(void *p);
void q_observe(void *p); void q_mutate(void *p); void q_consume(void *p); void q_make(void **d);
void *q_realloc(void *p, size_t n);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
void q_store(void *owner, void *item);
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p);
void *r_wrap(void *p);
int k_open(const char *path); int k_close(int fd);
void cg_sink(int value); void cg_sink2(int left, int right); void cg_log(const char *format, ...);
void cg_pair(void *p, int value);
int opaque(void);
";

pub const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";
pub const LOOP: &str = "unmodeled call, aggregate or indirect access in loop condition";
pub const FOR: &str = "unmodeled call, aggregate or indirect access in for-loop condition";
pub const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";
pub const IND: &str = "indirect memory access is outside the resource projection";
pub const INITIALIZER: &str = "braced initializer, expression, or declaration is not modeled";
pub const ASSIGNMENT: &str = "unsupported assignment expression";
pub const DESTINATION: &str = "unrecognized assignment destination";
pub const INCREMENT: &str = "unmodeled increment or decrement expression";

/// The fixture contract sources [`contracts`] loads (card CG26-VARIADICS
/// extends them with its own synthetic borrower).
pub fn contract_sources() -> BTreeMap<String, String> {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    BTreeMap::from([
        file(
            "q_base.qcore",
            include_str!("../fixtures/contracts/q_base.qv"),
        ),
        file(
            "q_rich_modes.qcore",
            include_str!("../fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_realloc.qcore",
            include_str!("../fixtures/contracts/q_realloc.qv"),
        ),
        file(
            "q_nullable.qcore",
            include_str!("../fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "q_container_store.qcore",
            include_str!("../fixtures/contracts/q_container_store.qv"),
        ),
        file(
            "r_return_call.qcore",
            include_str!("../fixtures/contracts/r_return_call.qv"),
        ),
        file(
            "descriptor_generic.qcore",
            include_str!("../fixtures/contracts/descriptor_generic.qv"),
        ),
        file(
            "clang_gap.qcore",
            include_str!("../fixtures/contracts/clang_gap.qv"),
        ),
    ])
}

pub fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&contract_sources()).expect("the CG00 fixture contracts load")
}

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG00 control: {reason}");
            false
        }
    }
}

/// Whether this Clang emits the atomic AST schema consumed by CG19/CG28.
/// Clang 17 reports these builtins as `AtomicExpr` or ordinary calls instead.
pub fn clang_atomic_ast_available() -> bool {
    if !clang_available() {
        return false;
    }
    match qcore_c_gateway::clang_version() {
        Ok(version) if version.supports_cg19_atomic_ast() => true,
        Ok(version) => {
            eprintln!(
                "skipping atomic-AST observation on Clang {}.{}.{}: CG19 requires Clang 18 or newer",
                version.major, version.minor, version.patch
            );
            false
        }
        Err(reason) => panic!("Clang was resolved but its release could not be detected: {reason}"),
    }
}

/// One G outcome for function `subject`.
#[derive(Debug)]
pub struct G {
    pub status: AnalysisStatus,
    /// Every diagnostic, rendered with `Debug`, in the route's order.
    pub diagnostics: Vec<String>,
    /// The `unsupported` descriptions only.
    pub unsupported: Vec<String>,
    pub calls: Vec<String>,
    /// Card NULL-SAFETY-PROPERTY (rule 14n): the variables whose dereference
    /// custody read non-null under a named `null-deref` premise.
    pub null_premises: Vec<String>,
}

impl G {
    pub fn refused(&self, description: &str) -> bool {
        self.unsupported.iter().any(|d| d == description)
    }
    pub fn refused_prefix(&self, prefix: &str) -> bool {
        self.unsupported.iter().any(|d| d.starts_with(prefix))
    }
    pub fn has(&self, fragment: &str) -> bool {
        self.diagnostics.iter().any(|d| d.contains(fragment))
    }
    pub fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    pub fn unknown(&self) -> bool {
        self.status == AnalysisStatus::Unknown
    }
    pub fn unsafe_(&self) -> bool {
        self.status == AnalysisStatus::Unsafe
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The marker [`G::diagnostics`] carries when the unit projected no summary
/// for the function at all (the whole unit was refused).
pub const NOT_PROJECTED: &str = "<function not projected>";

/// Run G over `DECLARATIONS` + `source`, reporting function `function`.
pub fn g_function(source: &str, function: &str) -> G {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        function,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let Some(result) = summary_of(&outcome).and_then(|summary| summary.functions.get(function))
    else {
        let reason = match &outcome {
            UnrestrictedLegacyResult::Unknown { reason, .. } => reason.clone(),
            other => panic!("a decided unit without `{function}`: {other:?}"),
        };
        return G {
            status: AnalysisStatus::Unknown,
            diagnostics: vec![NOT_PROJECTED.to_owned(), reason],
            unsupported: vec![],
            calls: vec![],
            null_premises: vec![],
        };
    };
    G {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.kind))
            .collect(),
        unsupported: result
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
        calls: result
            .cfg
            .nodes
            .iter()
            .filter_map(|node| match &node.instruction {
                LegacyInstruction::ContractCall { function, .. } => Some(function.clone()),
                _ => None,
            })
            .collect(),
        null_premises: result
            .null_premises
            .iter()
            .map(|premise| premise.variable.clone())
            .collect(),
    }
}

/// Run G over one function named `subject`.
pub fn g(source: &str) -> G {
    g_function(source, "subject")
}

/// The fixed `sizeof` operand types [`t`] admits.
pub const T_SIZEOF: &[&str] = &["int", "unsigned long"];

/// Run T over `source` (no declarations prepended), importing `function`
/// with no selected records and no signature constraint.
pub fn t_with(source: &str, function: &str, records: &[&str]) -> Result<SourceSummary, String> {
    let results = import_source_functions(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "cg00",
        &[function],
        &SourceImportProfile {
            records,
            sizeof_types: T_SIZEOF,
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap_or_else(|error| panic!("T refused the unit before import: {error}\n{source}"));
    match results.into_iter().next() {
        Some(SourceResult::Summary(summary)) => Ok(summary),
        Some(SourceResult::Unknown { reason, .. }) => Err(reason),
        None => panic!("T returned no result"),
    }
}

/// Run T over one function `subject`.
pub fn t(source: &str) -> Result<SourceSummary, String> {
    t_with(source, "subject", &[])
}

/// The reason T refused `subject`; panics when T admitted it.
pub fn t_refusal(source: &str) -> String {
    match t(source) {
        Ok(summary) => panic!("T admitted a form the entry says it refuses: {summary:?}"),
        Err(reason) => reason,
    }
}
