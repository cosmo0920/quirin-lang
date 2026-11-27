/* Window amplifier for card PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE,
 * generalised from EXECUTION-QUOTA-LOAD-INDEPENDENCE's scripts/amplify.c.
 *
 * Preloaded into one measured test binary only (LD_PRELOAD on that command),
 * never installed. It widens one window: the time a process holds a
 * descriptor open for writing on an executable fixture, during which a
 * concurrent fork by another thread inherits a duplicate.
 *
 *   QCORE_AMPLIFY_CLOSE_US=N         sleep N microseconds before closing a
 *                                    descriptor opened for writing;
 *   QCORE_AMPLIFY_CLOSE_SUFFIXES=S   only for files whose path ends in one of
 *                                    the colon-separated suffixes in S (default
 *                                    "/provider.sh", the earlier card's); an
 *                                    entry "~TEXT" matches TEXT anywhere.
 *
 * The preload is inherited by the measured binary's children, so in the fixed
 * tree the out-of-process writer (`cat`) is slowed the same way. That is
 * deliberate: the window is the same length, and only who holds it differs.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int (*real_close)(int);
static long close_delay_us;
static char suffixes[1024] = "/provider.sh";

/* Resolved once at load, so a forked child never takes the loader's lock. */
__attribute__((constructor)) static void amplify_init(void) {
    real_close = (int (*)(int))dlsym(RTLD_NEXT, "close");
    const char *delay = getenv("QCORE_AMPLIFY_CLOSE_US");
    close_delay_us = delay ? strtol(delay, NULL, 10) : 0;
    const char *list = getenv("QCORE_AMPLIFY_CLOSE_SUFFIXES");
    if (list && strlen(list) < sizeof suffixes) {
        strcpy(suffixes, list);
    }
}

static int matches(const char *target, size_t length) {
    const char *start = suffixes;
    while (*start) {
        const char *end = strchr(start, ':');
        size_t size = end ? (size_t)(end - start) : strlen(start);
        if (size > 1 && start[0] == '~') {
            /* "~TEXT": TEXT anywhere in the path. */
            char needle[1024];
            memcpy(needle, start + 1, size - 1);
            needle[size - 1] = '\0';
            if (strstr(target, needle)) {
                return 1;
            }
        } else if (size > 0 && length >= size &&
                   strncmp(target + length - size, start, size) == 0) {
            return 1;
        }
        if (!end) {
            break;
        }
        start = end + 1;
    }
    return 0;
}

int close(int fd) {
    if (close_delay_us > 0) {
        int flags = fcntl(fd, F_GETFL);
        if (flags >= 0 && (flags & O_ACCMODE) != O_RDONLY) {
            char link[64], target[4096];
            snprintf(link, sizeof link, "/proc/self/fd/%d", fd);
            ssize_t length = readlink(link, target, sizeof target - 1);
            if (length > 0) {
                target[length] = '\0';
                if (matches(target, (size_t)length)) {
                    usleep((useconds_t)close_delay_us);
                }
            }
        }
    }
    return real_close(fd);
}
