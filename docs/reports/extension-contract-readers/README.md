# Extension contract readers (EXT-01 delegated track)

Base: `64ddac3f60fe9b848dd789e895d67b809cce93e2`.

## Prediction before implementation

Compatibility-only acceptance should leave the existing contract corpus and
published-provider inputs byte-identical. I predict zero changes to pinned
contract counts, source identities, lock contents, verdicts, and emitted C for
existing `.qcore` inputs. New `.qv` temporary copies should load in contract
mode and retain their explicitly spelled path identities. A `.qv` file in a
runtime/library mode and a `.qr` file in contract mode should refuse. When
same-stem `.qcore` and `.qv` entries coexist in a discovery scope, the reader
should refuse rather than prefer either spelling. Explicit references and pins
should never fall back to the other extension when the named file is missing.

The reader will warn on legacy `.qcore` loads, including frozen archive inputs,
with path, parsed mode, and `.qv` recommendation. The warning is informational
and should not move a verdict. I expect no Lean change because filename
classification does not alter source semantics or proof obligations.
