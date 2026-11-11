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

/* qcore_socket (card QUIRIN-SOCKET-V1): the transport primitives an emitted
 * `source.socket.v1` program uses, and nothing more.
 *
 * Kept embedded, on the `c11_threads_compat.h` and `qcore_event_poll_compat.h`
 * precedent, so emitted C needs no QCore include path and no linked QCore
 * artifact. It is a *subset*, not a socket library: only TCP stream pairs this
 * same program creates on both ends, only single datagrams between two UDP
 * sockets of this same program, and no name resolution, no non-blocking mode and
 * no timers.
 *
 * Card SOCKET-INET-ADDRESSES adds the DECLARED-ADDRESS arms at the end of this
 * file — IPv4 and IPv6, with a port the source program wrote down — together
 * with exactly two socket options, IPV6_V6ONLY and SO_REUSEADDR, each set for a
 * stated reason and each decided per arm rather than inherited. Everything above
 * those arms is unchanged: a program that declares no address still binds IPv4
 * loopback on a kernel-chosen port through qcore_socket_bind_loopback and still
 * connects by getsockname through qcore_socket_connect.
 *
 * THREE ARMS, on the same shape the threads and poll shims use: `_WIN32`,
 * `__APPLE__`, and everything else (Linux and other POSIX). The two POSIX arms
 * select the *same* implementation below, by name, and they are separate arms
 * because what each has to declare to expose the BSD socket API under
 * `-std=c11 -pedantic` differs: glibc needs `_POSIX_C_SOURCE`, and Apple's
 * `sys/cdefs.h` keys its visibility level off `_DARWIN_C_SOURCE`. Writing the
 * shared body twice would be duplication, not portability; the arms are where
 * the platforms actually differ.
 *
 * THE DESCRIPTOR TYPE DIFFERS PER ARM, and the difference is contained:
 *
 *   - POSIX arms: `int`, from socket(2), closed with close(2).
 *   - Windows arm: `SOCKET` (an unsigned handle, not a CRT file descriptor),
 *     closed with closesocket(). Winsock also needs WSAStartup before any call
 *     and WSAPoll rather than poll(2), and `WSAPoll` takes SOCKETs.
 *
 * That boundary is decided honestly rather than papered over: the emitted
 * program never converts a SOCKET to an `int`, never prints one, and never
 * compares one against a POSIX fd. The **observation never carries either**:
 * every byte an emitted program writes is a line the source program asked for,
 * and a real descriptor number — which is not model-determined — reaches no
 * comparison. The same is true of the port: an UNDECLARED bind is to port 0 and
 * the kernel's choice is read back with getsockname only to hand it to this same
 * program's connect or sendto, which is why two concurrent runs of such a
 * program cannot collide. A DECLARED port is a check-time constant the source
 * wrote down; it reaches no observation either, and a collision with a live
 * holder of that address fails the run with the distinct bind status rather than
 * producing a different observation.
 *
 * What is claimed, and what is not:
 *
 *   - Every wait is BOUNDED. `qcore_socket_wait_readable` polls with a finite
 *     timeout, so a false premise produces a failed run in finite time rather
 *     than a hang. The checker has already proved that every receive is
 *     satisfied by bytes an earlier send of this same program committed, so a
 *     timeout here is evidence about the checker or the platform, never a
 *     normal outcome.
 *   - A receive DOES NOT TRUST the model. It compares the bytes it actually
 *     read against the bytes the checked program says arrive and returns a
 *     distinct non-zero result on disagreement, so the named premise
 *     SOCKET-V1-STREAM-BYTE-ORDER produces a *failed run* if it is false rather
 *     than a differently-shaped observation.
 *   - A stream receive reassembles short reads to the declared count. TCP
 *     delivers a byte sequence and the placement of segment boundaries is not
 *     model-determined; the count is, so the loop resolves the boundary by the
 *     declaration. A datagram receive does the opposite and takes exactly one
 *     datagram, requiring its length to be the declared one.
 *   - AN INTERRUPTED CALL IS NOT RESUMED. A -1 return fails the run with a
 *     distinct status rather than retrying, because the number of retries is
 *     not model-determined (SOCKET-V1-EINTR-RESUME).
 *   - A FAILED CLOSE IS NOT RETRIED. With no target pinned and errno
 *     unobserved, `qcore_core::os_endpoint::close_poststate` answers the
 *     poisoned DescriptorState::Attempted and permits no recovery, so a retry
 *     would act on a number whose fate nothing here determines.
 *   - No durability of any kind is claimed. Nothing here is an fsync.
 */
