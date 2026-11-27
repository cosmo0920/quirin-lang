# W47-PREMISES-INSTANCE: frozen prediction

Frozen after BEFORE was measured and before any Rust, Lean, census, facts or
arbiter change of this card. Base: local `main` at `fe603d49` (the
worktree's HEAD was already there). No code of this card existed when this
was written.

## BEFORE (measured)

- Base binaries (release, `private/pi/bin-before/`): `loop_owner_census`
  `2bc42bb0…`, `typed_dispatch_facts` `4b7655a4…`, `predefined_macro_facts`
  `9923a0cc…`.
- Typed facts regenerated: `library.json` (`75f252d7…`) and
  `fixed-test.json` (`35dabc49…`) byte-identical to the
  W47-SERVICE-PROPERTY-KEYS archive; `previous-test.json` differs only in
  the generated copy's path, as every card since has recorded.
- Predefined-macro facts regenerated: `fixed.json` = `previous.json` =
  `9dd8bb73…`, byte-identical to the W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS
  archive.
- Census, six readings (`w47-loop-owner-predefined-macro-arms/scripts/census.py`):
  see the table below; each JSON compared by SHA-256 with that card's
  archive.

| Reading | discharged / not / unresolved | edges | functions | seeds | SHA-256 (= archive) |
|---|---|---|---|---|---|
| `90978ac6` configured | 11 / 130 / 7 | 33,529 | 11,932 | 2,852 | `719a7d0d…` |
| `90978ac6` configured, no predefined facts (control) | 8 / 130 / 10 | 33,573 | 11,951 | 2,854 | `4955a12d…` |
| `90978ac6` raw | 8 / 126 / 10 | 33,493 | 11,964 | 2,902 | `983c74b9…` |
| `~1` configured | 8 / 136 / 7 | 33,503 | 11,930 | 2,852 | `418a9251…` |
| `~1` configured, no predefined facts (control) | 8 / 136 / 7 | 33,546 | 11,949 | 2,854 | `a5edf1ed…` |
| `~1` raw | 8 / 132 / 7 | 33,466 | 11,962 | 2,902 | `d05ebc0e…` |

All six are byte-identical to the archive of the base's last census card.
At the configured `90978ac6` reading the 11 discharged sites are the fixed
test's 3 and 8 plugin sites (`in_docker_events` ×4, `in_event_test` ×2,
`in_fluentbit_metrics`, `in_stdin`).

## What the card derives (design, before code)

Each premise is derived from the pinned source at the census's configured
reading, or narrowed to a named residual. A new census mode
(`--derive-premises`) runs the derivations; without it every reading is the
base's (a control).

- **P-FRESH-CONTEXT**: `flb_create` stores the one `flb_config_init()`
  result in its context's `config`; `flb_config_init` returns its one fresh
  allocation after `mk_list_init(&config->inputs)` and
  `mk_list_init(&config->in_plugins)` at its top level, and `mk_list_init`
  leaves an empty list. Underived ⇒ the instance flag is underived (route
  kept).
- **P-REGISTERED-DESCRIPTORS**: the generated `flb_plugins_register`
  (`generated/include/fluent-bit/flb_plugins.h`, resolved by each of
  `src/flb_config.c`'s commands' `-I` order) is read: one `flb_malloc` /
  `memcpy(in, &OBJ, …)` / `mk_list_add(&in->_head, &config->in_plugins)`
  block per descriptor, called once by `flb_config_init` after the list's
  initialisation; the literal must match a registered descriptor. Any other
  generated function naming an input list, and every unlink of an input
  instance (`struct flb_input_instance::_head` used other than by the
  constructor's append), become hazards of the list-head derivation.
- **P-HEAD-INSTANCE**: narrowed. Derived: `flb_input_new` returns non-null
  only after appending its allocation to `config->inputs`, and a registered
  descriptor matches `"tail"`. Residual (stated): the context's and the one
  creation's constructors succeed (`flb_create` non-null, `flb_input_new`
  non-null), which the test checks with `TEST_CHECK` alone before the read.
- **P-INSTANCE**: per obligated site. Derived when the operation's instance
  argument is the calling callback's unassigned first parameter and every
  edge into the callback is a loop dispatch `C->cb_collect(C->instance, …)`,
  with a collector's `instance` written only by `collector_create`. A
  refuted shape (another instance passed, the parameter reassigned, a
  collector re-pointed) makes the site unresolved; an unrecognised shape
  (`ctx->ins`) keeps the stated premise, reported per site.
- **P-INSTANCE-FIELDS-BY-MEMBER**: the typed reading records every whole
  assignment or copy-initialisation of an instance or collector, every byte
  function over one and every conversion of a pointer to one; a byte write,
  a copy or a character/other-record view makes the flag underived and the
  collector-dependent sites refuted. Residual: a pointer converted to
  `void *` (counted) is not written through bytewise, and no offset from
  another member's address reaches these members.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| P-FRESH-CONTEXT at the export | derived | 85% |
| P-REGISTERED-DESCRIPTORS at the export | derived (in_tail_plugin registered once; no generated or unlink hazard reached from the region) | 65% |
| P-HEAD-INSTANCE | narrowed to the creations' success (derived: non-null ⇒ appended, a registered match) | 85% |
| P-INSTANCE, the fixed test's 3 sites | derived | 80% |
| P-INSTANCE, the other 8 discharged sites | stay discharged under the stated residual (`ctx->ins` form) | 70% |
| P-INSTANCE-FIELDS-BY-MEMBER at the export | no copy, byte write or character view of an instance or collector; residual `void *` conversions > 0 | 55% |
| fixed test ×3, configured, with the derivations | **discharged** on `(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)` under narrower premises | 55% (else unresolved with a named derivation failure, reported as the finding) |
| old test ×8 (`~1`), configured | **not discharged** (T) | 99% |
| other sites | no verdict change | 80% |
| raw readings | verdicts unchanged | 95% |
| edges / functions / seeds | unchanged (the derivations add no edge) | 90% |
| control readings (no `--derive-premises`) | byte-identical to BEFORE | 90% |
| typed facts | earlier kinds byte-identical; new kinds added (record views, instance links) | 85% |
| corpus / system / slice | unchanged (the census is on no audit route) | 95% |
| condvar/lifecycle census | untouched | 95% |
