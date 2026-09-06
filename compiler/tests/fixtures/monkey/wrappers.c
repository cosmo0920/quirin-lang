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
