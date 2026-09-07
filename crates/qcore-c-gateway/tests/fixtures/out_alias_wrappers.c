void q_make(void **);
void q_free(void *);
void fill(void **out) {
    void **first=out;
    void **alias=first;
    q_make(alias);
}
void maybe(void **out, int flag) {
    void **alias=out;
    if(flag) { q_make(alias); return; }
    *alias=0;
}
void two(void **a, void **b) {
    void **first=a;
    void **second=b;
    q_make(first);
    q_make(second);
}
void subject(void) {
    void *p;
    void *q;
    fill(&p);
    q_free(p);
    maybe(&p,0);
    if(p) { q_free(p); }
    maybe(&p,1);
    if(p) { q_free(p); }
    two(&p,&q);
    q_free(p);
    q_free(q);
}
