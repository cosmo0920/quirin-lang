/*
 * harness.c -- W47-FB-DOUBLE-STOP-OBSERVATION native harness.
 *
 * Drives the Fluent Bit library through its public API only; Fluent Bit's
 * sources and tests are not modified. One scenario per process:
 *
 *   control  create, dummy -> null, start (expected 0), stop once, destroy
 *   v        create, dummy -> null, start (expected 0), stop, stop, destroy
 *   i        create, dummy with a property its config map does not declare
 *            -> null, start (expected -1: the engine thread's
 *            flb_input_init_all fails, it writes FLB_ENGINE_FAILED and
 *            do_start joins it), stop, destroy
 *   iii      create, service security.fips_mode=on (this host has no OpenSSL
 *            FIPS provider), dummy -> null, start (expected -1 from
 *            flb_fips_init before the spawn), stop, destroy
 *
 * Before each call it prints ctx->status and ctx->config->worker (the
 * identifier flb_stop reads at src/flb_lib.c:1047). Lines are prefixed
 * "HARNESS" and written unbuffered to stderr, interleaved with the
 * interposer's "JOINLOG" lines.
 */
#include <fluent-bit.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static void state(const char *when, flb_ctx_t *ctx)
{
    fprintf(stderr, "HARNESS %s status=%d worker=%#lx\n", when, ctx->status,
            (unsigned long) ctx->config->worker);
}

int main(int argc, char **argv)
{
    const char *scenario;
    flb_ctx_t *ctx;
    int in_ffd;
    int out_ffd;
    int ret;
    int stops;

    setvbuf(stderr, NULL, _IONBF, 0);
    if (argc != 2) {
        fprintf(stderr, "usage: harness control|v|i|iii\n");
        return 2;
    }
    scenario = argv[1];
    if (strcmp(scenario, "control") && strcmp(scenario, "v") &&
        strcmp(scenario, "i") && strcmp(scenario, "iii")) {
        fprintf(stderr, "unknown scenario %s\n", scenario);
        return 2;
    }
    stops = strcmp(scenario, "v") == 0 ? 2 : 1;

    ctx = flb_create();
    if (ctx == NULL) {
        fprintf(stderr, "HARNESS flb_create failed\n");
        return 3;
    }
    flb_service_set(ctx, "Flush", "1", "Grace", "1", "Log_Level", "info", NULL);
    if (strcmp(scenario, "iii") == 0) {
        ret = flb_service_set(ctx, "security.fips_mode", "on", NULL);
        fprintf(stderr, "HARNESS service_set fips_mode ret=%d\n", ret);
    }

    in_ffd = flb_input(ctx, "dummy", NULL);
    if (in_ffd < 0) {
        fprintf(stderr, "HARNESS flb_input failed\n");
        return 3;
    }
    flb_input_set(ctx, in_ffd, "tag", "test", "rate", "10", NULL);
    if (strcmp(scenario, "i") == 0) {
        ret = flb_input_set(ctx, in_ffd, "qcore_undeclared_property", "1", NULL);
        fprintf(stderr, "HARNESS input_set undeclared ret=%d\n", ret);
    }
    out_ffd = flb_output(ctx, "null", NULL);
    if (out_ffd < 0) {
        fprintf(stderr, "HARNESS flb_output failed\n");
        return 3;
    }
    flb_output_set(ctx, out_ffd, "match", "*", NULL);

    state("before-start", ctx);
    ret = flb_start(ctx);
    fprintf(stderr, "HARNESS flb_start ret=%d\n", ret);
    if (ret == 0) {
        usleep(300000);
    }

    for (int n = 1; n <= stops; n++) {
        char when[32];
        snprintf(when, sizeof(when), "before-stop%d", n);
        state(when, ctx);
        ret = flb_stop(ctx);
        fprintf(stderr, "HARNESS flb_stop%d ret=%d\n", n, ret);
    }

    state("before-destroy", ctx);
    flb_destroy(ctx);
    fprintf(stderr, "HARNESS flb_destroy done\n");
    return 0;
}
