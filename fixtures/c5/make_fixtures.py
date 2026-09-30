"""Builds the PDF fixtures in this folder (Agent W6c5, ADR-0048).

    cargo run -p textweaver-ocr --example make_c5_scans   # the two PNGs
    python fixtures/c5/make_fixtures.py

Standard library only. A tiny PDF writer with the standard 14 fonts
(Helvetica, WinAnsiEncoding); the text is written for these fixtures.

- comments.pdf: a lecturer's comments on a student's essay: a highlight
  with a comment, a reply, and a review state (resolved); a sticky note
  beside a line; a strike-out with nothing typed; a hidden note that must
  not be read. Links: a web address, a link to the Methods heading on
  page 2, a link to page 3 (which has no heading), and a script link that
  must be dropped.
- form.pdf: a filled form: a text field with a value (Name), an empty
  required one (Student ID), a check box after its words, a radio group
  (Payment), a list with an accessible name (Course), a signature field,
  a date written on underscores, and a Submit button to leave out.
- captions.pdf: a table under "Table 1: Scores by student" and a figure
  caption, "Figure 1. The water cycle, from sea to cloud to rain."
- sideways-scan.pdf: sideways-scan.png on a landscape page (the scan
  reads bottom to top).
- table-scan.pdf: table-scan.png, a scanned table with its caption.
"""

import pathlib
import struct
import zlib

HERE = pathlib.Path(__file__).resolve().parent

# Helvetica advance widths (per 1000), for placing rectangles over words.
W = {" ": 278, ".": 278, ",": 278, ":": 278, "-": 333, "'": 222, "(": 333,
     ")": 333, "?": 556, "/": 278, "_": 556, "*": 389}
for c, w in zip("abcdefghijklmnopqrstuvwxyz",
                [556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222,
                 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500,
                 500, 500]):
    W[c] = w
for c, w in zip("ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                [667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556,
                 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
                 667, 611]):
    W[c] = w
for c in "0123456789":
    W[c] = 556


def width(s, size):
    return sum(W.get(c, 556) for c in s) * size / 1000.0


def esc(s):
    out = bytearray()
    for c in s.encode("cp1252"):
        if c in (0x5C, 0x28, 0x29):
            out += b"\\" + bytes([c])
        else:
            out.append(c)
    return bytes(out)


def lit(s):
    return b"(" + esc(s) + b")"


class Pdf:
    """Objects by number; 1 is the catalog, 2 the page tree."""

    def __init__(self):
        self.objs = {}
        self.next = 3
        self.pages = []

    def add(self, body):
        n = self.next
        self.next += 1
        self.objs[n] = body
        return n

    def stream(self, data, extra=b""):
        return self.add(b"<< /Length %d %s >>\nstream\n" % (len(data), extra)
                        + data + b"\nendstream")

    def page(self, ops, annots=(), size=(612, 792), xobjects=b""):
        content = self.stream(ops)
        n = self.next
        self.next += 1
        self.pages.append(n)
        annots_ref = b" ".join(b"%d 0 R" % a for a in annots)
        self.objs[n] = (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %d %d] "
            b"/Resources << /Font << /F1 << /Type /Font /Subtype /Type1 "
            b"/BaseFont /Helvetica /Encoding /WinAnsiEncoding >> /F2 << "
            b"/Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold "
            b"/Encoding /WinAnsiEncoding >> >> /XObject << %s >> >> "
            b"/Contents %d 0 R /Annots [%s] >>"
            % (size[0], size[1], xobjects, content, annots_ref))
        return n

    def reserve(self):
        n = self.next
        self.next += 1
        return n

    def write(self, path, catalog_extra=b""):
        self.objs[1] = b"<< /Type /Catalog /Pages 2 0 R /Lang (en) %s >>" % catalog_extra
        kids = b" ".join(b"%d 0 R" % p for p in self.pages)
        self.objs[2] = b"<< /Type /Pages /Kids [%s] /Count %d >>" % (kids, len(self.pages))
        out = bytearray(b"%PDF-1.7\n")
        offsets = {}
        for n in sorted(self.objs):
            offsets[n] = len(out)
            out += b"%d 0 obj\n" % n + self.objs[n] + b"\nendobj\n"
        size = max(self.objs) + 1
        xref = len(out)
        out += b"xref\n0 %d\n0000000000 65535 f \n" % size
        for n in range(1, size):
            if n in offsets:
                out += b"%010d 00000 n \n" % offsets[n]
            else:
                out += b"0000000000 65535 f \n"
        out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (size, xref)
        path.write_bytes(bytes(out))
        print(f"{path.name}: {len(out)} bytes")


