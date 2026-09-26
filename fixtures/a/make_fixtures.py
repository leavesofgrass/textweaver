"""Builds the small EPUB and DOCX fixtures in this folder.

    python fixtures/a/make_fixtures.py

Writes `sample.epub` (EPUB 3 with a navigation document and an EPUB 2 NCX)
and `sample.docx` (WordprocessingML written by hand to cover headings,
lists, runs, links, images, tables, footnotes, and endnotes). The zip
timestamps are fixed so the output only changes when this script does.
Standard library only.
"""

import pathlib
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
STAMP = (1980, 1, 1, 0, 0, 0)


def write_zip(path, members, stored_first=None):
    with zipfile.ZipFile(path, "w") as z:
        if stored_first:
            name, data = stored_first
            info = zipfile.ZipInfo(name, STAMP)
            info.compress_type = zipfile.ZIP_STORED
            z.writestr(info, data)
        for name, data in members:
            info = zipfile.ZipInfo(name, STAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)


# --------------------------------------------------------------------------
# EPUB
# --------------------------------------------------------------------------

CONTAINER = """<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"""

OPF = """<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="id">urn:uuid:7b0c4a52-0d1e-4c1b-9d6c-textweaver-a2</dc:identifier>
    <dc:title>Sample EPUB Book</dc:title>
    <dc:creator>Ada Author</dc:creator>
    <dc:creator>Grace Writer</dc:creator>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="cover" href="text/cover.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="text/ch%202.xhtml" media-type="application/xhtml+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="cover"/>
    <itemref idref="nav" linear="no"/>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>
"""

NAV = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Contents</title></head>
<body>
  <nav epub:type="toc" id="toc">
    <h1>Contents</h1>
    <ol>
      <li><a href="text/ch1.xhtml">Chapter One: Beginnings</a>
        <ol><li><a href="text/ch1.xhtml#sec2">A Section Inside</a></li></ol>
      </li>
      <li><a href="text/ch%202.xhtml">Chapter Two: Tables and Lists</a></li>
    </ol>
  </nav>
  <nav epub:type="landmarks" hidden="">
    <ol><li><a epub:type="bodymatter" href="text/ch1.xhtml">Start</a></li></ol>
  </nav>
</body>
</html>
"""

NCX = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE ncx PUBLIC "-//NISO//DTD ncx 2005-1//EN" "http://www.daisy.org/z3986/2005/ncx-2005-1.dtd">
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="urn:uuid:7b0c4a52"/></head>
  <docTitle><text>Sample EPUB Book</text></docTitle>
  <navMap>
    <navPoint id="n1" playOrder="1"><navLabel><text>Chapter One (NCX)</text></navLabel><content src="text/ch1.xhtml"/></navPoint>
    <navPoint id="n2" playOrder="2"><navLabel><text>Chapter Two (NCX)</text></navLabel><content src="text/ch%202.xhtml"/></navPoint>
  </navMap>
</ncx>
"""

COVER = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">
<head><title>Cover</title><link rel="stylesheet" href="../style.css"/></head>
<body>
  <h1 class="title">Sample EPUB Book</h1>
  <p>By Ada Author and Grace Writer.</p>
</body>
</html>
"""

CH1 = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="en">
<head><title>Chapter One</title><meta charset="utf-8"/></head>
<body>
  <section epub:type="chapter">
    <h1>Chapter One: Beginnings</h1>
    <p>The <em>first</em> chapter opens with <strong>bold words</strong>.
    Dr. Smith opened the library at 9:30 a.m. on Friday.<a epub:type="noteref" href="#fn1">1</a></p>
    <aside epub:type="footnote" id="fn1"><p>An aside the reader skips.</p></aside>
    <section id="sec2">
      <h2>A Section Inside</h2>
      <p>This section is a nested entry in the table of contents.</p>
    </section>
  </section>
</body>
</html>
"""

