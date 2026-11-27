/* Request-pipe delay shim for card HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING)
 * (after EXECUTION-QUOTA-LOAD-INDEPENDENCE's amplify.c).
 *
 * Preloaded into one measured test binary only (LD_PRELOAD on that command),
 * never installed. It moves the provider's exit to either side of the host's
 * request write:
 *
 *   QCORE_DELAY_FIFO_WRITE_US=N  sleep N microseconds before the FIRST write to
 *                                each descriptor that is a pipe opened for
 *                                writing (the host's request write lands after
 *                                a fast provider has exited: the branch that
 *                                used to meet Broken pipe);
 *   QCORE_DELAY_FIFO_CLOSE_US=N  sleep N microseconds before closing such a
 *                                descriptor (a provider that reads exactly its
 *                                frame exits before the host closes its end).
 *
 * The test binary's own output goes to a regular file under the harness, so
 * only pipes the process creates are affected: the request pipe and the
 * fixture writer's input pipe.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

static ssize_t (*real_write)(int, const void *, size_t);
static int (*real_close)(int);
static long write_delay_us;
static long close_delay_us;

/* One "already delayed" flag per descriptor number, cleared on close. */
#define TRACKED 4096
static atomic_uchar delayed[TRACKED];

static long env_number(const char *name) {
    const char *value = getenv(name);
    return value ? strtol(value, NULL, 10) : 0;
}

/* Resolved once at load, so a forked child never takes the loader's lock. */
__attribute__((constructor)) static void delay_init(void) {
    real_write = (ssize_t(*)(int, const void *, size_t))dlsym(RTLD_NEXT, "write");
    real_close = (int (*)(int))dlsym(RTLD_NEXT, "close");
    write_delay_us = env_number("QCORE_DELAY_FIFO_WRITE_US");
    close_delay_us = env_number("QCORE_DELAY_FIFO_CLOSE_US");
}

static int is_pipe_writer(int fd) {
    struct stat st;
    if (fstat(fd, &st) != 0 || !S_ISFIFO(st.st_mode)) return 0;
    int flags = fcntl(fd, F_GETFL);
    return flags >= 0 && (flags & O_ACCMODE) == O_WRONLY;
}

ssize_t write(int fd, const void *buffer, size_t count) {
    if (write_delay_us > 0 && fd >= 0 && fd < TRACKED && !atomic_load(&delayed[fd]) &&
        is_pipe_writer(fd)) {
        atomic_store(&delayed[fd], 1);
        usleep((useconds_t)write_delay_us);
    }
    return real_write(fd, buffer, count);
}

int close(int fd) {
    if (close_delay_us > 0 && fd >= 0 && is_pipe_writer(fd)) {
        usleep((useconds_t)close_delay_us);
    }
    if (fd >= 0 && fd < TRACKED) atomic_store(&delayed[fd], 0);
    return real_close(fd);
}
