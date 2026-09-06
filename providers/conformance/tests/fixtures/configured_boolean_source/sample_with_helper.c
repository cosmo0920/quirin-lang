struct sample_record { int ready; int metadata; };
void sample_reset(struct sample_record *record);

int sample_prepare(struct sample_record *record) {
    sample_reset(record);
    return record->ready;
}
