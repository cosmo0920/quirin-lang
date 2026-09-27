# Runtime source imports

Implementation base: `3cb923d9a5707aafe88541e6a48b2fae5db8fcdb`, after the
physical extension migration and the explicit contract-load-path CLI feature.

## Prediction recorded before implementation

Canonical standard imports become `#import std.fmt;` and retain bare standard
import compatibility. Quoted `#import "./helper";` and `#import "./helper.qr";`
resolve the same runtime module. Existing source routes, standard capabilities,
contract loading, proof obligations and native observations remain unchanged for
programs without runtime imports. No pinned C-corpus verdict or artifact change
is expected. The parent integrator owns the pinned before/after arbiter and full
standing gate; this card records focused implementation checks only.

The ordinary compile APIs resolve supplied snapshots only; an explicit disk
entry point used by the CLI loads absent dependencies. Roots are explicit in
the new bundle API. Imports contribute global declarations, not namespaces or
capabilities. Unmarked and library dependencies are admitted; explicit
executable and contract dependencies refuse. Missing, ambiguous, cyclic,
oversized or unsupported imports refuse. Source snapshots and diagnostic path
spellings remain unchanged; only derived staging blanks resolved directives
and mode headers byte-for-byte with newlines retained.

Rust controls will cover graph closure and ordering, repeated/diamond imports,
cycles, aliases, snapshot precedence, file modes, capability isolation, exact
source locations, CLI dispatch and task/epoch replay. Matching audited Lean
will describe finite bundle admission and per-file permissions. Filesystem
identity, parser extraction and Rust-to-Lean correspondence remain declared
boundaries, not consequences of the abstract model.
