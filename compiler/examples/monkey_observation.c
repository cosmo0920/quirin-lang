/* Observation adapter around the actual pinned Monkey translation unit.
 * QCORE_MONKEY_SOURCE is supplied by the audited compilation-database runner.
 * Hooks delegate to the real Libevent calls; callbacks invoke the original
 * Monkey function pointers. No synthetic callback trace is generated.
 */
#include <assert.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <event2/event.h>
#include <event2/util.h>
#include <mk_core/mk_event.h>

static void cb_event(evutil_socket_t, short, void *);
static void cb_timeout(evutil_socket_t, short, void *);
static void cb_wait_2_timeout(evutil_socket_t, short, void *);

struct observed_callback {
    struct event *event;
    struct event_base *base;
    void (*callback)(evutil_socket_t, short, void *);
    void *argument;
    unsigned int identity;
    int freed;
};

static struct observed_callback callbacks[16];
static unsigned int callback_count;
static unsigned int base_id;
static unsigned int sequence;
static struct event_base *dispatch_base;
static pthread_t dispatcher;
static int dispatching;
static unsigned int seen_event;
static unsigned int seen_timeout;
static unsigned int seen_wait_timeout;

static const char *callback_name(void (*callback)(evutil_socket_t, short, void *))
{
    if (callback == cb_event) { return "cb_event"; }
    if (callback == cb_timeout) { return "cb_timeout"; }
    if (callback == cb_wait_2_timeout) { return "cb_wait_2_timeout"; }
    abort();
}

static void record(const char *kind, unsigned int event, const char *name, int result)
{
    printf("{\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,\"kind\":\"%s\",\"event\":%u,\"callback\":\"%s\",\"result\":%d}\n",
           ++sequence, base_id, (unsigned long) pthread_self(), kind, event, name, result);
}

static void callback_hook(evutil_socket_t fd, short flags, void *argument)
{
    struct observed_callback *entry = argument;
    assert(dispatching && pthread_equal(dispatcher, pthread_self()));
    assert(entry->base == dispatch_base && !entry->freed);
    record("callback_enter", entry->identity, callback_name(entry->callback), 0);
    if (entry->callback == cb_event) { seen_event++; }
    if (entry->callback == cb_timeout) { seen_timeout++; }
    if (entry->callback == cb_wait_2_timeout) { seen_wait_timeout++; }
    entry->callback(fd, flags, entry->argument);
    record("callback_exit", entry->identity, callback_name(entry->callback), 0);
}

static struct event_base *observed_base_new(void)
{
    struct event_base *base = event_base_new();
    assert(base != NULL);
    base_id++;
    printf("{\"sequence\":%u,\"base\":%u,\"dispatcher\":1,\"thread\":%lu,\"kind\":\"backend\",\"backend\":\"%s\"}\n",
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
    entry->callback = callback;
    entry->argument = argument;
    entry->event = event_new(base, fd, flags, callback_hook, entry);
    assert(entry->event != NULL);
    record("event_new", entry->identity, callback_name(callback), 0);
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
    return result;
}

static void observed_event_free(struct event *event)
{
    struct observed_callback *entry = find_event(event);
    entry->freed = 1;
    event_free(event);
    record("event_free", entry->identity, callback_name(entry->callback), 0);
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
    int result = evutil_closesocket(fd);
    record("descriptor_close", 0, "", result);
    return result;
}

static void observed_base_free(struct event_base *base)
{
    assert(!dispatching);
    event_base_free(base);
    record("base_free", 0, "", 0);
}

#define event_base_new observed_base_new
#define event_new observed_event_new
#define event_add observed_event_add
#define event_del observed_event_del
#define event_free observed_event_free
#define event_base_loop observed_loop
#define event_base_free observed_base_free
#define evutil_closesocket observed_close
#include QCORE_MONKEY_SOURCE
#undef event_base_new
#undef event_new
#undef event_add
#undef event_del
#undef event_free
#undef event_base_loop
#undef event_base_free
#undef evutil_closesocket

int main(void)
{
    struct mk_event_loop *loop;
    struct mk_event channel = {0};
    struct mk_event timer = {0};
    int reader;
    int writer;
    unsigned int index;
    char value = 'x';

    /* Timer teardown relies on observing a failed send to the closed reader. */
    setvbuf(stdout, NULL, _IONBF, 0);
    signal(SIGPIPE, SIG_IGN);
    loop = mk_event_loop_create(8);
    assert(loop != NULL);
    MK_EVENT_ZERO(&channel);
    assert(mk_event_channel_create(loop, &reader, &writer, &channel) == 0);
    if (send(writer, &value, 1, 0) != 1) { perror("channel send"); abort(); }
    assert(mk_event_wait_2(loop, 100) == 1);
    assert(((struct mk_event_ctx *) loop->data)->fired[0].data == &channel);
    assert(recv(reader, &value, 1, 0) == 1);
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

    assert(seen_event > 0 && seen_timeout > 0 && seen_wait_timeout > 0);
    for (index = 0; index < callback_count; index++) { assert(callbacks[index].freed); }
    record("completed", 0, "", 0);
    return 0;
}
