//! Card **RDHTTP-HEADER-SUMMARY-GAP** (gateway rules 13o, 13p): the
//! whole-unit typed AST index of a unit whose Clang JSON nests deeper than
//! `serde_json`'s default recursion limit.
//!
//! # The gap
//!
//! `serde_json` refuses a document nesting 128 levels or more (its default
//! recursion limit, a stack guard). Clang nests a statement two JSON levels per
//! AST level, so a deep expression or a long `else if` chain exceeds it. The
//! measured instance is librdkafka's `rdhttp.c` (224 levels: four definitions
//! calling `curl_easy_setopt`, whose GCC type-check macro from
//! `curl/typecheck-gcc.h` expands into nested conditionals). There the
//! whole-unit parse failed, the unit had no index, and
//! [`crate::typed_access_path::StatementIndex::read`] answered `None` for every
//! definition of the unit, header bodies included: `rd_malloc`'s
//! `rd_assert(p)` (glibc's `assert`, a GNU statement expression under `gnu17`,
//! rule 11g) was never read, W4 published no summary, and the unit examined
//! the body with no evidence, which kept the declaration a declared
//! assumption (rule 13g).
//!
//! # What this module decides
//!
//! * **Rule 13o.** [`from_slice`] parses as `serde_json::from_slice` does.
//!   Only when that fails is the document's nesting measured ([`depth`],
//!   counted exactly as `serde_json` counts it: each `[` or `{` outside a
//!   string one level); a document within [`ADMITTED_DEPTH`] (256, twice the
//!   default) is parsed again with the limit disabled, on a dedicated thread
//!   whose stack is sized for it ([`PARSE_STACK`]), as the typed dispatch
//!   reader and the index-extent reader already do. The text is the same and
//!   the parser is the same; the parsed value is the one the default parse
//!   would produce without its guard. Every reader then reads the unit as it
//!   reads any other: no reader's admission changes.
//! * **Rule 13p.** A document beyond the bound, a failed reparse (malformed
//!   JSON) or a thread that cannot start returns the default parse's own
//!   error: the unit keeps no index, exactly as before, and a body the unit
//!   examines without evidence stays `unvalidated` in the rule-13g fold. The
//!   fold is never told to skip a unit.
//!
//! `QCore.DeepUnitAst` states the depth scan, the admission and its
//! consequences; [`DEPTH_VECTORS`] are its shared vectors.

use serde::de::DeserializeOwned;

/// `serde_json`'s default recursion limit: a document nesting this many
/// levels or more is refused by the default parse.
pub(crate) const DEFAULT_LIMIT: usize = 128;

/// Rule 13o: the deepest whole-unit document read without the default limit.
pub(crate) const ADMITTED_DEPTH: usize = 2 * DEFAULT_LIMIT;

/// The reparse's own stack: far beyond what [`ADMITTED_DEPTH`] levels of
/// `serde_json`'s recursive descent need.
pub(crate) const PARSE_STACK: usize = 256 << 20;

/// The nesting depth of a JSON text as `serde_json` counts it: the greatest
/// number of `[` and `{` open at once, outside strings (a `\` escapes the next
/// byte inside a string).
pub(crate) fn depth(bytes: &[u8]) -> usize {
    let (mut open, mut deepest) = (0usize, 0usize);
    let (mut in_string, mut escaped) = (false, false);
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'[' | b'{' => {
                    open += 1;
                    deepest = deepest.max(open);
                }
                b']' | b'}' => open = open.saturating_sub(1),
                _ => {}
            }
        }
    }
    deepest
}

/// Rule 13o: whether a document the default parse refused is read again
/// without the limit.
pub(crate) fn admitted(depth: usize) -> bool {
    depth <= ADMITTED_DEPTH
}

/// Rules 13o and 13p: `serde_json::from_slice`, and, when it fails on a
/// document within [`ADMITTED_DEPTH`], the same parse without the default
/// limit on a dedicated stack. Otherwise the default parse's error.
pub(crate) fn from_slice<T: DeserializeOwned + Send>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    let error = match serde_json::from_slice(bytes) {
        Ok(value) => return Ok(value),
        Err(error) => error,
    };
    if !admitted(depth(bytes)) {
        return Err(error);
    }
    match unlimited(bytes) {
        Some(Ok(value)) => Ok(value),
        Some(Err(_)) | None => Err(error),
    }
}

