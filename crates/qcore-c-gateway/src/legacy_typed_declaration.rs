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

//! Card **MEMBER-ACCESS-PROJECTION** (gateway rule 38h): the parser hook of
//! [`crate::typed_declaration`]. Thin: it finds the statement's own Clang
//! node and hands its facts to the decision, recording the consultation on
//! parity runs.

use super::{Parser, Token};

impl Parser<'_> {
    /// The lexical gate's refusal `old` of the statement `tokens`, with rule
    /// 38h: a refused statement the declaration prefix test calls
    /// `declaration_like` is admitted only when the function's Clang AST shows
    /// it is one declaration of automatic, uninitialized, constant-typed
    /// objects. Asked only for a refusal.
    pub(super) fn typed_declaration_gate(
        &self,
        tokens: &[Token],
        old: bool,
        declaration_like: bool,
    ) -> bool {
        if !old || !declaration_like {
            return old;
        }
        let Some(line) = tokens.first().map(|token| token.line) else {
            return old;
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let facts = self.typed_positions_index().and_then(|index| {
            index
                .statement_node(&texts, line, &|| {
                    crate::typed_place::occurrence(
                        self.tokens,
                        self.typed_place_origin.get(),
                        tokens,
                    )
                })
                .map(crate::typed_declaration::facts)
        });
        let refused = crate::typed_declaration::refuses(old, declaration_like, facts);
        if crate::typed_access_path::parity_enabled() {
            crate::typed_access_path::record_row(serde_json::json!({
                "function": self.typed_path_function,
                "line": line,
                "texts": texts,
                "position": "typed-declaration",
                "rule": "38h",
                "facts": facts.map(|facts| format!("{facts:?}")),
                "admitted": !refused,
            }));
        }
        refused
    }
}
