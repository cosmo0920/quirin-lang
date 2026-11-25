// SPDX-License-Identifier: Apache-2.0
#include <limits.h>
#include <stdlib.h>

extern int qv_entry_run(unsigned char *, unsigned char *);

static int outcome, calls, callee_drops, result_drops, other_drops;
static unsigned char *subject, *other_owner;

/* The generated translation unit redirects only its own calls to free
 * here. Clear live slots before the underlying free ends the lifetime. */
void observed_free(void *p) {
    if (p != NULL && p == other_owner) {
        other_owner = NULL;
        ++other_drops;
    } else if (p != NULL && p == subject) {
        subject = NULL;
        ++result_drops;
    } else {
        abort();
    }
    free(p);
}

int take(unsigned char *p) {
    if (p != subject || ++calls != 1) abort();
    if (outcome == 0) {
        subject = NULL;
        ++callee_drops;
        free(p);
    }
    return outcome;
}

int main(void) {
    const int outcomes[] = {0, INT_MIN, INT_MAX, -17, 1};
    for (size_t i = 0; i < sizeof outcomes / sizeof outcomes[0]; ++i) {
        outcome = outcomes[i];
        calls = callee_drops = result_drops = other_drops = 0;
        subject = malloc(1);
        other_owner = malloc(1);
        if (subject == NULL || other_owner == NULL) abort();
        const int actual = qv_entry_run(subject, other_owner);
        if (actual != outcome || calls != 1 || other_drops != 1 ||
            callee_drops != (outcome == 0) || result_drops != (outcome != 0) ||
            subject != NULL || other_owner != NULL) return 1;
    }
    return 0;
}
