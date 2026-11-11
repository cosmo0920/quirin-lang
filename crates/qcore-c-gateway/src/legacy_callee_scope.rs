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

//! Card **W44-CORPUS-CALLEE-SUMMARIES**: the definitions W4 derives custody
//! summaries for.
//!
//! # The route gap this closes
//!
//! Under [`super::LegacyFunctionScope::MainSourceDefinitions`] (every pinned
//! corpus unit), card E01's main-source enumeration selects the definitions a
//! unit *reports*, and until this card the same selection also bounded the
//! definitions W4 *derived summaries for*. A `static inline` helper that a
//! header defines in the translation unit — `rd_kafka_buf_write_i8`,
//! `rd_list_elem`, `mk_list_add` — is compiled into the unit, is the body every
//! call in the unit reaches, and is analysed without selection under
//! [`super::LegacyFunctionScope::PreprocessedTranslationUnit`]. Under the
//! main-source scope it was nevertheless never offered to
//! [`crate::legacy_summary::derive_custody_summaries`], so every call to it
//! stayed `call to uncontracted function` whatever its body proved
//! (`LOADER-ABI-MODES-SELECTION` recorded the same gap from the declared side).
//!
//! # The rule
//!
//! The derivation scope is the selected definitions together with every
//! definition they reach through direct calls, transitively, that is
//!
//! 1. **not selected** — a selected definition is already in scope;
//! 2. **of internal linkage**, read off the definition's own declaration
//!    specifiers: `static` present and `extern` absent. C11 6.2.2p3 gives such
//!    an identifier internal linkage, so every call to it in this translation
//!    unit denotes *this* body and no other. A definition without `static` is
//!    refused, because an `inline` definition of external linkage may be
//!    bypassed for the external definition at any call (C11 6.7.4p7) and an
//!    ordinary external definition may be interposed; either way the body
//!    analysed here need not be the body the call runs.
//!
//! Card W44-CORPUS-CALLEE-SUMMARIES added a third condition, **not
//! implementation-reserved** (a name beginning `__` or `_` and an uppercase
//! letter, C11 7.1.3), leaving a C library header body such as `__bswap_32` to
//! the libc provider's decision. Card **W44-LIBC-RESIDUALS** made that
//! decision: a reserved-name definition is admitted on exactly the two
//! conditions above, so glibc's `static __inline` `__bswap_16/32/64` and
//! `__uint16/32/64_identity` bodies are *checked*, not declared. The reason the
//! filter existed was ownership, not soundness: the linkage premise is what
//! makes every call run the analysed body, and the name's reservation changes
//! nothing about that. The filter is lifted only where the unit carries the
//! body: a candidate is a definition in this unit's token stream whose
//! internal linkage is read off that definition, so a reserved name the unit
//! only declares, or calls without defining, is never a candidate and its calls
//! stay uncontracted. `quirin-provider-libc` declares none of these names, so no
//! declared contract and no derived summary both apply to one of them.
//!
//! Nothing else changes. A definition admitted to the scope is offered to the
//! **unchanged** W4 derivation: its summary is published only after
//! `observations_match_contract` validated it against a fresh analysis of its
//! own body in this unit, declared contracts still take precedence (and a
//! disagreement is still recorded, never resolved), and the declared-mode gate
//! compares exactly as it does for a selected definition. The scope never
//! changes which functions the unit **reports**: the analysed rows, the
//! effective entry and the whole-unit `Safe` scope are still the selection
//! alone, so a helper whose body does not validate simply publishes nothing and
//! its call sites keep their `call to uncontracted function` refusal.
//!
//! The certificate checker re-analyses the preprocessed source through the same
//! function with the same selection, so it re-derives exactly this scope and
//! exactly these summaries; a summary is never taken on the certificate's word.
//!
//! # Lean
//!
//! `QCore.CalleeSummaryScope` models the scope, the linkage premise and call
//! resolution. It proves that every scope member is selected or an
//! internal-linkage definition the unit carries, that a name the unit does
//! not define is never admitted, that reservation is not read, that the calls to a newly
//! admitted callee run only the body analysed here, that a declared contract is
//! never overridden and an inferred one is installed exactly as validated, and
//! that a summary derived before the widening survives it unchanged. As with
//! every gateway module, the Rust–Lean correspondence is maintained by review
//! (trust boundary 28).

use std::collections::BTreeSet;

use super::{Token, direct_calls, find_function_body};

