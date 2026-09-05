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


/* Test-only minimal layouts/prototypes. The callback bodies below are copied
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
struct mk_event { int fd; uint32_t mask; void *data; };
struct mk_event_ctx { struct mk_event *fired; int fired_count; };
struct ev_map { int pipe[2]; struct event *event; struct mk_event_ctx *ctx; };
long send(int, const void *, unsigned long, int);
void perror(const char *);
int evutil_closesocket(int);
int event_del(struct event *);
void event_free(struct event *);
void mk_mem_free(void *);

static void cb_event(evutil_socket_t fd, short flags, void *data)
{
    int i;
    int mask = 0;
    struct mk_event *event = data;
    struct mk_event *fired;
    struct mk_event_ctx *ctx;
    struct ev_map *map = event->data;

    ctx = map->ctx;

    /* Compose mask */
    if (flags & EV_READ) {
        mask |= MK_EVENT_READ;
    }
    if (flags & EV_WRITE) {
        mask |= MK_EVENT_WRITE;
    }

    /* Register the event in the fired array */
    i = ctx->fired_count;
    fired = &ctx->fired[i];
    fired->fd   = event->fd;
    fired->mask = mask;
    fired->data = event;

    ctx->fired_count++;
}

static void cb_timeout(evutil_socket_t fd, short flags, void *data)
{
    int ret;
    uint64_t val = 1;
    struct ev_map *ev_map = data;

    ret = send(ev_map->pipe[1], (char *) &val, sizeof(uint64_t), 0);

    if (ret == -1) {
        if (evutil_socket_geterror(fd) != ERR(ECONNABORTED)) {
            perror("write");
        }
        evutil_closesocket(ev_map->pipe[1]);
        event_del(ev_map->event);
        event_free(ev_map->event);
        mk_mem_free(ev_map);
    }
}

static void cb_wait_2_timeout(evutil_socket_t fd, short flags, void *data)
{
    return;
}
