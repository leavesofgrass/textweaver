"""Builds the W4c2 fixtures: handout.rtf, notes.odt, and comments.docx.

Run from the repository root with `py -3 fixtures/c2/make_fixtures.py`
(Windows) or `python3 fixtures/c2/make_fixtures.py`. The zip members get a
fixed date, so the files come out the same on every run. Names are
placeholders ("Ada Example", "Bo Example").
"""

import pathlib
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
BS = chr(92)  # a backslash, written this way so no tool rewrites RTF escapes
DATE = (2026, 9, 1, 10, 0, 0)


def rtf(text):
    """RTF source with every '~' turned into a backslash."""
    return text.replace("~", BS)


def uni(n):
    """An RTF Unicode character with a '?' fallback."""
    return BS + "u" + str(n) + "?"


HANDOUT = rtf(
    "{~rtf1~ansi~ansicpg1252~deff0"
    "{~fonttbl{~f0~fswiss~fcharset0 Arial;}{~f1~froman~fcharset204 Times New Roman Cyr;}}"
    "{~stylesheet{~s0 Normal;}{~s1~sbasedon0 heading 1;}{~s2~sbasedon0 heading 2;}}"
    "{~*~revtbl {Unknown;}{Ada Example;}}"
    "{~info{~title Week 3 Handout: Reading Primary Sources}{~author Ada Example}}"
    "~pard~s1 Week 3 Handout: Reading Primary Sources~par"
    "~pard~plain Read each source twice: once for what it says, and once for "
    "{~b who wrote it} and {~i why}. A caf~'e9 ledger from 1911 counts as a source"
    + uni(8212)
    + "so does a crow count survey.{~super~chftn}{~footnote~pard~plain{~super~chftn} "
    "Portland Audubon runs a winter crow count every year.}~par"
    "~pard~s2 Steps~par"
    "~pard~ls1~ilvl0{~listtext 1.~tab}Find the author and the date.~par"
    "{~listtext 2.~tab}Ask who the source was written for.~par"
    "{~listtext 3.~tab}Compare it with one other source.~par"
    "~pard~s2 Sources for this week~par"
    "~trowd~trhdr~cellx3000~cellx6000~pard~intbl Source~cell Year~cell~row"
    "~trowd~cellx3000~cellx6000~pard~intbl Caf~'e9 ledger~cell 1911~cell~row"
    "~trowd~cellx3000~cellx6000~pard~intbl Crow count~cell 2025~cell~row"
    "~pard~plain The word for crow in Russian is {~f1 ~'e2~'ee~'f0~'ee~'ed~'e0}. "
    "See {~field{~*~fldinst {HYPERLINK \"https://example.org/sources\"}}{~fldrslt {~ul the course page}}}"
    " for more. Bring {~deleted~revauthdel1 two }{~revised~revauth1 three }questions.~par"
    "}"
)

NS = (
    'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
    'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
    'xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" '
    'xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" '
    'xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" '
    'xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" '
    'xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" '
    'xmlns:xlink="http://www.w3.org/1999/xlink" '
    'xmlns:dc="http://purl.org/dc/elements/1.1/" '
    'xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" '
    'xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" '
    'office:version="1.4"'
)

ODT_CONTENT = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {NS}>
<office:automatic-styles>
<style:style style:name="T1" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
<text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/><text:list-level-style-number text:level="2" style:num-format="a" style:num-suffix=")"/></text:list-style>
</office:automatic-styles>
<office:body><office:text>
<text:tracked-changes>
<text:changed-region text:id="ct1"><text:insertion><office:change-info><dc:creator>Ada Example</dc:creator><dc:date>2026-09-01T10:00:00</dc:date></office:change-info></text:insertion></text:changed-region>
<text:changed-region text:id="ct2"><text:deletion><office:change-info><dc:creator>Bo Example</dc:creator><dc:date>2026-09-02T09:30:00</dc:date></office:change-info><text:p>Tuesday</text:p></text:deletion></text:changed-region>
</text:tracked-changes>
<text:h text:outline-level="1">Lecture Notes: The Water Cycle</text:h>
<text:p>Water moves between the <text:span text:style-name="T1">ocean</text:span>, the air, and the land.<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>Chapter 4 of the course reader.</text:p></text:note-body></text:note> <office:annotation office:name="c1"><dc:creator>Ada Example</dc:creator><dc:date>2026-09-01T11:00:00</dc:date><text:p>This is on the exam.</text:p></office:annotation>Evaporation is the first step<office:annotation-end office:name="c1"/>.</text:p>
<text:h text:outline-level="2">Stages</text:h>
<text:list text:style-name="L1">
<text:list-item><text:p>Evaporation</text:p><text:list><text:list-item><text:p>From oceans and lakes</text:p></text:list-item><text:list-item><text:p>From leaves, called transpiration</text:p></text:list-item></text:list></text:list-item>
<text:list-item><text:p>Condensation</text:p></text:list-item>
<text:list-item><text:p>Precipitation</text:p></text:list-item>
</text:list>
<text:p><draw:frame draw:name="Figure 1"><draw:image xlink:href="Pictures/cycle.png"/><svg:title>A diagram of the water cycle, with arrows from the ocean to a cloud to rain on a hill</svg:title></draw:frame></text:p>
<text:h text:outline-level="2">Rainfall by city</text:h>
<table:table table:name="Rainfall">
<table:table-column table:number-columns-repeated="2"/>
<table:table-header-rows><table:table-row><table:table-cell><text:p>City</text:p></table:table-cell><table:table-cell><text:p>Inches a year</text:p></table:table-cell></table:table-row></table:table-header-rows>
<table:table-row><table:table-cell><text:p>Portland</text:p></table:table-cell><table:table-cell><text:p>36</text:p></table:table-cell></table:table-row>
<table:table-row><table:table-cell><text:p>Phoenix</text:p></table:table-cell><table:table-cell><text:p>8</text:p></table:table-cell></table:table-row>
</table:table>
<text:p>The quiz is on <text:change text:change-id="ct2"/><text:change-start text:change-id="ct1"/>Thursday<text:change-end text:change-id="ct1"/>. Read more at <text:a xlink:href="https://example.org/water">the course page</text:a>.</text:p>
</office:text></office:body>
</office:document-content>
"""

ODT_META = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta {NS}><office:meta><dc:title>Lecture Notes: The Water Cycle</dc:title><meta:initial-creator>Ada Example</meta:initial-creator><dc:language>en-US</dc:language></office:meta></office:document-meta>
"""

