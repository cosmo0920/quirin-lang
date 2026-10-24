# CMetrics map and opts API contracts

The user requested API contracts to support auditing Fluent Bit's core mechanism. Canonical `.qv` declared models now cover `cmt_map_create`, `cmt_map_destroy`, `cmt_map_metric_get`, `cmt_map_metric_get_val`, `cmt_map_metric_destroy`, `cmt_map_metrics_expire`, `cmt_opts_init` and `cmt_opts_exit`. Source inspection uses Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; the report's source-audit receipt records exact header/body hashes. Validation is pending until the frozen initializer measurement finishes and the API source gate runs.

| API | Ownership/effect boundary |
|---|---|
| `cmt_map_create` | Fresh nullable owned map; copies label keys; retains nonowning opts/parent record views. Failed construction cleans its own partial graph. |
| `cmt_map_metric_get` | Nullable borrow of an embedded record or map-owned metric. Creating a metric can precede storage-preparation failure, so NULL does not promise rollback. Read lookup can reindex. |
| `cmt_map_metric_get_val` | Disjoint writable scalar output on success, unchanged output on lookup failure; lookup unlocks before the value read, requiring external retirement coordination. |
| `cmt_map_metric_destroy` | Removes a valid dynamic primary child, its secondary link and cache reference, then releases it and nested storage; embedded static metrics are excluded. |
| `cmt_map_metrics_expire` | Retires selected dynamic metrics and can reset embedded static storage; invalidates affected borrows while preserving the enclosing map. |
| `cmt_map_destroy` | Releases the map's transitive owned graph and invalidates descendants; retained opts/parent records are not freed. |
| `cmt_opts_init` | Requires empty pointer fields and stable input views. Both success and failure retain every committed SDS obligation; failure is not transactional rollback. |
| `cmt_opts_exit` | Releases live owned SDS contents, preserves enclosing storage and pointer bytes, invalidates string aliases. Nonempty repeated cleanup needs reestablished live fields. |

The primary list owns dynamic metrics; bucket chains and `last_metric` are secondary references. Backpointers and cache/index membership must agree with primary ownership. Returned metrics remain usable only with external coordination against expiration/destruction; an internal lookup lock is not a loan extending beyond unlock. Opts record lifetime and its string-content lifetime are distinct: `cmt_counter_destroy` calls `cmt_opts_exit` before `cmt_map_destroy`, and map teardown need not read retired opts strings.

The zero-hash route requires separate accounting. `metric_hash_lookup` returns the embedded metric for hash zero, including the labeled path, while teardown releases embedded storage under `metric_static_set`. The model therefore requires labeled zero hashes to be excluded or that embedded storage be accounted independently. Reachability and lifecycle closure remain Open under `CMT-LABELED-HASH-ZERO-STATIC-STORAGE`; this is an unproved source obligation, not a validated upstream defect.

There is no file-format blocker to declaring these contracts. Trusted executable adoption is still blocked on checked consumers for nested ownership/retention, primary versus secondary/cache relations, partial failure, retirement and alias invalidation, source identity and synchronization obligations. `CMT-MAP-OPTS-CONTRACT-ADOPTION` tracks that work. These declared models remain outside resource/bounds/outer interpreted views, grant no ordinary neutral summary and promote no API caller to Safe. They are usable as explicit audit specifications; their C-source correspondence remains premise 28.

The companion `QCore.CmetricsApiContracts` models partial-init obligations, cleanup without clearing stored pointers, non-idempotent nonempty cleanup, map-owned attachment before a NULL getter result, embedded/dynamic borrows and retirement epochs. Rust controls inspect the actual canonical source, reject interpreted-loader adoption and bind shared finite lifecycle witnesses to the audited Lean model. These controls do not prove C extraction or full implementation refinement.

The running initializer native comparison remains frozen at `e92d8aef35a8bd14550e90219d19a56eb3b196e7` and its original 310-file input snapshot. Later API-model source/package validation is a separate checkpoint; the native run is not relabelled as trusted API adoption.
