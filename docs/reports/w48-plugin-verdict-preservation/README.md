# W48-PLUGIN-VERDICT-PRESERVATION

Card W48-PLUGIN-VERDICT-PRESERVATION (user decision 2026-09-28, registered on
W47-CALLEE-CLOSURE-R2). Base `d7a497822`. Contract specification §C11.3 (new),
rule 10h amended in place, Lean `QCore.VerdictSemigroup` (Contributions
section, 21 new audited theorems in `AuditVerdictSemigroup.lean`).

## Headline

Before OpenSSL and libcurl contract plugins land, every published artifact a
plugin's verdict reaches now keeps each contributor's record — source, the
plugin's own answer (proposal, the firewall's class and code, its own reasons
or diagnostics), the verdict and reasons it contributes — and publishes their
join. `qcore_audit::plugin_verdicts` recovers each plugin's records from a
composed row and refuses a row that is not their join. No verdict value
moves: every change adds records or corrects a reason; statuses are computed
exactly as before.

## 1. What the host kept, and where it erased (by reading base `d7a497822`)

| Point | At base | Now |
|---|---|---|
| Firewall (`verify_plugin_claim`) | The plugin's proposal is dropped for every `Failure` (a refuted `Safe` and a crash look alike), and its own diagnostics are dropped for every non-`Unknown` answer | `verify_plugin_answer` returns `PluginAnswer { proposed, diagnostics, claim }`; the classification is unchanged |
| `plugin_checks` sidecar binding | class, code, verdict, reasons | plus `answer` (proposal and the plugin's diagnostics) |
| Checker function row | `status` overwritten by the fold: the function's own status is lost; `plugin_links` keep verdict and reasons but not what the plugin proposed (an uncertified `Safe` and `Unsafe` are both `Pending`, `plugin-claim-uncertified`, told apart only by free text) | `host_status`; each link carries `answer` and `package` (version, digest) |
| Checker unit row | `status` overwritten; `reason` set only if null, from the *first* non-`Safe` link, `reason_code` re-classified from that text. With a `Pending` link listed before an `Unknown` one the row published **`Unknown` with `reason_code` `plugin-claim-uncertified`** (a human-resolvable code on a non-`Pending` row); a host `Pending` raised by a plugin failure kept its `contract-pin-mismatch` code likewise | `host_verdict` (own status, reason, code); when a link raises the unit, the reason is the deciding link's (verdict equal to the join) with that link's typed code |
| Component sidecar (`<report>.components.json`, AUDIT-COMPOSITION) | one host component whose verdict is the post-fold status: the plugin is inseparable from the host (`COMPOSITION-PLUGIN-FIREWALL-WIRING`) | a linked row is the host's component (host's own verdict and reason) plus one component per plugin with `Provenance.external_checker` (id, package version, digest); the published row is their derivation; `render_row` takes the host's component as the row's base |
| Subcommand report row | `verdict`/`reasons` only; a claim's own `reason` reaches no row for `Safe`/`Unsafe` (the firewall reason replaces it), and no row says which source contributed what | `contributions`: host (`audit-report`), run (plugin label), each joined claim (plugin label, `answer` with proposal and the plugin's reason); `verdict`/`reasons` are `compose` of them |
| `.qo` checkpoints | rows stored byte for byte (`split_row`/`render_row`) | unchanged; the new members travel in the `verdicts` step and a composed report is byte-identical to the whole run's |

The unit-reason mismatch is stated from reading the base fold
(`user_plugins::fold_unit`, links ordered by applied callee then binding);
it was not executed at base. The tip's test pins the corrected reason.

## 2. Evidence

**Rust.** `crates/qcore-audit/src/plugin_verdicts.rs` (records, `compose`,
`project_row`, `project_function`, `project_unit`; 3 unit tests: two plugins
disagreeing recovered, a lowered/erased/dropped row refused, checker rows
projected). End-to-end traces, as modules of the two existing
exclusive-slot binaries (`tests/plugin_verdict_preservation/`):

