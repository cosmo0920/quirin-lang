# ONIGMO-PROVIDER-PLUGIN: Onigmo's regex contracts on the user-contract SDK

Base `5087f9295` (origin/main). Branch `r36-provider-onigmo`.

User decisions (2026-11-25, verbatim, relayed in the card brief): "We must
write down quirin-provider-onigmo with contracts of onigmo's regex
functions." and "This must be depending on quirin-provider-plugin crate and
using its plugin mechanism."

The template is [LIBCURL-PROVIDER-PLUGIN](../libcurl-provider-plugin/README.md)
and [OPENSSL-PROVIDER-PLUGIN](../openssl-provider-plugin/README.md); this
report states what differs.

## 1. Census (committed first)

**Sources.** The user's `current` run of 2026-11-24 09:58 (1,173 Safe /
3,750 Unknown / 11 Unsafe function rows,
`build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`,
read only), and the prepared checkout it audited
(`…/fluent-bit-clang-source/fluent-bit-clang-source`, Fluent Bit `046accf6`).
`rg -n '\bonig|\bONIG_'` over every `.c`/`.h` outside `lib/onigmo` finds two
units; nothing under `plugins/` names Onigmo. Fluent Bit's own callers of
Onigmo are its wrappers: 164 calls of `flb_regex_*` in 44 files of `src/` and
`plugins/`.

**Call sites, per function** (`src/flb_regex.c` unless named):

| Onigmo function | Sites (enclosing function) | Count |
|---|---|---:|
| `onig_new` | `:143` (`str_to_regex`, reached from `flb_regex_create`) | 1 |
| `onig_free` | `:317` (`flb_regex_destroy`) | 1 |
| `onig_search` | `:203` (`flb_regex_do`), `:282` (`flb_regex_match`) | 2 |
| `onig_match` | `src/flb_router.c:69` (`router_match`, under `FLB_HAVE_REGEX`) | 1 |
| `onig_region_new` | `:191` (`flb_regex_do`) | 1 |
| `onig_region_free` | `:211`, `:216`, `:227` (`flb_regex_do`), `:255` (`flb_regex_results_release`), `:307` (`flb_regex_parse`); every actual at `free_self` is the literal `1` | 5 |
| `onig_name_to_backref_number` | `:43` (`cb_onig_named`, the callback) | 1 |
| `onig_foreach_name` | `:306` (`flb_regex_parse`, callback `cb_onig_named`) | 1 |
| `onig_init` | `:157` (`flb_regex_init`, called once from `src/flb_config.c:574`) | 1 |
| `onig_end` | `:324` (`flb_regex_exit`) | 1 |
| `onig_named*` family, `onig_scan`, `onig_new_deluxe`, `onig_search_gpos` | none | 0 |
| `ONIG_*` macros | `ONIG_OPTION_*` (`:63-110`, `:208`, `:282`, `flb_router.c:73`), `ONIG_ENCODING_UTF8` and `ONIG_SYNTAX_RUBY` (`:146`, the addresses `&OnigEncodingUTF_8`, `&OnigSyntaxRuby` of file-scope objects, `onigmo.h:242`, `:514`), `ONIG_NORMAL` (`:148`), `ONIG_MISMATCH` (`:209`, `:284`): constants and object addresses, no call | — |

**How each call is read today.** Neither unit is audited by any route:

| Route | Units | `src/flb_regex.c` or `src/flb_router.c` among them | Rows naming an Onigmo callee or a `flb_regex_*` wrapper |
|---|---|---|---:|
| the user's `current` (11-24) | 187 (librdkafka 94, cmetrics 31, monkey 29, ctraces 17, chunkio 13, fluent-bit-chunk 2, monkey-libevent 1) | no | 0 |
| corpus (`eight-home`, `audit.json`'s 185-unit selection) | 185 | no | 0 |
| `twelve-home` | 179 | no | 0 |
| system (`system-selection.json`, 8 engine files) | 15 | no | 0 |
| slice (`tail-pause-selection.json`) | 16 | no | 0 |

(Every `contract/evaluations/*/baseline.json` and `evaluation.json`, and the
11-24 report with its three sidecars, were searched for `flb_regex`,
`flb_router` and `onig_`: the only hits are `-I…/lib/onigmo` include flags in
the contract sidecar's compile commands.) So **no Unknown row of any measured
route or of the user's `current` run is blocked by an Onigmo call**, alone or
with others: the yield of this card on those routes is zero by construction.
Two further reasons would keep it zero even for an audited unit: the crate is
reachable only through the user load path, which none of those routes reads
(rule 53; the profile's contract manifest `cycle19-resource-contracts.json`
names the shipped `contract-load-path.json`), and under rule 10h every
plugin-bound contract publishes at most `Pending`.

**The opt-in reading.** To read the calls as the analyser does today, the
card runs `quirin-audit current` over the two units alone
([`evidence/callers.sh`](evidence/callers.sh), selection
[`artifacts/onigmo-callers-selection.json`](artifacts/onigmo-callers-selection.json),
library `fluent_bit`, every configuration kept) on the shipped load path
(`without`); §6 records that run and the runs with this crate.
