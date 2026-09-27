# `ctraces.span.dropped_counts.v1` — the first CTraces provider capability

Status: **design only**. No production Rust changed for this document, no
capability is registered, no verdict moves, and nothing here discharges any
obligation. The implementation is split into the follow-on units in §8.

Card: P06 (W5, Providers lane). Record and measurements:
[`docs/reports/remaining-p06/README.md`](../reports/remaining-p06/README.md).

## 1. Why a design and not an implementation

P06 permits implementation only if an adequate core checker already exists, the
way `qcore.boolean_field_contract.v2` already existed for ChunkIO and CMetrics.
A census of all 273 function definitions in the 17 pinned CTraces translation
units (§2 of the record) finds **no CTraces function that any implemented
checker accepts**, and the gateway's own importer confirms the refusal with
exact reason strings (`crates/qcore-verification/tests/ctraces_capability_boundary.rs`).

Implementing anyway would mean either widening a gate to fit source it was not
designed for, or shipping a capability whose only subject is
`ctr_opts_exit` — a function the resource projection already reports `Safe`, so
a certificate over it would discharge nothing. Both are fabricated progress.
The design below therefore names exactly one new core semantic and stops.

## 2. Subject

The four dropped-count setters defined in `lib/ctraces/src/ctr_span.c` at
fluent-bit `9f0fa5730fba1761ebaf14bcd31220a3a9be9d6e`:

| Function | Signature | Body | Declared in `ctr_span.h` |
|---|---|---|---|
| `ctr_span_set_dropped_attributes_count` | `void (struct ctrace_span *, uint32_t)` | `span->dropped_attr_count = count;` | yes |
| `ctr_span_set_dropped_events_count` | `void (struct ctrace_span *, uint32_t)` | `span->dropped_events_count = count;` | yes |
| `ctr_span_set_dropped_links_count` | `void (struct ctrace_span *, uint32_t)` | `span->dropped_links_count = count;` | yes |
| `ctr_span_set_dropped_link_count` | `void (struct ctrace_span *, uint32_t)` | `span->dropped_links_count = count;` | **no prototype** |

All four are leaves: the census records zero direct calls, zero indirect calls
and zero branches in each. They allocate nothing, release nothing, touch no
`cfl_list`, take no lock and cross no library boundary. That is what makes them
the smallest honest CTraces subject.

The family is not vacuous. `ctr_span_set_dropped_link_count` and
`ctr_span_set_dropped_links_count` differ by one character and write the **same**
member; the first has no prototype in the public header. A checker that
reproduces the written field from source distinguishes them, and a contract that
asserts `only_selected_field_written` is falsifiable against the wrong one. This
is a naming hazard in upstream source, recorded as an observation — it is **not**
a defect claim, because both setters are reachable and neither is wrong on its
own terms.

### 2.1 Explicitly unsupported CTraces semantics

Named so no later reader mistakes silence for coverage. None of the following is
in this capability, and none becomes reachable by implementing it:

- **Allocation and teardown.** `ctr_create`, `ctr_destroy`, `ctr_span_create`,
  `ctr_span_destroy`, `ctr_id_create`, `ctr_id_destroy`,
  `ctr_attributes_create/destroy`, `ctr_resource_span_destroy`,
  `ctr_scope_span_destroy`, `ctr_instrumentation_scope_destroy`,
  `ctr_link_destroy`, and every `otlp_*`/`destroy_*` helper in
  `ctr_encode_opentelemetry.c`.
- **Intrusive lists.** Every `cfl_list_init`/`add`/`del`/`foreach_safe` use.
  CTraces' list operations belong to CFL, a *different* library in the profile;
  the implemented `check_list_unlink` route was written for Monkey's
  `mk_list_del` and its helper, not for `cfl_list_del`.
- **CFL string handling.** `cfl_sds_create*`, `cfl_sds_destroy`,
  `cfl_sds_printf`, `sds_cat_safe`. 46 of the 420 uncontracted-call diagnostics
  are `sds_cat_safe` alone.
- **msgpack / mpack and OpenTelemetry protobuf encode/decode.** 123 mpack and 22
  protobuf-C uncontracted calls; `ctr_encode_opentelemetry.c` is the largest unit
  and one of only two CTraces units with a provisional `Unsafe`.
- **Everything a count describes.** Writing `dropped_events_count` says nothing
  about the event list, about whether events were in fact dropped, or about any
  invariant relating the two. The contract deliberately makes no such claim.
- **Signedness.** `ctr_span_set_flags` takes `uint32_t` and stores into an
  `int32_t` member (`struct ctrace_span::flags`). That implementation-defined
  conversion is out of scope; `ctr_span_set_flags` is **not** in the family, and
  the family's own binding is `uint32_t` to `uint32_t`, where no conversion case
  exists.

