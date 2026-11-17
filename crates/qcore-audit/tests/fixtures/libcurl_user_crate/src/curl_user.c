/* Card LIBCURL-PROVIDER-PLUGIN test fixture: callers of libcurl functions
 * the user crate quirin-provider-libcurl contracts, declared as the pinned
 * 8.18.0 headers declare them (CURLcode an enumeration with no negative
 * value, CURLU an incomplete structure). `curl_global_init` is in every
 * version from 7.8, `curl_url` and `curl_url_cleanup` from 7.62.0. */
typedef enum { CURLE_OK = 0, CURLE_FAILED_INIT = 2 } CURLcode;
typedef struct Curl_URL CURLU;

CURLcode curl_global_init(long flags);
CURLU *curl_url(void);
void curl_url_cleanup(CURLU *handle);

CURLcode init_once(void) {
    return curl_global_init(3L);
}

void url_cycle(void) {
    CURLU *url = curl_url();
    curl_url_cleanup(url);
}