* `checker_route.rs` (in `user_contract_plugins`): a git checkout, the
  SDK's example user crate with two checker packages, `quirin-audit current`
  whole, with `--checkpoint-dir`, and `checkpoint compose`.
  * `pending_and_unknown_survive_checkpoints_and_composition`: plugin A
    proposes `Safe` without evidence (`Pending`, QP008), plugin B crashes
    (`Unknown`, QP006) — two plugins disagreeing and a failing plugin; a host
    `Unsafe` (leak) beside both stays `Unsafe`. Whole, checkpointed and
    composed reports and contract sidecars are byte-identical; both linked
    checkpoints' `verdicts` sections carry the records; every unit and
    function row projects; each plugin's proposal, class, code, own
    diagnostic and verdict come back out; the unit is `Unknown` with
    `provider-failure`; the component sidecar checks and splits host and
    plugins.
  * `certified_safe_and_validated_unsafe_survive_checkpoints_and_composition`:
    through the library entry (`CurrentRoute::plugin_checkers`, new) with test
    doubles of a witness and a counterexample checker, a certified `Safe` and
    a validated `Unsafe` (a plugin `Unsafe` beside a host `Safe`) reach row,
    sidecar, checkpoints and composition; the same packages on the command
    line are `Pending` (QP016, QP010).
* `subcommand_route.rs` (in `subcommand_plugins`): host `Safe`, `Pending`,
  `Unknown`, `Unsafe` rows beside plugin claims (uncertified `Safe` and
  `Unsafe` are `Pending`, `Unknown` is `Unknown`, two claims disagreeing on
  one artifact, a claim on an artifact the host lacks); every row projects
  and every claim is recovered with its proposal and own reason. A plugin
  that answers and then exits 1: every row gains the run's `Unknown`
  (`provider-failure`), its claims are recorded (`joined: false`) and joined
  nowhere.

Uncertified plugin `Safe`/`Unsafe` → `Pending`: confirmed unchanged on both
routes (the command line supplies no checker; `PluginCheckers` now requires
`Sync` checkers so the route can carry them).

**Lean** (`QCore.VerdictSemigroup`, Contributions): a `Row` of
`Contribution (source, answer, value, reasons)` records; `published_is_the_fold`
(the published reasoned verdict is the fold of the records),
`join_published`, `join_keeps_every_record`, `project_join` (projection
distributes over composition), `project_recovers`,
`project_only_from_the_source`, `project_and_rest_recompose` (a source's
records and the rest recompose the value, reasons up to permutation, via the
new `fold_perm`), `a_plugin_record_is_recovered`,
`an_uncertified_plugin_keeps_the_row_non_safe`,
`an_unknown_record_keeps_the_row_at_least_unknown`,
`a_failing_plugin_keeps_the_row_at_least_unknown`,
`a_row_with_a_plugin_is_safe_only_if_certified`, and the negative controls
`two_plugins_disagreeing`, `a_plugin_unsafe_beside_a_host_safe`,
`a_failing_plugin_beside_a_host_safe`. Axioms: `propext`, `Quot.sound` only.
That the Rust rows are these rows is pinned by the tests above, not proved.

## 3. Measurement

None. No published verdict can move: statuses are folded exactly as before;
rows without a checker binding or subcommand row are produced by unchanged
code (a single host component, no new member), and the pinned corpus and
system runs carry no checker binding. No prediction was required.

## 4. Gate

See [gate.json](gate.json) (scoped card gate, `scripts/gate_scope.py --base
d7a497822`); commands, exits, counts and log paths are in the reply and the
increment paragraph.

## 5. Boundaries and follow-ons

* `CHECKPOINT-COMPONENT-SIDECAR` (registered): a checkpointed `current` run
  and `checkpoint compose` write no `<report>.components.json`, so the
  per-plugin components exist only on whole runs; the rows themselves are
  preserved.
* `SUBCOMMAND-CLAIMS-INTO-AUDIT-ROWS` (registered): a subcommand plugin's
  claims reach only its own report's artifact rows; no route composes them
  into an audit report's function or unit rows or a `.qo`. The OpenSSL and
  libcurl plugins need one to change a corpus row.
* Rule id 51, which the brief reserved, is already rule 51
  (RULE50-CONSUMER-COMPOSITION); this card added no rule and amended 10h.
* Out of scope and untouched: `PLUGIN-UNSAFE-MISBOUND-EVIDENCE-PENDING`
  (firewall ordering in the file this card edited), the W45 certificate
  checkers.
