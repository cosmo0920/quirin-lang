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
 *     from a pathname-backed AF_UNIX stream pair when the SDK and running
 *     system support it, otherwise from a loopback TCP pair. Winsock has no
 *     socketpair(2), and WSAPoll takes SOCKETs rather than fds.
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
 *   - The Windows arm probes AF_UNIX at run time rather than inferring support
 *     from the SDK. Only an unsupported family/type/protocol response selects
 *     the loopback fallback; other probe failures and every failure after
 *     AF_UNIX was found refuse pair creation rather than hiding a resource,
 *     pathname or cleanup failure behind another transport.
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
#include <windows.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

/* AF_UNIX is a Windows 10 addition and older SDKs have no sockaddr_un. Keep
 * those SDKs building the measured loopback route; a newer header only makes
 * the AF_UNIX candidate compilable and is not evidence that the running system
 * supports it. */
#if !defined(QCORE_EVENT_TEST_DISABLE_AF_UNIX) && defined(__has_include)
#if __has_include(<afunix.h>)
#include <afunix.h>
#define QCORE_EVENT_WINDOWS_AF_UNIX 1
#endif
#endif

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

#if defined(QCORE_EVENT_WINDOWS_AF_UNIX)
struct qcore_event_afunix_pair {
    SOCKET first;
    SOCKET second;
    WCHAR listener_path[MAX_PATH];
    WCHAR connector_path[MAX_PATH];
    struct qcore_event_afunix_pair *next;
};

static struct qcore_event_afunix_pair *qcore_event_afunix_pairs;
static int qcore_event_afunix_cleanup_registered;
static int qcore_event_afunix_capability = -1;

/* Early primitive failure returns from main, so the C runtime's atexit path is
 * the one place that can close a published pair and then remove its pathname.
 * Normal explicit closes remove each record before this handler runs. */
static void qcore_event_afunix_cleanup(void)
{
    struct qcore_event_afunix_pair *record = qcore_event_afunix_pairs;
    while (record != NULL) {
        struct qcore_event_afunix_pair *next = record->next;
        if (record->first != INVALID_SOCKET) {
            closesocket(record->first);
        }
        if (record->second != INVALID_SOCKET) {
            closesocket(record->second);
        }
        DeleteFileW(record->listener_path);
        DeleteFileW(record->connector_path);
        HeapFree(GetProcessHeap(), 0, record);
        record = next;
    }
    qcore_event_afunix_pairs = NULL;
}

static int qcore_event_afunix_remember(
    SOCKET first, SOCKET second, const WCHAR listener_path[MAX_PATH],
    const WCHAR connector_path[MAX_PATH])
{
    struct qcore_event_afunix_pair *record =
        (struct qcore_event_afunix_pair *) HeapAlloc(
            GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof *record);
    if (record == NULL) {
        return 1;
    }
    if (!qcore_event_afunix_cleanup_registered) {
        if (atexit(qcore_event_afunix_cleanup) != 0) {
            HeapFree(GetProcessHeap(), 0, record);
            return 1;
        }
        qcore_event_afunix_cleanup_registered = 1;
    }
    record->first = first;
    record->second = second;
    memcpy(record->listener_path, listener_path, sizeof record->listener_path);
    memcpy(record->connector_path, connector_path, sizeof record->connector_path);
    record->next = qcore_event_afunix_pairs;
    qcore_event_afunix_pairs = record;
    return 0;
}

static int qcore_event_afunix_closed(SOCKET endpoint)
{
    struct qcore_event_afunix_pair **link = &qcore_event_afunix_pairs;
    while (*link != NULL) {
        struct qcore_event_afunix_pair *record = *link;
        if (record->first == endpoint) {
            record->first = INVALID_SOCKET;
        } else if (record->second == endpoint) {
            record->second = INVALID_SOCKET;
        } else {
            link = &record->next;
            continue;
        }
        if (record->first == INVALID_SOCKET
            && record->second == INVALID_SOCKET) {
            int deletion_failed = 0;
            if (!DeleteFileW(record->listener_path)
                && GetLastError() != ERROR_FILE_NOT_FOUND) {
                deletion_failed = 1;
            }
            if (!DeleteFileW(record->connector_path)
                && GetLastError() != ERROR_FILE_NOT_FOUND) {
                deletion_failed = 1;
            }
            if (deletion_failed) {
                return 1;
            }
            *link = record->next;
            HeapFree(GetProcessHeap(), 0, record);
        }
        return 0;
    }
    return 0;
}

