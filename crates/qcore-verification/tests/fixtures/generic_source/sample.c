struct sample_state { unsigned long remaining; unsigned long unrelated; };
unsigned long sample_decrement(struct sample_state *state) {
    state->remaining--;
    return state->remaining;
}
struct sample_payload { char *data; };
struct sample_owner { struct sample_payload *payload; };
void free(void *);
void sample_dispose(struct sample_owner *root) {
    struct sample_payload *payload = root->payload;
    if (!payload) return;
    free(payload->data);
    free(payload);
}
struct sample_link { struct sample_link *prev, *next; };
void sample_link(struct sample_link *prev, struct sample_link *next) {
    prev->next = next;
    next->prev = prev;
}
void sample_unlink(struct sample_link *entry) {
    sample_link(entry->prev, entry->next);
    entry->prev = (void *)0;
    entry->next = (void *)0;
}
struct sample_handle { int handle; int unrelated; };
int sample_release(int);
int *sample_errno(void);
int sample_report(int, const char *, int);
int sample_cleanup(struct sample_handle *state) {
    if (!state) return -1;
    if (state->handle != -1) {
        int result = sample_release(state->handle);
        if (result == -1) {
            sample_report(*sample_errno(), "sample.c", 1);
            return -1;
        }
        state->handle = -1;
    }
    return 0;
}
