#include <stdlib.h>
static void *fail_buffer(size_t size) {(void)size;return NULL;}
static void forbidden_free(void *p) {(void)p;abort();}
#define QCORE_BUFFER_MALLOC fail_buffer
#define QCORE_BUFFER_FREE forbidden_free
