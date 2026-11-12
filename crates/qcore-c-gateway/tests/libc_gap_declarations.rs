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

//! Card LIBC-GAP-DECLARATIONS: controls for the 23 `quirin-provider-libc`
//! declarations this card adds (`docs/reports/libc-gap-declarations/`).
//!
//! No route changes. The tests load the production modules byte for byte and
//! analyse C units that include the audited host's own glibc headers, so each
//! declaration is also compared with the prototype the units see (contract
//! specification C04.1, card LOADER-ABI). They check, per declaration:
//!
//! * **agreement and admission:** a call that passes its actuals through is
//!   `Safe` with no diagnostic, so the declaration is in force and was not
//!   withdrawn;
//! * **withdrawal:** without the card's module texts the same calls keep the
//!   `call to uncontracted function` refusal;
//! * **refusing:** a use after release of a tracked stream or descriptor is a
//!   candidate; a release of `pthread_getspecific`'s result or of `strcat`'s
//!   alias is never `Safe`; a heap or caller pointer handed to
//!   `pthread_setspecific` is `retention-undecided`; a `getpwnam_r` buffer
//!   released while its record lives is `retention-released-while-held`.
//!
//! A `Safe` here is `Safe` over the libc assumptions and promotes nothing.
//!
//! The units include glibc's own headers (`<pwd.h>`, `<sys/mman.h>`,
//! `<sys/random.h>`, ...) and the declarations are audited against glibc, so
//! the target is GNU/Linux; elsewhere the headers do not exist.
#![cfg(all(target_os = "linux", target_env = "gnu"))]

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const MODULES: &[(&str, &str)] = &[
    (
        "libc/math.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/math.qv"),
    ),
    (
        "libc/memory.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv"),
    ),
    (
        "libc/process.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/process.qv"),
    ),
    (
        "libc/socket.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/socket.qv"),
    ),
    (
        "libc/stdio.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdio.qv"),
    ),
    (
        "libc/stdlib.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdlib.qv"),
    ),
    (
        "libc/sys_stat.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/sys_stat.qv"),
    ),
    (
        "libc/threads.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/threads.qv"),
    ),
    (
        "libc/unistd.qv",
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/unistd.qv"),
    ),
];

/// The section marker each module's card section starts with.
const SECTION: &str = "// ## Card LIBC-GAP-DECLARATIONS";

/// The 23 names this card declares.
const DECLARED: [&str; 23] = [
    "abs",
    "chmod",
    "chown",
    "fread",
    "ftruncate",
    "getgrnam_r",
    "getpwnam_r",
    "getrandom",
    "lseek",
    "msync",
    "poll",
    "posix_fallocate",
    "pthread_cancel",
    "pthread_getspecific",
    "pthread_setspecific",
    "readlink",
    "sendmsg",
    "shutdown",
    "sigdelset",
    "sqrt",
    "strcat",
    "tolower",
    "toupper",
];

/// The production modules, or (`with_card == false`) the same modules with
/// this card's section cut off: the base the declarations were added to.
fn contracts(with_card: bool) -> ResourceContracts {
    let files: BTreeMap<String, String> = MODULES
        .iter()
        .map(|(name, text)| {
            let text = if with_card {
                (*text).to_owned()
            } else {
                strip_card_section(text)
            };
            ((*name).to_owned(), text)
        })
        .collect();
    load_resource_contract_sources(&files).expect("the libc modules load")
}

