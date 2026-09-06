# W7 provider diagnostic attribution

The host now accepts a provider diagnostic code only when it is the selected
manifest's complete plugin ID, a dot, and one nonempty local component. The
local component is 1–128 ASCII letters, digits, underscores or hyphens. Plugin
IDs already have a 128-byte bound, so a complete code has at most 257 bytes.
Both evidence-v1 and evidence-v2 envelope checks share this rule. Invalid codes
produce host `QP007` and no certificate, before handling the proposed verdict.

Previously, the prefix-only check let `example.echo` emit
`example.echo.child.warning` even with the distinct provider
`example.echo.child` registered. Empty suffixes and embedded control characters
also passed. The preserved [original probe](original-probe.rs) exercises the
public registry. Its [before output](original-probe.log.gz) and
[fixed output](fixed-probe.log.gz) show the change. Bare host/core codes and an
unrelated provider prefix were already rejected; this increment preserves that
behavior. This was an attribution ambiguity, not a demonstrated Safe-admission
bypass. Provider message contents remain untrusted text.

The wire records, manifest shape and version constants are unchanged. Valid
codes such as `qcore.libevent.L00` remain accepted. Libevent's earlier Q700–Q712
migration is already complete and is not repeated here. The namespace derives
from `plugin_id`; this patch introduces no independent namespace selector.

Five new host regressions exercise both versions, all three proposed verdicts,
registered nested IDs, empty/dotted/control/non-ASCII suffixes, near-prefix IDs,
the 128-byte local boundary, and maximal plugin IDs. They also verify that a
valid diagnostic cannot bypass the v1 Safe refusal or v2 missing-checker gate.
Before the fix, three tests passed and the two rejection tests failed; the
[before log](regression-before.log.gz) preserves that observation.

Validation completed:

- All nine host tests passed, including the five new regressions.
- Host all-target Clippy and formatting checks passed.
- All three unchanged configured second-codebase process conformance tests
  passed, preserving real source import, provider execution and trusted v2
  certificate admission, including false-policy rejection.

Logs and source hashes are recorded in [validation.json](validation.json).
Full standing gates and any combined corpus audit are part of the parent
integration run. To rerun the targeted checks:

```sh
cargo test -p qcore-verifier-host
cargo clippy -p qcore-verifier-host --all-targets -- -D warnings
cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance --test configured_source_contract
```
