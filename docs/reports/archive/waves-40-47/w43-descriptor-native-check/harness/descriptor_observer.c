/*
 * Native descriptor observation harness (W43-DESCRIPTOR-NATIVE-CHECK).
 *
 * Linked with `-Wl,--wrap=NAME` for every function below, so each reference
 * to NAME from an object or static archive in the link reaches __wrap_NAME.
 * References inside shared libraries (libc itself, libssl, ...) are not
 * interposed; the runner audits which descriptor APIs the link references.
 *
 * Each event is one JSON line (schema_version 1) written with the real
 * write(2) to QCORE_DESC_OUT. Records are trusted observer output, never a
 * static certificate. The observer stamps generations: a descriptor number
 * or pointer that is acquired again after its release is a new generation.
 * The Rust checker recomputes every generation independently.
 *
 * Ordering: releases (and dup2/dup3, which may release implicitly) execute
 * their real call while holding the observer lock, so a number freed by
 * one thread cannot be re-acquired and recorded before its release record.
 * Acquisitions are recorded after the real call. Uses are recorded before
 * the real call, so a use recorded after a release really started after it.
 * Signals are blocked while the lock is held.
 *
 * Deterministic failure injection: QCORE_DESC_INJECT=call:n:err[,call:n:err]
 * makes the n-th call (1-based, per base call name) fail without calling the
 * real function: -1 and errno=err for descriptor calls, NULL and errno=err
 * for stream/directory/dl constructors, MAP_FAILED for mmap, and the EAI
 * code err (negative) for getaddrinfo.
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <execinfo.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <pthread.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/timerfd.h>
#include <sys/types.h>
#include <sys/uio.h>
#include <ifaddrs.h>
#include <unistd.h>

#define SCHEMA_VERSION 1
#define MAX_FD 65536
#define PTR_SLOTS (1u << 17)
#define MAX_INJECT 16
#define MAX_OPERANDS 8
#define LINE_BYTES (1 << 20)
#define STACK_FRAMES 12

enum kind { K_FD, K_FILE, K_DIR, K_DL, K_ADDRINFO, K_MAPPING, K_PROCESS_STREAM, K_IFADDRS };
static const char *const kind_names[] = {"fd", "file", "dir", "dl_handle", "addrinfo", "mapping", "process_stream", "ifaddrs"};

struct fd_slot {
    uint32_t generation;
    uint8_t live;
    uint8_t owner_kind; /* K_FD when the caller owns it, else the stream kind */
    uintptr_t owner;
};

struct ptr_slot {
    uintptr_t id;
    uint32_t generation;
    uint8_t used;
    uint8_t live;
    uint8_t kind;
    uint32_t refs;
    int child_fd; /* underlying descriptor owned by a FILE or DIR, or -1 */
    size_t extent; /* mapping length */
};

struct operand {
    const char *role;
    int kind;
    long long id;
    uint32_t generation;
    long long extent;
};

struct injection {
    char call[32];
    unsigned long index;
    int err;
    unsigned long seen;
    /* Optional site range [lo, hi) of module offsets; 0/0 matches any site. */
    unsigned long long lo;
    unsigned long long hi;
};

static struct fd_slot fds[MAX_FD];
static struct ptr_slot ptrs[PTR_SLOTS];
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static int out_fd = -1;
static pid_t owner_pid;
static unsigned long long sequence;
static struct injection injections[MAX_INJECT];
static int injection_count;
static int finished;

/* Real entry points (resolved to libc by the linker). */
#define REAL(ret, name, params) ret __real_##name params;
REAL(int, open, (const char *, int, ...))
REAL(int, open64, (const char *, int, ...))
REAL(int, openat, (int, const char *, int, ...))
REAL(int, openat64, (int, const char *, int, ...))
REAL(int, creat, (const char *, mode_t))
REAL(int, socket, (int, int, int))
REAL(int, accept, (int, struct sockaddr *, socklen_t *))
REAL(int, accept4, (int, struct sockaddr *, socklen_t *, int))
REAL(int, dup, (int))
REAL(int, dup2, (int, int))
REAL(int, dup3, (int, int, int))
REAL(int, fcntl, (int, int, ...))
REAL(int, fcntl64, (int, int, ...))
REAL(int, pipe, (int[2]))
REAL(int, pipe2, (int[2], int))
REAL(int, socketpair, (int, int, int, int[2]))
REAL(int, epoll_create, (int))
REAL(int, epoll_create1, (int))
REAL(int, eventfd, (unsigned int, int))
REAL(int, timerfd_create, (int, int))
REAL(int, inotify_init1, (int))
REAL(int, memfd_create, (const char *, unsigned int))
REAL(int, mkstemp, (char *))
REAL(int, mkstemp64, (char *))
REAL(int, close, (int))
REAL(FILE *, fopen, (const char *, const char *))
REAL(FILE *, fopen64, (const char *, const char *))
REAL(FILE *, fdopen, (int, const char *))
REAL(FILE *, tmpfile, (void))
REAL(FILE *, tmpfile64, (void))
REAL(FILE *, popen, (const char *, const char *))
REAL(int, pclose, (FILE *))
REAL(int, fclose, (FILE *))
REAL(int, fileno, (FILE *))
REAL(DIR *, opendir, (const char *))
REAL(DIR *, fdopendir, (int))
REAL(int, closedir, (DIR *))
REAL(struct dirent *, readdir, (DIR *))
REAL(struct dirent64 *, readdir64, (DIR *))
REAL(int, dirfd, (DIR *))
REAL(void, rewinddir, (DIR *))
REAL(void *, dlopen, (const char *, int))
REAL(int, dlclose, (void *))
REAL(void *, dlsym, (void *, const char *))
REAL(int, getaddrinfo, (const char *, const char *, const struct addrinfo *, struct addrinfo **))
REAL(void, freeaddrinfo, (struct addrinfo *))
REAL(void *, mmap, (void *, size_t, int, int, int, off_t))
REAL(void *, mmap64, (void *, size_t, int, int, int, off_t))
REAL(int, munmap, (void *, size_t))
REAL(void *, mremap, (void *, size_t, size_t, int, ...))
REAL(int, msync, (void *, size_t, int))
REAL(int, madvise, (void *, size_t, int))
REAL(ssize_t, write, (int, const void *, size_t))

static uint32_t fd_generation(int fd)
{
    return (fd >= 0 && fd < MAX_FD) ? fds[fd].generation : 0;
}

static struct ptr_slot *ptr_find(uintptr_t id, int create)
{
    uint32_t h = (uint32_t) ((id >> 4) * 2654435761u) & (PTR_SLOTS - 1);
    for (uint32_t probe = 0; probe < PTR_SLOTS; probe++) {
        struct ptr_slot *slot = &ptrs[(h + probe) & (PTR_SLOTS - 1)];
        if (slot->used && slot->id == id) {
            return slot;
        }
        if (!slot->used) {
            if (!create) {
                return NULL;
            }
            slot->used = 1;
            slot->id = id;
            slot->child_fd = -1;
            return slot;
        }
    }
    return NULL;
}

static uint32_t ptr_generation(uintptr_t id)
{
    struct ptr_slot *slot = ptr_find(id, 0);
    return slot ? slot->generation : 0;
}

/* ---- serialization ---------------------------------------------------- */

struct line {
    char text[LINE_BYTES];
    size_t used;
    int overflow;
};
static struct line line_buffer; /* guarded by lock */

static void put(struct line *l, const char *fmt, ...)
{
    va_list ap;
    int n;
    if (l->overflow) {
        return;
    }
    va_start(ap, fmt);
    n = vsnprintf(l->text + l->used, sizeof(l->text) - l->used, fmt, ap);
    va_end(ap);
    if (n < 0 || (size_t) n >= sizeof(l->text) - l->used) {
        l->overflow = 1;
        return;
    }
    l->used += (size_t) n;
}

