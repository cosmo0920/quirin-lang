//! Cross-platform text handling for syntax fixtures included at compile time.

/// Return `source` with every Windows CRLF line ending represented as LF.
///
/// Tests may use the returned text for syntax-preserving substitutions without
/// depending on Git's checkout policy. Callers that assert a substitution took
/// place must compare against this normalized baseline, not the original text.
pub fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n")
}
