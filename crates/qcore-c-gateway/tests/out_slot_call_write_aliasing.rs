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

//! Card OUT-SLOT-CALL-WRITE-ALIASING (rule 11r(8)).
//!
//! A call whose actual may designate caller memory poisons every bound
//! sanctioned out slot: `*buf = data; memcpy(size, &n, sizeof n);` no longer
//! validates its out slot, because `size` may designate the caller's pointer
//! object (or a byte of it) and a `Borrowed` mode states no write frame. The
//! slot then reads `Ambiguous` at the exit, the W4 `Out` summary, the rule
//! 11r(6)/(7) class and the per-class row do not validate, and the function's
//! row is its uncontracted one. Excluded: scalars, null, string literals,
//! pointers to `const`, the address of a pointer-free local, and the slot a
//! callee's `Out` position itself writes. A slot not yet bound stays
//! unwritten.
//!
//! Every test asserts each function's status, complete diagnostic vector and
//! carried premises. The fixtures use a generic `x_*` surface whose
//! `x_memcpy`, `x_memmove` and `x_memset` have the C library's prototypes:
//! `Safe` here is `Safe` over synthetic trusted test assumptions and promotes
//! nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PREMISE: &str = "effective-type: a non-character integer or enumeration store through \
                       another out formal does not modify a pointer out slot (C11 6.5p7)";

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_memcpy(d: ptr<mut, u8>, s: ptr<const, u8>, n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(d);\n\
           ensures normal resource_borrowed(s); ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_alias(d, nonnull); };\n\
extern fn x_memmove(d: ptr<mut, u8>, s: ptr<const, u8>, n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(d);\n\
           ensures normal resource_borrowed(s); ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_alias(d, nonnull); };\n\
extern fn x_memset(d: ptr<mut, u8>, c: i32, n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(d);\n\
           ensures normal resource_borrowed(c); ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_alias(d, nonnull); };\n\
