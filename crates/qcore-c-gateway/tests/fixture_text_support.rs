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

#[path = "support/text.rs"]
mod text;

use text::normalize_newlines;

#[test]
fn newline_normalization_preserves_lf_syntax() {
    let source = "module contract;\ncontract {\n};\n";
    assert_eq!(normalize_newlines(source), source);
}

#[test]
fn newline_normalization_maps_crlf_and_mixed_syntax_to_lf() {
    assert_eq!(
        normalize_newlines("module contract;\r\ncontract {\n};\r\n"),
        "module contract;\ncontract {\n};\n"
    );
}

#[test]
fn newline_normalization_is_idempotent() {
    let once = normalize_newlines("a\r\nb\nc\r\n");
    assert_eq!(normalize_newlines(&once), once);
}