## 3. Premises

Carried by the contract as `requires` clauses and by the derived task's
assumption map. None is discharged by the checker.

| Premise | Owner | Meaning |
|---|---|---|
| `object_is_live` | caller | the `struct ctrace_span *` designates a live object for the call |
| `exclusive_object_access` | caller | no concurrent access to the object during the call |
| `c_uint32_matches_u32` | build/ABI | C `uint32_t` in the selected ABI is the QCore `u32` |
| `scalar_field__<name>` | profile | the declared member this function is contracted to write |
| declared source revision | outer audit | the imported bytes are the pinned revision's |
| Clang typed import is trusted | outer audit | unchanged from the Boolean-field route |
| source-projection policy is declared trusted | outer audit | unchanged from the Boolean-field route |

The premise set is deliberately the Boolean-field v3 set with
`field_is_boolean` replaced by `c_uint32_matches_u32` plus the field selector.
No new *trust* is introduced; the widening is in the value domain only.

## 4. Obligation kinds

The core derives exactly one obligation per selected function — the same
one-obligation-per-task shape E02 pinned for the Boolean-field capability, and
for the same reason: the checker returns its whole task coverage set, which is
sound only while a task carries exactly one obligation.

- `property`: the scalar field write, i.e. postcondition
  `field == assigned_count and only_selected_field_written and no_object_release`
  evaluated over the function's complete path set.
- `subject`: the function, scoped to the analyzed translation unit.
- `bounds`: `BoundedPaths { paths: 1 }`. A leaf with no branch has exactly one
  path, and the deriver must refuse — not widen — any candidate with more.
- `DerivationIncomplete` is recorded, and the obligation can never reach `Safe`,
  if any statement, expression or declaration in the body falls outside §5.

## 5. Witness checker `qcore.scalar_field_contract.v1`

A new checker beside — never replacing — `qcore.boolean_field_contract.v1|v2`.
`crates/qcore-audit/src/source_contract.rs` currently admits only those two
names, so the new name must be added there explicitly; a provider declaring an
unknown `certificate_checker` must keep failing closed.

**Accepted C.** Clang `qualType` exactly `void (struct R *, U)` where `R` is a
profile-declared record and `U` is the declared unsigned member type; exactly two
`ParmVarDecl` children and one `CompoundStmt`; the body is exactly one
`BinaryOperator` `=` whose LHS is a `MemberExpr` on the first formal selecting
the declared member and whose RHS is an lvalue-to-rvalue read of the second
formal; the member is a direct, non-bitfield, non-`volatile`, non-`_Atomic`
member of a `struct` defined exactly once whose every member declaration was
read; the member's `qualType` equals `U` exactly, so no conversion arises.
Anything else — a cast, a second statement, a call, a branch, a compound
assignment, an extra attribute — is `Unknown` with a keyed reason. This is the
same conjunctive-whitelist discipline C02 used, and it must be re-derivable from
the certificate's own content.

**Replay.** As on the ChunkIO/CMetrics routes and as tightened by P04: the
provider computes and proposes its own typed path projection; the core evaluates
the pinned contract against **its own sealed import** and only then requires
agreement. Agreement alone discharges nothing, and witness admission conveys no
provider authority.

**Certificate.** Names the function, the record, the selected member, its
declared type, the single path's write vector, the contract digest and the full
compile-task identity (G01/G02). An independent recheck must rederive every
field and must refuse either trusted premise through both the typed set and the
plain-string route.

## 6. Contract text

Archived, parseable, and pinned clause-for-clause by
`designed_ctraces_contract_parses_with_its_specified_vocabulary`:
[`docs/reports/remaining-p06/ctraces_span_dropped_counts.qcore`](../reports/remaining-p06/ctraces_span_dropped_counts.qv).

It is deliberately **not** in the canonical contract corpus. Landing it there is step 3 of
P06-I1 and moves the pinned corpus count in
`compiler/tests/qcore_contract_corpus.rs` from 59 to 60 modules — a deliberate
pin update, which P06 is not authorized to make.

## 7. Fixtures and the malicious-witness matrix

Specified concretely, to be built by P06-I3 (§8). Named here so the
implementation unit cannot quietly ship without them.

**Schema-only process fixture.** A `providers/conformance/src/bin/`
binary depending only on `qcore-schemas` and serde, in the style of
`sample_boolean_provider.rs`: it parses the v2 request, executes the declared
scalar-assignment program, and emits a `VerificationResponseV2` carrying the
computed path. It must answer `Unknown` — never a verdict — for any request
outside §5, and must reproduce byte-identical certificates against the
in-process route on all four subject functions, as P04 required of
`qcore.chunkio` 0.5.0.

