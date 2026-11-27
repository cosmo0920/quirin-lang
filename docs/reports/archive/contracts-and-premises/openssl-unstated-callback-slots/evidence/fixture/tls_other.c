/* Card OPENSSL-UNSTATED-CALLBACK-SLOTS: a second unit of the run. The run's
 * info and security slots hold tls_slots.c's internal app_info and
 * app_security, which have no contract in force here, so this unit's calls
 * refuse at the invocations: a callback is never read benign. */
#include <openssl/ssl.h>

int read_some(SSL *ssl, void *data, int len) {
    return SSL_read(ssl, data, len);
}

int load_other(SSL_CTX *ctx, X509 *x) {
    return SSL_CTX_use_certificate(ctx, x);
}
