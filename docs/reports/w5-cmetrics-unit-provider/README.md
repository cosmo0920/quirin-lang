# W5 CMetrics unit-result provider

2026-09-07, baseline `cc11895`. The actual CMetrics package now computes
Boolean-field witnesses, including explicit C void results. The configured
driver imports the pinned `cmt_counter_allow_reset` body, dispatches the package
and independently checks its proposed paths against `counter_reset.qcore`.
That function checks for both Boolean input states; the other six functions
remain Unknown. Its translation unit remains provisional Unsafe in resource
projection. This is a function contract, not complete counter or unit safety.

## Implementation and ABI

`BooleanFieldResult::{I32, Unit}` is carried by the opaque source summary, the
provider input and the proposed summary. Existing I32 JSON omits the default
field, retains input version 2 and profile `qcore.c_boolean_field_contract.v2`,
and still requires an explicit integer return. Unit JSON explicitly records its
kind, uses input version 3, profile `.v3` and digest key
`qcore:boolean-field-source-input:v3`. The host checks that corresponding key;
wrong ABI/version/namespace combinations cannot rebind an imported task.

The Clang importer admits an ordinary one-record-pointer `void` function and
preserves the distinction between bare return and fallthrough. Both end a unit
path successfully; an early return stops later effects. Unsupported statements,
non-Boolean stores, extra parameters/attributes and mismatched return forms reject.
The provider independently interprets the captured statement program; it never
receives expected paths. Its structural preflight also rejects wrong return forms
in unreachable nodes. Existing depth and operation budgets remain enforced.

Unit contracts use the selected C record as their nominal borrowed parameter,
require live/exclusive/Boolean premises and constrain `field` / `old_field`.
Two prestates cover the selected field. Scalar `result` predicates and the old
companion-field vocabulary reject. The path record's numeric return slot is a
zero placeholder, not a C return value, and nonzero unit slots reject. I32
contracts retain their old nominal types, predicates and four-state product.
Unit record/field names are captured in the opaque import and retained in the
task/witness assumptions; producer changes to either are rejected.

The schemas-only interpreter moves from the ChunkIO package into
`providers/common/source_effect.rs`, used by ChunkIO 0.4.0, CMetrics 0.2.0 and
the existing conformance processes. The new checker identity is
`qcore.boolean_field_contract.v2`; configured unit dispatch requires it. Legacy
v1 checker packages remain usable for I32 inputs. The schema Rust API is 2.0.0
because new public struct fields and an exhaustive enum variant break Rust
callers; all workspace and standalone-fixture dependencies/lockfiles are updated.
Old I32 wire serialization and unrelated v1/v2 envelopes stay compatible.

The CMetrics model is data under `contract/profiles/cmetrics-counter`; no CMetrics
function name or layout table is added to the generic checker. Its explicit
registry binds the actual package. The inherited Locked model slot selects the
declared `allow_reset` field; unit predicates refer to that field directly and
do not claim a mutex or ChunkIO lock protocol.

## Reproduce the actual source audit

Build packages into a new output directory, then prepare the analysis inputs:

```sh
python3 -B providers/package.py build \
  --output build/cycle19/provider-packages-unit \
  --target-dir providers/target/conformance-artifacts
python3 -B docs/reports/w5-cmetrics-unit-provider/prepare.py
cargo run --quiet -p qcore-audit -- \
  build/cycle19/cmetrics-counter/audit.json.config
```

Package creation rejects an existing output directory. The recorded checkout is
Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; `prepare.py --checkout PATH`
selects another location for that pinned source. The profile also pins the exact
counter source digest and checks tracked modifications under `lib/cmetrics`.

The original CMake compile entry fails the generic gateway's strict option gate
at `-Wall`. Preparation declares a separate GNU11 Clang analysis build, preserving
the original definitions/include paths but omitting and recording `-Wall`, `-g`,
`-fPIC` and `-Wa,--noexecstack`. Unreviewed options stop preparation. This is not
a silent fallback or an equivalence claim about the CMake build. The database,
configuration and provenance record persist under `build/cycle19/cmetrics-counter`.
The source contract retains the exact resulting preprocessing/compiler identities.

`cmetrics-audit.json.gz` records **1 checked function /6 Unknown**, with no gateway
failure. `cmetrics-resource-only.json.gz` runs the same build without process
selection; removing only the source-contract report field makes the complete
resource reports identical. Raw status remains **1 provisional Unsafe**; this is
not an asserted upstream defect.

## Native, process and proof evidence

`native_counter.c` links the actual counter source with function/data sections
and linker garbage collection. It calls the real setter on a live stack record
from initial values 0 and 1, checks final value 1, and compares all other object
bytes. ASan/UBSan/LSan complete with exit 0 and empty stderr. These two fixed runs
do not prove general caller validity, concurrency, heap ownership or complete
counter behavior. No heap-allocation/leak count is inferred from this stack test.
`native-run.json` binds source/executable hashes, invocation and observations.

Five new workspace tests cover wire compatibility, ABI/version/digest namespaces,
void fallthrough/early return, integer fallthrough rejection, invalid scalar
policies and forged record identities/paths. Provider tests run the real CMetrics
package on a separate source fixture, reject altered or unbound requests and
false contracts, and check return-form rejection even after an early return.
All **808 workspace tests**, **45 provider tests**, **43 Python tests**, formatting,
clippy, both reevaluation probes, Lean build and **20 Lean audit modules** pass.

`QCore.BooleanFieldUnit` adds eight theorems in `QCore.ChunkSource.UnitResult`:
two-prestate coverage and contract satisfaction, successful void fallthrough,
early-return cutoff, the setter's field-only result, rejected out-of-domain
stores, metadata preservation and unchanged integer fallthrough rejection.
The model separates the completion flag from scalar results. Clang translation,
overflow handling and source/native refinement remain outside those proofs.

The executed package set and two clean-target builds reproduce all five
executable, manifest and archive hashes. `package-reproducibility.json` records
the comparison. The CMetrics package archive is included; the other packages
are reproducible using the recorded build inputs and hashes.

## Existing corpus and remaining work

The full 185-unit audit still runs with the original explicit ChunkIO 0.3.0
package selection, providing backward-compatibility evidence for its v2 input.
Source inventory and resource diagnostics are byte-identical to the baseline:
**2 Safe /42 provisional Unsafe /137 Unknown plus 4 data-only**. Both additional
source-contract reports retain every status, task, witness and coverage item;
their only differences are 74 expanded ABI explanations and
`chunk_state_sync` now reaching an unsupported `DeclStmt`. The existing source
effect split stays **3 checked /93 Unknown**. `audit-parity.json` enumerates every
changed field; the new CMetrics audit is separate from this unchanged denominator.

The corpus now has 59 contract modules (19 resource-v1 declarations, 144 owned
by other consumers); CT coverage remains 11 mapped /39 unmapped. General CMetrics
semantics, default-corpus provider composition, wider lifecycle/source binding
and removal of remaining compatibility routes are still open.
