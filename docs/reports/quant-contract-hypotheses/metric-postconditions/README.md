# Low-level CMetrics setter postconditions

PREDICTION.md precedes implementation at c8a3b774a. Nine model-only functions
replace thirty sentence-shaped normal clauses with twenty-two typed normal
conditions. Twenty-two entry roots remain unchanged. migration.json records
each declaration's old clauses, new IDs and exact source identities.

The selected call and its nested helper events have explicit store/CAS target,
field, array index, integer value and order projections. Scalar setters retain
five ordered stores, including double bits, signed/unsigned mirrors, timestamp
and type tag. Count and bit-encoded sum setters remain distinct; histogram
bucket values use defined C numeric conversion. Exponential-histogram sum flags
normalize every nonzero integer to one and disabled sums write zero bits.
The quantile guard keeps null metric, null array, negative and out-of-range
cases as no-store paths; valid slots have successful CAS before timestamp.

No assertion creates a joint atomic record snapshot or establishes CAS-loop
termination/wait-free progress. Those limitations remain comments. Custody
conditions prohibit the selected call's release, ownership transfer or escape
of the metric and its nested footprint; they do not invent global guarantees
about other actors. Complete-call exclusion remains an explicit prerequisite.

Actual event completeness/order, selected backend, byte/array field layout,
scalar-slot index interpretation, numeric conversions/bit encoding and
Rust/Lean/source correspondence remain open. Bit encoding must preserve the
selected floating representation, including applicable NaN payloads; it cannot
silently use real-number equality or a canonicalized NaN. Numeric conversion
requires the existing finite/truncation/range premises. A parsed declaration
installs no interpreted resource/bounds summary or ordinary C permission.

`QCore.CmetricsMetricConditions` models ordered store projections, normalized
sum flags, all guarded no-store cases, admitted quantile pairs and conditional
ownership preservation. Its conversion/field/event inputs do not certify C.

The real typed inventory exited 0 in
`private/quant-contract-hypotheses/metric-post-models-1/`: nine declarations,
forty-four conditions, twenty-two entry roots. Initial Lean build/audit exited 0
with eight checks in `metric-post-lean-1/`. Focused all-target contract compilation
exited 0 in `metric-post-compile-1/`. Rust execution, the full provider/card gate
and native comparison remain pending; this is not card closure.

The final formula increment replaces the scalar signed/unsigned mirrors and
bucket numeric conversion with mathematical `trunc(val)` under the existing
finite/range prerequisites. For the scalar setter, both conversions share the
nonnegative signed-64 domain, so no hidden signed-bit conversion is needed.
Double bit encoding remains a distinct explicitly bound function. Inventory 2
exited 0 with unchanged counts. The complete Lean model, including five scalar
stores and their shared integer mirrors, passed nine checks in
`metric-post-lean-2/` (exit 0). Both official inventory syncs exited 0 in
`metric-post-records-{1,2}/`; the second owns the final source bytes and reviewed
clause-count changes. The current CMetrics manifest binding follows those bytes.
All-target Clippy exited 0 in `metric-post-clippy-1/`:
`cargo clippy --all-targets --offline --locked -- -D warnings`.
