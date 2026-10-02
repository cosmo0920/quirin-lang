# MONKEY-VHOST-PROPERTY-CUSTODY: `mk_string_dup` and the carried publication

Wave 48 supplemental card (round 12, High), 2026-10-02. Started on local
`main` at `958e76593` (BEFORE, prediction), rebased onto `bfbf0d673`
(VA-LOOP-AND-POINTER-ARG landed) and re-measured there; branch
`monkey-vhost-property-custody`, code tip `ec20edbe4`. Prediction frozen
first: [PREDICTION.md](PREDICTION.md) (`41fadc326` after the rebase,
`b6cce281a` before it), before any contract, Rust or Lean text.

## Why

The shared probe is `mk_vhost_set` at the tip of Monkey's
`cosmo0920-missing-va_end-on-null` (`21a2c190`). Its third blocker was the
uncontracted static `mk_vhost_set_property` (`mk_server/mk_lib.c:488-506`,
same body at master and tip), itself `Unknown` for an uncontracted
`mk_string_dup`, an unresolved `ha`, two indirect stores, and `mk_mem_alloc` /
`mk_list_add` pins naming only Fluent Bit's `lib/monkey/` paths. This card
makes it decidable and decides it: the `Name` branch is decided; the
`DocumentRoot` branch is a leak candidate and stays refused, so the function,
and with it `mk_vhost_set` at the tip, is `Unknown`.

## What changed

1. **`mk_string_dup` contract**, new module
   `providers/qcore-provider-libevent/contracts/monkey/string.qv`: borrowed
   `s`, `resource_return_owned(nullable)`, read against
   `mk_core/mk_string.c:319-337` (identical in the pinned Fluent Bit tree and
   in Monkey). Written out, not `inherit libc::strdup`: the custody statement
   is `strdup`'s, but `strdup(NULL)` is undefined and this body returns NULL.
   Pinned with `quirin-audit pin` ([scripts/pin.sh](scripts/pin.sh),
   [logs](artifacts/pin/)) at `66910c10` and `046accf6`, and at the user's
   Monkey checkout (`21a2c190`, read only: `pin` reads Git only; `HEAD` and the
   branch unchanged) together with `mk_mem_alloc` and `mk_list_add`, whose
   only identities were at `lib/monkey/` paths. Every Monkey site digest equals
   the Fluent Bit one, so these record the same reviewed text at a second
   source. Manifest-selected. Pins moved deliberately: `contract_loader.rs`
   123 → 124 modules, 59 → 60 selected modules and source assumptions,
   263 → 264 v1 declarations, `mk_string_dup` in the previous-mode table;
   `qcore_contract_corpus.rs` 123 → 124 sources and
   `("monkey/string.qv", "mk_string_dup", 3)`; `provider_selection.rs`,
   `neighbours.rs`, `prod_contracts.rs` 59 → 60; `test_contract_corpus.py`
   124 modules, 75 provider-owned. `monkey/memory.qv` was not extended: its
   declarations are pinned token for token by `contract_inherit_qcore_provider.rs`.
