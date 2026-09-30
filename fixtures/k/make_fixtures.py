"""Builds the W6k conversion-matrix fixtures that are packages:
lesson.pptx. The text fixtures beside it (book.dtbook, grades.csv) are
written by hand.

Run from the repository root with `py -3 fixtures/k/make_fixtures.py`
(Windows) or `python3 fixtures/k/make_fixtures.py`. The zip members get a
fixed date, so the file comes out the same on every run. Names are
placeholders ("Ada Example").
"""

import pathlib
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
DATE = (2026, 9, 1, 10, 0, 0)

P = (
    'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
    'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
)
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"


def rels(items):
    body = "".join(
        f'<Relationship Id="{i}" Type="{REL}{t}" Target="{target}"/>'
        for i, t, target in items
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        f"{body}</Relationships>"
    )


def shape(n, placeholder, paragraphs):
    ph = f'<p:ph type="{placeholder}"/>' if placeholder else ""
    paras = "".join(f"<a:p><a:r><a:t>{t}</a:t></a:r></a:p>" for t in paragraphs)
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{n}" name="Shape {n}"/><p:cNvSpPr/>'
        f"<p:nvPr>{ph}</p:nvPr></p:nvSpPr><p:txBody>{paras}</p:txBody></p:sp>"
    )


def slide(title, body):
    return (
        f'<?xml version="1.0" encoding="UTF-8"?><p:sld {P}><p:cSld><p:spTree>'
        + shape(2, "title", [title])
        + shape(3, "body", body)
        + "</p:spTree></p:cSld></p:sld>"
    )


def pptx(path):
    files = {
        "[Content_Types].xml": (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>'
            '<Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>'
            '<Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>'
            "</Types>"
        ),
        "_rels/.rels": rels([("rId1", "officeDocument", "ppt/presentation.xml")]),
        "ppt/presentation.xml": (
            f'<?xml version="1.0" encoding="UTF-8"?><p:presentation {P}>'
            '<p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/></p:sldIdLst>'
            "</p:presentation>"
        ),
        "ppt/_rels/presentation.xml.rels": rels(
            [
                ("rId2", "slide", "slides/slide1.xml"),
                ("rId3", "slide", "slides/slide2.xml"),
            ]
        ),
        "ppt/slides/slide1.xml": slide(
            "Crows in the City", ["They remember faces.", "They use tools."]
        ),
        "ppt/slides/slide2.xml": slide(
            "Counting Crows", ["Count at dusk.", "Write down the roost."]
        ),
        "docProps/core.xml": (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" '
            'xmlns:dc="http://purl.org/dc/elements/1.1/">'
            "<dc:title>Crow Lesson</dc:title><dc:creator>Ada Example</dc:creator>"
            "</cp:coreProperties>"
        ),
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, body in files.items():
            info = zipfile.ZipInfo(name, DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, body.encode("utf-8"))


if __name__ == "__main__":
    pptx(HERE / "lesson.pptx")
    print("wrote", HERE / "lesson.pptx")
