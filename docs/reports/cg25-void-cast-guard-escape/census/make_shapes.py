"""Census shapes for CG25-VOID-CAST-GUARD-ESCAPE (probe input generator).

Every regex shape S runs twice over a tracked `re` (compiled, status tested):
  pre:  S before k_regfree(&re)   -- S reads/uses live tracked storage
  post: S after  k_regfree(&re)   -- S reads/uses released tracked storage
A shape naming `re` outside a kinded position must never be Safe (rule 11v (4));
a kinded use must not be Safe in `post`.
"""
import sys

PRE = ("int w(const char *p, const char *s) { regex_t re; regex_t other; int r; "
       "int t; unsigned long n; void *q; r = k_regcomp(&re, p, 0); if (r) return 0; ")

SHAPES = [
    # (label, statement, kinded?)  kinded: every mention is &re at a regex position
    ("call-plain-kinded", "k_regexec(&re, s);", True),
    ("void-cast-kinded", "(void) k_regexec(&re, s);", True),
    ("assign-kinded", "t = k_regexec(&re, s);", True),
    ("assign-ne-kinded", "t = k_regexec(&re, s) != 1;", True),
    ("void-cast-assign-kinded", "(void) (t = k_regexec(&re, s));", True),
    ("cast-int-kinded", "t = (int) k_regexec(&re, s);", True),
    ("comma-kinded", "t = 0, k_regexec(&re, s);", True),
    ("void-comma-kinded", "(void) (t = 0, k_regexec(&re, s));", True),
    ("stmt-expr-kinded", "t = ({ k_regexec(&re, s); });", True),
    ("cond-kinded", "if (k_regexec(&re, s) == 0) t = 1;", True),
    ("call-plain-peek", "k_peek(&re);", False),
    ("void-cast-peek", "(void) k_peek(&re);", False),
    ("assign-peek", "t = k_peek(&re);", False),
    ("assign-ne-peek", "t = k_peek(&re) != 1;", False),
    ("void-cast-assign-peek", "(void) (t = k_peek(&re));", False),
    ("cast-int-peek", "t = (int) k_peek(&re);", False),
    ("cast-ptr-peek", "k_peek((const void *) &re);", False),
    ("void-cast-ptr-peek", "(void) k_peek((const void *) &re);", False),
    ("comma-peek", "t = 0, k_peek(&re);", False),
    ("void-comma-peek", "(void) (t = 0, k_peek(&re));", False),
    ("stmt-expr-peek", "t = ({ k_peek(&re); });", False),
    ("void-stmt-expr-peek", "(void) ({ k_peek(&re); });", False),
    ("cond-peek", "if (k_peek(&re) == 0) t = 1;", False),
    ("ternary-peek", "t = s ? k_peek(&re) : 0;", False),
    ("void-wipe", "(void) k_wipe(&re, sizeof re);", False),
    ("wipe", "k_wipe(&re, sizeof re);", False),
    ("void-take-member", "(void) k_take(re.used);", False),
    ("take-member", "k_take(re.used);", False),
    ("assign-take-member", "t = k_take(re.used);", False),
    ("void-take-member-cast", "(void) k_take((unsigned long) re.used);", False),
    ("member-read", "n = re.used;", False),
    ("member-read-cast", "n = (unsigned long) re.used;", False),
    ("void-member", "(void) re.used;", False),
    ("void-object", "(void) re;", False),
    ("void-member-assign", "(void) (n = re.used);", False),
    ("comma-member", "t = 0, n = re.used;", False),
    ("stmt-expr-member", "n = ({ re.used; });", False),
    ("struct-copy", "other = re;", False),
    ("void-struct-copy", "(void) (other = re);", False),
    ("address-store", "q = &re;", False),
    ("address-store-cast", "q = (void *) &re;", False),
    ("void-address-store", "(void) (q = &re);", False),
    ("member-write", "re.used = 0;", False),
    ("void-member-write", "(void) (re.used = 0);", False),
    ("cond-member", "if (re.used) t = 1;", False),
    ("kinded-plus-member", "t = k_regexec(&re, s) + (int) re.used;", False),
    ("void-kinded-plus-member", "(void) (k_regexec(&re, s) + (int) re.used);", False),
    ("free-address", "k_free(&re);", False),
    ("void-free-address", "(void) k_free(&re);", False),
    ("void-free-address-cast", "(void) k_free((void *) &re);", False),
    ("void-regfree", "(void) k_regfree(&re);", True),
]

RETURNS = [
    ("return-kinded", "return k_regexec(&re, s);", True),
    ("return-cast-kinded", "return (int) k_regexec(&re, s);", True),
    ("return-ne-kinded", "return k_regexec(&re, s) != 1;", True),
    ("return-peek", "return k_peek(&re);", False),
    ("return-cast-peek", "return (int) k_peek(&re);", False),
    ("return-member", "return (int) re.used;", False),
    ("return-take-member", "return k_take(re.used);", False),
    ("return-stmt-expr-peek", "return ({ k_peek(&re); });", False),
    ("return-comma-peek", "return (t = 0, k_peek(&re));", False),
]

DECLS = [
    ("decl-kinded", "int d = k_regexec(&re, s);", True),
    ("decl-ne-kinded", "int d = k_regexec(&re, s) != 1;", True),
    ("decl-peek", "int d = k_peek(&re);", False),
    ("decl-cast-peek", "int d = (int) k_peek(&re);", False),
    ("decl-member", "unsigned long d = re.used;", False),
    ("decl-copy", "regex_t d = re;", False),
    ("decl-address", "void *d = &re;", False),
    ("decl-take-member", "int d = k_take(re.used);", False),
]


def regex_lines():
    out = []
    for label, stmt, kinded in SHAPES + DECLS:
        out.append((f"regex/{label}/pre", f"{PRE}{stmt} k_regfree(&re); return 0; }}", kinded))
        out.append((f"regex/{label}/post", f"{PRE}k_regfree(&re); {stmt} return 0; }}", kinded))
    for label, stmt, kinded in RETURNS:
        # pre: return before free leaks; so free then... pre = return value
        # computed while live is impossible without leaking; post is the probe.
        out.append((f"regex/{label}/post", f"{PRE}k_regfree(&re); {stmt} }}", kinded))
    return out


THREAD_PRE = "void w(void) { pthread_key_t key; int r; void *v; r = pthread_key_create(&key, 0); if (r) return; "
STATIC_PRE = "pthread_key_t key; void w(void) { void *v; pthread_key_create(&key, 0); "
THREAD_SHAPES = [
    ("void-get", "(void) pthread_getspecific(key);"),
    ("get", "pthread_getspecific(key);"),
    ("assign-get", "v = pthread_getspecific(key);"),
    ("comma-get", "r = 0, pthread_getspecific(key);"),
    ("stmt-expr-get", "v = ({ pthread_getspecific(key); });"),
    ("void-set", "(void) pthread_setspecific(key, 0);"),
]


def thread_lines():
    out = []
    for label, stmt in THREAD_SHAPES:
        out.append((f"thread/auto/{label}/post",
                    f"{THREAD_PRE}pthread_key_delete(key); {stmt} }}", False))
        out.append((f"thread/static/{label}",
                    f"{STATIC_PRE}{stmt} }}", False))
    return out


if __name__ == "__main__":
    with open(sys.argv[1], "w") as f:
        for label, body, kinded in regex_lines():
            f.write(f"{label}{'/kinded' if kinded else ''}\tregex\t{body}\n")
        for label, body, kinded in thread_lines():
            f.write(f"{label}\tthread\t{body}\n")
