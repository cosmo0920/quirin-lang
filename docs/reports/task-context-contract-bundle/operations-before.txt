# Operations and integration policy

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

Authority update: references below to rules/status in `AGENTS.md` now
resolve through its topic router and `docs/STATUS.md`. New amendments
land in the canonical topic file. The cadence covers these pages as
well as the concise root guides. Migration changes location, not force.

<!-- preserved:operations:start -->
## Working rules

- **Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass** (`AGENTS.md` invariants 1 and 5). Where a
  pinned table — contract corpus counts, submodule revision, archived report,
  CT manifest — conflicts with your change, update the pin *deliberately* and
  say so in the commit message and report.
- **Status language matters.** Keep the Safe/Unsafe/Unknown verdict vocabulary
  and the "unpromoted" phrasing exactly as defined in `AGENTS.md` when editing
  docs, reports or commit messages. A provisional `Unsafe` is never an upstream
  defect claim; a grown denominator is not new conformance.
- **Run the full standing per-change gate from `AGENTS.md`** before reporting a
  task complete, and report probe/example exit codes honestly — a nonzero probe
  exit is a finding, not a build failure.
- **Land code in the target crate.** Before starting checker/architecture work,
  read `docs/GENERALIZATION_PLAN.md` and follow its crate layering (§3) and
  invariants (§2); do not add new code to the monolithic `compiler/` grab-bag
  once a target crate exists.
- **Close a card with Rust plus audited Lean.** A card closes only with a Rust
  implementation and matching audited Lean modules registered in an
  `Audit*.lean`; Python artifacts are supplementary evidence, never the close.
- **New status text goes in one place** — one paragraph, newest first, in the
  current record file named by `docs/development/increments/README.md`, plus
  `AGENTS.md`'s headline numbers, per that file's sync rule. Update the index
  when a new wave file opens. Do not re-grow a per-increment chronicle here
  or in the root development table of contents.
- **Compact these three files every 2–3 waves.** `AGENTS.md`, `CLAUDE.md` and
  `README.md` accrete: constraints restate their card reports, sections drift
  apart, README falls behind the ledger. A recurring compaction card
  (eight firings to date, `DOCS-COMPACT` through `DOCS-COMPACT-8`, each with
  its own report — `docs/reports/wave16-docs-compact/` for the first,
  `docs/reports/docs-compact-{2,3,4,5,6,7,8}/` for the rest; the dated roster is
  a checkpoint in the wave-37 record. Latest `DOCS-COMPACT-8`, wave 40; the
  ninth is due around wave 42 or 43) re-reads every headline
  number **from its source and never from the file being compacted**,
  inventories the standing constraints before and
  after, and merges redundancy. Compaction is **not** deletion: no constraint,
  hard invariant or evidence pointer may be dropped, and a fact that lives only
  in these files moves verbatim into the development history indexed by
  `DEVELOPMENT.md`, rather than disappearing. Historical copies are labelled
  as checkpoints; they do not become a second current rule authority.
  Expect single-digit percentages in the steady state, and a larger fall
  whenever one block has been allowed to become a chronicle — these files are
  references, and the rule exists to bound accretion, not to promise a
  shrinking file. **A pass may legitimately grow a file**, and
  `DOCS-COMPACT-6` did: the defects `STABILITY-REEVAL` #1 found were
  *understatements* — a conditional theorem presented as unconditional, a
  premise list missing its two largest members, a mutation count inflated — and
  correcting an understatement costs words. Compaction bounds accretion of
  *narrative*; it does not licence trading accuracy for bytes.
