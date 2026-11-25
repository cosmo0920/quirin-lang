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

//! Card CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION: probe.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const SHAPES: &[(&str, &str)] = &[
    ("observe", "void subject(void **out) { q_observe(*out); }"),
    ("pair", "void subject(void **out, int x) { cg_pair(*out, x); }"),
    ("void-cast", "void subject(void **out) { (void)q_observe(*out); }"),
    ("assigned", "int subject(void **out) { int r; r = r_borrow_code(*out); return r; }"),
    ("double", "void subject(void ***out) { q_observe(**out); }"),
    ("paren", "void subject(void **out) { q_observe((*out)); }"),
    ("toppar", "void subject(void **list, void *item) { if (!*list) *list = q_alloc(); q_store(*list, item); }"),
    ("toppar-call", "void subject(void **list, void *item) { q_store(*list, item); }"),
    ("fp-formal", "void subject(void (*cb)(void *), void **out) { cb(*out); }"),
    ("fp-member", "struct CB { void (*cb)(void *); };\nvoid subject(struct CB *s, void **out) { s->cb(*out); }"),
    ("uncontracted", "void undeclared_sink(void *p);\nvoid subject(void **out) { undeclared_sink(*out); }"),
    ("decl-int", "void subject(void *out) { { int (*out); } }"),
    ("decl-fn", "void subject(void *out) { { void (*out)(void); } }"),
    ("decl-array", "typedef int T;\nvoid subject(void *out) { { T (*out)[4]; } }"),
    ("decl-typeof", "void subject(void *out) { int x; { __typeof__(x) (*out); } }"),
    ("decl-typedef", "typedef void *T;\nvoid subject(void *out) { { T (*out); } }"),
];

#[test]
fn probe() {
    if !clang_available() {
        return;
    }
    for (name, source) in SHAPES {
        let run = g(source);
        eprintln!("PROBE {name}: {:?} unsupported={:?}", run.status, run.unsupported);
    }
}
