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

//! Unix native cleanup outcome profiles. Certificates prove conditional C field
//! transitions only: a -1 close result does not settle actual descriptor custody.
use super::{PINNED_FLUENT_BIT_REVISION, file_attempts::check_file_attempt_sources};
use crate::cleanup_outcome::{
    CleanupCertificate, CleanupField, CleanupProfile, ResetValue, check_cleanup_outcomes,
};
use crate::legacy_c::C11TranslationUnit;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv"
);

pub fn check_file_outcome_sources(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
) -> Result<Vec<CleanupCertificate>, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("filesystem outcome revision mismatch".into());
    }
    check_file_attempt_sources(unit, revision, true)?
        .iter()
        .map(|evidence| {
            let profile = profile(evidence.source().function())?;
            check_cleanup_outcomes(evidence.source(), &profile, CONTRACT)
        })
        .collect()
}
/// The record profile the retained in-process route composes under.
///
/// Public since card P04-COMPOSE so `compiler/tests/composition_route_parity.rs`
/// can assert directly that this table **is** the vocabulary the pinned
/// contract declares (P04-B1), which the moved process route derives for
/// itself. P04-B2 measured that equality end to end through certificate
/// parity; here it is asserted as an equality of profiles.
pub fn profile(function: &str) -> Result<CleanupProfile, String> {
    let field = |name: &str, c_type: &str, reset| CleanupField {
        name: name.into(),
        c_type: c_type.into(),
        reset,
    };
    let (syscall, signature, presence, fields, arguments, policy) = match function {
        "cio_file_native_close" => (
            "close",
            "int (int)",
            "fd",
            vec![field("fd", "int", ResetValue::Integer(-1))],
            vec!["fd"],
            "native_close_outcomes",
        ),
        "cio_file_native_unmap" => (
            "munmap",
            "int (void *, size_t)",
            "map",
            vec![
                field("map", "char *", ResetValue::Null),
                field("alloc_size", "unsigned long", ResetValue::Integer(0)),
                field("map_truncated_warned", "int", ResetValue::Integer(0)),
            ],
            vec!["map", "alloc_size"],
            "native_unmap_outcomes",
        ),
        _ => return Err("unsupported native cleanup function".into()),
    };
    Ok(CleanupProfile {
        function: function.into(),
        root_type: "struct cio_file *".into(),
        syscall: syscall.into(),
        syscall_signature: signature.into(),
        presence_field: presence.into(),
        fields,
        argument_fields: arguments.into_iter().map(str::to_owned).collect(),
        policy: policy.into(),
        errno_function: "__errno_location".into(),
        report_function: "cio_errno_print".into(),
    })
}

#[cfg(test)]
#[path = "../../tests/unit/file_outcomes.rs"]
mod tests;
