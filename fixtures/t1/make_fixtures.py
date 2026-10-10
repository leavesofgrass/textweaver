"""Builds the tracked changes fixtures: changes.docx (task B1-t1) and
word-review.docx (task B1-t2).

Run from the repository root with `py -3 fixtures/t1/make_fixtures.py`
(Windows) or `python3 fixtures/t1/make_fixtures.py`. The zip members get a
fixed date, so the file comes out the same on every run. Names are
placeholders ("Ada Example", "Bo Example").

The draft has an insertion, a deletion, a move (its two halves), a change
with no date, and threaded comments: one with a reply that is resolved, one
open with no date.

It also builds word-review.docx (task B1-t2), laid out as Word writes a
document, for writing a review back into the file in place; see the notes
above REVIEW_BODY.
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


# ----- word-review.docx (task B1-t2) -----------------------------------
#
# A document laid out the way Word writes one, for writing a review back
# in place: revision ids and rsids on every change, named move ranges, a
# whole paragraph deleted (its runs and its paragraph mark), formatting
# changes, an inserted table row, an insertion in the header, numbering,
# settings with tracking on, and threaded comments with commentsIds. The
# parts textweaver does not model (theme, settings, numbering, fonts, app
# properties) must come through a save byte for byte.

A = 'w:author="Ada Example" w:date="2026-03-03T09:15:00Z"'
B = 'w:author="Bo Example" w:date="2026-03-04T14:00:00Z"'
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"
XML_HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\r\n'


def wrun(text, props=""):
    rpr = f"<w:rPr>{props}</w:rPr>" if props else ""
    return f'<w:r w:rsidR="00A1B2C3">{rpr}<w:t xml:space="preserve">{text}</w:t></w:r>'


def wdel(text):
    return f'<w:r w:rsidDel="00D4E5F6"><w:delText xml:space="preserve">{text}</w:delText></w:r>'


def para(content, ppr="", pid="1A000001"):
    pp = f"<w:pPr>{ppr}</w:pPr>" if ppr else ""
    return f'<w:p w14:paraId="{pid}" w14:textId="77777777" w:rsidR="00A1B2C3">{pp}{content}</w:p>'


REVIEW_BODY = (
    para(wrun("Discharge Plan"), '<w:pStyle w:val="Heading1"/>', "1A000001")
    + para(
        wrun("The patient may go home ")
        + f'<w:ins w:id="10" {A}>' + wrun("tomorrow ") + "</w:ins>"
        + wrun("after ")
        + f'<w:del w:id="11" {B}>' + wdel("two ") + "</w:del>"
        + wrun("review."),
        "", "1A000002")
    # A whole paragraph deleted: its text and its paragraph mark.
    + para(
        f'<w:del w:id="13" {B}>' + wdel("Keep the drain in place.") + "</w:del>",
        f'<w:rPr><w:del w:id="12" {B}/></w:rPr>', "1A000003")
    + para(
        '<w:commentRangeStart w:id="0"/>' + wrun("Call the family.")
        + '<w:commentRangeEnd w:id="0"/>'
        + '<w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="0"/></w:r>'
        + '<w:commentRangeStart w:id="1"/><w:commentRangeEnd w:id="1"/>'
        + '<w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="1"/></w:r>',
        "", "1A000004")
    + para(
        f'<w:moveFromRangeStart w:id="20" w:name="move1" {A}/>'
        + f'<w:moveFrom w:id="21" {A}>' + wrun("Check the labs first. ") + "</w:moveFrom>"
        + '<w:moveFromRangeEnd w:id="20"/>'
        + wrun("Give fluids."),
        "", "1A000005")
    + para(
        wrun("Then ")
        + f'<w:moveToRangeStart w:id="22" w:name="move1" {A}/>'
        + f'<w:moveTo w:id="23" {A}>' + wrun("check the labs first") + "</w:moveTo>"
        + '<w:moveToRangeEnd w:id="22"/>' + wrun("."),
        '<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>', "1A000006")
    + para(
        wrun("Watch the dose.", f'<w:b/><w:rPrChange w:id="30" {A}><w:rPr><w:i/></w:rPr></w:rPrChange>')
        + '<w:commentRangeStart w:id="2"/>' + wrun(" Weigh daily.") + '<w:commentRangeEnd w:id="2"/>'
        + '<w:r><w:commentReference w:id="2"/></w:r>',
        f'<w:jc w:val="center"/><w:pPrChange w:id="31" {A}><w:pPr><w:jc w:val="left"/></w:pPr></w:pPrChange>',
        "1A000007")
    + '<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr><w:tblGrid><w:gridCol w:w="4000"/></w:tblGrid>'
    + '<w:tr><w:tc>' + para(wrun("Day one"), "", "1A000008") + "</w:tc></w:tr>"
    + f'<w:tr><w:trPr><w:ins w:id="40" {B}/></w:trPr><w:tc>'
    + para(f'<w:ins w:id="41" {B}>' + wrun("Day two") + "</w:ins>", "", "1A000009")
    + "</w:tc></w:tr></w:tbl>"
    + para(wrun("Signed."), "", "1A00000A")
    + '<w:sectPr><w:headerReference w:type="default" r:id="rId8"/><w:pgSz w:w="12240" w:h="15840"/></w:sectPr>'
)

REVIEW_DOCUMENT = (
    XML_HEAD
    + f'<w:document xmlns:mc="{MC}" '
    + 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
    + f'xmlns:w="{W}" xmlns:w14="{W14}" mc:Ignorable="w14">'
    + f"<w:body>{REVIEW_BODY}</w:body></w:document>"
)

REVIEW_HEADER = (
    XML_HEAD + f'<w:hdr xmlns:w="{W}">'
    + "<w:p>" + wrun("Ward 7 ") + f'<w:ins w:id="50" {A}>' + wrun("draft") + "</w:ins></w:p></w:hdr>"
)

REVIEW_STYLES = (
    XML_HEAD + f'<w:styles xmlns:w="{W}">'
    '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>'
    '<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/></w:style>'
    '<w:style w:type="character" w:styleId="CommentReference"><w:name w:val="annotation reference"/></w:style>'
    "</w:styles>"
)

REVIEW_NUMBERING = (
    XML_HEAD + f'<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0">'
    '<w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl></w:abstractNum>'
    '<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>'
)

REVIEW_SETTINGS = (
    XML_HEAD + f'<w:settings xmlns:w="{W}"><w:trackRevisions/><w:defaultTabStop w:val="720"/>'
    '<w:rsids><w:rsidRoot w:val="00A1B2C3"/><w:rsid w:val="00D4E5F6"/></w:rsids></w:settings>'
)

REVIEW_THEME = (
    XML_HEAD + '<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">'
    "<a:themeElements/></a:theme>"
)

REVIEW_FONTS = XML_HEAD + f'<w:fonts xmlns:w="{W}"><w:font w:name="Calibri"/></w:fonts>'

REVIEW_COMMENTS = (
    XML_HEAD
    + f'<w:comments xmlns:mc="{MC}" xmlns:w="{W}" xmlns:w14="{W14}" mc:Ignorable="w14">'
    + '<w:comment w:id="0" w:author="Bo Example" w:date="2026-03-04T15:00:00Z" w:initials="BE">'
    + '<w:p w14:paraId="2C000001" w14:textId="77777777"><w:r><w:annotationRef/></w:r>'
    + run("Which number?") + "</w:p></w:comment>"
    + '<w:comment w:id="1" w:author="Ada Example" w:date="2026-03-05T09:00:00Z" w:initials="AE">'
    + '<w:p w14:paraId="2C000002" w14:textId="77777777"><w:r><w:annotationRef/></w:r>'
    + run("The ward line.") + "</w:p></w:comment>"
    + '<w:comment w:id="2" w:author="Ada Example" w:date="2026-03-05T09:30:00Z" w:initials="AE">'
    + '<w:p w14:paraId="2C000003" w14:textId="77777777"><w:r><w:annotationRef/></w:r>'
    + run("In kilograms?") + "</w:p></w:comment>"
    + "</w:comments>"
)

REVIEW_EXTENDED = (
    XML_HEAD + f'<w15:commentsEx xmlns:mc="{MC}" xmlns:w15="{W15}" mc:Ignorable="w15">'
    '<w15:commentEx w15:paraId="2C000001" w15:done="0"/>'
    '<w15:commentEx w15:paraId="2C000002" w15:paraIdParent="2C000001" w15:done="0"/>'
    '<w15:commentEx w15:paraId="2C000003" w15:done="0"/>'
    "</w15:commentsEx>"
)

W16CID = "http://schemas.microsoft.com/office/word/2016/wordml/cid"
REVIEW_IDS = (
    XML_HEAD + f'<w16cid:commentsIds xmlns:mc="{MC}" xmlns:w16cid="{W16CID}" mc:Ignorable="w16cid">'
    '<w16cid:commentId w16cid:paraId="2C000001" w16cid:durableId="3D000001"/>'
    '<w16cid:commentId w16cid:paraId="2C000002" w16cid:durableId="3D000002"/>'
    '<w16cid:commentId w16cid:paraId="2C000003" w16cid:durableId="3D000003"/>'
    "</w16cid:commentsIds>"
)

REVIEW_APP = (
    XML_HEAD + '<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">'
    "<Application>Microsoft Office Word</Application></Properties>"
)

REVIEW_CORE = (
    XML_HEAD + '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
    'xmlns:dc="http://purl.org/dc/elements/1.1/">'
    "<dc:title>Discharge Plan</dc:title><dc:creator>Ada Example</dc:creator></cp:coreProperties>"
)

WP = "application/vnd.openxmlformats-officedocument.wordprocessingml"
REVIEW_TYPES = (
    XML_HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    + "".join(
        f'<Override PartName="/word/{p}.xml" ContentType="{WP}.{t}+xml"/>'
        for p, t in [
            ("document", "document.main"), ("styles", "styles"), ("numbering", "numbering"),
            ("settings", "settings"), ("fontTable", "fontTable"), ("header1", "header"),
            ("comments", "comments"), ("commentsExtended", "commentsExtended"),
            ("commentsIds", "commentsIds"),
        ]
    )
    + '<Override PartName="/word/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>'
    '<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>'
    '<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>'
    "</Types>"
)

R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
REVIEW_RELS = (
    XML_HEAD + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    f'<Relationship Id="rId1" Type="{R}/officeDocument" Target="word/document.xml"/>'
    '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>'
    f'<Relationship Id="rId3" Type="{R}/extended-properties" Target="docProps/app.xml"/>'
    "</Relationships>"
)

REVIEW_DOC_RELS = (
    XML_HEAD + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    + "".join(
        f'<Relationship Id="rId{i}" Type="{t}" Target="{target}"/>'
        for i, (t, target) in enumerate(
            [
                (f"{R}/styles", "styles.xml"),
                (f"{R}/numbering", "numbering.xml"),
                (f"{R}/settings", "settings.xml"),
                (f"{R}/fontTable", "fontTable.xml"),
                (f"{R}/theme", "theme/theme1.xml"),
                (f"{R}/comments", "comments.xml"),
                ("http://schemas.microsoft.com/office/2011/relationships/commentsExtended", "commentsExtended.xml"),
                (f"{R}/header", "header1.xml"),
                ("http://schemas.microsoft.com/office/2016/09/relationships/commentsIds", "commentsIds.xml"),
            ],
            start=1,
        )
    )
    + "</Relationships>"
)


def write_package(name, parts):
    with zipfile.ZipFile(HERE / name, "w") as z:
        for part_name, data in parts:
            info = zipfile.ZipInfo(part_name, DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    print("Wrote", name, "in", HERE)


def write_review():
    write_package(
        "word-review.docx",
        [
            ("[Content_Types].xml", REVIEW_TYPES),
            ("_rels/.rels", REVIEW_RELS),
            ("word/document.xml", REVIEW_DOCUMENT),
            ("word/styles.xml", REVIEW_STYLES),
            ("word/numbering.xml", REVIEW_NUMBERING),
            ("word/settings.xml", REVIEW_SETTINGS),
            ("word/fontTable.xml", REVIEW_FONTS),
            ("word/theme/theme1.xml", REVIEW_THEME),
            ("word/header1.xml", REVIEW_HEADER),
            ("word/comments.xml", REVIEW_COMMENTS),
            ("word/commentsExtended.xml", REVIEW_EXTENDED),
            ("word/commentsIds.xml", REVIEW_IDS),
            ("word/_rels/document.xml.rels", REVIEW_DOC_RELS),
            ("docProps/core.xml", REVIEW_CORE),
            ("docProps/app.xml", REVIEW_APP),
        ],
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
    write_review()


if __name__ == "__main__":
    main()
