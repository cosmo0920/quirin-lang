/* qcore_event (card STD-EVENT): the readiness primitives an emitted
 * `source.event.v1` program uses, and nothing more.
 *
 * Kept embedded, on the `c11_threads_compat.h` precedent, so emitted C needs no
 * QCore include path and no linked QCore artifact. It is a *subset*, not a
 * replacement for <poll.h>: only a program-created endpoint pair, only
 * POLLIN/POLLOUT, only a single-threaded wait, and no timers, signals or edge
 * triggering.
 *
 * THREE ARMS, on the same shape the threads shim uses: `_WIN32`, `__APPLE__`,
 * and everything else (Linux and other POSIX). The two POSIX arms select the
 * *same* implementation below, by name, and they are separate arms because what
 * each has to declare to expose poll(2) under `-std=c11 -pedantic` differs:
 * glibc needs `_POSIX_C_SOURCE`, and Apple's `sys/cdefs.h` keys its visibility
 * level off `_DARWIN_C_SOURCE`. Writing the shared body twice would be
 * duplication, not portability; the arms are where the platforms actually
 * differ.
 *
 * Deliberately NOT kqueue (Apple) or epoll (Linux): poll(2) is the portable
 * deterministic subset, it is what both platforms document for a socket-pair
 * endpoint, and v1's determinism argument is about a *declared* ready set of at
 * most one member — for which neither interface's extra scalability buys
 * anything.
 *
 * THE DESCRIPTOR TYPE DIFFERS PER ARM, and the difference is contained:
 *
 *   - POSIX arms: `int`, from socketpair(AF_UNIX, SOCK_STREAM, 0).
 *   - Windows arm: `SOCKET` (an unsigned handle, not a CRT file descriptor),
 *     from a loopback TCP pair — Windows has no socketpair(2), and WSAPoll
 *     takes SOCKETs rather than fds.
 *
 * The **trace never carries either**. Every identity an emitted program prints
 * is the model identity the checker assigned, so the byte-for-byte observation
 * is identical on all three arms and a real descriptor number — which is not
 * model-determined — never reaches a comparison.
 *
 * What is claimed, and what is not:
 *
 *   - `qcore_event_wait` does not trust the declared readiness. It compares the
 *     ready set the platform actually reported against the one the checked
 *     program declared and returns non-zero on disagreement, so the named
 *     premise EVENT-V1-LEVEL-TRIGGERED-READINESS produces a *failed run* if it
 *     is false rather than a differently-shaped trace.
 *   - No durability of any kind is claimed. Nothing here is an fsync.
 *   - The Windows pair is a loopback TCP connection rather than an AF_UNIX
 *     pair. It is still private to this process, connected, and two-ended, and
 *     the one-byte wake/drain semantics this shim uses are the same; its
 *     provenance is not, and that is stated rather than papered over.
 *   - WSAPoll's documented POLLOUT-on-connect defect is unreachable from an
 *     emitted v1 program: a write interest makes a module check-only, so
 *     QCORE_EVENT_POLLOUT is never emitted into a program that runs.
 */
#ifndef QCORE_EVENT_POLL_COMPAT_H
#define QCORE_EVENT_POLL_COMPAT_H

/* --- Arm 1 of 3: Windows. ------------------------------------------------ */
#if defined(_WIN32)

#include <winsock2.h>
#include <ws2tcpip.h>
#include <string.h>

/* Keep the emitted program self-contained on this arm too: without this the
 * caller would have to add `ws2_32.lib` to every build command, which the other
 * two arms need no analogue of. MSVC and clang-cl both honour it. */
#if defined(_MSC_VER)
#pragma comment(lib, "ws2_32.lib")
#endif

typedef SOCKET qcore_event_fd;
typedef WSAPOLLFD qcore_event_pollfd;
#define QCORE_EVENT_POLLIN POLLRDNORM
#define QCORE_EVENT_POLLOUT POLLWRNORM

/* --- Arm 2 of 3: Apple. -------------------------------------------------- */
#elif defined(__APPLE__)

/* Must precede every libc include in this translation unit. Apple's
 * `sys/cdefs.h` selects its visibility level from these macros, and under
 * `-std=c11 -pedantic` (which defines __STRICT_ANSI__) the default would
 * otherwise hide poll(2), socketpair(2), send(2) and recv(2). `_DARWIN_C_SOURCE`
 * rather than `_POSIX_C_SOURCE` is the platform's own spelling for "expose the
 * full set", and it is what the Apple arm of the threads shim's neighbours
 * use. */
#ifndef _DARWIN_C_SOURCE
#define _DARWIN_C_SOURCE 1
#endif

#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>

typedef int qcore_event_fd;
typedef struct pollfd qcore_event_pollfd;
#define QCORE_EVENT_POLLIN POLLIN
#define QCORE_EVENT_POLLOUT POLLOUT
/* Selects the shared POSIX implementation below. Apple and Linux differ in what
 * they must declare, not in what poll(2) and socketpair(2) then do. */
#define QCORE_EVENT_POSIX 1

/* --- Arm 3 of 3: Linux and other POSIX. ---------------------------------- */
#else

