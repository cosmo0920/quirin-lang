"""Rewrite docs/STATUS.md: keep untouched lines byte for byte, replace the compacted ones.

Every replaced base line range is listed in REPLACE; `checkpoint.py` moves the
same ranges verbatim into the status checkpoint, so the two cannot drift.
"""
import json
import subprocess
from pathlib import Path

BASE = '8c1f15eb'
base = subprocess.check_output(['git', 'show', f'{BASE}:docs/STATUS.md']).decode().split('\n')
CP = 'development/increments/wave-45-compaction-status.md'

REPLACE = {}
REPLACE[(3, 12)] = f"""Figures re-derived on 2026-09-27 by DOCS-COMPACT-9 from primary sources: the
Cycle 19 unit, function, diagnostic, unit-reason, uncontracted-call,
indirect-access and return-expression rows from the combined baseline
`docs/reports/w45-container-root-reach/integration/artifacts/`, which
W45-REFUTED-CONTRACT-WITHDRAWAL and W45-AUDIT-SUBCOMMAND-PLUGINS reproduced with
`verdict_bearing_identical: true`; the CT rows from the manifest; the follow-up
count from the matrix; contract counts from the tracked provider sources and
their pinned tests. Rows naming another card carry that card's census and were
not re-run here. This is the concise status authority routed by `AGENTS.md`;
reports own the underlying evidence. Update numbers from those sources, not from
an older summary. History belongs in the
[increment records](development/increments/README.md); the per-row chains this
page carried through wave 45 are in the wave-45 status checkpoints
([part 1]({CP}),
[part 2](development/increments/wave-45-compaction-status-prose.md)), not a second authority."""

REPLACE[(25, 25)] = """| Cycle 19 units | **5 Safe / 8 provisional Unsafe / 168 Unknown / 4 data-only**, 185 total; unmoved since W45-FB-BACKLOG-CUSTODY, which moved `cio_stats.c` Unknown → Safe, `mk_server/mk_http.c` Unknown → provisional Unsafe (a descriptor leak its declaration reader exposes, `W45-MK-HTTP-ERROR-FD-LEAK`, not an upstream defect claim) and `mk_core/mk_rconf.c`, `mk_server/mk_lib.c`, `mk_server/mk_vhost.c` provisional Unsafe → Unknown (their `MK-LIST-ADD-BORROWED-PUBLICATION` leaks withdrawn by its refused-call hand-on repair, not discharged); earlier moves are in each card's report | Baseline `diagnostics.json.gz`; [W45-FB-BACKLOG-CUSTODY](reports/w45-fb-backlog-custody/README.md#3-corpus-arbiter-185-units) |"""

REPLACE[(26, 26)] = """| Projected functions | **398 Safe / 9 provisional Unsafe / 4,427 Unknown**, 4,834 total; unmoved since W45-FB-BACKLOG-CUSTODY (9 read-only list traversals Unknown → Safe relative to gateway 5l's stated premises; 4 provisional Unsafe → Unknown, the `MK-LIST-ADD-BORROWED-PUBLICATION` leaks withdrawn, not discharged; `mk_http_error` Unknown → provisional Unsafe, not an upstream defect claim). Each `Safe` is relative to the premises its card names: 78 declared libc/POSIX premises (W43 libc cards), the typed scalar-access rule (W43-INDIRECT-ACCESS-GENERAL), inferred derived-result summaries (W43-RESULT-ALIAS-RETURNED), index traversal **with no bounds claim** (W43-INDEX-TRAVERSAL), the declared printf family (W43-LIBC-PRINTF-FAMILY), W4-validated glibc header bodies (W44-LIBC-RESIDUALS), the declared `memchr` (W44-C11-GENERIC-SELECTION), the declared result type and C02-P's untracked-base premise (W45-RETURN-EXPRESSION-FAMILIES), the declared release premises of gateway 5k (W45-CALLEE-SUMMARY-CASCADE), the sub-object store rule (W45-SUB-OBJECT-STORES) and gateway 5l (W45-FB-BACKLOG-CUSTODY). Soundness repairs moved `LZ4_write16`/`LZ4_write32` (W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE) and `flb_input_chunk_create` (W45-FB-RETRY-LIFETIME: its Safe rested on reading `mk_list_add` as a borrow) Safe → Unknown | Baseline `diagnostics.json.gz`; [combined baseline method](reports/w43-libc-restrict-prototypes/integration/integration-confirmation.md) |"""

