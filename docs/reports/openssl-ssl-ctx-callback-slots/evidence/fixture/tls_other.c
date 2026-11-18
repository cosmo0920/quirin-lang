/* Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: a second unit of the run. The run's
 * certificate slot holds tls_slots.c's internal app_certificate, which has
 * no contract in force here, so this unit's I/O refuses at the invocation:
 * a callback is never read benign. */
#include <openssl/ssl.h>

int read_some(SSL *ssl, void *data, int len) {
    return SSL_read(ssl, data, len);
}
