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

//! Opt-in observational IR capture for configured units. The normal analysis
//! still runs, including certificate replay and all source/typed consumers.

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::OnceLock;

use qcore_c_gateway::audit_ir_capture::with_capture;
use qcore_c_gateway::audit_ir_store::{FORMAT, FragmentStore, MAX_OBJECT_BYTES};
use qcore_c_gateway::legacy_c::{CPreprocessorOption, CSourceInput, PreparedTranslationUnit};
use qcore_schemas::contract::ResourceContracts;
use serde::Serialize;

use crate::certificate_context::AuditContractTrustPolicy;
use crate::compile_db::CompileTaskIdentity;

pub const DIRECTORY_VARIABLE: &str = "QCORE_AUDIT_IR_DIR";

pub(crate) fn configured<T>(
    prepared: &PreparedTranslationUnit,
    identity: &CompileTaskIdentity,
    contracts: &ResourceContracts,
    source_profile: &str,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let Some(directory) = std::env::var_os(DIRECTORY_VARIABLE) else {
        return work();
    };
    if directory.is_empty() {
        return Err("QCORE_AUDIT_IR_DIR must name an artifact directory".into());
    }
    #[derive(Serialize)]
    struct Context<'a> {
        format: &'static str,
        checker_binary_sha256: &'a str,
        compile_task: &'a CompileTaskIdentity,
        source_profile_sha256: &'a str,
        bounds_selected: bool,
        null_selected: bool,
        trust_policy: AuditContractTrustPolicy,
        contracts: &'a ResourceContracts,
        inventory: &'a qcore_c_gateway::legacy_c::SourceDefinitionInventory,
        raw_source: &'a str,
        preprocessed_source: Option<&'a str>,
        preprocessing_error: Option<String>,
        line_marked_source: Option<&'a str>,
        line_marked_error: Option<String>,
        request: Request<'a>,
        requires_fresh_source_and_typed_readings: bool,
    }
    #[derive(Serialize)]
    struct Request<'a> {
        include_paths: &'a [String],
        definitions: &'a [String],
        options: &'a [CPreprocessorOption],
        source_input: &'a CSourceInput,
        dialect: String,
        function_scope: String,
    }
    let context = Context {
        format: FORMAT,
        checker_binary_sha256: binary_identity()?,
        compile_task: identity,
        source_profile_sha256: source_profile,
        bounds_selected: std::env::var_os(crate::projection::BOUNDS_SELECTION).is_some(),
        null_selected: std::env::var_os(crate::projection::NULL_SELECTION).is_some(),
        trust_policy: AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions,
        contracts,
        inventory: prepared.inventory(),
        raw_source: prepared.raw_source(),
        preprocessed_source: prepared.preprocessed_source().ok(),
        preprocessing_error: prepared.validation_error().map(ToString::to_string),
        line_marked_source: prepared.line_marked_source(),
        line_marked_error: prepared.line_marked_error().map(ToString::to_string),
        request: Request {
            include_paths: prepared.include_paths(),
            definitions: prepared.definitions(),
            options: prepared.preprocessor_options(),
            source_input: prepared.input(),
            dialect: format!("{:?}", prepared.dialect()),
            function_scope: format!("{:?}", prepared.function_scope()),
        },
        requires_fresh_source_and_typed_readings: true,
    };
    with_capture(
        FragmentStore::new(PathBuf::from(directory), MAX_OBJECT_BYTES)?,
        &context,
        prepared.inventory().definitions(),
        work,
    )
    .map(|(value, _receipt)| value)
}

// Like checkpoint build identity, this binds the observed executable bytes;
// it is not compiler/process attestation. Cache once, never once per worker.
fn binary_identity() -> Result<&'static str, String> {
    static IDENTITY: OnceLock<Result<String, String>> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            const LIMIT: u64 = 512 * 1024 * 1024;
            let file = File::open(std::env::current_exe().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
                return Err("IR checker identity exceeds the 512 MiB hashing limit".into());
            }
            let mut bytes = Vec::new();
            file.take(LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > LIMIT {
                return Err("IR checker identity exceeds the 512 MiB hashing limit".into());
            }
            Ok(qcore_schemas::sha256_hex(&bytes))
        })
        .as_ref()
        .map(String::as_str)
        .map_err(Clone::clone)
}
