# quirin-provider-fluent-otel-proto

Library crate `quirin-provider-fluent-otel-proto` (Rust library
`quirin_provider_fluent_otel_proto`) in the separate `providers/` workspace, on
the shipped contract load path (card OTEL-PROTO-PHASE-B1; USER DECISIONS
2026-11-27, "New provider crate", "Phase it, accept 7"). It owns the custody
contracts of the generated fluent-otel-proto functions the OpenTelemetry
consumer units call and of the protobuf-c runtime entry points they reach,
under `contracts/fluent_otel_proto/` (2 modules, 82 declarations). The crate is
data only; it links no provider runtime and is not an executable package.

* `generated.qv`: 78 declarations (52 `__init`, 9 `__get_packed_size`, 9
  `__pack`, 4 `__unpack`, 4 `__free_unpacked`). `__unpack` returns an owned,
  nullable message; `__free_unpacked` releases it (NULL a no-op);
  `__init`/`__pack`/`__get_packed_size` borrow every parameter.
* `runtime.qv`: `protobuf_c_message_unpack`, `_free_unpacked`, `_pack`,
  `_get_packed_size`, the same shapes.

Both modules and `protocol-identity.json` are written by
[`scripts/generate.py`](scripts/generate.py) from the vendored tree
(`lib/fluent-otel-proto/proto_c/` at Fluent Bit 66910c10, identical at
046accf6), which refuses any function whose prototype or body is not the
protoc-c 1.5 shape of its family; `--check` refuses a stale module or identity.
Every declaration is pinned in `contracts.lock` (`quirin-audit pin`) to its
generated definition and prototype, so a regenerated file holds every
declaration whose site changed.

`protocol-identity.json` is the basis the declarations rest on, adopted by the
user and the integrator: the vendored descriptors, which equal
opentelemetry-proto v1.10.0. It pins the opentelemetry-proto commit and
`.proto` digests of `docs/reports/otel-proto-audit/census.json`, and the
sha256 of every compiled `.pb-c.c`/`.pb-c.h` and of `protobuf-c.c`/`.h`
(the stale, uncompiled `pprofextended.pb-c.*` excluded). No libc name is
declared here (a unit test reads `quirin-provider-libc/libc-names.txt`).

The crate carries no checker: the shipped load path carries no checker
bindings, and user crates (which do) never apply on the shipped routes
(`providers/contracts.py`, user decision 2026-11-16). The generated-code half
of the identity is enforced at audit time by the pins; the protocol half by
`scripts/generate.py --check` and `scripts/otel_proto_audit.py identity
--check`. The protocol facts of an unpacked message are the OTP premise
formulas of `docs/reports/otel-proto-audit/formulas.json`; no verdict reads
them (OTEL-PROTO-NULL-CONDITIONAL-READING). Report:
[docs/reports/otel-proto-phase-b1](../../docs/reports/otel-proto-phase-b1/README.md).
