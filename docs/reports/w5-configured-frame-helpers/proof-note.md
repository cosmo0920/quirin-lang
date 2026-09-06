# Generic frame-helper proof boundary

The existing Boolean statement evaluator treats frame writes as ordered metadata
writes with no effect on the selected Boolean state. Generalizing record and
field names preserves that evaluator. Ten additional theorems in
`proof/QCore/ChunkSource.lean`, audited by `proof/AuditCycle19.lean`, make its
noninterference premise explicit.

The generic field store is indexed by object identity and field name. A checked,
nonempty helper write list must be disjoint from the complete modeled-field
inventory for the selected record. Accepted writes preserve every modeled field
and every unrelated object. An attempted write to another modeled field cannot
be treated as metadata. Literal/negated-literal expressions are independent of
the Boolean prestate; their ordered frame program retains every write while
preserving Boolean values, Boolean-write traces and branch traces.

These are proofs about the checked finite model. The complete modeled-field
inventory, source import, constant i32 representability, direct borrowed parameter
identity, compiler/build provenance and concrete C layout are correspondence
premises. The proofs do not establish arbitrary source or linker behavior.

## Cross-unit layout remains a premise

The importer compares the declared ordered `(field name, type spelling)` vector.
That equality is not a proof of recursively equal C layouts. Both supplied
fixtures have the same outer declaration:

```c
struct sample { struct nested prefix; int active; int metadata; };
```

The helper fixture defines two integers inside `nested`; the caller fixture
contains one. Clang reports metadata at byte 12 with outer size 16 in the helper,
but at byte 8 with outer size 12 in the caller. The outer field/type vectors are
identical. This is a demonstration of the vector's logical limit, not an
end-to-end public-import acceptance probe or a defect claim about the pinned
production corpus.

Reproduce the archived `layout-premise/*.log` records with:

```sh
clang -Xclang -fdump-record-layouts-complete -fsyntax-only layout-premise/helper.c
clang -Xclang -fdump-record-layouts-complete -fsyntax-only layout-premise/caller.c
```

Exact declared-vector matching remains required. Actual cross-unit layout and
linker binding remain outer-audit premises; this increment does not add a
recursive ABI proof.