#ifndef QCORE_SOCKET_COMPAT_H
#define QCORE_SOCKET_COMPAT_H

/* --- Arm 1 of 3: Windows. ------------------------------------------------ */
#if defined(_WIN32)

#include <winsock2.h>
#include <ws2tcpip.h>
#include <stddef.h>
#include <string.h>
/* Card SOCKET-INET-ADDRESSES: the declared-address arms below use
 * `struct sockaddr_storage`, `struct sockaddr_in6` and `IPV6_V6ONLY`, all of
 * which this arm already has from <ws2tcpip.h>. Winsock reports its own error
 * codes through WSAGetLastError rather than errno, which is why the
 * family-availability probe has a separate arm rather than a shared body. */

/* Keep the emitted program self-contained on this arm too: without this the
 * caller would have to add `ws2_32.lib` to every build command, which the other
 * two arms need no analogue of. MSVC and clang-cl both honour it. */
#if defined(_MSC_VER)
#pragma comment(lib, "ws2_32.lib")
#endif

typedef SOCKET qcore_socket_fd;
typedef WSAPOLLFD qcore_socket_pollfd;
typedef int qcore_socket_len;
typedef int qcore_socket_ssize;
typedef int qcore_socket_addrlen;
#define QCORE_SOCKET_INVALID INVALID_SOCKET
#define QCORE_SOCKET_POLLIN POLLRDNORM

/* --- Arm 2 of 3: Apple. -------------------------------------------------- */
#elif defined(__APPLE__)

/* Must precede every libc include in this translation unit. Apple's
 * `sys/cdefs.h` selects its visibility level from these macros, and under
 * `-std=c11 -pedantic` (which defines __STRICT_ANSI__) the default would
 * otherwise hide socket(2), poll(2), send(2) and recv(2). `_DARWIN_C_SOURCE`
 * rather than `_POSIX_C_SOURCE` is the platform's own spelling for "expose the
 * full set", and it is what the Apple arm of the poll shim uses. */
#ifndef _DARWIN_C_SOURCE
#define _DARWIN_C_SOURCE 1
#endif

#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <poll.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

typedef int qcore_socket_fd;
typedef struct pollfd qcore_socket_pollfd;
typedef size_t qcore_socket_len;
typedef ssize_t qcore_socket_ssize;
typedef socklen_t qcore_socket_addrlen;
#define QCORE_SOCKET_INVALID (-1)
#define QCORE_SOCKET_POLLIN POLLIN
/* Selects the shared POSIX implementation below. Apple and Linux differ in what
 * they must declare, not in what the BSD socket calls then do. */
#define QCORE_SOCKET_POSIX 1

/* --- Arm 3 of 3: Linux and other POSIX. ---------------------------------- */
#else

/* Must precede every libc include in this translation unit: under
 * `-std=c11 -pedantic` glibc defines __STRICT_ANSI__ and would otherwise hide
 * socket(2), poll(2), send(2) and recv(2). */
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif

#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <poll.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

typedef int qcore_socket_fd;
typedef struct pollfd qcore_socket_pollfd;
typedef size_t qcore_socket_len;
typedef ssize_t qcore_socket_ssize;
typedef socklen_t qcore_socket_addrlen;
#define QCORE_SOCKET_INVALID (-1)
#define QCORE_SOCKET_POLLIN POLLIN
#define QCORE_SOCKET_POSIX 1

#endif

#if defined(__GNUC__) || defined(__clang__)
#define QCORE_SOCKET_UNUSED __attribute__((unused))
#else
#define QCORE_SOCKET_UNUSED
#endif

/* The model's own `MAX_PAYLOAD`. A receive never asks for more, because the
 * checker refuses a larger count; the array is sized from the same number so
 * that a defect in the checker overflows nothing. */
#define QCORE_SOCKET_MAX_PAYLOAD 1024

/* How long a receive waits before failing, in milliseconds. Bounded on purpose:
 * a false premise must produce a failed run in finite time rather than a hang.
 * The number is not a claim about latency — under the checked history the bytes
 * are already committed before the wait begins, so a correct run does not wait
 * at all. */
