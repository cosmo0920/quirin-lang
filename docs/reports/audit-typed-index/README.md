# Typed access statement-index optimization

Base: `4529c377`. Scope: preserve the existing Clang typed-access statement
selection and refusal rules while reducing repeated candidate walks, source
prefix scans, and retained AST cloning. This report is a work-in-progress
checkpoint before implementation; measurements and validation are owned by the
integrating audit run.
