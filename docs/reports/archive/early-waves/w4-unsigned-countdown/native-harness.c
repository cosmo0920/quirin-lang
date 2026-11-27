#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

static unsigned acquired, released, live;
void *q_alloc(void) {
    void *p = malloc(8);
    assert(p);
    acquired++;
    live++;
    return p;
}
void q_free(void *p) {
    assert(p && live);
    free(p);
    live--;
    released++;
}
void subject(void);
int main(void) {
    subject();
    assert(acquired == 6 && released == 6 && live == 0);
    printf("acquired=%u released=%u live=%u\n", acquired, released, live);
    return 0;
}