#define QCORE_SOCKET_TIMEOUT_MS 5000

/* Prepare the platform's socket layer. Zero on success. */
QCORE_SOCKET_UNUSED static int qcore_socket_startup(void)
{
#if defined(QCORE_SOCKET_POSIX)
    return 0;
#else
    WSADATA data;
    return WSAStartup(MAKEWORD(2, 2), &data) == 0 ? 0 : 1;
#endif
}

/* One socket. `datagram` selects UDP; otherwise TCP. Zero on success. */
QCORE_SOCKET_UNUSED static int qcore_socket_create(int datagram, qcore_socket_fd *out)
{
    qcore_socket_fd created = socket(AF_INET,
                                     datagram ? SOCK_DGRAM : SOCK_STREAM,
                                     datagram ? IPPROTO_UDP : IPPROTO_TCP);
    if (created == QCORE_SOCKET_INVALID) {
        return 1;
    }
    *out = created;
    return 0;
}

/* Bind to the loopback interface on a kernel-chosen port.
 *
 * Port 0 is not a convenience: a fixed port would make two concurrent runs of
 * the same emitted program collide, and the sanitized test suite runs its
 * binaries in parallel. The chosen port is read back only by connect and
 * sendto below, and never leaves this file. */
QCORE_SOCKET_UNUSED static int qcore_socket_bind_loopback(qcore_socket_fd endpoint)
{
    struct sockaddr_in address;
    memset(&address, 0, sizeof address);
    address.sin_family = AF_INET;
    address.sin_port = 0;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    return bind(endpoint, (struct sockaddr *) &address,
                (qcore_socket_addrlen) sizeof address) == 0
               ? 0
               : 1;
}

QCORE_SOCKET_UNUSED static int qcore_socket_listen(qcore_socket_fd endpoint)
{
    /* Depth one, matching the model's accept queue: the checker refuses a
     * second unaccepted connect, so a deeper backlog would describe a state no
     * admitted program reaches. */
    return listen(endpoint, 1) == 0 ? 0 : 1;
}

/* The address a bound socket of this program actually got. */
static int qcore_socket_address_of(qcore_socket_fd endpoint, struct sockaddr_in *address)
{
    qcore_socket_addrlen length = (qcore_socket_addrlen) sizeof *address;
    memset(address, 0, sizeof *address);
    if (getsockname(endpoint, (struct sockaddr *) address, &length) != 0) {
        return 1;
    }
    return length == (qcore_socket_addrlen) sizeof *address ? 0 : 1;
}

/* Connect to a listening socket OF THIS SAME PROGRAM.
 *
 * The target is a descriptor and not a spelled address, which is the whole of
 * v1's determinism story at the source level: there is no peer this program did
 * not create, so there is no completion it does not cause. */
QCORE_SOCKET_UNUSED static int qcore_socket_connect(qcore_socket_fd endpoint,
                                                    qcore_socket_fd listener)
{
    struct sockaddr_in address;
    if (qcore_socket_address_of(listener, &address) != 0) {
        return 1;
    }
    return connect(endpoint, (struct sockaddr *) &address,
                   (qcore_socket_addrlen) sizeof address) == 0
               ? 0
               : 1;
}

QCORE_SOCKET_UNUSED static int qcore_socket_accept(qcore_socket_fd listener,
                                                   qcore_socket_fd *out)
{
    qcore_socket_fd accepted = accept(listener, NULL, NULL);
    if (accepted == QCORE_SOCKET_INVALID) {
        return 1;
    }
    *out = accepted;
    return 0;
}

/* Wait until one endpoint is readable, or fail. Never waits forever. */
static int qcore_socket_wait_readable(qcore_socket_fd endpoint)
{
    qcore_socket_pollfd set[1];
    int result;
    memset(set, 0, sizeof set);
    set[0].fd = endpoint;
    set[0].events = QCORE_SOCKET_POLLIN;
    set[0].revents = 0;
#if defined(QCORE_SOCKET_POSIX)
    result = poll(set, (nfds_t) 1, QCORE_SOCKET_TIMEOUT_MS);
#else
    result = WSAPoll(set, (ULONG) 1, QCORE_SOCKET_TIMEOUT_MS);
#endif
    if (result != 1) {
        return 1;
    }
    return (set[0].revents & QCORE_SOCKET_POLLIN) != 0 ? 0 : 1;
}

