"""Builds the small PDF fixtures in this folder (and, on request, a large one).

    python fixtures/a/make_pdfs.py            # single.pdf, columns.pdf, running.pdf
    python fixtures/a/make_pdfs.py big OUT N  # an N-page PDF at OUT (not committed)

A tiny PDF writer using the standard 14 fonts (Helvetica and Times, with
WinAnsiEncoding), so the files are small, deterministic, and free of
copyrighted text. Each fixture targets one part of the PDF loader:

- single.pdf: one column; a title, headings by size, weight, and number,
  paragraphs with first-line indents and a hyphenated line end, bulleted
  and numbered lists, a small table, and bookmarks (an outline).
- columns.pdf: a full-width title, two columns, a full-width figure
  caption band between two column sections (star's divider rule).
- running.pdf: three pages with a running header, page-number footers, a
  paragraph that continues across a page break, and page labels (a
  roman-numbered first page).
- notes.pdf: a footnote in small type at the foot of page one, between a
  paragraph's first half and its continuation on page two.

Standard library only.
"""

import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent

FONTS = {
    "H": "Helvetica",
    "HB": "Helvetica-Bold",
    "T": "Times-Roman",
    "TI": "Times-Italic",
    "TB": "Times-Bold",
}


def esc(s):
    b = s.encode("cp1252")
    out = bytearray()
    for c in b:
        if c in (0x5C, 0x28, 0x29):
            out += b"\\" + bytes([c])
        else:
            out.append(c)
    return bytes(out)


def pdf_string(s):
    return b"(" + esc(s) + b")"


class Page:
    def __init__(self, width=612, height=792):
        self.width = width
        self.height = height
        self.ops = []

    def text(self, x, y, runs):
        """Draws runs [(font, size, text), ...] from (x, y) on one baseline."""
        parts = [b"BT"]
        parts.append(b"%.2f %.2f Td" % (x, y))
        for font, size, text in runs:
            parts.append(b"/%s %g Tf" % (font.encode(), size))
            parts.append(pdf_string(text) + b" Tj")
        parts.append(b"ET")
        self.ops.append(b" ".join(parts))

    def rule(self, x0, y, x1):
        self.ops.append(b"%.2f %.2f m %.2f %.2f l S" % (x0, y, x1, y))

    def content(self):
        return b"\n".join(self.ops) + b"\n"


def wrap(text, width_pt, size, avg=0.52):
    """Greedy word wrap with a conservative average glyph width."""
    max_chars = max(10, int(width_pt / (size * avg)))
    lines, cur = [], ""
    for word in text.split():
        cand = (cur + " " + word).strip()
        if len(cand) > max_chars and cur:
            lines.append(cur)
            cur = word
        else:
            cur = cand
    if cur:
        lines.append(cur)
    return lines


