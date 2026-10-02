# MONKEY-VHOST-PROPERTY-CUSTODY: prediction (frozen before implementation code)

Written and committed before any Rust, Lean or contract text of this card.
Base: local `main` at `958e76593`. BEFORE measured there with this worktree's
release `quirin-audit` (sha256 `5e13ce75…`), scripts in [scripts/](scripts/).

## BEFORE (measured, base `958e76593`)

| Run | Functions (rows) | `resource-diagnostics.json` |
|---|---|---|
| Corpus arbiter (`66910c10`, `--jobs 4`) | 647 / 10 / 4,177 (leak 41, summary_conflict 2) | `ed384eb0…` (= the brief's main figure) |
| Fluent Bit system route (`046accf6`) | 83 Safe / 4 Unsafe / 361 Unknown rows | `6dad47cc…` |
| Tail-pause slice | 45 Safe / 1 Unsafe / 356 Unknown / 2 Pending rows | `9415c399…` |

Census (`scripts/census.py`):

* `call to uncontracted function mk_string_dup`: **16 corpus leaves on 12
  functions** (`mk_rconf_read` 1, `mk_rconf_section_add` 1,
  `mk_rconf_section_entry_add` 2, `mk_config_listen_parse` 2,
  `mk_http_error_page` 1, `mk_http_send` 1, `mk_vhost_create` 2,
  `mk_vhost_set_property` 1, `mk_mimetype_add` 1, `mk_mimetype_init` 1,
  `mk_vhost_read` 2, `mk_vhost_set_single` 1); 0 in the system route and
  the slice. Every one of the 12 is Unknown with other blockers.
  `mk_string_dup`'s own corpus row (`mk_core/mk_string.c`) is Safe and its
  in-unit callers take its derived summary.
* Rule 10d's withdrawal `publication … of X, which owns linked objects` (the
  shape this card decides): **2 corpus leaves** (`cio_chunk_open` :75,
  `flb_input_chunk_create` :2599), **2 system rows** (`flb_input_chunk_create`
  :2611, two configurations), 0 in the slice.
