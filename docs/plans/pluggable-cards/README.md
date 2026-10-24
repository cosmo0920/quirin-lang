# Pluggable, isolated implementation cards

A plan, not a card. It proposes how to encapsulate the influence of an
analyzer change: today one implementation change ripples into the contract
pins, every route's corpus figures and every cycle's acceptance numbers, so
each card re-measures and re-pins the whole repository and all cards queue on
one gate. The plan changes no analyzer semantics by itself; every step is its
own card with its own gate and, where a route or a pin moves, its own parity
evidence.

Written 2026-10-24 from the W48 implementation round 1 (rules 32–35) and the
three Codex wave-2 cards. The [generalization plan](../../GENERALIZATION_PLAN.md)
§3 remains the crate-layer target; this plan adds one layer under the C gateway
and the tooling around cards. Its §2 invariants apply unchanged.

## 1. The problem, measured

**Implementation, contracts and corpus are one knot.**

- Contract knowledge is pinned in code. `compiler/tests/qcore_contract_corpus.rs`
  holds about 1,200 lines of per-file literals (`EXPECTED_CONTRACTS`,
  `EXPECTED_INHERITANCE`); `providers/tests/test_contract_load_path.py`
  asserts a literal resource-view count (65, then 66); the caller-frame
  membership `FRAMES` is a Rust table in `qcore-contract`, mirrored by a JSON
  file in a report. Adding one `.qv` file (rule 32's `log_caller_frame.qv`)
  required edits to three tests in two workspaces, and left `main`'s provider
  suite failing for a day because that edit sat on another branch.
- Every verdict figure is global. A rule that touches a dozen functions is
  measured as "corpus 977 → 999", a number that moves whenever any sibling
  lands. Rules 34 and 35 measured 999 on tips that did not yet have rule 32a;
  rule 32 then measured 998. Each card re-ran the whole corpus (and system and
  slice) for BASE and TIP; three cards built three BASE binaries of the same
  commit.
- Cycles bind to that global number. `docs/cycles/CYCLE_*`, the acceptance
  matrix's requirement rows and `STATUS.md`'s headline carry corpus totals, so
  an implementation change is also a records change for every cycle whose
  figures it moves, and historical reports can only be kept by declaring them
  historical.

**Every card edits the same files.** Of the last 300 commits on `main`,
`crates/qcore-c-gateway/src/legacy_c.rs` and `proof/QCore.lean` changed in at
least 300 each, `docs/agents/gateway.md` in 274 and the gateway `lib.rs` in 166.
Rules 32–35 all conflicted on exactly these, plus `docs/agents/README.md`,
`docs/STATUS.md` and the increment record.

**One gate, one lock, six cards.** The full standing gate measured 1,454 s on
the logger card: the Rust suite 606 s and the provider suite 560 s are 80 % of
it. Corpus measurements take hours. All of it serialises on one `flock`; on
2026-10-23 eight jobs were queued, the oldest for 5 h 46 min, while the host
(16 CPUs, 60 GB) ran one 7 GB audit.

**Every card rewrites the measurement.** `measure.py`, `compare.py`,
`mirror_inputs.py` and `gate_job.sh` exist in four report directories as
diverging copies (three `mirror_inputs.py` digests). The route each card must
use is folklore passed between briefs.

**Inputs are coupled to a path string.** The pinned compile databases spell the
old checkout path and the audit selects units by string comparison
(`pinned_route.rs` `plan_library`, `current.rs` `export_cross_unit_home`), so
moving the checkout broke every route (`PINNED-INPUTS-PATH-AFTER-MIGRATION`).

## 2. Goal and the invariants kept

An implementation change should have a **bounded, declared influence**: the
rows it moves, under which assumptions, on which evaluation profiles. Contracts
should be data a change consumes through the schema surface, pinned by lock
files rather than by literals in tests. A cycle should be an evaluation profile
whose figures are recomputed from per-rule contributions, or frozen, by
declaration. And a card should be a **package**: a crate, a Lean module, a rule
document, a manifest row and a report, editing no file another card edits.

Kept, without exception: fail-closed (a missing, disabled or refusing package
yields `Unknown`); no refactor flips `Unknown` to `Safe` (every move carries an
identity run); parity before deletion (a route leaves `legacy_c.rs` only after
its package reproduces the pinned verdicts byte for byte, with the diff
archived); pinned expectations stay intentional (a lock-file update is still a
deliberate, reviewed change, now diffable as data: generalization §2.6); the
verdict contract and Lean parity; the dependency direction of generalization
§2.7. The standing gate's content is unchanged; what changes is *who runs it
when* and *what a card must re-measure*.

## 3. Six seams

### A. Decouple implementation from contracts and corpus (the main issue)

- **Contract bundles with lock files.** Each provider crate's `contracts/`
  gets a generated `contracts.lock` (per-file digest, declaration and clause
  counts, inheritance edges, premise manifest), produced by
  `providers/contracts.py sync` and checked by one test: *tree equals lock*.
  `EXPECTED_CONTRACTS`, `EXPECTED_INHERITANCE` and the Python view count become
  readers of the locks, not literals. Hard invariant 5 holds: a contract change
  still requires an intentional lock update in the same commit, reviewed as a
  diff of counts and digests; what disappears is the Rust/Python edit in
  another card's territory.
- **Contract knowledge leaves code.** `FRAMES` (and any similar table that
  names declarations) becomes a declared clause in the contract itself
  (for example `requires resource_caller_frame(FRAME_ID)`), read through the
  loader like every other clause, so the rule consumes contract data instead
  of embedding it. Each such move is a card with a parity run.
- **Evaluation profiles as data.** A cycle's figures bind to a profile:
  `contract/profiles/<name>/evaluation.json` = route (inputs manifest digest,
  link map, property files), contract bundle digests, and a policy
  `frozen@<analyzer revision>` or `live`. The existing `profile.json` per
  source set is the base; the evaluation record adds the digests and the
  policy. A frozen profile's figures never move; a live one is recomputed by
  the integrator from published baselines. `docs/cycles/CYCLE_*` and the
  matrix's requirement rows name their profile and policy, so an
  implementation change can no longer silently re-date a cycle's claim.
- **Per-rule contribution ledgers.** Each rule package stores, per live
  profile, its measured contribution: rows moved (with their assumptions),
  leaves traded, rows that stay Unknown and why. It is measured from one
  binary as TIP against TIP-with-the-rule-off (seam B's switch). The profile
  total is the base plus the ordered contributions, and `STATUS.md` reports
  "rule 32: 7 rows under three premises" beside the total, which is what the
  cards already write by hand. A change to one rule re-measures that rule's
  contribution; siblings' ledgers are unchanged unless the comparison shows
  interaction, which the tool reports as a named row.
- **Impact declaration.** A card's prediction names the profiles and the rows
  or units it expects to move; the comparison tool checks the measured set
  against the declaration and lists every row outside it. Unexpected movement
  is a soundness question first, as the W48 briefs already say.

What this does not do: it does not make a contract verified (it stays a
declared assumption), and it does not shrink the denominator. Unknown rows stay
counted; a frozen profile is frozen for all rows, not for the convenient ones.

### B. Gateway rule packages (code)

- New crate `crates/qcore-gateway-rule-api` (depends on `qcore-schemas` and
  `qcore-contract` only): the query and result types a fallback or hold needs,
  the reason-key reservation, and one trait per position of
  [the route map](../../reports/gateway-analysis-path-optimization/ROUTE-MAP.md).
- One crate per rule under `crates/gateway-rules/rule-<id>-<name>/`, depending
  on the API crate (and `qcore-core` where a kernel model is read), never on
  another rule crate and never on `qcore-c-gateway`.
- A manifest `crates/gateway-rules/registry.toml`: `id`, crate, positions,
  consultation order, Lean modules, rule document. `legacy_route_table.rs`
  becomes the reader of that manifest (a generated table) with a test that the
  order equals the frozen ROUTE-MAP for every migrated entry.
- Composition is unchanged: first answer wins within a position; an absent,
  disabled or refusing package falls through; the end of the table is the
  existing refusal. The only answers are admit-with-obligations or refuse.
- **Rule switches at run time.** Every package reads
  `QCORE_GATEWAY_RULES_OFF` (comma list of ids) and refuses when listed; off
  is fail-closed by construction; `run.json` records the list. This replaces
  per-card compile-time constants (`DECAY_ARM`, `LITERAL_ARM`) and makes seam
  A's per-rule ledger a single-binary measurement.
- Migration order: rules 32a, 33, 34, 35, 36, 37 first, each with an identity
  run (switches on; corpus, system and slice reports byte-identical to `main`).
  Routes `legacy_route_table.rs` deliberately keeps inline stay in
  `legacy_c.rs` until a later card proves their parity.

### C. Proof packages (Lean)

- Each rule package owns `proof/QCore/<Module>.lean` and
  `proof/Audit<Module>.lean`, named in its registry row.
- `proof/QCore.lean` and `proof/AuditAll.lean` are generated by
  `scripts/gen_lean_roots.py`; a test asserts committed equals generated.
  Cards never edit the roots. A card builds and audits its own modules; the
  whole-project `AuditAll` runs in the integrator's gate.

### D. Record packages (docs)

- One file per rule under `docs/agents/rules/<id>.md` is the rule's single
  authority; `gateway.md`'s sections and the router rows are generated from
  them with anchors preserved, under a committed-equals-generated test.
- `integration-amendments.json` becomes a checked schema (`card`, `base`,
  `measured_tip`, `profiles[]`, `increment_paragraph`, `status_paragraph`,
  `ledger[]` with `register|amend|append|retire`, `contribution`), applied by
  `scripts/apply_amendments.py`. Rules 32–35 shipped three different shapes.

### E. Elastic gate

- **Scoped card gate.** `scripts/gate_scope.py --base <rev>` maps the diff to
  crates with `cargo metadata`, closes over reverse dependencies and runs the
  Rust suite restricted to that set (`run_test_binaries.py --packages`), fmt
  and Clippy on it, the changed Lean modules and audits, the Python suite, and
  the provider gate when `providers/` changed. It also treats fixture
  directories read by `include_str!`/`#[path]` as part of the reading crate
  (constraint CT-FIXTURE-EXTERNAL-READERS). A diff reaching
  `qcore-c-gateway`, `qcore-core`, `qcore-contract` or `qcore-schemas` gets the
  full suite. The affected set is written into the card's `gate.json`.
- **Full gate per landing**, by the integrator, unchanged in content.
- **Named slots instead of one lock.** `scripts/gate_slot.py <class> -- <cmd>`
  with classes `rust-suite` (1 slot; constraint 16), `audit` (N = min(3,
  floor(available memory / 9 GB)); two 7 GB audits side by side on this host)
  and `lean` (2). The `rust-suite` class takes every `audit` slot while it
  runs, so the suite never shares the host with an audit.

### F. Measurement as a tool

- `quirin-audit measure` (a subcommand plugin, `crates/quirin-audit-plugin-measure`,
  through the existing out-of-process firewall) replaces the copied scripts:
  `--profile <name>`, `--side <name>=<binary|rev|baseline:<sha>>`,
  `--rules-off <ids>`, `--compare <a> <b>`, `--declared <PREDICTION.md>`. Its
  comparison output is the `native-comparison` record under one schema, with
  the per-rule contribution (seam A) and the declared-versus-measured check.
- **Canonical paths.** The audit compares canonicalised paths in
  `plan_library` and `export_cross_unit_home`; the tool re-spells compile
  databases into a digest-manifested private mirror when the inputs' root
  differs from the checkout (what `mirror_inputs.py` does, once, with a test).
- **Published baselines.** The integrator's per-landing full gate also runs
  every live profile and publishes `private/baselines/<sha>/<profile>/`
  (reports, binary digest, inputs and bundle digests, `run.json`). A card based
  on that tip reuses it as BASE after identity verification; it never rebuilds
  BASE.

## 4. A card after this plan

1. Create the rule crate, its Lean modules, `docs/agents/rules/<id>.md`, a
   registry row, and `docs/reports/<card>/PREDICTION.md` naming the profiles
   and rows it expects to move. No shared file is edited; a contract change is
   a `.qv` edit plus `contracts.py sync` in the provider crate.
2. Implement; `cargo test -p rule-<id>-<name>`; `lake build <Module>`.
3. `quirin-audit measure --profile eight-home --side base=baseline:<sha>
   --side tip=. --rules-off <id> --declared PREDICTION.md`; commit the
   comparison and the rule's contribution ledger.
4. `scripts/gate_scope.py --base <sha>` under `gate_slot.py`; commit
   `gate.json`, `README.md`, `integration-amendments.json`.
5. The integrator merges, applies the amendments, runs the full gate, recomputes
   the live profiles from baselines plus contributions, publishes the new
   baseline, and pushes. Frozen profiles are untouched.

Three cards on disjoint rule crates then conflict on nothing, gate in parallel,
measure from one shared BASE with two audit slots, and each lands with a
contribution ledger instead of a moved global total.

## 5. Phasing (each phase is one or more cards with its own gate)

| Phase | Cards | Analyzer semantics | Evidence |
|---|---|---|---|
| P0 infrastructure | `MEASURE-TOOL` (F), `GATE-SLOTS`, `GATE-SCOPE` (E), `AUDIT-CANONICAL-PATHS` (F) | none, except canonical path comparison (identity run) | scoped-gate argument recorded; identity run |
| P1 decoupling data | `CONTRACT-LOCKS` (A: lock files, literal pins become readers), `EVALUATION-PROFILES` (A: profile records, cycle bindings, frozen/live policy), `LEAN-ROOTS-GENERATED` (C), `RULE-DOCS-GENERATED` and `AMENDMENTS-SCHEMA` (D) | none | lock-equals-tree and committed-equals-generated tests; every cycle names a profile |
| P2 rule API and pilots | `GATEWAY-RULE-API`, `RULE-SWITCHES`, `CONTRIBUTION-LEDGERS` (A), then one card per migrated rule (32a, 33, 34, 35, 36, 37) and `FRAMES-AS-CLAUSE` (A) | moves only; switches add a refusing arm; a clause replaces a table | identity run per move; ROUTE-MAP order test; contribution ledgers equal the cards' measured deltas |
| P3 template | card-brief template and `operations.md` card section updated to §4; wave 3 launched on it | none | first wave-3 cards land with zero shared-file edits and a ledger each |

P0 and P1 can run as parallel cards today. P2 waits for P0's tool and P1's
locks and generated roots. The Codex wave-2 cards (rules 36, 37) land on the
current structure first and are migrated in P2 like the others.

## 6. Decisions needed

1. **Contract pins as lock files** read by the tests, instead of literals in
   `qcore_contract_corpus.rs` and the provider Python. Recommended: yes; it is
   generalization §2.6 applied, and it keeps hard invariant 5 as a reviewed
   lock diff.
2. **Cycles as evaluation profiles with a frozen/live policy.** Which existing
   cycles freeze (at which analyzer revision) and which stay live is a
   per-cycle decision. Recommended default: cycles up to 19 freeze at the
   revision their last report measured; the eight-home corpus, system and
   tail-pause slice stay live.
3. **Rule switches at run time** (`QCORE_GATEWAY_RULES_OFF`) rather than
   compile-time features. Recommended: run time.
4. **Scoped gate as the card-level gate**, full gate once per landing. This
   changes the standing gate's force from *per card* to *per landing* and needs
   an edit to `docs/agents/validation.md`.
5. **Audit slots**: two or three. Recommended: two, measured before raising.
6. **Migration scope**: pilots (32a–37) only, or a standing rule that a card
   touching an older route packages it first. Recommended: pilots first.
7. **Path fix**: canonicalise in the audit (recommended) or re-spell the
   shared inputs once.
8. **Where the measurement tool lives**: a `quirin-audit` subcommand plugin
   (recommended) or `scripts/`.

## 7. Risks and boundaries

- Parity runs are the cost of seams A and B: one identity run per moved route,
  table or pin. With two audit slots and published baselines this is roughly
  one card-hour each.
- The consultation order *is* semantics; the ROUTE-MAP test and identity runs
  keep a row move from becoming a silent verdict change.
- Per-rule ledgers compose only if the comparison tool reports interactions
  (a row moved by two rules together) as their own named rows; the profile
  total is always re-derived from a real run at landing, never summed.
- Freezing a profile is a records decision, not evidence: a frozen cycle's
  figures describe the analyzer at that revision and say so.
- Generated roots, routers and locks add "committed equals generated/tree"
  tests; a card that hand-edits a generated file fails its gate, by design.
- Nothing here changes a verdict, a contract or a theorem. Acceptance: the
  first wave-3 cards land with zero shared-file edits, each with a contribution
  ledger on its profiles, and the per-landing full gate stays green.
