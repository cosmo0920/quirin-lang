# FB-LEDGER-MEM-BUF-LIMIT-BOUND prediction

Committed after the probe (`1dae0daee`) and before the semantic change.
Base: `287dce8ed` (main, r86).

**What changes.** A new Lean module, `QCore.MemBufLimitBound`, with its
audit. It models an input instance's resident chunk bytes, its
`mem_chunks_size` snapshot and its `mem_buf_status` across the append,
release and `set_limits` transitions, and it proves `mem_chunks_size <=
mem_buf_limit + W` under the premises the census names. The probe's fixture
tests tie the model's decision tables to route T's paths. No crate source,
gateway rule, provider contract, lock, profile or route changes.

**Can a verdict route carry the bound? No.** The bound is a numeric
invariant over a run of the engine (appends, releases, refreshes). It is not
a custody, null or lifetime property of one function, and no route reports
resource ceilings. Rule 92's ledger checker is a library consumer that no
route calls (FB-LEDGER-ROUTE-WIRING), and this card adds no checker. So no
corpus row can move in either direction: no `Unknown -> Safe` and no
`Safe -> Unknown`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current --monkey-linked` run** (Fluent Bit `046accf6`): base
binary against tip binary, each built from its own `git archive` tree. The
function rows and unit statuses should be identical, so no row moves. The
crate sources are byte-identical between base and tip (only a unit-test
module changes), so a difference would be a measurement finding, not an
effect of this card.