static void put_string(struct line *l, const char *s)
{
    put(l, "\"");
    for (size_t i = 0; s && s[i] && i < 256; i++) {
        unsigned char c = (unsigned char) s[i];
        if (c == '"' || c == '\\') {
            put(l, "\\%c", c);
        }
        else if (c < 0x20 || c >= 0x7f) {
            put(l, "\\u%04x", c);
        }
        else {
            put(l, "%c", c);
        }
    }
    put(l, "\"");
}

static void emit(struct line *l)
{
    size_t off = 0;
    if (l->overflow) {
        l->used = 0;
        l->overflow = 0;
        put(l, "{\"schema_version\":%d,\"sequence\":%llu,\"call\":\"overflow\"}", SCHEMA_VERSION, sequence);
    }
    put(l, "\n");
    while (off < l->used) {
        ssize_t n = __real_write(out_fd, l->text + off, l->used - off);
        if (n <= 0) {
            if (n < 0 && errno == EINTR) {
                continue;
            }
            return;
        }
        off += (size_t) n;
    }
}

static int active(void)
{
    return out_fd >= 0 && !finished && getpid() == owner_pid;
}

static void lock_observer(sigset_t *saved)
{
    sigset_t all;
    sigfillset(&all);
    pthread_sigmask(SIG_BLOCK, &all, saved);
    pthread_mutex_lock(&lock);
}

static void unlock_observer(const sigset_t *saved)
{
    pthread_mutex_unlock(&lock);
    pthread_sigmask(SIG_SETMASK, saved, NULL);
}

/* Record one call. `site` is the caller's return address. Must hold lock. */
static void record(const char *call, void *site, const long long *args, int nargs,
                   long long result, int err, int injected, const char *path,
                   const struct operand *ops, int nops)
{
    struct line *lp = &line_buffer;
    lp->used = 0;
    lp->overflow = 0;
    Dl_info info;
    unsigned long long offset = 0;
    const char *module = "?";

    if (site && dladdr(site, &info) && info.dli_fbase) {
        offset = (unsigned long long) ((uintptr_t) site - (uintptr_t) info.dli_fbase);
        module = info.dli_fname ? info.dli_fname : "?";
        const char *slash = strrchr(module, '/');
        if (slash) {
            module = slash + 1;
        }
    }
    sequence++;
    put(lp, "{\"schema_version\":%d,\"sequence\":%llu,\"pid\":%d,\"thread\":%ld,\"call\":\"%s\"",
        SCHEMA_VERSION, sequence, (int) owner_pid, (long) gettid(), call);
    put(lp, ",\"site\":{\"module\":");
    put_string(lp, module);
    put(lp, ",\"offset\":%llu}", offset);
    put(lp, ",\"args\":[");
    for (int i = 0; i < nargs; i++) {
        put(lp, "%s%lld", i ? "," : "", args[i]);
    }
    put(lp, "],\"result\":%lld,\"errno\":%d,\"injected\":%s", result, err, injected ? "true" : "false");
    if (path) {
        put(lp, ",\"path\":");
        put_string(lp, path);
    }
    /* Constructors, failures and releases carry a short call stack; uses
       (the high-volume records) carry only their immediate site. */
    int want_stack = nops == 0;
    for (int i = 0; i < nops; i++) {
        if (strcmp(ops[i].role, "used") != 0 && strcmp(ops[i].role, "related") != 0) {
            want_stack = 1;
        }
    }
    if (want_stack) {
        void *frames[STACK_FRAMES];
        int depth = backtrace(frames, STACK_FRAMES);
        put(lp, ",\"stack\":[");
        for (int i = 0, n = 0; i < depth; i++) {
            Dl_info frame;
            if (dladdr(frames[i], &frame) && frame.dli_fbase) {
                const char *name = frame.dli_fname ? frame.dli_fname : "?";
                const char *slash = strrchr(name, '/');
                put(lp, "%s{\"module\":", n++ ? "," : "");
                put_string(lp, slash ? slash + 1 : name);
                put(lp, ",\"offset\":%llu}", (unsigned long long) ((uintptr_t) frames[i] - (uintptr_t) frame.dli_fbase));
            }
        }
        put(lp, "]");
    }
    put(lp, ",\"operands\":[");
    for (int i = 0; i < nops; i++) {
        put(lp, "%s{\"role\":\"%s\",\"kind\":\"%s\",\"id\":%lld,\"generation\":%u", i ? "," : "",
            ops[i].role, kind_names[ops[i].kind], ops[i].id, ops[i].generation);
        if (ops[i].extent >= 0) {
            put(lp, ",\"extent\":%lld", ops[i].extent);
        }
        put(lp, "}");
    }
    put(lp, "]}");
    emit(lp);
}

static struct operand op(const char *role, int kind, long long id, uint32_t generation)
{
    struct operand o = {role, kind, id, generation, -1};
    return o;
}

/* ---- state transitions (observer bookkeeping, hold lock) --------------- */

static struct operand fd_acquire(const char *role, int fd, int owner_kind, uintptr_t owner)
{
    if (fd >= 0 && fd < MAX_FD) {
        fds[fd].generation++;
        fds[fd].live = 1;
        fds[fd].owner_kind = (uint8_t) owner_kind;
        fds[fd].owner = owner;
    }
    return op(role, K_FD, fd, fd_generation(fd));
}

static struct operand fd_release(const char *role, int fd)
{
    struct operand o = op(role, K_FD, fd, fd_generation(fd));
    if (fd >= 0 && fd < MAX_FD) {
        fds[fd].live = 0;
    }
    return o;
}

static struct operand ptr_acquire(int kind, uintptr_t id, int child_fd, size_t extent)
{
    struct ptr_slot *slot = ptr_find(id, 1);
    struct operand o;
    if (!slot) {
        return op("acquired", kind, (long long) id, 0);
    }
    if (kind == K_DL && slot->live && slot->kind == K_DL) {
        slot->refs++;
    }
    else {
        slot->generation++;
        slot->live = 1;
        slot->kind = (uint8_t) kind;
        slot->refs = 1;
    }
    slot->child_fd = child_fd;
    slot->extent = extent;
    o = op("acquired", kind, (long long) id, slot->generation);
    if (kind == K_MAPPING) {
        o.extent = (long long) extent;
    }
    return o;
}

static struct operand ptr_release(int kind, uintptr_t id, int *child_fd)
{
    struct ptr_slot *slot = ptr_find(id, 0);
    struct operand o = op("released", kind, (long long) id, slot ? slot->generation : 0);
    *child_fd = -1;
    if (slot && slot->live) {
        *child_fd = slot->child_fd;
        if (slot->refs > 1) {
            slot->refs--;
        }
        else {
            slot->refs = 0;
            slot->live = 0;
            slot->child_fd = -1;
        }
    }
    return o;
}

/* ---- injection --------------------------------------------------------- */

static unsigned long long site_offset(void *site)
{
    Dl_info info;
    if (site && dladdr(site, &info) && info.dli_fbase) {
        return (unsigned long long) ((uintptr_t) site - (uintptr_t) info.dli_fbase);
    }
    return 0;
}

/* Count calls per injection entry (only those inside its site range). */
static int inject(const char *call, void *site, int *err)
{
    unsigned long long offset = 0;
    int have_offset = 0;
    for (int i = 0; i < injection_count; i++) {
        struct injection *in = &injections[i];
        if (strcmp(in->call, call) != 0) {
            continue;
        }
        if (in->hi) {
            if (!have_offset) {
                offset = site_offset(site);
                have_offset = 1;
            }
            if (offset < in->lo || offset >= in->hi) {
                continue;
            }
        }
        in->seen++;
        if (in->seen == in->index) {
            *err = in->err;
            return 1;
        }
    }
    return 0;
}