/// The parse without the default limit, on its own stack. `None` when the
/// thread cannot start or panics.
fn unlimited<T: DeserializeOwned + Send>(bytes: &[u8]) -> Option<Result<T, serde_json::Error>> {
    std::thread::scope(|scope| {
        let parse = std::thread::Builder::new()
            .stack_size(PARSE_STACK)
            .spawn_scoped(scope, || {
                let mut deserializer = serde_json::Deserializer::from_slice(bytes);
                deserializer.disable_recursion_limit();
                let value = T::deserialize(&mut deserializer)?;
                // Trailing bytes are refused, as `from_slice` refuses them.
                deserializer.end()?;
                Ok(value)
            })
            .ok()?;
        parse.join().ok()
    })
}

/// Shared vectors with `QCore.DeepUnitAst.depthVectors`: a text, its depth,
/// whether the default parse reads it, and whether rule 13o reads it.
#[cfg(test)]
pub(crate) const DEPTH_VECTORS: &[(&str, usize, bool, bool)] = &[
    ("1", 0, true, true),
    ("[]", 1, true, true),
    ("{\"a\":[1,{\"b\":[]}]}", 4, true, true),
    ("\"[[[{\"", 0, true, true),
    ("[\"\\\"[\",[]]", 2, true, true),
    ("{\"s\":\"\\\\\",\"t\":[[]]}", 3, true, true),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn nested(levels: usize) -> String {
        format!("{}{}", "[".repeat(levels), "]".repeat(levels))
    }

    /// Whether the default parse reads a document of this depth (measured
    /// against `serde_json` itself below).
    fn default_reads(depth: usize) -> bool {
        depth < DEFAULT_LIMIT
    }

    #[test]
    fn depth_vectors_hold() {
        for &(text, expected, default, after) in DEPTH_VECTORS {
            assert_eq!(depth(text.as_bytes()), expected, "{text}");
            assert_eq!(default_reads(expected), default, "{text}");
            assert_eq!(
                default_reads(expected) || admitted(expected),
                after,
                "{text}"
            );
            let parsed = from_slice::<Value>(text.as_bytes()).is_ok();
            assert_eq!(parsed, after, "{text}");
        }
    }

    #[test]
    fn the_default_limit_is_serde_jsons() {
        assert!(serde_json::from_str::<Value>(&nested(DEFAULT_LIMIT - 1)).is_ok());
        assert!(serde_json::from_str::<Value>(&nested(DEFAULT_LIMIT)).is_err());
        assert_eq!(depth(nested(DEFAULT_LIMIT).as_bytes()), DEFAULT_LIMIT);
    }

    #[test]
    fn a_deep_document_within_the_bound_is_the_same_value() {
        for levels in [DEFAULT_LIMIT, 224, ADMITTED_DEPTH] {
            let text = nested(levels);
            let value: Value = from_slice(text.as_bytes()).expect("admitted");
            // The same value the unguarded parser reads.
            let mut deserializer = serde_json::Deserializer::from_str(&text);
            deserializer.disable_recursion_limit();
            let unguarded =
                <Value as serde::Deserialize>::deserialize(&mut deserializer).expect("unguarded");
            assert_eq!(value, unguarded, "{levels}");
        }
    }

    #[test]
    fn beyond_the_bound_or_malformed_the_default_error_stands() {
        assert!(from_slice::<Value>(nested(ADMITTED_DEPTH + 1).as_bytes()).is_err());
        // Deep and malformed: the reparse fails too, and the error is the
        // default parse's.
        let malformed = format!("{}x", nested(200));
        let error = from_slice::<Value>(malformed.as_bytes()).expect_err("malformed");
        let default = serde_json::from_slice::<Value>(malformed.as_bytes()).expect_err("default");
        assert_eq!(error.to_string(), default.to_string());
        // A shallow document parses exactly as the default parse does.
        let shallow = "{\"kind\":\"TranslationUnitDecl\",\"inner\":[]}";
        assert_eq!(
            from_slice::<Value>(shallow.as_bytes()).expect("shallow"),
            serde_json::from_str::<Value>(shallow).expect("default")
        );
    }
}
