struct sample_record { int ready; int metadata; };

void sample_reset(struct sample_record *record) {
    record->metadata = 0;
}
