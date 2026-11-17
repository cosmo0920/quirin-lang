/* Card OPENSSL-PROVIDER-PLUGIN test fixture: a caller of the fictional
 * `fakelib` library whose user crate states a library version range. */
void fakelib_reset(void);

void use_reset(void) {
    fakelib_reset();
}

int no_reset(int x) {
    return x + 1;
}