/// The definitions W4 derives summaries for: `selected` together with the
/// internal-linkage definitions they reach. `definitions` is every definition
/// in the unit's token stream; `selection_applied` says whether a main-source
/// selection narrowed `selected` from it at all. Without a selection every
/// definition is already in scope, and the answer is `selected` unchanged.
pub(crate) fn derivation_scope(
    tokens: &[Token],
    selected: &[String],
    definitions: &[String],
    selection_applied: bool,
) -> Vec<String> {
    if !selection_applied {
        return selected.to_vec();
    }
    let chosen = selected.iter().cloned().collect::<BTreeSet<_>>();
    // (1)+(2) The candidates: unselected definitions of internal linkage,
    // reserved names included (card W44-LIBC-RESIDUALS).
    let candidates = definitions
        .iter()
        .filter(|name| !chosen.contains(*name))
        .filter(|name| has_internal_linkage(tokens, name))
        .cloned()
        .collect::<BTreeSet<_>>();
    if candidates.is_empty() {
        return selected.to_vec();
    }
    // Reachability through direct calls, from the selection. The admitted set
    // only grows and is bounded by `candidates`, so the work list terminates.
    let mut admitted = BTreeSet::new();
    let mut work = selected.to_vec();
    while let Some(name) = work.pop() {
        for callee in direct_calls(tokens, &name, &candidates) {
            if admitted.insert(callee.clone()) {
                work.push(callee);
            }
        }
    }
    let mut scope = selected.to_vec();
    scope.extend(admitted);
    scope
}

/// Whether `function`'s definition carries `static` and not `extern` among the
/// tokens that precede its name back to the previous file-scope boundary
/// (`;`, `{` or `}`), which is where its declaration specifiers are.
///
/// Reads the *definition* only. A definition whose `static` is spelled on an
/// earlier declaration alone also has internal linkage (C11 6.2.2p5), but it is
/// refused here: the reader answers "not established", which keeps the call
/// uncontracted.
pub(crate) fn has_internal_linkage(tokens: &[Token], function: &str) -> bool {
    definition_has_specifier(tokens, function, "static")
        && !definition_has_specifier(tokens, function, "extern")
}

/// Conservative spelling check on a definition's own specifier sequence.
/// This is not a whole-program linkage decision; cross-unit transport still
/// requires an explicit linked-program resolution premise.
pub(crate) fn definition_has_specifier(tokens: &[Token], function: &str, specifier: &str) -> bool {
    let Some((body_start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(name_index) = definition_name_index(tokens, function, body_start) else {
        return false;
    };
    for token in tokens[..name_index].iter().rev() {
        match token.text.as_str() {
            ";" | "{" | "}" => break,
            word if word == specifier => return true,
            _ => {}
        }
    }
    false
}

/// The index of the name token whose parameter list closes at `body_start - 1`.
fn definition_name_index(tokens: &[Token], function: &str, body_start: usize) -> Option<usize> {
    let close = body_start.checked_sub(1)?;
    if tokens.get(close)?.text != ")" {
        return None;
    }
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match tokens[index].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let name = index.checked_sub(1)?;
                    return (tokens[name].text == function).then_some(name);
                }
            }
            _ => {}
        }
    }
    None
}

/// The census stream (card W44-CORPUS-CALLEE-SUMMARIES). Compiled only with the
/// off-by-default `indirect-census` feature, and written only when
/// [`census::CALLEE_SCOPE_CENSUS_PATH_VARIABLE`] names a path. It re-derives the
/// widened scope a second time on its own copy and reads what that derivation
/// decided; nothing it computes reaches the analysis, so it moves no verdict.
#[cfg(feature = "indirect-census")]
pub(crate) mod census {
    use std::collections::BTreeSet;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::sync::Mutex;

    use qcore_schemas::contract::ResourceContracts;

    use super::super::{AnalysisStatus, LegacyDiagnosticKind, Token, analyze_defined_function};
    use super::{derivation_scope, has_internal_linkage};
    use crate::legacy_c::declared_modes::ModeGate;

    /// The environment variable naming the JSON-lines census stream.
    pub const CALLEE_SCOPE_CENSUS_PATH_VARIABLE: &str = "QCORE_CALLEE_SCOPE_CENSUS";

    static SINK: Mutex<Option<Option<BufWriter<File>>>> = Mutex::new(None);

    /// One unselected definition the unit's selection calls.
    #[derive(Clone, Debug, serde::Serialize)]
    pub struct CalleeRow {
        pub name: String,
        /// `static` present and `extern` absent on the definition.
        pub internal_linkage: bool,
        /// Called directly by a selected definition (otherwise only through
        /// another admitted definition).
        pub direct: bool,
        /// `validated`, `no_summary`, `declared_agreed`, `declared_conflict`,
        /// `declared_no_summary`, or `refused_linkage`.
        pub outcome: String,
        /// The validated summary, as its `Debug` text.
        pub summary: Option<String>,
        /// The status of the body's own analysis under the widened derivation.
        pub body_status: Option<String>,
        /// Its distinct blocker descriptions, most frequent first (at most 6).
        pub body_blockers: Vec<(String, usize)>,
    }

