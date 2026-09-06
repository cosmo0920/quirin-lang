extern void *alloc(void); extern void dispose(void *p);
void balanced(void) { void *p = alloc(); dispose(p); }
