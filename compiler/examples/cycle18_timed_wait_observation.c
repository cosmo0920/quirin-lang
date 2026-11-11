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

/*
 * Cycle 18 bounded runtime observer.
 *
 * The real pinned Monkey mk_event.c is included by QCORE_MONKEY_EVENT_SOURCE.
 * Only its Libevent calls are interposed.  Collection is a fixed-size,
 * dispatcher-local buffer: the callback performs no allocation, locking or
 * stdio.  JSON is emitted only after mk_event_loop_destroy has quiesced and
 * freed the base.
 */

#include <event2/event.h>
#include <inttypes.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifndef QCORE_MONKEY_EVENT_SOURCE
#error "QCORE_MONKEY_EVENT_SOURCE must name the pinned Monkey mk_event.c"
#endif

enum observation_kind {
    OBS_BASE_CREATE,
    OBS_EVENT_NEW,
    OBS_EVENT_ADD,
    OBS_DISPATCH_BEGIN,
    OBS_ACTIVATE,
    OBS_CALLBACK_ENTER,
    OBS_CALLBACK_EXIT,
    OBS_DISPATCH_END,
    OBS_EVENT_FREE,
    OBS_ARGUMENT_END,
    OBS_BASE_FREE
};

struct observation {
    enum observation_kind kind;
    uintptr_t base;
    uintptr_t event;
    uintptr_t argument;
    int result;
    short flags;
    int priority;
};

struct observer_buffer {
    uint64_t run;
    uint64_t id;
    uint64_t dispatcher;
    size_t length;
    int overflow;
    const char *backend;
    struct observation records[11];
    struct event *event;
    void (*callback)(evutil_socket_t, short, void *);
    void *callback_argument;
    uintptr_t argument_identity;
    int callback_matches;
};

static _Thread_local struct observer_buffer *active_observer;

static void cb_wait_2_timeout(evutil_socket_t fd, short flags, void *data);

static void append_observation(struct observation record)
{
    struct observer_buffer *buffer = active_observer;

    if (buffer == NULL || buffer->length >= 11) {
        if (buffer != NULL) {
            buffer->overflow = 1;
        }
        return;
    }
    buffer->records[buffer->length++] = record;
}

static void observed_callback(evutil_socket_t fd, short flags, void *opaque)
{
    struct observer_buffer *buffer = opaque;
    struct observation record = {0};

    record.kind = OBS_ACTIVATE;
    record.event = (uintptr_t) buffer->event;
    record.flags = flags;
    append_observation(record);

    record.kind = OBS_CALLBACK_ENTER;
    record.argument = (uintptr_t) buffer->callback_argument;
    append_observation(record);

    buffer->callback(fd, flags, buffer->callback_argument);

    record.kind = OBS_CALLBACK_EXIT;
    append_observation(record);
}

static struct event_base *qcore_event_base_new(void)
{
    struct event_base *base = event_base_new();
    struct observation record = {0};

    if (base != NULL) {
        active_observer->backend = event_base_get_method(base);
    }
    record.kind = OBS_BASE_CREATE;
    record.base = (uintptr_t) base;
    record.result = base == NULL ? -1 : 0;
    append_observation(record);
    return base;
}

static void qcore_event_base_free(struct event_base *base)
{
    struct observation record = {0};

    record.base = (uintptr_t) base;
    event_base_free(base);
    record.kind = OBS_BASE_FREE;
    append_observation(record);
}

static struct event *qcore_event_new(struct event_base *base,
                                     evutil_socket_t fd,
                                     short flags,
                                     event_callback_fn callback,
                                     void *argument)
{
    struct observer_buffer *buffer = active_observer;
    struct observation record = {0};
    struct event *event;

    buffer->callback = callback;
    buffer->callback_argument = argument;
    buffer->argument_identity = (uintptr_t) argument;
    buffer->callback_matches = callback == cb_wait_2_timeout;
    event = event_new(base, fd, flags, observed_callback, buffer);
    buffer->event = event;

    record.kind = OBS_EVENT_NEW;
    record.base = (uintptr_t) base;
    record.event = (uintptr_t) event;
    record.argument = (uintptr_t) argument;
    record.flags = flags;
    record.priority = event == NULL ? -1 : event_get_priority(event);
    record.result = event == NULL ? -1 : 0;
    append_observation(record);
    return event;
}

static int qcore_event_add(struct event *event, const struct timeval *timeout)
{
    struct observation record = {0};
    int result = event_add(event, timeout);

    record.kind = OBS_EVENT_ADD;
    record.event = (uintptr_t) event;
    record.result = result;
    append_observation(record);
    return result;
}

static int qcore_event_base_loop(struct event_base *base, int flags)
{
    struct observation record = {0};
    int result;

    record.kind = OBS_DISPATCH_BEGIN;
    record.base = (uintptr_t) base;
    append_observation(record);
    result = event_base_loop(base, flags);
    record.kind = OBS_DISPATCH_END;
    record.result = result;
    append_observation(record);
    return result;
}

