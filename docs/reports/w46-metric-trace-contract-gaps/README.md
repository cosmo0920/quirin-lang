# W46-METRIC-TRACE-CONTRACT-GAPS

## Prediction, recorded before implementation

Base `9bce3986dd5703faa6d1fc2f485dc88a20902fdd`. The pinned Cycle 19
source is Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; the
system slice is `046accf6aded7906f66a1baf0202da349e521736`.

Adding custody-neutral scalar metric helpers and trace ID queries / scalar
span setters should reduce `uncontracted-callee` leaves in the relevant library
units. I predict no new validated Unsafe result and no increase in Safe due to
the many other blockers in encoder and decoder paths. Exact function/unit
movement is deliberately left open for the pinned corpus arbiter; this is a
prediction, not an after-the-fact claim. These contracts do not discharge a
caller-owned metric, ID, span, or nested resource. A returned trace ID buffer
is a member alias, and mutation that replaces reachable ownership is outside
the predicted neutral set.

The two CMetrics source/header files in scope have identical SHA-256 bytes at
both revisions. CTraces files differ by revision, so the selected functions
must be checked in both before declarations are added. ABI and custody
decisions, Rust refusal controls, Lean parity, inventories, measurements, and
integrator notes will follow in this report.
