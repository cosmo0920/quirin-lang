# SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS (draft: stopped for a decision)

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch both".

Base origin/main `2c60986b0`. The card discharges
`SP7-PLUGIN-GUARDED-APPLICATION-FORMULAS`. This draft holds the census and
the formalisation decision the card stopped on. Nothing else is
implemented yet; PREDICTION.md follows the decision, because its digest
deltas depend on it (option B changes provider files).

## 1. Census (base)

Pattern: the seven retired English texts of the frozen plugin sidecar
(`docs/reports/premise-guarded-decomposition/applications/plugin.json`,
bindings `environment`, `invoke`, `registered_case`, `default_case`,
`registered_lifetime_valid`, `default_lifetime_valid`, `unregistered`),
searched by their first 40 bytes over the whole tree except `target/`,
`private/`, `.git/`, `third_party/` and `.lake/`.

| File | Role | Base | After |
|---|---|---|---|
| `docs/reports/premise-guarded-decomposition/applications/plugin.json` | frozen sidecar (digest-held provenance) | 7 (one per symbol) | pending the decision |
| any file outside `docs/reports/` | | 0 | |

No production module, test, fixture, spec page or Lean file states any of
the seven texts. The plugin application lives only in its frozen sidecar,
its graph (`plugin-graph.json`), its document (`plugin.qgd`, already
symbolic) and the frozen source `docs/reports/quant-contract-hypotheses/plugin/before/…/kafka_mem.qv`.
`crates/qcore-contract/tests/guarded_decomposition.rs` replays it and
asserts the refusal (`QF003`).

## 2. Finding: all fourteen symbols are refused, not seven

The sidecar's seven `plugin_*` prerequisite symbols are English too. The
legacy reader (`lift_bindings`) writes each as its typed atom's formula
(SP6 record). Those formulas are not typed renderings. They are typed atoms
(C04.18.7) over their own parameters (`#env: environment`, `#loaded:
predicate<environment,plugin,family>`, `#time: function<environment,event,integer>`,
…), quantified over `plugin`, `event`, `family` and `symbol`. A temporary
probe (not committed; log `private/logs/probe.log`) ran
`premise_formula::guarded_proposition` on each lifted formula. All seven are
refused with `QF003`, for example "`#actual_abi` is not a registered term of
the typed rendering". The replay refuses at `default_case` only because
bindings iterate alphabetically.

So restoring the positive plugin replay needs a typed-rendering vocabulary
for the plugin model, for all fourteen symbols. The typed atoms have 60
parameter names. Three of them clash with registered C04.18.9 terms in
other sorts:

* `#loaded`: registered `set(fn)`; plugin `predicate<environment,plugin,family>`.
* `#result`: registered `set(obj)`; plugin `function<environment,event,integer>`.
* `#from_on_new`: registered `set(fn)`; plugin `function<environment,event,integer>`.

Also needed:

* the value sorts environment, event, plugin, family, action, callback,
  member, context, handle, activation, resource, symbol, digest and abi;
* a function sort in the rendering (`RenderedSort` has only value and
  predicate today; G01p `Sort::Function` exists).

## 3. Decisions

**D1 (blocking, every symbol): the vocabulary.**

* **A (recommended).** Add an SP9 block to `RENDERED_TERMS`, C04.18.9 and
  Lean. It holds the plugin model's terms under the typed atoms' own names,
  with the environment explicit (`#env`, the selected
  `PluginPremiseDecomposition.Environment`). The three clashing names are
  spelled differently in the rendering only (`#plugin_loaded`,
  `#event_result`, `#event_from_on_new`). It adds the value sorts and a
  function sort.
  * Each prerequisite symbol is then its atom's formula, except at those
    three names. Lean proves each equal to the atom's existing definition.
  * No provider file, lock or production digest changes.
  * The legacy reader records all fourteen sidecar texts.
  * `check_rendering` gates only guarded symbols and tests, so reusing the
    names reclassifies no production atom.
* **A′.** As A, but every SP9 term is prefixed and the environment is
  implicit (as SP7's entry terms are). It reserves no generic name (`#time`,
  `#action`, `#member`) globally. Every prerequisite rendering differs from
  its atom's formula at every name, so the proofs are longer but not harder.
* **B.** Rename the three clashing parameters in the production typed atoms.
  The atoms are stated 1,162 times in six librdkafka provider files:
  `client.qv` 623, `null_deref.qv` 497, `logging.qv` 21, and 7 each in
  `kafka_mem.qv`, `memory.qv` and `queue_destructor.qv`. The prerequisite symbols then repeat their atoms'
  formulas byte for byte (the SP6 precedent).
  * This needs `contracts.py sync` and lock diffs. It moves hypothesis and
    contract digests and needs a new atom migration record.
  * It contradicts the brief's "no production module states these symbols".
* **C.** Change C04.18.6 so that a guarded symbol may also name the
  parameters of the typed atoms it depends on, at their declared sorts. It
  adds no terms.
  * The `QF003` admission becomes graph-dependent: the graph is plumbed into
    `guarded_proposition`, and Lean `guarded_admitted_iff_typed` changes.
  * A name then means different things in different declarations.

**D2: `default_lifetime_valid`.** The prose is "live code and opaque
context through the last invocation; no loaded-envelope truth follows". The
frozen Lean `GuardedContractApplications.Plugin` has one postcondition,
`lifetimePostcondition`. It is code and context live, plus `unloadOrder` of
the event's plugin.

* **b (recommended).** `codeLive ∧ contextLive` only, as a new Lean
  definition in a new file. It is proved implied by `lifetimePostcondition`.
  This is faithful to the prose: it states no unload order, which is
  loader-envelope truth the disclaimer withholds.
* **a.** `lifetimePostcondition`, the existing definition. It adds an
  unload-order claim the prose does not make, so it strengthens the
  postcondition against its disclaimer.

`registered_lifetime_valid` is `lifetimePostcondition` under either choice.
Its prose names the unload order, destruction and copied registrations.

**D3: `unregistered` (same shape as cJSON's `invalid_membership`).**

* **I2-shape (recommended; the 2026-11-23 precedent).** A free SP9 warrant
  term `#refutes_registration(#ev)`. It is sound: it entails Lean
  `unregistered e ev` (¬registered ∧ ¬default). No model derives it, and the
  absence of a registration proof does not give it. It is proved to entail
  the Lean definition, as SP7 did.
* **I1.** The Lean definition itself (¬registered ∧ ¬default). "Positive
  evidence" is kept only by the condition role's C04.18.5 rule.

**Follows precedent, not asked:**

* `environment` is a new SP9 open assumption `#trace_corresponds()` (E2).
  SP7's `#source_corresponds` meaning is cJSON-specific.
* `invoke` is a free SP9 admission `#admits_callback_closure()`, as
  `#admits_removed` is.
* The cases and postconditions are read at a designated invocation event
  `#ev`, as the Lean definitions take `(e, ev)`. The alternatives'
  exhaustiveness is Lean `candidate_premise_decomposes_both_alternatives`.
  A whole-trace reading would make `invoke ||-> registered_case /
  default_case` false for any trace that mixes registered and default
  callbacks.
* `default_case` does not assert ¬registered, so "independent of the
  loaded registration case" is kept.
