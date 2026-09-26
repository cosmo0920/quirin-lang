/* Card W45-USER-CONTRACT-PLUGINS test fixture: the body a pinned user
 * contract was reviewed against. */
void acme_pool_put(void *item);
void acme_queue_take(void *item);

void acme_queue_take(void *item) {
    acme_pool_put(item);
}
