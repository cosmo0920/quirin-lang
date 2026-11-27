#!/usr/bin/env python3
"""Write a temporary probe test (never committed) that prints the outcome of
every control fixture, reusing the harness of `typed_const_pointer_value.rs`.

Usage: make_probe.py WORKTREE   (writes crates/qcore-c-gateway/tests/w47cp_probe.rs;
delete it after the run)
"""
import json
import pathlib
import sys

W = pathlib.Path(sys.argv[1])
SRC = W / "crates/qcore-c-gateway/tests/typed_const_pointer_value.rs"
OUT = W / "crates/qcore-c-gateway/tests/w47cp_probe.rs"
TWICE = "typedef struct g_s g_t; typedef struct g_s { char *host; } g_t;"
CASES = {
    "P-slots-return": "struct c_leaf *w(const struct c_item *it, int i) { return it->slots[i]; }",
    "P-conf-opaque": "void *w(const struct c_box *b) { return b->conf.opaque; }",
    "P-cells-name": "char *w(const struct c_item *it) { return it->cells[0].name; }",
    "P-rows-cname": "char *w(struct c_odds *s, int i) { return s->rows[i].cname; }",
    "P-cond": "int w(const struct c_item *it, int i) { if (it->slots[i] == 0) return 1; return 0; }",
    "P-actual": "void w(const struct c_item *it, int i) { c_peek(it->slots[i]); }",
    "P-decl": "int w(const struct c_item *it, int i) { struct c_leaf *l = it->slots[i]; c_peek(l); return 0; }",
    "P-assign": "int w(const struct c_item *it, int i) { struct c_leaf *l; l = it->slots[i]; c_peek(l); return 0; }",
    "N-vname": "char *w(struct c_odds *s, int i) { return s->rows[i].vname; }",
    "N-cvname": "char *w(struct c_odds *s, int i) { return s->rows[i].cvname; }",
    "N-rname": "char *w(struct c_odds *s, int i) { return s->rows[i].rname; }",
    "N-aname": "char *w(struct c_odds *s, int i) { return s->rows[i].aname; }",
    "M-formal": "int w(struct c_box *b) { return b->pp[0]->weight; }",
    "M-tracked": "int w(int n) { struct c_box *b = c_make(n); int r = 0; if (!b) return 0; if (b->pp[0]->weight) r = 1; c_drop(b); return r; }",
    "R-bound1": "struct c_item *w(int n) { const struct c_box *b = c_make(n); if (!b) return 0; return b->item; }",
    "R-bound2": "struct c_leaf *w(int n, int i) { const struct c_box *b = c_make(n); if (!b) return 0; return b->item->slots[i]; }",
    "R-nested": "struct c_item *w(int n) { const struct c_box *b; if (!(b = c_make(n))) return 0; return b->item; }",
    "B-assigned": "int w(const struct c_box *b) { const struct c_leaf *l; if (!(l = b->opaque)) return -1; return l->weight; }",
    "B-assigned-nc": "int w(struct c_box *b) { struct c_leaf *l; if (!(l = b->opaque)) return -1; return l->weight; }",
    "B-declared": "int w(struct c_box *b) { struct c_leaf *l = b->conf.opaque; c_peek(l); return 0; }",
    "B-stmt": "int w(struct c_box *b) { struct c_leaf *l; l = b->conf.opaque; c_peek(l); return 0; }",
    "B-declared-idx": "int w(struct c_box *b, int i) { struct c_leaf *l = b->item->slots[i]; c_peek(l); return 0; }",
    "X-explicit": "int w(struct c_box *b) { struct c_leaf *l = (struct c_leaf *) b->conf.opaque; c_peek(l); return 0; }",
    "X-fnptr": "int w(struct c_box *b) { c_fn f = b->conf.opaque; return f != 0; }",
    "X-midpath": "int w(struct c_box *b) { return ((struct c_leaf *) b->conf.opaque)->weight; }",
    "X-int": "int w(struct c_box *b) { long v = (long) b->conf.opaque; return v != 0; }",
    "X-nonvoid": "int w(struct c_box *b) { struct c_leaf *l = b->item; c_peek(l); return 0; }",
    "X-midpath2": "int w(struct c_box *b) { struct c_leaf *l = ((struct c_item *) b->conf.opaque)->leaf; c_peek(l); return 0; }",
    "X-midpath3": "int w(struct c_box *b, int i) { if (((struct c_item *) b->conf.opaque)->slots[i]) return 1; return 0; }",
    "X-nonvoid2": "int w(struct c_box *b, int i) { struct c_leaf *l = b->item->cells[i].name; c_peek(l); return 0; }",
    "X-nonvoid3": "int w(struct c_box *b, int i) { struct c_leaf *l; if (!(l = b->item->cells[i].name)) return 0; return 1; }",
    "T-twice-const": (TWICE, "const char *w(const g_t *node) { return node->host; }"),
    "T-twice": (TWICE, "char *w(g_t *node) { return node->host; }"),
}

head = SRC.read_text().split("const BRANCH")[0]
lines = ["#![allow(dead_code)]", head, "#[test]", "fn probe() {",
         "    if !clang_available() {", "        return;", "    }"]
for key, value in CASES.items():
    prefix, body = value if isinstance(value, tuple) else ("", value)
    lines.append(
        f"    eprintln!(\"PROBE {key}: {{:?}}\", std::panic::catch_unwind(|| "
        f"outcome_with({json.dumps(prefix)}, {json.dumps(body)})));"
    )
lines.append("}")
OUT.write_text("\n".join(lines) + "\n")
print(OUT)
