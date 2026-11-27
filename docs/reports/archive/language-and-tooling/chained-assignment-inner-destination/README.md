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
| `mk_http_parser.c:1100`, `:1023`, `:1038` (two `header_scope_eq` expansions) | `mk_http_parser` | member / member | scalar | admitted as one `IndirectScalarAccess` over the formal `p` (the typed scalar-field route reads every written place: path writes are custody-neutral, names are counted by `scalar_effect`); nothing custody-bearing dropped. Read from a fixture of the shape (`QCORE_TYPED_PATH_PARITY`: `scalar_field_noop`) | Unknown |
| `cJSON.c:1540`, `:1698` | `parse_array`, `parse_object` | local / local | name (`new_item`) | refused (`unsupported assignment expression`), lowered by rule 10s as two `Alias` | Unknown |
| `rdkafka_mock.c:458` | `rd_kafka_mock_partition_pidstate_get` | member ×3 | scalar | refused (`indirect memory access`: the base is acquired in the body), lowered by rule 10s; read from a fixture of the shape | Unknown |
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

## 3. Rule

Gateway rule 5i is amended in place (`docs/agents/rules/5i.md`, "A write
inside an assigned value is never dropped"), with a pointer from rule 10s. No
new rule id: 40 is reserved, 41 is SCALAR-STATEMENT-FORMS'. The model is
C11 6.5.16p3: a chain is its sequential assignments, the inner one first, and
both names then hold the same identity (`Alias`, with rule 10m's
identity-preserving conversion). A chain is admitted only where each link is a
destination the route already models as a single assignment; a destination
the route refuses alone keeps the whole chain refused by name.

## 4. Implementation

- `crates/qcore-c-gateway/src/legacy_chained_assignment.rs` (new):
  `inner_write` finds, per top-level comma segment, the first top-level
  assignment and any simple or compound assignment operator inside its value
  (`==`, `!=`, `<=`, `>=` are not assignments; `<<=`, `>>=` are), a braced group being opaque (a statement expression's own statements, such as `container_of`'s `*__mptr = (ptr)`, are read by the routes that admit one; gate-1 failed 20 tests in five list-iteration and foreign-unlink units before this was so);
  `chained_assignment_guard` replaces an admission of such a statement by the
  refusal `chained assignment writes an inner destination too; it is read
  only as its sequential assignments (C11 6.5.16p3)` (the value is, up to
  wrapping parentheses, that assignment) or `assignment nested in an operand
  of an assigned value is not modeled` (any other position), restores the
  probe state (no site number moves) and leaves a refusal byte for byte.
- `legacy_classify.rs` (SCALAR-STATEMENT-FORMS' file; the minimal edit): one
  call of the guard in `classify_simple`, after the self-init routes and
  before the list-iteration guard. `legacy_c.rs`: the module declaration.
- `legacy_elaboration.rs`: rule 10s's `steps` lowers `T a = b = E;` (and
  `T a = (b = E);`) as the inner assignment, then the declaration from `b`,
  when Clang shows two distinct declarations, exactly as its existing
  assignment chain.

Every chain now reaches rule 10s, whose pieces are the older statement
lowering of `b = E` and of `a = b`. Probed after the change: the three false
leaks of §1 are `Safe` over the fixture contracts (`s = q_alloc(); d = s`);
`d = s = q_alloc(); q_free(s); q_free(d);` is the `DoubleRelease` it is;
`o->p = s = q_alloc()` and `s = o->p = q_alloc()` keep the gate's
`indirect memory access`; `s = s = q_alloc()` and `x = (x = 1)` (two
unsequenced stores to one object) keep the guard's refusal; `x = y = 3`,
`x = y += k`, `a = b = s`, `d = s = 0` lower exactly as before; the
scalar-field chains of `mk_http_parser` become one `IndirectScalarAccess` per
link on the same base (still Safe in the fixture).

The `x = (x = 1)` case is finding `CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED`,
whose DISCHARGE ("the older route refuses a same-object unsequenced pair by
name, or defers to the elaborator's order model") the guard meets; its pin in
`tests/clang_gap_phase1.rs` (`e04_…`) is flipped and the entry retired with
this card's.

## 5. Fixtures and Lean

`crates/qcore-c-gateway/tests/chained_assignment.rs` (route G, Clang, the
synthetic `q_*` contracts):

- `the_scram_shape_withdraws_the_leak_and_does_not_become_safe`: the scram
  body (two-pass loop, `char *safe = 0, *d = 0`, `*(d++) = *s`,
  `d = safe = q_maybe_alloc()`, `return safe`). No leak, `Unknown`, the
  `:214` declaration's `indirect memory access` kept, `safe` bound and
  `d` its alias. The old reading spelled out (`d = q_maybe_alloc()`) is
  `Unsafe` with the leak and `loop re-acquisition`, as the run's row is.
- `the_inner_name_holds_the_value_c_assigns_it`: `return s` after
  `d = s = q_alloc()`, three-link, parenthesised, declaration and `return`
  chains are Safe; the bare chain still leaks.
- `a_release_through_the_inner_name_is_matched`: release through the inner or
  outer name is Safe; through both is `DoubleRelease`, also for `a = b = s`.
- `a_chain_with_a_refused_destination_stays_refused_by_name`: member
  destinations keep `indirect memory access`; `s = s = q_alloc()` keeps the
  guard's refusal.
- `a_scalar_chain_is_unchanged`: `x = y = 3` is `ForgetScalar y; x = y`,
  Safe; `x = y += k`, `x = y = k` Safe.
- Unit tests of `inner_write` in the module (chains in every spelling,
  nested writes, comparisons and declarators that are not).

`proof/QCore/ChainedAssignment.lean`, audited by
`proof/AuditChainedAssignment.lean` (12 theorems, no axioms beyond the
standard ones, no placeholders): `chain_eq_sequential` and
`chainN_eq_sequential` (a chain equals its sequential assignments, with each
destination's conversion), `both_names_hold_the_identity`,
`the_old_reading_drops_the_inner_write`, the guard's `the_guard_never_admits`,
`the_guard_keeps_refusals`, `the_guard_refuses_every_inner_write`,
`the_guard_is_identity_elsewhere`, `the_guard_never_promotes`, and the
non-vacuity cases `the_scram_shape_is_dropped_by_the_old_reading`,
`the_scram_shape_is_its_sequential_assignments`,
`a_scalar_chain_is_sequential`.

## 6. Prediction

[PREDICTION.md](PREDICTION.md), committed before the semantic change: corpus
`rd_kafka_sasl_safe_string` Unsafe → Unknown, system and slice none; on the
`current` run the scram unit leaves provisional Unsafe for Unknown.

## 7. Gate

`python3 scripts/gate_scope.py --base 1e522ca5d --out private/gate-N` (mode
full: the diff reaches `qcore-c-gateway`; Lean all: a proof root changed).

- **gate-1** (head `c97de4997`), exit 1, kept as `gate-1-failed.json`:
  submodules, fmt and clippy 0; rust-suite 101 (617 units, 9,276 passed,
  **20 failed** in five units: `intrusive_list_iteration`,
  `element_destroyer`, `container_root_reach`,
  `refuted_contract_foreign_unlink`, `sr5_soundness`); both reevaluation
  probes 0; Lean 0 (417 audits); Python 0 (685 tests). Cause: the guard read
  `container_of`'s statement expression as a write inside the assigned
  value and refused rule 3's element binding. Fixed in `4d9895a10` (a braced
  group is opaque); the five units were rerun green before gate-2.
- **gate-2** (head `4d9895a10`), **exit 0**, `gate.json`: submodules 0, fmt
  0, clippy 0, rust-suite 0 (617 units, 9,296 passed, 0 failed, 16 ignored,
  135 filtered), `reevaluation_probe` 0, `reevaluation_followup` 0, Lean 0
  (417 audits), Python 0 (685 tests, 0 skipped): 8 steps passed. Logs are in
  the worktree's `private/gate-2/`. The timing-sensitive slot test did not
  fail.

`python3 scripts/apply_amendments.py
docs/reports/chained-assignment-inner-destination/integration-amendments.json
--check` exits 0. It retires `CHAINED-ASSIGNMENT-INNER-DESTINATION` and
`CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED`.

## 8. Measurement and follow-ups

The integrator measures `--side base=baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7
--side tip=rev:4d9895a10 --declared
docs/reports/chained-assignment-inner-destination/PREDICTION.md` and reruns
the user's `current` run after landing.

No new follow-up is registered. Two observations stay here. After the fix,
the scram fixture's remaining refusal comes from the pass loop's
path-insensitive scalar correlation (`safe = NULL` reaching `return safe`),
the class `LEAK-PATH-SCALAR-CORRELATION` already tracks. The census is
lexical: it evaluates no `#if`, and the two "not compiled" rows were read by
hand. The arbiter measurement is the evidence for any chain the census
could not see.

## Integrator measurement (2026-11-08)

Declared measurement, base `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`, tip `rev:4d9895a1067e952c52d0c30820ff50df917846e4`, run from the
main checkout (`private/measure/chained-20261108T103349Z`): exit 0, all three routes
complete, no unmatched rows, declared result **match**.

- corpus: `rdkafka_sasl_scram.c::rd_kafka_sasl_safe_string` Unsafe to Unknown; 280 rows changed content only.
- system: no row moved; 0 rows changed content only.
- slice: no row moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`84c9ca9dd935564b54972f24631b0166a990e2298c79805836cb67d3413a4a05`).

The one move is provisional Unsafe to Unknown (a false leak withdrawn, never Safe). Landing merge: the guard's hook in `legacy_classify.rs` merged without conflict with SCALAR-STATEMENT-FORMS and OUT-PARAMETER-BODY-SUMMARY.