def text(x, y, s, font="F1", size=11):
    return b"BT /%s %g Tf %.2f %.2f Td %s Tj ET\n" % (font.encode(), size, x, y, lit(s))


def span(x, y, line, phrase, size=11):
    """The rectangle over `phrase` within `line` drawn at (x, y)."""
    start = line.index(phrase)
    x0 = x + width(line[:start], size)
    x1 = x0 + width(phrase, size)
    return (x0, y - 0.22 * size, x1, y + 0.75 * size)


def rect(r):
    return b"[%.2f %.2f %.2f %.2f]" % r


def quad(r):
    x0, y0, x1, y1 = r
    return b"[%.2f %.2f %.2f %.2f %.2f %.2f %.2f %.2f]" % (x0, y1, x1, y1, x0, y0, x1, y0)


def comments():
    pdf = Pdf()
    p1_lines = [
        "The river carries water from the hills to the sea.",
        "Most of the water in the river falls as rain in the spring.",
        "Please read the course page before class on Monday.",
        "The next section explains how the samples were taken; see Methods.",
        "The raw numbers are listed on page 3, see page 3.",
        "This sentence was a draft and should go.",
        "A final line for the script link test.",
    ]
    y = 700
    ops = text(72, 740, "Where the water goes", "F2", 18)
    pos = {}
    for line in p1_lines:
        ops += text(72, y, line)
        pos[line] = y
        y -= 24
    page2 = pdf.reserve()
    page3 = pdf.reserve()
    annots = []
    # A highlight with a comment, a reply, and a review state.
    l2 = p1_lines[1]
    hl = span(72, pos[l2], l2, "falls as rain in the spring")
    popup_parent = pdf.reserve()
    pdf.objs[popup_parent] = (
        b"<< /Type /Annot /Subtype /Highlight /Rect %s /QuadPoints %s "
        b"/Contents %s /T (Ada Example) /M (D:20260901103000Z) >>"
        % (rect(hl), quad(hl), lit("Cite a source for this."))
    )
    annots.append(popup_parent)
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Text /Rect [560 %d 580 %d] /IRT %d 0 R "
        b"/Contents %s /T (Bo Example) /M (D:20260902090000Z) >>"
        % (pos[l2], pos[l2] + 16, popup_parent, lit("Added a citation."))))
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Text /Rect [560 %d 580 %d] /IRT %d 0 R "
        b"/State (Completed) /StateModel (Review) /T (Ada Example) /F 2 >>"
        % (pos[l2], pos[l2] + 16, popup_parent)))
    # A sticky note in the margin beside the first line.
    l1 = p1_lines[0]
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Text /Rect [540 %d 560 %d] /Contents %s "
        b"/T (Ada Example) >>" % (pos[l1] - 4, pos[l1] + 12, lit("Good opening sentence."))))
    # A strike-out with nothing typed.
    l6 = p1_lines[5]
    so = span(72, pos[l6], l6, "This sentence was a draft")
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /StrikeOut /Rect %s /QuadPoints %s /T (Ada Example) >>"
        % (rect(so), quad(so))))
    # A hidden note: never read.
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Text /Rect [540 600 560 616] /F 2 /Contents %s >>"
        % lit("Hidden note, not for reading.")))
    # Links.
    l3 = p1_lines[2]
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Link /Rect %s /A << /S /URI /URI (https://example.org/course) >> >>"
        % rect(span(72, pos[l3], l3, "the course page"))))
    l4 = p1_lines[3]
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Link /Rect %s /Dest [%d 0 R /XYZ 0 740 0] >>"
        % (rect(span(72, pos[l4], l4, "see Methods")), page2)))
    l5 = p1_lines[4]
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Link /Rect %s /A << /S /GoTo /D (rawdata) >> >>"
        % rect(span(72, pos[l5], l5, "see page 3"))))
    l7 = p1_lines[6]
    annots.append(pdf.add(
        b"<< /Type /Annot /Subtype /Link /Rect %s /A << /S /JavaScript /JS (app.alert\\(1\\)) >> >>"
        % rect(span(72, pos[l7], l7, "script link"))))
    pdf.page(ops, annots)
    # Page 2: the Methods heading.
    ops2 = text(72, 720, "Methods", "F2", 18)
    ops2 += text(72, 690, "Samples were taken every week from three places on the river.")
    ops2 += text(72, 666, "Each sample was weighed, dried, and weighed again.")
    content2 = pdf.stream(ops2)
    fonts = (b"/Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
             b"/Encoding /WinAnsiEncoding >> /F2 << /Type /Font /Subtype /Type1 "
             b"/BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >> >> >>")
    pdf.objs[page2] = (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] %s /Contents %d 0 R >>"
                       % (fonts, content2))
    pdf.pages.append(page2)
    ops3 = text(72, 720, "Week one: 12.1, 11.8, and 12.4 grams of silt in each litre.")
    ops3 += text(72, 696, "Week two: 10.2, 10.9, and 11.3 grams of silt in each litre.")
    content3 = pdf.stream(ops3)
    pdf.objs[page3] = (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] %s /Contents %d 0 R >>"
                       % (fonts, content3))
    pdf.pages.append(page3)
    names = pdf.add(b"<< /Dests << /Names [(rawdata) [%d 0 R /Fit]] >> >>" % page3)
    pdf.write(HERE / "comments.pdf", b"/Names %d 0 R" % names)


