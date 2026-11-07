# CHAINED-ASSIGNMENT-INNER-DESTINATION

Card of the user decision of 2026-11-07 ("Register four as implementation
cards"), base `1e522ca5d`, measured against
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`. Acceptance: the
follow-up's DISCHARGE, "model or refuse a chained assignment's inner
destinations, measured by an arbiter", with `rd_kafka_sasl_safe_string`
(`rdkafka_sasl_scram.c:243` `d = safe = rd_malloc(len + 1)`, `:249`
`return safe`) leaving Unsafe and never reaching Safe by the fix alone.

## 1. The defect

C11 6.5.16p3: `a = b = E` assigns `E` to `b`, then the value of that
assignment (the value `b` now holds), converted to `a`'s type, to `a`. The
statement classifier (`legacy_classify.rs`, `classify_simple_base`) models one
write per assignment statement: the place `direct_assignment_destination`
names left of the statement's first `=`. On the call route it read
`d = safe = rd_malloc(n)` as `d = rd_malloc(n)` (`ContractCall { destination:
Some("d") }`): the write of `safe` was neither modelled nor refused. Probed on
the base with the `q_*` fixture contracts:

| Body | Base reading | Base result |
|---|---|---|
| `d = s = q_alloc(); return s;` | `q_alloc` bound to `d` only | Unsafe, `Leak` (false) |
| `d = (s = q_alloc()); return s;` | the same | Unsafe, `Leak` (false) |
| `void *d = s = q_alloc(); return s;` | the same | Unsafe, `Leak` (false) |
| `d = s = q_alloc(); q_free(s);` | `s` is a protected name, so the call route refused and rule 10s lowered `s = q_alloc(); d = s` | Safe |
| `x = y = 3;`, `a = b = s;`, `d = s = 0;` | refused by the older routes, lowered by rule 10s | unchanged |
| `p->start = p->header_val = p->i;` | admitted by the typed scalar-field route as one `IndirectScalarAccess` over `p` | Safe |
| `x = (x = 1);` | admitted by the scalar route as one write (finding `CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED`) | Safe |

So the drop was reached only where no older route refused: a contracted call
whose destinations are not released anywhere in the body (the release
inventory, `protected_resource_formals_under`, sends a released name to the
refusal and so to rule 10s). It accuses in the scram shape; the same drop
would leave a release or return through the inner name unmatched.

## 2. Census

`scripts/census.py` (run by `scripts/run_census.sh`; lexical, read-only)
scans every `.c` and `.h` of each route's source tree for an assignment whose
value holds another assignment, expands every `#define` whose body holds one
into the analysed unit files that use it, and joins each site to the function
rows of the base reports by function name and origin `file:line`
(diagnostics at the line; a leak by the acquisition line its description
names). Trees and reports: `current` = the user's run at `046accf6`
(`build/cycle19/clang/046accf6…/reports/resource-diagnostics.json`);
`corpus-fb`/`corpus-mk` = the pinned corpus trees at `66910c10` and Monkey
with `baseline:7b726587b`'s `eight-home` report; `system`, `slice` = their
prepared trees with that baseline's reports. `census.json` is the committed
record (whole-tree counts, every analysed site with its evidence).

Whole trees (`tree_counts`): `current` 1,390 sites (1,291 chains, 23
parenthesised chains, 65 assignments nested in another operand, 11 macro
expansions); the corpus Fluent Bit tree 1,206; Monkey 163; the system and
slice trees 1,197 each. Inside an analysed function: **33** sites in 23
functions of `current`, the same 33 in the corpus (Monkey's three functions
read from `corpus-mk`), **0** in system and slice. All 33 are chains; none is
a nested-operand write, a condition or a `for` clause.

How the base route reads each (destinations left to right; value `E`):

| Site | Function | Destinations | `E` | Base reading | Base row |
|---|---|---|---|---|---|
| `rdkafka_sasl_scram.c:243` | `rd_kafka_sasl_safe_string` | local / local | acquisition (`rd_malloc`) | **outer bound, inner dropped** (the defect): `leak` acquired at :243, exit :249; also `loop re-acquisition` at :243 | Unsafe |
| `mk_http_parser.c:1100`, `:1023`, `:1038` (two `header_scope_eq` expansions) | `mk_http_parser` | member / member | scalar | admitted as one `IndirectScalarAccess` over `p` (the typed scalar-field route reads every written place: path writes are custody-neutral, names are counted by `scalar_effect`); nothing custody-bearing dropped | Unknown |
| `cJSON.c:1540`, `:1698` | `parse_array`, `parse_object` | local / local | name (`new_item`) | refused (`unsupported assignment expression`), lowered by rule 10s as two `Alias` | Unknown |
| `rdkafka_mock.c:458` | `rd_kafka_mock_partition_pidstate_get` | member ×3 | scalar | refused (`indirect memory access`, the base is acquired here), lowered by rule 10s | Unknown |
| `cJSON.c:2008`, `lz4frame.c:1373`, `rdavl.c:141`, `rdkafka_mock.c:1336`, `rdlist.c:115`, `:117`, `pb_decode.c:1434`–`:1485` (8), `mk_http_parser.c:454` | 8 functions | member or deref in the chain | scalar, name, address, uncontracted call | refused: `indirect memory access is outside the resource projection` | Unknown |
| `rdkafka_ssl.c:410` | `rd_kafka_transport_ssl_cert_verify_cb` | local / local | uncontracted call | refused: `call to uncontracted function \`X509_STORE_CTX_get_error\`` | Unknown |
| `mk_kernel.c:68` | `mk_kernel_version` | local / local | `p + pos + 1` | refused: `unsupported assignment expression` (rule 10s declines pointer arithmetic) | Unknown |
| `rdxxhash.c:362`, `:821` | `XXH32_endian_align`, `XXH64_endian_align` | local / local | scalar | not compiled: under `XXH_ACCEPT_NULL_INPUT_POINTER >= 1`, defined 0 | Unknown |
| `rwlock_init` expansions (7: `rdavl.c:207`, `rdkafka.c:2587`, `rdkafka_sasl_oauthbearer.c:1304`, `:1799`, `rdkafka_telemetry_decode.c:537`, `rdkafka_topic.c:486`, `:653`) | 7 functions | member / member | scalar | not compiled: the chain is the `_TTHREAD_WIN32_` definition; on Linux `rwlock_init` is a call (refused at the line for its own access) | Unknown |

One defect site (scram) drops a custody-bearing inner write; three lines of
`mk_http_parser` are admitted by a route that reads every place they write;
every other analysed chain is refused or already lowered by rule 10s. The
census is lexical: it does not evaluate `#if` (the two "not compiled" rows
are read by hand), and a chain spelled through a macro defined by another
macro would be missed; the arbiter measurement covers both.