class Pdf:
    def __init__(self, title, author, lang="en-US"):
        self.title = title
        self.author = author
        self.lang = lang
        self.pages = []
        self.outline = []  # (title, page_index, y, level)

    def page(self):
        p = Page()
        self.pages.append(p)
        return p

    def bookmark(self, title, page_index, y, level=1):
        self.outline.append((title, page_index, y, level))

    def save(self, path):
        objs = []

        def add(body):
            objs.append(body)
            return len(objs)

        font_ids = {}
        for key, base in FONTS.items():
            font_ids[key] = add(
                b"<< /Type /Font /Subtype /Type1 /BaseFont /%s /Encoding /WinAnsiEncoding >>"
                % base.encode()
            )
        fonts = b" ".join(b"/%s %d 0 R" % (k.encode(), v) for k, v in font_ids.items())
        pages_id = len(objs) + 1
        objs.append(None)  # placeholder for /Pages
        page_ids = []
        for p in self.pages:
            data = p.content()
            cid = add(b"<< /Length %d >>\nstream\n" % len(data) + data + b"endstream")
            pid = add(
                b"<< /Type /Page /Parent %d 0 R /MediaBox [0 0 %d %d] /Resources << /Font << %s >> >> /Contents %d 0 R >>"
                % (pages_id, p.width, p.height, fonts, cid)
            )
            page_ids.append(pid)
        objs[pages_id - 1] = b"<< /Type /Pages /Kids [%s] /Count %d >>" % (
            b" ".join(b"%d 0 R" % i for i in page_ids),
            len(page_ids),
        )
        outline_ref = b""
        if self.outline:
            outline_ref = self._outline(objs, page_ids)
        if getattr(self, "labels", None):
            outline_ref += b" /PageLabels << /Nums [%s] >>" % b" ".join(
                b"%d << %s >>" % (start, style) for start, style in self.labels
            )
        catalog = add(
            b"<< /Type /Catalog /Pages %d 0 R /Lang %s%s >>"
            % (pages_id, pdf_string(self.lang), outline_ref)
        )
        info = add(
            b"<< /Title %s /Author %s /Producer (textweaver fixtures) >>"
            % (pdf_string(self.title), pdf_string(self.author))
        )
        out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
        offsets = []
        for i, body in enumerate(objs, 1):
            offsets.append(len(out))
            out += b"%d 0 obj\n" % i + body + b"\nendobj\n"
        xref = len(out)
        out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
        for off in offsets:
            out += b"%010d 00000 n \n" % off
        out += b"trailer\n<< /Size %d /Root %d 0 R /Info %d 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (
            len(objs) + 1,
            catalog,
            info,
            xref,
        )
        pathlib.Path(path).write_bytes(bytes(out))

    def _outline(self, objs, page_ids):
        # Two levels: level-2 entries nest under the preceding level-1 entry.
        root_id = len(objs) + 1
        objs.append(None)
        tops = []
        for title, page, y, level in self.outline:
            if level == 1 or not tops:
                tops.append([(title, page, y), []])
            else:
                tops[-1][1].append((title, page, y))
        ids = {}
        next_id = root_id + 1
        for t, kids in tops:
            ids[id(t)] = next_id
            next_id += 1
            for k in kids:
                ids[id(k)] = next_id
                next_id += 1

        def item(entry, parent, prev, nxt, kids):
            title, page, y = entry
            s = b"<< /Title %s /Parent %d 0 R /Dest [%d 0 R /XYZ 0 %d 0]" % (
                pdf_string(title), parent, page_ids[page], y)
            if prev:
                s += b" /Prev %d 0 R" % prev
            if nxt:
                s += b" /Next %d 0 R" % nxt
            if kids:
                s += b" /First %d 0 R /Last %d 0 R /Count %d" % (
                    ids[id(kids[0])], ids[id(kids[-1])], len(kids))
            return s + b" >>"

        bodies = {}
        for i, (t, kids) in enumerate(tops):
            prev = ids[id(tops[i - 1][0])] if i > 0 else 0
            nxt = ids[id(tops[i + 1][0])] if i + 1 < len(tops) else 0
            bodies[ids[id(t)]] = item(t, root_id, prev, nxt, kids)
            for j, k in enumerate(kids):
                kp = ids[id(kids[j - 1])] if j > 0 else 0
                kn = ids[id(kids[j + 1])] if j + 1 < len(kids) else 0
                bodies[ids[id(k)]] = item(k, ids[id(t)], kp, kn, [])
        objs[root_id - 1] = b"<< /Type /Outlines /First %d 0 R /Last %d 0 R /Count %d >>" % (
            ids[id(tops[0][0])], ids[id(tops[-1][0])], len(tops))
        for i in range(root_id + 1, next_id):
            objs.append(bodies[i])
        return b" /Outlines %d 0 R /PageMode /UseOutlines" % root_id


