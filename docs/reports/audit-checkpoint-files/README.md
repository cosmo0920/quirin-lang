# Per-configuration audit checkpoints

Base: `2e44e963`. This card changes `--checkpoint-dir` to seal one complete `.qo` for each planned compile configuration as its worker finishes, retaining ordered console and report publication. It preserves the existing four-value verdict and makes no checker or provider changes.

Evidence and limits will be recorded here after the implementation and standing gate.
