# TASK-CONTEXT-CONTRACT-BUNDLE — task reading and pinned policy data pilot

Base: `ad6016f21355ccfa2bc0a4746cb0c4d9fcd1bee6`. This increment implements
one documentation-routing pilot and one configured contract consumer. It retains
the existing compatibility adapters and changes no `.qcore` predicate, native
source admission rule, certificate checker, CT stage or corpus verdict claim.

## Task reading

`python3 scripts/task_context.py --base <revision> --topic contracts --path
<planned-path>` produces a compact reading list. `--json` emits the full packet;
`--render` emits exact canonical text. The tool unions committed changes since
the declared base, staged/unstaged changes, both rename endpoints, deletions,
untracked files and planned paths. All matching routes apply. Unknown paths,
executable report fixtures and an empty scope select every topic. Missing or
duplicated required operation headings reject, as do Git/read failures. Topics
supply semantic intent that paths cannot infer. Recompute when scope changes.

The list carries exact ranges, hashes, numbered rule IDs and reasons. Core gate
commands are read from validation policy; provider commands come from the provider
build instructions. Follow-up results are exact-path candidates plus canonical
tracker/report pointers, not a claim to have inferred every relevant obligation.

Operations now separates active requirements into startup, integration and
maintenance sections. All **18** former top-level operation bullets have checked
operative destinations; the complete **23,862-byte** former text is preserved
byte-for-byte in [operations-before.txt](operations-before.txt), indexed in the
history guide. [preservation.json](preservation.json) records the mapping;
`python3 docs/reports/task-context-contract-bundle/preserve.py` rechecks it.
This checks preservation and quoted destinations, not natural-language equivalence.
The authority change is deliberate: current requirements are in
`docs/agents/operations.md`; historical explanations are read when changing rules
or investigating their rationale. Other topic rules remain complete and unedited.
Frozen earlier reconstruction scripts remain evidence about their own checkout;
this card does not edit them to conceal the current authority relocation.

## Reading measurements

Reproduce with `python3 docs/reports/task-context-contract-bundle/measure.py`.
[measurements.json](measurements.json) includes base identities, exact before
files, after ranges and every unchanged contract digest.

| Representative task | Before bytes | After bytes | Reduction |
|---|---:|---:|---:|
| Documentation edit | 59,463 | 23,404 | 60.64% |
| One provider contract | 144,718 | 108,659 | 24.92% |
| Gateway edit | 134,908 | 98,849 | 26.73% |
| Gateway/provider/concurrency change | 204,034 | 167,975 | 17.67% |

These are synthetic single-pass source packets. Cumulative retrieval equals
unique bytes only because each selected range is counted once. Actual repeated
reads, navigation output, injected wrappers and tool metadata affect real session
cost; no actual token billing or latency reduction is measured here. Assumed
injected AGENTS+CLAUDE **grows from 6,203 to 6,516 bytes**. The saving comes from
retrieval scoping and operations shrinking from 23,862 to 8,015 bytes, not from a
smaller root index. Rule-editing and broad/unknown tasks can still require the
whole topic set and preserved rationale. Status remains required for planning
and coverage claims; the samples are edits that make no such claims.

## Contract data pilot

`qcore_contract::bundle::ContractBundle` loads the existing provider
`contracts.json` format against an independently held manifest SHA-256 and owner.
It checks inventory version, unique keys, exact recursive file set, every file
hash, portable contained asset names, regular non-symlink assets, UTF-8,
nonempty content and bounded bytes/file count/tree depth. It exposes immutable
snapshots only after the complete bundle validates. No executable or plugin
manifest is needed, and no process is launched by this loader.

Source-contract configuration version 2 accepts a bundle selection object on a
binding or frame helper; version 1 retains string paths and refuses bundle objects.
Version 2 also accepts strings for gradual migration. A required version-refusal
test deliberately moves its unsupported-version input from 2 to 3. Existing
policy snapshot budgets, target checks and independent source/witness replay are
retained. Expected manifest identities must come from reviewed consumer input,
not be computed from an untrusted inventory just before calling the loader.

