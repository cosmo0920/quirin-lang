#include <threads.h>
#include <stdio.h>
struct rec { thrd_t thread; unsigned long identity; int (*worker)(void*); long args[4]; long result; };
static struct rec R;
static int body(void *p) {
    struct rec *r = p;
    r->result = r->identity + 1;   /* an instrumented memory access */
    return 0;
}
int main(void) {
    R.identity = 41;
    if (thrd_create(&R.thread, body, &R) != thrd_success) return 1;
    int st; thrd_join(R.thread, &st);
    printf("result=%ld\n", R.result);
    return 0;
}