/* The largest number of sockets one dispatch turn may watch at once.
 *
 * The checker refuses a program that would hold more registrations than this, so
 * the array below is sized for a bound no admitted program crosses and a defect
 * in the checker overflows nothing. It is the same discipline
 * QCORE_SOCKET_MAX_PAYLOAD applies to a payload. */
#define QCORE_SOCKET_MAX_WATCHED 16

/* One dispatch turn of a checked socket program (card SOCKET-EVENT-COMPOSE).
 *
 * `set` is the sockets the checked program has registered with its reactor, in
 * registration order, and `expected` is the index of the one this turn DECLARES
 * will be ready — or a negative number for a turn declared to report nothing.
 *
 * There is no run-time registry, and that is the honest shape rather than a
 * shortcut: the checker knows the watched set at every turn, so the reactor an
 * emitted program runs IS the declared registration set, materialised here.
 *
 * The declaration is CHECKED and never trusted, exactly as a receive's payload
 * is. Three results, so a failed run says which: 0 the reported set is the
 * declared one, 1 a transport failure or a bounded timeout, 2 the reported set
 * disagrees with the declaration. A kernel that reports a different set
 * therefore fails the run rather than producing a differently shaped
 * observation.
 *
 * An idle turn waits zero milliseconds. It is a claim that nothing is ready
 * NOW, and under the checked history nothing this program committed can make a
 * watched socket ready while the program itself is between statements, so
 * waiting longer would test nothing and cost the bounded timeout. */
QCORE_SOCKET_UNUSED static int qcore_socket_dispatch(const qcore_socket_fd *set,
                                                     size_t count, long expected)
{
    qcore_socket_pollfd watched[QCORE_SOCKET_MAX_WATCHED];
    size_t index;
    int result;
    int ready = 0;
    long ready_index = -1;
    if (count > (size_t) QCORE_SOCKET_MAX_WATCHED) {
        return 1;
    }
    if (count == 0) {
        /* WSAPoll refuses a zero-length set, and poll(2) with no descriptors is
         * only a sleep, so the answer is decided here: a turn over no
         * registration reports nothing. */
        return expected < 0 ? 0 : 2;
    }
    memset(watched, 0, sizeof watched);
    for (index = 0; index < count; ++index) {
        watched[index].fd = set[index];
        watched[index].events = QCORE_SOCKET_POLLIN;
        watched[index].revents = 0;
    }
#if defined(QCORE_SOCKET_POSIX)
    result = poll(watched, (nfds_t) count,
                  expected < 0 ? 0 : QCORE_SOCKET_TIMEOUT_MS);
#else
    result = WSAPoll(watched, (ULONG) count,
                     expected < 0 ? 0 : QCORE_SOCKET_TIMEOUT_MS);
#endif
    if (result < 0) {
        return 1;
    }
    for (index = 0; index < count; ++index) {
        if ((watched[index].revents & QCORE_SOCKET_POLLIN) != 0) {
            ready += 1;
            ready_index = (long) index;
        }
    }
    if (expected < 0) {
        return ready == 0 ? 0 : 2;
    }
    return (ready == 1 && ready_index == expected) ? 0 : 2;
}

/* Commit `length` declared bytes to the peer of a connected stream socket.
 *
 * The loop exists because send(2) may accept fewer bytes than offered. It is
 * bounded by the model, not by this file: the checker refuses a send that would
 * leave more than its in-flight bound outstanding on one stream, and that bound
 * is inside the smallest default socket buffer of all three targets, so the
 * first call is expected to take everything. Zero on success. */
QCORE_SOCKET_UNUSED static int qcore_socket_send(qcore_socket_fd endpoint,
                                                 const unsigned char *bytes, size_t length)
{
    size_t sent = 0;
    while (sent < length) {
        qcore_socket_ssize written = send(endpoint, (const char *) bytes + sent,
                                          (qcore_socket_len) (length - sent), 0);
        if (written <= 0) {
            return 1;
        }
        sent += (size_t) written;
    }
    return 0;
}

/* Take exactly `length` bytes off a connected stream socket and compare them
 * with the bytes the checked program says arrive.
 *
 * Returns 0 on an exact match, 1 on a transport failure or a bounded timeout,
 * and 2 when the bytes disagree with the model — three results, so a failed run
 * says which. */