`contract/profiles/cmetrics-counter/source-contracts.json` is the migrated
consumer. Its prior `../../qcore/cmetrics/counter_reset.qcore` selected a deleted
legacy location; its audit configuration also retained the deleted shared source-policy
path, now repaired to the canonical shared tree; the new object pins the canonical CMetrics inventory. The test
copies the inventory and all four CMetrics policies to an unrelated temporary
directory, loads identical policy bytes without a provider, and confirms that
process registry construction still fails when the executable is absent.

The owner identifies the data bundle, not process capability or authorship.
Protect the directory from concurrent mutation during loading; the loader is a
filesystem snapshot check, not race-free confinement or an authenticity scheme.
Policy content still supplies declared premises, never automatically true facts.
The runtime needs only the relocated selected bundle, although its checked-in
configuration initially locates that bundle in the source tree.

## Parity and proof boundary

All **96** canonical contract sources match the base byte-for-byte, checked by
`measure.py`. A real schemas-only process fixture evaluates two functions through
both legacy path selection and bundle selection. The test compares complete task
and witness values, registry policy inventory and checked-state counts; both
functions check four states. [process-parity.json](process-parity.json) archives
the equal before/after task and witness hashes.

A second comparison runs the prepared pinned production CMetrics audit with an
explicit verified CMetrics package. [cmetrics-parity.json](cmetrics-parity.json)
records both exits 0 and identical report hashes: one checked function contract,
six source-contract Unknown functions, and an unchanged provisional Unsafe raw
unit with 2 leak, 4 unknown-alias and 32 unsupported diagnostics. The provenance
sidecar differs at exactly two paths: the configuration dependency hash (recomputed
from each selection file) and its enclosing `CompileTaskIdentity::digest` result.
Every other sidecar value is equal. Compressed report and both sidecars are
archived beside this record; `replay_cmetrics.py --output <fresh-private-dir>
--package <verified-package>` reproduces the comparison against the prepared build.
The legacy reference uses the corrected canonical string path, since the shipped
old location no longer existed. No 185-unit production replay is claimed and no
hardcoded route is deleted.

The first production attempt correctly rejected an old local package lacking the
required embedded template. The second ran both modes successfully but its initial
all-artifacts-equal assertion rejected the provenance changes above; the third
checks their exact attribution. All three attempts remain in the private logs. The final confirmation repeats
the successful comparison with this gate's freshly built and verified package;
its artifacts are the archived comparison above. All five new package artifact
identity records also match the preceding canonical-location checkpoint exactly,
including inventory, executable, manifest and archive hashes
([package-parity.json](package-parity.json)).

`QCore.ContractBundle`, registered in `AuditContractBundle.lean` and the whole
project audit, models manifest/owner/file-set agreement, all asset digest checks
and selected snapshot bytes. Theorems show manifest and file-set binding,
mismatch/invalid-structure refusal, and that selected bytes come from the snapshot;
a nonempty admitted example checks the other direction. Digests are abstract
values and parsing/filesystem validity is a stated premise. It proves neither
cryptographic collision resistance, filesystem atomicity, arbitrary predicates,
nor Rust–Lean extraction. Correspondence remains maintained by review.

## Validation

[gate.json](gate.json) is the matrix input; [validation.json](validation.json)
records exact commands, actual exits, source hashes, private log paths and the
initial attempt. All final commands below exited **0**. Clang was freshly measured
in this worktree; its recording is beside the gate.

