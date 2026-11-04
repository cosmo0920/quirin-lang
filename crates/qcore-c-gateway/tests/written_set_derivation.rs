//! Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (contract specification C09.4,
//! grammar G05j as amended, gateway rule 38e as amended): the slot
//! derivation over a real Clang AST.
//!
//! USER DECISIONS 2026-11-04 (verbatim): "Basically, Q1(a) and Q2(i) But we
//! must not handle descriptor 0 as a valid handle. So, we need to exclude the
//! 0 descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in
//! [1, MAX]"; "Write [1, MAX]".
//!
//! The deriving shapes are the creators' own bodies (Fluent Bit 046accf6,
//! reduced to the statements that decide the slots): Monkey's epoll channel
//! chain (`mk_event.c:181`, `mk_event_epoll.c:355`, `:101`) and
//! `flb_pipe_create` (`flb_pipe.c:100`). Every set they derive rests on
//! `pipe`'s declared `[1, MAX]`, a DECLARED POLICY: under POSIX's `[0, MAX]`
//! the same chain derives `[0, MAX]` (the contrast below). The refusing
//! shapes are the libevent backend (`mk_event_libevent.c:402`, an in-unit
//! helper over `evutil_socketpair`, which states no written set), the
//! Windows `flb_pipe_create` (`flb_pipe.c:55`), a store before the failure
//! exit, a returned local that is never narrowed, a callee with no declared
//! or derived written set, and a loop.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::derive_written_sets_c11;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};

const LIBC: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/result_sets.qv");

