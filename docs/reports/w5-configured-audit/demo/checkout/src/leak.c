extern void *alloc(void);
void leak(void) { void *p = alloc(); }