QCORE_SOCKET_UNUSED static int qcore_socket_recv(qcore_socket_fd endpoint,
                                                 const unsigned char *expected, size_t length)
{
    unsigned char received[QCORE_SOCKET_MAX_PAYLOAD];
    size_t taken = 0;
    if (length == 0 || length > (size_t) QCORE_SOCKET_MAX_PAYLOAD) {
        return 1;
    }
    while (taken < length) {
        qcore_socket_ssize read_bytes;
        if (qcore_socket_wait_readable(endpoint) != 0) {
            return 1;
        }
        read_bytes = recv(endpoint, (char *) received + taken,
                          (qcore_socket_len) (length - taken), 0);
        if (read_bytes <= 0) {
            return 1;
        }
        taken += (size_t) read_bytes;
    }
    return memcmp(received, expected, length) == 0 ? 0 : 2;
}

/* One datagram to a bound UDP socket OF THIS SAME PROGRAM. */
QCORE_SOCKET_UNUSED static int qcore_socket_sendto(qcore_socket_fd endpoint,
                                                   qcore_socket_fd target,
                                                   const unsigned char *bytes, size_t length)
{
    struct sockaddr_in address;
    qcore_socket_ssize written;
    if (qcore_socket_address_of(target, &address) != 0) {
        return 1;
    }
    written = sendto(endpoint, (const char *) bytes, (qcore_socket_len) length, 0,
                     (struct sockaddr *) &address, (qcore_socket_addrlen) sizeof address);
    return written == (qcore_socket_ssize) length ? 0 : 1;
}

/* Take exactly one datagram and compare BOTH its source and its bytes with what
 * the checked program says arrives.
 *
 * A datagram is taken whole: a returned length other than the declared one is a
 * failure, never a truncation. Same three results as `qcore_socket_recv`.
 *
 * The source comparison is load-bearing and not a formality. A UDP socket here
 * is bound but never connected, so the kernel would deliver a datagram from any
 * sender to the same port; `sender` is the descriptor of the socket this
 * program's own model says sent it, and its bound address is read back with
 * getsockname and compared field for field. Without that, a stray datagram
 * delivered to the same short-lived ephemeral port would be read as this one. */
QCORE_SOCKET_UNUSED static int qcore_socket_recvfrom(qcore_socket_fd endpoint,
                                                     qcore_socket_fd sender,
                                                     const unsigned char *expected, size_t length)
{
    unsigned char received[QCORE_SOCKET_MAX_PAYLOAD];
    struct sockaddr_in source;
    struct sockaddr_in declared;
    qcore_socket_addrlen source_length = (qcore_socket_addrlen) sizeof source;
    qcore_socket_ssize taken;
    if (length == 0 || length > (size_t) QCORE_SOCKET_MAX_PAYLOAD) {
        return 1;
    }
    if (qcore_socket_address_of(sender, &declared) != 0) {
        return 1;
    }
    if (qcore_socket_wait_readable(endpoint) != 0) {
        return 1;
    }
    memset(&source, 0, sizeof source);
    taken = recvfrom(endpoint, (char *) received, (qcore_socket_len) length, 0,
                     (struct sockaddr *) &source, &source_length);
    if (taken != (qcore_socket_ssize) length
        || source_length != (qcore_socket_addrlen) sizeof source) {
        return 1;
    }
    if (source.sin_family != declared.sin_family
        || source.sin_port != declared.sin_port
        || source.sin_addr.s_addr != declared.sin_addr.s_addr) {
        return 2;
    }
    return memcmp(received, expected, length) == 0 ? 0 : 2;
}

/* Release one socket. A failure is NOT retried — see the header. */
QCORE_SOCKET_UNUSED static int qcore_socket_close(qcore_socket_fd endpoint)
{
#if defined(QCORE_SOCKET_POSIX)
    return close(endpoint) == 0 ? 0 : 1;
#else
    return closesocket(endpoint) == 0 ? 0 : 1;
#endif
}