REPLACE[(27, 27)] = """| Corpus diagnostics | leak **34**; unknown_alias **3,571**; double_release **0**; unsupported **41,314**; summary_conflict **1**; invalid_release / use_after_release / counted_user **0 / 0 / 0**; last moved at W45-CONTAINER-ROOT-REACH (unknown_alias 3,570 → 3,571, unsupported 41,325 → 41,314: its foreign unlink removed 11 `mk_list_del` refusals and one element release and added 2 member null tests); every earlier move is attributed leaf by leaf in its card's report | Baseline `diagnostics.json.gz`, `diagnostic_counts`; [attribution](reports/w45-container-root-reach/artifacts/attribution/corpus.json) |"""

REPLACE[(28, 28)] = """| Leak classification | of the 271 baseline leaves on the card's base `c1e69bb5`: **139** validated (a), **31** projection artifacts (b), **101** premise or approximation gaps (c), **0** unclassifiable; **252** name their object in a refused statement before the exit and are withdrawn to Unknown with the cast-return leaf, **128** of them validated (a) by reading (`REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`). These classify the card's 271 leaves, not the current 34 (row above); provisional candidates, never upstream defect claims | [Classification](reports/w44-leak-pin-audit/classification.md), [hand-on overlay](reports/w44-leak-pin-audit/handoff-classification.md), [class (a) table](reports/w44-leak-pin-audit/class-a.md) |"""

REPLACE[(29, 29)] = """| Uncontracted-call refusals | **9,157** leaves on **2,619** Unknown functions, **436** blocked only by uncontracted calls, **0** to glibc `__bswap_*`, **0** to `_Generic`; by callee body on the baseline: 3,719 same-unit bodies that do not validate (25 of them in provisional-Unsafe functions), 3,154 in another pinned unit (149 with a `Safe` home body), 1,237 header `static` bodies that do not validate, 961 with no pinned body, 86 external-linkage `inline` | `callee_census.py` of [W44-CORPUS-CALLEE-SUMMARIES](reports/w44-corpus-callee-summaries/README.md) run on the baseline with its `definition-sites.json.gz` |"""

REPLACE[(30, 30)] = """| Condition-call refusals | at W44-BRANCH-CONDITION-CALLS: **292** condition leaves lowered by desugaring (branch 8,287 → 8,018 on Unknown functions); **4,479** call-bearing condition leaves stayed refused, by blocker: indirect call argument 1,686, call-free operand 1,211, uncontracted callee 1,000, type operator 259, assignment 196, other shapes 74, recursive local callee 31, errno read 22; 0 function moves. The branch-condition refusal now stands at **7,678** leaves on 2,161 Unknown functions (70 sole) | [W44 report](reports/w44-branch-condition-calls/README.md), [residuals](reports/w44-branch-condition-calls/residuals.txt); current figure by `blocker_census.py` of [W45-SUB-OBJECT-STORES](reports/w45-sub-object-stores/scripts/blocker_census.py) on the baseline |"""

REPLACE[(31, 31)] = """| Indirect-access refusals | **16,989** leaves on **3,266** Unknown functions, sole blocker of **418**; the refused shapes are sized in the [typed scalar-access residuals](reports/w43-indirect-access-general/residuals.txt), the [chain residuals](reports/w44-indirect-chain-access/residuals.txt), the [call-argument residuals](reports/w44-indirect-call-argument-access/residuals.txt) and the [sub-object store census](reports/w45-sub-object-stores/census/residual-after.txt) | `blocker_census.py` on the baseline; [W45-SUB-OBJECT-STORES](reports/w45-sub-object-stores/README.md) |"""

