/* The C struct the FFI survey cells of card W48-QR-USABLE-RECORDS import. */
#ifndef SURVEY_REC_H
#define SURVEY_REC_H

#include <stdint.h>

struct survey_rec {
    uint32_t len;
    uint8_t flags;
};

struct survey_outer {
    struct survey_rec inner;
    uint32_t n;
};

#endif
