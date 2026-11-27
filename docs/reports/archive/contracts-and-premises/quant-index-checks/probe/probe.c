/* QUANT-INDEX-CHECKS probe. The chunkio header functions are read from the
 * pinned corpus source through the include path (not copied here); the two
 * callers below are written for this card. The header uses `ssize_t` and
 * relies on its includer for <sys/types.h>, as chunkio's own sources do. */
#include <sys/types.h>
#include <chunkio/cio_file_st.h>

uint16_t probe_full_header(void)
{
    char header[24] = {0};
    return cio_file_st_get_meta_len(header);
}

uint16_t probe_short_header(void)
{
    char header[16] = {0};
    return cio_file_st_get_meta_len(header);
}
