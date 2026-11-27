import pathlib
p = pathlib.Path(__file__).resolve().parents[4] / "docs/development/increments/wave-48-part8.md"
t = p.read_text()
para = (
    "**`MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS` (2026-10-16, base `d708641bc`; measured on `5b1ceecd5`, rebased onto "
    "`711287771`; prediction frozen before implementation).** Rule 30's body check "
    "now reads a declaration's body as a bounded path graph ([report](../../reports/member-path-unit-helpers/README.md); "
    "gateway rule [30](../../agents/gateway.md#constraint-30) amended in place, contract "
    "specification C04.19.6, Lean `QCore.MemberPathUnitHelpers`): `goto`, labels, `switch` and "
    "loops are read to a fixpoint over each stated path's value set, a null test refines each "
    "branch, a `static` helper of the unit (a header `static inline` included) that receives the "
    "formal or a member path's address is followed with its parameters bound (a wrapper keeps its "
    "worker's outcome per returned value), any other call that receives such a view leaves what it "
    "reaches unknown, and an exit returning a `!=` guard's own enumerator is exempt from that "
    "guard. `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` and "
    "`rd_kafka_DeleteRecords_leaders_queried_cb` become partly validated; "
    "`rd_kafka_admin_fanout_worker` is corrected to a declared assumption (its formal reaches the "
    "indirect `partial_response` after the store, a call the base check never read); "
    "`rd_kafka_admin_worker` stays one (`rd_list_move`, indirect `parse` and `request`). Measured: "
    "corpus 936 / 9 / 3,889 (`71387bd3…` → `a2e95319…`, only two units' body evidence differs, "
    "no row moves), declarations partly validated 45 → 46; system and slice byte-identical; null "
    "rows identical. Lean audits 324 → 325; ledger 784 → 784 (the entry retired, "
    "`MEMBER-PATH-STATES-THROUGH-INDIRECT-AND-EXTERNAL-CALLS` registered).\n\n"
)
anchor = "**`LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`"
if "**`MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS`" not in t:
    i = t.index(anchor)
    t = t[:i] + para + t[i:]
    p.write_text(t)
print(len(t.encode()))