/* ==========================================================================
 * DECLARED IPv4 AND IPv6 ADDRESSES (card SOCKET-INET-ADDRESSES).
 *
 * Everything above stays exactly as it was, and a program that declares no
 * address reaches none of what follows: the implicit `bind NAME;` form still
 * binds IPv4 loopback on a kernel-chosen port through qcore_socket_bind_loopback
 * and still connects by getsockname through qcore_socket_connect, so its emitted
 * C is byte-identical to what it was before this card.
 *
 * THREE DECISIONS ARE MADE HERE AND NOWHERE ELSE:
 *
 *  1. AN ADDRESS ARRIVES AS BYTES, NEVER AS TEXT. The checker parsed the source
 *     spelling once; there is no inet_pton call anywhere below, so there is no
 *     second parser that could read a spelling differently from the first. The
 *     octets are already in network order, so they are copied rather than
 *     converted; only the port passes through htons.
 *
 *  2. IPV6_V6ONLY IS SET EXPLICITLY, TO 1, ON EVERY ARM. It is NOT inherited:
 *     Linux defaults it to 0 (net.ipv6.bindv6only) and Windows defaults it to 1,
 *     so an inherited default would be a silent per-platform difference in what
 *     an emitted program binds. The model keeps the two address families
 *     disjoint — `::` and `0.0.0.0` are different addresses — and setting the
 *     option makes the C agree with the model rather than with the host. A
 *     setsockopt failure fails socket creation; it is never ignored.
 *
 *  3. IPv6 AVAILABILITY IS PROBED ON THE RUNNING SYSTEM, on the same shape the
 *     Windows AF_UNIX arm of qcore_event_poll_compat.h uses. Compiling is not
 *     evidence that the host has IPv6: qcore_socket_create_family asks the
 *     kernel for the socket, and ONLY an explicit unsupported-family/type/
 *     protocol result selects the distinct "IPv6 unavailable" return (2). Every
 *     other failure returns 1 and refuses. Nothing falls back to IPv4 — a
 *     fallback would bind a different address from the one the program declared,
 *     which is exactly what the declaration exists to prevent.
 * ========================================================================== */

/* Whether the last socket(2)/socket() failure was the platform saying it does
 * not have this address family, rather than any other failure.
 *
 * This is the one place in the shim that reads errno, and it is deliberate: the
 * close discipline above does NOT read errno because no arm of its behaviour
 * depends on which failure occurred, whereas here two arms do — a recorded
 * "no IPv6 on this host" and a refusal. */
static int qcore_socket_family_unsupported(void)
{
#if defined(QCORE_SOCKET_POSIX)
    return errno == EAFNOSUPPORT || errno == EPROTONOSUPPORT
#if defined(EPFNOSUPPORT)
           || errno == EPFNOSUPPORT
#endif
        ;
#else
    int code = WSAGetLastError();
    return code == WSAEAFNOSUPPORT || code == WSAEPROTONOSUPPORT
           || code == WSAEPFNOSUPPORT;
#endif
}

/* Build a sockaddr from model-determined octets. Zero on success. */
static int qcore_socket_address_from(int v6, const unsigned char *host, unsigned port,
                                     struct sockaddr_storage *out,
                                     qcore_socket_addrlen *length)
{
    memset(out, 0, sizeof *out);
    if (v6) {
        struct sockaddr_in6 address;
        memset(&address, 0, sizeof address);
        address.sin6_family = AF_INET6;
        address.sin6_port = htons((unsigned short) port);
        memcpy(&address.sin6_addr, host, 16);
        memcpy(out, &address, sizeof address);
        *length = (qcore_socket_addrlen) sizeof address;
    } else {
        struct sockaddr_in address;
        memset(&address, 0, sizeof address);
        address.sin_family = AF_INET;
        address.sin_port = htons((unsigned short) port);
        memcpy(&address.sin_addr, host, 4);
        memcpy(out, &address, sizeof address);
        *length = (qcore_socket_addrlen) sizeof address;
    }
    return 0;
}

/* The address a bound socket of this program actually got, in either family. */
static int qcore_socket_storage_of(qcore_socket_fd endpoint, struct sockaddr_storage *address,
                                   qcore_socket_addrlen *length)
{
    *length = (qcore_socket_addrlen) sizeof *address;
    memset(address, 0, sizeof *address);
    return getsockname(endpoint, (struct sockaddr *) address, length) == 0 ? 0 : 1;
}

/* Whether two addresses this program's own model relates are the same endpoint.
 *
 * Compared field for field per family rather than with a blanket memcmp: a
 * sockaddr carries padding no platform promises anything about, and a padding
 * byte is not part of an address. */
