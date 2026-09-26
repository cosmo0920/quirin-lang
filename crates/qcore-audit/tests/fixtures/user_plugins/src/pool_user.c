/* Card W45-USER-CONTRACT-PLUGINS test fixture: a caller of the fictional
 * `acme` pool the SDK's example user crate declares contracts for. */
void *acme_pool_get(void);
void acme_pool_put(void *item);

void use_pool(void) {
    void *item;
    item = acme_pool_get();
    if (!item) {
        return;
    }
    acme_pool_put(item);
}

int no_pool(int x) {
    return x + 1;
}