/* One stable capability decision for this single-threaded event program.
 * Unsupported family/type/protocol errors select compatibility; resource or
 * provider failures refuse instead of silently changing transports. */
static int qcore_event_windows_afunix_capability(void)
{
    SOCKET probe;
    int error;
    if (qcore_event_afunix_capability >= 0) {
        return qcore_event_afunix_capability;
    }
    probe = socket(AF_UNIX, SOCK_STREAM, 0);
    if (probe == INVALID_SOCKET) {
        error = WSAGetLastError();
        if (error == WSAEAFNOSUPPORT || error == WSAESOCKTNOSUPPORT
            || error == WSAEPROTONOSUPPORT) {
            qcore_event_afunix_capability = 0;
            return 0;
        }
        return -1;
    }
    if (closesocket(probe) != 0) {
        return -1;
    }
    qcore_event_afunix_capability = 1;
    return 1;
}
#endif

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
    if (closesocket(endpoint) != 0) {
        return 1;
    }
#if defined(QCORE_EVENT_WINDOWS_AF_UNIX)
    return qcore_event_afunix_closed(endpoint);
#else
    return 0;
#endif
#endif
}

#if defined(QCORE_EVENT_WINDOWS_AF_UNIX)
/* A Windows pathname socket pair, following Libevent's runtime-selected
 * evutil_socketpair route and additionally binding the connector so its
 * accepted-peer identity is checkable. Two temporary ordinary files reserve
 * fresh names; after deleting those owned files, bind creates the AF_UNIX
 * reparse points. */
QCORE_EVENT_UNUSED static int qcore_event_pair_afunix(qcore_event_fd *first,
                                                       qcore_event_fd *second)
{
    SOCKET listener = INVALID_SOCKET;
    SOCKET connector = INVALID_SOCKET;
    SOCKET acceptor = INVALID_SOCKET;
    struct sockaddr_un listener_address;
    struct sockaddr_un connector_address;
    struct sockaddr_un accepted_address;
    WCHAR temporary_directory[MAX_PATH];
    WCHAR listener_path[MAX_PATH];
    WCHAR connector_path[MAX_PATH];
    DWORD directory_length;
    int address_length;
    int listener_bound = 0;
    int connector_bound = 0;
    int listener_temporary_created = 0;
    int connector_temporary_created = 0;

    memset(&listener_address, 0, sizeof listener_address);
    memset(&connector_address, 0, sizeof connector_address);
    memset(&accepted_address, 0, sizeof accepted_address);
    memset(temporary_directory, 0, sizeof temporary_directory);
    memset(listener_path, 0, sizeof listener_path);
    memset(connector_path, 0, sizeof connector_path);

    listener = socket(AF_UNIX, SOCK_STREAM, 0);
    if (listener == INVALID_SOCKET) {
        return 1;
    }
    directory_length = GetTempPathW(MAX_PATH, temporary_directory);
    if (directory_length == 0 || directory_length >= MAX_PATH
        || GetTempFileNameW(temporary_directory, L"qel", 0, listener_path) == 0) {
        goto fail;
    }
    listener_temporary_created = 1;
    if (!DeleteFileW(listener_path)) {
        goto fail;
    }
    listener_temporary_created = 0;
    listener_address.sun_family = AF_UNIX;
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, listener_path, -1,
                            listener_address.sun_path,
                            (int) sizeof listener_address.sun_path,
                            NULL, NULL) == 0) {
        goto fail;
    }
    if (bind(listener, (struct sockaddr *) &listener_address,
             (int) sizeof listener_address) != 0) {
        goto fail;
    }
    listener_bound = 1;
    if (listen(listener, 1) != 0) {
        goto fail;
    }

    connector = socket(AF_UNIX, SOCK_STREAM, 0);
    if (connector == INVALID_SOCKET) {
        goto fail;
    }
    if (GetTempFileNameW(temporary_directory, L"qec", 0, connector_path) == 0) {
        goto fail;
    }
    connector_temporary_created = 1;
    if (!DeleteFileW(connector_path)) {
        goto fail;
    }
    connector_temporary_created = 0;
    connector_address.sun_family = AF_UNIX;
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, connector_path, -1,
                            connector_address.sun_path,
                            (int) sizeof connector_address.sun_path,
                            NULL, NULL) == 0
        || bind(connector, (struct sockaddr *) &connector_address,
                (int) sizeof connector_address) != 0) {
        goto fail;
    }
    connector_bound = 1;
    if (connect(connector, (struct sockaddr *) &listener_address,
                (int) sizeof listener_address) != 0) {
        goto fail;
    }

    address_length = (int) sizeof accepted_address;
    acceptor = accept(listener, (struct sockaddr *) &accepted_address,
                      &address_length);
    if (acceptor == INVALID_SOCKET
        || address_length != (int) sizeof accepted_address) {
        goto fail;
    }
    memset(&connector_address, 0, sizeof connector_address);
    address_length = (int) sizeof connector_address;
    if (getsockname(connector, (struct sockaddr *) &connector_address,
                    &address_length) != 0
        || address_length <= (int) offsetof(struct sockaddr_un, sun_path)
        || address_length > (int) sizeof connector_address
        || memchr(connector_address.sun_path, '\0',
                  (size_t) address_length
                      - offsetof(struct sockaddr_un, sun_path)) == NULL
        || accepted_address.sun_family != AF_UNIX
        || connector_address.sun_family != AF_UNIX
        || _stricmp(accepted_address.sun_path, connector_address.sun_path) != 0) {
        goto fail;
    }

    if (closesocket(listener) != 0) {
        goto fail;
    }
    listener = INVALID_SOCKET;
    if (qcore_event_afunix_remember(connector, acceptor, listener_path,
                                    connector_path) != 0) {
        goto fail;
    }
    *first = connector;
    *second = acceptor;
