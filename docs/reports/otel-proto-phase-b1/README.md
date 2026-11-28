# OTEL-PROTO-PHASE-B1

USER DECISIONS 2026-11-27 (verbatim): "New provider crate", then "Phase it,
accept 7". Option 1, phase B1 of [OTEL-PROTO-AUDIT](../otel-proto-audit/README.md)
§6: a shipped provider crate for the generated fluent-otel-proto functions,
on the vendored v1.10.0 basis, with the user's acceptance of up to seven
custody rows moving. Base: the Phase A tip `7b918859c` (origin/main
`9903b027a`). Prediction: [PREDICTION.md](PREDICTION.md), committed first
(`dc8f5e543`). The OTP premises stay unread formulas; B2+B3 (the null model's
conditional reading) are registered as OTEL-PROTO-NULL-CONDITIONAL-READING.

## 1. The crate

[`providers/quirin-provider-fluent-otel-proto`](../../../providers/quirin-provider-fluent-otel-proto/README.md),
namespace `fluent_otel_proto`, a data crate on `providers/contract-load-path.json`:

| Module | Declarations | Shapes |
|---|---|---|
| `fluent_otel_proto/generated.qv` | 78: 52 `__init`, 9 `__get_packed_size`, 9 `__pack`, 4 `__unpack`, 4 `__free_unpacked` | `__unpack`: allocator, length, data borrowed, `resource_return_owned(nullable)`; `__free_unpacked`: `resource_released(message, nullable)`, allocator borrowed; the rest: every parameter borrowed, `resource_return_none()` |
| `fluent_otel_proto/runtime.qv` | 4: `protobuf_c_message_unpack`, `_free_unpacked`, `_pack`, `_get_packed_size` | the same shapes |

The declared set is every generated function an OpenTelemetry consumer unit
calls (cmetrics and ctraces codecs, `src/opentelemetry`, `in_`/`out_opentelemetry`,
the envelope processor, both cprofiles copies) and the runtime entry points
those wrappers reach: 82, inside the user's ~150 bound. Both modules are
written by [`scripts/generate.py`](../../../providers/quirin-provider-fluent-otel-proto/scripts/generate.py),
which reads each function's prototype from its generated header and its body
from its generated source and refuses either unless it is exactly the protoc-c
1.5 shape of its family (the five bodies are quoted in `generated.qv`'s
header); `--check` exits 0 on both pinned trees (66910c10, 046accf6) and 1 on a
stale module or identity. No libc name is declared (a crate test reads
`quirin-provider-libc/libc-names.txt`); allocation goes through the
protobuf-c allocator.

