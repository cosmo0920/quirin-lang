#include <threads.h>
#include <stdio.h>
static int body(void *p) { (void)p; return 0; }
int main(void) {
    thrd_t t;
    if (thrd_create(&t, body, 0) != thrd_success) return 1;
    int st; thrd_join(t, &st);
    printf("ok\n");
    return 0;
}
