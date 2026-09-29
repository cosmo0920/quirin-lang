# remaining-v04: corrections published beside the frozen report

The [report](README.md), its `probe_run.log` and its probes are frozen
(constraint 21); nothing in them is edited. A later card that changes what a
probe answers records the change here, with its reason.

## Card `W48-QR-SCALAR-TYPES-R3` (2026-09-29): rows 8 and 10

Rows 8 (`P08-valid-named-use`) and 10 (`P10-decl-after-use-site`) were
**accept** (`checked 1 function(s)`, exit 0). Both now **refuse** at `Q020`,
check and emit alike:

```text
P08-valid-named-use.qr:1:84: Q020: parameter `q` of `extern fn f` is `quant<n where n := 5>`, which is not a type an extern declaration can state; …
P10-decl-after-use-site.qr:1:16: Q020: parameter `q` of `extern fn f` is `quant<n where n := 5>`, which is not a type an extern declaration can state; …
```

**Why.** Both probes declare `extern fn f(q: quant<n where n := 5>) -> unit;`
beside an entry that never calls it. Before the card such a declaration's
annotations were read by nothing but name resolution and the quantity-index
elaboration, so an extern declaration no route could ever call — its parameter
is not in the §C04.1 ABI vocabulary — was admitted unread
(`W48-INERT-EXTERN-DECLARATION-TYPES`, found by card
`W48-QR-SCALAR-TYPES-R2`). Card `W48-QR-SCALAR-TYPES-R3` reads every `extern fn`
declaration of an executable or library module against that vocabulary, after
the admitting route's own passes, and refuses the rest.

**What the rows still establish.** What the two probes were written to measure
— that a named quantity axis resolves in a parameter's `quant<…>` type, before
or after its declarations — still holds: every pass up to and including the
index elaboration admits both programs, and the one refusal is the declaration
check that runs after them. Row 7 (`P07-valid-named-nouse`), which declares no
extern function, stays the accepting positive control for the named axis. The
same shape in three tests (`compiler/tests/quant_index_contracts.rs`, two tests,
and `compiler/tests/logical_connectives.rs`) was corrected the same way: each
still asserts the elaboration and now asserts that the one refusal is this
`Q020`.

Evidence: [card report](../w48-qr-scalar-types-r3/README.md), its
[fixture comparison](../w48-qr-scalar-types-r3/artifacts/fixtures-compare.txt).