#if defined(QCORE_EVENT_TEST_ABORT_AFTER_AF_UNIX_PAIR)
    return 1;
#else
    return 0;
#endif

fail:
    if (listener != INVALID_SOCKET) {
        closesocket(listener);
    }
    if (connector != INVALID_SOCKET) {
        closesocket(connector);
    }
    if (acceptor != INVALID_SOCKET) {
        closesocket(acceptor);
    }
    if (listener_bound || listener_temporary_created) {
        DeleteFileW(listener_path);
    }
    if (connector_bound || connector_temporary_created) {
        DeleteFileW(connector_path);
    }
    return 1;
}
#endif

#if defined(_WIN32)
QCORE_EVENT_UNUSED static int qcore_event_pair_loopback(qcore_event_fd *first,
                                                         qcore_event_fd *second)
{
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
}
#endif

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
#if !defined(QCORE_EVENT_WINDOWS_AF_UNIX)
#if defined(QCORE_EVENT_TEST_REQUIRE_AF_UNIX)
    return 1;
#else
    return qcore_event_pair_loopback(first, second);
#endif
#else
#if defined(QCORE_EVENT_TEST_FORCE_LOOPBACK)
    return qcore_event_pair_loopback(first, second);
#else
    int capability = qcore_event_windows_afunix_capability();
    if (capability > 0) {
        return qcore_event_pair_afunix(first, second);
    }
#if defined(QCORE_EVENT_TEST_REQUIRE_AF_UNIX)
    return 1;
#else
    return capability == 0 ? qcore_event_pair_loopback(first, second) : 1;
#endif
#endif
#endif
#endif
}

/* Write one byte into `peer`, so a read registration on the *other* endpoint of
 * the pair becomes ready. `send`/`recv` rather than `write`/`read` because the
 * Windows arm's endpoints are SOCKETs and not CRT file descriptors; both are
 * POSIX on the other two arms.
 *
 * The payload is the byte the source statement declared, or 'q' where it
 * declared none. It is not part of the trace: a `wake` projects to no reactor
 * operation, so its payload is a value rather than a reactor fact, and it
 * reaches an observation only where a source line renders it. */
QCORE_EVENT_UNUSED static int qcore_event_wake(qcore_event_fd peer, unsigned char payload)
{
    const char byte = (char) payload;
    return send(peer, &byte, 1, 0) == 1 ? 0 : 1;
}

/* Read one byte from `endpoint` into `*payload`. This is the whole body of a v1
 * callback. The caller compares the byte with the one the checked model says
 * this turn's wake delivered and exits non-zero on disagreement, so the named
 * premise EVENT-V1-STREAM-BYTE-ORDER is checked here rather than trusted. */
QCORE_EVENT_UNUSED static int qcore_event_drain(qcore_event_fd endpoint, unsigned char *payload)
{
    char byte = 0;
    if (recv(endpoint, &byte, 1, 0) != 1) {
        return 1;
    }
    *payload = (unsigned char) byte;
    return 0;
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
