/* Card OPENSSL-UNSTATED-CALLBACK-SLOTS: the context's callback slots the
 * narrowed premise covered, registered and invoked, audited through the real
 * OpenSSL 3.5.5 headers, the generated user load path and the real checker.
 * Read only, never compiled or run. With tls_other.c it is one run: each
 * slot's candidates are what the run's declared setters are handed. */
#include <openssl/ssl.h>

/* The application's callbacks: each takes nothing it receives. */
static int app_certificate(SSL *ssl, void *arg) {
    (void)ssl;
    (void)arg;
    return 1;
}

static void app_info(const SSL *ssl, int where, int ret) {
    (void)ssl;
    (void)where;
    (void)ret;
}

static int app_security(const SSL *s, const SSL_CTX *ctx, int op, int bits, int nid,
                        void *other, void *ex) {
    (void)s;
    (void)ctx;
    (void)op;
    (void)nid;
    (void)other;
    (void)ex;
    return bits >= 0;
}

/* The registrations: a null verify callback (the library's default), the
 * certificate callback with a null argument (RETENTION-NULL-CONSTANT-ACTUAL:
 * nothing is retained), the info and security callbacks. */
void configure(SSL_CTX *ctx) {
    SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
    SSL_CTX_set_cert_cb(ctx, app_certificate, NULL);
    SSL_CTX_set_info_callback(ctx, app_info);
    SSL_CTX_set_security_callback(ctx, app_security);
}

/* The security slot, consulted when a certificate is set. */
int load_certificate(SSL_CTX *ctx, X509 *x) {
    return SSL_CTX_use_certificate(ctx, x);
}

int load_chain(SSL_CTX *ctx, const char *file) {
    return SSL_CTX_use_certificate_chain_file(ctx, file);
}

/* Every slot, invoked by the handshake and I/O calls. */
int write_some(SSL *ssl, const void *data, int len) {
    return SSL_write(ssl, data, len);
}

/* The info and message slots, beside the narrowed premise. */
int shut(SSL *ssl) {
    return SSL_shutdown(ssl);
}
