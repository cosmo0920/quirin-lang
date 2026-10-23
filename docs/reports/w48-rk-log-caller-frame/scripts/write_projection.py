#!/usr/bin/env python3
"""Write caller-frame.json, the caller-frame projection (caller-frame-v1) of
LOGGER-EFFECT-CONTRACTS' reviewed logger-effects-v1 model (W48-RK-LOG-CALLER-FRAME,
rule 32). The document is reviewed data: qcore_contract::caller_frame derives
every path standing and refuses a stated one that differs.

Usage: write_projection.py   (from the repository root)
"""
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MODEL = ROOT / "docs/reports/logger-effect-contracts/model.json"
OUT = ROOT / "docs/reports/w48-rk-log-caller-frame/caller-frame.json"
NOEV = "rk_log_enqueue_no_event_callback"
FWD = "rk_log_enqueue_forward_not_last"
CB = "rk_plugin_callback_closure"


def internal(argument, reason):
    return {"internal": argument, "reason": reason}


def premise(name, reason):
    return {"premise": name, "reason": reason}


def main():
    model = MODEL.read_bytes()
    doc = {
        "version": "caller-frame-v1",
        "model_sha256": hashlib.sha256(model).hexdigest(),
        "root": "rd_kafka_log0",
        "declarations": ["rd_kafka_log0", "rd_kafka_log_buf"],
        "frame": {
            "formatting_extent": internal("borrowed-read", "rd_kafka_log0 reads extra, fmt and the trailing arguments through borrowed formals (vsnprintf reads; %n stores a count through a borrowed int *) and writes only its own 2,048-byte stack buffer; their liveness is the caller's borrowed-position obligation, checked at the call site."),
            "tls_thread_name": internal("library-state", "RD_TLS rd_kafka_thread_name is librdkafka's own thread-local array; no caller custody rests on it."),
            "configuration_lifetime": internal("borrowed-read", "conf and rk are borrowed formals the body only reads (log_level, log_thread_name, log_queue, log_cb, rk_logq); their liveness is the caller's borrowed-position obligation."),
            "source_model_correspondence": internal("trust-boundary", "Faithful extraction and source-to-model correspondence are trust boundary 28, beneath every verdict; the model's finite decomposition stays reviewed data."),
        },
        "events": {
            "create-op": {"frame": "in", "reason": "rd_kafka_op_new(RD_KAFKA_OP_LOG): rd_calloc, so every field but rko_type is zero (no reply queue, op callback, toppar or error).", "obligations": {
                "allocator_fail_stop": internal("fresh-object", "The allocation is the call's own; allocation failure is a fail-stop (non-returning) trace."),
                "generic_cleanup_effects": internal("fresh-object", "The zero-initialised LOG op holds no toppar, error, reply queue or op callback the caller could hold.")}},
            "create-text": {"frame": "in", "reason": "rd_strlcpy(fac) into the fresh op's array and rd_strdup(buf) of rd_kafka_log0's own stack buffer.", "obligations": {
                "allocator_fail_stop": internal("fresh-object", "The duplicated string is the call's own."),
                "copy_extent": internal("fresh-object", "Both copies write only the fresh op; the copied text is rd_kafka_log0's stack buffer and the borrowed fac.")}},
            "forward": {"frame": "in", "reason": "rd_kafka_q_enq1 on a forwarded queue: rd_kafka_q_fwd_get keeps the forward queue, the recursive enqueue runs, rd_kafka_q_destroy drops it.", "obligations": {
                "queue_synchronization": premise(FWD, "The keep and the drop are balanced, so in sequential execution the drop is never the last; only another thread unforwarding and releasing that queue's last other reference during the call makes it destroy a queue inside the caller's call."),
                "forwarding_termination": internal("normal-exit-only", "An unbounded forwarding chain is a non-returning trace; a normal exit carries no claim about it.")}},
            "transfer": {"frame": "in", "reason": "rd_kafka_q_enq0 links the fresh op into the final queue (counters, cnd_signal), copies the original queue's serve callback onto the op, and calls rd_kafka_q_io_event when the queue had been empty.", "obligations": {
                "queue_synchronization": internal("queue-internal", "The linked queue is rk->rk_logq or a queue it forwards to, reached only through the borrowed rk; linking a fresh op changes no caller custody."),
                "io_event_effects": premise(NOEV, "rd_kafka_q_io_event runs a registered queue event callback synchronously; with none registered it at most writes a wake-up to a descriptor the queue owns."),
                "serve_callback_effects": internal("fresh-object", "The serve callback and opaque are only stored on the fresh op; the enqueue never invokes them."),
                "eventual_consumer_cleanup": internal("after-frame", "The queued op and its string are released by a later consumer, purge or destroy, after the call returns.")}},
            "reply": {"frame": "infeasible", "reason": "A disabled queue replies to the op; rd_kafka_op_reply enqueues on a reply queue only when rko_replyq.q is set, which a fresh op's is not (it destroys the op instead: the disabled_no_reply path).", "obligations": {
                "reply_queue_effects": internal("fresh-object", "The fresh op's reply queue is empty, so no reply enqueue happens during the call.")}},
            "take": {"frame": "boundary", "reason": "A consumer takes the queued op, after the call has returned.", "obligations": {
                "eventual_consumer_cleanup": internal("after-frame", "Consumer cleanup happens after the call.")}},
            "callback": {"frame": "in", "reason": "conf->log_cb(rk, level, fac, buf) with rd_kafka_log0's stack text.", "obligations": {
                "log_callback_effects": premise(CB, "The log callback is arbitrary registered code; its candidates (cb_kafka_logger, rd_kafka_log_print) are declared borrow-only under the callback closure."),
                "configuration_lifetime": internal("borrowed-read", "The callback and rk are read through the borrowed conf and rk.")}},
            "destroy-text": {"frame": "in", "reason": "LOG destruction frees rko_u.log.str.", "obligations": {
                "generic_cleanup_effects": internal("fresh-object", "The freed string is the call's own rd_strdup.")}},
            "destroy-op": {"frame": "in", "reason": "rd_kafka_op_destroy of the fresh LOG op: no op callback (type LOG without the CB flag, rko_op_cb zero), no toppar, error or reply queue, then rd_free.", "obligations": {
                "destroy_callback_effects": internal("fresh-object", "A fresh LOG op has no rko_op_cb, so no destroy callback runs."),
                "generic_cleanup_effects": internal("fresh-object", "Every generic cleanup field of the fresh op is zero; only the op itself is freed.")}},
        },
        "paths": {
            "filtered": {"standing": "proved"},
            "queue_missing_terminating": {"standing": "proved"},
            "callback_absent": {"standing": "proved"},
            "immediate_callback": {"standing": "premised", "premises": [CB]},
            "local_queue": {"standing": "premised", "premises": [NOEV]},
            "forwarded_queue": {"standing": "premised", "premises": sorted([FWD, NOEV])},
            "disabled_no_reply": {"standing": "proved"},
            "disabled_reply_queue": {"standing": "infeasible"},
            "forwarded_disabled_no_reply": {"standing": "premised", "premises": [FWD]},
            "consumer_callback_cleanup": {"standing": "outside-frame", "prefix": "local_queue"},
        },
    }
    text = json.dumps(doc, indent=1, sort_keys=True) + "\n"
    OUT.write_text(text)
    print(hashlib.sha256(model).hexdigest(), hashlib.sha256(text.encode()).hexdigest())


if __name__ == "__main__":
    main()
