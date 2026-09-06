/* -*- Mode: C; tab-width: 4; indent-tabs-mode: nil; c-basic-offset: 4 -*- */

/*  Monkey HTTP Server
 *  ==================
 *  Copyright 2001-2017 Eduardo Silva <eduardo@monkey.io>
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */


/* Test-only minimal layouts/prototypes. The wrapper bodies below are copied
 * from the pinned adapter; the external audit checks the actual build/layouts. */
#include <stdint.h>
#include <stddef.h>
#include <errno.h>
typedef int evutil_socket_t;
#define EV_READ 2
#define EV_WRITE 4
#define MK_EVENT_READ 1
#define MK_EVENT_WRITE 4
#define ERR(e) (e)
#define evutil_socket_geterror(fd) (errno)
struct event;
struct mk_event { int fd; int type; uint32_t mask; void *data; };
struct event_base;
struct mk_event_ctx { struct mk_event *fired; int fired_count; struct event_base *base; unsigned long queue_size; };
struct ev_map { int pipe[2]; struct event *event; struct mk_event_ctx *ctx; };
long send(int, const void *, unsigned long, int);
void perror(const char *);
int evutil_closesocket(int);
int event_del(struct event *);
void event_free(struct event *);
void mk_mem_free(void *);


#define EV_PERSIST 16
#define MK_EVENT_UNMODIFIED -1
static void cb_event(evutil_socket_t fd, short flags, void *data);
void *mk_mem_alloc_z(unsigned long);
struct event_base *event_base_new(void);
void event_base_free(struct event_base *);
struct event *event_new(struct event_base *, int, short, void (*)(int, short, void *), void *);
int event_add(struct event *, const void *);

static inline int _mk_event_init()
{
#ifdef _WIN32
    int ret;
    WSADATA wsa_data;

    if (InterlockedCompareExchange(&mk_event_libevent_wsa_initialized,
                                   1, 0) != 0) {
        return 0;
    }

    ret = WSAStartup(MAKEWORD(2, 2), &wsa_data);
    if (ret != 0) {
        InterlockedExchange(&mk_event_libevent_wsa_initialized, 0);
        WSASetLastError(ret);
        return -1;
    }
#endif

    return 0;
}

static inline void *_mk_event_loop_create(int size)
{
    struct mk_event_ctx *ctx;

    if (_mk_event_init() != 0) {
        return NULL;
    }

    /* Main event context */
    ctx = mk_mem_alloc_z(sizeof(struct mk_event_ctx));
    if (!ctx) {
        return NULL;
    }

    /* Libevent context */
    ctx->base = event_base_new();

    /* Fired events (upon select(2) return) */
    ctx->fired = mk_mem_alloc_z(sizeof(struct mk_event) * size);
    if (!ctx->fired) {
        mk_mem_free(ctx);
        return NULL;
    }
    ctx->queue_size = size;

    return ctx;
}

static inline void _mk_event_loop_destroy(struct mk_event_ctx *ctx)
{
    event_base_free(ctx->base);
    mk_mem_free(ctx->fired);
    mk_mem_free(ctx);
}

static inline int _mk_event_update(struct mk_event_ctx *ctx, evutil_socket_t fd,
                                   int type, uint32_t events, void *data)
{
    int ret;
    int flags = 0;
    struct ev_map *ev_map;
    struct mk_event *event;
    struct event *libev;

    event = (struct mk_event *) data;
    ev_map = (struct ev_map *) event->data;

    /* Remove an existing timer first */
    event_del(ev_map->event);
    event_free(ev_map->event);

    /* Compose context */
    if (type != MK_EVENT_UNMODIFIED) {
        event->type = type;
    }
    if (events & MK_EVENT_READ) {
        flags |= EV_READ;
    }
    if (events & MK_EVENT_WRITE) {
        flags |= EV_WRITE;
    }
    flags |= EV_PERSIST;

    /* Register into libevent */
    libev = event_new(ctx->base, fd, flags, cb_event, event);
    if (libev == NULL) {
        return -1;
    }

    ret = event_add(libev, NULL);
    if (ret < 0) {
        return -1;
    }

    ev_map->event = libev;
    ev_map->ctx   = ctx;
    return 0;
}

/* Portable test declarations; production constants/layouts come from Clang. */
#define AF_UNIX 1
enum { SOCK_STREAM = 1 };
#define EV_TIMEOUT 1
#define EVLOOP_ONCE 1
#define EVLOOP_NONBLOCK 2
struct timeval { long tv_sec; long tv_usec; };
struct mk_event_loop { void *data; int n_events; };
int evutil_socketpair(int, int, int, int *);
int _mk_event_wait_with_flags(struct mk_event_loop *, int);
static void cb_wait_2_timeout(evutil_socket_t, short, void *);

static int mk_event_libevent_socketpair(evutil_socket_t fd[2])
{
#ifdef _WIN32
    int ret;
    SOCKET pair[2];

    ret = mk_win32_socketpair(pair);
    if (ret != 0) {
        return ret;
    }

    fd[0] = (evutil_socket_t) pair[0];
    fd[1] = (evutil_socket_t) pair[1];
    return 0;
#else
    return evutil_socketpair(AF_UNIX, SOCK_STREAM, 0, fd);
#endif
}

static inline int _mk_event_wait_2(struct mk_event_loop *loop, int timeout)
{
    struct mk_event_ctx *ctx = loop->data;
    struct timeval timev = {timeout / 1000, (timeout % 1000) * 1000}; /* (tv_sec, tv_usec} */
    int timedout_flag = 0;
    struct event *timeout_event;
    int ret;

    /* Infinite wait */
    if (timeout == -1) {
        return _mk_event_wait_with_flags(loop, EVLOOP_ONCE);
    }

    /* No wait: fast path, zero second timeout is nonblocking wait */
    if (timeout == 0) {
        return _mk_event_wait_with_flags(loop, EVLOOP_ONCE | EVLOOP_NONBLOCK);
    }

    /* Timed wait: slow path, blocking wait with timeout via timeout event */
    /* Add timeout */
    timeout_event = event_new(ctx->base, -1,
                      EV_TIMEOUT,
                      cb_wait_2_timeout, &timedout_flag);

    event_add(timeout_event, &timev);

    /* Blocking wait */
    ret = _mk_event_wait_with_flags(loop, EVLOOP_ONCE);

    /*
     * Remove timeout
     * "To deallocate an event, call event_free(). It is safe to call event_free()
     * on an event that is pending or active: doing so makes the event non-pending
     * and inactive before deallocating it."
     * ref: http://www.wangafu.net/~nickm/libevent-book/Ref4_event.html
     */
    event_free(timeout_event);

    return ret;
}
