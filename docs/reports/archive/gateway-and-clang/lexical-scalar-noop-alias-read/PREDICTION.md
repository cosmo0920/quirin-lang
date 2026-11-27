# LEXICAL-SCALAR-NOOP-ALIAS-READ prediction

Exact base: `b5a00e7dc23628f9b9317ab5e8f2fb5d7ec365d7`.

Before semantic implementation, reproduce the documented synthetic observation:
`struct S { int x; struct S *next; }; void subject(struct S *p, struct S *peer) { q_free(peer); p->x = 1; }`, with the trusted test-only `q_base.qv` release-nonnull declaration, empty entry annotation, and default Clang C11TranslationUnit profile. The earlier report observed Safe and scalar-store NoOp despite a same-type alias read on release. This is a candidate boundary, not yet a validated defect or upstream claim.

If reproduced, preserve the original route's source/type/evaluation guards and scalar effect, while attaching a complete alias/liveness read to every dereferenced record-pointer base. A name never directly bound by the function may still alias a released peer. A failed read must keep the function non-Safe; no route precedence shortcut may erase it. Direct record values need no pointer-liveness obligation. Reads, assignments, condition/return operands and source replay must be surveyed so the same erased access cannot survive in another position. Unsupported shapes remain refused. Rust and audited Lean must describe the same bounded rule; faithful Clang extraction and Rust–Lean correspondence remain maintained assumptions.

Expected measurements: no promised Safe gains. Soundness repair may demote Safe rows and expose diagnostics in already Unknown rows; attribute every change from full artifacts, all null/combined/runtime scenarios, and raw/normalized contract keys. Exact production impact is unknown until the three configured measurements complete. The baseline reproduction test initially pins the observed outcome and will be changed deliberately after the reproduction log is retained.
