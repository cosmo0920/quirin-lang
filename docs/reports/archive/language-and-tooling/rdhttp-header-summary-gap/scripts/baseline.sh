#!/usr/bin/env bash
# baseline.sh RUN -- RDHTTP-HEADER-SUMMARY-GAP: one run's own figures (its
# verdict-row digest by comparing it with itself, its section counts and Safe
# roll-up, the ranking, and the per-unit outcomes of the residual row's
# declarations) into private/rhsg/{projection,clauses,ranking,units}-RUN-self.*.
# Each step's exit is printed; nothing is piped into a step.
set -u
run="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rhsg"
cd "$repo" || exit 2
names=rd_atomic32_get,rd_atomic64_get,mk_utils_libc_error,rd_atomic64_add,rd_malloc,rd_calloc,rd_file_read,cb_queue_chunks,flb_log_check,mk_list_add,mk_list_del,rd_realloc,rd_strdup,sb_allocate_backlogs,sb_allocate_chunk,sb_append_chunk_to_segregated_backlog,sb_append_chunk_to_segregated_backlogs,sb_destroy_backlog,sb_destroy_backlogs,sb_destroy_chunk,sb_release_output_queue_space,sb_remove_chunk_from_segregated_backlog,sb_remove_chunk_from_segregated_backlogs,sb_segregate_chunks,rd_free,rd_strndup
python3 "$here/verdict_projection.py" "$out/$run" "$out/$run" "$out/projection-$run-self.json" > "$out/projection-$run-self.log" 2>&1
echo "projection exit $?"
python3 "$here/clause_rollup.py" "$out/$run" "$out/$run" "$out/clauses-$run-self.json" > "$out/clauses-$run-self.log" 2>&1
echo "clauses exit $?"
python3 "$here/ranking.py" "$out/$run" > "$out/ranking-$run.txt" 2>&1
echo "ranking exit $?"
python3 "$here/unit_evidence.py" "$out/$run" "$names" > "$out/units-$run.txt" 2>&1
echo "units exit $?"
