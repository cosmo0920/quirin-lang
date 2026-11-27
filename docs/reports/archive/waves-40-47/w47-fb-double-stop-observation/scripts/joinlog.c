/*
 * joinlog.c -- LD_PRELOAD interposer for W47-FB-DOUBLE-STOP-OBSERVATION.
 *
 * Logs every pthread_create (the new thread's identifier and the creating
 * call stack) and every pthread_join (the identifier it is given, before the
 * call, and the value it returns, after it) to stderr, one line each,
 * prefixed "JOINLOG". Stack frames are written as `object+offset`, resolved
 * offline with addr2line against the same object (scripts/resolve.py), so a
 * static function such as do_start is named from DWARF, not from dladdr.
 * It changes no argument and no result: it calls the next definition and
 * returns what that returned. Build:
 *   clang -O0 -g -shared -fPIC -o joinlog.so joinlog.c -ldl
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <execinfo.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

typedef int (*create_fn)(pthread_t *, const pthread_attr_t *,
                         void *(*)(void *), void *);
typedef int (*join_fn)(pthread_t, void **);

static atomic_int seq;

static void emit(const char *kind, pthread_t thread, int ret, int with_ret)
{
    void *frames[12];
    char buf[4096];
    int n;
    int len;
    int i;
    Dl_info info;
    const char *base;

    n = backtrace(frames, 12);
    len = snprintf(buf, sizeof(buf),
                   "JOINLOG %s seq=%d self=%#lx thread=%#lx",
                   kind, atomic_fetch_add(&seq, 1),
                   (unsigned long) pthread_self(), (unsigned long) thread);
    if (with_ret) {
        len += snprintf(buf + len, sizeof(buf) - len, " ret=%d", ret);
    }
    len += snprintf(buf + len, sizeof(buf) - len, " frames=");
    /* frame 0 is emit itself; frame 1 the interposed function */
    for (i = 2; i < n && len < (int) sizeof(buf) - 256; i++) {
        if (dladdr(frames[i], &info) && info.dli_fname != NULL) {
            base = strrchr(info.dli_fname, '/');
            base = base ? base + 1 : info.dli_fname;
            len += snprintf(buf + len, sizeof(buf) - len, "%s+%#lx;", base,
                            (unsigned long) ((char *) frames[i] -
                                             (char *) info.dli_fbase));
        }
        else {
            len += snprintf(buf + len, sizeof(buf) - len, "?%p;", frames[i]);
        }
    }
    len += snprintf(buf + len, sizeof(buf) - len, "\n");
    if (write(2, buf, (size_t) len) < 0) {
        /* nothing to do: the log is best effort, the call is unchanged */
    }
}

int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                   void *(*start)(void *), void *arg)
{
    static create_fn real;
    int ret;

    if (real == NULL) {
        real = (create_fn) dlsym(RTLD_NEXT, "pthread_create");
    }
    ret = real(thread, attr, start, arg);
    emit("create", ret == 0 ? *thread : 0, ret, 1);
    return ret;
}

int pthread_join(pthread_t thread, void **retval)
{
    static join_fn real;
    int ret;

    if (real == NULL) {
        real = (join_fn) dlsym(RTLD_NEXT, "pthread_join");
    }
    emit("join-enter", thread, 0, 0);
    ret = real(thread, retval);
    emit("join-return", thread, ret, 1);
    return ret;
}
