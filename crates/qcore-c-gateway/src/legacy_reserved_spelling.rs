//! Card **W45-SR5-SOUNDNESS**: a held or withdrawn call is only ever minted
//! by the tool, never read back from a name the source spells
//! (`HELD-PREFIX-SOURCE-FORGES-PENDING`).
//!
//! Six rewrites mark a call site by prefixing its callee's name, and every
//! later reader classifies a call by that prefix alone: a pending review
//! (`qcore_review_pending__`) and a pin mismatch (`qcore_pin_mismatch__`),
//! both `Pending` since card W45-USER-CONTRACT-PLUGINS; a refuted declaration
//! (`qcore_refuted_by_body__`), an ABI or mode withdrawal
//! (`qcore_abi_withdrawn__`, `qcore_mode_withdrawn__`) and an unmet literal
//! premise (`qcore_premise_unmet__`), all `Unknown`. STABILITY-REEVAL-5
//! measured that a source spelling `qcore_pin_mismatch__zz(0);` published
//! `Pending` with the reason `contract-pin-mismatch` where `Unknown` was due:
//! a human-resolvable reason the tool never decided, lowering the verdict.
//!
//! [`escape`] runs on the token stream before the first of those rewrites, on
//! every route that applies them. It renames every identifier the source
//! spells with a reserved prefix, the escape prefix included, by prepending
//! [`ESCAPE_PREFIX`]. The renaming is consistent across the unit, so a
//! definition and its calls still meet, and after it no source token starts
//! with a marker prefix: every marked name a later reader sees is one a
//! rewrite minted from the environment's own state (the withheld, pending,
//! refuted or withdrawn sets). An escaped callee is an uncontracted name and
//! refuses as one, under [`escaped_call_description`]; nothing it names is
//! `Pending`. A unit-defined function the source named with a reserved prefix
//! is analysed under its escaped name, so a selection naming its original
//! spelling finds no row and the unit is not `Safe` (the unread-definition
//! gate); the corpus spells none of these prefixes.
//!
//! Monotone in the refusing direction: only names that were marked (and so
//! never `Safe`) change, each to an uncontracted name, which is `Unknown`.
//! `QCore.ReservedSpelling`.

use super::Token;

/// The prefix an escaped source identifier carries.
pub(crate) const ESCAPE_PREFIX: &str = "qcore_source_spelled__";

/// Every prefix a rewrite reserves, and the escape prefix itself.
fn reserved_prefixes() -> [&'static str; 13] {
    [
        // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the dialect-refusal marker.
        crate::c_frontend_identity::DIALECT_REFUSED_PREFIX,
        // Card CG23-RELATIONAL-POSTCONDITIONS: a disposition class's name.
        crate::relational_post::CLASS_PREFIX,
        // Card CALLEE-RETENTION-CLASS1 (rule 12t): a retaining callee's
        // custody contract, read only where its retentions are decided.
        crate::retained_by::RETAINED_PREFIX,
        super::review_hold::HELD_PREFIX,
        super::pin_hold::HELD_PREFIX,
        super::refuted_contract::HELD_PREFIX,
        super::declared_abi::WITHDRAWN_PREFIX,
        super::declared_modes::WITHDRAWN_PREFIX,
        super::literal_premise::UNMET_PREFIX,
        // Card W48-RK-LOG-CALLER-FRAME (rule 32a).
        super::caller_frame_hold::HELD_PREFIX,
        // Card CONTRACT-SUPERSESSION.
        super::conflict_hold::HELD_PREFIX,
        // Card MEMBER-EFFECT-SUMMARY (rule 38j): a member-effect contract.
        super::member_effect::MINTED_PREFIX,
        ESCAPE_PREFIX,
    ]
}

/// Whether `text` starts with a prefix a rewrite reserves.
fn is_reserved(text: &str) -> bool {
    reserved_prefixes()
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// Rename every source identifier spelled with a reserved prefix. `None`
/// means the stream is unchanged, which is every stream that spells none.
pub(crate) fn escape(tokens: &[Token]) -> Option<Vec<Token>> {
    let mut escaped: Option<Vec<Token>> = None;
    for (index, token) in tokens.iter().enumerate() {
        if !is_reserved(&token.text) {
            continue;
        }
        let target = escaped.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{ESCAPE_PREFIX}{}", token.text);
    }
    escaped
}

/// [`escape`] over an owned stream.
pub(crate) fn escape_owned(tokens: Vec<Token>) -> Vec<Token> {
    escape(&tokens).unwrap_or(tokens)
}

/// The source spelling an escaped name stands for.
pub(crate) fn escaped_callee(function: &str) -> Option<&str> {
    function.strip_prefix(ESCAPE_PREFIX)
}

/// The refusal a call to an escaped name carries.
#[must_use]
pub fn escaped_call_description(callee: &str) -> String {
    format!(
        "call to `{callee}`, whose name spells a prefix the analyzer reserves for its own \
         rewrites (reserved-prefix-in-source)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(texts: &[&str]) -> Vec<Token> {
        texts
            .iter()
            .map(|text| Token {
                text: (*text).to_owned(),
                line: 1,
            })
            .collect()
    }

    fn texts(tokens: &[Token]) -> Vec<&str> {
        tokens.iter().map(|token| token.text.as_str()).collect()
    }

    #[test]
    fn a_stream_without_a_reserved_spelling_is_unchanged() {
        assert!(escape(&tokens(&["zz", "(", "0", ")", ";"])).is_none());
    }

    #[test]
    fn every_reserved_prefix_and_the_escape_itself_are_escaped() {
        for prefix in reserved_prefixes() {
            let name = format!("{prefix}zz");
            let escaped = escape(&tokens(&[&name, "(", "0", ")"])).expect("escaped");
            assert_eq!(texts(&escaped)[0], format!("{ESCAPE_PREFIX}{name}"));
            assert!(
                reserved_prefixes()[..8]
                    .iter()
                    .all(|marker| !texts(&escaped)[0].starts_with(marker)),
                "{prefix}"
            );
            assert_eq!(escaped_callee(texts(&escaped)[0]), Some(name.as_str()));
        }
    }

    #[test]
    fn a_definition_and_its_call_are_renamed_alike() {
        let escaped = escape(&tokens(&[
            "void",
            "qcore_pin_mismatch__zz",
            "(",
            ")",
            "{",
            "}",
            "qcore_pin_mismatch__zz",
            "(",
            ")",
        ]))
        .expect("escaped");
        assert_eq!(texts(&escaped)[1], texts(&escaped)[6]);
    }
}
