/* Card OPENSSL-PROVIDER-PLUGIN evidence fixture: callers of the contracted
 * OpenSSL functions through the real (pinned) headers, shaped like the six
 * rows OpenSSL alone blocks on the user's current run. */
#include <openssl/engine.h>
#include <openssl/err.h>
#include <openssl/ssl.h>

/* rd_kafka_transport_ssl_clear_error */
void clear_error(void) {
    ERR_clear_error();
}

/* Monkey tls_init_library (OpenSSL >= 1.1.0 branch) */
void init_library(void) {
    OPENSSL_init_ssl(0, NULL);
    SSL_load_error_strings();
}

/* Monkey tls_create_server_context (OpenSSL >= 1.1.0 branch) */
SSL_CTX *create_server_context(void) {
    return SSL_CTX_new(TLS_server_method());
}

struct holder {
    SSL_CTX *ctx;
    ENGINE *engine;
};

/* rd_kafka_ssl_ctx_term: drops through member reads, refused by rule 13b */
void ctx_term(struct holder *h) {
    if (h->engine) {
        ENGINE_free(h->engine);
    }
    if (h->ctx) {
        SSL_CTX_free(h->ctx);
    }
}

/* An acquired reference, dropped once on the non-NULL path */
void ctx_cycle(void) {
    SSL_CTX *ctx = SSL_CTX_new(TLS_server_method());
    if (!ctx) {
        return;
    }
    SSL_CTX_free(ctx);
}