- **Re-evaluate stability every 2 waves** (user direction, 2026-09-13, after
  wave 31). Green tests are an *observation*, and an observation is not a
  proof: a recurring **`STABILITY-REEVAL`** card (#1 in wave 34, #2 in wave 36,
  #3 in wave 38, #4 in wave 40, reports under
  `docs/reports/stability-reeval-{1,2,3,4}/README.md`; **#5 is due after wave
  41** — recorded as due by the wave-40 close, not run — and #4's own §6 item 12
  names the **transcription family** (`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`,
  `CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND` and
  `PROOF-LEG-DIGESTS-UNCROSSCHECKED` being one defect wearing three names, so a
  census of every "proved in Lean" claim against what binds it to Rust is the
  cheapest way to size it) and **the standing constraints' own sentences** as
  the highest-value unexamined ground, #4 having found two of them simply false
  by reading them against the tree rather than against each other; where #3's §6
  had named the Windows arm (`F02-WINDOWS-PIN-RUN`, the one live red row) and
  the tracker-only residuals, and #2's §6 the corpus artifacts — pinned by
  no test — and the remaining Lean docstrings) asks, of the claims this project currently rests on, **under what conditions
  each is unstable or unproven** — and it reports that as a finding rather
  than as reassurance. The method, in the order that has caught the most:
  1. **Hunt vacuous greenness.** For every fail-closed arm added or changed in
     the last two waves, ask whether any test actually *drives* it. An arm no
     test reaches is not verified by a green suite; the suite is evidence
     about the paths it did take. Measure reachability (who calls the
     function; does any fixture produce the refused shape) rather than
     inferring it from the diff. The occasioning case:
     `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, where a semantic union
     resolved at integration was "verified" by two green suites that never
     touched the arm in question. That residual was **discharged** by wave 32's
     `TRACE-ALPHABET-LEASE-BOUNDARY`; it is named here as the case that
     occasioned the cadence, not as open work. The cadence earned itself on it
     twice over — `STABILITY-REEVAL` #1 found `AGENTS.md`'s CT row still
     describing the arm as unproved two closes after its own constraint 15n
     recorded the discharge, and found the discharging card's mutation evidence
     overstated in that constraint at five tests where the measurement is
     three. Both were repaired by `DOCS-COMPACT-6`.
  2. **Separate the proved side from the observed side of a boundary.** A
     theorem about one arm (CT03's write conservativity) says nothing about
     its sibling arm (the lease refusal). Where a boundary is proved on one
     side and observed on the other, say so in those words.
  3. **Find semantic edits made without a card.** An integrator resolving a
     merge, a hotfix, a "mechanical" rename: each can move meaning with no
     report, no Lean parity and no residual name (invariant 6). These are the
     hardest to see later precisely because nothing indexes them.
  4. **Ask what a later card would do to the claim.** A rule that is correct
     today because a construct is unreachable becomes load-bearing the moment
     a queued card makes it reachable — name that card in the finding.
  Every finding lands as a **named residual with a discharge condition** (prove
  it, exercise it, or state it as a measured boundary). Postponement is a valid
  outcome and must be recorded as one; what is forbidden is letting an
  observation stand where the text implies a proof.
  **What actually gets a residual discharged is brief-reach, not the class of
  work it needs** — firing #4's §1.3, which corrected firing #3's own headline:
  #3 had measured 0 of 18 "mechanism-needing" residuals discharged and blamed
  the class; measured again, mechanism runs 5/11 and sentence 3/3, every
  discharged name appears in a close's item table or in a card's brief, and the
  three names reaching no close document are all open. So **write the open
  residuals a card can plausibly touch into that card's brief**, and treat a
  name that reaches no brief and no close document as the one at risk.
- **Work every card's integrator reconciliation list at the wave close, and
  leave the record that you did** (card `W32-RECONCILE-SWEEP`, wave 32,
  discharging `DOCS-COMPACT-5-RECONCILIATION-LIST`). Cards close with a section
  addressed to the integrator — "Integrator reconciliation list", "For the
  integrator", "Integrator carry list", "Proposed amendment text for standing
  constraint N", "Proposed proof-stage evidence (the integrator's flip)". Three
  consecutive closes (waves 29, 30, 31) applied those sections' **headline
  numbers** and left the rest, costing one entire unlanded standing-constraint
  amendment. Two prior findings asked for more care and were skipped, because
  advice is skippable; the step is therefore mechanical:

  ```sh
  python3 scripts/reconciliation_sweep.py . --since <previous close> \
      --check docs/reports/wave<N>-close/reconciliation.json
  python3 scripts/reconciliation_sweep.py . --cards <the wave's cards> --residuals
  ```

  The close **writes** `docs/reports/wave<N>-close/reconciliation.json` beside
  its `gate.json`: one entry per swept item, verdict `applied` / `declined` /
  `superseded` / `deferred` / `noted`, every non-`applied` verdict carrying its
  ground, and a `no_section` reason for any swept card the sweep found nothing
  in (checked both ways, so it cannot wave away a card that does have one).
  `--residuals` lists the residual names the wave's reports introduced that
  reach no tracker, which is the same defect's other half — `DOCS-COMPACT-4`'s
  and `DOCS-COMPACT-5`'s own findings about this process were themselves in
  that untracked set for two waves. **The tool decides nothing.** A green
  `--check` means every item carries a recorded human verdict, never that any
  item was applied; only a person can measure the tree against a paragraph of
  English, and a reconciliation record that says `applied` without saying where
  is worth no more than silence. An amendment lands **in the words the card
  proposed** or is recorded `declined` with its ground — never silently
  softened while being "reconciled" (invariant 1).
  Since wave 36 the "saying where" is **mechanical**: an `applied` verdict
  carries a `verify` list of `{file, contains}` / `{file, absent}` claims that
  `--check` opens and reads, or an `unverifiable` ground saying why the claim
  has no quotable text. Whitespace is normalised on both sides so a wrapped
  quotation still matches; nothing else is, so a **softened** amendment is
  still a changed claim. The arm exists because a wave-35 verdict recorded
  `applied` had its asserted text absent from the file it named
  (`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`) — prose naming a location is
  not a measurement of it. It still decides nothing: it decides whether the
  close's own string is where the close said it is, never whether it is the
  right string.
  **A `--check` is green at its own close and only there, and that is the
  rule** — decided at the wave-40 close on
  `RECONCILIATION-VERIFY-CLAIM-PERISHABLE`, whose occasioning case was one
  claim quoting the increments index's mutable "Current record file" pointer.
  Measured at that base, the wave-39 close's own record fails **16 of its 57**
  claims, not one: **15** of those 16 were broken by the wave-40 compaction
  merge rewriting the very `AGENTS.md`/`CLAUDE.md` sentences the close had
  verified, and only the sixteenth by the wave-opening commit. Durability is a
  property of the **target file**, not of the wording: every claim into the
  acceptance matrix (17), a frozen increment record (4), the spec (3), crate
  source (2), the CT manifest and its fixture (2) and a frozen card report (1)
  survived, while 15 of the 27 into the two compaction-eligible files did not.
  So "quote only durable text" is **not sufficient on its own** — verifying an
  `AGENTS.md` amendment *in `AGENTS.md`* is the point of the arm, and a
  compaction card is chartered to rewrite exactly that prose. Operative until a
  `--check` mode distinguishes "applied and later legitimately superseded" from
  "never applied": **prefer the most durable location an amendment allows**
  (the tracker entry, the wave record, the spec, crate source) when the
  amendment genuinely lives there; **each close `--check`s its own record
  against its own tree** and is never obliged to re-green a predecessor's; and
  a later red on a frozen close's record is **evidence of supersession, not of
  a defect**, and must not be repaired by editing that frozen record.
- **A retirement is named, checked, and swept after.** Before counting a name
  as retired, confirm it is **actually a registered entry** — wave 36 took
  three retirements and two of that wave's cards each recorded a "retirement"
  of a name that had never been in the matrix, which is a no-op that later
  reads as a lost removal unless it is recorded as one. And **retiring an entry
  requires re-homing every name it was the sole tracker for**, checked by
  `--residuals` run *after* the removal and compared with the run before it, not
  by intention (`STABILITY-REEVAL` #2). Expect the retired names themselves to
  appear in that later run: a discharged name stays written in the history that
  discharged it, and joins the "discharged, with the discharging card named"
  class rather than becoming open work.

## Worktree and measurement hygiene

- **Prefer temporary WIP commits over `git stash`.** The shared git stash stack
  is not per-worktree; it caused one cross-worktree stash consumption during
  the second delegated wave (recovered from dangling commits, nothing lost).
- **Measurement outputs belong in worktree-private directories** — `measure/`
  or `private/`, both gitignored — not the shared session scratchpad and **not
  under `target/`**. Delegated agents share the scratchpad, and a mid-run
  overwrite was caught and re-measured during wave 15; a wave-17 card lost a
  whole providers run to its own `cargo clean` because the log lived under
  `target/`.
  **Scope a gate run's outputs to the run** (`measure/<run-name>/…`), never to
  fixed paths. At the wave-36 close a *confirmation* re-run truncated the
  `cargo-test.log` of the run it was confirming, then died at 79 of 233
  summaries with exit 101 when the filesystem hit 100% — so a published figure
  briefly had no log behind it. The re-run is the dangerous case precisely
  because it is meant to change nothing. A gate that runs out of disk measures
  the disk: **discard it, do not interpret it**, and re-measure after freeing
  space (this worktree's own `target/` is the thing to remove, which is safe
  exactly because no log lives there).
- **A committed artifact is not a finished card.** When reclaiming disk from a
  delegated worktree under pressure, reclaim only after that card's **final
  report** — not when its `gate.json` or another artifact appears in the log.
  At the wave-36 close an integrator read a committed `gate.json` as *finished*
  and reclaimed a `target/` out from under a card that was still confirming,
  costing a full rebuild at the worst moment (six cards at their build peaks).
  Recorded by the integrator against their own action. It is the same class of
  error as killing by pattern rather than by PID: **inferring another agent's
  state from a signal that does not carry it**, when the agent itself can be
  asked.
- **A mutation harness restores by rewriting, never by an mtime-preserving
  copy.** `shutil.copy2` restores the backup's timestamp, which leaves
  cargo's fingerprint from the mutant build *newer* than the restored
  source: cargo skips the rebuild and every later run in that worktree
  links the mutated crate while `git status` reads clean. Measured in wave
  35 (`MUTATION-HARNESS-MTIME-RESTORE`), where it produced two false CT
  failures; the same mistake in the discharging direction produces a false
  **green**, which is why this is a rule and not a tip. Restore by writing
  the original bytes back, and assert the round-trip.
  The rule is landed here; the **second reader** card `MUTEX-SHARE-CROSS-ARMS`
  asked for — an audit of `docs/reports/c-gateway-return-call-w4/scripts/mutations.py`,
  the other harness in the tree, which that card did not examine — was done
  by the wave-39 close (it restores by `path.write_text(text)`, a rewrite),
  which retired `MUTATION-HARNESS-MTIME-RESTORE` on both halves of its
  discharge. **`mv` of a `cp` backup and `cp -p` preserve the mtime the same
  way**: wave 39's `LOOPS-ON-COMPOSED-ROUTES` measured it — its first full
  gate ran against the rlib of its last mutation, three red rows of its own
  file (`gate/run1/`) — so a harness that must move a file `touch`es it after
  every restore and runs a green baseline after the last one. The wave-39
  close surveyed all seventeen shipped harnesses in `docs/reports/`
  (`MUTATION-HARNESS-STALE-ARTIFACTS`, which carries the mechanical check
  still owed): twelve restore by rewriting, four by `cp` from a backup
  (three of them also `touch`), one by `mv` plus `touch` (the loops card's
  own, repaired); none is mtime-preserving today.
- Delegated cards work in isolated worktrees, merge one at a time under the
  failure-gated chain discipline, and close with a per-card report under
  `docs/reports/<card>/README.md`. **A mutation count is cited in a report
  only with its script and its per-mutation log in that report directory**;
  a count whose harness lives under `measure/` or `private/` is written as
  *measured once, harness not shipped*, and a later close publishes that
  restatement beside the report rather than editing it
  (`W36-W37-MUTATION-HARNESSES-UNSHIPPED`, restated for four wave-36/37
  cards by the wave-38 close). **An integrator who cannot regenerate the
  matrix with any card's `gate.json`** — two sibling cards having added Lean
  audits, so no card's figure agrees with the merged tree — measures the tip,
  writes `docs/reports/<card>/integration-gate.json` beside the card's report
  and says so in the merge message (wave 39, three merges: `e80b58d6`,
  `0150c904`, `b6d1f101`); the close **sweeps every such file into its own
  record** and re-measures the merged-gate row regardless. The generator is
  deliberately not taught a second layout — its `--gate` path already accepts
  the file — and what its check cannot yet see is a Clang recording borrowed
  from the card's own gate directory (`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`,
  registered by the wave-39 close).
  **When the file is owed, decided at the wave-40 close**
  (`INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE`, taken on its second arm):
  a merge whose message states a figure **differing from the card's own
  `gate.json`** must publish `docs/reports/<card>/integration-gate.json`; a
  merge whose stated figure **equals** the card's need not, the card's own file
  being the record. Applied retrospectively: wave 39's `0d9e922f` and
  `45a33285` stated differing figures and published none, so the close appended
  a dated note to each card's report carrying the figure, the measured
  pre-amendment tip **and the in-history merge sha beside it** — a measured tip
  is cited with its successor, because all five wave-39 tips and both wave-40
  tips are non-ancestors of `main` (`INTEGRATION-TIP-CITATION-UNREACHABLE`).
  Wave 40's two merges stated figures **equal** to their cards' gates
  (4,727 / 141 / 303 both times, measured), so neither owed a file.
- **A delegated card never advances `main`, and since wave 36 a mechanism says
  so.** The instruction alone failed once: a wave-33 measurement card ran
  `git -C <main checkout> merge` and put its own commit on `main` as a bare
  fast-forward, with no merge commit, no review and no gate
  (`CARD-SCOPE-SELF-MERGE`). `CARD-SCOPE-GUARD` landed versioned hooks in
  `.githooks/` — a `reference-transaction` hook, which sees **local** ref
  updates from every worktree, plus `pre-push` for the remote door. An update to
  `refs/heads/main` is refused outright from a delegated `agent-*` worktree,
  with or without authorisation, and refused for everyone else unless it carries
  the repository-local token on that one command
  (`QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)" …`,
  never `export`ed). The hooks are **inert until the integrator runs
  `python3 scripts/install_main_guard.py` once in the main checkout**, which a
  card cannot do. They are also **bypassable by intent** — `--no-verify`, a
  `core.hooksPath` override, a hand-written ref — and are a guard against a
  card's *mistake*, not a security control; do not describe them as more.
  `docs/reports/card-scope-guard/README.md`, and `.git/qcore-main-guard.log` for
  what was refused or authorised.
  **The guard is now active** (the integrator ran the installer in the main
  checkout at the wave-36 close; the log carries the self-test refusals and every
  later merge as authorised), and **`git push` of `main` needs the token exactly
  as `git merge` does** — the installer's message named merges only, and the
  integrator's first two pushes of this wave were refused
  (`refused-push-unauthorised`, twice) before the token was carried on the push
  command too. That is the guard working; it is written here because the
  activation message does not say it.
  **One false positive is measured and open**: `git pack-refs` — which git's
  automatic `gc` runs — re-writes `refs/heads/main` with old = the null OID and
  new = the ref's *existing* value, and the guard refuses that transaction even
  though it advances nothing. The commit itself succeeds; the background `gc`
  fails behind it, printing the guard's banner for an operation that was never a
  main update (`MAIN-GUARD-REFUSES-PACK-REFS`, reproduced at both `477b733d` and
  `2caf7369`). Expect it, and do not read it as your commit having been refused.