    /// One unit's record.
    #[derive(Clone, Debug, serde::Serialize)]
    pub struct UnitRow {
        pub unit_seq: u64,
        pub selected: Vec<String>,
        pub callees: Vec<CalleeRow>,
    }

    fn describe(kind: &LegacyDiagnosticKind) -> String {
        match kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            LegacyDiagnosticKind::UnknownAlias { variable } => {
                format!("unresolved resource identity `{variable}`")
            }
            other => format!("{other:?}")
                .split([' ', '{', '('])
                .next()
                .unwrap_or_default()
                .to_owned(),
        }
    }

    /// Record the widened scope's decisions for one unit. A no-op unless the
    /// variable names a path, and a no-op without a selection.
    pub(crate) fn record(
        tokens: &[Token],
        selected: &[String],
        definitions: &[String],
        contracts: &ResourceContracts,
    ) {
        if std::env::var_os(CALLEE_SCOPE_CENSUS_PATH_VARIABLE).is_none() {
            return;
        }
        let chosen = selected.iter().cloned().collect::<BTreeSet<_>>();
        let unselected = definitions
            .iter()
            .filter(|name| !chosen.contains(*name))
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut direct = BTreeSet::new();
        for name in selected {
            direct.extend(super::direct_calls(tokens, name, &unselected));
        }
        let scope = derivation_scope(tokens, selected, definitions, true);
        let admitted = scope
            .iter()
            .filter(|name| !chosen.contains(*name))
            .cloned()
            .collect::<BTreeSet<_>>();
        let gate = ModeGate::apply(tokens, &scope, contracts);
        let mut derivation = gate.derivation;
        let rewritten = gate.tokens;
        let tokens = rewritten.as_deref().unwrap_or(tokens);
        let mut callees = Vec::new();
        let mut names = admitted.clone();
        names.extend(direct.iter().cloned());
        for name in names {
            let internal = has_internal_linkage(tokens, &name);
            let mut row = CalleeRow {
                name: name.clone(),
                internal_linkage: internal,
                direct: direct.contains(&name),
                outcome: String::new(),
                summary: None,
                body_status: None,
                body_blockers: Vec::new(),
            };
            if !admitted.contains(&name) {
                row.outcome = "refused_linkage".to_owned();
                callees.push(row);
                continue;
            }
            let inferred = derivation.inferred(&name).cloned();
            row.outcome = match (contracts.contract(&name), &inferred) {
                (Some(_), None) => "declared_no_summary",
                (Some(declared), Some(inferred)) if declared == inferred => "declared_agreed",
                (Some(_), Some(_)) => "declared_conflict",
                (None, Some(_)) => "validated",
                (None, None) => "no_summary",
            }
            .to_owned();
            row.summary = inferred.map(|contract| format!("{contract:?}"));
            if let Ok(summary) = analyze_defined_function(tokens, &name, &mut derivation) {
                row.body_status = Some(
                    match summary.status {
                        AnalysisStatus::Safe => "Safe",
                        AnalysisStatus::Unsafe => "Unsafe",
                        AnalysisStatus::Unknown => "Unknown",
                        AnalysisStatus::Pending => "Pending",
                    }
                    .to_owned(),
                );
                let mut counts = std::collections::BTreeMap::<String, usize>::new();
                for diagnostic in &summary.diagnostics {
                    *counts.entry(describe(&diagnostic.kind)).or_default() += 1;
                }
                let mut counts = counts.into_iter().collect::<Vec<_>>();
                counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                counts.truncate(6);
                row.body_blockers = counts;
            }
            callees.push(row);
        }
        let row = UnitRow {
            unit_seq: crate::indirect_census::unit_seq(),
            selected: selected.to_vec(),
            callees,
        };
        let Ok(mut guard) = SINK.lock() else {
            return;
        };
        let opened = guard.get_or_insert_with(|| {
            let path = std::env::var_os(CALLEE_SCOPE_CENSUS_PATH_VARIABLE)?;
            File::create(path).ok().map(BufWriter::new)
        });
        let Some(writer) = opened.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(&row) {
            let _ = writeln!(writer, "{line}");
            let _ = writer.flush();
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_callee_scope.rs"]
mod tests;
