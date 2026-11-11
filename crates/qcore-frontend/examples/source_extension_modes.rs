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

//! Batch parser-derived intent for packaging; not declaration verification.
//! JSON strings are escaped here to keep the frontend's dependency boundary.
use qcore_frontend::{frontend::parse_source_module_mode, source_extension::mode_name};
use std::fmt::Write;

fn json_string(text: &str) -> String {
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c < ' ' => write!(&mut result, "\\u{:04x}", c as u32).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn main() -> Result<(), String> {
    let mut rows = Vec::new();
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let path = json_string(&path);
        rows.push(match parse_source_module_mode(&source) {
            Ok(mode) => format!(
                "{{\"path\":{path},\"mode\":{}}}",
                json_string(mode_name(mode))
            ),
            Err(error) => format!(
                "{{\"path\":{path},\"mode\":null,\"error\":{}}}",
                json_string(&error.message)
            ),
        });
    }
    println!("[{}]", rows.join(","));
    Ok(())
}
