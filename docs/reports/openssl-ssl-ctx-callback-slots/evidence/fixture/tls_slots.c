/* Card OPENSSL-SSL-CTX-CALLBACK-SLOTS: the context's callback slots,
 * registered and invoked, audited through the real OpenSSL 3.5.5 headers,
 * the generated user load path and the real checker. Read only, never
 * compiled or run. With tls_other.c it is one run: each slot's candidates
 * are what the run's declared setters are handed. */
#include <openssl/ssl.h>

/* The application's callbacks: each takes nothing it receives. */
static int app_password(char *buf, int size, int rwflag, void *u) {
    (void)rwflag;
    (void)u;
    if (size < 1)
        return 0;
    buf[0] = '\0';
    return 0;
}

static int app_certificate(SSL *ssl, void *arg) {
    (void)ssl;
    (void)arg;
    return 1;
}

/* The registrations (rd_kafka_ssl_ctx_init shaped). */
void configure(SSL_CTX *ctx) {
    SSL_CTX_set_default_passwd_cb(ctx, app_password);
    SSL_CTX_set_cert_cb(ctx, app_certificate, NULL);
}

/* The password slot, invoked by the loaders (rd_kafka_ssl_set_certs). */
int load_key(SSL_CTX *ctx, const char *file) {
    return SSL_CTX_use_PrivateKey_file(ctx, file, SSL_FILETYPE_PEM);
}

int load_chain(SSL_CTX *ctx, const char *file) {
    return SSL_CTX_use_certificate_chain_file(ctx, file);
}

/* The certificate slot, invoked by the handshake and I/O calls. */
int write_some(SSL *ssl, const void *data, int len) {
    return SSL_write(ssl, data, len);
}

int handshake(SSL *ssl) {
    return SSL_do_handshake(ssl);
}

/* The premise made explicit: the alert's info callback. */
int shut(SSL *ssl) {
    return SSL_shutdown(ssl);
}
