# ONIGMO-PROVIDER-PLUGIN: prediction

Committed before the crate is registered or bound (base `5087f9295`; census
`171110a9c`). The contract modules, the pinned prototypes
(`pinned/onigmo-6.2.0.json`) and the documentation record
(`doc-snapshots.json`) exist in the worktree, read by no route.

## What can move

`providers/quirin-provider-onigmo` (namespace `onigmo`) is a providers
workspace member loaded **only** through the generated user load path
(`providers/user-crates-load-path.json`, rule 53), never through
`providers/contract-load-path.json`. The card changes no analyzer crate (its
one `crates/` change is a gateway test). The census (README §1) shows that no
unit of any measured route, and no unit of the user's 11-24 `current` run,
calls Onigmo, and none of them reads a user load path. So every measured route
is **verdict-identical** to its base:

```measure-prediction
route corpus
none
route twelve-home
none
route system
none
route slice
none
```

**`current` as the user runs it** (`quirin-audit current` with the profile's
contract manifest `cycle19-resource-contracts.json`, whose load path is the
shipped one): **no move**, 1,173 Safe / 3,750 Unknown / 11 Unsafe as on
11-24, for the same two reasons.

## The opt-in `current` run (prose)

`evidence/callers.sh` over the selection `src/flb_regex.c`, `src/flb_router.c`
(library `fluent_bit`, every configuration: the static and shared objects,
two compile commands each in the prepared database), the profile's default
modules as a provider selection, `without` on the shipped load path and
`with-*` on the user load path with `--provider quirin-provider-onigmo`.

Every contract is bound to the crate's checker, so under rule 10h a row that
applies one is at most **`Pending`** (`plugin-claim-uncertified`): **no row
becomes `Safe`**, and none moves into or out of `Unsafe`.

| Side | Probe configuration | Onigmo declarations | Predicted move against `without` |
|---|---|---|---|
| `with-selected` | the four selected compile commands (`QCORE_LIBRARY_VERSION_COMPILE_DATABASE`), every one passing `-I…/lib/onigmo` | all nine in force: 6.2.0, equal to the lock | below |
| `with-full` | the whole prepared database: 1,419 commands, 655 of which pass no `lib/onigmo` directory, so `#include <onigmo.h>` does not resolve there and no `/usr/include/onigmo.h` exists on this host | all nine held, `library-version-unreadable` | none: every Onigmo call held `Unknown` (`stale-evidence`), as an uncontracted callee is today |

`with-selected`, per function (each row once per configuration):

| Function | Onigmo calls | Predicted |
|---|---|---|
| `flb_regex_init` | `onig_init` | **Unknown → Pending**, exactly when `without` names the uncontracted `onig_init` as its only reason (the declaration states no parameter and no custody) |
| `str_to_regex` | `onig_new` | stays **Unknown**: `enc`/`syntax` are `&OnigEncodingUTF_8`/`&OnigSyntaxRuby`, retained by the `global` holder, which admits only string literals (`retention-undecided`); the reason changes from an uncontracted callee to that refusal |
| `flb_regex_create` | none directly (`str_to_regex` with `&r->regex`) | stays **Unknown** |
| `flb_regex_do` | `onig_region_new`, `onig_search`, `onig_region_free(_, 1)` ×3 | stays **Unknown**: `onig_search`'s `reg` is `r->regex`, a member of a formal, at `resource_owned(reg, nonnull)` (no custody this body shows) |
| `flb_regex_match` | `onig_search` | stays **Unknown**: the same member actual, and a null pointer constant at `resource_owned(region, nullable)` |
| `flb_regex_parse` | `onig_foreach_name`, `onig_region_free(_, 1)` | stays **Unknown**: `reg` is `r->regex`; the candidate `cb_onig_named` calls `s->cb_match` through a member, so its own row is not the identity |
| `cb_onig_named` | `onig_name_to_backref_number` | stays **Unknown** (the indirect `s->cb_match` call) |
| `flb_regex_results_release` | `onig_region_free(result->region, 1)` | stays **Unknown** (a member of a formal released) |
| `flb_regex_destroy` | `onig_free(r->regex)` | stays **Unknown** (a member of a formal released) |
| `flb_regex_exit` | `onig_end` | stays **Unknown**: `onig_end` is not declared |
| `router_match` (`flb_router.c`) | `onig_match` | stays **Unknown**: the member actual `match_regex->regex` and a null region constant |
| every other function of the two units | none | unchanged |

So the predicted move is at most `flb_regex_init` Unknown → Pending (two rows,
one per configuration), and no other status change. Reasons on the other
Onigmo-calling rows change from "call to uncontracted function `onig_*`" to
the contract-level refusals named above where the call is otherwise read.
