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

/* Observation adapter around the actual pinned Monkey translation unit.
 * QCORE_MONKEY_SOURCE is supplied by the audited compilation-database runner.
 * Hooks delegate to the real Libevent calls; callbacks invoke the original
 * Monkey function pointers. No synthetic callback trace is generated.
 */
#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <event2/event.h>
#include <event2/util.h>
#include <mk_core/mk_event.h>
#include <mk_core/mk_memory.h>
#include <stdint.h>

static void cb_event(evutil_socket_t, short, void *);
static void cb_timeout(evutil_socket_t, short, void *);
static void cb_wait_2_timeout(evutil_socket_t, short, void *);

struct observed_callback {
    struct event *event;
    struct event_base *base;
    void (*callback)(evutil_socket_t, short, void *);
    void *argument;
    unsigned int identity;
    evutil_socket_t descriptor;
    unsigned int generation;
    int freed;
};

/* Separate schema: references the exact records in the identity trace. */
static unsigned int sequence;
static FILE *bindings;
static int binding_active;
static unsigned int binding_calls;
static int binding_injected;
static void *released_map;
static void binding_begin(struct observed_callback *, evutil_socket_t, short);
static void binding_end(struct observed_callback *);
static void binding_call(const char *name, const char *arguments, long result,
                         long memory_value)
{
    if (!binding_active) { return; }
    fprintf(bindings, "%s{\"function\":\"%s\",\"arguments\":%s,\"result\":%ld,"
            "\"sequence\":%u,\"memory_value\":%ld,\"origin\":\"%s\"}",
            binding_calls++ ? "," : "", name, arguments, result, sequence, memory_value,
            binding_injected ? "injected_failure" : "native");
}

static struct observed_callback callbacks[32];
static unsigned int callback_count;
static unsigned int base_id;
static struct event_base *dispatch_base;
static pthread_t dispatcher;
static int dispatching;
static unsigned int seen_event;
static unsigned int seen_timeout;
static unsigned int seen_wait_timeout;

/* Generations persist across bases and descriptor-number reuse. */
struct observed_descriptor {
    evutil_socket_t fd;
    unsigned int generation;
    int live;
};
static struct observed_descriptor descriptors[32];
static unsigned int descriptor_count;
static int fail_socketpair;
static int fail_send_errno;

static struct observed_descriptor *find_descriptor(evutil_socket_t fd)
{
    unsigned int i;
    for (i = 0; i < descriptor_count; i++) {
        if (descriptors[i].fd == fd) { return &descriptors[i]; }
    }
    assert(descriptor_count < sizeof(descriptors) / sizeof(descriptors[0]));
    descriptors[descriptor_count].fd = fd;
    return &descriptors[descriptor_count++];
}

static unsigned int live_generation(evutil_socket_t fd)
{
    struct observed_descriptor *entry;
    if (fd == -1) { return 0; }
    entry = find_descriptor(fd);
    assert(entry->live);
    return entry->generation;
}

static void record_descriptor(const char *kind, evutil_socket_t fd,
                              unsigned int generation, long result)
{
    printf("{\"schema_version\":1,\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,"
           "\"action\":{\"kind\":\"%s\",\"descriptor\":{\"fd\":%d,\"generation\":%u},\"result\":%ld}}\n",
           ++sequence, base_id, (unsigned long) pthread_self(), kind, fd, generation, result);
}

static int observed_socketpair(int domain, int type, int protocol, evutil_socket_t pair[2])
{
    int result;
    unsigned int i;
    if (fail_socketpair) {
        fail_socketpair = 0;
        errno = EMFILE;
        result = -1;
    }
    else { result = evutil_socketpair(domain, type, protocol, pair); }
    if (result == 0) {
        assert(pair[0] != pair[1]);
        for (i = 0; i < 2; i++) {
            struct observed_descriptor *entry = find_descriptor(pair[i]);
            assert(!entry->live);
            entry->generation++;
            assert(entry->generation != 0);
            entry->live = 1;
        }
    }
    printf("{\"schema_version\":1,\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,"
           "\"action\":{\"kind\":\"socketpair\",\"result\":%d,\"descriptors\":[",
           ++sequence, base_id, (unsigned long) pthread_self(), result);
    if (result == 0) {
        printf("{\"fd\":%d,\"generation\":%u},{\"fd\":%d,\"generation\":%u}",
               pair[0], live_generation(pair[0]), pair[1], live_generation(pair[1]));
    }
    printf("]}}\n");
    return result;
}

