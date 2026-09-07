/* P06 fixture: reduced excerpt of CTraces at fluent-bit
 * 9f0fa5730fba1761ebaf14bcd31220a3a9be9d6e.
 *
 * The record members and the six function bodies below are copied from
 * lib/ctraces/include/ctraces/ctr_span.h, lib/ctraces/include/ctraces/ctraces.h
 * and lib/ctraces/src/ctr_span.c / lib/ctraces/src/ctraces.c. Members that are
 * irrelevant to the designed capability and that would drag in the CFL and
 * protobuf include trees are omitted, so this excerpt is NOT layout-compatible
 * with the real struct and must never be used for an ABI claim. It exists only
 * to drive the real Boolean-field importer over the real upstream statement
 * shapes and record the exact rejection reason.
 */

typedef unsigned int uint32_t;
typedef int int32_t;

struct ctrace_span {
    int32_t flags;
    int kind;
    uint32_t dropped_attr_count;
    uint32_t dropped_events_count;
    uint32_t dropped_links_count;
};

/* ctraces.h: "options is unused for now" */
struct ctrace_opts {
    int _make_windows_happy;
};

int ctr_span_set_flags(struct ctrace_span *span, uint32_t flags)
{
    span->flags = flags;
    return 0;
}

void ctr_span_set_dropped_link_count(struct ctrace_span *span, uint32_t count)
{
    span->dropped_links_count = count;
}

void ctr_span_set_dropped_events_count(struct ctrace_span *span, uint32_t count)
{
    span->dropped_events_count = count;
}

void ctr_span_set_dropped_links_count(struct ctrace_span *span, uint32_t count)
{
    span->dropped_links_count = count;
}

void ctr_span_set_dropped_attributes_count(struct ctrace_span *span, uint32_t count)
{
    span->dropped_attr_count = count;
}

void ctr_opts_exit(struct ctrace_opts *opts)
{
    if (!opts) {
        return;
    }
}
