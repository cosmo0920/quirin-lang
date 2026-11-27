#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#define FLB_SCHED_TIMER_REQUEST 1
#define MK_EVENT_EMPTY 0
#define FLB_SCHED_REQUEST_FRAME 60
#define flb_error(...) ((void) 0)
struct flb_sched { int unused; };
struct flb_config { struct flb_sched *sched; int is_shutting_down; int sched_base; int sched_cap; };
struct fake_event { int mask; };
struct flb_sched_request;
struct flb_sched_timer { int type; void *data; struct fake_event event; };
struct flb_sched_request { int fd; time_t created; int timeout; void *data; struct flb_sched_timer *timer; };
static struct flb_sched_timer *allocated_timer;
static int live_timers;
static struct flb_sched_timer *flb_sched_timer_create(struct flb_sched *sched) {
    (void) sched;
    allocated_timer = calloc(1, sizeof *allocated_timer);
    if (allocated_timer) live_timers++;
    return allocated_timer;
}
static int flb_sched_timer_destroy(struct flb_sched_timer *timer) {
    free(timer);
    live_timers--;
    return 0;
}
static void *flb_malloc(size_t size) { (void) size; return NULL; }
static void flb_free(void *ptr) { free(ptr); }
static int flb_errno(void) { return 0; }
static int backoff_full_jitter(int base, int cap, int tries) {
    (void) base; (void) cap; (void) tries; return 1;
}
static int schedule_request_wait(struct flb_sched_request *request, struct flb_config *config) {
    (void) request; (void) config; return 0;
}
static int schedule_request_now(int seconds, struct flb_sched_timer *timer,
                                struct flb_sched_request *request, struct flb_config *config) {
    (void) seconds; (void) timer; (void) request; (void) config; return 0;
}

/* Exact bytes extracted from pinned src/flb_scheduler.c */
int flb_sched_request_create(struct flb_config *config, void *data, int tries)
{
    int ret;
    int seconds;
    struct flb_sched_timer *timer;
    struct flb_sched_request *request;

    /* Allocate timer context */
    timer = flb_sched_timer_create(config->sched);
    if (!timer) {
        return -1;
    }

    /* Allocate request node */
    request = flb_malloc(sizeof(struct flb_sched_request));
    if (!request) {
        flb_errno();
        return -1;
    }

    /* Link timer references */
    timer->type = FLB_SCHED_TIMER_REQUEST;
    timer->data = request;
    timer->event.mask = MK_EVENT_EMPTY;

    /* Get suggested wait_time for this request. If shutting down, set to 0. */
    if (config->is_shutting_down) {
        seconds = 0;
    } else {
        seconds = backoff_full_jitter((int)config->sched_base, (int)config->sched_cap,
                                      tries);
    }
    seconds += 1;

    /* Populare request */
    request->fd      = -1;
    request->created = time(NULL);
    request->timeout = seconds;
    request->data    = data;
    request->timer   = timer;

    /* Request to be placed into the sched_requests_wait list */
    if (seconds > FLB_SCHED_REQUEST_FRAME) {
        schedule_request_wait(request, config);
    }
    else {
        ret = schedule_request_now(seconds, timer, request, config);
        if (ret == -1) {
            flb_error("[sched]  'retry request' could not be created. the "
                      "system might be running out of memory or file "
                      "descriptors.");
            flb_sched_timer_destroy(timer);
            flb_free(request);
            return -1;
        }
    }

    return seconds;
}

int main(void) {
    struct flb_sched sched = {0};
    struct flb_config config = {&sched, 0, 5, 10};
    int result = flb_sched_request_create(&config, NULL, 1);
    printf("result=%d live_timers_before_teardown=%d timer_pointer=%p\n",
           result, live_timers, (void *) allocated_timer);
    if (result != -1 || live_timers != 1 || allocated_timer == NULL) return 1;
    flb_sched_timer_destroy(allocated_timer); /* simulated eventual scheduler teardown */
    printf("live_timers_after_simulated_teardown=%d\n", live_timers);
    return live_timers == 0 ? 0 : 1;
}