REPLACE[(33, 33)] = """| Return-expression refusals | **304** leaves on **248** Unknown functions, sole blocker of **71**; since W45-RETURN-EXPRESSION-FAMILIES no cast-call or type-operator leaf remains, the rest sized in its [residuals](reports/w45-return-expression-families/census/residual-sizing.txt) (pointer-result reads, scalar expressions with a call, comma stores, bound-base or aggregate-result reads); whole-call leaves carry the call classifier's own refusal | `blocker_census.py` on the baseline; [W45 report](reports/w45-return-expression-families/README.md) |"""

REPLACE[(34, 34)] = """| Unit reasons | **167 unsupported-syntax / 1 incomplete-obligation-coverage / 17 without a reason code** | Baseline `diagnostics.json.gz`, `reason_code`; earlier fields in the [combined attribution](reports/w42-priority-integration/attribution.json) |"""

REPLACE[(42, 42)] = """| Contract corpus | **114** modules; **174** resource-v1 declarations (elaborated, pinned by `compiler/tests/contract_loader.rs`); **50** manifest-selected, 4 of them by `crate:module` reference, and exactly the eleven crates' checked resource views (W44-PROVIDER-RESOURCE-VIEW); **13** declarations inherit a libc contract | [Provider ownership](../providers/README.md#contract-ownership) (72 in five provider crates, 42 in six [`quirin-provider-*` shared crates](reports/quirin-provider-shared/README.md#crate-map)), canonical index `contract/corpus.json` (schema 2, 11 roots), [selection](../cycle19-resource-contracts.json) (schema 2), [load path](../providers/contract-load-path.json) and pinned tests; [adoption](reports/contract-inherit-adoption/README.md) |"""

REPLACE[(45, 45)] = """| Fluent Bit system slice (current build) | **8 files / 15 units: 0 Safe / 2 provisional Unsafe / 13 Unknown**; **237 functions: 28 Safe / 2 provisional Unsafe / 204 Unknown / 3 not projected**, at `046accf6` (re-pinned by the user) through `--selection contract/profiles/fluent-bit/system-selection.json`; unmoved since W45-FB-BACKLOG-CUSTODY. The obligations OR-1 (`flb_output_task_singleplex_enqueue`, under the synchronous-proxy premise) and OR-7 (`flb_output_task_flush`) are checked Fluent Bit provider contracts, each decided **refuted** by its own body (provisional Unsafe candidates, never upstream defects), and every caller call site is held Unknown (`contract-refuted-by-body`, W45-REFUTED-CONTRACT-WITHDRAWAL); OR-6 is not stated and OR-2 not reached; backlog obligations OB-1 … OB-10 are all Unknown, two custody legs held by pinned checked specifications. A separate audit scope, not counted in the Cycle 19 rows above, and no promotion | [W45-FB-BACKLOG-CUSTODY](reports/w45-fb-backlog-custody/README.md) §4–5, [W45-FB-RETRY-LIFETIME](reports/w45-fb-retry-lifetime/README.md) §4–5, [W45-REFUTED-CONTRACT-WITHDRAWAL](reports/w45-refuted-contract-withdrawal/README.md), [W45-CONTAINER-ROOT-REACH](reports/w45-container-root-reach/README.md) §6–7, [W45-FB-SYSTEM-SELECTION](reports/w45-fb-system-selection/README.md) |"""

REPLACE[(48, 48)] = """| Registered follow-ups | **513**; last moved by DOCS-COMPACT-9 (registered `W45-DOCS-PRESERVATION-VERIFIER-DECAYED` and `W45-RECONCILIATION-SUBSTRING-CLAIMS`, retired `INTEGRATION-GATE-PATH-RULE-DRIFT`, amended four STABILITY-REEVAL-5 findings it partly discharged), 512 before it after STABILITY-REEVAL-5 registered forty. Which card registered or retired which name is in its report and wave record | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |"""

