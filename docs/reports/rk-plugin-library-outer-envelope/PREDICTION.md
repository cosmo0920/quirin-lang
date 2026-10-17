# RK-PLUGIN-LIBRARY-OUTER-ENVELOPE prediction

Frozen before the implementation and measurements. The baseline is OUTER-CONTRACT stage 1 on `fa04aab0f`. This card adds an outer loader at `rd_kafka_plugin_new`, with a pointer to the configuration and a success guard, and retires the premise that no plugin library is configured. Static verdicts should stay fixed. The replacement premise will state the callback registration boundary rather than assert that no library is loaded.

The `rk_plugin_library` scenario should turn a subset of corpus custody and null rows from Safe to Pending. A row must be covered on *every* closed-world dependency, including stated/run-set members and premises. Some rows may stay Unknown where the library envelope cannot vouch for cJSON global allocator hooks or other facts.

In the `all` scenario, Fluent Bit's `flb_external_plugin` reaches every record through a `void *` in `struct flb_config`. Its existing envelope covers only the four `flb_*_instance.p` members. Thus librdkafka rows remain Unknown unless the Fluent Bit envelope can faithfully state the needed librdkafka facts. This card will not assert that an arbitrary Fluent Bit plugin preserves those internals merely to reach the 32-row target. System and slice have no librdkafka units; their runtime statuses should be unchanged. Every moved row will be read from the measured diff.

The predicted counts are directional, not a claim that the 32 custody rows all move. The arbiter will compare the actual static, `rk_plugin_library`, `flb_external_plugin` and `all` scenarios for the corpus, system and slice.