extern fn x_down(ch: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(ch);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_make(out: ptr<mut, ptr<mut, u8>>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_out(out, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_make_sized(out: ptr<mut, ptr<mut, u8>>, n: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_out(out, nonnull);\n\
           ensures normal resource_borrowed(n); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
char *x_alloc(__SIZE_TYPE__ n); void x_free(void *p); \
void *x_memcpy(void *d, const void *s, __SIZE_TYPE__ n); \
void *x_memmove(void *d, const void *s, __SIZE_TYPE__ n); \
void *x_memset(void *d, int c, __SIZE_TYPE__ n); \
typedef unsigned long x_size_t; \
struct x_chunk { char *data; x_size_t len; }; \
int x_down(struct x_chunk *ch); \
void x_make(char **out); void x_make_sized(char **out, x_size_t *n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "out_slot_call_write_aliasing.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent out-slot call-write test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}"),
    }
}

type Outcome = (String, Vec<String>, Vec<String>);

/// Every function's status, sorted diagnostics and carried premise names.
fn outcomes(body: &str) -> BTreeMap<String, Outcome> {
    let source = format!("{DECLARATIONS}\n{body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate)
            .expect("a Safe result carries a certificate that rechecks");
    }
    program(&result)
        .functions
        .iter()
        .map(|(name, function)| {
            let mut keys = function
                .diagnostics
                .iter()
                .map(|diagnostic| key(&diagnostic.kind))
                .collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            let premises = function
                .rule_obligations
                .premises
                .iter()
                .map(|carriage| format!("{}:{}", carriage.rule, carriage.name))
                .collect::<Vec<_>>();
            (
                name.clone(),
                (format!("{:?}", function.status), keys, premises),
            )
        })
        .collect()
}

fn expect(label: &str, body: &str, expected: &[(&str, Outcome)]) {
    if !clang_available() {
        return;
    }
    let measured = outcomes(body);
    let expected = expected
        .iter()
        .map(|(name, outcome)| ((*name).to_owned(), outcome.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(measured, expected, "{label}\n{body}");
}

fn safe() -> Outcome {
    ("Safe".to_owned(), Vec::new(), Vec::new())
}

fn premised() -> Outcome {
    (
        "Safe".to_owned(),
        Vec::new(),
        vec![format!("11r:{PREMISE}")],
    )
}

/// The callee's row when its out slot no longer validates: the uncontracted
/// reading of the store.
fn refused(slot: &str) -> Outcome {
    (
        "Unknown".to_owned(),
        vec![format!(
            "unsupported:store through `*{slot}` without an out-parameter contract"
        )],
        Vec::new(),
    )
}

/// A caller testing an uncontracted callee's code.
fn caller_refused(variable: &str) -> Outcome {
    (
        "Unknown".to_owned(),
        vec![
            format!("unknown_alias:{variable}"),
            "unsupported:unmodeled call, aggregate or indirect access in branch condition"
                .to_owned(),
        ],
        Vec::new(),
    )
}

const CALLER: &str = "int w(void) {\n char *b;\n x_size_t m;\n if (r(&b, &m) != 0) return -1;\n \
                      x_free(b);\n return 0;\n }";

fn slot_then(call: &str) -> String {
    format!(
        "static int r(char **buf, x_size_t *size) {{\n char *data = x_alloc(8);\n x_size_t n = 8;\n \
         if (!data) {{\n return -1;\n }} *buf = data;\n {call}\n return 0;\n }} {CALLER}"
    )
}

// -- A copy, a move or a fill through a second formal now refuses -------------

#[test]
fn a_memcpy_into_the_second_formal_poisons_the_slot() {
    expect(
        "`x_memcpy(size, &n, sizeof n)` after the slot: `size` may designate `*buf`",
        &slot_then("x_memcpy(size, &n, sizeof n);"),
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_memmove_into_the_second_formal_poisons_the_slot() {
    expect(
        "`x_memmove(size, &n, sizeof n)` after the slot",
        &slot_then("x_memmove(size, &n, sizeof n);"),
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_memset_of_the_second_formal_poisons_the_slot() {
    expect(
        "`x_memset(size, 0, sizeof *size)` after the slot",
        &slot_then("x_memset(size, 0, sizeof *size);"),
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_bodied_callee_writing_through_its_parameter_poisons_the_slot() {
    // `put`'s validated summary is all `Borrowed`; its body writes `*p`.
    expect(
        "`put(size)` after the slot, `put` storing through its formal",
        &format!(
            "static void put(x_size_t *p) {{\n *p = 8;\n }} {}",
            slot_then("put(size);")
        ),
        &[
            ("put", safe()),
            ("r", refused("buf")),
            ("w", caller_refused("b")),
        ],
    );
}

#[test]
fn the_chunkio_file_content_copy_shape_refuses() {
    // cio_file_content_copy: `cio_chunk_down(ch)` after both stores.
    expect(
        "`*out_buf = buf; *out_size = size; if (set_down) x_down(ch);`",
        "static int r(struct x_chunk *ch, void **out_buf, x_size_t *out_size, int set_down) {\n \
         char *buf;\n x_size_t size;\n size = ch->len;\n buf = x_alloc(size + 1);\n \
         if (!buf) {\n return -1;\n } x_memcpy(buf, ch->data, size);\n buf[size] = 0;\n \
         *out_buf = buf;\n *out_size = size;\n if (set_down) {\n x_down(ch);\n }\n return 0;\n } \
         int w(struct x_chunk *ch) {\n void *b;\n x_size_t n;\n \
         if (r(ch, &b, &n, 1) != 0) return -1;\n x_free(b);\n return 0;\n }",
        &[("r", refused("out_buf")), ("w", caller_refused("b"))],
    );
}

#[test]
fn a_member_or_a_pointer_local_reaches_the_slot() {
    expect(
        "`x_memset(ch->data, 0, 1)` after the slot: a pointer read from caller memory",
        "static int r(struct x_chunk *ch, char **buf) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } *buf = data;\n x_memset(ch->data, 0, 1);\n return 0;\n } \
         int w(struct x_chunk *ch) {\n char *b;\n if (r(ch, &b) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
    expect(
        "`x_memset(p, 0, 1)`, `p` a local copy of a formal",
        "static int r(struct x_chunk *ch, char **buf) {\n char *data = x_alloc(8);\n \
         char *p = ch->data;\n if (!data) {\n return -1;\n } *buf = data;\n x_memset(p, 0, 1);\n \
         return 0;\n } \
         int w(struct x_chunk *ch) {\n char *b;\n if (r(ch, &b) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
}

// -- What is unchanged ----------------------------------------------------------

#[test]
fn a_const_pointer_actual_is_unchanged() {
    // `size` reaches `x_memcpy` only at its `const void *` source position;
    // the destination is a pointer-free local array.
    expect(
        "`x_memcpy(tmp, size, sizeof *size)` after the slot",
        &slot_then("char tmp[16];\n x_memcpy(tmp, size, sizeof *size);"),
        &[("r", safe()), ("w", safe())],
    );
}

#[test]
fn distinct_local_actuals_are_unchanged() {
    expect(
        "`x_memset(&n, …)` and `x_memcpy(&k, &n, …)` after the slot",
        &slot_then("x_size_t k;\n x_memset(&n, 0, sizeof n);\n x_memcpy(&k, &n, sizeof n);"),
        &[("r", safe()), ("w", safe())],
    );
}

#[test]
fn a_call_before_the_slot_store_is_unchanged() {
    expect(
        "`x_memset(size, …)` before `*buf = data`: the store binds the slot after it",
        "static int r(char **buf, x_size_t *size) {\n char *data = x_alloc(8);\n \
         if (!data) {\n return -1;\n } x_memset(size, 0, sizeof *size);\n *buf = data;\n \
         return 0;\n } int w(void) {\n char *b;\n x_size_t m;\n if (r(&b, &m) != 0) return -1;\n \
         x_free(b);\n return 0;\n }",
        &[("r", safe()), ("w", safe())],
    );
}

#[test]
fn an_unwritten_class_keeps_its_slot_across_a_call() {
    // On `-1` the slot was never bound: a `Borrowed` write passes no custody
    // into it, and the caller reads `b` as untracked on that class anyway.
    expect(
        "`x_down(ch)` on the failure path, before any slot store",
        "static int r(struct x_chunk *ch, char **buf) {\n char *data = x_alloc(8);\n \
         if (!data) {\n x_down(ch);\n return -1;\n } *buf = data;\n return 0;\n } \
         int w(struct x_chunk *ch) {\n char *b;\n if (r(ch, &b) != 0) return -1;\n x_free(b);\n \
         return 0;\n }",
        &[("r", safe()), ("w", safe())],
    );
}

const TWO_SLOT_CALLER: &str = "int w(void) {\n char *p;\n char *q;\n x_size_t k;\n \
                               if (r(&p, &q, &k) != 0) return -1;\n x_free(p);\n x_free(q);\n \
                               return 0;\n }";

#[test]
fn a_callee_contract_stating_the_written_object_is_unchanged() {
    // `x_make`'s `Out` position states that it writes `t` (`&t` is a pointer
    // local's address, which would otherwise reach): `*a` keeps its binding.
    expect(
        "`*a = d; x_make(&t); *b = t;`",
        &format!(
            "static int r(char **a, char **b, x_size_t *n) {{\n char *d = x_alloc(8);\n char *t;\n \
             if (!d) return -1;\n *a = d;\n x_make(&t);\n *b = t;\n return 0;\n }} {TWO_SLOT_CALLER}"
        ),
        &[("r", safe()), ("w", safe())],
    );
}

#[test]
fn a_stated_out_position_beside_a_reaching_one_poisons_the_slot() {
    // `x_make_sized` states it writes `t`; its `Borrowed` `n`, a formal, may
    // still designate `*a`.
    expect(
        "`*a = d; x_make_sized(&t, n); *b = t;`",
        &format!(
            "static int r(char **a, char **b, x_size_t *n) {{\n char *d = x_alloc(8);\n char *t;\n \
             if (!d) return -1;\n *a = d;\n x_make_sized(&t, n);\n *b = t;\n return 0;\n }} \
             {TWO_SLOT_CALLER}"
        ),
        &[
            (
                "r",
                (
                    "Unknown".to_owned(),
                    vec![
                        "unsupported:store through `*a` without an out-parameter contract"
                            .to_owned(),
                        "unsupported:store through `*b` without an out-parameter contract"
                            .to_owned(),
                    ],
                    Vec::new(),
                ),
            ),
            (
                "w",
                (
                    "Unknown".to_owned(),
                    vec![
                        "unknown_alias:p".to_owned(),
                        "unknown_alias:q".to_owned(),
                        "unsupported:unmodeled call, aggregate or indirect access in branch condition"
                            .to_owned(),
                    ],
                    Vec::new(),
                ),
            ),
        ],
    );
}

#[test]
fn the_effective_type_premise_still_spares_a_typed_store_only() {
    // The read-file shape keeps its premise; a copy beside it refuses.
    expect(
        "`*buf = data; *size = 8;` certifies under the premise",
        &slot_then("*size = 8;"),
        &[("r", premised()), ("w", premised())],
    );
    expect(
        "`*size = 8; x_memset(size, 0, 1);`: the call is never spared",
        &slot_then("*size = 8;\n x_memset(size, 0, 1);"),
        &[("r", refused("buf")), ("w", caller_refused("b"))],
    );
}
