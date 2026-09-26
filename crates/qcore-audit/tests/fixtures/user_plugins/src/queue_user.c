/* Card W45-USER-CONTRACT-PLUGINS test fixture: a caller that relies on the
 * pinned `acme_queue_take` contract. */
void *acme_pool_get(void);
void acme_queue_take(void *item);

void use_queue(void) {
    void *item;
    item = acme_pool_get();
    if (!item) {
        return;
    }
    acme_queue_take(item);
}