CH2 = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">
<head><title>Chapter Two</title></head>
<body>
  <h1>Chapter Two: Tables and Lists</h1>
  <p><img src="../images/map.png" alt="A map of the old town"/></p>
  <p><img src="../images/rule.png" alt=""/>The café serves crème brûlée.</p>
  <table>
    <caption>Opening hours</caption>
    <tr><th>Day</th><th>Hours</th></tr>
    <tr><td>Monday</td><td>9 to 5</td></tr>
  </table>
  <ol>
    <li>Arrive early.</li>
    <li>Find a seat.</li>
  </ol>
</body>
</html>
"""


def make_epub():
    write_zip(
        HERE / "sample.epub",
        [
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", OPF),
            ("OEBPS/nav.xhtml", NAV),
            ("OEBPS/toc.ncx", NCX),
            ("OEBPS/text/cover.xhtml", COVER),
            ("OEBPS/text/ch1.xhtml", CH1),
            ("OEBPS/text/ch 2.xhtml", CH2),
            ("OEBPS/style.css", "h1.title { text-align: center; }\n"),
        ],
        stored_first=("mimetype", "application/epub+zip"),
    )


# --------------------------------------------------------------------------
# DOCX
# --------------------------------------------------------------------------

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
R = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
WP = 'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"'
A = 'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"'

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
  <Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>
  <Override PartName="/word/endnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>
"""

ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>
"""

DOC_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org" TargetMode="External"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/>
</Relationships>
"""

CORE = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>Sample Word Document</dc:title>
  <dc:creator>Test Author</dc:creator>
  <dc:language>en-US</dc:language>
</cp:coreProperties>
"""

STYLES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles {W}>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Heading1"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet"><w:name w:val="List Bullet"/><w:basedOn w:val="Normal"/><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr></w:style>
  <w:style w:type="character" w:styleId="Strong"><w:name w:val="Strong"/><w:rPr><w:b/></w:rPr></w:style>
</w:styles>
"""

NUMBERING = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering {W}>
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/></w:lvl>
    <w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="o"/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl>
    <w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="%1.%2)"/></w:lvl>
  </w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
  <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
</w:numbering>
"""

FOOTNOTES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes {W}>
  <w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote>
  <w:footnote w:type="continuationSeparator" w:id="0"><w:p><w:r><w:continuationSeparator/></w:r></w:p></w:footnote>
  <w:footnote w:id="1"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:t xml:space="preserve"> The footnote text lives here.</w:t></w:r></w:p></w:footnote>
</w:footnotes>
"""

ENDNOTES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:endnotes {W}>
  <w:endnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:endnote>
  <w:endnote w:id="1"><w:p><w:r><w:endnoteRef/></w:r><w:r><w:t xml:space="preserve"> An endnote at the very end.</w:t></w:r></w:p></w:endnote>
