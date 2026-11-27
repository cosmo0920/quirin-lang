# W47-ALIAS-RESIDUALS-CENSUS: frozen prediction

Frozen before any classification was run, on base `3ba72320` (local `main`,
"Merge runtime imports and explicit contract checks after complete gate").
This card changes no analyzer, contract or Lean code.

## Identity

1. **BEFORE reproduces the P3 AFTER.** The wave-47 corpus arbiter at
   `3ba72320` reproduces `docs/reports/w47-access-path-positions/artifacts/after/corpus/`
   (built at `9b2ac682`): the diagnostics byte for byte (507 Safe / 9
   provisional Unsafe / 4,318 Unknown; leak 34; `unknown_alias` 7,440;
   unsupported 36,262; Pending 0), chunk contracts and Monkey source byte for
   byte, the contracts sidecar after the checkout-root alias, the source
   effects up to path-derived digests only, and the sorted typed-path parity
   rows and contexts byte for byte. The one expected difference is the
   command: the retired compiler example is replaced by `quirin-audit pinned`
   (`b64be69e`, archived parity in `docs/reports/audit-pinned-replay-cli/`).
   Any other difference must be attributed to a named commit on
   `9b2ac682..3ba72320` before the census proceeds.
2. **The system slice reproduces the P3 AFTER** (33 / 2 / 199 unique
   functions; `unknown_alias` 985) up to the same path-derived digests.
3. **The census build is inert:** the `indirect-census` run's report
   artifacts equal the default run's.

## Census (forecast, not a claim)

4. Every `unknown_alias` leaf reads `unresolved resource identity `V``; the
   class totals will sum to 7,440 (corpus) and 985 (system), with an explicit
   `unclassified` count, expected below 2%.
5. The largest classes will be pointers whose binding the projection cannot
   settle at a dereference: a **borrowed pointer read through an access path**
   (P2/P3's typed destinations: unknown, possibly null) and a **call result**
   (an uncontracted or corpus callee's pointer), together more than half the
   leaves; most of them are dereferenced without a dominating null test.
6. Pointer-valued field stores into a caller-held aggregate (P5 territory),
   C02 peer-release composites, literal actuals and non-object names (enum
   constants copied as if pointers) will each be a minority class (each below
   10%).
7. Functions blocked by `unknown_alias` **alone** will be a few hundred at
   most (P1 counted 1,359 functions with any alias leaf at `b1744a2a`; P3
   raised the leaf count by 2,680); no single class will unblock more than
   about 150 functions alone, and the call-graph closure will add fewer than
   the round-0 count.
8. No function's verdict moves (measurement only).
