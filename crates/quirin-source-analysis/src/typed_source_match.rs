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

//! Reviewed-library witness boundary over generic typed C body comparison.
//! The gateway accepts arbitrary references, so its comparison result alone
//! must never become this reviewed witness through a public conversion.
use crate::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source_match::{TypedBodyComparison, compare_typed_body};
use serde::Serialize;

pub(crate) use qcore_c_gateway::typed_source_match::SourceFieldProfile;

/// Library-selected typed reference match. Construction remains private to the
/// reviewed integration routes; no public constructor, conversion or deserialize
/// implementation admits a caller's arbitrary reference as library evidence.
///
/// ```compile_fail
/// use quirin_source_analysis::typed_source_match::SourceMatch;
/// use qcore_c_gateway::typed_source_match::TypedBodyComparison;
/// fn forge(comparison: TypedBodyComparison) -> SourceMatch {
///     SourceMatch { comparison }
/// }
/// ```
///
/// ```compile_fail
/// use quirin_source_analysis::typed_source_match::SourceMatch;
/// let forged: SourceMatch = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SourceMatch {
    comparison: TypedBodyComparison,
}
impl SourceMatch {
    pub fn function(&self) -> &str {
        self.comparison.function()
    }
    pub fn source_revision(&self) -> &str {
        self.comparison.source_revision()
    }
}

/// Internal callers select a reviewed reference and independently check pins
/// and library policy. Preserve the historical serialized evidence shape.
pub(crate) fn check_source_match(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
    reference: &str,
    fields: SourceFieldProfile<'_>,
) -> Result<SourceMatch, String> {
    compare_typed_body(unit, revision, function, reference, fields)
        .map(|comparison| SourceMatch { comparison })
}
