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

// CG01-T-OPERATOR-CLOSURE: the shared C source vectors for route T's operator
// and literal closure. Each row is (entry, position, C source of `f`). The
// BEFORE and AFTER tests read the same rows, so the executed refusal at the
// base and the admission after it are measured on identical inputs.

pub(super) const RECORD: &str = "struct item { int n; long l; short h; };";

/// Binary operators E01 names (absent from T's allowlist at the base).
pub(super) const E01_BINARY: [&str; 6] = ["-", "<=", ">=", "^", "<<", ">>"];
/// Unary operators E02 names.
pub(super) const E02_UNARY: [&str; 2] = ["+", "~"];
/// Compound assignments E03 names for T, plus `|=` (int-only at the base).
pub(super) const E03_COMPOUND: [&str; 10] =
    ["+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "^=", "|="];

/// (position, source) for a binary operator spelled `@`.
pub(super) fn binary_positions(op: &str) -> Vec<(&'static str, String)> {
    [
        ("return", "int f(int a,int b){return a @ b;}"),
        ("assign", "int f(int a,int b){int x; x = a @ b; return x;}"),
        ("branch", "int f(int a,int b){if(a @ b)return 1;return 0;}"),
        (
            "guard",
            "int f(int a,int b){if(a && (a @ b))return 1;return 0;}",
        ),
        ("index", "int f(int *p,int a,int b){return p[a @ b];}"),
        ("argument", "void g(int);void f(int a,int b){g(a @ b);}"),
    ]
    .into_iter()
    .map(|(position, text)| (position, text.replace('@', op)))
    .collect()
}

/// (position, source) for a unary operator spelled `@`.
pub(super) fn unary_positions(op: &str) -> Vec<(&'static str, String)> {
    [
        ("return", "int f(int a){return @a;}"),
        ("assign", "int f(int a){int x; x = @a; return x;}"),
        ("branch", "int f(int a){if(@a)return 1;return 0;}"),
        ("guard", "int f(int a,int b){if(b && @a)return 1;return 0;}"),
        ("index", "int f(int *p,int a){return p[@a];}"),
        ("argument", "void g(int);void f(int a){g(@a);}"),
    ]
    .into_iter()
    .map(|(position, text)| (position, text.replace('@', op)))
    .collect()
}

/// (position, source) for a compound assignment spelled `@` (statements only).
pub(super) fn compound_positions(op: &str) -> Vec<(&'static str, String)> {
    [
        ("local", "int f(int a){int x = a; x @ 3; return x;}"),
        ("deref", "void f(int *p,int a){*p @ a;}"),
        ("field", "void f(struct item *p,int a){p->n @ a;}"),
        ("index", "void f(int *p,int i,int a){p[i] @ a;}"),
        ("widened-short", "void f(struct item *p,int a){p->h @ a;}"),
        ("long-field", "void f(struct item *p,long a){p->l @ a;}"),
        (
            "branch-body",
            "int f(int a,int b){int x = a; if(b) x @ 1; return x;}",
        ),
    ]
    .into_iter()
    .map(|(position, text)| (position, format!("{RECORD}{}", text.replace('@', op))))
    .collect()
}

/// (entry, position, source) for the E14 literals.
pub(super) fn literal_positions() -> Vec<(&'static str, &'static str, String)> {
    [
        ("float", "argument", "void g(double);void f(void){g(1.5);}"),
        (
            "float",
            "argument-f",
            "void g(float);void f(void){g(2.5f);}",
        ),
        ("char", "return", "int f(int c){return c == 'x';}"),
        (
            "char",
            "branch",
            "int f(int c){if(c == 'x')return 1;return 0;}",
        ),
        ("char", "index", "int f(int *p){return p['a'];}"),
        ("char", "argument", "void g(int);void f(void){g('x');}"),
    ]
    .into_iter()
    .map(|(entry, position, text)| (entry, position, text.to_owned()))
    .collect()
}
