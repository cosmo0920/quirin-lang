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

## 3. Seven seams

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
  `contract/evaluations/<name>/` = route (inputs manifest digest,
  link map, property files), contract bundle digests, and a policy
  `frozen@<analyzer revision>` or `live`, with cycle bindings in
  `contract/evaluations/bindings.json` (card EVALUATION-PROFILES: a directory
  under `contract/profiles/` without `profile.json` empties the first-party
  provider registry, so evaluation records live beside, not inside, the
  source-set profiles; `scripts/evaluation_profiles.py` is the authority). A frozen profile's figures never move; a live one is recomputed by
  the integrator from published baselines. `docs/cycles/CYCLE_*` and the
  matrix's requirement rows name their profile and policy, so an
  implementation change can no longer silently re-date a cycle's claim.
  **Corpus cadence (user decision 2026-10-24).** Live profiles follow the
  Fluent Bit repository: every one to two months, best effort, a
  `REBASE-CORPUS` card prepares the inputs of a newer Fluent Bit revision,
  re-reviews the pins whose sites changed (`pin --check` at the new revision,
  C06.2), publishes the new baselines and moves the live profiles' inputs
  manifest to it; the previous baseline stays published, so the two revisions
  can be compared row by row. Frozen cycles are not moved. A re-base is a
  records-and-inputs change with its own gate; it never edits a rule.
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

### G. The semigroup integration interface (user direction, 2026-10-24)

Every integration of results or contracts goes through one interface whose
operation is associative and erases nothing, so no contribution can
contaminate another unit's audited result. The repository already has the
pieces: the four-value join `Safe < Pending < Unknown < Unsafe` (§L21.2), the
subcommand-plugin rule that "a row is the join of the host's verdict, the run's
and the claims, so it never lowers a host verdict", and the first-answer route
table. This seam names them, gives them one schema and one set of laws, and
makes them the only path by which seams A and F integrate anything.

Two compositions, kept distinct:

1. **Across results (the integration semigroup).** The carrier is a
   *contribution*: a set of components, each
   `(unit identity, row key, property, profile, provenance, verdict tuple,
   reason leaves)`, where provenance is the analyzer revision, the rule set
   (with its off-list), the contract bundle digests and, for an external
   checker, its identity. The operation `⊕` is provenance-keyed union of
   components; a row's **published** verdict is derived at read time as the
   join over its components, carrying every component's assumptions (§L21.2).
   Laws, tested on vectors in Rust and proved in `QCore.AuditComposition`:
   associative, commutative, identity (the empty contribution), idempotent
   for byte-identical components; the join never lowers a verdict; adding a
   component never removes an assumption. **Conflict is not resolved by
   choice**: two components with the same provenance key and different bytes
   leave the row `Unknown` with reason `composition-conflict`, and both
   components stay. Removing a contribution (a rule switched off, a contract
   bundle withdrawn, a checker dropped) is a re-join without it; nothing is
   rewritten, so the removed atoms remain auditable.
2. **Within one unit's analysis (the route semigroup).** Packages at a
   position compose by ordered consultation: first answer wins, the identity
   is refusal, the order is the registry's and is semantics. A package's
   answer is admit-with-obligations or refuse; it cannot edit another's
   answer. This composition is deliberately not commutative; what it shares
   with the first is that nothing is erased and absence is refusal.

**Contracts compose by supersession (new requirement, user direction
2026-10-24).** A contract bundle is a contribution to the loaded environment.
Bundles union by `(library, function)`. Where two bundles declare the same
function with different text, the declaration with the **newer pin stamp** is in
force and the older is **retained as superseded**, listed in the run's
contract-revision section with both digests; it is never deleted. The stamp is
`pinned_at`, written by `quirin-audit pin` into the declaration's lock entry
when its current text is reviewed (lock version 3), so it is tied to the
contract unit and to a human review, not to a file's mtime or a clock at load
time. `pin --check` requires a re-pin to carry a later stamp than the entry it
replaces. The operation is "greater stamp wins", a max over a total order, so
it stays associative and commutative; its fail-closed residue is exactly the
cases the order does not decide: equal stamps with different text, a missing or
malformed stamp, or an unpinned declaration beside a pinned one (an unpinned
declaration is not in force under C06.2 and never supersedes). Those withhold
the declaration (`contract-conflict`; `Unknown` at every call site) with both
texts recorded. `include`/`inherit` are unchanged. Call sites record the
in-force declaration's digest in their assumptions as they do now, so a
supersession that changes a verdict is a reviewed contract change (C06.2), not a
refactor. This replaces today's hard loader error on a duplicate declaration;
the loader card carries an identity run (no pinned corpus module currently
declares a function twice, so every report must stay byte-identical).