def appearance(pdf, on):
    """A check box's two appearances: on (a cross) and off (empty)."""
    yes = pdf.stream(b"0 0 m 10 10 l 0 10 m 10 0 l S", b"/Type /XObject /Subtype /Form /BBox [0 0 10 10]")
    off = pdf.stream(b"", b"/Type /XObject /Subtype /Form /BBox [0 0 10 10]")
    return b"/AP << /N << /%s %d 0 R /Off %d 0 R >> >>" % (on, yes, off)


def form():
    pdf = Pdf()
    page = pdf.reserve()
    ops = text(72, 740, "Field trip permission form", "F2", 18)
    ops += text(72, 700, "Name:")
    ops += text(72, 670, "Student ID:")
    ops += text(72, 640, "Date: ____________________")
    ops += text(92, 610, "I agree to the terms of the trip")
    ops += text(72, 580, "How will you pay?")
    ops += text(92, 560, "Credit card")
    ops += text(92, 540, "Check")
    ops += text(72, 510, "Course:")
    ops += text(72, 470, "Signature:")
    fields = []

    def widget(extra):
        n = pdf.add(b"<< /Type /Annot /Subtype /Widget /P %d 0 R %s >>" % (page, extra))
        fields.append(n)
        return n

    widget(b"/FT /Tx /T (name) /V (Ada Example) /Rect [110 695 300 712]")
    widget(b"/FT /Tx /T (student_id) /Ff 2 /Rect [140 665 300 682]")
    widget(b"/FT /Tx /T (date) /Rect [100 636 230 652]")
    widget(b"/FT /Btn /T (agree) /V /Yes /AS /Yes /Rect [72 607 84 619] "
           + appearance(pdf, b"Yes"))
    # The radio group: a parent field with two widget kids.
    group = pdf.reserve()
    card = pdf.add(b"<< /Type /Annot /Subtype /Widget /P %d 0 R /Parent %d 0 R "
                   b"/AS /Card /Rect [72 557 84 569] %s >>" % (page, group, appearance(pdf, b"Card")))
    check = pdf.add(b"<< /Type /Annot /Subtype /Widget /P %d 0 R /Parent %d 0 R "
                    b"/AS /Off /Rect [72 537 84 549] %s >>" % (page, group, appearance(pdf, b"Check")))
    pdf.objs[group] = (b"<< /FT /Btn /Ff 49152 /T (payment) /V /Card /Kids [%d 0 R %d 0 R] >>"
                       % (card, check))
    widget(b"/FT /Ch /T (course) /TU (Course you are taking) /V (bio101) "
           b"/Opt [[(bio101) (Biology 101)] [(chem110) (Chemistry 110)]] /Rect [130 505 300 522]")
    widget(b"/FT /Sig /T (signature) /Rect [140 462 320 482]")
    widget(b"/FT /Btn /Ff 65536 /T (submit) /Rect [72 420 150 440]")
    widgets = fields + [card, check]
    content = pdf.stream(ops)
    fonts = (b"/Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
             b"/Encoding /WinAnsiEncoding >> /F2 << /Type /Font /Subtype /Type1 "
             b"/BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >> >> >>")
    pdf.objs[page] = (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] %s /Contents %d 0 R "
                      b"/Annots [%s] >>" % (fonts, content, b" ".join(b"%d 0 R" % w for w in widgets)))
    pdf.pages.append(page)
    roots = fields + [group]
    acro = pdf.add(b"<< /Fields [%s] >>" % b" ".join(b"%d 0 R" % f for f in roots))
    pdf.write(HERE / "form.pdf", b"/AcroForm %d 0 R" % acro)


