#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""FB-LEDGER-MEM-BUF-LIMIT-BOUND: the checked census of mem_buf_limit.

    python3 census.py SOURCE_ROOT OUT.json

SOURCE_ROOT is the prepared Fluent Bit 046accf6 checkout
(private/fb-046accf6/fluent-bit-clang-source, read-only). The script

* records the SHA-256 of every file it reads;
* checks that each census row's pinned line still holds its needle;
* checks completeness: every occurrence of the three fields, every
  content-writing call, every chunkio `buf_len` assignment and every append
  entry point in src/, plugins/, include/ and lib/chunkio/ is a census row
  (an unclassified hit fails);
* checks the order of the sites inside `input_chunk_append_raw` and
  `flb_input_chunk_protect` (every write after the paused gate; every
  `return 0` after the write is one of the three exits the Lean model names;
  `protect` once, after the last `set_limits`; the rate-gate return before
  the memory test).

Exit 0 when every check holds; 1 otherwise, naming each failure.
"""

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1])
OUT = Path(sys.argv[2])
IC = "src/flb_input_chunk.c"

# (file, line, needle, class, note). Classes:
#   counter-write  : assigns mem_chunks_size
#   decision       : a test the bound depends on
#   status-write   : assigns mem_buf_status
#   limit-write    : assigns mem_buf_limit (configuration time only)
#   content-write  : changes a chunk's content length
#   entry          : an append entry point
#   neighbour      : reads the fields or limits outside the bound's transitions
#   declaration    : a declaration or comment
ROWS = [
    # mem_chunks_size: assigned only from the stream total (or 0 at creation).
    ("src/flb_input.c", 663, "instance->mem_chunks_size = 0;", "counter-write", "instance creation"),
    (IC, 3017, "in->mem_chunks_size = total;", "counter-write", "set_limits: total = cio_stream_size_chunks_up (:2993, :3014)"),
    (IC, 3151, "in->mem_chunks_size = total;", "counter-write", "set_up_down snapshot (:3148)"),
    (IC, 3159, "in->mem_chunks_size = total;", "counter-write", "set_up_down after cio_chunk_down (filesystem only; a no-op for memory)"),
    # The overlimit test and the pause/resume transitions.
    (IC, 2957, "if (i->mem_buf_limit <= 0) {", "decision", "limit 0 disables the test (P-MBL-LIMIT-SET)"),
    (IC, 2961, "if (i->mem_chunks_size >= i->mem_buf_limit) {", "decision", "overlimit: snapshot >= limit"),
    (IC, 3032, "in->mem_buf_status == FLB_INPUT_PAUSED) {", "decision", "resume test: not overlimit, running, ingestion active, paused (:3029-3032)"),
    (IC, 3033, "in->mem_buf_status = FLB_INPUT_RUNNING;", "status-write", "resume, only below the limit"),
    (IC, 3054, "in->mem_buf_status == FLB_INPUT_RUNNING &&", "decision", "collector resume (flb_input_resume) reads the status; writes none"),
    (IC, 3126, "i->mem_buf_status = FLB_INPUT_PAUSED;", "status-write", "pause after the write (protect)"),
    ("src/flb_input.c", 661, "instance->mem_buf_status = FLB_INPUT_RUNNING;", "status-write", "instance creation"),
    ("src/flb_input.c", 2704, "if (ins->mem_buf_status == FLB_INPUT_RUNNING &&", "decision", "rate gate resume reads the status; writes none"),
    ("include/fluent-bit/flb_input.h", 785, "if (i->mem_buf_status == FLB_INPUT_PAUSED) {", "decision", "flb_input_buf_paused, read by the append gate through flb_input_paused"),
    # The limit: written at configuration time only.
    ("src/flb_input.c", 662, "instance->mem_buf_limit = 0;", "limit-write", "instance creation"),
    ("src/flb_input.c", 859, "ins->mem_buf_limit = (size_t) limit;", "limit-write", "property mem_buf_limit"),
    ("plugins/filter_rewrite_tag/rewrite_tag.c", 66, "ins->mem_buf_limit = ctx->emitter_mem_buf_limit;", "limit-write", "emitter creation"),
    ("plugins/filter_log_to_metrics/log_to_metrics.c", 914, "input_ins->mem_buf_limit = ctx->emitter_mem_buf_limit;", "limit-write", "emitter creation"),
    ("plugins/filter_multiline/ml.c", 138, "ins->mem_buf_limit = ctx->emitter_mem_buf_limit;", "limit-write", "emitter creation"),
    # Neighbours: read the fields or derive other limits, change neither.
    ("src/flb_input_ingest.c", 468, "ins->ingress_queue_byte_limit = ins->mem_buf_limit;", "neighbour", "ingress queue limit (bytes before the append, not in mem_chunks_size)"),
    ("src/flb_input_ingest.c", 472, "if (ins->mem_buf_limit > 0 &&", "neighbour", "ingress queue limit"),
    ("src/flb_input_ingest.c", 474, "ins->mem_buf_limit < ins->ingress_queue_byte_limit)) {", "neighbour", "ingress queue limit"),
    ("src/flb_input_ingest.c", 475, "ins->ingress_queue_byte_limit = ins->mem_buf_limit;", "neighbour", "ingress queue limit"),
    ("plugins/in_emitter/emitter.c", 120, "if (ctx->ins->mem_buf_limit == 0) {", "neighbour", "emitter's own pre-append queue test"),
    ("plugins/in_emitter/emitter.c", 124, "if (ctx->ins->mem_chunks_size + ctx->pending_bytes + append_size >", "neighbour", "emitter's own pre-append queue test (stricter, emitter only)"),
    ("plugins/in_emitter/emitter.c", 125, "ctx->ins->mem_buf_limit) {", "neighbour", "emitter's own pre-append queue test"),
    ("plugins/in_emitter/emitter.c", 136, "if (ctx->ins->mem_buf_limit == 0) {", "neighbour", "emitter owner queue test"),
    ("plugins/in_emitter/emitter.c", 140, "limit = ctx->ins->mem_buf_limit;", "neighbour", "emitter owner queue test"),
    ("src/flb_storage.c", 202, "if (i->mem_buf_limit > 0) {", "neighbour", "metrics"),
    ("src/flb_storage.c", 203, "if (i->mem_chunks_size >= i->mem_buf_limit) {", "neighbour", "metrics"),
    ("src/flb_storage.c", 225, "cmt_gauge_set(i->cmt_storage_memory_bytes, ts, i->mem_chunks_size,", "neighbour", "metrics"),
    ("src/flb_storage.c", 233, "flb_utils_bytes_to_human_readable_size(i->mem_chunks_size,", "neighbour", "metrics"),
    ("src/flb_storage.c", 243, "flb_utils_bytes_to_human_readable_size(i->mem_buf_limit,", "neighbour", "metrics"),
    ("src/flb_dump.c", 81, "if (i->mem_buf_limit > 0) {", "neighbour", "dump"),
    ("src/flb_dump.c", 82, "if (i->mem_chunks_size >= i->mem_buf_limit) {", "neighbour", "dump"),
    ("src/flb_dump.c", 90, "flb_utils_bytes_to_human_readable_size(i->mem_chunks_size,", "neighbour", "dump"),
    ("src/flb_dump.c", 93, "tmp, i->mem_chunks_size);", "neighbour", "dump"),
    ("src/flb_dump.c", 96, "flb_utils_bytes_to_human_readable_size(i->mem_buf_limit,", "neighbour", "dump"),
    ("src/flb_dump.c", 99, "tmp, i->mem_buf_limit);", "neighbour", "dump"),
    ("src/flb_sosreport.c", 260, "if (ins_in->mem_buf_limit > 0) {", "neighbour", "sosreport"),
    ("src/flb_sosreport.c", 261, "flb_utils_bytes_to_human_readable_size(ins_in->mem_buf_limit,", "neighbour", "sosreport"),
    ("plugins/custom_calyptia/calyptia.c", 132, "if (i_ins->mem_buf_limit > 0) {", "neighbour", "configuration dump"),
    ("plugins/custom_calyptia/calyptia.c", 133, "flb_utils_bytes_to_human_readable_size(i_ins->mem_buf_limit,", "neighbour", "configuration dump"),
    ("plugins/custom_calyptia/calyptia.c", 135, "mem_buf_limit %s", "neighbour", "configuration dump"),
    (IC, 3063, "in->mem_chunks_size,", "neighbour", "log argument"),
    (IC, 3064, "in->mem_buf_limit);", "neighbour", "log argument"),
    (IC, 3122, "i->mem_buf_limit,", "neighbour", "log argument"),
    (IC, 3123, "i->mem_chunks_size", "neighbour", "log argument"),
    ("src/flb_input.c", 171, "\"mem_buf_limit\", \"0\"", "declaration", "config map"),
    ("src/flb_input.c", 853, "prop_key_check(\"mem_buf_limit\", k, len)", "declaration", "property key"),
    ("include/fluent-bit/flb_input.h", 316, "size_t mem_chunks_size;", "declaration", "field"),
    ("include/fluent-bit/flb_input.h", 331, "size_t mem_buf_limit;", "declaration", "field"),
    ("include/fluent-bit/flb_input.h", 339, "int mem_buf_status;", "declaration", "field"),
    ("plugins/in_emitter/emitter.c", 250, "Restricted by mem_buf_limit", "declaration", "comment"),
    ("plugins/in_systemd/systemd.c", 278, "Restricted by mem_buf_limit", "declaration", "comment"),
    # Every call that changes a chunk's content length.
    (IC, 799, "ret = cio_chunk_write(ic->chunk, buf, len);", "content-write", "flb_input_chunk_write's body"),
    (IC, 811, "ret = cio_chunk_write_at(ic->chunk, offset, buf, len);", "content-write", "flb_input_chunk_write_at's body; no caller"),
    (IC, 1540, "cio_chunk_write_at(chunk, offset, NULL, 0);", "content-write", "flb_input_chunk_map truncation (filesystem backlog; shrinks)"),
    (IC, 1573, "cio_chunk_write_at(chunk, offset, NULL, 0);", "content-write", "flb_input_chunk_map truncation (filesystem backlog; shrinks)"),
    (IC, 1613, "cio_chunk_write_at(chunk, offset, NULL, 0);", "content-write", "flb_input_chunk_map truncation, offset < bytes (:1612; shrinks)"),
    (IC, 3455, "ret = flb_input_chunk_write(ic,", "content-write", "THE append write: final_data_size bytes, after the gate"),
    ("src/flb_storage.c", 956, "if (cio_chunk_write(dst, buf, size) != CIO_OK ||", "content-write", "DLQ copy into the 'rejected' filesystem stream, not an input stream"),
    ("src/flb_fstore.c", 313, "ret = cio_chunk_write(fsf->chunk, data, size);", "content-write", "flb_fstore's own chunkio context (output plugins), not an input stream"),
    ("lib/chunkio/src/cio_chunk.c", 208, "return cio_chunk_write(ch, buf, count);", "content-write", "cio_chunk_write_at's tail"),
    ("lib/chunkio/src/cio_chunk.c", 220, "ret = cio_memfs_write(ch, buf, count);", "content-write", "cio_chunk_write, memory backend"),
    ("lib/chunkio/src/cio_chunk.c", 196, "mf->buf_len = offset;", "content-write", "write_at: content length set to offset"),
    ("lib/chunkio/src/cio_chunk.c", 492, "mf->buf_len = ch->tx_content_length;", "content-write", "rollback restores the length at tx_begin"),
    ("lib/chunkio/src/cio_memfs.c", 56, "mf->buf_len = 0;", "content-write", "memfs open: an empty chunk"),
    ("lib/chunkio/src/cio_memfs.c", 112, "mf->buf_len += count;", "content-write", "memfs write: +count after a successful realloc; unchanged on failure (:101-105)"),
    # Append entry points: all reach input_chunk_append_raw on the engine thread.
    (IC, 3251, "static int input_chunk_append_raw(struct flb_input_instance *in,", "entry", "the append (definition)"),
    (IC, 3616, "static int append_to_ring_buffer(struct flb_input_instance *ins,", "entry", "threaded inputs: enqueue only"),
    (IC, 3773, "input_chunk_append_raw(cr->ins, cr->event_type, cr->records,", "entry", "engine-thread ring-buffer collector, after its own paused test (:3747)"),
    (IC, 3786, "int flb_input_chunk_append_raw(struct flb_input_instance *in,", "entry", "public API (definition)"),
    (IC, 3799, "ret = append_to_ring_buffer(in, event_type, 0, records,", "entry", "threaded instance: ring buffer"),
    (IC, 3804, "ret = input_chunk_append_raw(in, event_type, records,", "entry", "non-threaded instance: direct, engine thread"),
    (IC, 3811, "int flb_input_chunk_append_raw_local(struct flb_input_instance *in,", "entry", "local append (definition)"),
    (IC, 3817, "return input_chunk_append_raw(in, event_type, records,", "entry", "local append: only from flb_input_log_append_processed (collector, :3768)"),
    (IC, 3821, "int flb_input_chunk_ring_buffer_enqueue(struct flb_input_instance *in,", "entry", "enqueue (definition)"),
    (IC, 3827, "return append_to_ring_buffer(in, event_type, 0, records,", "entry", "enqueue"),
    (IC, 3832, "int flb_input_chunk_ring_buffer_enqueue_log_routing(struct flb_input_instance *in,", "entry", "enqueue (definition)"),
    (IC, 3838, "return append_to_ring_buffer(in, event_type, FLB_INPUT_CHUNK_RAW_LOG_ROUTING,", "entry", "enqueue"),
    ("src/flb_input_blob.c", 208, "ret = flb_input_chunk_append_raw(ins, FLB_INPUT_BLOBS, 0,", "entry", "public API caller"),
    ("src/flb_input_profiles.c", 71, "ret = flb_input_chunk_append_raw(ins, FLB_INPUT_PROFILES, 0,", "entry", "public API caller"),
    ("src/flb_input_trace.c", 101, "ret = flb_input_chunk_append_raw(ins, FLB_INPUT_TRACES, 0,", "entry", "public API caller"),
    ("src/flb_input_metric.c", 122, "ret = flb_input_chunk_append_raw(ins, FLB_INPUT_METRICS, records,", "entry", "public API caller"),
    ("src/flb_input_log.c", 1392, "ret = flb_input_chunk_append_raw_local(ins,", "entry", "local_append (collector path)"),
    ("src/flb_input_log.c", 1401, "ret = flb_input_chunk_append_raw(ins,", "entry", "public API caller"),
    ("src/flb_input_log.c", 1503, "ret = flb_input_chunk_append_raw_local(ins, FLB_INPUT_LOGS, records,", "entry", "local_append (collector path)"),
    ("src/flb_input_log.c", 1507, "ret = flb_input_chunk_append_raw(ins, FLB_INPUT_LOGS, records,", "entry", "public API caller"),
    ("src/flb_input_log.c", 1578, "ret = flb_input_chunk_ring_buffer_enqueue_log_routing(ins,", "entry", "threaded conditional routing: enqueue"),
    ("include/fluent-bit/flb_input_chunk.h", 122, "int flb_input_chunk_append_raw(struct flb_input_instance *in,", "declaration", "prototype"),
    ("include/fluent-bit/flb_input_chunk.h", 127, "int flb_input_chunk_append_raw_local(struct flb_input_instance *in,", "declaration", "prototype"),
    ("include/fluent-bit/flb_input_chunk.h", 132, "int flb_input_chunk_ring_buffer_enqueue(struct flb_input_instance *in,", "declaration", "prototype"),
    ("include/fluent-bit/flb_input_chunk.h", 137, "int flb_input_chunk_ring_buffer_enqueue_log_routing(struct flb_input_instance *in,", "declaration", "prototype"),
]

# Completeness families: every hit of each pattern must be a row.
FAMILIES = [
    ("fields", re.compile(r"\b(mem_chunks_size|mem_buf_limit|mem_buf_status)\b"),
     ["src", "plugins", "include"]),
    ("content writers", re.compile(
        r"\b(cio_chunk_write|cio_chunk_write_at|flb_input_chunk_write|flb_input_chunk_write_at|cio_memfs_write)\s*\("),
     ["src", "plugins", "lib/chunkio/src"]),
    ("memfs length", re.compile(r"\bbuf_len\s*(\+|-)?=(?!=)"), ["lib/chunkio/src"]),
    ("append entry points", re.compile(
        r"\b(input_chunk_append_raw|flb_input_chunk_append_raw|flb_input_chunk_append_raw_local|append_to_ring_buffer|"
        r"flb_input_chunk_ring_buffer_enqueue|flb_input_chunk_ring_buffer_enqueue_log_routing)\s*\("),
     ["src", "plugins", "include"]),
]
# Definitions and prototypes of the writers themselves, and comments.
IGNORED = {
    ("src/flb_input_chunk.c", 792), ("src/flb_input_chunk.c", 803), ("src/flb_input_chunk.c", 1924),
    ("lib/chunkio/src/cio_chunk.c", 184), ("lib/chunkio/src/cio_chunk.c", 211),
    ("lib/chunkio/src/cio_memfs.c", 80),
}

failures = []
digests = {}
lines = {}


def read(rel):
    if rel not in lines:
        data = (ROOT / rel).read_bytes()
        digests[rel] = hashlib.sha256(data).hexdigest()
        lines[rel] = data.decode("utf-8", errors="replace").split("\n")
    return lines[rel]


def is_comment(text):
    stripped = text.strip()
    return stripped.startswith(("*", "/*", "//"))


rows_by_site = {}
for file, line, needle, klass, note in ROWS:
    text = read(file)[line - 1]
    if needle not in text:
        failures.append(f"{file}:{line}: expected {needle!r}, found {text.strip()!r}")
    rows_by_site[(file, line)] = klass

family_counts = {}
for name, pattern, roots in FAMILIES:
    hits = 0
    for root in roots:
        for path in sorted((ROOT / root).rglob("*")):
            if path.suffix not in (".c", ".h") or "tests" in path.parts:
                continue
            rel = path.relative_to(ROOT).as_posix()
            text = path.read_text(errors="replace")
            if not pattern.search(text):
                continue
            for number, line in enumerate(read(rel), 1):
                if not pattern.search(line):
                    continue
                hits += 1
                if (rel, number) in rows_by_site or (rel, number) in IGNORED:
                    continue
                failures.append(f"{rel}:{number}: unclassified {name} hit: {line.strip()}")
    family_counts[name] = hits


def find(rel, needle, start, end):
    return [n for n in range(start, end + 1) if needle in read(rel)[n - 1]]


# Order inside input_chunk_append_raw (:3251-3600).
A0, A1 = 3251, 3600
gate = find(IC, "if (flb_input_paused(in) == FLB_TRUE &&", A0, A1)
write = find(IC, "flb_input_chunk_write(ic,", A0, A1)
set_limits = find(IC, "flb_input_chunk_set_limits(in);", A0, A1)
protect = find(IC, "flb_input_chunk_protect(in, final_data_size);", A0, A1)
release = find(IC, "memrb_input_chunk_release_space(in, buf_size,", A0, A1)
returns0 = [n for n in range(A0, A1 + 1) if read(IC)[n - 1].strip() == "return 0;"]
order = {
    "memrb_release": release, "gate": gate, "write": write,
    "set_limits": set_limits, "protect": protect, "return_0": returns0,
}
if not (len(release) == 1 and len(gate) == 1 and len(write) == 1 and len(protect) == 1):
    failures.append(f"append sites not unique: {order}")
else:
    if not release[0] < gate[0] < write[0]:
        failures.append("the memrb release or the gate does not precede the write")
    if not (set_limits and max(set_limits) < protect[0]):
        failures.append("protect does not follow the last set_limits")
    # Every `return 0` lies after the write and is one of the three exits.
    exits = {3513: "emptied", 3543: "unroutable", 3600: "stored"}
    if returns0 != sorted(exits):
        failures.append(f"append returns 0 at {returns0}, expected {sorted(exits)}")
    if any(n <= write[0] for n in returns0):
        failures.append("a return 0 precedes the write")

# Order inside flb_input_chunk_protect (:3081-3132).
P0, P1 = 3081, 3132
protect_order = [
    find(IC, "flb_input_chunk_is_storage_overlimit(i) == FLB_TRUE", P0, P1),
    find(IC, "flb_input_rate_gate_protect(i) == FLB_TRUE", P0, P1),
    find(IC, "storage->type == FLB_STORAGE_FS", P0, P1),
    find(IC, "flb_input_chunk_is_mem_overlimit(i) == FLB_TRUE", P0, P1),
    find(IC, "i->storage_type == FLB_STORAGE_MEMRB", P0, P1),
    find(IC, "i->mem_buf_status = FLB_INPUT_PAUSED;", P0, P1),
]
if any(len(s) != 1 for s in protect_order) or [s[0] for s in protect_order] != sorted(s[0] for s in protect_order):
    failures.append(f"protect order: {protect_order}")

# The snapshot is the stream total; set_limits' resume is under the test.
for rel, needle, line in [
    (IC, "total = cio_stream_size_chunks_up(storage->stream);", 2993),
    (IC, "total = flb_input_chunk_total_size(in);", 3014),
    (IC, "if (flb_input_chunk_is_mem_overlimit(in) == FLB_FALSE &&", 3029),
    ("lib/chunkio/src/cio_stream.c", "bytes = cio_chunk_get_content_size(ch);", 268),
    ("lib/chunkio/src/cio_chunk.c", "return mf->buf_len;", 326),
    ("include/fluent-bit/flb_input.h", "static inline int flb_input_paused(struct flb_input_instance *i)", 795),
    (IC, "if (flb_input_is_threaded(in)) {", 3798),
    (IC, "if (flb_input_paused(ins) == FLB_TRUE) {", 3747),
    ("src/flb_task.c", "flb_input_chunk_set_limits(task->i_ins);", 940),
]:
    if needle not in read(rel)[line - 1]:
        failures.append(f"{rel}:{line}: expected {needle!r}")

classes = {}
for _, _, _, klass, _ in ROWS:
    classes[klass] = classes.get(klass, 0) + 1
report = {
    "schema": "fb-ledger-mem-buf-limit-bound-census/1",
    "source": "fluent-bit 046accf6",
    "inputs": dict(sorted(digests.items())),
    "rows": [
        {"site": f"{f}:{n}", "class": k, "note": note} for f, n, _, k, note in ROWS
    ],
    "classes": dict(sorted(classes.items())),
    "family_hits": family_counts,
    "append_order": order,
    "failures": failures,
}
OUT.write_text(json.dumps(report, indent=1) + "\n")
for failure in failures:
    print(failure)
print(f"{len(ROWS)} rows, {len(digests)} inputs, families {family_counts}, failures {len(failures)}")
sys.exit(1 if failures else 0)