**Pin and identity.** Every declaration is pinned in `contracts.lock`
(`quirin-audit pin`, 82 pinned, reviewed at git `046accf6` and `66910c10`, the
lock tied to the live set's `046accf6`) to its generated definition
(`.pb-c.c`) and prototype (`.pb-c.h`): a regenerated or edited file holds each
declaration whose site changed (`contract-pin-mismatch`, Unknown), never an
optimistic reading. `protocol-identity.json` records the adopted basis (the
vendored descriptors = opentelemetry-proto v1.10.0), the census's pinned
opentelemetry-proto commit `3a6e438` and its 11 `.proto` digests, and the
sha256 of the 22 compiled files (10 `.pb-c.c`, 10 `.pb-c.h`, `protobuf-c.c`,
`protobuf-c.h`; the stale `pprofextended.pb-c.*` excluded). `check_identity`
(crate) lists every difference and a crate test refuses a changed commit, a
changed, missing or extra digest (`otp_identity_refused_unless_pinned`);
`generate.py --check` and `scripts/otel_proto_audit.py identity --check`
refuse a protocol clone or vendored tree that differs.

**No checker.** The user asked for a libcurl-style checker; the shipped load
path carries no checker bindings, and a user crate (which has one) never
applies on a shipped route (`providers/contracts.py`, user decision
2026-11-16, "User load path"). The checker's role here, refusing declarations
whose generated code or protocol identity changed, is carried by the per-site
pins (audit time) and the identity checks above (check time). Recorded as a
deviation.

**Registration.** `providers/Cargo.toml` (member and workspace dependency),
`quirin-provider-shared` (re-exports it with the other per-library data
crates: `fluent_otel_proto`, `CONTRACT_SETS`, its manifest test),
`providers/contracts.py sync`
(the load-path record with its resource view of both modules, and
`contract-inventory.lock`: 82 declarations, 272 clauses), `contract/corpus.json`
(root), `cycle19-resource-contracts.json` (both modules, so the corpus route
loads them) and `contract/profiles/fluent-bit/library-providers.json` (the
crate added to the cmetrics, ctraces and fluent_bit libraries, so the
`current` route loads them too).

## 2. Links

The linkage rule requires a `linkage_contract_v1` link where a declaration's
`Safe` or `Unsafe` rests on another crate's declaration (a body-checked
specification's callee, a frame, an inherit). No existing cmetrics, ctraces
or fluent-bit declaration reads a generated function (`providers/linkage-reliances.json`
and every contract module read), so no link is required and none was added: a
link that is no recorded reliance is refused (`cx_reliance_is_linked`). The
consumer bodies resting on the new declarations are analysed rows, not
declarations. The fixture
`providers/tests/test_contract_load_path.py::FluentOtelProtoLinkage` shows the
rule applies to the new crate: a body-checked cmetrics wrapper resting on the
real generated `__free_unpacked` passes linked and is refused unlinked.

## 3. Fixtures

| Fixture | What it pins |
|---|---|
| `crates/qcore-c-gateway/tests/fluent_otel_proto_contracts.rs` | the crate's modules load with libc's; an `__unpack` result released by `__free_unpacked` is Safe; never released Unsafe (leak); released twice Unsafe; handed to an undeclared callee never Safe; `__init` of a held `calloc` result returned is Safe, lost Unsafe |
| crate tests (`src/lib.rs`) | the index equals the tree; the identity pins the census's protocol and the 22 compiled files and names exactly the declared functions; a stale identity is refused; no libc name is declared |
| `providers/tests/test_contract_load_path.py::FluentOtelProtoLinkage` | an unlinked reliance on a generated release fails the linkage check |

**Deliberate pin updates** (a twelfth crate on the load path, a seventh
shared data crate): `compiler/tests/contract_inherit_adoption.rs` and
`contract_inherit_qcore_provider.rs` (the load-path crate list),
`crates/qcore-audit/tests/provider_selection.rs` (the cmetrics/ctraces/fluent_bit
closure and the crate count, 11 -> 12), `crates/qcore-contract/tests/contract_inherit.rs`
(the namespace derivation table), `providers/tests/test_contract_load_path.py`
(11 -> 12), `providers/conformance/tests/packages.rs` (shared data crates 6 -> 7)
and `quirin-provider-shared`'s manifest test (6 -> 7 linked crates).

## 4. Prediction and what was measured here

[PREDICTION.md](PREDICTION.md): the seven accepted custody rows
(`cmt_encode_opentelemetry_batch.c#metrics_batch_view_init`;
`ctr_encode_opentelemetry.c#initialize_export_service_request`,
`#initialize_instrumentation_scope`, `#initialize_resource`,
`#initialize_resource_span`, `#initialize_scope_span`, `#initialize_span`)
Unknown -> Safe on corpus and twelve-home; system and slice no move; no other
row, no `Safe -> non-Safe`, no new `Unsafe`. The routes were not re-run here
(no card gate; the corpus route alone took 3.7 h at r40); the integrator's
`quirin-audit measure --declared PREDICTION.md` decides, and a move outside the
table is a stop by the user's condition.

On `current` (not a pinned route, prose only): the same leaves disappear from
the in_/out_opentelemetry and `src/opentelemetry` units; a row there moves only
if a generated callee was its sole obstacle.

## 5. Follow-ups

`OTEL-PROTO-PHASE-B` amended (B1 done; what remains is
OTEL-PROTO-NULL-CONDITIONAL-READING), `OTEL-PROTO-VENDORED-BASIS` retired
(basis adopted), `OTEL-PROTO-NULL-CONDITIONAL-READING` registered
([integration-amendments.json](integration-amendments.json)). The amend and
retire target ids OTEL-PROTO-AUDIT registers: apply its amendments first.
