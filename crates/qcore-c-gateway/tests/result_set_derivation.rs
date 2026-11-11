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

//! Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
//! gateway rule 38e): the result-set derivation over a real Clang AST. The
//! fixture is Monkey's epoll channel creator chain (`mk_event_epoll.c` at
//! Fluent Bit 046accf6, reduced to the statements that decide its result):
//! with the C library's declared result sets of `pipe` and `epoll_ctl` the
//! chain returns {-1, 0}; from the bodies alone it returns every `int`; a set
//! out of force (held, or unbound because the unit defines the name) is never
//! read.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::derive_result_set_c11;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};

const LIBC: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/result_sets.qv");

const EPOLL: &str = "\
struct mk_event { int fd; int mask; };
struct mk_event_ctx { int efd; };
extern int pipe(int fds[2]);
extern int epoll_ctl(int epfd, int op, int fd, void *event);
extern int close(int fd);
static inline int _mk_event_add(struct mk_event_ctx *ctx, int fd, void *data)
{
    int ret;
    struct mk_event *event = data;
    ret = epoll_ctl(ctx->efd, 1, fd, 0);
    if (ret < 0) {
        return -1;
    }
    event->mask = 1;
    return ret;
}
static inline int _mk_event_channel_create(struct mk_event_ctx *ctx,
                                           int *r_fd, int *w_fd, void *data)
{
    int ret;
    int fd[2];
    struct mk_event *event;
    ret = pipe(fd);
    if (ret < 0) {
        return ret;
    }
    event = data;
    event->fd = fd[0];
    ret = _mk_event_add(ctx, fd[0], event);
    if (ret != 0) {
        close(fd[0]);
        close(fd[1]);
        return ret;
    }
    *r_fd = fd[0];
    *w_fd = fd[1];
    return 0;
}
int mk_event_channel_create(struct mk_event_ctx *ctx, int *r_fd, int *w_fd, void *data)
{
    return _mk_event_channel_create(ctx, r_fd, w_fd, data);
}
";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

fn bounds(text: &str) -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([(
        "quirin-provider-libc:libc/result_sets.qv".to_owned(),
        text.to_owned(),
    )]))
    .expect("the libc result sets load")
}

fn derive(
    source: &str,
    function: &str,
    contracts: &BoundsContracts,
) -> (String, Vec<String>, Vec<String>) {
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let derived = derive_result_set_c11(&unit, function, contracts).expect("derives");
    (
        derived.set.canonical(),
        derived.helpers.into_iter().collect(),
        derived.rests_on.into_iter().collect(),
    )
}

#[test]
fn the_epoll_channel_creator_returns_minus_one_or_zero_under_the_c_library() {
    if !clang_available() {
        return;
    }
    let (set, helpers, rests_on) = derive(EPOLL, "mk_event_channel_create", &bounds(LIBC));
    assert_eq!(set, "[-1, 0]");
    assert_eq!(helpers, ["_mk_event_add", "_mk_event_channel_create"]);
    assert_eq!(rests_on, ["epoll_ctl", "pipe"]);
}

#[test]
fn from_the_bodies_alone_the_chain_returns_every_int() {
    if !clang_available() {
        return;
    }
    let (set, _, rests_on) = derive(
        EPOLL,
        "mk_event_channel_create",
        &BoundsContracts::default(),
    );
    assert_eq!(set, "[MIN, MAX]");
    assert!(rests_on.is_empty());
}

#[test]
fn a_held_or_unbound_set_is_never_read() {
    if !clang_available() {
        return;
    }
    // A review marker would be refused on a flexible declaration; a set the
    // unit binds to its own `pipe` is unbound and reads nothing.
    let own_pipe = EPOLL.replace(
        "extern int pipe(int fds[2]);",
        "int pipe(int fds[2]) { return 7; }",
    );
    let (set, helpers, rests_on) = derive(&own_pipe, "mk_event_channel_create", &bounds(LIBC));
    // The unit's own `pipe` is a helper (7), and epoll_ctl's set still holds.
    assert_eq!(set, "[-1, 0] | 7");
    assert!(helpers.contains(&"pipe".to_owned()));
    assert_eq!(rests_on, ["epoll_ctl"]);
    // A call whose C type is not the declared result type binds nothing.
    let long_pipe = "extern long pipe(int fds[2]);\nlong q(int *fd) { return pipe(fd); }\n";
    let (set, _, rests_on) = derive(long_pipe, "q", &bounds(LIBC));
    assert_eq!(set, "[MIN, MAX]");
    assert!(rests_on.is_empty());
}

#[test]
fn a_local_with_any_other_write_reads_its_type() {
    if !clang_available() {
        return;
    }
    let source = "\
extern int pipe(int fds[2]);
int f(int *fd) { int ret = pipe(fd); ret += 0; return ret; }
int g(int *fd) { int ret; int *p = &ret; ret = pipe(fd); return ret; }
int h(int *fd) { int ret = pipe(fd); if (ret) ret = 0; return ret; }
";
    let libc = bounds(LIBC);
    assert_eq!(derive(source, "f", &libc).0, "[MIN, MAX]");
    assert_eq!(derive(source, "g", &libc).0, "[MIN, MAX]");
    assert_eq!(derive(source, "h", &libc).0, "[-1, 0]");
}
