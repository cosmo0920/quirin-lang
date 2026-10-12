/* Window amplifiers for card EXECUTION-QUOTA-LOAD-INDEPENDENCE.
 *
 * Preloaded into one measured test binary only (LD_PRELOAD on that command),
 * never installed. Each mode widens one window the card names, so a race that
 * a loaded host hits rarely is hit often enough to compare before and after:
 *
 *   QCORE_AMPLIFY_CLOSE_US=N    sleep N microseconds before closing a
 *                               descriptor opened for writing on a file whose
 *                               name ends in "/provider.sh" (mechanism b: the
 *                               window in which a concurrent fork inherits the
 *                               fixture's write descriptor);
 *   QCORE_AMPLIFY_REALTIME_NS=N round CLOCK_REALTIME down to a multiple of N
 *                               nanoseconds (mechanism d: a coarse wall clock,
 *                               so two threads of one process read the same
 *                               value).
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static int (*real_close)(int);
static int (*real_clock_gettime)(clockid_t, struct timespec *);
static long close_delay_us;
static long realtime_grain_ns;

static long env_number(const char *name) {
    const char *value = getenv(name);
    return value ? strtol(value, NULL, 10) : 0;
}

/* Resolved once at load, so a forked child never takes the loader's lock. */
__attribute__((constructor)) static void amplify_init(void) {
    real_close = (int (*)(int))dlsym(RTLD_NEXT, "close");
    real_clock_gettime =
        (int (*)(clockid_t, struct timespec *))dlsym(RTLD_NEXT, "clock_gettime");
    close_delay_us = env_number("QCORE_AMPLIFY_CLOSE_US");
    realtime_grain_ns = env_number("QCORE_AMPLIFY_REALTIME_NS");
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
                size_t suffix = strlen("/provider.sh");
                if ((size_t)length >= suffix &&
                    strcmp(target + length - suffix, "/provider.sh") == 0) {
                    usleep((useconds_t)close_delay_us);
                }
            }
        }
    }
    return real_close(fd);
}

int clock_gettime(clockid_t clock, struct timespec *now) {
    int result = real_clock_gettime(clock, now);
    if (result == 0 && clock == CLOCK_REALTIME && realtime_grain_ns > 1) {
        now->tv_nsec -= now->tv_nsec % realtime_grain_ns;
    }
    return result;
}
