# RK outer loader scope correction

The user directs that `rd_kafka_plugin_new` need not promise preservation of Fluent Bit's input, output, filter and processor instance `p` members. This correction removes those four `#kept` envelope clauses and the equivalent English promise from `rk_plugin_library`. The Fluent Bit envelope, all librdkafka member/premise clauses and the interceptor inventory remain unchanged. The intentional contract-corpus clause pin changes from 85 to 81.

[PREDICTION.md](PREDICTION.md) was committed at `4f3b46f0b` before implementation. Historical lexical and W48/RK measurements keep their original outer contract identities. The subsequent S3/integration measurement must explicitly include this correction.

The projection retains its conservative loader reachability. Missing coverage is Unknown under an applicable RK scenario; the Fluent Bit envelope does not supply a promise on behalf of a different loader. The new `rk_loader_does_not_vouch_for_fluent_bit_instance_members` regression loads both actual provider modules and checks each of the four member facts, both loaders together, and an unchanged librdkafka member. The existing executable projection and audited `QCore.OuterContract.an_uncovered_fact_is_never_safe_or_pending` theorem apply without a semantic-model change.

Validation is pending: reviewed outer pin refresh, provider inventory sync/check, the full standing/provider gates and final runtime row comparison will run on the composed lexical/S3 integration. This report does not claim those checks have passed or publish new coverage counts yet. No reachability exemption, cross-loader coverage fallback or outer-envelope expansion is added.