</w:endnotes>
"""


def p(text, style=None, extra=""):
    ppr = f'<w:pPr><w:pStyle w:val="{style}"/>{extra}</w:pPr>' if style else (
        f"<w:pPr>{extra}</w:pPr>" if extra else ""
    )
    return f"<w:p>{ppr}{text}</w:p>"


def r(text, props=""):
    rpr = f"<w:rPr>{props}</w:rPr>" if props else ""
    return f'<w:r>{rpr}<w:t xml:space="preserve">{text}</w:t></w:r>'


def num(num_id, ilvl=0):
    return f'<w:numPr><w:ilvl w:val="{ilvl}"/><w:numId w:val="{num_id}"/></w:numPr>'


def drawing(descr, title=""):
    return (
        f"<w:r><w:drawing><wp:inline><wp:docPr id=\"1\" name=\"Picture 1\" descr=\"{descr}\" title=\"{title}\"/>"
        "<a:graphic><a:graphicData/></a:graphic></wp:inline></w:drawing></w:r>"
    )


def cell(*paras):
    return "<w:tc>" + "".join(paras) + "</w:tc>"


BODY = "".join([
    p(r("Sample Word Document"), "Title"),
    p(r("Introduction"), "Heading1"),
    p(
        r("This paragraph has ")
        + r("bold text", "<w:b/>")
        + r(", ")
        + r("italic text", "<w:i/>")
        + r(", ")
        + r("underlined words", '<w:u w:val="single"/>')
        + r(", ")
        + r("strong style", '<w:rStyle w:val="Strong"/>')
        + r(", not bold", '<w:b w:val="0"/>')
        + r(", and a ")
        + '<w:hyperlink r:id="rId3">' + r("link to the example site") + "</w:hyperlink>"
        + r(". Dr. Jones arrived at 3:30 p.m.")
        + '<w:r><w:footnoteReference w:id="1"/></w:r>'
        + r(" It ends here.")
    ),
    p(
        r("Tab")
        + "<w:r><w:tab/></w:r>"
        + r("separated")
        + "<w:r><w:br/></w:r>"
        + r("second line with well")
        + "<w:r><w:noBreakHyphen/></w:r>"
        + r("known words, ")
        + '<w:del w:id="1" w:author="x"><w:r><w:delText>removed </w:delText></w:r></w:del>'
        + '<w:ins w:id="2" w:author="x">' + r("inserted ") + "</w:ins>"
        + '<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r>'
        + '<w:r><w:fldChar w:fldCharType="separate"/></w:r>' + r("7")
        + '<w:r><w:fldChar w:fldCharType="end"/></w:r>'
        + r(" and a field.")
    ),
    p(r("Lists"), "Heading1"),
    p(r("First bullet item"), "ListBullet"),
    p(r("Second bullet item with ") + r("emphasis", "<w:i/>"), "ListBullet"),
    p(r("Nested bullet under the second"), "ListBullet", num(1, 1)),
    p(r("Third bullet item"), "ListBullet"),
    p(r("Step one of the procedure."), None, num(2)),
    p(r("Step two costs $5.25."), None, num(2)),
    p(r("Sub-step a."), None, num(2, 1)),
    p(r("Sub-step b."), None, num(2, 1)),
    p(r("Step three."), None, num(2)),
    p(r("A Table"), "Heading2"),
    "<w:tbl><w:tblPr><w:tblLook w:val=\"04A0\" w:firstRow=\"1\"/></w:tblPr>"
    + "<w:tr>" + cell(p(r("Name"))) + cell(p(r("Role"))) + cell(p(r("Score"))) + "</w:tr>"
    + "<w:tr>" + cell(p(r("Ada"))) + cell(p(r("Engineer")), p(r("and poet"))) + cell(p(r("98"))) + "</w:tr>"
    + "<w:tr>" + cell(p(r("Grace", "<w:b/>"))) + cell(p(r("Admiral"))) + cell(p("")) + "</w:tr>"
    + "</w:tbl>",
    p(drawing("A diagram of the reading pipeline") + drawing("")),
    p(r("Outline Heading"), None, '<w:outlineLvl w:val="1"/>'),
    '<w:sdt><w:sdtPr/><w:sdtContent>' + p(r("Inside a content control.")) + "</w:sdtContent></w:sdt>",
    p(r("The final paragraph mentions an endnote.") + '<w:r><w:endnoteReference w:id="1"/></w:r>' + r(" Does it end with a question?")),
    '<w:sectPr><w:pgSz w:w="12240" w:h="15840"/></w:sectPr>',
])

DOCUMENT = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {W} {R} {WP} {A}>
  <w:body>{BODY}</w:body>
</w:document>
"""


def make_docx():
    write_zip(
        HERE / "sample.docx",
        [
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", ROOT_RELS),
            ("docProps/core.xml", CORE),
            ("word/document.xml", DOCUMENT),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
            ("word/numbering.xml", NUMBERING),
            ("word/footnotes.xml", FOOTNOTES),
            ("word/endnotes.xml", ENDNOTES),
        ],
    )


if __name__ == "__main__":
    make_epub()
    make_docx()
    print("wrote sample.epub and sample.docx")