/// `text` without this card's section (each module's last section).
fn strip_card_section(text: &str) -> String {
    match text.find(SECTION) {
        Some(at) => text[..at].trim_end().to_owned() + "\n",
        None => text.to_owned(),
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent LIBC-GAP-DECLARATIONS test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    diagnostics: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe && self.diagnostics.is_empty()
    }
    fn has(&self, key: &str) -> bool {
        self.diagnostics.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The glibc headers the audited units include, under `_GNU_SOURCE` as
/// Monkey and librdkafka define it.
const HEADERS: &str = "\
#define _GNU_SOURCE
#include <ctype.h>
#include <fcntl.h>
#include <grp.h>
#include <math.h>
#include <poll.h>
#include <pthread.h>
#include <pwd.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>
pthread_key_t key;
";

/// Every row of the unit `HEADERS + source`, keyed by function.
fn rows(source: &str, environment: &ResourceContracts) -> BTreeMap<String, Row> {
    let text = format!("{HEADERS}\n{source}\n");
    let entry = source
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        environment,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    diagnostics: row
                        .diagnostics
                        .iter()
                        .map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                description.clone()
                            }
                            other => format!("{other:?}"),
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

/// One pass-through caller per declaration: each must be `Safe` with the
/// card's declarations. The `get*nam_r` buffer is the caller's (an unrebound
/// formal), which outlives the frame's record (rule 12t); a local array there
/// is `retention-undecided`, which the last test pins.
const PASS_THROUGH: &str = "\
int c_tolower(int c) { return tolower(c); }
int c_toupper(int c) { return toupper(c); }
int c_abs(int c) { return abs(c); }
double c_sqrt(double c) { return sqrt(c); }
char *c_strcat(char *d, const char *s) { return strcat(d, s); }
int c_sigdelset(sigset_t *s) { return sigdelset(s, SIGPIPE); }
int c_pthread_cancel(pthread_t t) { return pthread_cancel(t); }
void *c_pthread_getspecific(void) { return pthread_getspecific(key); }
int c_pthread_setspecific(void) { return pthread_setspecific(key, \"static\"); }
size_t c_fread(void *b, FILE *f) { return fread(b, 1, 4, f); }
off_t c_lseek(int fd) { return lseek(fd, 0, SEEK_SET); }
int c_ftruncate(int fd, off_t n) { return ftruncate(fd, n); }
int c_posix_fallocate(int fd, off_t n) { return posix_fallocate(fd, 0, n); }
ssize_t c_readlink(const char *p, char *b, size_t n) { return readlink(p, b, n); }
int c_chown(const char *p, uid_t u, gid_t g) { return chown(p, u, g); }
int c_chmod(const char *p, mode_t m) { return chmod(p, m); }
int c_msync(void *a, size_t n) { return msync(a, n, MS_SYNC); }
ssize_t c_getrandom(void *b, size_t n) { return getrandom(b, n, 0); }
int c_shutdown(int fd) { return shutdown(fd, SHUT_RDWR); }
ssize_t c_sendmsg(int fd, const struct msghdr *m) { return sendmsg(fd, m, 0); }
int c_poll(struct pollfd *f, nfds_t n) { return poll(f, n, 0); }
int c_getpwnam_r(const char *u, char *buf, size_t n) {
    struct passwd pw; struct passwd *res = NULL;
    return getpwnam_r(u, &pw, buf, n, &res);
}
int c_getgrnam_r(const char *g, char *buf, size_t n) {
    struct group gr; struct group *res = NULL;
    return getgrnam_r(g, &gr, buf, n, &res);
}
";

/// The pass-through function that exercises `name`.
fn caller_of(name: &str) -> String {
    format!("c_{name}")
}

#[test]
fn every_declaration_agrees_with_glibc_and_admits_its_pass_through_caller() {
    if !clang_available() {
        return;
    }
    let rows = rows(PASS_THROUGH, &contracts(true));
    for name in DECLARED {
        let row = &rows[&caller_of(name)];
        assert!(row.safe(), "{name}: {row:?}");
    }
}

#[test]
fn without_the_card_every_pass_through_call_stays_uncontracted() {
    if !clang_available() {
        return;
    }
    let rows = rows(PASS_THROUGH, &contracts(false));
    for name in DECLARED {
        let row = &rows[&caller_of(name)];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{name}: {row:?}");
        assert!(
            row.has(&format!("call to uncontracted function `{name}`")),
            "{name}: {row:?}"
        );
    }
}

#[test]
fn the_module_index_names_exactly_the_cards_declarations() {
    let mut names: Vec<String> = MODULES
        .iter()
        .filter_map(|(_, text)| text.find(SECTION).map(|at| &text[at..]))
        .flat_map(|section| {
            section
                .lines()
                .filter_map(|line| line.strip_prefix("extern fn "))
                .map(|rest| rest.split('(').next().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    names.sort();
    assert_eq!(names, DECLARED);
}

#[test]
fn a_use_after_release_of_a_stream_or_descriptor_is_a_candidate() {
    if !clang_available() {
        return;
    }
    let source = "\
void stream_after_close(void *b) {
    FILE *f = fopen(\"x\", \"r\");
    if (!f) return;
    fclose(f);
    fread(b, 1, 4, f);
}
void descriptor_after_close(void) {
    int fd = open(\"x\", O_RDONLY);
    if (fd < 0) return;
    close(fd);
    lseek(fd, 0, SEEK_SET);
}
void descriptor_use_then_close(void) {
    int fd = open(\"x\", O_RDWR);
    if (fd < 0) return;
    ftruncate(fd, 0);
    posix_fallocate(fd, 0, 16);
    shutdown(fd, SHUT_RDWR);
    close(fd);
}
";
    let rows = rows(source, &contracts(true));
    let stream = &rows["stream_after_close"];
    assert_eq!(stream.status, AnalysisStatus::Unsafe, "{stream:?}");
    assert!(stream.has("UseAfterClose"), "{stream:?}");
    let descriptor = &rows["descriptor_after_close"];
    assert_eq!(descriptor.status, AnalysisStatus::Unsafe, "{descriptor:?}");
    assert!(descriptor.has("UseAfterClose"), "{descriptor:?}");
    let used = &rows["descriptor_use_then_close"];
    assert!(used.safe(), "{used:?}");
}

#[test]
fn a_thread_specific_value_is_retained_and_its_read_back_owns_nothing() {
    if !clang_available() {
        return;
    }
    let source = "\
void set_heap(void) { void *p = malloc(4); if (!p) return; pthread_setspecific(key, p); }
void set_formal(void *p) { pthread_setspecific(key, p); }
void set_then_free(void) { void *p = malloc(4); if (!p) return; pthread_setspecific(key, p); free(p); }
void get_free(void) { void *p = pthread_getspecific(key); free(p); }
void get_free_twice(void) { void *p = pthread_getspecific(key); free(p); free(p); }
";
    let rows = rows(source, &contracts(true));
    for name in ["set_heap", "set_formal", "set_then_free"] {
        let row = &rows[name];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{name}: {row:?}");
        assert!(row.has("retention-undecided"), "{name}: {row:?}");
    }
    // The result carries no identity: a release of it is refused, never
    // credited and never accused.
    for name in ["get_free", "get_free_twice"] {
        let row = &rows[name];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{name}: {row:?}");
        assert!(!row.has("call to uncontracted"), "{name}: {row:?}");
    }
}

#[test]
fn a_release_of_a_record_buffer_or_an_alias_is_never_safe() {
    if !clang_available() {
        return;
    }
    let source = "\
int buffer_freed_while_held(const char *u) {
    struct passwd pw; struct passwd *res = NULL; char *buf = malloc(64); int ok;
    if (!buf) return -1;
    ok = getpwnam_r(u, &pw, buf, 64, &res);
    free(buf);
    return ok;
}
int local_buffer(const char *u) {
    struct passwd pw; struct passwd *res = NULL; char buf[256];
    return getpwnam_r(u, &pw, buf, sizeof buf, &res);
}
int group_buffer_freed_while_held(const char *g) {
    struct group gr; struct group *res = NULL; char *buf = malloc(64); int ok;
    if (!buf) return -1;
    ok = getgrnam_r(g, &gr, buf, 64, &res);
    free(buf);
    return ok;
}
void alias_released(void) {
    char *d = malloc(8);
    if (!d) return;
    d[0] = 0;
    char *r = strcat(d, \"a\");
    free(r);
}
void base_released(void) {
    char *d = malloc(8);
    if (!d) return;
    d[0] = 0;
    strcat(d, \"a\");
    free(d);
}
";
    let rows = rows(source, &contracts(true));
    for name in ["buffer_freed_while_held", "group_buffer_freed_while_held"] {
        let row = &rows[name];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{name}: {row:?}");
        assert!(row.has("retention-released-while-held"), "{name}: {row:?}");
    }
    let local = &rows["local_buffer"];
    assert_eq!(local.status, AnalysisStatus::Unknown, "{local:?}");
    assert!(local.has("retention-undecided"), "{local:?}");
    let alias = &rows["alias_released"];
    assert_ne!(alias.status, AnalysisStatus::Safe, "{alias:?}");
    let base = &rows["base_released"];
    assert!(base.safe(), "{base:?}");
}
