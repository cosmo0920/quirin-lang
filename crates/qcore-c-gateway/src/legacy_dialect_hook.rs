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

//! Card **CG30-DIALECTS-FLAGS-TYPES** (gateway rule 11y): route G's two thin
//! hooks into `analyze_translation_unit_under_holds`. Everything they decide
//! lives in `crate::c_frontend_identity`.
//!
//! 1. [`refuse`], before any reader: the unit's own target selections are
//!    admitted under the LP64 premise (a refusal refuses the unit); under a
//!    dialect with registered
//!    differences (C23, C2y), a digit separator refuses the unit, and each
//!    definition spelling a registered difference has its body withdrawn to
//!    one call of a reserved marker no environment contracts. The four baseline
//!    dialects and C17 register none, so their streams are returned untouched.
//! 2. [`name_refusals`], per analysed row: the dialect refusal by name, and an
//!    extension type read from the definition's typed AST where the unit spells
//!    one. A unit spelling none is never asked for an AST by this hook.

use std::collections::BTreeMap;

use super::{
    CCompilerDialect, CPreprocessorOption, ConditionalMark, LegacyCError, LegacyCertificateRequest,
    LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionSummary, Token, classify_diagnostics,
    find_function_body, function_names,
};
use crate::c_frontend_identity as identity;

/// Hook 1. Returns the stream every later reader sees and each withdrawn
/// definition's refusal.
pub(super) fn refuse(
    dialect: CCompilerDialect,
    request: Option<&LegacyCertificateRequest>,
    source: &str,
    tokens: Vec<Token>,
    conditionals: &[ConditionalMark],
) -> Result<(Vec<Token>, BTreeMap<String, String>), LegacyCError> {
    // The unit's own target selections, admitted under the readers' LP64
    // premise whoever built the unit (not only through the compile database).
    if let Some(request) = request {
        identity::CTargetIdentity::admit_flags(request.preprocessor_options.iter().filter_map(
            |option| match option {
                CPreprocessorOption::FrontendFlag(flag) => Some(flag),
                _ => None,
            },
        ))
        .map_err(LegacyCError::ProductionParserRejected)?;
    }
    if identity::dialect_differences(dialect).is_empty() {
        return Ok((tokens, BTreeMap::new()));
    }
    if let Some(reason) = identity::digit_separator_refusal(dialect, source) {
        return Err(LegacyCError::ProductionParserRejected(reason));
    }
    let definitions = function_names(&tokens, conditionals).definitions;
    let (tokens, refusals) = identity::refuse_dialect_differences(dialect, tokens, &definitions);
    // Conditional marks index the stream; a withdrawn body moves every later
    // index. Preprocessed input has none, so this only refuses raw input.
    if !refusals.is_empty() && !conditionals.is_empty() {
        return Err(LegacyCError::ProductionParserRejected(format!(
            "registered {} differences cannot be withdrawn from a stream with conditional directives (CG30)",
            dialect.std_name()
        )));
    }
    Ok((tokens, refusals))
}

/// Whether the unit spells any extension (the prefilter for hook 2).
pub(super) struct Extensions {
    spelled: bool,
}

impl Extensions {
    pub(super) fn of(tokens: &[Token]) -> Self {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        Self {
            spelled: identity::unit_spells_extension(&texts),
        }
    }
}

/// Hook 2. Adds each refusal as an `Unsupported` diagnostic of the row and
/// re-classifies it; a row with none is untouched.
pub(super) fn name_refusals(
    summary: &mut LegacyFunctionSummary,
    tokens: &[Token],
    name: &str,
    dialect_refusals: &BTreeMap<String, String>,
    extensions: &Extensions,
) {
    let mut descriptions = Vec::new();
    if let Some(refusal) = dialect_refusals.get(name) {
        descriptions.push(refusal.clone());
    }
    if extensions.spelled
        && let Some(refusal) = extension_refusal(name)
    {
        descriptions.push(refusal);
    }
    if descriptions.is_empty() {
        return;
    }
    let line = find_function_body(tokens, name).map_or(0, |(_, line)| line);
    for description in descriptions {
        summary.diagnostics.push(LegacyDiagnostic {
            function: name.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported { description },
        });
    }
    summary.diagnostics.sort();
    summary.diagnostics.dedup();
    summary.status = classify_diagnostics(&summary.diagnostics);
}

/// The definition's extension use, from its typed AST: the unit's index, or,
/// where the unit's AST cannot be indexed, the definition's own filtered AST.
/// A definition whose AST cannot be read either way, in a unit that spells an
/// extension, is refused as well: the use cannot be excluded.
fn extension_refusal(name: &str) -> Option<String> {
    let index = crate::typed_access_path::StatementIndex::read(name);
    let fallback;
    let ast = match &index {
        Some(index) => Some(index.function.as_ref()),
        None => {
            fallback = crate::typed_access_path::definition_ast(name);
            fallback.as_ref()
        }
    };
    match ast {
        Some(ast) => identity::extension_in_ast(ast, &crate::typed_access_path::type_id)
            .map(|(family, spelling)| identity::extension_refusal(family, &spelling)),
        None => Some(
            "C extension types could not be excluded: the definition's typed AST is unavailable (CG30)"
                .into(),
        ),
    }
}