class Flow:
    """Lays out blocks top to bottom in one column of a page."""

    def __init__(self, pdf, x, width, top=720, bottom=72, page=None):
        self.pdf = pdf
        self.x = x
        self.width = width
        self.top = top
        self.bottom = bottom
        self.page = page or pdf.page()
        self.y = top

    def need(self, h):
        if self.y - h < self.bottom:
            self.page = self.pdf.page()
            self.y = self.top

    def heading(self, text, font="HB", size=14, before=10, after=6):
        self.y -= before
        self.need(size)
        self.page.text(self.x, self.y - size, [(font, size, text)])
        self.y -= size + after
        return self.pdf.pages.index(self.page), int(self.y + size + after)

    def paragraph(self, text, size=11, leading=14, indent=0, font="T", after=6,
                  lines=None):
        lines = lines or wrap(text, self.width - indent, size)
        for i, line in enumerate(lines):
            self.need(leading)
            dx = indent if i == 0 else 0
            runs = line if isinstance(line, list) else [(font, size, line)]
            self.page.text(self.x + dx, self.y - size, runs)
            self.y -= leading
        self.y -= after

    def item(self, marker, text, size=11, leading=14, depth=0):
        inset = 18 * (depth + 1)
        lines = wrap(text, self.width - inset - 14, size)
        for i, line in enumerate(lines):
            self.need(leading)
            if i == 0:
                self.page.text(self.x + inset - 14, self.y - size, [("T", size, marker)])
            self.page.text(self.x + inset, self.y - size, [("T", size, line)])
            self.y -= leading
        self.y -= 2

    def row(self, cells, xs, font="T", size=11, leading=15):
        self.need(leading)
        for text, x in zip(cells, xs):
            self.page.text(self.x + x, self.y - size, [(font, size, text)])
        self.y -= leading


LOREM = [
    "Reading aloud helps students who find print hard to follow. A reader "
    "that highlights each word as it is spoken keeps the eyes and ears "
    "together, and a reader that remembers where the student stopped "
    "makes long assignments manageable.",
    "The loader rebuilds structure from the page: it finds the body text "
    "size, treats larger or bolder short lines as headings, joins wrapped "
    "lines into paragraphs, and removes the hyphens that typesetting adds "
    "at line ends.",
    "Columns are read one after another, top to bottom, so a sentence is "
    "never interrupted by the text beside it. Running headers and page "
    "numbers repeat on every page and are left out of the reading.",
]


def make_single():
    pdf = Pdf("A Guide to Accessible Reading", "Test Author")
    f = Flow(pdf, 72, 468)
    f.heading("A Guide to Accessible Reading", size=20, before=0, after=4)
    f.paragraph("Test Author", size=11, font="TI", after=12)
    pg, y = f.heading("1 Introduction")
    pdf.bookmark("1 Introduction", pg, y)
    f.paragraph(LOREM[0], indent=18)
    # A hyphenated line end: "informa-" / "tion".
    f.paragraph("", lines=[
        "Every page carries informa-",
        "tion that a student may need, and the reader should say the words",
        "exactly as they are printed, with well-known compounds intact.",
    ], indent=18)
    pg, y = f.heading("1.1 Background", size=12)
    pdf.bookmark("1.1 Background", pg, y, level=2)
    f.paragraph(LOREM[1], indent=18)
    f.paragraph("The reader offers three kinds of help:", after=4)
    f.item("•", "highlighting each word as it is spoken, so the eyes can follow;")
    f.item("•", "moving by sentence, paragraph, or heading with single keys;")
    f.item("•", "remembering the place in every document.")
    f.y -= 6
    f.paragraph("To start reading:", after=4)
    f.item("1.", "Open the document.")
    f.item("2.", "Press the space bar to start and stop.")
    f.item("3.", "Use the arrow keys to move by word.")
    f.y -= 6
    f.heading("Summary", size=11)
    f.paragraph(LOREM[2], indent=18)
    pg, y = f.heading("2 Results")
    pdf.bookmark("2 Results", pg, y)
    f.paragraph("The table lists three readers and their scores.", after=8)
    xs = [0, 150, 300]
    f.row(["Name", "Role", "Score"], xs, font="TB")
    f.row(["Ada", "Engineer", "98"], xs)
    f.row(["Grace", "Admiral", "100"], xs)
    f.row(["Alan", "Mathematician", "95"], xs)
    f.y -= 10
    f.paragraph("The scores are close, and every reader finished the test.")
    pdf.save(HERE / "single.pdf")


