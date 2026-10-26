# CMetrics opts postcondition decomposition — before implementation

Base e0278e186. Replace the remaining sentence-shaped normal clauses in the
model-only `cmetrics/opts_lifetime.qv` with typed mathematical conditions. Keep
actual null-name and success guards; failed initialization may retain committed
ownership, but does not always do so. Express release count/ownership/retirement
and byte preservation separately. Resource-index preservation concerns raw
representation bytes, since initialization never writes or initializes it.
Possibility/classification statements and separate map teardown eligibility are
explanatory comments, not unconditional call guarantees.

Add explicit prerequisite roots for call exclusion and physical allocator/SDS
separation needed by preservation claims. Identity inequality alone is not
physical separation. Exact five-field interpretation, complete call/event
trace, allocator behavior, field layout and Rust/Lean/source correspondence
remain open. No ordinary resource/bounds summary or C permission is installed.
Match the new model properties with audited Lean and inspect via the real typed
reader; refresh official provider inventories and the current manifest binding.
Predict zero live corpus/system/slice verdict movements; no safety promotion.