- **Never `pkill`/`pgrep` on a pattern that names a tool invocation** —
  `cargo`, `lake`, `python3` — because every parallel card runs the same
  commands. **The process table is shared across worktrees exactly as the stash
  stack is.** Kill by the PID you captured when you started the process, or use
  `TaskStop` on the background task id. Landed in card
  `MUTEX-SHARE-CROSS-EMISSION`'s own words after that card ran
  `pkill -f "cargo test --workspace"` and killed a **sibling** wave-36 card's
  standing-gate run mid-flight, whose evidence file stops after `SUB=0` with no
  `TESTS=` line; it disclosed the kill in full, touched no repository state and
  deliberately did not restart the other card's job, on the ground that guessing
  at another card's intent is worse than reporting the fact. The card *had* the
  PID — it had already used `kill -0` to wait on its own run — and still reached
  for the pattern, which is why this is a rule rather than advice.
- **Never end a turn with a pristine tree while work is in flight.** The harness
  removes an isolation worktree when its agent ends a turn with an **unchanged**
  tree — clean and no commits ahead of base — *even while that agent's own
  background jobs are still using it*; it cost a wave-36 card its worktree
  mid-run. Make a checkpoint WIP commit as soon as there is anything to keep
  (the rule above already prefers a WIP commit to `git stash`), stay at least one
  commit ahead of the base for the rest of the card, and amend or squash later.
  `WORKTREE-AUTOREMOVAL-PRISTINE-TREE`.

<!-- preserved:operations:end -->