ODT_MANIFEST = """<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.4">
<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W14 = "http://schemas.microsoft.com/office/word/2010/wordml"
W15 = "http://schemas.microsoft.com/office/word/2012/wordml"


def run(text):
    return f'<w:r><w:t xml:space="preserve">{text}</w:t></w:r>'


DOCX_BODY = (
    '<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' + run("Lab Report Draft") + "</w:p>"
    "<w:p>" + run("We measured the ")
    + '<w:commentRangeStart w:id="0"/>' + run("boiling point of salt water")
    + '<w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r>'
    + run(" at ")
    + '<w:del w:id="10" w:author="Bo Example" w:date="2026-09-02T09:00:00Z"><w:r><w:delText xml:space="preserve">three </w:delText></w:r></w:del>'
    + '<w:ins w:id="11" w:author="Ada Example" w:date="2026-09-02T09:05:00Z">' + run("five ") + "</w:ins>"
    + run("concentrations.") + "</w:p>"
    "<w:p>" + run("The results table is below.")
    + '<w:r><w:commentReference w:id="2"/></w:r></w:p>'
)

DOCX_DOCUMENT = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:document xmlns:w="{W}"><w:body>{DOCX_BODY}</w:body></w:document>'
)

DOCX_STYLES = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Heading1">'
    f'<w:name w:val="heading 1"/></w:style></w:styles>'
)

DOCX_COMMENTS = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:comments xmlns:w="{W}" xmlns:w14="{W14}">'
    '<w:comment w:id="0" w:author="Ada Example" w:date="2026-09-02T10:00:00Z" w:initials="AE">'
    '<w:p w14:paraId="1A000001"><w:r><w:annotationRef/></w:r>' + run("Say how salty, in grams per liter.") + "</w:p></w:comment>"
    '<w:comment w:id="1" w:author="Bo Example" w:date="2026-09-02T11:00:00Z" w:initials="BE">'
    '<w:p w14:paraId="1A000002"><w:r><w:annotationRef/></w:r>' + run("Added: 35 grams per liter.") + "</w:p></w:comment>"
    '<w:comment w:id="2" w:author="Bo Example" w:date="2026-09-02T11:05:00Z" w:initials="BE">'
    '<w:p w14:paraId="1A000003"><w:r><w:annotationRef/></w:r>' + run("The table is missing.") + "</w:p></w:comment>"
    "</w:comments>"
)

DOCX_COMMENTS_EXTENDED = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w15:commentsEx xmlns:w15="{W15}">'
    '<w15:commentEx w15:paraId="1A000001" w15:done="1"/>'
    '<w15:commentEx w15:paraId="1A000002" w15:paraIdParent="1A000001" w15:done="0"/>'
    '<w15:commentEx w15:paraId="1A000003" w15:done="0"/>'
    "</w15:commentsEx>"
)

DOCX_CORE = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
    'xmlns:dc="http://purl.org/dc/elements/1.1/">'
    "<dc:title>Lab Report Draft</dc:title><dc:creator>Ada Example</dc:creator></cp:coreProperties>"
)

DOCX_TYPES = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
    '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
    '<Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>'
    '<Override PartName="/word/commentsExtended.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml"/>'
    '<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>'
    "</Types>"
)

DOCX_RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
    '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>'
    "</Relationships>"
)

DOCX_DOC_RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
    '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>'
    '<Relationship Id="rId3" Type="http://schemas.microsoft.com/office/2011/relationships/commentsExtended" Target="commentsExtended.xml"/>'
    "</Relationships>"
)


def write_zip(path, members, stored_first=None):
    with zipfile.ZipFile(path, "w") as z:
        if stored_first:
            name, data = stored_first
            info = zipfile.ZipInfo(name, DATE)
            info.compress_type = zipfile.ZIP_STORED
            z.writestr(info, data)
        for name, data in members:
            info = zipfile.ZipInfo(name, DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)


def main():
    (HERE / "handout.rtf").write_bytes(HANDOUT.encode("ascii"))
    write_zip(
        HERE / "notes.odt",
        [
            ("META-INF/manifest.xml", ODT_MANIFEST),
            ("content.xml", ODT_CONTENT),
            ("meta.xml", ODT_META),
        ],
        stored_first=("mimetype", "application/vnd.oasis.opendocument.text"),
    )
    write_zip(
        HERE / "comments.docx",
        [
            ("[Content_Types].xml", DOCX_TYPES),
            ("_rels/.rels", DOCX_RELS),
            ("word/_rels/document.xml.rels", DOCX_DOC_RELS),
            ("word/document.xml", DOCX_DOCUMENT),
            ("word/styles.xml", DOCX_STYLES),
            ("word/comments.xml", DOCX_COMMENTS),
            ("word/commentsExtended.xml", DOCX_COMMENTS_EXTENDED),
            ("docProps/core.xml", DOCX_CORE),
        ],
    )
    print("Wrote handout.rtf, notes.odt, and comments.docx in", HERE)


if __name__ == "__main__":
    main()
