# CONTRACT-CHECK-LOAD-PATH

Base `380e48f3`. This card adds an opt-in `quirin check --contract-load-path MANIFEST FILE.qv` route for an input module owned by the supplied, validated contract load path. It is a contract-assumption admission check, not a C-body safety proof or a `Safe` verdict. The existing `check` and `emit` paths keep refusing unresolved `#include` without the option.

## Pre-implementation prediction

The fixture `providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv` has `#include "quirin-provider-libc:libc/memory.qv"` and is currently refused by the ordinary structural parser at `Q000`. Its child is **owned** by the Fluent Bit provider and selected in that provider's resource view, but not itself published; `libc/memory.qv` is the published, digest-pinned parent. The explicit route should admit the child only after the manifest reader validates ownership and pins, the inclusion elaborator checks dependency/publication/cycles/capture, and the resource-v1 loader checks inherited-clause agreement. It should report accepted trusted contract declarations without an entry point, emission, or a safety verdict.

Expected deltas: one new CLI option and focused tests; zero changes to default compiler behavior, provider contracts/manifests, CT mapping or pinned verdict counts. Wrong-mode inputs, an input outside the manifest's owned modules, an unavailable or malformed manifest, a changed pinned parent, unpublished included modules and unsupported option combinations must still refuse. The selected CLI spelling remains the presented input identity; canonical paths are used only to verify membership. No Lean semantic definition should change because this route composes existing load-path and resource-v1 admission checks; if implementation changes their meaning, matching Lean will be needed before closure.

The integrator owns the full standing gate on the combined tree. This card will report focused static checks and any focused runtime checks once shared build resources are released; no gate result is predicted as observed evidence.
