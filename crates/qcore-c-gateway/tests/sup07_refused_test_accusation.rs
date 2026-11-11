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

//! SUP07-REFUSED-TEST-ACCUSATION — a leak claim must not rest on a refused
//! test (gateway rule 12e, `docs/reports/sup07-refused-test-accusation/`).
//!
//! The shape is Monkey's `mk_http_error_page` (`mk_server/mk_http.c:392-399`):
//! `temp` is joined from a refused `mk_ptr_to_buf` statement and a nullable
//! `mk_string_dup` copy, and `if (!temp) return -1;` is a null test rule 10b
//! refuses. Before the card the copy was accused at the `return -1`, where it
//! is NULL or was never made. The control is never `Unsafe` and never `Safe`.
//!
//! Beside it, every accusation whose decisive facts are established stays: a
//! second copy leaked on the same exit, a copy lost by a refused overwrite
//! before the test, and a copy leaked on the test's non-null edge that joins
//! the null edge at one exit.
//!
//! The second shape is `mk_sched_add_connection` (`mk_server/mk_scheduler.c:
//! 197-210`): `conn` joins two nullable allocations, one through a cast
//! copy, and its only `&` uses are member addresses (`&conn->event`), which
//! rule 12e reads as C does.
//!
//! `mk_string_dup` is the production `monkey/string.qv`; the allocator is
//! restated with the clauses `monkey/memory.qv` elaborates to, as in
//! `monkey_vhost_property_custody.rs`. `mk_ptr_to_buf`,
//! `mk_string_html_escape` and `mk_string_build` are uncontracted, as on the
//! pinned corpus route.

#[path = "support/host_abi.rs"]
mod host_abi;

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const STRING: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/string.qv");

const SUPPORT: &str = r#"module contract;

extern fn mk_mem_alloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn mk_mem_free(ptr: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ptr, nullable);
    ensures normal resource_return_none();
};
"#;

const PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    r#"
typedef struct { char *data; unsigned long len; } mk_ptr_t;
void *mk_mem_alloc(size_t size);
void mk_mem_free(void *ptr);
char *mk_string_dup(const char *s);
char *mk_ptr_to_buf(mk_ptr_t p);
char *mk_string_html_escape(const char *buf);
int mk_string_build(char **buffer, unsigned long *len, const char *format, ...);
struct mk_list { struct mk_list *prev, *next; };
struct mk_sched_conn { int fd; struct mk_list streams; };
void mk_list_init(struct mk_list *list);
"#
);

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("monkey/string.qv".to_owned(), STRING.to_owned()),
        ("support.qcore".to_owned(), SUPPORT.to_owned()),
    ]))
    .expect("the contracts load")
}

fn analyse(body: &str, entry: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}\n{body}"), entry, &contracts())
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

/// Each diagnostic as `kind@line`, a leak with its acquisition line.
fn leaves(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Leak { acquisition_line } => {
                format!("leak@{}<-{acquisition_line}", diagnostic.line)
            }
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported@{}:{description}", diagnostic.line)
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => {
                format!("unknown_alias@{}:{variable}", diagnostic.line)
            }
            other => format!("{other:?}@{}", diagnostic.line),
        })
        .collect()
}

fn verdict(body: &str, name: &str) -> (AnalysisStatus, Vec<String>) {
    let summary = analyse(body, name);
    let row = function(&summary, name);
    (row.status, leaves(row))
}

