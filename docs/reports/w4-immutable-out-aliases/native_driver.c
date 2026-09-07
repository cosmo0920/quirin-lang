/* Execute the same C wrappers the public gateway/certificate regression imports. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "../../../crates/qcore-c-gateway/tests/fixtures/out_alias_wrappers.c"
static void *live[4];
static unsigned acquired,released;
void q_make(void **out) {
    if (acquired>=4) abort();
    void *pointer=malloc(32);
    if (!pointer) abort();
    live[acquired]=pointer;
    ((unsigned char *)pointer)[0]=(unsigned char)(acquired+1);
    ++acquired;
    *out=pointer;
    printf("ACQUIRE %u\n",acquired);
}
void q_free(void *pointer) {
    unsigned slot=0;
    while (slot<acquired && live[slot]!=pointer) ++slot;
    if (!pointer || slot==acquired) abort();
    if (((unsigned char *)pointer)[0] != slot+1) abort();
    live[slot]=NULL;
    printf("RELEASE %u\n",slot+1);
    ++released;
    free(pointer);
}
int main(void) {
    subject();
    if (acquired!=4 || released!=4) abort();
    for (unsigned i=0;i<4;++i) if(live[i]) abort();
    puts("RUN_CLEAN 4");
    return 0;
}