/// `mk_event_epoll.c` and `mk_event.c` at 046accf6, reduced (the `mk_bug`
/// macro kept as written, `do { if (...) { ...; abort(); } } while (0)`).
const EPOLL: &str = "\
struct mk_event { int fd; int type; int mask; };
struct mk_event_ctx { int efd; };
struct mk_event_loop { void *data; };
extern int pipe(int fds[2]);
extern int epoll_ctl(int epfd, int op, int fd, void *event);
extern int close(int fd);
extern void abort(void);
extern void mk_utils_libc_error(const char *c, const char *file, int line);
#define mk_bug(condition) do { if (__builtin_expect(((condition)!=0), 0)) { abort(); } } while(0)
#define mk_libc_error(c) mk_utils_libc_error(c, \"mk_event_epoll.c\", 0)
static inline int _mk_event_add(struct mk_event_ctx *ctx, int fd, void *data)
{
    int ret;
    struct mk_event *event = data;
    mk_bug(ctx == 0);
    ret = epoll_ctl(ctx->efd, 1, fd, 0);
    if (ret < 0) {
        mk_libc_error(\"epoll_ctl\");
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
    mk_bug(data == 0);
    ret = pipe(fd);
    if (ret < 0) {
        mk_libc_error(\"pipe\");
        return ret;
    }
    event = data;
    event->fd = fd[0];
    event->type = 4;
    event->mask = 0;
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
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data)
{
    struct mk_event_ctx *ctx;
    mk_bug(!data);
    ctx = loop->data;
    return _mk_event_channel_create(ctx, r_fd, w_fd, data);
}
";

/// `mk_event_libevent.c:402` at 046accf6, reduced: the pair comes from an
/// in-unit helper over libevent's `evutil_socketpair`.
const LIBEVENT: &str = "\
struct mk_event { int fd; int type; int mask; };
struct mk_event_ctx { int base; };
extern int evutil_socketpair(int d, int type, int protocol, int sv[2]);
extern int evutil_closesocket(int fd);
static int mk_event_libevent_socketpair(int fd[2])
{
    return evutil_socketpair(1, 1, 0, fd);
}
static inline int _mk_event_add(struct mk_event_ctx *ctx, int fd, void *data)
{
    return 0;
}
static inline int _mk_event_channel_create(struct mk_event_ctx *ctx,
                                           int *r_fd, int *w_fd, void *data)
{
    struct mk_event *event;
    int fd[2];
    int ret;
    ret = mk_event_libevent_socketpair(fd);
    if (ret == -1) {
        return -1;
    }
    event = data;
    event->fd = fd[0];
    ret = _mk_event_add(ctx, fd[0], event);
    if (ret != 0) {
        evutil_closesocket(fd[0]);
        evutil_closesocket(fd[1]);
        return ret;
    }
    event->mask = 1;
    *r_fd = fd[0];
    *w_fd = fd[1];
    return 0;
}
";

/// `flb_pipe.c` at 046accf6: the POSIX body and the Windows one.
const FLB_PIPE: &str = "\
extern int pipe(int fds[2]);
int flb_pipe_create(int pipefd[2])
{
    return pipe(pipefd);
}
";
const FLB_PIPE_WINDOWS: &str = "\
extern int evutil_socketpair(int d, int type, int protocol, int sv[2]);
extern void flb_pipe_destroy(int pipefd[2]);
int flb_pipe_create(int pipefd[2])
{
    if (evutil_socketpair(1, 1, 0, pipefd) == -1) {
        return -1;
    }
    return 0;
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

/// A derived set, its helpers and the C library declarations it rested on.
type Derived = (String, Vec<String>, Vec<String>);

/// `Ok((set, helpers, rests_on))` or the refusal, per requested slot.
fn derive(
    source: &str,
    function: &str,
    slots: &[(usize, u32, i128)],
    contracts: &BoundsContracts,
) -> Vec<Result<Derived, String>> {
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    derive_written_sets_c11(&unit, function, slots, contracts)
        .expect("the unit reads")
        .into_iter()
        .map(|derived| {
            derived.map(|derived| {
                (
                    derived.set.canonical(),
                    derived.helpers.into_iter().collect(),
                    derived.rests_on.into_iter().collect(),
                )
            })
        })
        .collect()
}

fn one(source: &str, function: &str, slot: (usize, u32, i128)) -> Result<String, String> {
    derive(source, function, &[slot], &bounds(LIBC))
        .remove(0)
        .map(|(set, _, _)| set)
}

#[test]
fn the_epoll_channel_creator_writes_both_ends_in_the_declared_bound() {
    if !clang_available() {
        return;
    }
    let derived = derive(
        EPOLL,
        "mk_event_channel_create",
        &[(1, 0, 0), (2, 0, 0)],
        &bounds(LIBC),
    );
    for slot in derived {
        let (set, helpers, rests_on) = slot.expect("derives");
        assert_eq!(set, "[1, MAX]");
        assert_eq!(helpers, ["_mk_event_add", "_mk_event_channel_create"]);
        assert_eq!(rests_on, ["epoll_ctl", "pipe"]);
    }
}

#[test]
fn flb_pipe_create_forwards_pipe_s_written_sets() {
    if !clang_available() {
        return;
    }
    let derived = derive(
        FLB_PIPE,
        "flb_pipe_create",
        &[(0, 0, 0), (0, 1, 0)],
        &bounds(LIBC),
    );
    for slot in derived {
        let (set, helpers, rests_on) = slot.expect("derives");
        assert_eq!(set, "[1, MAX]");
        assert!(helpers.is_empty());
        assert_eq!(rests_on, ["pipe"]);
    }
}

/// The DECLARED bound is what decides: under POSIX's set the same bodies
/// derive [0, MAX], which `live_set([1, MAX])` does not contain.
#[test]
fn under_posix_s_set_the_chain_derives_descriptor_zero() {
    if !clang_available() {
        return;
    }
    let posix = bounds(&LIBC.replace(
        "written_set(pipedes[0], [1, MAX])",
        "written_set(pipedes[0], [0, MAX])",
    ));
    let epoll = derive(EPOLL, "mk_event_channel_create", &[(1, 0, 0)], &posix);
    assert_eq!(epoll[0].as_ref().map(|d| d.0.as_str()), Ok("[0, MAX]"));
    let pipe = derive(FLB_PIPE, "flb_pipe_create", &[(0, 0, 0), (0, 1, 0)], &posix);
    assert_eq!(pipe[0].as_ref().map(|d| d.0.as_str()), Ok("[0, MAX]"));
    assert_eq!(pipe[1].as_ref().map(|d| d.0.as_str()), Ok("[1, MAX]"));
}

/// Each backend's body decides its own set: libevent's pair comes from
/// `evutil_socketpair`, which states no written set, so its ends are every
/// `int` (nothing a declared bound contains is pinned), and the Windows
/// `flb_pipe_create` never stores the slot on the path returning 0.
#[test]
fn a_backend_whose_call_states_no_written_set_derives_nothing() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        one(LIBEVENT, "_mk_event_channel_create", (1, 0, 0)),
        Ok("[MIN, MAX]".to_owned())
    );
    let windows = one(FLB_PIPE_WINDOWS, "flb_pipe_create", (0, 0, 0)).expect_err("refuses");
    assert!(windows.contains("written-set-unwritten"), "{windows}");
    let forwarded = "\
extern int evutil_socketpair(int d, int type, int protocol, int sv[2]);
int f(int sv[2]) { return evutil_socketpair(1, 1, 0, sv); }
";
    let refused = one(forwarded, "f", (0, 0, 0)).expect_err("refuses");
    assert!(refused.contains("states no written set"), "{refused}");
}

/// A store before the C library call's failure exit stores an end the walk
/// does not know yet (the call's result is not narrowed there).
#[test]
fn a_store_before_the_failure_exit_reads_every_int() {
    if !clang_available() {
        return;
    }
    let source = "\
extern int pipe(int fds[2]);
int f(int *r_fd)
{
    int ret;
    int fd[2];
    ret = pipe(fd);
    *r_fd = fd[0];
    if (ret < 0) {
        return ret;
    }
    return 0;
}
";
    assert_eq!(one(source, "f", (0, 0, 0)), Ok("[MIN, MAX]".to_owned()));
}

/// A returned local the walk does not narrow (its address is taken, or the
/// test leaves 0 in its set) may return 0 with the slot unwritten.
#[test]
fn an_unnarrowed_local_refuses() {
    if !clang_available() {
        return;
    }
    let taken = "\
extern int pipe(int fds[2]);
extern void observe(int *p);
int f(int *r_fd)
{
    int ret;
    int fd[2];
    observe(&ret);
    ret = pipe(fd);
    if (ret < 0) {
        return ret;
    }
    *r_fd = fd[0];
    return 0;
}
";
    let refused = one(taken, "f", (0, 0, 0)).expect_err("refuses");
    assert!(refused.contains("written-set-unwritten"), "{refused}");
    let wide = "\
extern int pipe(int fds[2]);
int f(int *r_fd)
{
    int ret;
    int fd[2];
    ret = pipe(fd);
    if (ret == -2) {
        return ret;
    }
    *r_fd = fd[0];
    return ret;
}
";
    assert_eq!(one(wide, "f", (0, 0, 0)), Ok("[MIN, MAX]".to_owned()));
}

/// A call after the store, or an out array handed to a callee that states
/// nothing, leaves the end unknown; a loop refuses.
#[test]
fn what_the_walk_does_not_read_refuses() {
    if !clang_available() {
        return;
    }
    let call_after = "\
extern int pipe(int fds[2]);
extern void touch(void);
int f(int *r_fd)
{
    int fd[2];
    int ret = pipe(fd);
    if (ret < 0) {
        return -1;
    }
    *r_fd = fd[0];
    touch();
    return 0;
}
";
    let refused = one(call_after, "f", (0, 0, 0)).expect_err("refuses");
    assert!(refused.contains("written-set-unwritten"), "{refused}");
    let handed = "\
extern int pipe(int fds[2]);
extern void keep(int *p);
int f(int *r_fd)
{
    int fd[2];
    int ret = pipe(fd);
    if (ret < 0) {
        return -1;
    }
    keep(fd);
    *r_fd = fd[0];
    return 0;
}
";
    assert_eq!(one(handed, "f", (0, 0, 0)), Ok("[MIN, MAX]".to_owned()));
    let looped = "\
extern int pipe(int fds[2]);
int f(int *r_fd)
{
    int fd[2];
    while (pipe(fd) < 0) {
    }
    *r_fd = fd[0];
    return 0;
}
";
    let refused = one(looped, "f", (0, 0, 0)).expect_err("refuses");
    assert!(refused.contains("WhileStmt"), "{refused}");
    // The slot written through the test's own assignment, `(ret = pipe(fd))`.
    let inline = "\
extern int pipe(int fds[2]);
int f(int *r_fd)
{
    int ret;
    int fd[2];
    if ((ret = pipe(fd)) < 0) {
        return ret;
    }
    *r_fd = fd[1];
    return 0;
}
";
    assert_eq!(one(inline, "f", (0, 0, 0)), Ok("[1, MAX]".to_owned()));
}