/// The 1-based line of the fixture text containing `needle`, as the analyzer
/// numbers it (the prelude, a newline, then the body).
fn line_of(body: &str, needle: &str) -> usize {
    format!("{PRELUDE}\n{body}")
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` is not in the fixture"))
        + 1
}

fn leaks(leaves: &[String]) -> Vec<&String> {
    leaves
        .iter()
        .filter(|leaf| leaf.starts_with("leak@"))
        .collect()
}

/// `mk_http_error_page`'s :392/:395/:398/:399 with the rest of the body
/// reduced to a release of the buffer.
const SHAPE: &str = r#"
int shape(mk_ptr_t *message)
{
    char *temp;

    if (message) {
        temp = mk_ptr_to_buf(*message);
    }
    else {
        temp = mk_string_dup("");
    }

    if (!temp) {
        return -1;
    }

    mk_mem_free(temp);
    return 0;
}
"#;

/// Monkey `mk_server/mk_http.c:383-411` (master and the pinned corpus copy),
/// with `MK_REQUEST_DEFAULT_PAGE` spelled as a literal.
const ERROR_PAGE: &str = r#"
static int mk_http_error_page(char *title, mk_ptr_t *message, char *signature,
                              char **out_buf, unsigned long *out_size)
{
    char *temp;
    char *escaped;

    *out_buf = 0;

    if (message) {
        temp = mk_ptr_to_buf(*message);
    }
    else {
        temp = mk_string_dup("");
    }

    if (!temp) {
        return -1;
    }

    escaped = mk_string_html_escape(temp);
    mk_mem_free(temp);
    if (!escaped) {
        return -1;
    }

    mk_string_build(out_buf, out_size,
                    "<html>%s%s%s</html>", title, escaped, signature);
    mk_mem_free(escaped);
    return 0;
}
"#;

#[test]
fn the_refused_test_shape_is_neither_unsafe_nor_safe() {
    let (status, leaves) = verdict(SHAPE, "shape");
    assert_eq!(status, AnalysisStatus::Unknown, "{leaves:?}");
    assert!(leaks(&leaves).is_empty(), "{leaves:?}");
    // The named reason: the test's identity refusal, kept by rule 10b.
    let test = line_of(SHAPE, "if (!temp) {");
    assert!(
        leaves.contains(&format!("unknown_alias@{test}:temp")),
        "{leaves:?}"
    );
}

#[test]
fn monkey_mk_http_error_page_is_unknown_without_a_leak() {
    let (status, leaves) = verdict(ERROR_PAGE, "mk_http_error_page");
    assert_eq!(status, AnalysisStatus::Unknown, "{leaves:?}");
    assert!(leaks(&leaves).is_empty(), "{leaves:?}");
    let test = line_of(ERROR_PAGE, "if (!temp) {");
    assert!(
        leaves.contains(&format!("unknown_alias@{test}:temp")),
        "{leaves:?}"
    );
}

#[test]
fn a_decided_test_of_the_copy_alone_is_safe() {
    let (status, leaves) = verdict(
        r#"
int decided(void)
{
    char *temp;

    temp = mk_string_dup("");
    if (!temp) {
        return -1;
    }
    mk_mem_free(temp);
    return 0;
}
"#,
        "decided",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// A second copy no test relates to is leaked at the same `return -1`: its
/// decisive facts are established and the accusation stays.
#[test]
fn a_leak_beside_the_refused_test_stays_accused() {
    const BODY: &str = r#"
int beside(mk_ptr_t *message)
{
    char *temp;
    char *other;

    other = mk_string_dup("x");
    if (message) {
        temp = mk_ptr_to_buf(*message);
    }
    else {
        temp = mk_string_dup("");
    }

    if (!temp) {
        return -1;
    }

    mk_mem_free(temp);
    mk_mem_free(other);
    return 0;
}
"#;
    let (status, leaves) = verdict(BODY, "beside");
    assert_eq!(status, AnalysisStatus::Unsafe, "{leaves:?}");
    let exit = line_of(BODY, "return -1;");
    let other = line_of(BODY, "other = mk_string_dup(\"x\");");
    let copy = line_of(BODY, "temp = mk_string_dup(\"\");");
    assert!(
        leaves.contains(&format!("leak@{exit}<-{other}")),
        "{leaves:?}"
    );
    assert!(
        !leaves.contains(&format!("leak@{exit}<-{copy}")),
        "{leaves:?}"
    );
}

/// The copy is made first and a refused statement overwrites `temp` on one
/// path: there the copy is live under no name whatever the test says, so
/// provenance does not tie it to `temp` and the accusation stays.
#[test]
fn a_copy_lost_before_the_refused_test_stays_accused() {
    const BODY: &str = r#"
int lost(mk_ptr_t *message)
{
    char *temp;

    temp = mk_string_dup("");
    if (message) {
        temp = mk_ptr_to_buf(*message);
    }

    if (!temp) {
        return -1;
    }

    mk_mem_free(temp);
    return 0;
}
"#;
    let (status, leaves) = verdict(BODY, "lost");
    assert_eq!(status, AnalysisStatus::Unsafe, "{leaves:?}");
    let exit = line_of(BODY, "return -1;");
    let copy = line_of(BODY, "temp = mk_string_dup(\"\");");
    assert!(
        leaves.contains(&format!("leak@{exit}<-{copy}")),
        "{leaves:?}"
    );
}

/// The copy is never released on the test's non-null edge, which joins the
/// null edge before the one exit: that leak is established on the non-null
/// edge and stays accused. Only the null edge's claim is withdrawn.
#[test]
fn a_leak_on_the_non_null_edge_stays_accused_across_the_join() {
    const BODY: &str = r#"
int joined(mk_ptr_t *message)
{
    char *temp;
    int rc;

    if (message) {
        temp = mk_ptr_to_buf(*message);
    }
    else {
        temp = mk_string_dup("");
    }

    if (!temp) {
        rc = -1;
    }
    else {
        rc = 0;
    }
    return rc;
}
"#;
    let (status, leaves) = verdict(BODY, "joined");
    assert_eq!(status, AnalysisStatus::Unsafe, "{leaves:?}");
    let exit = line_of(BODY, "return rc;");
    let copy = line_of(BODY, "temp = mk_string_dup(\"\");");
    assert!(
        leaves.contains(&format!("leak@{exit}<-{copy}")),
        "{leaves:?}"
    );
}

/// `mk_sched_add_connection`'s :197-210: two nullable allocations joined in
/// `conn`, one through a cast copy of `data`; `if (!conn)` refuses (two
/// targets). `&conn->streams` takes the address of a member of `*conn`, not
/// of `conn`, so provenance ties both allocations to `conn` and neither is
/// accused at the `return -1`. Never `Unsafe`, never `Safe`.
#[test]
fn the_scheduler_shape_is_neither_unsafe_nor_safe() {
    const BODY: &str = r#"
int add_connection(int extra)
{
    struct mk_sched_conn *conn;

    if (extra > 0) {
        void *data;
        data = mk_mem_alloc(sizeof(struct mk_sched_conn) + extra);
        conn = (struct mk_sched_conn *) data;
    }
    else {
        conn = mk_mem_alloc(sizeof(struct mk_sched_conn));
    }

    if (!conn) {
        return -1;
    }

    mk_list_init(&conn->streams);
    return 0;
}
"#;
    let (status, leaves) = verdict(BODY, "add_connection");
    assert_eq!(status, AnalysisStatus::Unknown, "{leaves:?}");
    let exit = line_of(BODY, "return -1;");
    assert!(
        !leaves
            .iter()
            .any(|leaf| leaf.starts_with(&format!("leak@{exit}<-"))),
        "{leaves:?}"
    );
    let test = line_of(BODY, "if (!conn) {");
    assert!(
        leaves.contains(&format!("unknown_alias@{test}:conn")),
        "{leaves:?}"
    );
}
