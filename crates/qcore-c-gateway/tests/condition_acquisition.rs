//! Card CG04-CONDITION-ACQUISITION: probe (temporary).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

#[test]
fn probe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p; if ((p = q_alloc()) != 0) { q_free(p); }",
        "void *p; if ((p = q_maybe_alloc()) != 0) { q_free(p); }",
        "void *p; if ((p = q_maybe_alloc()) == 0) { return; } q_free(p);",
        "void *p; if (!(p = q_maybe_alloc())) { return; } q_free(p);",
        "void *p; if ((p = q_maybe_alloc())) { q_free(p); }",
        "void *p; while ((p = q_alloc()) != 0) { q_free(p); }",
        "int fd; if ((fd = k_open(\"x\")) >= 0) { k_close(fd); }",
        "int fd; if ((fd = k_open(\"x\")) < 0) { return; } k_close(fd);",
        "void *p = q_alloc(); void *q; if ((q = q_realloc(p, 4)) == 0) { q_free(p); return; } q_free(q);",
        "void *p = q_alloc(); void *q; if ((q = q_realloc(p, 4)) == 0) { return; } q_free(q);",
        "void *p; if (flag && (p = q_maybe_alloc()) != 0) { q_free(p); }",
        "void *p; if ((p = q_maybe_alloc()) != 0 && flag) { q_free(p); }",
        "void *p; if ((p = q_maybe_alloc()) != 0 && flag) { q_free(p); } else if (p) { q_free(p); }",
        "void *p; if (flag && (p = q_alloc()) != 0) { } q_free(p);",
        "void *p; if ((p = q_maybe_alloc()) == 0) { } q_observe(p); q_free(p);",
        "if (q_alloc()) { }",
        "void *p; if ((p = q_alloc()) != 0) { }",
    ] {
        let run = g(&format!("void subject(int flag) {{ {body} }}"));
        eprintln!(
            "{body}\n  => {:?} {:?} calls={:?}",
            run.status, run.diagnostics, run.calls
        );
    }
    for condition in [
        "r_borrow_code(p) == i++",
        "r_borrow_code(p) == sizeof(int)",
        "r_borrow_code(p) == s->a",
        "!(r_borrow_code(p) == 0 || (r_borrow_code(p) == 1 && opaque()))",
        "i++ < 3 && r_borrow_code(p)",
        "i-- > 0 && flag",
        "r_borrow_code(p) && i < sizeof(struct S)",
        "i < sizeof(r_borrow_code(p)) && flag",
    ] {
        let run = g(&format!(
            "struct S {{ int a; }}; void subject(struct S *s, int flag) {{ void *p = q_alloc(); int i = 0; if ({condition}) {{ }} q_free(p); }}"
        ));
        eprintln!(
            "{condition}\n  => {:?} {:?} calls={:?}",
            run.status, run.diagnostics, run.calls
        );
    }
}
