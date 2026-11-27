| Function | Pairs | Class | Proposed | Published | Undischarged obligation |
|---|---:|---|---|---|---|
| `ERR_clear_error` | 3 | no-pointer-interface | Safe | **Pending** |  |
| `X509_free` | 6 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `BIO_free` | 5 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `BIO_new_mem_buf` | 4 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_PKEY_free` | 4 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `ENGINE_free` | 3 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `BIO_new_file` | 2 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_sha256` | 2 | static-object-return | Safe | **Pending** |  |
| `OPENSSL_sk_pop_free` | 2 | release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `OPENSSL_sk_push` | 2 | container | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `OPENSSL_sk_value` | 2 | container | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `PEM_read_bio_PrivateKey` | 2 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `PKCS12_free` | 2 | release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_ctrl` | 2 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_free` | 2 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_set_default_verify_paths` | 2 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_write` | 2 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `d2i_PKCS12_bio` | 2 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `CRYPTO_free` | 1 | release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `ENGINE_by_id` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `ENGINE_load_ssl_client_cert` | 1 | ownership-transfer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `ERR_error_string_n` | 1 | buffer-writer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `ERR_peek_error` | 1 | no-pointer-interface | Safe | **Pending** |  |
| `ERR_peek_last_error` | 1 | no-pointer-interface | Safe | **Pending** |  |
| `ERR_peek_last_error_all` | 1 | buffer-writer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_EncodeBlock` | 1 | buffer-writer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_MD_CTX_free` | 1 | release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_MD_CTX_new` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `EVP_sha512` | 1 | static-object-return | Safe | **Pending** |  |
| `OPENSSL_sk_free` | 1 | release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `OPENSSL_sk_new_null` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `OSSL_PROVIDER_load` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `OSSL_PROVIDER_unload` | 1 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `PEM_read_bio_X509` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_get_cert_store` | 1 | pure-reader | Safe | **Pending** |  |
| `SSL_CTX_load_verify_locations` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_new` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_set_cert_cb` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_set_default_passwd_cb` | 1 | member-store | Safe | **Pending** |  |
| `SSL_CTX_set_default_passwd_cb_userdata` | 1 | member-store | Safe | **Pending** |  |
| `SSL_CTX_set_options` | 1 | member-store | Safe | **Pending** |  |
| `SSL_CTX_use_PrivateKey` | 1 | ownership-transfer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_use_PrivateKey_file` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_use_cert_and_key` | 1 | ownership-transfer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_use_certificate` | 1 | ownership-transfer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_CTX_use_certificate_chain_file` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_certs_clear` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_connect` | 1 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_ctrl` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_do_handshake` | 1 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_free` | 1 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_get1_peer_certificate` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_get_certificate` | 1 | pure-reader | Unknown | **Unknown** | `class-rule`: the body holds CallExpr, which the rule does not admit |
| `SSL_get_client_CA_list` | 1 | pure-reader | Unknown | **Unknown** | `class-rule`: the body holds CallExpr, which the rule does not admit |
| `SSL_get_error` | 1 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_read` | 1 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `SSL_shutdown` | 1 | protocol-io | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `X509_NAME_oneline` | 1 | buffer-writer | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `X509_OBJECT_get0_X509` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_STORE_CTX_get_current_cert` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_STORE_CTX_get_error` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_STORE_CTX_get_error_depth` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_STORE_CTX_set_error` | 1 | member-store | Safe | **Pending** |  |
| `X509_STORE_free` | 1 | refcounted-release | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `X509_STORE_get0_objects` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_STORE_set_flags` | 1 | configuration | Unknown | **Unknown** | `class-rule`: no rule for the class |
| `X509_get_issuer_name` | 1 | pure-reader | Safe | **Pending** |  |
| `X509_new` | 1 | constructor | Unknown | **Unknown** | `class-rule`: no rule for the class |

published: {'Unknown': 52, 'Pending': 16} ; report verdict: Unknown
