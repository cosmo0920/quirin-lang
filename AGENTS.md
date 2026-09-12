# QCore-C agent briefing

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

Compressed project state for LLM agents; this file is the source of truth for
agents. Humans should read [README.md](README.md) for the summary and
[DEVELOPMENT.md](DEVELOPMENT.md) for implementation and status detail —
repository layout, cycle records, per-increment evidence narratives and the
full build/validation instructions.

**Sync rule (steady state since card DOCS-COMPACT, 2026-09-10).** Status
*history* lives in `DEVELOPMENT.md`'s "Increment records" — one paragraph per
increment, nowhere else. This file carries only the compact ledger, the hard
invariants, the standing subsystem constraints, the per-change gate and the
entry points. A new increment therefore adds **one** paragraph to
`DEVELOPMENT.md`, updates this file's headline numbers (and, if it lands a
standing "when touching X, do not Y" rule, adds it to the constraints section
below), and leaves `README.md` alone. Do not reintroduce per-increment
chronicles here or in `CLAUDE.md`. A recurring compaction pass over these three
files runs every 2–3 waves; the cadence rule is in `CLAUDE.md`.

## What this project is

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency, lowering to C11.
It also ships a conservative checker for *existing* C source (the
"unrestricted-C gateway"). Normative specification: `docs/QCORE_SPEC_r0_3.md`
and `docs/QCORE_CONTRACT_SPEC_r0_3.md`. Current consumer grammars are in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`; implementation acceptance is tracked
in `docs/reviews/r0_3/acceptance.json`. Revision 0.2 remains historical; broad
parser/model support is not full source-language conformance. Work proceeds in
numbered bounded "cycles" recorded under `docs/cycles/`.

## Repository map

| Path | Contents |
|---|---|
| `compiler/` | Rust library crate `qcore-compiler` (lib `qcore_compiler`, 0.4.0): parser, checker, reference semantics, C backend, gateway/host compatibility APIs, first-party providers. Still the L1/L2 grab-bag; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `crates/qcore-core/` | L1 kernel: quantitative types, ownership/error/cleanup/CFG semantics, concurrency/translation models, chunk lifecycle, generic library/container/reference-count and reactor/descriptor custody, checked task IR/reference execution and C11 task emission. Also `os_endpoint`, `chunk_integrity`, `capacity_boundary`, `mutex_invariant`. No runtime or workspace dependencies |
| `crates/qcore-frontend/` | L1 front end: `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration and source lowering. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery: total-or-reject loader, observation and declared chunk-path policy evaluation. Depends on core/frontend/schemas |
| `crates/qcore-schemas/` | L0 published schema surface (Rust API **14.0.0**): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records, `reason` category vocabulary. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization. Its three load-bearing modules: `crates/qcore-c-gateway/src/legacy_c.rs` (generic unrestricted-C checker — Clang gateway, CFG, resource abstract interpretation, certificate recheck; contracts enter via `ResourceContracts`), `crates/qcore-c-gateway/src/legacy_summary.rs` (W4 interprocedural custody summary inference: SCC-ordered derivation, validation by re-analysis, declared-contract precedence — constraint 4), `crates/qcore-c-gateway/src/typed_source.rs` (opaque typed C path import and source def/use) |
| `crates/qcore-verifier-host/` | L2 generic registry (0.4.0), source-scope resolution, bounded process transport, execution profiles, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter (0.4.0): explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection, per-unit projection and reports over W2 profiles/W1 contracts. Links no provider module |
| `crates/qcorec/` | CLI binary `quirin` (crate `qcorec`) plus its process-level tests |
| `compiler/src/` facades | `compiler/src/verifier_plugin.rs` is the compatibility facade for the host and first-party adapters; `compiler/src/verifier_plugins/{libevent,chunkio}/` are the in-process first-party providers (Libevent/Monkey, ChunkIO); `compiler/src/external_{integration,source}.rs` are compatibility facades plus Monkey-specific composition |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `providers/` | Separate six-member Cargo workspace (five `qcore-provider-*` packages plus the `conformance` harness), excluded from the core workspace. Artifact crates depend only on `qcore-schemas` and serialization; `providers/package.py` builds, verifies and archives the installable packages |
| `contract/qcore/` | Declarative `.qcore` contract corpus (**73 modules**); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `Audit*.lean` print axiom surfaces |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) — one directory per card |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking, schema 2) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. `crates/` holds nine crates; the core
Cargo workspace declares **ten** members (those nine plus `compiler`), and
`providers/` is a separate six-member workspace excluded from it.

Headline numbers at the wave-26 close. The **corpus baseline is still
`docs/reports/wave23-c-operations/artifacts/`** — no arbiter since has re-pinned
it — with the attributed AFTER runs chained beneath it:
`docs/reports/aom-destroy-siblings/artifacts/` (wave 23) and
`docs/reports/aom-binding-poison/artifacts/` (wave 25), which is still the run
the corpus rows below are read from. Wave 26 ran **no arbiter**: not one of its
five cards touched a gateway, contract, audit or provider file, so the corpus
rows are **carried** from that wave-25 measurement rather than re-measured, and
they are labelled as carried rather than presented as a fresh run. What did move
this wave is the CT row (by **mapping**, not conformance — constraint 23) and
the gate row. Every preceding corpus archive stays frozen (constraint 21):