**Where it lives.** `qcore-schemas::composition` (serde types and the join
functions; the only crate providers and plugins may depend on), consumed by the
report emitter in `qcore-audit` (components stored per row; the report's rows
become a deterministic derived view, byte-identical to today's output under an
identity run), by `quirin-audit measure` (a baseline `⊕` a TIP contribution),
by the checker-plugin firewall (already a join) and by the contract loader
(bundle union). Published baselines (seam F) store components, not totals, so
a profile total is a derivation over atoms from real runs, and a rule's ledger
(seam A) is attribution over those atoms, never a substitute for the run.

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
  is fail-closed by construction; `run.json` records the list. A hold is the
  exception: it is not switchable, and a list naming it is refused (§6
  item 10). This replaces
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
  with classes `rust-suite` (1 slot; constraint 16), `audit` and `lean` (2).
  The `audit` class has `floor(available memory / budget)` slots, where the
  **budget is 7 GB per audit by default** and can be set to 4 or 5 GB
  (`--memory-budget 4G|5G`, user decision 2026-10-24); the budget is passed to
  the audit's existing MiB planning budget (`audit-ir-working-set`), so a
  smaller budget lowers that run's own concurrency rather than overrunning
  its slot, and `run.json` records both. The `rust-suite` class takes every
  `audit` slot while it runs, so the suite never shares the host with an audit.

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
| P1 decoupling data | `AUDIT-COMPOSITION` (G: schema types, join laws in Rust vectors and Lean, per-row components in the report with an identity run), `CONTRACT-SUPERSESSION` (G: lock version 3 with `pinned_at`, loader supersession replacing the duplicate error, `contract-conflict` residue, spec clause under C06, Lean supersession laws, identity run), `CONTRACT-LOCKS` (A: lock files, literal pins become readers), `EVALUATION-PROFILES` (A: profile records, cycle bindings, frozen/live policy), `LEAN-ROOTS-GENERATED` (C), `RULE-DOCS-GENERATED` and `AMENDMENTS-SCHEMA` (D) | none; composition is a derived view (identity run) | lock-equals-tree and committed-equals-generated tests; composition laws audited; every cycle names a profile |
| P2 rule API and pilots | `GATEWAY-RULE-API`, `RULE-SWITCHES`, `CONTRIBUTION-LEDGERS` (A), then one card per migrated rule (32a, 33, 34, 35, 36, 37) and `FRAMES-AS-CLAUSE` (A) | moves only; switches add a refusing arm; a clause replaces a table | identity run per move; ROUTE-MAP order test; contribution ledgers equal the cards' measured deltas |
| P3 template | card-brief template and `operations.md` card section updated to §4; wave 3 launched on it | none | first wave-3 cards land with zero shared-file edits and a ledger each |

P0 and P1 can run as parallel cards today, except that `MEASURE-TOOL` consumes
`AUDIT-COMPOSITION`'s schema and so follows it. P2 waits for P0's tool and
P1's locks, composition and generated roots. The Codex wave-2 cards (rules 36, 37) land on the
current structure first and are migrated in P2 like the others.

## 6. Decisions

Decided by the user on 2026-10-24, 2026-10-25 and 2026-10-26 (items 1–9 and 9a).

1. **Contract pins as lock files**: yes. Literal count tables become readers of
   generated locks; a contract change is an intentional lock diff in the same
   commit.
2. **Cycles as evaluation profiles**: freeze cycles at the revision their last
   report measured; keep the eight-home corpus, system and tail-pause slice
   live, and re-base the live profiles onto a newer Fluent Bit revision every
   one to two months, best effort (the `REBASE-CORPUS` cadence in seam A).
3. **Rule switches at run time** (`QCORE_GATEWAY_RULES_OFF`): yes.
4. **Scoped gate per card, full gate per landing**: yes. `docs/agents/validation.md`
   is amended by the `GATE-SCOPE` card on this authority.
