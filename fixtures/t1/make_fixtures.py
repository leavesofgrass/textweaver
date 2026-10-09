"""Builds the tracked changes fixture (task B1-t1): changes.docx.

Run from the repository root with `py -3 fixtures/t1/make_fixtures.py`
(Windows) or `python3 fixtures/t1/make_fixtures.py`. The zip members get a
fixed date, so the file comes out the same on every run. Names are
placeholders ("Ada Example", "Bo Example").

The draft has an insertion, a deletion, a move (its two halves), a change
with no date, and threaded comments: one with a reply that is resolved, one
open with no date.
"""

import pathlib
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
DATE = (2026, 9, 1, 10, 0, 0)

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W14 = "http://schemas.microsoft.com/office/word/2010/wordml"
W15 = "http://schemas.microsoft.com/office/word/2012/wordml"


def run(text):
    return f'<w:r><w:t xml:space="preserve">{text}</w:t></w:r>'


def ins(i, author, date, text):
    when = f' w:date="{date}"' if date else ""
    return f'<w:ins w:id="{i}" w:author="{author}"{when}>{run(text)}</w:ins>'


def dele(i, author, date, text):
    return (
        f'<w:del w:id="{i}" w:author="{author}" w:date="{date}"><w:r>'
        f'<w:delText xml:space="preserve">{text}</w:delText></w:r></w:del>'
    )


BODY = (
    '<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' + run("Case Notes") + "</w:p>"
    "<w:p>" + run("The patient has acute ")
    + ins(1, "Ada Example", "2026-03-03T09:15:00Z", "renal")
    + run(" failure.") + "</w:p>"
    "<w:p>" + run("Fluids were ")
    + dele(2, "Bo Example", "2026-03-04T14:00:00Z", "rarely ")
    + run("given every hour.") + "</w:p>"
    "<w:p>"
    + '<w:moveFrom w:id="3" w:author="Ada Example" w:date="2026-03-05T08:00:00Z">'
    + run("Check the labs first. ") + "</w:moveFrom>"
    + '<w:commentRangeStart w:id="0"/>' + run("Call the family.")
    + '<w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r>'
    + "</w:p>"
    "<w:p>" + run("Then ")
    + '<w:moveTo w:id="4" w:author="Ada Example" w:date="2026-03-05T08:00:00Z">'
    + run("check the labs first") + "</w:moveTo>"
    + run(".") + ins(5, "Bo Example", None, " Repeat tomorrow.")
    + '<w:r><w:commentReference w:id="2"/></w:r></w:p>'
)

DOCUMENT = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:document xmlns:w="{W}"><w:body>{BODY}</w:body></w:document>'
)

STYLES = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Heading1">'
    f'<w:name w:val="heading 1"/></w:style></w:styles>'
)

COMMENTS = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:comments xmlns:w="{W}" xmlns:w14="{W14}">'
    '<w:comment w:id="0" w:author="Bo Example" w:date="2026-03-04T15:00:00Z" w:initials="BE">'
    '<w:p w14:paraId="2B000001"><w:r><w:annotationRef/></w:r>' + run("check this date") + "</w:p></w:comment>"
    '<w:comment w:id="1" w:author="Ada Example" w:date="2026-03-05T09:00:00Z" w:initials="AE">'
    '<w:p w14:paraId="2B000002"><w:r><w:annotationRef/></w:r>' + run("Done, it is March 4.") + "</w:p></w:comment>"
    '<w:comment w:id="2" w:author="Ada Example" w:initials="AE">'
    '<w:p w14:paraId="2B000003"><w:r><w:annotationRef/></w:r>' + run("Who repeats it?") + "</w:p></w:comment>"
    "</w:comments>"
)

COMMENTS_EXTENDED = (
    f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w15:commentsEx xmlns:w15="{W15}">'
    '<w15:commentEx w15:paraId="2B000001" w15:done="1"/>'
    '<w15:commentEx w15:paraId="2B000002" w15:paraIdParent="2B000001" w15:done="0"/>'
    '<w15:commentEx w15:paraId="2B000003" w15:done="0"/>'
    "</w15:commentsEx>"
)

CORE = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
    'xmlns:dc="http://purl.org/dc/elements/1.1/">'
    "<dc:title>Case Notes</dc:title><dc:creator>Ada Example</dc:creator></cp:coreProperties>"
)

TYPES = (
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

RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
    '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>'
    "</Relationships>"
)

DOC_RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
    '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>'
    '<Relationship Id="rId3" Type="http://schemas.microsoft.com/office/2011/relationships/commentsExtended" Target="commentsExtended.xml"/>'
    "</Relationships>"
)


def main():
    with zipfile.ZipFile(HERE / "changes.docx", "w") as z:
        for name, data in [
            ("[Content_Types].xml", TYPES),
            ("_rels/.rels", RELS),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/document.xml", DOCUMENT),
            ("word/styles.xml", STYLES),
            ("word/comments.xml", COMMENTS),
            ("word/commentsExtended.xml", COMMENTS_EXTENDED),
            ("docProps/core.xml", CORE),
        ]:
            info = zipfile.ZipInfo(name, DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    print("Wrote changes.docx in", HERE)


if __name__ == "__main__":
    main()
