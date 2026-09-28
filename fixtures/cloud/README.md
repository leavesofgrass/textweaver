# Fuzz seeds for the parser targets

Small inputs that `cargo xtask fuzz-seed` copies into the corpus of the fuzz targets added by the Cloud Agent's first task. See [fuzz/README.md](../../fuzz/README.md).

- `latex_math/`: LaTeX math, one expression per file, and one page of prose with math in it, for the `latex_math` target.
- `asciimath/`: ASCIIMath expressions, for the `asciimath` target.
- `rpc/`: JSON-RPC sessions, one message per line, for the `rpc` target. One is a normal session; the other is a list of malformed requests.

The citation, theme, and vault targets take their seeds from `fixtures/p/`, the built-in themes, and `fixtures/l/`. The lexicon target builds its own small data file.