def captions():
    pdf = Pdf()
    ops = text(72, 740, "Results", "F2", 18)
    ops += text(72, 710, "The scores rose in the second term for every student.")
    ops += text(72, 680, "Table 1: Scores by student")
    rows = [("Student", "First term", "Second term"),
            ("Ada", "71", "78"), ("Bo", "64", "70"), ("Cy", "82", "85")]
    y = 656
    for i, row in enumerate(rows):
        for x, cell in zip((72, 220, 360), row):
            ops += text(x, y, cell, "F2" if i == 0 else "F1")
        y -= 18
    ops += text(72, y - 20, "Figure 1. The water cycle, from sea to cloud to rain.")
    ops += text(72, y - 50, "Figure 1 shows how the water moves through the valley.")
    pdf.page(ops)
    pdf.write(HERE / "captions.pdf")


def png_gray(path):
    """Width, height, and zlib data of an 8-bit greyscale PNG."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, w, h = 8, b"", 0, 0
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and color == 0 and interlace == 0, "8-bit grey only"
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    zlib.decompress(idat)  # check it is whole
    return w, h, idat


def scan_pdf(png, out, size):
    w, h, idat = png_gray(HERE / png)
    pdf = Pdf()
    img = pdf.stream(idat, b"/Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace "
                     b"/DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /DecodeParms "
                     b"<< /Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns %d >>" % (w, h, w))
    ops = b"q %d 0 0 %d 0 0 cm /Im0 Do Q" % size
    pdf.page(ops, size=size, xobjects=b"/Im0 %d 0 R" % img)
    pdf.write(HERE / out)


if __name__ == "__main__":
    comments()
    form()
    captions()
    scan_pdf("sideways-scan.png", "sideways-scan.pdf", (792, 612))
    scan_pdf("table-scan.png", "table-scan.pdf", (612, 792))
