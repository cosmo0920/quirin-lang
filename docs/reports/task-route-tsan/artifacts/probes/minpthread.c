#include <pthread.h>
#include <stdio.h>
static long result;
static void *body(void *p) { (void)p; result = 42; return 0; }
int main(void) {
    pthread_t t;
    if (pthread_create(&t, 0, body, 0) != 0) return 1;
    pthread_join(t, 0);
    printf("result=%ld\n", result);
    return 0;
}
