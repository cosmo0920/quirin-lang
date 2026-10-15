//! Card **RELATIONAL-REWRITE-LINE-ORDER** (gateway rule 11r, amended in
//! place): the line order of the stream the parser reads.
//!
//! `typed_place::occurrence` places a statement on its line by binary search
//! over the parser's stream, so it declines every statement of a stream whose
//! lines are not non-decreasing. A stream rewrite that leaves a token on a
//! line before an earlier token's therefore turns off the typed occurrence of
//! every statement of the unit, silently.
//!
//! The census (parity runs only, `QCORE_TYPED_PATH_PARITY`): one row per
//! stream rewrite that fired in a build, with its sites, and one row per
//! rewrite stage of `build_function_cfg_with` that fired, with whether the
//! stream was sorted before and after it. Nothing here decides anything.

use super::Token;

/// The index of the first token whose line is greater than its successor's;
/// `None` when the stream's lines are non-decreasing.
pub(super) fn first_disorder(tokens: &[Token]) -> Option<usize> {
    tokens
        .windows(2)
        .position(|pair| pair[0].line > pair[1].line)
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