static ssize_t observed_send(int fd, const void *buffer, size_t length, int flags)
{
    unsigned int generation = live_generation(fd);
    ssize_t result;
    int saved_errno;
    assert(!binding_injected);
    if (fail_send_errno) {
        assert(binding_active);
        errno = fail_send_errno;
        fail_send_errno = 0;
        binding_injected = 1;
        result = -1;
    }
    else { result = send(fd, buffer, length, flags); }
    saved_errno = errno;
    record_descriptor("descriptor_use", fd, generation, result);
    if (binding_active) {
        char args[256];
        uint64_t value;
        assert(length == sizeof(value));
        memcpy(&value, buffer, sizeof(value));
        snprintf(args, sizeof(args), "[%d,{\"value\":%lu,\"size\":%lu},%lu,%d]",
                 fd, (unsigned long) value, (unsigned long) length, (unsigned long) length, flags);
        binding_call("send", args, result, 0);
    }
    binding_injected = 0;
    errno = saved_errno;
    return result;
}

static ssize_t observed_recv(int fd, void *buffer, size_t length, int flags)
{
    unsigned int generation = live_generation(fd);
    ssize_t result = recv(fd, buffer, length, flags);
    int saved_errno = errno;
    record_descriptor("descriptor_use", fd, generation, result);
    errno = saved_errno;
    return result;
}

static const char *callback_name(void (*callback)(evutil_socket_t, short, void *))
{
    if (callback == cb_event) { return "cb_event"; }
    if (callback == cb_timeout) { return "cb_timeout"; }
    if (callback == cb_wait_2_timeout) { return "cb_wait_2_timeout"; }
    abort();
}

static void record(const char *kind, unsigned int event, const char *name, int result)
{
    printf("{\"schema_version\":1,\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,\"action\":{",
           ++sequence, base_id, (unsigned long) pthread_self());
    if (event != 0) {
        struct observed_callback *entry = &callbacks[event - 1];
        printf("\"kind\":\"event\",\"operation\":\"%s\",\"event\":%u,\"callback\":\"%s\",\"result\":%d,\"descriptor\":",
               kind, event, name, result);
        if (entry->descriptor == -1) { printf("null"); }
        else { printf("{\"fd\":%d,\"generation\":%u}", entry->descriptor, entry->generation); }
    }
    else { printf("\"kind\":\"%s\",\"result\":%d", kind, result); }
    printf("}}\n");
}

static void callback_hook(evutil_socket_t fd, short flags, void *argument)
{
    struct observed_callback *entry = argument;
    assert(dispatching && pthread_equal(dispatcher, pthread_self()));
    assert(entry->base == dispatch_base && !entry->freed);
    assert(fd == entry->descriptor);
    if (fd != -1) { assert(live_generation(fd) == entry->generation); }
    record("callback_enter", entry->identity, callback_name(entry->callback), 0);
    if (entry->callback == cb_event) { seen_event++; }
    if (entry->callback == cb_timeout) { seen_timeout++; }
    if (entry->callback == cb_wait_2_timeout) { seen_wait_timeout++; }
    binding_begin(entry, fd, flags);
    entry->callback(fd, flags, entry->argument);
    record("callback_exit", entry->identity, callback_name(entry->callback), 0);
    binding_end(entry);
}

static struct event_base *observed_base_new(void)
{
    struct event_base *base = event_base_new();
    assert(base != NULL);
    base_id++;
    printf("{\"schema_version\":1,\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,\"action\":{\"kind\":\"backend\",\"backend\":\"%s\"}}\n",
           ++sequence, base_id, (unsigned long) pthread_self(), event_base_get_method(base));
    return base;
}