5. **Audit slots**: by memory budget, 7 GB per audit by default, with a
   selectable 4–5 GB budget; the load-sensitive Rust suite still runs alone.
6. **Migration scope**: pilots first (rules 32a, 33, 34, 35, 36, 37); a
   standing rule is decided after their parity cost is known.
7. **Path fix**: canonicalise paths in the audit's unit selection.
8. **Measurement tool**: a `quirin-audit` subcommand plugin (user decision
   2026-10-25), behind the same firewall and receipts as other subcommands.
   Also decided 2026-10-25: `scripts/gate_scope.py` claims only the exact file a
   compile-time read compiles in (`--reader-granularity file`, the default);
   `directory` remains available as the conservative reading.
9. **Semigroup integration and newer-stamp supersession**: yes (seam G).
   **9a, stamp migration** (decided 2026-10-26): re-pin every provider once
   against the latest Fluent Bit revision, so each declaration gets a reviewed
   `pinned_at` (back-filling the lock's generation time was rejected). Pin lock
   sets are tied to a Fluent Bit revision and kept in a rolling window of three
   revisions (the latest and the two before it); each re-base adds a set and
   retires the oldest. Frozen cycles outside the window keep their report
   evidence but no live lock.
10. **What "off" means for a hold** (`GATEWAY-RULE-API-HOLD-OFF`; decided by
   card RULE32A-PILOT on 2026-10-31 under the hard invariants, not a user
   decision): **a hold is not switchable.** Its registry row is
   `off = "not-switchable"`, a `QCORE_GATEWAY_RULES_OFF` list naming it is
   refused, and its host asks it under every list, a refused one included
   (`gateway_rules::enabled`). Reasoning, on rule 32a, the first hold: a
   hold's admission is the refusal of a reading. (a) A refusing off, as for
   every other package, removes the hold, and the logger's own row reads its
   sibling's caller-frame declaration and is `Safe` again, the regression
   rule 32a exists to refuse; it widens, which invariant 1 forbids. (b) An off
   that holds more (every call to a frame member held, from any caller)
   never widens, but it changes no frame member's row: every row it moves is
   a caller outside the frame, which rule 32a leaves to the declaration by
   definition. Its measured "contribution" would be rule 32's (the outside
   callers' reading of the declarations) filed under 32a, a mislabelled
   ledger. Both readings are theorems of `QCore.CallerFrameHoldSwitch`
   (`a_refusing_off_would_widen`, `holding_more_changes_no_member_row`,
   `holding_more_moves_an_outside_row`). This follows rule 34's precedent: a
   guard whose absence would widen stays unswitchable. Consequence for the
   ledgers (seam A): a hold is registered, so `known_rule` accepts its id,
   and it has **no switch-measured contribution ledger**; none is written or
   transcribed. The measurable quantity behind it, what the caller-frame
   declarations give their outside callers, is rule 32's and needs a rule-32
   switch (its off refuses an admission, so it never widens).

## 7. Risks and boundaries

- Parity runs are the cost of seams A and B: one identity run per moved route,
  table or pin. With two audit slots and published baselines this is roughly
  one card-hour each.
- The consultation order *is* semantics; the ROUTE-MAP test and identity runs
  keep a row move from becoming a silent verdict change.
- Per-rule ledgers compose only if the comparison tool reports interactions
  (a row moved by two rules together) as their own named rows; the profile
  total is always re-derived from a real run at landing, never summed.
- The two compositions of seam G must not be confused: the route semigroup is
  order-sensitive and lives inside one run; the integration semigroup is
  commutative and lives across runs. A tool that joined route answers across
  runs, or consulted components in registry order, would be wrong in a way the
  laws' tests are there to catch.
- Storing components per row grows the report. The derived view keeps today's
  bytes; the component store is a sidecar, and the identity run is what shows
  the view unchanged.
- Freezing a profile is a records decision, not evidence: a frozen cycle's
  figures describe the analyzer at that revision and say so.
- Generated roots, routers and locks add "committed equals generated/tree"
  tests; a card that hand-edits a generated file fails its gate, by design.
- Nothing here changes a verdict, a contract or a theorem. Acceptance: the
  first wave-3 cards land with zero shared-file edits, each with a contribution
  ledger on its profiles, and the per-landing full gate stays green.
