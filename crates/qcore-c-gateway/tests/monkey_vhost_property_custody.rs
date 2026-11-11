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

//! MONKEY-VHOST-PROPERTY-CUSTODY — `mk_string_dup`'s contract and gateway
//! rule 12d, the carry of an owner's field-linked children by its publication
//! (`docs/reports/monkey-vhost-property-custody/README.md`).
//!
//! The production modules `monkey/string.qv` (`mk_string_dup`) and
//! `monkey/intrusive_list.qv` (`mk_list_add`/`mk_list_del`) are used byte for
//! byte. The allocator and the libc string helpers are restated here with the
//! clauses their production declarations elaborate to (`monkey/memory.qv`
//! inherits `libc::malloc`/`libc::free`, whose module includes are not
//! resolved by this loader).
//!
//! Accepting controls: the `Name` branch of Monkey's `mk_vhost_set_property`
//! (an owned copy linked into a body-acquired alias that is published into
//! the caller's list); a duplicate released; an unlink then a full teardown.
//! Refusing controls: the `DocumentRoot` branch (an owned copy stored over a
//! member of the caller's object, never `Safe`), the whole function, a
//! partial link, a child released before the publication. Violation and leak
//! controls: a duplicate leaked; a carried child released while published;
//! an unlink followed by a release of the owner alone.
//!
//! `Safe` here is `Safe` for a declared projection under the premises the
//! report names, never a statement about Monkey.

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
const LIST: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv");

/// `monkey/memory.qv`'s two declarations as they elaborate (`libc::malloc`,
/// `libc::free`), and the two libc string reads the bodies make.
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

extern fn strlen(s: ptr<const, u8>) -> usize
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_return_none();
};

extern fn strcasecmp(s1: ptr<const, u8>, s2: ptr<const, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s1);
    ensures normal resource_borrowed(s2);
    ensures normal resource_return_none();
};
"#;

const PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    r#"

struct mk_list { struct mk_list *prev, *next; };
typedef struct { char *data; unsigned long len; } mk_ptr_t;
struct mk_vhost_alias { char *name; unsigned int len; struct mk_list _head; };
struct mk_vhost { mk_ptr_t documentroot; struct mk_list server_names; };
void *mk_mem_alloc(size_t size);
void mk_mem_free(void *ptr);
char *mk_string_dup(const char *s);
void mk_list_add(struct mk_list *_new, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
size_t strlen(const char *s);
int strcasecmp(const char *s1, const char *s2);
char *mk_ptr_to_buf(mk_ptr_t p);
"#
);

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("monkey/string.qv".to_owned(), STRING.to_owned()),
        ("monkey/intrusive_list.qv".to_owned(), LIST.to_owned()),
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

fn kinds(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect()
}

fn verdict(body: &str, name: &str) -> (AnalysisStatus, Vec<String>) {
    let summary = analyse(body, name);
    let row = function(&summary, name);
    (row.status, kinds(row))
}

/// Monkey `mk_server/mk_lib.c:488-506` (master and 21a2c190), with its
/// `config_eq` macro (`mk_lib.c:37`, `strcasecmp(a, b)`) expanded as the
/// pinned route's preprocessed unit has it.
const SET_PROPERTY: &str = r#"
static int mk_vhost_set_property(struct mk_vhost *vh, char *k, char *v)
{
    struct mk_vhost_alias *ha;

    if (strcasecmp(k, "Name") == 0) {
        ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
        if (!ha) {
            return -1;
        }
        ha->name = mk_string_dup(v);
        ha->len  = strlen(v);
        mk_list_add(&ha->_head, &vh->server_names);
    }
    else if (strcasecmp(k, "DocumentRoot") == 0) {
        vh->documentroot.data = mk_string_dup(v);
        vh->documentroot.len  = strlen(v);
    }

    return 0;
}
"#;

// ---- mk_string_dup ----

