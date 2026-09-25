#!/usr/bin/env python3
"""Extract the unedited pinned request-create function into a stubbed C experiment."""
import argparse
import hashlib
import pathlib

root = pathlib.Path('/home/cosmo/Documents/GitHub/fluent-bit')
parser = argparse.ArgumentParser()
parser.add_argument('--out', type=pathlib.Path, required=True, help='fresh private output directory')
args = parser.parse_args()
out = args.out
out.mkdir(parents=True, exist_ok=False)
source = (root / 'src/flb_scheduler.c').read_text()
start = source.index('int flb_sched_request_create(struct flb_config *config, void *data, int tries)')
end = source.index('\nint flb_sched_request_destroy(', start)
function = source[start:end].rstrip() + '\n'
assert hashlib.sha256(function.encode()).hexdigest() == '7d65c645bd2b5ecb0b275d357689b7c83cbc20e0ab63595599dc109758becc44'
assert function.count('flb_sched_timer_create(config->sched)') == 1
assert function.count('request = flb_malloc(sizeof(struct flb_sched_request));') == 1
assert 'flb_sched_timer_destroy(timer);\n        return -1;' not in function
(out / 'c1-extracted-function.c.txt').write_text(function)
(out / 'c1-extracted-function.sha256').write_text(hashlib.sha256(function.encode()).hexdigest() + '\n')
prefix = r'''#include <stddef.h>
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
'''
suffix = r'''
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
'''
(out / 'c1-harness.c').write_text(prefix + '\n/* Exact bytes extracted from pinned src/flb_scheduler.c */\n' + function + suffix)
print('extracted_sha256=' + hashlib.sha256(function.encode()).hexdigest())
print('harness=' + str(out / 'c1-harness.c'))
