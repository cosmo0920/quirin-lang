struct sample_record { int ready; int metadata; };

int sample_ready(struct sample_record *record) {
    return record->ready;
}

int sample_toggle(struct sample_record *record) {
    record->ready = !record->ready;
    return record->ready;
}

extern void opaque(void); void unsupported(void) {opaque();}
#if 0
int excluded(void) {return 7;}
#endif
