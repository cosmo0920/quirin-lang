/*
 * libc descriptor probe for W43-DESCRIPTOR-NATIVE-CHECK.
 *
 * Fluent Bit's pinned paths never call several descriptor APIs whose libc
 * contracts are declared (dup, dup2, fcntl(F_DUPFD), fdopen, eventfd,
 * socketpair, ...). This driver executes each such libc callee directly,
 * once on a success path and once on a natural failure path, so every
 * declared callee has recorded executions. It is a correct client: it
 * releases everything it acquires. Deterministic failure injection
 * (QCORE_DESC_INJECT) exercises the injected-failure arm of the same calls.
 * Usage: libc_descriptor_probe SCRATCH_DIR
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

static char scratch[4096];

static const char *path_in(const char *name)
{
    static char buffer[8192];
    snprintf(buffer, sizeof(buffer), "%s/%s", scratch, name);
    return buffer;
}

static void descriptors(void)
{
    int fd = open(path_in("probe.txt"), O_RDWR | O_CREAT | O_TRUNC, 0600);
    if (fd >= 0) {
        (void) write(fd, "probe\n", 6);
        int copy = dup(fd);
        if (copy >= 0) {
            close(copy);
        }
        int high = fcntl(fd, F_DUPFD_CLOEXEC, 50);
        if (high >= 0) {
            close(high);
        }
        int target = open("/dev/null", O_RDONLY);
        if (target >= 0) {
            /* dup2 onto a live descriptor replaces its generation. */
            if (dup2(fd, target) == target) {
                close(target);
            }
            else {
                close(target);
            }
        }
        /* dup2(fd, fd) returns fd and changes nothing. */
        (void) dup2(fd, fd);
        close(fd);
    }
    /* Natural failures. */
    (void) open(path_in("missing/none"), O_RDONLY);
    (void) openat(AT_FDCWD, path_in("missing/none"), O_RDONLY);
    int dir = open(scratch, O_RDONLY | O_DIRECTORY);
    if (dir >= 0) {
        int child = openat(dir, "probe.txt", O_RDONLY);
        if (child >= 0) {
            close(child);
        }
        close(dir);
    }
}

static void pairs_and_events(void)
{
    int pair[2];
    if (pipe(pair) == 0) {
        close(pair[0]);
        close(pair[1]);
    }
    if (pipe2(pair, O_CLOEXEC) == 0) {
        close(pair[0]);
        close(pair[1]);
    }
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, pair) == 0) {
        close(pair[0]);
        close(pair[1]);
    }
    (void) socketpair(AF_INET, SOCK_STREAM, 12345, pair); /* natural failure */
    int ep = epoll_create1(EPOLL_CLOEXEC);
    int ev = eventfd(0, EFD_CLOEXEC);
    if (ep >= 0 && ev >= 0) {
        struct epoll_event event = {.events = EPOLLIN};
        (void) epoll_ctl(ep, EPOLL_CTL_ADD, ev, &event);
        (void) epoll_ctl(ep, EPOLL_CTL_DEL, ev, NULL);
    }
    if (ev >= 0) {
        close(ev);
    }
    if (ep >= 0) {
        close(ep);
    }
    (void) epoll_create1(-1); /* natural failure: EINVAL */
    (void) eventfd(0, -1);    /* natural failure: EINVAL */
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock >= 0) {
        close(sock);
    }
    (void) socket(-1, SOCK_STREAM, 0); /* natural failure */
    int listener = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0);
    if (listener >= 0) {
        (void) listen(listener, 1);
        (void) accept4(listener, NULL, NULL, SOCK_CLOEXEC); /* EAGAIN: nothing pending */
        close(listener);
    }
}

static void streams(void)
{
    FILE *fp = fopen(path_in("probe.txt"), "r");
    if (fp) {
        char line[64];
        (void) fgets(line, sizeof(line), fp);
        (void) fileno(fp);
        fclose(fp);
    }
    (void) fopen(path_in("missing/none"), "r"); /* natural failure */
    int fd = open(path_in("probe.txt"), O_RDONLY);
    if (fd >= 0) {
        FILE *adopted = fdopen(fd, "r");
        if (adopted) {
            (void) fileno(adopted);
            fclose(adopted); /* releases fd with the stream */
        }
        else {
            close(fd);
        }
    }
    int readonly = open(path_in("probe.txt"), O_RDONLY);
    if (readonly >= 0) {
        FILE *bad = fdopen(readonly, "w"); /* natural failure: EINVAL, fd stays with caller */
        if (bad) {
            fclose(bad);
        }
        else {
            close(readonly);
        }
    }
    FILE *tmp = tmpfile();
    if (tmp) {
        fclose(tmp);
    }
    DIR *dir = opendir(scratch);
    if (dir) {
        while (readdir(dir) != NULL) {
        }
        (void) dirfd(dir);
        closedir(dir);
    }
    (void) opendir(path_in("missing")); /* natural failure */
    int dfd = open(scratch, O_RDONLY | O_DIRECTORY);
    if (dfd >= 0) {
        DIR *adopted = fdopendir(dfd);
        if (adopted) {
            closedir(adopted);
        }
        else {
            close(dfd);
        }
    }
}

static void handles_lists_mappings(void)
{
    void *self = dlopen("libm.so.6", RTLD_LAZY);
    void *again = dlopen("libm.so.6", RTLD_LAZY);
    if (self) {
        (void) dlsym(self, "cos");
    }
    if (again) {
        dlclose(again);
    }
    if (self) {
        dlclose(self);
    }
    (void) dlopen("/nonexistent/libnone.so", RTLD_LAZY); /* natural failure */

    struct addrinfo hints;
    struct addrinfo *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_flags = AI_NUMERICHOST;
    if (getaddrinfo("127.0.0.1", "80", &hints, &res) == 0) {
        freeaddrinfo(res);
    }
    res = NULL;
    (void) getaddrinfo("not-a-number", "80", &hints, &res); /* EAI_NONAME, no network */

    long page = sysconf(_SC_PAGESIZE);
    void *map = mmap(NULL, (size_t) page * 2, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (map != MAP_FAILED) {
        (void) msync(map, (size_t) page, MS_ASYNC);
        void *grown = mremap(map, (size_t) page * 2, (size_t) page * 4, MREMAP_MAYMOVE);
        if (grown != MAP_FAILED) {
            munmap(grown, (size_t) page * 4);
        }
        else {
            munmap(map, (size_t) page * 2);
        }
    }
    int fd = open(path_in("probe.txt"), O_RDONLY);
    if (fd >= 0) {
        void *file_map = mmap(NULL, (size_t) page, PROT_READ, MAP_SHARED, fd, 0);
        close(fd); /* the mapping outlives the descriptor */
        if (file_map != MAP_FAILED) {
            munmap(file_map, (size_t) page);
        }
    }
    (void) mmap(NULL, 0, PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0); /* natural failure: EINVAL */
}

int main(int argc, char **argv)
{
    if (argc != 2 || strlen(argv[1]) >= sizeof(scratch)) {
        return 2;
    }
    strcpy(scratch, argv[1]);
    descriptors();
    pairs_and_events();
    streams();
    handles_lists_mappings();
    unlink(path_in("probe.txt"));
    return 0;
}
