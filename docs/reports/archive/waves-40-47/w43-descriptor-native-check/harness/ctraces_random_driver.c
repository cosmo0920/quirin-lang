/*
 * ctraces random driver for W43-DESCRIPTOR-NATIVE-CHECK (Phase B).
 *
 * W43-DESCRIPTOR-RESOURCE-TRACKING reports `ctr_random_get` provisional
 * Unsafe: its `if (fd > 0)` guard does not close a descriptor `open` returned
 * as 0. The pinned build compiles that path: its configure test leaves
 * CTR_HAVE_GETRANDOM empty, so `open("/dev/urandom")` is the live Linux route.
 * No Fluent Bit scenario of this card reaches it, so this driver calls the
 * actual lib/ctraces/src/ctr_random.c function, compiled into the pinned
 * static archive, directly.
 *
 * Usage: ctraces-random-driver SCENARIO
 *   stdin_open    descriptor 0 is inherited and live: open returns a higher number
 *   stdin_closed  the driver first closes its own stdin stream (`fclose(stdin)`,
 *                 which releases descriptor 0 with it), so the lowest free number
 *                 `open` returns is 0
 * Exit status is 0 when ctr_random_get returned a nonnegative count.
 */
#include <ctraces/ctr_random.h>

#include <stdio.h>
#include <string.h>

int main(int argc, char **argv)
{
    unsigned char buffer[16];
    if (argc != 2) {
        return 2;
    }
    if (strcmp(argv[1], "stdin_closed") == 0) {
        fclose(stdin);
    }
    else if (strcmp(argv[1], "stdin_open") != 0) {
        return 2;
    }
    return ctr_random_get(buffer, sizeof(buffer)) < 0 ? 1 : 0;
}
