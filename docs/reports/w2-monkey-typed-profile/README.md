# W2 H4: declared Monkey typed import profiles

2026-09-07. Bounded data migration, validated on integrated base `6eea8be`.
The 18 function roles, five records, seven sizeof bounds, three callback
signatures, 21 permitted callback callees and three callback effect records now
come from `libraries.monkey.typed_import` / `callback_summaries` in the Fluent Bit
profile. `declaration-parity.json` checks each prior table against the data before
removal. Existing Q5 source-contract text remains an interpreter input; its
function declaration order preserves the default wrapper report ordering.

`typed_source::import_source_functions_from_profile` accepts an explicit loaded
profile, library and role, and returns functions in lexical order. The default
Monkey adapters load the same profile through the existing resolver. Missing
profiles, missing selections, mismatched declared revisions, duplicate JSON
object keys or set entries, empty declarations, dangling signatures/effects and
callbacks without exact signatures reject. Repeated positional signature types
remain valid. The signature requirement prevents removal of a callback signature
from silently disabling its call allowlist. Wrappers without signatures retain
their existing bounded parameter grammar and opaque calls.

`MonkeyCallbackSummary::audited` now returns `Result`; it never panics or fabricates
a fallback effect. `audited_from_profile` and
`check_monkey_composition_with_profile` support explicit profile consumers.
Missing required callback effects fail composition. Successful syntax summaries
and composition certificates retain the exact declared profile assumptions,
including the manifest digest; the new summary accessor exposes those premises.
This additive source-report metadata is intentional, not discarded for parity.

The consumer distinction required by review R08/R10 remains explicit:

- Profile validation checks declaration structure and references. Callback effect
  Booleans remain trusted declarations, not observed or proved C behavior.
- Typed import checks a bounded syntax projection. Revision equality checks a
  caller-supplied assertion; direct import does not authenticate a checkout.
  Opaque calls provide no call-effect, resource-custody or safety proof.
- The existing pinned audit separately checks source/build identities. Its raw
  resource diagnostics and the specialized event-composition checks retain their
  separate scopes. Profile data does not establish refinement between them.
- Unsupported paths remain Unknown. No raw production unit is promoted and no
  whole-program, general OS, callback scheduling or durability guarantee follows.

## Evidence

`parity.json`, the compressed before/after reports and `metadata-diff.json` show:
185 TUs, **3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only**, with the
entire resource diagnostic report byte-identical to the prior gateway/W4 evidence.
Every prior source-report field is equal after removing only the 18 newly added
`profile_assumptions` maps; exact added maps and paths are archived. All functions,
paths, projection coverage and source-contract checks are unchanged. The new
Fluent Bit manifest digest is recorded as declared metadata.

Five new integration tests exercise malformed declarations, duplicate parsing,
renamed callback selection, restricted-call rejection, missing signatures,
revision/library mismatch, explicit opaque wrapper calls, missing/changed callback
effects and assumption retention. Existing source and composition tests remain.
The full standing gate passes: **521 workspace tests**, formatting, clippy with
warnings denied, both reevaluation examples, Lean build and all **18** Lean audit
files. The compiled project axiom audit permits only Lean's standard foundational
axioms; this migration adds no Lean semantics or theorem claims.

The first workspace run on the old base encountered the existing process-provider
stdin/exit race (Timeout versus Broken pipe). Its log is retained. After integrating
main's independently reviewed process fix, the complete gate passed; no process
code was changed here. Logs, return codes and artifact hashes are included.

Remaining W2/W5 work includes configured orchestration and broader provider
extraction. General source-to-model identity and truthful clause/evidence coverage
remain separate work; loading declared profile effects does not discharge them.
