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

//! Card **RELATIONAL-REWRITE-LINE-ORDER** (gateway rule 11r, amended in
//! place): the line order of the stream the parser reads.
//!
//! `typed_place::occurrence` places a statement on its line by binary search
//! over the parser's stream, so it declines every statement of a stream whose
//! lines are not non-decreasing. A stream rewrite that leaves a token on a
//! line before an earlier token's therefore turns off the typed occurrence of
//! every statement of the unit, silently. Every rewrite of
//! `build_function_cfg_with` keeps the order (rule 11r's and the hand-off's
//! stored-return rewrites since this card), and [`guard`] makes a stream that
//! does not keep it impossible to miss:
//!
//! * in debug and test builds it is a **hard error** (a panic naming the
//!   function and the first out-of-order pair), so a rewrite that breaks the
//!   order fails the test that exercises it;
//! * in release builds it is a **counted refusal**: the function's analysis
//!   begins with an `Unknown` instruction naming the disorder
//!   ([`REFUSAL`]), so its row is never `Safe` and the leaf counts it in every
//!   report, and the parity stream records it.
//!
//! The census (parity runs only, `QCORE_TYPED_PATH_PARITY`): one row per
//! stored-return rewrite that fired in a build, with its sites; one row per
//! rewrite stage of `build_function_cfg_with` that fired, with whether the
//! stream was sorted before and after it; one row per refusal.

use super::Token;

/// The key of the release-build refusal.
pub(super) const REFUSAL: &str = "stream-line-order";

/// The index of the first token whose line is greater than its successor's;
/// `None` when the stream's lines are non-decreasing.
pub(super) fn first_disorder(tokens: &[Token]) -> Option<usize> {
    tokens
        .windows(2)
        .position(|pair| pair[0].line > pair[1].line)
}

/// The refusal of a stream whose lines are not non-decreasing.
pub(super) fn disorder_description(tokens: &[Token]) -> Option<String> {
    let index = first_disorder(tokens)?;
    let (left, right) = (&tokens[index], &tokens[index + 1]);
    Some(format!(
        "token stream out of line order after a rewrite: `{}` on line {} stands before `{}` on \
         line {}, so no statement has a typed occurrence ({REFUSAL})",
        left.text, left.line, right.text, right.line
    ))
}

#[cfg(test)]
thread_local! {
    /// Tests only: take the release builds' counted refusal instead of the
    /// hard error ([`with_counted_refusal`]).
    static COUNTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Tests only: run `work` with the release builds' answer to a disordered
/// stream, so the refusal itself is testable in a debug build.
#[cfg(test)]
pub(super) fn with_counted_refusal<R>(work: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            COUNTED.with(|counted| counted.set(self.0));
        }
    }
    let _restore = Restore(COUNTED.with(|counted| counted.replace(true)));
    work()
}

fn hard_error() -> bool {
    #[cfg(test)]
    if COUNTED.with(std::cell::Cell::get) {
        return false;
    }
    cfg!(debug_assertions)
}

/// The guard on the stream `function`'s parser reads: `None` when its lines
/// are non-decreasing. Otherwise a hard error in debug and test builds, and
/// in release builds the refusal's description (recorded on the parity
/// stream), which the caller puts at the function's entry.
pub(super) fn guard(function: &str, tokens: &[Token]) -> Option<String> {
    let description = disorder_description(tokens)?;
    assert!(
        !hard_error(),
        "C function `{function}`: {description} (rule 11r, card RELATIONAL-REWRITE-LINE-ORDER)"
    );
    if crate::typed_access_path::parity_enabled() {
        crate::typed_access_path::record_row(serde_json::json!({
            "position": "stream-refusal",
            "function": function,
            "line": first_disorder(tokens).map_or(0, |index| tokens[index].line),
            "texts": [],
            "refusal": REFUSAL,
        }));
    }
    Some(description)
}

/// Census: a stored-return rewrite (`rewrite`: `relational-11r` or
/// `handoff-direct`) fired at `sites` (the line of each rewritten `NAME`).
pub(super) fn record_rewrite(rewrite: &str, sites: &[usize], after: &[Token]) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "position": "stream-rewrite",
        "function": "",
        "line": sites.first().copied().unwrap_or(0),
        "texts": [],
        "rewrite": rewrite,
        "sites": sites,
        "sorted_after": first_disorder(after).is_none(),
    }));
}

/// Census: the rewrite stage `stage` of `function`'s build fired, turning
/// `before` into `after`.
pub(super) fn record_stage(function: &str, stage: &str, before: &[Token], after: &[Token]) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    let disorder = first_disorder(after);
    crate::typed_access_path::record_row(serde_json::json!({
        "position": "stream-stage",
        "function": function,
        "line": disorder.map_or(0, |index| after[index].line),
        "texts": [],
        "stage": stage,
        "sorted_before": first_disorder(before).is_none(),
        "sorted_after": disorder.is_none(),
    }));
}

#[cfg(test)]
mod tests {
    use super::super::{AnalysisStatus, LegacyInstruction, analyze_cfg, build_function_cfg};
    use super::*;
    use qcore_schemas::contract::ResourceContracts;

    const SOURCE: &str = "void f(void)\n{\n  int a;\n  a = 1;\n}\n";

    /// `SOURCE` with the `int` of line 3 moved to line 9: the stream's lines
    /// are no longer non-decreasing.
    fn disordered() -> Vec<Token> {
        let mut tokens = super::super::tokenize(SOURCE);
        let at = tokens
            .iter()
            .position(|token| token.text == "int")
            .expect("the declaration");
        tokens[at].line = 9;
        tokens
    }

    #[test]
    fn the_first_disorder_is_named() {
        assert_eq!(first_disorder(&super::super::tokenize(SOURCE)), None);
        let tokens = disordered();
        let index = first_disorder(&tokens).expect("disordered");
        assert_eq!(tokens[index].text, "int");
        let description = disorder_description(&tokens).expect("refused");
        assert!(
            description.contains("`int` on line 9 stands before `a` on line 3"),
            "{description}"
        );
        assert!(
            description.ends_with("(stream-line-order)"),
            "{description}"
        );
    }

    /// Debug and test builds: a disordered stream is a hard error.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "stream-line-order")]
    fn a_disordered_stream_is_a_hard_error_in_test_builds() {
        let _ = build_function_cfg(&disordered(), "f", &ResourceContracts::default());
    }

    /// Release builds' answer, taken here explicitly: the function's first
    /// instruction is the refusal, and the function is `Unknown`.
    #[test]
    fn a_disordered_stream_is_a_counted_refusal_in_release_builds() {
        let refused = |node: &super::super::LegacyCfgNode| {
            matches!(
                &node.instruction,
                LegacyInstruction::Unknown { description, .. } if description.contains(REFUSAL)
            )
        };
        let cfg = with_counted_refusal(|| {
            build_function_cfg(&disordered(), "f", &ResourceContracts::default())
        })
        .expect("built");
        assert!(cfg.nodes.iter().any(refused));
        assert_eq!(analyze_cfg(cfg).status, AnalysisStatus::Unknown);
        // The same body in order is analysed as before.
        let sorted = super::super::tokenize(SOURCE);
        let cfg = build_function_cfg(&sorted, "f", &ResourceContracts::default()).expect("built");
        assert!(!cfg.nodes.iter().any(refused));
        assert_eq!(analyze_cfg(cfg).status, AnalysisStatus::Safe);
    }
}
