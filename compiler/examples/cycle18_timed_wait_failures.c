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

/* Bounded fault injection around the exact copied pinned Monkey wrapper bodies. */
#include <stdio.h>
#include <string.h>

#include "cycle18_timed_wait_failures.h"

#include "../tests/fixtures/monkey/wrappers.c"

struct event { int marker; };
struct event_base { int marker; };

enum scenario { CONTEXT_ALLOC, BASE_CREATE, FIRED_ALLOC, TIMER_CREATE, TIMER_ADD, SUCCESS };
static enum scenario active;
static int alloc_call, base_created, base_freed, context_freed;
static int add_nonnull, free_nonnull, add_result, activated;
static struct mk_event_ctx context_storage;
static struct mk_event fired_storage[2];
static struct event_base base_storage;
static struct event event_storage;

void *mk_mem_alloc_z(unsigned long size)
{
    (void) size;
    alloc_call++;
    if (active == CONTEXT_ALLOC && alloc_call == 1) return NULL;
    if (active == FIRED_ALLOC && alloc_call == 2) return NULL;
    if (alloc_call == 1) { memset(&context_storage, 0, sizeof(context_storage)); return &context_storage; }
    memset(fired_storage, 0, sizeof(fired_storage)); return fired_storage;
}

void mk_mem_free(void *pointer)
{
    if (pointer == &context_storage) context_freed = 1;
}

struct event_base *event_base_new(void)
{
    if (active == BASE_CREATE) return NULL;
    base_created = 1; return &base_storage;
}

void event_base_free(struct event_base *base)
{
    if (base == &base_storage) base_freed = 1;
}

struct event *event_new(struct event_base *base, int fd, short flags,
                        void (*callback)(int, short, void *), void *argument)
{
    (void) base; (void) fd; (void) flags; (void) callback; (void) argument;
    return active == TIMER_CREATE ? NULL : &event_storage;
}

int event_add(struct event *event, const void *timeout)
{
    (void) timeout; add_nonnull = event != NULL;
    add_result = active == TIMER_ADD ? -1 : (event == NULL ? -1 : 0);
    return add_result;
}

void event_free(struct event *event) { free_nonnull = event != NULL; }

int _mk_event_wait_with_flags(struct mk_event_loop *loop, int flags)
{
    (void) loop; (void) flags;
    if (active == SUCCESS) activated = 1;
    return 7;
}

static void cb_wait_2_timeout(evutil_socket_t fd, short flags, void *data)
{
    (void) fd; (void) flags; *(int *) data = 1;
}

static void cb_event(evutil_socket_t fd, short flags, void *data)
{ (void) fd; (void) flags; (void) data; }

static void cb_timeout(evutil_socket_t fd, short flags, void *data)
{ (void) fd; (void) flags; (void) data; }

static void reset(enum scenario selected)
{
    active = selected; alloc_call = 0; base_created = 0; base_freed = 0;
    context_freed = 0; add_nonnull = 0; free_nonnull = 0;
    add_result = 0; activated = 0;
}

static void print_create(const char *name, enum scenario selected,
                         const char *trace)
{
    struct mk_event_ctx *ctx;
    reset(selected); ctx = _mk_event_loop_create(2);
    printf("{\"schema_version\":%d,\"source_revision\":\"%s\","
           "\"scenario\":\"%s\",\"path\":\"_mk_event_loop_create\","
           "\"trace\":[%s],\"returned_null\":%s,\"returned_context_base_nonnull\":%s,"
           "\"base_created\":%s,\"base_freed\":%s,\"context_freed\":%s,"
           "\"returned_dispatch_result\":false,\"event_add_argument_nonnull\":false,"
           "\"event_free_argument_nonnull\":false,\"event_add_result\":0,\"timeout_activated\":false}",
           QCORE_WAIT_FAILURE_SCHEMA_VERSION, QCORE_WAIT_FAILURE_SOURCE_REVISION,
           name, trace, ctx == NULL ? "true" : "false",
           ctx != NULL && ctx->base != NULL ? "true" : "false",
           base_created ? "true" : "false", base_freed ? "true" : "false",
           context_freed ? "true" : "false");
}

static void print_wait(const char *name, enum scenario selected)
{
    struct mk_event_ctx ctx = {0}; struct mk_event_loop loop = {&ctx, 0}; int result;
    reset(selected); result = _mk_event_wait_2(&loop, 2);
    printf("{\"schema_version\":%d,\"source_revision\":\"%s\","
           "\"scenario\":\"%s\",\"path\":\"_mk_event_wait_2\","
           "\"trace\":[\"event_new\",\"event_add\",\"dispatch\",\"event_free\"],"
           "\"returned_dispatch_result\":%s,\"event_add_argument_nonnull\":%s,"
           "\"event_free_argument_nonnull\":%s,\"event_add_result\":%d,"
           "\"timeout_activated\":%s,\"returned_null\":false,"
           "\"returned_context_base_nonnull\":false,\"base_created\":false,"
           "\"base_freed\":false,\"context_freed\":false}",
           QCORE_WAIT_FAILURE_SCHEMA_VERSION, QCORE_WAIT_FAILURE_SOURCE_REVISION,
           name, result == 7 ? "true" : "false",
           add_nonnull ? "true" : "false", free_nonnull ? "true" : "false", add_result,
           activated ? "true" : "false");
}

int main(void)
{
    puts("[");
    print_create("context_allocation_failure", CONTEXT_ALLOC, "\"event_init\",\"context_alloc\""); puts(",");
    print_create("base_creation_failure", BASE_CREATE, "\"event_init\",\"context_alloc\",\"event_base_new\",\"fired_alloc\""); puts(",");
    print_create("fired_allocation_failure", FIRED_ALLOC, "\"event_init\",\"context_alloc\",\"event_base_new\",\"fired_alloc\",\"context_free\""); puts(",");
    print_wait("timer_creation_failure", TIMER_CREATE); puts(",");
    print_wait("timer_registration_failure", TIMER_ADD); puts(",");
    print_wait("success_observation", SUCCESS); puts("\n]");
    return 0;
}