static int qcore_socket_same_address(const struct sockaddr_storage *left,
                                     const struct sockaddr_storage *right)
{
    if (left->ss_family != right->ss_family) {
        return 0;
    }
    if (left->ss_family == AF_INET6) {
        struct sockaddr_in6 a;
        struct sockaddr_in6 b;
        memcpy(&a, left, sizeof a);
        memcpy(&b, right, sizeof b);
        return a.sin6_port == b.sin6_port
               && memcmp(&a.sin6_addr, &b.sin6_addr, sizeof a.sin6_addr) == 0;
    }
    if (left->ss_family == AF_INET) {
        struct sockaddr_in a;
        struct sockaddr_in b;
        memcpy(&a, left, sizeof a);
        memcpy(&b, right, sizeof b);
        return a.sin_port == b.sin_port
               && memcmp(&a.sin_addr, &b.sin_addr, sizeof a.sin_addr) == 0;
    }
    return 0;
}

/* One socket in a DECLARED family.
 *
 * Returns 0 on success, 2 when the host explicitly does not support this address
 * family, and 1 on every other failure — three results, so a failed run says
 * which, and so that "this host has no IPv6" is never read as "IPv6 is broken"
 * and never silently becomes IPv4. */
QCORE_SOCKET_UNUSED static int qcore_socket_create_family(int v6, int datagram,
                                                          qcore_socket_fd *out)
{
    qcore_socket_fd created = socket(v6 ? AF_INET6 : AF_INET,
                                     datagram ? SOCK_DGRAM : SOCK_STREAM,
                                     datagram ? IPPROTO_UDP : IPPROTO_TCP);
    if (created == QCORE_SOCKET_INVALID) {
        return qcore_socket_family_unsupported() && v6 ? 2 : 1;
    }
    if (v6) {
        int only = 1;
        /* Decision 2 in the header above: explicit on every arm, never
         * inherited, and a failure to set it fails creation. */
        if (setsockopt(created, IPPROTO_IPV6, IPV6_V6ONLY, (const char *) &only,
                       (qcore_socket_addrlen) sizeof only)
            != 0) {
            (void) qcore_socket_close(created);
            return 1;
        }
    }
    *out = created;
    return 0;
}

/* Bind the address the program DECLARED.
 *
 * ONE DECIDED PROPERTY, TWO SPELLINGS — the same shape as _POSIX_C_SOURCE versus
 * _DARWIN_C_SOURCE above, and for the same reason. What is decided is:
 *
 *     A declared bind SUCCEEDS when no LIVE socket holds the address, and FAILS
 *     when one does.
 *
 * That is not the default on either family of platform, and the two need
 * opposite options to reach it:
 *
 *   - POSIX. Without SO_REUSEADDR a listener cannot rebind an address while a
 *     connection THIS PROGRAM ALREADY CLOSED is in TIME_WAIT, so running the same
 *     declared-address program twice inside a minute fails the second run. That
 *     is not a live holder and must not fail. SO_REUSEADDR permits exactly that
 *     rebind and still refuses a live duplicate (permitting a live duplicate is
 *     SO_REUSEPORT, which is NOT set).
 *   - Windows. Winsock's SO_REUSEADDR means something else: it permits a SECOND
 *     LIVE socket to bind the same address, which is address hijacking and the
 *     opposite of the decided property. It is therefore NOT set on that arm,
 *     where the default already refuses a live duplicate and already permits a
 *     TIME_WAIT rebind for a listening socket.
 *
 * So a collision with a live holder — inside or outside this program — is still
 * answered loudly, with the distinct bind status, on every arm. */
QCORE_SOCKET_UNUSED static int qcore_socket_bind_address(qcore_socket_fd endpoint, int v6,
                                                         const unsigned char *host,
                                                         unsigned port)
{
    struct sockaddr_storage address;
    qcore_socket_addrlen length = 0;
#if defined(QCORE_SOCKET_POSIX)
    int reuse = 1;
    if (setsockopt(endpoint, SOL_SOCKET, SO_REUSEADDR, (const char *) &reuse,
                   (qcore_socket_addrlen) sizeof reuse)
        != 0) {
        return 1;
    }
#endif
    if (qcore_socket_address_from(v6, host, port, &address, &length) != 0) {
        return 1;
    }
    return bind(endpoint, (struct sockaddr *) &address, length) == 0 ? 0 : 1;
}

/* Connect to the address the program DECLARED.
 *
 * The checker has already proved a socket of this same program is listening
 * there, so this is outbound transport to a spelled endpoint whose peer is still
 * decided at check time. */