**Rejection matrix.** Every row must be refused with no certificate and no
verdict change. Rows 1–13 are the E02 matrix re-instantiated on this capability;
rows 14–19 are specific to scalar assignment.

| # | Mutation | Must be refused by |
|---:|---|---|
| 1 | task digest rebound to another function's real task | outermost binding |
| 2 | obligation identifier permuted / foreign / another function's real id | core task derivation |
| 3 | one witness claiming coverage of two obligations | composition |
| 4 | same certificate submitted twice at composition | composition |
| 5 | evidence version unsupported, in both directions | version gate |
| 6 | analyzed function scope widened | task identity |
| 7 | imported function set widened | task identity |
| 8 | a used header changed, source and command unchanged | compile-task identity (G01) |
| 9 | declared input path changed (`__FILE__` sensitivity) | `source_input` (G02) |
| 10 | contract or profile bytes changed | dependency digests |
| 11 | proposal echoing assertion text / theorem family / verdict label | path comparison |
| 12 | oversized and degenerate proposals | bounded, no panic |
| 13 | v1 `Safe` envelope for the same subject | v1 firewall, terminal refusal |
| 14 | path claims the member of a *different* declared field | sealed-import comparison |
| 15 | path claims `dropped_links_count` for `ctr_span_set_dropped_events_count` | sealed-import comparison |
| 16 | witness for `ctr_span_set_dropped_link_count` replayed against `..._links_count` (same field, different function) | task subject binding |
| 17 | proposal asserting the *old* value, i.e. no write | contract evaluation |
| 18 | proposal claiming a second write, or a write to an unmodeled member | `only_selected_field_written` |
| 19 | provider absent, failing, timing out, or exceeding its declared limits | QP006 scoped `Unknown` |

**Negative source controls.** Held-out C fixtures the importer must refuse, each
with its own measured reason: a setter with a cast on the RHS; a setter that
writes two members; a setter whose member is a bitfield; a setter whose member
type is `int` while the formal is `uint32_t`; a setter with an early return; a
setter that calls a helper.

## 8. Follow-on implementation units

Created only for genuinely new core semantics, per the card. Each is independent
and none may land a stub that asserts `Safe`.

- **P06-I1 — core checker `qcore.scalar_field_contract.v1`.** Implement §5 in
  `qcore-verification` beside the Boolean-field route; add the new
  `certificate_checker` name to the `qcore-audit` admission list; land the
  contract module in `providers/qcore-provider-ctraces/contracts/ctraces/` and update the corpus pin from 59
  to 60 deliberately. Lean parity: the scalar-assignment path relation and the
  `only_selected_field_written` frame lemma, with no new axiom.
- **P06-I2 — gateway import for the two-formal unsigned unit ABI.** Extend
  `crates/qcore-c-gateway/src/boolean_source.rs`'s ABI selector and
  `validate_field_layout` to admit `void (struct R *, U)` with a declared
  unsigned member, *without* touching the existing `int`/`void` one-formal
  routes or their pinned reason strings. The refusals pinned by
  `crates/qcore-verification/tests/ctraces_capability_boundary.rs` change
  deliberately when this lands, and that test must be updated in the same
  change, not deleted.
- **P06-I3 — provider package and fixtures.** A `qcore.ctraces` package version
  that actually computes a projection, plus §7's schema-only fixture, rejection
  matrix and negative source controls. Until it exists, the shipped
  `qcore-provider-ctraces` must keep returning `Unknown`.
- **P06-I4 — profile and manifest repair (independent of I1–I3).** The shipped
  `qcore.ctraces` manifests declare
  `certificate_checker: "qcore.unsupported_assertions.v1"`, a name no code in
  this repository implements; the configured source-contract route would refuse
  the package for it. This is currently harmless — the built-in only ever
  answers `Unknown` — but it is a dangling identity that should either name a
  real checker or be recorded as intentionally unsatisfiable. Fixing it changes
  the shipped manifests and their digests, so it needs its own change.

## 9. What this design does not establish

It does not audit CTraces, does not promote any CTraces unit, does not change
the 11/50 CT mapping, and does not touch the pinned 185-unit artifacts. Even
fully implemented, the capability would give checked function contracts for four
of 273 CTraces functions, all inside `ctr_span.c`, whose raw unit would remain
`Unknown` on its other 36 functions. The two provisional `Unsafe` CTraces units
(`ctr_encode_opentelemetry.c`, `ctr_resource.c`) are untouched by it.
