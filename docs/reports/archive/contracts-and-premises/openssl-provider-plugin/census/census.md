| Route | Units | OpenSSL callees | Sites | Pairs | Rows blocked | Sole | Co |
|---|---:|---:|---:|---:|---:|---:|---:|
| corpus | 185 | 68 | 147 | 99 | 29 | 4 | 25 |
| system | 15 | 0 | 0 | 0 | 0 | 0 | 0 |
| slice | 16 | 0 | 0 | 0 | 0 | 0 | 0 |
| current | 187 | 72 | 174 | 116 | 38 | 6 | 32 |

| Source | Analysed by | OpenSSL functions | Sites |
|---|---|---:|---:|
| `src/tls/openssl.c` | none | 73 | 133 |
| `src/flb_hmac.c` | none | 21 | 27 |
| `src/flb_hash.c` | none | 10 | 14 |
| `src/flb_crypto.c` | none | 35 | 69 |
| `src/flb_utils.c` | none | 1 | 1 |
| `src/flb_oauth2.c` | none | 8 | 17 |
| `src/flb_fips.c` | none | 9 | 20 |
| `plugins/out_azure_kusto/azure_kusto_conf.c` | none | 1 | 2 |
| `lib/monkey/plugins/tls/tls_openssl.c` | none | 25 | 30 |
| `lib/monkey/tls/tls_openssl.c` | current | 37 | 54 |

### corpus

| Callee | Sites | Pairs | Rows sole (single callee) | Rows co | Libraries |
|---|---:|---:|---:|---:|---|
| `X509_free` | 18 | 6 | 0 (0) | 6 | librdkafka |
| `BIO_free` | 13 | 5 | 1 (0) | 4 | librdkafka |
| `BIO_new_mem_buf` | 4 | 4 | 1 (0) | 3 | librdkafka |
| `EVP_PKEY_free` | 10 | 4 | 0 (0) | 4 | librdkafka |
| `ENGINE_free` | 6 | 3 | 1 (0) | 2 | librdkafka |
| `ERR_clear_error` | 3 | 3 | 1 (1) | 2 | librdkafka |
| `BIO_new_file` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `EVP_sha256` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_pop_free` | 10 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_push` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_value` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `PEM_read_bio_PrivateKey` | 3 | 2 | 1 (0) | 1 | librdkafka |
| `PKCS12_free` | 4 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_CTX_ctrl` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_CTX_free` | 2 | 2 | 1 (0) | 1 | librdkafka |
| `SSL_CTX_set_default_verify_paths` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_write` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `d2i_PKCS12_bio` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `CRYPTO_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ENGINE_by_id` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `ENGINE_load_ssl_client_cert` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_error_string_n` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_last_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_last_error_all` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_EncodeBlock` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_MD_CTX_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_MD_CTX_new` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_sha512` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `OPENSSL_sk_free` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `OPENSSL_sk_new_null` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `OSSL_PROVIDER_load` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `OSSL_PROVIDER_unload` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `PEM_read_bio_X509` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_get_cert_store` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_load_verify_locations` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_new` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_cert_cb` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_default_passwd_cb` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_default_passwd_cb_userdata` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_options` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_PrivateKey` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_PrivateKey_file` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_cert_and_key` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_certificate` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_certificate_chain_file` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_certs_clear` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_connect` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_ctrl` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_do_handshake` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_free` | 1 | 1 | 1 (0) | 0 | librdkafka |
| `SSL_get1_peer_certificate` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get_certificate` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get_client_CA_list` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_read` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_shutdown` | 1 | 1 | 1 (0) | 0 | librdkafka |
| `X509_NAME_oneline` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `X509_OBJECT_get0_X509` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_current_cert` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_error_depth` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_set_error` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_get0_objects` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_set_flags` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_get_issuer_name` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_new` | 1 | 1 | 0 (0) | 1 | librdkafka |

### current

| Callee | Sites | Pairs | Rows sole (single callee) | Rows co | Libraries |
|---|---:|---:|---:|---:|---|
| `X509_free` | 23 | 7 | 0 (0) | 7 | librdkafka, monkey |
| `BIO_free` | 15 | 6 | 1 (0) | 5 | librdkafka, monkey |
| `BIO_new_mem_buf` | 6 | 5 | 1 (0) | 4 | librdkafka, monkey |
| `EVP_PKEY_free` | 13 | 5 | 0 (0) | 5 | librdkafka, monkey |
| `SSL_write` | 5 | 4 | 0 (0) | 4 | librdkafka, monkey |
| `ENGINE_free` | 6 | 3 | 1 (0) | 2 | librdkafka |
| `ERR_clear_error` | 3 | 3 | 1 (1) | 2 | librdkafka |
| `PEM_read_bio_PrivateKey` | 4 | 3 | 1 (0) | 2 | librdkafka, monkey |
| `SSL_free` | 3 | 3 | 1 (0) | 2 | librdkafka, monkey |
| `BIO_new_file` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `EVP_sha256` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_pop_free` | 10 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_push` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `OPENSSL_sk_value` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `PEM_read_bio_X509` | 2 | 2 | 0 (0) | 2 | librdkafka, monkey |
| `PKCS12_free` | 4 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_CTX_ctrl` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_CTX_free` | 2 | 2 | 1 (0) | 1 | librdkafka |
| `SSL_CTX_new` | 2 | 2 | 1 (1) | 1 | librdkafka, monkey |
| `SSL_CTX_set_default_verify_paths` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `SSL_get_error` | 2 | 2 | 0 (0) | 2 | librdkafka, monkey |
| `SSL_shutdown` | 2 | 2 | 1 (0) | 1 | librdkafka, monkey |
| `d2i_PKCS12_bio` | 2 | 2 | 0 (0) | 2 | librdkafka |
| `CRYPTO_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ENGINE_by_id` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `ENGINE_load_ssl_client_cert` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_error_string_n` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_last_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `ERR_peek_last_error_all` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_EncodeBlock` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_MD_CTX_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_MD_CTX_new` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `EVP_sha512` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `OPENSSL_init_ssl` | 2 | 1 | 1 (1) | 0 | monkey |
| `OPENSSL_sk_free` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `OPENSSL_sk_new_null` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `OSSL_PROVIDER_load` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `OSSL_PROVIDER_unload` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_get_cert_store` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_load_verify_locations` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_cert_cb` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_default_passwd_cb` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_default_passwd_cb_userdata` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_set_options` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_PrivateKey` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_PrivateKey_file` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_cert_and_key` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_certificate` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_CTX_use_certificate_chain_file` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_certs_clear` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_connect` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_ctrl` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_do_handshake` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get1_peer_certificate` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get_certificate` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_get_client_CA_list` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_new` | 1 | 1 | 0 (0) | 1 | monkey |
| `SSL_read` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `SSL_set_accept_state` | 1 | 1 | 0 (0) | 1 | monkey |
| `SSL_set_fd` | 1 | 1 | 0 (0) | 1 | monkey |
| `X509_NAME_oneline` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `X509_OBJECT_get0_X509` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_current_cert` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_error` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_get_error_depth` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_CTX_set_error` | 2 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_free` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_get0_objects` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_STORE_set_flags` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_get_issuer_name` | 1 | 1 | 0 (0) | 1 | librdkafka |
| `X509_new` | 1 | 1 | 0 (0) | 1 | librdkafka |
