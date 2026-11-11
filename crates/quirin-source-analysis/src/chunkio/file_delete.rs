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

//! Native deletion eligibility and conditional return/frame outcomes. Successful
//! unlink is not inferred from a call or from freeing the path allocation.
use super::{PINNED_FLUENT_BIT_REVISION, file_attempts::check_file_delete_source};
use crate::cleanup_outcome::{
    CleanupField, CleanupProfile, GuardedDeleteCertificate, ResetValue,
    check_guarded_delete_outcomes,
};
use crate::legacy_c::C11TranslationUnit;
pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete.qv"
);
pub fn check_native_delete_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
) -> Result<GuardedDeleteCertificate, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("native delete revision mismatch".into());
    }
    let evidence = check_file_delete_source(unit, revision)?;
    check_guarded_delete_outcomes(evidence.source(), &profile(), "map", CONTRACT)
}
pub fn recheck_native_delete(
    certificate: &GuardedDeleteCertificate,
) -> Result<GuardedDeleteCertificate, String> {
    if certificate.source().source_revision() != PINNED_FLUENT_BIT_REVISION {
        return Err("native delete dependency revision mismatch".into());
    }
    check_guarded_delete_outcomes(certificate.source(), &profile(), "map", CONTRACT)
}
/// The guarded-delete record profile the retained in-process route composes
/// under. Public within the adapter since card `P04-COMPOSE-A` so the relocated
/// `file_close` decision procedure receives it as an explicit parameter, and so
/// `compiler/tests/composition_route_parity.rs` can assert directly that this
/// table **is** the vocabulary the pinned contract declares.
pub fn profile() -> CleanupProfile {
    CleanupProfile {
        function: "cio_file_native_delete".into(),
        root_type: "struct cio_file *".into(),
        syscall: "unlink".into(),
        syscall_signature: "int (const char *)".into(),
        presence_field: "fd".into(),
        fields: vec![
            CleanupField {
                name: "fd".into(),
                c_type: "int".into(),
                reset: ResetValue::Integer(-1),
            },
            CleanupField {
                name: "map".into(),
                c_type: "char *".into(),
                reset: ResetValue::Null,
            },
            CleanupField {
                name: "path".into(),
                c_type: "char *".into(),
                reset: ResetValue::Null,
            },
        ],
        argument_fields: vec!["path".into()],
        policy: "native_delete_outcomes".into(),
        errno_function: "__errno_location".into(),
        report_function: "cio_errno_print".into(),
    }
}
#[cfg(test)]
#[path = "../../tests/unit/file_delete.rs"]
mod tests;