2. **Gateway rule [12d](../../agents/gateway.md#constraint-12d), the carried
   publication** (`crates/qcore-c-gateway/src/carried_publication.rs`, hooks
   `carried_children`, `handed_back`, `one_sided` in
   `legacy_container_publication.rs`; rule id after 12c, VA-LOOP-AND-POINTER-ARG's).
   With the contract, `ha->name = mk_string_dup(v)` is an O00-B08 field link,
   and rule 10d withdrew the publication of an owner with linked objects
   because an unlink could not hand them back. The rule carries each child
   that is exactly live, linked on every path, with no unresolved effect, not
   itself published and owning nothing; any other child, an untracked one
   included, keeps the withdrawal byte for byte. An unlink of the owner hands
   every carried child back and refuses when one may have been carried on some
   paths only; releasing the owner afterwards does not cascade
   (P-B08-SHALLOW). The rest of the `Name` branch is existing vocabulary: the
   null test of `ha`, the scalar `ha->len` store, `&vh->server_names` as a
   caller's container (10d). The nested-container relation
   W45-CONTAINER-ROOT-REACH names was not needed and is not added.
3. **DocumentRoot**: no rule admits it; see below.

## Lean

`QCore.CarriedPublication`, 27 theorems in the new
`proof/AuditCarriedPublication.lean` (audits 279 → 280): `carry_iff`,
`restore_iff`, each encumbrance and an untracked child refuse,
`publish_vectors_hold` and `unlink_vectors_hold` (bound to the Rust
`PUBLISH_VECTORS`/`UNLINK_VECTORS` by `tests/unit/carried_publication.rs`);
the `Name` branch `Safe` and `Unknown` under the withdrawal (non-vacuity); an
unlink returns owner and child, releasing only the owner then leaks the
child, releasing a carried child or the published owner is a violation;
refusals absorb and `the_carry_only_replaces_a_refusal`. No placeholders, no
project axioms.

## Tests

`crates/qcore-c-gateway/tests/monkey_vhost_property_custody.rs`, 9 tests on
the production `string.qv` and `intrusive_list.qv`: a leaked duplicate is a
leak, a released one `Safe`; the `Name` branch `Safe` with no diagnostic; a
partial link and a child released before the publication keep the
withdrawal; a carried child released while published is never `Safe`; an
unlink then a release of the owner alone leaks the child; the DocumentRoot
store is never `Safe`; Monkey's whole `mk_vhost_set_property` is `Unknown`
with every leaf on the two DocumentRoot lines. The full teardown after an
unlink is not expressible: the child read back through its member is a
borrowed alias (rule 10t) and rule 5k releases a member only through a
formal (`CARRIED-CHILD-TEARDOWN-AFTER-UNLINK`).

One pinned test changed deliberately: `retry_lifetime.rs`'s
`a_publication_of_an_object_that_owns_members_is_refused` pinned rule 10d's
withdrawal for an owner with one live `p_malloc` member, exactly the shape
rule 12d carries. It is now
`a_publication_of_an_object_that_owns_members_carries_only_live_ones`: that
owner is `Safe`, and the same owner with its member released before the
publication keeps the withdrawal (not `Safe`).

## DocumentRoot: a leak candidate, not `Safe`

`vh->documentroot.data = mk_string_dup(v);` stores a fresh copy over a member
of the caller's host without releasing what it held. By reading, the host
owns that copy (`mk_vhost_destroy`, `mk_server/mk_vhost.c:752`, releases it at
:795 through `mk_ptr_free`) and `mk_vhost_create` zero-allocates the host, so
the first store overwrites NULL and a second `DocumentRoot` for the same host
(two pairs in one `mk_vhost_set`, or two calls) loses the first copy. That
path is a **leak candidate** at master and at the tip `21a2c190`, by reading
only: not validated, not an upstream defect claim, registered for the user
beside SUP04 as `MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE`. The gateway keeps
its refusal at :502/:503 (`indirect memory access is outside the resource
projection`): no contract or tracked state says whose the member's old value
is. So `mk_vhost_set_property` cannot be `Safe`, and that is the answer.

Same family, read during the census: `mk_config_listen_parse` sets
`port = NULL` (`mk_server/mk_config.c:222`) after a failed `strtol` on its
`mk_string_dup` copy, and its `error:` label frees only a non-null `port`.
Also a reading only, in the same follow-up; the gateway does not reach it.

## Measurements

Corpus arbiter (`66910c10`, `--jobs 4`), Fluent Bit system route
(`046accf6`) and tail-pause slice, each BEFORE at `main` and AFTER at the
card's code ([scripts/measure.sh](scripts/measure.sh); censuses, change lists,
probe rows and digests in [artifacts/](artifacts/)):

| Run | BEFORE | AFTER |
|---|---|---|
| Corpus, base `958e76593` | 647 / 10 / 4,177, `ed384eb0…` | 647 / 11 / 4,176, `fd5a7e89…` |
| Corpus, base `bfbf0d673` (final) | 647 / 10 / 4,177, `7f7d8328…` (= the integrator's main) | **647 / 11 / 4,176**, `a7d68d49…` (leak 41 → 42, unsupported 29,131 → 29,110, unknown_alias 11,463) |
| System rows, both bases | 83 / 4 / 361, `6dad47cc…` | 83 / 4 / 361, `21b08170…` (unsupported 2,616 → 2,612, unknown_alias 1,117 → 1,119) |
| Slice rows, base `bfbf0d673` | 45 / 1 / 354 / 4 Pending, `74aa4732…` | custody identical, `59171e06…` (only the contract manifest and source digests in each unit's compile identity) |

The first AFTER slice (base `958e76593`, `6c9f5b45…`) is discarded: the
rebase changed the tree while it was queued behind the shared lock.

**Corpus attribution (base `bfbf0d673`, 15 functions change, identical to the
pair on `958e76593`):**

- **`mk_http_error_page` Unknown → provisional Unsafe** — a **correction to
  the prediction** and, by reading, **not a leak**. The leak is reported at the
  `return -1` of :399 for the copy acquired at :395. `temp` is joined from a
  refused `mk_ptr_to_buf` statement (:392) and the nullable copy; the null test
  at :398 refuses (`unresolved resource identity temp`) and, under rule 10b,
  narrows the binding but keeps the site, so the null edge's exit still holds
  a copy that on that path is NULL or was never made. Fail-closed holds; the
  published Unsafe is wrong and is registered as
  `MONKEY-REFUSED-NULL-TEST-JOINED-LEAK`, never as an upstream defect.
- The 16 `mk_string_dup` leaves → 0 over the 12 predicted functions, all still
  Unknown on other blockers: `mk_vhost_set_property` (only :502/:503 left),
  `mk_vhost_create` (now `h` unresolved at the alias publication into a list
  inside the body-acquired host, W45's owner relation), `mk_vhost_set_single`,
  `mk_rconf_read`, `mk_rconf_section_add`, `mk_rconf_section_entry_add`,
  `mk_config_listen_parse`, `mk_http_send`, `mk_mimetype_add`,
  `mk_mimetype_init`, `mk_vhost_read`.
- The 2 `owns linked objects` leaves → 0 (`cio_chunk_open`,
  `flb_input_chunk_create`: carried), both still Unknown.
- **`mk_rconf_section_get_key`**, not predicted: its `return (void *)
  mk_string_dup(entry->val)` (mk_rconf.c:757) now reaches the contract; the
  indirect-access leaf becomes `unresolved resource identity` and `nullable
  resource __qcore_returned_value used by a non-null contract`. Still Unknown.

**System:** `flb_input_chunk_create` loses its `owns linked objects` leaf
(2 rows); **`flb_output_task_singleplex_enqueue`**, not predicted, Unsafe →
Unsafe: its callee `flb_output_task_queue_enqueue` links the caller's `retry`
into `queued_task->retry` and publishes `queued_task` into the caller's queue,
so the W4 derivation (with `retry` bound) now carries it and derives a
summary, and the caller's `call to uncontracted function` leaf becomes
`unresolved resource identity retry` at :375 (2 rows); its counted-user
refutation (OR-1) is unchanged.

No Safe → non-Safe move, no other added leak or violation.

## Prediction against outcome

Predicted: no status move, no added leak; the 16 + 2 leaves gone in 14 named
functions; system only `flb_input_chunk_create`; slice custody identical; the
probe below. Outcome: the leaves, the slice and the probe as predicted; one
status move not predicted (`mk_http_error_page`, a false accusation exposed by
the contract, above); one corpus function and one system function change
leaves outside the named set (`mk_rconf_section_get_key`,
`flb_output_task_singleplex_enqueue`), each attributed.

## The probe (`mk_vhost_set` at the tip)

Private copy of the scratch tree, `tip-contracts.sh` contracts-only
([rows](artifacts/probe-rows.json)):

| Base | Function | BEFORE | AFTER |
|---|---|---|---|
| `958e76593` | `mk_vhost_set_property` (tip and master) | Unknown, 6 leaves (pins out of force at :493/:499, `mk_string_dup` :497, `ha` :498, :502, :503) | Unknown, **:502 and :503 only** |
| `958e76593` | `mk_vhost_set` at tip | Unknown (:525, :526, :533) | unchanged |
| `bfbf0d673` | `mk_vhost_set_property` | as above | Unknown, **:502 and :503 only** |
| `bfbf0d673` | `mk_vhost_set` at tip | Unknown, only `call to uncontracted function mk_vhost_set_property` (:533) | **unchanged: Unknown on :533** |
| `bfbf0d673` | `mk_vhost_set` at master | Unknown (5 `va_end` refusals, :532) | unchanged |

Blocker (3) is decided: it remains, now for a named reason, the DocumentRoot
leak candidate. `mk_vhost_set` at the tip is `Unknown`, which is the correct
answer until that candidate is refuted or the store is fixed upstream.

## Follow-ups

Registered (693 → 696): `MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE` (2 reading
candidates), `MONKEY-REFUSED-NULL-TEST-JOINED-LEAK` (1 corpus function),
`CARRIED-CHILD-TEARDOWN-AFTER-UNLINK` (0 audited sites). Amended:
`W45-CONTAINER-ROOT-REACH` (its entry blocker gone; the owner relation
re-measured unchanged, 8 corpus sites in 5 functions plus 1 owner-local, so it
stays open), `W44-CALLEE-CROSS-UNIT-SUMMARY` (its `mk_string_dup` part
discharged by a declared contract, 16 → 0 leaves), `SUP04-VARIADIC-UPSTREAM-CANDIDATES`
(pointer to the second Monkey candidate). Edits by
[scripts/update_ledger.py](scripts/update_ledger.py).

## Gate

See [gate.json](gate.json) and [gate/](gate/).
