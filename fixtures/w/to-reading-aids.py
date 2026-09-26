"""Integration helper (Agent W): after merging `wave2/w-fonts-pdf` into a
main that has Agent D3's `[reading_aids]` section, move the GUI's font
setting from `[display.font]` to `[reading_aids.font]`, so there is one
font setting. Run from the repository root:

    python fixtures/w/to-reading-aids.py

Resolve the merge conflicts first (checked on 2026-09-25 against main at
d64ada4):

- crates/textweaver-cli/src/cmd/convert.rs: keep both sides (main's
  `asciimath` argument and field, and W's `#[command(flatten)] layout` and
  `args.layout.apply(WriteOptions { .. })`).
- crates/textweaver-writers/src/pdf/mod.rs: take W's file (`check_fonts`
  calls `Fonts::discover(&options.pdf)`).
- crates/textweaver-store/src/settings.rs: take main's `STRUCT_TABLES`
  (20 entries) and drop W's `"display.font"` line.
- docs/adr/0017-writers.md, docs/tasks.md: keep both sides.

Then this script. With it, on the trial merge: workspace clippy and tests
green, the GUI builds and passes clippy, and
crates/textweaver-gui/tools/font-dialog-report.ps1 passes and saves
`[reading_aids.font]`. Every replacement asserts its anchor, so the script
stops rather than editing the wrong place."""


def edit(p, pairs):
    s = open(p, encoding="utf-8").read()
    for a, b in pairs:
        assert a in s, (p, a[:70])
        s = s.replace(a, b)
    open(p, "w", encoding="utf-8", newline="\n").write(s)


edit("crates/textweaver-store/src/settings.rs", [
    ("    /// `[display.font]`: the GUI's reading font (family, size in points,\n"
     "    /// weight). The terminal UI always shows the terminal's own font.\n"
     "    pub font: textweaver_aids::FontSettings,\n", ""),
    ("            font: textweaver_aids::FontSettings::default(),\n", ""),
    ("self.display.font.validate()", "self.reading_aids.font.validate()"),
    ("self.display.font.clamped()", "self.reading_aids.font.clamped()"),
    ('"display.font".into(),', '"reading_aids.font".into(),'),
    ("self.display.font = fixed;", "self.reading_aids.font = fixed;"),
    ("s.display.font.", "s.reading_aids.font."),
    ("[display.font]\\n", "[reading_aids.font]\\n"),
    ("store.load().0.display.font, s.display.font", "store.load().0.reading_aids.font, s.reading_aids.font"),
    ('w.starts_with("display.font")', 'w.starts_with("reading_aids.font")'),
])
edit("crates/textweaver-store/src/settings_io.rs", [
    ('        "display.font" => fits::<textweaver_aids::FontSettings>(key, value),\n', ""),
    ('        "display",\n        "display.font",\n', '        "display",\n'),
])
p = "crates/textweaver-store/src/settings_io_tests.rs"
s = open(p, encoding="utf-8").read()
i = s.index("    d.font = textweaver_aids::FontSettings {")
j = s.index("};", i) + 3
s = s[:i] + s[j:]
open(p, "w", encoding="utf-8", newline="\n").write(s)
for p in ["crates/textweaver-gui/src/window.rs", "crates/textweaver-gui/src/fonts.rs",
          "crates/textweaver-app/tests/settings_update.rs"]:
    s = open(p, encoding="utf-8").read()
    open(p, "w", encoding="utf-8", newline="\n").write(s.replace("display.font", "reading_aids.font"))
edit("docs/reading-aids.md", [
    ("under `[display.font]`:", "under `[reading_aids.font]`:"),
    ("[display.font]\nfamily", "[reading_aids.font]\nfamily"),
])
print("ok")
