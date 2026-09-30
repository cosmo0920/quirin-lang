/* The C side of examples/ffi/record_passing.qr (card W48-QR-USABLE-RECORDS). */
#include "qr_point.h"

static int32_t magnitude(int32_t value)
{
    return value < 0 ? -value : value;
}

int32_t qr_point_norm(const struct qr_point *point)
{
    return (magnitude(point->x) + magnitude(point->y)) * (int32_t) point->weight;
}

struct qr_point qr_point_moved(struct qr_point point, int32_t dx, int32_t dy)
{
    point.x += dx;
    point.y += dy;
    return point;
}
