Contracted by `quirin-provider-openssl`: 57 functions.

### corpus

Rows with an OpenSSL callee: 29; every OpenSSL callee contracted: 18 (3 with no other diagnostic); still blocked by an uncontracted OpenSSL callee: 11 (1 sole). Uncontracted callees: 15 of 68, 20 of 99 pairs.

| Callee | Rows it blocks with no other diagnostic | Pairs | Sites |
|---|---:|---:|---:|
| `BIO_new_mem_buf` | 1 | 4 | 4 |
| `OPENSSL_sk_pop_free` | 0 | 2 | 10 |
| `SSL_CTX_ctrl` | 0 | 2 | 2 |
| `CRYPTO_free` | 0 | 1 | 1 |
| `ENGINE_load_ssl_client_cert` | 0 | 1 | 1 |
| `ERR_peek_last_error_all` | 0 | 1 | 1 |
| `OSSL_PROVIDER_load` | 0 | 1 | 1 |
| `OSSL_PROVIDER_unload` | 0 | 1 | 1 |
| `SSL_CTX_set_cert_cb` | 0 | 1 | 1 |
| `SSL_CTX_set_default_passwd_cb` | 0 | 1 | 1 |
| `SSL_CTX_use_cert_and_key` | 0 | 1 | 1 |
| `SSL_certs_clear` | 0 | 1 | 1 |
| `SSL_ctrl` | 0 | 1 | 1 |
| `X509_NAME_oneline` | 0 | 1 | 2 |
| `X509_OBJECT_get0_X509` | 0 | 1 | 1 |

| Row still blocked | Other diagnostics | Uncontracted OpenSSL callees |
|---|---:|---|
| `rdkafka_ssl.c::rd_kafka_ssl_PKEY_from_string` | 0 | `BIO_new_mem_buf` |
| `rdkafka_ssl.c::rd_kafka_ssl_OSSL_PROVIDER_free` | 1 | `OSSL_PROVIDER_unload` |
| `rdkafka_ssl.c::rd_kafka_ssl_last_error_str` | 1 | `ERR_peek_last_error_all` |
| `rdkafka_ssl.c::rd_kafka_ssl_X509_from_string` | 2 | `BIO_new_mem_buf` |
| `rdkafka_ssl.c::rd_kafka_ssl_cert_callback` | 6 | `SSL_certs_clear`, `SSL_ctrl` |
| `rdkafka_ssl.c::rd_kafka_ssl_ctx_load_providers` | 11 | `OSSL_PROVIDER_load` |
| `rdkafka_ssl.c::rd_kafka_transport_ssl_cert_verify_cb` | 13 | `CRYPTO_free`, `X509_NAME_oneline` |
| `rdkafka_ssl.c::rd_kafka_ssl_ctx_init` | 28 | `SSL_CTX_ctrl`, `SSL_CTX_set_default_passwd_cb` |
| `rdkafka_cert.c::rd_kafka_cert_new` | 63 | `BIO_new_mem_buf`, `OPENSSL_sk_pop_free` |
| `rdkafka_ssl.c::rd_kafka_ssl_set_certs` | 71 | `ENGINE_load_ssl_client_cert`, `OPENSSL_sk_pop_free`, `SSL_CTX_ctrl`, `SSL_CTX_set_cert_cb`, `SSL_CTX_use_cert_and_key`, `X509_OBJECT_get0_X509` |
| `rdkafka_sasl_oauthbearer_oidc.c::rd_kafka_oidc_assertion_create` | 72 | `BIO_new_mem_buf` |

### current

Rows with an OpenSSL callee: 38; every OpenSSL callee contracted: 25 (5 with no other diagnostic); still blocked by an uncontracted OpenSSL callee: 13 (1 sole). Uncontracted callees: 16 of 72, 22 of 116 pairs.

| Callee | Rows it blocks with no other diagnostic | Pairs | Sites |
|---|---:|---:|---:|
| `BIO_new_mem_buf` | 1 | 5 | 6 |
| `OPENSSL_sk_pop_free` | 0 | 2 | 10 |
| `SSL_CTX_ctrl` | 0 | 2 | 2 |
| `CRYPTO_free` | 0 | 1 | 1 |
| `ENGINE_load_ssl_client_cert` | 0 | 1 | 1 |
| `ERR_peek_last_error_all` | 0 | 1 | 1 |
| `OSSL_PROVIDER_load` | 0 | 1 | 1 |
| `OSSL_PROVIDER_unload` | 0 | 1 | 1 |
| `SSL_CTX_set_cert_cb` | 0 | 1 | 1 |
| `SSL_CTX_set_default_passwd_cb` | 0 | 1 | 1 |
| `SSL_CTX_use_cert_and_key` | 0 | 1 | 1 |
| `SSL_certs_clear` | 0 | 1 | 1 |
| `SSL_ctrl` | 0 | 1 | 1 |
| `SSL_set_fd` | 0 | 1 | 1 |
| `X509_NAME_oneline` | 0 | 1 | 2 |
| `X509_OBJECT_get0_X509` | 0 | 1 | 1 |

| Row still blocked | Other diagnostics | Uncontracted OpenSSL callees |
|---|---:|---|
| `rdkafka_ssl.c::rd_kafka_ssl_PKEY_from_string` | 0 | `BIO_new_mem_buf` |
| `rdkafka_ssl.c::rd_kafka_ssl_OSSL_PROVIDER_free` | 1 | `OSSL_PROVIDER_unload` |
| `rdkafka_ssl.c::rd_kafka_ssl_last_error_str` | 1 | `ERR_peek_last_error_all` |
| `rdkafka_ssl.c::rd_kafka_ssl_X509_from_string` | 2 | `BIO_new_mem_buf` |
| `tls_openssl.c::tls_load_builtin_credentials` | 2 | `BIO_new_mem_buf` |
| `rdkafka_ssl.c::rd_kafka_ssl_cert_callback` | 6 | `SSL_certs_clear`, `SSL_ctrl` |
| `tls_openssl.c::context_new` | 7 | `SSL_set_fd` |
| `rdkafka_ssl.c::rd_kafka_ssl_ctx_load_providers` | 11 | `OSSL_PROVIDER_load` |
| `rdkafka_ssl.c::rd_kafka_transport_ssl_cert_verify_cb` | 13 | `CRYPTO_free`, `X509_NAME_oneline` |
| `rdkafka_ssl.c::rd_kafka_ssl_ctx_init` | 28 | `SSL_CTX_ctrl`, `SSL_CTX_set_default_passwd_cb` |
| `rdkafka_cert.c::rd_kafka_cert_new` | 63 | `BIO_new_mem_buf`, `OPENSSL_sk_pop_free` |
| `rdkafka_ssl.c::rd_kafka_ssl_set_certs` | 71 | `ENGINE_load_ssl_client_cert`, `OPENSSL_sk_pop_free`, `SSL_CTX_ctrl`, `SSL_CTX_set_cert_cb`, `SSL_CTX_use_cert_and_key`, `X509_OBJECT_get0_X509` |
| `rdkafka_sasl_oauthbearer_oidc.c::rd_kafka_oidc_assertion_create` | 72 | `BIO_new_mem_buf` |