static int should_inject_at(const char *call, void *site, int *err)
{
    sigset_t saved;
    int hit;
    if (!active() || injection_count == 0) {
        return 0;
    }
    lock_observer(&saved);
    hit = inject(call, site, err);
    unlock_observer(&saved);
    return hit;
}

/* In a wrapper body the caller's site is the wrapper's return address. */
#define should_inject(call, errp) should_inject_at(call, __builtin_return_address(0), errp)

/* Spec: call:index:error[:lo:hi], comma separated; lo/hi are decimal offsets. */
static void parse_injections(const char *spec)
{
    while (spec && *spec) {
        struct injection *in;
        const char *end = strchr(spec, ',');
        size_t len = end ? (size_t) (end - spec) : strlen(spec);
        char item[160];
        char *fields[5];
        int count = 0;
        char *cursor;
        if (injection_count >= MAX_INJECT || len >= sizeof(item)) {
            abort();
        }
        in = &injections[injection_count];
        memcpy(item, spec, len);
        item[len] = '\0';
        cursor = item;
        while (count < 5) {
            fields[count++] = cursor;
            cursor = strchr(cursor, ':');
            if (!cursor) {
                break;
            }
            *cursor++ = '\0';
        }
        if ((count != 3 && count != 5) || cursor || strlen(fields[0]) >= sizeof(in->call)) {
            abort();
        }
        strcpy(in->call, fields[0]);
        in->index = strtoul(fields[1], NULL, 10);
        in->err = (int) strtol(fields[2], NULL, 10);
        if (count == 5) {
            in->lo = strtoull(fields[3], NULL, 10);
            in->hi = strtoull(fields[4], NULL, 10);
            if (in->hi <= in->lo) {
                abort();
            }
        }
        if (in->index == 0) {
            abort();
        }
        injection_count++;
        spec = end ? end + 1 : NULL;
    }
}

/* ---- lifecycle -------------------------------------------------------- */

__attribute__((constructor)) static void observer_start(void)
{
    const char *path = getenv("QCORE_DESC_OUT");
    struct operand ops[MAX_OPERANDS];
    int nops = 0;
    sigset_t saved;
    DIR *dir;
    struct dirent *entry;
    int self;

    if (!path) {
        return;
    }
    {
        void *prime[2];
        (void) backtrace(prime, 2);
    }
    parse_injections(getenv("QCORE_DESC_INJECT"));
    out_fd = __real_open(path, O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC, 0600);
    if (out_fd < 0) {
        abort();
    }
    /* Keep the record descriptor out of the program's low numbers. */
    int high = __real_fcntl(out_fd, F_DUPFD_CLOEXEC, 1000);
    if (high >= 0) {
        __real_close(out_fd);
        out_fd = high;
    }
    owner_pid = getpid();
    lock_observer(&saved);
    dir = __real_opendir("/proc/self/fd");
    self = dir ? __real_dirfd(dir) : -1;
    struct line *lp = &line_buffer;
    lp->used = 0;
    lp->overflow = 0;
    sequence++;
    put(lp, "{\"schema_version\":%d,\"sequence\":%llu,\"pid\":%d,\"thread\":%ld,\"call\":\"start\",\"site\":{\"module\":\"observer\",\"offset\":0},\"args\":[],\"result\":0,\"errno\":0,\"injected\":false,\"injections\":[",
        SCHEMA_VERSION, sequence, (int) owner_pid, (long) gettid());
    for (int i = 0; i < injection_count; i++) {
        put(lp, "%s{\"call\":\"%s\",\"index\":%lu,\"error\":%d,\"site_lo\":%llu,\"site_hi\":%llu}", i ? "," : "",
            injections[i].call, injections[i].index, injections[i].err, injections[i].lo, injections[i].hi);
    }
    put(lp, "],\"operands\":[");
    while (dir && (entry = __real_readdir(dir)) != NULL) {
        char *end;
        long fd = strtol(entry->d_name, &end, 10);
        if (*end != '\0' || entry->d_name[0] == '.' || fd == self || fd == out_fd || fd < 0 || fd >= MAX_FD) {
            continue;
        }
        struct operand o = fd_acquire("inherited", (int) fd, K_FD, 0);
        put(lp, "%s{\"role\":\"inherited\",\"kind\":\"fd\",\"id\":%lld,\"generation\":%u}", nops ? "," : "", o.id, o.generation);
        nops++;
    }
    if (dir) {
        __real_closedir(dir);
    }
    /* Standard streams are inherited FILE objects over descriptors 0..2. */
    FILE *streams[3] = {stdin, stdout, stderr};
    for (int i = 0; i < 3; i++) {
        struct operand o = ptr_acquire(K_FILE, (uintptr_t) streams[i], i, 0);
        if (i < MAX_FD && fds[i].live) {
            fds[i].owner_kind = K_FILE;
            fds[i].owner = (uintptr_t) streams[i];
        }
        put(lp, "%s{\"role\":\"inherited\",\"kind\":\"file\",\"id\":%lld,\"generation\":%u,\"child\":%d}", nops ? "," : "",
            o.id, o.generation, i);
        nops++;
    }
    (void) ops;
    put(lp, "]}");
    emit(lp);
    unlock_observer(&saved);
}

__attribute__((destructor)) static void observer_finish(void)
{
    sigset_t saved;
    struct line *lp = &line_buffer;
    lp->used = 0;
    lp->overflow = 0;
    int n = 0;
    if (!active()) {
        return;
    }
    lock_observer(&saved);
    sequence++;
    put(lp, "{\"schema_version\":%d,\"sequence\":%llu,\"pid\":%d,\"thread\":%ld,\"call\":\"exit\",\"site\":{\"module\":\"observer\",\"offset\":0},\"args\":[],\"result\":0,\"errno\":0,\"injected\":false,\"operands\":[",
        SCHEMA_VERSION, sequence, (int) owner_pid, (long) gettid());
    for (int fd = 0; fd < MAX_FD; fd++) {
        if (fds[fd].live) {
            put(lp, "%s{\"role\":\"live\",\"kind\":\"fd\",\"id\":%d,\"generation\":%u}", n++ ? "," : "", fd, fds[fd].generation);
        }
    }
    for (uint32_t i = 0; i < PTR_SLOTS; i++) {
        if (ptrs[i].used && ptrs[i].live) {
            put(lp, "%s{\"role\":\"live\",\"kind\":\"%s\",\"id\":%lld,\"generation\":%u}", n++ ? "," : "",
                kind_names[ptrs[i].kind], (long long) ptrs[i].id, ptrs[i].generation);
        }
    }
    put(lp, "]}");
    emit(lp);
    finished = 1;
    unlock_observer(&saved);
}

/* ---- helpers for wrapper bodies ----------------------------------------- */

#define SITE __builtin_return_address(0)
#define ARGS(...) ((const long long[]){__VA_ARGS__}), (int) (sizeof((const long long[]){__VA_ARGS__}) / sizeof(long long))
#define NOARGS NULL, 0