#[test]
fn a_leaked_duplicate_is_a_leak() {
    let (status, kinds) = verdict(
        "int f(const char *v)\n{\n    char *s;\n    s = mk_string_dup(v);\n    return 0;\n}\n",
        "f",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{kinds:?}");
    assert!(kinds.contains(&"leak".to_owned()), "{kinds:?}");
}

#[test]
fn a_released_duplicate_is_safe() {
    let (status, kinds) = verdict(
        "int f(const char *v)\n{\n    char *s;\n    s = mk_string_dup(v);\n    mk_mem_free(s);\n    return 0;\n}\n",
        "f",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{kinds:?}");
}

// ---- the Name branch: rule 12d ----

const NAME_BRANCH: &str = r#"
int set_name(struct mk_vhost *vh, char *v)
{
    struct mk_vhost_alias *ha;

    ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
    if (!ha) {
        return -1;
    }
    ha->name = mk_string_dup(v);
    ha->len  = strlen(v);
    mk_list_add(&ha->_head, &vh->server_names);
    return 0;
}
"#;

#[test]
fn the_name_branch_publishes_the_alias_and_its_copy() {
    let (status, kinds) = verdict(NAME_BRANCH, "set_name");
    assert_eq!(status, AnalysisStatus::Safe, "{kinds:?}");
    assert!(kinds.is_empty(), "{kinds:?}");
}

#[test]
fn a_partial_link_keeps_the_withdrawal() {
    let (status, kinds) = verdict(
        r#"
int set_name(struct mk_vhost *vh, char *v, int named)
{
    struct mk_vhost_alias *ha;

    ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
    if (!ha) {
        return -1;
    }
    ha->name = 0;
    if (named) {
        ha->name = mk_string_dup(v);
    }
    mk_list_add(&ha->_head, &vh->server_names);
    return 0;
}
"#,
        "set_name",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{kinds:?}");
    assert!(
        kinds
            .iter()
            .any(|kind| kind.contains("which owns linked objects")),
        "{kinds:?}"
    );
}

#[test]
fn a_child_released_before_the_publication_keeps_the_withdrawal() {
    let (status, kinds) = verdict(
        r#"
int set_name(struct mk_vhost *vh, char *v)
{
    struct mk_vhost_alias *ha;
    char *s;

    ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
    if (!ha) {
        return -1;
    }
    ha->name = mk_string_dup(v);
    s = ha->name;
    mk_mem_free(s);
    mk_list_add(&ha->_head, &vh->server_names);
    return 0;
}
"#,
        "set_name",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{kinds:?}");
}

/// A carried child read back through its member is a borrowed alias (rule
/// 10t), so no release through it is admitted: the shape is refused, never
/// `Safe`. (Releasing an escaped identity is `release_after_escape`, the
/// existing violation; `QCore.CarriedPublication` proves the carried case.)
#[test]
fn releasing_a_carried_child_while_published_is_never_safe() {
    let (status, kinds) = verdict(
        r#"
int set_name(struct mk_vhost *vh, char *v)
{
    struct mk_vhost_alias *ha;
    char *s;

    ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
    if (!ha) {
        return -1;
    }
    ha->name = mk_string_dup(v);
    s = ha->name;
    mk_list_add(&ha->_head, &vh->server_names);
    mk_mem_free(s);
    return 0;
}
"#,
        "set_name",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{kinds:?}");
}

#[test]
fn an_unlink_hands_the_child_back_and_the_owner_release_does_not_discharge_it() {
    let (status, kinds) = verdict(
        r#"
int cycle(struct mk_vhost *vh, char *v)
{
    struct mk_vhost_alias *ha;
    char *s;

    ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
    if (!ha) {
        return -1;
    }
    ha->name = mk_string_dup(v);
    s = ha->name;
    mk_list_add(&ha->_head, &vh->server_names);
    mk_list_del(&ha->_head);
    mk_mem_free(ha);
    return 0;
}
"#,
        "cycle",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{kinds:?}");
    assert!(kinds.contains(&"leak".to_owned()), "{kinds:?}");
}

// ---- DocumentRoot: never Safe ----

#[test]
fn the_document_root_overwrite_is_never_safe() {
    let (status, kinds) = verdict(
        r#"
int set_root(struct mk_vhost *vh, char *v)
{
    vh->documentroot.data = mk_string_dup(v);
    vh->documentroot.len  = strlen(v);
    return 0;
}
"#,
        "set_root",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{kinds:?}");
    assert!(!kinds.is_empty(), "{kinds:?}");
}

#[test]
fn the_whole_function_is_decided_except_for_the_document_root_branch() {
    let summary = analyse(SET_PROPERTY, "mk_vhost_set_property");
    let row = function(&summary, "mk_vhost_set_property");
    let kinds = kinds(row);
    assert_eq!(row.status, AnalysisStatus::Unknown, "{kinds:?}");
    // Every remaining leaf is the DocumentRoot branch's; the Name branch
    // (allocation, copy, length, publication) carries none.
    let lines: Vec<usize> = row.diagnostics.iter().map(|d| d.line).collect();
    let source = format!("{PRELUDE}\n{SET_PROPERTY}");
    let document_root = source
        .lines()
        .position(|line| line.contains("vh->documentroot.data = mk_string_dup(v);"))
        .expect("the DocumentRoot store")
        + 1;
    assert!(
        lines
            .iter()
            .all(|line| *line == document_root || *line == document_root + 1),
        "{lines:?} {kinds:?} (DocumentRoot at {document_root})"
    );
    assert!(!lines.is_empty());
}

// ---- rule 12e: a refused null test establishes no leak ----

/// Monkey `mk_server/mk_http.c:388-412` (`mk_http_error_page`), its first
/// half: `temp` joins a refused statement's value (`mk_ptr_to_buf`, which
/// has no contract here, as the indirect `*message` read is refused in the
/// corpus) and the nullable copy, and its null test is refused. Before rule
/// 12e the `return -1` accused the copy of a leak; it is NULL or was never
/// made there. Never `Unsafe`, never `Safe`.
#[test]
fn the_error_page_shape_is_unknown_not_a_leak() {
    let (status, kinds) = verdict(
        r#"
int error_page(mk_ptr_t *message)
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
"#,
        "error_page",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{kinds:?}");
    assert!(!kinds.contains(&"leak".to_owned()), "{kinds:?}");
}

/// The rule withholds only what the tested binding may denote: a copy the
/// refused test says nothing about keeps its established leak.
#[test]
fn a_refused_test_of_another_binding_keeps_the_leak() {
    let (status, kinds) = verdict(
        r#"
int other(mk_ptr_t *message, const char *v)
{
    char *s;
    char *temp;

    s = mk_string_dup(v);
    temp = mk_ptr_to_buf(*message);
    if (!temp) {
        return -1;
    }
    mk_mem_free(temp);
    mk_mem_free(s);
    return 0;
}
"#,
        "other",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{kinds:?}");
    assert!(kinds.contains(&"leak".to_owned()), "{kinds:?}");
}

/// A decided null test is unchanged: the copy is refuted on the null edge
/// and released on the other, `Safe`.
#[test]
fn a_decided_null_test_is_unchanged() {
    let (status, kinds) = verdict(
        r#"
int decided(const char *v)
{
    char *temp;

    temp = mk_string_dup(v);
    if (!temp) {
        return -1;
    }
    mk_mem_free(temp);
    return 0;
}
"#,
        "decided",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{kinds:?}");
}