| Measure | Value |
|---|---|
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — nothing promoted, zero unit and zero function status moves; 4,834 projected functions (150 Safe / 84 provisional Unsafe / 4,600 Unknown incl. `flb_input_chunk_create` function-level **Safe**), leak **266** (271 → 272 → 273 → 272 → 266, each deliberate; see constraint 7 — **unmoved** at the wave-25 re-measurement). Two non-pinned diagnostic totals moved at that re-measurement and are attributed leaf by leaf in `docs/reports/aom-binding-poison/README.md` §4: `unknown_alias` 4,421 → **4,344** (−77) and `unsupported` 49,781 → **49,773** (−8), 87 `Unknown`-class leaves across 26 units, 1 added and 86 removed |
| CT01–CT50 conformance | **36/50 mapped**, 14 unmapped; sanitized-native 5 of 36, proof 0 of 36 (wave 26: CT06, CT11 and CT49 mapped — a grown denominator is mapping, not conformance) |
| Contract corpus | **73 modules** |
| `qcore-schemas` | 14.0.0 · `qcore-compiler` 0.4.0 · `qcore-verifier-host` / `qcore-verification` 0.4.0 · ChunkIO provider package 0.13.0 |
| Merged gate | 4,209 workspace tests (8 ignored) · 122 provider (carried; unchanged) · 193 Python · **110 Lean audits** · both probes exit 0 · fmt/clippy clean (`docs/reports/wave26-close/gate.json`, measured at the integration tip over the five merged cards; each card's own gate is in its report) |
| Full language conformance | **false** |

Cycles 1–17 are complete at their bounded milestones; Cycle 18 is bounded
implementation complete with aggregate acceptance pending; Cycle 19 (the
unrestricted-C gateway on pinned Fluent Bit) is in progress with **raw units
unpromoted** until per-path identity imports and real observation evidence
satisfy the fail-closed gates in `docs/cycles/CYCLE_19.md`; Cycle 20 (the
verifier-plugin ecosystem) is active — protocol v1 carries Boolean assertions
only, the core **refuses all v1 `Safe` proposals** including empty scopes, and
unchecked `Unsafe` proposals downgrade to `Unknown` (`QP010`).

Where the detail lives, and the only places to look for it: per-increment
history in `DEVELOPMENT.md` → "Increment records" (every wave and card
paragraph, in order, each keeping its own checkpoint and boundaries); open
follow-ons in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`,
35 entries) plus each report's own follow-on list; per-card evidence in
`docs/reports/<card>/README.md`, which is authoritative for that card's counts,
boundaries and premises; direction in `docs/GENERALIZATION_PLAN.md` (W0–W7) and
`docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table; trust/verdict rules and
audit history in `docs/REEVALUATION_SPEC.md` and `docs/REEVALUATION_FOLLOWUP.md`;
the architecture boundary and open gates in `docs/cycles/CYCLE_20.md`.

## Hard invariants (violating these is a defect, not a style issue)

1. **Fail-closed.** Missing contracts, unmatched profiles, unsupported
   constructs, provider absence/failure/mismatch, unproven call effects →
   `Unknown`. Never optimistic fallback. Refactors must not flip any
   `Unknown` to `Safe`.
2. **Verdict contract.** Every published result carries
   `(artifact, property, semantic profile, scope, bounds, assumptions,
   evidence version)`. `Safe` only after all obligations discharge; `Unsafe`
   only for a validated reachable violation; everything else `Unknown`.
   Projection coverage is not safety coverage; unknowns stay in the
   denominator; a provisional `Unsafe` is never presented as an upstream
   defect.
3. **Parity before deletion.** A hardcoded analysis route is removed only
   after its replacement reproduces the pinned verdicts/coverage, with the
   diff archived under `docs/reports/`.
4. **Gate ordering** (`docs/REEVALUATION_SPEC.md` §9): certificate
   containment (G1) before provider expansion; Cycle 19 routes stay until
   G4 parity.
5. **Pinned expectations are intentional.** `qcore_contract_corpus.rs` pins
   per-file contract/clause counts (editing `contract/qcore` requires
   updating its tables); a test asserts the `third_party/libevent` submodule
   revision; pinned commit hashes are load-bearing identity checks.
6. **Lean parity.** Semantic changes need matching Lean definitions/theorems;
   audits must stay placeholder-free with no project axioms.

## Standing constraints when touching specific subsystems

Each rule below was landed by a card and must survive refactors. The evidence
pointer is the card that established it; read it before deviating.

**Diagnostics and codes**

1. `docs/QCORE_SPEC_r0_3.md` §L21 is the canonical per-code ownership table. A
   **semantic pass** owns a range, `qcore_frontend::frontend::source_diagnostic`
   is the authoritative pass → code map, any crate running that pass emits from
   its range, and a code may not be reused by a second pass. Codes are stable:
   a code change is semantic, and new codes go through the §L21 process (as
   `Q624`/`Q625` and `Q626`/`Q627` did). Fixing where a diagnostic is
   *attributed* (file, line, span) is not a code change. `remaining-d01`,
   `remaining-ct33-ranks`, `remaining-ct34-growth`.
2. `Q020` (task route) and `Q021` (sequential route) both refuse an unenforced
   source `contract { … }` and **must stay separate** until a source-contract
   consumer lands: the task gate also refuses an empty `contract { }` and the
   sequential gate accepts it, so unifying them would change rejection
   coverage. `source.extern-call.v1` is not that consumer — it consumes
   explicitly selected *declared* contracts. §L21, `remaining-d01`,
   `extern-call-v1`.
3. §L21.1's reason vocabulary is implemented as `qcore_schemas::reason`, but
   **do not add a category field to `DerivationIncomplete`** without
   regenerating every task digest, provider package and witness in the same
   change. An `environment-unavailable` row must never be relabeled a source
   defect, and must never contribute to a provisional `Unsafe`.
   `remaining-reason-codes`, `remaining-d01`.

**W4 custody inference and the C gateway**

4. A call into an internal C function is modeled only through a declared
   `.qcore` contract or a summary that `observations_match_contract` validated
   against a fresh analyzer run; everything else stays `Unknown`. When touching
   `crates/qcore-c-gateway/src/legacy_summary.rs`: **do not** let a summary be
   published without validation, **do not** seed a formal parameter with entry
   custody the body did not demand (that asserts non-nullness and deletes
   failure paths), and **do not** accept an assume-guarantee fixed point for a
   recursive component. Declared contracts always win; a disagreement is a
   `SummaryConflict` diagnostic, never a silent override.
   `w4-custody-inference`, `w4-custody-extensions`, `w4-gateway-crate`.
5. The typed scalar-field whitelist's last premise — the base is a name this
   function never binds to a resource identity — exists because dropping it
   turned a read through a released pointer into `Safe`. The fail-closed pin
   caught it and **was not weakened**; keep it that way. `remaining-c02`.
5a. A fixed C `sizeof`/`_Alignof` query's operand-class marker is **never a
    computed value**. Do not use normalized query tokens to construct a CFG or
    refine nullness. Scalar assignments forget old scalar facts; expression
    mutations need modeled writes rather than a `NoOp`. `wave23-c-operations`.
6. **C02-P-ALIAS-LEAK-ORDER:** `report_leaks` on intermediate iterates is
   scheduling-sensitive; a pre-pass keeps it scheduling-neutral with
   `AbstractState` byte-identical. Recorded, not repaired — do not disturb it
   incidentally. `remaining-c02-p-alias`, `remaining-borrow-liveness`,
   `remaining-hygiene-14`.
7. Leak diagnostics are **266** across the pinned 185-unit corpus. A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. It has moved exactly four times, each deliberately and each either
   in the *accusing* direction or by **deciding** a leaf.
   271 → 272 (wave 17): `flb_input_chunk_get_direct_routes` L19399,
   accusing, on a function and a unit already `Unsafe`, held provisional as
   `CONSTRUCTOR-RESIDUALS-LEAK-271`. 272 → 273 (wave 20): `input_chunk_get`
   L19849, accusing — modelling the constructor exposed a release through the
   then-uncontracted `flb_input_chunk_destroy`. 273 → 272 (wave 22): that same
   leaf decided by the faithful `flb_input_chunk_destroy` contract — a
   **withdrawal**, not a discharge, since the release refuses at the call on
   joined provenance and `input_chunk_get` goes provisional `Unsafe` →
   `Unknown`; residual `AOM-DESTROY-JOINED-PROVENANCE`. 272 → **266**
   (wave 23): the faithful **`nullable`**
   `flb_input_chunk_destroy_direct_routes` contract (`nullable` where its
   already-declared neighbour carries `nonnull`, because that body's third
   statement is `if (!routes) { return; }`) retires six leaves of
   `flb_input_chunk_get_direct_routes` — L19384, L19392, L19399, L19434,
   L19489, L19505 — again by **withdrawal-by-refusal**: at each preceding call
   site the projection cannot resolve the actual it is handed, so the site is
   marked through the unresolved-effect mechanism and `report_leaks` skips it.
   The cause is that every refused statement poisons every pointer binding in
   the state, so an acquisition's binding does not survive to its release —
   residual `AOM-SIBLINGS-BINDING-POISON`, whose repair moves those sites
   toward the *discharging* arm, a direction invariant 1 constrains. That move
   **DECIDES — withdraws** `CONSTRUCTOR-RESIDUALS-LEAK-271` at a measured cost
   of **five** standing claims, not the six wave 17 recorded: the sixth leaf,
   L19544, is on the success path where no destroy call runs, so it **stands**,
   the function keeps its leak and its provisional `Unsafe`, and the pin reads
   266 rather than 265. Nothing is promoted by any of the four: zero unit status
   moves, zero function status moves. There is **no fifth move**: at the wave-25
   re-measurement the L19399 leaf that wave 23 decided by *withdrawal* is decided
   instead by the **discharging** arm of the same
   `flb_input_chunk_destroy_direct_routes` contract — its first firing anywhere
   in the pinned corpus — which is a strictly better answer to the same question
   and changes no count, the pin still reading **266** with not one leak leaf
   added or removed. `remaining-o04-b08`, `corpus-repin`,
   `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`, `wave22-aom-destroy`,
   `wave22-corpus`, `aom-destroy-siblings` §6 and §6.1,
   `aom-binding-poison` Headline and §5.2.
8. **A declared contract must agree with the C the unit declares** — in types
   and in modes. A contract whose parameter or result ABI projections the
   analysed translation unit's own C declaration contradicts, or whose custody
   *duty* the validated W4 summary of that callee's body in this unit
   contradicts, is **withdrawn for that unit**: every call site is rewritten to
   a marked uncontracted callee (`mode-parameter-contradiction` /
   `mode-result-contradiction` for the mode arm) and the name is removed from
   the environment the derivation seeds. Both ABI sides are read from what a
   certificate already holds (the preprocessed token stream; the environment's
   retained texts), never from the typed Clang AST, so W4 derivation, W4
   validation and certificate rechecking reproduce the decision — do not move
   either reader. A withdrawal is monotone in the refusing direction but **can
   turn a `Safe` unit into a provisional `Unsafe`** when the withdrawn
   declaration was discharging an acquisition; that is `Unknown`-class evidence
   about the declaration, never an upstream defect claim (invariant 2). The
   derived summary is **never installed in the declaration's place** — an
   inferred summary can be weaker than the declaration it convicts. The mode
   gate takes the *duty* inequality
   (`Retained`/`Transferred`/`Delivered`/`Restored`; `Absent`/`Produced`) and
   deliberately does **not** withdraw on a refinement inside one duty:
   nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
   parameter keep `SummaryConflict` as their only report, because every
   derivable comparison in the pinned corpus is a nullability refinement and
   withdrawing on inequality would withdraw a release contract in every unit
   including `rd_free`/`mk_mem_free`/`flb_free`'s headers. Absence of evidence
   is not evidence: nothing is withdrawn on a body this unit does not define or
   W4 derives no summary for. The scalar projection assumes the LP64 model of
   the audited targets and the reader has no target (`LOADER-ABI-LP64`).
   `remaining-loader-abi`, `remaining-loader-abi-modes`.
9. **A fixture prelude is not a production declaration.** A fixture result is
   a result *for that fixture's declarations*: card O04-B08's flagship
   aggregate store was measured never to reach production, because the real
   member is `void *` and the reduced prelude's is `struct cio_chunk *`. When
   a card publishes a per-function count, say which declarations it rests on
   (`NEIGHBOURS-FIXTURE-PROXY`). `remaining-alias-aggregate`,
   `remaining-neighbours`.
10. **A store into an object that outlives the call is refused, not escaped.**
    Publishing a tracked identity into a file-scope object or a `static` local
    withdraws both directions and retires the residual leak through the
    unresolved-effect mechanism. The escape model was measured to **promote**
    the publish-without-release body from `Unsafe` to `Safe`, which invariant 1
    forbids; do not reintroduce it. A null store and a scalar store stay
    admitted on stated grounds, and the decision reads no per-site status and no
    provenance (one exclusion, a declared *scalar* source, was added by
    measurement). **The borrowed and `Out` admissions are withdrawn** (wave 17):
    a body publishing a *borrowed* formal no longer certifies, because
    `Borrowed` asserts the caller's obligations are unchanged and a publication
    falsifies exactly that — the launder it enabled crossed the **summary**
    boundary, so W4 published `Borrowed` for the helper and the helper's caller
    certified too. A declared `Out` slot naming such a destination is refused
    before the call's effects apply, being the `Acquire` arm seen from the
    callee's side. The indirect (pointer-base) shape carried no false `Safe`, so
    its residual *violation* claim is withdrawn rather than strengthened, each
    statement keeping the refusal it already had. `remaining-ctwin-publish`,
    `remaining-ctwin-residuals`.
10a. **A refused statement invalidates only the bindings it could have
    written.** The blanket poison — every refused statement setting `unknown`
    and `may_be_null` on every binding in the state, including bindings its own
    text cannot have written — is bounded, and only where the reader positively
    recognised all three premises: a plain name, an automatic object of this
    body whose address is never taken, and no write position for that name in
    the refused text. The allow-list is built **positively**, so an
    unrecognised place is always poisoned, and every refusal that does not stand
    for one complete statement — every condition, every control-flow refusal —
    **keeps the blanket poison** (`AOM-POISON-CONDITION-FRAME`). The refusal
    itself is never weakened: the `Unsupported` diagnostic, the `unmodeled` path
    flag and the `Unknown` function status are untouched, which is what makes it
    impossible for this rule to promote anything — measured as zero unit and
    zero function status moves, and a leak pin that does not move. The frame
    premise is the ordinary C object model under the **no-undefined-behaviour**
    reading (`AOM-POISON-NO-UB`), and the declaration reader
    (`publication::body_declarations`) over-approximates in the direction that
    *reduces* refusal, so the file-scope exclusion is applied unconditionally
    here rather than with `publication::destinations`' shadowing exemption
    (`AOM-POISON-DECLARATION-READER`). Do not widen `may_write_names` without
    measuring: sparing a binding can **unmask** a stale scalar fact a `NoOp`
    mutation left behind (`AOM-POISON-NOOP-INCREMENT`, constraint 5a's already
    recorded hazard). `aom-binding-poison`.

**Compile-task identity and source transport**

11. **Do not normalize the spelled input path** (that changes `__FILE__`), **do
    not** let a declared identity survive onto derived text, and **do not** move
    the pinned Cycle 19 compatibility route off `HistoricalStdin` without
    re-measuring its 185-unit artifacts. `CompileTaskIdentity` binds this
    transport as `source_input` — keep it there, or a standard-input run and a
    declared-file run record the same identity for different preprocessing.
    `remaining-g02`, `remaining-g01`.
12. A `#line` directive in analysed source must not be able to move the audited
    scope; attribution is by include-depth flags no directive can move, and
    `Safe` requires full analysed coverage at the certificate gate.
    `remaining-e01-tail`.

**Concurrency source routes**

13. Concurrency is opt-in: a file declares `import std.concurrency;`, the
    context is unavailable by default, and imports do not leak across
    compilation inputs (`Q014`/`Q015` refuse before checking each file). The
    task profile keeps its separate `concurrency checked;` marker.
    `core-concurrency-prelude`.
14. New keywords are a recorded compatibility hazard and must be checked
    against every fixture: T03-F added five reserved words, T08-EPOCH-SRC eight
    (with `work` renamed `effort` rather than editing the pinned golden
    surface), `source.extern-call.v1` added `succeeds`, STD-MODULES added eight
    (`print`, `log`, `flush`, `dec`, `hex`, `info`, `warn`, `fatal`), and the
    quiet-emission amendment added `trace`, `debug`, and the explicit
    system-trace `log_level` declaration after a full fixture
    census. The already-reserved `error` keyword is accepted contextually as a
    logger level only after `log`; it was not removed from the global keyword
    set. EXTERN-MODES deliberately added **none**:
    `move NAME` spells all four departing modes and the declaration decides
    which ran. `remaining-t03-f`, `remaining-t08-epoch-src`, `extern-call-v1`,
    `wave17-std-modules`, `remaining-extern-modes`. `NATIVE-LIBRARY-SOURCE-BINDING`
    also added **none**: `extern library NAME;` reuses `extern` and `library`,
    claiming only the two-word sequence at declaration position, which was a
    `Q000` parse failure before. Its census found the two whole-token-stream
    route selectors — `module_imports::concurrency_span` and
    `source_tasks::has_source_tasks` — and **excluded binding spans from both**,
    so a library named `spawn`, `join`, `qcore_buffer_new`, `qcore_buffer_read`,
    `qcore_buffer_write` or `qcore_error_payload` cannot select a route from its
    own spelling. Do not remove that exclusion. `native-library-source-binding`.
    `QUIRIN-SOCKET-V1` added **ten** — `tcp`, `udp`, `bind`, `listen`, `connect`,
    `accept`, `send`, `recv`, `sendto`, `recvfrom` — each a clause word in a
    statement and never a reference, each measured load-bearing, and all ten
    census-clean across every `.qcore`/`.qc` fixture (412 files) and every Rust
    string literal (41,529). `let`, `close` and `return` are reused rather than
    respelled, which is why the release is `close NAME;`. Two candidate spellings
    were **rejected by the census** and must stay rejected: `stream` occurs as a
    contract `extern fn` parameter name (shielded only by the following `:`), and
    `socket` would name a transport the route deliberately makes explicit at
    creation. `quirin-socket-v1`. `LOOP-V1` added **one** — `for`, census-clean
    at zero code occurrences across all 418 `.qcore`/`.qc` files — and **zero**
    new tokens, `..` being two existing `Dot` tokens. **`in` is deliberately not
    reserved**: the census found it as a genuine reference inside the pinned
    contract corpus (`ensures normal resource_borrowed(in);`), exactly the shape
    that kept `len` out of the global list, so it takes that card's narrow
    contextual rule instead. Do not promote it to the global set.
    `SOCKET-EVENT-COMPOSE` and `STRING-CONCAT` each added **none**, both censuses
    run rather than assumed. `SOCKET-INET-ADDRESSES` added **none and changed no
    lexer rule**: a declared address (`bind S 127.0.0.1:8080;`,
    `connect C [::1]:8080;`) is read back out of the tokens the existing lexer
    already produces — a dotted quad is `Float Dot Float` because `127.0` is a
    float spelling, `::` is the existing atomic `ColonColon`, and `[|` still wins
    over `[` by longest match, so the `[| |]` and `..` golden surfaces are
    untouched. An address position is told from a name position by the one fact
    that **every address of this route spells a port**, so its token run carries a
    `:`; the run's tokens must be lexically adjacent. Its one name-resolution
    exclusion is **the address reader itself** — `source_socket::address_host_span`
    returns the token range of a statement's address *host*, deliberately leaving
    the **port** outside it so a port name stays a genuine reference — and the
    census measures it claiming a span in **no** `.qcore`/`.qc` file of this
    repository outside the card's own two examples. `LOOP-UNROLL-BUDGET` added
    **no reserved word at all** — a first for this family — and **one new token**,
    `#`, promoted from a lexer error on `OR-AND-LOWERING`'s precedent for `^` and
    census-clean at **zero** code occurrences across 422 `.qcore`/`.qc` files. It
    reserves nothing because a `[| … |]` clause list contributes **no referenced
    name**: `frontend::lexical_references` returns the empty set for the attribute
    statement, so `unroll` and `where` stay contextual and `let unroll = 1;` is
    still a program. That matters most for `where`, which the census found **eight**
    times in code — every one of them inside a quantitative index type whose
    readable alias is spelled `quant[| n where n := 5 |]`, the very brackets this
    attribute borrows. The disambiguation therefore takes **two** tokens (a
    statement head is `[|` *followed by* `#`) and the alias's continued behaviour is
    **re-measured**, not asserted; `#` is also still not an identifier character, so
    the scoped-name premise behind `c#0` survives. `HTTP-V1-COMPUTED-LENGTH` and
    `CT-WIRE-LANE` each added **none**, both censuses run rather than assumed, and
    `EXAMPLES-SUITE-WIRE` does not engage this constraint at all — it reserves no
    word and touches no crate source. `loop-v1`, `socket-event-compose`,
    `string-concat`, `socket-inet-addresses`, `loop-unroll-budget`,
    `http-v1-computed-length`.
15. Lock ranks and protection growth are **opt-in** disciplines: unranked
    modules stay byte-identical, and a rank is a declared position, not an
    assumption. `remaining-ct33-ranks`, `remaining-ct34-growth`.
15a. **A source line in a composed task module stands in the parent's body
    or in a spawned worker's, and the two are compared differently.** A
    parent line's position is determined because every *trace* write an
    emitted task program performs is performed by the parent's thread
    (workers buffer events privately; the parent replays them at collect),
    so a parent-only program's streams are compared **byte for byte** and
    every published `examples/threads` program is of that kind. A
    **worker**-body line is written by that worker's own thread, and placing
    one **declares** `qcore.obs-reordering.v1`: that program's streams are
    then compared window by window and emitter by emitter, where a window
    boundary is a point at which no worker is live and an emitter is the
    thread that wrote the bytes — the opposite of F02's causal attribution,
    and right for the opposite reason. The strict comparison is that
    relation's own degenerate case (`strict_iff_byte_equality`), so nothing
    was replaced. Do **not** weaken the two premises the declaration rests
    on: two emitters of one window may not share a line's bytes
    (`AmbiguousAlphabet` refuses the declaration, never a multiset
    comparison), and a worker that writes declares `io` in its **own**
    effect row. `wave21-compose-concurrency`, `wave22-obs-reordering`.
15b. **A route reads its statement tokens from the parsed program's
    post-desugaring stream, never by re-lexing source bytes,** and no reader
    may attribute a diagnostic to a synthesized token's span (synthesized
    tokens are zero-width). The last raw re-lexing site (the sequential
    driver's `statement_tokens`) made `qcore_acquire` through a pipe refuse
    while naming the hand-written form; it now uses the same span filter the
    extern, std-io and task readers have used since their own cards.
    `wave22-pipe-intrinsic-stage`.
15c. **An event program's streams are compared byte for byte, and that is a
    measurement about the reactor profile rather than a convention.**
    `source.event.v1`'s one accepted profile is single-threaded and the reactor
    refuses a cross-thread activation under it (`L04`), so the thread running a
    callback is the entry's own, every record of the derived
    `qcore.obs-reordering.v1` relation (constraint 15a) is a **barrier**, and
    the relation is strict. `observe_event_program`'s live-set walk over this
    model's regions (a dispatch turn) derives the window structure, and the
    emitters a region makes live are read from
    `ReactorProfile::threading_enabled`. A route that runs a callback
    on another thread, or gives a callback a source-level body, **supplies its
    emitters there** rather than declaring strictness or a reordering by hand.
    Do not weaken the two premises the composition rests on: a source line is
    program output and writes no trace record, and a `wake`'s payload is a value
    and never a reactor fact — which together are why every transcript this
    route published before the card is byte-identical after it.
    `wave22-std-event`, `wave22-obs-reordering`, `compose-event-lines`.
15d. **Windows `std.event` selects AF_UNIX by capability, not by compilation.**
    A usable `<afunix.h>` only exposes the candidate; the emitted shim probes
    `socket(AF_UNIX, SOCK_STREAM, 0)` on the running system. Only an explicit
    unsupported-family/type/protocol result from that probe selects the
    retained AF_INET loopback pair; other probe failures refuse. Once the
    probe succeeds, pathname, bind/listen/connect/accept or peer-validation
    failure refuses pair creation and must not be hidden by changing
    transports. Both routes stay native-tested and preserve `SOCKET`,
    `WSAPoll`, the abstract endpoint identities and the byte-identical trace.
    `windows-std-event-af-unix`.
15e. **A rendered string's observation is the view's exact bytes, and the
    emitted write is length-bounded.** `source.string.v1`'s text lines write no
    escaping, no terminator and no newline beyond the framing every line already
    carries, because a payload's `0` is an ordinary byte inside `len` and a
    scan-to-terminator write and the model **disagree** on a value the route
    admits. Do not render a string through any `printf` conversion: `%s` and
    `%.*s` both stop at the first `0`. The write lives in `io_emit`'s second
    prelude fragment, inserted only into a program that renders text — which is
    what keeps every other emitted program byte-identical — and exempted from the
    single-emission-route scan by whole-line identity exactly as the first is;
    its `fwrite` result is **checked**, because nothing in the checker rules a
    short write out. The stream, the `[LEVEL] ` prefix and the single newline are
    `std_io::line_framing` for **both** line kinds, so a rendered scalar line is
    the text line of its own digits (`rfl` in Lean); do not give either kind a
    second framing. A string takes **no** rendering selector and one written in
    front of a string refuses at `Q072`. `string-slice`, `string-slice-io`.
15f. **An HTTP message's framing is constructed, and its `Content-Length` is
    decided.** `source.http.v1` writes every CRLF itself — one after the start
    line, one after each field, one to end the head — and **no admitted component
    may carry a `CR` or an `LF`**. Do not relax that exclusion: it is what makes
    "every CRLF in an admitted head is one the compiler wrote" true, and header
    injection unexpressible rather than merely unintended. The exclusion applies to
    **composed** bytes exactly as to written ones — a composition is framing-free
    *if and only if* every piece is — so injection stays unexpressible however a
    value was built. A declared `Content-Length` must **equal** the content's byte
    count, decided at check time **because every component is check-time-known**: a
    quoted token, or (card `HTTP-V1-COMPUTED-LENGTH`) a value the entry composed
    from check-time-known pieces. Do not turn that equality into a premise, and do
    not admit a content whose length the route cannot compute. A component is
    **resolved to bytes before `admit_field` and `admit_framing` run**, which is
    what keeps both gates unedited and keeps a computed length decided rather than
    assumed; the field **name** stays a written token. The transport half is
    `source.socket.v1`'s **unedited**
    checker over a program this route lowers to, and its refusals are
    re-attributed at `Q095` carrying that checker's own error value — do not
    re-implement a custody, transport-state or determinism rule here. The
    composition's emission goes through `c_program::merge` plus one
    `c_program::assemble`; do not fork a whole-program backend. `std.http` grants
    **no** transport and **no** text: a quoted token at a message component is
    this module's syntax and constructs no `string` value
    (`STRING_SLICE_V1.md` §2), while a `let`-bound one does and takes
    `std.string`'s gate. `http-quirin-v1`.
15g. **A reactor registration is a loan, and the reactor owns no descriptor of
    `source.socket.v1`.** A composed entry's projection carries **no**
    `CreateSocketPair` and **no** `CloseSocket` — those are the operations by
    which `event_reactor` owns a number — and `watch E S …;` is an
    `OwnershipOp::BorrowShared` of `S` to `E` running to the matching
    `unwatch E;`. Do not give the reactor a descriptor of this route: the three
    rules a reader wants (a close while registered refuses, a
    `bind`/`listen`/`connect` on a watched socket refuses, a
    `send`/`recv`/`accept` on one is admitted) are the **unedited** ownership
    checker's drop, write and read under a shared loan, and "deregister before
    close" is their consequence rather than a rule. The loan's span is
    **lexical**, because the ownership IR's borrow nodes are; a crossed pair is
    refused by name and must not be approximated
    (`SOCKET-EVENT-CROSSED-SPANS`). A dispatch turn **drains nothing** — it
    observes level-triggered readiness and the entry's own transport statements
    do the work — and the four bookkeeping statements emit no call and derive no
    effect, because the reactor an emitted program runs *is* the declared
    registration set materialised as the `poll` set at each turn. `Q083`'s
    condition is "an inherited, unedited custody model refused, and the variant
    carries that model's own error value": do not split it per model.
    `socket-event-compose`.
15h. **A `concat` result is static storage the compiler computed, and a
    `borrow_utf8` view is the buffer's own.** `source.string.v1` composes text
    from **check-time-known** pieces into a new `static const` payload of the
    emitted program: no allocation, no run-time copy, no custody, and a length
    that is the **sum of the pieces' lengths** and is itself a check-time
    constant. Do not make the composition materialise into an owned buffer —
    that was measured and rejected, because it would add an allocation, a copy
    and a release to an operation whose result the compiler already holds, and
    would turn `len` of the result into a run-time quantity, which is the one
    property the operation exists to preserve (`STRING-SLICE-CONCAT-RUNTIME`).
    The `std.buffer` bridge is **borrow only**: an owned string is a new custody
    shape the `{storage, offset, len}` view has nowhere to carry, and composing
    a borrowed view already launders its bytes into static storage, so nothing
    is blocked on the freeze (`STRING-SLICE-FREEZE`). A borrow's UTF-8 validity
    is decided **statically**, out of the buffer's own write history —
    `source.buffer.v1` admits only constant bytes at constant indices — and its
    **window** runs from the borrow to the last statement reading the view or
    **anything derived from it**, a slice of a borrow being itself a borrow of
    the same buffer. Do not widen the window's computation without revisiting
    it: the interval is exact because the route is one flat statement sequence,
    and a loop construct invalidates that premise — which is why the composed
    route **refuses** a `for` today (`STRING-CONCAT-LOOP-WINDOW`). A borrow
    contributes an ordinary `Read` to the cycle-3 ownership operation list; do
    not re-implement the custody rules beside it. **Route selection changes only
    for the bridge construct** — a module holding a buffer without spelling
    `borrow_utf8` keeps `source.buffer.v1` — and a composed module is one that
    **acquires** a buffer, which is what keeps `drop s;` on a string refusing at
    `Q068` where it always did. `string-concat`.
15i. **A declared address is a check-time constant parsed once, and the two
    address families are disjoint.** `source.socket.v1`'s `bind NAME ADDRESS;`,
    `connect NAME ADDRESS;` and `sendto NAME ADDRESS …;` carry a literal IPv4
    dotted quad or a bracketed IPv6 literal and a port that is an integer literal
    or a check-time-constant binding. The address is parsed **once**, by
    `qcore_core::socket_address`, and the emitted program carries its **octets** —
    do not add an `inet_pton` (or any other text conversion) to the emitted C, or
    two parsers can read one spelling differently. **The address form admits no
    external peer**: a target is resolved against the addresses this program's own
    binds declared, and one no bind answers to refuses at `Q085`
    (`SOCKET-EXTERNAL-PEER`); an **ephemeral** port never names a socket, which is
    also what keeps every pre-existing implicit-bind program unreachable by
    address. Custody and the transport check read **one** resolution
    (`std_socket::resolve_addresses`) so a `connect` through an address and a
    `connect` through a name write the *same* listener — do not add a second
    resolution, or constraint 15g's loan could be evaded by spelling a socket
    differently. Two per-arm decisions are **explicit and must stay explicit**,
    because the platforms' defaults differ and an inherited default is a silent
    divergence: every IPv6 socket sets `IPV6_V6ONLY` to **1** on every arm (proved
    constant in `QCore.SocketInetAddresses`), and a declared bind must *succeed*
    when no live socket holds the address and *fail* when one does — which needs
    `SO_REUSEADDR` on the POSIX arms (a `TIME_WAIT` remnant is not a live holder)
    and needs it **absent** on Winsock, where it would admit a second live socket.
    `SO_REUSEPORT` is set nowhere. A host without IPv6 is **probed on the running
    system** (constraint 15d's shape): only an explicit
    unsupported-family/type/protocol result selects the distinct recorded exit
    status, every other failure refuses, and **nothing falls back to IPv4** — a
    fallback would bind a different address from the declared one. The implicit
    `bind NAME;` and `connect NAME NAME;` forms emit the statements they always
    did. `socket-inet-addresses`.

**Providers, packages and limits**

16. **P02-FLAKE:** five provider/host rows are load- or parallelism-sensitive
    and are verified in isolation, never weakened: the `RLIMIT_NPROC`
    conformance test (it counts every process of the real UID, and keeps a
    deterministic companion), `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
    (wall-clock, measured failing at load 50 and passing at load 37 on the same
    binary), the confined-fixture write test (`Io(Broken pipe)` under load),
    `real_packages_preserve_unknown_and_reject_mutations` (`ExecutableFileBusy`:
    a sibling test in the same binary executes the file it mutates), and
    `enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
    (added wave 20; the same `Io(Broken pipe)` shape as its sibling row, at
    load ~40). `remaining-p03` §9.1, `remaining-p03-f`,
    `wave17-gen-ctreasons` §4.1, `wave17-corpus`.
17. **EXTERN-F1-STORE-REACH:** the abandonment gate refuses a second live
    acquisition and was deliberately not weakened, which is why
    `resource_stored` is checked but unreachable from a single source module.
    `extern-call-f1`, `remaining-extern-err-rel`.
18. Provider executables embed their manifest template and installed manifests
    are compared field-for-field against it, so **all five packages must be
    rebuilt and verified** after any change to the template or to a
    `qcore-schemas` major version; `providers/package.py --locked` is part of
    CI. `remaining-p01-f2`, `remaining-p01-f3`, `remaining-p03-f`.
19. **E03-B deliberately supersedes E03's `QP012`** unequal-premise-set refusal
    with the premise union (differing *values* still refuse). Do not restore
    the old refusal. `remaining-e03-b`, `remaining-e03-b-o`.
20. P04's in-process counter route was deliberately **not** removed (live
    production callers), and routes P09-A..F are retained with named gaps.
    Removal requires invariant 3 parity first. `remaining-p09`,
    `remaining-p09-routes`.

**Pins, archives and measurement**

21. **Never edit a frozen archive in place.** Card T00's `coverage.json` was
    rewritten by a later lane and is restored byte-exactly (including its
    original missing final newline) by HYGIENE-16. Publish label provenance
    **beside** the frozen archive (`blocker_current`, `label_source`,
    `archived_bucket`) rather than editing it — the dual publication makes the
    edit unnecessary, and `QCore.BucketTally` proves an archive edit under lane
    coverage moves no count. `wave16-hygiene`, `wave15-ct-labels`,
    `wave11-ct33`.
22. `contracts.json` is the fifth artifact `diff_artifacts.py` itemizes
    (document `schema_version: 2`, HYGIENE-16); before that it was not opened
    at all, so waves 14 and 15 verified inventory growth by hand. Use the tool
    now, and read its **normalised and raw** module keys together: the raw keys
    carry the checkout root, and a normalisation collision demotes the section
    to raw keys rather than merging two modules into one row. The archived
    pre-itemizer diff documents of the consolidation chain are **backfilled**
    (GEN-CTREASONS): append-only, behind a `backfilled_sections` provenance
    record, with each document its own oracle — the four entries it already
    carried must reproduce byte-identically from the artifact pair it names or
    nothing is written. `schema_version` is deliberately not bumped, and the
    per-card diffs under `docs/reports/remaining-*/artifacts/` are outside that
    chain (`DIFF-TOOL-CARD-DIFFS`). `wave16-hygiene`, `wave14-corpus`,
    `wave17-gen-ctreasons`.
23. CT coverage grows by **mapping, not conformance**. A grown denominator is
    not new conformance: sanitized-native is 5 of 36 and proof 0 of 36, and a
    stage may read `executed` only where a named workspace test runs it on the
    case's own artifact. A mapping lane retires an earlier lane's
    unmapped-bucket row by declaring `retired_unmapped_reasons` in **its own**
    stage-evidence record, checked total-or-reject in both derivations — never
    by editing the earlier record (constraint 21). A lane may also **withdraw a
    row it measured** rather than loosen a test around it: wave 26 re-measured
    CT24 and CT47, and because `newly_decidable_not_mapped_reasons` is derived
    from that judgment's own key set (constraint 25), publishing labelled rows
    for them would have put three Python tests red against a matrix the lane may
    not regenerate. The rows were withdrawn, the measurements published in the
    lane's own record and in `acceptance.json`'s hand-kept reasons, and both
    cases keep card T00's DECIDABLE-NOW bucket — a re-verification reaching the
    same bucket is the record saying the label holds. `wave5-ct-integration`,
    `remaining-t00-ct-schema`, `remaining-ct34-sanitized`, `wave17-ct40`,
    `ct-wire-lane`.
24. **REASON-CODES-B3**'s sidecar `reason_code` tripwire test is flipped at the
    corpus re-pin, per its own design — not silently. `wave9-gen-reason`.
25. The acceptance-matrix generator carries a `section_ownership` block:
    sections are derived, integrator-supplied via `--gate`, or carried verbatim.
    Whole-file regeneration must not clobber later reconciliations. The CT
    `newly_decidable_not_mapped_reasons` map is **derived** as of wave 17 — its
    keys are the derived newly-decidable list's ids by construction, so a row
    for a mapped case and a row outside the published list are both structurally
    impossible; do not reintroduce a hand-carried copy. Two derived fields read
    from a **frozen** archive rather than from disk
    (`corpus.contract_modules_per_directory`, `corpus.contract_clause_pin`, both
    from card B00's ledger) and therefore describe the 59-module checkpoint
    beside a live 73 — recorded as `D02-B00-LEDGER-SNAPSHOT`, to be repaired by
    publishing provenance beside them, never by editing the archive.
    `wave9-gen-reason`, `wave17-gen-ctreasons`, `wave17-corpus`.

**Trust boundaries that are true statements, not TODOs**

26. Process providers have process-group containment plus the opt-in
    target-qualified `linux-rlimit-v1` and `linux-confined-v1` profiles — this
    is limit and two-resource-class confinement, **not an OS sandbox** (no
    seccomp, no other namespace). The recorded default remains
    `unenforced-legacy`. `remaining-p02`, `remaining-p03`, `remaining-p03-f`.
27. The mutex model supports initially-unlocked non-recursive mutexes only.
    Dynamic shapes, condition variables, try/timed locks, rwlocks and recursive
    acquisition are represented refusals, not omissions. `remaining-t02`.
28. Declared profiles, `observations_trusted`, `GuardFootprintAuthority`,
    `SynchronizedCustodyEdges`, `DeclaredFootprintShape`,
    `InvariantPredicateDeclared` and the C layout/linker equalities are
    **premises**, not proofs. A witness admission conveys no provider
    authority: acceptance terminates in the core re-evaluating its own sealed
    import, so agreement alone discharges nothing. `remaining-t02`,
    `remaining-e02`, `remaining-p04`.

## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # once per checkout
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
python3 scripts/check_lean_audits.py
```

**`--no-fail-fast` is part of the gate as of the wave-26 close (`GATE-NO-FAIL-FAST`,
a deliberate strengthening):** cargo's default stops at the first failing binary,
so one of constraint 16's five load-sensitive rows silently hides every target
after it in crate order — a wave-26 card's first run ended at
`qcore-verifier-host` and never reached the `qcorec` crate at all,
`examples_cli.rs` included, making the number that run printed a lower bound
rather than a measurement. A flake must not be able to shrink the suite it is
reported against.

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `quirin
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan). Provider changes additionally
need `providers/package.py` build + verify (constraint 18).

## Lean CI hygiene

CI builds `proof/lean-toolchain` with Lean and runs every `Audit*.lean` via
`scripts/check_lean_audits.py`, which builds from the pinned proof directory
under finite time limits and cleans up workers on timeout or cancellation.
`AuditAll.lean` checks the compiled QCore axiom closure, allowing only
`propext`, `Classical.choice` and `Quot.sound`; stale audit names and warnings
fail the gate. Both reevaluation examples run in Linux and Clang-enabled
Windows CI. Generic Lean names keep historical aliases
(`QCore.DescriptorCustody`, `EventComposition` / `checkEventComposition`,
`busy_effect_preserves_lock`), so the old Monkey module, composition accessors
and ChunkIO lock theorem remain usable. `docs/reports/w7-lean-ci/README.md`,
`docs/reports/w7-generic-lean-names/README.md`,
`docs/reports/lean-build-performance/README.md`.

## Key entry points

- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI (binary `quirin`, crate `qcorec`): `quirin check <file>`,
  `quirin emit -o out.c <file>`,
  `quirin plugins list|inspect <id>|verify <package>|enable|disable`;
  configured audits run as `quirin-audit CONFIG.json` (crate `qcore-audit`).
- Configured audits: `qcore-audit CONFIG.json`;
  `scripts/prepare_cycle19_clang_audit.py`,
  `scripts/prepare_cmetrics_clang_audit.py`,
  `scripts/prepare_libevent_core_audit.py`.
- Published examples: `examples/README.md` indexes one directory per family —
  `stdio`, `pipes`, `branching`, `extern`, `unsafe-linkage`, `threads`,
  `events`, `strings` (text reaching a stream), `sockets` (checked TCP/UDP over
  loopback, now including declared IPv4/IPv6 addresses), `http` (an executed
  HTTP/1.1 subset, beside the same bytes written by hand) and `loops`. Every
  program compiles through the real pipeline and is byte-compared against its
  reference execution by a workspace test. Since card `EXAMPLES-SUITE-WIRE` all
  **eleven** families are held to one standard by `compiler/tests/examples_suite.rs`,
  which reaches them by **directory scan** rather than by a hand-kept list — so a
  new family is covered structurally rather than by an author's diligence — while
  `examples/{strings,sockets,http}` keep their own card test files' sanitized
  native runs unedited beside it (`STRING-SLICE-IO-EXAMPLE-SUITE`,
  `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE` all closed there).
- Native threading examples: `examples/threads/README.md` (`cl.exe`);
  `build-windows.ps1 -Run` emits, builds and runs the copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
- Editor: `editors/emacs/qcore-mode.el` gives lexical `.qcore`
  highlighting; see `editors/emacs/README.md`. Highlighting does not imply
  executable or verified support.

### Diagnostics namespaces

Authority is `docs/QCORE_SPEC_r0_3.md` §L21 (ownership by semantic pass) —
this table is a summary of it, not a second allocation.

**Q-space note — `Q-NAMESPACE-DECISION`, an open decision deferred to the
icebox.** The `Q000`–`Q099` route-code space is **exhausted**: `Q077`–`Q079`
(`STRING-CONCAT`) and `Q097`–`Q099` (`LOOP-V1`) were the last two pools and are
now spent, so there is no free range left below `Q100`. The next allocation
therefore needs a §L21 namespace decision rather than a range — candidates being
to open `Q101`/`Q102`/`Q104`+ around the legacy three-digit bands, or to take a
new prefix. **By user direction (2026-09-12) that decision is in the ice box**,
on the stated ground that naming collisions happen easily; it is recorded here
as an open decision named `Q-NAMESPACE-DECISION`, **not** as a rule, and it is
integrator/spec work when it is taken up. Its operative consequence until then:
**no card allocates new `Q` codes.** A new refusal lands as a **condition
extension of an existing band**, on the `SOCKET-EVENT-COMPOSE` /
`NATIVE-LIBRARY-HEADER-DECORATION` precedent — the same condition reached by a
new route — with the widening stated in §L21 and in the card's report rather
than taken silently. Constraint 1 is unchanged: a code is still allocated to a
condition, still stable, and still may not be reused by a second pass. Wave 26
ran that consequence three times without strain — `LOOP-UNROLL-BUDGET`
(`Q097`/`Q098`), `HTTP-V1-COMPUTED-LENGTH` (`Q090`/`Q093`/`Q094`) and
`SOCKET-INET-ADDRESSES` (inside the `Q080`–`Q089` band, widening `Q084`'s
condition) — each stating its widening in both places and allocating nothing.

| Codes | Owning pass | Meaning |
|---|---|---|
| `Q000` | `NameResolution` (per file) | input file does not parse as a QCore declaration |
| `Q001`, `Q016` | `Index` | index/quant presentation; dimension/axis validity and quant-index type resolution |
| `Q010`–`Q015` | `NameResolution` | duplicates, unknown name, `Q014` unknown imported module, `Q015` concurrency without its file-local import |
| `Q020`, `Q021` | `OrdinaryTypes` | unsupported ordinary construct / task-route source-contract gate; sequential whole-program contract gate. **Route-specific, not to be unified** (constraint 2) |
| `Q022`–`Q027` | `ExternCall` | admission and bridging of `source.extern-call.v1` declared extern calls |
| `Q028`–`Q031` | `StandardIo` | `source.std-io.v1` print/log/flush admission behind `std.fmt`/`std.io`/`std.logger` |
| `Q032`–`Q035` | `UncheckedLinkage` | `source.extern-call.unsafe.v1` explicit unchecked linkage; scalars only, every pointer refused |
| `Q036`–`Q038` | `Pipe` | `\|>` pipeline desugaring shapes; `Q038` an operator token used where this revision gives it no meaning (`\|`, `&`, `^` reserved for bitwise) |
| `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067` | `NativeLibrary` | `source.native-library.v1` in four bands: manifest load, binding and agreement plus manifest-resident waivers (`Q039`–`Q042`); manifest-bound contract slices, identity stacking, withdrawal (`Q047`–`Q049`); exported-header reading and manifest/header disagreement (`Q063`–`Q064`); the source file's own `extern library NAME;` binding — resolution refusals, a foreign manifest's symbol, the publication tripwire (`Q065`–`Q067`) |
| `Q043` | `ComposedStdIo` | composed extern/std-io route's own refusals (`source.std-io.v1` × `source.extern-call.v1`) |
| `Q044`–`Q046` | `Branch` | `source.branch.v1` `if`/`else` admission: shape and arm subset, decidable condition tier (connectives, `!`, bitwise, constant bindings), custody join over survivors |
| `Q050` | `ComposedTasks` | `source.std-io-tasks.v1`: the composed task/std-io route's own refusals — a worker-body line now admissible where `qcore.obs-reordering.v1` covers it (constraint 15a) |
| `Q051`–`Q057` | `StandardBuffer` | `source.buffer.v1`: the owned extent-carrying byte buffer (`bytes`, `[i]`, `extent`, `drop`) |
| `Q058`–`Q062`, `Q074` | `Event` | `source.event.v1` in two bands: the checked event vocabulary over the reactor/descriptor custody models, `Q061` the determinism boundary (`Q058`–`Q062`); its composition with the source line vocabulary (`print`, `log`, `flush`) and that composition's one value form, `let NAME = dispatch ready E;` (`Q074`, relocated from `Q065` at integration) |
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` in three bands: the immutable UTF-8 `{ptr, len}` string view behind `std.string` — entry shape, an undecidable slice bound, a quoted token that does not decode, a bound out of range or off a code-point boundary, a string at a position it may not cross, and the declared `effects` row (`Q068`–`Q073`, relocated from `Q065`–`Q070` at integration); its composition with the output modules — `print TEXT;` and `log LEVEL TEXT;` over a quoted literal, a bound string or one inline slice — where `Q075` is the text line's shape and `Q076` a text operand naming no string this entry bound (`Q075`–`Q076`, card `STRING-SLICE-IO`, which returned `Q077`–`Q079` to the pool); and **computed text** (`Q077`–`Q079`, card `STRING-CONCAT`, reassigned to the same pass) — `concat(…)` over check-time-known pieces into a new static payload, `dec(NAME)`/`hex(NAME)` for the text of a bound integer, and `borrow_utf8(NAME)` bridging an owned `std.buffer` into an immutable view, where `Q077` is the band's shape and bound refusals **including an embedded `source.buffer.v1` statement carrying that route's own message under this pass's code**, `Q078` the borrow-exclusion window together with the inherited ownership checker's own refusal, and `Q079` the **static UTF-8 decision** on a buffer's write history |
| `Q080`–`Q089` | `StandardSocket` | `source.socket.v1`: the `std.socket` bundled module's checked TCP and UDP surface over loopback — entry shape, the operand rule, the transport state machine, the inherited custody refusal, byte/payload/count ranges, `Q085` the **determinism boundary** (a receive the program's own committed history does not satisfy), the crossing boundary, the `effects` row, the certificate recheck and the backend refusal. Band assigned by the integrator before the card, with `Q075`–`Q079` held for a sibling card. Since card `SOCKET-EVENT-COMPOSE` the row also covers the **reactor composition** — a checked socket registered with `source.event.v1`'s reactor — which allocates **no code**: a registration's refusals land on `Q080` (shape, including crossed spans), `Q081` (a name at a registration position), `Q082` (a socket in no state this program can make ready), `Q083` (whose condition is **widened** from "the inherited ownership checker refused" to "an inherited, unedited **custody model** refused", so a reactor `L00`–`L12` diagnostic lands there carrying its own value), `Q085` (a turn the committed history does not decide, including a two-member ready set) and `Q087`. Since card `SOCKET-INET-ADDRESSES` the row also covers **declared IPv4 and IPv6 addresses with ports** — `bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`, `sendto U ADDRESS […];` — which likewise allocates **no code**: a spelling that is not an address lands on `Q080`, a port naming nothing on `Q081`, a family mismatch or a second bind of one address on `Q082`, a socket at a port position on `Q086`, and a target address no bind of this program declares (or one naming a kernel-chosen port) on `Q085` as the external-peer refusal. One condition is **widened**: `Q084`'s, from a byte/payload-length/receive-count range to those and a **port** range |
| `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the `std.http` bundled module's checked HTTP/1.1 message subset — `Q090` entry and statement shape, `Q091` the start line, `Q092` header fields (including the required single `Host` and the `CR`/`LF` exclusion that makes the constructed framing unforgeable, so header injection is not expressible), `Q093` framing (whose signature condition is a declared `Content-Length` that is not the literal content's byte count), `Q094` a name at a message position that is not one, `Q095` the **inherited, unedited** `source.socket.v1` refusal carrying that checker's own error value, `Q096` an undeclared derived effect. Band pre-assigned as `Q090`–`Q099`; `Q097`–`Q099` unused and returned to the pool (`HTTP-QUIRIN-V1`). Since card `HTTP-V1-COMPUTED-LENGTH` a header value or a content may be a **computed** value — a quoted token, a name this entry bound to text, a `concat(…)` of check-time-known pieces, or `dec(NAME)`/`hex(NAME)` — which allocates **no code** and widens no gate: every component is resolved to bytes *before* `admit_field` and `admit_framing` run, so `Q093`'s equality is still decided rather than trusted (constraint 15f). The conditions widened are `Q090`'s (a malformed `concat`/`len`, a composition of fewer than two pieces, a composed payload above the 4096-byte bound, a selector without its operand, a value name rebound) and `Q094`'s (a component, piece or `len` operand naming a binding of the other kind). The field **name** stays a written token |
| `Q097`–`Q099` | `Loop` | `source.loop.v1`: the counted `for NAME in 0..BOUND { … }` loop, admitted on `source.buffer.v1` only — `Q097` the loop's **shape** (a header outside that form, a low end not the written-out `0`, a `break`/`continue`/`return` in the body, a body binding shadowing one the entry made), `Q098` the **bound** (not a check-time constant the lowering can compute, zero, negative, above `MAX_ITERATIONS`, or an entry above `MAX_UNROLLED_EVENTS` unrolled), `Q099` one iteration of the body is not the **identity** on custody. A separate pass on the `IF-STATEMENT` precedent: a control shape, not a capability of a bundled module. The band `HTTP-QUIRIN-V1` returned to the pool, now spent (`LOOP-V1`). Card `LOOP-UNROLL-BUDGET` widens two of the three conditions and allocates **no code**: `Q097` also covers the **shape** of the statement-adjacent `[| #unroll, where #bound < N |]` clause list and a list that licenses nothing (before a non-`for`, doubled, or dangling; an *unterminated* list stays a `Q000` parse failure), and `Q098` becomes the **three-layer budget** — an absolute ceiling of 4096 no declaration may raise, a default budget of 256 an unattributed `for` is held to (refusing *with the licensing clause list written into the message*), and a licence that is **checked, never trusted**, a declared `N` above the ceiling refusing before this loop's bound is read. The licence **erases from emission**: an attributed and an unattributed program of the same admitted bound emit byte-identical C |
| `Q100`, `Q103` | `Usage`, `Ownership` | usage grade; ownership |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | resource flow; cleanup lowering and reference interpretation |
| `Q300`, `Q302` | `TypedErrors`, `ErrorRegion` | typed errors; error region |
| `Q400`, `Q401`, `Q403` | driver invocation | no inputs; missing/ambiguous entry; `--entry` against a library module |
| `Q500` | `CEmission` | construct the reduced C11 CFG cannot encode |
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile |
| `Q610`–`Q627` | `Concurrency` | `CheckedMutexInvariantV1`; `Q624`/`Q625` opt-in lock ranks, `Q626`/`Q627` opt-in protection growth |
| `Q630`–`Q641` | `Concurrency` | `FlatEpochBoundV1` flat-epoch accounting (`source.epoch.bound.v1`) |
| `Q650`–`Q667` | `Concurrency` | `DisjointFootprintsV1` disjoint footprints |
| `Q670`–`Q677` | `Concurrency` | `CheckedRegionTaskCreditV1` error-region task credit |
| `Q700`–`Q703` | `PluginVerification` | scope resolution, undischarged obligations, conflicting evidence, unvalidatable provider allegation |
| `QP001`–`QP016` | provider host | protocol and evidence-firewall diagnostics |
| `<plugin-id>.<local-code>` | providers | provider-owned; exact plugin ID plus one 1–128-byte ASCII local component. Libevent uses `qcore.libevent.L00`–`L12` |

## Where to read more

Beyond the pointers in the ledger above: `docs/QCORE_MODULE_MODES.md` and
`docs/QTT_ALIGNMENT.md` for the language surface, `contract/qcore/README.md`
for contract corpus conventions, `docs/CYCLE19_BUILD_INPUTS.md` for the
persistent audit setup (it replaces ephemeral `/tmp` build paths), and
`docs/design/` for design notes.
