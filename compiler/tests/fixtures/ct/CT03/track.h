/* CT03 heap witness. Records the single shared object by address so "two
   readers, one allocation, counted once" is checked by identity rather than by
   the emitted trace alone. */
#include <stdio.h>
#include <stdlib.h>
static void *share_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "SHARE_ALLOC %p\n", p); return p; }
static void share_free(void *p) { fprintf(stderr, "SHARE_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC share_alloc
#define QCORE_BUFFER_FREE share_free
