#!/usr/bin/env python3
"""sweep.py REPO REV OUT.json -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS:
the comment sweep of every tracked `.qv` under `providers/` and `contract/`
(the card brief's step 2).

Every comment line that says `premise`, `assum…` (assume, assumed,
assumption) is a hit; the supplementary phrasings (`rests on`, `relies on`,
`holds under`, `unchecked`, `not modelled`, `invariant`) were read by hand
and are listed in the report, not here. Each hit is classified by the first
rule below that matches it, in order: a manual entry for that exact
`(file, line)` (the hits read and decided one by one), then the patterns.
A hit no rule classifies fails the run (exit 1), so a new comment cannot
slip through unread; so does a manual entry no hit uses. The texts are read
at REV (`git show REV:PATH`): the line numbers are the base's, before this
card's edits (run at `9564eeb6d`).

Classes (the report's §2 says why none but A is a premise of a declaration):
  A  attached: a condition its declaration rests on that no clause states;
     now `requires resource_premise(NAME, "...")`.
  D  the declaration as a whole is a trusted assumption ("ONE TRUSTED
     ASSUMPTION", "a declaration is a premise"): reported per row as the
     declaration and its body standing (C06.5), not a separate premise.
  B  the declaration is unchecked because no analysed body checks it
     (cross-unit, header text, no normal exit): its body standing,
     `declared-assumption`, already says so.
  U  a note about a function deliberately NOT declared ("cost a premise and
     buy nothing", an unpaid premise): no declaration to attach to.
  M  methodology: a fact measured, compared or checked "rather than assumed",
     or what callers "may assume" of a declared defined function.
  C  already a machine-readable clause: a literal-argument premise (C04.5),
     a variadic data tail, a review-pending reason (C06.1).
  R  a run-level named assumption: the libc/C-standard ABI premise
     (`libc-c-standard-abi`) or the tip-tracked `mk_vhost_set`.
  S  a scope boundary: a property outside the custody clauses is not
     claimed (bounds, values, a model the vocabulary does not have).
  N  not a resource-v1 module (a lifecycle, policy or source-frame
     contract): C04.18's clause is not in its grammar; its caller premises
     are that contract family's own.
"""
import json
import pathlib
import re
import subprocess
import sys

HIT = re.compile(r"premise|assum", re.I)

