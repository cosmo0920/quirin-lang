# CONTRACT-ADOPTION-NOT-IN-FORCE prediction

This prediction is committed after the census (`3d6ede03e`) and before
anything else changes. Base: `baseline:c8c7edf21`, from the main checkout's
`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`: `eight-home`
(corpus), `fluent-bit-system` (system) and `fluent-bit-tail-pause` (slice).

**What changes.** The census (README §2) found no declaration whose reason
is mechanical. Step 2 therefore adopts nothing. This card changes no
analyzer code, no provider contract, no module list or view, no load path
and no lock. Its commits touch `docs/reports/contract-adoption-not-in-force/`
only.

**Affected rows at the base: none.** No row's contract environment, source
or binary changes.

- The rows the census names keep their baseline status and leaves. These
  are the callers of the 107 callees, including the 20 functions that the
  107 alone block in the user's run.
- The run's 107 callees reach the three profiles' rows through the same
  modules, which are unchanged.
- The tip binary is built from a tree whose code is byte-identical to the
  base. Contract-bundle digests are equal.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing:

- No callee leaves the uncontracted list: 2,790 callees and 7,264 pairs,
  of which 107 callees and 473 pairs have a provider declaration.
- No function leaves the "blocked only by uncontracted calls" set, which
  stays at 508.
- No unit changes status: 8 Safe, 7 provisional Unsafe, 167 Unknown and
  4 data-only.

Any difference is a measurement finding, not an effect of this card.
