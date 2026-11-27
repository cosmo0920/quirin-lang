# OTEL-PROTO-AUDIT

USER DECISION 2026-11-27 (verbatim): "Also, we need to audit for otel related
modules. This can be validated with otel protocols which is cloned in
opentelemetry-proto where is the same heirachy of this qcore(quirin-lang)
repo."

Round 42. Base origin/main `9903b027a`. Phase A (the audit) is complete and
committed; it changes no analyzer route, no contract and no verdict. Phase B
(how the protocol becomes checked contracts) is proposed in §6 and is **not
implemented**: it waits for the user's choice.

Inputs (read-only): the protocol clone `/home/cosmo/Documents/GitHub/opentelemetry-proto`;
the pinned Fluent Bit tree `build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`
(HEAD `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, compile database sha256
`5e6868d9…f7e78`); the last landing measurement
`private/measure/r40-landing/sides/tip/` (quirin-audit built from `c0cb77da3`).
Everything below is in [census.json](census.json) (schema `otel-proto-audit/1`;
its per-function and per-site rows are in the gzipped
[census-rows.json.gz](census-rows.json.gz), which `load_census` merges back),
produced by [scripts/otel_proto_audit.py](../../../scripts/otel_proto_audit.py)
and [scripts/otel_proto_sites.py](../../../scripts/otel_proto_sites.py)
(§8 has the commands). The formulas are in [formulas.json](formulas.json).

## 1. Protocol identity

Commit `3a6e438003e673112efd4a00e9836c00672eae3c` (`git describe`:
`v1.11.1-19-g3a6e438`), 11 `.proto` files:

| File | sha256 |
|---|---|
| `opentelemetry/proto/collector/logs/v1/logs_service.proto` | `0252687fc8f59d139ced709d383e83678e9bd4957bdbe76077242f1c32363fa5` |
| `opentelemetry/proto/collector/metrics/v1/metrics_service.proto` | `2bb3c6adee5f7609c8a32c5e4b1ec57057320015e298e9cf136699a42abc24d7` |
| `opentelemetry/proto/collector/profiles/v1development/profiles_service.proto` | `5d2d5e2994fa72130474faaff0eee284857687efb339730451e3f22dc5cbf2eb` |
| `opentelemetry/proto/collector/trace/v1/trace_service.proto` | `03c8cc4e3e101087d884392d6eda32152ad5cd696e6344f50deaa59804a75c7a` |
| `opentelemetry/proto/common/v1/common.proto` | `4c8c143ac8c190544a3d31dcd5719d6fbc943b89037faa1cef6015c76cbdefc1` |
| `opentelemetry/proto/logs/v1/logs.proto` | `0c9d0c45eea723d38ef28ee13135ff61c037387f41cacc476ccac3485d6c138a` |
| `opentelemetry/proto/metrics/v1/metrics.proto` | `9663b2670f980041e73a71a7c09fdf4fe0bc64056be762f2fcbd2642c4d112a3` |
| `opentelemetry/proto/processcontext/v1development/process_context.proto` | `e755272f6761620f26e983d520ad9d0256d1e1bd37ff23775a72212332cda4cd` |
| `opentelemetry/proto/profiles/v1development/profiles.proto` | `231e9967a9e91ab41fc0a4dee970d2f51c35ca3a52e132bc8f606f1ae1ec39a8` |
| `opentelemetry/proto/resource/v1/resource.proto` | `34fe1c26b8959f09d97656348ae03b105d15a5f07ada0f8f1614ef5dff7d1569` |
| `opentelemetry/proto/trace/v1/trace.proto` | `41613c59a81c1742a9d2d7c7b8f6ddc7a4bae27438ef7a01ff464c240434dddf` |

`otel_proto_audit.py identity PROTO_ROOT --check census.json` exits 1, naming
every difference, unless the commit, the file set and every digest equal the
pinned ones (`otp_identity_refused_unless_pinned`, `otp_pinned_identity_admitted`);
on the clone at this tip it exits 0. A later use of the pinned protocol rests on
that check and never reconciles a difference.

## 2. Coverage census

Function inventories come from Clang's AST of each unit (the pinned compile
database; every main-file definition with a body). Route rows come from the
four pinned routes' `resource-diagnostics.json` (custody) and `*-null.jsonl`
(null property and combined verdict). For the five measured units the AST
inventory equals the route's `source_functions` exactly (the report refuses
otherwise).

**Only the cmetrics and ctraces OTLP codecs are measured, and only on corpus
and twelve-home.** system and slice carry no OTel unit. corpus and twelve-home
give identical rows for every OTel function. Verdicts are Safe/Unknown/Unsafe
counts on corpus (= twelve-home):

| Module | Unit | Functions | Routes | Custody | Null | Combined |
|---|---|---|---|---|---|---|
| cmetrics OTLP codec | `lib/cmetrics/src/cmt_decode_opentelemetry.c` | 40 | corpus, twelve-home | 1/39/0 | 11/29/0 | 1/39/0 |
| cmetrics OTLP codec | `lib/cmetrics/src/cmt_encode_opentelemetry.c` | 100 | corpus, twelve-home | 26/74/0 | 60/40/0 | 20/80/0 |
| cmetrics OTLP codec | `lib/cmetrics/src/cmt_encode_opentelemetry_batch.c` | 19 | corpus, twelve-home | 4/15/0 | 9/10/0 | 3/16/0 |
| ctraces OTLP codec | `lib/ctraces/src/ctr_decode_opentelemetry.c` | 16 | corpus, twelve-home | 6/10/0 | 1/15/0 | 0/16/0 |
| ctraces OTLP codec | `lib/ctraces/src/ctr_encode_opentelemetry.c` | 70 | corpus, twelve-home | 14/56/0 | 39/26/5 | 14/51/5 |
| **measured total** | 5 units | **245** | | 51/194/0 | 120/120/5 | 38/202/5 |

Reason class of every Unknown (functions carrying the class; one function can
carry several; custody from the closed `reason_class` map of the diagnostic
text, null from the candidate checks' source kinds):

| Unit | missing contract | unsupported construct | unresolved alias | null: formal | null: memory field | null: other |
|---|---|---|---|---|---|---|
| `cmt_decode_opentelemetry.c` | 35 | 20 | 15 | 12 | 27 | 0 |
| `cmt_encode_opentelemetry.c` | 62 | 58 | 52 | 19 | 27 | 2 (memory field + null literal) |
| `cmt_encode_opentelemetry_batch.c` | 12 | 9 | 9 | 5 | 7 | 0 |
| `ctr_decode_opentelemetry.c` | 10 | 5 | 4 | 15 | 9 | 1 (uncontracted result) |
| `ctr_encode_opentelemetry.c` | 52 | 37 | 38 | 20 | 18 | 2 (uncontracted result) |

No custody Unknown is held for review or carries an unproven-call-effect text;
no text falls outside the closed classes (`other:*` is empty). The five
`ctr_encode_opentelemetry.c` Unsafe rows are the null property's provisional
findings (`ctr_variant_binary_to_otlp_any_value`: a NULL-literal result
dereferenced at :492; `otel_span_set_links`, `otel_span_set_status`, `set_event`,
`set_events_from_ctr`: unchecked `calloc` results at :691-:810). They concern
allocation results, not a protocol field; they are provisional and are not
presented as upstream defects.

**Unmeasured, never counted as covered** (no row on any pinned route):

| Module | Units | Functions |
|---|---|---|
| fluent-otel-proto generated descriptors (`proto_c/opentelemetry/**/*.pb-c.c`) | 11 | 348 |
| fluent-otel-proto protobuf-c runtime (`proto_c/protobuf-c/protobuf-c.c`) | 1 | 90 |
| fluent-otel-proto helpers (`src/fluent-otel.c`) | 1 | 1 |
| `src/opentelemetry/` | 6 | 219 |
| `plugins/in_opentelemetry/` | 6 | 60 |
| `plugins/out_opentelemetry/` | 4 | 70 |
| `plugins/processor_opentelemetry_envelope/` | 1 | 7 |
| *adjacent, not named by the brief:* `lib/cprofiles/cprof_{encode,decode}_opentelemetry.c` | 2 | 97 |
| *adjacent:* `plugins/processor_content_modifier/cm_opentelemetry.c` | 1 | 5 |

Of the brief's modules, 245 functions are measured and 795 are not. The
corpus route also holds librdkafka's nanopb-generated
`opentelemetry/{common,metrics,resource}.pb.c` as `data_only` units with no
function (outside this audit). The compile database builds the root copies
`lib/cprofiles/cprof_{encode,decode}_opentelemetry.c`, not `lib/cprofiles/src/`
(recorded on CPROFILES-STRAY-DUPLICATE-TREE).

## 3. Protocol conformance (fact table)

The generated descriptors (`ProtobufCFieldDescriptor` arrays, message and enum
descriptors of every compiled `.pb-c.c`) were read back and compared with the
pinned `.proto` declarations: every message and enum, and per field its number,
protobuf-c type (so the wire type), label, presence (proto3 implicit, explicit
or repeated), oneof, packing and deprecation; per enum every value. protobuf-c
spells a singular sub-message and a oneof member `PROTOBUF_C_LABEL_OPTIONAL`;
that is read back as the `.proto`'s label and recorded as `pbc_label`.

Against the pinned protocol (61 messages, 7 enums upstream; 60 and 7 vendored):

| # | Declaration | Difference |
|---|---|---|
| D1 | `opentelemetry.proto.trace.v1.Span.type` (field 17, `string`, implicit presence) | absent in the vendored code: the decoders cannot see it, the encoders cannot emit it |
| D2 | `opentelemetry.proto.processcontext.v1development.ProcessContext` (2 fields) | message absent in the vendored code (its file `process_context.proto` has no generated counterpart); no Fluent Bit unit names it |

No field is renumbered, retyped, relabelled, moved between oneofs or
repacked; no enum value differs. The pinned protocol marks no field or value
`[deprecated = true]`, so no deprecated field is in use.

**Which protocol the vendored code was generated from.** Compared with every
tag of the clone, the compiled descriptors equal **v1.10.0 exactly** (0
differences); v1.11.0 and v1.11.1 differ only by D2, the pinned commit by D1
and D2, v1.9.0 by 13 differences, v1.8.0 by 34, older tags by 41 to 103
(`conformance.by_revision` in the JSON). The headers were generated by protoc-c
1.5.2 (`#elif 1005002 < PROTOBUF_C_MIN_COMPILER_VERSION`), the vendored runtime
header is 1.5.0 (`PROTOBUF_C_VERSION_NUMBER 1005000`; the guard requires only
`>= 1003000`).

| # | Vendored-only fact |
|---|---|
| D3 | `proto_c/opentelemetry/proto/profiles/v1development/pprofextended.pb-c.c` is a stale file: generated by protoc-c 1.4.1, absent from the compile database, it defines 8 message names `profiles.pb-c.c` also defines (`Profile`, `Sample`, `Location`, …) plus `AttributeUnit`, `Label` and `AggregationTemporality`, which no tag after v1.3.x declares. The comparison refuses a twice-generated name and excludes this file by name. |

## 4. Protocol-derived obligations and their sites

**The protobuf-c facts** (vendored protobuf-c 1.5, `protobuf-c.c` sha256
`e43a2e4f…34ce`). After a successful `protobuf_c_message_unpack` (:3028; any
member's failure fails the whole unpack, which returns NULL):

* a singular sub-message is NULL exactly when the wire did not carry it
  (`parse_required_member`, :2593-:2624; `message_init_generic` :2945 leaves the
  default NULL);
* a repeated field has `n_x` elements in an array allocated for them; each
  message element is unpacked into its slot before `n_x` is incremented
  (`parse_repeated_member` :2710), so no element below `n_x` is NULL, and the
  array pointer is NULL when `n_x` is 0;
* a oneof's `*_case` is set to the member's field number only after the member
  parsed (`parse_oneof_member` :2633); only that member is valid, and a message
  member it selects is non-NULL;
* a string is never NULL (allocated, or the descriptor default
  `protobuf_c_empty_string`), but may be empty; a `bytes` field's `data` is NULL
  exactly when `len` is 0 (:2567-:2592);
* an enum field keeps any `int32`, named or not.

The representation's facts are proved in Lean; the unpack postcondition is the
premise `WellFormed` on the C runtime (§5).

**Obligations.** O0: an unpack result is tested before it is dereferenced.
O1: a singular (or oneof) sub-message pointer is tested before it is
dereferenced or handed to a callee that dereferences it. O2: a repeated field's
array is subscripted below its count (directly, or by a callee handed the array
with its count). O3: a oneof member is read only under its case. O4: a `bytes`
field's `data` is dereferenced only when `len > 0` (or `data` tested). O5: a
decoded enum value used as an index (or handed to a callee) admits unnamed
values.

**The reader** (`otel_proto_sites.py`) walks each function's Clang AST and
records every site of O0-O5. A site is `guarded` when a guard establishes the
fact on every path to it, and the guard is cited (`detail`: kind and line): an
enclosing `if`/`while`/`for`/`?:`/`&&`/`||` condition on the path, an earlier
statement of an enclosing block whose `if` branch always exits, a `switch` on
the case discriminant whose every reaching label selects the member, or a
callee in the same unit that tests (or bounds) its parameter before every
dereference. An assignment to the tested expression or a prefix between the
guard and the site, or anywhere in a loop around the site the guard lies
outside of, kills the guard. A site is a `candidate` when no guard is found
and its function is reachable from an unpack of wire bytes in its unit
(through an unpacker or a wrapper returning one); otherwise it is
`not-determinable` with a named reason. Stores into a field are no sites;
releasing or resizing a field's storage (`free`, `flb_free`, `realloc`, …)
reads no element. The reader is syntactic: it does not see a write through
another alias of the message, a callee's side effect on a tested field, or a
`goto` into a guarded block, and a guard it cannot recognise leaves a site
unguarded. The fixture `scripts/tests/fixtures/otel_proto/otel_sites.c` pins
both directions: its four unguarded wire sites are candidates and its guarded
ones are cited.

**Sites** (guarded / candidate / not-determinable; protobuf-c runtime and
generated code carry no site of their own):

| Module | O0 | O1 | O2 | O3 | O4 | O5 | Sites |
|---|---|---|---|---|---|---|---|
| cmetrics OTLP codec | 4/0/0 | 116/0/2 | 65/0/33 | 92/0/46 | 5/0/1 | - | 364 |
| ctraces OTLP codec | 3/0/0 | 13/0/5 | 14/0/12 | 16/0/11 | 0/0/7 | 0/0/2 | 83 |
| in_opentelemetry | 4/0/0 | 33/0/0 | 17/0/0 | 7/0/0 | 0/0/1 | 0/0/1 | 63 |
| out_opentelemetry | - | 21/0/4 | 4/0/7 | 8/0/9 | 5/0/0 | - | 58 |
| src/opentelemetry | - | 0/0/2 | 2/0/6 | 0/0/9 | 1/0/1 | - | 21 |
| *adjacent:* cprofiles OTLP codec | 1/0/0 | 18/0/4 | 56/0/54 | 10/0/11 | 11/0/4 | - | 169 |

758 sites: 526 guarded, **0 candidates**, 232 not determinable. Reachable from
wire input (the decoders: `cmt_decode_opentelemetry.c`,
`cmt_encode_opentelemetry_batch.c`'s split path, `ctr_decode_opentelemetry.c`,
`in_opentelemetry/opentelemetry_logs.c`, `cprof_decode_opentelemetry.c`): 326
sites, 307 guarded, 19 not determinable, none unguarded. The decoders test
every sub-message before use, bound every array by its count, and read every
oneof member under its case. `src/opentelemetry`, `out_opentelemetry` and the
encoders build their own messages: their unguarded sites are not determinable,
never candidates.

| Not-determinable class | Reason codes | Sites (wire / off wire) |
|---|---|---|
| N1 encoder construction | `encoder-constructed-value` | 0 / 173 |
| N2 callee chains | `callee-passes-parameter-on`, `callee-variadic-or-deep` | 1 / 16 |
| N3 arrays without their count | `array-passed-without-its-count` | 0 / 15 |
| N4 bytes to a callee | `bytes-passed-with-length-to-callee`, `-to-unit-callee`, `bytes-passed-without-length` | 8 / 5 |
| N5 cross-unit callees | `callee-outside-unit` | 7 / 4 |
| N6 enum values to callees | `enum-to-callee-outside-unit` | 3 / 0 |

The 19 wire-reachable ones: `bytes` handed with their length to
`ctr_span_set_trace_id`/`_span_id`/`_parent_span_id`, `ctr_link_create`,
`cfl_variant_create_from_bytes`, `msgpack_pack_bin_with_body` and the unit's
`convert_bytes_value` (8; correct only if each callee accepts `data = NULL`
with `len = 0`); arrays and sub-messages handed to callees in other units
(`decode_histogram_data_point` :1031 `explicit_bounds`; cprofiles' attribute
and dictionary tables, 7); `cprof_decode` :772 (a deep chain); and decoded
enums handed on (`otel_span->kind` to `ctr_span_kind_set`,
`otel_span->status->code` to `ctr_span_set_status`, `severity_number` to
`msgpack_pack_uint64`; 3).

**Cross-check with the null property.** On corpus, 17 of the 120 null-Unknown
OTel rows carry only candidates that are protocol-shaped in a wire-reachable
function: a candidate at the line of a guarded site on the same field path (an
array element, a oneof member, a sub-message), an element reached through such
a path, or a dereference of a formal of an OpenTelemetry message type. The null
model already reads many guards (`tested:LINE`); what it lacks there is the
protocol facts themselves (an element below the count is non-NULL, the array
is non-NULL when the count is positive, a selected oneof message member is
non-NULL).

## 5. The obligations as formulas

Every obligation, premise, class and refusal is a formula of the C04.18.9 typed
rendering over the new **OTP block** (35 terms, 5 sorts `otp_msg`, `otp_field`,
`otp_pos`, `otp_site`, `otp_identity`, appended after CX in
`crates/qcore-frontend/src/premise_formula.rs`), checked by
`guarded_proposition` (`crates/qcore-frontend/tests/otel_proto_formulas.rs`),
modelled in Lean `QCore.OtelProtoObligations` with one theorem per formula
(`proof/AuditOtelProtoObligations.lean`; standard axioms only). The census JSON
cites them by id (`obligation_formulas`, `classification_formulas`,
`premise_formulas`, `identity_formulas`), and the report generator refuses a
census whose sites break a classification formula. The gloss after each formula
only reads it.

| id | formula |
|---|---|
| `otp_absent_message_is_null` | `forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_message_typed(#f) && !#otp_repeated(#f)) && !#otp_oneof(#f))) => ((#otp_present(#m,#f) => #otp_nonnull(#m,#f)) && (#otp_nonnull(#m,#f) => #otp_present(#m,#f)))) } }` |
| `otp_repeated_elements_nonnull` | `forall (#m: otp_msg) { forall (#f: otp_field) { forall (#p: otp_pos) { (((#otp_unpacked(#m) && (#otp_repeated(#f) && #otp_message_typed(#f))) && (0 <= #otp_at(#p) && #otp_at(#p) < #otp_count(#m,#f))) => #otp_elem_nonnull(#m,#f,#p)) } } }` |
| `otp_repeated_array_nonnull` | `forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && #otp_repeated(#f)) => (0 < #otp_count(#m,#f) => #otp_nonnull(#m,#f))) } }` |
| `otp_selected_member_nonnull` | `forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_unpacked(#m) && #otp_oneof(#f)) && #otp_message_typed(#f)) => (#otp_case(#m,#f) == #otp_number(#f) => #otp_nonnull(#m,#f))) } }` |
| `otp_string_nonnull` | `forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_string(#f) && !#otp_repeated(#f)) && (!#otp_oneof(#f) \|\| #otp_case(#m,#f) == #otp_number(#f)))) => #otp_nonnull(#m,#f)) } }` |
| `otp_bytes_data_iff_length` | `forall (#m: otp_msg) { forall (#f: otp_field) { ((#otp_unpacked(#m) && ((#otp_bytes(#f) && !#otp_repeated(#f)) && (!#otp_oneof(#f) \|\| #otp_case(#m,#f) == #otp_number(#f)))) => ((#otp_data_nonnull(#m,#f) => 0 < #otp_len(#m,#f)) && (0 < #otp_len(#m,#f) => #otp_data_nonnull(#m,#f)))) } }` |
| `otp_unpack_result_tested` | `forall (#s: otp_site) { ((#otp_unpack_site(#s) && #otp_guarded(#s)) => #otp_result_nonnull(#s)) }` |
| `otp_deref_needs_nonnull` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_deref_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => #otp_nonnull(#m,#f)) } } }` |
| `otp_subscript_needs_bound` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_subscript_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => (0 <= #otp_at(#otp_pos_of(#s)) && #otp_at(#otp_pos_of(#s)) < #otp_count(#m,#f))) } } }` |
| `otp_member_needs_case` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_member_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => #otp_case(#m,#f) == #otp_number(#f)) } } }` |
| `otp_data_needs_length` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((#otp_data_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) => (0 < #otp_len(#m,#f) \|\| #otp_data_nonnull(#m,#f))) } } }` |
| `otp_enum_index_needs_name` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { ((((#otp_enum_site(#s) && #otp_reads(#s,#m,#f)) && #otp_enum(#f)) && #otp_guarded(#s)) => #otp_value_named(#m,#f)) } } }` |
| `otp_bounded_element_nonnull` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((((#otp_subscript_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) && #otp_unpacked(#m)) && (#otp_repeated(#f) && #otp_message_typed(#f))) => #otp_elem_nonnull(#m,#f,#otp_pos_of(#s))) } } }` |
| `otp_selected_member_guarded_nonnull` | `forall (#s: otp_site) { forall (#m: otp_msg) { forall (#f: otp_field) { (((((#otp_member_site(#s) && #otp_reads(#s,#m,#f)) && #otp_guarded(#s)) && #otp_unpacked(#m)) && (#otp_oneof(#f) && #otp_message_typed(#f))) => #otp_nonnull(#m,#f)) } } }` |
| `otp_classes_partition` | `forall (#s: otp_site) { (((#otp_guarded(#s) \|\| #otp_candidate(#s)) \|\| #otp_undetermined(#s)) && ((!(#otp_guarded(#s) && #otp_candidate(#s)) && !(#otp_guarded(#s) && #otp_undetermined(#s))) && !(#otp_candidate(#s) && #otp_undetermined(#s)))) }` |
| `otp_candidate_is_wire_and_unguarded` | `forall (#s: otp_site) { (#otp_candidate(#s) => (#otp_wire(#s) && !#otp_guarded(#s))) }` |
| `otp_unguarded_off_wire_undetermined` | `forall (#s: otp_site) { ((!#otp_guarded(#s) && !#otp_wire(#s)) => #otp_undetermined(#s)) }` |
| `otp_guard_is_cited` | `forall (#s: otp_site) { (#otp_guarded(#s) => #otp_cited(#s)) }` |
| `otp_undetermined_names_reason` | `forall (#s: otp_site) { (#otp_undetermined(#s) => #otp_reason_named(#s)) }` |
| `otp_no_unsafe_claimed` | `forall (#s: otp_site) { !#otp_unsafe_claimed(#s) }` |
| `otp_identity_refused_unless_pinned` | `forall (#d: otp_identity) { (!#otp_pinned(#d) => #otp_refused(#d)) }` |
| `otp_pinned_identity_admitted` | `forall (#d: otp_identity) { (#otp_pinned(#d) => !#otp_refused(#d)) }` |

Glosses: the first six are protobuf-c's unpack facts (an absent sub-message is
NULL and a present one is not; elements below the count are non-NULL; the
array is non-NULL when the count is positive; the selected oneof message is
non-NULL; strings are non-NULL; `data` is non-NULL exactly when `len` is
positive). The next six are what a guarded O0-O5 site has. The two derived
formulas combine a guard with a fact (a bounded element, a case-selected
message, is non-NULL). The classes partition the sites; a candidate is on the
wire and unguarded; an unguarded site off the wire is undetermined; a guard is
cited and an undetermined site names its reason; no Unsafe is claimed. A use
of the protocol is refused exactly when its identity is not the pinned one.

In Lean, `otp_repeated_elements_nonnull`, `otp_repeated_array_nonnull` and
`otp_bytes_data_iff_length` hold of protobuf-c's storage representation
(`Slot`); `otp_absent_message_is_null`, `otp_selected_member_nonnull` and
`otp_string_nonnull` follow from the stated premise `WellFormed` (the unpack
postcondition, cited to the runtime lines above, not proved of the C); the site
obligations follow from a census being `Sound` (every cited guard holds at its
site); the classification and identity formulas hold of `classify` and of
`pinned`/`refused` by construction.

## 6. Phase B: how the protocol becomes checked contracts (proposal; not implemented)

What must exist for a verdict to rest on the protocol: (i) the protobuf-c
unpack postcondition as declared premises on the generated functions' results
(`*__unpack` returns NULL or a message satisfying `WellFormed`, keyed by the
vendored descriptors); (ii) a null-model reading of the conditional facts at
consumer sites (an element below a tested count, a sub-message under a tested
presence, a member under a tested case), which today's declared non-null members
cannot state (they are unconditional); (iii) the identity pin. (ii) is an
analyzer change with Lean parity, whichever crate owns (i).

**Options for (i) and (iii).**

1. **A new provider crate `quirin-provider-fluent-otel-proto`** on
   `providers/contract-load-path.json`, owning the generated functions
   (`opentelemetry__proto__*__unpack`, `__free_unpacked`, `__init`,
   `__pack`, `__get_packed_size`) and the protobuf-c runtime entry points they
   call, with the OTP premises as its declarations' result facts and, as in
   `quirin-provider-libcurl`, a plugin-SDK checker package bound to its modules
   that re-checks the generated descriptors against the identity. Its pin holds
   the protocol identity of §1 plus the sha256 of every compiled `.pb-c.c`/`.pb-c.h`
   and of `protobuf-c.c`. cmetrics, ctraces, fluent-bit and cprofiles
   declarations whose Safe or Unsafe rests on these facts state
   `linkage_contract_v1` links to it (`cx_reliance_is_linked`;
   `linkage-reliances.json`). A stale identity (§1 refusal, or a changed
   generated-file digest) makes the crate's module load refuse (Unknown on every
   dependent row) or hold it `Pending` under the named reason "protocol identity
   changed: re-run the conformance table" (human-resolvable).
2. **A plugin-SDK user crate on `providers/quirin-provider-plugin`**, outside the
   shipped load path (`user-crates-load-path.json`), with the same declarations
   and checker. Every use is an explicit user load path; shipped crates cannot
   link to it, so the shipped routes' rows cannot rest on it.
3. **The existing crates** (`qcore-provider-cmetrics`, `qcore-provider-ctraces`,
   `quirin-provider-fluent-bit`, later cprofiles): each restates the facts for
   the generated types its library uses. Four authorities for one fact, four
   pins to keep equal; the generated functions are no library of theirs.

**Recommendation: option 1**, with the vendored descriptors (v1.10.0, §3) as
the contract basis and the pinned protocol's D1/D2 recorded as declarations the
vendored code cannot decode. It keeps one authority for the protocol facts, is
reachable from every shipped route, follows the standing decision that an
external library's contracts live in their own crate with their own checker
(protobuf-c and its generated code are vendored, not Fluent Bit's), and makes
the identity a load-time gate.

**Expected verdict effect** (an upper bound, never a prediction of Safe):
system and slice, none (no OTel unit). corpus and twelve-home: at most 17
null-property rows Unknown -> Safe on each, and on the combined verdict at most
1 (`cmt_encode_opentelemetry_batch.c#request_data_point_count`, custody
already Safe); every other OTel row stays Unknown on custody (missing
contracts, unsupported constructs, unresolved aliases, untouched by protocol
facts). The 17: `cmt_decode_opentelemetry.c` `clone_array`,
`clone_exemplars_to_kvlist`, `clone_kvlist`, `clone_variant`,
`decode_exponential_histogram_data_point_list`, `decode_metrics_entry`,
`decode_numerical_data_point_list`, `decode_resource_metadata_and_attributes`,
`decode_scope_metadata_and_attributes`, `decode_scope_metrics_entry`,
`decode_service_request`; `cmt_encode_opentelemetry_batch.c`
`cmt_encode_opentelemetry_split_payload`, `metric_view_create`,
`metrics_batch_view_destroy`, `request_data_point_count`;
`ctr_decode_opentelemetry.c` `ctr_span_set_links`, `span_set_events`. No
Unsafe moves (the five provisional findings are allocation results). The
unmeasured 795 functions move nothing until a route measures them (the
user's `current` route covers the whole build; it is not a pinned route and
was not read here).

## 7. Follow-ups (registered in [integration-amendments.json](integration-amendments.json))

* `OTEL-PROTO-PHASE-B` — the §6 choice and its implementation.
* `OTEL-PROTO-VENDORED-BASIS` — the vendored code is v1.10.0, the pinned
  protocol is 19 commits past v1.11.1: which identity contracts rest on (D1, D2).
* `OTEL-PROTO-SPAN-TYPE-ABSENT` (D1), `OTEL-PROTO-PROCESSCONTEXT-ABSENT` (D2),
  `OTEL-PROTO-STALE-PPROFEXTENDED` (D3).
* `OTEL-SITES-ENCODER-CONSTRUCTION` (N1), `OTEL-SITES-CALLEE-CHAINS` (N2),
  `OTEL-SITES-ARRAY-WITHOUT-COUNT` (N3), `OTEL-SITES-BYTES-TO-CALLEE` (N4),
  `OTEL-SITES-CROSS-UNIT-CALLEE` (N5), `OTEL-SITES-ENUM-TO-CALLEE` (N6).
* `OTEL-ROUTE-COVERAGE` — the 795 unmeasured functions of the brief's modules
  and the adjacent cprofiles and content-modifier units.
* Amended: `CG13-UNION-PUNNING-FLOW-INSENSITIVE` (the oneof reads are
  case-tested in the decoders: the live-member reading it asks for has its
  sites here), `CPROFILES-STRAY-DUPLICATE-TREE` (which tree the database builds).

## 8. Commands and checks

```sh
P=/home/cosmo/Documents/GitHub/opentelemetry-proto
F=build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
python3 scripts/otel_proto_audit.py conformance $P $F/lib/fluent-otel-proto/proto_c \
  --exclude opentelemetry/proto/profiles/v1development/pprofextended.pb-c.c \
  --rev v1.3.2 … --rev v1.11.1 --output conformance.json
python3 scripts/otel_proto_sites.py --compile-db $F/build/compile_commands.json \
  --path-map /home/cosmo/Documents/GitHub/qcore-c/=/media/Data/GitHub/quirin-lang/ \
  --pbc-root $F/lib/fluent-otel-proto/proto_c \
  --exclude opentelemetry/proto/profiles/v1development/pprofextended.pb-c.c \
  --file <each of the 37 OTel units> --output sites.json
python3 scripts/otel_proto_audit.py report --measure-tip private/measure/r40-landing/sides/tip \
  --sites sites.json --conformance conformance.json --proto-root $P --tree-root $F/ \
  --compile-db $F/build/compile_commands.json --inputs-base <checkout>/ \
  --output docs/reports/otel-proto-audit/census.json
python3 scripts/otel_proto_audit.py identity $P --check docs/reports/otel-proto-audit/census.json
```

The compile database names the checkout through the `qcore-c` symlink; the
`--path-map` rewrites it so nothing reaches this repository through the
symlink. No card gate (brief); the targeted checks and their results are in
the hand-back and the increment paragraph.
