# RK outer loader scope: prediction

Before-change source: `f96132e236813a3767839824e33d25e0f4295db0`.

The user directs removal of the four `flb_{input,output,filter,processor}_instance.p |-> #kept` envelope clauses from `rd_kafka_plugin_new`. Remove their equivalent English promise too. Keep the Fluent Bit loader's own envelope and librdkafka callback/interceptor candidates unchanged. Refresh the reviewed outer pins and the intentional clause count, 85 to 81.

Expected static custody, null and combined results: unchanged. This edits only an outer declaration. Runtime facts depending on one of those four members can become uncovered under `rk_plugin_library`; the conservative opaque-loader reachability rule remains in force. In an all-loaded scenario those facts must still be covered by each applicable envelope, so the separate Fluent Bit envelope does not automatically repair the missing RK promise. Such rows must become Unknown rather than keep an unsupported Pending/Safe result. No new runtime coverage or deployment claim is expected.

The frozen lexical-card measurements retain the previous envelope. Final S3/integration measurements must disclose this subsequent scope correction and compare complete runtime rows separately from the lifetime changes. Full standing/provider gates and final source/pin checks are required before completion; this prediction is not a gate receipt.