* Stores of an owned pointer into a field of a body-acquired object that is
  then published into a container reached from a formal: by reading the 38
  `mk_string_dup` call sites of the corpus Monkey units, `mk_vhost_set_property`
  (`ha->name`, published into `vh->server_names`), `mk_rconf_section_add`
  (`new->name`, published into `conf->sections`, then returned),
  `mk_rconf_section_entry_add` (`new->key`/`new->val` into an unchecked
  allocation), `mk_vhost_create` and `mk_vhost_set_single` (an alias
  published into a list inside a body-acquired host: W45's nested-container
  relation, not this card's), `mk_mimetype_add` (nested `type.data` places),
  plus the two non-Monkey owners above (`ch->name = strdup(name)`,
  `ic->chunk`/`ic->routes_mask`).
* DocumentRoot-like overwrites (an owned copy stored into a member of an
  object the caller holds, over a value the body does not release): by
  reading, `mk_vhost_set_property:502` (`vh->documentroot.data`),
  `mk_lib.c:356` (`server->conf_user_pub`), `mk_config.c:309`,
  `mk_vhost.c:379/455`, `mk_http.c:215/862/871`, `mk_plugin.c:189`. All are
  refused today (`indirect memory access` or an unresolved owner).
* Shared probe (`tip-contracts.sh`, contracts-only, private copy of the scratch
  tree): `mk_vhost_set_property` Unknown with 6 leaves (pins of `mk_mem_alloc`
  and `mk_list_add` not in force at Monkey paths, `mk_string_dup`
  uncontracted, `ha` unresolved, two indirect accesses at :502/:503);
  `mk_vhost_set` Unknown at tip (3 leaves: :525, :526, :533) and master (6).

The DocumentRoot branch, read against Monkey (`mk_server/mk_vhost.c:795`,
`mk_vhost_free_all` releases `documentroot` through `mk_ptr_free`, so the
member owns its copy; `mk_vhost_create` zero-allocates the host): a second
`DocumentRoot` (two pairs in one `mk_vhost_set`, or two calls) overwrites the
first copy without releasing it. That is a **leak candidate** at master and at
the tip (21a2c190, same body), by reading only.

## What this card changes

1. **`mk_string_dup` contract**, a new pinned module
   `providers/qcore-provider-libevent/contracts/monkey/string.qv`: borrowed
   `s`, `resource_return_owned(nullable)` (an obligation `mk_mem_free`/`free`
   discharges), written out (not `inherit libc::strdup`: `strdup(NULL)` is
   undefined, `mk_string_dup(NULL)` returns NULL; the custody statement is the
   same). Pinned with `quirin-audit pin` at the pinned Fluent Bit tree
   (`66910c10`) and the system tree (`046accf6`), and at the user's Monkey
   checkout (read only) together with `mk_mem_alloc` and `mk_list_add`, whose
   Fluent Bit-path pins are what keep the contracts-only probe out of force.
   The module is manifest-selected; contract-corpus pins move 123 → 124
   modules, 59 → 60 selected, 263 → 264 declarations.
2. **The carry rule** (gateway rule **12d**; `12c` is taken by the concurrent
   VA-LOOP-AND-POINTER-ARG card): a publication into a caller's (or static)
   container of an owner whose owned children are exactly its field links,
   each exactly live, linked on every path, with no unresolved effect, not
   itself published or holding links or contained children, carries those
   children into the container with it (`ESCAPED`), instead of rule 10d's
   withdrawal. An unlink of that owner hands every carried child back
   (`LIVE`), so a later release of the owner leaves the children owed
   (P-B08-SHALLOW unchanged). Any other owner keeps 10d's refusal. Lean
   module `QCore.CarriedPublication` with shared vectors.
3. **DocumentRoot**: no rule admits it. The store of an owned copy into a
   member of the caller's object stays refused (the function cannot be Safe);
   the leak candidate is registered for the user beside SUP04, never presented
   as an upstream defect.

## Predicted AFTER

* **Corpus: 647 / 10 / 4,177, no status move, no correction**; leak count 41
  and the 12 functions with a leak/violation leaf unchanged (no added leak or
  violation). `resource-diagnostics.json` changes (leaves move):
  * the 16 `mk_string_dup` leaves → 0; the poison they caused lifts, so the
    `unresolved resource identity` leaves that follow them in the same
    function (`ha` :498/:499, `halias`, `new`, `port`, `temp`, `name`) may
    vanish or be replaced by the next real blocker (an unchecked owner, a
    container in a body-acquired owner, an uncontracted callee);
  * `mk_vhost_set_property`: Unknown, the Name branch decided (no leaf at
    :493–:499), the DocumentRoot branch still refused at :502 (and :503);
  * the 2 `owns linked objects` leaves → 0 (`ch->name`'s `strdup` child and
    `ic`'s children are carried); both functions stay Unknown on their other
    blockers (`cio_chunk_open`'s uncontracted callees, `flb_input_chunk_create`'s
    `return` of a published object);
  * no leaf outside these 14 functions moves.
* **System route**: no status move; `flb_input_chunk_create`'s 2 rows lose
  the `owns linked objects` leaf and keep the return refusal; nothing else
  moves.
* **Slice**: byte-identical custody (no `mk_string_dup` call, no owner
  publication).
* **Probe** (after pinning the three helpers at the Monkey checkout):
  `mk_vhost_set_property` Unknown with only the DocumentRoot leaves
  (:502/:503); `mk_vhost_set` unchanged (Unknown at tip with its 3 leaves:
  blocker (3) stays, now for a real reason, the DocumentRoot candidate; at
  master 6). If the VA-LOOP card lifts (1) and (2), `mk_vhost_set` still
  waits on `mk_vhost_set_property`, which cannot be Safe.

A move outside this list is a correction to this prediction and is named in
the report.
