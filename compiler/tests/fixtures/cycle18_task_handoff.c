#define _POSIX_C_SOURCE 200809L
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static pthread_t task_thread;
static pthread_mutex_t state_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t state_cond = PTHREAD_COND_INITIALIZER;
static int released;
static int started;
static int cancel_requested;
static int cancel_acknowledged;
static unsigned long callback_thread_id;
static unsigned long worker_thread_id;

static void *task_body(void *unused)
{
    (void) unused;
    worker_thread_id = (unsigned long) pthread_self();
    pthread_mutex_lock(&state_mutex);
    while (!released) {
        pthread_cond_wait(&state_cond, &state_mutex);
    }
    started = 1;
    pthread_cond_broadcast(&state_cond);
    while (!cancel_requested) {
        pthread_cond_wait(&state_cond, &state_mutex);
    }
    cancel_acknowledged = 1;
    pthread_cond_broadcast(&state_cond);
    pthread_mutex_unlock(&state_mutex);
    return NULL;
}

int qcore_task_spawn(int callback, int task, int argument, int retains_argument)
{
    if (callback != 1 || task != 10 || argument != 100 || retains_argument != 1) {
        return -1;
    }
    return pthread_create(&task_thread, NULL, task_body, NULL);
}

int qcore_defer_register(int callback, int work, int argument, int retains_argument)
{
    return (callback == 1 && work == 20 && argument == 100 && retains_argument == 1) ? 0 : -1;
}

int qcore_task_release(int task)
{
    if (task != 10) return -1;
    pthread_mutex_lock(&state_mutex);
    released = 1;
    pthread_cond_broadcast(&state_cond);
    while (!started) pthread_cond_wait(&state_cond, &state_mutex);
    pthread_mutex_unlock(&state_mutex);
    return 0;
}

int qcore_task_cancel_request(int callback, int task)
{
    if (callback != 1 || task != 10) return -1;
    pthread_mutex_lock(&state_mutex);
    cancel_requested = 1;
    pthread_cond_broadcast(&state_cond);
    pthread_mutex_unlock(&state_mutex);
    return 0;
}

int qcore_task_wait_cancel_ack(int task)
{
    if (task != 10) return -1;
    pthread_mutex_lock(&state_mutex);
    while (!cancel_acknowledged) pthread_cond_wait(&state_cond, &state_mutex);
    pthread_mutex_unlock(&state_mutex);
    return 0;
}

int qcore_task_join(int callback, int task)
{
    if (callback != 1 || task != 10 || pthread_join(task_thread, NULL) != 0) return -1;
    return 0;
}

int qcore_defer_start(int work)
{
    return work == 20 ? 0 : -1;
}

int qcore_defer_complete(int work)
{
    return work == 20 ? 0 : -1;
}

int qcore_argument_drop(int argument)
{
    return argument == 100 ? 0 : -1;
}

int qcore_callback_teardown(int callback)
{
    return callback == 1 ? 0 : -1;
}

void qcore_observe_callback_enter(int callback, int argument)
{
    if (callback != 1 || argument != 100) abort();
    callback_thread_id = (unsigned long) pthread_self();
}

void qcore_observe_callback_exit(int callback)
{
    if (callback != 1) abort();
}

void cycle18_taskful_callback(int fd, short flags, void *data)
{
    qcore_observe_callback_enter(1, 100);
    qcore_task_spawn(1, 10, 100, 1);
    qcore_defer_register(1, 20, 100, 1);
    qcore_observe_callback_exit(1);
}

void cycle18_taskful_after_return(void)
{
    qcore_task_release(10);
    qcore_task_cancel_request(1, 10);
    qcore_task_wait_cancel_ack(10);
    qcore_task_join(1, 10);
    qcore_defer_start(20);
    qcore_defer_complete(20);
    qcore_argument_drop(100);
    qcore_callback_teardown(1);
}

int main(void)
{
    cycle18_taskful_callback(0, 0, NULL);
    cycle18_taskful_after_return();
    if (!started || !cancel_acknowledged || callback_thread_id == worker_thread_id) abort();
    printf("{\"schema_version\":1,\"producer\":\"cycle18-task-handoff-pthread-v1\",\"capabilities\":[\"callback_boundaries\",\"task_lifecycle\",\"deferred_lifecycle\"],\"records\":["
           "{\"schema_version\":1,\"sequence\":1,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"callback_enter\",\"callback\":\"cycle18_taskful_callback\",\"argument\":100}},"
           "{\"schema_version\":1,\"sequence\":2,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"task_spawn\",\"callback\":\"cycle18_taskful_callback\",\"task\":10}},"
           "{\"schema_version\":1,\"sequence\":3,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"deferred_register\",\"callback\":\"cycle18_taskful_callback\",\"work\":20}},"
           "{\"schema_version\":1,\"sequence\":4,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"callback_exit\",\"callback\":\"cycle18_taskful_callback\"}},"
           "{\"schema_version\":1,\"sequence\":5,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"task_start\",\"task\":10}},"
           "{\"schema_version\":1,\"sequence\":6,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"task_cancel_request\",\"callback\":\"cycle18_taskful_callback\",\"task\":10}},"
           "{\"schema_version\":1,\"sequence\":7,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"task_cancel_ack\",\"task\":10}},"
           "{\"schema_version\":1,\"sequence\":8,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"task_join\",\"task\":10}},"
           "{\"schema_version\":1,\"sequence\":9,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"deferred_start\",\"work\":20}},"
           "{\"schema_version\":1,\"sequence\":10,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"deferred_complete\",\"work\":20}},"
           "{\"schema_version\":1,\"sequence\":11,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"argument_drop\",\"argument\":100}},"
           "{\"schema_version\":1,\"sequence\":12,\"thread\":%lu,\"order\":\"causal_replay\",\"action\":{\"action\":\"teardown\",\"callback\":\"cycle18_taskful_callback\"}}]}\n",
           callback_thread_id, callback_thread_id, callback_thread_id,
           callback_thread_id, worker_thread_id, callback_thread_id,
           worker_thread_id, callback_thread_id, callback_thread_id,
           callback_thread_id, callback_thread_id, callback_thread_id);
    return 0;
}
