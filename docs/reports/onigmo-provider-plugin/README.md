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

## 2. The crate

`providers/quirin-provider-onigmo` (namespace `onigmo`), laid out as the
libcurl crate and depending on `quirin-provider-plugin` for its checker
runtime (`Cargo.toml`: `quirin-provider-plugin = { path =
"../quirin-provider-plugin" }`; the conformance pin admits exactly
`qcore-schemas`, `serde`, `serde_json` and the SDK):
`contracts/onigmo/{init,names,regex,region,search}.qv`, `contracts.lock`,
`library-versions.json` (`onigmo.h`, `ONIGMO_VERSION_MAJOR`/`_MINOR`/`_TEENY`),
`quirin-plugin.json` (one binding per function, all on
`checkers/onigmo-check`), the checker `quirin-onigmo-check`
(`src/bin/onigmo_check.rs`, logic in `src/check.rs`), the pinned prototypes
`pinned/onigmo-6.2.0.json` (`scripts/pin_prototypes.py`) and the
documentation record `doc-snapshots.json` (`scripts/snapshot_docs.py`).

**What differs from libcurl and OpenSSL.**

* **Pinned, not flexible.** Fluent Bit vendors Onigmo 6.2.0 at `lib/onigmo`,
  so the bodies are in the audited tree, and every declaration is pinned to
  the sites `quirin-audit pin` finds there (`contracts.lock`: 9 pinned, 0
  flexible, reviewed at `046accf6`). A pinned declaration is in force on a
  strict subset of where a flexible one is, so this is the fail-closed choice;
  the version range is evaluated as well (rule 53). The scanner finds no
  definition of `onig_region_free`, `onig_match` or `onig_search`
  (§7, finding C-IDENTITY-SITES-AFTER-UNBALANCED-BODY): their identities are
  the `onigmo.h` declaration alone.
