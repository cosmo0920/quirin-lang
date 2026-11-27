/* Card OTEL-PROTO-AUDIT fixture: the protobuf-c shapes the site reader
 * classifies (no includes, so its Clang AST stays small). */
typedef struct ProtobufCBinaryData { unsigned long len; unsigned char *data; } ProtobufCBinaryData;
typedef enum {
    OPENTELEMETRY__PROTO__T__X__V__NOT_SET = 0,
    OPENTELEMETRY__PROTO__T__X__V_S = 1,
    OPENTELEMETRY__PROTO__T__X__V_M = 2
} Opentelemetry__Proto__T__X__VCase;
typedef struct Opentelemetry__Proto__T__M { int x; } Opentelemetry__Proto__T__M;
typedef struct Opentelemetry__Proto__T__X {
    Opentelemetry__Proto__T__X__VCase v_case;
    union { char *s; Opentelemetry__Proto__T__M *m; };
    unsigned long n_e;
    Opentelemetry__Proto__T__M **e;
    Opentelemetry__Proto__T__M *sub;
    ProtobufCBinaryData b;
} Opentelemetry__Proto__T__X;

Opentelemetry__Proto__T__X *opentelemetry__proto__t__x__unpack(void *a, unsigned long n,
                                                               const unsigned char *d);
int use(Opentelemetry__Proto__T__M *m);

static int take(Opentelemetry__Proto__T__M *m)
{
    if (m == 0) {
        return -1;
    }
    return m->x;
}

static int each(unsigned long n, Opentelemetry__Proto__T__M **arr)
{
    unsigned long i;
    int t = 0;
    for (i = 0; i < n; i++) {
        t += arr[i]->x;
    }
    return t;
}

int decode(const unsigned char *buf, unsigned long len)
{
    Opentelemetry__Proto__T__X *x;
    Opentelemetry__Proto__T__M *alias;
    unsigned long i;
    int t = 0;

    x = opentelemetry__proto__t__x__unpack(0, len, buf);
    if (x == 0) {
        return -1;
    }
    if (x->sub != 0) {
        t += x->sub->x;
    }
    t += x->sub->x;
    for (i = 0; i < x->n_e; i++) {
        t += x->e[i]->x;
    }
    t += x->e[0]->x;
    switch (x->v_case) {
    case OPENTELEMETRY__PROTO__T__X__V_M:
        t += x->m->x;
        break;
    default:
        break;
    }
    t += x->m->x;
    t += take(x->sub);
    t += use(x->sub);
    t += each(x->n_e, x->e);
    if (x->b.len > 0) {
        t += x->b.data[0];
    }
    alias = x->sub;
    if (alias == 0) {
        return t;
    }
    t += alias->x;
    return t;
}

static int encode(Opentelemetry__Proto__T__X *x)
{
    return x->sub->x;
}

int encode_entry(Opentelemetry__Proto__T__X *x)
{
    return encode(x);
}
