# W2 public source-profile boundary

2026-09-07, baseline `ad92245`. The generic source-summary API accepted a
publicly constructed `LibraryProfile` without applying the structural rules
used by the JSON loader. A digest witness could therefore grant a mechanism
for a file omitted from `audited_files`. If that witness file was also absent
from the source bundle, it was silently skipped. Empty predicate descriptions,
empty substring checks and duplicate inventories could also pass this API.

`red.log.gz` records six regression tests against the original implementation:
four fail, including a direct digest grant from an unaudited file, while two
valid-input/source-drift controls pass. This is a defect in the declared
source-profile boundary; it is not evidence of an upstream library defect or
a demonstrated bypass of the separate v2 Safe-admission firewall.

`qcore_contract::profile_loader::validate_library_source_profile` now owns the
audited-file and witness validation rules. Manifest loading and
`qcore_verification::external_source::derive_library_summary_profile` call the
same function. It checks relative non-traversing file paths, lowercase SHA-256
syntax, unique file/witness names, witness membership in the audited inventory,
nonblank digest-construct descriptions, and nonempty substring needles with a
nonblank rationale. The source importer returns `InvalidProfile` before deriving
any summaries from malformed profiles, then retains its exact revision, digest
and whole-library witness checks. Missing witness files also explicitly record
`MissingFile`; they cannot silently disappear from the inventory.

The helper validates only the source-witness portion of a library profile.
Pin lookup, source roots, typed import rules and provider bindings remain the
whole-manifest loader's responsibility. It is not an alternate full manifest
validator. Declared constructs and the supplied source/revision identity remain
assumptions; hash equality does not establish library semantics or authenticate
a caller's checkout. Empty profiles with no witnesses grant no mechanisms.

The structural rules themselves are unchanged: `check-rules.py` compares all
319 rule tokens with the two original loader loops, preserving exact diagnostic
strings. The only loader behavior difference is error priority if an input has
both invalid source roots and invalid witnesses: witness validation now runs
first. Invalid inputs remain rejected. No serialized schema or protocol changes.

Six generic tests cover valid digest/substring inventory, present and absent
unaudited witness files, vacuous checks, duplicate/malformed inventories,
content/revision drift and mandatory zero-grant witness failure. A serialized
second-codebase profile verifies that JSON loading and direct derivation agree
on witness validity. All six pass after repair. No first-party test fixture is
required by these tests.

`QCore.SourceProfileWitness` adds nine executable gate theorems, printed by the
existing Cycle19 audit. Accepted grants have a checked witness and an audited,
present, digest-matching source file. Unaudited/malformed/failed witnesses,
missing files and duplicate witness names reject the entire grant set. An empty
profile grants nothing. Path/hash syntax, hashing, predicate evaluation and
revision comparison remain explicit model premises; these theorems are not a
Rust extraction/refinement or cryptographic proof.

All gates pass: 803 workspace tests, formatting, clippy, both reevaluation
probes, Lean build and 20 placeholder-free Lean audit modules. The excluded
provider workspace passes 31 tests plus formatting/clippy; all 43 Python tests
pass. `validation.json` records source hashes, commands, results and compressed
logs. The intentionally failing baseline regression is recorded separately from
the passing final gates.

The full audit uses the persistent Monkey compile database:

```sh
QCORE_SOURCE_CONTRACTS="$PWD/contract/profiles/fluent-bit/source-contracts.json" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  /tmp/qcore-source-profile-source.json /tmp/qcore-source-profile-diagnostics.json
```

All four output artifacts are archived here and byte-identical to
`docs/reports/w5-generic-driver-consolidation`. The 185-unit inventory remains
2 Safe /42 provisional Unsafe /137 Unknown plus 4 data-only units. Source-effect
replay remains 3 checked /93 Unknown; no raw unit is promoted. Contract coverage
stays at 58 modules and CT coverage at 11 mapped /39 unmapped. General provider
source binding, lifecycle composition and remaining W2–W7 requirements stay open.
