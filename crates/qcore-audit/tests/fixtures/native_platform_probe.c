#include <event2/event.h>
#include <event2/thread.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

struct probe {
    struct event_base *base;
    struct event *event;
    atomic_ullong activating_thread;
    atomic_ullong callback_thread;
    atomic_uint activations;
    atomic_uint callbacks;
    atomic_uint callback_flags;
};

static unsigned long long thread_identity(void)
{
    return (unsigned long long) syscall(SYS_gettid);
}

static void activated(evutil_socket_t descriptor, short flags, void *argument)
{
    struct probe *probe = argument;
    (void) descriptor;
    atomic_store_explicit(&probe->callback_flags, (unsigned int) flags, memory_order_relaxed);
    atomic_store_explicit(&probe->callback_thread, thread_identity(), memory_order_relaxed);
    atomic_fetch_add_explicit(&probe->callbacks, 1, memory_order_relaxed);
    if (event_base_loopbreak(probe->base) != 0) {
        abort();
    }
}

static void *activate_from_worker(void *argument)
{
    struct probe *probe = argument;
    const struct timespec delay = {.tv_sec = 0, .tv_nsec = 50000000};
    (void) nanosleep(&delay, NULL);
    atomic_store_explicit(&probe->activating_thread, thread_identity(), memory_order_relaxed);
    atomic_fetch_add_explicit(&probe->activations, 1, memory_order_relaxed);
    event_active(probe->event, EV_READ, 1);
    return NULL;
}

static void guard_expired(evutil_socket_t descriptor, short flags, void *argument)
{
    (void) descriptor;
    (void) flags;
    (void) argument;
    abort();
}

int main(void)
{
    struct probe probe = {0};
    struct event *guard;
    pthread_t worker;
    const struct timeval guard_timeout = {.tv_sec = 5, .tv_usec = 0};
    unsigned long long dispatch_thread;
    const char *backend;
    int dispatch_result;

    if (evthread_use_pthreads() != 0) {
        return 10;
    }
    probe.base = event_base_new();
    if (probe.base == NULL) {
        return 11;
    }
    backend = event_base_get_method(probe.base);
    if (backend == NULL) {
        return 12;
    }
    probe.event = event_new(probe.base, -1, EV_PERSIST, activated, &probe);
    guard = evtimer_new(probe.base, guard_expired, NULL);
    if (probe.event == NULL || guard == NULL || event_add(guard, &guard_timeout) != 0) {
        return 13;
    }
    dispatch_thread = thread_identity();
    if (pthread_create(&worker, NULL, activate_from_worker, &probe) != 0) {
        return 14;
    }
    dispatch_result = event_base_dispatch(probe.base);
    if (pthread_join(worker, NULL) != 0) {
        return 15;
    }
    printf("{\"backend\":\"%s\",\"dispatch_thread\":%llu,"
           "\"activating_thread\":%llu,\"callback_thread\":%llu,"
           "\"activation_count\":%u,\"callback_count\":%u,\"callback_flags\":%u,"
           "\"dispatch_result\":%d}\n",
           backend, dispatch_thread,
           atomic_load_explicit(&probe.activating_thread, memory_order_relaxed),
           atomic_load_explicit(&probe.callback_thread, memory_order_relaxed),
           atomic_load_explicit(&probe.activations, memory_order_relaxed),
           atomic_load_explicit(&probe.callbacks, memory_order_relaxed),
           atomic_load_explicit(&probe.callback_flags, memory_order_relaxed),
           dispatch_result);
    event_free(guard);
    event_free(probe.event);
    event_base_free(probe.base);
    return dispatch_result == 0 ? 0 : 16;
}
