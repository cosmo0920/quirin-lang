# RK-PLUGIN-INTERCEPTOR-CANDIDATES

Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`. [Prediction](PREDICTION.md)
was committed at `d623c3670`, with the runtime-only representation amendment at
`78c6af225`, before implementation. This report will record the completed
measurements and gates; the card is still in progress.

The candidate inventory distinguishes 14 `rd_kafka_conf_s.interceptors` lists,
their 14 `rd_kafka_s.rk_conf` aliases and their method union arms. It pins exact
source bytes at both audited Fluent Bit revisions and preserves the C signatures.
The candidate restriction is neutral custody for callbacks and their transitive
calls, with borrowed pointer arguments and scalar resource handles, balanced local
acquisitions, public registration lifecycle restrictions, and callback code/opaque
contexts valid through their last possible invocation. It is a human review
premise for deployed code; it does not declare the setters neutral, establish a
static list closure or decide an unsupported static invocation.

The generic runtime loader binds the inventory's exact digest into each defining
outer statement, refuses unsupported/missing/unbound inventories, and holds the
citing loader on source mismatch. Both outer envelopes add the 28 exact list paths
only under those review restrictions. Static rule 12n remains unchanged.

## Follow-ons

`RK-INTERCEPTOR-STATIC-LIST-CLOSURE`: static interceptor list invocation remains unsupported: a list/element/union-arm
closure and candidate composition need their own sound rule. Callbacks that free
or retain opaque state or transfer ownership do not meet this card's neutral
candidate and remain unsupported. No deployed callback has been certified.
