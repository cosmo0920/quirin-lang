/*
 * Fixture dynamic output plugin for W43-DESCRIPTOR-NATIVE-CHECK.
 *
 * Built as `flb-out_qdesc.so` with the pinned Fluent Bit include/define
 * profile and loaded by the actual `flb_plugin_load` (fluent-bit -e PATH).
 * The fixture only accepts and drops records; the code under observation is
 * the upstream loader's dlopen/dlsym/dlclose lifecycle, not this plugin.
 */
#include <fluent-bit/flb_output_plugin.h>

static int cb_qdesc_init(struct flb_output_instance *ins, struct flb_config *config, void *data)
{
    (void) ins;
    (void) config;
    (void) data;
    return 0;
}

static void cb_qdesc_flush(struct flb_event_chunk *event_chunk, struct flb_output_flush *out_flush,
                           struct flb_input_instance *i_ins, void *out_context, struct flb_config *config)
{
    (void) event_chunk;
    (void) out_flush;
    (void) i_ins;
    (void) out_context;
    (void) config;
    FLB_OUTPUT_RETURN(FLB_OK);
}

static int cb_qdesc_exit(void *data, struct flb_config *config)
{
    (void) data;
    (void) config;
    return 0;
}

struct flb_output_plugin out_qdesc_plugin = {
    .name = "qdesc",
    .description = "W43 descriptor observation fixture",
    .cb_init = cb_qdesc_init,
    .cb_flush = cb_qdesc_flush,
    .cb_exit = cb_qdesc_exit,
    .event_type = FLB_OUTPUT_LOGS,
    .flags = 0,
};