static struct event *observed_event_new(struct event_base *base, evutil_socket_t fd,
                                      short flags, event_callback_fn callback, void *argument)
{
    struct observed_callback *entry;
    assert(callback_count < sizeof(callbacks) / sizeof(callbacks[0]));
    entry = &callbacks[callback_count++];
    entry->identity = callback_count;
    entry->base = base;
    entry->descriptor = fd;
    entry->generation = live_generation(fd);
    entry->callback = callback;
    entry->argument = argument;
    entry->event = event_new(base, fd, flags, callback_hook, entry);
    assert(entry->event != NULL);
    record("event_new", entry->identity, callback_name(callback), 0);
    fprintf(bindings, "{\"kind\":\"registration\",\"schema_version\":1,\"sequence\":%u,"
            "\"event\":%u,\"argument\":%lu,\"event_pointer\":%lu}\n",
            sequence, entry->identity, (unsigned long) argument, (unsigned long) entry->event);
    return entry->event;
}

static struct observed_callback *find_event(struct event *event)
{
    unsigned int index;
    for (index = 0; index < callback_count; index++) {
        if (callbacks[index].event == event && !callbacks[index].freed) {
            return &callbacks[index];
        }
    }
    abort();
}

static int observed_event_add(struct event *event, const struct timeval *timeout)
{
    struct observed_callback *entry = find_event(event);
    int result = event_add(event, timeout);
    record("event_add", entry->identity, callback_name(entry->callback), result);
    return result;
}

static int observed_event_del(struct event *event)
{
    struct observed_callback *entry = find_event(event);
    int result = event_del(event);
    record("event_del", entry->identity, callback_name(entry->callback), result);
    char args[64];
    snprintf(args, sizeof(args), "[%lu]", (unsigned long) event);
    binding_call("event_del", args, result, 0);
    return result;
}

static void observed_event_free(struct event *event)
{
    struct observed_callback *entry = find_event(event);
    char args[64];
    snprintf(args, sizeof(args), "[%lu]", (unsigned long) event);
    entry->freed = 1;
    event_free(event);
    record("event_free", entry->identity, callback_name(entry->callback), 0);
    binding_call("event_free", args, 0, 0);
}

static int observed_loop(struct event_base *base, int flags)
{
    int result;
    assert(!dispatching);
    dispatcher = pthread_self();
    dispatch_base = base;
    dispatching = 1;
    record("dispatch_begin", 0, "", flags);
    result = event_base_loop(base, flags);
    record("dispatch_end", 0, "", result);
    dispatching = 0;
    dispatch_base = NULL;
    return result;
}

static int observed_close(evutil_socket_t fd)
{
    struct observed_descriptor *entry = find_descriptor(fd);
    unsigned int generation = live_generation(fd);
    int result = evutil_closesocket(fd);
    int saved_errno = errno;
    if (result == 0) { entry->live = 0; }
    record_descriptor("descriptor_close", fd, generation, result);
    char args[64];
    snprintf(args, sizeof(args), "[%d]", fd);
    binding_call("evutil_closesocket", args, result, 0);
    errno = saved_errno;
    return result;
}

static void observed_base_free(struct event_base *base)
{
    assert(!dispatching);
    event_base_free(base);
    record("base_free", 0, "", 0);
}

static int *observed_errno_location(void)
{
    int *location = __errno_location();
    int value = *location;
    binding_call("__errno_location", "[]", (long) location, value);
    *location = value;
    return location;
}

static void observed_perror(const char *message)
{
    int saved_errno = errno;
    assert(!binding_active || strcmp(message, "write") == 0);
    binding_call("perror", "[\"write\"]", 0, 0);
    errno = saved_errno;
    perror(message);
}

static void observed_mem_free(void *pointer)
{
    char args[64];
    snprintf(args, sizeof(args), "[%lu]", (unsigned long) pointer);
    if (binding_active) { assert(released_map == NULL); released_map = pointer; }
    binding_call("mk_mem_free", args, 0, 0);
    mk_mem_free(pointer);
}