# (file suffix, line) -> (class, reason). Read and decided one by one.
MANUAL = {
    ("fluent_bit/output_singleplex.qv", 28): ("A", "flb_output_task_singleplex_enqueue: synchronous_output_reachability"),
    ("fluent_bit/output_singleplex.qv", 31): ("A", "the same premise's last line"),
    ("fluent_bit/loop_owner.qv", 39): ("A", "flb_input_collector_resume: ingestion_active (C3)"),
    ("libc/errno.qv", 52): ("A", "__errno_location: nested_call_admission (N1-N10, N4-D)"),
    ("libc/errno.qv", 55): ("A", "the same premise (N10)"),
    ("libc/errno.qv", 104): ("A", "strerror: nested_call_admission (N3, N6, N7)"),
    ("librdkafka/cjson.qv", 237): ("A", "add_item_to_array: cjson_list_invariant"),
    ("librdkafka/cjson.qv", 185): ("A", "cJSON_New_Item: cjson_allocator_hooks"),
    ("librdkafka/client.qv", 582): ("A", "rd_kafka_toppar_new0, rd_kafka_toppar_desired_add, rd_kafka_q_new0, ut_create_mock_queue: asserted_allocation (the non-null results); the count arithmetic in the same sentence is the declarations' own reference clauses (class D)"),
    ("fluent_bit/input_chunk_name.qv", 51): ("A", "flb_input_chunk_get_name: untracked_handback (O04-TRACE N6/N8)"),
    # The ledger names AOM-DESTROY-CORRUPTED-UNPAID-PREMISE; it is a note
    # about a third function deliberately left undeclared.
    ("fluent_bit/input_chunk_destroy.qv", 120): ("U", "AOM-DESTROY-CORRUPTED-UNPAID-PREMISE names flb_input_chunk_destroy_corrupted, deliberately undeclared (payoff zero); neither declared function rests on it"),
    ("fluent_bit/input_chunk_destroy_direct_routes.qv", 114): ("U", "AOM-DESTROY-CORRUPTED-UNPAID-PREMISE, as in input_chunk_destroy.qv"),
    ("fluent_bit/input_chunk_destroy_direct_routes.qv", 116): ("U", "the same note's name"),
    ("librdkafka/client.qv", 516): ("C", "rd_kafka_topic_partition_destroy0's literal premise (C04.5), already a clause"),
    ("librdkafka/client.qv", 769): ("C", "the served-queue premise, already a clause (VERDICT-CONTRACT-DEPENDENCIES)"),
    ("librdkafka/client.qv", 773): ("C", "the served-queue premise, already a clause"),
    ("librdkafka/client.qv", 779): ("C", "the served-queue premise, already a clause"),
    ("librdkafka/client.qv", 780): ("C", "the served-queue premise, already a clause"),
    ("librdkafka/topic_partition.qv", 73): ("M", "the same rationale's last line"),
    ("fluent_bit/hash_table_publication.qv", 79): ("C", "the literal premise's loading rule"),
    ("fluent_bit/task_users.qv", 42): ("C", "the literal premise, already a clause"),
    ("libc/pthread_sync.qv", 7): ("R", "the same sentence's last line"),
    ("librdkafka/cjson.qv", 269): ("U", "add_item_to_object is not declared: no literal premise reads its cast arguments"),
    ("librdkafka/lz4_stream.qv", 182): ("S", "bounds of the unsafe decoder are not claimed (a bounds property, not custody)"),
    ("librdkafka/topic_partition.qv", 72): ("M", "design rationale: custody is tracked through the list pointer, which the projection's model fixes; not an assumption about the C function"),
    ("cmetrics/metric_values.qv", 118): ("U", "cmt_metric_set was that card's target, not a premise; still undeclared"),
    ("fluent_bit/task_users.qv", 40): ("C", "flb_task_users_release's literal premise (C04.5), already a clause"),
    ("fluent_bit/task_users.qv", 79): ("B", "a declaration is a cross-unit premise outside its defining unit: body standing"),
    ("fluent_bit/task_users.qv", 99): ("B", "flb_output_thread_pool_flush: unchecked cross-unit declaration (W45-FB-THREAD-POOL-FLUSH-UNCHECKED), body standing declared-assumption"),
    ("fluent_bit/hash_table_publication.qv", 72): ("C", "flb_hash_table_add's literal premise (C04.5), already a clause"),
    ("fluent_bit/hash_table_publication.qv", 77): ("C", "the literal premise, already a clause"),
    ("fluent_bit/lib_engine_thread.qv", 17): ("C", "review-pending reasons (C06.1): data for the reviewer, the declaration justifies neither Safe nor Unsafe"),
    ("fluent_bit/storage_backlog.qv", 12): ("M", "bodies are checked specifications, not premises (rule 11r and kin)"),
    ("fluent_bit/time.qv", 36): ("M", "a gateway fixture's declared type, not production text"),
    ("libc/process.qv", 18): ("C", "prctl's literal premise (C04.5), already a clause"),
    ("libc/stdio.qv", 275): ("C", "the variadic data tail, already a clause; the va_list members state it for the list"),
    ("libc/unistd.qv", 171): ("U", "fcntl's other commands are not declared: one literal premise per parameter"),
    ("libc/pthread_sync.qv", 6): ("R", "the libc/C-standard ABI premise"),
    ("libc/string.qv", 303): ("D", "strndup: a user-directed declared assumption"),
    ("librdkafka/memory.qv", 159): ("B", "unchecked premise pinned to the header text: body standing declared-assumption"),
    ("librdkafka/memory.qv", 254): ("B", "rd_kafka_crash: no normal exit, so no summary validates it: body standing declared-assumption"),
    ("monkey/logging.qv", 238): ("B", "mk_utils_libc_error: no analysed body checks it: body standing declared-assumption"),
    ("monkey/vhost_set.qv", 87): ("R", "the run's named assumption tip-tracked:mk_vhost_set"),
    ("monkey/composition.qv", 50): ("N", "not resource-v1"),
    ("cfl/sds_lifetime.qv", 447): ("B", "withdrawn where its body fails rule 13a, a declared assumption elsewhere: body standing"),
    ("cmetrics/mpack_utils.qv", 103): ("U", "the cmt_mpack_* family is not declared"),
}

