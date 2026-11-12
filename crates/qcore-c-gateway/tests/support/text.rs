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

//! Cross-platform text handling for syntax fixtures included at compile time.

/// Return `source` with every Windows CRLF line ending represented as LF.
///
/// Tests may use the returned text for syntax-preserving substitutions without
/// depending on Git's checkout policy. Callers that assert a substitution took
/// place must compare against this normalized baseline, not the original text.
pub fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n")
}

/// `source` after the Apache-2.0 notice a tracked file opens with (card
/// APACHE-2.0-CONTRACT-HEADERS): the `// SPDX-License-Identifier:` line, the
/// `//` notice lines that follow it and the one blank line closing the block.
/// A file that does not open with that line is returned unchanged; nothing but
/// the notice is ever skipped, so a module header behind any other text still
/// fails its caller's check. Expects newline-normalized text.
#[allow(dead_code)]
pub fn strip_licence_notice(source: &str) -> &str {
    if !source.starts_with("// SPDX-License-Identifier: ") {
        return source;
    }
    let mut rest = source;
    while rest.starts_with("//") {
        match rest.find('\n') {
            Some(end) => rest = &rest[end + 1..],
            None => return "",
        }
    }
    rest.strip_prefix('\n').unwrap_or(rest)
}