#ifdef QCORE_CB_EVENT_ONLY
struct payload_allocation { void *pointer; size_t bytes; };
static struct payload_allocation payload_allocations[16];
static unsigned int payload_allocation_count;
static void *observed_mem_alloc_z(size_t bytes)
{
    void *pointer = mk_mem_alloc_z(bytes);
    assert(payload_allocation_count < 16);
    payload_allocations[payload_allocation_count++] =
        (struct payload_allocation) {pointer, bytes};
    return pointer;
}
static size_t allocated_bytes(void *pointer)
{
    for (unsigned int i = 0; i < payload_allocation_count; i++) {
        if (payload_allocations[i].pointer == pointer) return payload_allocations[i].bytes;
    }
    abort();
}
#define mk_mem_alloc_z observed_mem_alloc_z
#endif

#define __errno_location observed_errno_location
#define perror observed_perror
#define mk_mem_free observed_mem_free
#define event_base_new observed_base_new
#define event_new observed_event_new
#define event_add observed_event_add
#define event_del observed_event_del
#define event_free observed_event_free
#define event_base_loop observed_loop
#define event_base_free observed_base_free
#define evutil_closesocket observed_close
#define evutil_socketpair observed_socketpair
#define send observed_send
#include QCORE_MONKEY_SOURCE
#undef event_base_new
#undef event_new
#undef event_add
#undef event_del
#undef event_free
#undef event_base_loop
#undef event_base_free
#undef evutil_closesocket
#undef evutil_socketpair
#undef send
#undef __errno_location
#undef perror
#undef mk_mem_free
#ifdef QCORE_CB_EVENT_ONLY
#undef mk_mem_alloc_z
#endif

/* Read actual typed objects before/after the callback; never dereference a freed map.
 * Ordinary reads/writes between these snapshots are reconstructed from source
 * under the checked exclusive-dispatch scope, not individually instrumented. */
static void cell(void *object, const char *field, const char *type, long value, int *count)
{
    fprintf(bindings, "%s{\"object\":%lu,\"field\":\"%s\",\"c_type\":\"%s\",\"value\":%ld}",
            (*count)++ ? "," : "", (unsigned long) object, field, type, value);
}

static void snapshot(struct observed_callback *entry)
{
    int count = 0;
    fprintf(bindings, "[");
    if (entry->callback == cb_event) {
        struct mk_event *payload = entry->argument;
        struct ev_map *map = payload->data;
        struct mk_event_ctx *ctx = map->ctx;
        assert(ctx->base == entry->base);
        assert(ctx->queue_size > 0 && ctx->queue_size <= 64);
        cell(payload, "data", "void *", (long) map, &count);
        cell(payload, "fd", "int", payload->fd, &count);
        cell(map, "ctx", "struct mk_event_ctx *", (long) ctx, &count);
        cell(ctx, "fired", "struct mk_event *", (long) ctx->fired, &count);
        cell(ctx, "fired_count", "int", ctx->fired_count, &count);
        cell(ctx, "queue_size", "unsigned long", (long) ctx->queue_size, &count);
        for (int i = 0; i < ctx->queue_size; i++) {
            cell(&ctx->fired[i], "fd", "int", ctx->fired[i].fd, &count);
            cell(&ctx->fired[i], "mask", "unsigned int", ctx->fired[i].mask, &count);
            cell(&ctx->fired[i], "data", "void *", (long) ctx->fired[i].data, &count);
        }
    }
    else if (entry->callback == cb_timeout && !released_map) {
        struct ev_map *map = entry->argument;
        assert(map->ctx->base == entry->base && map->event == entry->event);
        cell(map, "pipe[1]", "int", map->pipe[1], &count);
        cell(map, "event", "struct event *", (long) map->event, &count);
    }
    fprintf(bindings, "]");
}

