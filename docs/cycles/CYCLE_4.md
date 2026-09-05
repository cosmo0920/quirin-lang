# Cycle 4: Resource Flow and Bounds

**Status: Done**

Cycle 4 implements Stage 4 of the prototype roadmap. A cycle is complete only
when its executable Rust behavior and corresponding Lean model both pass their
builds.

## Phase 0: warm-up prerequisite

Before cycle 4 implementation begins:

- mark cycle 3 as done;
- move the cycle 3 result out of the repository root into `docs/cycles/`;
- update repository documentation to point at cycle 4 as the active cycle.

## Executable semantics

The Rust core provides:

- multidimensional `Flow(net, peak, total)` summaries with an internal signed
  net vector;
- acquisition and release summaries with explicit cost vectors;
- checked sequential composition using
  `peak = join(p1, nonnegative(d1 + p2))`;
- conditional joins that reject different branch net changes and take
  componentwise maximum peak and total;
- reusable function summaries that compose at call sites;
- bounded-loop summaries that require a zero-net body, preserve its peak, and
  scale cumulative total by the finite bound;
- componentwise declared-bound checking;
- a separate trace interpreter for checking summaries against direct resource
  behavior.

All unsigned and signed resource arithmetic overflow is reported explicitly.

## Lean verification obligations

The Lean core models the same flow operations and proves:

1. acquisition and release have the specified net and peak summaries;
2. sequential composition adds net and total;
3. a sequential peak covers both the first peak and the shifted second peak;
4. a branch join covers each branch's peak and total and preserves an agreed
   net change;
5. a function call uses its checked summary unchanged;
6. a bounded zero-net loop has zero net, preserves body peak, and scales total;
7. an accepted static summary bounds every covered runtime observation.

These obligations are checked by `lake build`; theorem statements containing
`sorry` or `admit` do not satisfy the cycle.

## Acceptance commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd proof && lake build
```

Completion was verified with 42 passing Rust tests, including exhaustive
generated valid traces of length six, warning-free strict Clippy, and a
successful Lean build containing no proof placeholders.