* **`documented "6.1.2"`, never `since`.** Onigmo's API reference
  (`lib/onigmo/doc/API` and `doc/API.ja`, both headed "Version 6.1.2
  2017/05/10", SHA-256 `f46442df…` and `b87b858b…`) names no version that
  added any function. The bound is the series of the reference examined (C06.2);
  no upstream history was fetched, so no lower bound is claimed. The checker
  refuses `since`.
* **The header's spellings, as the gateway reads them.** The checker maps the
  vendored header's spellings with the typedef map the pin script verified by
  `_Static_assert(__builtin_types_compatible_p(…))`; a pointer typedef
  (`OnigRegex`, `OnigEncoding`) is `ptr<mut, u8>`, because the gateway's ABI
  reader folds the typedef chain and carries no pointee qualifier through it
  (`legacy_declared_abi.rs`, `project_scalar`). The fixture run found this:
  `enc: ptr<const, u8>` was withdrawn with `abi-parameter-type-disagreement`.

## 3. The contracts, per function, with the body lines

Vendored at Fluent Bit `046accf6`, `lib/onigmo/` (file:line). Every
declaration also states `requires resource_contract_v1;` and
`requires resource_library_version(onigmo, documented "6.1.2");`.

| Function | Clauses | Read off |
|---|---|---|
| `onig_new(reg, pattern, pattern_end, option, enc, syntax, einfo) -> i32` | `resource_out(reg, nullable)`; `borrowed` pattern, pattern_end, option, enc, syntax, einfo; `retained_by(enc, global)`, `retained_by(syntax, global)`, `retained_by(pattern, einfo)`, `retained_by(pattern_end, einfo)`; `return_none()` | `regcomp.c:5882-5883` (`*reg = xmalloc(...)`, NULL → `ONIGERR_MEMORY`), `:5886-5893` (any nonzero code → `onig_free(*reg); *reg = NULL;`), `:5894` (`return r;`): the slot is NULL or the fresh object on every exit, fresh exactly on `0` (`ONIG_NORMAL`, `onigmo.h:627`). `:5849`, `:5851`: `reg->enc = enc; reg->syntax = syntax;` (kept until `onig_free`). `:5802-5806`: the error exit stores pointers into the pattern into `*einfo`. Names copied (`regparse.c:799`), exact string copied (`regcomp.c:5224`). |
| `onig_free(reg)` | `released(reg, nullable)`; `return_none()` | `regcomp.c:5548-5555` (`if (IS_NOT_NULL(reg)) { onig_free_body(reg); xfree(reg); }`), `:5533-5546` |
| `onig_search(reg, string, string_end, search_start, search_range, region, option) -> isize` | `owned(reg, nonnull)`; `borrowed` the four subject pointers and option; `owned(region, nullable)`; `return_none()` | `regexec.c:3792-3797` → `onig_search_gpos` `:3799-4170`: `if (region)` resize/clear `:3817-3818`; only `start > end \|\| start < str` `:3822` exits before `reg->anchor` `:3874`; `msa` automatic, `MATCH_ARG_FREE` `:4144`, `:4168`; results `s - str` `:4169`, `ONIG_MISMATCH` `:4141`, `:4159`, an error code `:3819` |
| `onig_match(reg, string, string_end, at, region, option) -> isize` | as `onig_search` | `regexec.c:3523-3556`: `MATCH_ARG_INIT` `:3531`, `if (region)` `:3539-3540`, `reg->enc`, `match_at` `:3546-3547`, `MATCH_ARG_FREE`, `return r;` `:3554-3555` |
| `onig_region_new() -> ptr` | `return_owned(nullable)` | `regexec.c:331-340` (`xmalloc`; `onig_region_init` `:319-329` allocates nothing) |
| `onig_region_free(region, free_self)` | `requires resource_literal_argument(free_self, 1)`; `released(region, nullable)`; `borrowed(free_self)`; `return_none()` | `regexec.c:342-355`: `if (r) { arrays freed; history_root_free(r); if (free_self) xfree(r); }`. doc/API "free_self: [1: free all, 0: free memory used in region but not self]" |
| `onig_name_to_backref_number(reg, name, name_end, region) -> i32` | `owned(reg, nonnull)`; `borrowed` name, name_end; `owned(region, nullable)`; `return_none()` | `regparse.c:916-938` (USE_NAMED_GROUP, `regint.h:69`): `name_find` `:541-552` reads `reg->name_table`; `IS_NOT_NULL(region)` `:930-933` |
| `onig_foreach_name(reg, func, arg) -> i32` | `owned(reg, nonnull)`; `borrowed` func, arg; `resource_callback_invoked(func, _, _, _, _, reg, arg)`; `return_none()` | `regparse.c:581-597` (USE_ST_LIBRARY, `regint.h:83`): `reg->name_table` `:586`, `onig_st_foreach(t, i_names, &narg)` `:594`; `i_names` `:562-578` calls `(*(arg->func))(e->name, e->name + e->name_len, e->back_num, group list, arg->reg, arg->arg)` and stops at the first nonzero result |
| `onig_init() -> i32` | `return_none()` | `regcomp.c:5903-5922`: the flag `onig_inited` (`:5815`) and `onigenc_init()` (`regenc.c:35-39`, `return 0;`) |

`onig_end` is **not** declared (§4).

**Why `owned(reg, nonnull)` and `owned(region, nullable)`.** The first fixture
run stated every use as `borrowed`, as the libcurl and OpenSSL crates do. A
borrowed position demands nothing, and the analyser's caller-side refusal
(`ALIAS-LOCALS-BORROW-LIVENESS`) leaves a possibly-null binding unaffected, so
`onig_new(&reg, …); onig_free(reg); onig_search(reg, …)` was **`Safe`**. Each body
dereferences `reg` on every path that reaches its work and documents it as a
"regex object", so the honest requirement is a live object the caller keeps:
`resource_owned(reg, nonnull)`. The region may be NULL ("NULL is allowed"),
and otherwise is resized in place, so `resource_owned(region, nullable)`.
Both refuse the use after free. The cost is precision only (§4).

**Result sets.** Every function's result classes are recorded in the
checker's table and the modules' comments as formulas over `#result`
(`onig_search`: `#result >= 0`, `#result == -1`, `#result <= -2`). No
`result_set` clause is stated: C09.4 admits one only from
`quirin-provider-libc` (an audit route holds one from any other crate), and
the classes partition the type and carry no custody.

**Lean.** No new clause kind: the crate uses C04.2's modes, C04.5's literal
premise, C04.14's retention, C04.19's synchronous invocation and C06.2's
range, whose models are `QCore.LiteralArgumentPremise`, `QCore.RetainedBy`,
`QCore.CallbackContracts`, `QCore.LibraryVersionRange` and
`QCore.ContractRevisionMode`. The new module
[`QCore.OnigmoContractPlugin`](../../../proof/QCore/OnigmoContractPlugin.lean)
([audit](../../../proof/AuditOnigmoContractPlugin.lean), 19 theorems,
`propext`/`Quot.sound` only, no project axiom, no placeholder) proves the
three readings the declarations rest on and the checker's composition:
`fresh_iff_normal`, `a_failure_leaves_null`, `every_exit_is_a_nullable_out`
(the slot); `the_declared_arm_agrees_with_the_body`,
`other_actuals_are_undeclared`, `the_zero_arm_is_not_a_release`,
`an_undeclared_region_free_is_never_safe` (the `free_self` split);
`global_admits_only_static`, `global_admission_holds_for_every_holder`,
`a_file_scope_address_is_refused_under_global` (the holder);
`one_failed_check_establishes_nothing`, `never_published_safe`,
`never_published_unsafe`, `published_is_pending_or_unknown`,
`an_established_binding_is_pending`, `caller_never_below_host`,
`a_bound_caller_is_never_safe`,
`a_safe_host_over_one_established_binding_is_pending`, `every_reason_is_kept`.

## 4. Refused shapes and their reasons, as formulas

Every refusal reason below is stated as the condition under which it fires.
Notation (ASCII, the C04.18.9 tokens plus `!=` and `||`): `#actual(f, p)`
is the call's actual at `f`'s parameter `p`; `#class(a)` its class in rule
12t's table (`static`: a string literal; `nothing`: a null pointer constant;
`caller`: an unrebound formal; `tracked`; `other`); `#holder(f, p)` the
holder `f`'s declaration names for `p`; `#custody(a)` the custody the body
holds of `a` (`live`, `nullable`, `released`, `none`); `#literal(a, k)` that
`a` is the decimal constant `k` (C04.5.1); `#candidates(a)` a callback
actual's candidate set and `#closed`, `#identity` the C04.19 conditions;
`#declared(f)` that `f` has a declaration in force.