REPLACE[(65, 78)] = """**Raw units remain unpromoted.** The unit Safe count is 5; each is a
function-level roll-up, not a promotion. `flb_input_chunk_create` is
function-level Unknown since W45-FB-RETRY-LIFETIME, and the production
`input_chunk_get` residual remains Unknown. Function-level safety and the chunk
**lifecycle** result are separate dimensions: no selected chunk function's
lifecycle post-state is established, and a checked field contract is not a
lifecycle result. Provisional Unsafe findings are not validated upstream
defects. Unknowns stay in the denominator. The baseline is
`docs/reports/w45-container-root-reach/integration/artifacts/`, the
combined-tree run of that integration (its predecessor
`docs/reports/w45-fb-backlog-custody/integration/artifacts` is frozen); an
arbiter must reproduce its predecessor's verdict-bearing artifacts
byte-for-byte and attribute every delta. The chain and leaf identities are
preserved in the individual reports, [wave history](development/increments/README.md),
and [gateway rules](agents/gateway.md#constraint-7)."""

REPLACE[(97, 139)] = """The latest merged gate is the
[W45-AUDIT-SUBCOMMAND-PLUGINS integration gate](reports/w45-audit-subcommand-plugins/integration/gate.json)
on merge `658717fb`: **5,995 core Rust passed, 0 failed, 11 ignored over 331
summaries (333 until `33558c67` corrected the count); 129 provider Rust passed, 0 failed, 1 ignored over 43 summaries;
207 Lean audits; 364 root Python tests; 15 package Python tests**. Core and
provider fmt and clippy, both reevaluation examples, the docs index, the
contract check, the isolated package mutation control and all five package
builds and verifications exit 0; Clang 21.1.8 was measured in that worktree.
Its arbiter reproduced the baseline (`verdict_bearing_identical: true`).

The five provider crates own **72** canonical contract modules; **42** shared or
application-level modules live in six `providers/quirin-provider-*` crates: five
per-library data crates (libc only in `quirin-provider-libc`) and
`quirin-provider-shared`, which also holds the providers' shared runtime and
bundles the five. `contract/corpus.json` indexes all **114** without duplicate
snapshots. Retained integration adapters embed the provider-owned data directly;
generic core crates retain no provider executable dependency;
package inventories bind supplied contract bytes to their executables, and
policy selection and independent checking remain explicit. Contract modules can
take a declaration's contract from another published module (`#include` /
`contract inherit NS::NAME`, [CONTRACT-INHERIT](reports/contract-inherit/README.md)),
wired into the provider crates through a generated, digest-pinned load path
([adoption](reports/contract-inherit-adoption/README.md),
[naming](reports/contract-inherit-qcore-provider-naming/README.md)). Every
function declared in more than one module is a
[checked view relation](reports/contract-view-projection-check/README.md).
Resource custody is kind-indexed: a release discharges only an obligation of its
own kind, and a file descriptor is tracked as an `int` with sentinel validity
([W43-DESCRIPTOR-RESOURCE-TRACKING](reports/w43-descriptor-resource-tracking/README.md),
Lean `QCore.KindedCustody`)."""

REPLACE[(141, 189)] = """The three broader Cycle 18 follow-ups remain registered
(`CYCLE18-CALLBACK-TASK-HANDOFF`, `CYCLE18-NATIVE-PLATFORM-MATRIX`,
`CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`). Production callback composition,
task/cancellation/event-generation coverage and current foreign-host
observations remain outstanding. Callback, CT and full-conformance coverage are
unchanged."""

OLD44 = "(two source-confirmed upstream double closes)"
NEW44 = ("(two double closes read against the Fluent Bit source: local observations, "
         "provisional candidates, not upstream defect claims)")
assert base[43].count(OLD44) == 1
REPLACE[(44, 44)] = base[43].replace(OLD44, NEW44)

# Sanity: ranges are disjoint and inside the file.
spans = sorted(REPLACE)
for (a, b), (c, d) in zip(spans, spans[1:]):
    assert b < c, (a, b, c, d)
out = []
i = 1
while i <= len(base):
    hit = next((s for s in spans if s[0] == i), None)
    if hit:
        out.append(REPLACE[hit])
        i = hit[1] + 1
    else:
        out.append(base[i - 1])
        i += 1
Path('docs/STATUS.md').write_text('\n'.join(out))
Path('docs/reports/docs-compact-9/artifacts/status-ranges.json').write_text(json.dumps([list(s) for s in spans]))
print('written', len('\n'.join(out).encode()), 'bytes')
