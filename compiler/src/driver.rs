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

//! Compatibility driver with default first-party discovery.
//! Generic source compilation lives in qcore-verification and takes an explicit registry.
use crate::first_party_registry;
pub use qcore_verification::driver::*;

/// Parses one or more source files as a module, checks the whole program, and
/// emits a standalone C11 translation unit for an executable module. Explicit
/// library/contract modules return checked declarations without an entry or C
/// artifact. Their contracts are not automatically discharged by compilation.
/// `CompiledModule::result_profile` records the bounded scalar/custody profile:
/// i32/unit scalar checking does not establish typed ownership of the separate
/// completion payload. General result/error and owned-result ABIs are rejected.
pub fn compile_files(
    files: &[SourceFile],
    options: &CompileOptions,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let registry = first_party_registry();
    compile_files_with_registry(files, options, &registry)
}

#[cfg(test)]
#[path = "../tests/unit/driver.rs"]
mod tests;

/// Default-registry compilation with explicit filesystem import loading.
pub fn compile_files_with_filesystem(
    files: &[SourceFile],
    options: &CompileOptions,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_with_filesystem_with_registry(files, options, &first_party_registry())
}

/// [`compile_files`] for a builder's recorded target (card
/// `W48-QR-SCALAR-TYPES`): the target supplies the width of `isize`/`usize`.
pub fn compile_files_for_target(
    files: &[SourceFile],
    options: &CompileOptions,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_with_registry_for_target(files, options, &first_party_registry(), target)
}

/// [`compile_files_with_filesystem`] for a builder's recorded target.
pub fn compile_files_with_filesystem_for_target(
    files: &[SourceFile],
    options: &CompileOptions,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_with_filesystem_with_registry_for_target(
        files,
        options,
        &first_party_registry(),
        target,
    )
}