def make_columns():
    pdf = Pdf("Two Column Layout", "Test Author")
    page = pdf.page()
    page.text(72, 720, [("HB", 18, "Two Column Layout")])
    colw = 222
    left = Flow(pdf, 72, colw, top=700, page=page)
    right = Flow(pdf, 72 + colw + 24, colw, top=700, page=page)
    left.heading("Left Column", size=12, before=0)
    left.paragraph("The left column begins the article. " + LOREM[0])
    left.paragraph("It ends with this sentence in the left column.")
    right.heading("Right Column", size=12, before=0)
    right.paragraph("The right column continues the article. " + LOREM[1])
    right.paragraph("It ends with this sentence in the right column.")
    band = min(left.y, right.y) - 16
    page.text(72, band, [("TI", 10, "Figure 1. A caption that runs across both columns of the page, below the first section.")])
    left2 = Flow(pdf, 72, colw, top=band - 20, page=page)
    right2 = Flow(pdf, 72 + colw + 24, colw, top=band - 20, page=page)
    left2.paragraph("Below the band, the left column resumes. " + LOREM[2][:120] + ".")
    right2.paragraph("Below the band, the right column ends the page. This is the last sentence.")
    pdf.save(HERE / "columns.pdf")


def make_running():
    pdf = Pdf("Running Heads", "Test Author")
    body = []
    for i in range(24):
        body.append(
            f"Paragraph {i + 1} of the running example. " + LOREM[i % 3]
        )
    f = Flow(pdf, 72, 468, top=700, bottom=90)
    f.heading("Running Heads and Page Numbers", size=16, before=0)
    for i, text in enumerate(body):
        f.paragraph(text, indent=18)
    # A paragraph that continues across the page break: fill the page
    # exactly so it breaks inside.
    for p in pdf.pages:
        n = pdf.pages.index(p) + 1
        p.text(72, 750, [("H", 9, "Journal of Reading Examples, Volume 3")])
        p.text(300, 40, [("H", 9, str(n))])
    # Printed page labels: a roman-numbered first page, then 1, 2, ...
    pdf.labels = [(0, b"/S /r"), (1, b"/S /D")]
    pdf.save(HERE / "running.pdf")


def make_notes():
    pdf = Pdf("Notes at the Foot", "Test Author")
    p1 = pdf.page()
    p1.text(72, 700, [("HB", 16, "Notes at the Foot")])
    y = 670
    for line in [
        "The first paragraph is short and complete. It ends here.",
    ]:
        p1.text(72, y, [("T", 11, line)])
        y -= 14
    y -= 8
    for line in [
        "The second paragraph begins near the foot of the first page, and its",
        "sentence carries on past the footnote below, which is printed in",
    ]:
        p1.text(72, y, [("T", 11, line)])
        y -= 14
    p1.rule(72, 110, 200)
    p1.text(72, 96, [("T", 8, "1 The footnote sits at the foot of page one, in smaller type.")])
    p2 = pdf.page()
    y = 700
    for line in [
        "smaller type, and ends on the second page without a break.",
        "",
        "A last paragraph closes the document.",
    ]:
        if line:
            p2.text(72, y, [("T", 11, line)])
        y -= 14
    pdf.save(HERE / "notes.pdf")


def make_big(out, pages):
    pdf = Pdf("A Very Long Document", "Test Author")
    f = Flow(pdf, 72, 468, top=700, bottom=90)
    chapter = 0
    while len(pdf.pages) < pages:
        chapter += 1
        f.heading(f"Chapter {chapter}", size=16)
        for i in range(12):
            f.paragraph(f"Chapter {chapter}, paragraph {i + 1}. " + LOREM[i % 3], indent=18)
    while len(pdf.pages) > pages:
        pdf.pages.pop()
    for n, p in enumerate(pdf.pages, 1):
        p.text(72, 750, [("H", 9, "A Very Long Document")])
        p.text(300, 40, [("H", 9, str(n))])
    pdf.save(out)


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "big":
        make_big(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 300)
    else:
        make_single()
        make_columns()
        make_running()
        make_notes()
        print("wrote single.pdf, columns.pdf, running.pdf, notes.pdf")
