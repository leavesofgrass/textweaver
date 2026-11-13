### Security and privacy

- `tw text`, `tw search` and `tw info` leave out control characters other than tab and line feed when standard output is a terminal, so a document cannot set the terminal's title or write the clipboard through an escape sequence; error lines on a terminal follow the same rule. Piped output is unchanged.
- Web pages and reference lookups now send the same neutral User-Agent as every other request, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, without textweaver's version. A workspace test fails on any other User-Agent in the sources.
- The hot folder writes only plain names: a name taken in `processed` or `failed` gets `stem.STAMP-2.ext` instead of a space and parentheses, and outside names have spaces, dollar signs and other special characters replaced with `_` before anything is written.
- The hot folder no longer follows symbolic links or junctions; each one is skipped and named in the watch log.
- Images for OCR and QR codes are checked against the size limits from their headers before decoding, and an image of more than 40 million pixels is refused for a QR code before it is read.
