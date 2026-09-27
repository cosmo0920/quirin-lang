# Typed access statement-index optimization

Base: `4529c377`. The typed access fallback previously rebuilt the candidate
list for every queried statement and counted newlines in each candidate's
entire source prefix. It also cloned the selected function's JSON subtree while
the complete translation-unit AST remained in the source context.

The source context now decodes top-level declarations one at a time, collects
the same recursive field facts, and retains only body-bearing top-level
`FunctionDecl` nodes, including duplicate names. The decoder consumes and
validates the entire JSON document; duplicate root `inner` keys preserve the
original parser's last-value behavior. Each
selected function is shared through `Arc<Value>` across CFG rebuilds. A
`StatementIndex` walks exactly the old statement shapes once per selected
function and shares its cached candidate paths across repeated CFG builds. The
paths are grouped by byte-derived source line. Per-statement lookup still
tokenizes candidates on that line, requires an exact token match, refuses
multiple matches, and calls
the reader with the current `never_bound` predicate. Missing or out-of-range
offsets, conflicting field facts, and ambiguous function definitions still
refuse. The change makes no new admission rule. Peak memory during the Clang
subprocess and JSON stdout buffering remains unmeasured and may be unchanged.
The decoder avoids materializing the whole translation-unit `Value` in Rust;
one declaration `Value` plus retained function nodes are live during decoding.

Focused check in the isolated worktree: `cargo test -p qcore-c-gateway --lib
typed_access_path::tests` exited 0 with 12 passed, 0 failed, 594 filtered;
`cargo clippy -p qcore-c-gateway --lib -- -D warnings` exited 0.
The full standing gate and before/after corpus measurement are integration
work and are not claimed here.