static void binding_begin(struct observed_callback *entry, evutil_socket_t fd, short flags)
{
    int saved_errno = errno;
    assert(!binding_active);
    released_map = NULL;
    binding_calls = 0;
    fprintf(bindings, "{\"kind\":\"callback\",\"schema_version\":1,\"enter\":%u,"
            "\"event\":%u,\"fd\":%d,\"flags\":%d,\"argument\":%lu,"
            "\"event_size\":%lu,\"before\":",
            sequence, entry->identity, fd, flags, (unsigned long) entry->argument,
            (unsigned long) sizeof(struct mk_event));
    snapshot(entry);
    fprintf(bindings, ",\"calls\":[");
    binding_active = 1;
    errno = saved_errno;
}

static void binding_end(struct observed_callback *entry)
{
    int saved_errno = errno;
    binding_active = 0;
    fprintf(bindings, "],\"exit\":%u,\"after\":", sequence);
    snapshot(entry);
    fprintf(bindings, "}\n");
    errno = saved_errno;
}

#ifdef QCORE_CB_EVENT_ONLY
/* This mode emits the complete dedicated cb_event run, never a filtered
 * fragment of the broader callback observation below. */
int main(void)
{
    struct mk_event channel = {0};
    struct mk_event_loop *loop;
    struct mk_event_ctx *ctx;
    int reader, writer;
    const int requested_capacity = 8;
    const short flags[] = {EV_READ, EV_WRITE, EV_READ | EV_WRITE, EV_TIMEOUT};
    const unsigned int masks[] = {MK_EVENT_READ, MK_EVENT_WRITE,
                                 MK_EVENT_READ | MK_EVENT_WRITE, 0};
    bindings = fopen(getenv("QCORE_MONKEY_BINDINGS"), "w");
    assert(bindings != NULL);
    setvbuf(stdout, NULL, _IONBF, 0);
    loop = mk_event_loop_create(requested_capacity);
    assert(loop != NULL);
    ctx = loop->data;
    FILE *runtime = fopen(getenv("QCORE_CB_EVENT_RUNTIME"), "w");
    assert(runtime != NULL);
    fprintf(runtime, "{\"backend\":\"%s\",\"dispatcher_count\":1}\n", event_base_get_method(ctx->base));
    assert(fclose(runtime) == 0);
    FILE *allocation = fopen(getenv("QCORE_CB_EVENT_ALLOCATION"), "w");
    assert(allocation != NULL);
    fprintf(allocation,
            "{\"schema_version\":1,\"context_object\":%lu,\"fired_base\":%lu,"
            "\"fired_capacity\":%lu,\"fired_element_size\":%lu,"
            "\"fired_allocation_bytes\":%lu,\"loop_size_argument\":%d,"
            "\"context_allocation_bytes\":%lu,\"sizeof_context\":%lu}\n",
            (unsigned long) ctx, (unsigned long) ctx->fired,
            (unsigned long) ctx->queue_size, (unsigned long) sizeof(struct mk_event),
            (unsigned long) allocated_bytes(ctx->fired), requested_capacity,
            (unsigned long) allocated_bytes(ctx), (unsigned long) sizeof(*ctx));
    assert(fclose(allocation) == 0);
    MK_EVENT_ZERO(&channel);
    assert(mk_event_channel_create(loop, &reader, &writer, &channel) == 0);
    struct ev_map *map = channel.data;
    for (unsigned int i = 0; i < 4; i++) {
        event_active(map->event, flags[i], 1);
        assert(mk_event_wait_2(loop, 0) == 1);
        assert(ctx->fired[0].data == &channel && ctx->fired[0].mask == masks[i]);
    }
    assert(mk_event_channel_destroy(loop, reader, writer, &channel) == 0);
    mk_event_loop_destroy(loop);
    assert(seen_event == 4 && seen_timeout == 0 && seen_wait_timeout == 0);
    for (unsigned int i = 0; i < callback_count; i++) assert(callbacks[i].freed);
    for (unsigned int i = 0; i < descriptor_count; i++) assert(!descriptors[i].live);
    record("completed", 0, "", 0);
    assert(fclose(bindings) == 0);
    return 0;
}
#else
int main(void)
{
    struct mk_event_loop *loop;
    struct mk_event channel = {0};
    struct mk_event timer = {0};
    int reader;
    int writer;
    unsigned int index;
    char value = 'x';

    bindings = fopen(getenv("QCORE_MONKEY_BINDINGS"), "w");
    assert(bindings != NULL);
    /* Timer teardown relies on observing a failed send to the closed reader. */
    setvbuf(stdout, NULL, _IONBF, 0);
    signal(SIGPIPE, SIG_IGN);
    loop = mk_event_loop_create(8);
    assert(loop != NULL);
    /* Actual wrapper failure edge: socketpair creates no descriptor custody. */
    fail_socketpair = 1;
    MK_EVENT_ZERO(&channel);
    assert(mk_event_channel_create(loop, &reader, &writer, &channel) == -1);
    assert(!fail_socketpair);
    MK_EVENT_ZERO(&channel);
    assert(mk_event_channel_create(loop, &reader, &writer, &channel) == 0);
    if (observed_send(writer, &value, 1, 0) != 1) { perror("channel send"); abort(); }
    assert(mk_event_wait_2(loop, 100) == 1);
    assert(((struct mk_event_ctx *) loop->data)->fired[0].data == &channel);
    assert(observed_recv(reader, &value, 1, 0) == 1);
    /* Drive all remaining mask branches through Libevent's actual dispatcher.
     * event_active supplies controlled flags; these are not readiness claims. */
    {
        struct ev_map *map = channel.data;
        const short flags[] = {EV_WRITE, EV_READ | EV_WRITE, EV_TIMEOUT};
        const unsigned int masks[] = {MK_EVENT_WRITE, MK_EVENT_READ | MK_EVENT_WRITE, 0};
        for (index = 0; index < sizeof(flags) / sizeof(flags[0]); index++) {
            event_active(map->event, flags[index], 1);
            assert(mk_event_wait_2(loop, 0) == 1);
            assert(((struct mk_event_ctx *) loop->data)->fired[0].mask == masks[index]);
        }
    }
    assert(mk_event_wait_2(loop, 2) == 0);
    assert(mk_event_channel_destroy(loop, reader, writer, &channel) == 0);
    mk_event_loop_destroy(loop);

    loop = mk_event_loop_create(8);
    assert(loop != NULL);
    MK_EVENT_ZERO(&timer);
    assert(mk_event_timeout_create(loop, 0, 1000000, &timer) >= 0);
    assert(mk_event_timeout_destroy(loop, &timer) == 0);
    assert(mk_event_wait_2(loop, 100) == 0);
    mk_event_loop_destroy(loop);

    /* Successful timer write, then a labeled injected ECONNABORTED send.
     * The second run executes native cleanup but is not an OS failure sample. */
    loop = mk_event_loop_create(8);
    assert(loop != NULL);
    MK_EVENT_ZERO(&timer);
    assert(mk_event_timeout_create(loop, 60, 0, &timer) >= 0);
    {
        struct observed_callback *timeout = NULL;
        uint64_t tick = 0;
        for (index = 0; index < callback_count; index++) {
            if (!callbacks[index].freed && callbacks[index].callback == cb_timeout) {
                assert(timeout == NULL);
                timeout = &callbacks[index];
            }
        }
        assert(timeout != NULL);
        event_active(timeout->event, EV_TIMEOUT, 1);
        mk_event_wait_2(loop, 0);
        assert(!timeout->freed);
        assert(observed_recv(timer.fd, &tick, sizeof(tick), 0) == sizeof(tick));
        assert(tick == 1);
        fail_send_errno = ECONNABORTED;
        event_active(timeout->event, EV_TIMEOUT, 1);
        mk_event_wait_2(loop, 0);
        assert(timeout->freed && fail_send_errno == 0);
    }
    assert(mk_event_timeout_destroy(loop, &timer) == 0);
    mk_event_loop_destroy(loop);

    assert(seen_event >= 4 && seen_timeout == 3 && seen_wait_timeout > 0);
    for (index = 0; index < callback_count; index++) { assert(callbacks[index].freed); }
    for (index = 0; index < descriptor_count; index++) { assert(!descriptors[index].live); }
    record("completed", 0, "", 0);
    assert(fclose(bindings) == 0);
    return 0;
}

#endif
