/* Card OPENSSL-PROVIDER-REMAINING-CALLEES: functions shaped like the census
 * rows the new declarations reach, audited through the real OpenSSL 3.5.5
 * headers, the generated user load path and the real checker. Read only,
 * never compiled or run. */
#include <openssl/err.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/pkcs12.h>
#include <openssl/ssl.h>
#include <openssl/x509.h>
#include <openssl/x509_vfy.h>
#include <openssl/engine.h>

/* Plain calls: the error queue (rd_kafka_transport_ssl_io_update). */
unsigned long peek(void) {
    return ERR_peek_error();
}

void error_string(char *buf, size_t size) {
    unsigned long code = ERR_peek_last_error();
    ERR_error_string_n(code, buf, size);
}

/* I/O under the registered-callbacks premise (rd_kafka_transport_ssl_send). */
int write_some(SSL *ssl, const void *data, int len) {
    int r = SSL_write(ssl, data, len);
    if (r <= 0)
        return SSL_get_error(ssl, r);
    return r;
}

/* A returned reference acquired, tested and dropped
 * (rd_kafka_transport_ssl_verify). */
int peer_check(SSL *ssl) {
    X509 *cert = SSL_get1_peer_certificate(ssl);
    if (!cert)
        return 0;
    X509_free(cert);
    return 1;
}

/* The PEM reader at a NULL `x` and NULL callback (Monkey
 * tls_load_builtin_credentials), dropped. */
int read_cert(BIO *bio) {
    X509 *cert = PEM_read_bio_X509(bio, NULL, NULL, NULL);
    if (!cert)
        return -1;
    X509_free(cert);
    return 0;
}

/* The reuse arm (`&x` at `x`): outside the declaration's premise. */
int read_cert_reuse(BIO *bio, X509 **slot) {
    return PEM_read_bio_X509(bio, slot, NULL, NULL) != NULL;
}

/* Fresh objects that are not reference counted. */
int digest_cycle(void) {
    EVP_MD_CTX *ctx = EVP_MD_CTX_new();
    if (!ctx)
        return -1;
    const EVP_MD *md = EVP_sha256();
    (void)md;
    EVP_MD_CTX_free(ctx);
    return 0;
}

int pkcs12_cycle(BIO *bio) {
    PKCS12 *p12 = d2i_PKCS12_bio(bio, NULL);
    if (!p12)
        return -1;
    PKCS12_free(p12);
    return 0;
}

/* A borrowed result used through its parent (rd_kafka_ssl_set_certs). */
int store_flags(SSL_CTX *ctx) {
    X509_STORE *store = SSL_CTX_get_cert_store(ctx);
    return X509_STORE_set_flags(store, 0);
}

/* The verification callback's getters (rd_kafka_transport_ssl_cert_verify_cb). */
int verify_depth(X509_STORE_CTX *ctx) {
    X509 *cert = X509_STORE_CTX_get_current_cert(ctx);
    int depth = X509_STORE_CTX_get_error_depth(ctx);
    if (cert && X509_STORE_CTX_get_error(ctx) != X509_V_OK)
        X509_STORE_CTX_set_error(ctx, X509_V_OK);
    return depth;
}

/* A retention into a holder the caller supplied: undecided. */
void keep_userdata(SSL_CTX *ctx, void *opaque) {
    SSL_CTX_set_default_passwd_cb_userdata(ctx, opaque);
}

/* A structural reference looked up and released (rd_kafka_ssl_ctx_init_engine). */
void engine_cycle(const char *id) {
    ENGINE *e = ENGINE_by_id(id);
    if (!e)
        return;
    ENGINE_free(e);
}