/* A use: recorded before the real call. */
static void use_fd(const char *call, void *site, int fd, const long long *args, int nargs)
{
    sigset_t saved;
    if (!active()) {
        return;
    }
    lock_observer(&saved);
    struct operand o = op("used", K_FD, fd, fd_generation(fd));
    record(call, site, args, nargs, 0, 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
}

static void use_ptr(const char *call, void *site, int kind, const void *p)
{
    sigset_t saved;
    if (!active()) {
        return;
    }
    lock_observer(&saved);
    struct operand o = op("used", kind, (long long) (uintptr_t) p, ptr_generation((uintptr_t) p));
    record(call, site, NULL, 0, 0, 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
}

/* A descriptor constructor returning one fd (or -1). `source` >= 0 marks a used operand. */
static int fd_result(const char *call, void *site, int result, int err, int injected, int source,
                     const long long *args, int nargs, const char *path)
{
    sigset_t saved;
    struct operand ops[2];
    int n = 0;
    if (!active()) {
        errno = err;
        return result;
    }
    lock_observer(&saved);
    if (source != -2) {
        ops[n++] = op("used", K_FD, source, fd_generation(source));
    }
    if (result >= 0) {
        ops[n++] = fd_acquire("acquired", result, K_FD, 0);
    }
    record(call, site, args, nargs, result, result < 0 ? err : 0, injected, path, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

/* ---- descriptor constructors ------------------------------------------- */

static int open_common(const char *call, void *site, int dirfd, const char *path, int flags, mode_t mode,
                       int use64, int at)
{
    int err = 0;
    int result;
    if (should_inject_at(call, site, &err)) {
        return fd_result(call, site, -1, err, 1, at && dirfd != AT_FDCWD ? dirfd : -2,
                         ARGS(at ? dirfd : -100, flags, mode), path);
    }
    if (at) {
        result = use64 ? __real_openat64(dirfd, path, flags, mode) : __real_openat(dirfd, path, flags, mode);
    }
    else {
        result = use64 ? __real_open64(path, flags, mode) : __real_open(path, flags, mode);
    }
    err = errno;
    return fd_result(call, site, result, err, 0, at && dirfd != AT_FDCWD ? dirfd : -2,
                     ARGS(at ? dirfd : -100, flags, mode), path);
}

static mode_t open_mode(int flags, va_list ap)
{
    if (flags & (O_CREAT | O_TMPFILE)) {
        return (mode_t) va_arg(ap, int);
    }
    return 0;
}

int __wrap_open(const char *path, int flags, ...)
{
    va_list ap;
    va_start(ap, flags);
    mode_t mode = open_mode(flags, ap);
    va_end(ap);
    return open_common("open", SITE, AT_FDCWD, path, flags, mode, 0, 0);
}

int __wrap_open64(const char *path, int flags, ...)
{
    va_list ap;
    va_start(ap, flags);
    mode_t mode = open_mode(flags, ap);
    va_end(ap);
    return open_common("open", SITE, AT_FDCWD, path, flags, mode, 1, 0);
}

int __wrap_openat(int dirfd, const char *path, int flags, ...)
{
    va_list ap;
    va_start(ap, flags);
    mode_t mode = open_mode(flags, ap);
    va_end(ap);
    return open_common("openat", SITE, dirfd, path, flags, mode, 0, 1);
}

int __wrap_openat64(int dirfd, const char *path, int flags, ...)
{
    va_list ap;
    va_start(ap, flags);
    mode_t mode = open_mode(flags, ap);
    va_end(ap);
    return open_common("openat", SITE, dirfd, path, flags, mode, 1, 1);
}

int __wrap_creat(const char *path, mode_t mode)
{
    return open_common("open", SITE, AT_FDCWD, path, O_CREAT | O_WRONLY | O_TRUNC, mode, 0, 0);
}

#define SIMPLE_FD_CONSTRUCTOR(name, params, call_args, arg_values)                  \
    int __wrap_##name params                                                         \
    {                                                                                \
        int err = 0;                                                                 \
        int result;                                                                  \
        if (should_inject(#name, &err)) {                                            \
            return fd_result(#name, SITE, -1, err, 1, -2, ARGS arg_values, NULL);    \
        }                                                                            \
        result = __real_##name call_args;                                            \
        err = errno;                                                                 \
        return fd_result(#name, SITE, result, err, 0, -2, ARGS arg_values, NULL);    \
    }

SIMPLE_FD_CONSTRUCTOR(socket, (int domain, int type, int protocol), (domain, type, protocol), (domain, type, protocol))
SIMPLE_FD_CONSTRUCTOR(epoll_create, (int size), (size), (size))
SIMPLE_FD_CONSTRUCTOR(epoll_create1, (int flags), (flags), (flags))
SIMPLE_FD_CONSTRUCTOR(eventfd, (unsigned int initval, int flags), (initval, flags), (initval, flags))
SIMPLE_FD_CONSTRUCTOR(timerfd_create, (int clockid, int flags), (clockid, flags), (clockid, flags))
SIMPLE_FD_CONSTRUCTOR(inotify_init1, (int flags), (flags), (flags))

int __wrap_memfd_create(const char *name, unsigned int flags)
{
    int err = 0;
    int result;
    if (should_inject("memfd_create", &err)) {
        return fd_result("memfd_create", SITE, -1, err, 1, -2, ARGS(flags), name);
    }
    result = __real_memfd_create(name, flags);
    err = errno;
    return fd_result("memfd_create", SITE, result, err, 0, -2, ARGS(flags), name);
}

int __wrap_mkstemp(char *template)
{
    int err = 0;
    int result;
    if (should_inject("mkstemp", &err)) {
        return fd_result("mkstemp", SITE, -1, err, 1, -2, NOARGS, template);
    }
    result = __real_mkstemp(template);
    err = errno;
    return fd_result("mkstemp", SITE, result, err, 0, -2, NOARGS, template);
}

int __wrap_accept(int fd, struct sockaddr *addr, socklen_t *len)
{
    int err = 0;
    int result;
    use_fd("accept_enter", SITE, fd, ARGS(fd));
    if (should_inject("accept", &err)) {
        return fd_result("accept", SITE, -1, err, 1, fd, ARGS(fd, 0), NULL);
    }
    result = __real_accept(fd, addr, len);
    err = errno;
    return fd_result("accept", SITE, result, err, 0, fd, ARGS(fd, 0), NULL);
}

int __wrap_accept4(int fd, struct sockaddr *addr, socklen_t *len, int flags)
{
    int err = 0;
    int result;
    use_fd("accept_enter", SITE, fd, ARGS(fd));
    if (should_inject("accept4", &err)) {
        return fd_result("accept4", SITE, -1, err, 1, fd, ARGS(fd, flags), NULL);
    }
    result = __real_accept4(fd, addr, len, flags);
    err = errno;
    return fd_result("accept4", SITE, result, err, 0, fd, ARGS(fd, flags), NULL);
}

int __wrap_dup(int fd)
{
    int err = 0;
    int result;
    if (should_inject("dup", &err)) {
        return fd_result("dup", SITE, -1, err, 1, fd, ARGS(fd), NULL);
    }
    result = __real_dup(fd);
    err = errno;
    return fd_result("dup", SITE, result, err, 0, fd, ARGS(fd), NULL);
}

static int dup_to(const char *call, void *site, int oldfd, int newfd, int flags, int three)
{
    sigset_t saved;
    struct operand ops[3];
    int n = 0;
    int result;
    int err;
    if (!active()) {
        return three ? __real_dup3(oldfd, newfd, flags) : __real_dup2(oldfd, newfd);
    }
    lock_observer(&saved);
    ops[n++] = op("used", K_FD, oldfd, fd_generation(oldfd));
    int was_live = newfd >= 0 && newfd < MAX_FD && fds[newfd].live;
    result = three ? __real_dup3(oldfd, newfd, flags) : __real_dup2(oldfd, newfd);
    err = errno;
    if (result >= 0 && result != oldfd) {
        if (was_live) {
            ops[n++] = fd_release("replaced", newfd);
        }
        ops[n++] = fd_acquire("acquired", result, K_FD, 0);
    }
    record(call, site, ARGS(oldfd, newfd, flags), result, result < 0 ? err : 0, 0, NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

int __wrap_dup2(int oldfd, int newfd)
{
    return dup_to("dup2", SITE, oldfd, newfd, 0, 0);
}

int __wrap_dup3(int oldfd, int newfd, int flags)
{
    return dup_to("dup3", SITE, oldfd, newfd, flags, 1);
}

static int fcntl_common(void *site, int fd, int cmd, void *arg, int use64)
{
    int err = 0;
    int result;
    if (cmd == F_DUPFD || cmd == F_DUPFD_CLOEXEC) {
        if (should_inject_at("fcntl_dupfd", site, &err)) {
            return fd_result("fcntl_dupfd", site, -1, err, 1, fd, ARGS(fd, cmd, (long long) (intptr_t) arg), NULL);
        }
        result = use64 ? __real_fcntl64(fd, cmd, arg) : __real_fcntl(fd, cmd, arg);
        err = errno;
        return fd_result("fcntl_dupfd", site, result, err, 0, fd, ARGS(fd, cmd, (long long) (intptr_t) arg), NULL);
    }
    use_fd("fcntl", site, fd, ARGS(fd, cmd));
    result = use64 ? __real_fcntl64(fd, cmd, arg) : __real_fcntl(fd, cmd, arg);
    return result;
}

int __wrap_fcntl(int fd, int cmd, ...)
{
    va_list ap;
    va_start(ap, cmd);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    return fcntl_common(SITE, fd, cmd, arg, 0);
}

int __wrap_fcntl64(int fd, int cmd, ...)
{
    va_list ap;
    va_start(ap, cmd);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    return fcntl_common(SITE, fd, cmd, arg, 1);
}

static int pair_result(const char *call, void *site, int result, int err, int injected, int pair[2],
                       const long long *args, int nargs)
{
    sigset_t saved;
    struct operand ops[2];
    int n = 0;
    if (!active()) {
        errno = err;
        return result;
    }
    lock_observer(&saved);
    if (result == 0) {
        ops[n++] = fd_acquire("acquired", pair[0], K_FD, 0);
        ops[n++] = fd_acquire("acquired", pair[1], K_FD, 0);
    }
    record(call, site, args, nargs, result, result != 0 ? err : 0, injected, NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

int __wrap_pipe(int pair[2])
{
    int err = 0;
    if (should_inject("pipe", &err)) {
        return pair_result("pipe", SITE, -1, err, 1, pair, NOARGS);
    }
    int result = __real_pipe(pair);
    err = errno;
    return pair_result("pipe", SITE, result, err, 0, pair, NOARGS);
}

int __wrap_pipe2(int pair[2], int flags)
{
    int err = 0;
    if (should_inject("pipe2", &err)) {
        return pair_result("pipe2", SITE, -1, err, 1, pair, ARGS(flags));
    }
    int result = __real_pipe2(pair, flags);
    err = errno;
    return pair_result("pipe2", SITE, result, err, 0, pair, ARGS(flags));
}

int __wrap_socketpair(int domain, int type, int protocol, int pair[2])
{
    int err = 0;
    if (should_inject("socketpair", &err)) {
        return pair_result("socketpair", SITE, -1, err, 1, pair, ARGS(domain, type, protocol));
    }
    int result = __real_socketpair(domain, type, protocol, pair);
    err = errno;
    return pair_result("socketpair", SITE, result, err, 0, pair, ARGS(domain, type, protocol));
}

/* ---- descriptor release ------------------------------------------------ */

int __wrap_close(int fd)
{
    sigset_t saved;
    int result;
    int err;
    if (!active()) {
        return __real_close(fd);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_close(fd);
    err = errno;
    struct operand o = fd_release("released", fd);
    record("close", site, ARGS(fd), result, result < 0 ? err : 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
    errno = err;
    return result;
}

/* ---- FILE streams -------------------------------------------------------- */

static FILE *stream_result(const char *call, void *site, FILE *result, int err, int injected, int adopted,
                           const char *path)
{
    sigset_t saved;
    struct operand ops[3];
    int n = 0;
    if (!active()) {
        errno = err;
        return result;
    }
    lock_observer(&saved);
    if (adopted != -2) {
        ops[n++] = op("used", K_FD, adopted, fd_generation(adopted));
    }
    if (result) {
        int child = adopted != -2 ? adopted : __real_fileno(result);
        ops[n++] = ptr_acquire(K_FILE, (uintptr_t) result, child, 0);
        if (adopted != -2) {
            if (adopted >= 0 && adopted < MAX_FD) {
                fds[adopted].owner_kind = K_FILE;
                fds[adopted].owner = (uintptr_t) result;
            }
            ops[n++] = op("adopted", K_FD, adopted, fd_generation(adopted));
        }
        else if (child >= 0) {
            ops[n++] = fd_acquire("derived", child, K_FILE, (uintptr_t) result);
        }
    }
    record(call, site, NOARGS, (long long) (uintptr_t) result, result ? 0 : err, injected, path, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

FILE *__wrap_fopen(const char *path, const char *mode)
{
    int err = 0;
    if (should_inject("fopen", &err)) {
        return stream_result("fopen", SITE, NULL, err, 1, -2, path);
    }
    FILE *result = __real_fopen(path, mode);
    err = errno;
    return stream_result("fopen", SITE, result, err, 0, -2, path);
}

FILE *__wrap_fopen64(const char *path, const char *mode)
{
    int err = 0;
    if (should_inject("fopen", &err)) {
        return stream_result("fopen", SITE, NULL, err, 1, -2, path);
    }
    FILE *result = __real_fopen64(path, mode);
    err = errno;
    return stream_result("fopen", SITE, result, err, 0, -2, path);
}

FILE *__wrap_tmpfile(void)
{
    int err = 0;
    if (should_inject("tmpfile", &err)) {
        return stream_result("tmpfile", SITE, NULL, err, 1, -2, NULL);
    }
    FILE *result = __real_tmpfile();
    err = errno;
    return stream_result("tmpfile", SITE, result, err, 0, -2, NULL);
}

FILE *__wrap_fdopen(int fd, const char *mode)
{
    int err = 0;
    if (should_inject("fdopen", &err)) {
        return stream_result("fdopen", SITE, NULL, err, 1, fd, NULL);
    }
    FILE *result = __real_fdopen(fd, mode);
    err = errno;
    return stream_result("fdopen", SITE, result, err, 0, fd, NULL);
}

int __wrap_fclose(FILE *fp)
{
    sigset_t saved;
    int result;
    int err;
    int child;
    struct operand ops[2];
    int n = 0;
    if (!active()) {
        return __real_fclose(fp);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_fclose(fp);
    err = errno;
    ops[n++] = ptr_release(K_FILE, (uintptr_t) fp, &child);
    if (child >= 0) {
        ops[n++] = fd_release("released_child", child);
    }
    record("fclose", site, NOARGS, result, result != 0 ? err : 0, 0, NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

int __wrap_fileno(FILE *fp)
{
    sigset_t saved;
    int result = __real_fileno(fp);
    int err = errno;
    if (active()) {
        struct operand ops[2];
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        ops[n++] = op("used", K_FILE, (long long) (uintptr_t) fp, ptr_generation((uintptr_t) fp));
        if (result >= 0) {
            ops[n++] = op("related", K_FD, result, fd_generation(result));
        }
        record("fileno", site, NOARGS, result, result < 0 ? err : 0, 0, NULL, ops, n);
        unlock_observer(&saved);
    }
    errno = err;
    return result;
}

/* FILE uses: recorded before the real call. */
#define FILE_USE(ret, name, params, fp, call_args)             \
    ret __real_##name params;                                   \
    ret __wrap_##name params                                    \
    {                                                           \
        use_ptr(#name, SITE, K_FILE, fp);                       \
        return __real_##name call_args;                         \
    }

FILE_USE(size_t, fread, (void *p, size_t s, size_t n, FILE *f), f, (p, s, n, f))
FILE_USE(size_t, fwrite, (const void *p, size_t s, size_t n, FILE *f), f, (p, s, n, f))
FILE_USE(char *, fgets, (char *s, int n, FILE *f), f, (s, n, f))
FILE_USE(int, fputs, (const char *s, FILE *f), f, (s, f))
FILE_USE(int, fputc, (int c, FILE *f), f, (c, f))
FILE_USE(int, fgetc, (FILE *f), f, (f))
FILE_USE(int, fflush, (FILE *f), f, (f))
FILE_USE(int, fseek, (FILE *f, long o, int w), f, (f, o, w))
FILE_USE(long, ftell, (FILE *f), f, (f))
FILE_USE(int, feof, (FILE *f), f, (f))
FILE_USE(int, ferror, (FILE *f), f, (f))
FILE_USE(ssize_t, getline, (char **l, size_t *n, FILE *f), f, (l, n, f))
FILE_USE(int, setvbuf, (FILE *f, char *b, int m, size_t s), f, (f, b, m, s))

int __real_vfprintf(FILE *f, const char *fmt, va_list ap);
int __wrap_vfprintf(FILE *f, const char *fmt, va_list ap)
{
    use_ptr("vfprintf", SITE, K_FILE, f);
    return __real_vfprintf(f, fmt, ap);
}

int __wrap_fprintf(FILE *f, const char *fmt, ...)
{
    va_list ap;
    int result;
    use_ptr("fprintf", SITE, K_FILE, f);
    va_start(ap, fmt);
    result = __real_vfprintf(f, fmt, ap);
    va_end(ap);
    return result;
}

/* ---- directories ------------------------------------------------------- */

static DIR *dir_result(const char *call, void *site, DIR *result, int err, int injected, int adopted,
                       const char *path)
{
    sigset_t saved;
    struct operand ops[3];
    int n = 0;
    if (!active()) {
        errno = err;
        return result;
    }
    lock_observer(&saved);
    if (adopted != -2) {
        ops[n++] = op("used", K_FD, adopted, fd_generation(adopted));
    }
    if (result) {
        int child = adopted != -2 ? adopted : __real_dirfd(result);
        ops[n++] = ptr_acquire(K_DIR, (uintptr_t) result, child, 0);
        if (adopted != -2) {
            if (adopted >= 0 && adopted < MAX_FD) {
                fds[adopted].owner_kind = K_DIR;
                fds[adopted].owner = (uintptr_t) result;
            }
            ops[n++] = op("adopted", K_FD, adopted, fd_generation(adopted));
        }
        else if (child >= 0) {
            ops[n++] = fd_acquire("derived", child, K_DIR, (uintptr_t) result);
        }
    }
    record(call, site, NOARGS, (long long) (uintptr_t) result, result ? 0 : err, injected, path, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

DIR *__wrap_opendir(const char *path)
{
    int err = 0;
    if (should_inject("opendir", &err)) {
        return dir_result("opendir", SITE, NULL, err, 1, -2, path);
    }
    DIR *result = __real_opendir(path);
    err = errno;
    return dir_result("opendir", SITE, result, err, 0, -2, path);
}

DIR *__wrap_fdopendir(int fd)
{
    int err = 0;
    if (should_inject("fdopendir", &err)) {
        return dir_result("fdopendir", SITE, NULL, err, 1, fd, NULL);
    }
    DIR *result = __real_fdopendir(fd);
    err = errno;
    return dir_result("fdopendir", SITE, result, err, 0, fd, NULL);
}

int __wrap_closedir(DIR *dir)
{
    sigset_t saved;
    int result;
    int err;
    int child;
    struct operand ops[2];
    int n = 0;
    if (!active()) {
        return __real_closedir(dir);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_closedir(dir);
    err = errno;
    ops[n++] = ptr_release(K_DIR, (uintptr_t) dir, &child);
    if (child >= 0) {
        ops[n++] = fd_release("released_child", child);
    }
    record("closedir", site, NOARGS, result, result != 0 ? err : 0, 0, NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

struct dirent *__wrap_readdir(DIR *dir)
{
    use_ptr("readdir", SITE, K_DIR, dir);
    return __real_readdir(dir);
}

struct dirent64 *__wrap_readdir64(DIR *dir)
{
    use_ptr("readdir", SITE, K_DIR, dir);
    return __real_readdir64(dir);
}

void __wrap_rewinddir(DIR *dir)
{
    use_ptr("rewinddir", SITE, K_DIR, dir);
    __real_rewinddir(dir);
}

int __wrap_dirfd(DIR *dir)
{
    sigset_t saved;
    int result = __real_dirfd(dir);
    int err = errno;
    if (active()) {
        struct operand ops[2];
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        ops[n++] = op("used", K_DIR, (long long) (uintptr_t) dir, ptr_generation((uintptr_t) dir));
        if (result >= 0) {
            ops[n++] = op("related", K_FD, result, fd_generation(result));
        }
        record("dirfd", site, NOARGS, result, result < 0 ? err : 0, 0, NULL, ops, n);
        unlock_observer(&saved);
    }
    errno = err;
    return result;
}

/* ---- dynamic loader handles ---------------------------------------------- */

void *__wrap_dlopen(const char *path, int flags)
{
    sigset_t saved;
    int err = 0;
    int injected = should_inject("dlopen", &err);
    void *result = injected ? NULL : __real_dlopen(path, flags);
    if (active()) {
        struct operand o;
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        if (result) {
            o = ptr_acquire(K_DL, (uintptr_t) result, -1, 0);
            n = 1;
        }
        record("dlopen", site, ARGS(flags), (long long) (uintptr_t) result, 0, injected, path, &o, n);
        unlock_observer(&saved);
    }
    return result;
}

int __wrap_dlclose(void *handle)
{
    sigset_t saved;
    int result;
    int child;
    if (!active()) {
        return __real_dlclose(handle);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_dlclose(handle);
    struct operand o = ptr_release(K_DL, (uintptr_t) handle, &child);
    record("dlclose", site, NOARGS, result, 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
    return result;
}

void *__wrap_dlsym(void *handle, const char *symbol)
{
    if (handle != RTLD_DEFAULT && handle != RTLD_NEXT) {
        use_ptr("dlsym", SITE, K_DL, handle);
    }
    return __real_dlsym(handle, symbol);
}

/* ---- addrinfo lists ------------------------------------------------------ */

int __wrap_getaddrinfo(const char *node, const char *service, const struct addrinfo *hints,
                       struct addrinfo **res)
{
    sigset_t saved;
    int err = 0;
    int result;
    int injected = should_inject("getaddrinfo", &err);
    int saved_errno;
    if (injected) {
        /* EAI_SYSTEM reports its cause in errno; any other EAI code does not. */
        result = err;
        saved_errno = err == EAI_SYSTEM ? EIO : 0;
    }
    else {
        result = __real_getaddrinfo(node, service, hints, res);
        saved_errno = errno;
    }
    if (active()) {
        struct operand o;
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        if (result == 0 && *res) {
            o = ptr_acquire(K_ADDRINFO, (uintptr_t) *res, -1, 0);
            n = 1;
        }
        record("getaddrinfo", site, ARGS(hints ? hints->ai_family : 0, hints ? hints->ai_socktype : 0),
               result, result == EAI_SYSTEM ? saved_errno : 0, injected, node, &o, n);
        unlock_observer(&saved);
    }
    errno = saved_errno;
    return result;
}

void __wrap_freeaddrinfo(struct addrinfo *ai)
{
    sigset_t saved;
    int child;
    if (!active()) {
        __real_freeaddrinfo(ai);
        return;
    }
    void *site = SITE;
    lock_observer(&saved);
    __real_freeaddrinfo(ai);
    struct operand o = ptr_release(K_ADDRINFO, (uintptr_t) ai, &child);
    record("freeaddrinfo", site, NOARGS, 0, 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
}

/* ---- mappings ------------------------------------------------------------- */

static void *map_common(void *site, void *addr, size_t len, int prot, int flags, int fd, off_t off, int use64)
{
    sigset_t saved;
    int err = 0;
    int injected = should_inject_at("mmap", site, &err);
    void *result;
    struct operand ops[2];
    int n = 0;
    if (injected) {
        result = MAP_FAILED;
    }
    else {
        result = use64 ? __real_mmap64(addr, len, prot, flags, fd, off) : __real_mmap(addr, len, prot, flags, fd, off);
        err = errno;
    }
    if (!active()) {
        errno = err;
        return result;
    }
    lock_observer(&saved);
    if (!(flags & MAP_ANONYMOUS) && fd >= 0) {
        ops[n++] = op("used", K_FD, fd, fd_generation(fd));
    }
    if (result != MAP_FAILED) {
        ops[n++] = ptr_acquire(K_MAPPING, (uintptr_t) result, -1, len);
    }
    record("mmap", site, ARGS((long long) len, prot, flags, fd, (long long) off),
           result == MAP_FAILED ? -1 : (long long) (uintptr_t) result, result == MAP_FAILED ? err : 0, injected,
           NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

void *__wrap_mmap(void *addr, size_t len, int prot, int flags, int fd, off_t off)
{
    return map_common(SITE, addr, len, prot, flags, fd, off, 0);
}

void *__wrap_mmap64(void *addr, size_t len, int prot, int flags, int fd, off_t off)
{
    return map_common(SITE, addr, len, prot, flags, fd, off, 1);
}

int __wrap_munmap(void *addr, size_t len)
{
    sigset_t saved;
    int result;
    int err;
    int child;
    if (!active()) {
        return __real_munmap(addr, len);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_munmap(addr, len);
    err = errno;
    struct operand o = ptr_release(K_MAPPING, (uintptr_t) addr, &child);
    o.extent = (long long) len;
    record("munmap", site, ARGS((long long) len), result, result < 0 ? err : 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
    errno = err;
    return result;
}

void *__wrap_mremap(void *old, size_t old_len, size_t new_len, int flags, ...)
{
    sigset_t saved;
    void *fixed = NULL;
    void *result;
    int err;
    int child;
    struct operand ops[2];
    int n = 0;
    if (flags & MREMAP_FIXED) {
        va_list ap;
        va_start(ap, flags);
        fixed = va_arg(ap, void *);
        va_end(ap);
    }
    if (!active()) {
        return __real_mremap(old, old_len, new_len, flags, fixed);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_mremap(old, old_len, new_len, flags, fixed);
    err = errno;
    if (result != MAP_FAILED) {
        ops[n] = ptr_release(K_MAPPING, (uintptr_t) old, &child);
        ops[n++].extent = (long long) old_len;
        ops[n++] = ptr_acquire(K_MAPPING, (uintptr_t) result, -1, new_len);
    }
    else {
        ops[n] = op("used", K_MAPPING, (long long) (uintptr_t) old, ptr_generation((uintptr_t) old));
        ops[n++].extent = (long long) old_len;
    }
    record("mremap", site, ARGS((long long) old_len, (long long) new_len, flags),
           result == MAP_FAILED ? -1 : (long long) (uintptr_t) result, result == MAP_FAILED ? err : 0, 0, NULL,
           ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

int __wrap_msync(void *addr, size_t len, int flags)
{
    use_ptr("msync", SITE, K_MAPPING, addr);
    return __real_msync(addr, len, flags);
}

int __wrap_madvise(void *addr, size_t len, int advice)
{
    use_ptr("madvise", SITE, K_MAPPING, addr);
    return __real_madvise(addr, len, advice);
}

/* ---- descriptor uses (recorded before the call) ---------------------------- */

#define FD_USE(ret, name, params, fd, call_args)            \
    ret __real_##name params;                                \
    ret __wrap_##name params                                 \
    {                                                        \
        use_fd(#name, SITE, fd, ARGS(fd));                   \
        return __real_##name call_args;                      \
    }

ssize_t __wrap_write(int fd, const void *buf, size_t n)
{
    use_fd("write", SITE, fd, ARGS(fd));
    return __real_write(fd, buf, n);
}

FD_USE(ssize_t, read, (int fd, void *b, size_t n), fd, (fd, b, n))
FD_USE(ssize_t, pread, (int fd, void *b, size_t n, off_t o), fd, (fd, b, n, o))
FD_USE(ssize_t, pwrite, (int fd, const void *b, size_t n, off_t o), fd, (fd, b, n, o))
FD_USE(ssize_t, pread64, (int fd, void *b, size_t n, off_t o), fd, (fd, b, n, o))
FD_USE(ssize_t, pwrite64, (int fd, const void *b, size_t n, off_t o), fd, (fd, b, n, o))
FD_USE(ssize_t, readv, (int fd, const struct iovec *v, int n), fd, (fd, v, n))
FD_USE(ssize_t, writev, (int fd, const struct iovec *v, int n), fd, (fd, v, n))
FD_USE(ssize_t, recv, (int fd, void *b, size_t n, int f), fd, (fd, b, n, f))
FD_USE(ssize_t, recvfrom, (int fd, void *b, size_t n, int f, struct sockaddr *a, socklen_t *l), fd, (fd, b, n, f, a, l))
FD_USE(ssize_t, recvmsg, (int fd, struct msghdr *m, int f), fd, (fd, m, f))
FD_USE(ssize_t, send, (int fd, const void *b, size_t n, int f), fd, (fd, b, n, f))
FD_USE(ssize_t, sendto, (int fd, const void *b, size_t n, int f, const struct sockaddr *a, socklen_t l), fd, (fd, b, n, f, a, l))
FD_USE(ssize_t, sendmsg, (int fd, const struct msghdr *m, int f), fd, (fd, m, f))
FD_USE(int, connect, (int fd, const struct sockaddr *a, socklen_t l), fd, (fd, a, l))
FD_USE(int, bind, (int fd, const struct sockaddr *a, socklen_t l), fd, (fd, a, l))
FD_USE(int, listen, (int fd, int n), fd, (fd, n))
FD_USE(int, shutdown, (int fd, int how), fd, (fd, how))
FD_USE(int, getsockopt, (int fd, int l, int o, void *v, socklen_t *n), fd, (fd, l, o, v, n))
FD_USE(int, setsockopt, (int fd, int l, int o, const void *v, socklen_t n), fd, (fd, l, o, v, n))
FD_USE(int, getsockname, (int fd, struct sockaddr *a, socklen_t *l), fd, (fd, a, l))
FD_USE(int, getpeername, (int fd, struct sockaddr *a, socklen_t *l), fd, (fd, a, l))
FD_USE(int, fstat, (int fd, struct stat *s), fd, (fd, s))
FD_USE(int, fstat64, (int fd, struct stat64 *s), fd, (fd, s))
FD_USE(int, ftruncate, (int fd, off_t n), fd, (fd, n))
FD_USE(int, ftruncate64, (int fd, off_t n), fd, (fd, n))
FD_USE(off_t, lseek, (int fd, off_t o, int w), fd, (fd, o, w))
FD_USE(off_t, lseek64, (int fd, off_t o, int w), fd, (fd, o, w))
FD_USE(int, fsync, (int fd), fd, (fd))
FD_USE(int, fdatasync, (int fd), fd, (fd))
FD_USE(int, fchmod, (int fd, mode_t m), fd, (fd, m))
FD_USE(int, flock, (int fd, int op), fd, (fd, op))
FD_USE(int, fallocate, (int fd, int m, off_t o, off_t l), fd, (fd, m, o, l))
FD_USE(int, posix_fallocate, (int fd, off_t o, off_t l), fd, (fd, o, l))
FD_USE(int, isatty, (int fd), fd, (fd))
FD_USE(int, epoll_wait, (int fd, struct epoll_event *e, int n, int t), fd, (fd, e, n, t))
FD_USE(int, epoll_pwait, (int fd, struct epoll_event *e, int n, int t, const sigset_t *s), fd, (fd, e, n, t, s))
FD_USE(int, timerfd_settime, (int fd, int f, const struct itimerspec *n, struct itimerspec *o), fd, (fd, f, n, o))

int __real_epoll_ctl(int epfd, int op, int fd, struct epoll_event *event);
int __wrap_epoll_ctl(int epfd, int op_code, int fd, struct epoll_event *event)
{
    sigset_t saved;
    if (active()) {
        void *site = SITE;
        struct operand ops[2];
        lock_observer(&saved);
        ops[0] = op("used", K_FD, epfd, fd_generation(epfd));
        ops[1] = op("used", K_FD, fd, fd_generation(fd));
        record("epoll_ctl", site, ARGS(epfd, op_code, fd), 0, 0, 0, NULL, ops, 2);
        unlock_observer(&saved);
    }
    return __real_epoll_ctl(epfd, op_code, fd, event);
}

int __real_ioctl(int fd, unsigned long request, ...);
int __wrap_ioctl(int fd, unsigned long request, ...)
{
    va_list ap;
    va_start(ap, request);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    use_fd("ioctl", SITE, fd, ARGS(fd, (long long) request));
    return __real_ioctl(fd, request, arg);
}

/* ---- large-file aliases, process streams, interface lists, raw syscalls --- */

int __wrap_mkstemp64(char *template)
{
    int err = 0;
    int result;
    if (should_inject("mkstemp", &err)) {
        return fd_result("mkstemp", SITE, -1, err, 1, -2, NOARGS, template);
    }
    result = __real_mkstemp64(template);
    err = errno;
    return fd_result("mkstemp", SITE, result, err, 0, -2, NOARGS, template);
}

FILE *__wrap_tmpfile64(void)
{
    int err = 0;
    if (should_inject("tmpfile", &err)) {
        return stream_result("tmpfile", SITE, NULL, err, 1, -2, NULL);
    }
    FILE *result = __real_tmpfile64();
    err = errno;
    return stream_result("tmpfile", SITE, result, err, 0, -2, NULL);
}

/* A popen stream is released only by pclose; its descriptor is derived. */
FILE *__wrap_popen(const char *command, const char *type)
{
    sigset_t saved;
    int err = 0;
    int injected = should_inject("popen", &err);
    FILE *result = injected ? NULL : __real_popen(command, type);
    if (!injected) {
        err = errno;
    }
    if (active()) {
        struct operand ops[2];
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        if (result) {
            int child = __real_fileno(result);
            ops[n++] = ptr_acquire(K_PROCESS_STREAM, (uintptr_t) result, child, 0);
            if (child >= 0) {
                ops[n++] = fd_acquire("derived", child, K_PROCESS_STREAM, (uintptr_t) result);
            }
        }
        record("popen", site, NOARGS, (long long) (uintptr_t) result, result ? 0 : err, injected, command, ops, n);
        unlock_observer(&saved);
    }
    errno = err;
    return result;
}

int __wrap_pclose(FILE *fp)
{
    sigset_t saved;
    int result;
    int err;
    int child;
    struct operand ops[2];
    int n = 0;
    if (!active()) {
        return __real_pclose(fp);
    }
    void *site = SITE;
    lock_observer(&saved);
    result = __real_pclose(fp);
    err = errno;
    ops[n++] = ptr_release(K_PROCESS_STREAM, (uintptr_t) fp, &child);
    if (child >= 0) {
        ops[n++] = fd_release("released_child", child);
    }
    record("pclose", site, NOARGS, result, result == -1 ? err : 0, 0, NULL, ops, n);
    unlock_observer(&saved);
    errno = err;
    return result;
}

int __real_getifaddrs(struct ifaddrs **out);
void __real_freeifaddrs(struct ifaddrs *list);

int __wrap_getifaddrs(struct ifaddrs **out)
{
    sigset_t saved;
    int err = 0;
    int injected = should_inject("getifaddrs", &err);
    int result = injected ? -1 : __real_getifaddrs(out);
    if (!injected) {
        err = errno;
    }
    if (active()) {
        struct operand o;
        int n = 0;
        void *site = SITE;
        lock_observer(&saved);
        if (result == 0 && *out) {
            o = ptr_acquire(K_IFADDRS, (uintptr_t) *out, -1, 0);
            n = 1;
        }
        record("getifaddrs", site, NOARGS, result, result == 0 ? 0 : err, injected, NULL, &o, n);
        unlock_observer(&saved);
    }
    errno = err;
    return result;
}

void __wrap_freeifaddrs(struct ifaddrs *list)
{
    sigset_t saved;
    int child;
    if (!active()) {
        __real_freeifaddrs(list);
        return;
    }
    void *site = SITE;
    lock_observer(&saved);
    __real_freeifaddrs(list);
    struct operand o = ptr_release(K_IFADDRS, (uintptr_t) list, &child);
    record("freeifaddrs", site, NOARGS, 0, 0, 0, NULL, &o, 1);
    unlock_observer(&saved);
}

/*
 * Raw syscall(2): descriptor-affecting numbers are recorded as out-of-model
 * events (the checker counts them and cannot claim completeness past them);
 * every other number passes through unrecorded.
 */
long __real_syscall(long number, ...);
long __wrap_syscall(long number, ...)
{
    va_list ap;
    long a[6];
    void *site = SITE;
    va_start(ap, number);
    for (int i = 0; i < 6; i++) {
        a[i] = va_arg(ap, long);
    }
    va_end(ap);
    long result = __real_syscall(number, a[0], a[1], a[2], a[3], a[4], a[5]);
    int err = errno;
    switch (number) {
    case SYS_open: case SYS_openat: case SYS_close: case SYS_socket: case SYS_accept: case SYS_accept4:
    case SYS_dup: case SYS_dup2: case SYS_dup3: case SYS_pipe: case SYS_pipe2: case SYS_socketpair:
    case SYS_eventfd: case SYS_eventfd2: case SYS_epoll_create: case SYS_epoll_create1:
    case SYS_memfd_create: case SYS_mmap: case SYS_munmap: case SYS_mremap: case SYS_close_range:
    case SYS_timerfd_create: case SYS_inotify_init1: case SYS_signalfd4: case SYS_fcntl:
        if (active()) {
            sigset_t saved;
            lock_observer(&saved);
            record("syscall_descriptor", site, ARGS(number, a[0]), result, result < 0 ? err : 0, 0, NULL, NULL, 0);
            unlock_observer(&saved);
        }
        break;
    default:
        break;
    }
    errno = err;
    return result;
}

/* The observer stops in a forked child (pid check); the parent records it. */
pid_t __real_fork(void);
pid_t __wrap_fork(void)
{
    void *site = SITE;
    pid_t result = __real_fork();
    int err = errno;
    if (result != 0 && active()) {
        sigset_t saved;
        lock_observer(&saved);
        record("fork", site, NOARGS, result, result < 0 ? err : 0, 0, NULL, NULL, 0);
        unlock_observer(&saved);
    }
    errno = err;
    return result;
}