| Shape | Reason (fires when) | Diagnostic |
|---|---|---|
| `onig_new` with Fluent Bit's `ONIG_ENCODING_UTF8`/`ONIG_SYNTAX_RUBY` (`&OnigEncodingUTF_8`, `&OnigSyntaxRuby`) | `#holder(onig_new, enc) == global && #class(#actual(onig_new, enc)) == other` (likewise `syntax`) | `retention-undecided` |
| `onig_new` with a derived or member pattern, a non-frame `einfo` | `#holder(onig_new, pattern) == einfo && (#class(#actual(onig_new, pattern)), #class(#actual(onig_new, einfo))) != (caller, frame)` | `retention-undecided` |
| a use of the regex after `onig_free`, a search on a failed compile's slot, a NULL regex, a regex read from a member or after a null test of the slot's variable | `#custody(#actual(f, reg)) != live` for `f in {onig_search, onig_match, onig_name_to_backref_number, onig_foreach_name}` | `arguments do not match resource contract` |
| no region (a null constant) at `onig_search`/`onig_match` | `#literal(#actual(f, region), 0) && #custody(#actual(f, region)) == none` (a null constant at `resource_owned(region, nullable)` is not admitted; precision) | `arguments do not match resource contract` |
| a region used after `onig_region_free` | `#custody(#actual(f, region)) == released` | `UnknownAlias` (the released binding) |
| `onig_region_free(r, 0)`, `onig_region_free(r, x)` | `#literal(#actual(onig_region_free, free_self), 1) -> #false` | `call to onig_region_free outside its declared literal-argument premise` |
| a callback that keeps a name pointer, an open callback actual | `#identity(#candidates(#actual(onig_foreach_name, func))) -> #false` or `#closed(#candidates(#actual(onig_foreach_name, func))) -> #false` | `callback-uncontracted` / `callback-effectful`, `callback-undecided` |
| any `onig_end` call | `#declared(onig_end) -> #false` | `call to uncontracted function onig_end` |

Accused (provisional `Unsafe`, a validated path, never presented as an
upstream defect): a fresh regex never released (`Leak`), `onig_free` twice
(`DoubleRelease`), a region never released (`Leak`), `onig_region_free(r, 1)`
twice (`DoubleRelease`).

**`onig_end` is not declared.** Its body (`regcomp.c:5957-5972`) runs every
function `onig_add_end_call` registered (`:5927-5955`, a global registry; no
caller in the vendored tree or in Fluent Bit) and clears `onig_inited`, and
doc/API says "It is not allowed to use regex objects which created before
onig_end() call". Stated as the formula the vocabulary lacks:
`quant all #o in #created_before(#call): #used_after(#o, #call) -> #false`.
No resource-v1 clause invalidates every live object of a library by a global
call, so the call stays an uncontracted callee (`ONIGMO-END-PROTOCOL`).

## 5. The checker

`onigmo.contract_check` (target `onigmo`, revision `0.1.0`) answers one
request per binding from what is compiled into it, as the libcurl checker
does, with five checks (`src/check.rs`): the module digest is a built one;
the prototype's spellings project to the contract's types; exactly one range,
`documented` the series of the function's `doc/API` section, admitting 6.2.0;
pinned (no revision marker) with exactly the table's premises (only
`onig_region_free`'s `free_self == 1`) and no string anywhere but the range's
versions; and the `ensures` clauses exactly the table's set, with the
section carrying each cited sentence. Every pass is a `Safe` claim without
evidence (`Pending`); any failure `Unknown`. **Every diagnostic is a formula**
(user decision 2026-11-25): a note is the conjunction of what was checked
(`onig_new: #prototype("…") == #signature(#contract) && #sha256(onigmo.h) ==
6217b2834613 && 6.2.0 in #admitted(resource_library_version(onigmo,
documented "6.1.2")) && #ensures(#contract) == #ensures(#body) && … &&
#result_classes == #result == 0 | #result != 0`), a refusal the conjunction of
the refuting facts ending `-> #false` (`onig_region_free: #ensures(#contract)
== {…} && #ensures(#body) == {…} && #ensures(#contract) == #ensures(#body) ->
#false`). The unit tests refuse 17 drifts: a non-nullable release, a
non-null out slot, a dropped retention, `since` for `documented`, another
series, a range excluding 6.2.0, another library, no range, a flexible
revision, two wrong signatures (`option: i32`, `enc: ptr<const, u8>`), the
`free_self` premise dropped or bound to `0`, a non-null region, the regex
read as borrowed, a string premise, a moved callback position.