QCORE_SOCKET_UNUSED static int qcore_socket_connect_address(qcore_socket_fd endpoint, int v6,
                                                            const unsigned char *host,
                                                            unsigned port)
{
    struct sockaddr_storage address;
    qcore_socket_addrlen length = 0;
    if (qcore_socket_address_from(v6, host, port, &address, &length) != 0) {
        return 1;
    }
    return connect(endpoint, (struct sockaddr *) &address, length) == 0 ? 0 : 1;
}

/* Connect to a listening socket of this program by its actual address, in either
 * family — what qcore_socket_connect does, for a listener that is not IPv4. */
QCORE_SOCKET_UNUSED static int qcore_socket_connect_at(qcore_socket_fd endpoint,
                                                       qcore_socket_fd listener)
{
    struct sockaddr_storage address;
    qcore_socket_addrlen length = 0;
    if (qcore_socket_storage_of(listener, &address, &length) != 0) {
        return 1;
    }
    return connect(endpoint, (struct sockaddr *) &address, length) == 0 ? 0 : 1;
}

/* One datagram to the address the program DECLARED. */
QCORE_SOCKET_UNUSED static int qcore_socket_sendto_address(qcore_socket_fd endpoint, int v6,
                                                           const unsigned char *host,
                                                           unsigned port,
                                                           const unsigned char *bytes,
                                                           size_t length)
{
    struct sockaddr_storage address;
    qcore_socket_addrlen address_length = 0;
    qcore_socket_ssize written;
    if (qcore_socket_address_from(v6, host, port, &address, &address_length) != 0) {
        return 1;
    }
    written = sendto(endpoint, (const char *) bytes, (qcore_socket_len) length, 0,
                     (struct sockaddr *) &address, address_length);
    return written == (qcore_socket_ssize) length ? 0 : 1;
}

/* One datagram to a bound socket of this program in either family — what
 * qcore_socket_sendto does, for a target that is not IPv4. */
QCORE_SOCKET_UNUSED static int qcore_socket_sendto_at(qcore_socket_fd endpoint,
                                                      qcore_socket_fd target,
                                                      const unsigned char *bytes, size_t length)
{
    struct sockaddr_storage address;
    qcore_socket_addrlen address_length = 0;
    qcore_socket_ssize written;
    if (qcore_socket_storage_of(target, &address, &address_length) != 0) {
        return 1;
    }
    written = sendto(endpoint, (const char *) bytes, (qcore_socket_len) length, 0,
                     (struct sockaddr *) &address, address_length);
    return written == (qcore_socket_ssize) length ? 0 : 1;
}

/* qcore_socket_recvfrom for a receiver that is not IPv4.
 *
 * Same three results and the same load-bearing source comparison: the datagram's
 * reported sender is compared with the bound address of the socket this
 * program's own model says sent it, so a stray datagram on the same port fails
 * the run instead of being read as this one. */
QCORE_SOCKET_UNUSED static int qcore_socket_recvfrom_at(qcore_socket_fd endpoint,
                                                        qcore_socket_fd sender,
                                                        const unsigned char *expected,
                                                        size_t length)
{
    unsigned char received[QCORE_SOCKET_MAX_PAYLOAD];
    struct sockaddr_storage source;
    struct sockaddr_storage declared;
    qcore_socket_addrlen declared_length = 0;
    qcore_socket_addrlen source_length = (qcore_socket_addrlen) sizeof source;
    qcore_socket_ssize taken;
    if (length == 0 || length > (size_t) QCORE_SOCKET_MAX_PAYLOAD) {
        return 1;
    }
    if (qcore_socket_storage_of(sender, &declared, &declared_length) != 0) {
        return 1;
    }
    if (qcore_socket_wait_readable(endpoint) != 0) {
        return 1;
    }
    memset(&source, 0, sizeof source);
    taken = recvfrom(endpoint, (char *) received, (qcore_socket_len) length, 0,
                     (struct sockaddr *) &source, &source_length);
    if (taken != (qcore_socket_ssize) length) {
        return 1;
    }
    if (!qcore_socket_same_address(&source, &declared)) {
        return 2;
    }
    return memcmp(received, expected, length) == 0 ? 0 : 2;
}

#endif /* QCORE_SOCKET_COMPAT_H */
