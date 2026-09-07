#include <cmetrics/cmt_counter.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    for (int initial = 0; initial <= 1; ++initial) {
        struct cmt_counter counter;
        struct cmt_counter expected;
        memset(&counter, 0, sizeof counter);
        counter.allow_reset = initial;
        memcpy(&expected, &counter, sizeof counter);
        expected.allow_reset = 1;
        cmt_counter_allow_reset(&counter);
        if (memcmp(&counter, &expected, sizeof counter) != 0) return 2;
        printf("CMT_RESET initial=%d final=%d remaining_bytes_unchanged=1\n",
               initial, counter.allow_reset);
    }
    puts("CMT_RESET_RUN_CLEAN calls=2");
    return 0;
}