static void qcore_event_free(struct event *event)
{
    struct observation record = {0};

    record.event = (uintptr_t) event;
    event_free(event);
    record.kind = OBS_EVENT_FREE;
    append_observation(record);
}

#define event_base_new qcore_event_base_new
#define event_base_free qcore_event_base_free
#define event_new qcore_event_new
#define event_add qcore_event_add
#define event_base_loop qcore_event_base_loop
#define event_free qcore_event_free
#define mk_event_wait_2 qcore_source_mk_event_wait_2
#include QCORE_MONKEY_EVENT_SOURCE
#undef mk_event_wait_2
#undef event_free
#undef event_base_loop
#undef event_add
#undef event_new
#undef event_base_free
#undef event_base_new

int mk_event_wait_2(struct mk_event_loop *loop, int timeout)
{
    struct observation record = {0};
    int result = qcore_source_mk_event_wait_2(loop, timeout);

    record.kind = OBS_ARGUMENT_END;
    record.argument = active_observer->argument_identity;
    append_observation(record);
    return result;
}

static const char *kind_name(enum observation_kind kind)
{
    static const char *const names[] = {
        "base_create", "event_new", "event_add", "dispatch_begin",
        "activate", "callback_enter", "callback_exit", "dispatch_end",
        "event_free", "argument_end", "base_free"
    };
    return names[kind];
}

static void print_identity(const char *name, uintptr_t identity)
{
    printf("\"%s\":{\"id\":%" PRIuPTR ",\"generation\":1}", name,
           identity);
}

static void print_record(const struct observer_buffer *buffer, size_t index)
{
    const struct observation *record = &buffer->records[index];

    printf("{\"schema_version\":1,\"run\":%" PRIu64
           ",\"buffer\":%" PRIu64 ",\"local_sequence\":%zu"
           ",\"dispatcher\":%" PRIu64 ",\"action\":{\"kind\":\"%s\"",
           buffer->run, buffer->id, index + 1, buffer->dispatcher,
           kind_name(record->kind));
    switch (record->kind) {
    case OBS_BASE_CREATE:
        printf(","); print_identity("base", record->base);
        printf(",\"backend\":\"%s\"", buffer->backend);
        break;
    case OBS_EVENT_NEW:
        printf(","); print_identity("base", record->base);
        printf(","); print_identity("event", record->event);
        printf(","); print_identity("argument", record->argument);
        printf(",\"callback\":\"cb_wait_2_timeout\",\"flags\":%d"
               ",\"priority\":%d,\"result\":%d",
               record->flags, record->priority, record->result);
        break;
    case OBS_EVENT_ADD:
        printf(","); print_identity("event", record->event);
        printf(",\"result\":%d", record->result);
        break;
    case OBS_DISPATCH_BEGIN:
        printf(","); print_identity("base", record->base);
        break;
    case OBS_ACTIVATE:
        printf(","); print_identity("event", record->event);
        printf(",\"reason\":\"timeout\"");
        break;
    case OBS_CALLBACK_ENTER:
        printf(","); print_identity("event", record->event);
        printf(","); print_identity("argument", record->argument);
        printf(",\"callback\":\"cb_wait_2_timeout\",\"reason\":\"timeout\"");
        break;
    case OBS_CALLBACK_EXIT:
        printf(","); print_identity("event", record->event);
        printf(","); print_identity("argument", record->argument);
        printf(",\"callback\":\"cb_wait_2_timeout\"");
        break;
    case OBS_DISPATCH_END:
        printf(","); print_identity("base", record->base);
        printf(",\"result\":%d", record->result);
        break;
    case OBS_EVENT_FREE:
        printf(","); print_identity("event", record->event);
        break;
    case OBS_ARGUMENT_END:
        printf(","); print_identity("argument", record->argument);
        break;
    case OBS_BASE_FREE:
        printf(","); print_identity("base", record->base);
        break;
    }
    puts("}}");
}

int main(void)
{
    struct observer_buffer buffer = {0};
    struct mk_event_loop *loop;
    int result;
    size_t index;

    buffer.run = (uint64_t) getpid();
    buffer.id = (uint64_t) (uintptr_t) &buffer;
    buffer.dispatcher = (uint64_t) (uintptr_t) pthread_self();
    active_observer = &buffer;

    loop = mk_event_loop_create(8);
    if (loop == NULL) {
        return 2;
    }
    result = mk_event_wait_2(loop, 2);
    mk_event_loop_destroy(loop);
    active_observer = NULL;

    if (result != 0 || buffer.overflow || buffer.length != 11 ||
        !buffer.callback_matches || buffer.records[1].result != 0 ||
        buffer.records[2].result != 0 ||
        buffer.records[1].flags != EV_TIMEOUT ||
        buffer.records[4].flags != EV_TIMEOUT ||
        buffer.records[5].flags != EV_TIMEOUT) {
        return 3;
    }
    for (index = 0; index < buffer.length; index++) {
        print_record(&buffer, index);
    }
    return 0;
}