/* Must precede every libc include in this translation unit: under
 * `-std=c11 -pedantic` glibc defines __STRICT_ANSI__ and would otherwise hide
 * poll(2), socketpair(2), send(2) and recv(2). */
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif

#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>

typedef int qcore_event_fd;
typedef struct pollfd qcore_event_pollfd;
#define QCORE_EVENT_POLLIN POLLIN
#define QCORE_EVENT_POLLOUT POLLOUT
#define QCORE_EVENT_POSIX 1

#endif

#if defined(__GNUC__) || defined(__clang__)
#define QCORE_EVENT_UNUSED __attribute__((unused))
#else
#define QCORE_EVENT_UNUSED
#endif

/* Prepare the platform's socket layer. Zero on success. */
QCORE_EVENT_UNUSED static int qcore_event_startup(void)
{
#if defined(QCORE_EVENT_POSIX)
    return 0;
#else
    WSADATA data;
    return WSAStartup(MAKEWORD(2, 2), &data) == 0 ? 0 : 1;
#endif
}

QCORE_EVENT_UNUSED static int qcore_event_close(qcore_event_fd endpoint)
{
#if defined(QCORE_EVENT_POSIX)
    return close(endpoint) == 0 ? 0 : 1;
#else
    return closesocket(endpoint) == 0 ? 0 : 1;
#endif
}

/* One connected endpoint pair, both owned by this process. Zero on success. */
QCORE_EVENT_UNUSED static int qcore_event_pair(qcore_event_fd *first, qcore_event_fd *second)
{
#if defined(QCORE_EVENT_POSIX)
    int pair[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, pair) != 0) {
        return 1;
    }
    *first = pair[0];
    *second = pair[1];
    return 0;
#else
    /* Windows has no socketpair(2). A loopback listen/connect/accept gives the
     * same private, connected, two-ended pair; see the header comment. */
    SOCKET listener = INVALID_SOCKET;
    SOCKET client = INVALID_SOCKET;
    SOCKET server = INVALID_SOCKET;
    struct sockaddr_in address;
    int length = (int) sizeof address;

    listener = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (listener == INVALID_SOCKET) {
        return 1;
    }
    memset(&address, 0, sizeof address);
    address.sin_family = AF_INET;
    address.sin_port = 0;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    if (bind(listener, (struct sockaddr *) &address, (int) sizeof address) != 0
        || listen(listener, 1) != 0
        || getsockname(listener, (struct sockaddr *) &address, &length) != 0) {
        closesocket(listener);
        return 1;
    }
    client = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (client == INVALID_SOCKET) {
        closesocket(listener);
        return 1;
    }
    if (connect(client, (struct sockaddr *) &address, length) != 0) {
        closesocket(client);
        closesocket(listener);
        return 1;
    }
    server = accept(listener, NULL, NULL);
    closesocket(listener);
    if (server == INVALID_SOCKET) {
        closesocket(client);
        return 1;
    }
    *first = client;
    *second = server;
    return 0;
#endif
}

/* Write one byte into `peer`, so a read registration on the *other* endpoint of
 * the pair becomes ready. `send`/`recv` rather than `write`/`read` because the
 * Windows arm's endpoints are SOCKETs and not CRT file descriptors; both are
 * POSIX on the other two arms. */
QCORE_EVENT_UNUSED static int qcore_event_wake(qcore_event_fd peer)
{
    const char byte = 'q';
    return send(peer, &byte, 1, 0) == 1 ? 0 : 1;
}

/* Read one byte from `endpoint`. This is the whole body of a v1 callback. */
QCORE_EVENT_UNUSED static int qcore_event_drain(qcore_event_fd endpoint)
{
    char byte = 0;
    return recv(endpoint, &byte, 1, 0) == 1 ? 0 : 1;
}

/* One dispatch turn, verified against the declaration.
 *
 * `expected` is the index in `set` the checked program declared ready, or a
 * negative number for a turn declared to report nothing. Returns zero when the
 * reported ready set is exactly the declared one, and non-zero otherwise —
 * including when the wait itself fails, and when it times out on a turn that
 * declared a ready registration. */
QCORE_EVENT_UNUSED static int qcore_event_wait(qcore_event_pollfd *set, unsigned int count,
                                               int timeout_ms, int expected)
{
    unsigned int index = 0;
    int reported = 0;
    int result;

    for (index = 0; index < count; index++) {
        set[index].revents = 0;
    }
#if defined(QCORE_EVENT_POSIX)
    result = poll(set, (nfds_t) count, timeout_ms);
#else
    result = WSAPoll(set, (ULONG) count, timeout_ms);
#endif
    if (result < 0) {
        return 1;
    }
    for (index = 0; index < count; index++) {
        if (set[index].revents != 0) {
            reported++;
            if (expected < 0 || (unsigned int) expected != index) {
                return 1;
            }
        }
    }
    if (expected < 0) {
        return reported == 0 ? 0 : 1;
    }
    return reported == 1 ? 0 : 1;
}

#endif /* QCORE_EVENT_POLL_COMPAT_H */