| Check | Final result | Private log below `private/task-context-contract-bundle/` |
|---|---|---|
| Recursive submodule initialization | exit 0 | recorded in `validation.json` |
| Full core Cargo workspace | **5,043 passed / 0 failed / 11 ignored**, 269 summaries | `core-unsandboxed/cargo-test.log` |
| Core and provider fmt/clippy; both reevaluation probes | all exit 0 | `gate-1/` named command logs |
| Lean build and audits | **165 audits passed** | `lean-final-clean/audits.log` |
| Root Python suite after matrix regeneration | **336 passed** | `python-final/test.log` |
| Full provider Cargo workspace | **123 passed / 0 failed / 0 ignored**, 27 summaries | `providers-unsandboxed/test.log` |
| Provider packaging Python suite | **5 passed** | `package-python-final/test.log` |
| Contract inventories, five package builds and five verifications | all exit 0 | `gate-1/` named command logs |
| Five isolated host controls and isolated package mutation control | **6 passed**, unchanged bounds | `isolated-final/` named command logs |

The 9-test core increase is seven bundle tests and two configuration tests. The
provider increase is one real-process parity test. Eight task-context tests account
for the root Python increase. Earlier focused and process-fixture checks remain
under the same private root. The final Python recording follows the final gate-fed
matrix regeneration; it is also recorded in `validation.json`.

The initial restricted run remains at
`private/task-context-contract-bundle/gate-1/`: core Cargo exited 101 with
4,936 passed / 106 failed / 11 ignored, and provider Cargo exited 101 with
122 passed / 1 failed. Native LeakSanitizer/ptrace and confined namespace creation
were refused by that environment. Both full suites were rerun without changing
assertions, sanitizer options or confinement requirements. The initial Lean exit 1
was an import placed after the file's terminal documentation comment; moving it
beside the other imports repaired the build. The initial Python exit 1 exposed the
matrix's stale 164-audit count; canonical regeneration now records 165. The earlier
parity-print test compile failed on a missing reference argument and was fixed
before its successful replay. These attempts are not counted as green validation.

The index was regenerated with its canonical script, which also restores links
to previously unindexed reports; no historical report is rewritten. All 281
registered follow-ups, CT mappings, and acceptance flags are preserved.

## Follow-ons and discharge conditions

- **TASK-CONTEXT-PRECISE-ROUTING**: broad shared implementation paths still read
  all topic pages. Narrow only with explicit dependency rules and regressions
  covering cross-topic changes; paths alone cannot establish semantic relevance.
- **TASK-CONTEXT-TOKEN-TELEMETRY**: measure actual injected, retrieved and repeated
  context/token usage on representative sessions before claiming billed savings.
- **CONTRACT-BUNDLE-ADAPTER-EXTRACTION**: migrate remaining compile-time provider
  asset references one consumer at a time; archive pinned production parity
  before deleting any retained adapter. The gateway's shared policy embedding
  also remains. This card does not claim provider-free full-workspace tests.
- **CONTRACT-BUNDLE-SELECTION-ROLLOUT**: migrate other explicit path configurations
  after equivalent consumer tests; maintain independent inventory pins on upgrades.
  Bundle ownership and provider capability selection must remain separate.

## Integrator reconciliation list

1. Use the new AGENTS startup protocol and operation sections as current authority;
   keep the byte-preserved rationale historical. Do not silently narrow shared
   task packets or replace complete topic rules with summaries.
2. Retain the version-1 path route and compatibility adapters. The CMetrics
   inventory hash is an intentional consumer pin; update it deliberately when
   that bundle changes, with policy/corpus checks as required.
3. Publish actual gate results and distinguish sandbox-restricted attempts from
   unchanged assertions rerun outside that environment. Keep private logs by run.
4. No acceptance promotion or registered follow-up retirement is requested.
   The four report follow-ons above remain open at this pilot boundary.


Root integration dispositions: items 1–3 are applied by the routed authority,
retained v1 tests and compatibility code, pinned configuration, and validation
record above. Item 4 is noted: the matrix retains 281 named follow-ups and the
four new report follow-ons remain open; no retirement or promotion is recorded.
