/* CT07 heap witness. Records the single leased object by address so "two
   threads, one allocation, counted once, released once" is checked by identity
   rather than by the emitted trace alone. The two workers write disjoint bytes
   of exactly this object, so a second allocation would break the scenario
   silently without it. */
#include <stdio.h>
#include <stdlib.h>
static void *lease_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "LEASE_ALLOC %p\n", p); return p; }
static void lease_free(void *p) { fprintf(stderr, "LEASE_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC lease_alloc
#define QCORE_BUFFER_FREE lease_free