PATTERNS = [
    ("R", re.compile(r"libc/C-standard ABI premise", re.I)),
    ("D", re.compile(r"^// ASSUMPTIONS?\b|custody assumptions for|delimit these assumptions|restricts that assumption|TRUSTED\s+ASSUMPTION|trusted assumption|a declaration is a premise|declared assumption|resource-projection assumptions|^// declaration is a premise", re.I)),
    ("M", re.compile(r"(measured|compared|checked|enforced),? (rather than|not) assumed|rather than assumed|not assumed|may assume|state, not assume|stated rather than|recorded rather than assumed", re.I)),
    ("U", re.compile(r"cost (a )?premises?|premise with no measured payoff|unpaid", re.I)),
]


def tracked(repo, rev):
    out = subprocess.run(["git", "-C", repo, "ls-tree", "-r", "--name-only", rev, "--", "providers", "contract"],
                         capture_output=True, text=True, check=True).stdout
    return [p for p in out.split() if p.endswith(".qv")]


def text_at(repo, rev, path):
    return subprocess.run(["git", "-C", repo, "show", f"{rev}:{path}"],
                          capture_output=True, text=True, check=True).stdout


def classify(path, number, text, v1):
    for (suffix, line), entry in MANUAL.items():
        if path.endswith(suffix) and line == number:
            return entry
    if not v1:
        return ("N", "not a resource-v1 module")
    for name, pattern in PATTERNS:
        if pattern.search(text):
            return (name, pattern.pattern)
    return None


def main():
    repo, rev, out = sys.argv[1], sys.argv[2], sys.argv[3]
    hits, unclassified = [], []
    for path in tracked(repo, rev):
        lines = text_at(repo, rev, path).split("\n")
        v1 = any("requires resource_contract_v1" in l for l in lines) or any(l.startswith("#include") for l in lines)
        for index, line in enumerate(lines):
            if not line.lstrip().startswith("//") or not HIT.search(line):
                continue
            entry = classify(path, index + 1, line, v1)
            record = {"file": path, "line": index + 1, "text": line.strip()}
            if entry is None:
                unclassified.append(record)
                continue
            record["class"], record["reason"] = entry
            hits.append(record)
    used = {(h["file"], h["line"]) for h in hits}
    stale = [key for key in MANUAL if not any(f.endswith(key[0]) and l == key[1] for f, l in used)]
    counts = {}
    for h in hits:
        counts[h["class"]] = counts.get(h["class"], 0) + 1
    result = {"hits": len(hits), "by_class": dict(sorted(counts.items())),
              "files": len({h["file"] for h in hits}), "unclassified": unclassified,
              "stale_manual_entries": [list(k) for k in stale], "rows": hits}
    pathlib.Path(out).write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({k: result[k] for k in ("hits", "by_class", "files")}), f"unclassified={len(unclassified)}", f"stale={len(stale)}")
    for record in unclassified:
        print("UNCLASSIFIED", record["file"], record["line"], record["text"])
    for key in stale:
        print("STALE", key)
    sys.exit(1 if unclassified or stale else 0)


main()
