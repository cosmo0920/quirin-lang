void *q_alloc(void);
void q_free(void *);

void countdown_release(void *p, unsigned n) {
    if (n) { countdown_release(p, n - 1); return; }
    q_free(p);
}
void *countdown_make(unsigned n) {
    if (n) { void *p = countdown_make(n - 1); return p; }
    void *p = q_alloc();
    return p;
}
void narrow(unsigned char n, void *p);
void wide(void *p, unsigned long n) {
    if (n) { narrow(n - 1, p); return; }
    q_free(p);
}
void narrow(unsigned char n, void *p) {
    if (n) { wide(p, n - 1); return; }
    q_free(p);
}
void subject(void) {
    void *a = q_alloc(); countdown_release(a, 0);
    void *b = q_alloc(); countdown_release(b, 1);
    void *c = q_alloc(); countdown_release(c, 7);
    void *d = countdown_make(7); q_free(d);
    void *e = q_alloc(); wide(e, 513);
    void *f = q_alloc(); narrow(7, f);
}
