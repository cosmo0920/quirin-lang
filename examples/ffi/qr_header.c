/* The implementation behind examples/ffi/qr_header.h. The emitted C of
 * `header_sum.qr` is linked with this object; nothing here is generated. */
#include "qr_header.h"

uint32_t qr_header_total(const struct qr_header *header)
{
    return header->len + (uint32_t) header->flags;
}

struct qr_header qr_header_make(uint32_t len, uint8_t flags)
{
    struct qr_header header = {0};
    header.len = len;
    header.flags = flags;
    header.id = (int64_t) len * 3;
    header.valid = true;
    header.label = "made";
    return header;
}
